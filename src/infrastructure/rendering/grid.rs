//! Grid Renderer
//!
//! Renders adaptive background gridlines using OpenGL.
//! Grid spacing adapts to the current zoom level, showing
//! major and minor gridlines with different opacities.

use crate::application::ports::ViewState;
use crate::domain::value_objects::{Color, Point2D};
use crate::infrastructure::rendering::LineBatch;

/// Grid renderer that draws adaptive gridlines behind content.
pub struct GridRenderer {
    major_batch: LineBatch,
    minor_batch: LineBatch,
}

/// Pick a "nice" grid spacing in mm for the given zoom level.
/// Returns (minor_spacing, major_spacing) in world units (mm).
fn pick_grid_spacing(zoom: f32) -> (f32, f32) {
    // Target: minor gridlines ~20-80 screen pixels apart.
    // screen_pixels = world_mm * zoom
    // world_mm = target_px / zoom
    let target_minor_px = 40.0;
    let ideal_minor = target_minor_px / zoom;

    // Round to a "nice" value: 0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1, 2, 5, 10, 20, 50, 100, ...
    let nice_steps: &[f32] = &[0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0];
    let minor = nice_steps
        .iter()
        .copied()
        .find(|&s| s >= ideal_minor)
        .unwrap_or(1000.0);

    // Major gridlines every 5 minor steps (or 10 if minor is a "2" step)
    let major = minor * 5.0;

    (minor, major)
}

impl GridRenderer {
    pub fn new() -> Self {
        Self {
            major_batch: LineBatch::new(),
            minor_batch: LineBatch::new(),
        }
    }

    /// Prepare grid geometry for the current view.
    pub fn prepare(
        &mut self,
        view: &ViewState,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        self.prepare_with_colors(
            view,
            viewport_width,
            viewport_height,
            Color::new(0.0, 0.0, 0.0, 0.08),
            Color::new(0.0, 0.0, 0.0, 0.20),
        );
    }

    /// Prepare grid geometry with custom minor/major colors.
    pub fn prepare_with_colors(
        &mut self,
        view: &ViewState,
        viewport_width: f32,
        viewport_height: f32,
        minor_color: Color,
        major_color: Color,
    ) {
        self.major_batch.clear();
        self.minor_batch.clear();

        let (minor_spacing, major_spacing) = pick_grid_spacing(view.zoom);
        let (vis_min, vis_max) = view.visible_bounds(viewport_width, viewport_height);

        // Slight padding so lines extend past the visible area
        let pad = major_spacing;
        let x_start = ((vis_min.x - pad) / minor_spacing).floor() * minor_spacing;
        let x_end = ((vis_max.x + pad) / minor_spacing).ceil() * minor_spacing;
        let y_start = ((vis_min.y - pad) / minor_spacing).floor() * minor_spacing;
        let y_end = ((vis_max.y + pad) / minor_spacing).ceil() * minor_spacing;

        // Cap line count to prevent performance issues at extreme zoom
        let max_lines = 400;
        let x_count = ((x_end - x_start) / minor_spacing) as usize;
        let y_count = ((y_end - y_start) / minor_spacing) as usize;
        if x_count + y_count > max_lines {
            // Fall back to major lines only
            let x_start_m = ((vis_min.x - pad) / major_spacing).floor() * major_spacing;
            let x_end_m = ((vis_max.x + pad) / major_spacing).ceil() * major_spacing;
            let y_start_m = ((vis_min.y - pad) / major_spacing).floor() * major_spacing;
            let y_end_m = ((vis_max.y + pad) / major_spacing).ceil() * major_spacing;

            let mut x = x_start_m;
            while x <= x_end_m {
                let p0 = Point2D::new(x, vis_min.y - pad);
                let p1 = Point2D::new(x, vis_max.y + pad);
                self.major_batch.add_line(&p0, &p1, &major_color);
                x += major_spacing;
            }
            let mut y = y_start_m;
            while y <= y_end_m {
                let p0 = Point2D::new(vis_min.x - pad, y);
                let p1 = Point2D::new(vis_max.x + pad, y);
                self.major_batch.add_line(&p0, &p1, &major_color);
                y += major_spacing;
            }
        } else {
            // Vertical lines
            let mut x = x_start;
            while x <= x_end {
                let is_major = (x / major_spacing).round() * major_spacing == x
                    || (x % major_spacing).abs() < minor_spacing * 0.01;
                let p0 = Point2D::new(x, vis_min.y - pad);
                let p1 = Point2D::new(x, vis_max.y + pad);
                if is_major {
                    self.major_batch.add_line(&p0, &p1, &major_color);
                } else {
                    self.minor_batch.add_line(&p0, &p1, &minor_color);
                }
                x += minor_spacing;
            }

            // Horizontal lines
            let mut y = y_start;
            while y <= y_end {
                let is_major = (y / major_spacing).round() * major_spacing == y
                    || (y % major_spacing).abs() < minor_spacing * 0.01;
                let p0 = Point2D::new(vis_min.x - pad, y);
                let p1 = Point2D::new(vis_max.x + pad, y);
                if is_major {
                    self.major_batch.add_line(&p0, &p1, &major_color);
                } else {
                    self.minor_batch.add_line(&p0, &p1, &minor_color);
                }
                y += minor_spacing;
            }
        }

        self.minor_batch.set_line_width(1.0);
        self.major_batch.set_line_width(1.5);
    }

    /// Render the grid. Must be called after prepare() and with shader/projection already set.
    pub fn render(&mut self) {
        self.minor_batch.render();
        self.major_batch.render();
    }

    /// Get the current grid spacing in world units (minor, major).
    /// Call after prepare() for accurate values.
    pub fn spacing(zoom: f32) -> (f32, f32) {
        pick_grid_spacing(zoom)
    }
}

impl Default for GridRenderer {
    fn default() -> Self {
        Self::new()
    }
}
