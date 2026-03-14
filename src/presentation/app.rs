//! Main Application
//!
//! Orchestrates all components and manages application state.

use anyhow::Result;
use log::{info, warn, error};
use std::path::Path;
use std::sync::Arc;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;

use crate::application::ports::{ConfigManager, FileLoader, GridUnit, Renderer};
use crate::application::use_cases::{
    LoadToolpathUseCase, NavigateLayersUseCase, RenderViewUseCase, DisplayOption,
};
use crate::domain::entities::Toolpath;
use crate::domain::value_objects::{Bounds2D, Color, Point2D};
use crate::infrastructure::config::JsonConfigManager;
use crate::infrastructure::file_adapters::IltLoader;
use crate::infrastructure::rendering::GlRenderer;
use crate::presentation::{
    AppEvent, AppWindow, InputAction, MouseButton, ParamRanges, WindowConfig,
    create_window, run_event_loop, UiRenderer, TOOLBAR_HEIGHT,
    ToolMode, RulerMeasurement, SnapshotFormat,
};

/// Height of the top toolbar in pixels
const UI_TOOLBAR_HEIGHT: f32 = TOOLBAR_HEIGHT;

/// Application state
pub struct AppState {
    /// Currently loaded toolpath
    pub toolpath: Option<Toolpath>,
    /// Layer navigation
    pub navigation: NavigateLayersUseCase,
    /// Render state
    pub render: RenderViewUseCase,
    /// Whether a redraw is needed
    pub needs_redraw: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            toolpath: None,
            navigation: NavigateLayersUseCase::new(),
            render: RenderViewUseCase::new(),
            needs_redraw: true,
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

        Ok(Self { config })
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
        // Offset content center downward to account for top toolbar
        renderer.set_view_offset_y(UI_TOOLBAR_HEIGHT / 2.0);

        // Create UI renderer
        let mut ui = UiRenderer::new(app_window.glow_context.clone(), &app_window.window);

        // Create file loader
        let loaders: Vec<Arc<dyn FileLoader>> = vec![Arc::new(IltLoader::new())];
        let load_use_case = LoadToolpathUseCase::new(loaders);

        // Check for command line arguments
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            let file_path = &args[1];
            match load_file(&load_use_case, file_path, &mut state, width, height) {
                Err(e) => error!("Failed to load file: {}", e),
                Ok(ranges) => {
                    ui.state.param_ranges = ranges.clone();
                    // Set wait time range for marker coloring
                    if let Some((wmin, wmax)) = ranges.wait_time {
                        state.render.display_options.wait_time_min = wmin;
                        state.render.display_options.wait_time_max = wmax;
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
                    &load_use_case,
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

/// Load a file into the application state
fn load_file(
    use_case: &LoadToolpathUseCase,
    path: &str,
    state: &mut AppState,
    viewport_width: u32,
    viewport_height: u32,
) -> Result<ParamRanges> {
    info!("Loading file: {}", path);

    let toolpath = use_case.execute(Path::new(path))?;
    let stats = toolpath.slice_stack.stats();

    info!(
        "Loaded {} layers, {} vectors, {} points",
        stats.layer_count, stats.total_vectors, stats.total_points
    );

    // Log bounds information
    if let Some((min, max)) = toolpath.slice_stack.bounds() {
        info!(
            "Stack bounds: min=({:.3}, {:.3}), max=({:.3}, {:.3}), size=({:.3} x {:.3})",
            min.x, min.y, max.x, max.y, max.x - min.x, max.y - min.y
        );
    }

    // Initialize navigation
    state.navigation.initialize(&toolpath.slice_stack);

    // Fit view to content (subtract toolbar height from available render area)
    let render_height = (viewport_height as f32 - UI_TOOLBAR_HEIGHT).max(100.0);
    state.render.fit_to_stack(
        &toolpath.slice_stack,
        viewport_width as f32,
        render_height,
    );

    // Compute global parameter ranges for color mapping
    let ranges = compute_param_ranges(&toolpath);
    state.toolpath = Some(toolpath);
    state.needs_redraw = true;

    Ok(ranges)
}

/// Handle an application event
fn handle_event(
    window: &mut AppWindow,
    event: AppEvent,
    raw_event: Option<&WindowEvent>,
    state: &mut AppState,
    renderer: &mut GlRenderer,
    ui: &mut UiRenderer,
    load_use_case: &LoadToolpathUseCase,
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
            state.needs_redraw = true;
        }

        AppEvent::Redraw => {
            if let Err(e) = render_frame(window, state, renderer, ui, load_use_case) {
                error!("Render error: {}", e);
            }
        }

        AppEvent::MouseMove { x: _, y: _ } => {
            // Handle panning only when pointer is in viewport
            if pointer_in_viewport {
                let tool_mode = ui.state.tool_state.active_mode;
                let shift_held = window.input_state.shift_held;

                // Zoom selection drag (Shift+left drag OR ZoomSelect mode)
                if window.input_state.left_pressed && (shift_held || tool_mode == ToolMode::ZoomSelect) {
                    // Update end point of zoom rectangle (screen coords)
                    let mouse = &window.input_state.mouse_pos;
                    ui.state.tool_state.zoom_rect_end = Some(Point2D::new(mouse.x, mouse.y));
                    window.request_redraw();
                }
                // Ruler live preview (move cursor while placing second point)
                else if tool_mode == ToolMode::Ruler && ui.state.tool_state.ruler_start.is_some() {
                    let mouse = &window.input_state.mouse_pos;
                    let (width, height) = window.size();
                    let viewport_h = height as f32 - UI_TOOLBAR_HEIGHT;
                    let viewport_mouse_x = mouse.x;
                    let viewport_mouse_y = mouse.y - UI_TOOLBAR_HEIGHT;
                    let world = state.render.view_state.screen_to_world(
                        viewport_mouse_x, viewport_mouse_y, width as f32, viewport_h,
                    );
                    ui.state.tool_state.ruler_end = Some(world);
                    window.request_redraw();
                }
                // Normal panning
                else if window.input_state.middle_pressed || 
                   (window.input_state.left_pressed && window.input_state.ctrl_held) {
                    let delta = window.input_state.mouse_delta();
                    state.render.pan(delta.x, -delta.y);
                    state.needs_redraw = true;
                    window.request_redraw();
                }
            }
        }

        AppEvent::EguiEvent => {
            // egui forwarding already happened above; nothing else to do
        }

        AppEvent::MouseButton { button, pressed } => {
            if pointer_in_viewport {
                let tool_mode = ui.state.tool_state.active_mode;
                let shift_held = window.input_state.shift_held;
                let mouse = &window.input_state.mouse_pos;

                match button {
                    MouseButton::Left if pressed => {
                        // Zoom selection start
                        if shift_held || tool_mode == ToolMode::ZoomSelect {
                            ui.state.tool_state.zoom_rect_start = Some(Point2D::new(mouse.x, mouse.y));
                            ui.state.tool_state.zoom_rect_end = Some(Point2D::new(mouse.x, mouse.y));
                        }
                        // Ruler click
                        else if tool_mode == ToolMode::Ruler {
                            let (width, height) = window.size();
                            let viewport_h = height as f32 - UI_TOOLBAR_HEIGHT;
                            let viewport_mouse_x = mouse.x;
                            let viewport_mouse_y = mouse.y - UI_TOOLBAR_HEIGHT;
                            let world = state.render.view_state.screen_to_world(
                                viewport_mouse_x, viewport_mouse_y, width as f32, viewport_h,
                            );

                            if ui.state.tool_state.ruler_start.is_none() {
                                // Set start point
                                ui.state.tool_state.ruler_start = Some(world);
                                ui.state.tool_state.ruler_end = Some(world);
                            } else {
                                // Set end point → create measurement
                                let start = ui.state.tool_state.ruler_start.unwrap();
                                let distance_mm = start.distance_to(&world);
                                ui.state.tool_state.ruler_measurements.push(RulerMeasurement {
                                    start,
                                    end: world,
                                    distance_mm,
                                });
                                // Reset for next measurement
                                ui.state.tool_state.ruler_start = None;
                                ui.state.tool_state.ruler_end = None;
                            }
                            window.request_redraw();
                        }
                    }
                    MouseButton::Left if !pressed => {
                        // Zoom selection release → zoom to rectangle
                        if let (Some(start), Some(end)) = (
                            ui.state.tool_state.zoom_rect_start.take(),
                            ui.state.tool_state.zoom_rect_end.take(),
                        ) {
                            let dx = (end.x - start.x).abs();
                            let dy = (end.y - start.y).abs();
                            if dx > 5.0 && dy > 5.0 {
                                // Convert screen rect to world bounds
                                let (width, height) = window.size();
                                let viewport_h = height as f32 - UI_TOOLBAR_HEIGHT;
                                let viewport_w = width as f32;

                                let w1 = state.render.view_state.screen_to_world(
                                    start.x, start.y - UI_TOOLBAR_HEIGHT, viewport_w, viewport_h,
                                );
                                let w2 = state.render.view_state.screen_to_world(
                                    end.x, end.y - UI_TOOLBAR_HEIGHT, viewport_w, viewport_h,
                                );

                                let bounds = Bounds2D::new(
                                    Point2D::new(w1.x.min(w2.x), w1.y.min(w2.y)),
                                    Point2D::new(w1.x.max(w2.x), w1.y.max(w2.y)),
                                );
                                state.render.view_state.fit_to_bounds(&bounds, viewport_w, viewport_h);
                                state.needs_redraw = true;
                            }
                            window.request_redraw();
                        }
                    }
                    _ => {}
                }
            }
        }

        AppEvent::Scroll { delta } => {
            // Handle zoom only when pointer is in viewport
            if pointer_in_viewport {
                let (width, height) = window.size();
                let zoom_factor = if delta > 0.0 { 1.1 } else { 0.9 };
                let mouse = &window.input_state.mouse_pos;
                
                // Subtract toolbar height so zoom anchors correctly in viewport
                let viewport_mouse_y = mouse.y - UI_TOOLBAR_HEIGHT;
                let viewport_height = height as f32 - UI_TOOLBAR_HEIGHT;
                
                state.render.zoom(
                    zoom_factor,
                    mouse.x,
                    viewport_height - viewport_mouse_y, // Flip Y
                    width as f32,
                    viewport_height,
                );
                state.needs_redraw = true;
                window.request_redraw();
            }
        }

        AppEvent::KeyAction(action) => {
            handle_key_action(action, state, window, ui, load_use_case);
        }

        AppEvent::FileDropped(path) => {
            let (width, height) = window.size();
            match load_file(load_use_case, &path, state, width, height) {
                Err(e) => error!("Failed to load dropped file: {}", e),
                Ok(ranges) => {
                    ui.state.param_ranges = ranges.clone();
                    // Set wait time range for marker coloring
                    if let Some((wmin, wmax)) = ranges.wait_time {
                        state.render.display_options.wait_time_min = wmin;
                        state.render.display_options.wait_time_max = wmax;
                    }
                    update_window_title(window, state);
                }
            }
            window.request_redraw();
        }
    }

    false
}

/// Handle a key action
fn handle_key_action(
    action: InputAction,
    state: &mut AppState,
    window: &mut AppWindow,
    ui: &mut UiRenderer,
    load_use_case: &LoadToolpathUseCase,
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
            if let Some(ref toolpath) = state.toolpath {
                let (width, height) = window.size();
                let render_height = (height as f32 - UI_TOOLBAR_HEIGHT).max(100.0);
                state.render.fit_to_stack(
                    &toolpath.slice_stack,
                    width as f32,
                    render_height,
                );
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

        InputAction::Quit => {
            // Handled in main event handler
        }
    }
}

/// Open a native file dialog and load the selected file
fn open_file_dialog(
    window: &mut AppWindow,
    state: &mut AppState,
    ui: &mut UiRenderer,
    load_use_case: &LoadToolpathUseCase,
) {
    let file = rfd::FileDialog::new()
        .add_filter("ILT/CLI Files", &["ilt", "cli"])
        .add_filter("All Files", &["*"])
        .set_title("Open Toolpath File")
        .pick_file();

    if let Some(path) = file {
        let (width, height) = window.size();
        match load_file(load_use_case, path.to_string_lossy().as_ref(), state, width, height) {
            Err(e) => error!("Failed to load file: {}", e),
            Ok(ranges) => {
                ui.state.param_ranges = ranges.clone();
                // Set wait time range for marker coloring
                if let Some((wmin, wmax)) = ranges.wait_time {
                    state.render.display_options.wait_time_min = wmin;
                    state.render.display_options.wait_time_max = wmax;
                }
                update_window_title(window, state);
                info!("Loaded file: {}", path.display());
            }
        }
        window.request_redraw();
    }
}

/// Render a frame
fn render_frame(
    window: &mut AppWindow,
    state: &mut AppState,
    renderer: &mut GlRenderer,
    ui: &mut UiRenderer,
    load_use_case: &LoadToolpathUseCase,
) -> Result<()> {
    // Update view transform for UI overlays (ruler, grid labels, scale bar)
    let (vp_w, vp_h) = window.size();
    let viewport_height = vp_h as f32 - UI_TOOLBAR_HEIGHT;
    let viewport_width = vp_w as f32;
    ui.update_view_transform(&state.render.view_state, viewport_width, viewport_height);

    // 1. Run egui logic to get current toggle/slider values (no painting yet)
    let ui_output = ui.run_ui(&window.window);

    // Handle file open request from Load button
    if ui_output.open_file_requested {
        open_file_dialog(window, state, ui, load_use_case);
    }

    // Handle zoom in/out/fit button clicks from the tool panel
    if ui_output.zoom_in_requested {
        state.render.zoom(1.2, viewport_width / 2.0, viewport_height / 2.0, viewport_width, viewport_height);
        state.needs_redraw = true;
    }
    if ui_output.zoom_out_requested {
        state.render.zoom(0.8, viewport_width / 2.0, viewport_height / 2.0, viewport_width, viewport_height);
        state.needs_redraw = true;
    }
    if ui_output.fit_view_requested {
        if let Some(ref toolpath) = state.toolpath {
            let render_height = viewport_height.max(100.0);
            state.render.fit_to_stack(&toolpath.slice_stack, viewport_width, render_height);
            state.needs_redraw = true;
        }
    }

    // Handle layer changes from slider/buttons
    let nav_state = state.navigation.state();
    let layer_changed = ui_output.layer_index != nav_state.current_index;
    if layer_changed {
        state.navigation.go_to_layer(ui_output.layer_index);
        state.needs_redraw = true;
    }

    // Sync visibility toggles from UI to display options BEFORE rendering
    let toggles_changed =
        state.render.display_options.show_slices != ui_output.show_slices
        || state.render.display_options.show_contours != ui_output.show_contours
        || state.render.display_options.show_hatches != ui_output.show_hatches
        || state.render.display_options.show_arrows != ui_output.show_arrows
        || state.render.display_options.show_wait_markers != ui_output.show_wait_markers
        || state.render.display_options.param_mode != ui_output.param_mode
        || state.render.display_options.param_filter_min != ui_output.param_filter_min
        || state.render.display_options.param_filter_max != ui_output.param_filter_max
        || state.render.display_options.show_grid != ui_output.tool_state.show_grid;

    state.render.display_options.show_slices = ui_output.show_slices;
    state.render.display_options.show_contours = ui_output.show_contours;
    state.render.display_options.show_hatches = ui_output.show_hatches;
    state.render.display_options.show_arrows = ui_output.show_arrows;
    state.render.display_options.show_wait_markers = ui_output.show_wait_markers;
    state.render.display_options.param_mode = ui_output.param_mode;
    state.render.display_options.param_filter_min = ui_output.param_filter_min;
    state.render.display_options.param_filter_max = ui_output.param_filter_max;
    state.render.display_options.show_grid = ui_output.tool_state.show_grid;
    state.render.display_options.grid_unit = ui_output.tool_state.grid_unit;

    // 2. Restore GL state first (egui leaves scissor test enabled), then clear
    renderer.begin_frame()?;

    let bg_color = state.render.display_options.background_color;
    renderer.clear(&bg_color)?;

    // Render background grid BEFORE layer content
    if state.render.display_options.show_grid {
        renderer.render_grid(&state.render.view_state)?;
    }

    // Render current layer if we have a toolpath
    if let Some(ref toolpath) = state.toolpath {
        if let Some(layer) = state.navigation.get_current_layer(&toolpath.slice_stack) {
            // Count vectors by type for UI display
            use crate::domain::entities::VectorType;
            use crate::presentation::VectorCounts;
            let mut counts = VectorCounts::default();
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
            // Extract per-source parameters (vk/vs)
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
            ui.state.vector_counts = counts.clone();

            // Only log on layer/toggle changes to reduce spam
            if layer_changed || toggles_changed {
                info!(
                    "Rendering layer {}: {} hatches, {} contours, {} boundaries | show_hatches={}",
                    state.navigation.state().current_index,
                    counts.hatches,
                    counts.contours,
                    counts.boundaries,
                    state.render.display_options.show_hatches
                );
            }

            renderer.render_layer(
                layer,
                &state.render.view_state,
                &state.render.display_options,
            )?;
        }
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
    if layer_changed || toggles_changed || ui_output.needs_repaint {
        window.request_redraw();
    }

    // Swap buffers
    window.swap_buffers()?;

    state.needs_redraw = false;

    Ok(())
}

/// Update the window title with layer info
fn update_window_title(window: &AppWindow, state: &AppState) {
    let nav_state = state.navigation.state();
    if nav_state.total_layers > 0 {
        let title = format!(
            "Toolpath Viewer - Layer {}/{} (z = {:.3} mm)",
            nav_state.current_index + 1,
            nav_state.total_layers,
            nav_state.current_z
        );
        window.set_title(&title);
    } else {
        window.set_title("Toolpath Viewer - No file loaded");
    }
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

    let toolpath = state.toolpath.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No toolpath loaded"))?;
    let layer = state.navigation.get_current_layer(&toolpath.slice_stack)
        .ok_or_else(|| anyhow::anyhow!("No current layer"))?;

    // Compute bounds for SVG viewBox
    let (min_pt, max_pt) = layer.bounds()
        .unwrap_or((crate::domain::value_objects::Point2D::zero(), crate::domain::value_objects::Point2D::new(100.0, 100.0)));
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

    svg.push_str("</svg>\n");

    let mut file = std::fs::File::create(path)?;
    file.write_all(svg.as_bytes())?;
    info!("SVG exported: {}", path.display());

    Ok(())
}
