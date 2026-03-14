//! UI Module - egui integration for toolbar and floating panels
//!
//! Provides an overlay UI with:
//! - Top toolbar with visibility toggles, parameter mode, file info, controls
//! - Right floating full-height vertical layer slider with jump-to input
//! - Left floating vertical gradient scale with filtering (when param mode active)

use egui::{Context, Slider, DragValue, RichText, ViewportId, Align2, Color32, Rounding, Stroke, Vec2, FontData, FontDefinitions, FontFamily};
use egui_glow::Painter;
use egui_winit::EventResponse;
use egui_winit::State as EguiWinitState;
use std::sync::Arc;

use crate::application::ports::{GridUnit, ParameterMode};
use crate::domain::value_objects::Color;
use lucide_icons::{Icon as LucideIcon, LUCIDE_FONT_BYTES};

/// Lucide font family name
const LUCIDE_FONT: &str = "lucide";

/// Height of the top toolbar in logical pixels
pub const TOOLBAR_HEIGHT: f32 = 52.0;

// ── Tool Mode types ───────────────────────────────────────────────────────

/// Active tool mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolMode {
    /// Default mode — pan/zoom with existing controls
    None,
    /// Shift+drag rectangle zoom-to-fit
    ZoomSelect,
    /// Click-to-click distance measurement
    Ruler,
}

impl Default for ToolMode {
    fn default() -> Self {
        ToolMode::None
    }
}

/// A persistent ruler measurement drawn on the viewport
#[derive(Debug, Clone)]
pub struct RulerMeasurement {
    /// Start point in world coordinates
    pub start: crate::domain::value_objects::Point2D,
    /// End point in world coordinates
    pub end: crate::domain::value_objects::Point2D,
    /// Measured distance in mm
    pub distance_mm: f32,
}

/// Tool state managed by the UI and consumed by the app
#[derive(Debug, Clone)]
pub struct ToolState {
    /// Currently active tool mode
    pub active_mode: ToolMode,
    /// Zoom selection rectangle start (screen coords, viewport-relative)
    pub zoom_rect_start: Option<crate::domain::value_objects::Point2D>,
    /// Zoom selection rectangle end (screen coords, viewport-relative)
    pub zoom_rect_end: Option<crate::domain::value_objects::Point2D>,
    /// Ruler measurement start (world coords)
    pub ruler_start: Option<crate::domain::value_objects::Point2D>,
    /// Ruler measurement end (world coords, live preview while placing)
    pub ruler_end: Option<crate::domain::value_objects::Point2D>,
    /// Persistent ruler measurements
    pub ruler_measurements: Vec<RulerMeasurement>,
    /// Show background grid
    pub show_grid: bool,
    /// Grid display unit
    pub grid_unit: GridUnit,
    /// Show scale bar
    pub show_scale_bar: bool,
    /// Snapshot requested this frame
    pub snapshot_requested: bool,
    /// Snapshot format requested
    pub snapshot_format: SnapshotFormat,
}

impl Default for ToolState {
    fn default() -> Self {
        Self {
            active_mode: ToolMode::None,
            zoom_rect_start: None,
            zoom_rect_end: None,
            ruler_start: None,
            ruler_end: None,
            ruler_measurements: Vec::new(),
            show_grid: false,
            grid_unit: GridUnit::Millimeters,
            show_scale_bar: true,
            snapshot_requested: false,
            snapshot_format: SnapshotFormat::Png,
        }
    }
}

/// Snapshot export format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotFormat {
    Png,
    Svg,
}

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
    /// Show wait time markers
    pub show_wait_markers: bool,
    /// Parameter visualization mode
    pub param_mode: Option<ParameterMode>,
    /// Parameter filter min/max
    pub param_filter_min: f32,
    pub param_filter_max: f32,
    /// Whether UI wants to repaint (hover, drag, etc.)
    pub needs_repaint: bool,
    /// User requested to open a file via load button
    pub open_file_requested: bool,
    /// Tool state output
    pub tool_state: ToolState,
    /// Zoom in button clicked
    pub zoom_in_requested: bool,
    /// Zoom out button clicked
    pub zoom_out_requested: bool,
    /// Fit view button clicked
    pub fit_view_requested: bool,
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
    /// Show wait time markers
    pub show_wait_markers: bool,
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
    /// Tool state
    pub tool_state: ToolState,
    /// Cached zoom level for scale bar (updated from app each frame)
    pub tool_state_zoom: f32,
    /// Cached view transform for coordinate conversion (viewport_w, viewport_h, ViewState)
    pub tool_state_view_transform: Option<(f32, f32, crate::application::ports::ViewState)>,
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
            show_wait_markers: true,
            vector_counts: VectorCounts::default(),
            param_mode: None,
            param_filter_min: 0.0,
            param_filter_max: f32::MAX,
            param_ranges: ParamRanges::default(),
            prev_param_mode: None,
            show_file_info: false,
            show_controls: false,
            layer_jump_value: String::new(),
            tool_state: ToolState::default(),
            tool_state_zoom: 1.0,
            tool_state_view_transform: None,
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

/// Set up fonts including the Lucide icon font
fn setup_fonts(ctx: &Context) {
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
        setup_fonts(&ctx);
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

    /// Update the cached view transform so overlays (ruler, grid labels, scale bar) can convert coords
    pub fn update_view_transform(&mut self, view: &crate::application::ports::ViewState, viewport_width: f32, viewport_height: f32) {
        self.state.tool_state_zoom = view.zoom;
        self.state.tool_state_view_transform = Some((viewport_width, viewport_height, view.clone()));
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

        // Track if user requested to open a file this frame
        let mut open_file_requested = false;

        // Track tool actions requested this frame
        let mut zoom_in_requested = false;
        let mut zoom_out_requested = false;
        let mut fit_view_requested = false;
        let mut snapshot_requested = false;

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

                        // ── Load File button ──
                        let icon_folder = LucideIcon::FolderOpen.unicode();
                        let load_btn = egui::Button::new(RichText::new(format!("{} Load", icon_folder)).size(13.0).color(TEXT_PRIMARY))
                            .fill(Color32::TRANSPARENT)
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(0.0, 30.0));
                        if ui.add(load_btn).on_hover_text("Load file (Ctrl+O)").clicked() {
                            open_file_requested = true;
                        }

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // ── Visibility toggles (using Lucide icons) ──
                        let icon_circle = &LucideIcon::Circle.unicode().to_string();
                        let icon_grid = &LucideIcon::Grid3x3.unicode().to_string();
                        let icon_arrow = &LucideIcon::MoveRight.unicode().to_string();
                        let icon_clock = &LucideIcon::Clock.unicode().to_string();
                        toolbar_toggle(ui, icon_circle, &mut self.state.show_contours, "Toggle Contours (C)");
                        toolbar_toggle(ui, icon_grid, &mut self.state.show_hatches, "Toggle Hatches (H)");
                        toolbar_toggle(ui, icon_arrow, &mut self.state.show_arrows, "Toggle Direction Arrows (A)");
                        toolbar_toggle(ui, icon_clock, &mut self.state.show_wait_markers, "Toggle Wait Time Markers (W)");

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // ── Parameter mode selector ──
                        ui.label(RichText::new("Color:").size(12.0).color(TEXT_SECONDARY));
                        let modes = [
                            (None, "None", "No parameter coloring"),
                            (Some(ParameterMode::Power), "Power", "Color by laser power"),
                            (Some(ParameterMode::Speed), "Speed", "Color by scan speed"),
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
                        let icon_info = LucideIcon::FileText.unicode();
                        let info_btn = egui::Button::new(RichText::new(format!("{} Info", icon_info)).size(13.0).color(TEXT_PRIMARY))
                            .fill(if self.state.show_file_info { TOGGLE_ACTIVE_BG } else { Color32::TRANSPARENT })
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(0.0, 30.0));
                        if ui.add(info_btn).on_hover_text("Layer info & parameters").clicked() {
                            self.state.show_file_info = !self.state.show_file_info;
                        }

                        // ── Controls button ──
                        let icon_kbd = LucideIcon::Keyboard.unicode();
                        let ctrl_btn = egui::Button::new(RichText::new(format!("{} Controls", icon_kbd)).size(13.0).color(TEXT_PRIMARY))
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

            // ━━━━━━━━━━━━━━━ RIGHT FLOATING LAYER SLIDER (full height) ━━━━━━━━━━━━━━━━
            if self.state.total_layers > 0 {
                let screen = ctx.screen_rect();
                let slider_panel_w = 64.0;
                let slider_margin = 12.0;
                let slider_top = TOOLBAR_HEIGHT + 12.0;
                let slider_bottom_margin = 12.0;
                let panel_h = (screen.height() - slider_top - slider_bottom_margin).max(200.0);

                // Fixed heights for controls
                let btn_h = 24.0;
                let goto_h = 40.0;
                let spacing = 8.0;
                let margin = 16.0; // inner margin total (8+8)
                let controls_h = btn_h + btn_h + goto_h + spacing * 3.0 + margin;
                let slider_h = (panel_h - controls_h).max(80.0);
                let max_layer = self.state.total_layers.saturating_sub(1);

                egui::Area::new(egui::Id::new("layer_slider_area"))
                    .anchor(Align2::RIGHT_TOP, egui::vec2(-slider_margin, slider_top))
                    .order(egui::Order::Foreground)
                    .interactable(true)
                    .movable(false)
                    .fixed_pos(egui::pos2(screen.right() - slider_margin - slider_panel_w, slider_top))
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
                            .inner_margin(egui::Margin::symmetric(8.0, 8.0))
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                                
                                // Allocate fixed size for entire content
                                let content_w = slider_panel_w - margin;
                                let content_h = panel_h - margin;
                                let (content_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(content_w, content_h),
                                    egui::Sense::hover(),
                                );
                                
                                // Place widgets manually within the allocated rect
                                let center_x = content_rect.center().x;
                                
                                // First button
                                let first_btn_rect = egui::Rect::from_center_size(
                                    egui::pos2(center_x, content_rect.top() + btn_h / 2.0),
                                    egui::vec2(36.0, btn_h),
                                );
                                let first_btn = egui::Button::new(RichText::new("⏮").size(14.0))
                                    .rounding(Rounding::same(4.0));
                                if ui.put(first_btn_rect, first_btn).on_hover_text("First layer (Home)").clicked() {
                                    new_layer = 0;
                                    layer_changed = true;
                                }

                                // Slider
                                let slider_top_y = content_rect.top() + btn_h + spacing;
                                let slider_rect = egui::Rect::from_center_size(
                                    egui::pos2(center_x, slider_top_y + slider_h / 2.0),
                                    egui::vec2(20.0, slider_h),
                                );
                                let mut layer = self.state.current_layer;
                                let slider = Slider::new(&mut layer, 0..=max_layer)
                                    .vertical()
                                    .show_value(false);
                                if ui.put(slider_rect, slider).changed() {
                                    new_layer = layer;
                                    layer_changed = true;
                                }

                                // Last button
                                let last_btn_y = slider_top_y + slider_h + spacing + btn_h / 2.0;
                                let last_btn_rect = egui::Rect::from_center_size(
                                    egui::pos2(center_x, last_btn_y),
                                    egui::vec2(36.0, btn_h),
                                );
                                let last_btn = egui::Button::new(RichText::new("⏭").size(14.0))
                                    .rounding(Rounding::same(4.0));
                                if ui.put(last_btn_rect, last_btn).on_hover_text("Last layer (End)").clicked() {
                                    new_layer = max_layer;
                                    layer_changed = true;
                                }

                                // Go to label
                                let goto_label_y = last_btn_y + btn_h / 2.0 + spacing + 8.0;
                                ui.painter().text(
                                    egui::pos2(center_x, goto_label_y),
                                    egui::Align2::CENTER_CENTER,
                                    "Go to",
                                    egui::FontId::proportional(10.0),
                                    TEXT_SECONDARY,
                                );

                                // Jump input
                                let jump_rect = egui::Rect::from_center_size(
                                    egui::pos2(center_x, goto_label_y + 18.0),
                                    egui::vec2(40.0, 20.0),
                                );
                                let mut jump_val = (self.state.current_layer + 1) as i64;
                                let dv = DragValue::new(&mut jump_val)
                                    .clamp_range(1..=(self.state.total_layers as i64))
                                    .speed(1.0);
                                if ui.put(jump_rect, dv).changed() {
                                    new_layer = (jump_val as usize)
                                        .saturating_sub(1)
                                        .min(self.state.total_layers.saturating_sub(1));
                                    layer_changed = true;
                                }
                            });
                    });
            }

            // ━━━━━━━━━━━━━━━ LEFT FLOATING GRADIENT SCALE ━━━━━━━━━━━━━━━━
            if self.state.param_mode.is_some() {
                let screen = ctx.screen_rect();
                let grad_top = TOOLBAR_HEIGHT + 12.0;
                let grad_margin = 12.0;
                let grad_panel_w = 92.0;
                let grad_bottom_margin = 12.0;
                let grad_panel_h = (screen.height() - grad_top - grad_bottom_margin).max(200.0);
                
                let bar_w = 16.0;
                let margin_h = 16.0; // inner margin (8+8)
                let margin_w = 20.0; // inner margin (10+10)
                // Fixed control heights
                let label_h = 16.0;
                let val_label_h = 14.0;
                let filter_section_h = 100.0; // Filter label + Min/Max labels + inputs + reset button
                let spacing = 4.0;
                let controls_h = label_h + val_label_h + val_label_h + filter_section_h + spacing * 4.0 + margin_h;
                let bar_h = (grad_panel_h - controls_h).max(50.0);

                egui::Area::new(egui::Id::new("gradient_scale_area"))
                    .fixed_pos(egui::pos2(grad_margin, grad_top))
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
                            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                                
                                // Allocate exact content size
                                let content_w = grad_panel_w - margin_w;
                                let content_h = grad_panel_h - margin_h;
                                let (content_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(content_w, content_h),
                                    egui::Sense::hover(),
                                );
                                
                                let center_x = content_rect.center().x;
                                let mut y = content_rect.top();
                                
                                // Mode label with units
                                let mode_label = match self.state.param_mode {
                                    Some(ParameterMode::Power) => "Power (W)",
                                    Some(ParameterMode::Speed) => "Speed (mm/s)",
                                    Some(ParameterMode::WaitTime) => "Wait (µs)",
                                    None => "",
                                };
                                ui.painter().text(
                                    egui::pos2(center_x, y + label_h / 2.0),
                                    egui::Align2::CENTER_CENTER,
                                    mode_label,
                                    egui::FontId::proportional(11.0),
                                    TEXT_PRIMARY,
                                );
                                y += label_h + spacing;

                                // Max value label
                                ui.painter().text(
                                    egui::pos2(center_x, y + val_label_h / 2.0),
                                    egui::Align2::CENTER_CENTER,
                                    format!("{:.1}", self.state.param_filter_max),
                                    egui::FontId::proportional(10.0),
                                    TEXT_SECONDARY,
                                );
                                y += val_label_h + spacing;

                                // Gradient bar
                                let bar_rect = egui::Rect::from_min_size(
                                    egui::pos2(center_x - bar_w / 2.0, y),
                                    egui::vec2(bar_w, bar_h),
                                );
                                let n = 64;
                                let seg_h = bar_rect.height() / n as f32;
                                for i in 0..n {
                                    let t = 1.0 - (i as f32 / (n - 1) as f32);
                                    let c = Color::viridis_gradient(t);
                                    let y0 = bar_rect.top() + i as f32 * seg_h;
                                    let seg = egui::Rect::from_min_max(
                                        egui::pos2(bar_rect.left(), y0),
                                        egui::pos2(bar_rect.right(), y0 + seg_h + 0.5),
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
                                    bar_rect,
                                    Rounding::same(2.0),
                                    Stroke::new(1.0, Color32::from_rgb(180, 180, 180)),
                                );

                                // Tick marks
                                let num_ticks = 5;
                                let val_min = self.state.param_filter_min;
                                let val_max = self.state.param_filter_max;
                                for tick_i in 0..num_ticks {
                                    let frac = tick_i as f32 / (num_ticks - 1) as f32;
                                    let tick_y = bar_rect.bottom() - frac * bar_rect.height();
                                    ui.painter().line_segment(
                                        [egui::pos2(bar_rect.right(), tick_y), egui::pos2(bar_rect.right() + 4.0, tick_y)],
                                        Stroke::new(1.0, Color32::from_rgb(100, 100, 100)),
                                    );
                                    if tick_i > 0 && tick_i < num_ticks - 1 {
                                        let val = val_min + frac * (val_max - val_min);
                                        ui.painter().text(
                                            egui::pos2(bar_rect.right() + 6.0, tick_y),
                                            egui::Align2::LEFT_CENTER,
                                            format!("{:.0}", val),
                                            egui::FontId::proportional(8.0),
                                            Color32::from_rgb(100, 100, 100),
                                        );
                                    }
                                }
                                y += bar_h + spacing;

                                // Min value label
                                ui.painter().text(
                                    egui::pos2(center_x, y + val_label_h / 2.0),
                                    egui::Align2::CENTER_CENTER,
                                    format!("{:.1}", self.state.param_filter_min),
                                    egui::FontId::proportional(10.0),
                                    TEXT_SECONDARY,
                                );
                                y += val_label_h + spacing;

                                // Filter section
                                ui.painter().text(
                                    egui::pos2(center_x, y + 8.0),
                                    egui::Align2::CENTER_CENTER,
                                    "Filter",
                                    egui::FontId::proportional(10.0),
                                    TEXT_PRIMARY,
                                );
                                y += 18.0;

                                let data_range = match self.state.param_mode {
                                    Some(ParameterMode::Power) => self.state.param_ranges.power,
                                    Some(ParameterMode::Speed) => self.state.param_ranges.speed,
                                    Some(ParameterMode::WaitTime) => self.state.param_ranges.wait_time,
                                    None => None,
                                };
                                let (lo, hi) = data_range.unwrap_or((0.0, 1.0));
                                let speed = (hi - lo).abs() * 0.01;

                                // Min label + input
                                ui.painter().text(
                                    egui::pos2(center_x, y + 6.0),
                                    egui::Align2::CENTER_CENTER,
                                    "Min",
                                    egui::FontId::proportional(10.0),
                                    TEXT_SECONDARY,
                                );
                                y += 14.0;
                                let min_rect = egui::Rect::from_center_size(
                                    egui::pos2(center_x, y + 9.0),
                                    egui::vec2(52.0, 18.0),
                                );
                                ui.put(
                                    min_rect,
                                    DragValue::new(&mut self.state.param_filter_min)
                                        .speed(speed.max(0.1))
                                        .clamp_range(lo..=self.state.param_filter_max),
                                );
                                y += 20.0;

                                // Max label + input
                                ui.painter().text(
                                    egui::pos2(center_x, y + 6.0),
                                    egui::Align2::CENTER_CENTER,
                                    "Max",
                                    egui::FontId::proportional(10.0),
                                    TEXT_SECONDARY,
                                );
                                y += 14.0;
                                let max_rect = egui::Rect::from_center_size(
                                    egui::pos2(center_x, y + 9.0),
                                    egui::vec2(52.0, 18.0),
                                );
                                ui.put(
                                    max_rect,
                                    DragValue::new(&mut self.state.param_filter_max)
                                        .speed(speed.max(0.1))
                                        .clamp_range(self.state.param_filter_min..=hi),
                                );
                                y += 22.0;

                                // Reset button
                                let reset_rect = egui::Rect::from_center_size(
                                    egui::pos2(center_x, y + 10.0),
                                    egui::vec2(52.0, 20.0),
                                );
                                let reset_btn = egui::Button::new(RichText::new("Reset").size(10.0))
                                    .rounding(Rounding::same(4.0));
                                if ui.put(reset_rect, reset_btn).clicked() {
                                    self.state.param_filter_min = lo;
                                    self.state.param_filter_max = hi;
                                }
                            });
                    });
            }

            // ━━━━━━━━━━━━━━━━━━ FILE INFO POPUP ━━━━━━━━━━━━━━━━━━━━━━
            if self.state.show_file_info {
                let file_icon = LucideIcon::FileText.unicode();
                egui::Window::new(RichText::new(format!("{} File Info", file_icon)).size(14.0).color(TEXT_PRIMARY))
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
                            ("Shift + Left Drag", "Zoom to selection"),
                            ("R", "Reset view (fit to content)"),
                            ("B", "Toggle boundaries"),
                            ("C", "Toggle contours"),
                            ("H", "Toggle hatches"),
                            ("A", "Toggle direction arrows"),
                            ("G", "Toggle background grid"),
                            ("Ctrl + P", "Take snapshot"),
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

            // ━━━━━━━━━━━━━━━━━━ FLOATING TOOL PANEL (top-right, horizontal) ━━━━━━━━━━━━━━━━━━
            {
                let screen = ctx.screen_rect();
                // Position: top-right, left of the layer slider (64 + 12 margin + 8 gap)
                let tool_panel_right_offset = 64.0 + 12.0 + 8.0;
                let tool_panel_top = TOOLBAR_HEIGHT + 8.0;

                egui::Area::new(egui::Id::new("tool_panel_area"))
                    .fixed_pos(egui::pos2(screen.right() - tool_panel_right_offset, tool_panel_top))
                    .pivot(Align2::RIGHT_TOP)
                    .order(egui::Order::Foreground)
                    .interactable(true)
                    .movable(false)
                    .show(ctx, |ui| {
                        egui::Frame::none()
                            .fill(Color32::from_rgba_premultiplied(255, 255, 255, 230))
                            .rounding(Rounding::same(8.0))
                            .shadow(egui::epaint::Shadow {
                                offset: egui::vec2(0.0, 1.0),
                                blur: 6.0,
                                spread: 0.0,
                                color: PANEL_SHADOW,
                            })
                            .inner_margin(egui::Margin::symmetric(4.0, 4.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                                    let btn_size = Vec2::new(28.0, 28.0);
                                    let icon_sz = 14.0;

                                    // Helper macro-like closure for tool buttons
                                    let mut tool_btn = |ui: &mut egui::Ui, icon: &str, tooltip: &str, active: bool| -> bool {
                                        let fill = if active { TOGGLE_ACTIVE_BG } else { Color32::TRANSPARENT };
                                        let tc = if active { ACCENT } else { TEXT_PRIMARY };
                                        let btn = egui::Button::new(RichText::new(icon).size(icon_sz).color(tc))
                                            .fill(fill)
                                            .rounding(Rounding::same(4.0))
                                            .min_size(btn_size);
                                        ui.add(btn).on_hover_text(tooltip).clicked()
                                    };

                                    // ── Zoom In ──
                                    if tool_btn(ui, &LucideIcon::ZoomIn.unicode().to_string(), "Zoom In", false) {
                                        zoom_in_requested = true;
                                    }
                                    // ── Zoom Out ──
                                    if tool_btn(ui, &LucideIcon::ZoomOut.unicode().to_string(), "Zoom Out", false) {
                                        zoom_out_requested = true;
                                    }
                                    // ── Fit View ──
                                    if tool_btn(ui, &LucideIcon::Maximize.unicode().to_string(), "Fit View (R)", false) {
                                        fit_view_requested = true;
                                    }
                                    // ── Zoom Selection ──
                                    let is_zoom_select = self.state.tool_state.active_mode == ToolMode::ZoomSelect;
                                    if tool_btn(ui, &LucideIcon::Scan.unicode().to_string(), "Zoom Selection (Shift+Drag)", is_zoom_select) {
                                        self.state.tool_state.active_mode = if is_zoom_select { ToolMode::None } else { ToolMode::ZoomSelect };
                                    }

                                    // ── Separator ──
                                    ui.add(egui::Separator::default().vertical().spacing(4.0));

                                    // ── Ruler ──
                                    let is_ruler = self.state.tool_state.active_mode == ToolMode::Ruler;
                                    if tool_btn(ui, &LucideIcon::Ruler.unicode().to_string(), "Ruler Tool", is_ruler) {
                                        if is_ruler {
                                            self.state.tool_state.active_mode = ToolMode::None;
                                            self.state.tool_state.ruler_start = None;
                                            self.state.tool_state.ruler_end = None;
                                        } else {
                                            self.state.tool_state.active_mode = ToolMode::Ruler;
                                        }
                                    }
                                    // ── Clear measurements ──
                                    if !self.state.tool_state.ruler_measurements.is_empty() {
                                        let btn = egui::Button::new(RichText::new(&LucideIcon::X.unicode().to_string()).size(11.0).color(TEXT_SECONDARY))
                                            .fill(Color32::TRANSPARENT)
                                            .rounding(Rounding::same(4.0))
                                            .min_size(Vec2::new(20.0, 28.0));
                                        if ui.add(btn).on_hover_text("Clear measurements").clicked() {
                                            self.state.tool_state.ruler_measurements.clear();
                                        }
                                    }
                                    // ── Scale bar toggle ──
                                    if tool_btn(ui, &LucideIcon::RulerDimensionLine.unicode().to_string(), "Scale Bar", self.state.tool_state.show_scale_bar) {
                                        self.state.tool_state.show_scale_bar = !self.state.tool_state.show_scale_bar;
                                    }

                                    // ── Separator ──
                                    ui.add(egui::Separator::default().vertical().spacing(4.0));

                                    // ── Grid ──
                                    if tool_btn(ui, &LucideIcon::Grid3x3.unicode().to_string(), "Grid (G)", self.state.tool_state.show_grid) {
                                        self.state.tool_state.show_grid = !self.state.tool_state.show_grid;
                                    }
                                    // ── Grid unit selector (compact, inline) ──
                                    if self.state.tool_state.show_grid {
                                        let units = [GridUnit::Millimeters, GridUnit::Micrometers, GridUnit::Inches];
                                        for unit in &units {
                                            let is_sel = self.state.tool_state.grid_unit == *unit;
                                            let fill = if is_sel { ACCENT } else { TOGGLE_INACTIVE_BG };
                                            let tc = if is_sel { Color32::WHITE } else { TEXT_PRIMARY };
                                            let btn = egui::Button::new(RichText::new(unit.label()).size(9.0).color(tc))
                                                .fill(fill)
                                                .rounding(Rounding::same(3.0))
                                                .min_size(Vec2::new(22.0, 20.0));
                                            if ui.add(btn).clicked() {
                                                self.state.tool_state.grid_unit = *unit;
                                            }
                                        }
                                    }

                                    // ── Separator ──
                                    ui.add(egui::Separator::default().vertical().spacing(4.0));

                                    // ── Snapshot ──
                                    if tool_btn(ui, &LucideIcon::Camera.unicode().to_string(), "Snapshot (Ctrl+P)", false) {
                                        snapshot_requested = true;
                                    }
                                });
                            });
                    });
            }

            // ━━━━━━━━━━━━━━━ SCALE BAR (bottom-left corner) ━━━━━━━━━━━━━━━━
            if self.state.tool_state.show_scale_bar {
                let screen = ctx.screen_rect();
                let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("scale_bar")));
                let bar_margin = 20.0;
                let bar_y = screen.bottom() - bar_margin;
                let bar_x_start = bar_margin + 10.0;

                // Pick a nice bar length: find largest round value ≤ 200px at current zoom
                let zoom = self.state.tool_state_zoom;
                if zoom > 0.0 {
                    let nice_values: &[f32] = &[0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0];
                    let max_px = 200.0;
                    let mut bar_world = 1.0_f32;
                    for &v in nice_values.iter().rev() {
                        if v * zoom <= max_px {
                            bar_world = v;
                            break;
                        }
                    }
                    let bar_px = bar_world * zoom;
                    let bar_x_end = bar_x_start + bar_px;
                    let tick_h = 6.0;
                    let bar_color = Color32::from_rgb(60, 60, 60);

                    // Bar line
                    painter.line_segment(
                        [egui::pos2(bar_x_start, bar_y), egui::pos2(bar_x_end, bar_y)],
                        Stroke::new(2.0, bar_color),
                    );
                    // Left tick
                    painter.line_segment(
                        [egui::pos2(bar_x_start, bar_y - tick_h), egui::pos2(bar_x_start, bar_y + tick_h)],
                        Stroke::new(2.0, bar_color),
                    );
                    // Right tick
                    painter.line_segment(
                        [egui::pos2(bar_x_end, bar_y - tick_h), egui::pos2(bar_x_end, bar_y + tick_h)],
                        Stroke::new(2.0, bar_color),
                    );
                    // Label
                    let unit = self.state.tool_state.grid_unit;
                    let value = unit.from_mm(bar_world);
                    let label = if value >= 1.0 {
                        format!("{:.0} {}", value, unit.label())
                    } else {
                        format!("{:.2} {}", value, unit.label())
                    };
                    painter.text(
                        egui::pos2((bar_x_start + bar_x_end) / 2.0, bar_y - tick_h - 4.0),
                        egui::Align2::CENTER_BOTTOM,
                        label,
                        egui::FontId::proportional(11.0),
                        bar_color,
                    );
                }
            }

            // ━━━━━━━━━━ RULER MEASUREMENTS overlay ━━━━━━━━━━━━━━━━━━
            {
                let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("ruler_overlay")));
                let ruler_color = Color32::from_rgb(220, 50, 50);
                let label_bg = Color32::from_rgba_premultiplied(255, 255, 255, 200);
                let unit = self.state.tool_state.grid_unit;

                // Draw persistent measurements
                for m in &self.state.tool_state.ruler_measurements {
                    let s = self.state.tool_state_view_transform.as_ref().map(|(vw, vh, view)| {
                        let s_start = view.world_to_screen(m.start.x, m.start.y, *vw, *vh);
                        let s_end = view.world_to_screen(m.end.x, m.end.y, *vw, *vh);
                        // Offset by toolbar
                        (
                            egui::pos2(s_start.x, s_start.y + TOOLBAR_HEIGHT),
                            egui::pos2(s_end.x, s_end.y + TOOLBAR_HEIGHT),
                        )
                    });
                    if let Some((p1, p2)) = s {
                        painter.line_segment([p1, p2], Stroke::new(2.0, ruler_color));
                        // End dots
                        painter.circle_filled(p1, 4.0, ruler_color);
                        painter.circle_filled(p2, 4.0, ruler_color);
                        // Label
                        let mid = egui::pos2((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0);
                        let dist_display = unit.from_mm(m.distance_mm);
                        let label = format!("{:.3} {}", dist_display, unit.label());
                        let text_rect = painter.text(
                            mid + egui::vec2(0.0, -12.0),
                            egui::Align2::CENTER_BOTTOM,
                            &label,
                            egui::FontId::proportional(12.0),
                            ruler_color,
                        );
                        painter.rect_filled(text_rect.expand(2.0), Rounding::same(2.0), label_bg);
                        painter.text(
                            mid + egui::vec2(0.0, -12.0),
                            egui::Align2::CENTER_BOTTOM,
                            &label,
                            egui::FontId::proportional(12.0),
                            ruler_color,
                        );
                    }
                }

                // Draw live ruler line (start → current cursor)
                if self.state.tool_state.active_mode == ToolMode::Ruler {
                    if let Some(start) = self.state.tool_state.ruler_start {
                        if let Some(end) = self.state.tool_state.ruler_end {
                            if let Some((vw, vh, view)) = self.state.tool_state_view_transform.as_ref() {
                                let s_start = view.world_to_screen(start.x, start.y, *vw, *vh);
                                let s_end = view.world_to_screen(end.x, end.y, *vw, *vh);
                                let p1 = egui::pos2(s_start.x, s_start.y + TOOLBAR_HEIGHT);
                                let p2 = egui::pos2(s_end.x, s_end.y + TOOLBAR_HEIGHT);
                                painter.line_segment([p1, p2], Stroke::new(1.5, ruler_color));
                                painter.circle_filled(p1, 4.0, ruler_color);
                                painter.circle_filled(p2, 3.0, Color32::from_rgba_premultiplied(220, 50, 50, 150));
                                let dist_mm = start.distance_to(&end);
                                let dist_display = unit.from_mm(dist_mm);
                                let mid = egui::pos2((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0);
                                let label = format!("{:.3} {}", dist_display, unit.label());
                                painter.text(
                                    mid + egui::vec2(0.0, -12.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    &label,
                                    egui::FontId::proportional(12.0),
                                    ruler_color,
                                );
                            }
                        }
                    }
                }
            }

            // ━━━━━━━━━━━ ZOOM SELECTION RECTANGLE overlay ━━━━━━━━━━━━━
            if let (Some(start), Some(end)) = (self.state.tool_state.zoom_rect_start, self.state.tool_state.zoom_rect_end) {
                let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("zoom_rect")));
                let rect = egui::Rect::from_two_pos(
                    egui::pos2(start.x, start.y),
                    egui::pos2(end.x, end.y),
                );
                painter.rect_filled(rect, 0.0, Color32::from_rgba_premultiplied(25, 118, 210, 30));
                painter.rect_stroke(rect, 0.0, Stroke::new(1.5, ACCENT));
            }

            // ━━━━━━━━━━━━━━━ GRID LABELS overlay ━━━━━━━━━━━━━━━━━━━━
            if self.state.tool_state.show_grid {
                if let Some((vw, vh, view)) = self.state.tool_state_view_transform.as_ref() {
                    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("grid_labels")));
                    let (_, major_spacing) = crate::infrastructure::rendering::GridRenderer::spacing(view.zoom);
                    let (vis_min, vis_max) = view.visible_bounds(*vw, *vh);
                    let unit = self.state.tool_state.grid_unit;
                    let label_color = Color32::from_rgb(100, 100, 100);
                    let label_bg = Color32::from_rgba_premultiplied(250, 250, 250, 200);

                    // Bottom edge labels (X axis)
                    let x_start = (vis_min.x / major_spacing).floor() * major_spacing;
                    let mut x = x_start;
                    while x <= vis_max.x {
                        let screen_pt = view.world_to_screen(x, vis_min.y, *vw, *vh);
                        let sx = screen_pt.x;
                        let sy = *vh + TOOLBAR_HEIGHT - 2.0; // bottom of viewport
                        if sx > 50.0 && sx < *vw - 20.0 {
                            let val = unit.from_mm(x);
                            let label = if val.abs() >= 1.0 { format!("{:.0}", val) } else { format!("{:.2}", val) };
                            let r = painter.text(
                                egui::pos2(sx, sy),
                                egui::Align2::CENTER_BOTTOM,
                                &label,
                                egui::FontId::proportional(9.0),
                                label_color,
                            );
                            painter.rect_filled(r.expand(1.0), 0.0, label_bg);
                            painter.text(
                                egui::pos2(sx, sy),
                                egui::Align2::CENTER_BOTTOM,
                                &label,
                                egui::FontId::proportional(9.0),
                                label_color,
                            );
                        }
                        x += major_spacing;
                    }

                    // Left edge labels (Y axis)
                    let y_start = (vis_min.y / major_spacing).floor() * major_spacing;
                    let mut y = y_start;
                    while y <= vis_max.y {
                        let screen_pt = view.world_to_screen(vis_min.x, y, *vw, *vh);
                        let sx = 4.0;
                        let sy = screen_pt.y + TOOLBAR_HEIGHT;
                        if sy > TOOLBAR_HEIGHT + 20.0 && sy < TOOLBAR_HEIGHT + *vh - 20.0 {
                            let val = unit.from_mm(y);
                            let label = if val.abs() >= 1.0 { format!("{:.0}", val) } else { format!("{:.2}", val) };
                            let r = painter.text(
                                egui::pos2(sx, sy),
                                egui::Align2::LEFT_CENTER,
                                &label,
                                egui::FontId::proportional(9.0),
                                label_color,
                            );
                            painter.rect_filled(r.expand(1.0), 0.0, label_bg);
                            painter.text(
                                egui::pos2(sx, sy),
                                egui::Align2::LEFT_CENTER,
                                &label,
                                egui::FontId::proportional(9.0),
                                label_color,
                            );
                        }
                        y += major_spacing;
                    }
                }
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

        // Build tool state output
        self.state.tool_state.snapshot_requested = snapshot_requested;
        let mut tool_output = self.state.tool_state.clone();
        // Propagate zoom/fit/snapshot requests via special fields read by the app
        // We use a simple convention: the app reads tool_state from UiOutput
        // and the tool_state already contains snapshot_requested, active_mode, etc.

        // Store output for deferred painting
        self.pending_output = Some(full_output);

        UiOutput {
            layer_index: self.state.current_layer,
            show_slices: self.state.show_slices,
            show_contours: self.state.show_contours,
            show_hatches: self.state.show_hatches,
            show_arrows: self.state.show_arrows,
            show_wait_markers: self.state.show_wait_markers,
            param_mode: self.state.param_mode,
            param_filter_min: self.state.param_filter_min,
            param_filter_max: self.state.param_filter_max,
            needs_repaint: needs_repaint || zoom_in_requested || zoom_out_requested || fit_view_requested,
            open_file_requested,
            tool_state: tool_output,
            zoom_in_requested,
            zoom_out_requested,
            fit_view_requested,
        }
    }

    /// Whether a zoom-in was requested this frame via toolbar button
    pub fn zoom_in_requested(&self) -> bool {
        // We use a frame-local flag — let the app poll via UiOutput
        false // handled through tool_state indirectly
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
