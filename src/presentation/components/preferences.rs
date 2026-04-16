//! Preferences Dialog Component
//!
//! Tabbed popup window with **Appearance** (theme mode) and
//! **Color Palette** (palette selection + custom picker) tabs.
//! Changes are applied live.

use egui::{Color32, Context, RichText, Rounding, Stroke, Vec2};

use crate::application::ports::{CanvasConfig, PaletteId, ThemeMode, ALL_PALETTE_IDS};
use crate::presentation::palette::{
    self, ResolvedMode, ThemePalette, resolve_mode, resolve_palette,
};
use crate::presentation::theme::{self, to_color32};

/// Output from the preferences dialog.
#[derive(Debug, Clone, Default)]
pub struct PreferencesOutput {
    /// Whether theme/palette was changed this frame.
    pub changed: bool,
    /// Updated palette (if changed).
    pub palette: Option<ThemePalette>,
    /// Whether canvas settings were changed this frame.
    pub canvas_changed: bool,
}

/// Which tab is selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Appearance,
    Palette,
    Canvas,
}

/// Render the preferences dialog.  Returns info about changes.
pub fn show_preferences(
    ctx: &Context,
    open: &mut bool,
    theme_mode: &mut ThemeMode,
    palette_id: &mut PaletteId,
    light_palette_id: &mut PaletteId,
    dark_palette_id: &mut PaletteId,
    system_is_dark: bool,
    current_palette: &ThemePalette,
    canvas_settings: &mut CanvasConfig,
) -> PreferencesOutput {
    let mut output = PreferencesOutput::default();
    let t = theme::active();

    // Persistent state for the selected tab (stored in egui's memory)
    let tab_id = egui::Id::new("prefs_tab");
    let mut selected_tab: Tab = ctx.data_mut(|d| *d.get_temp_mut_or(tab_id, Tab::Appearance));

    egui::Window::new(RichText::new("Preferences").size(14.0).color(t.text_primary))
        .open(open)
        .collapsible(false)
        .resizable(false)
        .default_width(340.0)
        .default_pos(egui::pos2(
            ctx.screen_rect().center().x - 170.0,
            ctx.screen_rect().center().y - 200.0,
        ))
        .show(ctx, |ui| {
            // Elevate this window above Foreground-order overlays
            ctx.move_to_top(ui.layer_id());
            // ── Tab bar ──
            ui.horizontal(|ui| {
                if tab_button(ui, "Appearance", selected_tab == Tab::Appearance, &t) {
                    selected_tab = Tab::Appearance;
                }
                if tab_button(ui, "Color Palette", selected_tab == Tab::Palette, &t) {
                    selected_tab = Tab::Palette;
                }
                if tab_button(ui, "Canvas", selected_tab == Tab::Canvas, &t) {
                    selected_tab = Tab::Canvas;
                }
            });
            ui.separator();
            ui.add_space(8.0);

            match selected_tab {
                Tab::Appearance => {
                    output = appearance_tab(ui, theme_mode, palette_id, light_palette_id, dark_palette_id, system_is_dark, &t);
                }
                Tab::Palette => {
                    output = palette_tab(ui, theme_mode, palette_id, light_palette_id, dark_palette_id, system_is_dark, current_palette, &t);
                }
                Tab::Canvas => {
                    let changed = canvas_tab(ui, canvas_settings, &t);
                    output.canvas_changed = changed;
                }
            }
        });

    // Persist tab state
    ctx.data_mut(|d| d.insert_temp(tab_id, selected_tab));

    output
}

// ── Tab button helper ────────────────────────────────────────────────────

fn tab_button(ui: &mut egui::Ui, label: &str, selected: bool, t: &theme::ActiveTheme) -> bool {
    let fill = if selected { t.accent } else { Color32::TRANSPARENT };
    let text_color = if selected {
        if t.is_dark { Color32::BLACK } else { Color32::WHITE }
    } else {
        t.text_primary
    };
    let btn = egui::Button::new(RichText::new(label).size(12.0).color(text_color))
        .fill(fill)
        .rounding(Rounding::same(6.0))
        .min_size(Vec2::new(0.0, 28.0));
    ui.add(btn).clicked()
}

// ── Appearance tab ───────────────────────────────────────────────────────

fn appearance_tab(
    ui: &mut egui::Ui,
    theme_mode: &mut ThemeMode,
    palette_id: &mut PaletteId,
    light_palette_id: &mut PaletteId,
    dark_palette_id: &mut PaletteId,
    system_is_dark: bool,
    t: &theme::ActiveTheme,
) -> PreferencesOutput {
    let mut output = PreferencesOutput::default();

    ui.label(RichText::new("Theme Mode").size(13.0).strong().color(t.text_primary));
    ui.add_space(4.0);

    let modes = [ThemeMode::Light, ThemeMode::Dark, ThemeMode::System];
    for mode in &modes {
        let is_selected = *theme_mode == *mode;
        let label = match mode {
            ThemeMode::System => {
                let detected = if system_is_dark { "Dark" } else { "Light" };
                format!("{} (detected: {})", mode.label(), detected)
            }
            _ => mode.label().to_string(),
        };
        if ui.radio(is_selected, RichText::new(label).size(12.0).color(t.text_primary)).clicked() && !is_selected {
            *theme_mode = *mode;
            // Resolve palette for the new mode using the correct per-mode palette ID
            let resolved = resolve_mode(*theme_mode, system_is_dark);
            let mode_palette_id = match resolved {
                ResolvedMode::Light => *light_palette_id,
                ResolvedMode::Dark => *dark_palette_id,
            };
            *palette_id = mode_palette_id;
            let custom = None; // TODO: pass custom config
            let new_palette = resolve_palette(resolved, mode_palette_id, custom);
            theme::apply_theme(ui.ctx(), &new_palette);
            output.changed = true;
            output.palette = Some(new_palette);
        }
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(4.0);

    // Quick info
    let resolved = resolve_mode(*theme_mode, system_is_dark);
    let mode_name = match resolved {
        ResolvedMode::Light => "Light",
        ResolvedMode::Dark => "Dark",
    };
    ui.label(
        RichText::new(format!("Active: {} / {}", mode_name, palette_id.label()))
            .size(11.0)
            .color(t.text_secondary),
    );

    output
}

// ── Palette tab ──────────────────────────────────────────────────────────

fn palette_tab(
    ui: &mut egui::Ui,
    theme_mode: &mut ThemeMode,
    palette_id: &mut PaletteId,
    light_palette_id: &mut PaletteId,
    dark_palette_id: &mut PaletteId,
    system_is_dark: bool,
    _current_palette: &ThemePalette,
    t: &theme::ActiveTheme,
) -> PreferencesOutput {
    let mut output = PreferencesOutput::default();
    let resolved = resolve_mode(*theme_mode, system_is_dark);

    ui.label(RichText::new("Select Palette").size(13.0).strong().color(t.text_primary));
    ui.add_space(4.0);

    let avail = palette::available_palettes();
    let swatch_size = Vec2::new(280.0, 36.0);

    egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
        for (id, name) in &avail {
            let is_selected = *palette_id == *id;
            let p = resolve_palette(resolved, *id, None);

            // Swatch frame
            let (rect, response) = ui.allocate_exact_size(swatch_size, egui::Sense::click());
            let painter = ui.painter_at(rect);

            // Background
            let bg = if is_selected { t.accent_light } else if response.hovered() {
                if t.is_dark { Color32::from_rgb(60, 60, 60) } else { Color32::from_rgb(235, 235, 235) }
            } else {
                Color32::TRANSPARENT
            };
            painter.rect_filled(rect, Rounding::same(6.0), bg);

            // Color swatch strip (5 small squares)
            let strip_y = rect.top() + 4.0;
            let sq = 14.0;
            let strip_x = rect.left() + 8.0;
            let colors = [p.boundary, p.contour, p.hatch, p.background, p.accent];
            for (i, color) in colors.iter().enumerate() {
                let x = strip_x + i as f32 * (sq + 3.0);
                let sq_rect = egui::Rect::from_min_size(
                    egui::pos2(x, strip_y),
                    egui::vec2(sq, sq),
                );
                painter.rect_filled(sq_rect, Rounding::same(2.0), to_color32(color));
                painter.rect_stroke(
                    sq_rect,
                    Rounding::same(2.0),
                    Stroke::new(0.5, if t.is_dark { Color32::from_rgb(80, 80, 80) } else { Color32::from_rgb(180, 180, 180) }),
                );
            }

            // Name and badge
            let text_x = strip_x + colors.len() as f32 * (sq + 3.0) + 8.0;
            painter.text(
                egui::pos2(text_x, rect.top() + 6.0),
                egui::Align2::LEFT_TOP,
                name,
                egui::FontId::proportional(11.5),
                t.text_primary,
            );
            if let Some(badge) = id.badge() {
                painter.text(
                    egui::pos2(text_x, rect.top() + 20.0),
                    egui::Align2::LEFT_TOP,
                    badge,
                    egui::FontId::proportional(9.0),
                    t.text_secondary,
                );
            }

            // Selection border
            if is_selected {
                painter.rect_stroke(
                    rect,
                    Rounding::same(6.0),
                    Stroke::new(2.0, t.accent),
                );
            }

            if response.clicked() && !is_selected {
                *palette_id = *id;
                // Update the correct per-mode palette ID
                match resolved {
                    ResolvedMode::Light => *light_palette_id = *id,
                    ResolvedMode::Dark => *dark_palette_id = *id,
                }
                let new_palette = resolve_palette(resolved, *id, None);
                theme::apply_theme(ui.ctx(), &new_palette);
                output.changed = true;
                output.palette = Some(new_palette);
            }

            ui.add_space(2.0);
        }
    });

    // Custom palette editor (only when Custom is selected)
    if *palette_id == PaletteId::Custom {
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        ui.label(
            RichText::new("Custom palette color editing coming soon")
                .size(10.0)
                .color(t.text_secondary),
        );
    }

    output
}

// ── Canvas tab ───────────────────────────────────────────────────────────

/// Section header helper.
fn section_header(ui: &mut egui::Ui, label: &str, t: &theme::ActiveTheme) {
    ui.label(RichText::new(label).size(13.0).strong().color(t.text_primary));
    ui.add_space(4.0);
}

/// Labeled slider row returning true if value changed.
fn slider_row(ui: &mut egui::Ui, label: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>, t: &theme::ActiveTheme) -> bool {
    let before = *value;
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(11.0).color(t.text_secondary));
        ui.add(
            egui::DragValue::new(value)
                .speed(0.01)
                .clamp_range(range)
                .max_decimals(2),
        );
    });
    (*value - before).abs() > f32::EPSILON
}

/// Render the Canvas settings tab.  Returns `true` if any setting changed.
fn canvas_tab(
    ui: &mut egui::Ui,
    cfg: &mut CanvasConfig,
    t: &theme::ActiveTheme,
) -> bool {
    let mut changed = false;

    egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
        // ── Line Thickness ──
        section_header(ui, "Line Thickness", t);
        changed |= slider_row(ui, "Base line width", &mut cfg.line_width, 0.5..=5.0, t);
        changed |= slider_row(ui, "Boundary multiplier", &mut cfg.boundary_width_multiplier, 0.1..=3.0, t);
        changed |= slider_row(ui, "Contour multiplier", &mut cfg.contour_width_multiplier, 0.1..=3.0, t);
        changed |= slider_row(ui, "Hatch multiplier", &mut cfg.hatch_width_multiplier, 0.1..=3.0, t);

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // ── Markers ──
        section_header(ui, "Markers", t);
        changed |= slider_row(ui, "Arrow size", &mut cfg.arrow_size_multiplier, 0.2..=3.0, t);
        changed |= slider_row(ui, "Wait dot size", &mut cfg.wait_marker_size_multiplier, 0.2..=3.0, t);

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // ── Vector Display ──
        section_header(ui, "Vector Display", t);
        changed |= slider_row(ui, "Future vector alpha", &mut cfg.future_vector_alpha, 0.0..=1.0, t);
        {
            let before = cfg.show_direction_gradient;
            ui.horizontal(|ui| {
                ui.checkbox(&mut cfg.show_direction_gradient,
                    RichText::new("Direction gradient").size(11.0).color(t.text_secondary));
            });
            if cfg.show_direction_gradient != before {
                changed = true;
            }
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // ── Grid ──
        section_header(ui, "Grid", t);
        changed |= slider_row(ui, "Minor line width", &mut cfg.grid_line_width_minor, 0.5..=3.0, t);
        changed |= slider_row(ui, "Major line width", &mut cfg.grid_line_width_major, 0.5..=3.0, t);
        changed |= slider_row(ui, "Grid opacity", &mut cfg.grid_opacity, 0.0..=1.0, t);

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // ── Quality ──
        section_header(ui, "Quality", t);
        {
            let before = cfg.antialiasing;
            ui.horizontal(|ui| {
                ui.checkbox(&mut cfg.antialiasing,
                    RichText::new("Anti-aliasing (line smoothing)").size(11.0).color(t.text_secondary));
            });
            if cfg.antialiasing != before {
                changed = true;
            }
        }

        ui.add_space(12.0);

        // ── Reset to defaults ──
        if ui.add(
            egui::Button::new(RichText::new("Reset to Defaults").size(11.0))
                .rounding(Rounding::same(4.0))
        ).clicked() {
            *cfg = CanvasConfig::default();
            changed = true;
        }
    });

    changed
}
