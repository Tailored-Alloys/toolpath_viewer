//! UI Module - egui integration for toolbar and floating panels
//!
//! Thin orchestrator that delegates to specialized components in `components/`.
//! Each component renders within layout regions computed by `layout.rs`.
//! Theme constants and shared helpers live in `theme.rs`.

use egui::{Context, ViewportId};
use egui_glow::Painter;
use egui_winit::EventResponse;
use egui_winit::State as EguiWinitState;
use std::sync::Arc;

use crate::application::ports::{GlobalUnits, ParameterMode};

use super::layout::{LayoutRegions, VisibilityFlags, TOOLBAR_HEIGHT};
use super::theme;
use super::components;

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
            show_grid: true,
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

/// Hover information for the vector under the cursor
#[derive(Debug, Clone)]
pub struct HoverInfo {
    /// Laser power in file-native units (watts)
    pub power: Option<f32>,
    /// Processing speed in file-native units (mm/s)
    pub speed: Option<f32>,
    /// Wait time in file-native units (microseconds)
    pub wait_time: Option<f32>,
    /// Screen position for tooltip placement
    pub screen_pos: (f32, f32),
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
    /// Global unit settings
    pub global_units: GlobalUnits,
    /// Zoom in button clicked
    pub zoom_in_requested: bool,
    /// Zoom out button clicked
    pub zoom_out_requested: bool,
    /// Fit view button clicked
    pub fit_view_requested: bool,
    /// Whether vector-by-vector view is enabled
    pub vector_view_enabled: bool,
    /// Current vector index (0-based) when vector view is active
    pub vector_index: usize,
    /// Whether vector playback is active
    pub vector_view_playing: bool,
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
    /// Global unit settings
    pub global_units: GlobalUnits,
    /// Cached zoom level for scale bar (updated from app each frame)
    pub tool_state_zoom: f32,
    /// Cached view transform for coordinate conversion (viewport_w, viewport_h, ViewState)
    pub tool_state_view_transform: Option<(f32, f32, crate::application::ports::ViewState)>,
    /// File currently being loaded in a background thread (None = idle)
    pub loading_file: Option<String>,
    /// Hover information for vector tooltip
    pub hover_info: Option<HoverInfo>,
    /// Whether vector-by-vector view is enabled
    pub vector_view_enabled: bool,
    /// Current vector index within the layer
    pub current_vector_index: usize,
    /// Total vectors in the current layer
    pub total_vectors_in_layer: usize,
    /// Whether vector-by-vector playback is active
    pub vector_view_playing: bool,
    /// Accumulated real time (seconds) for playback advancement
    pub playback_time_accumulator: f64,
    /// Playback speed multiplier (e.g. 0.5, 1.0, 2.0, 5.0)
    pub playback_speed: f32,
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
            show_wait_markers: false,
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
            global_units: GlobalUnits::default(),
            tool_state_zoom: 1.0,
            tool_state_view_transform: None,
            loading_file: None,
            hover_info: None,
            vector_view_enabled: false,
            current_vector_index: 0,
            total_vectors_in_layer: 0,
            vector_view_playing: false,
            playback_time_accumulator: 0.0,
            playback_speed: 1.0,
        }
    }
}

impl UiState {
    /// Reset all state to defaults for a new file load, preserving user
    /// preferences like global_units.
    pub fn reset_for_new_file(&mut self) {
        let preserved_units = self.global_units.clone();
        *self = UiState::default();
        self.global_units = preserved_units;
    }
}

/// Main UI renderer using egui
pub struct UiRenderer {
    pub ctx: Context,
    pub winit_state: EguiWinitState,
    pub painter: Painter,
    pub state: UiState,
    /// Deferred egui output for split run/paint cycle
    pending_output: Option<egui::FullOutput>,
    /// Logo texture for the toolbar
    logo_texture: Option<egui::TextureHandle>,
}

impl UiRenderer {
    /// Create a new UI renderer
    pub fn new(gl: Arc<glow::Context>, window: &winit::window::Window) -> Self {
        let ctx = Context::default();
        theme::setup_fonts(&ctx);
        theme::apply_light_theme(&ctx);
        egui_extras::install_image_loaders(&ctx);

        let winit_state = EguiWinitState::new(
            ctx.clone(),
            ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
        );
        let painter = Painter::new(gl, "", None)
            .expect("Failed to create egui painter");

        // Load logo texture from embedded PNG
        let logo_texture = {
            let logo_bytes = include_bytes!("assets/logo.png");
            image::load_from_memory(logo_bytes).ok().map(|img| {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.into_raw();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                ctx.load_texture("logo", color_image, egui::TextureOptions::LINEAR)
            })
        };

        Self {
            ctx,
            winit_state,
            painter,
            state: UiState::default(),
            pending_output: None,
            logo_texture,
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
        // Get raw input from winit state
        let raw_input = self.winit_state.take_egui_input(window);

        // Compute layout regions from current screen size and visibility flags
        let screen = self.ctx.input(|i| i.screen_rect());
        let flags = VisibilityFlags {
            has_layers: self.state.total_layers > 0,
            show_gradient: self.state.param_mode.is_some(),
            show_scale_bar: self.state.tool_state.show_scale_bar,
            show_file_info: self.state.show_file_info,
            show_controls: self.state.show_controls,
            show_grid: self.state.tool_state.show_grid,
            vector_view_active: self.state.vector_view_enabled,
        };
        let regions = LayoutRegions::compute(screen, &flags);

        // Track component outputs
        let mut open_file_requested = false;
        let mut zoom_in_requested = false;
        let mut zoom_out_requested = false;
        let mut fit_view_requested = false;
        let mut snapshot_requested = false;
        let mut layer_changed = false;
        let mut new_layer = self.state.current_layer;
        let mut vector_changed = false;
        let mut new_vector = self.state.current_vector_index;

        // Run egui
        let full_output = self.ctx.run(raw_input, |ctx| {
            // ── Toolbar ──
            let toolbar_out = components::show_toolbar(
                ctx,
                regions.compact_toolbar,
                &mut self.state.show_contours,
                &mut self.state.show_hatches,
                &mut self.state.show_arrows,
                &mut self.state.vector_view_enabled,
                &mut self.state.param_mode,
                &mut self.state.show_file_info,
                &mut self.state.show_controls,
                &mut self.state.global_units,
            );
            open_file_requested = toolbar_out.open_file_requested;

            // ── Reset param filter on mode change ──
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

            // ── Layer slider ──
            if let Some(ref slider_region) = regions.layer_slider {
                let slider_out = components::show_layer_slider(
                    ctx,
                    slider_region,
                    self.state.current_layer,
                    self.state.total_layers,
                    self.state.current_z,
                    &self.state.global_units,
                );
                if slider_out.layer_changed {
                    layer_changed = true;
                    new_layer = slider_out.new_layer;
                }
            }

            // ── Vector slider ──
            if let Some(ref vs_region) = regions.vector_slider {
                let vs_out = components::show_vector_slider(
                    ctx,
                    vs_region,
                    self.state.current_vector_index,
                    self.state.total_vectors_in_layer,
                    self.state.vector_view_playing,
                    &mut self.state.playback_speed,
                );
                if vs_out.vector_changed {
                    vector_changed = true;
                    new_vector = vs_out.new_vector;
                }
                if vs_out.playing_toggled {
                    self.state.vector_view_playing = !self.state.vector_view_playing;
                    if self.state.vector_view_playing {
                        self.state.playback_time_accumulator = 0.0;
                        // If at last vector, reset to 0 to replay
                        if self.state.current_vector_index >= self.state.total_vectors_in_layer.saturating_sub(1) {
                            self.state.current_vector_index = 0;
                        }
                    }
                }
            }

            // ── Gradient scale ──
            if let Some(ref grad_region) = regions.gradient_panel {
                if let Some(param_mode) = self.state.param_mode {
                    components::show_gradient_scale(
                        ctx,
                        grad_region,
                        param_mode,
                        &mut self.state.param_filter_min,
                        &mut self.state.param_filter_max,
                        &self.state.param_ranges,
                        &self.state.global_units,
                    );
                }
            }

            // ── Tool panel ──
            let tool_out = components::show_tool_panel(
                ctx,
                &regions.tool_panel,
                &mut self.state.tool_state.active_mode,
                &mut self.state.tool_state.ruler_start,
                &mut self.state.tool_state.ruler_end,
                &mut self.state.tool_state.ruler_measurements,
                &mut self.state.tool_state.show_grid,
            );
            zoom_in_requested = tool_out.zoom_in_requested;
            zoom_out_requested = tool_out.zoom_out_requested;
            fit_view_requested = tool_out.fit_view_requested;
            snapshot_requested = tool_out.snapshot_requested;

            // ── Overlays (rendered first so popups appear on top) ──
            if self.state.tool_state.show_scale_bar {
                let left_offset = if let Some(ref gp) = regions.gradient_panel {
                    gp.pos.x + super::layout::GRADIENT_PANEL_WIDTH
                } else {
                    0.0
                };
                components::show_scale_bar(
                    ctx,
                    self.state.tool_state_zoom,
                    self.state.global_units.length,
                    left_offset,
                );
            }

            components::show_ruler_overlay(
                ctx,
                &self.state.tool_state.ruler_measurements,
                self.state.tool_state.ruler_start,
                self.state.tool_state.ruler_end,
                self.state.tool_state_view_transform.as_ref(),
                self.state.global_units.length,
            );

            // Set crosshair cursor when a measurement dot has been placed (waiting for second point)
            if self.state.tool_state.ruler_start.is_some() {
                ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
            }

            if let (Some(start), Some(end)) = (
                self.state.tool_state.zoom_rect_start,
                self.state.tool_state.zoom_rect_end,
            ) {
                components::show_zoom_rect(ctx, start, end);
            }

            if self.state.tool_state.show_grid {
                if let Some(ref vt) = self.state.tool_state_view_transform {
                    components::show_grid_labels(ctx, vt, self.state.global_units.length);
                }
            }

            // ── Hover tooltip ──
            if let Some(ref hover) = self.state.hover_info {
                components::show_hover_tooltip(ctx, hover, &self.state.global_units);
            }

            // ── File info popup (rendered last for high z-order) ──
            if self.state.show_file_info {
                components::show_file_info(ctx, &self.state.vector_counts);
            }

            // ── Controls popup (rendered last for high z-order) ──
            if self.state.show_controls {
                components::show_controls_popup(ctx);
            }

            // ── Empty state message (no file loaded) ──
            if self.state.total_layers == 0 && self.state.loading_file.is_none() {
                let screen = ctx.screen_rect();
                let center = egui::pos2(
                    screen.center().x,
                    (screen.top() + TOOLBAR_HEIGHT + screen.bottom()) / 2.0,
                );
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Foreground,
                    egui::Id::new("empty_state"),
                ));
                painter.text(
                    center + egui::vec2(0.0, -10.0),
                    egui::Align2::CENTER_CENTER,
                    "No file loaded",
                    egui::FontId::proportional(20.0),
                    egui::Color32::from_rgb(160, 160, 160),
                );
                painter.text(
                    center + egui::vec2(0.0, 16.0),
                    egui::Align2::CENTER_CENTER,
                    "Drop an ILT file or press Ctrl+O to open",
                    egui::FontId::proportional(13.0),
                    egui::Color32::from_rgb(190, 190, 190),
                );
                painter.text(
                    center + egui::vec2(0.0, 46.0),
                    egui::Align2::CENTER_CENTER,
                    crate::APP_DEVELOPER,
                    egui::FontId::proportional(11.0),
                    egui::Color32::from_rgb(120, 120, 120),
                );
            }


            // ── Loading overlay ──
            if let Some(ref file_name) = self.state.loading_file {
                let screen = ctx.screen_rect();
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Tooltip,
                    egui::Id::new("loading_overlay"),
                ));
                // Semi-transparent backdrop
                painter.rect_filled(
                    screen,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 140),
                );
                let center = screen.center();
                // Animated dots
                let t = ctx.input(|i| i.time);
                let dots = match ((t * 2.0) as usize) % 4 {
                    0 => "",
                    1 => ".",
                    2 => "..",
                    _ => "...",
                };
                painter.text(
                    center + egui::vec2(0.0, -14.0),
                    egui::Align2::CENTER_CENTER,
                    format!("Loading{}", dots),
                    egui::FontId::proportional(22.0),
                    egui::Color32::WHITE,
                );
                painter.text(
                    center + egui::vec2(0.0, 14.0),
                    egui::Align2::CENTER_CENTER,
                    file_name,
                    egui::FontId::proportional(14.0),
                    egui::Color32::from_rgb(200, 200, 200),
                );
                // Request continuous redraws for animation
                ctx.request_repaint();
            }
        });

        // Update state if layer changed via slider
        if layer_changed {
            self.state.current_layer = new_layer;
        }

        // Update state if vector changed via slider
        if vector_changed {
            self.state.current_vector_index = new_vector;
            // Pause playback on manual slider interaction
            if self.state.vector_view_playing {
                self.state.vector_view_playing = false;
                self.state.playback_time_accumulator = 0.0;
            }
        }

        // Handle platform output
        self.winit_state
            .handle_platform_output(window, full_output.platform_output.clone());

        let needs_repaint = full_output
            .viewport_output
            .get(&ViewportId::ROOT)
            .map(|v| v.repaint_delay.is_zero())
            .unwrap_or(false);

        // Build tool state output
        self.state.tool_state.snapshot_requested = snapshot_requested;
        let tool_output = self.state.tool_state.clone();

        // Store output for deferred painting
        self.pending_output = Some(full_output);

        UiOutput {
            layer_index: self.state.current_layer,
            show_slices: self.state.show_slices,
            show_contours: self.state.show_contours,
            show_hatches: self.state.show_hatches,
            show_arrows: self.state.show_arrows,
            show_wait_markers: self.state.param_mode == Some(crate::application::ports::ParameterMode::WaitTime),
            param_mode: self.state.param_mode,
            param_filter_min: self.state.param_filter_min,
            param_filter_max: self.state.param_filter_max,
            needs_repaint: needs_repaint
                || zoom_in_requested
                || zoom_out_requested
                || fit_view_requested,
            open_file_requested,
            tool_state: tool_output,
            global_units: self.state.global_units.clone(),
            zoom_in_requested,
            zoom_out_requested,
            fit_view_requested,
            vector_view_enabled: self.state.vector_view_enabled,
            vector_index: self.state.current_vector_index,
            vector_view_playing: self.state.vector_view_playing,
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
