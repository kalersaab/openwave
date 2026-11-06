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
    pub(crate) video_texture: Option<egui::TextureHandle>,
}

impl App {
    pub fn new() -> Self {
        Self {
            player: Player::new(),
            error_handler: ErrorHandler::new(),
            ui: UiState::default(),
            video_texture: None,
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
        self.video_texture = None;

        match self.player.open(&path) {
            Ok(format) => {
                if let Some(entry) = self.ui.playlist.get_mut(index) {
                    entry.format = Some(format);
                    entry.error = None;
                }
                self.ui.last_error = None;
                self.ui.playback.position = Duration::ZERO;
                self.ui.playback.duration =
                    Duration::from_millis(self.player.duration().max(0) as u64);
                self.apply_audio_settings();
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

    pub fn set_volume(&mut self, volume: f32) {
        self.ui.playback.set_volume(volume);
        self.apply_audio_settings();
    }

    pub fn change_volume(&mut self, delta: f32) {
        self.ui.playback.change_volume(delta);
        self.apply_audio_settings();
    }

    pub fn toggle_mute(&mut self) {
        self.ui.playback.toggle_mute();
        self.apply_audio_settings();
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.ui.playback.speed = speed.clamp(0.25, 4.0);
        self.apply_audio_settings();
    }

    fn apply_audio_settings(&mut self) -> bool {
        match self.player.set_audio_settings(
            self.ui.playback.engine_volume(),
            self.ui.playback.muted,
            self.ui.playback.speed,
        ) {
            Ok(()) => true,
            Err(error) => {
                self.ui.last_error = Some(error.to_string());
                self.ui.status = error.to_string();
                self.error_handler.report(&error);
                false
            }
        }
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
            if !self.apply_audio_settings() {
                return;
            }

            if let Err(error) = self.player.play() {
                self.ui.playback.playing = false;
                self.ui.last_error = Some(error.to_string());
                self.ui.status = error.to_string();
                self.error_handler.report(&error);
                return;
            }

            self.ui.playback.playing = true;
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
        if let Some(error) = self.player.take_missing_codec_error() {
            self.ui.playback.playing = false;
            self.ui.last_error = Some(error.to_string());
            self.ui.status = "Video playback failed because a codec is unavailable".to_string();
            self.error_handler.report(&error);
        }
        self.update_video_texture(ui.ctx());
        self.update_playback_timer();
        if self.ui.playback.playing {
            if let Err(error) = self.player.sync_audio_settings() {
                self.ui.last_error = Some(error.to_string());
                self.ui.status = error.to_string();
                self.error_handler.report(&error);
            }
            ui.ctx().request_repaint_after(Duration::from_millis(16));
        }
        window::show(self, ui, frame);
    }
}

impl App {
    fn update_playback_timer(&mut self) {
        if !self.ui.has_media() {
            return;
        }

        let duration_ms = self.player.duration();
        if duration_ms > 0 {
            self.ui.playback.duration = Duration::from_millis(duration_ms as u64);
        }

        let position_ms = self.player.position();
        if position_ms >= 0 {
            self.ui.playback.position = Duration::from_millis(position_ms as u64);
        }
    }

    fn update_video_texture(&mut self, ctx: &egui::Context) {
        if !self.ui.current_format().is_some_and(MediaFormat::is_video) {
            self.video_texture = None;
            return;
        }

        let Some(frame) = self.player.take_decoded_frame() else {
            return;
        };

        let width = frame.width as usize;
        let height = frame.height as usize;

        let mut rgba = Vec::with_capacity(width * height * 4);
        let row_bytes = frame.pitch as usize;
        if row_bytes < width * 3 || frame.pixels.len() < row_bytes.saturating_mul(height) {
            return;
        }
        for row in frame.pixels.chunks_exact(row_bytes).take(height) {
            for pixel in row.chunks_exact(3).take(width) {
                rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
            }
        }

        let image = egui::ColorImage::from_rgba_unmultiplied([width, height], &rgba);
        if let Some(texture) = &mut self.video_texture
            && texture.size() == [width, height]
        {
            texture.set(image, egui::TextureOptions::LINEAR);
        } else {
            self.video_texture =
                Some(ctx.load_texture("video_frame", image, egui::TextureOptions::LINEAR));
        }
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
