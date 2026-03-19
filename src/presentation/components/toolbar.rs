//! Toolbar Component
//!
//! Top toolbar with: load button, visibility toggles, parameter mode selector,
//! file info / controls toggles, and right-aligned layer info.

use egui::{Color32, Context, RichText, Rounding, Stroke, Vec2};
use lucide_icons::Icon as LucideIcon;

use crate::application::ports::{GlobalUnits, GridUnit, ParameterMode, PowerUnit, TimeUnit};
use crate::presentation::layout::TOOLBAR_HEIGHT;
use crate::presentation::theme::*;

/// Output from the toolbar component
#[derive(Debug, Clone, Default)]
pub struct ToolbarOutput {
    pub open_file_requested: bool,
}

/// Render the top toolbar panel.
///
/// Mutates visibility state in-place; returns action requests.
pub fn show_toolbar(
    ctx: &Context,
    compact: bool,
    // Mutable visibility state
    show_contours: &mut bool,
    show_hatches: &mut bool,
    show_arrows: &mut bool,
    show_wait_markers: &mut bool,
    show_scale_bar: &mut bool,
    show_vector_view: &mut bool,
    param_mode: &mut Option<ParameterMode>,
    show_file_info: &mut bool,
    show_controls: &mut bool,
    global_units: &mut GlobalUnits,
    // Read-only display state
    current_layer: usize,
    total_layers: usize,
    current_z: f32,
) -> ToolbarOutput {
    let mut output = ToolbarOutput::default();

    egui::TopBottomPanel::top("toolbar")
        .exact_height(TOOLBAR_HEIGHT)
        .frame(
            egui::Frame::none()
                .fill(TOOLBAR_BG)
                .stroke(Stroke::new(1.0, TOOLBAR_BORDER))
                .inner_margin(egui::Margin::symmetric(12.0, 8.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                // ── Load File button ──
                let icon_folder = LucideIcon::FolderOpen.unicode();
                let load_label = if compact {
                    format!("{}", icon_folder)
                } else {
                    format!("{} Load", icon_folder)
                };
                let load_btn =
                    egui::Button::new(RichText::new(load_label).size(13.0).color(TEXT_PRIMARY))
                        .fill(Color32::TRANSPARENT)
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(0.0, 30.0));
                if ui
                    .add(load_btn)
                    .on_hover_text("Load file (Ctrl+O)")
                    .clicked()
                {
                    output.open_file_requested = true;
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // ── Visibility toggles (using Lucide icons) ──
                let icon_contour = &LucideIcon::Spline.unicode().to_string();
                let icon_hatch = &LucideIcon::ScanLine.unicode().to_string();
                let icon_arrow = &LucideIcon::Navigation2.unicode().to_string();
                let icon_clock = &LucideIcon::Clock.unicode().to_string();
                toolbar_toggle(ui, icon_contour, show_contours, "Toggle Contours (C)");
                toolbar_toggle(ui, icon_hatch, show_hatches, "Toggle Hatches (H)");
                toolbar_toggle(ui, icon_arrow, show_arrows, "Toggle Direction Arrows (A)");
                toolbar_toggle(
                    ui,
                    icon_clock,
                    show_wait_markers,
                    "Toggle Wait Time Markers (T)",
                );
                let icon_scale = &LucideIcon::RulerDimensionLine.unicode().to_string();
                toolbar_toggle(ui, icon_scale, show_scale_bar, "Toggle Scale Bar (V)");
                toolbar_toggle_image(
                    ui,
                    egui::ImageSource::Bytes {
                        uri: "bytes://hatch-time.svg".into(),
                        bytes: egui::load::Bytes::Static(include_bytes!("../assets/icons/hatch-time.svg")),
                    },
                    show_vector_view,
                    "Toggle Vector View (N)",
                );

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // ── Parameter mode selector ──
                if !compact {
                    ui.label(RichText::new("Color:").size(12.0).color(TEXT_SECONDARY));
                }
                let modes = [
                    (None, "None", "No parameter coloring (1)"),
                    (Some(ParameterMode::Power), "Power", "Color by laser power (2)"),
                    (Some(ParameterMode::Speed), "Speed", "Color by scan speed (3)"),
                    (Some(ParameterMode::WaitTime), "Wait", "Color by wait time (4)"),
                ];
                for (mode, label, tip) in &modes {
                    if toolbar_mode_btn(ui, label, *param_mode == *mode, tip) {
                        *param_mode = *mode;
                    }
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // ── Global Units selector (dropdowns) ──
                if !compact {
                    ui.label(RichText::new("Units:").size(12.0).color(TEXT_SECONDARY));
                }

                // Distance dropdown
                let dist_label = format!("Distance: {}", global_units.length.label());
                egui::ComboBox::from_id_source("unit_distance")
                    .selected_text(RichText::new(global_units.length.label()).size(11.0))
                    .width(44.0)
                    .show_ui(ui, |ui| {
                        ui.label(RichText::new("Distance").size(11.0).strong().color(TEXT_PRIMARY));
                        let length_opts: [(GridUnit, &str); 3] = [
                            (GridUnit::Millimeters, "mm"),
                            (GridUnit::Micrometers, "µm"),
                            (GridUnit::Inches, "in"),
                        ];
                        for (unit, label) in length_opts {
                            ui.selectable_value(&mut global_units.length, unit, label);
                        }
                    })
                    .response
                    .on_hover_text(dist_label);

                ui.add_space(2.0);

                // Time dropdown
                let time_label = format!("Time: {}", global_units.time.label());
                egui::ComboBox::from_id_source("unit_time")
                    .selected_text(RichText::new(global_units.time.label()).size(11.0))
                    .width(36.0)
                    .show_ui(ui, |ui| {
                        ui.label(RichText::new("Time").size(11.0).strong().color(TEXT_PRIMARY));
                        let time_opts: [(TimeUnit, &str); 3] = [
                            (TimeUnit::Microseconds, "µs"),
                            (TimeUnit::Milliseconds, "ms"),
                            (TimeUnit::Seconds, "s"),
                        ];
                        for (unit, label) in time_opts {
                            ui.selectable_value(&mut global_units.time, unit, label);
                        }
                    })
                    .response
                    .on_hover_text(time_label);

                ui.add_space(2.0);

                // Power dropdown
                let power_label = format!("Power: {}", global_units.power.label());
                egui::ComboBox::from_id_source("unit_power")
                    .selected_text(RichText::new(global_units.power.label()).size(11.0))
                    .width(36.0)
                    .show_ui(ui, |ui| {
                        ui.label(RichText::new("Power").size(11.0).strong().color(TEXT_PRIMARY));
                        let power_opts: [(PowerUnit, &str); 2] = [
                            (PowerUnit::Watts, "W"),
                            (PowerUnit::Kilowatts, "kW"),
                        ];
                        for (unit, label) in power_opts {
                            ui.selectable_value(&mut global_units.power, unit, label);
                        }
                    })
                    .response
                    .on_hover_text(power_label);

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // ── File Info button ──
                let icon_info = LucideIcon::FileText.unicode();
                let info_label = if compact {
                    format!("{}", icon_info)
                } else {
                    format!("{} Info", icon_info)
                };
                let info_btn = egui::Button::new(
                    RichText::new(info_label).size(13.0).color(TEXT_PRIMARY),
                )
                .fill(if *show_file_info {
                    TOGGLE_ACTIVE_BG
                } else {
                    Color32::TRANSPARENT
                })
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(0.0, 30.0));
                if ui
                    .add(info_btn)
                    .on_hover_text("Layer info & parameters (I)")
                    .clicked()
                {
                    *show_file_info = !*show_file_info;
                }

                // ── Controls button ──
                let icon_kbd = LucideIcon::Keyboard.unicode();
                let ctrl_label = if compact {
                    format!("{}", icon_kbd)
                } else {
                    format!("{} Controls", icon_kbd)
                };
                let ctrl_btn = egui::Button::new(
                    RichText::new(ctrl_label).size(13.0).color(TEXT_PRIMARY),
                )
                .fill(if *show_controls {
                    TOGGLE_ACTIVE_BG
                } else {
                    Color32::TRANSPARENT
                })
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(0.0, 30.0));
                if ui
                    .add(ctrl_btn)
                    .on_hover_text("Keyboard shortcuts (F1)")
                    .clicked()
                {
                    *show_controls = !*show_controls;
                }

                // ── Layer info on the right side ──
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        if total_layers > 0 {
                            let z_display = global_units.length.from_mm(current_z);
                            ui.label(
                                RichText::new(format!(
                                    "Layer {}/{}  •  Z = {:.3} {}",
                                    current_layer + 1,
                                    total_layers,
                                    z_display,
                                    global_units.length.label()
                                ))
                                .size(12.0)
                                .color(TEXT_SECONDARY),
                            );
                        }
                    },
                );
            });
        });

    output
}
