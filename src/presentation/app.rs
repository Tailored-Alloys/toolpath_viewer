//! Main Application
//!
//! Orchestrates all components and manages application state.

use anyhow::Result;
use log::{info, warn, error};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;

use crate::application::dto::FileCollection;
use crate::application::ports::{ColorMode, ConfigManager, DisplayOptions, FileLoader, ParameterMode, Renderer, ViewMode};
use crate::application::use_cases::{
    LoadToolpathUseCase, NavigateLayersUseCase, RenderViewUseCase, DisplayOption,
    ColorScheme,
};
use crate::domain::entities::{Toolpath, VectorType};
use crate::domain::services::find_nearest_vector;
use crate::domain::value_objects::{Bounds2D, Color, Point2D};
use crate::infrastructure::config::JsonConfigManager;
use crate::infrastructure::file_adapters::IltLoader;
use crate::infrastructure::rendering::GlRenderer;
use crate::presentation::{
    AppEvent, AppWindow, InputAction, MouseButton, ParamRanges, WindowConfig,
    create_window, run_event_loop, UiRenderer, ToolMode,
    RulerMeasurement, SnapshotFormat, TabManager, SplitPane,
};
use crate::presentation::palette::{self, ResolvedMode, ThemePalette, resolve_mode, resolve_palette};

/// State of background file loading
enum LoadingState {
    /// No loading in progress
    Idle,
    /// Loading in progress — carries the display name(s)
    Loading(String),
    /// Loading finished successfully — one or more files loaded
    Complete(Vec<(PathBuf, Toolpath, ParamRanges)>),
    /// Loading finished with an error
    Error(String),
}

/// Application state
pub struct AppState {
    /// Collection of loaded files
    pub files: FileCollection,
    /// Layer navigation (global fallback for overlay mode)
    pub navigation: NavigateLayersUseCase,
    /// Render state
    pub render: RenderViewUseCase,
    /// Tab manager — per-tab camera, navigation, playback, toggles
    pub tab_manager: TabManager,
    /// Whether a redraw is needed
    pub needs_redraw: bool,
    /// Shared loading state for background file loading
    pub loading_state: Arc<Mutex<LoadingState>>,
    /// Last frame timestamp for delta-time computation
    pub last_frame_time: Option<Instant>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            files: FileCollection::new(),
            navigation: NavigateLayersUseCase::new(),
            render: RenderViewUseCase::new(),
            tab_manager: TabManager::new(),
            needs_redraw: true,
            loading_state: Arc::new(Mutex::new(LoadingState::Idle)),
            last_frame_time: None,
        }
    }
}

/// Combined application context holding all mutable state
pub struct AppContext {
    pub state: AppState,
    pub renderer: GlRenderer,
    pub ui: UiRenderer,
    pub load_use_case: LoadToolpathUseCase,
}

/// Main application
pub struct App {
    config: WindowConfig,
    theme_config: crate::application::ports::ThemeConfig,
    canvas_config: crate::application::ports::CanvasConfig,
}

impl App {
    /// Create a new application
    pub fn new() -> Result<Self> {
        // Load configuration
        let config_manager = JsonConfigManager::new();
        let app_config = config_manager.load().unwrap_or_default();

        let config = WindowConfig {
            title: app_config.window.title,
            width: app_config.window.width,
            height: app_config.window.height,
            ..Default::default()
        };

        Ok(Self { config, theme_config: app_config.theme, canvas_config: app_config.canvas })
    }

    /// Run the application
    pub fn run(self) -> Result<()> {
        // Create event loop
        let event_loop = EventLoop::new()?;

        // Create window
        let app_window = create_window(self.config, &event_loop)?;

        // Create application state
        let mut state = AppState::default();

        // Create renderer
        let mut renderer = GlRenderer::new();
        let (width, height) = app_window.size();
        renderer.initialize(width, height)?;
        // Set DPI scale factor for correct logical-pixel projection
        renderer.set_scale_factor(app_window.scale_factor);
        // GL projection offsets are updated dynamically in render_frame
        // to account for sidebar and bottom panel visibility

        // Create UI renderer
        let mut ui = UiRenderer::new(app_window.glow_context.clone(), &app_window.window);

        // Apply saved theme
        {
            let tc = &self.theme_config;
            let sys_dark = app_window.window.theme()
                .map(|t| t == winit::window::Theme::Dark)
                .unwrap_or(false);
            let resolved = resolve_mode(tc.mode, sys_dark);
            let palette_id = match resolved {
                ResolvedMode::Light => tc.light_palette,
                ResolvedMode::Dark => tc.dark_palette,
            };
            let custom = match resolved {
                ResolvedMode::Light => tc.custom_light.as_ref(),
                ResolvedMode::Dark => tc.custom_dark.as_ref(),
            };
            let palette = resolve_palette(resolved, palette_id, custom);
            crate::presentation::theme::apply_theme(&ui.ctx, &palette);

            // Sync into UI state
            ui.state.theme_mode = tc.mode;
            ui.state.active_palette_id = palette_id;
            ui.state.light_palette_id = tc.light_palette;
            ui.state.dark_palette_id = tc.dark_palette;
            ui.state.system_is_dark = sys_dark;

            // Sync into renderer
            renderer.set_color_scheme(ColorScheme::from_palette(&palette));
            renderer.set_gradient_stops(palette.gradient_stops.clone());
            state.render.display_options.background_color = palette.background;
            state.render.display_options.grid_minor_color = palette.grid_minor;
            state.render.display_options.grid_major_color = palette.grid_major;

            ui.state.active_palette = palette;
        }

        // Apply saved canvas settings
        {
            let cc = &self.canvas_config;
            ui.state.canvas_settings = cc.clone();
            state.render.display_options.line_width = cc.line_width;
            state.render.display_options.boundary_width_multiplier = cc.boundary_width_multiplier;
            state.render.display_options.contour_width_multiplier = cc.contour_width_multiplier;
            state.render.display_options.hatch_width_multiplier = cc.hatch_width_multiplier;
            state.render.display_options.arrow_size_multiplier = cc.arrow_size_multiplier;
            state.render.display_options.wait_marker_size_multiplier = cc.wait_marker_size_multiplier;
            state.render.display_options.future_vector_alpha = cc.future_vector_alpha;
            state.render.display_options.show_direction_gradient = cc.show_direction_gradient;
            state.render.display_options.grid_line_width_minor = cc.grid_line_width_minor;
            state.render.display_options.grid_line_width_major = cc.grid_line_width_major;
            state.render.display_options.grid_opacity = cc.grid_opacity;
            state.render.display_options.antialiasing = cc.antialiasing;
        }

        // Create file loader
        let loaders: Vec<Arc<dyn FileLoader>> = vec![Arc::new(IltLoader::new())];
        let load_use_case = Arc::new(LoadToolpathUseCase::new(loaders));

        // Check for command line arguments
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            let file_path = &args[1];
            // Startup load is synchronous (no window to show yet)
            let (logical_w, logical_h) = app_window.logical_size();
            match load_file_sync(&load_use_case, file_path, &mut state, logical_w, logical_h) {
                Err(e) => error!("Failed to load file: {}", e),
                Ok(ranges) => {
                    ui.state.param_ranges = ranges.clone();
                    if let Some((wmin, wmax)) = ranges.wait_time {
                        state.render.display_options.wait_time_min = wmin;
                        state.render.display_options.wait_time_max = wmax;
                        ui.state.wait_filter_min = wmin;
                        ui.state.wait_filter_max = wmax;
                    }
                    let nav_state = state.navigation.state();
                    ui.update_from_navigation(
                        nav_state.current_index,
                        nav_state.total_layers,
                        nav_state.current_z,
                    );
                }
            }
        }

        info!("Application started");
        state.needs_redraw = true;

        // Run event loop with combined handler
        let load_use_case_clone = load_use_case.clone();
        run_event_loop(
            event_loop,
            app_window,
            move |window, event, raw_event| {
                handle_event(
                    window,
                    event,
                    raw_event,
                    &mut state,
                    &mut renderer,
                    &mut ui,
                    &load_use_case_clone,
                )
            },
        )
    }
}

/// Compute min/max parameter ranges across all vectors in the file
fn compute_param_ranges(toolpath: &Toolpath) -> ParamRanges {
    let mut pmin = f32::MAX;
    let mut pmax = f32::MIN;
    let mut smin = f32::MAX;
    let mut smax = f32::MIN;
    let mut wmin = f32::MAX;
    let mut wmax = f32::MIN;
    let (mut hp, mut hs, mut hw) = (false, false, false);

    for layer in &toolpath.slice_stack.layers {
        for v in &layer.vectors {
            if let Some(val) = v.parameters.power {
                pmin = pmin.min(val);
                pmax = pmax.max(val);
                hp = true;
            }
            if let Some(val) = v.parameters.speed {
                smin = smin.min(val);
                smax = smax.max(val);
                hs = true;
            }
            if let Some(val) = v.parameters.wait_time {
                wmin = wmin.min(val);
                wmax = wmax.max(val);
                hw = true;
            }
        }
    }

    ParamRanges {
        power: if hp { Some((pmin, pmax)) } else { None },
        speed: if hs { Some((smin, smax)) } else { None },
        wait_time: if hw { Some((wmin, wmax)) } else { None },
    }
}

/// Load a file synchronously into the application state (used for startup)
fn load_file_sync(
    use_case: &LoadToolpathUseCase,
    path: &str,
    state: &mut AppState,
    viewport_width: f32,
    viewport_height: f32,
) -> Result<ParamRanges> {
    info!("Loading file (sync): {}", path);

    let toolpath = use_case.execute(Path::new(path))?;
    let ranges = compute_param_ranges(&toolpath);
    let path_buf = PathBuf::from(path);
    finalize_load(vec![(path_buf, toolpath, ranges.clone())], state, viewport_width, viewport_height)
}

/// Spawn a background thread to load one or more files. The result will be picked up
/// by `poll_loading_state` on the next frame.
fn start_background_load(
    use_case: &Arc<LoadToolpathUseCase>,
    paths: Vec<PathBuf>,
    loading_state: &Arc<Mutex<LoadingState>>,
) {
    if paths.is_empty() {
        return;
    }

    // Check if a load is already in progress
    {
        let state = loading_state.lock().unwrap();
        if matches!(*state, LoadingState::Loading(_)) {
            warn!("Load already in progress, ignoring request");
            return;
        }
    }

    let display_name = if paths.len() == 1 {
        paths[0].file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| paths[0].display().to_string())
    } else {
        format!("{} files", paths.len())
    };

    // Set state to Loading
    {
        let mut state = loading_state.lock().unwrap();
        *state = LoadingState::Loading(display_name.clone());
    }

    let use_case = use_case.clone();
    let loading_state = loading_state.clone();

    std::thread::spawn(move || {
        info!("Background load started: {} file(s)", paths.len());
        let mut results = Vec::new();
        let mut errors = Vec::new();

        for path in paths {
            match use_case.execute(&path) {
                Ok(toolpath) => {
                    let ranges = compute_param_ranges(&toolpath);
                    let stats = toolpath.slice_stack.stats();
                    info!(
                        "Loaded {}: {} layers, {} vectors",
                        path.display(), stats.layer_count, stats.total_vectors
                    );
                    results.push((path, toolpath, ranges));
                }
                Err(e) => {
                    error!("Failed to load {}: {}", path.display(), e);
                    errors.push(format!("{}: {}", path.display(), e));
                }
            }
        }

        let mut state = loading_state.lock().unwrap();
        if results.is_empty() {
            *state = LoadingState::Error(errors.join("\n"));
        } else {
            if !errors.is_empty() {
                warn!("Some files failed to load: {}", errors.join("; "));
            }
            *state = LoadingState::Complete(results);
        }
    });
}

/// Finalize loaded files into the application state (appends to existing collection)
fn finalize_load(
    loaded_files: Vec<(PathBuf, Toolpath, ParamRanges)>,
    state: &mut AppState,
    vp_w: f32,
    vp_h: f32,
) -> Result<ParamRanges> {
    let is_first_load = state.files.is_empty();

    for (path, toolpath, _ranges) in loaded_files {
        let stats = toolpath.slice_stack.stats();
        info!(
            "Adding file {}: {} layers, {} vectors, {} points",
            path.display(), stats.layer_count, stats.total_vectors, stats.total_points
        );
        let id = state.files.add_file(path, toolpath);
        // Create a tab for the newly loaded file
        // We need to get the SliceStack from the file we just added
        if let Some(file) = state.files.files.iter().find(|f| f.id == id) {
            state.tab_manager.open_tab(id, &file.toolpath.slice_stack);
            // Fit the new tab's view to the file's bounds
            if let Some((min, max)) = file.toolpath.slice_stack.bounds() {
                let bounds = Bounds2D::new(min, max);
                if let Some(tab) = state.tab_manager.tab_mut(id) {
                    tab.view_state.fit_to_bounds(&bounds, vp_w, vp_h.max(100.0));
                }
            }
        }
    }

    // Re-initialize navigation from merged Z-heights
    let z_heights = state.files.merged_z_heights();
    state.navigation.initialize_from_z_heights(z_heights);

    // Fit view if first load (use effective viewport area)
    if is_first_load {
        if let Some((min, max)) = state.files.merged_bounds() {
            let bounds = Bounds2D::new(min, max);
            state.render.view_state.fit_to_bounds(&bounds, vp_w, vp_h.max(100.0));
        }
    }

    let ranges = state.files.merged_param_ranges();
    state.needs_redraw = true;

    Ok(ranges)
}

/// Check if background loading is done and finalize if so.
/// Returns true if the UI should keep requesting redraws (loading in progress or just completed).
fn poll_loading_state(
    state: &mut AppState,
    ui: &mut UiRenderer,
    window: &AppWindow,
) -> bool {
    let loading_state_arc = state.loading_state.clone();
    let mut ls = loading_state_arc.lock().unwrap();

    match std::mem::replace(&mut *ls, LoadingState::Idle) {
        LoadingState::Loading(name) => {
            // Still loading — put it back and tell UI
            *ls = LoadingState::Loading(name.clone());
            ui.state.loading_file = Some(name);
            true // keep redrawing for spinner
        }
        LoadingState::Complete(loaded_files) => {
            // Loading done
            let vp = ui.state.cached_viewport_rect;
            match finalize_load(loaded_files, state, vp.width(), vp.height()) {
                Ok(ranges) => {
                    ui.state.param_ranges = ranges.clone();
                    if let Some((wmin, wmax)) = ranges.wait_time {
                        state.render.display_options.wait_time_min = wmin;
                        state.render.display_options.wait_time_max = wmax;
                        ui.state.wait_filter_min = wmin;
                        ui.state.wait_filter_max = wmax;
                    }
                    ui.state.has_multiple_files = state.tab_manager.tab_count() > 1;
                    // Sync active tab from TabManager into UI state
                    ui.state.active_tab_file = state.tab_manager.active_tab_id;
                    // Auto-set split partner if in split mode and none set
                    if state.tab_manager.split_partner_id.is_none() && state.tab_manager.tab_count() >= 2 {
                        state.tab_manager.split_partner_id = state.tab_manager.open_tab_ids
                            .iter()
                            .find(|&&id| Some(id) != state.tab_manager.active_tab_id)
                            .copied();
                    }
                    update_window_title(window, state);
                    let nav_state = state.navigation.state();
                    ui.update_from_navigation(
                        nav_state.current_index,
                        nav_state.total_layers,
                        nav_state.current_z,
                    );
                    info!("File(s) loaded successfully");
                }
                Err(e) => {
                    error!("Failed to finalize loaded file: {}", e);
                }
            }
            *ls = LoadingState::Idle;
            false
        }
        LoadingState::Error(msg) => {
            ui.state.loading_file = None;
            error!("File load error: {}", msg);
            *ls = LoadingState::Idle;
            false
        }
        LoadingState::Idle => {
            ui.state.loading_file = None;
            false
        }
    }
}

/// Handle an application event
fn handle_event(
    window: &mut AppWindow,
    event: AppEvent,
    raw_event: Option<&WindowEvent>,
    state: &mut AppState,
    renderer: &mut GlRenderer,
    ui: &mut UiRenderer,
    load_use_case: &Arc<LoadToolpathUseCase>,
) -> bool {
    // Always forward raw events to egui
    if let Some(raw) = raw_event {
        let response = ui.handle_event(&window.window, raw);
        // Request redraw only if egui reports that this event changed UI state.
        if !matches!(event, AppEvent::Redraw) && response.repaint {
            window.request_redraw();
        }
    }

    // Use egui's pointer ownership to determine if mouse is over viewport
    let pointer_in_viewport = !ui.wants_pointer();

    match event {
        AppEvent::CloseRequested => {
            info!("Close requested");
            return true;
        }

        AppEvent::Resize { width, height } => {
            if let Err(e) = renderer.resize(width, height) {
                error!("Resize error: {}", e);
            }
            // Update scale factor (may change on DPI/monitor change)
            window.scale_factor = window.window.scale_factor() as f32;
            renderer.set_scale_factor(window.scale_factor);
            state.needs_redraw = true;
            window.request_redraw();
        }

        AppEvent::Redraw => {
            if let Err(e) = render_frame(window, state, renderer, ui, load_use_case) {
                error!("Render error: {}", e);
            }
        }

        AppEvent::MouseMove { x: _, y: _ } => {
            // Handle panning only when pointer is in viewport
            if pointer_in_viewport {
                let mouse = &window.input_state.mouse_pos;
                let vp = ui.state.cached_viewport_rect;

                // ── Split-pane aware viewport + view state selection ──
                let is_split = ui.state.view_mode == ViewMode::Split;
                let pane_hit = if is_split {
                    detect_split_pane(mouse.x, vp, ui.state.split_ratio)
                } else {
                    None
                };

                // Effective viewport rect and view state for the pane under the cursor
                let (eff_vp, use_right) = if let Some(ref hit) = pane_hit {
                    ui.state.active_split_pane = hit.pane;
                    (hit.pane_rect, hit.pane == SplitPane::Right)
                } else if is_split {
                    // Cursor is in the divider gap — skip interaction
                    ui.state.hover_info = None;
                    ui.state.mouse_world_pos = None;
                    return false;
                } else {
                    (vp, false)
                };
                let eff_w = eff_vp.width();
                let eff_h = eff_vp.height();

                // Pick the correct view state (right pane uses independent state when unsynced)
                let use_right_independent = use_right && !state.tab_manager.split_cameras_synced;
                let view_state_for_read = if use_right_independent {
                    state.tab_manager.split_right_view_state.clone()
                        .unwrap_or_else(|| state.render.view_state.clone())
                } else {
                    state.render.view_state.clone()
                };

                // Track world position for status bar
                let viewport_mouse_x = mouse.x - eff_vp.left();
                let viewport_mouse_y = mouse.y - eff_vp.top();
                let world_pos = view_state_for_read.screen_to_world(
                    viewport_mouse_x, viewport_mouse_y, eff_w, eff_h,
                );
                ui.state.mouse_world_pos = Some((world_pos.x, world_pos.y));

                // Left-drag behavior depends on active tool mode
                if window.input_state.left_pressed {
                    match ui.state.tool_state.active_mode {
                        ToolMode::Pan => {
                            let delta = window.input_state.mouse_delta();
                            if use_right_independent {
                                if let Some(ref mut rv) = state.tab_manager.split_right_view_state {
                                    rv.pan(delta.x, -delta.y);
                                }
                            } else {
                                state.render.pan(delta.x, -delta.y);
                            }
                            state.needs_redraw = true;
                            ui.state.hover_info = None;
                        }
                        ToolMode::ZoomSelect => {
                            ui.state.tool_state.zoom_rect_end = Some(Point2D::new(mouse.x, mouse.y));
                        }
                        ToolMode::Ruler => {
                            // Ruler drag: update live preview
                            if ui.state.tool_state.ruler_start.is_some() {
                                ui.state.tool_state.ruler_end = Some(world_pos);
                            }
                        }
                    }
                    window.request_redraw();
                }
                // Ruler live preview (move cursor while placing second point)
                else if ui.state.tool_state.ruler_start.is_some() {
                    ui.state.tool_state.ruler_end = Some(world_pos);
                    window.request_redraw();
                }
                // Middle-drag: always pan (regardless of tool mode)
                else if window.input_state.middle_pressed {
                    let delta = window.input_state.mouse_delta();
                    if use_right_independent {
                        if let Some(ref mut rv) = state.tab_manager.split_right_view_state {
                            rv.pan(delta.x, -delta.y);
                        }
                    } else {
                        state.render.pan(delta.x, -delta.y);
                    }
                    state.needs_redraw = true;
                    ui.state.hover_info = None;
                    window.request_redraw();
                }
                // Hover tooltip hit-test (no buttons pressed)
                else {
                    update_hover_info(state, ui, window);
                    window.request_redraw();
                }
            } else {
                ui.state.hover_info = None;
                ui.state.mouse_world_pos = None;
            }
        }

        AppEvent::EguiEvent => {
            // egui forwarding already happened above; nothing else to do
        }

        AppEvent::MouseButton { button, pressed } => {
            if pointer_in_viewport {
                let mouse = &window.input_state.mouse_pos;
                let vp = ui.state.cached_viewport_rect;

                // ── Split-pane aware viewport + view state selection ──
                let is_split = ui.state.view_mode == ViewMode::Split;
                let pane_hit = if is_split {
                    detect_split_pane(mouse.x, vp, ui.state.split_ratio)
                } else {
                    None
                };
                let (eff_vp, use_right) = if let Some(ref hit) = pane_hit {
                    ui.state.active_split_pane = hit.pane;
                    (hit.pane_rect, hit.pane == SplitPane::Right)
                } else if is_split {
                    return false; // cursor in divider gap
                } else {
                    (vp, false)
                };
                let eff_w = eff_vp.width();
                let eff_h = eff_vp.height();
                let use_right_independent = use_right && !state.tab_manager.split_cameras_synced;

                // Reference clone of the view state for coordinate conversion
                let view_ref = if use_right_independent {
                    state.tab_manager.split_right_view_state.clone()
                        .unwrap_or_else(|| state.render.view_state.clone())
                } else {
                    state.render.view_state.clone()
                };

                match button {
                    // Left button: behavior depends on active tool mode
                    MouseButton::Left if pressed => {
                        match ui.state.tool_state.active_mode {
                            ToolMode::Pan => {
                                // Pan starts on drag (handled in MouseMove)
                            }
                            ToolMode::ZoomSelect => {
                                ui.state.tool_state.zoom_rect_start = Some(Point2D::new(mouse.x, mouse.y));
                                ui.state.tool_state.zoom_rect_end = Some(Point2D::new(mouse.x, mouse.y));
                            }
                            ToolMode::Ruler => {
                                // Ruler click: place or complete measurement
                                let viewport_mouse_x = mouse.x - eff_vp.left();
                                let viewport_mouse_y = mouse.y - eff_vp.top();
                                let world = view_ref.screen_to_world(
                                    viewport_mouse_x, viewport_mouse_y, eff_w, eff_h,
                                );
                                place_ruler_point(ui, &view_ref, world);
                            }
                        }
                        window.request_redraw();
                    }
                    MouseButton::Left if !pressed => {
                        match ui.state.tool_state.active_mode {
                            ToolMode::ZoomSelect => {
                                // Zoom selection release → zoom to rectangle
                                if let (Some(start), Some(end)) = (
                                    ui.state.tool_state.zoom_rect_start.take(),
                                    ui.state.tool_state.zoom_rect_end.take(),
                                ) {
                                    let dx = (end.x - start.x).abs();
                                    let dy = (end.y - start.y).abs();
                                    if dx > 5.0 && dy > 5.0 {
                                        let w1 = view_ref.screen_to_world(
                                            start.x - eff_vp.left(), start.y - eff_vp.top(), eff_w, eff_h,
                                        );
                                        let w2 = view_ref.screen_to_world(
                                            end.x - eff_vp.left(), end.y - eff_vp.top(), eff_w, eff_h,
                                        );

                                        let bounds = Bounds2D::new(
                                            Point2D::new(w1.x.min(w2.x), w1.y.min(w2.y)),
                                            Point2D::new(w1.x.max(w2.x), w1.y.max(w2.y)),
                                        );
                                        if use_right_independent {
                                            if let Some(ref mut rv) = state.tab_manager.split_right_view_state {
                                                rv.fit_to_bounds(&bounds, eff_w, eff_h);
                                            }
                                        } else {
                                            state.render.view_state.fit_to_bounds(&bounds, eff_w, eff_h);
                                        }
                                        state.needs_redraw = true;
                                    }
                                    window.request_redraw();
                                }
                            }
                            _ => {}
                        }
                    }
                    // Right button click: always ruler (add/remove measurement point)
                    MouseButton::Right if pressed => {
                        let viewport_mouse_x = mouse.x - eff_vp.left();
                        let viewport_mouse_y = mouse.y - eff_vp.top();
                        let world = view_ref.screen_to_world(
                            viewport_mouse_x, viewport_mouse_y, eff_w, eff_h,
                        );

                        place_ruler_point(ui, &view_ref, world);
                        window.request_redraw();
                    }
                    _ => {}
                }
            }
        }

        AppEvent::Scroll { delta } => {
            // Handle zoom only when pointer is in viewport
            if pointer_in_viewport {
                let vp = ui.state.cached_viewport_rect;
                let zoom_factor = if delta > 0.0 { 1.1 } else { 0.9 };
                let mouse = &window.input_state.mouse_pos;

                // ── Split-pane aware viewport + view state selection ──
                let is_split = ui.state.view_mode == ViewMode::Split;
                let pane_hit = if is_split {
                    detect_split_pane(mouse.x, vp, ui.state.split_ratio)
                } else {
                    None
                };
                let (eff_vp, use_right) = if let Some(ref hit) = pane_hit {
                    (hit.pane_rect, hit.pane == SplitPane::Right)
                } else if is_split {
                    return false; // cursor in divider gap
                } else {
                    (vp, false)
                };
                let eff_w = eff_vp.width();
                let eff_h = eff_vp.height();
                let use_right_independent = use_right && !state.tab_manager.split_cameras_synced;

                let viewport_mouse_x = mouse.x - eff_vp.left();
                let viewport_mouse_y = mouse.y - eff_vp.top();

                if use_right_independent {
                    if let Some(ref mut rv) = state.tab_manager.split_right_view_state {
                        rv.zoom_at(
                            zoom_factor,
                            viewport_mouse_x,
                            eff_h - viewport_mouse_y, // Flip Y
                            eff_w,
                            eff_h,
                        );
                    }
                } else {
                    state.render.zoom(
                        zoom_factor,
                        viewport_mouse_x,
                        eff_h - viewport_mouse_y, // Flip Y
                        eff_w,
                        eff_h,
                    );
                }
                state.needs_redraw = true;
                window.request_redraw();
            }
        }

        AppEvent::KeyAction(action) => {
            // Skip shortcuts when egui has keyboard focus (e.g. text fields)
            if !ui.wants_keyboard() {
                handle_key_action(action, state, window, ui, load_use_case);
            }
        }

        AppEvent::FileDropped(path) => {
            let path_buf = PathBuf::from(&path);
            start_background_load(load_use_case, vec![path_buf], &state.loading_state);
            window.request_redraw();
        }

        AppEvent::SystemThemeChanged { is_dark } => {
            ui.state.system_is_dark = is_dark;
            if ui.state.theme_mode == crate::application::ports::ThemeMode::System {
                let resolved = resolve_mode(ui.state.theme_mode, is_dark);
                let palette_id = match resolved {
                    ResolvedMode::Light => ui.state.light_palette_id,
                    ResolvedMode::Dark => ui.state.dark_palette_id,
                };
                ui.state.active_palette_id = palette_id;
                let palette = resolve_palette(resolved, palette_id, None);
                crate::presentation::theme::apply_theme(&ui.ctx, &palette);
                renderer.set_color_scheme(ColorScheme::from_palette(&palette));
                renderer.set_gradient_stops(palette.gradient_stops.clone());
                state.render.display_options.background_color = palette.background;
                state.render.display_options.grid_minor_color = palette.grid_minor;
                state.render.display_options.grid_major_color = palette.grid_major;
                ui.state.active_palette = palette;
                state.needs_redraw = true;
            }
            window.request_redraw();
        }
    }

    // ── Sync global view_state back to active tab after every event ──
    // Pan/zoom modify state.render.view_state; persist those changes into the tab.
    if let Some(tab) = state.tab_manager.active_tab_mut() {
        tab.view_state = state.render.view_state.clone();
    }

    false
}

/// Handle a key action
fn handle_key_action(
    action: InputAction,
    state: &mut AppState,
    window: &mut AppWindow,
    ui: &mut UiRenderer,
    load_use_case: &Arc<LoadToolpathUseCase>,
) {
    match action {
        InputAction::NextLayer => {
            if state.navigation.next_layer() {
                state.needs_redraw = true;
                window.request_redraw();
                update_window_title(window, state);
            }
        }

        InputAction::PrevLayer => {
            if state.navigation.previous_layer() {
                state.needs_redraw = true;
                window.request_redraw();
                update_window_title(window, state);
            }
        }

        InputAction::JumpForward => {
            if state.navigation.jump_forward(10) {
                state.needs_redraw = true;
                window.request_redraw();
                update_window_title(window, state);
            }
        }

        InputAction::JumpBackward => {
            if state.navigation.jump_backward(10) {
                state.needs_redraw = true;
                window.request_redraw();
                update_window_title(window, state);
            }
        }

        InputAction::FirstLayer => {
            if state.navigation.first_layer() {
                state.needs_redraw = true;
                window.request_redraw();
                update_window_title(window, state);
            }
        }

        InputAction::LastLayer => {
            if state.navigation.last_layer() {
                state.needs_redraw = true;
                window.request_redraw();
                update_window_title(window, state);
            }
        }

        InputAction::ResetView => {
            if let Some((min, max)) = state.files.merged_bounds() {
                let vp = ui.state.cached_viewport_rect;
                let bounds = Bounds2D::new(min, max);
                state.render.view_state.fit_to_bounds(&bounds, vp.width(), vp.height());
                state.needs_redraw = true;
                window.request_redraw();
            }
        }

        InputAction::ToggleBoundaries => {
            state.render.toggle_option(DisplayOption::Slices);
            state.needs_redraw = true;
            window.request_redraw();
        }

        InputAction::ToggleContours => {
            state.render.toggle_option(DisplayOption::Contours);
            state.needs_redraw = true;
            window.request_redraw();
        }

        InputAction::ToggleHatches => {
            state.render.toggle_option(DisplayOption::Hatches);
            state.needs_redraw = true;
            window.request_redraw();
        }

        InputAction::ToggleArrows => {
            state.render.toggle_option(DisplayOption::Arrows);
            state.needs_redraw = true;
            window.request_redraw();
        }

        InputAction::OpenFile => {
            open_file_dialog(window, state, ui, load_use_case);
        }

        InputAction::Snapshot => {
            // Signal snapshot request via UI state — will be handled in render_frame
            ui.state.tool_state.snapshot_requested = true;
            window.request_redraw();
        }

        InputAction::ToggleGrid => {
            ui.state.tool_state.show_grid = !ui.state.tool_state.show_grid;
            state.render.display_options.show_grid = ui.state.tool_state.show_grid;
            state.needs_redraw = true;
            window.request_redraw();
        }

        InputAction::ToggleWaitMarkers => {
            ui.state.show_wait_markers = !ui.state.show_wait_markers;
            state.needs_redraw = true;
            window.request_redraw();
        }

        InputAction::ToggleScaleBar => {
            // Scale bar is always visible — toggle disabled
        }

        InputAction::ZoomIn => {
            let vp = ui.state.cached_viewport_rect;
            state.render.zoom(1.2, vp.width() / 2.0, vp.height() / 2.0, vp.width(), vp.height());
            state.needs_redraw = true;
            window.request_redraw();
        }

        InputAction::ZoomOut => {
            let vp = ui.state.cached_viewport_rect;
            state.render.zoom(0.8, vp.width() / 2.0, vp.height() / 2.0, vp.width(), vp.height());
            state.needs_redraw = true;
            window.request_redraw();
        }

        InputAction::ToolPan => {
            ui.state.tool_state.active_mode = ToolMode::Pan;
            window.request_redraw();
        }

        InputAction::ToggleZoomSelect => {
            if ui.state.tool_state.active_mode == ToolMode::ZoomSelect {
                ui.state.tool_state.active_mode = ToolMode::Pan;
            } else {
                ui.state.tool_state.active_mode = ToolMode::ZoomSelect;
            }
            window.request_redraw();
        }

        InputAction::ToggleRuler => {
            if ui.state.tool_state.active_mode == ToolMode::Ruler {
                ui.state.tool_state.active_mode = ToolMode::Pan;
                ui.state.tool_state.ruler_start = None;
                ui.state.tool_state.ruler_end = None;
            } else {
                ui.state.tool_state.active_mode = ToolMode::Ruler;
            }
            window.request_redraw();
        }

        InputAction::ClearMeasurements => {
            ui.state.tool_state.ruler_measurements.clear();
            window.request_redraw();
        }

        InputAction::ParamModeNone => {
            ui.state.param_mode = None;
            ui.state.color_mode = ColorMode::ByVectorType;
            state.needs_redraw = true;
            window.request_redraw();
        }

        InputAction::ParamModePower => {
            ui.state.param_mode = Some(ParameterMode::Power);
            ui.state.color_mode = ColorMode::ByParameter(ParameterMode::Power);
            state.needs_redraw = true;
            window.request_redraw();
        }

        InputAction::ParamModeSpeed => {
            ui.state.param_mode = Some(ParameterMode::Speed);
            ui.state.color_mode = ColorMode::ByParameter(ParameterMode::Speed);
            state.needs_redraw = true;
            window.request_redraw();
        }

        InputAction::ToggleFileInfo => {
            ui.state.show_file_info = !ui.state.show_file_info;
            window.request_redraw();
        }

        InputAction::ToggleControls => {
            ui.state.show_controls = !ui.state.show_controls;
            window.request_redraw();
        }

        InputAction::ToggleVectorView => {
            ui.state.vector_view_enabled = !ui.state.vector_view_enabled;
            if ui.state.vector_view_enabled {
                // Reset to show all vectors
                ui.state.current_vector_index = ui.state.total_vectors_in_layer.saturating_sub(1);
            }
            // Stop playback when toggling vector view
            ui.state.vector_view_playing = false;
            ui.state.playback_time_accumulator = 0.0;
            state.needs_redraw = true;
            window.request_redraw();
        }

        InputAction::NextVector => {
            if ui.state.vector_view_enabled && ui.state.current_vector_index + 1 < ui.state.total_vectors_in_layer {
                ui.state.current_vector_index += 1;
                // Stop playback on manual navigation
                ui.state.vector_view_playing = false;
                ui.state.playback_time_accumulator = 0.0;
                state.needs_redraw = true;
                window.request_redraw();
            }
        }

        InputAction::PrevVector => {
            if ui.state.vector_view_enabled && ui.state.current_vector_index > 0 {
                ui.state.current_vector_index -= 1;
                // Stop playback on manual navigation
                ui.state.vector_view_playing = false;
                ui.state.playback_time_accumulator = 0.0;
                state.needs_redraw = true;
                window.request_redraw();
            }
        }

        InputAction::ToggleVectorPlayback => {
            if ui.state.vector_view_enabled {
                ui.state.vector_view_playing = !ui.state.vector_view_playing;
                if ui.state.vector_view_playing {
                    ui.state.playback_time_accumulator = 0.0;
                    // If at last vector, reset to 0 to replay
                    if ui.state.current_vector_index >= ui.state.total_vectors_in_layer.saturating_sub(1) {
                        ui.state.current_vector_index = 0;
                    }
                }
                state.needs_redraw = true;
                window.request_redraw();
            }
        }

        InputAction::ToggleSidebar => {
            ui.state.sidebar_open = !ui.state.sidebar_open;
            state.needs_redraw = true;
            window.request_redraw();
        }

        InputAction::NextTab => {
            let ids = &state.tab_manager.open_tab_ids;
            if ids.len() > 1 {
                if let Some(active) = state.tab_manager.active_tab_id {
                    let idx = ids.iter().position(|&id| id == active).unwrap_or(0);
                    let next = ids[(idx + 1) % ids.len()];
                    state.tab_manager.set_active(next);
                    ui.state.active_tab_file = Some(next);
                    state.needs_redraw = true;
                    window.request_redraw();
                }
            }
        }

        InputAction::PrevTab => {
            let ids = &state.tab_manager.open_tab_ids;
            if ids.len() > 1 {
                if let Some(active) = state.tab_manager.active_tab_id {
                    let idx = ids.iter().position(|&id| id == active).unwrap_or(0);
                    let prev = ids[(idx + ids.len() - 1) % ids.len()];
                    state.tab_manager.set_active(prev);
                    ui.state.active_tab_file = Some(prev);
                    state.needs_redraw = true;
                    window.request_redraw();
                }
            }
        }

        InputAction::CloseTab => {
            if let Some(close_id) = state.tab_manager.active_tab_id {
                state.files.toggle_visibility_off(close_id);
                state.tab_manager.close_tab(close_id);
                ui.state.active_tab_file = state.tab_manager.active_tab_id;
                ui.state.has_multiple_files = state.tab_manager.tab_count() > 1;
                if state.tab_manager.open_tab_ids.len() < 2 && ui.state.view_mode == ViewMode::Split {
                    ui.state.view_mode = ViewMode::Tab;
                }
                let z_heights = state.files.merged_z_heights();
                state.navigation.initialize_from_z_heights(z_heights);
                ui.state.param_ranges = state.files.merged_param_ranges();
                state.needs_redraw = true;
                window.request_redraw();
            }
        }

        InputAction::FirstVector => {
            if ui.state.vector_view_enabled && ui.state.total_vectors_in_layer > 0 {
                ui.state.current_vector_index = 0;
                ui.state.vector_view_playing = false;
                ui.state.playback_time_accumulator = 0.0;
                state.needs_redraw = true;
                window.request_redraw();
            }
        }

        InputAction::LastVector => {
            if ui.state.vector_view_enabled && ui.state.total_vectors_in_layer > 0 {
                ui.state.current_vector_index = ui.state.total_vectors_in_layer.saturating_sub(1);
                ui.state.vector_view_playing = false;
                ui.state.playback_time_accumulator = 0.0;
                state.needs_redraw = true;
                window.request_redraw();
            }
        }

        InputAction::PlaybackSpeedUp => {
            let speed_opts: [f32; 4] = [0.5, 1.0, 2.0, 5.0];
            let idx = speed_opts.iter()
                .position(|&s| (s - ui.state.playback_speed).abs() < 0.01)
                .unwrap_or(1);
            if idx + 1 < speed_opts.len() {
                ui.state.playback_speed = speed_opts[idx + 1];
            }
            window.request_redraw();
        }

        InputAction::PlaybackSpeedDown => {
            let speed_opts: [f32; 4] = [0.5, 1.0, 2.0, 5.0];
            let idx = speed_opts.iter()
                .position(|&s| (s - ui.state.playback_speed).abs() < 0.01)
                .unwrap_or(1);
            if idx > 0 {
                ui.state.playback_speed = speed_opts[idx - 1];
            }
            window.request_redraw();
        }

        InputAction::UndoMeasurement => {
            if ui.state.tool_state.active_mode == ToolMode::Ruler {
                ui.state.tool_state.ruler_measurements.pop();
                window.request_redraw();
            }
        }

        InputAction::FocusLayerInput => {
            ui.state.focus_layer_input = true;
            window.request_redraw();
        }

        InputAction::CycleViewMode => {
            if ui.state.has_multiple_files {
                ui.state.view_mode = match ui.state.view_mode {
                    ViewMode::Overlay => ViewMode::Tab,
                    ViewMode::Tab => ViewMode::Split,
                    ViewMode::Split => ViewMode::Overlay,
                };
                state.needs_redraw = true;
                window.request_redraw();
            }
        }

        InputAction::ToggleSplit => {
            if ui.state.has_multiple_files {
                if ui.state.view_mode == ViewMode::Split {
                    ui.state.view_mode = ViewMode::Tab;
                } else {
                    ui.state.view_mode = ViewMode::Split;
                    ui.state.split_ratio = 0.5;
                }
                state.needs_redraw = true;
                window.request_redraw();
            }
        }

        InputAction::JumpToTab(index) => {
            let ids = &state.tab_manager.open_tab_ids;
            if index < ids.len() {
                let target = ids[index];
                state.tab_manager.set_active(target);
                ui.state.active_tab_file = Some(target);
                state.needs_redraw = true;
                window.request_redraw();
            }
        }

        InputAction::FocusLeftPane => {
            if ui.state.view_mode == ViewMode::Split {
                ui.state.active_split_pane = SplitPane::Left;
                state.needs_redraw = true;
                window.request_redraw();
            }
        }

        InputAction::FocusRightPane => {
            if ui.state.view_mode == ViewMode::Split {
                ui.state.active_split_pane = SplitPane::Right;
                state.needs_redraw = true;
                window.request_redraw();
            }
        }

        InputAction::ParamModeFile => {
            if ui.state.has_multiple_files {
                ui.state.param_mode = None;
                ui.state.color_mode = ColorMode::ByFile;
                state.needs_redraw = true;
                window.request_redraw();
            }
        }

        InputAction::TogglePreferences => {
            ui.state.show_preferences = !ui.state.show_preferences;
            window.request_redraw();
        }

        InputAction::Quit => {
            // Handled in main event handler
        }
    }
}

/// Open a native file dialog and load the selected file(s) in a background thread
fn open_file_dialog(
    window: &mut AppWindow,
    state: &mut AppState,
    _ui: &mut UiRenderer,
    load_use_case: &Arc<LoadToolpathUseCase>,
) {
    // Don't open dialog if already loading
    {
        let ls = state.loading_state.lock().unwrap();
        if matches!(*ls, LoadingState::Loading(_)) {
            return;
        }
    }

    let files = rfd::FileDialog::new()
        .add_filter("ILT/CLI Files", &["ilt", "cli"])
        .add_filter("All Files", &["*"])
        .set_title("Open Toolpath Files")
        .pick_files();

    if let Some(paths) = files {
        if !paths.is_empty() {
            info!("Opening {} file(s)", paths.len());
            start_background_load(load_use_case, paths, &state.loading_state);
            window.request_redraw();
        }
    }
}

/// Helper: accumulate vector type counts from a layer into totals
fn accumulate_vector_counts(layer: &crate::domain::entities::Layer, counts: &mut crate::presentation::VectorCounts) {
    for v in &layer.vectors {
        match v.vector_type {
            VectorType::Hatch => counts.hatches += 1,
            VectorType::Contour | VectorType::BaseContour | VectorType::CoincidingContour | VectorType::DepthContour => counts.contours += 1,
            VectorType::Boundary => counts.boundaries += 1,
            _ => {}
        }
        counts.total_vectors += 1;
        if v.parameters.wait_time.is_some() {
            counts.wait_count += 1;
        }
    }
    // Extract per-source parameters (vk/vs) from first layer that has them
    if counts.vk_power.is_none() {
        for (label, params) in &layer.params {
            match label.as_str() {
                "vk" => {
                    counts.vk_power = params.power;
                    counts.vk_speed = params.speed;
                }
                "vs" => {
                    counts.vs_power = params.power;
                    counts.vs_speed = params.speed;
                }
                _ => {}
            }
        }
    }
}

/// Render a frame
fn render_frame(
    window: &mut AppWindow,
    state: &mut AppState,
    renderer: &mut GlRenderer,
    ui: &mut UiRenderer,
    load_use_case: &Arc<LoadToolpathUseCase>,
) -> Result<()> {
    // Poll background loading state
    let loading_active = poll_loading_state(state, ui, window);

    // Update view transform for UI overlays (ruler, grid labels, scale bar)
    // Use cached viewport rect from last frame for pre-UI calculations
    let cached_vp = ui.state.cached_viewport_rect;
    ui.update_view_transform(&state.render.view_state, cached_vp.width(), cached_vp.height());

    // ── Sync active tab's ViewState ↔ global render view state ──
    // Before rendering: copy active tab's camera into state.render.view_state
    // so that non-split code paths (Overlay/Tab) use the per-tab camera.
    if let Some(tab) = state.tab_manager.active_tab() {
        state.render.view_state = tab.view_state.clone();
    }

    // 1. Run egui logic to get current toggle/slider values (no painting yet)
    let ui_output = ui.run_ui(&window.window, &state.files, &mut state.tab_manager);

    // Cache the authoritative viewport rect from LayoutRegions for use by event handlers
    ui.state.cached_viewport_rect = ui_output.viewport_rect;

    // Extract viewport parameters from the single source of truth (LayoutRegions)
    let vp = ui_output.viewport_rect;
    let vp_w = vp.width();
    let vp_h = vp.height();
    let sidebar_w = ui_output.sidebar_total_width;
    let bottom_h = ui_output.bottom_bar_height;
    let hdr_h = ui_output.header_height;

    // Handle file open request from Load button or file panel Add button
    if ui_output.open_file_requested || ui_output.add_files_requested {
        open_file_dialog(window, state, ui, load_use_case);
    }

    // ── Handle tab bar actions ──
    if let Some(switch_id) = ui_output.switch_tab {
        // Reopen the tab if it was closed (e.g. user clicked file in sidebar)
        if !state.tab_manager.open_tab_ids.contains(&switch_id) {
            state.tab_manager.reopen_tab(switch_id);
            // Restore file visibility (close_tab sets it invisible)
            if let Some(f) = state.files.files.iter_mut().find(|f| f.id == switch_id) {
                f.visible = true;
            }
            // Re-merge navigation for the now-visible file
            let z_heights = state.files.merged_z_heights();
            state.navigation.initialize_from_z_heights(z_heights);
            ui.state.param_ranges = state.files.merged_param_ranges();
            ui.state.has_multiple_files = state.tab_manager.tab_count() > 1;
        }
        state.tab_manager.set_active(switch_id);
        ui.state.active_tab_file = Some(switch_id);
        state.needs_redraw = true;
    }
    if let Some(close_id) = ui_output.close_tab {
        state.files.toggle_visibility_off(close_id);
        state.tab_manager.close_tab(close_id);
        ui.state.active_tab_file = state.tab_manager.active_tab_id;
        ui.state.has_multiple_files = state.tab_manager.tab_count() > 1;
        if state.tab_manager.open_tab_ids.len() < 2 && ui.state.view_mode == ViewMode::Split {
            ui.state.view_mode = ViewMode::Tab;
        }
        // Re-initialize global navigation from merged Z-heights of remaining visible files
        let z_heights = state.files.merged_z_heights();
        state.navigation.initialize_from_z_heights(z_heights);
        ui.state.param_ranges = state.files.merged_param_ranges();
        state.needs_redraw = true;
    }
    if let Some(partner_id) = ui_output.split_partner_changed {
        state.tab_manager.split_partner_id = Some(partner_id);
        state.needs_redraw = true;
    }

    // Handle drag-to-split: set the dragged tab as split partner + switch to Split mode
    if let Some(partner_id) = ui_output.drag_to_split {
        state.tab_manager.split_partner_id = Some(partner_id);
        ui.state.view_mode = ViewMode::Split;
        ui.state.split_ratio = 0.5;
        state.needs_redraw = true;
    }

    // ── Overlay mode actions ──
    if let Some(toggle_id) = ui_output.overlay_toggle {
        state.tab_manager.toggle_overlay_visible(toggle_id);
        state.needs_redraw = true;
    }
    if ui_output.overlay_select_all {
        state.tab_manager.overlay_select_all();
        state.needs_redraw = true;
    }
    if let Some(only_id) = ui_output.overlay_select_only {
        state.tab_manager.overlay_select_none();
        state.tab_manager.overlay_visible_ids.insert(only_id);
        state.tab_manager.set_active(only_id);
        ui.state.active_tab_file = Some(only_id);
        state.needs_redraw = true;
    }

    // ── Context menu: close others / close all / show in split ──
    if let Some(keep_id) = ui_output.close_others {
        let ids_to_close: Vec<usize> = state.tab_manager.open_tab_ids.iter()
            .filter(|&&id| id != keep_id)
            .copied()
            .collect();
        for id in ids_to_close {
            state.files.toggle_visibility_off(id);
            state.tab_manager.close_tab(id);
        }
        state.tab_manager.set_active(keep_id);
        ui.state.active_tab_file = Some(keep_id);
        ui.state.has_multiple_files = state.tab_manager.tab_count() > 1;
        if state.tab_manager.open_tab_ids.len() < 2 && ui.state.view_mode == ViewMode::Split {
            ui.state.view_mode = ViewMode::Tab;
        }
        let z_heights = state.files.merged_z_heights();
        state.navigation.initialize_from_z_heights(z_heights);
        ui.state.param_ranges = state.files.merged_param_ranges();
        state.needs_redraw = true;
    }
    if ui_output.close_all {
        let ids_to_close: Vec<usize> = state.tab_manager.open_tab_ids.clone();
        for id in ids_to_close {
            state.files.toggle_visibility_off(id);
            state.tab_manager.close_tab(id);
        }
        ui.state.active_tab_file = None;
        ui.state.has_multiple_files = false;
        if ui.state.view_mode == ViewMode::Split {
            ui.state.view_mode = ViewMode::Tab;
        }
        let z_heights = state.files.merged_z_heights();
        state.navigation.initialize_from_z_heights(z_heights);
        ui.state.param_ranges = state.files.merged_param_ranges();
        state.needs_redraw = true;
    }
    if let Some(split_id) = ui_output.show_in_split {
        state.tab_manager.split_partner_id = Some(split_id);
        state.tab_manager.split_right_active_id = Some(split_id);
        ui.state.view_mode = ViewMode::Split;
        ui.state.split_ratio = 0.5;
        state.needs_redraw = true;
    }

    // ── Split mode: right pane tab switch ──
    if let Some(right_id) = ui_output.split_right_switch {
        state.tab_manager.split_right_active_id = Some(right_id);
        state.tab_manager.split_partner_id = Some(right_id);
        state.needs_redraw = true;
    }

    // ── Tab reorder ──
    if let Some((from, to)) = ui_output.reorder_tab {
        state.tab_manager.reorder_tab(from, to);
        state.needs_redraw = true;
    }

    // ── Split sync toggle ──
    if ui_output.split_sync_toggled {
        state.tab_manager.toggle_split_sync();
        state.needs_redraw = true;
    }

    // Handle file removal from file panel
    if let Some(remove_id) = ui_output.remove_file {
        state.files.remove_file(remove_id);
        state.tab_manager.remove_tab(remove_id);
        // Re-initialize navigation from merged Z-heights
        let z_heights = state.files.merged_z_heights();
        state.navigation.initialize_from_z_heights(z_heights);
        ui.state.has_multiple_files = state.tab_manager.tab_count() > 1;
        if state.tab_manager.open_tab_ids.len() < 2 && ui.state.view_mode == ViewMode::Split {
            ui.state.view_mode = ViewMode::Tab;
        }
        ui.state.param_ranges = state.files.merged_param_ranges();
        // Reset active tab if the removed file was active
        ui.state.active_tab_file = state.tab_manager.active_tab_id;
        let nav_state = state.navigation.state();
        ui.update_from_navigation(nav_state.current_index, nav_state.total_layers, nav_state.current_z);
        state.needs_redraw = true;
    }

    // Handle file visibility toggle from file panel
    if let Some(toggle_id) = ui_output.toggle_file_visibility {
        state.files.toggle_visibility(toggle_id);
        // Re-initialize navigation from merged Z-heights of visible files
        let z_heights = state.files.merged_z_heights();
        state.navigation.initialize_from_z_heights(z_heights);
        ui.state.param_ranges = state.files.merged_param_ranges();
        let nav_state = state.navigation.state();
        ui.update_from_navigation(nav_state.current_index, nav_state.total_layers, nav_state.current_z);
        state.needs_redraw = true;
    }

    // Handle zoom in/out/fit button clicks from the tool panel
    if ui_output.zoom_in_requested {
        state.render.zoom(1.2, vp_w / 2.0, vp_h / 2.0, vp_w, vp_h);
        state.needs_redraw = true;
    }
    if ui_output.zoom_out_requested {
        state.render.zoom(0.8, vp_w / 2.0, vp_h / 2.0, vp_w, vp_h);
        state.needs_redraw = true;
    }
    if ui_output.fit_view_requested {
        if let Some((min, max)) = state.files.merged_bounds() {
            let render_height = vp_h.max(100.0);
            let bounds = Bounds2D::new(min, max);
            state.render.view_state.fit_to_bounds(&bounds, vp_w, render_height);
            state.needs_redraw = true;
        }
    }

    // Handle layer changes from slider/buttons — now routed through active tab
    let layer_changed = if let Some(tab) = state.tab_manager.active_tab() {
        ui_output.layer_index != tab.navigation.state().current_index
    } else {
        false
    };
    if layer_changed {
        // The tab's navigation was already updated in run_ui sync-back
        state.navigation.go_to_layer(ui_output.layer_index);
        state.needs_redraw = true;
        // Stop playback on layer change
        if let Some(tab) = state.tab_manager.active_tab_mut() {
            tab.vector_view_playing = false;
            tab.playback_time_accumulator = 0.0;
        }
        ui.state.vector_view_playing = false;
        ui.state.playback_time_accumulator = 0.0;
    }

    // ── Vector playback advancement ──
    // Compute delta time
    let now = Instant::now();
    let delta_seconds = state.last_frame_time
        .map(|prev| now.duration_since(prev).as_secs_f64())
        .unwrap_or(0.0)
        .min(0.1); // cap at 100ms to avoid jumps after pauses
    state.last_frame_time = Some(now);

    let playback_active = ui.state.vector_view_playing && ui.state.vector_view_enabled;
    if playback_active {
        // Use active tab's current Z for playback
        let current_z = state.tab_manager.active_tab()
            .map(|t| t.navigation.state().current_z)
            .unwrap_or(state.navigation.state().current_z);
        // For playback, use the active tab's file
        let active_file_id = state.tab_manager.active_tab_id;
        let playback_layer = active_file_id.and_then(|id| {
            state.files.files.iter().find(|f| f.id == id)
                .and_then(|f| f.toolpath.slice_stack.get_layer_by_z(current_z))
        }).or_else(|| {
            state.files.visible_files()
                .filter_map(|f| f.toolpath.slice_stack.get_layer_by_z(current_z))
                .next()
        });
        if let Some(layer) = playback_layer {
            ui.state.playback_time_accumulator += delta_seconds * ui.state.playback_speed as f64;

            // Advance through vectors based on their real scan time
            loop {
                let idx = ui.state.current_vector_index;
                if idx >= layer.vectors.len().saturating_sub(1) {
                    // Reached last vector — stop playback
                    ui.state.vector_view_playing = false;
                    ui.state.playback_time_accumulator = 0.0;
                    break;
                }

                let vec = &layer.vectors[idx];
                let length_mm = vec.length();
                let speed_mm_s = vec.parameters.speed.unwrap_or(1000.0) as f64;
                let scan_time_s = if speed_mm_s > 0.0 {
                    length_mm as f64 / speed_mm_s
                } else {
                    0.001 // fallback: 1ms
                };
                // Add wait time (stored in microseconds)
                let wait_s = vec.parameters.wait_time.unwrap_or(0.0) as f64 / 1_000_000.0;
                let total_duration = scan_time_s + wait_s;

                if ui.state.playback_time_accumulator >= total_duration {
                    ui.state.playback_time_accumulator -= total_duration;
                    ui.state.current_vector_index += 1;
                    state.needs_redraw = true;
                } else {
                    break;
                }
            }
        }
    }
    // Sync playback state back to active tab (may have been advanced above)
    if let Some(tab) = state.tab_manager.active_tab_mut() {
        tab.current_vector_index = ui.state.current_vector_index;
        tab.vector_view_playing = ui.state.vector_view_playing;
        tab.playback_time_accumulator = ui.state.playback_time_accumulator;
    }
    // Sync visibility toggles from UI to display options BEFORE rendering
    // Use live vector index from ui.state (may have been advanced by playback)
    let live_vector_index = ui.state.current_vector_index;
    let live_vector_view_enabled = ui.state.vector_view_enabled;
    let vector_view_changed = state.render.display_options.max_vector_index != if live_vector_view_enabled {
        Some(live_vector_index)
    } else {
        None
    };
    let toggles_changed =
        state.render.display_options.show_slices != ui_output.show_slices
        || state.render.display_options.show_contours != ui_output.show_contours
        || state.render.display_options.show_hatches != ui_output.show_hatches
        || state.render.display_options.show_arrows != ui_output.show_arrows
        || state.render.display_options.show_wait_markers != ui_output.show_wait_markers
        || state.render.display_options.param_mode != ui_output.param_mode
        || state.render.display_options.param_filter_min != ui_output.param_filter_min
        || state.render.display_options.param_filter_max != ui_output.param_filter_max
        || state.render.display_options.wait_time_min != ui_output.wait_filter_min
        || state.render.display_options.wait_time_max != ui_output.wait_filter_max
        || state.render.display_options.show_grid != ui_output.tool_state.show_grid
        || vector_view_changed
        || ui_output.canvas_changed;

    // Handle theme/palette changes from preferences dialog
    if ui_output.theme_changed {
        let pal = &ui_output.active_palette;
        renderer.set_color_scheme(ColorScheme::from_palette(pal));
        renderer.set_gradient_stops(pal.gradient_stops.clone());
        state.render.display_options.background_color = pal.background;
        state.render.display_options.grid_minor_color = pal.grid_minor;
        state.render.display_options.grid_major_color = pal.grid_major;

        // Persist theme preferences
        let cm = JsonConfigManager::new();
        if let Ok(mut cfg) = cm.load() {
            cfg.theme.mode = ui.state.theme_mode;
            cfg.theme.light_palette = ui.state.light_palette_id;
            cfg.theme.dark_palette = ui.state.dark_palette_id;
            let _ = cm.save(&cfg);
        }
    }

    // Handle canvas settings changes from preferences dialog
    if ui_output.canvas_changed {
        let cc = &ui.state.canvas_settings;
        state.render.display_options.line_width = cc.line_width;
        state.render.display_options.boundary_width_multiplier = cc.boundary_width_multiplier;
        state.render.display_options.contour_width_multiplier = cc.contour_width_multiplier;
        state.render.display_options.hatch_width_multiplier = cc.hatch_width_multiplier;
        state.render.display_options.arrow_size_multiplier = cc.arrow_size_multiplier;
        state.render.display_options.wait_marker_size_multiplier = cc.wait_marker_size_multiplier;
        state.render.display_options.future_vector_alpha = cc.future_vector_alpha;
        state.render.display_options.show_direction_gradient = cc.show_direction_gradient;
        state.render.display_options.grid_line_width_minor = cc.grid_line_width_minor;
        state.render.display_options.grid_line_width_major = cc.grid_line_width_major;
        state.render.display_options.grid_opacity = cc.grid_opacity;
        state.render.display_options.antialiasing = cc.antialiasing;

        // Persist canvas preferences
        let cm = JsonConfigManager::new();
        if let Ok(mut cfg) = cm.load() {
            cfg.canvas = ui.state.canvas_settings.clone();
            let _ = cm.save(&cfg);
        }
    }

    state.render.display_options.show_slices = ui_output.show_slices;
    state.render.display_options.show_contours = ui_output.show_contours;
    state.render.display_options.show_hatches = ui_output.show_hatches;
    state.render.display_options.show_arrows = ui_output.show_arrows;
    state.render.display_options.show_wait_markers = ui_output.show_wait_markers;
    state.render.display_options.param_mode = ui_output.param_mode;
    state.render.display_options.param_filter_min = ui_output.param_filter_min;
    state.render.display_options.param_filter_max = ui_output.param_filter_max;
    state.render.display_options.wait_time_min = ui_output.wait_filter_min;
    state.render.display_options.wait_time_max = ui_output.wait_filter_max;
    state.render.display_options.show_grid = ui_output.tool_state.show_grid;
    state.render.display_options.grid_unit = ui_output.global_units.length;
    state.render.display_options.max_vector_index = if live_vector_view_enabled {
        Some(live_vector_index)
    } else {
        None
    };

    // Update GL projection center offsets from authoritative viewport rect
    renderer.set_view_offset_x(-sidebar_w / 2.0);
    renderer.set_view_offset_y((hdr_h - bottom_h) / 2.0);

    // 2. Restore GL state first (egui leaves scissor test enabled), then clear
    renderer.begin_frame()?;

    let bg_color = state.render.display_options.background_color;
    renderer.clear(&bg_color)?;

    // Compute view mode and determine which files to render
    let view_mode = ui_output.view_mode;
    // Use active tab's current Z for layer lookup
    let current_z = state.tab_manager.active_tab()
        .map(|t| t.navigation.state().current_z)
        .unwrap_or_else(|| state.navigation.state().current_z);
    let color_mode = ui_output.color_mode;
    let mut total_counts = crate::presentation::VectorCounts::default();
    let mut any_layer_rendered = false;

    // Save full viewport size for split mode restoration
    let (full_phys_w, full_phys_h) = renderer.viewport_size();

    // Only render canvas content (grid + layers) when files are open
    let has_open_files = !state.tab_manager.open_tab_ids.is_empty();

    if has_open_files {
    match view_mode {
        ViewMode::Overlay => {
            // Clip GL rendering to the canvas area (excludes sidebar, toolbar, status bar, layer slider)
            let scale = window.scale_factor;
            let phys_x = (vp.left() * scale) as i32;
            let phys_y = (bottom_h * scale) as i32; // GL Y=0 is bottom
            let phys_w = (vp_w * scale) as u32;
            let phys_h = (vp_h * scale) as u32;
            renderer.set_sub_viewport(phys_x, phys_y, phys_w, phys_h);
            renderer.set_view_offset_x(0.0);
            renderer.set_view_offset_y(0.0);

            // Render background grid BEFORE layer content
            if state.render.display_options.show_grid {
                renderer.render_grid(
                    &state.render.view_state,
                    &state.render.display_options,
                )?;
            }

            // Overlay mode: render only files selected in overlay visibility
            let overlay_ids = &state.tab_manager.overlay_visible_ids;
            for file in state.files.visible_files() {
                // Skip files not selected in overlay mode (if any selections exist)
                if !overlay_ids.is_empty() && !overlay_ids.contains(&file.id) {
                    continue;
                }
                if let Some(layer) = file.toolpath.slice_stack.get_layer_by_z(current_z) {
                    accumulate_vector_counts(layer, &mut total_counts);

                    state.render.display_options.file_color_override = match color_mode {
                        ColorMode::ByFile => Some(file.color),
                        _ => None,
                    };
                    state.render.display_options.param_mode = match color_mode {
                        ColorMode::ByParameter(pm) => Some(pm),
                        _ => None,
                    };

                    renderer.render_layer(
                        layer,
                        &state.render.view_state,
                        &state.render.display_options,
                    )?;
                    any_layer_rendered = true;
                }
            }
            state.render.display_options.file_color_override = None;

            // Restore full viewport and view offsets
            renderer.restore_full_viewport(full_phys_w, full_phys_h);
            renderer.set_view_offset_x(-sidebar_w / 2.0);
            renderer.set_view_offset_y((hdr_h - bottom_h) / 2.0);
        }

        ViewMode::Tab => {
            // Clip GL rendering to the canvas area (excludes sidebar, toolbar, status bar, layer slider)
            let scale = window.scale_factor;
            let phys_x = (vp.left() * scale) as i32;
            let phys_y = (bottom_h * scale) as i32;
            let phys_w = (vp_w * scale) as u32;
            let phys_h = (vp_h * scale) as u32;
            renderer.set_sub_viewport(phys_x, phys_y, phys_w, phys_h);
            renderer.set_view_offset_x(0.0);
            renderer.set_view_offset_y(0.0);

            // Render background grid
            if state.render.display_options.show_grid {
                renderer.render_grid(
                    &state.render.view_state,
                    &state.render.display_options,
                )?;
            }

            // Tab mode: render only the active tab file
            let active_id = ui_output.active_tab_file
                .or_else(|| state.files.files.first().map(|f| f.id));

            if let Some(id) = active_id {
                if let Some(file) = state.files.files.iter().find(|f| f.id == id) {
                    if let Some(layer) = file.toolpath.slice_stack.get_layer_by_z(current_z) {
                        accumulate_vector_counts(layer, &mut total_counts);

                        state.render.display_options.file_color_override = match color_mode {
                            ColorMode::ByFile => Some(file.color),
                            _ => None,
                        };
                        state.render.display_options.param_mode = match color_mode {
                            ColorMode::ByParameter(pm) => Some(pm),
                            _ => None,
                        };

                        renderer.render_layer(
                            layer,
                            &state.render.view_state,
                            &state.render.display_options,
                        )?;
                        any_layer_rendered = true;
                    }
                }
            }
            state.render.display_options.file_color_override = None;

            // Restore full viewport and view offsets
            renderer.restore_full_viewport(full_phys_w, full_phys_h);
            renderer.set_view_offset_x(-sidebar_w / 2.0);
            renderer.set_view_offset_y((hdr_h - bottom_h) / 2.0);
        }

        ViewMode::Split => {
            // Split mode: render active tab + split partner side-by-side.
            // Cameras can be synced (shared) or independent per pane.
            let scale = window.scale_factor;
            let vp_phys_x = (vp.left() * scale) as u32;
            let vp_phys_w = (vp_w * scale) as u32;
            let vp_phys_h = (vp_h * scale) as u32;
            let split_ratio = ui_output.split_ratio;
            let gap_phys = (super::layout::SPLIT_GAP * scale) as u32;
            let usable_w = vp_phys_w.saturating_sub(gap_phys);
            let left_phys_w = (usable_w as f32 * split_ratio) as u32;
            let right_phys_w = usable_w.saturating_sub(left_phys_w);

            let canvas_y = (bottom_h * scale) as i32;

            renderer.set_view_offset_x(0.0);
            renderer.set_view_offset_y(0.0);

            // Left pane always uses active tab's camera
            let left_view = state.render.view_state.clone();

            // Right pane: independent camera when unsynced, shared when synced
            let right_view = if state.tab_manager.split_cameras_synced {
                left_view.clone()
            } else {
                state.tab_manager.split_right_view_state.clone()
                    .unwrap_or_else(|| left_view.clone())
            };

            // Right pane Z: independent layer when unsynced
            let right_z = if state.tab_manager.split_cameras_synced {
                current_z
            } else {
                state.tab_manager.split_right_navigation.as_ref()
                    .map(|nav| nav.state().current_z)
                    .unwrap_or(current_z)
            };

            // Determine left and right file IDs
            let left_id = state.tab_manager.active_tab_id
                .or_else(|| state.files.files.first().map(|f| f.id));
            let right_id = state.tab_manager.split_right_active_id
                .or(state.tab_manager.split_partner_id)
                .or_else(|| {
                    state.tab_manager.open_tab_ids.iter()
                        .find(|&&id| Some(id) != left_id)
                        .copied()
                })
                .or_else(|| state.files.files.iter().find(|f| Some(f.id) != left_id).map(|f| f.id));

            // Render left half
            if let Some(lid) = left_id {
                renderer.set_sub_viewport(vp_phys_x as i32, canvas_y, left_phys_w, vp_phys_h);
                renderer.clear(&bg_color)?;

                if state.render.display_options.show_grid {
                    renderer.render_grid(&left_view, &state.render.display_options)?;
                }

                if let Some(file) = state.files.files.iter().find(|f| f.id == lid) {
                    if let Some(layer) = file.toolpath.slice_stack.get_layer_by_z(current_z) {
                        accumulate_vector_counts(layer, &mut total_counts);
                        state.render.display_options.file_color_override = match color_mode {
                            ColorMode::ByFile => Some(file.color),
                            _ => None,
                        };
                        state.render.display_options.param_mode = match color_mode {
                            ColorMode::ByParameter(pm) => Some(pm),
                            _ => None,
                        };
                        renderer.render_layer(layer, &left_view, &state.render.display_options)?;
                        any_layer_rendered = true;
                    }
                }
            }

            // Render right half
            if let Some(rid) = right_id {
                renderer.set_sub_viewport(
                    (vp_phys_x + left_phys_w + gap_phys) as i32,
                    canvas_y, right_phys_w, vp_phys_h,
                );
                renderer.clear(&bg_color)?;

                if state.render.display_options.show_grid {
                    renderer.render_grid(&right_view, &state.render.display_options)?;
                }

                if let Some(file) = state.files.files.iter().find(|f| f.id == rid) {
                    if let Some(layer) = file.toolpath.slice_stack.get_layer_by_z(right_z) {
                        accumulate_vector_counts(layer, &mut total_counts);
                        state.render.display_options.file_color_override = match color_mode {
                            ColorMode::ByFile => Some(file.color),
                            _ => None,
                        };
                        state.render.display_options.param_mode = match color_mode {
                            ColorMode::ByParameter(pm) => Some(pm),
                            _ => None,
                        };
                        renderer.render_layer(layer, &right_view, &state.render.display_options)?;
                        any_layer_rendered = true;
                    }
                }
            }

            // Restore full viewport and view offsets before egui overlay
            renderer.restore_full_viewport(full_phys_w, full_phys_h);
            renderer.set_view_offset_x(-sidebar_w / 2.0);
            renderer.set_view_offset_y((hdr_h - bottom_h) / 2.0);
            state.render.display_options.file_color_override = None;
        }
    }
    } // end if has_open_files

    ui.state.vector_counts = total_counts.clone();

    // Update vector count for the vector slider (use active tab's file's layer)
    if any_layer_rendered {
        let active_layer = state.tab_manager.active_tab_id.and_then(|id| {
            state.files.files.iter().find(|f| f.id == id)
                .and_then(|f| f.toolpath.slice_stack.get_layer_by_z(current_z))
        }).or_else(|| {
            state.files.visible_files()
                .filter_map(|f| f.toolpath.slice_stack.get_layer_by_z(current_z))
                .next()
        });
        if let Some(layer) = active_layer {
            let layer_vec_count = layer.vector_count();
            if ui.state.total_vectors_in_layer != layer_vec_count {
                ui.state.total_vectors_in_layer = layer_vec_count;
                if layer_changed {
                    ui.state.current_vector_index = layer_vec_count.saturating_sub(1);
                }
            }
            // Sync vector counts to active tab
            if let Some(tab) = state.tab_manager.active_tab_mut() {
                tab.total_vectors_in_layer = ui.state.total_vectors_in_layer;
                tab.current_vector_index = ui.state.current_vector_index;
            }
        }
    }

    // Only log on layer/toggle changes to reduce spam
    if layer_changed || toggles_changed {
        info!(
            "Rendering layer z={:.3}: {} hatches, {} contours, {} boundaries | show_hatches={}",
            current_z,
            total_counts.hatches,
            total_counts.contours,
            total_counts.boundaries,
            state.render.display_options.show_hatches
        );
    }

    renderer.end_frame()?;

    // Handle snapshot request AFTER GL rendering, BEFORE egui overlay
    let snapshot_requested = ui_output.tool_state.snapshot_requested;
    if snapshot_requested {
        trigger_snapshot(renderer, state, &ui_output.tool_state);
    }

    // 3. Paint egui overlay on top of GL content
    ui.paint(&window.window);

    // Update UI with current navigation state for next frame
    let nav_state = state.navigation.state();
    ui.update_from_navigation(
        nav_state.current_index,
        nav_state.total_layers,
        nav_state.current_z,
    );

    // Only request another redraw if state actually changed
    let playback_running = ui.state.vector_view_playing && ui.state.vector_view_enabled;
    if layer_changed || toggles_changed || ui_output.needs_repaint || loading_active || playback_running {
        window.request_redraw();
    }

    // Swap buffers
    window.swap_buffers()?;

    state.needs_redraw = false;

    Ok(())
}

/// Place or remove a ruler measurement point at the given world position.
fn place_ruler_point(
    ui: &mut UiRenderer,
    view_state: &crate::application::ports::ViewState,
    world: Point2D,
) {
    let hit_radius_world = 6.0 / view_state.zoom.max(0.001);
    let mut removed = false;
    ui.state.tool_state.ruler_measurements.retain(|m| {
        if removed { return true; }
        let near_start = m.start.distance_to(&world) < hit_radius_world;
        let near_end = m.end.distance_to(&world) < hit_radius_world;
        if near_start || near_end {
            removed = true;
            false
        } else {
            true
        }
    });

    if !removed {
        if let Some(start_pt) = ui.state.tool_state.ruler_start {
            if start_pt.distance_to(&world) < hit_radius_world {
                ui.state.tool_state.ruler_start = None;
                ui.state.tool_state.ruler_end = None;
                removed = true;
            }
        }
    }

    if !removed {
        if ui.state.tool_state.ruler_start.is_none() {
            ui.state.tool_state.ruler_start = Some(world);
            ui.state.tool_state.ruler_end = Some(world);
        } else {
            let start = ui.state.tool_state.ruler_start.unwrap();
            let distance_mm = start.distance_to(&world);
            ui.state.tool_state.ruler_measurements.push(RulerMeasurement {
                start,
                end: world,
                distance_mm,
            });
            ui.state.tool_state.ruler_start = None;
            ui.state.tool_state.ruler_end = None;
        }
    }
}

/// Result of split-pane hit-testing: which pane the cursor is over, plus the
/// pane-local viewport rect (in logical screen coords) for coordinate conversion.
struct SplitPaneHit {
    pane: SplitPane,
    /// The viewport rect for this pane (logical coords, excluding the gap).
    pane_rect: egui::Rect,
}

/// Determine which split pane the mouse is over, and return the pane-local
/// viewport rect.  Returns `None` if the cursor is inside the divider gap.
fn detect_split_pane(
    mouse_x: f32,
    viewport: egui::Rect,
    split_ratio: f32,
) -> Option<SplitPaneHit> {
    let gap = super::layout::SPLIT_GAP;
    let half_gap = gap / 2.0;
    let divider_x = viewport.left() + viewport.width() * split_ratio;

    if mouse_x < divider_x - half_gap {
        // Left pane
        let pane_rect = egui::Rect::from_min_max(
            viewport.left_top(),
            egui::pos2(divider_x - half_gap, viewport.bottom()),
        );
        Some(SplitPaneHit { pane: SplitPane::Left, pane_rect })
    } else if mouse_x > divider_x + half_gap {
        // Right pane
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(divider_x + half_gap, viewport.top()),
            viewport.right_bottom(),
        );
        Some(SplitPaneHit { pane: SplitPane::Right, pane_rect })
    } else {
        // In the divider gap
        None
    }
}

/// Update hover tooltip info by hit-testing the nearest visible vector.
fn update_hover_info(
    state: &AppState,
    ui: &mut UiRenderer,
    window: &AppWindow,
) {
    if state.files.is_empty() {
        ui.state.hover_info = None;
        return;
    }

    let mouse = &window.input_state.mouse_pos;
    let vp = ui.state.cached_viewport_rect;

    // ── Split-pane aware viewport + view state selection ──
    let is_split = ui.state.view_mode == ViewMode::Split;
    let pane_hit = if is_split {
        detect_split_pane(mouse.x, vp, ui.state.split_ratio)
    } else {
        None
    };
    let (eff_vp, use_right) = if let Some(ref hit) = pane_hit {
        (hit.pane_rect, hit.pane == SplitPane::Right)
    } else if is_split {
        ui.state.hover_info = None;
        return;
    } else {
        (vp, false)
    };
    let eff_w = eff_vp.width();
    let eff_h = eff_vp.height();
    let use_right_independent = use_right && !state.tab_manager.split_cameras_synced;

    let view_ref = if use_right_independent {
        state.tab_manager.split_right_view_state.clone()
            .unwrap_or_else(|| state.render.view_state.clone())
    } else {
        state.render.view_state.clone()
    };

    let world = view_ref.screen_to_world(
        mouse.x - eff_vp.left(), mouse.y - eff_vp.top(), eff_w, eff_h,
    );

    let opts = &state.render.display_options;
    let threshold = 5.0 / view_ref.zoom.max(0.001);

    // Determine which Z to use (right pane may have independent navigation)
    let current_z = if use_right_independent {
        state.tab_manager.split_right_navigation.as_ref()
            .map(|nav| nav.state().current_z)
            .unwrap_or_else(|| state.navigation.state().current_z)
    } else {
        state.navigation.state().current_z
    };

    // In split mode, only hit-test against the file shown in this pane
    let files_to_test: Vec<&crate::domain::entities::FileEntry> = if is_split {
        let left_id = state.tab_manager.active_tab_id
            .or_else(|| state.files.files.first().map(|f| f.id));
        let right_id = state.tab_manager.split_right_active_id
            .or(state.tab_manager.split_partner_id)
            .or_else(|| {
                state.tab_manager.open_tab_ids.iter()
                    .find(|&&id| Some(id) != left_id)
                    .copied()
            })
            .or_else(|| state.files.files.iter().find(|f| Some(f.id) != left_id).map(|f| f.id));
        let target_id = if use_right { right_id } else { left_id };
        if let Some(tid) = target_id {
            state.files.files.iter().filter(|f| f.id == tid).collect()
        } else {
            vec![]
        }
    } else {
        state.files.visible_files().collect()
    };

    // Search across target files' layers at current Z
    let mut best_hit: Option<(f32, Option<f32>, Option<f32>, Option<f32>)> = None;
    for file in files_to_test {
        if let Some(layer) = file.toolpath.slice_stack.get_layer_by_z(current_z) {
            let visible_indices: Vec<usize> = layer.vectors.iter().enumerate()
                .filter(|(_, v)| match v.vector_type {
                    VectorType::Boundary => opts.show_slices,
                    VectorType::Contour | VectorType::BaseContour | VectorType::CoincidingContour => opts.show_contours,
                    VectorType::DepthContour => opts.show_depth_contours,
                    VectorType::Hatch => opts.show_hatches,
                    VectorType::Support | VectorType::Travel => true,
                })
                .map(|(i, _)| i)
                .collect();

            if let Some(hit) = find_nearest_vector(&world, layer, &visible_indices) {
                if hit.distance <= threshold {
                    let is_closer = best_hit.as_ref().map_or(true, |(d, _, _, _)| hit.distance < *d);
                    if is_closer {
                        let params = &layer.vectors[hit.vector_index].parameters;
                        best_hit = Some((hit.distance, params.power, params.speed, params.wait_time));
                    }
                }
            }
        }
    }

    if let Some((_dist, power, speed, wait_time)) = best_hit {
        ui.state.hover_info = Some(crate::presentation::HoverInfo {
            power,
            speed,
            wait_time,
            screen_pos: (mouse.x, mouse.y),
        });
    } else {
        ui.state.hover_info = None;
    }
}

/// Update the window title
fn update_window_title(window: &AppWindow, _state: &AppState) {
    window.set_title(&format!("Toolpath Viewer [{}]", crate::APP_VERSION));
}

/// Trigger a snapshot (screenshot or SVG export)
fn trigger_snapshot(
    renderer: &GlRenderer,
    state: &AppState,
    tool_state: &crate::presentation::ToolState,
) {
    // Show save dialog with format filter
    let file = rfd::FileDialog::new()
        .add_filter("PNG Image", &["png"])
        .add_filter("SVG Vector", &["svg"])
        .set_title("Save Snapshot")
        .save_file();

    if let Some(path) = file {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
            .to_lowercase();

        match ext.as_str() {
            "svg" => {
                if let Err(e) = export_svg(&path, state, tool_state) {
                    error!("SVG export failed: {}", e);
                }
            }
            _ => {
                // PNG capture
                let (w, h, pixels) = renderer.capture_framebuffer();
                match image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(w, h, pixels) {
                    Some(img) => {
                        if let Err(e) = img.save(&path) {
                            error!("Failed to save PNG: {}", e);
                        } else {
                            info!("Snapshot saved: {}", path.display());
                        }
                    }
                    None => {
                        error!("Failed to create image buffer");
                    }
                }
            }
        }
    }
}

/// Export the current view as SVG
fn export_svg(
    path: &std::path::Path,
    state: &AppState,
    tool_state: &crate::presentation::ToolState,
) -> Result<()> {
    use std::io::Write;
    use crate::domain::entities::VectorType;

    if state.files.is_empty() {
        return Err(anyhow::anyhow!("No files loaded"));
    }
    let current_z = state.navigation.state().current_z;

    // Collect all layers at current Z from visible files
    let layers: Vec<&crate::domain::entities::Layer> = state.files.visible_files()
        .filter_map(|f| f.toolpath.slice_stack.get_layer_by_z(current_z))
        .collect();
    if layers.is_empty() {
        return Err(anyhow::anyhow!("No layers at current Z height"));
    }

    // Compute combined bounds
    let (min_pt, max_pt) = {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for layer in &layers {
            if let Some((lo, hi)) = layer.bounds() {
                min_x = min_x.min(lo.x);
                min_y = min_y.min(lo.y);
                max_x = max_x.max(hi.x);
                max_y = max_y.max(hi.y);
            }
        }
        if min_x == f32::MAX {
            (crate::domain::value_objects::Point2D::zero(), crate::domain::value_objects::Point2D::new(100.0, 100.0))
        } else {
            (crate::domain::value_objects::Point2D::new(min_x, min_y), crate::domain::value_objects::Point2D::new(max_x, max_y))
        }
    };
    let padding = 5.0;
    let vb_x = min_pt.x - padding;
    let vb_y = min_pt.y - padding;
    let vb_w = (max_pt.x - min_pt.x) + padding * 2.0;
    let vb_h = (max_pt.y - min_pt.y) + padding * 2.0;

    let mut svg = String::new();
    svg.push_str(&format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="{:.3} {:.3} {:.3} {:.3}" width="{:.0}" height="{:.0}">
<rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" fill="#FAFAFA"/>
"##,
        vb_x, vb_y, vb_w, vb_h,
        vb_w.max(800.0), vb_h.max(600.0),
        vb_x, vb_y, vb_w, vb_h,
    ));

    // Color map for vector types
    let color_for_type = |vt: VectorType| -> &str {
        match vt {
            VectorType::Boundary => "#2C3E50",
            VectorType::Contour | VectorType::BaseContour | VectorType::CoincidingContour => "#00BCD4",
            VectorType::DepthContour => "#AD1457",
            VectorType::Hatch => "#E91E63",
            VectorType::Support => "#616161",
            VectorType::Travel => "#388E3C",
        }
    };

    for layer in &layers {
        for vector in &layer.vectors {
        // Apply same visibility filters as display
        let visible = match vector.vector_type {
            VectorType::Boundary => state.render.display_options.show_slices,
            VectorType::Contour | VectorType::BaseContour | VectorType::CoincidingContour => state.render.display_options.show_contours,
            VectorType::DepthContour => state.render.display_options.show_depth_contours,
            VectorType::Hatch => state.render.display_options.show_hatches,
            _ => true,
        };
        if !visible || vector.points.len() < 2 {
            continue;
        }

        let color = color_for_type(vector.vector_type);
        let stroke_width = match vector.vector_type {
            VectorType::Hatch => 0.05,
            VectorType::Boundary => 0.1,
            _ => 0.07,
        };

        if vector.vector_type == VectorType::Hatch && vector.points.len() == 2 {
            // Single line segment
            svg.push_str(&format!(
                r#"<line x1="{:.3}" y1="{:.3}" x2="{:.3}" y2="{:.3}" stroke="{}" stroke-width="{:.3}" stroke-linecap="round"/>"#,
                vector.points[0].x, vector.points[0].y,
                vector.points[1].x, vector.points[1].y,
                color, stroke_width,
            ));
            svg.push('\n');
        } else {
            // Polyline
            let points_str: String = vector.points.iter()
                .map(|p| format!("{:.3},{:.3}", p.x, p.y))
                .collect::<Vec<_>>()
                .join(" ");
            svg.push_str(&format!(
                r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="{:.3}" stroke-linecap="round" stroke-linejoin="round"/>"#,
                points_str, color, stroke_width,
            ));
            svg.push('\n');
        }
    }
    }

    svg.push_str("</svg>\n");

    let mut file = std::fs::File::create(path)?;
    file.write_all(svg.as_bytes())?;
    info!("SVG exported: {}", path.display());

    Ok(())
}
