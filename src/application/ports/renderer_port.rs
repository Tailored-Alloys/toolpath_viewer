//! Renderer Port
//!
//! Interface for rendering operations.

use crate::domain::entities::{Layer, SliceStack, Vector};
use crate::domain::value_objects::{Bounds2D, Color, Point2D};
use thiserror::Error;

/// Errors that can occur during rendering
#[derive(Debug, Error)]
pub enum RenderError {
    #[error("OpenGL error: {0}")]
    OpenGLError(String),
    
    #[error("Shader compilation error: {0}")]
    ShaderError(String),
    
    #[error("Buffer allocation error: {0}")]
    BufferError(String),
    
    #[error("Invalid state: {0}")]
    InvalidState(String),
}

/// Result type for render operations
pub type RenderResult<T> = Result<T, RenderError>;

/// Display options for rendering
#[derive(Debug, Clone)]
pub struct DisplayOptions {
    /// Show slice boundaries
    pub show_slices: bool,
    /// Show manufacturing contours
    pub show_contours: bool,
    /// Show depth contours
    pub show_depth_contours: bool,
    /// Show hatch/infill lines
    pub show_hatches: bool,
    /// Show direction arrows
    pub show_arrows: bool,
    /// Show power markers (asterisks on vectors with power data)
    pub show_power_markers: bool,
    /// Parameter visualization mode
    pub param_mode: Option<ParameterMode>,
    /// Parameter filter range minimum
    pub param_filter_min: f32,
    /// Parameter filter range maximum
    pub param_filter_max: f32,
    /// Background color
    pub background_color: Color,
    /// Line width multiplier
    pub line_width: f32,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            show_slices: true,
            show_contours: true,
            show_depth_contours: true,
            show_hatches: true,
            show_arrows: false,
            show_power_markers: false,
            param_mode: None,
            param_filter_min: 0.0,
            param_filter_max: f32::MAX,
            background_color: Color::from_hex("#FAFAFA").unwrap_or(Color::WHITE),
            line_width: 1.5,
        }
    }
}

/// Parameter visualization mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterMode {
    Power,
    Speed,
    WaitTime,
}

/// Camera/view state
#[derive(Debug, Clone)]
pub struct ViewState {
    /// View center in world coordinates
    pub center: Point2D,
    /// Zoom level (pixels per unit)
    pub zoom: f32,
    /// Rotation angle in radians
    pub rotation: f32,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            center: Point2D::zero(),
            zoom: 1.0,
            rotation: 0.0,
        }
    }
}

impl ViewState {
    /// Fit the view to show the given bounds
    pub fn fit_to_bounds(&mut self, bounds: &Bounds2D, viewport_width: f32, viewport_height: f32) {
        self.center = bounds.center();
        
        let bounds_width = bounds.width();
        let bounds_height = bounds.height();
        
        if bounds_width > 0.0 && bounds_height > 0.0 {
            let zoom_x = viewport_width / bounds_width;
            let zoom_y = viewport_height / bounds_height;
            self.zoom = zoom_x.min(zoom_y) * 0.9; // 10% padding
            log::debug!(
                "fit_to_bounds: center=({:.3}, {:.3}), bounds_size=({:.3} x {:.3}), zoom={:.3}",
                self.center.x, self.center.y, bounds_width, bounds_height, self.zoom
            );
        }
    }
    
    /// Pan the view by a screen delta
    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.center.x -= dx / self.zoom;
        self.center.y -= dy / self.zoom;
    }
    
    /// Zoom at a specific point
    pub fn zoom_at(&mut self, factor: f32, screen_x: f32, screen_y: f32, viewport_width: f32, viewport_height: f32) {
        // Convert screen to world before zoom
        let world_x = self.center.x + (screen_x - viewport_width / 2.0) / self.zoom;
        let world_y = self.center.y + (screen_y - viewport_height / 2.0) / self.zoom;
        
        // Apply zoom
        self.zoom *= factor;
        self.zoom = self.zoom.clamp(0.001, 10000.0);
        
        // Adjust center to keep point under cursor stationary
        self.center.x = world_x - (screen_x - viewport_width / 2.0) / self.zoom;
        self.center.y = world_y - (screen_y - viewport_height / 2.0) / self.zoom;
    }
}

/// Port for rendering operations
pub trait Renderer: Send {
    /// Initialize the renderer with a viewport size
    fn initialize(&mut self, width: u32, height: u32) -> RenderResult<()>;
    
    /// Resize the viewport
    fn resize(&mut self, width: u32, height: u32) -> RenderResult<()>;
    
    /// Clear the framebuffer
    fn clear(&mut self, color: &Color) -> RenderResult<()>;
    
    /// Begin a new frame
    fn begin_frame(&mut self) -> RenderResult<()>;
    
    /// End the current frame
    fn end_frame(&mut self) -> RenderResult<()>;
    
    /// Render a layer
    fn render_layer(
        &mut self,
        layer: &Layer,
        view: &ViewState,
        options: &DisplayOptions,
    ) -> RenderResult<()>;
    
    /// Render a single vector
    fn render_vector(
        &mut self,
        vector: &Vector,
        color: &Color,
        line_width: f32,
    ) -> RenderResult<()>;
    
    /// Get current viewport dimensions
    fn viewport_size(&self) -> (u32, u32);
}

/// Port for batch rendering (optimization)
pub trait BatchRenderer: Renderer {
    /// Upload layer data to GPU buffers for efficient rendering
    fn upload_layer(&mut self, layer: &Layer) -> RenderResult<()>;
    
    /// Render a previously uploaded layer
    fn render_uploaded_layer(
        &mut self,
        layer_index: usize,
        view: &ViewState,
        options: &DisplayOptions,
    ) -> RenderResult<()>;
    
    /// Clear all uploaded data
    fn clear_uploaded(&mut self);
}
