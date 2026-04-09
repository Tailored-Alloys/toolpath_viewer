//! File Panel Component
//!
//! Right-side collapsible panel showing loaded files with visibility toggles.

use egui::{Color32, Context, RichText, Rounding, Stroke, Vec2};
use lucide_icons::Icon as LucideIcon;

use crate::application::dto::FileCollection;
use crate::application::ports::{ColorMode, ViewMode};
use crate::domain::value_objects::Color;
use crate::presentation::theme;

/// Region info for the file panel
#[derive(Debug, Clone)]
pub struct FilePanelRegion {
    /// Position for the egui Area (top-left)
    pub pos: egui::Pos2,
    /// Available content height
    pub content_height: f32,
    /// Width of the panel
    pub width: f32,
}

/// Output from the file panel
#[derive(Debug, Clone, Default)]
pub struct FilePanelOutput {
    /// User requested to add more files
    pub add_files_requested: bool,
    /// File ID to remove (if any)
    pub remove_file: Option<usize>,
    /// File ID to toggle visibility (if any)
    pub toggle_visibility: Option<usize>,
    /// File ID selected as active tab (for Tab mode)
    pub select_tab_file: Option<usize>,
}

/// Convert domain Color to egui Color32
fn color_to_color32(c: &Color) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (c.r * 255.0) as u8,
        (c.g * 255.0) as u8,
        (c.b * 255.0) as u8,
        (c.a * 255.0) as u8,
    )
}

/// Render the file panel
pub fn show_file_panel(
    ctx: &Context,
    region: &FilePanelRegion,
    files: &FileCollection,
    color_mode: ColorMode,
    view_mode: &mut ViewMode,
    active_tab_file: Option<usize>,
) -> FilePanelOutput {
    let mut output = FilePanelOutput::default();
    let t = theme::active();

    egui::Area::new(egui::Id::new("file_panel"))
        .fixed_pos(region.pos)
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(t.toolbar_bg)
                .stroke(Stroke::new(1.0, t.toolbar_border))
                .rounding(Rounding::same(8.0))
                .inner_margin(egui::Margin::symmetric(8.0, 8.0))
                .show(ui, |ui| {
                    ui.set_width(region.width - 16.0);
                    ui.set_max_height(region.content_height);

                    // Header
                    ui.horizontal(|ui| {
                        let icon = LucideIcon::Files.unicode();
                        ui.label(
                            RichText::new(format!("{} Files ({})", icon, files.len()))
                                .size(12.0)
                                .strong()
                                .color(t.text_primary),
                        );
                    });

                    ui.add_space(4.0);

                    // View mode selector (only when 2+ files)
                    if files.len() >= 2 {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            let modes = [
                                (ViewMode::Overlay, LucideIcon::Layers.unicode(), "Overlay"),
                                (ViewMode::Tab, LucideIcon::SquareStack.unicode(), "Tab"),
                                (ViewMode::Split, LucideIcon::Columns2.unicode(), "Split"),
                            ];
                            for (mode, icon, tooltip) in &modes {
                                let selected = *view_mode == *mode;
                                let btn_color = if selected { t.accent } else { t.text_secondary };
                                let fill = if selected {
                                    Color32::from_rgba_unmultiplied(
                                        (t.accent.r() as u16 * 40 / 255) as u8,
                                        (t.accent.g() as u16 * 40 / 255) as u8,
                                        (t.accent.b() as u16 * 40 / 255) as u8,
                                        40,
                                    )
                                } else {
                                    Color32::TRANSPARENT
                                };
                                let btn = egui::Button::new(
                                    RichText::new(icon.to_string()).size(12.0).color(btn_color),
                                )
                                .fill(fill)
                                .rounding(Rounding::same(4.0))
                                .min_size(Vec2::new(28.0, 22.0));
                                if ui.add(btn).on_hover_text(*tooltip).clicked() {
                                    *view_mode = *mode;
                                }
                            }
                        });
                        ui.add_space(2.0);
                    }

                    ui.separator();
                    ui.add_space(4.0);

                    // File list (scrollable)
                    egui::ScrollArea::vertical()
                        .max_height(region.content_height - 100.0)
                        .show(ui, |ui| {
                            for file in &files.files {
                                let is_active_tab = *view_mode == ViewMode::Tab
                                    && active_tab_file == Some(file.id);

                                // Highlight active tab file
                                let row_fill = if is_active_tab {
                                    Color32::from_rgba_unmultiplied(
                                        (t.accent.r() as u16 * 30 / 255) as u8,
                                        (t.accent.g() as u16 * 30 / 255) as u8,
                                        (t.accent.b() as u16 * 30 / 255) as u8,
                                        30,
                                    )
                                } else {
                                    Color32::TRANSPARENT
                                };

                                egui::Frame::none()
                                    .fill(row_fill)
                                    .rounding(Rounding::same(4.0))
                                    .inner_margin(egui::Margin::symmetric(2.0, 1.0))
                                    .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    // Color swatch
                                    let swatch_color = color_to_color32(&file.color);
                                    let (rect, _) = ui.allocate_exact_size(
                                        Vec2::new(12.0, 12.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(rect, 2.0, swatch_color);

                                    // Visibility toggle (eye icon) — only in Overlay mode
                                    if *view_mode == ViewMode::Overlay {
                                        let eye_icon = if file.visible {
                                            LucideIcon::Eye.unicode()
                                        } else {
                                            LucideIcon::EyeOff.unicode()
                                        };
                                        let eye_color = if file.visible {
                                            t.text_primary
                                        } else {
                                            t.text_secondary
                                        };
                                        let eye_btn = egui::Button::new(
                                            RichText::new(eye_icon.to_string()).size(11.0).color(eye_color),
                                        )
                                        .fill(Color32::TRANSPARENT)
                                        .rounding(Rounding::same(4.0))
                                        .min_size(Vec2::new(20.0, 20.0));
                                        if ui.add(eye_btn).on_hover_text("Toggle visibility").clicked() {
                                            output.toggle_visibility = Some(file.id);
                                        }
                                    }

                                    // File name (clickable in Tab mode to select tab)
                                    let name_color = if is_active_tab {
                                        t.accent
                                    } else if file.visible || *view_mode != ViewMode::Overlay {
                                        t.text_primary
                                    } else {
                                        t.text_secondary
                                    };
                                    let max_name_len = 16;
                                    let display_name = if file.name.len() > max_name_len {
                                        format!("{}…", &file.name[..max_name_len])
                                    } else {
                                        file.name.clone()
                                    };

                                    if *view_mode == ViewMode::Tab {
                                        // Clickable label to select tab
                                        let name_btn = egui::Button::new(
                                            RichText::new(display_name).size(11.0).color(name_color),
                                        )
                                        .fill(Color32::TRANSPARENT)
                                        .frame(false)
                                        .min_size(Vec2::ZERO);
                                        if ui.add(name_btn)
                                            .on_hover_text(&file.path.display().to_string())
                                            .clicked()
                                        {
                                            output.select_tab_file = Some(file.id);
                                        }
                                    } else {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(display_name).size(11.0).color(name_color),
                                            )
                                            .truncate(true),
                                        )
                                        .on_hover_text(&file.path.display().to_string());
                                    }

                                    // Remove button
                                    let x_btn = egui::Button::new(
                                        RichText::new(LucideIcon::X.unicode().to_string())
                                            .size(10.0)
                                            .color(t.text_secondary),
                                    )
                                    .fill(Color32::TRANSPARENT)
                                    .rounding(Rounding::same(4.0))
                                    .min_size(Vec2::new(18.0, 18.0));
                                    if ui.add(x_btn).on_hover_text("Remove file").clicked() {
                                        output.remove_file = Some(file.id);
                                    }
                                });
                                    });

                                ui.add_space(2.0);
                            }
                        });

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Add files button
                    let add_icon = LucideIcon::Plus.unicode();
                    let add_btn = egui::Button::new(
                        RichText::new(format!("{} Add Files", add_icon))
                            .size(11.0)
                            .color(t.text_primary),
                    )
                    .fill(Color32::TRANSPARENT)
                    .rounding(Rounding::same(4.0))
                    .min_size(Vec2::new(0.0, 24.0));
                    if ui
                        .add(add_btn)
                        .on_hover_text("Add more files (Ctrl+O)")
                        .clicked()
                    {
                        output.add_files_requested = true;
                    }
                });
        });

    output
}
