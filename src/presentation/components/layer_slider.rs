//! Layer Slider Component
//!
//! Right-side floating vertical layer navigation panel with:
//! - First/last layer buttons
//! - Next/prev layer buttons
//! - Vertical egui Slider for visual scrubbing
//! - Numeric DragValue input for direct layer entry
//! - Z-height display

use egui::{Color32, CursorIcon, Context, DragValue, RichText, Rounding, Stroke};
use lucide_icons::Icon as LucideIcon;

use crate::application::ports::GlobalUnits;
use crate::presentation::layout::LayerSliderRegion;
use crate::presentation::theme::{self, floating_panel_frame};

/// Output from the layer slider component
#[derive(Debug, Clone, Default)]
pub struct LayerSliderOutput {
    pub layer_changed: bool,
    pub new_layer: usize,
}

/// Render the layer slider panel.
pub fn show_layer_slider(
    ctx: &Context,
    region: &LayerSliderRegion,
    current_layer: usize,
    total_layers: usize,
    current_z: f32,
    global_units: &GlobalUnits,
    focus_layer_input: &mut bool,
) -> LayerSliderOutput {
    let mut output = LayerSliderOutput::default();
    let t = theme::active();
    let max_layer = total_layers.saturating_sub(1);
    let content_h = region.content_height;

    // Fixed heights for controls
    let btn_h = 24.0;
    let goto_h = 36.0;
    let spacing = 6.0;
    // Height consumed by buttons + input + z label + spacing
    let controls_h = btn_h * 4.0 + goto_h + spacing * 4.0;
    let slider_h = (content_h - controls_h).max(60.0);

    egui::Area::new(egui::Id::new("layer_slider_area"))
        .fixed_pos(region.pos)
        .order(egui::Order::Foreground)
        .interactable(true)
        .movable(false)
        .show(ctx, |ui| {
            floating_panel_frame()
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                    let panel_inner_w = region.width - 16.0;
                    ui.set_max_width(panel_inner_w);
                    let (content_rect, _) = ui.allocate_exact_size(
                        egui::vec2(panel_inner_w, content_h),
                        egui::Sense::hover(),
                    );

                    let center_x = content_rect.center().x;

                    // Last layer button (top — highest layer number)
                    let last_icon = LucideIcon::ChevronsDown.unicode().to_string();
                    let first_btn_rect = egui::Rect::from_center_size(
                        egui::pos2(center_x, content_rect.top() + btn_h / 2.0),
                        egui::vec2(panel_inner_w, btn_h),
                    );
                    let last_btn_top = egui::Button::new(
                        RichText::new(&last_icon).size(16.0).color(t.text_primary),
                    )
                    .rounding(Rounding::same(6.0))
                    .fill(Color32::TRANSPARENT);
                    if ui
                        .put(first_btn_rect, last_btn_top)
                        .on_hover_text("Last layer (End)")
                        .clicked()
                    {
                        output.new_layer = max_layer;
                        output.layer_changed = true;
                    }

                    // Next layer button (+1)
                    let next_icon = LucideIcon::ChevronUp.unicode().to_string();
                    let next_btn_rect = egui::Rect::from_center_size(
                        egui::pos2(center_x, content_rect.top() + btn_h + btn_h / 2.0),
                        egui::vec2(panel_inner_w, btn_h),
                    );
                    let next_btn = egui::Button::new(
                        RichText::new(&next_icon).size(16.0).color(t.text_primary),
                    )
                    .rounding(Rounding::same(6.0))
                    .fill(Color32::TRANSPARENT);
                    if ui
                        .put(next_btn_rect, next_btn)
                        .on_hover_text("Next layer (↑)")
                        .clicked()
                    {
                        output.new_layer = (current_layer + 1).min(max_layer);
                        output.layer_changed = true;
                    }

                    // ── Vertical custom slider (fully manual interaction + painting) ──
                    let slider_top_y = content_rect.top() + btn_h * 2.0 + spacing;
                    let slider_w = panel_inner_w;
                    let rail_thickness = 6.0;
                    let handle_radius = 12.0;

                    let slider_rect = egui::Rect::from_center_size(
                        egui::pos2(center_x, slider_top_y + slider_h / 2.0),
                        egui::vec2(slider_w, slider_h),
                    );

                    // Rail extents (handle center stays within these bounds)
                    let rail_top = slider_rect.top() + handle_radius;
                    let rail_bottom = slider_rect.bottom() - handle_radius;
                    let rail_len = rail_bottom - rail_top;

                    // Current handle position based on layer
                    let frac = if max_layer > 0 {
                        current_layer as f32 / max_layer as f32
                    } else {
                        0.0
                    };
                    let handle_y = rail_bottom - frac * rail_len;

                    // Allocate interaction area (click + drag sense)
                    let resp = ui.allocate_rect(slider_rect, egui::Sense::click_and_drag());

                    // Convert pointer Y to a layer index
                    let layer_from_pointer = |pointer_y: f32| -> usize {
                        if rail_len <= 0.0 || max_layer == 0 {
                            return 0;
                        }
                        let clamped_y = pointer_y.clamp(rail_top, rail_bottom);
                        let t = (rail_bottom - clamped_y) / rail_len;
                        let layer = (t * max_layer as f32).round() as usize;
                        layer.min(max_layer)
                    };

                    if resp.dragged() {
                        if let Some(pos) = resp.interact_pointer_pos() {
                            let new = layer_from_pointer(pos.y);
                            if new != current_layer {
                                output.new_layer = new;
                                output.layer_changed = true;
                            }
                        }
                    } else if resp.clicked() {
                        if let Some(pos) = resp.interact_pointer_pos() {
                            let new = layer_from_pointer(pos.y);
                            if new != current_layer {
                                output.new_layer = new;
                                output.layer_changed = true;
                            }
                        }
                    }

                    // Cursor feedback
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }
                    if resp.dragged() {
                        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                    }

                    // ── Paint custom slider visuals ──
                    let display_layer = if output.layer_changed {
                        output.new_layer
                    } else {
                        current_layer
                    };
                    let draw_frac = if max_layer > 0 {
                        display_layer as f32 / max_layer as f32
                    } else {
                        0.0
                    };
                    let draw_handle_y = rail_bottom - draw_frac * rail_len;

                    let painter = ui.painter();
                    let rail_round = Rounding::same(rail_thickness / 2.0);

                    // Background rail (gray, full length)
                    let rail_bg = if t.is_dark {
                        Color32::from_rgb(60, 60, 60)
                    } else {
                        Color32::from_rgb(210, 210, 210)
                    };
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(center_x - rail_thickness / 2.0, rail_top),
                            egui::pos2(center_x + rail_thickness / 2.0, rail_bottom),
                        ),
                        rail_round,
                        rail_bg,
                    );

                    // Filled portion (accent, from handle down to bottom)
                    if draw_handle_y < rail_bottom - 0.5 {
                        painter.rect_filled(
                            egui::Rect::from_min_max(
                                egui::pos2(center_x - rail_thickness / 2.0, draw_handle_y),
                                egui::pos2(center_x + rail_thickness / 2.0, rail_bottom),
                            ),
                            rail_round,
                            t.accent,
                        );
                    }

                    // Handle glow (soft accent halo)
                    let glow_alpha = if resp.hovered() || resp.dragged() {
                        60u8
                    } else {
                        35u8
                    };
                    let glow_color = Color32::from_rgba_unmultiplied(
                        t.accent.r(), t.accent.g(), t.accent.b(), glow_alpha,
                    );
                    painter.circle_filled(
                        egui::pos2(center_x, draw_handle_y),
                        handle_radius * 1.5,
                        glow_color,
                    );

                    // Handle circle (grows slightly on hover/drag)
                    let draw_radius = if resp.dragged() {
                        handle_radius + 2.0
                    } else if resp.hovered() {
                        handle_radius + 1.0
                    } else {
                        handle_radius
                    };
                    painter.circle_filled(
                        egui::pos2(center_x, draw_handle_y),
                        draw_radius,
                        t.accent,
                    );

                    // ── Layer number DragValue input (centered, always visible) ──
                    let input_y = slider_top_y + slider_h + spacing;
                    let input_rect = egui::Rect::from_center_size(
                        egui::pos2(center_x, input_y + 11.0),
                        egui::vec2(panel_inner_w, 22.0),
                    );
                    let mut jump_val = (current_layer + 1) as i64;
                    let dv = DragValue::new(&mut jump_val)
                        .clamp_range(1..=(total_layers as i64))
                        .speed(1.0)
                        .suffix(format!("/{}", total_layers));
                    let dv_response = ui.put(input_rect, dv);
                    if *focus_layer_input {
                        dv_response.request_focus();
                        *focus_layer_input = false;
                    }
                    if dv_response.changed() {
                        output.new_layer = (jump_val as usize)
                            .saturating_sub(1)
                            .min(total_layers.saturating_sub(1));
                        output.layer_changed = true;
                    }

                    // Z value below input field
                    if total_layers > 0 {
                        let z_display = global_units.length.from_mm(current_z);
                        let z_text = format!(
                            "Z = {:.3} {}",
                            z_display,
                            global_units.length.label()
                        );
                        ui.painter().text(
                            egui::pos2(center_x, input_y + 28.0),
                            egui::Align2::CENTER_TOP,
                            &z_text,
                            egui::FontId::proportional(9.0),
                            t.text_secondary,
                        );
                    }

                    // Previous layer button (-1)
                    let prev_btn_y = input_y + goto_h + spacing + btn_h / 2.0;
                    let prev_icon = LucideIcon::ChevronDown.unicode().to_string();
                    let prev_btn_rect = egui::Rect::from_center_size(
                        egui::pos2(center_x, prev_btn_y),
                        egui::vec2(panel_inner_w, btn_h),
                    );
                    let prev_btn = egui::Button::new(
                        RichText::new(&prev_icon).size(16.0).color(t.text_primary),
                    )
                    .rounding(Rounding::same(6.0))
                    .fill(Color32::TRANSPARENT);
                    if ui
                        .put(prev_btn_rect, prev_btn)
                        .on_hover_text("Previous layer (↓)")
                        .clicked()
                    {
                        output.new_layer = current_layer.saturating_sub(1);
                        output.layer_changed = true;
                    }

                    // First layer button (bottom — lowest layer number)
                    let first_icon = LucideIcon::ChevronsUp.unicode().to_string();
                    let first_btn_y = prev_btn_y + btn_h;
                    let first_btn_rect_bottom = egui::Rect::from_center_size(
                        egui::pos2(center_x, first_btn_y),
                        egui::vec2(panel_inner_w, btn_h),
                    );
                    let first_btn_bottom = egui::Button::new(
                        RichText::new(&first_icon).size(16.0).color(t.text_primary),
                    )
                    .rounding(Rounding::same(6.0))
                    .fill(Color32::TRANSPARENT);
                    if ui
                        .put(first_btn_rect_bottom, first_btn_bottom)
                        .on_hover_text("First layer (Home)")
                        .clicked()
                    {
                        output.new_layer = 0;
                        output.layer_changed = true;
                    }
                });
        });

    output
}
