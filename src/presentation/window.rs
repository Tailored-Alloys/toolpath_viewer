//! Window Management
//!
//! Handles window creation and event loop using glutin/winit.

use anyhow::Result;
use glutin::{
    config::{Config, ConfigTemplateBuilder, GlConfig},
    context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext},
    display::GetGlDisplay,
    prelude::*,
    surface::{Surface, SurfaceAttributesBuilder, WindowSurface},
};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasRawWindowHandle;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent, ElementState, MouseButton as WinitMouseButton, MouseScrollDelta},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowBuilder},
};
use std::num::NonZeroU32;
use std::sync::Arc;
use log::{debug, info};

use crate::presentation::{InputState, InputAction, key_to_action, MouseButton};

/// Window configuration
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub vsync: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Toolpath Viewer".to_string(),
            width: 1280,
            height: 720,
            vsync: true,
        }
    }
}

/// Application event for the event callback
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Window resize
    Resize { width: u32, height: u32 },
    /// Redraw requested
    Redraw,
    /// Forward to egui only (no rendering)
    EguiEvent,
    /// Mouse moved
    MouseMove { x: f32, y: f32 },
    /// Mouse button event
    MouseButton { button: MouseButton, pressed: bool },
    /// Mouse scroll
    Scroll { delta: f32 },
    /// Key press action
    KeyAction(InputAction),
    /// File dropped
    FileDropped(String),
    /// Close requested
    CloseRequested,
}

/// Main window wrapper
pub struct AppWindow {
    pub window: Window,
    pub gl_surface: Surface<WindowSurface>,
    pub gl_context: PossiblyCurrentContext,
    pub glow_context: Arc<glow::Context>,
    pub input_state: InputState,
}

impl AppWindow {
    /// Get current window size
    pub fn size(&self) -> (u32, u32) {
        let size = self.window.inner_size();
        (size.width, size.height)
    }

    /// Swap buffers
    pub fn swap_buffers(&self) -> Result<()> {
        self.gl_surface.swap_buffers(&self.gl_context)?;
        Ok(())
    }

    /// Request redraw
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Update window title
    pub fn set_title(&self, title: &str) {
        self.window.set_title(title);
    }
}

/// Create a window and OpenGL context
pub fn create_window(
    config: WindowConfig,
    event_loop: &EventLoop<()>,
) -> Result<AppWindow> {
    info!("Creating window: {}x{}", config.width, config.height);

    // Build the window
    let window_builder = WindowBuilder::new()
        .with_title(&config.title)
        .with_inner_size(LogicalSize::new(config.width, config.height))
        .with_resizable(true);

    // Configure OpenGL
    let template = ConfigTemplateBuilder::new()
        .with_alpha_size(8)
        .with_multisampling(4);

    let display_builder = DisplayBuilder::new().with_window_builder(Some(window_builder));

    // Create display and window
    let (window, gl_config) = display_builder
        .build(event_loop, template, |configs: Box<dyn Iterator<Item = Config> + '_>| {
            configs
                .reduce(|accum: Config, config: Config| {
                    if config.num_samples() > accum.num_samples() {
                        config
                    } else {
                        accum
                    }
                })
                .unwrap()
        })
        .map_err(|e| anyhow::anyhow!("Failed to build display: {}", e))?;

    let window = window.expect("Window should be created");
    let raw_window_handle = window.raw_window_handle();

    // Create GL context
    let context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(glutin::context::Version::new(3, 3))))
        .build(Some(raw_window_handle));

    let gl_display = gl_config.display();
    let not_current_context = unsafe {
        gl_display.create_context(&gl_config, &context_attributes)?
    };

    // Create surface
    let (width, height) = {
        let size = window.inner_size();
        (size.width, size.height)
    };

    let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        raw_window_handle,
        NonZeroU32::new(width).unwrap(),
        NonZeroU32::new(height).unwrap(),
    );

    let gl_surface = unsafe { gl_display.create_window_surface(&gl_config, &attrs)? };

    // Make context current
    let gl_context = not_current_context.make_current(&gl_surface)?;

    // Load GL function pointers
    gl::load_with(|symbol| {
        let symbol = std::ffi::CString::new(symbol).unwrap();
        gl_display.get_proc_address(symbol.as_c_str()).cast()
    });

    // Create glow context for egui
    let glow_context = unsafe {
        glow::Context::from_loader_function(|s| {
            let symbol = std::ffi::CString::new(s).unwrap();
            gl_display.get_proc_address(symbol.as_c_str()).cast()
        })
    };
    let glow_context = Arc::new(glow_context);

    // Log GL info
    unsafe {
        let version = std::ffi::CStr::from_ptr(gl::GetString(gl::VERSION) as *const _);
        let renderer = std::ffi::CStr::from_ptr(gl::GetString(gl::RENDERER) as *const _);
        info!("OpenGL version: {:?}", version);
        info!("OpenGL renderer: {:?}", renderer);
    }

    // Configure vsync
    if config.vsync {
        gl_surface.set_swap_interval(&gl_context, glutin::surface::SwapInterval::Wait(NonZeroU32::new(1).unwrap()))?;
    }

    Ok(AppWindow {
        window,
        gl_surface,
        gl_context,
        glow_context,
        input_state: InputState::new(),
    })
}

/// Run the event loop
pub fn run_event_loop<F>(
    event_loop: EventLoop<()>,
    mut app_window: AppWindow,
    mut event_handler: F,
) -> Result<()>
where
    F: FnMut(&mut AppWindow, AppEvent, Option<&WindowEvent>) -> bool + 'static,
{
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent { ref event, .. } => {
                // Clone the raw event for passing to handler
                let raw_event = Some(event);
                
                match event {
                    WindowEvent::CloseRequested => {
                        if event_handler(&mut app_window, AppEvent::CloseRequested, raw_event) {
                            elwt.exit();
                        }
                    }

                    WindowEvent::Resized(ref size) => {
                        if size.width > 0 && size.height > 0 {
                            app_window.gl_surface.resize(
                                &app_window.gl_context,
                                NonZeroU32::new(size.width).unwrap(),
                                NonZeroU32::new(size.height).unwrap(),
                            );
                            event_handler(&mut app_window, AppEvent::Resize {
                                width: size.width,
                                height: size.height,
                            }, raw_event);
                        }
                    }

                    WindowEvent::RedrawRequested => {
                        event_handler(&mut app_window, AppEvent::Redraw, raw_event);
                    }

                    WindowEvent::CursorMoved { ref position, .. } => {
                        app_window.input_state.update_mouse(
                            position.x as f32,
                            position.y as f32,
                        );
                        event_handler(&mut app_window, AppEvent::MouseMove {
                            x: position.x as f32,
                            y: position.y as f32,
                        }, raw_event);
                    }

                    WindowEvent::MouseInput { ref state, ref button, .. } => {
                        let pressed = *state == ElementState::Pressed;
                        let btn = match button {
                            WinitMouseButton::Left => {
                                app_window.input_state.left_pressed = pressed;
                                MouseButton::Left
                            }
                            WinitMouseButton::Middle => {
                                app_window.input_state.middle_pressed = pressed;
                                MouseButton::Middle
                            }
                            WinitMouseButton::Right => {
                                app_window.input_state.right_pressed = pressed;
                                MouseButton::Right
                            }
                            _ => return,
                        };
                        event_handler(&mut app_window, AppEvent::MouseButton {
                            button: btn,
                            pressed,
                        }, raw_event);
                    }

                    WindowEvent::MouseWheel { ref delta, .. } => {
                        let scroll = match delta {
                            MouseScrollDelta::LineDelta(_, y) => *y,
                            MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 100.0,
                        };
                        event_handler(&mut app_window, AppEvent::Scroll { delta: scroll }, raw_event);
                    }

                    WindowEvent::KeyboardInput { ref event, .. } => {
                        if event.state == ElementState::Pressed {
                            let ctrl = app_window.input_state.ctrl_held;
                            let shift = app_window.input_state.shift_held;
                            
                            let key_str = match &event.logical_key {
                                Key::Named(NamedKey::ArrowUp) => "up",
                                Key::Named(NamedKey::ArrowDown) => "down",
                                Key::Named(NamedKey::ArrowLeft) => "left",
                                Key::Named(NamedKey::ArrowRight) => "right",
                                Key::Named(NamedKey::PageUp) => "pageup",
                                Key::Named(NamedKey::PageDown) => "pagedown",
                                Key::Named(NamedKey::Home) => "home",
                                Key::Named(NamedKey::End) => "end",
                                Key::Named(NamedKey::Escape) => "escape",
                                Key::Character(c) => c.as_str(),
                                _ => "",
                            };

                            if let Some(action) = key_to_action(key_str, ctrl, shift) {
                                event_handler(&mut app_window, AppEvent::KeyAction(action), raw_event);
                            }
                        }
                    }

                    WindowEvent::ModifiersChanged(ref modifiers) => {
                        let state = modifiers.state();
                        app_window.input_state.ctrl_held = state.control_key();
                        app_window.input_state.shift_held = state.shift_key();
                        app_window.input_state.alt_held = state.alt_key();
                    }

                    WindowEvent::DroppedFile(ref path) => {
                        if let Some(path_str) = path.to_str() {
                            event_handler(&mut app_window, AppEvent::FileDropped(path_str.to_string()), raw_event);
                        }
                    }

                    _ => {
                        // Forward other events to egui for state tracking only
                        event_handler(&mut app_window, AppEvent::EguiEvent, raw_event);
                    }
                }
            }

            Event::AboutToWait => {
                // Request redraw for continuous rendering if needed
                // app_window.window.request_redraw();
            }

            _ => {}
        }
    })?;

    Ok(())
}
