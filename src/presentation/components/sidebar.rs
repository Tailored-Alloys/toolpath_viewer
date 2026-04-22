//! Sidebar Content Panel
//!
//! Renders the content panel portion of the VS Code–style sidebar.
//! The Activity Bar (icon strip) is rendered separately by `activity_bar.rs`.
//! This component renders the Toolpaths tab: file list (add/remove/visibility).

use egui::{Color32, Context, RichText, Rounding, Stroke, Vec2};
use lucide_icons::Icon as LucideIcon;

use crate::application::dto::FileCollection;
use crate::application::ports::{ColorMode, ViewMode};
use crate::domain::value_objects::Color;
use crate::presentation::layout::SidebarRegion;
use crate::presentation::theme;

/// Output from the sidebar component
#[derive(Debug, Clone, Default)]
pub struct SidebarOutput {
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

/// Render the sidebar content panel (right of the activity bar).
///
/// Only rendered when `region.content_visible` is true.
/// Renders the Toolpaths tab content.
pub fn show_sidebar(
    ctx: &Context,
    region: &SidebarRegion,
    files: &FileCollection,
    color_mode: ColorMode,
    view_mode: ViewMode,
    active_tab_file: Option<usize>,
) -> SidebarOutput {
    let mut output = SidebarOutput::default();

    if !region.content_visible {
        return output;
    }

    let t = theme::active();

    egui::SidePanel::left("sidebar_content_panel")
        .exact_width(region.content_width)
        .resizable(false)
        .frame(
            egui::Frame::none()
                .fill(t.toolbar_bg)
                .stroke(Stroke::new(1.0, t.toolbar_border))
                .inner_margin(egui::Margin::symmetric(8.0, 8.0)),
        )
        .show(ctx, |ui| {
            // Tab header label
            let tab_label = "TOOLPATHS";
            let tab_icon = LucideIcon::Files.unicode();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{} {}", tab_icon, tab_label))
                        .size(11.0)
                        .strong()
                        .color(t.text_secondary),
                );
            });
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    show_toolpaths_tab(ui, files, color_mode, view_mode, active_tab_file, &mut output, &t);
                });
        });

    output
}

// ── Toolpaths tab content ──

fn show_toolpaths_tab(
    ui: &mut egui::Ui,
    files: &FileCollection,
    _color_mode: ColorMode,
    view_mode: ViewMode,
    active_tab_file: Option<usize>,
    output: &mut SidebarOutput,
    t: &theme::ActiveTheme,
) {
    // ── Import Toolpath button (prominent CTA at the top) ──
    let import_icon = LucideIcon::FolderOpen.unicode();
    let import_btn = egui::Button::new(
        RichText::new(format!("{} Import Toolpath", import_icon))
            .size(12.5)
            .strong()
            .color(Color32::WHITE),
    )
    .fill(t.accent)
    .stroke(Stroke::NONE)
    .rounding(Rounding::same(6.0))
    .min_size(Vec2::new(ui.available_width(), 36.0));
    if ui.add(import_btn).on_hover_text("Import toolpath file (Ctrl+O)").clicked() {
        output.add_files_requested = true;
    }

    ui.add_space(6.0);

    if files.files.is_empty() {
        // ── Empty state: subtle placeholder instead of stacked separators ──
        ui.add_space(12.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("No files loaded")
                    .size(11.0)
                    .color(t.text_secondary),
            );
        });
    } else {
        // ── File list ──
        ui.separator();
        ui.add_space(4.0);
        ui.spacing_mut().item_spacing.y = 1.0;

        let row_h = 28.0;
        let available_w = ui.available_width();

        for file in &files.files {
            let is_active_tab = view_mode == ViewMode::Tab && active_tab_file == Some(file.id);
            let row_id = ui.id().with("file_row").with(file.id);

            // Allocate a fixed row region so we can detect hover on the full row
            let (row_rect, row_resp) = ui.allocate_exact_size(
                Vec2::new(available_w, row_h),
                egui::Sense::click(),
            );
            // Use rect_contains_pointer instead of row_resp.hovered() to avoid
            // hover flicker when child widgets (close button) steal the hover.
            let is_hovered = ui.rect_contains_pointer(row_rect);

            // ── Row background fill ──
            let row_fill = if is_active_tab {
                if t.is_dark {
                    Color32::from_rgba_unmultiplied(
                        t.accent.r(), t.accent.g(), t.accent.b(), 35,
                    )
                } else {
                    Color32::from_rgba_unmultiplied(
                        t.accent.r(), t.accent.g(), t.accent.b(), 25,
                    )
                }
            } else if is_hovered {
                if t.is_dark {
                    Color32::from_rgba_unmultiplied(255, 255, 255, 15)
                } else {
                    Color32::from_rgba_unmultiplied(0, 0, 0, 12)
                }
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(row_rect, Rounding::same(4.0), row_fill);

            // ── Left accent bar on active/selected row ──
            if is_active_tab {
                let accent_bar = egui::Rect::from_min_size(
                    row_rect.left_top(),
                    Vec2::new(2.5, row_rect.height()),
                );
                ui.painter().rect_filled(accent_bar, Rounding::same(1.0), t.accent);
            }

            let center_y = row_rect.center().y;
            let painter = ui.painter();

            // ── Color swatch ──
            let swatch_x = row_rect.left() + 8.0;
            let swatch_color = color_to_color32(&file.color);
            let swatch_rect = egui::Rect::from_center_size(
                egui::pos2(swatch_x + 5.0, center_y),
                Vec2::new(10.0, 10.0),
            );
            painter.rect_filled(swatch_rect, 2.0, swatch_color);

            let mut content_x = swatch_x + 16.0;

            // ── Visibility toggle (eye icon) — only in Overlay mode ──
            if view_mode == ViewMode::Overlay {
                let eye_icon = if file.visible {
                    LucideIcon::Eye.unicode()
                } else {
                    LucideIcon::EyeOff.unicode()
                };
                let eye_color = if file.visible { t.text_primary } else { t.text_secondary };
                let eye_rect = egui::Rect::from_center_size(
                    egui::pos2(content_x + 8.0, center_y),
                    Vec2::new(18.0, 18.0),
                );
                let eye_resp = ui.interact(eye_rect, row_id.with("eye"), egui::Sense::click());
                let eye_galley = painter.layout_no_wrap(
                    eye_icon.to_string(),
                    egui::FontId::proportional(11.0),
                    eye_color,
                );
                painter.galley(
                    egui::pos2(eye_rect.center().x - eye_galley.size().x / 2.0, center_y - eye_galley.size().y / 2.0),
                    eye_galley,
                    Color32::TRANSPARENT,
                );
                if eye_resp.on_hover_text("Toggle visibility").clicked() {
                    output.toggle_visibility = Some(file.id);
                }
                content_x += 20.0;
            }

            // ── Tab mode: radio indicator ──
            if view_mode == ViewMode::Tab {
                let dot_center = egui::pos2(content_x + 6.0, center_y);
                if is_active_tab {
                    painter.circle_filled(dot_center, 4.0, t.accent);
                } else {
                    let ring_color = if t.is_dark {
                        Color32::from_rgb(100, 100, 100)
                    } else {
                        Color32::from_rgb(180, 180, 180)
                    };
                    painter.circle_stroke(dot_center, 4.0, Stroke::new(1.2, ring_color));
                }
                content_x += 16.0;
            }

            // ── File name ──
            let name_color = if is_active_tab {
                t.accent
            } else if file.visible || view_mode != ViewMode::Overlay {
                t.text_primary
            } else {
                t.text_secondary
            };
            let max_name_len = 20;
            let display_name = if file.name.len() > max_name_len {
                format!("{}…", &file.name[..max_name_len])
            } else {
                file.name.clone()
            };
            let name_galley = painter.layout_no_wrap(
                display_name,
                egui::FontId::proportional(11.0),
                name_color,
            );
            let name_pos = egui::pos2(content_x + 2.0, center_y - name_galley.size().y / 2.0);
            painter.galley(name_pos, name_galley, Color32::TRANSPARENT);

            // ── Close button (right-aligned, visible on hover or active) ──
            if is_active_tab || is_hovered {
                let x_center = egui::pos2(row_rect.right() - 14.0, center_y);
                let x_size = 4.0;
                let x_rect = egui::Rect::from_center_size(
                    x_center,
                    Vec2::new(18.0, 18.0),
                );
                let x_resp = ui.interact(x_rect, row_id.with("close"), egui::Sense::click());
                let x_color = if x_resp.hovered() { t.text_primary } else { t.text_secondary };
                // Draw hover background on close button
                if x_resp.hovered() {
                    let hover_bg = if t.is_dark {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 20)
                    } else {
                        Color32::from_rgba_unmultiplied(0, 0, 0, 18)
                    };
                    painter.rect_filled(x_rect, Rounding::same(3.0), hover_bg);
                }
                let x_stroke = Stroke::new(1.2, x_color);
                painter.line_segment(
                    [egui::pos2(x_center.x - x_size, x_center.y - x_size),
                     egui::pos2(x_center.x + x_size, x_center.y + x_size)],
                    x_stroke,
                );
                painter.line_segment(
                    [egui::pos2(x_center.x + x_size, x_center.y - x_size),
                     egui::pos2(x_center.x - x_size, x_center.y + x_size)],
                    x_stroke,
                );
                if x_resp.on_hover_text("Remove file").clicked() {
                    output.remove_file = Some(file.id);
                }
            }

            // ── Row click handling (all view modes — triggers reopen for closed tabs) ──
            if row_resp.clicked()
                && output.remove_file != Some(file.id)
                && output.toggle_visibility != Some(file.id)
            {
                output.select_tab_file = Some(file.id);
            }
            row_resp.on_hover_text(&file.path.display().to_string());
        }

        ui.add_space(6.0);

        // ── Add more toolpaths button (secondary/outline style) ──
        let add_icon = LucideIcon::Plus.unicode();
        let accent_outline = Color32::from_rgba_unmultiplied(
            t.accent.r(), t.accent.g(), t.accent.b(), 40,
        );
        let add_btn = egui::Button::new(
            RichText::new(format!("{} Add Toolpath", add_icon))
                .size(11.0)
                .color(t.accent),
        )
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0, accent_outline))
        .rounding(Rounding::same(5.0))
        .min_size(Vec2::new(ui.available_width(), 26.0));
        if ui.add(add_btn).on_hover_text("Add more toolpath files (Ctrl+O)").clicked() {
            output.add_files_requested = true;
        }
    }
}
