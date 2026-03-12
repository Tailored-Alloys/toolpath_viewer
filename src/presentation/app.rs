//! Main Application
//!
//! Orchestrates all components and manages application state.

use anyhow::Result;
use log::{info, warn, error};
use std::path::Path;
use std::sync::Arc;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;

use crate::application::ports::{ConfigManager, FileLoader, Renderer};
use crate::application::use_cases::{
    LoadToolpathUseCase, NavigateLayersUseCase, RenderViewUseCase, DisplayOption,
};
use crate::domain::entities::Toolpath;
use crate::domain::value_objects::{Bounds2D, Color};
use crate::infrastructure::config::JsonConfigManager;
use crate::infrastructure::file_adapters::IltLoader;
use crate::infrastructure::rendering::GlRenderer;
use crate::presentation::{
    AppEvent, AppWindow, InputAction, MouseButton, ParamRanges, WindowConfig,
    create_window, run_event_loop, UiRenderer, TOOLBAR_HEIGHT,
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
                    ui.state.param_ranges = ranges;
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
            if let Err(e) = render_frame(window, state, renderer, ui) {
                error!("Render error: {}", e);
            }
        }

        AppEvent::MouseMove { x: _, y: _ } => {
            // Handle panning only when pointer is in viewport
            if pointer_in_viewport {
                if window.input_state.middle_pressed || 
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

        AppEvent::MouseButton { button: _, pressed: _ } => {
            // Could handle click events here
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
            handle_key_action(action, state, window, load_use_case);
        }

        AppEvent::FileDropped(path) => {
            let (width, height) = window.size();
            match load_file(load_use_case, &path, state, width, height) {
                Err(e) => error!("Failed to load dropped file: {}", e),
                Ok(ranges) => {
                    ui.state.param_ranges = ranges;
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
    _load_use_case: &LoadToolpathUseCase,
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
            // Would open file dialog here
            info!("Open file dialog not yet implemented");
        }

        InputAction::Quit => {
            // Handled in main event handler
        }
    }
}

/// Render a frame
fn render_frame(
    window: &mut AppWindow,
    state: &mut AppState,
    renderer: &mut GlRenderer,
    ui: &mut UiRenderer,
) -> Result<()> {
    // 1. Run egui logic to get current toggle/slider values (no painting yet)
    let ui_output = ui.run_ui(&window.window);

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
        || state.render.display_options.show_power_markers != ui_output.show_power_markers
        || state.render.display_options.param_mode != ui_output.param_mode
        || state.render.display_options.param_filter_min != ui_output.param_filter_min
        || state.render.display_options.param_filter_max != ui_output.param_filter_max;

    state.render.display_options.show_slices = ui_output.show_slices;
    state.render.display_options.show_contours = ui_output.show_contours;
    state.render.display_options.show_hatches = ui_output.show_hatches;
    state.render.display_options.show_arrows = ui_output.show_arrows;
    state.render.display_options.show_power_markers = ui_output.show_power_markers;
    state.render.display_options.param_mode = ui_output.param_mode;
    state.render.display_options.param_filter_min = ui_output.param_filter_min;
    state.render.display_options.param_filter_max = ui_output.param_filter_max;

    // 2. Restore GL state first (egui leaves scissor test enabled), then clear
    renderer.begin_frame()?;

    let bg_color = state.render.display_options.background_color;
    renderer.clear(&bg_color)?;

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
