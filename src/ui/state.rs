use crate::media::decoder::{FormatDetector, MediaFormat};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const MIN_VOLUME: f32 = 0.0;
pub const MAX_VOLUME: f32 = 200.0;
pub const VOLUME_STEP: f32 = 5.0;
pub const SEEK_STEP: f64 = 5.0;

pub const EQUALIZER_BANDS: [&str; 10] = [
    "31 Hz", "62 Hz", "125 Hz", "250 Hz", "500 Hz", "1 kHz", "2 kHz", "4 kHz", "8 kHz", "16 kHz",
];

#[derive(Debug, Clone)]
pub struct PlaylistEntry {
    pub path: PathBuf,
    pub format: Option<MediaFormat>,
    pub error: Option<String>,
    pub file_size: Option<u64>,
}

impl PlaylistEntry {
    pub fn from_detection(path: PathBuf, format: MediaFormat) -> Self {
        let file_size = std::fs::metadata(&path).ok().map(|metadata| metadata.len());
        Self {
            path,
            format: Some(format),
            error: None,
            file_size,
        }
    }

    pub fn from_error(path: PathBuf, error: String) -> Self {
        let file_size = std::fs::metadata(&path).ok().map(|metadata| metadata.len());
        Self {
            path,
            format: None,
            error: Some(error),
            file_size,
        }
    }

    pub fn display_name(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.path.display().to_string())
    }

    pub fn is_audio_only(&self) -> bool {
        self.format.as_ref().is_some_and(|format| format.is_audio())
    }

    pub fn format_summary(&self) -> String {
        match &self.format {
            Some(format) => {
                let container = format
                    .container
                    .map(|container| container.as_str().to_string())
                    .or_else(|| format.audio_format.map(|audio| audio.as_str().to_string()))
                    .unwrap_or_else(|| "Media".to_string());

                let codecs: Vec<String> = format
                    .required_codecs()
                    .iter()
                    .map(|codec| codec.to_string())
                    .collect();

                if codecs.is_empty() {
                    container
                } else {
                    format!("{container} · {}", codecs.join(" · "))
                }
            }
            None => "Unsupported".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    System,
    Dark,
    Light,
}

impl ThemeMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizationStyle {
    AlbumArt,
    Waveform,
    Spectrum,
}

impl VisualizationStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::AlbumArt => "Album art",
            Self::Waveform => "Waveform",
            Self::Spectrum => "Spectrum",
        }
    }
}

#[derive(Debug, Clone)]
pub struct UiSettings {
    pub show_playlist: bool,
    pub theme: ThemeMode,
    pub hardware_acceleration: bool,
    pub visualizations_enabled: bool,
    pub visualization_style: VisualizationStyle,
    pub screenshot_directory: String,
    pub resume_playback: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            show_playlist: true,
            theme: ThemeMode::System,
            hardware_acceleration: true,
            visualizations_enabled: true,
            visualization_style: VisualizationStyle::Spectrum,
            screenshot_directory: "screenshots".to_string(),
            resume_playback: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub playing: bool,
    pub position: Duration,
    pub duration: Duration,
    pub volume: f32,
    pub muted: bool,
    pub speed: f32,
    pub buffering: Option<u8>,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            playing: false,
            position: Duration::ZERO,
            duration: Duration::ZERO,
            volume: 100.0,
            muted: false,
            speed: 1.0,
            buffering: None,
        }
    }
}

impl PlaybackState {
    pub fn position_secs(&self) -> f64 {
        self.position.as_secs_f64()
    }

    pub fn duration_secs(&self) -> f64 {
        self.duration.as_secs_f64()
    }

    pub fn effective_volume(&self) -> f32 {
        if self.muted { 0.0 } else { self.volume }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(MIN_VOLUME, MAX_VOLUME);
        if self.volume > 0.0 {
            self.muted = false;
        }
    }

    pub fn change_volume(&mut self, delta: f32) {
        self.set_volume(self.volume + delta);
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }

    pub fn seek_to(&mut self, seconds: f64) {
        let max = self.duration_secs();
        let target = if max > 0.0 {
            seconds.clamp(0.0, max)
        } else {
            0.0
        };
        self.position = Duration::from_secs_f64(target);
    }

    pub fn seek_relative(&mut self, delta: f64) {
        self.seek_to(self.position_secs() + delta);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EqualizerPreset {
    Flat,
    Rock,
    Jazz,
    Classical,
    Speech,
    Custom,
}

impl EqualizerPreset {
    pub fn label(self) -> &'static str {
        match self {
            Self::Flat => "Flat",
            Self::Rock => "Rock",
            Self::Jazz => "Jazz",
            Self::Classical => "Classical",
            Self::Speech => "Speech",
            Self::Custom => "Custom",
        }
    }

    pub fn gains(self) -> Option<[f32; 10]> {
        match self {
            Self::Flat => Some([0.0; 10]),
            Self::Rock => Some([5.0, 4.0, 2.0, 0.0, -1.0, -1.0, 1.0, 3.0, 4.0, 5.0]),
            Self::Jazz => Some([4.0, 3.0, 1.0, 0.0, -1.0, -1.0, 0.0, 1.0, 2.0, 3.0]),
            Self::Classical => Some([3.0, 2.0, 1.0, 0.0, 0.0, 0.0, -1.0, -2.0, -2.0, -3.0]),
            Self::Speech => Some([-3.0, -2.0, -1.0, 0.0, 2.0, 4.0, 4.0, 3.0, 1.0, 0.0]),
            Self::Custom => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EqualizerState {
    pub enabled: bool,
    pub preset: EqualizerPreset,
    pub gains: [f32; 10],
}

impl Default for EqualizerState {
    fn default() -> Self {
        Self {
            enabled: false,
            preset: EqualizerPreset::Flat,
            gains: [0.0; 10],
        }
    }
}

impl EqualizerState {
    pub fn apply_preset(&mut self, preset: EqualizerPreset) {
        self.preset = preset;
        if let Some(gains) = preset.gains() {
            self.gains = gains;
        }
    }

    pub fn set_gain(&mut self, index: usize, gain: f32) {
        if let Some(slot) = self.gains.get_mut(index) {
            *slot = gain.clamp(-12.0, 12.0);
            self.preset = EqualizerPreset::Custom;
        }
    }
}

#[derive(Debug)]
pub struct UiState {
    pub playlist: Vec<PlaylistEntry>,
    pub current: Option<usize>,
    pub playback: PlaybackState,
    pub settings: UiSettings,
    pub equalizer: EqualizerState,
    pub icon_manager: crate::ui::IconManager,
    pub show_settings: bool,
    pub show_media_info: bool,
    pub show_equalizer: bool,
    pub show_shortcuts: bool,
    pub show_about: bool,
    pub fullscreen: bool,
    pub drag_active: bool,
    pub zoom: f32,
    pub fit_to_window: bool,
    pub status: String,
    pub last_error: Option<String>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            playlist: Vec::new(),
            current: None,
            playback: PlaybackState::default(),
            settings: UiSettings::default(),
            equalizer: EqualizerState::default(),
            icon_manager: crate::ui::IconManager::new(),
            show_settings: false,
            show_media_info: false,
            show_equalizer: false,
            show_shortcuts: false,
            show_about: false,
            fullscreen: false,
            drag_active: false,
            zoom: 1.0,
            fit_to_window: true,
            status: "Drop media files here or use File > Open".to_string(),
            last_error: None,
        }
    }
}

impl UiState {
    pub fn current_entry(&self) -> Option<&PlaylistEntry> {
        self.current.and_then(|index| self.playlist.get(index))
    }

    pub fn current_path(&self) -> Option<PathBuf> {
        self.current_entry().map(|entry| entry.path.clone())
    }

    pub fn current_format(&self) -> Option<&MediaFormat> {
        self.current_entry().and_then(|entry| entry.format.as_ref())
    }

    pub fn has_media(&self) -> bool {
        self.current_entry().is_some()
    }

    pub fn add_paths(
        &mut self,
        paths: impl IntoIterator<Item = PathBuf>,
        detector: &FormatDetector,
    ) -> Vec<usize> {
        let mut paths: Vec<PathBuf> = paths
            .into_iter()
            .filter(|path| !self.playlist.iter().any(|entry| entry.path == *path))
            .collect();

        paths.sort_by(|left, right| {
            file_name_for_sort(left)
                .cmp(&file_name_for_sort(right))
                .then_with(|| left.cmp(right))
        });

        let first_new = self.playlist.len();
        for path in paths {
            let entry = match detector.detect(&path) {
                Ok(format) => PlaylistEntry::from_detection(path, format),
                Err(error) => PlaylistEntry::from_error(path, error.to_string()),
            };
            self.playlist.push(entry);
        }

        let added: Vec<usize> = (first_new..self.playlist.len()).collect();
        if self.current.is_none() && !added.is_empty() {
            self.select_index(added[0]);
        }

        if !added.is_empty() {
            self.status = format!(
                "Added {} file{} to the playlist",
                added.len(),
                if added.len() == 1 { "" } else { "s" }
            );
        }

        added
    }

    pub fn select_index(&mut self, index: usize) -> Option<PathBuf> {
        if index >= self.playlist.len() {
            return None;
        }

        self.current = Some(index);
        self.playback.playing = false;
        self.playback.position = Duration::ZERO;
        self.status = format!("Opened {}", self.playlist[index].display_name());
        Some(self.playlist[index].path.clone())
    }

    pub fn remove_at(&mut self, index: usize) {
        if index >= self.playlist.len() {
            return;
        }

        self.playlist.remove(index);
        match self.current {
            Some(_) if self.playlist.is_empty() => {
                self.current = None;
                self.playback = PlaybackState {
                    volume: self.playback.volume,
                    muted: self.playback.muted,
                    speed: self.playback.speed,
                    ..PlaybackState::default()
                };
            }
            Some(current) if current == index => {
                let next = index.min(self.playlist.len() - 1);
                self.current = Some(next);
                self.playback.position = Duration::ZERO;
                self.playback.playing = false;
            }
            Some(current) if current > index => {
                self.current = Some(current - 1);
            }
            _ => {}
        }
    }

    pub fn clear(&mut self) {
        self.playlist.clear();
        self.current = None;
        self.playback.position = Duration::ZERO;
        self.playback.playing = false;
        self.status = "Playlist cleared".to_string();
    }

    pub fn next_index(&self) -> Option<usize> {
        if self.playlist.is_empty() {
            None
        } else {
            Some((self.current.unwrap_or(0) + 1) % self.playlist.len())
        }
    }

    pub fn previous_index(&self) -> Option<usize> {
        if self.playlist.is_empty() {
            None
        } else {
            let current = self.current.unwrap_or(0);
            Some((current + self.playlist.len() - 1) % self.playlist.len())
        }
    }
}

fn file_name_for_sort(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

pub fn format_time(seconds: f64) -> String {
    let total = if seconds.is_finite() && seconds > 0.0 {
        seconds.floor() as u64
    } else {
        0
    };
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    format!("{hours}:{minutes:02}:{seconds:02}")
}

pub fn format_file_size(bytes: Option<u64>) -> String {
    let Some(bytes) = bytes else {
        return "Unknown".to_string();
    };

    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let bytes = bytes as f64;

    if bytes >= GB {
        format!("{:.2} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{bytes:.0} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_playback_time_as_hms() {
        assert_eq!(format_time(0.0), "0:00:00");
        assert_eq!(format_time(65.0), "0:01:05");
        assert_eq!(format_time(3_661.0), "1:01:01");
    }

    #[test]
    fn mute_preserves_volume_and_unmutes_on_change() {
        let mut playback = PlaybackState::default();
        playback.set_volume(80.0);
        playback.toggle_mute();
        assert!(playback.muted);
        assert_eq!(playback.volume, 80.0);
        assert_eq!(playback.effective_volume(), 0.0);

        playback.change_volume(5.0);
        assert!(!playback.muted);
        assert_eq!(playback.volume, 85.0);
    }

    #[test]
    fn volume_is_clamped_to_supported_range() {
        let mut playback = PlaybackState::default();
        playback.set_volume(500.0);
        assert_eq!(playback.volume, MAX_VOLUME);
        playback.set_volume(-10.0);
        assert_eq!(playback.volume, MIN_VOLUME);
    }

    #[test]
    fn equalizer_presets_and_custom_edits() {
        let mut equalizer = EqualizerState::default();
        equalizer.apply_preset(EqualizerPreset::Rock);
        assert_eq!(equalizer.preset, EqualizerPreset::Rock);
        assert!(equalizer.gains.iter().any(|gain| *gain != 0.0));

        equalizer.set_gain(0, 20.0);
        assert_eq!(equalizer.preset, EqualizerPreset::Custom);
        assert_eq!(equalizer.gains[0], 12.0);
    }

    #[test]
    fn playlist_navigation_wraps() {
        let mut state = UiState {
            playlist: vec![
                PlaylistEntry::from_error(PathBuf::from("a.mp3"), "test".into()),
                PlaylistEntry::from_error(PathBuf::from("b.mp3"), "test".into()),
            ],
            current: Some(0),
            ..UiState::default()
        };
        assert_eq!(state.next_index(), Some(1));
        state.current = Some(1);
        assert_eq!(state.next_index(), Some(0));
        assert_eq!(state.previous_index(), Some(0));
    }

    #[test]
    fn file_size_is_human_readable() {
        assert_eq!(format_file_size(None), "Unknown");
        assert_eq!(format_file_size(Some(512)), "512 B");
        assert_eq!(format_file_size(Some(1_048_576)), "1.0 MB");
    }
}
