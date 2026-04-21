//! Vector Player Component
//!
//! Floating video-player-style bar centered at the bottom of the viewport:
//! |◀ First| ▶ Play/Pause | Last ▶| [DragValue input] | 1x Speed |

use egui::{Color32, Context, DragValue, RichText, Rounding, Slider, Stroke, Vec2};
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

                // ── Horizontal egui Slider (themed) ──
                {
                    let available_w = ui.available_width() - 100.0; // reserve space for input + speed
                    let slider_w = available_w.max(80.0);
                    let mut slider_val = (current_vector + 1) as i32;
                    let slider_widget = Slider::new(&mut slider_val, 1..=(total_vectors as i32))
                        .show_value(false);

                    let slider_resp = ui.scope(|ui| {
                        let rail_color = if t.is_dark {
                            Color32::from_rgb(60, 60, 60)
                        } else {
                            Color32::from_rgb(210, 210, 210)
                        };
                        let accent = Color32::from_rgb(255, 167, 38); // orange accent for vector player
                        let accent_active = Color32::from_rgb(245, 124, 0);
                        let handle_radius = 7.0;

                        // Inactive: rail appearance
                        ui.style_mut().visuals.widgets.inactive.bg_fill = rail_color;
                        ui.style_mut().visuals.widgets.inactive.fg_stroke = Stroke::new(0.0, Color32::TRANSPARENT);
                        ui.style_mut().visuals.widgets.inactive.rounding = Rounding::same(3.0);
                        ui.style_mut().visuals.widgets.inactive.expansion = 0.0;
                        // Hovered
                        ui.style_mut().visuals.widgets.hovered.bg_fill = accent;
                        ui.style_mut().visuals.widgets.hovered.fg_stroke = Stroke::new(2.0, accent);
                        ui.style_mut().visuals.widgets.hovered.rounding = Rounding::same(handle_radius);
                        ui.style_mut().visuals.widgets.hovered.expansion = 2.0;
                        // Active (dragging)
                        ui.style_mut().visuals.widgets.active.bg_fill = accent_active;
                        ui.style_mut().visuals.widgets.active.fg_stroke = Stroke::new(2.0, accent_active);
                        ui.style_mut().visuals.widgets.active.rounding = Rounding::same(handle_radius);
                        ui.style_mut().visuals.widgets.active.expansion = 2.0;
                        // Slider rail sizing
                        ui.style_mut().spacing.slider_rail_height = 4.0;

                        ui.add_sized(egui::vec2(slider_w, 26.0), slider_widget)
                    }).inner;

                    if slider_resp.changed() {
                        let target = (slider_val as usize)
                            .saturating_sub(1)
                            .min(max_vector);
                        output.new_vector = target;
                        output.vector_changed = true;
                    }
                }

                ui.add_space(4.0);

                // ── Vector DragValue input ──
                let mut jump_val = (current_vector + 1) as i64;
                let dv = DragValue::new(&mut jump_val)
                    .clamp_range(1..=(total_vectors as i64))
                    .speed(1.0)
                    .suffix(format!("/{}", total_vectors));
                if ui.add(dv).changed() {
                    let target = (jump_val as usize)
                        .saturating_sub(1)
                        .min(max_vector);
                    output.new_vector = target;
                    output.vector_changed = true;
                }

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
