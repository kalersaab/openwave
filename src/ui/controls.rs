use crate::ui::icons::{BACK_SVG, MUTE_SVG, NEXT_SVG, PAUSE_SVG, PLAY_SVG, STOP_SVG, UNMUTE_SVG};
use crate::ui::state::{MAX_VOLUME, MIN_VOLUME, UiState};
use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportAction {
    PlayPause,
    Stop,
    Previous,
    Next,
}

pub fn show(ui: &mut egui::Ui, state: &mut UiState) -> Option<TransportAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        let has_media = state.has_media();
        let ctx = ui.ctx().clone();

        // Center transport controls
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.allocate_space(egui::vec2(ui.available_width() * 0.5 - 120.0, 0.0));
            
            // Previous button
            if let Ok(texture) = state.icon_manager.get_texture(&ctx, "back", BACK_SVG, 32) {
                let img = egui::Image::new(&texture).fit_to_exact_size(egui::vec2(24.0, 24.0));
                let button = ui.add_enabled(state.playlist.len() > 1, 
                    egui::Button::image(img).frame(false)
                );
                if button.on_hover_text("Previous track").clicked() {
                    action = Some(TransportAction::Previous);
                }
            }

            ui.add_space(4.0);

            // Play/Pause button (larger)
            let (icon_name, icon_svg) = if state.playback.playing {
                ("pause", PAUSE_SVG)
            } else {
                ("play", PLAY_SVG)
            };
            
            if let Ok(texture) = state.icon_manager.get_texture(&ctx, icon_name, icon_svg, 32) {
                let img = egui::Image::new(&texture).fit_to_exact_size(egui::vec2(32.0, 32.0));
                let button = ui.add_enabled(has_media, 
                    egui::Button::image(img).frame(false)
                );
                if button.on_hover_text("Play / pause (Space)").clicked() {
                    action = Some(TransportAction::PlayPause);
                }
            }

            ui.add_space(4.0);

            // Stop button
            if let Ok(texture) = state.icon_manager.get_texture(&ctx, "stop", STOP_SVG, 32) {
                let img = egui::Image::new(&texture).fit_to_exact_size(egui::vec2(24.0, 24.0));
                let button = ui.add_enabled(has_media, 
                    egui::Button::image(img).frame(false)
                );
                if button.on_hover_text("Stop").clicked() {
                    action = Some(TransportAction::Stop);
                }
            }

            ui.add_space(4.0);

            // Next button
            if let Ok(texture) = state.icon_manager.get_texture(&ctx, "next", NEXT_SVG, 32) {
                let img = egui::Image::new(&texture).fit_to_exact_size(egui::vec2(24.0, 24.0));
                let button = ui.add_enabled(state.playlist.len() > 1, 
                    egui::Button::image(img).frame(false)
                );
                if button.on_hover_text("Next track").clicked() {
                    action = Some(TransportAction::Next);
                }
            }
        });

        // Push volume controls to the right
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Speed control (rightmost)
            egui::ComboBox::from_id_salt("playback_speed")
                .selected_text(format!("{:.2}x", state.playback.speed))
                .width(72.0)
                .show_ui(ui, |ui| {
                    for speed in [0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0] {
                        ui.selectable_value(&mut state.playback.speed, speed, format!("{speed:.2}x"));
                    }
                });
            ui.label("Speed");
            
            ui.separator();

            // Volume slider
            let mut volume = state.playback.volume;
            ui.spacing_mut().slider_width = 120.0;
            if ui
                .add(
                    egui::Slider::new(&mut volume, MIN_VOLUME..=MAX_VOLUME)
                        .suffix("%")
                        .show_value(true),
                )
                .on_hover_text("Volume")
                .changed()
            {
                state.playback.set_volume(volume);
            }

            // Mute button
            let (mute_name, mute_svg) = if state.playback.muted {
                ("mute", MUTE_SVG)
            } else {
                ("unmute", UNMUTE_SVG)
            };
            
            if let Ok(texture) = state.icon_manager.get_texture(&ctx, mute_name, mute_svg, 32) {
                let img = egui::Image::new(&texture).fit_to_exact_size(egui::vec2(20.0, 20.0));
                let button = ui.add(egui::Button::image(img).frame(false));
                if button.on_hover_text("Mute / unmute (M)").clicked() {
                    state.playback.toggle_mute();
                }
            }

            ui.separator();
        });
    });

    action
}
