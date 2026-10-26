use crate::ui::state::{ThemeMode, UiSettings, UiState, VisualizationStyle};
use eframe::egui;

pub fn show(ctx: &egui::Context, state: &mut UiState) {
    if !state.show_settings {
        return;
    }

    let mut open = state.show_settings;
    egui::Window::new("Settings")
        .open(&mut open)
        .default_width(480.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Appearance");
            ui.horizontal(|ui| {
                ui.label("Theme");
                egui::ComboBox::from_id_salt("theme")
                    .selected_text(state.settings.theme.label())
                    .show_ui(ui, |ui| {
                        for theme in [ThemeMode::System, ThemeMode::Dark, ThemeMode::Light] {
                            ui.selectable_value(&mut state.settings.theme, theme, theme.label());
                        }
                    });
            });
            ui.checkbox(&mut state.settings.show_playlist, "Show playlist panel");
            ui.separator();

            ui.heading("Playback");
            ui.checkbox(
                &mut state.settings.hardware_acceleration,
                "Use hardware acceleration when available",
            );
            ui.checkbox(
                &mut state.settings.resume_playback,
                "Resume playback from the last position",
            );
            ui.checkbox(
                &mut state.settings.visualizations_enabled,
                "Show audio visualizations",
            );
            ui.horizontal(|ui| {
                ui.label("Visualization style");
                egui::ComboBox::from_id_salt("visualization_style")
                    .selected_text(state.settings.visualization_style.label())
                    .show_ui(ui, |ui| {
                        for style in [
                            VisualizationStyle::AlbumArt,
                            VisualizationStyle::Waveform,
                            VisualizationStyle::Spectrum,
                        ] {
                            ui.selectable_value(
                                &mut state.settings.visualization_style,
                                style,
                                style.label(),
                            );
                        }
                    });
            });
            ui.separator();

            ui.heading("Screenshots");
            ui.horizontal(|ui| {
                ui.label("Directory");
                ui.text_edit_singleline(&mut state.settings.screenshot_directory);
            });
            ui.separator();

            ui.heading("Tools");
            ui.horizontal(|ui| {
                if ui.button("Equalizer").clicked() {
                    state.show_equalizer = true;
                }
                if ui.button("Keyboard shortcuts").clicked() {
                    state.show_shortcuts = true;
                }
            });
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Restore defaults").clicked() {
                    state.settings = UiSettings::default();
                }
                if ui.button("Close").clicked() {
                    state.show_settings = false;
                }
            });
        });

    state.show_settings = open;
}
