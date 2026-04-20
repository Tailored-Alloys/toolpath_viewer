//! Renderer Port
//!
//! Interface for rendering operations.

use crate::domain::entities::{Layer, Vector};
use crate::domain::value_objects::{Bounds2D, Color, Point2D};
use thiserror::Error;

/// Errors that can occur during rendering
#[derive(Debug, Error)]
pub enum RenderError {
    #[error("Shader compilation error: {0}")]
    ShaderError(String),
    
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
    /// Parameter visualization mode
    pub param_mode: Option<ParameterMode>,
    /// Parameter filter range minimum
    pub param_filter_min: f32,
    /// Parameter filter range maximum
    pub param_filter_max: f32,
    /// Wait time range minimum (for marker coloring)
    pub wait_time_min: f32,
    /// Wait time range maximum (for marker coloring)
    pub wait_time_max: f32,
    /// Show wait time markers (asterisks)
    pub show_wait_markers: bool,
    /// Show background grid
    pub show_grid: bool,
    /// Grid unit for display
    pub grid_unit: GridUnit,
    /// Background color
    pub background_color: Color,
    /// Line width multiplier
    pub line_width: f32,
    /// Boundary line width multiplier (relative to base line_width)
    pub boundary_width_multiplier: f32,
    /// Contour line width multiplier (relative to base line_width)
    pub contour_width_multiplier: f32,
    /// Hatch line width multiplier (relative to base line_width)
    pub hatch_width_multiplier: f32,
    /// Arrow size multiplier (scales arrow arm length)
    pub arrow_size_multiplier: f32,
    /// Wait marker (dot) size multiplier
    pub wait_marker_size_multiplier: f32,
    /// Alpha for future (not-yet-drawn) vectors in playback
    pub future_vector_alpha: f32,
    /// Show directional gradient (fade from start to end of vector)
    pub show_direction_gradient: bool,
    /// Minor grid line width
    pub grid_line_width_minor: f32,
    /// Major grid line width
    pub grid_line_width_major: f32,
    /// Grid opacity multiplier (0.0–1.0)
    pub grid_opacity: f32,
    /// Anti-aliasing (line smoothing)
    pub antialiasing: bool,
    /// Maximum vector index to render (None = all vectors, Some(n) = 0..=n)
    pub max_vector_index: Option<usize>,
    /// Minor grid line color
    pub grid_minor_color: Color,
    /// Major grid line color
    pub grid_major_color: Color,
    /// Override color for all vectors (used in "color by file" mode)
    pub file_color_override: Option<Color>,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            show_slices: true,
            show_contours: true,
            show_depth_contours: true,
            show_hatches: true,
            show_arrows: false,
            param_mode: None,
            param_filter_min: 0.0,
            param_filter_max: f32::MAX,
            wait_time_min: 0.0,
            wait_time_max: f32::MAX,
            show_wait_markers: false,
            show_grid: true,
            grid_unit: GridUnit::Millimeters,
            background_color: Color::from_hex("#FAFAFA").unwrap_or(Color::WHITE),
            line_width: 1.5,
            boundary_width_multiplier: 1.5,
            contour_width_multiplier: 1.0,
            hatch_width_multiplier: 0.8,
            arrow_size_multiplier: 1.0,
            wait_marker_size_multiplier: 1.0,
            future_vector_alpha: 0.15,
            show_direction_gradient: true,
            grid_line_width_minor: 1.0,
            grid_line_width_major: 1.5,
            grid_opacity: 1.0,
            antialiasing: true,
            max_vector_index: None,
            grid_minor_color: Color::new(0.0, 0.0, 0.0, 0.08),
            grid_major_color: Color::new(0.0, 0.0, 0.0, 0.20),
            file_color_override: None,
        }
    }
}

/// Parameter visualization mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterMode {
    Power,
    Speed,
}

/// Color mode for the viewport
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    /// Color vectors by their type (contour, hatch, boundary, etc.)
    ByVectorType,
    /// Color all vectors in a file with that file's assigned color
    ByFile,
    /// Color vectors by a parameter value using a gradient
    ByParameter(ParameterMode),
}

/// View mode controlling how multiple files are displayed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// All visible files rendered on the same viewport (default)
    #[default]
    Overlay,
    /// Show one file at a time, switchable via tabs
    Tab,
    /// Side-by-side split showing two files simultaneously
    Split,
}

/// Grid unit for measurement display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridUnit {
    Millimeters,
    Micrometers,
    Inches,
}

impl GridUnit {
    /// Display label for the unit
    pub fn label(&self) -> &'static str {
        match self {
            GridUnit::Millimeters => "mm",
            GridUnit::Micrometers => "µm",
            GridUnit::Inches => "in",
        }
    }

    /// Conversion factor from mm to this unit
    pub fn from_mm(&self, mm: f32) -> f32 {
        match self {
            GridUnit::Millimeters => mm,
            GridUnit::Micrometers => mm * 1000.0,
            GridUnit::Inches => mm / 25.4,
        }
    }

    /// Speed label derived from this length unit
    pub fn speed_label(&self) -> &'static str {
        match self {
            GridUnit::Millimeters => "mm/s",
            GridUnit::Micrometers => "µm/s",
            GridUnit::Inches => "in/s",
        }
    }

    /// Convert speed value from mm/s to this unit per second
    pub fn speed_from_mm_per_s(&self, mm_per_s: f32) -> f32 {
        self.from_mm(mm_per_s)
    }
}

/// Time unit for display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    Microseconds,
    Milliseconds,
    Seconds,
}

impl TimeUnit {
    /// Display label
    pub fn label(&self) -> &'static str {
        match self {
            TimeUnit::Microseconds => "µs",
            TimeUnit::Milliseconds => "ms",
            TimeUnit::Seconds => "s",
        }
    }

    /// Convert from microseconds (file native) to this unit
    pub fn from_us(&self, us: f32) -> f32 {
        match self {
            TimeUnit::Microseconds => us,
            TimeUnit::Milliseconds => us / 1000.0,
            TimeUnit::Seconds => us / 1_000_000.0,
        }
    }
}

/// Power unit for display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerUnit {
    Watts,
    Kilowatts,
}

impl PowerUnit {
    /// Display label
    pub fn label(&self) -> &'static str {
        match self {
            PowerUnit::Watts => "W",
            PowerUnit::Kilowatts => "kW",
        }
    }

    /// Convert from watts (file native) to this unit
    pub fn from_watts(&self, w: f32) -> f32 {
        match self {
            PowerUnit::Watts => w,
            PowerUnit::Kilowatts => w / 1000.0,
        }
    }
}

/// Global unit settings for the application
#[derive(Debug, Clone)]
pub struct GlobalUnits {
    pub length: GridUnit,
    pub time: TimeUnit,
    pub power: PowerUnit,
}

impl Default for GlobalUnits {
    fn default() -> Self {
        Self {
            length: GridUnit::Millimeters,
            time: TimeUnit::Microseconds,
            power: PowerUnit::Watts,
        }
    }
}

impl GlobalUnits {
    /// Get the display label for a parameter mode value, using current units
    pub fn param_label(&self, mode: ParameterMode) -> String {
        match mode {
            ParameterMode::Power => format!("Power ({})", self.power.label()),
            ParameterMode::Speed => format!("Speed ({})", self.length.speed_label()),
        }
    }

    /// Get the unit suffix for a parameter mode
    pub fn param_suffix(&self, mode: ParameterMode) -> String {
        match mode {
            ParameterMode::Power => format!(" {}", self.power.label()),
            ParameterMode::Speed => format!(" {}", self.length.speed_label()),
        }
    }

    /// Convert a raw parameter value from file-native units to display units
    pub fn convert_param(&self, mode: ParameterMode, raw: f32) -> f32 {
        match mode {
            ParameterMode::Power => self.power.from_watts(raw),
            ParameterMode::Speed => self.length.speed_from_mm_per_s(raw),
        }
    }
}

/// Camera/view state
#[derive(Debug, Clone)]
pub struct ViewState {
    /// View center in world coordinates
    pub center: Point2D,
    /// Zoom level (pixels per unit)
    pub zoom: f32,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            center: Point2D::zero(),
            zoom: 1.0,
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

    /// Convert screen coordinates to world coordinates.
    /// `screen_x`, `screen_y` are in viewport space (0,0 = top-left of render area).
    /// Y is flipped: viewport top = 0, increasing downward.
    pub fn screen_to_world(&self, screen_x: f32, screen_y: f32, viewport_width: f32, viewport_height: f32) -> Point2D {
        let world_x = self.center.x + (screen_x - viewport_width / 2.0) / self.zoom;
        let world_y = self.center.y + (viewport_height / 2.0 - screen_y) / self.zoom;
        Point2D::new(world_x, world_y)
    }

    /// Convert world coordinates to screen coordinates.
    /// Returns viewport-space coords (0,0 = top-left of render area).
    pub fn world_to_screen(&self, world_x: f32, world_y: f32, viewport_width: f32, viewport_height: f32) -> Point2D {
        let screen_x = (world_x - self.center.x) * self.zoom + viewport_width / 2.0;
        let screen_y = viewport_height / 2.0 - (world_y - self.center.y) * self.zoom;
        Point2D::new(screen_x, screen_y)
    }

    /// Get the visible world-coordinate bounds for the current view.
    pub fn visible_bounds(&self, viewport_width: f32, viewport_height: f32) -> (Point2D, Point2D) {
        let half_w = viewport_width / (2.0 * self.zoom);
        let half_h = viewport_height / (2.0 * self.zoom);
        (
            Point2D::new(self.center.x - half_w, self.center.y - half_h),
            Point2D::new(self.center.x + half_w, self.center.y + half_h),
        )
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
