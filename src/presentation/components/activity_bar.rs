//! Activity Bar Component
//!
//! VS Code–style vertical icon strip on the left edge of the window.
//! Always visible (even when the sidebar content panel is collapsed).
//! Each icon represents a sidebar tab (Toolpaths, Parameter Legend, etc.).
//! Clicking the active tab toggles the content panel; clicking an
//! inactive tab switches to it and opens the content panel.
//!
//! Tab list is data-driven — adding a new tab is a single-line addition
//! to the `SIDEBAR_TABS` array.

use egui::{Color32, Context, Rounding, Stroke, Vec2, RichText};
use lucide_icons::Icon as LucideIcon;

use crate::presentation::layout::ACTIVITY_BAR_WIDTH;
use crate::presentation::theme;
use crate::presentation::ui::SidebarTab;

/// Metadata for a sidebar tab rendered in the activity bar.
struct SidebarTabInfo {
    tab: SidebarTab,
    icon: LucideIcon,
    tooltip: &'static str,
}

/// Registry of all sidebar tabs. Add new tabs here.
const SIDEBAR_TABS: &[SidebarTabInfo] = &[
    SidebarTabInfo { tab: SidebarTab::Toolpaths, icon: LucideIcon::Files, tooltip: "Toolpaths" },
    SidebarTabInfo { tab: SidebarTab::ParameterLegend, icon: LucideIcon::Palette, tooltip: "Parameter Legend" },
];

/// Output from the activity bar component
#[derive(Debug, Clone, Default)]
pub struct ActivityBarOutput {
    /// Tab that was clicked (if any)
    pub clicked_tab: Option<SidebarTab>,
}

/// Render the activity bar.
///
/// The activity bar is a narrow vertical strip left of the sidebar content panel.
/// It is always visible and shows one icon per tab.
pub fn show_activity_bar(
    ctx: &Context,
    active_tab: SidebarTab,
    sidebar_open: bool,
) -> ActivityBarOutput {
    let mut output = ActivityBarOutput::default();
    let t = theme::active();

    // Activity bar background — slightly darker than sidebar (VS Code style)
    let bar_bg = if t.is_dark {
        Color32::from_rgb(
            t.toolbar_bg.r().saturating_sub(18),
            t.toolbar_bg.g().saturating_sub(18),
            t.toolbar_bg.b().saturating_sub(18),
        )
    } else {
        Color32::from_rgb(
            t.toolbar_bg.r().saturating_sub(12),
            t.toolbar_bg.g().saturating_sub(12),
            t.toolbar_bg.b().saturating_sub(12),
        )
    };

    egui::SidePanel::left("activity_bar")
        .exact_width(ACTIVITY_BAR_WIDTH)
        .resizable(false)
        .frame(
            egui::Frame::none()
                .fill(bar_bg)
                .stroke(Stroke::NONE)
                .inner_margin(egui::Margin::symmetric(0.0, 0.0)),
        )
        .show(ctx, |ui| {
            ui.add_space(4.0);

            ui.vertical_centered(|ui| {
                for info in SIDEBAR_TABS {
                    let is_active = info.tab == active_tab && sidebar_open;

                    // Button fill: active gets subtle highlight
                    let btn_fill = if is_active {
                        Color32::from_rgba_unmultiplied(
                            t.accent.r(),
                            t.accent.g(),
                            t.accent.b(),
                            if t.is_dark { 25 } else { 20 },
                        )
                    } else {
                        Color32::TRANSPARENT
                    };

                    // Icon color: active = primary text, inactive = dimmed
                    let icon_color = if is_active {
                        t.text_primary
                    } else {
                        t.text_secondary
                    };

                    let icon_str = info.icon.unicode().to_string();
                    let btn = egui::Button::new(
                        RichText::new(icon_str)
                            .size(18.0)
                            .color(icon_color),
                    )
                    .fill(btn_fill)
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(ACTIVITY_BAR_WIDTH - 10.0, 38.0));

                    let response = ui.add(btn).on_hover_text(info.tooltip);

                    // Active indicator — left accent border (VS Code style)
                    if is_active {
                        let rect = response.rect;
                        let bar_left = ui.min_rect().left();
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(
                                egui::pos2(bar_left, rect.top() + 6.0),
                                egui::vec2(2.5, rect.height() - 12.0),
                            ),
                            1.0,
                            t.accent,
                        );
                    }

                    if response.clicked() {
                        output.clicked_tab = Some(info.tab);
                    }

                    ui.add_space(2.0);
                }
            });

            // Right-edge separator line between activity bar and content panel
            let panel_rect = ui.max_rect();
            ui.painter().line_segment(
                [
                    egui::pos2(panel_rect.right(), panel_rect.top()),
                    egui::pos2(panel_rect.right(), panel_rect.bottom()),
                ],
                Stroke::new(1.0, t.toolbar_border),
            );
        });

    output
}
