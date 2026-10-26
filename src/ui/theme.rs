use crate::ui::state::ThemeMode;
use eframe::egui;

/// Apply the selected theme preference. This is inexpensive and keeps the
/// native window decoration in sync with the chosen egui theme.
pub fn apply(ctx: &egui::Context, mode: ThemeMode) {
    let preference = match mode {
        ThemeMode::System => egui::ThemePreference::System,
        ThemeMode::Dark => egui::ThemePreference::Dark,
        ThemeMode::Light => egui::ThemePreference::Light,
    };
    ctx.set_theme(preference);
}
