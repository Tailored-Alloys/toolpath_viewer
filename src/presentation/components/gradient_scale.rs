//! Gradient Scale Overlay Component
//!
//! Floating vertical gradient legend on the left side of the viewport.
//! Layout (matching reference design):
//! - Palette icon at top — click opens dropdown to pick colormap
//! - Mode label (e.g. "Power (W)")
//! - Max value — DragValue input
//! - Tall gradient bar
//! - Min value — DragValue input
//! - Reset button

use egui::{Color32, Context, DragValue, RichText, Rounding, Stroke, Vec2};
use lucide_icons::Icon as LucideIcon;

use crate::application::ports::{GlobalUnits, GradientPaletteId, ParameterMode, ALL_GRADIENT_PALETTE_IDS};
use crate::domain::value_objects::Color;
use crate::presentation::layout::GradientRegion;
use crate::presentation::palette;
use crate::presentation::theme::{self, floating_panel_frame};

use super::super::ui::ParamRanges;

/// Output from the gradient scale component
#[derive(Debug, Clone)]
pub struct GradientOutput {
    pub filter_min: f32,
    pub filter_max: f32,
    pub palette_changed: bool,
}

/// Render the parameter gradient scale overlay.
pub fn show_gradient_scale(
    ctx: &Context,
    region: &GradientRegion,
    param_mode: ParameterMode,
    param_filter_min: &mut f32,
    param_filter_max: &mut f32,
    param_ranges: &ParamRanges,
    global_units: &GlobalUnits,
    gradient_palette_id: &mut GradientPaletteId,
    _expanded: &mut bool,
) -> GradientOutput {
    let stops = palette::resolve_gradient_stops(*gradient_palette_id);
    let mode_label = global_units.param_label(param_mode);
    let data_range = match param_mode {
        ParameterMode::Power => param_ranges.power,
        ParameterMode::Speed => param_ranges.speed,
    };
    let (lo, hi) = data_range.unwrap_or((0.0, 1.0));
    let unit_suffix = global_units.param_suffix(param_mode);
    let speed = (hi - lo).abs() * 0.01;

    let mut palette_changed = false;

    render_scale_overlay(
        ctx,
        "gradient_scale_area",
        "gradient_editing_max",
        "gradient_editing_min",
        region,
        &mode_label,
        &stops,
        gradient_palette_id,
        &mut palette_changed,
        param_filter_max,
        param_filter_min,
        lo,
        hi,
        speed,
        &unit_suffix,
        |raw| global_units.convert_param(param_mode, raw),
    );

    GradientOutput {
        filter_min: *param_filter_min,
        filter_max: *param_filter_max,
        palette_changed,
    }
}

// ── Shared rendering logic ──────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn render_scale_overlay(
    ctx: &Context,
    area_id: &str,
    _editing_max_key: &str,
    _editing_min_key: &str,
    region: &GradientRegion,
    mode_label: &str,
    stops: &[(f32, Color)],
    palette_id: &mut GradientPaletteId,
    palette_changed: &mut bool,
    filter_max: &mut f32,
    filter_min: &mut f32,
    lo: f32,
    hi: f32,
    drag_speed: f32,
    unit_suffix: &str,
    convert_display: impl Fn(f32) -> f32,
) {
    let t = theme::active();

    // Conversion factor: display = convert_display(raw), so factor = convert_display(1) / 1
    let factor = if hi.abs() > f32::EPSILON {
        convert_display(hi) / hi
    } else if lo.abs() > f32::EPSILON {
        convert_display(lo) / lo
    } else {
        1.0
    };

    let content_h = region.content_height;

    // Fixed heights for controls (matching layer_slider layout)
    let icon_h = 24.0;
    let label_h = 16.0;
    let input_h = 22.0;
    let reset_h = 20.0;
    let spacing = 6.0;
    // Height consumed by icon + label + max input + min input + reset + spacing
    let controls_h = icon_h + label_h + input_h * 2.0 + reset_h + spacing * 6.0;
    let bar_h = (content_h - controls_h).max(60.0);

    egui::Area::new(egui::Id::new(area_id))
        .fixed_pos(region.pos)
        .order(egui::Order::Foreground)
        .interactable(true)
        .movable(false)
        .show(ctx, |ui| {
            floating_panel_frame()
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                    let panel_inner_w = region.width - 16.0; // 8+8 margin from floating_panel_frame
                    ui.set_max_width(panel_inner_w);
                    let (content_rect, _) = ui.allocate_exact_size(
                        egui::vec2(panel_inner_w, content_h),
                        egui::Sense::hover(),
                    );

                    let center_x = content_rect.center().x;
                    let mut y_cursor = content_rect.top();

                    // ── Palette icon (top, centered) ──
                    let icon_rect = egui::Rect::from_center_size(
                        egui::pos2(center_x, y_cursor + icon_h / 2.0),
                        egui::vec2(44.0, icon_h),
                    );
                    let icon_str = LucideIcon::Palette.unicode().to_string();
                    let icon_btn = egui::Button::new(
                        RichText::new(&icon_str).size(16.0).color(t.text_secondary),
                    )
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE)
                    .min_size(Vec2::new(24.0, icon_h));

                    let popup_id = ui.id().with("palette_popup");
                    let icon_resp = ui.put(icon_rect, icon_btn)
                        .on_hover_text("Change color palette");
                    if icon_resp.clicked() {
                        ui.memory_mut(|m| m.toggle_popup(popup_id));
                    }

                    // Dropdown popup below the icon
                    egui::popup_below_widget(ui, popup_id, &icon_resp, |ui| {
                        ui.set_min_width(100.0);
                        for &pid in ALL_GRADIENT_PALETTE_IDS {
                            let selected = *palette_id == pid;
                            let label = if selected {
                                RichText::new(format!("● {}", pid.label())).size(11.0).strong()
                            } else {
                                RichText::new(format!("  {}", pid.label())).size(11.0)
                            };
                            if ui.selectable_label(selected, label).clicked() {
                                *palette_id = pid;
                                *palette_changed = true;
                                ui.memory_mut(|m| m.toggle_popup(popup_id));
                            }
                        }
                    });
                    y_cursor += icon_h + spacing;

                    // ── Mode label (centered) ──
                    ui.painter().text(
                        egui::pos2(center_x, y_cursor + label_h / 2.0),
                        egui::Align2::CENTER_CENTER,
                        mode_label,
                        egui::FontId::proportional(10.0),
                        t.text_primary,
                    );
                    y_cursor += label_h + spacing;

                    // ── Max value (DragValue input, centered) ──
                    let max_rect = egui::Rect::from_center_size(
                        egui::pos2(center_x, y_cursor + input_h / 2.0),
                        egui::vec2(panel_inner_w, input_h),
                    );
                    {
                        let suffix_owned = unit_suffix.to_owned();
                        let mut display_val = convert_display(*filter_max);
                        let dv = DragValue::new(&mut display_val)
                            .speed(drag_speed * factor.abs().max(0.001))
                            .clamp_range(convert_display(*filter_min)..=convert_display(hi))
                            .custom_formatter(|v, _| format!("{:.2}", v))
                            .suffix(suffix_owned);
                        let resp = ui.put(max_rect, dv);
                        if resp.changed() {
                            let raw = if factor.abs() > f32::EPSILON {
                                display_val / factor
                            } else {
                                display_val
                            };
                            *filter_max = raw.clamp(*filter_min, hi);
                        }
                    }
                    y_cursor += input_h + spacing;

                    // ── Gradient bar (centered, narrower than panel) ──
                    let bar_w = (panel_inner_w - 8.0).max(16.0); // 4px padding each side
                    let bar_rect = egui::Rect::from_center_size(
                        egui::pos2(center_x, y_cursor + bar_h / 2.0),
                        egui::vec2(bar_w, bar_h),
                    );

                    // Draw gradient segments
                    let n = 128;
                    let seg_h = bar_rect.height() / n as f32;
                    for i in 0..n {
                        let frac = 1.0 - (i as f32 / (n - 1) as f32);
                        let c = palette::gradient_color_from_stops(stops, frac);
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

                    // Border
                    let border_color = if t.is_dark {
                        Color32::from_rgb(80, 80, 80)
                    } else {
                        Color32::from_rgb(180, 180, 180)
                    };
                    ui.painter().rect_stroke(
                        bar_rect,
                        Rounding::same(3.0),
                        Stroke::new(1.0, border_color),
                    );
                    y_cursor += bar_h + spacing;

                    // ── Min value (DragValue input, centered) ──
                    let min_rect = egui::Rect::from_center_size(
                        egui::pos2(center_x, y_cursor + input_h / 2.0),
                        egui::vec2(panel_inner_w, input_h),
                    );
                    {
                        let suffix_owned = unit_suffix.to_owned();
                        let mut display_val = convert_display(*filter_min);
                        let dv = DragValue::new(&mut display_val)
                            .speed(drag_speed * factor.abs().max(0.001))
                            .clamp_range(convert_display(lo)..=convert_display(*filter_max))
                            .custom_formatter(|v, _| format!("{:.2}", v))
                            .suffix(suffix_owned);
                        let resp = ui.put(min_rect, dv);
                        if resp.changed() {
                            let raw = if factor.abs() > f32::EPSILON {
                                display_val / factor
                            } else {
                                display_val
                            };
                            *filter_min = raw.clamp(lo, *filter_max);
                        }
                    }
                    y_cursor += input_h + spacing;

                    // ── Reset button (centered, compact) ──
                    let reset_rect = egui::Rect::from_center_size(
                        egui::pos2(center_x, y_cursor + reset_h / 2.0),
                        egui::vec2(panel_inner_w, reset_h),
                    );
                    let reset_icon = LucideIcon::RotateCcw.unicode().to_string();
                    let reset_btn = egui::Button::new(
                        RichText::new(format!("{} Reset", reset_icon))
                            .size(9.0)
                            .color(t.text_secondary),
                    )
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::new(
                        1.0,
                        if t.is_dark {
                            Color32::from_rgb(70, 70, 70)
                        } else {
                            Color32::from_rgb(200, 200, 200)
                        },
                    ))
                    .rounding(Rounding::same(4.0))
                    .min_size(Vec2::new(0.0, 18.0))
                    .wrap(false);
                    if ui.put(reset_rect, reset_btn).on_hover_text("Reset to data range").clicked() {
                        *filter_min = lo;
                        *filter_max = hi;
                    }
                });
        });
}
