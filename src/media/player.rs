use crate::error::Result;
use crate::media::decoder::{FormatDetector, MediaFormat};
use crate::media::vlc_player::VlcPlayer;
use std::path::Path;

pub struct Player {
    detector: FormatDetector,
    media: Option<MediaFormat>,
    vlc_player: VlcPlayer,
}

impl Player {
    pub fn new() -> Self {
        Self {
            detector: FormatDetector::new(),
            media: None,
            vlc_player: VlcPlayer::new().expect("Failed to initialize VLC"),
        }
    }

    /// Probe the media file and make it the current item.
    pub fn open(&mut self, path: impl AsRef<Path>) -> Result<MediaFormat> {
        let format = self.detector.detect(&path)?;
        
        // Open with VLC player
        self.vlc_player
            .open(&path)
            .map_err(|e| crate::error::OpenWaveError::Io {
                path: Some(path.as_ref().to_path_buf()),
                details: format!("VLC open failed: {}", e),
            })?;
        
        self.media = Some(format.clone());
        Ok(format)
    }

    pub fn media_format(&self) -> Option<&MediaFormat> {
        self.media.as_ref()
    }

    pub fn detector(&self) -> &FormatDetector {
        &self.detector
    }

    pub fn play(&mut self) {
        let _ = self.vlc_player.play();
    }

    pub fn pause(&mut self) {
        let _ = self.vlc_player.pause();
    }

    pub fn stop(&mut self) {
        let _ = self.vlc_player.stop();
    }

    pub fn is_playing(&self) -> bool {
        self.vlc_player.is_playing()
    }

    pub fn set_volume(&mut self, volume: i32) {
        let _ = self.vlc_player.set_volume(volume);
    }

    pub fn volume(&self) -> i32 {
        self.vlc_player.volume()
    }

    pub fn set_mute(&mut self, muted: bool) {
        self.vlc_player.set_mute(muted);
    }

    pub fn is_muted(&self) -> bool {
        self.vlc_player.is_muted()
    }

    pub fn position(&self) -> i64 {
        self.vlc_player.position()
    }

    pub fn set_position(&mut self, time_ms: i64) {
        let _ = self.vlc_player.set_position(time_ms);
    }

    pub fn duration(&self) -> i64 {
        self.vlc_player.duration()
    }

    pub fn set_rate(&mut self, rate: f32) {
        let _ = self.vlc_player.set_rate(rate);
    }

    pub fn rate(&self) -> f32 {
        self.vlc_player.rate()
    }

    pub fn has_media(&self) -> bool {
        self.vlc_player.has_media()
    }

    pub fn vlc_player(&self) -> &VlcPlayer {
        &self.vlc_player
    }

    pub fn vlc_player_mut(&mut self) -> &mut VlcPlayer {
        &mut self.vlc_player
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}
