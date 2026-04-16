//! Sidebar Content Panel
//!
//! Renders the content panel portion of the VS Code–style sidebar.
//! The Activity Bar (icon strip) is rendered separately by `activity_bar.rs`.
//! This component dispatches to the active tab's content:
//! - Toolpaths: file list (add/remove/visibility)
//! - Parameter Legend: parameter gradient + wait gradient + filter controls

use egui::{Color32, Context, DragValue, RichText, Rounding, Stroke, Vec2};
use lucide_icons::Icon as LucideIcon;

use crate::application::dto::FileCollection;
use crate::application::ports::{ColorMode, GlobalUnits, ParameterMode, ViewMode};
use crate::domain::value_objects::Color;
use crate::presentation::layout::SidebarRegion;
use crate::presentation::palette::{self, ThemePalette};
use crate::presentation::theme;

use super::super::ui::{ParamRanges, SidebarTab};

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
/// Dispatches to the active tab's content.
pub fn show_sidebar(
    ctx: &Context,
    region: &SidebarRegion,
    active_tab: SidebarTab,
    files: &FileCollection,
    color_mode: ColorMode,
    view_mode: ViewMode,
    active_tab_file: Option<usize>,
    // Gradient state
    show_gradient: bool,
    param_mode: Option<ParameterMode>,
    param_filter_min: &mut f32,
    param_filter_max: &mut f32,
    param_ranges: &ParamRanges,
    // Wait gradient state
    show_wait_gradient: bool,
    wait_filter_min: &mut f32,
    wait_filter_max: &mut f32,
    // Shared
    global_units: &GlobalUnits,
    active_palette: &ThemePalette,
    show_ticks: bool,
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
            let tab_label = match active_tab {
                SidebarTab::Toolpaths => "TOOLPATHS",
                SidebarTab::ParameterLegend => "PARAMETER LEGEND",
            };
            let tab_icon = match active_tab {
                SidebarTab::Toolpaths => LucideIcon::Files.unicode(),
                SidebarTab::ParameterLegend => LucideIcon::Palette.unicode(),
            };
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
                    match active_tab {
                        SidebarTab::Toolpaths => {
                            show_toolpaths_tab(ui, files, color_mode, view_mode, active_tab_file, &mut output, &t);
                        }
                        SidebarTab::ParameterLegend => {
                            show_parameter_legend_tab(
                                ui,
                                show_gradient,
                                param_mode,
                                param_filter_min,
                                param_filter_max,
                                param_ranges,
                                show_wait_gradient,
                                wait_filter_min,
                                wait_filter_max,
                                global_units,
                                active_palette,
                                show_ticks,
                                &t,
                            );
                        }
                    }
                });
        });

    output
}

// ── Toolpaths tab content ──

fn show_toolpaths_tab(
    ui: &mut egui::Ui,
    files: &FileCollection,
    color_mode: ColorMode,
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

    ui.separator();
    ui.add_space(4.0);

    // File list (scrollable within sidebar scroll area)
    for file in &files.files {
        let is_active_tab = view_mode == ViewMode::Tab && active_tab_file == Some(file.id);
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
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, swatch_color);

                    // Visibility toggle (eye icon) — only in Overlay mode
                    if view_mode == ViewMode::Overlay {
                        let eye_icon = if file.visible {
                            LucideIcon::Eye.unicode()
                        } else {
                            LucideIcon::EyeOff.unicode()
                        };
                        let eye_color = if file.visible { t.text_primary } else { t.text_secondary };
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

                    // Tab mode: radio button to select active file
                    if view_mode == ViewMode::Tab {
                        let selected = active_tab_file == Some(file.id);
                        if ui.radio(selected, "").clicked() {
                            output.select_tab_file = Some(file.id);
                        }
                    }

                    // File name
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

                    if view_mode == ViewMode::Tab {
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

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    // Add more toolpaths button (secondary/outline style)
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

// ── Parameter Legend tab content ──

fn show_parameter_legend_tab(
    ui: &mut egui::Ui,
    show_gradient: bool,
    param_mode: Option<ParameterMode>,
    param_filter_min: &mut f32,
    param_filter_max: &mut f32,
    param_ranges: &ParamRanges,
    show_wait_gradient: bool,
    wait_filter_min: &mut f32,
    wait_filter_max: &mut f32,
    global_units: &GlobalUnits,
    active_palette: &ThemePalette,
    show_ticks: bool,
    t: &theme::ActiveTheme,
) {
    if show_gradient {
        if let Some(pm) = param_mode {
            show_inline_gradient(
                ui, pm, param_filter_min, param_filter_max,
                param_ranges, global_units, active_palette, show_ticks, t,
            );
        }
    }
    if show_wait_gradient {
        if show_gradient { ui.add_space(12.0); }
        show_inline_wait_gradient(
            ui, wait_filter_min, wait_filter_max,
            param_ranges.wait_time, global_units, active_palette,
            show_ticks, t,
        );
    }
    if !show_gradient && !show_wait_gradient {
        ui.add_space(8.0);
        ui.label(
            RichText::new("Select a color mode (Power, Speed) from the toolbar to view the parameter legend.")
                .size(10.0)
                .color(t.text_secondary),
        );
    }
}

// ── Inline gradient scale ──

fn show_inline_gradient(
    ui: &mut egui::Ui,
    param_mode: ParameterMode,
    param_filter_min: &mut f32,
    param_filter_max: &mut f32,
    param_ranges: &ParamRanges,
    global_units: &GlobalUnits,
    active_palette: &ThemePalette,
    show_ticks: bool,
    t: &theme::ActiveTheme,
) {
    let available_w = ui.available_width();
    let bar_w = 20.0;
    let bar_h = 120.0_f32.min(ui.available_height() * 0.3).max(60.0);

    // Mode label
    let mode_label = global_units.param_label(param_mode);
    ui.label(RichText::new(&mode_label).size(11.0).strong().color(t.text_primary));
    ui.add_space(4.0);

    // Max value
    let display_max = global_units.convert_param(param_mode, *param_filter_max);
    ui.label(RichText::new(format!("{:.1}", display_max)).size(10.0).color(t.text_secondary));
    ui.add_space(2.0);

    // Gradient bar (horizontal layout: bar + ticks)
    let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(available_w, bar_h), egui::Sense::hover());
    let bar_draw = egui::Rect::from_min_size(
        egui::pos2(bar_rect.left() + (available_w - bar_w) / 2.0, bar_rect.top()),
        egui::vec2(bar_w, bar_h),
    );
    let n = 64;
    let seg_h = bar_draw.height() / n as f32;
    for i in 0..n {
        let frac = 1.0 - (i as f32 / (n - 1) as f32);
        let c = palette::gradient_color(active_palette, frac);
        let y0 = bar_draw.top() + i as f32 * seg_h;
        let seg = egui::Rect::from_min_max(
            egui::pos2(bar_draw.left(), y0),
            egui::pos2(bar_draw.right(), y0 + seg_h + 0.5),
        );
        ui.painter().rect_filled(
            seg, 0.0,
            Color32::from_rgb((c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8),
        );
    }
    let border_color = if t.is_dark { Color32::from_rgb(80, 80, 80) } else { Color32::from_rgb(180, 180, 180) };
    ui.painter().rect_stroke(bar_draw, Rounding::same(2.0), Stroke::new(1.0, border_color));

    // Tick marks
    if show_ticks {
        let num_ticks = 5;
        let val_min = *param_filter_min;
        let val_max = *param_filter_max;
        for tick_i in 0..num_ticks {
            let frac = tick_i as f32 / (num_ticks - 1) as f32;
            let tick_y = bar_draw.bottom() - frac * bar_draw.height();
            ui.painter().line_segment(
                [egui::pos2(bar_draw.right(), tick_y), egui::pos2(bar_draw.right() + 4.0, tick_y)],
                Stroke::new(1.0, t.text_secondary),
            );
            if tick_i > 0 && tick_i < num_ticks - 1 {
                let raw_val = val_min + frac * (val_max - val_min);
                let display_val = global_units.convert_param(param_mode, raw_val);
                ui.painter().text(
                    egui::pos2(bar_draw.right() + 6.0, tick_y),
                    egui::Align2::LEFT_CENTER,
                    format!("{:.0}", display_val),
                    egui::FontId::proportional(8.0),
                    t.text_secondary,
                );
            }
        }
    }

    ui.add_space(2.0);

    // Min value
    let display_min = global_units.convert_param(param_mode, *param_filter_min);
    ui.label(RichText::new(format!("{:.1}", display_min)).size(10.0).color(t.text_secondary));
    ui.add_space(4.0);

    // Filter controls
    ui.separator();
    ui.add_space(4.0);

    let data_range = match param_mode {
        ParameterMode::Power => param_ranges.power,
        ParameterMode::Speed => param_ranges.speed,
    };
    let (lo, hi) = data_range.unwrap_or((0.0, 1.0));
    let speed = (hi - lo).abs() * 0.01;
    let unit_suffix = global_units.param_suffix(param_mode);

    ui.horizontal(|ui| {
        ui.label(RichText::new("Min").size(9.0).color(t.text_secondary));
        ui.add(
            DragValue::new(param_filter_min)
                .speed(speed.max(0.1))
                .clamp_range(lo..=*param_filter_max)
                .suffix(&unit_suffix),
        );
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("Max").size(9.0).color(t.text_secondary));
        ui.add(
            DragValue::new(param_filter_max)
                .speed(speed.max(0.1))
                .clamp_range(*param_filter_min..=hi)
                .suffix(&unit_suffix),
        );
    });

    ui.add_space(2.0);
    if ui.button(RichText::new("Reset").size(9.0)).clicked() {
        *param_filter_min = lo;
        *param_filter_max = hi;
    }
}

fn show_inline_wait_gradient(
    ui: &mut egui::Ui,
    wait_filter_min: &mut f32,
    wait_filter_max: &mut f32,
    wait_range: Option<(f32, f32)>,
    global_units: &GlobalUnits,
    active_palette: &ThemePalette,
    show_ticks: bool,
    t: &theme::ActiveTheme,
) {
    let available_w = ui.available_width();
    let bar_w = 20.0;
    let bar_h = 120.0_f32.min(ui.available_height() * 0.3).max(60.0);

    // Mode label
    let mode_label = format!("Dwell ({})", global_units.time.label());
    ui.label(RichText::new(&mode_label).size(11.0).strong().color(t.text_primary));
    ui.add_space(4.0);

    // Max value
    let display_max = global_units.time.from_us(*wait_filter_max);
    ui.label(RichText::new(format!("{:.1}", display_max)).size(10.0).color(t.text_secondary));
    ui.add_space(2.0);

    // Gradient bar
    let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(available_w, bar_h), egui::Sense::hover());
    let bar_draw = egui::Rect::from_min_size(
        egui::pos2(bar_rect.left() + (available_w - bar_w) / 2.0, bar_rect.top()),
        egui::vec2(bar_w, bar_h),
    );
    let n = 64;
    let seg_h = bar_draw.height() / n as f32;
    for i in 0..n {
        let frac = 1.0 - (i as f32 / (n - 1) as f32);
        let c = palette::gradient_color(active_palette, frac);
        let y0 = bar_draw.top() + i as f32 * seg_h;
        let seg = egui::Rect::from_min_max(
            egui::pos2(bar_draw.left(), y0),
            egui::pos2(bar_draw.right(), y0 + seg_h + 0.5),
        );
        ui.painter().rect_filled(
            seg, 0.0,
            Color32::from_rgb((c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8),
        );
    }
    let border_color = if t.is_dark { Color32::from_rgb(80, 80, 80) } else { Color32::from_rgb(180, 180, 180) };
    ui.painter().rect_stroke(bar_draw, Rounding::same(2.0), Stroke::new(1.0, border_color));

    // Tick marks
    if show_ticks {
        let num_ticks = 5;
        let val_min = *wait_filter_min;
        let val_max = *wait_filter_max;
        for tick_i in 0..num_ticks {
            let frac = tick_i as f32 / (num_ticks - 1) as f32;
            let tick_y = bar_draw.bottom() - frac * bar_draw.height();
            ui.painter().line_segment(
                [egui::pos2(bar_draw.right(), tick_y), egui::pos2(bar_draw.right() + 4.0, tick_y)],
                Stroke::new(1.0, t.text_secondary),
            );
            if tick_i > 0 && tick_i < num_ticks - 1 {
                let raw_val = val_min + frac * (val_max - val_min);
                let display_val = global_units.time.from_us(raw_val);
                ui.painter().text(
                    egui::pos2(bar_draw.right() + 6.0, tick_y),
                    egui::Align2::LEFT_CENTER,
                    format!("{:.0}", display_val),
                    egui::FontId::proportional(8.0),
                    t.text_secondary,
                );
            }
        }
    }

    ui.add_space(2.0);

    // Min value
    let display_min = global_units.time.from_us(*wait_filter_min);
    ui.label(RichText::new(format!("{:.1}", display_min)).size(10.0).color(t.text_secondary));
    ui.add_space(4.0);

    // Filter controls
    ui.separator();
    ui.add_space(4.0);

    let (lo, hi) = wait_range.unwrap_or((0.0, 1.0));
    let speed = (hi - lo).abs() * 0.01;
    let unit_suffix = format!(" {}", global_units.time.label());

    ui.horizontal(|ui| {
        ui.label(RichText::new("Min").size(9.0).color(t.text_secondary));
        ui.add(
            DragValue::new(wait_filter_min)
                .speed(speed.max(0.1))
                .clamp_range(lo..=*wait_filter_max)
                .suffix(&unit_suffix),
        );
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("Max").size(9.0).color(t.text_secondary));
        ui.add(
            DragValue::new(wait_filter_max)
                .speed(speed.max(0.1))
                .clamp_range(*wait_filter_min..=hi)
                .suffix(&unit_suffix),
        );
    });

    ui.add_space(2.0);
    if ui.button(RichText::new("Reset").size(9.0)).clicked() {
        *wait_filter_min = lo;
        *wait_filter_max = hi;
    }
}
