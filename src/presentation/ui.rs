//! UI Module - egui integration for toolbar and floating panels
//!
//! Thin orchestrator that delegates to specialized components in `components/`.
//! Each component renders within layout regions computed by `layout.rs`.
//! Theme constants and shared helpers live in `theme.rs`.

use egui::{Context, ViewportId};
use egui_glow::Painter;
use egui_winit::EventResponse;
use egui_winit::State as EguiWinitState;
use lucide_icons::Icon as LucideIcon;
use std::sync::Arc;

use crate::application::ports::{ColorMode, GlobalUnits, GradientPaletteId, PaletteId, ParameterMode, ThemeMode, ViewMode};

use super::layout::{LayoutRegions, VisibilityFlags, TOOLBAR_BOTTOM, TAB_BAR_HEIGHT};
use super::palette::ThemePalette;
use super::theme;
use super::components;
use super::tab_state::TabManager;

// ── Sidebar Tab ──────────────────────────────────────────────────────────

/// Active sidebar tab in the Activity Bar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    /// Toolpaths / file browser tab
    Toolpaths,
}

impl Default for SidebarTab {
    fn default() -> Self {
        SidebarTab::Toolpaths
    }
}

// ── Split Pane ────────────────────────────────────────────────────────────

/// Which pane the cursor is currently over (in Split mode).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitPane {
    Left,
    Right,
}

impl Default for SplitPane {
    fn default() -> Self {
        SplitPane::Left
    }
}

// ── Tool Mode types ───────────────────────────────────────────────────────

/// Active tool mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolMode {
    /// Pan mode — left-drag pans viewport (touchpad friendly)
    Pan,
    /// Zoom selection mode — left-drag draws zoom rectangle
    ZoomSelect,
    /// Ruler/measure mode — left-click places measurement points
    Ruler,
}

impl Default for ToolMode {
    fn default() -> Self {
        ToolMode::Pan
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
}

impl Default for ToolState {
    fn default() -> Self {
        Self {
            active_mode: ToolMode::Pan,
            zoom_rect_start: None,
            zoom_rect_end: None,
            ruler_start: None,
            ruler_end: None,
            ruler_measurements: Vec::new(),
            show_grid: true,
            show_scale_bar: true,
            snapshot_requested: false,
        }
    }
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
    /// Wait time filter min/max
    pub wait_filter_min: f32,
    pub wait_filter_max: f32,
    /// Whether UI wants to repaint (hover, drag, etc.)
    pub needs_repaint: bool,
    /// User requested to open a file via load button
    pub open_file_requested: bool,
    /// User requested to add more files via file panel
    pub add_files_requested: bool,
    /// File ID to remove (if any)
    pub remove_file: Option<usize>,
    /// File ID to toggle visibility (if any)
    pub toggle_file_visibility: Option<usize>,
    /// Active color mode
    pub color_mode: ColorMode,
    /// Active view mode (overlay/tab/split)
    pub view_mode: ViewMode,
    /// Active tab file ID (for Tab mode)
    pub active_tab_file: Option<usize>,
    /// Split ratio (0.0–1.0, default 0.5)
    pub split_ratio: f32,
    /// Tab bar: user clicked a tab to switch to it
    pub switch_tab: Option<usize>,
    /// Tab bar: user clicked × to close a tab
    pub close_tab: Option<usize>,
    /// Tab bar: user changed the split partner
    pub split_partner_changed: Option<usize>,
    /// Drag-to-split: activate split mode with this tab as partner
    pub drag_to_split: Option<usize>,
    /// Overlay mode: toggle visibility of this file ID
    pub overlay_toggle: Option<usize>,
    /// Overlay mode: select all files visible
    pub overlay_select_all: bool,
    /// Overlay mode: show only this file (file ID)
    pub overlay_select_only: Option<usize>,
    /// Context menu: close all tabs except this one
    pub close_others: Option<usize>,
    /// Context menu: close all tabs
    pub close_all: bool,
    /// Context menu: show in split view
    pub show_in_split: Option<usize>,
    /// Split mode: user clicked a tab in the right pane
    pub split_right_switch: Option<usize>,
    /// Tab reorder: (from_index, to_index) in open_tab_ids
    pub reorder_tab: Option<(usize, usize)>,
    /// Split sync toggle was clicked
    pub split_sync_toggled: bool,
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
    /// Whether theme/palette changed this frame
    pub theme_changed: bool,
    /// Whether canvas settings changed this frame
    pub canvas_changed: bool,
    /// Whether a gradient scale palette was changed this frame
    pub gradient_palette_changed: bool,
    /// The active palette (for renderer sync)
    pub active_palette: ThemePalette,
    /// Viewport rect from LayoutRegions (single source of truth for GL + input)
    pub viewport_rect: egui::Rect,
    /// Total sidebar width (activity bar + content) in logical pixels
    pub sidebar_total_width: f32,
    /// Height of bottom bars (status bar + optional vector player) in logical pixels
    pub bottom_bar_height: f32,
    /// Header height (toolbar + optional tab bar) in logical pixels
    pub header_height: f32,
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
    /// Active color mode
    pub color_mode: ColorMode,
    /// Parameter filter min
    pub param_filter_min: f32,
    /// Parameter filter max
    pub param_filter_max: f32,
    /// Wait time filter min
    pub wait_filter_min: f32,
    /// Wait time filter max
    pub wait_filter_max: f32,
    /// Global parameter ranges from the file
    pub param_ranges: ParamRanges,
    /// Previous param mode (to detect mode changes and reset filter)
    prev_param_mode: Option<ParameterMode>,
    /// Whether file info popup is open
    pub show_file_info: bool,
    /// Whether controls popup is open
    pub show_controls: bool,
    /// Whether the sidebar content panel is open
    pub sidebar_open: bool,
    /// Active sidebar tab
    pub active_sidebar_tab: SidebarTab,
    /// Dynamic sidebar content panel width (user-resizable)
    pub sidebar_content_width: f32,
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
    /// Active theme mode
    pub theme_mode: ThemeMode,
    /// Active palette id (for the current resolved mode)
    pub active_palette_id: PaletteId,
    /// Palette id for light mode
    pub light_palette_id: PaletteId,
    /// Palette id for dark mode
    pub dark_palette_id: PaletteId,
    /// Resolved palette
    pub active_palette: ThemePalette,
    /// Whether the preferences dialog is open
    pub show_preferences: bool,
    /// Whether the system reports dark mode
    pub system_is_dark: bool,
    /// Whether theme/palette changed this frame (signals renderer update)
    pub theme_changed: bool,
    /// Whether canvas settings changed this frame
    pub canvas_changed: bool,
    /// Whether multiple files are loaded (controls ByFile color mode availability)
    pub has_multiple_files: bool,
    /// Active view mode
    pub view_mode: ViewMode,
    /// Active tab file ID (for Tab mode)
    pub active_tab_file: Option<usize>,
    /// Split ratio (0.0–1.0, default 0.5) for Split mode
    pub split_ratio: f32,
    /// Mouse world coordinates for status bar (None if not hovering viewport)
    pub mouse_world_pos: Option<(f32, f32)>,
    /// Tab currently being dragged (file ID), for drag-to-split
    pub tab_drag_id: Option<usize>,
    /// Current drag position on screen (logical pixels)
    pub tab_drag_pos: Option<egui::Pos2>,
    /// Cached viewport rect from LayoutRegions (updated each frame, used by event handlers)
    pub cached_viewport_rect: egui::Rect,
    /// Canvas rendering settings (line thickness, markers, grid, etc.)
    pub canvas_settings: crate::application::ports::CanvasConfig,
    /// Flag to request focus on the "Go to layer" input field (consumed by layer_slider)
    pub focus_layer_input: bool,
    /// Which split pane the mouse is currently over (Split mode only)
    pub active_split_pane: SplitPane,
    /// Whether the gradient scale overlays are expanded [param, wait]
    pub gradient_expanded: [bool; 2],
    /// Gradient palette for the parameter scale overlay
    pub gradient_palette_id: GradientPaletteId,
    /// Whether a gradient scale palette changed this frame
    pub gradient_palette_changed: bool,
    /// Shared update state (for auto-update notifications)
    pub update_state: crate::infrastructure::updater::updater::SharedUpdateState,
    /// Whether the user dismissed the update notification
    pub update_dismissed: bool,
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
            color_mode: ColorMode::ByVectorType,
            param_filter_min: 0.0,
            param_filter_max: f32::MAX,
            wait_filter_min: 0.0,
            wait_filter_max: f32::MAX,
            param_ranges: ParamRanges::default(),
            prev_param_mode: None,
            show_file_info: false,
            show_controls: false,
            sidebar_open: false,
            active_sidebar_tab: SidebarTab::default(),
            sidebar_content_width: super::layout::SIDEBAR_WIDTH,
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
            theme_mode: ThemeMode::default(),
            active_palette_id: PaletteId::default(),
            light_palette_id: PaletteId::default(),
            dark_palette_id: PaletteId::default(),
            active_palette: ThemePalette::default(),
            show_preferences: false,
            system_is_dark: false,
            theme_changed: false,
            canvas_changed: false,
            has_multiple_files: false,
            view_mode: ViewMode::Tab,
            active_tab_file: None,
            split_ratio: 0.5,
            mouse_world_pos: None,
            tab_drag_id: None,
            tab_drag_pos: None,
            cached_viewport_rect: egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0)),
            canvas_settings: crate::application::ports::CanvasConfig::default(),
            focus_layer_input: false,
            active_split_pane: SplitPane::default(),
            gradient_expanded: [false; 2],
            gradient_palette_id: GradientPaletteId::default(),
            gradient_palette_changed: false,
            update_state: crate::infrastructure::updater::updater::new_shared_state(),
            update_dismissed: false,
        }
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
    pub fn run_ui(&mut self, window: &winit::window::Window, files: &crate::application::dto::FileCollection, tab_manager: &mut TabManager) -> UiOutput {
        // ── Sync active tab state → UiState (so existing components work unchanged) ──
        if let Some(tab) = tab_manager.active_tab() {
            self.state.show_slices = tab.show_slices;
            self.state.show_contours = tab.show_contours;
            self.state.show_hatches = tab.show_hatches;
            self.state.show_arrows = tab.show_arrows;
            self.state.show_wait_markers = tab.show_wait_markers;
            self.state.vector_view_enabled = tab.vector_view_enabled;
            self.state.current_vector_index = tab.current_vector_index;
            self.state.total_vectors_in_layer = tab.total_vectors_in_layer;
            self.state.vector_view_playing = tab.vector_view_playing;
            self.state.playback_speed = tab.playback_speed;
            let nav = tab.navigation.state();
            self.state.current_layer = nav.current_index;
            self.state.total_layers = nav.total_layers;
            self.state.current_z = nav.current_z;
        }
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
            sidebar_open: self.state.sidebar_open,
            has_tab_bar: tab_manager.has_tabs(),
            sidebar_content_width: self.state.sidebar_content_width,
            gradient_expanded: self.state.gradient_expanded,
        };
        let regions = LayoutRegions::compute(screen, &flags);

        // Track component outputs
        let open_file_requested = false;
        let mut zoom_in_requested = false;
        let mut zoom_out_requested = false;
        let mut fit_view_requested = false;
        let mut snapshot_requested = false;
        let mut layer_changed = false;
        let mut new_layer = self.state.current_layer;
        let mut vector_changed = false;
        let mut new_vector = self.state.current_vector_index;
        let mut add_files_requested = false;
        let mut remove_file: Option<usize> = None;
        let mut toggle_file_visibility: Option<usize> = None;
        let mut switch_tab: Option<usize> = None;
        let mut close_tab_action: Option<usize> = None;
        let mut view_mode_changed: Option<ViewMode> = None;
        let mut split_partner_changed: Option<usize> = None;
        let mut drag_to_split: Option<usize> = None;
        let mut overlay_toggle: Option<usize> = None;
        let mut overlay_select_all = false;
        let mut overlay_select_only: Option<usize> = None;
        let mut close_others: Option<usize> = None;
        let mut close_all = false;
        let mut show_in_split: Option<usize> = None;
        let mut split_right_switch: Option<usize> = None;
        let mut reorder_tab: Option<(usize, usize)> = None;
        let mut split_sync_toggled = false;

        // Build tab info for the tab bar component
        let tab_infos: Vec<components::TabInfo> = tab_manager.open_tab_ids.iter()
            .filter_map(|&id| {
                files.files.iter().find(|f| f.id == id).map(|f| components::TabInfo {
                    file_id: f.id,
                    name: f.name.clone(),
                    color: f.color,
                    is_active: tab_manager.active_tab_id == Some(f.id),
                    is_overlay_visible: tab_manager.is_overlay_visible(f.id),
                })
            })
            .collect();
        let tab_bar_view_mode = self.state.view_mode;
        let tab_bar_has_multiple = files.len() >= 2;
        let tab_bar_split_partner = tab_manager.split_partner_id;
        let tab_bar_split_right_active = tab_manager.split_right_active_id;
        let tab_bar_overlay_visible = tab_manager.overlay_visible_ids.clone();

        // Run egui
        let full_output = self.ctx.run(raw_input, |ctx| {
            // ── Panel ordering ──
            // egui panels claim space in the order they are added.
            // Correct order: Toolbar (top) → Status bar (bottom) → Vector player (bottom)
            // → Activity bar (left) → Sidebar content (left) → Tab bar (top, remaining width)
            // → floating Areas (layer slider, tool panel, overlays)

            // ── Toolbar (must be first TopBottomPanel so it claims the top row) ──
            let _toolbar_out = components::show_toolbar(
                ctx,
                regions.compact_toolbar,
                &mut self.state.show_contours,
                &mut self.state.show_hatches,
                &mut self.state.show_arrows,
                &mut self.state.show_wait_markers,
                &mut self.state.vector_view_enabled,
                &mut self.state.param_mode,
                &mut self.state.color_mode,
                &mut self.state.show_file_info,
                &mut self.state.show_controls,
                &mut self.state.show_preferences,
                self.state.has_multiple_files,
                flags.has_tab_bar,
            );

            // ── Status bar (bottom, full width — claims bottom space early) ──
            {
                let hover_info = &self.state.hover_info;
                let sb_info = components::status_bar::StatusBarInfo {
                    zoom: self.state.tool_state_zoom,
                    current_layer: self.state.current_layer,
                    total_layers: self.state.total_layers,
                    current_z: self.state.current_z,
                    mouse_world_x: self.state.mouse_world_pos.map(|(x, _)| x),
                    mouse_world_y: self.state.mouse_world_pos.map(|(_, y)| y),
                    hover_power: hover_info.as_ref().and_then(|h| h.power),
                    hover_speed: hover_info.as_ref().and_then(|h| h.speed),
                    hover_wait: hover_info.as_ref().and_then(|h| h.wait_time),
                };
                components::show_status_bar(
                    ctx,
                    &regions.status_bar,
                    &sb_info,
                    &mut self.state.global_units,
                );
            }

            // ── Vector player bar (bottom, above status bar) ──
            if let Some(ref vp_region) = regions.vector_player {
                let vs_out = components::show_vector_player(
                    ctx,
                    vp_region,
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

            // ── Activity Bar (left SidePanel — claims left strip between toolbar and bottom panels) ──
            {
                let ab_out = components::show_activity_bar(
                    ctx,
                    self.state.active_sidebar_tab,
                    self.state.sidebar_open,
                );
                if let Some(clicked_tab) = ab_out.clicked_tab {
                    if clicked_tab == self.state.active_sidebar_tab && self.state.sidebar_open {
                        // Clicking active tab closes the content panel
                        self.state.sidebar_open = false;
                    } else {
                        // Switch to clicked tab and open
                        self.state.active_sidebar_tab = clicked_tab;
                        self.state.sidebar_open = true;
                    }
                }
            }

            // ── Sidebar content panel (left SidePanel — Toolpaths tab) ──
            {
                let sidebar_out = components::show_sidebar(
                    ctx,
                    &regions.sidebar,
                    files,
                    self.state.color_mode,
                    self.state.view_mode,
                    self.state.active_tab_file,
                );
                if sidebar_out.add_files_requested {
                    add_files_requested = true;
                }
                if sidebar_out.remove_file.is_some() {
                    remove_file = sidebar_out.remove_file;
                }
                if sidebar_out.toggle_visibility.is_some() {
                    toggle_file_visibility = sidebar_out.toggle_visibility;
                }
                if sidebar_out.select_tab_file.is_some() {
                    self.state.active_tab_file = sidebar_out.select_tab_file;
                    // Also trigger a tab switch in the tab manager
                    if switch_tab.is_none() {
                        switch_tab = sidebar_out.select_tab_file;
                    }
                }
            }

            // ── Tab bar (top, after sidebar — only spans remaining width right of sidebar) ──
            if let Some(ref tab_bar_rect) = regions.tab_bar {
                let tb_out = components::show_tab_bar(
                    ctx,
                    tab_bar_rect,
                    &tab_infos,
                    tab_bar_view_mode,
                    tab_bar_has_multiple,
                    tab_bar_split_partner,
                    tab_bar_split_right_active,
                    &tab_bar_overlay_visible,
                    self.state.split_ratio,
                    regions.viewport.width(),
                );
                if tb_out.switch_to_tab.is_some() {
                    switch_tab = tb_out.switch_to_tab;
                }
                if tb_out.close_tab.is_some() {
                    close_tab_action = tb_out.close_tab;
                }
                if tb_out.view_mode_changed.is_some() {
                    view_mode_changed = tb_out.view_mode_changed;
                }
                if tb_out.split_partner_changed.is_some() {
                    split_partner_changed = tb_out.split_partner_changed;
                }
                if tb_out.overlay_toggle.is_some() {
                    overlay_toggle = tb_out.overlay_toggle;
                }
                if tb_out.overlay_select_all {
                    overlay_select_all = true;
                }
                if tb_out.overlay_select_only.is_some() {
                    overlay_select_only = tb_out.overlay_select_only;
                }
                if tb_out.close_others.is_some() {
                    close_others = tb_out.close_others;
                }
                if tb_out.close_all {
                    close_all = true;
                }
                if tb_out.show_in_split.is_some() {
                    show_in_split = tb_out.show_in_split;
                }
                if tb_out.split_right_switch.is_some() {
                    split_right_switch = tb_out.split_right_switch;
                }
                if tb_out.reorder_tab.is_some() {
                    reorder_tab = tb_out.reorder_tab;
                }

                // ── Drag-to-split handling ──
                if let Some(drag_id) = tb_out.dragging_tab {
                    self.state.tab_drag_id = Some(drag_id);
                    self.state.tab_drag_pos = tb_out.drag_pos;
                }
                if let Some((released_id, pos)) = tb_out.drag_released {
                    // Check if the tab was dropped in the canvas area
                    let vp = regions.viewport;
                    if vp.contains(pos) && tab_bar_has_multiple {
                        // Dropped in canvas → activate split mode
                        // If dropped on right half → dragged tab becomes split partner
                        // If dropped on left half → dragged tab becomes active, old active becomes partner
                        let midpoint = vp.left() + vp.width() / 2.0;
                        if pos.x > midpoint {
                            // Right side: dragged tab becomes split partner
                            drag_to_split = Some(released_id);
                        } else {
                            // Left side: dragged tab becomes active, current active becomes partner
                            let current_active = self.state.active_tab_file;
                            switch_tab = Some(released_id);
                            if let Some(old) = current_active {
                                if old != released_id {
                                    drag_to_split = Some(old);
                                }
                            }
                        }
                    }
                    self.state.tab_drag_id = None;
                    self.state.tab_drag_pos = None;
                }
                if tb_out.dragging_tab.is_none() && tb_out.drag_released.is_none() {
                    // No drag activity this frame — clear stale drag state
                    if self.state.tab_drag_id.is_some() {
                        self.state.tab_drag_id = None;
                        self.state.tab_drag_pos = None;
                    }
                }
            }

            // ── Reset param filter on mode change ──
            if self.state.param_mode != self.state.prev_param_mode {
                let range = match self.state.param_mode {
                    Some(ParameterMode::Power) => self.state.param_ranges.power,
                    Some(ParameterMode::Speed) => self.state.param_ranges.speed,
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

            // ── Layer slider (floating Area) ──
            if let Some(ref slider_region) = regions.layer_slider {
                let slider_out = components::show_layer_slider(
                    ctx,
                    slider_region,
                    self.state.current_layer,
                    self.state.total_layers,
                    self.state.current_z,
                    &self.state.global_units,
                    &mut self.state.focus_layer_input,
                );
                if slider_out.layer_changed {
                    layer_changed = true;
                    new_layer = slider_out.new_layer;
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

            // ── Gradient scale overlays (floating, to the left of tool panel) ──
            self.state.gradient_palette_changed = false;
            {
                let show_gradient = self.state.param_mode.is_some();

                if show_gradient {
                    if let (Some(pm), Some(region)) =
                        (self.state.param_mode, regions.gradient_scales.get(0))
                    {
                        let grad_out = components::gradient_scale::show_gradient_scale(
                            ctx,
                            region,
                            pm,
                            &mut self.state.param_filter_min,
                            &mut self.state.param_filter_max,
                            &self.state.param_ranges,
                            &self.state.global_units,
                            &mut self.state.gradient_palette_id,
                            &mut self.state.gradient_expanded[0],
                        );
                        if grad_out.palette_changed {
                            self.state.gradient_palette_changed = true;
                        }
                    }
                }
            }

            // (file panel now in sidebar)

            // ── Split divider overlay (draggable, with sync toggle on hover) ──
            if self.state.view_mode == ViewMode::Split {
                let vp = regions.viewport;
                let divider_x = vp.left() + vp.width() * self.state.split_ratio;
                let divider_top = vp.top();
                let divider_bottom = vp.bottom();
                let gap = super::layout::SPLIT_GAP;
                let half_gap = gap / 2.0;

                let t = super::theme::active();
                let divider_border = if t.is_dark {
                    egui::Color32::from_rgb(55, 55, 55)
                } else {
                    egui::Color32::from_rgb(195, 195, 195)
                };

                // Drag handle — covers the full divider height, captures drag + hover
                let drag_area_id = egui::Id::new("split_divider_drag");
                let drag_area_resp = egui::Area::new(drag_area_id)
                    .fixed_pos(egui::pos2(divider_x - half_gap - 2.0, divider_top))
                    .order(egui::Order::Foreground)
                    .interactable(true)
                    .show(ctx, |ui| {
                        let handle_size = egui::vec2(gap + 4.0, divider_bottom - divider_top);
                        let (_, drag_resp) = ui.allocate_exact_size(handle_size, egui::Sense::click_and_drag());
                        if drag_resp.dragged() {
                            let new_x = (divider_x + drag_resp.drag_delta().x)
                                .clamp(vp.left() + 100.0, vp.right() - 100.0);
                            self.state.split_ratio = (new_x - vp.left()) / vp.width();
                        }
                        let is_hovered = drag_resp.hovered() || drag_resp.dragged();
                        if is_hovered {
                            ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        }

                        // Click on divider area toggles sync
                        if drag_resp.clicked() {
                            split_sync_toggled = true;
                        }

                        is_hovered
                    });
                let divider_hovered = drag_area_resp.inner;

                // Draw the divider visuals
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Middle,
                    egui::Id::new("split_divider_line"),
                ));

                if divider_hovered {
                    // Highlighted: accent-colored line
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(divider_x - 1.0, divider_top),
                            egui::pos2(divider_x + 1.0, divider_bottom),
                        ),
                        0.0,
                        t.accent,
                    );
                } else {
                    // Default: thin subtle border line
                    painter.line_segment(
                        [egui::pos2(divider_x, divider_top), egui::pos2(divider_x, divider_bottom)],
                        egui::Stroke::new(1.0, divider_border),
                    );
                }

                // ── Sync indicator: small icon on the divider, visible on hover ──
                if divider_hovered {
                    let is_synced = tab_manager.split_cameras_synced;
                    let grip_center_y = (divider_top + divider_bottom) / 2.0;
                    let icon = if is_synced {
                        LucideIcon::Link.unicode()
                    } else {
                        LucideIcon::Unlink.unicode()
                    };
                    let icon_color = if is_synced { t.accent } else { t.text_secondary };
                    let btn_size = 22.0;
                    let btn_rect = egui::Rect::from_center_size(
                        egui::pos2(divider_x, grip_center_y),
                        egui::vec2(btn_size, btn_size),
                    );
                    let btn_bg = if t.is_dark {
                        egui::Color32::from_rgb(45, 45, 45)
                    } else {
                        egui::Color32::from_rgb(240, 240, 240)
                    };
                    painter.rect_filled(btn_rect, egui::Rounding::same(btn_size / 2.0), btn_bg);
                    painter.rect_stroke(btn_rect, egui::Rounding::same(btn_size / 2.0), egui::Stroke::new(1.0, divider_border));

                    let icon_galley = painter.layout_no_wrap(
                        icon.to_string(),
                        egui::FontId::proportional(10.0),
                        icon_color,
                    );
                    let icon_pos = egui::pos2(
                        btn_rect.center().x - icon_galley.size().x / 2.0,
                        btn_rect.center().y - icon_galley.size().y / 2.0,
                    );
                    painter.galley(icon_pos, icon_galley, egui::Color32::TRANSPARENT);

                    let tooltip = if is_synced { "Click to unlink cameras" } else { "Click to sync cameras" };
                    // Show tooltip near the button
                    egui::containers::popup::show_tooltip_at_pointer(ctx, egui::Id::new("sync_tip"),
                        |ui| { ui.label(tooltip); }
                    );
                }
            }

            // ── Sidebar / canvas vertical separator + drag-to-resize handle ──
            {
                let t = super::theme::active();
                let sep_x = regions.sidebar.total_width;
                let sep_top = regions.toolbar.bottom();
                let sep_bottom = regions.viewport.bottom();

                // Draw the separator line
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Middle,
                    egui::Id::new("sidebar_canvas_separator"),
                ));
                painter.line_segment(
                    [egui::pos2(sep_x, sep_top), egui::pos2(sep_x, sep_bottom)],
                    egui::Stroke::new(1.0, t.toolbar_border),
                );

                // Drag handle — only when sidebar content is visible
                if regions.sidebar.content_visible {
                    let drag_width = 6.0;
                    let drag_rect = egui::Rect::from_min_max(
                        egui::pos2(sep_x - drag_width / 2.0, sep_top),
                        egui::pos2(sep_x + drag_width / 2.0, sep_bottom),
                    );
                    let drag_id = egui::Id::new("sidebar_resize_drag");
                    let drag_area = egui::Area::new(drag_id)
                        .order(egui::Order::Foreground)
                        .fixed_pos(drag_rect.left_top())
                        .interactable(true);
                    drag_area.show(ctx, |ui| {
                        let response = ui.allocate_rect(
                            egui::Rect::from_min_size(
                                drag_rect.left_top(),
                                drag_rect.size(),
                            ),
                            egui::Sense::drag(),
                        );
                        if response.dragged() {
                            let delta = response.drag_delta().x;
                            self.state.sidebar_content_width = (self.state.sidebar_content_width + delta)
                                .clamp(
                                    super::layout::SIDEBAR_MIN_WIDTH,
                                    super::layout::SIDEBAR_MAX_WIDTH,
                                );
                        }
                        if response.hovered() || response.dragged() {
                            ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                            // Highlight the separator on hover/drag
                            let highlight_color = egui::Color32::from_rgba_unmultiplied(
                                t.accent.r(), t.accent.g(), t.accent.b(), 80,
                            );
                            ui.painter().line_segment(
                                [egui::pos2(sep_x, sep_top), egui::pos2(sep_x, sep_bottom)],
                                egui::Stroke::new(2.0, highlight_color),
                            );
                        }
                    });
                }
            }

            // ── Drag-to-split drop zone indicators ──
            if let Some(_drag_id) = self.state.tab_drag_id {
                if let Some(drag_pos) = self.state.tab_drag_pos {
                    let vp = regions.viewport;
                    if drag_pos.y > vp.top() {
                        let t = super::theme::active();
                        let painter = ctx.layer_painter(egui::LayerId::new(
                            egui::Order::Tooltip,
                            egui::Id::new("drag_drop_zones"),
                        ));
                        let midpoint = vp.left() + vp.width() / 2.0;
                        let active_half_color = t.accent.linear_multiply(0.15);
                        let inactive_half_color = egui::Color32::from_rgba_unmultiplied(
                            t.text_secondary.r(), t.text_secondary.g(), t.text_secondary.b(), 20,
                        );

                        // Left half highlight
                        let left_half = egui::Rect::from_min_max(
                            vp.left_top(),
                            egui::pos2(midpoint, vp.bottom()),
                        );
                        let right_half = egui::Rect::from_min_max(
                            egui::pos2(midpoint, vp.top()),
                            vp.right_bottom(),
                        );

                        if drag_pos.x <= midpoint {
                            painter.rect_filled(left_half, 0.0, active_half_color);
                            painter.rect_filled(right_half, 0.0, inactive_half_color);
                        } else {
                            painter.rect_filled(left_half, 0.0, inactive_half_color);
                            painter.rect_filled(right_half, 0.0, active_half_color);
                        }

                        // Divider preview line
                        painter.line_segment(
                            [egui::pos2(midpoint, vp.top()), egui::pos2(midpoint, vp.bottom())],
                            egui::Stroke::new(2.0, t.accent.linear_multiply(0.6)),
                        );

                        // Label — "Drop to split"
                        let label_pos = if drag_pos.x <= midpoint {
                            egui::pos2(left_half.center().x, left_half.center().y)
                        } else {
                            egui::pos2(right_half.center().x, right_half.center().y)
                        };
                        painter.text(
                            label_pos,
                            egui::Align2::CENTER_CENTER,
                            "Drop to split",
                            egui::FontId::proportional(14.0),
                            t.accent,
                        );

                        ctx.request_repaint();
                    }
                }
            }

            // ── Overlays (Order::Middle so panels render on top) ──
            if self.state.tool_state.show_scale_bar {
                let mut left_offset = regions.sidebar_width();
                // Shift scale bar right when gradient scale overlays are visible
                if !regions.gradient_scales.is_empty() {
                    left_offset += super::layout::GRADIENT_SCALE_WIDTH + super::layout::PANEL_MARGIN + super::layout::PANEL_GAP;
                }
                let mut bottom_offset = ctx.screen_rect().bottom() - regions.viewport_bottom();
                // Shift scale bar up when the floating vector player is visible
                if regions.vector_player.is_some() {
                    bottom_offset += super::layout::VECTOR_PLAYER_HEIGHT + super::layout::PANEL_MARGIN;
                }
                components::show_scale_bar(
                    ctx,
                    self.state.tool_state_zoom,
                    self.state.global_units.length,
                    left_offset,
                    bottom_offset,
                );
            }

            let sidebar_w = regions.sidebar_width();
            let content_top = regions.viewport.top();
            components::show_ruler_overlay(
                ctx,
                &self.state.tool_state.ruler_measurements,
                self.state.tool_state.ruler_start,
                self.state.tool_state.ruler_end,
                self.state.tool_state_view_transform.as_ref(),
                self.state.global_units.length,
                sidebar_w,
                content_top,
            );

            // ── Cursor icon — system defaults when over the viewport ──
            if !self.ctx.wants_pointer_input() {
                let primary_down = ctx.input(|i| i.pointer.primary_down());
                let icon = match self.state.tool_state.active_mode {
                    ToolMode::Pan => {
                        if primary_down {
                            egui::CursorIcon::Grabbing
                        } else {
                            egui::CursorIcon::Grab
                        }
                    }
                    ToolMode::ZoomSelect => {
                        if primary_down {
                            egui::CursorIcon::ZoomIn
                        } else {
                            egui::CursorIcon::ZoomIn
                        }
                    }
                    ToolMode::Ruler => egui::CursorIcon::Crosshair,
                };
                ctx.set_cursor_icon(icon);
            }

            if let (Some(start), Some(end)) = (
                self.state.tool_state.zoom_rect_start,
                self.state.tool_state.zoom_rect_end,
            ) {
                components::show_zoom_rect(ctx, start, end);
            }

            if self.state.tool_state.show_grid {
                if let Some(ref vt) = self.state.tool_state_view_transform {
                    components::show_grid_labels(ctx, vt, self.state.global_units.length, sidebar_w, content_top);
                }
            }

            // ── Hover tooltip ──
            if let Some(ref hover) = self.state.hover_info {
                components::show_hover_tooltip(ctx, hover, &self.state.global_units);
            }

            // ── File info popup (rendered last for high z-order) ──
            if self.state.show_file_info {
                components::show_file_info(ctx, &mut self.state.show_file_info, &self.state.vector_counts);
            }

            // ── Controls popup (rendered last for high z-order) ──
            if self.state.show_controls {
                components::show_controls_popup(ctx, &mut self.state.show_controls);
            }

            // ── Preferences dialog ──
            if self.state.show_preferences {
                let prefs_out = components::preferences::show_preferences(
                    ctx,
                    &mut self.state.show_preferences,
                    &mut self.state.theme_mode,
                    &mut self.state.active_palette_id,
                    &mut self.state.light_palette_id,
                    &mut self.state.dark_palette_id,
                    self.state.system_is_dark,
                    &self.state.active_palette,
                    &mut self.state.canvas_settings,
                );
                if prefs_out.changed {
                    if let Some(new_pal) = prefs_out.palette {
                        self.state.active_palette = new_pal;
                        self.state.theme_changed = true;
                    }
                }
                if prefs_out.canvas_changed {
                    self.state.canvas_changed = true;
                }
            }

            // ── Empty state message (no file loaded) ──
            if self.state.total_layers == 0 && self.state.loading_file.is_none() {
                let t = theme::active();
                let center = regions.viewport.center();
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Background,
                    egui::Id::new("empty_state"),
                ));
                // Cover the GL canvas with the panel background so it is fully hidden
                painter.rect_filled(regions.viewport, 0.0, t.panel_bg);
                painter.text(
                    center + egui::vec2(0.0, -10.0),
                    egui::Align2::CENTER_CENTER,
                    "No file loaded",
                    egui::FontId::proportional(20.0),
                    t.text_secondary,
                );
                painter.text(
                    center + egui::vec2(0.0, 16.0),
                    egui::Align2::CENTER_CENTER,
                    "Drop an ILT file or press Ctrl+O to open",
                    egui::FontId::proportional(13.0),
                    t.text_secondary,
                );
                painter.text(
                    center + egui::vec2(0.0, 46.0),
                    egui::Align2::CENTER_CENTER,
                    crate::APP_DEVELOPER,
                    egui::FontId::proportional(11.0),
                    t.text_secondary,
                );
            }


            // ── Update notification ──
            {
                let current_state = self.state.update_state.lock()
                    .map(|s| s.clone())
                    .unwrap_or(crate::infrastructure::updater::updater::UpdateState::Idle);
                let notif_out = components::update_notification::show_update_notification(
                    ctx,
                    &current_state,
                    &mut self.state.update_dismissed,
                );
                if notif_out.download_requested {
                    if let crate::infrastructure::updater::updater::UpdateState::Available(ref info) = current_state {
                        crate::infrastructure::updater::updater::download_and_install(
                            info,
                            self.state.update_state.clone(),
                        );
                    }
                }
                if notif_out.install_requested {
                    if let crate::infrastructure::updater::updater::UpdateState::ReadyToInstall(ref path) = current_state {
                        if let Err(e) = crate::infrastructure::updater::updater::launch_installer(
                            path,
                            self.state.update_state.clone(),
                        ) {
                            log::error!("Failed to launch installer: {}", e);
                        }
                    }
                }
                if notif_out.view_release {
                    if let crate::infrastructure::updater::updater::UpdateState::Available(ref info) = current_state {
                        let _ = open::that(&info.release_url);
                    }
                }
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

        // ── Handle tab bar actions ──
        if let Some(new_mode) = view_mode_changed {
            self.state.view_mode = new_mode;
            // Clear stale hover tooltip from previous view mode immediately
            self.state.hover_info = None;
        }

        // ── Sync UiState changes back → active TabState ──
        if let Some(tab) = tab_manager.active_tab_mut() {
            tab.show_slices = self.state.show_slices;
            tab.show_contours = self.state.show_contours;
            tab.show_hatches = self.state.show_hatches;
            tab.show_arrows = self.state.show_arrows;
            tab.show_wait_markers = self.state.show_wait_markers;
            tab.vector_view_enabled = self.state.vector_view_enabled;
            tab.current_vector_index = self.state.current_vector_index;
            tab.total_vectors_in_layer = self.state.total_vectors_in_layer;
            tab.vector_view_playing = self.state.vector_view_playing;
            tab.playback_speed = self.state.playback_speed;
            if layer_changed {
                tab.navigation.go_to_layer(new_layer);
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

        let output = UiOutput {
            layer_index: self.state.current_layer,
            show_slices: self.state.show_slices,
            show_contours: self.state.show_contours,
            show_hatches: self.state.show_hatches,
            show_arrows: self.state.show_arrows,
            show_wait_markers: self.state.show_wait_markers,
            param_mode: self.state.param_mode,
            param_filter_min: self.state.param_filter_min,
            param_filter_max: self.state.param_filter_max,
            wait_filter_min: self.state.wait_filter_min,
            wait_filter_max: self.state.wait_filter_max,
            needs_repaint: needs_repaint
                || zoom_in_requested
                || zoom_out_requested
                || fit_view_requested,
            open_file_requested,
            add_files_requested,
            remove_file,
            toggle_file_visibility,
            color_mode: self.state.color_mode,
            tool_state: tool_output,
            global_units: self.state.global_units.clone(),
            zoom_in_requested,
            zoom_out_requested,
            fit_view_requested,
            vector_view_enabled: self.state.vector_view_enabled,
            theme_changed: self.state.theme_changed,
            canvas_changed: self.state.canvas_changed,
            gradient_palette_changed: self.state.gradient_palette_changed,
            active_palette: self.state.active_palette.clone(),
            view_mode: self.state.view_mode,
            active_tab_file: self.state.active_tab_file,
            split_ratio: self.state.split_ratio,
            switch_tab,
            close_tab: close_tab_action,
            split_partner_changed,
            drag_to_split,
            overlay_toggle,
            overlay_select_all,
            overlay_select_only,
            close_others,
            close_all,
            show_in_split,
            split_right_switch,
            reorder_tab,
            split_sync_toggled,
            viewport_rect: regions.viewport,
            sidebar_total_width: regions.sidebar.total_width,
            bottom_bar_height: regions.status_bar.height,
            header_height: TOOLBAR_BOTTOM
                + if flags.has_tab_bar { TAB_BAR_HEIGHT } else { 0.0 },
        };

        // Clear the one-shot flags
        self.state.theme_changed = false;
        self.state.canvas_changed = false;

        output
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
}
