//! Tab Bar Component
//!
//! Secondary header strip below the toolbar showing one tab per open file,
//! a view-mode toggle (Overlay / Tab / Split) on the right, and an optional
//! split-partner selector in Split mode.

use egui::{Color32, Context, Rect, RichText, Rounding, Stroke, Vec2};
use lucide_icons::Icon as LucideIcon;

use crate::application::ports::ViewMode;
use crate::domain::value_objects::Color;
use crate::presentation::layout::TAB_BAR_HEIGHT;
use crate::presentation::theme;

// ── Output ───────────────────────────────────────────────────────────────

/// Actions emitted by the tab bar each frame.
#[derive(Debug, Clone, Default)]
pub struct TabBarOutput {
    /// User clicked a tab → switch to this file ID.
    pub switch_to_tab: Option<usize>,
    /// User clicked the × on a tab → close (hide) this file ID.
    pub close_tab: Option<usize>,
    /// User changed the view mode via the toggle.
    pub view_mode_changed: Option<ViewMode>,
    /// User changed the split partner via the dropdown.
    pub split_partner_changed: Option<usize>,
    /// Tab being dragged (file ID) — set while drag is in progress.
    pub dragging_tab: Option<usize>,
    /// Current screen position of the drag (for drop zone detection).
    pub drag_pos: Option<egui::Pos2>,
    /// Tab drag just released (file ID + final position).
    pub drag_released: Option<(usize, egui::Pos2)>,
}

// ── Tab info passed in ──────────────────────────────────────────────────

/// Lightweight descriptor for one tab; built by the caller from TabManager + FileCollection.
pub struct TabInfo {
    pub file_id: usize,
    pub name: String,
    pub color: Color,
    pub is_active: bool,
}

// ── Render ───────────────────────────────────────────────────────────────

/// Render the tab bar as a TopBottomPanel below the toolbar.
///
/// The `sidebar_width` is used to add left margin so the tab content
/// starts after the sidebar (activity bar + optional content panel).
///
/// Returns actions for the application to handle.
pub fn show_tab_bar(
    ctx: &Context,
    tab_bar_rect: &Rect,
    tabs: &[TabInfo],
    view_mode: ViewMode,
    has_multiple_files: bool,
    split_partner_id: Option<usize>,
) -> TabBarOutput {
    let mut output = TabBarOutput::default();
    let t = theme::active();

    egui::TopBottomPanel::top("tab_bar_panel")
        .exact_height(TAB_BAR_HEIGHT)
        .frame(
            egui::Frame::none()
                .fill(t.toolbar_bg)
                .stroke(Stroke::new(1.0, t.toolbar_border))
                .inner_margin(egui::Margin { left: 4.0, right: 4.0, top: 0.0, bottom: 0.0 }),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;

                // ── Tabs (scrollable if many) ──
                let available_w = tab_bar_rect.width();
                let mode_toggle_width = if has_multiple_files { 110.0 } else { 0.0 };
                let tabs_available_width = (available_w - mode_toggle_width - 16.0).max(60.0);

                egui::ScrollArea::horizontal()
                    .max_width(tabs_available_width)
                    .auto_shrink([true, false])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 1.0;
                            for tab in tabs {
                                let tab_out = draw_single_tab(ui, tab, &t);
                                if tab_out.clicked {
                                    output.switch_to_tab = Some(tab.file_id);
                                }
                                if tab_out.close_clicked {
                                    output.close_tab = Some(tab.file_id);
                                }
                                if tab_out.dragging {
                                    output.dragging_tab = Some(tab.file_id);
                                    output.drag_pos = tab_out.drag_pos;
                                }
                                if tab_out.drag_released {
                                    if let Some(pos) = tab_out.drag_pos {
                                        output.drag_released = Some((tab.file_id, pos));
                                    }
                                }
                            }
                        });
                    });

                // ── Right side: view mode toggle ──
                if has_multiple_files {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        draw_view_mode_toggle(ui, view_mode, &t, &mut output);

                        // Split partner selector (only in Split mode)
                        if view_mode == ViewMode::Split {
                            draw_split_partner_selector(
                                ui, tabs, split_partner_id, &t, &mut output,
                            );
                        }
                    });
                }
            });
        });

    output
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Output from drawing a single tab.
struct SingleTabOutput {
    clicked: bool,
    close_clicked: bool,
    dragging: bool,
    drag_released: bool,
    drag_pos: Option<egui::Pos2>,
}

/// Draw one tab button with click and drag support.
fn draw_single_tab(
    ui: &mut egui::Ui,
    tab: &TabInfo,
    t: &theme::ActiveTheme,
) -> SingleTabOutput {
    let mut out = SingleTabOutput {
        clicked: false,
        close_clicked: false,
        dragging: false,
        drag_released: false,
        drag_pos: None,
    };

    let max_name = 20;
    let display_name = if tab.name.len() > max_name {
        format!("{}…", &tab.name[..max_name])
    } else {
        tab.name.clone()
    };

    let tab_fill = if tab.is_active {
        if t.is_dark {
            Color32::from_rgba_unmultiplied(255, 255, 255, 15)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, 200)
        }
    } else {
        Color32::TRANSPARENT
    };

    let swatch_color = Color32::from_rgba_unmultiplied(
        (tab.color.r * 255.0) as u8,
        (tab.color.g * 255.0) as u8,
        (tab.color.b * 255.0) as u8,
        (tab.color.a * 255.0) as u8,
    );

    let resp = egui::Frame::none()
        .fill(tab_fill)
        .rounding(Rounding { nw: 4.0, ne: 4.0, sw: 0.0, se: 0.0 })
        .inner_margin(egui::Margin::symmetric(8.0, 0.0))
        .show(ui, |ui| {
            ui.set_height(TAB_BAR_HEIGHT - 2.0);
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;

                // Color indicator dot
                let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                ui.painter().circle_filled(dot_rect.center(), 4.0, swatch_color);

                // File name — supports click and drag
                let name_color = if tab.is_active { t.text_primary } else { t.text_secondary };
                let name_resp = ui.add(
                    egui::Label::new(
                        RichText::new(&display_name).size(11.5).color(name_color),
                    )
                    .selectable(false)
                    .sense(egui::Sense::click_and_drag()),
                );
                if name_resp.clicked() {
                    out.clicked = true;
                }
                if name_resp.dragged() {
                    out.dragging = true;
                    out.drag_pos = name_resp.interact_pointer_pos();
                }
                if name_resp.drag_stopped() {
                    out.drag_released = true;
                    out.drag_pos = name_resp.interact_pointer_pos();
                }

                // Close button (× icon)
                let x_icon = LucideIcon::X.unicode().to_string();
                let x_color = if tab.is_active { t.text_secondary } else {
                    Color32::from_rgba_unmultiplied(
                        t.text_secondary.r(), t.text_secondary.g(), t.text_secondary.b(), 100,
                    )
                };
                let x_btn = egui::Button::new(
                    RichText::new(&x_icon).size(10.0).color(x_color),
                )
                .fill(Color32::TRANSPARENT)
                .rounding(Rounding::same(3.0))
                .min_size(Vec2::new(16.0, 16.0));
                if ui.add(x_btn).on_hover_text("Close tab").clicked() {
                    out.close_clicked = true;
                }
            });

            // Active tab bottom accent bar
            if tab.is_active {
                let rect = ui.min_rect();
                let bar_rect = Rect::from_min_max(
                    egui::pos2(rect.left(), rect.bottom() - 2.0),
                    egui::pos2(rect.right(), rect.bottom()),
                );
                ui.painter().rect_filled(bar_rect, 0.0, swatch_color);
            }
        });

    // Click on the background area of the tab frame also switches
    if resp.response.clicked() {
        out.clicked = true;
    }
    // Drag on the background area
    if resp.response.dragged() {
        out.dragging = true;
        if out.drag_pos.is_none() {
            out.drag_pos = resp.response.interact_pointer_pos();
        }
    }
    if resp.response.drag_stopped() {
        out.drag_released = true;
        if out.drag_pos.is_none() {
            out.drag_pos = resp.response.interact_pointer_pos();
        }
    }

    out
}

/// Draw Overlay / Tab / Split toggle buttons.
fn draw_view_mode_toggle(
    ui: &mut egui::Ui,
    current: ViewMode,
    t: &theme::ActiveTheme,
    output: &mut TabBarOutput,
) {
    let modes = [
        (ViewMode::Split, LucideIcon::Columns2.unicode(), "Split"),
        (ViewMode::Tab, LucideIcon::SquareStack.unicode(), "Tab"),
        (ViewMode::Overlay, LucideIcon::Layers.unicode(), "Overlay"),
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
        let btn = egui::Button::new(
            RichText::new(icon.to_string()).size(12.0).color(btn_color),
        )
        .fill(fill)
        .rounding(Rounding::same(4.0))
        .min_size(Vec2::new(26.0, 22.0));
        if ui.add(btn).on_hover_text(*tooltip).clicked() {
            output.view_mode_changed = Some(*mode);
        }
    }
}

/// Draw a small combo-box to pick the split partner file.
fn draw_split_partner_selector(
    ui: &mut egui::Ui,
    tabs: &[TabInfo],
    current_partner: Option<usize>,
    t: &theme::ActiveTheme,
    output: &mut TabBarOutput,
) {
    // Only show non-active tabs as partner candidates
    let candidates: Vec<_> = tabs.iter().filter(|tab| !tab.is_active).collect();
    if candidates.is_empty() {
        return;
    }

    let partner_name = current_partner
        .and_then(|pid| tabs.iter().find(|tab| tab.file_id == pid))
        .map(|tab| {
            if tab.name.len() > 12 {
                format!("{}…", &tab.name[..12])
            } else {
                tab.name.clone()
            }
        })
        .unwrap_or_else(|| "Select…".to_string());

    let split_icon = LucideIcon::Split.unicode().to_string();
    egui::ComboBox::from_id_source("split_partner_combo")
        .width(100.0)
        .selected_text(
            RichText::new(format!("{} {}", split_icon, partner_name))
                .size(10.0)
                .color(t.text_secondary),
        )
        .show_ui(ui, |ui: &mut egui::Ui| {
            for cand in &candidates {
                let sel = current_partner == Some(cand.file_id);
                if ui.selectable_label(sel, &cand.name).clicked() {
                    output.split_partner_changed = Some(cand.file_id);
                }
            }
        });
}
