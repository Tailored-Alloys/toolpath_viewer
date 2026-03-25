//! Vector Slider Component
//!
//! Right-side floating vertical vector navigation panel (left of layer slider):
//! - First/last vector buttons
//! - Custom vertical slider with draggable thumb
//! - Optional "Go to" direct input

use egui::{Color32, Context, DragValue, RichText, Rounding, Stroke};
use lucide_icons::Icon as LucideIcon;

use crate::presentation::layout::VectorSliderRegion;
use crate::presentation::theme::*;

/// Output from the vector slider component
#[derive(Debug, Clone, Default)]
pub struct VectorSliderOutput {
    pub vector_changed: bool,
    pub new_vector: usize,
    pub playing_toggled: bool,
}

/// Render the vector slider panel.
pub fn show_vector_slider(
    ctx: &Context,
    region: &VectorSliderRegion,
    current_vector: usize,
    total_vectors: usize,
    playing: bool,
    playback_speed: &mut f32,
) -> VectorSliderOutput {
    let mut output = VectorSliderOutput::default();

    if total_vectors == 0 {
        return output;
    }

    let max_vector = total_vectors.saturating_sub(1);
    let content_h = region.content_height;

    // Fixed heights for controls
    let btn_h = 28.0;
    let play_btn_h = 28.0;
    let speed_h = 20.0;
    let goto_h = 42.0;
    let spacing = 8.0;
    let controls_h = btn_h
        + btn_h
        + play_btn_h
        + speed_h
        + (if region.show_goto { goto_h } else { 0.0 })
        + spacing * 5.0;
    let slider_h = (content_h - controls_h).max(60.0);

    egui::Area::new(egui::Id::new("vector_slider_area"))
        .fixed_pos(region.pos)
        .order(egui::Order::Foreground)
        .interactable(true)
        .movable(false)
        .show(ctx, |ui| {
            floating_panel_frame()
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                    let panel_inner_w = region.width - 16.0;
                    let (content_rect, _) = ui.allocate_exact_size(
                        egui::vec2(panel_inner_w, content_h),
                        egui::Sense::hover(),
                    );

                    let center_x = content_rect.center().x;

                    // Last vector button (top — highest vector number)
                    let last_icon = LucideIcon::ChevronsDown.unicode().to_string();
                    let last_btn_rect = egui::Rect::from_center_size(
                        egui::pos2(center_x, content_rect.top() + btn_h / 2.0),
                        egui::vec2(44.0, btn_h),
                    );
                    let last_btn_top = egui::Button::new(
                        RichText::new(&last_icon).size(16.0).color(TEXT_PRIMARY),
                    )
                    .rounding(Rounding::same(6.0))
                    .fill(Color32::TRANSPARENT);
                    if ui
                        .put(last_btn_rect, last_btn_top)
                        .on_hover_text("Last vector")
                        .clicked()
                    {
                        output.new_vector = max_vector;
                        output.vector_changed = true;
                    }

                    // ── Custom vertical slider (track + thumb) ──
                    let track_top_y = content_rect.top() + btn_h + spacing;
                    let track_w = 10.0;
                    let thumb_radius = 12.0;
                    let track_rect = egui::Rect::from_min_size(
                        egui::pos2(center_x - track_w / 2.0, track_top_y),
                        egui::vec2(track_w, slider_h),
                    );

                    // Interaction area (wider than visible track for easier grabbing)
                    let interact_rect = track_rect.expand2(egui::vec2(thumb_radius, 0.0));
                    let slider_id = ui.id().with("custom_vector_slider");
                    let response =
                        ui.interact(interact_rect, slider_id, egui::Sense::click_and_drag());

                    // Compute current thumb position (inverted: top = last vector, bottom = first)
                    let usable_h = slider_h - thumb_radius * 2.0;
                    let frac = if max_vector > 0 {
                        1.0 - (current_vector as f32 / max_vector as f32)
                    } else {
                        0.0
                    };
                    let thumb_y = track_top_y + thumb_radius + frac * usable_h;

                    // Handle drag / click → update vector
                    if response.dragged() || response.clicked() {
                        if let Some(pos) = response.interact_pointer_pos() {
                            let clamped_y = pos.y.clamp(
                                track_top_y + thumb_radius,
                                track_top_y + thumb_radius + usable_h,
                            );
                            let new_frac = (clamped_y - track_top_y - thumb_radius) / usable_h;
                            let target = ((1.0 - new_frac) * max_vector as f32).round() as usize;
                            output.new_vector = target.min(max_vector);
                            output.vector_changed = true;
                        }
                    }

                    let painter = ui.painter();

                    // Draw track background (full rail)
                    painter.rect_filled(
                        track_rect,
                        Rounding::same(track_w / 2.0),
                        Color32::from_rgb(220, 220, 220),
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
                            Color32::from_rgb(255, 167, 38), // Orange accent to distinguish from layer slider
                        );
                    }

                    // Draw thumb circle
                    let thumb_center = egui::pos2(center_x, thumb_y);
                    let is_active = response.dragged();
                    let is_hovered = response.hovered();
                    let thumb_color = if is_active {
                        Color32::from_rgb(245, 124, 0) // Deep orange
                    } else if is_hovered {
                        Color32::from_rgb(255, 167, 38) // Orange
                    } else {
                        Color32::WHITE
                    };
                    let thumb_stroke_color = if is_active || is_hovered {
                        Color32::from_rgb(245, 124, 0)
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

                    // Vector number next to thumb
                    let vec_label = format!("{}", current_vector + 1);
                    painter.text(
                        egui::pos2(center_x + thumb_radius + 4.0, thumb_y),
                        egui::Align2::LEFT_CENTER,
                        &vec_label,
                        egui::FontId::proportional(9.0),
                        TEXT_SECONDARY,
                    );

                    // First vector button (bottom — lowest vector number)
                    let first_icon = LucideIcon::ChevronsUp.unicode().to_string();
                    let first_btn_y = track_top_y + slider_h + spacing + btn_h / 2.0;
                    let first_btn_rect_bottom = egui::Rect::from_center_size(
                        egui::pos2(center_x, first_btn_y),
                        egui::vec2(44.0, btn_h),
                    );
                    let first_btn_bottom = egui::Button::new(
                        RichText::new(&first_icon).size(16.0).color(TEXT_PRIMARY),
                    )
                    .rounding(Rounding::same(6.0))
                    .fill(Color32::TRANSPARENT);
                    if ui
                        .put(first_btn_rect_bottom, first_btn_bottom)
                        .on_hover_text("First vector")
                        .clicked()
                    {
                        output.new_vector = 0;
                        output.vector_changed = true;
                    }

                    // Play/Pause button
                    let play_btn_y = first_btn_y + btn_h / 2.0 + spacing + play_btn_h / 2.0;
                    let play_icon = if playing {
                        LucideIcon::Pause.unicode().to_string()
                    } else {
                        LucideIcon::Play.unicode().to_string()
                    };
                    let play_btn_color = if playing {
                        Color32::from_rgb(245, 124, 0) // Deep orange when playing
                    } else {
                        Color32::from_rgb(255, 167, 38) // Orange
                    };
                    let play_btn_rect = egui::Rect::from_center_size(
                        egui::pos2(center_x, play_btn_y),
                        egui::vec2(44.0, play_btn_h),
                    );
                    let play_btn = egui::Button::new(
                        RichText::new(&play_icon).size(16.0).color(Color32::WHITE),
                    )
                    .rounding(Rounding::same(6.0))
                    .fill(play_btn_color);
                    let play_tooltip = if playing { "Pause (Space)" } else { "Play (Space)" };
                    if ui
                        .put(play_btn_rect, play_btn)
                        .on_hover_text(play_tooltip)
                        .clicked()
                    {
                        output.playing_toggled = true;
                    }

                    // Speed selector
                    let speed_y = play_btn_y + play_btn_h / 2.0 + spacing + speed_h / 2.0;
                    let speed_opts: [f32; 4] = [0.5, 1.0, 2.0, 5.0];
                    let speed_label = if *playback_speed == 0.5 {
                        "½×".to_string()
                    } else {
                        format!("{}×", *playback_speed as u32)
                    };
                    let speed_btn_rect = egui::Rect::from_center_size(
                        egui::pos2(center_x, speed_y),
                        egui::vec2(44.0, speed_h),
                    );
                    let speed_btn = egui::Button::new(
                        RichText::new(&speed_label).size(11.0).color(TEXT_PRIMARY),
                    )
                    .rounding(Rounding::same(4.0))
                    .fill(Color32::from_rgb(240, 240, 240));
                    if ui
                        .put(speed_btn_rect, speed_btn)
                        .on_hover_text("Playback speed")
                        .clicked()
                    {
                        // Cycle to next speed
                        let current_idx = speed_opts.iter()
                            .position(|&s| (s - *playback_speed).abs() < 0.01)
                            .unwrap_or(1);
                        *playback_speed = speed_opts[(current_idx + 1) % speed_opts.len()];
                    }

                    // Go to section (hidden on small windows)
                    if region.show_goto {
                        let goto_label_y = speed_y + speed_h / 2.0 + spacing + 8.0;
                        ui.painter().text(
                            egui::pos2(center_x, goto_label_y),
                            egui::Align2::CENTER_CENTER,
                            "Go to",
                            egui::FontId::proportional(10.0),
                            TEXT_SECONDARY,
                        );

                        let jump_rect = egui::Rect::from_center_size(
                            egui::pos2(center_x, goto_label_y + 18.0),
                            egui::vec2(56.0, 22.0),
                        );
                        let mut jump_val = (current_vector + 1) as i64;
                        let dv = DragValue::new(&mut jump_val)
                            .clamp_range(1..=(total_vectors as i64))
                            .speed(1.0);
                        if ui.put(jump_rect, dv).changed() {
                            output.new_vector = (jump_val as usize)
                                .saturating_sub(1)
                                .min(total_vectors.saturating_sub(1));
                            output.vector_changed = true;
                        }
                    }
                });
        });

    output
}
