//! Palette Module
//!
//! Defines the resolved `ThemePalette` struct and curated palette definitions.
//! Theme mode, palette ID, and custom config types live in
//! `application::ports::config_port` to avoid circular dependencies.

use crate::domain::value_objects::Color;

// Re-export from application layer for convenience
pub use crate::application::ports::{
    CustomPaletteConfig, GradientPaletteId, PaletteId, ThemeMode, ALL_PALETTE_IDS,
};

// ── Resolved mode ───────────────────────────────────────────────────────

/// Resolved (non-System) mode used when building a palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedMode {
    Light,
    Dark,
}

/// Resolve a `ThemeMode` to a concrete `ResolvedMode` given an OS dark-mode hint.
pub fn resolve_mode(mode: ThemeMode, system_is_dark: bool) -> ResolvedMode {
    match mode {
        ThemeMode::Light => ResolvedMode::Light,
        ThemeMode::Dark => ResolvedMode::Dark,
        ThemeMode::System => {
            if system_is_dark { ResolvedMode::Dark } else { ResolvedMode::Light }
        }
    }
}

/// Return the palette list with display names.
pub fn available_palettes() -> Vec<(PaletteId, &'static str)> {
    ALL_PALETTE_IDS.iter().map(|id| (*id, id.label())).collect()
}

// ── Resolved theme palette ──────────────────────────────────────────────

/// The full resolved color set consumed by the UI and renderer.
#[derive(Debug, Clone)]
pub struct ThemePalette {
    // ── Canvas / renderer ──
    pub background: Color,
    pub grid_minor: Color,
    pub grid_major: Color,

    // ── Vector type colors ──
    pub boundary: Color,
    pub contour: Color,
    pub base_contour: Color,
    pub depth_contour: Color,
    pub hatch: Color,
    pub support: Color,
    pub travel: Color,

    // ── Parameter gradient stops (replaces viridis) ──
    pub gradient_stops: Vec<(f32, Color)>,

    // ── UI chrome ──
    pub accent: Color,
    pub accent_light: Color,
    pub toolbar_bg: Color,
    pub toolbar_border: Color,
    pub panel_bg: Color,
    pub panel_bg_translucent: Color,
    pub panel_shadow: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub toggle_active_bg: Color,
    pub toggle_inactive_bg: Color,

    /// Whether this palette uses a dark base (affects egui Visuals base).
    pub is_dark: bool,
}

impl Default for ThemePalette {
    fn default() -> Self {
        light_default()
    }
}

// ── Gradient helper ─────────────────────────────────────────────────────

/// Evaluate the palette's gradient colormap at parameter `t` ∈ [0, 1].
pub fn gradient_color(palette: &ThemePalette, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let stops = &palette.gradient_stops;
    if stops.is_empty() {
        return Color::WHITE;
    }
    if stops.len() == 1 || t <= stops[0].0 {
        return stops[0].1;
    }
    if t >= stops[stops.len() - 1].0 {
        return stops[stops.len() - 1].1;
    }
    for i in 0..stops.len() - 1 {
        let (t0, c0) = &stops[i];
        let (t1, c1) = &stops[i + 1];
        if t >= *t0 && t <= *t1 {
            let s = if (t1 - t0).abs() < 1e-6 {
                0.0
            } else {
                (t - t0) / (t1 - t0)
            };
            return c0.blend(c1, s);
        }
    }
    stops[stops.len() - 1].1
}

/// Evaluate a gradient colormap at parameter `t` ∈ [0, 1] using explicit stops.
pub fn gradient_color_from_stops(stops: &[(f32, Color)], t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    if stops.is_empty() {
        return Color::WHITE;
    }
    if stops.len() == 1 || t <= stops[0].0 {
        return stops[0].1;
    }
    if t >= stops[stops.len() - 1].0 {
        return stops[stops.len() - 1].1;
    }
    for i in 0..stops.len() - 1 {
        let (t0, c0) = &stops[i];
        let (t1, c1) = &stops[i + 1];
        if t >= *t0 && t <= *t1 {
            let s = if (t1 - t0).abs() < 1e-6 {
                0.0
            } else {
                (t - t0) / (t1 - t0)
            };
            return c0.blend(c1, s);
        }
    }
    stops[stops.len() - 1].1
}

/// Resolve gradient stops for a given `GradientPaletteId`.
pub fn resolve_gradient_stops(id: GradientPaletteId) -> Vec<(f32, Color)> {
    match id {
        GradientPaletteId::Viridis => viridis_stops(),
        GradientPaletteId::Cividis => cividis_stops(),
        GradientPaletteId::Turbo => turbo_stops(),
        GradientPaletteId::Ocean => ocean_stops(),
        GradientPaletteId::Inferno => inferno_stops(),
    }
}

// ── Resolve palette ─────────────────────────────────────────────────────

/// Build a `ThemePalette` for the given resolved mode and palette id.
/// For `Custom`, supply the optional persisted config; falls back to defaults.
pub fn resolve_palette(
    mode: ResolvedMode,
    id: PaletteId,
    custom: Option<&CustomPaletteConfig>,
) -> ThemePalette {
    match mode {
        ResolvedMode::Light => match id {
            PaletteId::Default => light_default(),
            PaletteId::Professional => light_professional(),
            PaletteId::Vibrant => light_vibrant(),
            PaletteId::Ocean => light_ocean(),
            PaletteId::Warm => light_warm(),
            PaletteId::ColorblindRG => light_colorblind_rg(),
            PaletteId::ColorblindBY => light_colorblind_by(),
            PaletteId::HighContrast => light_high_contrast(),
            PaletteId::Custom => light_custom(custom),
        },
        ResolvedMode::Dark => match id {
            PaletteId::Default => dark_default(),
            PaletteId::Professional => dark_professional(),
            PaletteId::Vibrant => dark_vibrant(),
            PaletteId::Ocean => dark_ocean(),
            PaletteId::Warm => dark_warm(),
            PaletteId::ColorblindRG => dark_colorblind_rg(),
            PaletteId::ColorblindBY => dark_colorblind_by(),
            PaletteId::HighContrast => dark_high_contrast(),
            PaletteId::Custom => dark_custom(custom),
        },
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Gradient stop presets
// ═══════════════════════════════════════════════════════════════════════

fn viridis_stops() -> Vec<(f32, Color)> {
    vec![
        (0.000, c("#440154")),
        (0.333, c("#31688E")),
        (0.667, c("#35B779")),
        (1.000, c("#FDE725")),
    ]
}

fn cividis_stops() -> Vec<(f32, Color)> {
    vec![
        (0.000, c("#002051")),
        (0.250, c("#3B4F78")),
        (0.500, c("#80804B")),
        (0.750, c("#C0B524")),
        (1.000, c("#FDEA45")),
    ]
}

fn turbo_stops() -> Vec<(f32, Color)> {
    vec![
        (0.000, c("#30123B")),
        (0.170, c("#4662D7")),
        (0.330, c("#36AAF9")),
        (0.500, c("#1BD0D5")),
        (0.670, c("#6FE62B")),
        (0.830, c("#CEBB19")),
        (1.000, c("#F9FB0E")),
    ]
}

fn ocean_stops() -> Vec<(f32, Color)> {
    vec![
        (0.000, c("#0D1B2A")),
        (0.250, c("#1B3A5C")),
        (0.500, c("#1B7A8E")),
        (0.750, c("#48CAE4")),
        (1.000, c("#CAF0F8")),
    ]
}

fn inferno_stops() -> Vec<(f32, Color)> {
    vec![
        (0.000, c("#000004")),
        (0.250, c("#420A68")),
        (0.500, c("#BC3754")),
        (0.750, c("#F98C0A")),
        (1.000, c("#FCFFA4")),
    ]
}

fn batlow_stops() -> Vec<(f32, Color)> {
    vec![
        (0.000, c("#011959")),
        (0.250, c("#3C5068")),
        (0.500, c("#A38E6C")),
        (0.750, c("#D9A86C")),
        (1.000, c("#FAE69E")),
    ]
}

fn bw_stops() -> Vec<(f32, Color)> {
    vec![
        (0.000, c("#000000")),
        (1.000, c("#FFFFFF")),
    ]
}

fn white_yellow_stops() -> Vec<(f32, Color)> {
    vec![
        (0.000, c("#FFFFFF")),
        (0.500, c("#FFD700")),
        (1.000, c("#FFFF00")),
    ]
}

// ═══════════════════════════════════════════════════════════════════════
//  Light mode palettes
// ═══════════════════════════════════════════════════════════════════════

fn light_chrome(accent: Color, accent_light: Color) -> ThemePalette {
    ThemePalette {
        background: Color::WHITE,
        grid_minor: Color::new(0.0, 0.0, 0.0, 0.08),
        grid_major: Color::new(0.0, 0.0, 0.0, 0.20),
        // vector colors filled by caller
        boundary: Color::BLACK,
        contour: Color::CYAN,
        base_contour: Color::CYAN,
        depth_contour: Color::MAGENTA,
        hatch: Color::RED,
        support: Color::GRAY,
        travel: Color::GREEN.with_alpha(0.4),
        gradient_stops: viridis_stops(),
        accent,
        accent_light,
        toolbar_bg: c("#F5F5F5"),
        toolbar_border: c("#C8C8C8"),
        panel_bg: Color::WHITE,
        panel_bg_translucent: Color::new(1.0, 1.0, 1.0, 0.90),
        panel_shadow: Color::new(0.0, 0.0, 0.0, 0.157),
        text_primary: c("#212121"),
        text_secondary: c("#616161"),
        toggle_active_bg: Color::new(0.78, 0.90, 1.0, 1.0),
        toggle_inactive_bg: c("#E6E6E6"),
        is_dark: false,
    }
}

fn light_default() -> ThemePalette {
    let mut p = light_chrome(c("#1976D2"), c("#BBDEFB"));
    p.background = c("#FAFAFA");
    p.boundary = c("#2C3E50");
    p.contour = c("#00BCD4");
    p.base_contour = c("#009688");
    p.depth_contour = c("#AD1457");
    p.hatch = c("#E91E63");
    p.support = c("#616161");
    p.travel = c("#388E3C").with_alpha(0.4);
    p.gradient_stops = viridis_stops();
    p
}

fn light_professional() -> ThemePalette {
    let mut p = light_chrome(c("#455A64"), c("#CFD8DC"));
    p.background = c("#F5F5F5");
    p.boundary = c("#37474F");
    p.contour = c("#546E7A");
    p.base_contour = c("#607D8B");
    p.depth_contour = c("#795548");
    p.hatch = c("#8D6E63");
    p.support = c("#9E9E9E");
    p.travel = c("#78909C").with_alpha(0.4);
    p.gradient_stops = cividis_stops();
    p
}

fn light_vibrant() -> ThemePalette {
    let mut p = light_chrome(c("#6200EA"), c("#D1C4E9"));
    p.background = Color::WHITE;
    p.boundary = c("#1B5E20");
    p.contour = c("#0277BD");
    p.base_contour = c("#00838F");
    p.depth_contour = c("#880E4F");
    p.hatch = c("#C62828");
    p.support = c("#757575");
    p.travel = c("#2E7D32").with_alpha(0.4);
    p.gradient_stops = turbo_stops();
    p
}

fn light_ocean() -> ThemePalette {
    let mut p = light_chrome(c("#01579B"), c("#B3E5FC"));
    p.background = c("#F0F4F8");
    p.boundary = c("#1A237E");
    p.contour = c("#0288D1");
    p.base_contour = c("#0097A7");
    p.depth_contour = c("#006064");
    p.hatch = c("#00838F");
    p.support = c("#78909C");
    p.travel = c("#00695C").with_alpha(0.4);
    p.gradient_stops = ocean_stops();
    p
}

fn light_warm() -> ThemePalette {
    let mut p = light_chrome(c("#BF360C"), c("#FFCCBC"));
    p.background = c("#FFF8E1");
    p.boundary = c("#4E342E");
    p.contour = c("#E65100");
    p.base_contour = c("#F57F17");
    p.depth_contour = c("#AD1457");
    p.hatch = c("#AD1457");
    p.support = c("#8D6E63");
    p.travel = c("#6D4C41").with_alpha(0.4);
    p.gradient_stops = inferno_stops();
    p
}

fn light_colorblind_rg() -> ThemePalette {
    // Okabe-Ito inspired — avoids red/green confusion
    let mut p = light_chrome(c("#0072B2"), c("#BAD9EB"));
    p.background = c("#FAFAFA");
    p.boundary = c("#0072B2");     // blue
    p.contour = c("#56B4E9");      // sky blue
    p.base_contour = c("#009E73"); // teal
    p.depth_contour = c("#CC79A7"); // pink
    p.hatch = c("#E69F00");        // orange
    p.support = c("#999999");
    p.travel = c("#009E73").with_alpha(0.4);
    p.gradient_stops = cividis_stops(); // perceptually uniform, CB-safe
    p
}

fn light_colorblind_by() -> ThemePalette {
    // Avoids blue/yellow confusion (tritanopia safe)
    let mut p = light_chrome(c("#CC79A7"), c("#F2D0E4"));
    p.background = c("#FAFAFA");
    p.boundary = c("#CC79A7");     // pink
    p.contour = c("#D55E00");      // vermillion
    p.base_contour = c("#882255"); // wine
    p.depth_contour = c("#332288"); // indigo
    p.hatch = c("#009E73");        // teal
    p.support = c("#888888");
    p.travel = c("#882255").with_alpha(0.4);
    p.gradient_stops = batlow_stops();
    p
}

fn light_high_contrast() -> ThemePalette {
    let mut p = light_chrome(c("#000000"), c("#CCCCCC"));
    p.background = Color::WHITE;
    p.toolbar_bg = Color::WHITE;
    p.toolbar_border = Color::BLACK;
    p.panel_bg = Color::WHITE;
    p.panel_bg_translucent = Color::new(1.0, 1.0, 1.0, 0.95);
    p.text_primary = Color::BLACK;
    p.text_secondary = c("#333333");
    p.toggle_active_bg = c("#CCCCCC");
    p.toggle_inactive_bg = c("#E0E0E0");
    p.boundary = Color::BLACK;
    p.contour = c("#0000FF");
    p.base_contour = c("#0000CC");
    p.depth_contour = c("#800080");
    p.hatch = c("#FF0000");
    p.support = c("#666666");
    p.travel = c("#008000").with_alpha(0.5);
    p.gradient_stops = bw_stops();
    p
}

fn light_custom(custom: Option<&CustomPaletteConfig>) -> ThemePalette {
    let cfg = match custom {
        Some(c) => c.clone(),
        None => CustomPaletteConfig::default(),
    };
    let accent = Color::from_hex(&cfg.accent).unwrap_or(c("#1976D2"));
    let mut p = light_chrome(accent, lighten(accent, 0.5));
    p.background = Color::from_hex(&cfg.background).unwrap_or(c("#FAFAFA"));
    p.boundary = Color::from_hex(&cfg.boundary).unwrap_or(c("#2C3E50"));
    p.contour = Color::from_hex(&cfg.contour).unwrap_or(c("#00BCD4"));
    p.base_contour = p.contour;
    p.depth_contour = p.contour;
    p.hatch = Color::from_hex(&cfg.hatch).unwrap_or(c("#E91E63"));
    p.support = c("#616161");
    p.travel = c("#388E3C").with_alpha(0.4);
    p.gradient_stops = viridis_stops();
    p
}

// ═══════════════════════════════════════════════════════════════════════
//  Dark mode palettes
// ═══════════════════════════════════════════════════════════════════════

fn dark_chrome(accent: Color, accent_light: Color) -> ThemePalette {
    ThemePalette {
        background: c("#1E1E1E"),
        grid_minor: Color::new(1.0, 1.0, 1.0, 0.06),
        grid_major: Color::new(1.0, 1.0, 1.0, 0.15),
        boundary: Color::WHITE,
        contour: Color::CYAN,
        base_contour: Color::CYAN,
        depth_contour: Color::MAGENTA,
        hatch: Color::new(1.0, 0.5, 0.3, 1.0),
        support: Color::GRAY,
        travel: Color::GREEN.with_alpha(0.4),
        gradient_stops: viridis_stops(),
        accent,
        accent_light,
        toolbar_bg: c("#2D2D2D"),
        toolbar_border: c("#3E3E3E"),
        panel_bg: c("#2D2D2D"),
        panel_bg_translucent: Color::new(0.18, 0.18, 0.18, 0.92),
        panel_shadow: Color::new(0.0, 0.0, 0.0, 0.30),
        text_primary: c("#E0E0E0"),
        text_secondary: c("#9E9E9E"),
        toggle_active_bg: Color::new(0.20, 0.30, 0.45, 1.0),
        toggle_inactive_bg: c("#424242"),
        is_dark: true,
    }
}

fn dark_default() -> ThemePalette {
    let mut p = dark_chrome(c("#64B5F6"), c("#1E3A5F"));
    p.background = c("#1E1E1E");
    p.boundary = c("#4DB6AC");
    p.contour = c("#4FC3F7");
    p.base_contour = c("#26C6DA");
    p.depth_contour = c("#F48FB1");
    p.hatch = c("#FF8A65");
    p.support = c("#9E9E9E");
    p.travel = c("#81C784").with_alpha(0.4);
    p.gradient_stops = viridis_stops();
    p
}

fn dark_professional() -> ThemePalette {
    let mut p = dark_chrome(c("#90A4AE"), c("#37474F"));
    p.background = c("#212121");
    p.boundary = c("#78909C");
    p.contour = c("#90A4AE");
    p.base_contour = c("#B0BEC5");
    p.depth_contour = c("#BCAAA4");
    p.hatch = c("#A1887F");
    p.support = c("#757575");
    p.travel = c("#78909C").with_alpha(0.4);
    p.gradient_stops = cividis_stops();
    p
}

fn dark_vibrant() -> ThemePalette {
    let mut p = dark_chrome(c("#B388FF"), c("#4A148C"));
    p.background = c("#121212");
    p.boundary = c("#69F0AE");
    p.contour = c("#40C4FF");
    p.base_contour = c("#18FFFF");
    p.depth_contour = c("#FF80AB");
    p.hatch = c("#FF5252");
    p.support = c("#BDBDBD");
    p.travel = c("#69F0AE").with_alpha(0.4);
    p.gradient_stops = turbo_stops();
    p
}

fn dark_ocean() -> ThemePalette {
    let mut p = dark_chrome(c("#0096C7"), c("#023E73"));
    p.background = c("#0D1B2A");
    p.boundary = c("#48CAE4");
    p.contour = c("#90E0EF");
    p.base_contour = c("#ADE8F4");
    p.depth_contour = c("#CAF0F8");
    p.hatch = c("#00B4D8");
    p.support = c("#778DA9");
    p.travel = c("#48CAE4").with_alpha(0.4);
    p.gradient_stops = ocean_stops();
    p
}

fn dark_warm() -> ThemePalette {
    let mut p = dark_chrome(c("#FF6E40"), c("#4E2C1E"));
    p.background = c("#1A1210");
    p.boundary = c("#FFAB91");
    p.contour = c("#FF8A65");
    p.base_contour = c("#FFCC80");
    p.depth_contour = c("#F48FB1");
    p.hatch = c("#F48FB1");
    p.support = c("#A1887F");
    p.travel = c("#BCAAA4").with_alpha(0.4);
    p.gradient_stops = inferno_stops();
    p
}

fn dark_colorblind_rg() -> ThemePalette {
    let mut p = dark_chrome(c("#56B4E9"), c("#1A4A6E"));
    p.background = c("#1E1E1E");
    p.boundary = c("#56B4E9");     // sky blue
    p.contour = c("#009E73");      // teal
    p.base_contour = c("#66D9A0"); // mint
    p.depth_contour = c("#CC79A7"); // pink
    p.hatch = c("#E69F00");        // orange
    p.support = c("#999999");
    p.travel = c("#009E73").with_alpha(0.4);
    p.gradient_stops = cividis_stops();
    p
}

fn dark_colorblind_by() -> ThemePalette {
    let mut p = dark_chrome(c("#CC79A7"), c("#5E2E4A"));
    p.background = c("#1E1E1E");
    p.boundary = c("#CC79A7");     // pink
    p.contour = c("#D55E00");      // vermillion
    p.base_contour = c("#EE8866"); // salmon
    p.depth_contour = c("#AA99EE"); // light indigo
    p.hatch = c("#009E73");        // teal
    p.support = c("#AAAAAA");
    p.travel = c("#CC79A7").with_alpha(0.4);
    p.gradient_stops = batlow_stops();
    p
}

fn dark_high_contrast() -> ThemePalette {
    let mut p = dark_chrome(Color::WHITE, c("#444444"));
    p.background = Color::BLACK;
    p.toolbar_bg = c("#111111");
    p.toolbar_border = Color::WHITE;
    p.panel_bg = c("#111111");
    p.panel_bg_translucent = Color::new(0.07, 0.07, 0.07, 0.95);
    p.text_primary = Color::WHITE;
    p.text_secondary = c("#CCCCCC");
    p.toggle_active_bg = c("#444444");
    p.toggle_inactive_bg = c("#333333");
    p.boundary = Color::WHITE;
    p.contour = c("#00FFFF");
    p.base_contour = c("#00CCCC");
    p.depth_contour = c("#FF00FF");
    p.hatch = c("#FFFF00");
    p.support = c("#AAAAAA");
    p.travel = c("#00FF00").with_alpha(0.5);
    p.gradient_stops = white_yellow_stops();
    p
}

fn dark_custom(custom: Option<&CustomPaletteConfig>) -> ThemePalette {
    let default_dark_custom = CustomPaletteConfig {
        background: "#1E1E1E".into(),
        boundary: "#4DB6AC".into(),
        contour: "#4FC3F7".into(),
        hatch: "#FF8A65".into(),
        accent: "#64B5F6".into(),
    };
    let cfg = custom.cloned().unwrap_or(default_dark_custom);
    let accent = Color::from_hex(&cfg.accent).unwrap_or(c("#64B5F6"));
    let mut p = dark_chrome(accent, darken(accent, 0.5));
    p.background = Color::from_hex(&cfg.background).unwrap_or(c("#1E1E1E"));
    p.boundary = Color::from_hex(&cfg.boundary).unwrap_or(c("#4DB6AC"));
    p.contour = Color::from_hex(&cfg.contour).unwrap_or(c("#4FC3F7"));
    p.base_contour = p.contour;
    p.depth_contour = p.contour;
    p.hatch = Color::from_hex(&cfg.hatch).unwrap_or(c("#FF8A65"));
    p.support = c("#9E9E9E");
    p.travel = c("#81C784").with_alpha(0.4);
    p.gradient_stops = viridis_stops();
    p
}

// ═══════════════════════════════════════════════════════════════════════
//  Helpers
// ═══════════════════════════════════════════════════════════════════════

/// Shorthand: parse a hex color (panics on bad input — only used with literals).
fn c(hex: &str) -> Color {
    Color::from_hex(hex).expect("invalid hex literal in palette definition")
}

/// Lighten a color towards white by factor `t` ∈ [0, 1].
fn lighten(color: Color, t: f32) -> Color {
    color.blend(&Color::WHITE, t)
}

/// Darken a color towards black by factor `t` ∈ [0, 1].
fn darken(color: Color, t: f32) -> Color {
    color.blend(&Color::BLACK, t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_palettes_resolve() {
        for mode in [ResolvedMode::Light, ResolvedMode::Dark] {
            for id in ALL_PALETTE_IDS {
                let p = resolve_palette(mode, *id, None);
                assert!(p.gradient_stops.len() >= 2, "{:?}/{:?} has <2 gradient stops", mode, id);
            }
        }
    }

    #[test]
    fn gradient_endpoints() {
        let p = light_default();
        let c0 = gradient_color(&p, 0.0);
        let c1 = gradient_color(&p, 1.0);
        // Should match first and last stops
        assert!((c0.r - p.gradient_stops[0].1.r).abs() < 0.01);
        assert!((c1.r - p.gradient_stops.last().unwrap().1.r).abs() < 0.01);
    }

    #[test]
    fn gradient_mid() {
        let p = light_default();
        let mid = gradient_color(&p, 0.5);
        // Should be an interpolated value, not white/black/zero
        assert!(mid.r > 0.0 || mid.g > 0.0 || mid.b > 0.0);
    }

    #[test]
    fn theme_mode_serde_roundtrip() {
        let mode = ThemeMode::Dark;
        let json = serde_json::to_string(&mode).unwrap();
        let back: ThemeMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, back);
    }

    #[test]
    fn palette_id_serde_roundtrip() {
        let id = PaletteId::ColorblindRG;
        let json = serde_json::to_string(&id).unwrap();
        let back: PaletteId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
