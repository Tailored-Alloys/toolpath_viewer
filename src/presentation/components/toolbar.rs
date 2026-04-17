//! Toolbar Component
//!
//! Top toolbar with grouped controls (CAD naming conventions):
//! - Display: visibility toggles (Contours, Hatches, Arrows, Dwell, Animate)
//! - Color By: Type, Power, Speed, Part
//! - Right: Info, Shortcuts, Preferences

use egui::{Color32, Context, RichText, Rounding, Stroke, Vec2};
use lucide_icons::Icon as LucideIcon;

use crate::application::ports::{ColorMode, ParameterMode};
use crate::presentation::layout::TOOLBAR_HEIGHT;
use crate::presentation::theme::{self, toolbar_toggle, toolbar_toggle_image, toolbar_mode_btn};

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
    show_infills: &mut bool,
    show_arrows: &mut bool,
    show_wait_markers: &mut bool,
    show_vector_view: &mut bool,
    param_mode: &mut Option<ParameterMode>,
    color_mode: &mut ColorMode,
    show_file_info: &mut bool,
    show_controls: &mut bool,
    show_preferences: &mut bool,
    has_multiple_files: bool,
    has_tab_bar: bool,
) -> ToolbarOutput {
    let output = ToolbarOutput::default();
    let t = theme::active();

    egui::TopBottomPanel::top("toolbar")
        .exact_height(TOOLBAR_HEIGHT)
        .show_separator_line(false)
        .frame(
            egui::Frame::none()
                .fill(t.toolbar_bg)
                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                .stroke(Stroke::NONE),
        )
        .show(ctx, |ui| {
            // Draw bottom border at the full panel edge
            {
                let full_rect = ui.max_rect().expand2(
                    egui::vec2(12.0, 8.0), // match frame inner_margin
                );
                ui.painter().line_segment(
                    [
                        egui::pos2(full_rect.left(), full_rect.bottom()),
                        egui::pos2(full_rect.right(), full_rect.bottom()),
                    ],
                    Stroke::new(1.0, t.toolbar_border),
                );
            }

            ui.horizontal_centered(|ui| {
                // ── Group 1: Display ──
                let icon_arrow = &LucideIcon::Navigation2.unicode().to_string();
                toolbar_toggle_image(
                    ui,
                    egui::ImageSource::Bytes {
                        uri: "bytes://contour.svg".into(),
                        bytes: egui::load::Bytes::Static(include_bytes!("../assets/icons/contour.svg")),
                    },
                    show_contours,
                    "Toggle Contours (C)",
                );
                toolbar_toggle_image(
                    ui,
                    egui::ImageSource::Bytes {
                        uri: "bytes://infill.svg".into(),
                        bytes: egui::load::Bytes::Static(include_bytes!("../assets/icons/infill.svg")),
                    },
                    show_infills,
                    "Toggle Hatches (H)",
                );
                toolbar_toggle(ui, icon_arrow, show_arrows, "Toggle Arrows (A)");
                let icon_timer = &LucideIcon::Timer.unicode().to_string();
                toolbar_toggle(ui, icon_timer, show_wait_markers, "Toggle Dwell (T)");
                let icon_play = &LucideIcon::Play.unicode().to_string();
                toolbar_toggle(ui, icon_play, show_vector_view, "Toggle Animate (N)");

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // ── Group 3: Color By ──
                if !compact {
                    ui.label(RichText::new("Color:").size(12.0).color(t.text_secondary));
                }
                let modes: Vec<(Option<ParameterMode>, &str, &str)> = vec![
                    (None, "Type", "Color by vector type (1)"),
                    (Some(ParameterMode::Power), "Power", "Color by laser power (2)"),
                    (Some(ParameterMode::Speed), "Speed", "Color by scan speed (3)"),
                ];
                for (mode, label, tip) in &modes {
                    if toolbar_mode_btn(ui, label, *param_mode == *mode, tip) {
                        *param_mode = *mode;
                        *color_mode = match *mode {
                            None => ColorMode::ByVectorType,
                            Some(pm) => ColorMode::ByParameter(pm),
                        };
                    }
                }
                // "By Part" color mode option (only when multiple files loaded)
                if has_multiple_files {
                    let is_by_file = matches!(color_mode, ColorMode::ByFile);
                    if toolbar_mode_btn(ui, "Part", is_by_file, "Color by part") {
                        if is_by_file {
                            *color_mode = ColorMode::ByVectorType;
                            *param_mode = None;
                        } else {
                            *color_mode = ColorMode::ByFile;
                            *param_mode = None;
                        }
                    }
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // ── Group 4: Right-aligned (Info, Shortcuts, Preferences, Explorer) ──
                let icon_info = LucideIcon::FileText.unicode();
                let info_label = if compact {
                    format!("{}", icon_info)
                } else {
                    format!("{} Info", icon_info)
                };
                let info_btn = egui::Button::new(
                    RichText::new(info_label).size(13.0).color(t.text_primary),
                )
                .fill(if *show_file_info { t.toggle_active_bg } else { Color32::TRANSPARENT })
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(0.0, 30.0));
                if ui.add(info_btn).on_hover_text("Layer info & parameters (I)").clicked() {
                    *show_file_info = !*show_file_info;
                }

                let icon_kbd = LucideIcon::Keyboard.unicode();
                let ctrl_label = if compact {
                    format!("{}", icon_kbd)
                } else {
                    format!("{} Shortcuts", icon_kbd)
                };
                let ctrl_btn = egui::Button::new(
                    RichText::new(ctrl_label).size(13.0).color(t.text_primary),
                )
                .fill(if *show_controls { t.toggle_active_bg } else { Color32::TRANSPARENT })
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(0.0, 30.0));
                if ui.add(ctrl_btn).on_hover_text("Keyboard shortcuts (F1)").clicked() {
                    *show_controls = !*show_controls;
                }

                ui.add_space(4.0);

                let icon_settings = LucideIcon::Settings.unicode();
                let pref_btn = egui::Button::new(
                    RichText::new(format!("{}", icon_settings)).size(13.0).color(t.text_primary),
                )
                .fill(if *show_preferences { t.toggle_active_bg } else { Color32::TRANSPARENT })
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(0.0, 30.0));
                if ui.add(pref_btn).on_hover_text("Preferences").clicked() {
                    *show_preferences = !*show_preferences;
                }
            });
        });

    output
}
