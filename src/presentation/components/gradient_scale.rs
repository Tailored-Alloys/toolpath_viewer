//! Gradient Scale Component
//!
//! Left-side floating gradient legend with viridis color bar,
//! tick labels, and min/max parameter filter controls.

use egui::{Color32, Context, DragValue, RichText, Rounding, Stroke, Vec2};

use crate::application::ports::{GlobalUnits, ParameterMode};
use crate::presentation::layout::GradientRegion;
use crate::presentation::palette::{self, ThemePalette};
use crate::presentation::theme;

use super::super::ui::ParamRanges;

/// Output from the gradient scale component
#[derive(Debug, Clone)]
pub struct GradientOutput {
    pub filter_min: f32,
    pub filter_max: f32,
}

/// Render the gradient scale panel.
pub fn show_gradient_scale(
    ctx: &Context,
    region: &GradientRegion,
    param_mode: ParameterMode,
    param_filter_min: &mut f32,
    param_filter_max: &mut f32,
    param_ranges: &ParamRanges,
    global_units: &GlobalUnits,
    active_palette: &ThemePalette,
) -> GradientOutput {
    let content_h = region.content_height;

    let bar_w = 20.0;
    let label_h = 16.0;
    let val_label_h = 14.0;
    let filter_section_h = 66.0;
    let spacing = 4.0;
    let controls_h = label_h + val_label_h + val_label_h + filter_section_h + spacing * 4.0;
    let bar_h = (content_h - controls_h).max(50.0);

    let t = theme::active();

    egui::Area::new(egui::Id::new("gradient_scale_area"))
        .fixed_pos(region.pos)
        .order(egui::Order::Foreground)
        .interactable(true)
        .movable(false)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(t.panel_bg_translucent)
                .rounding(Rounding::same(10.0))
                .shadow(egui::epaint::Shadow {
                    offset: egui::vec2(0.0, 2.0),
                    blur: 8.0,
                    spread: 0.0,
                    color: t.panel_shadow,
                })
                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                    let panel_inner_w = region.width - 24.0; // 12+12 horizontal margin
                    let (content_rect, _) = ui.allocate_exact_size(
                        egui::vec2(panel_inner_w, content_h),
                        egui::Sense::hover(),
                    );

                    let center_x = content_rect.center().x;
                    let mut y = content_rect.top();

                    // Mode label with units
                    let mode_label = global_units.param_label(param_mode);
                    ui.painter().text(
                        egui::pos2(center_x, y + label_h / 2.0),
                        egui::Align2::CENTER_CENTER,
                        &mode_label,
                        egui::FontId::proportional(11.0),
                        t.text_primary,
                    );
                    y += label_h + spacing;

                    // Max value label
                    let display_max = global_units.convert_param(param_mode, *param_filter_max);
                    ui.painter().text(
                        egui::pos2(center_x, y + val_label_h / 2.0),
                        egui::Align2::CENTER_CENTER,
                        format!("{:.1}", display_max),
                        egui::FontId::proportional(10.0),
                        t.text_secondary,
                    );
                    y += val_label_h + spacing;

                    // Gradient bar
                    let bar_rect = egui::Rect::from_min_size(
                        egui::pos2(center_x - bar_w / 2.0, y),
                        egui::vec2(bar_w, bar_h),
                    );
                    let n = 64;
                    let seg_h = bar_rect.height() / n as f32;
                    for i in 0..n {
                        let frac = 1.0 - (i as f32 / (n - 1) as f32);
                        let c = palette::gradient_color(active_palette, frac);
                        let y0 = bar_rect.top() + i as f32 * seg_h;
                        let seg = egui::Rect::from_min_max(
                            egui::pos2(bar_rect.left(), y0),
                            egui::pos2(bar_rect.right(), y0 + seg_h + 0.5),
                        );
                        ui.painter().rect_filled(
                            seg,
                            0.0,
                            Color32::from_rgb(
                                (c.r * 255.0) as u8,
                                (c.g * 255.0) as u8,
                                (c.b * 255.0) as u8,
                            ),
                        );
                    }
                    let border_color = if t.is_dark { Color32::from_rgb(80, 80, 80) } else { Color32::from_rgb(180, 180, 180) };
                    ui.painter().rect_stroke(
                        bar_rect,
                        Rounding::same(2.0),
                        Stroke::new(1.0, border_color),
                    );

                    // Tick marks
                    let num_ticks = 5;
                    let val_min = *param_filter_min;
                    let val_max = *param_filter_max;
                    for tick_i in 0..num_ticks {
                        let frac = tick_i as f32 / (num_ticks - 1) as f32;
                        let tick_y = bar_rect.bottom() - frac * bar_rect.height();
                        ui.painter().line_segment(
                            [
                                egui::pos2(bar_rect.right(), tick_y),
                                egui::pos2(bar_rect.right() + 4.0, tick_y),
                            ],
                            Stroke::new(1.0, t.text_secondary),
                        );
                        if region.show_ticks && tick_i > 0 && tick_i < num_ticks - 1 {
                            let raw_val = val_min + frac * (val_max - val_min);
                            let display_val = global_units.convert_param(param_mode, raw_val);
                            ui.painter().text(
                                egui::pos2(bar_rect.right() + 6.0, tick_y),
                                egui::Align2::LEFT_CENTER,
                                format!("{:.0}", display_val),
                                egui::FontId::proportional(8.0),
                                t.text_secondary,
                            );
                        }
                    }
                    y += bar_h + spacing;

                    // Min value label
                    let display_min = global_units.convert_param(param_mode, *param_filter_min);
                    ui.painter().text(
                        egui::pos2(center_x, y + val_label_h / 2.0),
                        egui::Align2::CENTER_CENTER,
                        format!("{:.1}", display_min),
                        egui::FontId::proportional(10.0),
                        t.text_secondary,
                    );
                    y += val_label_h + spacing;

                    // Filter section
                    y += 2.0;
                    ui.painter().line_segment(
                        [
                            egui::pos2(content_rect.left() + 8.0, y),
                            egui::pos2(content_rect.right() - 8.0, y),
                        ],
                        Stroke::new(1.0, if t.is_dark { Color32::from_rgba_premultiplied(100, 100, 100, 80) } else { Color32::from_rgba_premultiplied(180, 180, 180, 80) }),
                    );
                    y += 6.0;

                    let data_range = match param_mode {
                        ParameterMode::Power => param_ranges.power,
                        ParameterMode::Speed => param_ranges.speed,
                        ParameterMode::WaitTime => param_ranges.wait_time,
                    };
                    let (lo, hi) = data_range.unwrap_or((0.0, 1.0));
                    let speed = (hi - lo).abs() * 0.01;
                    let unit_suffix = global_units.param_suffix(param_mode);

                    let row_h = 18.0;
                    let label_w = 26.0;
                    let input_w = panel_inner_w - label_w - 4.0;
                    let margin_x = content_rect.left() + 2.0;

                    // Min label + DragValue
                    ui.painter().text(
                        egui::pos2(margin_x + label_w / 2.0, y + row_h / 2.0),
                        egui::Align2::CENTER_CENTER,
                        "Min",
                        egui::FontId::proportional(9.0),
                        t.text_secondary,
                    );
                    let min_input_rect = egui::Rect::from_min_size(
                        egui::pos2(margin_x + label_w + 2.0, y + 1.0),
                        egui::vec2(input_w, row_h - 2.0),
                    );
                    ui.put(
                        min_input_rect,
                        DragValue::new(param_filter_min)
                            .speed(speed.max(0.1))
                            .clamp_range(lo..=*param_filter_max)
                            .suffix(&unit_suffix),
                    );
                    y += row_h + 2.0;

                    // Max label + DragValue
                    ui.painter().text(
                        egui::pos2(margin_x + label_w / 2.0, y + row_h / 2.0),
                        egui::Align2::CENTER_CENTER,
                        "Max",
                        egui::FontId::proportional(9.0),
                        t.text_secondary,
                    );
                    let max_input_rect = egui::Rect::from_min_size(
                        egui::pos2(margin_x + label_w + 2.0, y + 1.0),
                        egui::vec2(input_w, row_h - 2.0),
                    );
                    ui.put(
                        max_input_rect,
                        DragValue::new(param_filter_max)
                            .speed(speed.max(0.1))
                            .clamp_range(*param_filter_min..=hi)
                            .suffix(&unit_suffix),
                    );
                    y += row_h + 4.0;

                    // Reset button
                    let reset_rect = egui::Rect::from_center_size(
                        egui::pos2(center_x, y + 9.0),
                        egui::vec2(panel_inner_w - 8.0, 18.0),
                    );
                    let reset_btn = egui::Button::new(
                        RichText::new("Reset").size(9.0),
                    )
                    .rounding(Rounding::same(4.0))
                    .min_size(Vec2::new(0.0, 16.0));
                    if ui.put(reset_rect, reset_btn).clicked() {
                        *param_filter_min = lo;
                        *param_filter_max = hi;
                    }
                });
        });

    GradientOutput {
        filter_min: *param_filter_min,
        filter_max: *param_filter_max,
    }
}
