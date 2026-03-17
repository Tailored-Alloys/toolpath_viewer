//! Overlay Components
//!
//! Non-interactive overlays painted on the egui foreground layer:
//! - Scale bar (bottom-left)
//! - Ruler measurements (persistent + live)
//! - Zoom selection rectangle
//! - Grid coordinate labels

use egui::{Color32, Context, Rounding, Stroke};

use crate::application::ports::{GridUnit, ViewState};
use crate::presentation::layout::{TOOLBAR_BOTTOM, TOOLBAR_HEIGHT};
use crate::presentation::theme::ACCENT;

use super::super::ui::RulerMeasurement;

/// Render the scale bar overlay in the bottom-left corner.
/// `left_offset` shifts the bar right (e.g., when gradient panel is visible).
pub fn show_scale_bar(ctx: &Context, zoom: f32, grid_unit: GridUnit, left_offset: f32) {
    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("scale_bar"),
    ));
    let bar_margin = 20.0;
    let bar_y = screen.bottom() - bar_margin;
    let bar_x_start = left_offset + bar_margin + 10.0;

    if zoom <= 0.0 {
        return;
    }

    let nice_values: &[f32] = &[
        0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0,
    ];
    let max_px = 200.0;
    let mut bar_world = 1.0_f32;
    for &v in nice_values.iter().rev() {
        if v * zoom <= max_px {
            bar_world = v;
            break;
        }
    }
    let bar_px = bar_world * zoom;
    let bar_x_end = bar_x_start + bar_px;
    let tick_h = 6.0;
    let bar_color = Color32::from_rgb(60, 60, 60);

    // Bar line
    painter.line_segment(
        [egui::pos2(bar_x_start, bar_y), egui::pos2(bar_x_end, bar_y)],
        Stroke::new(2.0, bar_color),
    );
    // Left tick
    painter.line_segment(
        [
            egui::pos2(bar_x_start, bar_y - tick_h),
            egui::pos2(bar_x_start, bar_y + tick_h),
        ],
        Stroke::new(2.0, bar_color),
    );
    // Right tick
    painter.line_segment(
        [
            egui::pos2(bar_x_end, bar_y - tick_h),
            egui::pos2(bar_x_end, bar_y + tick_h),
        ],
        Stroke::new(2.0, bar_color),
    );
    // Label
    let value = grid_unit.from_mm(bar_world);
    let label = if value >= 1.0 {
        format!("{:.0} {}", value, grid_unit.label())
    } else {
        format!("{:.2} {}", value, grid_unit.label())
    };
    painter.text(
        egui::pos2((bar_x_start + bar_x_end) / 2.0, bar_y - tick_h - 4.0),
        egui::Align2::CENTER_BOTTOM,
        label,
        egui::FontId::proportional(11.0),
        bar_color,
    );
}

/// Render ruler measurement overlays (persistent + live preview).
pub fn show_ruler_overlay(
    ctx: &Context,
    measurements: &[RulerMeasurement],
    ruler_start: Option<crate::domain::value_objects::Point2D>,
    ruler_end: Option<crate::domain::value_objects::Point2D>,
    view_transform: Option<&(f32, f32, ViewState)>,
    grid_unit: GridUnit,
) {
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("ruler_overlay"),
    ));
    let ruler_color = Color32::from_rgb(220, 50, 50);
    let label_bg = Color32::from_rgba_premultiplied(255, 255, 255, 200);

    let dot_color = Color32::from_rgb(140, 140, 140);
    let dot_radius = 5.0;

    // Draw pending start dot (grey) when first measurement point is placed
    if let Some(start) = ruler_start {
        if let Some((vw, vh, view)) = view_transform {
            let s_start = view.world_to_screen(start.x, start.y, *vw, *vh);
            let p = egui::pos2(s_start.x, s_start.y + TOOLBAR_BOTTOM);
            painter.circle_filled(p, dot_radius, dot_color);
        }
    }

    // Draw persistent measurements
    for m in measurements {
        let s = view_transform.map(|(vw, vh, view)| {
            let s_start = view.world_to_screen(m.start.x, m.start.y, *vw, *vh);
            let s_end = view.world_to_screen(m.end.x, m.end.y, *vw, *vh);
            (
                egui::pos2(s_start.x, s_start.y + TOOLBAR_BOTTOM),
                egui::pos2(s_end.x, s_end.y + TOOLBAR_BOTTOM),
            )
        });
        if let Some((p1, p2)) = s {
            painter.line_segment([p1, p2], Stroke::new(2.0, ruler_color));
            painter.circle_filled(p1, dot_radius, dot_color);
            painter.circle_filled(p2, dot_radius, dot_color);
            let mid = egui::pos2((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0);
            let dist_display = grid_unit.from_mm(m.distance_mm);
            let label = format!("{:.3} {}", dist_display, grid_unit.label());
            let text_rect = painter.text(
                mid + egui::vec2(0.0, -12.0),
                egui::Align2::CENTER_BOTTOM,
                &label,
                egui::FontId::proportional(12.0),
                ruler_color,
            );
            painter.rect_filled(text_rect.expand(2.0), Rounding::same(2.0), label_bg);
            painter.text(
                mid + egui::vec2(0.0, -12.0),
                egui::Align2::CENTER_BOTTOM,
                &label,
                egui::FontId::proportional(12.0),
                ruler_color,
            );
        }
    }

    // Draw live ruler line (grey preview: start → current cursor)
    if ruler_start.is_some() {
        if let (Some(start), Some(end)) = (ruler_start, ruler_end) {
            if let Some((vw, vh, view)) = view_transform {
                let s_start = view.world_to_screen(start.x, start.y, *vw, *vh);
                let s_end = view.world_to_screen(end.x, end.y, *vw, *vh);
                let p1 = egui::pos2(s_start.x, s_start.y + TOOLBAR_BOTTOM);
                let p2 = egui::pos2(s_end.x, s_end.y + TOOLBAR_BOTTOM);
                let preview_color = Color32::from_rgb(160, 160, 160);
                painter.line_segment([p1, p2], Stroke::new(1.5, preview_color));
                painter.circle_filled(
                    p2,
                    3.0,
                    Color32::from_rgba_premultiplied(140, 140, 140, 150),
                );
                let dist_mm = start.distance_to(&end);
                let dist_display = grid_unit.from_mm(dist_mm);
                let mid = egui::pos2((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0);
                let label = format!("{:.3} {}", dist_display, grid_unit.label());
                painter.text(
                    mid + egui::vec2(0.0, -12.0),
                    egui::Align2::CENTER_BOTTOM,
                    &label,
                    egui::FontId::proportional(12.0),
                    preview_color,
                );
            }
        }
    }
}

/// Render zoom selection rectangle overlay.
pub fn show_zoom_rect(
    ctx: &Context,
    start: crate::domain::value_objects::Point2D,
    end: crate::domain::value_objects::Point2D,
) {
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("zoom_rect"),
    ));
    let rect = egui::Rect::from_two_pos(egui::pos2(start.x, start.y), egui::pos2(end.x, end.y));
    painter.rect_filled(
        rect,
        0.0,
        Color32::from_rgba_premultiplied(25, 118, 210, 30),
    );
    painter.rect_stroke(rect, 0.0, Stroke::new(1.5, ACCENT));
}

/// Render grid coordinate labels overlay along viewport edges.
pub fn show_grid_labels(
    ctx: &Context,
    view_transform: &(f32, f32, ViewState),
    grid_unit: GridUnit,
) {
    let (vw, vh, view) = view_transform;
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("grid_labels"),
    ));
    let (_, major_spacing) = crate::infrastructure::rendering::GridRenderer::spacing(view.zoom);
    let (vis_min, vis_max) = view.visible_bounds(*vw, *vh);
    let label_color = Color32::from_rgb(100, 100, 100);
    let label_bg = Color32::from_rgba_premultiplied(250, 250, 250, 200);

    // Bottom edge labels (X axis)
    let x_start = (vis_min.x / major_spacing).floor() * major_spacing;
    let mut x = x_start;
    while x <= vis_max.x {
        let screen_pt = view.world_to_screen(x, vis_min.y, *vw, *vh);
        let sx = screen_pt.x;
        let sy = *vh + TOOLBAR_HEIGHT - 2.0;
        if sx > 50.0 && sx < *vw - 20.0 {
            let val = grid_unit.from_mm(x);
            let label = if val.abs() >= 1.0 {
                format!("{:.0}", val)
            } else {
                format!("{:.2}", val)
            };
            let r = painter.text(
                egui::pos2(sx, sy),
                egui::Align2::CENTER_BOTTOM,
                &label,
                egui::FontId::proportional(9.0),
                label_color,
            );
            painter.rect_filled(r.expand(1.0), 0.0, label_bg);
            painter.text(
                egui::pos2(sx, sy),
                egui::Align2::CENTER_BOTTOM,
                &label,
                egui::FontId::proportional(9.0),
                label_color,
            );
        }
        x += major_spacing;
    }

    // Left edge labels (Y axis)
    let y_start = (vis_min.y / major_spacing).floor() * major_spacing;
    let mut y = y_start;
    while y <= vis_max.y {
        let screen_pt = view.world_to_screen(vis_min.x, y, *vw, *vh);
        let sx = 4.0;
        let sy = screen_pt.y + TOOLBAR_HEIGHT;
        if sy > TOOLBAR_HEIGHT + 20.0 && sy < TOOLBAR_HEIGHT + *vh - 20.0 {
            let val = grid_unit.from_mm(y);
            let label = if val.abs() >= 1.0 {
                format!("{:.0}", val)
            } else {
                format!("{:.2}", val)
            };
            let r = painter.text(
                egui::pos2(sx, sy),
                egui::Align2::LEFT_CENTER,
                &label,
                egui::FontId::proportional(9.0),
                label_color,
            );
            painter.rect_filled(r.expand(1.0), 0.0, label_bg);
            painter.text(
                egui::pos2(sx, sy),
                egui::Align2::LEFT_CENTER,
                &label,
                egui::FontId::proportional(9.0),
                label_color,
            );
        }
        y += major_spacing;
    }
}
