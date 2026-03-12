//! UI Module - egui integration for toolbar and floating panels
//!
//! Provides an overlay UI with:
//! - Top toolbar with visibility toggles, parameter mode, file info, controls
//! - Right floating vertical layer slider with jump-to input
//! - Left floating vertical gradient scale with filtering (when param mode active)

use egui::{Context, Slider, DragValue, RichText, ViewportId, Align2, Color32, Rounding, Stroke, Vec2};
use egui_glow::Painter;
use egui_winit::EventResponse;
use egui_winit::State as EguiWinitState;
use std::sync::Arc;

use crate::application::ports::ParameterMode;
use crate::domain::value_objects::Color;

/// Height of the top toolbar in logical pixels
pub const TOOLBAR_HEIGHT: f32 = 52.0;

/// Global parameter ranges computed from the entire file
#[derive(Debug, Clone, Default)]
pub struct ParamRanges {
    pub power: Option<(f32, f32)>,
    pub speed: Option<(f32, f32)>,
    pub wait_time: Option<(f32, f32)>,
}

/// UI state that gets passed to the main app
#[derive(Debug, Clone)]
pub struct UiOutput {
    /// Current layer index (0-based)
    pub layer_index: usize,
    /// Visibility toggles
    pub show_slices: bool,
    pub show_contours: bool,
    pub show_hatches: bool,
    pub show_arrows: bool,
    pub show_power_markers: bool,
    /// Parameter visualization mode
    pub param_mode: Option<ParameterMode>,
    /// Parameter filter min/max
    pub param_filter_min: f32,
    pub param_filter_max: f32,
    /// Whether UI wants to repaint (hover, drag, etc.)
    pub needs_repaint: bool,
}

/// Vector count info for display
#[derive(Debug, Clone, Default)]
pub struct VectorCounts {
    pub hatches: usize,
    pub contours: usize,
    pub boundaries: usize,
    pub total_vectors: usize,
    /// Power value from VK (contours)
    pub vk_power: Option<f32>,
    /// Speed value from VK (contours)
    pub vk_speed: Option<f32>,
    /// Power value from VS (infills)
    pub vs_power: Option<f32>,
    /// Speed value from VS (infills)
    pub vs_speed: Option<f32>,
    /// Number of vectors with wait times
    pub wait_count: usize,
}

/// UI state managed internally
pub struct UiState {
    /// Current layer (for slider)
    pub current_layer: usize,
    /// Total layers
    pub total_layers: usize,
    /// Current Z height
    pub current_z: f32,
    /// Visibility toggles
    pub show_slices: bool,
    pub show_contours: bool,
    pub show_hatches: bool,
    pub show_arrows: bool,
    pub show_power_markers: bool,
    /// Vector counts for current layer
    pub vector_counts: VectorCounts,
    /// Parameter visualization mode
    pub param_mode: Option<ParameterMode>,
    /// Parameter filter min
    pub param_filter_min: f32,
    /// Parameter filter max
    pub param_filter_max: f32,
    /// Global parameter ranges from the file
    pub param_ranges: ParamRanges,
    /// Previous param mode (to detect mode changes and reset filter)
    prev_param_mode: Option<ParameterMode>,
    /// Whether file info popup is open
    pub show_file_info: bool,
    /// Whether controls popup is open
    pub show_controls: bool,
    /// Layer jump input value (text field for direct entry)
    pub layer_jump_value: String,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            current_layer: 0,
            total_layers: 0,
            current_z: 0.0,
            show_slices: true,
            show_contours: true,
            show_hatches: true,
            show_arrows: false,
            show_power_markers: false,
            vector_counts: VectorCounts::default(),
            param_mode: None,
            param_filter_min: 0.0,
            param_filter_max: f32::MAX,
            param_ranges: ParamRanges::default(),
            prev_param_mode: None,
            show_file_info: false,
            show_controls: false,
            layer_jump_value: String::new(),
        }
    }
}

// ── Theme colors ──────────────────────────────────────────────────────────
const TOOLBAR_BG: Color32 = Color32::from_rgb(245, 245, 245);           // #F5F5F5
const TOOLBAR_BORDER: Color32 = Color32::from_rgb(200, 200, 200);       // #C8C8C8
const ACCENT: Color32 = Color32::from_rgb(25, 118, 210);                // #1976D2
const ACCENT_LIGHT: Color32 = Color32::from_rgb(187, 222, 251);         // #BBDEFB
const TOGGLE_ACTIVE_BG: Color32 = Color32::from_rgb(200, 230, 255);     // light blue
const TOGGLE_INACTIVE_BG: Color32 = Color32::from_rgb(230, 230, 230);   // #E6E6E6
const TEXT_PRIMARY: Color32 = Color32::from_rgb(33, 33, 33);            // #212121
const TEXT_SECONDARY: Color32 = Color32::from_rgb(97, 97, 97);          // #616161
const PANEL_BG: Color32 = Color32::from_rgb(255, 255, 255);             // white
const PANEL_SHADOW: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 40);

/// Main UI renderer using egui
pub struct UiRenderer {
    pub ctx: Context,
    pub winit_state: EguiWinitState,
    pub painter: Painter,
    pub state: UiState,
    /// Deferred egui output for split run/paint cycle
    pending_output: Option<egui::FullOutput>,
}

/// Apply the light theme to the egui context
fn apply_light_theme(ctx: &Context) {
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

/// Helper: draw a toolbar toggle button (icon + active state)
fn toolbar_toggle(ui: &mut egui::Ui, icon: &str, active: &mut bool, tooltip: &str) -> bool {
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

/// Helper: draw a toolbar mode button (for parameter mode selection)
fn toolbar_mode_btn(ui: &mut egui::Ui, label: &str, is_selected: bool, tooltip: &str) -> bool {
    let fill = if is_selected { ACCENT } else { TOGGLE_INACTIVE_BG };
    let text_color = if is_selected { Color32::WHITE } else { TEXT_PRIMARY };
    let btn = egui::Button::new(RichText::new(label).size(12.0).color(text_color))
        .fill(fill)
        .rounding(Rounding::same(6.0))
        .min_size(Vec2::new(0.0, 30.0));
    let response = ui.add(btn).on_hover_text(tooltip);
    response.clicked()
}

impl UiRenderer {
    /// Create a new UI renderer
    pub fn new(gl: Arc<glow::Context>, window: &winit::window::Window) -> Self {
        let ctx = Context::default();
        apply_light_theme(&ctx);

        let winit_state = EguiWinitState::new(
            ctx.clone(),
            ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
        );
        let painter = Painter::new(gl, "", None)
            .expect("Failed to create egui painter");

        Self {
            ctx,
            winit_state,
            painter,
            state: UiState::default(),
            pending_output: None,
        }
    }

    /// Update state from navigation
    pub fn update_from_navigation(&mut self, current: usize, total: usize, z: f32) {
        self.state.current_layer = current;
        self.state.total_layers = total;
        self.state.current_z = z;
    }

    /// Handle window event, forwards to egui
    pub fn handle_event(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) -> EventResponse {
        self.winit_state.on_window_event(window, event)
    }

    /// Returns true if egui wants to consume pointer events (mouse is over UI)
    pub fn wants_pointer(&self) -> bool {
        self.ctx.wants_pointer_input()
    }

    /// Returns true if egui wants to consume keyboard events
    pub fn wants_keyboard(&self) -> bool {
        self.ctx.wants_keyboard_input()
    }

    /// Run egui logic (UI layout, handle interactions), returning output.
    /// Call `paint()` afterwards to draw the egui overlay on top of GL content.
    pub fn run_ui(&mut self, window: &winit::window::Window) -> UiOutput {
        let mut layer_changed = false;
        let mut new_layer = self.state.current_layer;

        // Get raw input from winit state
        let raw_input = self.winit_state.take_egui_input(window);

        // Run egui
        let full_output = self.ctx.run(raw_input, |ctx| {
            // ━━━━━━━━━━━━━━━━━━━━━━━ TOP TOOLBAR ━━━━━━━━━━━━━━━━━━━━━━━
            egui::TopBottomPanel::top("toolbar")
                .exact_height(TOOLBAR_HEIGHT)
                .frame(egui::Frame::none()
                    .fill(TOOLBAR_BG)
                    .stroke(Stroke::new(1.0, TOOLBAR_BORDER))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0)))
                .show(ctx, |ui| {
                    ui.horizontal_centered(|ui| {
                        // App title
                        ui.label(RichText::new("Toolpath Viewer").size(16.0).strong().color(TEXT_PRIMARY));

                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // ── Visibility toggles ──
                        toolbar_toggle(ui, "🔲", &mut self.state.show_slices, "Toggle Boundaries (B)");
                        toolbar_toggle(ui, "⭕", &mut self.state.show_contours, "Toggle Contours (C)");
                        toolbar_toggle(ui, "▤", &mut self.state.show_hatches, "Toggle Hatches (H)");
                        toolbar_toggle(ui, "➤", &mut self.state.show_arrows, "Toggle Direction Arrows (A)");
                        toolbar_toggle(ui, "✱", &mut self.state.show_power_markers, "Toggle Power Markers");

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // ── Parameter mode selector ──
                        ui.label(RichText::new("Color:").size(12.0).color(TEXT_SECONDARY));
                        let modes = [
                            (None, "None", "No parameter coloring"),
                            (Some(ParameterMode::Power), "Power", "Color by laser power"),
                            (Some(ParameterMode::Speed), "Speed", "Color by scan speed"),
                            (Some(ParameterMode::WaitTime), "Wait", "Color by wait time"),
                        ];
                        for (mode, label, tip) in &modes {
                            if toolbar_mode_btn(ui, label, self.state.param_mode == *mode, tip) {
                                self.state.param_mode = *mode;
                            }
                        }

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // ── File Info button ──
                        let info_btn = egui::Button::new(RichText::new("📋 Info").size(13.0).color(TEXT_PRIMARY))
                            .fill(if self.state.show_file_info { TOGGLE_ACTIVE_BG } else { Color32::TRANSPARENT })
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(0.0, 30.0));
                        if ui.add(info_btn).on_hover_text("Layer info & parameters").clicked() {
                            self.state.show_file_info = !self.state.show_file_info;
                        }

                        // ── Controls button ──
                        let ctrl_btn = egui::Button::new(RichText::new("⌨ Controls").size(13.0).color(TEXT_PRIMARY))
                            .fill(if self.state.show_controls { TOGGLE_ACTIVE_BG } else { Color32::TRANSPARENT })
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(0.0, 30.0));
                        if ui.add(ctrl_btn).on_hover_text("Keyboard shortcuts").clicked() {
                            self.state.show_controls = !self.state.show_controls;
                        }

                        // ── Layer info on the right side ──
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if self.state.total_layers > 0 {
                                ui.label(RichText::new(format!(
                                    "Layer {}/{}  •  Z = {:.3} mm",
                                    self.state.current_layer + 1,
                                    self.state.total_layers,
                                    self.state.current_z
                                )).size(12.0).color(TEXT_SECONDARY));
                            } else {
                                ui.label(RichText::new("No file loaded — drop an ILT file").size(12.0).color(TEXT_SECONDARY));
                            }
                        });
                    });
                });

            // ━━━━━━━━━━━━━━━━ RESET PARAM FILTER ON MODE CHANGE ━━━━━━━━━━━━━━━━
            if self.state.param_mode != self.state.prev_param_mode {
                let range = match self.state.param_mode {
                    Some(ParameterMode::Power) => self.state.param_ranges.power,
                    Some(ParameterMode::Speed) => self.state.param_ranges.speed,
                    Some(ParameterMode::WaitTime) => self.state.param_ranges.wait_time,
                    None => None,
                };
                if let Some((lo, hi)) = range {
                    self.state.param_filter_min = lo;
                    self.state.param_filter_max = hi;
                } else {
                    self.state.param_filter_min = 0.0;
                    self.state.param_filter_max = 1.0;
                }
                self.state.prev_param_mode = self.state.param_mode;
            }

            // ━━━━━━━━━━━━━━━ RIGHT FLOATING LAYER SLIDER ━━━━━━━━━━━━━━━━
            if self.state.total_layers > 0 {
                let screen = ctx.screen_rect();
                let slider_panel_w = 64.0;
                let slider_margin = 12.0;
                let slider_top = TOOLBAR_HEIGHT + 16.0;
                let slider_bottom_margin = 16.0;
                let panel_h = (screen.height() - slider_top - slider_bottom_margin).max(220.0);

                egui::Area::new(egui::Id::new("layer_slider_area"))
                    .anchor(Align2::RIGHT_TOP, egui::vec2(-slider_margin, slider_top))
                    .order(egui::Order::Foreground)
                    .interactable(true)
                    .movable(false)
                    .show(ctx, |ui| {
                        egui::Frame::none()
                            .fill(Color32::from_rgba_premultiplied(255, 255, 255, 230))
                            .rounding(Rounding::same(10.0))
                            .shadow(egui::epaint::Shadow {
                                offset: egui::vec2(0.0, 2.0),
                                blur: 8.0,
                                spread: 0.0,
                                color: PANEL_SHADOW,
                            })
                            .inner_margin(egui::Margin::symmetric(8.0, 12.0))
                            .show(ui, |ui| {
                                // Keep this panel strictly fixed-size so no child widget can expand it.
                                ui.set_min_width(slider_panel_w);
                                ui.set_max_width(slider_panel_w);

                                let inner_h = panel_h - 24.0;
                                ui.allocate_ui_with_layout(
                                    egui::vec2(slider_panel_w, inner_h),
                                    egui::Layout::top_down(egui::Align::Center),
                                    |ui| {
                                        let max_layer = self.state.total_layers.saturating_sub(1);
                                        let slider_h = (inner_h - 120.0).max(100.0);

                                        let first_btn = egui::Button::new(RichText::new("⏮").size(14.0))
                                            .min_size(Vec2::new(36.0, 24.0))
                                            .rounding(Rounding::same(4.0));
                                        if ui.add(first_btn).on_hover_text("First layer (Home)").clicked() {
                                            new_layer = 0;
                                            layer_changed = true;
                                        }

                                        ui.add_space(4.0);

                                        // Vertical slider (egui puts min at bottom, max at top)
                                        let mut layer = self.state.current_layer;
                                        let slider = Slider::new(&mut layer, 0..=max_layer)
                                            .vertical()
                                            .show_value(false);
                                        let slider_response = ui.add_sized(egui::vec2(20.0, slider_h), slider);
                                        if slider_response.changed() {
                                            new_layer = layer;
                                            layer_changed = true;
                                        }

                                        ui.add_space(4.0);

                                        let last_btn = egui::Button::new(RichText::new("⏭").size(14.0))
                                            .min_size(Vec2::new(36.0, 24.0))
                                            .rounding(Rounding::same(4.0));
                                        if ui.add(last_btn).on_hover_text("Last layer (End)").clicked() {
                                            new_layer = max_layer;
                                            layer_changed = true;
                                        }

                                        ui.add_space(8.0);

                                        // Jump-to input
                                        ui.label(RichText::new("Go to").size(10.0).color(TEXT_SECONDARY));
                                        let mut jump_val = (self.state.current_layer + 1) as i64;
                                        let dv = DragValue::new(&mut jump_val)
                                            .clamp_range(1..=(self.state.total_layers as i64))
                                            .speed(1.0);
                                        if ui.add_sized(egui::vec2(40.0, 20.0), dv).changed() {
                                            new_layer = (jump_val as usize)
                                                .saturating_sub(1)
                                                .min(self.state.total_layers.saturating_sub(1));
                                            layer_changed = true;
                                        }
                                    },
                                );
                            });
                    });
            }

            // ━━━━━━━━━━━━━━━ LEFT FLOATING GRADIENT SCALE ━━━━━━━━━━━━━━━━
            if self.state.param_mode.is_some() {
                let screen = ctx.screen_rect();
                let grad_top = TOOLBAR_HEIGHT + 16.0;
                let grad_margin = 12.0;
                let slider_panel_w = 64.0;
                let grad_panel_w = 92.0;
                // Keep gradient panel just left of the right slider panel.
                let right_offset = -(grad_margin + slider_panel_w + 10.0 + grad_panel_w);
                let grad_panel_h = (screen.height() - grad_top - 16.0).max(240.0);

                egui::Area::new(egui::Id::new("gradient_scale_area"))
                    .anchor(Align2::RIGHT_TOP, egui::vec2(right_offset, grad_top))
                    .order(egui::Order::Foreground)
                    .interactable(true)
                    .movable(false)
                    .show(ctx, |ui| {
                        egui::Frame::none()
                            .fill(Color32::from_rgba_premultiplied(255, 255, 255, 230))
                            .rounding(Rounding::same(10.0))
                            .shadow(egui::epaint::Shadow {
                                offset: egui::vec2(0.0, 2.0),
                                blur: 8.0,
                                spread: 0.0,
                                color: PANEL_SHADOW,
                            })
                            .inner_margin(egui::Margin::symmetric(10.0, 12.0))
                            .show(ui, |ui| {
                                // Hard bounds: no expansion beyond this panel size.
                                ui.set_min_width(grad_panel_w);
                                ui.set_max_width(grad_panel_w);

                                let inner_h = grad_panel_h - 24.0;
                                ui.allocate_ui_with_layout(
                                    egui::vec2(grad_panel_w, inner_h),
                                    egui::Layout::top_down(egui::Align::Center),
                                    |ui| {
                                        let bar_w = 16.0;
                                        let bar_h = (inner_h - 160.0).max(120.0);

                                        // Mode label
                                        let mode_label = match self.state.param_mode {
                                            Some(ParameterMode::Power) => "Power",
                                            Some(ParameterMode::Speed) => "Speed",
                                            Some(ParameterMode::WaitTime) => "Wait Time",
                                            None => "",
                                        };
                                        ui.label(RichText::new(mode_label).size(11.0).strong().color(TEXT_PRIMARY));
                                        ui.add_space(4.0);

                                        // Max value label (top of scale)
                                        ui.label(
                                            RichText::new(format!("{:.1}", self.state.param_filter_max))
                                                .size(10.0)
                                                .color(TEXT_SECONDARY),
                                        );
                                        ui.add_space(2.0);

                                        // Vertical gradient bar
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::vec2(bar_w, bar_h),
                                            egui::Sense::hover(),
                                        );
                                        let n = 64;
                                        let seg_h = rect.height() / n as f32;
                                        for i in 0..n {
                                            // t=1 at top (max/hot), t=0 at bottom (min/cold)
                                            let t = 1.0 - (i as f32 / (n - 1) as f32);
                                            let c = Color::heat_gradient(t);
                                            let y0 = rect.top() + i as f32 * seg_h;
                                            let seg = egui::Rect::from_min_max(
                                                egui::pos2(rect.left(), y0),
                                                egui::pos2(rect.right(), y0 + seg_h + 0.5),
                                            );
                                            ui.painter().rect_filled(
                                                seg,
                                                0.0,
                                                Color32::from_rgb(
                                                    (c.r * 255.0) as u8,
                                                    (c.g * 255.0) as u8,
                                                    (c.b * 255.0) as u8,
                                                ),
                                            );
                                        }
                                        ui.painter().rect_stroke(
                                            rect,
                                            Rounding::same(2.0),
                                            Stroke::new(1.0, Color32::from_rgb(180, 180, 180)),
                                        );

                                        ui.add_space(2.0);
                                        ui.label(
                                            RichText::new(format!("{:.1}", self.state.param_filter_min))
                                                .size(10.0)
                                                .color(TEXT_SECONDARY),
                                        );

                                        ui.add_space(8.0);
                                        ui.label(RichText::new("Filter").size(10.0).strong().color(TEXT_PRIMARY));
                                        ui.add_space(2.0);

                                        let data_range = match self.state.param_mode {
                                            Some(ParameterMode::Power) => self.state.param_ranges.power,
                                            Some(ParameterMode::Speed) => self.state.param_ranges.speed,
                                            Some(ParameterMode::WaitTime) => self.state.param_ranges.wait_time,
                                            None => None,
                                        };
                                        let (lo, hi) = data_range.unwrap_or((0.0, 1.0));
                                        let speed = (hi - lo).abs() * 0.01;

                                        ui.label(RichText::new("Min").size(10.0).color(TEXT_SECONDARY));
                                        ui.add_sized(
                                            egui::vec2(52.0, 18.0),
                                            DragValue::new(&mut self.state.param_filter_min)
                                                .speed(speed.max(0.1))
                                                .clamp_range(lo..=self.state.param_filter_max),
                                        );

                                        ui.label(RichText::new("Max").size(10.0).color(TEXT_SECONDARY));
                                        ui.add_sized(
                                            egui::vec2(52.0, 18.0),
                                            DragValue::new(&mut self.state.param_filter_max)
                                                .speed(speed.max(0.1))
                                                .clamp_range(self.state.param_filter_min..=hi),
                                        );

                                        ui.add_space(2.0);
                                        let reset_btn = egui::Button::new(RichText::new("Reset").size(10.0))
                                            .rounding(Rounding::same(4.0))
                                            .min_size(Vec2::new(52.0, 20.0));
                                        if ui.add(reset_btn).clicked() {
                                            self.state.param_filter_min = lo;
                                            self.state.param_filter_max = hi;
                                        }
                                    },
                                );
                            });
                    });
            }

            // ━━━━━━━━━━━━━━━━━━ FILE INFO POPUP ━━━━━━━━━━━━━━━━━━━━━━
            if self.state.show_file_info {
                egui::Window::new(RichText::new("📋 File Info").size(14.0).color(TEXT_PRIMARY))
                    .collapsible(false)
                    .resizable(false)
                    .default_width(220.0)
                    .anchor(Align2::LEFT_TOP, egui::vec2(12.0, TOOLBAR_HEIGHT + 12.0))
                    .show(ctx, |ui| {
                        ui.label(RichText::new("Layer Info").size(13.0).strong().color(TEXT_PRIMARY));
                        ui.add_space(4.0);
                        let vc = &self.state.vector_counts;
                        let info_items = [
                            ("Hatches", vc.hatches),
                            ("Contours", vc.contours),
                            ("Boundaries", vc.boundaries),
                            ("Total vectors", vc.total_vectors),
                        ];
                        for (label, val) in &info_items {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(*label).size(12.0).color(TEXT_SECONDARY));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(RichText::new(format!("{}", val)).size(12.0).strong().color(TEXT_PRIMARY));
                                });
                            });
                        }

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);
                        ui.label(RichText::new("Parameters").size(13.0).strong().color(TEXT_PRIMARY));
                        ui.add_space(4.0);

                        let params: Vec<(&str, Option<f32>)> = vec![
                            ("VK Power", vc.vk_power),
                            ("VK Speed", vc.vk_speed),
                            ("VS Power", vc.vs_power),
                            ("VS Speed", vc.vs_speed),
                        ];
                        for (label, val) in &params {
                            if let Some(v) = val {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(*label).size(12.0).color(TEXT_SECONDARY));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(RichText::new(format!("{:.0}", v)).size(12.0).strong().color(TEXT_PRIMARY));
                                    });
                                });
                            }
                        }
                        if vc.wait_count > 0 {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Wait times").size(12.0).color(TEXT_SECONDARY));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(RichText::new(format!("{}", vc.wait_count)).size(12.0).strong().color(TEXT_PRIMARY));
                                });
                            });
                        }
                    });
            }

            // ━━━━━━━━━━━━━━━━━━ CONTROLS POPUP ━━━━━━━━━━━━━━━━━━━━━━
            if self.state.show_controls {
                egui::Window::new(RichText::new("⌨ Controls").size(14.0).color(TEXT_PRIMARY))
                    .collapsible(false)
                    .resizable(false)
                    .default_width(240.0)
                    .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                    .show(ctx, |ui| {
                        let shortcuts = [
                            ("↑ / ↓", "Navigate layers"),
                            ("Page Up / Down", "Jump 10 layers"),
                            ("Home / End", "First / Last layer"),
                            ("Scroll Wheel", "Zoom in / out"),
                            ("Middle Drag", "Pan view"),
                            ("Ctrl + Left Drag", "Pan view"),
                            ("R", "Reset view (fit to content)"),
                            ("B", "Toggle boundaries"),
                            ("C", "Toggle contours"),
                            ("H", "Toggle hatches"),
                            ("A", "Toggle direction arrows"),
                        ];
                        egui::Grid::new("controls_grid")
                            .num_columns(2)
                            .spacing([16.0, 4.0])
                            .show(ui, |ui| {
                                for (key, action) in &shortcuts {
                                    ui.label(RichText::new(*key).size(12.0).strong().color(ACCENT));
                                    ui.label(RichText::new(*action).size(12.0).color(TEXT_PRIMARY));
                                    ui.end_row();
                                }
                            });
                    });
            }
        });

        // Update state if layer changed via slider
        if layer_changed {
            self.state.current_layer = new_layer;
        }

        // Handle platform output
        self.winit_state.handle_platform_output(window, full_output.platform_output.clone());

        let needs_repaint = full_output
            .viewport_output
            .get(&ViewportId::ROOT)
            .map(|v| v.repaint_delay.is_zero())
            .unwrap_or(false);

        // Store output for deferred painting
        self.pending_output = Some(full_output);

        UiOutput {
            layer_index: self.state.current_layer,
            show_slices: self.state.show_slices,
            show_contours: self.state.show_contours,
            show_hatches: self.state.show_hatches,
            show_arrows: self.state.show_arrows,
            show_power_markers: self.state.show_power_markers,
            param_mode: self.state.param_mode,
            param_filter_min: self.state.param_filter_min,
            param_filter_max: self.state.param_filter_max,
            needs_repaint,
        }
    }

    /// Paint the egui overlay. Must be called after `run_ui()` and after GL content is rendered.
    pub fn paint(&mut self, window: &winit::window::Window) {
        let full_output = match self.pending_output.take() {
            Some(o) => o,
            None => return,
        };

        let size = window.inner_size();
        let pixels_per_point = full_output.pixels_per_point;
        let clipped_primitives = self.ctx.tessellate(full_output.shapes, pixels_per_point);

        self.painter.paint_and_update_textures(
            [size.width, size.height],
            pixels_per_point,
            &clipped_primitives,
            &full_output.textures_delta,
        );
    }

    /// Destroy the UI renderer
    pub fn destroy(&mut self) {
        self.painter.destroy();
    }
}
