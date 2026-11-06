use crate::error::{OpenWaveError, Result};
use crate::media::decoder::{FormatDetector, MediaFormat};
use crate::media::native_player::{DecodedFrame, NativePlayer};
use openwave_core::AudioEngine;
use std::path::{Path, PathBuf};

pub struct Player {
    detector: FormatDetector,
    media: Option<MediaFormat>,
    current_path: Option<PathBuf>,
    native_player: NativePlayer,
    audio_engine: Option<AudioEngine>,
}

impl Player {
    pub fn new() -> Self {
        Self {
            detector: FormatDetector::new(),
            media: None,
            current_path: None,
            native_player: NativePlayer::new().expect("Failed to initialize NativePlayer"),
            audio_engine: None,
        }
    }

    /// Probe the media file and make it the current item.
    pub fn open(&mut self, path: impl AsRef<Path>) -> Result<MediaFormat> {
        let path_ref = path.as_ref();
        let format = self.detector.detect(path_ref)?;
        let ext = path_ref
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "mp3" || ext == "wav" || ext == "wave" {
            match AudioEngine::open(path_ref) {
                Ok(engine) => {
                    self.audio_engine = Some(engine);
                }
                Err(e) => {
                    return Err(crate::error::OpenWaveError::Io {
                        path: Some(path_ref.to_path_buf()),
                        details: format!("Player open failed: {}", e),
                    });
                }
            }
            self.native_player.stop().ok();
            self.current_path = Some(path_ref.to_path_buf());
            self.media = Some(format.clone());
            return Ok(format);
        }

        // Open with native player
        self.native_player
            .open_with_frame_output(path_ref, format.is_video())
            .map_err(|e| crate::error::OpenWaveError::Io {
                path: Some(path_ref.to_path_buf()),
                details: format!("Player open failed: {}", e),
            })?;

        self.audio_engine = None;
        self.current_path = Some(path_ref.to_path_buf());
        self.media = Some(format.clone());
        Ok(format)
    }

    pub fn media_format(&self) -> Option<&MediaFormat> {
        self.media.as_ref()
    }

    pub fn detector(&self) -> &FormatDetector {
        &self.detector
    }

    pub fn play(&mut self) -> Result<()> {
        if let Some(engine) = self.audio_engine.as_ref() {
            engine.play();
            return Ok(());
        }
        self.native_player.play().map_err(|error| {
            OpenWaveError::playback(
                self.current_path.as_deref(),
                format!("Could not start playback: {error}"),
            )
        })
    }

    pub fn sync_audio_settings(&self) -> Result<bool> {
        self.native_player.sync_audio_output().map_err(|error| {
            OpenWaveError::playback(
                self.current_path.as_deref(),
                format!("Video audio output failed: {error}"),
            )
        })
    }

    pub fn pause(&mut self) {
        if let Some(engine) = self.audio_engine.as_ref() {
            engine.pause();
            return;
        }
        let _ = self.native_player.pause();
    }

    pub fn stop(&mut self) {
        if let Some(engine) = self.audio_engine.as_mut() {
            engine.stop();
        }
        let _ = self.native_player.stop();
    }

    pub fn is_playing(&self) -> bool {
        if let Some(engine) = self.audio_engine.as_ref() {
            return engine.is_playing();
        }
        self.native_player.is_playing()
    }

    pub fn set_audio_settings(&mut self, volume: i32, muted: bool, rate: f32) -> Result<()> {
        self.native_player
            .set_audio_settings(volume, muted, rate)
            .map_err(|error| {
                OpenWaveError::playback(
                    self.current_path.as_deref(),
                    format!("Could not update video audio output: {error}"),
                )
            })
    }

    pub fn set_volume(&mut self, volume: i32) -> Result<()> {
        self.native_player.set_volume(volume).map_err(|error| {
            OpenWaveError::playback(
                self.current_path.as_deref(),
                format!("Could not update video volume: {error}"),
            )
        })
    }

    pub fn volume(&self) -> i32 {
        self.native_player.volume()
    }

    pub fn set_mute(&mut self, muted: bool) -> Result<()> {
        self.native_player.set_mute(muted).map_err(|error| {
            OpenWaveError::playback(
                self.current_path.as_deref(),
                format!("Could not update video mute state: {error}"),
            )
        })
    }

    pub fn is_muted(&self) -> bool {
        self.native_player.is_muted()
    }

    pub fn position(&self) -> i64 {
        if let Some(engine) = self.audio_engine.as_ref() {
            return (engine.position_seconds() * 1000.0) as i64;
        }
        self.native_player.position()
    }

    pub fn set_position(&mut self, time_ms: i64) {
        if let Some(engine) = self.audio_engine.as_ref() {
            let _ = engine.seek(time_ms as f64 / 1000.0);
            return;
        }
        let _ = self.native_player.set_position(time_ms);
    }

    pub fn duration(&self) -> i64 {
        if let Some(engine) = self.audio_engine.as_ref() {
            return (engine.duration_seconds() * 1000.0) as i64;
        }
        self.native_player.duration()
    }

    pub fn set_rate(&mut self, rate: f32) -> Result<()> {
        self.native_player.set_rate(rate).map_err(|error| {
            OpenWaveError::playback(
                self.current_path.as_deref(),
                format!("Could not update video playback rate: {error}"),
            )
        })
    }

    pub fn rate(&self) -> f32 {
        self.native_player.rate()
    }

    pub fn has_media(&self) -> bool {
        self.native_player.has_media()
    }

    pub fn enable_decoded_frames(&mut self, capacity: usize) -> anyhow::Result<()> {
        self.native_player.enable_decoded_frames(capacity)?;
        Ok(())
    }

    pub fn take_decoded_frame(&self) -> Option<DecodedFrame> {
        self.native_player.take_decoded_frame()
    }

    pub fn dropped_decoded_frames(&self) -> u64 {
        self.native_player.dropped_decoded_frames()
    }

    /// Native player doesn't have codec detection errors in the same way VLC did
    pub fn take_missing_codec_error(&self) -> Option<OpenWaveError> {
        None
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}
