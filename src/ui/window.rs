use crate::app::App;
use crate::ui::controls::{self, TransportAction};
use crate::ui::equalizer_ui;
use crate::ui::keyboard::{self, SHORTCUTS, ShortcutAction};
use crate::ui::media_info;
use crate::ui::playlist_panel::{self, PlaylistAction};
use crate::ui::seekbar;
use crate::ui::settings;
use crate::ui::state::{UiState, format_file_size};
use crate::ui::theme;
use eframe::egui;

pub fn show(app: &mut App, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    let ctx = ui.ctx().clone();
    theme::apply(&ctx, app.ui.settings.theme);
    handle_dropped_files(app, &ctx);

    if let Some(action) = keyboard::handle(&ctx) {
        apply_shortcut(app, &ctx, action);
    }

    show_menu_bar(app, ui);

    egui::Panel::bottom("transport_panel")
        .resizable(false)
        .show(ui, |ui| {
            show_transport(app, ui);
        });

    if app.ui.settings.show_playlist {
        egui::Panel::right("playlist_panel")
            .default_size(310.0)
            .resizable(true)
            .show(ui, |ui| {
                if let Some(action) = playlist_panel::show(ui, &app.ui) {
                    match action {
                        PlaylistAction::Select(index) => app.open_index(index),
                        PlaylistAction::Remove(index) => app.remove_index(index),
                        PlaylistAction::Clear => app.clear_playlist(),
                        PlaylistAction::OpenFiles => app.open_file_dialog(),
                    }
                }
            });
    }

    egui::CentralPanel::default().show(ui, |ui| {
        if app.ui.has_media() {
            paint_preview(ui, app);
        } else {
            paint_welcome(ui, app);
        }

        if app.ui.drag_active {
            paint_drop_overlay(ui);
        }
    });

    media_info::show(&ctx, &mut app.ui);
    settings::show(&ctx, &mut app.ui);
    equalizer_ui::show(&ctx, &mut app.ui);
    show_shortcuts(&ctx, &mut app.ui);
    show_about(&ctx, &mut app.ui);

    if app.ui.playback.playing && app.ui.settings.visualizations_enabled {
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
}

fn show_menu_bar(app: &mut App, ui: &mut egui::Ui) {
    egui::Panel::top("menu_bar").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open files…").clicked() {
                    app.open_file_dialog();
                    ui.close();
                }
                if ui.button("Clear playlist").clicked() {
                    app.clear_playlist();
                    ui.close();
                }
                ui.separator();
                if ui.button("Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    ui.close();
                }
            });

            ui.menu_button("Playback", |ui| {
                let play_label = if app.ui.playback.playing {
                    "Pause"
                } else {
                    "Play"
                };
                if ui.button(play_label).clicked() {
                    app.toggle_play_pause();
                    ui.close();
                }
                if ui.button("Stop").clicked() {
                    app.stop();
                    ui.close();
                }
                if ui.button("Previous track").clicked() {
                    app.previous_track();
                    ui.close();
                }
                if ui.button("Next track").clicked() {
                    app.next_track();
                    ui.close();
                }
                ui.separator();
                if ui.button("Volume up").clicked() {
                    app.change_volume(crate::ui::state::VOLUME_STEP);
                    ui.close();
                }
                if ui.button("Volume down").clicked() {
                    app.change_volume(-crate::ui::state::VOLUME_STEP);
                    ui.close();
                }
                if ui.button("Mute / unmute").clicked() {
                    app.toggle_mute();
                    ui.close();
                }
                ui.menu_button("Speed", |ui| {
                    for speed in [0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0] {
                        let selected = (app.ui.playback.speed - speed).abs() < f32::EPSILON;
                        if ui
                            .selectable_label(selected, format!("{speed:.2}x"))
                            .clicked()
                        {
                            app.set_speed(speed);
                            ui.close();
                        }
                    }
                });
                ui.separator();
                if ui.button("Equalizer").clicked() {
                    app.ui.show_equalizer = true;
                    ui.close();
                }
            });

            ui.menu_button("View", |ui| {
                ui.checkbox(&mut app.ui.settings.show_playlist, "Show playlist panel");
                if ui.button("Media information").clicked() {
                    app.ui.show_media_info = true;
                    ui.close();
                }
                ui.separator();
                ui.checkbox(&mut app.ui.fit_to_window, "Fit to window");
                if ui.button("Zoom in").clicked() {
                    app.ui.fit_to_window = false;
                    app.ui.zoom = (app.ui.zoom * 1.1).min(4.0);
                    ui.close();
                }
                if ui.button("Zoom out").clicked() {
                    app.ui.fit_to_window = false;
                    app.ui.zoom = (app.ui.zoom / 1.1).max(0.25);
                    ui.close();
                }
                if ui.button("Actual size").clicked() {
                    app.ui.fit_to_window = false;
                    app.ui.zoom = 1.0;
                    ui.close();
                }
                ui.separator();
                if ui.button("Toggle fullscreen").clicked() {
                    toggle_fullscreen(app, ui.ctx());
                    ui.close();
                }
                if ui.button("Settings").clicked() {
                    app.ui.show_settings = true;
                    ui.close();
                }
            });

            ui.menu_button("Help", |ui| {
                if ui.button("Keyboard shortcuts").clicked() {
                    app.ui.show_shortcuts = true;
                    ui.close();
                }
                if ui.button("About OpenWave").clicked() {
                    app.ui.show_about = true;
                    ui.close();
                }
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(entry) = app.ui.current_entry() {
                    ui.label(egui::RichText::new(entry.display_name()).small().weak());
                }
            });
        });
    });
}

fn show_transport(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(2.0);
    if let Some(seconds) = seekbar::show(
        ui,
        app.ui.playback.position_secs(),
        app.ui.playback.duration_secs(),
    ) {
        app.seek_to(seconds);
    }

    if let Some(action) = controls::show(ui, &mut app.ui) {
        match action {
            TransportAction::PlayPause => app.toggle_play_pause(),
            TransportAction::Stop => app.stop(),
            TransportAction::Previous => app.previous_track(),
            TransportAction::Next => app.next_track(),
        }
    }

    ui.add_space(2.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(&app.ui.status).small());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(entry) = app.ui.current_entry() {
                ui.label(egui::RichText::new(entry.format_summary()).small().weak());
                ui.label(
                    egui::RichText::new(format_file_size(entry.file_size))
                        .small()
                        .weak(),
                );
            }
        });
    });

    if let Some(error) = app.ui.last_error.clone() {
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(egui::Color32::LIGHT_RED, format!("⚠ {error}"));
            if ui.small_button("Dismiss").clicked() {
                app.ui.last_error = None;
            }
        });
    }
}

fn paint_welcome(ui: &mut egui::Ui, app: &mut App) {
    ui.allocate_ui_with_layout(
        ui.available_size(),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            ui.add_space((ui.available_height() * 0.22).max(12.0));
            ui.label(egui::RichText::new("OpenWave").size(42.0).strong());
            ui.label(
                egui::RichText::new("A native audio and video player")
                    .size(16.0)
                    .weak(),
            );
            ui.add_space(18.0);
            if ui
                .add(egui::Button::new("Open media files…").min_size(egui::vec2(180.0, 34.0)))
                .clicked()
            {
                app.open_file_dialog();
            }
            ui.add_space(10.0);
            ui.label("or drop files anywhere in this window");
            ui.add_space(28.0);
            ui.label(
                egui::RichText::new(
                    "Supported: MP4, MOV, MKV, WebM, AVI, FLV, OGG, MPEG, ASF, MP3, FLAC, WAV",
                )
                .small()
                .weak(),
            );
        },
    );
}

fn paint_preview(ui: &mut egui::Ui, app: &App) {
    let Some(entry) = app.ui.current_entry() else {
        return;
    };

    let time = ui.input(|input| input.time) as f32;
    let available = ui.available_size();
    let (rect, _) = ui.allocate_exact_size(available, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let visuals = ui.visuals();

    painter.rect_filled(rect, 10.0, visuals.extreme_bg_color);

    let accent = visuals.selection.bg_fill;
    let text_color = visuals.strong_text_color();
    let weak_color = visuals.weak_text_color();
    let center = rect.center();

    painter.circle_filled(center - egui::vec2(0.0, 76.0), 38.0, accent);
    painter.text(
        center - egui::vec2(0.0, 76.0),
        egui::Align2::CENTER_CENTER,
        "OW",
        egui::FontId::proportional(24.0),
        egui::Color32::WHITE,
    );
    painter.text(
        center - egui::vec2(0.0, 16.0),
        egui::Align2::CENTER_CENTER,
        entry.display_name(),
        egui::FontId::proportional(26.0),
        text_color,
    );
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        entry.format_summary(),
        egui::FontId::proportional(14.0),
        weak_color,
    );

    if app.ui.settings.visualizations_enabled {
        let width = (rect.width() * 0.65).min(720.0);
        let baseline = rect.center().y + rect.height() * 0.2;
        let bar_gap = 4.0;
        let bar_count = (width / (8.0 + bar_gap)).floor().max(8.0) as usize;
        let bar_width = ((width - bar_gap * (bar_count as f32 - 1.0)) / bar_count as f32).max(1.0);
        let start_x = center.x - width / 2.0;

        for index in 0..bar_count {
            let phase = time * 3.2 + index as f32 * 0.37;
            let pulse = (phase.sin() * 0.5 + 0.5).powi(2);
            let height = 8.0 + pulse * (rect.height() * 0.16).min(110.0);
            let x = start_x + index as f32 * (bar_width + bar_gap);
            let bar = egui::Rect::from_min_max(
                egui::pos2(x, baseline - height),
                egui::pos2(x + bar_width, baseline),
            );
            let color = if app.ui.playback.playing {
                accent
            } else {
                weak_color
            };
            painter.rect_filled(bar, 2.0, color);
        }
    }

    if !app.ui.fit_to_window {
        painter.text(
            rect.right_top() + egui::vec2(-12.0, 12.0),
            egui::Align2::RIGHT_TOP,
            format!("{:.0}%", app.ui.zoom * 100.0),
            egui::FontId::monospace(12.0),
            weak_color,
        );
    }
}

fn paint_drop_overlay(ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        0.0,
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 185),
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "Drop media files to add them to the playlist",
        egui::FontId::proportional(22.0),
        egui::Color32::WHITE,
    );
}

fn handle_dropped_files(app: &mut App, ctx: &egui::Context) {
    let (hovered_files, dropped_files) = ctx.input(|input| {
        (
            input.raw.hovered_files.len(),
            input.raw.dropped_files.clone(),
        )
    });

    app.ui.drag_active = hovered_files > 0;

    if !dropped_files.is_empty() {
        let paths: Vec<std::path::PathBuf> = dropped_files
            .iter()
            .map(|file| file.path().to_path_buf())
            .collect();
        app.add_files(paths);
    }
}

fn apply_shortcut(app: &mut App, ctx: &egui::Context, action: ShortcutAction) {
    match action {
        ShortcutAction::PlayPause => app.toggle_play_pause(),
        ShortcutAction::SeekBy(delta) => app.seek_relative(delta),
        ShortcutAction::VolumeBy(delta) => app.change_volume(delta),
        ShortcutAction::ToggleMute => app.toggle_mute(),
        ShortcutAction::ToggleFullscreen => toggle_fullscreen(app, ctx),
        ShortcutAction::ExitFullscreen => {
            if app.ui.fullscreen {
                app.ui.fullscreen = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            }
        }
        ShortcutAction::OpenFiles => app.open_file_dialog(),
    }
}

fn toggle_fullscreen(app: &mut App, ctx: &egui::Context) {
    app.ui.fullscreen = !app.ui.fullscreen;
    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(app.ui.fullscreen));
}

fn show_shortcuts(ctx: &egui::Context, state: &mut UiState) {
    if !state.show_shortcuts {
        return;
    }

    let mut open = state.show_shortcuts;
    egui::Window::new("Keyboard Shortcuts")
        .open(&mut open)
        .default_width(420.0)
        .resizable(false)
        .show(ctx, |ui| {
            egui::Grid::new("shortcuts_grid")
                .num_columns(2)
                .striped(true)
                .spacing([24.0, 6.0])
                .show(ui, |ui| {
                    for (keys, description) in SHORTCUTS {
                        ui.label(egui::RichText::new(*keys).monospace().strong());
                        ui.label(*description);
                        ui.end_row();
                    }
                });
        });

    state.show_shortcuts = open;
}

fn show_about(ctx: &egui::Context, state: &mut UiState) {
    if !state.show_about {
        return;
    }

    let mut open = state.show_about;
    egui::Window::new("About OpenWave")
        .open(&mut open)
        .default_width(380.0)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("OpenWave");
            ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
            ui.separator();
            ui.label("A native audio and video player built with Rust and egui.");
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(
                    "Media decoding will be connected in a later implementation stage.",
                )
                .small()
                .weak(),
            );
        });

    state.show_about = open;
}
