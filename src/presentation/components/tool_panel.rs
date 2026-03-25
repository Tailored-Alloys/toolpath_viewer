//! Tool Panel Component
//!
//! Top-right floating horizontal tool strip with:
//! zoom in/out, fit view, zoom selection, ruler, scale bar toggle,
//! grid toggle + unit selector, snapshot button.

use egui::{Align2, Color32, Context, RichText, Rounding, Vec2};
use lucide_icons::Icon as LucideIcon;

use crate::presentation::layout::ToolPanelRegion;
use crate::presentation::theme::*;

use super::super::ui::ToolMode;

/// Output from the tool panel component
#[derive(Debug, Clone, Default)]
pub struct ToolPanelOutput {
    pub zoom_in_requested: bool,
    pub zoom_out_requested: bool,
    pub fit_view_requested: bool,
    pub snapshot_requested: bool,
}

/// Render the tool panel.
pub fn show_tool_panel(
    ctx: &Context,
    region: &ToolPanelRegion,
    tool_mode: &mut ToolMode,
    ruler_start: &mut Option<crate::domain::value_objects::Point2D>,
    ruler_end: &mut Option<crate::domain::value_objects::Point2D>,
    ruler_measurements: &mut Vec<super::super::ui::RulerMeasurement>,
    show_grid: &mut bool,
) -> ToolPanelOutput {
    let mut output = ToolPanelOutput::default();

    egui::Area::new(egui::Id::new("tool_panel_area"))
        .fixed_pos(region.anchor_pos)
        .pivot(Align2::RIGHT_TOP)
        .order(egui::Order::Foreground)
        .interactable(true)
        .movable(false)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(PANEL_BG_TRANSLUCENT)
                .rounding(Rounding::same(8.0))
                .shadow(egui::epaint::Shadow {
                    offset: egui::vec2(0.0, 1.0),
                    blur: 6.0,
                    spread: 0.0,
                    color: PANEL_SHADOW,
                })
                .inner_margin(egui::Margin::symmetric(4.0, 4.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                        let btn_size = Vec2::new(28.0, 28.0);
                        let icon_sz = 14.0;

                        let mut tool_btn = |ui: &mut egui::Ui,
                                            icon: &str,
                                            tooltip: &str,
                                            active: bool|
                         -> bool {
                            let fill = if active {
                                TOGGLE_ACTIVE_BG
                            } else {
                                Color32::TRANSPARENT
                            };
                            let tc = if active { ACCENT } else { TEXT_PRIMARY };
                            let btn = egui::Button::new(
                                RichText::new(icon).size(icon_sz).color(tc),
                            )
                            .fill(fill)
                            .rounding(Rounding::same(4.0))
                            .min_size(btn_size);
                            ui.add(btn).on_hover_text(tooltip).clicked()
                        };

                        // ── Zoom In ──
                        if tool_btn(
                            ui,
                            &LucideIcon::ZoomIn.unicode().to_string(),
                            "Zoom In (+)",
                            false,
                        ) {
                            output.zoom_in_requested = true;
                        }
                        // ── Zoom Out ──
                        if tool_btn(
                            ui,
                            &LucideIcon::ZoomOut.unicode().to_string(),
                            "Zoom Out (-)",
                            false,
                        ) {
                            output.zoom_out_requested = true;
                        }
                        // ── Fit View ──
                        if tool_btn(
                            ui,
                            &LucideIcon::Expand.unicode().to_string(),
                            "Fit View (R)",
                            false,
                        ) {
                            output.fit_view_requested = true;
                        }
                        // ── Zoom Selection & Ruler removed — now on mouse ──
                        // Left-drag: zoom selection, Right-click: ruler

                        // ── Clear measurements ──
                        if !ruler_measurements.is_empty() {
                            // ── Separator ──
                            ui.add(egui::Separator::default().vertical().spacing(4.0));

                            let btn = egui::Button::new(
                                RichText::new(
                                    &LucideIcon::X.unicode().to_string(),
                                )
                                .size(11.0)
                                .color(TEXT_SECONDARY),
                            )
                            .fill(Color32::TRANSPARENT)
                            .rounding(Rounding::same(4.0))
                            .min_size(Vec2::new(20.0, 28.0));
                            if ui
                                .add(btn)
                                .on_hover_text("Clear measurements")
                                .clicked()
                            {
                                ruler_measurements.clear();
                            }
                        }

                        // ── Separator ──
                        ui.add(egui::Separator::default().vertical().spacing(4.0));

                        // ── Grid ──
                        if tool_btn(
                            ui,
                            &LucideIcon::Grid3x3.unicode().to_string(),
                            "Grid (G)",
                            *show_grid,
                        ) {
                            *show_grid = !*show_grid;
                        }

                        // ── Separator ──
                        ui.add(egui::Separator::default().vertical().spacing(4.0));

                        // ── Snapshot ──
                        if tool_btn(
                            ui,
                            &LucideIcon::Camera.unicode().to_string(),
                            "Snapshot (Ctrl+P)",
                            false,
                        ) {
                            output.snapshot_requested = true;
                        }
                    });
                });
        });

    output
}
