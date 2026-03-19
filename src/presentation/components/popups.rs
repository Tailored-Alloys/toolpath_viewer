//! Popup Components
//!
//! Floating popup windows:
//! - File Info: layer vector counts and parameter values
//! - Controls: keyboard/mouse shortcut reference

use egui::{Context, RichText, Rounding};

use crate::presentation::layout::TOOLBAR_HEIGHT;
use crate::presentation::theme::*;

use super::super::ui::VectorCounts;

/// Render the file info popup window.
pub fn show_file_info(ctx: &Context, vector_counts: &VectorCounts) {
    let file_icon = lucide_icons::Icon::FileText.unicode();
    egui::Window::new(
        RichText::new(format!("{} File Info", file_icon))
            .size(14.0)
            .color(TEXT_PRIMARY),
    )
    .collapsible(false)
    .resizable(false)
    .default_width(220.0)
    .default_pos(egui::pos2(12.0, TOOLBAR_HEIGHT + 12.0))
    .show(ctx, |ui| {
        ui.label(
            RichText::new("Layer Info")
                .size(13.0)
                .strong()
                .color(TEXT_PRIMARY),
        );
        ui.add_space(4.0);
        let vc = vector_counts;
        let info_items = [
            ("Hatches", vc.hatches),
            ("Contours", vc.contours),
            ("Boundaries", vc.boundaries),
            ("Total vectors", vc.total_vectors),
        ];
        for (label, val) in &info_items {
            ui.horizontal(|ui| {
                ui.label(RichText::new(*label).size(12.0).color(TEXT_SECONDARY));
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label(
                            RichText::new(format!("{}", val))
                                .size(12.0)
                                .strong()
                                .color(TEXT_PRIMARY),
                        );
                    },
                );
            });
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        ui.label(
            RichText::new("Parameters")
                .size(13.0)
                .strong()
                .color(TEXT_PRIMARY),
        );
        ui.add_space(4.0);

        let params: Vec<(&str, Option<f32>)> = vec![
            ("VK Power", vc.vk_power),
            ("VK Speed", vc.vk_speed),
            ("VS Power", vc.vs_power),
            ("VS Speed", vc.vs_speed),
        ];
        for (label, val) in &params {
            if let Some(v) = val {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(*label).size(12.0).color(TEXT_SECONDARY));
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label(
                                RichText::new(format!("{:.0}", v))
                                    .size(12.0)
                                    .strong()
                                    .color(TEXT_PRIMARY),
                            );
                        },
                    );
                });
            }
        }
        if vc.wait_count > 0 {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Wait times").size(12.0).color(TEXT_SECONDARY));
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label(
                            RichText::new(format!("{}", vc.wait_count))
                                .size(12.0)
                                .strong()
                                .color(TEXT_PRIMARY),
                        );
                    },
                );
            });
        }
    });
}

/// Render the controls popup window (keyboard shortcut reference).
pub fn show_controls_popup(ctx: &Context) {
    egui::Window::new(
        RichText::new("⌨ Controls")
            .size(14.0)
            .color(TEXT_PRIMARY),
    )
    .collapsible(false)
    .resizable(false)
    .default_width(240.0)
    .default_pos(egui::pos2(12.0, TOOLBAR_HEIGHT + 12.0))
    .show(ctx, |ui| {
        let shortcuts = [
            ("↑ / ↓", "Navigate layers"),
            ("Page Up / Down", "Jump 10 layers"),
            ("Home / End", "First / Last layer"),
            ("Scroll Wheel", "Zoom in / out"),
            ("+ / -", "Zoom in / out"),
            ("Middle Drag", "Pan view"),
            ("Left Drag", "Zoom to selection"),
            ("Right Click", "Add measurement point"),
            ("R", "Reset view (fit to content)"),
            ("B", "Toggle boundaries"),
            ("C", "Toggle contours"),
            ("H", "Toggle hatches"),
            ("A", "Toggle direction arrows"),
            ("T", "Toggle wait time markers"),
            ("V", "Toggle scale bar"),
            ("G", "Toggle background grid"),
            ("Z", "Toggle zoom selection mode"),
            ("M", "Toggle ruler / measure tool"),
            ("X", "Clear measurements"),
            ("1 / 2 / 3", "Color: None / Power / Speed"),
            ("I", "Toggle file info panel"),
            ("F1", "Toggle this help"),
            ("Ctrl + O", "Open file"),
            ("Ctrl + P", "Take snapshot"),
        ];
        egui::Grid::new("controls_grid")
            .num_columns(2)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                for (key, action) in &shortcuts {
                    ui.label(RichText::new(*key).size(12.0).strong().color(ACCENT));
                    ui.label(RichText::new(*action).size(12.0).color(TEXT_PRIMARY));
                    ui.end_row();
                }
            });
    });
}
