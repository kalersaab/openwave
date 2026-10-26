use crate::ui::state::UiState;
use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaylistAction {
    Select(usize),
    Remove(usize),
    Clear,
    OpenFiles,
}

pub fn show(ui: &mut egui::Ui, state: &UiState) -> Option<PlaylistAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        ui.heading("Playlist");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(!state.playlist.is_empty(), egui::Button::new("Clear"))
                .clicked()
            {
                action = Some(PlaylistAction::Clear);
            }
            if ui.button("Add files").clicked() {
                action = Some(PlaylistAction::OpenFiles);
            }
        });
    });

    ui.label(
        egui::RichText::new(format!(
            "{} item{}",
            state.playlist.len(),
            if state.playlist.len() == 1 { "" } else { "s" }
        ))
        .weak(),
    );
    ui.separator();

    if state.playlist.is_empty() {
        ui.add_space(12.0);
        ui.label("The playlist is empty.");
        ui.label(egui::RichText::new("Drop files here or use Add files.").weak());
        return action;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (index, entry) in state.playlist.iter().enumerate() {
                let selected = state.current == Some(index);
                ui.horizontal(|ui| {
                    let label = format!("{:02}  {}", index + 1, entry.display_name());
                    let response = ui
                        .selectable_label(selected, egui::RichText::new(label).strong())
                        .on_hover_text(entry.path.display().to_string());
                    if response.clicked() {
                        action = Some(PlaylistAction::Select(index));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button("✕")
                            .on_hover_text("Remove from playlist")
                            .clicked()
                        {
                            action = Some(PlaylistAction::Remove(index));
                        }
                        if selected {
                            ui.label(egui::RichText::new("Playing").small().strong());
                        }
                    });
                });

                ui.label(egui::RichText::new(entry.format_summary()).small().weak());
                if let Some(error) = &entry.error {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }
                ui.separator();
            }
        });

    action
}
