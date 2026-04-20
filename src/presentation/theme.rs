//! Theme Module
//!
//! Shared colors, font setup, and reusable UI helper widgets used across all
//! components.  The `ActiveTheme` struct holds palette-derived `Color32`
//! values that every component reads.  The static `ACTIVE` cell is set once
//! per palette switch via `apply_theme()`.

use egui::{Color32, Context, FontData, FontDefinitions, FontFamily, RichText, Rounding, Stroke, Vec2};
use lucide_icons::LUCIDE_FONT_BYTES;
use std::sync::OnceLock;
use std::sync::RwLock;

use crate::domain::value_objects::Color;
use crate::presentation::palette::ThemePalette;

// ── Font constants ───────────────────────────────────────────────────────

pub const LUCIDE_FONT: &str = "lucide";

// ── Active theme (global, palette-driven) ────────────────────────────────

/// Convert domain `Color` (f32 RGBA) to egui `Color32` (u8 RGBA).
pub fn to_color32(c: &Color) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (c.r * 255.0) as u8,
        (c.g * 255.0) as u8,
        (c.b * 255.0) as u8,
        (c.a * 255.0) as u8,
    )
}

/// Runtime-resolved egui colors derived from the current `ThemePalette`.
#[derive(Debug, Clone)]
pub struct ActiveTheme {
    pub toolbar_bg: Color32,
    pub toolbar_border: Color32,
    pub accent: Color32,
    pub accent_light: Color32,
    pub toggle_active_bg: Color32,
    pub toggle_inactive_bg: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub panel_bg: Color32,
    pub panel_bg_translucent: Color32,
    pub panel_shadow: Color32,
    pub is_dark: bool,
}

impl ActiveTheme {
    pub fn from_palette(p: &ThemePalette) -> Self {
        Self {
            toolbar_bg: to_color32(&p.toolbar_bg),
            toolbar_border: to_color32(&p.toolbar_border),
            accent: to_color32(&p.accent),
            accent_light: to_color32(&p.accent_light),
            toggle_active_bg: to_color32(&p.toggle_active_bg),
            toggle_inactive_bg: to_color32(&p.toggle_inactive_bg),
            text_primary: to_color32(&p.text_primary),
            text_secondary: to_color32(&p.text_secondary),
            panel_bg: to_color32(&p.panel_bg),
            panel_bg_translucent: to_color32(&p.panel_bg_translucent),
            panel_shadow: to_color32(&p.panel_shadow),
            is_dark: p.is_dark,
        }
    }
}

impl Default for ActiveTheme {
    fn default() -> Self {
        Self {
            toolbar_bg: Color32::from_rgb(245, 245, 245),
            toolbar_border: Color32::from_rgb(200, 200, 200),
            accent: Color32::from_rgb(25, 118, 210),
            accent_light: Color32::from_rgb(187, 222, 251),
            toggle_active_bg: Color32::from_rgb(200, 230, 255),
            toggle_inactive_bg: Color32::from_rgb(230, 230, 230),
            text_primary: Color32::from_rgb(33, 33, 33),
            text_secondary: Color32::from_rgb(97, 97, 97),
            panel_bg: Color32::from_rgb(255, 255, 255),
            panel_bg_translucent: Color32::from_rgba_premultiplied(255, 255, 255, 230),
            panel_shadow: Color32::from_rgba_premultiplied(0, 0, 0, 40),
            is_dark: false,
        }
    }
}

/// Global active theme (read via `active()`).
static ACTIVE: OnceLock<RwLock<ActiveTheme>> = OnceLock::new();

fn theme_lock() -> &'static RwLock<ActiveTheme> {
    ACTIVE.get_or_init(|| RwLock::new(ActiveTheme::default()))
}

/// Get a snapshot of the current active theme.
pub fn active() -> ActiveTheme {
    theme_lock().read().unwrap().clone()
}

// ── Font setup ───────────────────────────────────────────────────────────

/// Set up fonts including the Lucide icon font.
pub fn setup_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        LUCIDE_FONT.to_owned(),
        FontData::from_static(LUCIDE_FONT_BYTES),
    );

    fonts.families
        .entry(FontFamily::Proportional)
        .or_default()
        .push(LUCIDE_FONT.to_owned());

    fonts.families.insert(
        FontFamily::Name(LUCIDE_FONT.into()),
        vec![LUCIDE_FONT.to_owned()],
    );

    ctx.set_fonts(fonts);
}

// ── Theme application ────────────────────────────────────────────────────

/// Apply a palette to egui visuals and update the global `ActiveTheme`.
pub fn apply_theme(ctx: &Context, palette: &ThemePalette) {
    let at = ActiveTheme::from_palette(palette);

    // Store globally
    {
        let mut lock = theme_lock().write().unwrap();
        *lock = at.clone();
    }

    // Build egui visuals
    let mut visuals = if palette.is_dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.panel_fill = at.toolbar_bg;
    visuals.window_fill = at.panel_bg;
    visuals.window_rounding = Rounding::same(8.0);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 2.0),
        blur: 8.0,
        spread: 0.0,
        color: at.panel_shadow,
    };
    visuals.widgets.inactive.bg_fill = at.toggle_inactive_bg;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, at.text_primary);
    visuals.widgets.inactive.rounding = Rounding::same(6.0);
    visuals.widgets.hovered.bg_fill = at.accent_light;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, at.accent);
    visuals.widgets.hovered.rounding = Rounding::same(6.0);
    visuals.widgets.active.bg_fill = at.accent;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, if palette.is_dark { Color32::BLACK } else { Color32::WHITE });
    visuals.widgets.active.rounding = Rounding::same(6.0);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, at.text_primary);
    visuals.selection.bg_fill = at.accent;
    visuals.selection.stroke = Stroke::new(1.0, if palette.is_dark { Color32::BLACK } else { Color32::WHITE });
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(12.0);
    style.animation_time = 0.0;
    ctx.set_style(style);
}

/// Legacy: apply the light-default theme.
pub fn apply_light_theme(ctx: &Context) {
    apply_theme(ctx, &ThemePalette::default());
}

// ── Reusable widget helpers ──────────────────────────────────────────────

/// Draw a toolbar toggle button (icon + active state). Returns true if clicked.
pub fn toolbar_toggle(ui: &mut egui::Ui, icon: &str, active: &mut bool, tooltip: &str) -> bool {
    let t = self::active();
    let fill = if *active { t.toggle_active_bg } else { Color32::TRANSPARENT };
    let text_color = if *active { t.accent } else { t.text_primary };
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
    let t = self::active();
    let fill = if *active { t.toggle_active_bg } else { Color32::TRANSPARENT };
    let tint = if *active { t.accent } else { t.text_primary };
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
    let t = active();
    let fill = if is_selected { t.accent } else { t.toggle_inactive_bg };
    let text_color = if is_selected {
        if t.is_dark { Color32::BLACK } else { Color32::WHITE }
    } else {
        t.text_primary
    };
    let btn = egui::Button::new(RichText::new(label).size(12.0).color(text_color))
        .fill(fill)
        .rounding(Rounding::same(6.0))
        .min_size(Vec2::new(0.0, 30.0));
    let response = ui.add(btn).on_hover_text(tooltip);
    response.clicked()
}

/// Standard floating panel frame (uses active theme colors).
pub fn floating_panel_frame() -> egui::Frame {
    let t = active();
    egui::Frame::none()
        .fill(t.panel_bg_translucent)
        .rounding(Rounding::same(10.0))
        .shadow(egui::epaint::Shadow {
            offset: egui::vec2(0.0, 2.0),
            blur: 8.0,
            spread: 0.0,
            color: t.panel_shadow,
        })
        .inner_margin(egui::Margin::symmetric(8.0, 8.0))
}
