//! Tab Bar Component
//!
//! VS Code–style flat tab strip below the toolbar. One tab per open file.
//! Adapts to the active ViewMode:
//!
//! - **Tab mode**: Single tab strip; clicking switches the viewport file.
//! - **Split mode**: Two independent tab strips (left/right pane), each
//!   selecting which file is shown in its half.
//! - **Overlay mode**: Tabs act as visibility toggles (multi-select);
//!   a filled/hollow dot indicates visible/hidden.

use std::collections::HashSet;

use egui::{Color32, Context, Rect, RichText, Rounding, Stroke, Vec2};
use lucide_icons::Icon as LucideIcon;

use crate::application::ports::ViewMode;
use crate::domain::value_objects::Color;
use crate::presentation::layout::TAB_BAR_HEIGHT;
use crate::presentation::theme;

// ── Output ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct TabBarOutput {
    pub switch_to_tab: Option<usize>,
    pub close_tab: Option<usize>,
    pub view_mode_changed: Option<ViewMode>,
    pub split_partner_changed: Option<usize>,
    pub dragging_tab: Option<usize>,
    pub drag_pos: Option<egui::Pos2>,
    pub drag_released: Option<(usize, egui::Pos2)>,
    pub overlay_toggle: Option<usize>,
    pub overlay_select_all: bool,
    pub overlay_select_only: Option<usize>,
    pub close_others: Option<usize>,
    pub close_all: bool,
    pub show_in_split: Option<usize>,
    pub split_right_switch: Option<usize>,
    pub reorder_tab: Option<(usize, usize)>,
}

// ── Tab info passed in ──────────────────────────────────────────────────

#[derive(Clone)]
pub struct TabInfo {
    pub file_id: usize,
    pub name: String,
    pub color: Color,
    pub is_active: bool,
    pub is_overlay_visible: bool,
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn color_to_egui(c: &Color) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (c.r * 255.0) as u8,
        (c.g * 255.0) as u8,
        (c.b * 255.0) as u8,
        (c.a * 255.0) as u8,
    )
}

// ── Main entry point ─────────────────────────────────────────────────────

pub fn show_tab_bar(
    ctx: &Context,
    tab_bar_rect: &Rect,
    tabs: &[TabInfo],
    view_mode: ViewMode,
    has_multiple_files: bool,
    split_partner_id: Option<usize>,
    split_right_active_id: Option<usize>,
    overlay_visible_ids: &HashSet<usize>,
    split_ratio: f32,
    viewport_width: f32,
) -> TabBarOutput {
    let mut output = TabBarOutput::default();
    let t = theme::active();

    egui::TopBottomPanel::top("tab_bar_panel")
        .exact_height(TAB_BAR_HEIGHT)
        .frame(
            egui::Frame::none()
                .fill(t.toolbar_bg)
                .inner_margin(egui::Margin::ZERO),
        )
        .show(ctx, |ui| {
            // Disable default panel stroke — we draw our own border
            ui.style_mut().visuals.widgets.noninteractive.bg_stroke = Stroke::NONE;
            // Clip children to the panel rect so tabs never paint outside the bar
            ui.set_clip_rect(ui.max_rect());
            ui.spacing_mut().item_spacing.y = 0.0;

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;

                let available_w = ui.available_width();
                let mode_toggle_width = if has_multiple_files { 110.0 } else { 0.0 };

                match view_mode {
                    ViewMode::Split if has_multiple_files => {
                        // Compute left_w from viewport width so the tab bar divider
                        // aligns exactly with the canvas split divider.
                        let divider_w = 1.0;
                        let left_w = (viewport_width * split_ratio - divider_w / 2.0).max(30.0);
                        let right_w = (available_w - left_w - divider_w).max(30.0);
                        let right_active = split_right_active_id.or(split_partner_id);

                        // Partition tabs: right pane gets its active file, left pane gets the rest
                        let (left_tabs, right_tabs): (Vec<&TabInfo>, Vec<&TabInfo>) =
                            tabs.iter().partition(|tab| Some(tab.file_id) != right_active);
                        let left_owned: Vec<TabInfo> = left_tabs.into_iter().cloned().collect();
                        let right_owned: Vec<TabInfo> = right_tabs.into_iter().cloned().collect();

                        // ── Left split zone: fixed width, left-aligned tabs ──
                        let (left_rect, _) = ui.allocate_exact_size(
                            Vec2::new(left_w, TAB_BAR_HEIGHT),
                            egui::Sense::hover(),
                        );
                        // Subtle background tint for left zone
                        let left_bg = if t.is_dark {
                            Color32::from_rgba_unmultiplied(255, 255, 255, 4)
                        } else {
                            Color32::from_rgba_unmultiplied(0, 0, 0, 4)
                        };
                        ui.painter().rect_filled(left_rect, Rounding::ZERO, left_bg);
                        let mut left_ui = ui.child_ui(left_rect, egui::Layout::left_to_right(egui::Align::Min));
                        left_ui.spacing_mut().item_spacing.x = 0.0;
                        draw_tab_strip(&mut left_ui, &left_owned, &t, left_w, TabStripMode::SplitLeft, overlay_visible_ids, &mut output);

                        // ── Divider ──
                        let (div_rect, _) = ui.allocate_exact_size(Vec2::new(divider_w, TAB_BAR_HEIGHT), egui::Sense::hover());
                        ui.painter().line_segment(
                            [div_rect.center_top(), div_rect.center_bottom()],
                            Stroke::new(1.0, t.toolbar_border),
                        );

                        // ── Right split zone: fixed width, left-aligned tab ──
                        let (right_rect, _) = ui.allocate_exact_size(
                            Vec2::new(right_w, TAB_BAR_HEIGHT),
                            egui::Sense::hover(),
                        );
                        // Slightly different background tint for right zone
                        let right_bg = if t.is_dark {
                            Color32::from_rgba_unmultiplied(255, 255, 255, 8)
                        } else {
                            Color32::from_rgba_unmultiplied(0, 0, 0, 8)
                        };
                        ui.painter().rect_filled(right_rect, Rounding::ZERO, right_bg);
                        let mut right_ui = ui.child_ui(right_rect, egui::Layout::left_to_right(egui::Align::Min));
                        right_ui.spacing_mut().item_spacing.x = 0.0;
                        draw_tab_strip(&mut right_ui, &right_owned, &t, right_w, TabStripMode::SplitRight { active_id: right_active }, overlay_visible_ids, &mut output);
                    }
                    ViewMode::Overlay if has_multiple_files => {
                        let tabs_w = (available_w - mode_toggle_width - 16.0).max(60.0);
                        draw_tab_strip(ui, tabs, &t, tabs_w, TabStripMode::Overlay, overlay_visible_ids, &mut output);
                    }
                    _ => {
                        let tabs_w = (available_w - mode_toggle_width - 16.0).max(60.0);
                        draw_tab_strip(ui, tabs, &t, tabs_w, TabStripMode::Standard, overlay_visible_ids, &mut output);
                    }
                }

                if has_multiple_files {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        draw_view_mode_toggle(ui, view_mode, &t, &mut output);
                    });
                }
            });
        });

    output
}

// ── Tab strip modes ──────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum TabStripMode {
    Standard,
    Overlay,
    SplitLeft,
    SplitRight { active_id: Option<usize> },
}

// ── Tab strip drawing ────────────────────────────────────────────────────

fn draw_tab_strip(
    ui: &mut egui::Ui,
    tabs: &[TabInfo],
    t: &theme::ActiveTheme,
    max_width: f32,
    mode: TabStripMode,
    overlay_visible_ids: &HashSet<usize>,
    output: &mut TabBarOutput,
) {
    let strip_id = match mode {
        TabStripMode::SplitRight { .. } => "tab_strip_right",
        TabStripMode::SplitLeft => "tab_strip_left",
        TabStripMode::Overlay => "tab_strip_overlay",
        TabStripMode::Standard => "tab_strip_std",
    };

    let mut tab_rects: Vec<(usize, Rect)> = Vec::with_capacity(tabs.len());

    let _ = egui::ScrollArea::horizontal()
        .max_width(max_width)
        .auto_shrink([true, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .id_source(strip_id)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;

                if matches!(mode, TabStripMode::Overlay) {
                    let all_visible = tabs.iter().all(|tab| overlay_visible_ids.contains(&tab.file_id));
                    let btn_text = if all_visible { "All \u{2713}" } else { "All" };
                    let btn = egui::Button::new(RichText::new(btn_text).size(10.0).color(t.text_secondary))
                        .fill(Color32::TRANSPARENT)
                        .rounding(Rounding::same(3.0))
                        .min_size(Vec2::new(30.0, TAB_BAR_HEIGHT - 4.0));
                    if ui.add(btn).on_hover_text("Toggle all files visible").clicked() {
                        output.overlay_select_all = true;
                    }
                    ui.add_space(2.0);
                }

                for (tab_index, tab) in tabs.iter().enumerate() {
                    let is_active_in_strip = match mode {
                        TabStripMode::Standard | TabStripMode::SplitLeft => tab.is_active,
                        TabStripMode::SplitRight { active_id } => active_id == Some(tab.file_id),
                        TabStripMode::Overlay => tab.is_active,
                    };
                    let is_visible_in_overlay = overlay_visible_ids.contains(&tab.file_id);

                    let tab_out = draw_single_tab(ui, tab, t, mode, is_active_in_strip, is_visible_in_overlay);

                    if let Some(rect) = tab_out.tab_rect {
                        tab_rects.push((tab_index, rect));
                    }

                    match mode {
                        TabStripMode::Standard | TabStripMode::SplitLeft => {
                            if tab_out.clicked { output.switch_to_tab = Some(tab.file_id); }
                        }
                        TabStripMode::SplitRight { .. } => {
                            if tab_out.clicked { output.split_right_switch = Some(tab.file_id); }
                        }
                        TabStripMode::Overlay => {
                            if tab_out.clicked { output.switch_to_tab = Some(tab.file_id); }
                            if tab_out.visibility_toggled { output.overlay_toggle = Some(tab.file_id); }
                        }
                    }

                    if tab_out.close_clicked || tab_out.middle_clicked {
                        output.close_tab = Some(tab.file_id);
                    }
                    if tab_out.dragging {
                        output.dragging_tab = Some(tab.file_id);
                        output.drag_pos = tab_out.drag_pos;
                    }
                    if tab_out.drag_released {
                        if let Some(pos) = tab_out.drag_pos {
                            let in_tab_bar = tab_rects.first()
                                .map(|(_, r)| pos.y >= r.top() - 4.0 && pos.y <= r.bottom() + 4.0)
                                .unwrap_or(false);
                            if in_tab_bar && tabs.len() > 1 {
                                let target_index = tab_rects.iter()
                                    .position(|(_, r)| pos.x < r.center().x)
                                    .unwrap_or(tabs.len());
                                let adjusted = if target_index > tab_index { target_index.saturating_sub(1) } else { target_index };
                                if adjusted != tab_index { output.reorder_tab = Some((tab_index, adjusted)); }
                            } else {
                                output.drag_released = Some((tab.file_id, pos));
                            }
                        }
                    }
                    if tab_out.ctx_close_others { output.close_others = Some(tab.file_id); }
                    if tab_out.ctx_close_all { output.close_all = true; }
                    if tab_out.ctx_select_only { output.overlay_select_only = Some(tab.file_id); }
                }
            });

            // Reorder insertion indicator
            if let Some(drag_id) = output.dragging_tab {
                if let Some(drag_pos) = output.drag_pos {
                    let in_tab_bar = tab_rects.first()
                        .map(|(_, r)| drag_pos.y >= r.top() - 4.0 && drag_pos.y <= r.bottom() + 4.0)
                        .unwrap_or(false);
                    if in_tab_bar && tabs.len() > 1 {
                        let insert_x = tab_rects.iter()
                            .find(|(_, r)| drag_pos.x < r.center().x)
                            .map(|(_, r)| r.left())
                            .unwrap_or_else(|| tab_rects.last().map(|(_, r)| r.right()).unwrap_or(drag_pos.x));
                        let source_idx = tabs.iter().position(|tab| tab.file_id == drag_id);
                        let target_idx = tab_rects.iter().position(|(_, r)| drag_pos.x < r.center().x).unwrap_or(tabs.len());
                        let show = source_idx.map(|si| target_idx != si && target_idx != si + 1).unwrap_or(true);
                        if show {
                            let top = tab_rects.first().map(|(_, r)| r.top() + 4.0).unwrap_or(0.0);
                            let bottom = tab_rects.first().map(|(_, r)| r.bottom() - 4.0).unwrap_or(0.0);
                            ui.painter().line_segment(
                                [egui::pos2(insert_x, top), egui::pos2(insert_x, bottom)],
                                Stroke::new(2.0, t.accent),
                            );
                        }
                    }
                }
            }
        });
}

// ── Single tab ───────────────────────────────────────────────────────────

struct SingleTabOutput {
    clicked: bool,
    middle_clicked: bool,
    close_clicked: bool,
    dragging: bool,
    drag_released: bool,
    drag_pos: Option<egui::Pos2>,
    visibility_toggled: bool,
    tab_rect: Option<Rect>,
    ctx_close_others: bool,
    ctx_close_all: bool,
    ctx_select_only: bool,
}

fn draw_single_tab(
    ui: &mut egui::Ui,
    tab: &TabInfo,
    t: &theme::ActiveTheme,
    mode: TabStripMode,
    is_active_in_strip: bool,
    is_visible_in_overlay: bool,
) -> SingleTabOutput {
    let mut out = SingleTabOutput {
        clicked: false, middle_clicked: false, close_clicked: false,
        dragging: false, drag_released: false, drag_pos: None,
        visibility_toggled: false, tab_rect: None,
        ctx_close_others: false, ctx_close_all: false, ctx_select_only: false,
    };

    let max_name = 20;
    let display_name = if tab.name.len() > max_name {
        format!("{}…", &tab.name[..max_name])
    } else {
        tab.name.clone()
    };

    let swatch_color = color_to_egui(&tab.color);

    // ── Allocate a fixed-width region for the entire tab ──
    // This avoids layout shifts when hover shows/hides the close button.
    let tab_id = ui.id().with("tab").with(tab.file_id);

    // Use a simple allocate + manual painting approach to avoid flicker.
    // We measure content width first to know how wide the tab should be.
    let name_galley = ui.painter().layout_no_wrap(
        display_name.clone(),
        egui::FontId::proportional(11.5),
        t.text_primary,
    );
    let name_w = name_galley.size().x;
    // dot(8) + gap(5) + name + gap(5) + close(16) + padding(20)
    let tab_w = 8.0 + 5.0 + name_w + 5.0 + 16.0 + 20.0;
    let tab_h = TAB_BAR_HEIGHT;

    let (tab_rect, tab_resp) = ui.allocate_exact_size(Vec2::new(tab_w, tab_h), egui::Sense::click_and_drag());
    out.tab_rect = Some(tab_rect);

    let is_hovered = tab_resp.hovered();
    let painter = ui.painter();

    // ── Background fill ──
    let bg = if is_active_in_strip {
        if t.is_dark { Color32::from_rgba_unmultiplied(255, 255, 255, 15) }
        else { Color32::from_rgba_unmultiplied(255, 255, 255, 200) }
    } else if is_hovered {
        if t.is_dark { Color32::from_rgba_unmultiplied(255, 255, 255, 8) }
        else { Color32::from_rgba_unmultiplied(0, 0, 0, 8) }
    } else {
        Color32::TRANSPARENT
    };
    painter.rect_filled(tab_rect, Rounding::ZERO, bg);

    // ── Bottom accent bar (active) ──
    if is_active_in_strip {
        let bar = Rect::from_min_max(
            egui::pos2(tab_rect.left(), tab_rect.bottom() - 2.0),
            egui::pos2(tab_rect.right(), tab_rect.bottom()),
        );
        painter.rect_filled(bar, 0.0, swatch_color);
    }

    // ── Right separator (inactive tabs only) ──
    if !is_active_in_strip {
        let sep_color = if t.is_dark { Color32::from_rgb(55, 55, 55) } else { Color32::from_rgb(210, 210, 210) };
        painter.line_segment(
            [egui::pos2(tab_rect.right(), tab_rect.top() + 6.0), egui::pos2(tab_rect.right(), tab_rect.bottom() - 6.0)],
            Stroke::new(1.0, sep_color),
        );
    }

    // All content positioned relative to tab_rect
    let content_left = tab_rect.left() + 10.0;
    let center_y = tab_rect.center().y;

    // ── Color dot ──
    let dot_center = egui::pos2(content_left + 4.0, center_y);
    if matches!(mode, TabStripMode::Overlay) {
        if is_visible_in_overlay {
            painter.circle_filled(dot_center, 4.5, swatch_color);
        } else {
            painter.circle_stroke(dot_center, 4.5, Stroke::new(1.5, swatch_color));
        }
        // Overlay dot click zone — check tab_resp pointer position
        if tab_resp.clicked() {
            if let Some(pos) = tab_resp.interact_pointer_pos() {
                if (pos.x - dot_center.x).abs() < 8.0 && (pos.y - dot_center.y).abs() < 8.0 {
                    out.visibility_toggled = true;
                }
            }
        }
    } else {
        painter.circle_filled(dot_center, 3.5, swatch_color);
    }

    // ── File name ──
    let name_x = content_left + 13.0;
    let name_color = if matches!(mode, TabStripMode::Overlay) && !is_visible_in_overlay {
        let base = t.text_secondary;
        Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), 100)
    } else if is_active_in_strip {
        t.text_primary
    } else {
        t.text_secondary
    };
    let name_galley = painter.layout_no_wrap(display_name, egui::FontId::proportional(11.5), name_color);
    let name_pos = egui::pos2(name_x, center_y - name_galley.size().y / 2.0);
    painter.galley(name_pos, name_galley, Color32::TRANSPARENT);

    // ── Close button (visible on active or hovered) ──
    if is_active_in_strip || is_hovered {
        let x_center = egui::pos2(tab_rect.right() - 18.0, center_y);
        let x_size = 5.0;
        let x_stroke = Stroke::new(1.2, t.text_secondary);
        painter.line_segment(
            [egui::pos2(x_center.x - x_size, x_center.y - x_size), egui::pos2(x_center.x + x_size, x_center.y + x_size)],
            x_stroke,
        );
        painter.line_segment(
            [egui::pos2(x_center.x + x_size, x_center.y - x_size), egui::pos2(x_center.x - x_size, x_center.y + x_size)],
            x_stroke,
        );
        // Close click zone
        if tab_resp.clicked() {
            if let Some(pos) = tab_resp.interact_pointer_pos() {
                if (pos.x - x_center.x).abs() < 10.0 && (pos.y - x_center.y).abs() < 10.0 {
                    out.close_clicked = true;
                }
            }
        }
    }

    // ── Interactions ──
    if tab_resp.clicked() && !out.close_clicked && !out.visibility_toggled {
        out.clicked = true;
    }
    if tab_resp.middle_clicked() {
        out.middle_clicked = true;
    }
    if tab_resp.dragged() {
        out.dragging = true;
        out.drag_pos = tab_resp.interact_pointer_pos();
    }
    if tab_resp.drag_stopped() {
        out.drag_released = true;
        out.drag_pos = tab_resp.interact_pointer_pos();
    }

    // ── Context menu ──
    tab_resp.context_menu(|ui| {
        if ui.button("Close").clicked() { out.close_clicked = true; ui.close_menu(); }
        if ui.button("Close Others").clicked() { out.ctx_close_others = true; ui.close_menu(); }
        if ui.button("Close All").clicked() { out.ctx_close_all = true; ui.close_menu(); }
        if matches!(mode, TabStripMode::Overlay) {
            ui.separator();
            if ui.button("Show Only This").clicked() { out.ctx_select_only = true; ui.close_menu(); }
        }
    });

    out
}

// ── View-mode toggle ─────────────────────────────────────────────────────

fn draw_view_mode_toggle(
    ui: &mut egui::Ui,
    current: ViewMode,
    t: &theme::ActiveTheme,
    output: &mut TabBarOutput,
) {
    let modes = [
        (ViewMode::Split, LucideIcon::Columns2.unicode(), "Split view"),
        (ViewMode::Tab, LucideIcon::SquareStack.unicode(), "Tab view"),
        (ViewMode::Overlay, LucideIcon::Layers.unicode(), "Overlay view"),
    ];
    for (mode, icon, tooltip) in &modes {
        let selected = current == *mode;
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
        let btn = egui::Button::new(RichText::new(icon.to_string()).size(12.0).color(btn_color))
            .fill(fill)
            .rounding(Rounding::same(4.0))
            .min_size(Vec2::new(26.0, 22.0));
        if ui.add(btn).on_hover_text(*tooltip).clicked() {
            output.view_mode_changed = Some(*mode);
        }
    }
}
