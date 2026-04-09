//! Status Bar Component
//!
//! Bottom bar always visible, showing:
//! - Left: zoom level, current layer/Z, mouse world-coordinates
//! - Right: unit dropdowns (distance, time, power)

use egui::{Context, RichText, Rounding, Stroke};

use crate::application::ports::{GlobalUnits, GridUnit, PowerUnit, TimeUnit};
use crate::presentation::layout::StatusBarRegion;
use crate::presentation::theme;

/// Hover coordinate info for the status bar
#[derive(Debug, Clone, Default)]
pub struct StatusBarInfo {
    /// Current zoom level
    pub zoom: f32,
    /// Current layer index (0-based)
    pub current_layer: usize,
    /// Total layers
    pub total_layers: usize,
    /// Current Z height in mm
    pub current_z: f32,
    /// Mouse world X (None if not hovering viewport)
    pub mouse_world_x: Option<f32>,
    /// Mouse world Y (None if not hovering viewport)
    pub mouse_world_y: Option<f32>,
    /// Hover power value (W)
    pub hover_power: Option<f32>,
    /// Hover speed value (mm/s)
    pub hover_speed: Option<f32>,
    /// Hover wait time (µs)
    pub hover_wait: Option<f32>,
}

/// Render the status bar.
pub fn show_status_bar(
    ctx: &Context,
    _region: &StatusBarRegion,
    info: &StatusBarInfo,
    global_units: &mut GlobalUnits,
) {
    let t = theme::active();

    egui::TopBottomPanel::bottom("status_bar")
        .exact_height(24.0)
        .frame(
            egui::Frame::none()
                .fill(t.toolbar_bg)
                .stroke(Stroke::new(1.0, t.toolbar_border))
                .inner_margin(egui::Margin::symmetric(8.0, 0.0)),
        )
        .show(ctx, |ui| {
            // Shrink widget sizes to fit the compact bar
            ui.style_mut().spacing.interact_size.y = 16.0;
            ui.style_mut().spacing.combo_height = 16.0;
            ui.style_mut().spacing.button_padding = egui::vec2(4.0, 1.0);
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;

                // ── Left: zoom + layer + coords ──

                // Zoom level
                ui.label(
                    RichText::new(format!("Zoom: {:.1}x", info.zoom))
                        .size(10.0)
                        .color(t.text_secondary),
                );

                ui.separator();

                // Layer info
                if info.total_layers > 0 {
                    let z_display = global_units.length.from_mm(info.current_z);
                    ui.label(
                        RichText::new(format!(
                            "Layer {}/{} · Z = {:.3} {}",
                            info.current_layer + 1,
                            info.total_layers,
                            z_display,
                            global_units.length.label(),
                        ))
                        .size(10.0)
                        .color(t.text_primary),
                    );

                    ui.separator();
                }

                // Mouse world coordinates
                if let (Some(wx), Some(wy)) = (info.mouse_world_x, info.mouse_world_y) {
                    let dx = global_units.length.from_mm(wx);
                    let dy = global_units.length.from_mm(wy);
                    ui.label(
                        RichText::new(format!(
                            "X: {:.2}  Y: {:.2} {}",
                            dx, dy,
                            global_units.length.label(),
                        ))
                        .size(10.0)
                        .color(t.text_secondary),
                    );
                }

                // Hover parameter values
                if info.hover_power.is_some() || info.hover_speed.is_some() || info.hover_wait.is_some() {
                    ui.separator();
                    if let Some(p) = info.hover_power {
                        let pv = global_units.power.from_watts(p);
                        ui.label(
                            RichText::new(format!("P: {:.1} {}", pv, global_units.power.label()))
                                .size(10.0).color(t.text_secondary),
                        );
                    }
                    if let Some(s) = info.hover_speed {
                        let sv = global_units.length.from_mm(s);
                        ui.label(
                            RichText::new(format!("V: {:.1} {}/s", sv, global_units.length.label()))
                                .size(10.0).color(t.text_secondary),
                        );
                    }
                    if let Some(w) = info.hover_wait {
                        let wv = global_units.time.from_us(w);
                        ui.label(
                            RichText::new(format!("W: {:.1} {}", wv, global_units.time.label()))
                                .size(10.0).color(t.text_secondary),
                        );
                    }
                }

                // ── Right: unit dropdowns ──
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;

                    // Power dropdown
                    egui::ComboBox::from_id_source("sb_unit_power")
                        .selected_text(RichText::new(global_units.power.label()).size(10.0))
                        .width(36.0)
                        .show_ui(ui, |ui| {
                            let opts: [(PowerUnit, &str); 2] = [
                                (PowerUnit::Watts, "W"),
                                (PowerUnit::Kilowatts, "kW"),
                            ];
                            for (unit, label) in opts {
                                ui.selectable_value(&mut global_units.power, unit, label);
                            }
                        });

                    // Time dropdown
                    egui::ComboBox::from_id_source("sb_unit_time")
                        .selected_text(RichText::new(global_units.time.label()).size(10.0))
                        .width(36.0)
                        .show_ui(ui, |ui| {
                            let opts: [(TimeUnit, &str); 3] = [
                                (TimeUnit::Microseconds, "µs"),
                                (TimeUnit::Milliseconds, "ms"),
                                (TimeUnit::Seconds, "s"),
                            ];
                            for (unit, label) in opts {
                                ui.selectable_value(&mut global_units.time, unit, label);
                            }
                        });

                    // Distance dropdown
                    egui::ComboBox::from_id_source("sb_unit_distance")
                        .selected_text(RichText::new(global_units.length.label()).size(10.0))
                        .width(44.0)
                        .show_ui(ui, |ui| {
                            let opts: [(GridUnit, &str); 3] = [
                                (GridUnit::Millimeters, "mm"),
                                (GridUnit::Micrometers, "µm"),
                                (GridUnit::Inches, "in"),
                            ];
                            for (unit, label) in opts {
                                ui.selectable_value(&mut global_units.length, unit, label);
                            }
                        });

                    ui.label(RichText::new("Units:").size(10.0).color(t.text_secondary));
                });
            });
        });
}
