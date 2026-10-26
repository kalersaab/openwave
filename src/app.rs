use crate::error::{ErrorHandler, OpenWaveError, Result};
use crate::media::decoder::MediaFormat;
use crate::media::player::Player;
use crate::ui::state::UiState;
use crate::ui::{theme, window};
use eframe::egui;
use std::path::{Path, PathBuf};
use std::time::Duration;

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "mkv", "webm", "avi", "flv", "ogg", "ogv", "mpeg", "mpg", "asf", "mp3", "flac",
    "wav", "m4a", "aac", "opus",
];

pub struct App {
    pub(crate) player: Player,
    pub(crate) error_handler: ErrorHandler,
    pub(crate) ui: UiState,
}

impl App {
    pub fn new() -> Self {
        Self {
            player: Player::new(),
            error_handler: ErrorHandler::new(),
            ui: UiState::default(),
        }
    }

    pub fn run(self) -> eframe::Result {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("OpenWave")
                .with_inner_size([1100.0, 720.0])
                .with_min_inner_size([760.0, 480.0]),
            ..Default::default()
        };

        eframe::run_native(
            "OpenWave",
            options,
            Box::new(move |cc| {
                theme::apply(&cc.egui_ctx, self.ui.settings.theme);
                Ok(Box::new(self))
            }),
        )
    }

    /// Open a media file, reporting detection failures through the error
    /// handler so the UI layer can surface an unsupported-format message.
    pub fn open_media(&mut self, path: impl AsRef<Path>) -> Result<MediaFormat> {
        match self.player.open(path) {
            Ok(format) => Ok(format),
            Err(error) => {
                self.error_handler.report(&error);
                Err(error)
            }
        }
    }

    pub fn add_files(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        let detector = self.player.detector().clone();
        let added = self.ui.add_paths(paths, &detector);

        if let Some(&index) = added.first() {
            self.open_index(index);
        } else if !self.ui.playlist.is_empty() {
            self.ui.status = "Those files are already in the playlist".to_string();
        }
    }

    pub fn open_file_dialog(&mut self) {
        let files = rfd::FileDialog::new()
            .set_title("Open media files")
            .add_filter("Media files", SUPPORTED_EXTENSIONS)
            .add_filter("All files", &["*"])
            .pick_files();

        if let Some(files) = files {
            self.add_files(files);
        }
    }

    pub fn open_index(&mut self, index: usize) {
        let Some(path) = self.ui.select_index(index) else {
            return;
        };

        match self.player.open(&path) {
            Ok(format) => {
                if let Some(entry) = self.ui.playlist.get_mut(index) {
                    entry.format = Some(format);
                    entry.error = None;
                }
                self.ui.last_error = None;
                self.ui.playback.position = Duration::ZERO;
            }
            Err(error) => {
                self.ui.playback.playing = false;
                self.ui.last_error = Some(error.to_string());
                self.ui.status = format!("Could not open {}", path.display());
                self.error_handler.report(&error);
            }
        }
    }

    pub fn toggle_play_pause(&mut self) {
        if !self.ui.has_media() {
            self.ui.status = "Open a media file first".to_string();
            return;
        }

        if self.ui.playback.playing {
            self.ui.playback.playing = false;
            self.player.pause();
            self.ui.status = "Paused".to_string();
        } else {
            self.start_playback();
        }
    }

    pub fn stop(&mut self) {
        self.player.stop();
        self.ui.playback.playing = false;
        self.ui.playback.position = Duration::ZERO;
        if self.ui.has_media() {
            self.ui.status = "Stopped".to_string();
        }
    }

    pub fn seek_to(&mut self, seconds: f64) {
        let time_ms = (seconds * 1000.0) as i64;
        self.player.set_position(time_ms);
        self.ui.playback.seek_to(seconds);
    }

    pub fn seek_relative(&mut self, seconds: f64) {
        let current_ms = self.player.position();
        let new_ms = (current_ms as f64 + seconds * 1000.0).max(0.0) as i64;
        self.player.set_position(new_ms);
        self.ui.playback.seek_relative(seconds);
    }

    pub fn change_volume(&mut self, delta: f32) {
        self.ui.playback.change_volume(delta);
        // Convert from 0-200% to VLC's 0-100
        let vlc_volume = (self.ui.playback.volume as f32 / 2.0) as i32;
        self.player.set_volume(vlc_volume);
    }

    pub fn toggle_mute(&mut self) {
        self.ui.playback.toggle_mute();
        self.player.set_mute(self.ui.playback.muted);
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.ui.playback.speed = speed.clamp(0.25, 4.0);
        self.player.set_rate(speed);
    }

    pub fn next_track(&mut self) {
        let was_playing = self.ui.playback.playing;
        let Some(index) = self.ui.next_index() else {
            return;
        };
        self.open_index(index);
        if was_playing && self.ui.last_error.is_none() {
            self.start_playback();
        }
    }

    pub fn previous_track(&mut self) {
        let was_playing = self.ui.playback.playing;
        let Some(index) = self.ui.previous_index() else {
            return;
        };
        self.open_index(index);
        if was_playing && self.ui.last_error.is_none() {
            self.start_playback();
        }
    }

    pub fn remove_index(&mut self, index: usize) {
        let removing_current = self.ui.current == Some(index);
        let was_playing = self.ui.playback.playing;
        self.ui.remove_at(index);

        if removing_current {
            if let Some(current) = self.ui.current {
                self.open_index(current);
                if was_playing && self.ui.last_error.is_none() {
                    self.start_playback();
                }
            } else {
                self.player.stop();
            }
        }

        self.ui.status = "Removed from playlist".to_string();
    }

    pub fn clear_playlist(&mut self) {
        self.player.stop();
        self.ui.clear();
    }

    fn start_playback(&mut self) {
        if self.ui.has_media() {
            self.ui.playback.playing = true;
            self.player.play();
            self.ui.status = self
                .ui
                .current_entry()
                .map(|entry| format!("Playing {}", entry.display_name()))
                .unwrap_or_else(|| "Playing".to_string());
            self.ui.last_error = None;
        }
    }

    pub fn error_handler(&self) -> &ErrorHandler {
        &self.error_handler
    }

    pub fn last_error(&self) -> Option<&OpenWaveError> {
        self.error_handler.last_reported()
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        window::show(self, ui, frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("openwave-app-{name}-{nonce}"))
    }

    #[test]
    fn opening_unsupported_file_reports_error() {
        let path = temp_path("unsupported.bin");
        fs::write(&path, b"not a media file").unwrap();

        let mut app = App::new();
        let result = app.open_media(&path);
        fs::remove_file(&path).unwrap();

        assert!(result.is_err());
        let error = app.last_error().unwrap();
        assert!(error.is_unsupported_format());
        assert_eq!(app.error_handler().reported().len(), 1);
    }
}
