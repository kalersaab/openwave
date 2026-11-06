use anyhow::{Context, Result};
use openwave_core::demux::mp4::{Mp4Reader, TrackType};
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct DecodedFrame {
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub sequence: u64,
    pub captured_at: Duration,
    pub pixels: Vec<u8>,
}

struct PlaybackState {
    path: PathBuf,
    has_audio: bool,
    is_playing: bool,
    started_at: Instant,
    position_ms: i64,
    duration_ms: i64,
    video_width: u32,
    video_height: u32,
}

pub struct NativePlayer {
    state: Arc<Mutex<Option<PlaybackState>>>,
    frame_queue: Arc<Mutex<VecDeque<DecodedFrame>>>,
    ffmpeg_child: Arc<Mutex<Option<Child>>>,
    ffplay_child: Arc<Mutex<Option<Child>>>,
    volume: i32,
    muted: bool,
    rate: f32,
    max_frames: usize,
    sequence: Arc<Mutex<u64>>,
    dropped_frames: Arc<Mutex<u64>>,
}

impl NativePlayer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            state: Arc::new(Mutex::new(None)),
            frame_queue: Arc::new(Mutex::new(VecDeque::new())),
            ffmpeg_child: Arc::new(Mutex::new(None)),
            ffplay_child: Arc::new(Mutex::new(None)),
            volume: 100,
            muted: false,
            rate: 1.0,
            max_frames: 10,
            sequence: Arc::new(Mutex::new(0)),
            dropped_frames: Arc::new(Mutex::new(0)),
        })
    }

    pub fn open<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.open_with_frame_output(path, true)
    }

    pub fn open_with_frame_output<P: AsRef<Path>>(
        &mut self,
        path: P,
        _frame_output_enabled: bool,
    ) -> Result<()> {
        self.stop()?;
        let path = path.as_ref();
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if !matches!(
            extension.to_ascii_lowercase().as_str(),
            "mp4" | "m4v" | "mov"
        ) {
            anyhow::bail!("NativePlayer currently supports MP4/MOV video files");
        }

        let reader = BufReader::new(File::open(path)?);
        let mp4_reader = Mp4Reader::new(reader).context("Failed to parse MP4 video metadata")?;
        let video_track = mp4_reader
            .tracks()
            .iter()
            .find(|track| track.track_type == TrackType::Video)
            .context("No video track found")?;
        let has_audio = mp4_reader
            .tracks()
            .iter()
            .any(|track| track.track_type == TrackType::Audio);
        let timescale = i64::from(video_track.timescale);
        anyhow::ensure!(timescale > 0, "Invalid video track timescale");
        let duration_ms =
            (video_track.duration as u128 * 1000 / timescale as u128).min(i64::MAX as u128) as i64;
        let video_width = video_track.width.context("Video width is missing")?;
        let video_height = video_track.height.context("Video height is missing")?;
        anyhow::ensure!(
            video_width > 0 && video_height > 0,
            "Invalid video dimensions"
        );

        *self.state.lock().unwrap() = Some(PlaybackState {
            path: path.to_path_buf(),
            has_audio,
            is_playing: false,
            started_at: Instant::now(),
            position_ms: 0,
            duration_ms,
            video_width,
            video_height,
        });
        self.frame_queue.lock().unwrap().clear();
        *self.sequence.lock().unwrap() = 0;
        *self.dropped_frames.lock().unwrap() = 0;
        Ok(())
    }

    pub fn play(&mut self) -> Result<()> {
        let Some((path, position_ms, width, height, has_audio)) = ({
            let mut state = self.state.lock().unwrap();
            state.as_mut().map(|state| {
                if !state.is_playing {
                    state.is_playing = true;
                    state.started_at = Instant::now();
                }
                (
                    state.path.clone(),
                    state.position_ms,
                    state.video_width,
                    state.video_height,
                    state.has_audio,
                )
            })
        }) else {
            anyhow::bail!("No video is open");
        };

        if self.ffmpeg_child.lock().unwrap().is_none()
            && let Err(error) =
                self.start_decode_thread(path, position_ms, width, height, has_audio)
        {
            if let Some(state) = self.state.lock().unwrap().as_mut() {
                state.is_playing = false;
            }
            return Err(error);
        }
        Ok(())
    }

    fn start_decode_thread(
        &self,
        path: PathBuf,
        position_ms: i64,
        width: u32,
        height: u32,
        has_audio: bool,
    ) -> Result<()> {
        self.start_audio_player(&path, position_ms, has_audio)?;
        let read_rate = self.rate.clamp(0.25, 4.0);
        let child = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-nostdin"])
            .arg("-ss")
            .arg(format!("{:.3}", position_ms.max(0) as f64 / 1000.0))
            .arg("-readrate")
            .arg(read_rate.to_string())
            .arg("-i")
            .arg(&path)
            .args([
                "-map",
                "0:v:0",
                "-an",
                "-sn",
                "-dn",
                "-fps_mode",
                "passthrough",
                "-pix_fmt",
                "rgb24",
                "-f",
                "rawvideo",
                "pipe:1",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Could not start FFmpeg. Install FFmpeg and ensure it is available on PATH.");
        let mut child = match child {
            Ok(child) => child,
            Err(error) => {
                self.stop_ffplay()?;
                return Err(error);
            }
        };

        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                self.stop_ffplay()?;
                anyhow::bail!("FFmpeg did not provide a video frame stream");
            }
        };
        *self.ffmpeg_child.lock().unwrap() = Some(child);

        let state = Arc::clone(&self.state);
        let frame_queue = Arc::clone(&self.frame_queue);
        let ffmpeg_child = Arc::clone(&self.ffmpeg_child);
        let sequence = Arc::clone(&self.sequence);
        let dropped_frames = Arc::clone(&self.dropped_frames);
        let ffplay_child = Arc::clone(&self.ffplay_child);
        let max_frames = self.max_frames.max(1);
        thread::spawn(move || {
            let frame_size = width as usize * height as usize * 3;
            let mut stdout = BufReader::new(stdout);

            loop {
                let is_playing = state
                    .lock()
                    .unwrap()
                    .as_ref()
                    .is_some_and(|state| state.is_playing);
                if !is_playing {
                    break;
                }

                if frame_queue.lock().unwrap().len() >= max_frames {
                    thread::sleep(Duration::from_millis(4));
                    continue;
                }

                let mut pixels = vec![0; frame_size];
                match stdout.read_exact(&mut pixels) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                        if let Some(state) = state.lock().unwrap().as_mut()
                            && state.is_playing
                        {
                            state.position_ms = state.duration_ms;
                            state.is_playing = false;
                        }
                        stop_child(&ffplay_child);
                        break;
                    }
                    Err(error) => {
                        eprintln!("FFmpeg frame read failed: {error}");
                        if let Some(state) = state.lock().unwrap().as_mut() {
                            state.is_playing = false;
                        }
                        break;
                    }
                }

                let captured_at = {
                    let state = state.lock().unwrap();
                    let Some(state) = state.as_ref().filter(|state| state.is_playing) else {
                        break;
                    };
                    Duration::from_millis(
                        state.position_ms.max(0) as u64
                            + state.started_at.elapsed().as_millis() as u64,
                    )
                };
                let frame_sequence = {
                    let mut sequence = sequence.lock().unwrap();
                    let result = *sequence;
                    *sequence = sequence.wrapping_add(1);
                    result
                };
                let mut frame_queue = frame_queue.lock().unwrap();
                if frame_queue.len() >= max_frames {
                    *dropped_frames.lock().unwrap() += 1;
                    frame_queue.pop_front();
                }
                frame_queue.push_back(DecodedFrame {
                    width,
                    height,
                    pitch: width * 3,
                    sequence: frame_sequence,
                    captured_at,
                    pixels,
                });
            }

            if let Some(mut child) = ffmpeg_child.lock().unwrap().take() {
                let _ = child.wait();
            }
        });
        Ok(())
    }

    fn stop_ffmpeg(&self) -> Result<()> {
        stop_child_result(&self.ffmpeg_child, "FFmpeg decoder")
    }

    fn start_audio_player(&self, path: &Path, position_ms: i64, has_audio: bool) -> Result<()> {
        self.stop_ffplay()?;
        if !has_audio {
            return Ok(());
        }

        let mut command = Command::new("ffplay");
        command
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nodisp",
                "-autoexit",
                "-ss",
            ])
            .arg(format!("{:.3}", position_ms.max(0) as f64 / 1000.0))
            .arg("-i")
            .arg(path)
            .args(["-vn", "-sn", "-volume", "100", "-af"])
            .arg(audio_filter(self.volume, self.muted, self.rate))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());

        let child = command.spawn().context(
            "Could not start FFplay for video audio. Install FFmpeg with FFplay and ensure it is available on PATH.",
        )?;
        *self.ffplay_child.lock().unwrap() = Some(child);
        Ok(())
    }

    fn stop_ffplay(&self) -> Result<()> {
        stop_child_result(&self.ffplay_child, "FFplay audio output")
    }

    fn restart_audio_player(&self) -> Result<()> {
        let (path, position_ms) = {
            let state_guard = self.state.lock().unwrap();
            let Some(state) = state_guard.as_ref() else {
                return Ok(());
            };
            if !state.is_playing || !state.has_audio {
                return Ok(());
            }
            (state.path.clone(), position_with_rate(state, self.rate))
        };
        self.start_audio_player(&path, position_ms, true)
    }

    pub fn sync_audio_output(&self) -> Result<bool> {
        let mut child_guard = self.ffplay_child.lock().unwrap();
        let Some(child) = child_guard.as_mut() else {
            return Ok(false);
        };
        let Some(status) = child
            .try_wait()
            .context("Could not check FFplay audio output")?
        else {
            return Ok(true);
        };
        child_guard.take();
        if !status.success()
            && self
                .state
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|state| state.is_playing)
        {
            anyhow::bail!("FFplay audio output exited with status {status}");
        }
        Ok(false)
    }

    pub fn pause(&mut self) -> Result<()> {
        if let Some(state) = self.state.lock().unwrap().as_mut()
            && state.is_playing
        {
            state.position_ms = position_with_rate(state, self.rate);
            state.is_playing = false;
        }
        self.stop_ffmpeg()?;
        self.stop_ffplay()?;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(state) = self.state.lock().unwrap().as_mut() {
            state.is_playing = false;
            state.position_ms = 0;
        }
        self.stop_ffmpeg()?;
        self.stop_ffplay()?;
        self.frame_queue.lock().unwrap().clear();
        Ok(())
    }

    pub fn is_playing(&self) -> bool {
        self.state
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|state| state.is_playing)
    }

    pub fn set_audio_settings(&mut self, volume: i32, muted: bool, rate: f32) -> Result<()> {
        let volume = volume.clamp(0, 200);
        let rate = rate.clamp(0.25, 4.0);
        let rate_changed = (self.rate - rate).abs() > f32::EPSILON;
        let audio_settings_changed = self.volume != volume || self.muted != muted;
        if !rate_changed && !audio_settings_changed {
            return Ok(());
        }

        let was_playing = self.is_playing();
        let position_ms = self.position();
        self.volume = volume;
        self.muted = muted;
        self.rate = rate;

        if was_playing && rate_changed {
            self.set_position(position_ms)?;
        } else if was_playing && audio_settings_changed {
            self.restart_audio_player()?;
        }
        Ok(())
    }

    pub fn set_volume(&mut self, volume: i32) -> Result<()> {
        self.set_audio_settings(volume, self.muted, self.rate)
    }

    pub fn volume(&self) -> i32 {
        self.volume
    }

    pub fn set_mute(&mut self, muted: bool) -> Result<()> {
        self.set_audio_settings(self.volume, muted, self.rate)
    }

    pub fn is_muted(&self) -> bool {
        self.muted
    }

    pub fn position(&self) -> i64 {
        self.state
            .lock()
            .unwrap()
            .as_ref()
            .map(|state| {
                if state.is_playing {
                    position_with_rate(state, self.rate)
                } else {
                    state.position_ms
                }
            })
            .unwrap_or(0)
    }

    pub fn set_position(&mut self, time_ms: i64) -> Result<()> {
        let (was_playing, position_ms, path, width, height, has_audio) = {
            let mut state_guard = self.state.lock().unwrap();
            let Some(state) = state_guard.as_mut() else {
                return Ok(());
            };
            let was_playing = state.is_playing;
            state.is_playing = false;
            state.position_ms = time_ms.clamp(0, state.duration_ms);
            (
                was_playing,
                state.position_ms,
                state.path.clone(),
                state.video_width,
                state.video_height,
                state.has_audio,
            )
        };

        self.stop_ffmpeg()?;
        self.stop_ffplay()?;
        self.frame_queue.lock().unwrap().clear();
        if was_playing {
            if let Some(state) = self.state.lock().unwrap().as_mut() {
                state.is_playing = true;
                state.started_at = Instant::now();
            }
            self.start_decode_thread(path, position_ms, width, height, has_audio)?;
        }
        Ok(())
    }

    pub fn duration(&self) -> i64 {
        self.state
            .lock()
            .unwrap()
            .as_ref()
            .map(|state| state.duration_ms)
            .unwrap_or(0)
    }

    pub fn set_rate(&mut self, rate: f32) -> Result<()> {
        self.set_audio_settings(self.volume, self.muted, rate)
    }

    pub fn rate(&self) -> f32 {
        self.rate
    }

    pub fn has_media(&self) -> bool {
        self.state.lock().unwrap().is_some()
    }

    pub fn enable_decoded_frames(&mut self, capacity: usize) -> Result<()> {
        self.max_frames = capacity.max(1);
        Ok(())
    }

    pub fn take_decoded_frame(&self) -> Option<DecodedFrame> {
        self.frame_queue.lock().unwrap().pop_front()
    }

    pub fn dropped_decoded_frames(&self) -> u64 {
        *self.dropped_frames.lock().unwrap()
    }

    pub fn video_dimensions(&self) -> Option<(u32, u32)> {
        self.state
            .lock()
            .unwrap()
            .as_ref()
            .map(|state| (state.video_width, state.video_height))
    }
}

impl Drop for NativePlayer {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock()
            && let Some(state) = state.as_mut()
        {
            state.is_playing = false;
        }
        if let Ok(mut child) = self.ffmpeg_child.lock()
            && let Some(mut child) = child.take()
        {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Ok(mut child) = self.ffplay_child.lock()
            && let Some(mut child) = child.take()
        {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn stop_child_result(child: &Mutex<Option<Child>>, label: &str) -> Result<()> {
    if let Some(mut child) = child.lock().unwrap().take() {
        if child
            .try_wait()
            .with_context(|| format!("Failed to check {label} process"))?
            .is_none()
        {
            child
                .kill()
                .with_context(|| format!("Failed to stop {label}"))?;
        }
        child
            .wait()
            .with_context(|| format!("Failed to wait for {label}"))?;
    }
    Ok(())
}

fn stop_child(child: &Mutex<Option<Child>>) {
    if let Ok(mut child_guard) = child.lock()
        && let Some(mut child) = child_guard.take()
    {
        if child.try_wait().ok().flatten().is_none() {
            let _ = child.kill();
        }
        let _ = child.wait();
    }
}

fn position_with_rate(state: &PlaybackState, rate: f32) -> i64 {
    let elapsed_ms = (state.started_at.elapsed().as_secs_f64() * rate as f64 * 1000.0)
        .min(i64::MAX as f64) as i64;
    state
        .position_ms
        .saturating_add(elapsed_ms)
        .min(state.duration_ms)
}

fn audio_filter(volume: i32, muted: bool, rate: f32) -> String {
    let mut filters = Vec::new();
    let mut tempo = rate.clamp(0.25, 4.0);
    while tempo > 2.0 {
        filters.push("atempo=2.0".to_string());
        tempo /= 2.0;
    }
    while tempo < 0.5 {
        filters.push("atempo=0.5".to_string());
        tempo /= 0.5;
    }
    if (tempo - 1.0).abs() > f32::EPSILON {
        filters.push(format!("atempo={tempo:.4}"));
    }

    let gain = if muted {
        0.0
    } else {
        volume.clamp(0, 200) as f32 / 100.0
    };
    filters.push(format!("volume={gain:.2}"));
    filters.join(",")
}

#[cfg(test)]
mod tests {
    use super::audio_filter;

    #[test]
    fn audio_filter_applies_volume_mute_and_rate() {
        assert_eq!(audio_filter(75, false, 1.0), "volume=0.75");
        assert_eq!(
            audio_filter(200, true, 4.0),
            "atempo=2.0,atempo=2.0000,volume=0.00"
        );
        assert_eq!(
            audio_filter(100, false, 0.25),
            "atempo=0.5,atempo=0.5000,volume=1.00"
        );
    }
}

impl Default for NativePlayer {
    fn default() -> Self {
        Self::new().expect("Failed to create NativePlayer")
    }
}
