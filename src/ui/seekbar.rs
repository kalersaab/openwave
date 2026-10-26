use crate::ui::state::format_time;
use eframe::egui;

/// Draw an interactive seek bar. Returns the requested position in seconds
/// when the user clicks or drags inside the bar.
pub fn show(ui: &mut egui::Ui, position: f64, duration: f64) -> Option<f64> {
    let progress = if duration > 0.0 {
        (position / duration).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let desired_size = egui::vec2(ui.available_width(), 18.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

    let visuals = ui.visuals();
    let track_color = visuals.widgets.inactive.bg_fill;
    let fill_color = visuals.selection.bg_fill;
    let painter = ui.painter();
    let radius = rect.height().min(10.0) / 2.0;

    painter.rect_filled(rect, radius, track_color);
    let filled_width = rect.width() * progress as f32;
    if filled_width > 0.0 {
        let filled = egui::Rect::from_min_size(rect.min, egui::vec2(filled_width, rect.height()));
        painter.rect_filled(filled, radius, fill_color);
    }

    if response.hovered() || duration > 0.0 {
        let knob = egui::pos2(rect.left() + filled_width, rect.center().y);
        painter.circle_filled(knob, 6.0, fill_color);
    }

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format_time(position)).monospace());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(format_time(duration)).monospace());
        });
    });

    if duration > 0.0
        && (response.clicked() || response.dragged())
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let fraction = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        return Some(fraction as f64 * duration);
    }

    None
}
