use crate::ui::state::{EQUALIZER_BANDS, EqualizerPreset, UiState};
use eframe::egui;

pub fn show(ctx: &egui::Context, state: &mut UiState) {
    if !state.show_equalizer {
        return;
    }

    let mut open = state.show_equalizer;
    egui::Window::new("Equalizer")
        .open(&mut open)
        .default_width(720.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut state.equalizer.enabled, "Enable equalizer");
                ui.separator();
                ui.label("Preset");

                let mut preset = state.equalizer.preset;
                egui::ComboBox::from_id_salt("equalizer_preset")
                    .selected_text(preset.label())
                    .show_ui(ui, |ui| {
                        for option in [
                            EqualizerPreset::Flat,
                            EqualizerPreset::Rock,
                            EqualizerPreset::Jazz,
                            EqualizerPreset::Classical,
                            EqualizerPreset::Speech,
                        ] {
                            ui.selectable_value(&mut preset, option, option.label());
                        }
                    });

                if preset != state.equalizer.preset {
                    state.equalizer.apply_preset(preset);
                }

                if ui.button("Reset").clicked() {
                    state.equalizer.apply_preset(EqualizerPreset::Flat);
                }
            });

            ui.separator();
            ui.label(egui::RichText::new("-12 dB to +12 dB").weak());

            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (index, band) in EQUALIZER_BANDS.iter().enumerate() {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(*band).small());
                            let mut gain = state.equalizer.gains[index];
                            let response = ui.add(
                                egui::Slider::new(&mut gain, -12.0..=12.0)
                                    .vertical()
                                    .show_value(false),
                            );
                            if response.changed() {
                                state.equalizer.set_gain(index, gain);
                            }
                            ui.label(format!("{:.1}", state.equalizer.gains[index]));
                        });
                    }
                });
            });
        });

    state.show_equalizer = open;
}
