//! Render View Use Case
//!
//! Orchestrates rendering of the toolpath visualization.

use crate::application::ports::{DisplayOptions, RenderResult, Renderer, ViewState};
use crate::domain::entities::{Layer, SliceStack, VectorType};
use crate::domain::value_objects::{Bounds2D, Color};

/// Color scheme for rendering different vector types
#[derive(Debug, Clone)]
pub struct ColorScheme {
    pub boundary: Color,
    pub contour: Color,
    pub base_contour: Color,
    pub depth_contour: Color,
    pub hatch: Color,
    pub support: Color,
    pub travel: Color,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            boundary: Color::from_hex("#2E7D32").unwrap_or(Color::GREEN),
            contour: Color::from_hex("#1565C0").unwrap_or(Color::BLUE),
            base_contour: Color::from_hex("#00695C").unwrap_or(Color::CYAN),
            depth_contour: Color::from_hex("#AD1457").unwrap_or(Color::MAGENTA),
            hatch: Color::from_hex("#BF360C").unwrap_or(Color::new(0.9, 0.3, 0.0, 1.0)),
            support: Color::from_hex("#616161").unwrap_or(Color::GRAY),
            travel: Color::from_hex("#388E3C").unwrap_or(Color::GREEN).with_alpha(0.4),
        }
    }
}

impl ColorScheme {
    /// Get color for a vector type
    pub fn color_for_type(&self, vector_type: VectorType) -> &Color {
        match vector_type {
            VectorType::Boundary => &self.boundary,
            VectorType::Contour => &self.contour,
            VectorType::BaseContour => &self.base_contour,
            VectorType::DepthContour => &self.depth_contour,
            VectorType::CoincidingContour => &self.contour,
            VectorType::Hatch => &self.hatch,
            VectorType::Support => &self.support,
            VectorType::Travel => &self.travel,
        }
    }
}

/// Use case for rendering the visualization
pub struct RenderViewUseCase {
    pub view_state: ViewState,
    pub display_options: DisplayOptions,
    pub color_scheme: ColorScheme,
}

impl RenderViewUseCase {
    /// Create a new render use case
    pub fn new() -> Self {
        Self {
            view_state: ViewState::default(),
            display_options: DisplayOptions::default(),
            color_scheme: ColorScheme::default(),
        }
    }

    /// Fit view to show entire slice stack
    pub fn fit_to_stack(&mut self, stack: &SliceStack, viewport_width: f32, viewport_height: f32) {
        if let Some((min, max)) = stack.bounds() {
            let bounds = Bounds2D::new(min, max);
            self.view_state.fit_to_bounds(&bounds, viewport_width, viewport_height);
        }
    }

    /// Fit view to show a specific layer
    pub fn fit_to_layer(&mut self, layer: &Layer, viewport_width: f32, viewport_height: f32) {
        if let Some((min, max)) = layer.bounds() {
            let bounds = Bounds2D::new(min, max);
            self.view_state.fit_to_bounds(&bounds, viewport_width, viewport_height);
        }
    }

    /// Render a layer using the given renderer
    pub fn render_layer<R: Renderer>(
        &self,
        renderer: &mut R,
        layer: &Layer,
    ) -> RenderResult<()> {
        renderer.render_layer(layer, &self.view_state, &self.display_options)
    }

    /// Render a single vector with appropriate color
    pub fn render_vector<R: Renderer>(
        &self,
        renderer: &mut R,
        vector: &crate::domain::entities::Vector,
    ) -> RenderResult<()> {
        // Check visibility based on vector type
        let visible = match vector.vector_type {
            VectorType::Boundary => self.display_options.show_slices,
            VectorType::Contour | VectorType::BaseContour | VectorType::CoincidingContour => {
                self.display_options.show_contours
            }
            VectorType::DepthContour => self.display_options.show_depth_contours,
            VectorType::Hatch => self.display_options.show_hatches,
            VectorType::Support | VectorType::Travel => true,
        };

        if !visible {
            return Ok(());
        }

        let color = self.color_scheme.color_for_type(vector.vector_type);
        renderer.render_vector(vector, color, self.display_options.line_width)
    }

    /// Handle pan gesture
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        self.view_state.pan(delta_x, delta_y);
    }

    /// Handle zoom gesture
    pub fn zoom(
        &mut self,
        factor: f32,
        center_x: f32,
        center_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        self.view_state.zoom_at(factor, center_x, center_y, viewport_width, viewport_height);
    }

    /// Reset view to defaults
    pub fn reset_view(&mut self) {
        self.view_state = ViewState::default();
    }

    /// Toggle a display option
    pub fn toggle_option(&mut self, option: DisplayOption) {
        match option {
            DisplayOption::Slices => self.display_options.show_slices = !self.display_options.show_slices,
            DisplayOption::Contours => self.display_options.show_contours = !self.display_options.show_contours,
            DisplayOption::DepthContours => self.display_options.show_depth_contours = !self.display_options.show_depth_contours,
            DisplayOption::Hatches => self.display_options.show_hatches = !self.display_options.show_hatches,
            DisplayOption::Arrows => self.display_options.show_arrows = !self.display_options.show_arrows,
        }
    }
}

impl Default for RenderViewUseCase {
    fn default() -> Self {
        Self::new()
    }
}

/// Display options that can be toggled
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayOption {
    Slices,
    Contours,
    DepthContours,
    Hatches,
    Arrows,
}
