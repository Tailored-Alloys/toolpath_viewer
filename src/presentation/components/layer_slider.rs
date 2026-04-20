//! Layer Slider Component
//!
//! Right-side floating vertical layer navigation panel with:
//! - First/last layer buttons
//! - Custom vertical slider with draggable thumb
//! - Optional "Go to" direct input

use egui::{Color32, Context, DragValue, RichText, Rounding, Stroke};
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
    let btn_h = 28.0;
    let goto_h = 42.0;
    let spacing = 8.0;
    let controls_h = btn_h * 4.0
        + (if region.show_goto { goto_h } else { 0.0 })
        + spacing * 3.0;
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
                        egui::vec2(44.0, btn_h),
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
                        egui::vec2(44.0, btn_h),
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

                    // ── Custom vertical slider (track + thumb) ──
                    let track_top_y = content_rect.top() + btn_h * 2.0 + spacing;
                    let track_w = 10.0;
                    let thumb_radius = 12.0;
                    let track_rect = egui::Rect::from_min_size(
                        egui::pos2(center_x - track_w / 2.0, track_top_y),
                        egui::vec2(track_w, slider_h),
                    );

                    // Interaction area (wider than visible track for easier grabbing)
                    let interact_rect = track_rect.expand2(egui::vec2(thumb_radius, 0.0));
                    let slider_id = ui.id().with("custom_layer_slider");
                    let response =
                        ui.interact(interact_rect, slider_id, egui::Sense::click_and_drag());

                    // Compute current thumb position (inverted: top = last layer, bottom = first)
                    let usable_h = slider_h - thumb_radius * 2.0;
                    let frac = if max_layer > 0 {
                        1.0 - (current_layer as f32 / max_layer as f32)
                    } else {
                        0.0
                    };
                    let thumb_y = track_top_y + thumb_radius + frac * usable_h;

                    // Handle drag / click → update layer
                    if response.dragged() || response.clicked() {
                        if let Some(pos) = response.interact_pointer_pos() {
                            let clamped_y = pos.y.clamp(
                                track_top_y + thumb_radius,
                                track_top_y + thumb_radius + usable_h,
                            );
                            let new_frac = (clamped_y - track_top_y - thumb_radius) / usable_h;
                            let target = ((1.0 - new_frac) * max_layer as f32).round() as usize;
                            output.new_layer = target.min(max_layer);
                            output.layer_changed = true;
                        }
                    }

                    let painter = ui.painter();

                    // Draw track background (full rail)
                    painter.rect_filled(
                        track_rect,
                        Rounding::same(track_w / 2.0),
                        if t.is_dark { Color32::from_rgb(60, 60, 60) } else { Color32::from_rgb(220, 220, 220) },
                    );

                    // Draw filled portion (top to thumb = progress)
                    if thumb_y > track_top_y + 1.0 {
                        let filled_rect = egui::Rect::from_min_max(
                            track_rect.left_top(),
                            egui::pos2(track_rect.right(), thumb_y),
                        );
                        painter.rect_filled(
                            filled_rect,
                            Rounding::same(track_w / 2.0),
                            t.accent_light,
                        );
                    }

                    // Draw thumb circle
                    let thumb_center = egui::pos2(center_x, thumb_y);
                    let is_active = response.dragged();
                    let is_hovered = response.hovered();
                    let thumb_color = if is_active {
                        t.accent
                    } else if is_hovered {
                        t.accent_light
                    } else if t.is_dark {
                        Color32::from_rgb(180, 180, 180)
                    } else {
                        Color32::WHITE
                    };
                    let thumb_stroke_color = if is_active || is_hovered {
                        t.accent
                    } else if t.is_dark {
                        Color32::from_rgb(100, 100, 100)
                    } else {
                        Color32::from_rgb(160, 160, 160)
                    };

                    // Shadow
                    painter.circle_filled(
                        thumb_center + egui::vec2(0.0, 1.0),
                        thumb_radius + 1.0,
                        Color32::from_rgba_premultiplied(0, 0, 0, 25),
                    );
                    // Fill
                    painter.circle_filled(thumb_center, thumb_radius, thumb_color);
                    // Border
                    painter.circle_stroke(
                        thumb_center,
                        thumb_radius,
                        Stroke::new(1.5, thumb_stroke_color),
                    );

                    // Layer number next to thumb
                    let layer_label = format!("{}", current_layer + 1);
                    painter.text(
                        egui::pos2(center_x + thumb_radius + 4.0, thumb_y),
                        egui::Align2::LEFT_CENTER,
                        &layer_label,
                        egui::FontId::proportional(9.0),
                        t.text_secondary,
                    );

                    // Previous layer button (-1)
                    let prev_icon = LucideIcon::ChevronDown.unicode().to_string();
                    let prev_btn_y = track_top_y + slider_h + spacing + btn_h / 2.0;
                    let prev_btn_rect = egui::Rect::from_center_size(
                        egui::pos2(center_x, prev_btn_y),
                        egui::vec2(44.0, btn_h),
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
                        egui::vec2(44.0, btn_h),
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

                    // Go to section (hidden on small windows)
                    if region.show_goto {
                        let goto_y = first_btn_y + btn_h / 2.0 + spacing;

                        // Input field showing "current/total" with editable current
                        let input_rect = egui::Rect::from_center_size(
                            egui::pos2(center_x, goto_y + 11.0),
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
                                egui::pos2(center_x, goto_y + 28.0),
                                egui::Align2::CENTER_TOP,
                                &z_text,
                                egui::FontId::proportional(9.0),
                                t.text_secondary,
                            );
                        }
                    }
                });
        });

    output
}
