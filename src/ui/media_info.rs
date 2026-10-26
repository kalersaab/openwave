use crate::media::decoder::MediaKind;
use crate::ui::state::{UiState, format_file_size, format_time};
use eframe::egui;

pub fn show(ctx: &egui::Context, state: &mut UiState) {
    if !state.show_media_info {
        return;
    }

    let entry = state.current_entry().cloned();
    let mut open = state.show_media_info;

    egui::Window::new("Media Information")
        .open(&mut open)
        .default_width(520.0)
        .resizable(true)
        .show(ctx, |ui| {
            let Some(entry) = &entry else {
                ui.label("No media file is open.");
                return;
            };

            ui.heading(entry.display_name());
            ui.label(egui::RichText::new(entry.path.display().to_string()).weak());
            ui.separator();

            egui::Grid::new("media_info_grid")
                .num_columns(2)
                .striped(true)
                .spacing([18.0, 6.0])
                .show(ui, |ui| {
                    row(ui, "Title", &entry.display_name());
                    row(ui, "Artist", "Not available");
                    row(ui, "Album", "Not available");
                    row(ui, "Year", "Not available");
                    row(ui, "Genre", "Not available");
                    row(ui, "Duration", &format_time(state.playback.duration_secs()));

                    let Some(format) = &entry.format else {
                        row(ui, "Format", "Unsupported or unreadable");
                        if let Some(error) = &entry.error {
                            row(ui, "Error", error);
                        }
                        return;
                    };

                    row(
                        ui,
                        "Container",
                        format
                            .container
                            .map_or("Audio", |container| container.as_str()),
                    );
                    row(ui, "Kind", kind_label(format.kind));
                    if let Some(audio_format) = format.audio_format {
                        row(ui, "Audio format", audio_format.as_str());
                    }
                    row(ui, "File size", &format_file_size(entry.file_size));

                    let video_codecs: Vec<String> = format
                        .video_codecs
                        .iter()
                        .map(|codec| codec.to_string())
                        .collect();
                    let audio_codecs: Vec<String> = format
                        .audio_codecs
                        .iter()
                        .map(|codec| codec.to_string())
                        .collect();

                    let video_codec_text = if video_codecs.is_empty() {
                        "None".to_string()
                    } else {
                        video_codecs.join(", ")
                    };
                    let audio_codec_text = if audio_codecs.is_empty() {
                        "None".to_string()
                    } else {
                        audio_codecs.join(", ")
                    };
                    row(ui, "Video codec", &video_codec_text);
                    row(ui, "Audio codec", &audio_codec_text);
                    row(ui, "Video tracks", &format.video_codecs.len().to_string());
                    row(ui, "Audio tracks", &format.audio_codecs.len().to_string());
                    row(ui, "Resolution", "Not available");
                    row(ui, "Frame rate", "Not available");
                    row(ui, "Bitrate", "Not available");
                    row(ui, "Sample rate", "Not available");
                    row(ui, "Subtitles", "None detected");
                });
        });

    state.show_media_info = open;
}

fn row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(egui::RichText::new(label).strong());
    ui.label(value);
    ui.end_row();
}

fn kind_label(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Audio => "Audio",
        MediaKind::Video => "Video",
        MediaKind::AudioVideo => "Audio + video",
        MediaKind::Unknown => "Unknown",
    }
}
