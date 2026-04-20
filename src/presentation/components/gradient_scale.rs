//! Gradient Scale Overlay Component
//!
//! Floating vertical gradient legend on the left side of the viewport.
//! Layout (matching reference design):
//! - Palette icon at top — click opens dropdown to pick colormap
//! - Mode label (e.g. "Power (W)")
//! - Max value — click to edit inline
//! - Tall gradient bar
//! - Min value — click to edit inline
//! - Reset button

use egui::{Color32, Context, RichText, Rounding, Stroke, TextEdit, Vec2};
use lucide_icons::Icon as LucideIcon;

use crate::application::ports::{GlobalUnits, GradientPaletteId, ParameterMode, ALL_GRADIENT_PALETTE_IDS};
use crate::domain::value_objects::Color;
use crate::presentation::layout::GradientRegion;
use crate::presentation::palette;
use crate::presentation::theme;

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
    editing_max_key: &str,
    editing_min_key: &str,
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

    // Persistent editing state stored in egui's per-frame memory
    let editing_max_id = egui::Id::new(editing_max_key);
    let editing_min_id = egui::Id::new(editing_min_key);
    // Staging text buffers: edits accumulate here, only committed on Enter / click-away
    let staging_max_id = egui::Id::new(format!("{}_staging", editing_max_key));
    let staging_min_id = egui::Id::new(format!("{}_staging", editing_min_key));
    // Track whether we already requested focus (to avoid re-requesting each frame)
    let focus_max_id = egui::Id::new(format!("{}_focus", editing_max_key));
    let focus_min_id = egui::Id::new(format!("{}_focus", editing_min_key));

    egui::Area::new(egui::Id::new(area_id))
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
                .inner_margin(egui::Margin::symmetric(10.0, 10.0))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 4.0);

                    let panel_inner_w = region.width - 20.0; // 10+10 margin
                    ui.set_max_width(panel_inner_w);

                    // ── Palette icon (top, centered) ──
                    ui.vertical_centered(|ui| {
                        let icon_str = LucideIcon::Palette.unicode().to_string();
                        let icon_btn = egui::Button::new(
                            RichText::new(&icon_str).size(16.0).color(t.text_secondary),
                        )
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE)
                        .min_size(Vec2::new(24.0, 24.0));

                        let popup_id = ui.id().with("palette_popup");
                        let icon_resp = ui.add(icon_btn).on_hover_text("Change color palette");
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
                    });

                    // ── Mode label (centered) ──
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(mode_label)
                                .size(10.0)
                                .strong()
                                .color(t.text_primary),
                        );
                    });

                    ui.add_space(2.0);

                    // ── Max value (clickable → text input, commits on Enter/click-away) ──
                    let editing_max = ui.memory(|m| m.data.get_temp::<bool>(editing_max_id).unwrap_or(false));
                    ui.vertical_centered(|ui| {
                        if editing_max {
                            let mut buf = ui.memory(|m| {
                                m.data.get_temp::<String>(staging_max_id).unwrap_or_default()
                            });
                            let resp = ui.add(
                                TextEdit::singleline(&mut buf)
                                    .desired_width(panel_inner_w)
                                    .font(egui::TextStyle::Small)
                                    .horizontal_align(egui::Align::Center),
                            );
                            // Filter to numeric characters only
                            buf.retain(|c| c.is_ascii_digit() || c == '.' || c == '-');
                            ui.memory_mut(|m| m.data.insert_temp(staging_max_id, buf.clone()));
                            // Request focus once on the first frame
                            let did_focus = ui.memory(|m| m.data.get_temp::<bool>(focus_max_id).unwrap_or(false));
                            if !did_focus {
                                resp.request_focus();
                                ui.memory_mut(|m| m.data.insert_temp(focus_max_id, true));
                            }
                            // Cancel on Escape
                            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                ui.memory_mut(|m| {
                                    m.data.insert_temp(editing_max_id, false);
                                    m.data.insert_temp(focus_max_id, false);
                                });
                            }
                            // Commit on Enter / click-away
                            else if resp.lost_focus() {
                                if let Ok(v) = buf.trim().parse::<f32>() {
                                    *filter_max = v.clamp(*filter_min, hi);
                                }
                                ui.memory_mut(|m| {
                                    m.data.insert_temp(editing_max_id, false);
                                    m.data.insert_temp(focus_max_id, false);
                                });
                            }
                        } else {
                            let display_val = convert_display(*filter_max);
                            let label_resp = ui.add(
                                egui::Label::new(
                                    RichText::new(format!("{:.2}{}", display_val, unit_suffix))
                                        .size(10.0)
                                        .color(t.text_secondary),
                                )
                                .sense(egui::Sense::click()),
                            );
                            if label_resp.on_hover_cursor(egui::CursorIcon::Text).clicked() {
                                let display_val = convert_display(*filter_max);
                                ui.memory_mut(|m| {
                                    m.data.insert_temp(editing_max_id, true);
                                    m.data.insert_temp(staging_max_id, format!("{:.2}", display_val));
                                    m.data.insert_temp(focus_max_id, false);
                                });
                            }
                        }
                    });

                    ui.add_space(2.0);

                    // ── Gradient bar (full width, fills remaining height) ──
                    let bar_h = (region.content_height - 130.0).max(60.0);
                    let bar_w = panel_inner_w;
                    let (bar_rect, _) = ui.allocate_exact_size(
                        egui::vec2(bar_w, bar_h),
                        egui::Sense::hover(),
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

                    ui.add_space(2.0);

                    // ── Min value (clickable → text input, commits on Enter/click-away) ──
                    let editing_min = ui.memory(|m| m.data.get_temp::<bool>(editing_min_id).unwrap_or(false));
                    ui.vertical_centered(|ui| {
                        if editing_min {
                            let mut buf = ui.memory(|m| {
                                m.data.get_temp::<String>(staging_min_id).unwrap_or_default()
                            });
                            let resp = ui.add(
                                TextEdit::singleline(&mut buf)
                                    .desired_width(panel_inner_w)
                                    .font(egui::TextStyle::Small)
                                    .horizontal_align(egui::Align::Center),
                            );
                            // Filter to numeric characters only
                            buf.retain(|c| c.is_ascii_digit() || c == '.' || c == '-');
                            ui.memory_mut(|m| m.data.insert_temp(staging_min_id, buf.clone()));
                            // Request focus once on the first frame
                            let did_focus = ui.memory(|m| m.data.get_temp::<bool>(focus_min_id).unwrap_or(false));
                            if !did_focus {
                                resp.request_focus();
                                ui.memory_mut(|m| m.data.insert_temp(focus_min_id, true));
                            }
                            // Cancel on Escape
                            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                ui.memory_mut(|m| {
                                    m.data.insert_temp(editing_min_id, false);
                                    m.data.insert_temp(focus_min_id, false);
                                });
                            }
                            // Commit on Enter / click-away
                            else if resp.lost_focus() {
                                if let Ok(v) = buf.trim().parse::<f32>() {
                                    *filter_min = v.clamp(lo, *filter_max);
                                }
                                ui.memory_mut(|m| {
                                    m.data.insert_temp(editing_min_id, false);
                                    m.data.insert_temp(focus_min_id, false);
                                });
                            }
                        } else {
                            let display_val = convert_display(*filter_min);
                            let label_resp = ui.add(
                                egui::Label::new(
                                    RichText::new(format!("{:.2}{}", display_val, unit_suffix))
                                        .size(10.0)
                                        .color(t.text_secondary),
                                )
                                .sense(egui::Sense::click()),
                            );
                            if label_resp.on_hover_cursor(egui::CursorIcon::Text).clicked() {
                                let display_val = convert_display(*filter_min);
                                ui.memory_mut(|m| {
                                    m.data.insert_temp(editing_min_id, true);
                                    m.data.insert_temp(staging_min_id, format!("{:.2}", display_val));
                                    m.data.insert_temp(focus_min_id, false);
                                });
                            }
                        }
                    });

                    ui.add_space(4.0);

                    // ── Reset button (centered, compact, no wrap) ──
                    ui.vertical_centered(|ui| {
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
                        if ui.add(reset_btn).on_hover_text("Reset to data range").clicked() {
                            *filter_min = lo;
                            *filter_max = hi;
                        }
                    });
                });
        });
}
