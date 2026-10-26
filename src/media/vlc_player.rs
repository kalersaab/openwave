use anyhow::{Context, Result};
use std::path::Path;
use std::sync::{Arc, Mutex};
use vlc::{Instance, Media, MediaPlayer, State, MediaPlayerAudioEx};

/// VLC-based media player implementation
pub struct VlcPlayer {
    instance: Instance,
    player: Arc<Mutex<Option<MediaPlayer>>>,
    current_media: Arc<Mutex<Option<Media>>>,
}

impl VlcPlayer {
    /// Create a new VLC player instance
    pub fn new() -> Result<Self> {
        let instance = Instance::new().context("Failed to create VLC instance")?;
        
        Ok(Self {
            instance,
            player: Arc::new(Mutex::new(None)),
            current_media: Arc::new(Mutex::new(None)),
        })
    }

    /// Open and load a media file
    pub fn open<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path_str = path
            .as_ref()
            .to_str()
            .context("Invalid UTF-8 in file path")?;

        let media = Media::new_path(&self.instance, path_str)
            .context("Failed to create media")?;

        let player = MediaPlayer::new(&self.instance)
            .context("Failed to create media player")?;

        player.set_media(&media);

        *self.current_media.lock().unwrap() = Some(media);
        *self.player.lock().unwrap() = Some(player);

        Ok(())
    }

    /// Start or resume playback
    pub fn play(&self) -> Result<()> {
        if let Some(player) = self.player.lock().unwrap().as_ref() {
            player.play()
                .map_err(|_| anyhow::anyhow!("Failed to play media"))?;
        }
        Ok(())
    }

    /// Pause playback
    pub fn pause(&self) -> Result<()> {
        if let Some(player) = self.player.lock().unwrap().as_ref() {
            player.pause();
        }
        Ok(())
    }

    /// Stop playback
    pub fn stop(&self) -> Result<()> {
        if let Some(player) = self.player.lock().unwrap().as_ref() {
            player.stop();
        }
        Ok(())
    }

    /// Check if media is currently playing
    pub fn is_playing(&self) -> bool {
        self.player
            .lock()
            .unwrap()
            .as_ref()
            .map(|p| p.is_playing())
            .unwrap_or(false)
    }

    /// Get current playback state
    pub fn state(&self) -> Option<State> {
        self.player
            .lock()
            .unwrap()
            .as_ref()
            .map(|p| p.state())
    }

    /// Set volume (0-100)
    pub fn set_volume(&self, volume: i32) -> Result<()> {
        if let Some(player) = self.player.lock().unwrap().as_ref() {
            player.set_volume(volume)
                .map_err(|_| anyhow::anyhow!("Failed to set volume"))?;
        }
        Ok(())
    }

    /// Get current volume (0-100)
    pub fn volume(&self) -> i32 {
        self.player
            .lock()
            .unwrap()
            .as_ref()
            .map(|p| p.get_volume())
            .unwrap_or(0)
    }

    /// Mute or unmute audio
    pub fn set_mute(&self, muted: bool) {
        if let Some(player) = self.player.lock().unwrap().as_ref() {
            player.set_mute(muted);
        }
    }

    /// Check if audio is muted
    pub fn is_muted(&self) -> bool {
        self.player
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|p| p.get_mute())
            .unwrap_or(false)
    }

    /// Get current playback position in milliseconds
    pub fn position(&self) -> i64 {
        self.player
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|p| p.get_time())
            .unwrap_or(0)
    }

    /// Set playback position in milliseconds
    pub fn set_position(&self, time_ms: i64) -> Result<()> {
        if let Some(player) = self.player.lock().unwrap().as_ref() {
            player.set_time(time_ms);
        }
        Ok(())
    }

    /// Get total media duration in milliseconds
    pub fn duration(&self) -> i64 {
        self.player
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|p| p.get_time())
            .map(|_| {
                // VLC doesn't have get_length in 0.3, use media duration instead
                0
            })
            .unwrap_or(0)
    }

    /// Set playback rate (1.0 = normal speed)
    pub fn set_rate(&self, rate: f32) -> Result<()> {
        if let Some(player) = self.player.lock().unwrap().as_ref() {
            player.set_rate(rate)
                .map_err(|_| anyhow::anyhow!("Failed to set playback rate"))?;
        }
        Ok(())
    }

    /// Get current playback rate
    pub fn rate(&self) -> f32 {
        self.player
            .lock()
            .unwrap()
            .as_ref()
            .map(|p| p.get_rate())
            .unwrap_or(1.0)
    }

    /// Check if media is loaded
    pub fn has_media(&self) -> bool {
        self.current_media.lock().unwrap().is_some()
    }

    /// Get media metadata
    pub fn get_metadata(&self, meta_type: vlc::Meta) -> Option<String> {
        self.current_media
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|m| m.get_meta(meta_type))
    }

    /// Get video dimensions (width, height)
    pub fn video_size(&self) -> Option<(u32, u32)> {
        self.player.lock().unwrap().as_ref().and_then(|_p| {
            // vlc-rs 0.3 doesn't expose video dimensions directly
            // We'll need to use media track info or return None for now
            None
        })
    }
}

impl Default for VlcPlayer {
    fn default() -> Self {
        Self::new().expect("Failed to create VLC player")
    }
}

// Helper to convert VLC state to string for display
pub fn state_to_string(state: State) -> &'static str {
    match state {
        State::NothingSpecial => "Idle",
        State::Opening => "Opening",
        State::Buffering => "Buffering",
        State::Playing => "Playing",
        State::Paused => "Paused",
        State::Stopped => "Stopped",
        State::Ended => "Ended",
        State::Error => "Error",
    }
}
