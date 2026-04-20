//! Vector Player Component
//!
//! Floating video-player-style bar centered at the bottom of the viewport:
//! |◀ First| ▶ Play/Pause | Last ▶| [═══ slider ═══] | 12/450 | 1x Speed |

use egui::{Color32, Context, RichText, Rounding, Stroke, Vec2};
use lucide_icons::Icon as LucideIcon;

use crate::presentation::layout::VectorPlayerRegion;
use crate::presentation::theme;

/// Output from the vector player component
#[derive(Debug, Clone, Default)]
pub struct VectorSliderOutput {
    pub vector_changed: bool,
    pub new_vector: usize,
    pub playing_toggled: bool,
}

/// Render the horizontal vector player bar.
pub fn show_vector_player(
    ctx: &Context,
    region: &VectorPlayerRegion,
    current_vector: usize,
    total_vectors: usize,
    playing: bool,
    playback_speed: &mut f32,
) -> VectorSliderOutput {
    let mut output = VectorSliderOutput::default();

    if total_vectors == 0 {
        return output;
    }

    let t = theme::active();
    let max_vector = total_vectors.saturating_sub(1);

    egui::Area::new(egui::Id::new("vector_player_floating"))
        .fixed_pos(region.anchor_pos)
        .order(egui::Order::Foreground)
        .interactable(true)
        .show(ctx, |ui| {
            let frame = egui::Frame::none()
                .fill(t.toolbar_bg)
                .stroke(Stroke::new(1.0, t.toolbar_border))
                .rounding(Rounding::same(8.0))
                .inner_margin(egui::Margin::symmetric(12.0, 6.0))
                .shadow(egui::epaint::Shadow {
                    offset: egui::vec2(0.0, 2.0),
                    blur: 8.0,
                    spread: 0.0,
                    color: Color32::from_black_alpha(40),
                });
            frame.show(ui, |ui| {
                ui.set_width(region.width - 24.0); // account for inner_margin
                ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let btn_size = Vec2::new(26.0, 26.0);
                let icon_sz = 14.0;

                // ── First vector button ──
                let first_icon = LucideIcon::SkipBack.unicode().to_string();
                let first_btn = egui::Button::new(
                    RichText::new(&first_icon).size(icon_sz).color(t.text_primary),
                )
                .fill(Color32::TRANSPARENT)
                .rounding(Rounding::same(4.0))
                .min_size(btn_size);
                if ui.add(first_btn).on_hover_text("First vector").clicked() {
                    output.new_vector = 0;
                    output.vector_changed = true;
                }

                // ── Play/Pause button ──
                let play_icon = if playing {
                    LucideIcon::Pause.unicode().to_string()
                } else {
                    LucideIcon::Play.unicode().to_string()
                };
                let play_btn_color = if playing {
                    Color32::from_rgb(245, 124, 0)
                } else {
                    Color32::from_rgb(255, 167, 38)
                };
                let play_btn = egui::Button::new(
                    RichText::new(&play_icon).size(icon_sz).color(Color32::WHITE),
                )
                .fill(play_btn_color)
                .rounding(Rounding::same(4.0))
                .min_size(btn_size);
                let play_tooltip = if playing { "Pause (Space)" } else { "Play (Space)" };
                if ui.add(play_btn).on_hover_text(play_tooltip).clicked() {
                    output.playing_toggled = true;
                }

                // ── Last vector button ──
                let last_icon = LucideIcon::SkipForward.unicode().to_string();
                let last_btn = egui::Button::new(
                    RichText::new(&last_icon).size(icon_sz).color(t.text_primary),
                )
                .fill(Color32::TRANSPARENT)
                .rounding(Rounding::same(4.0))
                .min_size(btn_size);
                if ui.add(last_btn).on_hover_text("Last vector").clicked() {
                    output.new_vector = max_vector;
                    output.vector_changed = true;
                }

                ui.add_space(4.0);

                // ── Horizontal slider (track + thumb) ──
                let available_w = ui.available_width() - 120.0; // reserve space for counter + speed
                let track_h = 6.0;
                let thumb_radius = 8.0;
                let slider_w = available_w.max(80.0);

                let (slider_rect, slider_resp) = ui.allocate_exact_size(
                    egui::vec2(slider_w, 26.0),
                    egui::Sense::click_and_drag(),
                );

                let track_rect = egui::Rect::from_center_size(
                    slider_rect.center(),
                    egui::vec2(slider_w - thumb_radius * 2.0, track_h),
                );

                // Compute thumb position
                let usable_w = track_rect.width();
                let frac = if max_vector > 0 {
                    current_vector as f32 / max_vector as f32
                } else {
                    0.0
                };
                let thumb_x = track_rect.left() + frac * usable_w;
                let thumb_center_y = track_rect.center().y;

                // Handle drag / click
                if slider_resp.dragged() || slider_resp.clicked() {
                    if let Some(pos) = slider_resp.interact_pointer_pos() {
                        let clamped_x = pos.x.clamp(track_rect.left(), track_rect.right());
                        let new_frac = (clamped_x - track_rect.left()) / usable_w;
                        let target = (new_frac * max_vector as f32).round() as usize;
                        output.new_vector = target.min(max_vector);
                        output.vector_changed = true;
                    }
                }

                let painter = ui.painter();

                // Track background
                painter.rect_filled(
                    track_rect,
                    Rounding::same(track_h / 2.0),
                    if t.is_dark { Color32::from_rgb(60, 60, 60) } else { Color32::from_rgb(220, 220, 220) },
                );

                // Filled portion (left to thumb)
                if thumb_x > track_rect.left() + 1.0 {
                    let filled = egui::Rect::from_min_max(
                        track_rect.left_top(),
                        egui::pos2(thumb_x, track_rect.bottom()),
                    );
                    painter.rect_filled(
                        filled,
                        Rounding::same(track_h / 2.0),
                        Color32::from_rgb(255, 167, 38),
                    );
                }

                // Thumb
                let is_active = slider_resp.dragged();
                let is_hovered = slider_resp.hovered();
                let thumb_color = if is_active {
                    Color32::from_rgb(245, 124, 0)
                } else if is_hovered {
                    Color32::from_rgb(255, 167, 38)
                } else if t.is_dark {
                    Color32::from_rgb(180, 180, 180)
                } else {
                    Color32::WHITE
                };
                let thumb_stroke_color = if is_active || is_hovered {
                    Color32::from_rgb(245, 124, 0)
                } else if t.is_dark {
                    Color32::from_rgb(100, 100, 100)
                } else {
                    Color32::from_rgb(160, 160, 160)
                };

                painter.circle_filled(egui::pos2(thumb_x, thumb_center_y), thumb_radius, thumb_color);
                painter.circle_stroke(
                    egui::pos2(thumb_x, thumb_center_y),
                    thumb_radius,
                    Stroke::new(1.5, thumb_stroke_color),
                );

                ui.add_space(8.0);

                // ── Vector counter ──
                ui.label(
                    RichText::new(format!("{}/{}", current_vector + 1, total_vectors))
                        .size(11.0)
                        .color(t.text_primary),
                );

                ui.add_space(4.0);

                // ── Speed selector ──
                let speed_opts: [f32; 4] = [0.5, 1.0, 2.0, 5.0];
                let speed_label = if *playback_speed == 0.5 {
                    "½×".to_string()
                } else {
                    format!("{}×", *playback_speed as u32)
                };
                let speed_btn = egui::Button::new(
                    RichText::new(&speed_label).size(10.0).color(t.text_primary),
                )
                .rounding(Rounding::same(4.0))
                .fill(if t.is_dark { Color32::from_rgb(50, 50, 50) } else { Color32::from_rgb(240, 240, 240) })
                .min_size(Vec2::new(32.0, 22.0));
                if ui.add(speed_btn).on_hover_text("Playback speed").clicked() {
                    let current_idx = speed_opts.iter()
                        .position(|&s| (s - *playback_speed).abs() < 0.01)
                        .unwrap_or(1);
                    *playback_speed = speed_opts[(current_idx + 1) % speed_opts.len()];
                }
            });
            });
        });

    output
}
