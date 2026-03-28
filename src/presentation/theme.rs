//! Theme Module
//!
//! Shared colors, font setup, and reusable UI helper widgets used across all components.

use egui::{Color32, Context, FontData, FontDefinitions, FontFamily, RichText, Rounding, Stroke, Vec2};
use lucide_icons::LUCIDE_FONT_BYTES;

// ── Font constants ───────────────────────────────────────────────────────

/// Lucide font family name
pub const LUCIDE_FONT: &str = "lucide";

// ── Theme colors ─────────────────────────────────────────────────────────

pub const TOOLBAR_BG: Color32 = Color32::from_rgb(245, 245, 245);           // #F5F5F5
pub const TOOLBAR_BORDER: Color32 = Color32::from_rgb(200, 200, 200);       // #C8C8C8
pub const ACCENT: Color32 = Color32::from_rgb(25, 118, 210);                // #1976D2
pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(187, 222, 251);         // #BBDEFB
pub const TOGGLE_ACTIVE_BG: Color32 = Color32::from_rgb(200, 230, 255);     // light blue
pub const TOGGLE_INACTIVE_BG: Color32 = Color32::from_rgb(230, 230, 230);   // #E6E6E6
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(33, 33, 33);            // #212121
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(97, 97, 97);          // #616161
pub const PANEL_BG: Color32 = Color32::from_rgb(255, 255, 255);             // white
pub const PANEL_SHADOW: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 40);

/// Semi-transparent white for floating panel backgrounds
pub const PANEL_BG_TRANSLUCENT: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 230);

// ── Font setup ───────────────────────────────────────────────────────────

/// Set up fonts including the Lucide icon font
pub fn setup_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();

    // Add lucide font
    fonts.font_data.insert(
        LUCIDE_FONT.to_owned(),
        FontData::from_static(LUCIDE_FONT_BYTES),
    );

    // Add lucide as a fallback for proportional fonts so icons render in text
    fonts.families
        .entry(FontFamily::Proportional)
        .or_default()
        .push(LUCIDE_FONT.to_owned());

    // Also register as its own family for explicit use
    fonts.families.insert(
        FontFamily::Name(LUCIDE_FONT.into()),
        vec![LUCIDE_FONT.to_owned()],
    );

    ctx.set_fonts(fonts);
}

/// Apply the light theme to the egui context
pub fn apply_light_theme(ctx: &Context) {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = TOOLBAR_BG;
    visuals.window_fill = PANEL_BG;
    visuals.window_rounding = Rounding::same(8.0);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 2.0),
        blur: 8.0,
        spread: 0.0,
        color: PANEL_SHADOW,
    };
    visuals.widgets.inactive.bg_fill = TOGGLE_INACTIVE_BG;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.inactive.rounding = Rounding::same(6.0);
    visuals.widgets.hovered.bg_fill = ACCENT_LIGHT;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.hovered.rounding = Rounding::same(6.0);
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.active.rounding = Rounding::same(6.0);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(12.0);
    // Disable all animations to prevent slide-in / fade effects
    style.animation_time = 0.0;
    ctx.set_style(style);
}

// ── Reusable widget helpers ──────────────────────────────────────────────

/// Draw a toolbar toggle button (icon + active state). Returns true if clicked.
pub fn toolbar_toggle(ui: &mut egui::Ui, icon: &str, active: &mut bool, tooltip: &str) -> bool {
    let fill = if *active { TOGGLE_ACTIVE_BG } else { Color32::TRANSPARENT };
    let text_color = if *active { ACCENT } else { TEXT_PRIMARY };
    let btn = egui::Button::new(RichText::new(icon).size(20.0).color(text_color))
        .fill(fill)
        .rounding(Rounding::same(6.0))
        .min_size(Vec2::new(36.0, 36.0));
    let response = ui.add(btn).on_hover_text(tooltip);
    if response.clicked() {
        *active = !*active;
    }
    response.clicked()
}

/// Draw a toolbar toggle button using an SVG image (tinted by active state). Returns true if clicked.
pub fn toolbar_toggle_image(
    ui: &mut egui::Ui,
    image_source: egui::ImageSource<'_>,
    active: &mut bool,
    tooltip: &str,
) -> bool {
    let fill = if *active { TOGGLE_ACTIVE_BG } else { Color32::TRANSPARENT };
    let tint = if *active { ACCENT } else { TEXT_PRIMARY };
    let image = egui::Image::new(image_source)
        .fit_to_exact_size(egui::vec2(20.0, 20.0))
        .tint(tint);
    let btn = egui::Button::image(image)
        .fill(fill)
        .rounding(Rounding::same(6.0))
        .min_size(Vec2::new(36.0, 36.0));
    let response = ui.add(btn).on_hover_text(tooltip);
    if response.clicked() {
        *active = !*active;
    }
    response.clicked()
}

/// Draw a toolbar mode button (for parameter mode selection). Returns true if clicked.
pub fn toolbar_mode_btn(ui: &mut egui::Ui, label: &str, is_selected: bool, tooltip: &str) -> bool {
    let fill = if is_selected { ACCENT } else { TOGGLE_INACTIVE_BG };
    let text_color = if is_selected { Color32::WHITE } else { TEXT_PRIMARY };
    let btn = egui::Button::new(RichText::new(label).size(12.0).color(text_color))
        .fill(fill)
        .rounding(Rounding::same(6.0))
        .min_size(Vec2::new(0.0, 30.0));
    let response = ui.add(btn).on_hover_text(tooltip);
    response.clicked()
}

/// Standard floating panel frame (translucent white with shadow and rounding)
pub fn floating_panel_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(PANEL_BG_TRANSLUCENT)
        .rounding(Rounding::same(10.0))
        .shadow(egui::epaint::Shadow {
            offset: egui::vec2(0.0, 2.0),
            blur: 8.0,
            spread: 0.0,
            color: PANEL_SHADOW,
        })
        .inner_margin(egui::Margin::symmetric(8.0, 8.0))
}
