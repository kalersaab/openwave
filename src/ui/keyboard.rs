use crate::ui::state::{SEEK_STEP, VOLUME_STEP};
use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShortcutAction {
    PlayPause,
    SeekBy(f64),
    VolumeBy(f32),
    ToggleMute,
    ToggleFullscreen,
    ExitFullscreen,
    OpenFiles,
}

pub fn handle(ctx: &egui::Context) -> Option<ShortcutAction> {
    if ctx.egui_wants_keyboard_input() {
        return None;
    }

    let open = ctx.input_mut(|input| {
        input.consume_shortcut(&egui::KeyboardShortcut::new(
            egui::Modifiers::COMMAND,
            egui::Key::O,
        ))
    });
    if open {
        return Some(ShortcutAction::OpenFiles);
    }

    if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
        return Some(ShortcutAction::ExitFullscreen);
    }
    if ctx.input(|input| input.key_pressed(egui::Key::F)) {
        return Some(ShortcutAction::ToggleFullscreen);
    }
    if ctx.input(|input| input.key_pressed(egui::Key::M)) {
        return Some(ShortcutAction::ToggleMute);
    }
    if ctx.input(|input| input.key_pressed(egui::Key::Space)) {
        return Some(ShortcutAction::PlayPause);
    }
    if ctx.input(|input| input.key_pressed(egui::Key::ArrowLeft)) {
        return Some(ShortcutAction::SeekBy(-SEEK_STEP));
    }
    if ctx.input(|input| input.key_pressed(egui::Key::ArrowRight)) {
        return Some(ShortcutAction::SeekBy(SEEK_STEP));
    }
    if ctx.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
        return Some(ShortcutAction::VolumeBy(VOLUME_STEP));
    }
    if ctx.input(|input| input.key_pressed(egui::Key::ArrowDown)) {
        return Some(ShortcutAction::VolumeBy(-VOLUME_STEP));
    }

    None
}

/// Human-readable shortcut list shared by the help window.
pub const SHORTCUTS: &[(&str, &str)] = &[
    ("Space", "Play / pause"),
    ("Left / Right", "Seek backward / forward 5 seconds"),
    ("Up / Down", "Volume up / down 5%"),
    ("M", "Mute / unmute"),
    ("F", "Toggle fullscreen"),
    ("Esc", "Leave fullscreen"),
    ("Ctrl/Cmd+O", "Open media files"),
];
