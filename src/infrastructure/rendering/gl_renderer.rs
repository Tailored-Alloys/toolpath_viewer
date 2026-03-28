//! OpenGL Renderer Implementation
//!
//! Implements the Renderer port using OpenGL.

use crate::application::ports::{DisplayOptions, ParameterMode, RenderError, RenderResult, Renderer, ViewState};
use crate::application::use_cases::ColorScheme;
use crate::domain::entities::{Layer, Vector, VectorType};
use crate::domain::value_objects::{Color, Point2D};
use crate::infrastructure::rendering::{
    GridRenderer, LineBatch, ShaderProgram,
    DEFAULT_FRAGMENT_SHADER, DEFAULT_VERTEX_SHADER,
};
use log::{debug, info};

/// Create an orthographic projection matrix
fn ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> [f32; 16] {
    let tx = -(right + left) / (right - left);
    let ty = -(top + bottom) / (top - bottom);
    let tz = -(far + near) / (far - near);

    [
        2.0 / (right - left), 0.0, 0.0, 0.0,
        0.0, 2.0 / (top - bottom), 0.0, 0.0,
        0.0, 0.0, -2.0 / (far - near), 0.0,
        tx, ty, tz, 1.0,
    ]
}

/// Create a view matrix from view state
fn view_matrix(view: &ViewState) -> [f32; 16] {
    let scale = view.zoom;
    let tx = -view.center.x * scale;
    let ty = -view.center.y * scale;

    [
        scale, 0.0, 0.0, 0.0,
        0.0, scale, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        tx, ty, 0.0, 1.0,
    ]
}

/// OpenGL 2D renderer
///
/// Arrow angle half-width in radians (~25°).
const ARROW_HALF_ANGLE: f32 = 0.4363; // std::f32::consts is not const-fn friendly, pre-computed

/// Minimum arrow arm length (world units, mm) — prevents invisible arrows on very short vectors.
const ARROW_ARM_MIN: f32 = 0.02;
/// Maximum arrow arm length (world units, mm) — prevents oversized arrows on long vectors.
const ARROW_ARM_MAX: f32 = 0.15;

/// Minimum star marker radius (world units, mm) — prevents invisible wait time markers.
const STAR_RADIUS_MIN: f32 = 0.015;
/// Maximum star marker radius (world units, mm) — prevents oversized wait time markers.
const STAR_RADIUS_MAX: f32 = 0.12;

/// Generate per-vertex gradient colors for a polyline, fading from transparent
/// at the start to the full color at the end to indicate scan direction.
fn progress_gradient(points: &[Point2D], color: &Color) -> Vec<Color> {
    let n = points.len();
    if n < 2 {
        return vec![*color; n];
    }
    (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1) as f32; // 0.0 at start, 1.0 at end
            let alpha = color.a * (0.3 + 0.7 * t); // fade from 30% to 100%
            color.with_alpha(alpha)
        })
        .collect()
}

/// Add an arrowhead to `batch` at `tip` pointing in direction (`dx`, `dy`).
/// `arm_len` controls the size of the arrow arms.
fn add_arrowhead(batch: &mut LineBatch, tip: &Point2D, dx: f32, dy: f32, arm_len: f32, color: &Color) {
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-12 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;

    // Back direction
    let bx = -ux;
    let by = -uy;

    let cos_a = ARROW_HALF_ANGLE.cos();
    let sin_a = ARROW_HALF_ANGLE.sin();

    // Left arm
    let lx = bx * cos_a - by * sin_a;
    let ly = bx * sin_a + by * cos_a;
    let left = Point2D::new(tip.x + arm_len * lx, tip.y + arm_len * ly);

    // Right arm
    let rx = bx * cos_a + by * sin_a;
    let ry = -bx * sin_a + by * cos_a;
    let right = Point2D::new(tip.x + arm_len * rx, tip.y + arm_len * ry);

    batch.add_line(tip, &left, color);
    batch.add_line(tip, &right, color);
}

/// Add a star/asterisk marker (*) to `batch` centered at `center`.
/// Draws 3 crossing line segments (|, /, \) of half-length `r`.
fn add_star_marker(batch: &mut LineBatch, center: &Point2D, r: f32, color: &Color) {
    // Vertical line |
    let top = Point2D::new(center.x, center.y + r);
    let bot = Point2D::new(center.x, center.y - r);
    batch.add_line(&top, &bot, color);

    // 60° line /
    let cos60: f32 = 0.5;
    let sin60: f32 = 0.866_025_4;
    let a = Point2D::new(center.x + r * cos60, center.y + r * sin60);
    let b = Point2D::new(center.x - r * cos60, center.y - r * sin60);
    batch.add_line(&a, &b, color);

    // 120° line \
    let c = Point2D::new(center.x - r * cos60, center.y + r * sin60);
    let d = Point2D::new(center.x + r * cos60, center.y - r * sin60);
    batch.add_line(&c, &d, color);
}

/// Compute marker size from layer bounds (0.5% of bounding diagonal).
fn compute_marker_size(layer: &Layer) -> f32 {
    if let Some((min_pt, max_pt)) = layer.bounds() {
        let dx = max_pt.x - min_pt.x;
        let dy = max_pt.y - min_pt.y;
        let diag = (dx * dx + dy * dy).sqrt();
        (diag * 0.005).max(0.1)
    } else {
        1.0 // fallback
    }
}

pub struct GlRenderer {
    width: u32,
    height: u32,
    /// DPI scale factor (physical pixels per logical pixel)
    scale_factor: f32,
    /// Horizontal offset for centering (to account for UI panel)
    view_offset_x: f32,
    /// Vertical offset for centering (to account for top toolbar)
    view_offset_y: f32,
    shader: Option<ShaderProgram>,
    contour_batch: LineBatch,
    boundary_batch: LineBatch,
    hatch_batch: LineBatch,
    arrow_batch: LineBatch,
    wait_marker_batch: LineBatch,
    /// Whether to render wait markers
    show_wait_markers: bool,
    /// Grid renderer
    grid_renderer: GridRenderer,
    color_scheme: ColorScheme,
    initialized: bool,
    /// Track last hatch count to reduce log spam
    last_hatch_count: usize,
    /// Whether we've logged shader/uniform diagnostics
    logged_once: bool,
}

impl GlRenderer {
    /// Create a new OpenGL renderer
    pub fn new() -> Self {
        Self {
            width: 800,
            height: 600,
            scale_factor: 1.0,
            view_offset_x: 0.0,
            view_offset_y: 0.0,
            shader: None,
            contour_batch: LineBatch::new(),
            boundary_batch: LineBatch::new(),
            hatch_batch: LineBatch::new(),
            arrow_batch: LineBatch::new(),
            wait_marker_batch: LineBatch::new(),
            show_wait_markers: true,
            grid_renderer: GridRenderer::new(),
            color_scheme: ColorScheme::default(),
            initialized: false,
            last_hatch_count: usize::MAX,
            logged_once: false,
        }
    }

    /// Set color scheme
    pub fn set_color_scheme(&mut self, scheme: ColorScheme) {
        self.color_scheme = scheme;
    }

    /// Set horizontal view offset (to shift content away from UI panel)
    pub fn set_view_offset_x(&mut self, offset: f32) {
        self.view_offset_x = offset;
    }

    /// Set vertical view offset (to shift content away from top toolbar)
    pub fn set_view_offset_y(&mut self, offset: f32) {
        self.view_offset_y = offset;
    }

    /// Set DPI scale factor for correct projection on HiDPI displays
    pub fn set_scale_factor(&mut self, factor: f32) {
        self.scale_factor = factor;
    }

    /// Render the background grid before layer content.
    pub fn render_grid(&mut self, view: &ViewState) -> RenderResult<()> {
        let shader = self.shader.as_ref().ok_or_else(|| {
            RenderError::InvalidState("Shader not initialized".to_string())
        })?;

        shader.use_program();

        // Use logical pixels for projection (physical / scale_factor)
        // gl::Viewport remains in physical pixels; projection defines the coordinate system
        let logical_w = self.width as f32 / self.scale_factor;
        let logical_h = self.height as f32 / self.scale_factor;
        let half_w = logical_w / 2.0;
        let half_h = logical_h / 2.0;
        let off_x = self.view_offset_x;
        let off_y = self.view_offset_y;
        let projection = ortho(-half_w + off_x, half_w + off_x, -half_h + off_y, half_h + off_y, -1.0, 1.0);
        shader.set_mat4("uProjection", &projection);
        let view_mat = view_matrix(view);
        shader.set_mat4("uView", &view_mat);

        let viewport_w = logical_w;
        let viewport_h = logical_h;
        self.grid_renderer.prepare(view, viewport_w, viewport_h);
        self.grid_renderer.render();

        Ok(())
    }

    /// Capture the current framebuffer as RGBA pixels.
    /// Returns (width, height, rgba_data).
    pub fn capture_framebuffer(&self) -> (u32, u32, Vec<u8>) {
        let w = self.width;
        let h = self.height;
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        unsafe {
            gl::ReadPixels(
                0, 0, w as i32, h as i32,
                gl::RGBA, gl::UNSIGNED_BYTE,
                pixels.as_mut_ptr() as *mut _,
            );
        }
        // Flip vertically (OpenGL origin is bottom-left)
        let row_size = (w * 4) as usize;
        let mut flipped = vec![0u8; pixels.len()];
        for y in 0..h as usize {
            let src_row = (h as usize - 1 - y) * row_size;
            let dst_row = y * row_size;
            flipped[dst_row..dst_row + row_size].copy_from_slice(&pixels[src_row..src_row + row_size]);
        }
        (w, h, flipped)
    }

    /// Initialize OpenGL state
    fn init_gl(&mut self) -> RenderResult<()> {
        unsafe {
            // Enable multisampling
            gl::Enable(gl::MULTISAMPLE);
            
            // Enable blending
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            
            // Line settings
            gl::Enable(gl::LINE_SMOOTH);
            gl::Hint(gl::LINE_SMOOTH_HINT, gl::NICEST);
        }

        // Create shader program
        self.shader = Some(
            ShaderProgram::new(DEFAULT_VERTEX_SHADER, DEFAULT_FRAGMENT_SHADER)
                .map_err(|e| RenderError::ShaderError(e))?
        );

        self.initialized = true;
        info!("OpenGL renderer initialized");
        
        Ok(())
    }

    /// Prepare a layer for rendering (populate batches)
    pub fn prepare_layer(&mut self, layer: &Layer, options: &DisplayOptions) {
        self.contour_batch.clear();
        self.boundary_batch.clear();
        self.hatch_batch.clear();
        self.arrow_batch.clear();
        self.wait_marker_batch.clear();
        self.show_wait_markers = options.show_wait_markers;

        let dim_color = Color::rgb(0.3, 0.3, 0.3);

        let marker_size = compute_marker_size(layer);
        let arrow_arm = (marker_size * 0.8).clamp(ARROW_ARM_MIN, ARROW_ARM_MAX);
        let star_radius = (marker_size * 0.6).clamp(STAR_RADIUS_MIN, STAR_RADIUS_MAX);
        // Place arrows every `arrow_spacing` world-units along polylines
        let arrow_spacing = marker_size * 8.0;

        // Limit vectors when vector-by-vector view is active
        let vector_limit = match options.max_vector_index {
            Some(max_idx) => (max_idx + 1).min(layer.vectors.len()),
            None => layer.vectors.len(),
        };

        for (vec_idx, vector) in layer.vectors.iter().enumerate() {
            let is_active = vec_idx < vector_limit;
            // Determine color: parameter gradient when a mode is active, else type-based
            let color = if let Some(param_mode) = options.param_mode {
                let param_value = match param_mode {
                    ParameterMode::Power => vector.parameters.power,
                    ParameterMode::Speed => vector.parameters.speed,
                    ParameterMode::WaitTime => vector.parameters.wait_time,
                };
                match param_value {
                    Some(val) => {
                        if val < options.param_filter_min || val > options.param_filter_max {
                            continue; // filtered out
                        }
                        let range = options.param_filter_max - options.param_filter_min;
                        let t = if range > 0.0 {
                            (val - options.param_filter_min) / range
                        } else {
                            0.5
                        };
                        Color::viridis_gradient(t)
                    }
                    None => dim_color, // no param data, show dimmed
                }
            } else {
                *self.color_scheme.color_for_type(vector.vector_type)
            };

            // Mute future vectors when vector-by-vector view is active
            let color = if !is_active && options.max_vector_index.is_some() {
                color.with_alpha(0.15)
            } else {
                color
            };

            // Track whether this vector is visible (for arrow/marker placement)
            let mut visible = false;

            // Use gradient coloring for active vectors in vector view mode
            let use_gradient = is_active && options.max_vector_index.is_some();

            match vector.vector_type {
                VectorType::Boundary => {
                    if options.show_slices {
                        if use_gradient && vector.points.len() >= 2 {
                            let colors = progress_gradient(&vector.points, &color);
                            self.boundary_batch.add_polyline_colored(&vector.points, &colors);
                        } else {
                            self.boundary_batch.add_polyline(&vector.points, &color);
                        }
                        visible = true;
                    }
                }
                VectorType::Contour | VectorType::BaseContour | VectorType::CoincidingContour => {
                    if options.show_contours {
                        if use_gradient && vector.points.len() >= 2 {
                            let colors = progress_gradient(&vector.points, &color);
                            self.contour_batch.add_polyline_colored(&vector.points, &colors);
                        } else {
                            self.contour_batch.add_polyline(&vector.points, &color);
                        }
                        visible = true;
                    }
                }
                VectorType::DepthContour => {
                    if options.show_depth_contours {
                        if use_gradient && vector.points.len() >= 2 {
                            let colors = progress_gradient(&vector.points, &color);
                            self.contour_batch.add_polyline_colored(&vector.points, &colors);
                        } else {
                            self.contour_batch.add_polyline(&vector.points, &color);
                        }
                        visible = true;
                    }
                }
                VectorType::Hatch => {
                    if options.show_hatches && vector.points.len() >= 2 {
                        if use_gradient {
                            let start_color = color.with_alpha(color.a * 0.3);
                            self.hatch_batch.add_line_gradient(
                                &vector.points[0], &start_color,
                                &vector.points[1], &color,
                            );
                        } else {
                            self.hatch_batch.add_line(
                                &vector.points[0],
                                &vector.points[1],
                                &color,
                            );
                        }
                        visible = true;
                    }
                }
                VectorType::Support | VectorType::Travel => {
                    if use_gradient && vector.points.len() >= 2 {
                        let colors = progress_gradient(&vector.points, &color);
                        self.contour_batch.add_polyline_colored(&vector.points, &colors);
                    } else {
                        self.contour_batch.add_polyline(&vector.points, &color);
                    }
                    visible = true;
                }
            }

            if !visible {
                continue;
            }

            // --- Direction arrows (only for active vectors) ---
            if !is_active { continue; }
            if options.show_arrows && vector.points.len() >= 2 {
                match vector.vector_type {
                    VectorType::Hatch => {
                        // Arrow at the end of each hatch, size relative to hatch length
                        let p0 = &vector.points[0];
                        let p1 = &vector.points[1];
                        let dx = p1.x - p0.x;
                        let dy = p1.y - p0.y;
                        let hatch_len = (dx * dx + dy * dy).sqrt();
                        let hatch_arm = (hatch_len * 0.25).clamp(ARROW_ARM_MIN, ARROW_ARM_MAX);
                        add_arrowhead(&mut self.arrow_batch, p1, dx, dy, hatch_arm, &color);
                    }
                    VectorType::Contour | VectorType::BaseContour
                    | VectorType::CoincidingContour | VectorType::DepthContour
                    | VectorType::Boundary => {
                        // Place arrows at regular intervals along the polyline
                        let mut accum = arrow_spacing * 0.5; // start offset
                        for pair in vector.points.windows(2) {
                            let dx = pair[1].x - pair[0].x;
                            let dy = pair[1].y - pair[0].y;
                            let seg_len = (dx * dx + dy * dy).sqrt();
                            accum += seg_len;
                            if accum >= arrow_spacing {
                                accum -= arrow_spacing;
                                add_arrowhead(&mut self.arrow_batch, &pair[1], dx, dy, arrow_arm.clamp(ARROW_ARM_MIN, ARROW_ARM_MAX), &color);
                            }
                        }
                    }
                    _ => {}
                }
            }

            // --- Wait time markers (at END of vectors with wait_time) ---
            // Color based on wait time value using heat gradient
            if let Some(wait_val) = vector.parameters.wait_time {
                if vector.points.len() >= 2 {
                    // Compute normalized value for viridis gradient
                    let range = options.wait_time_max - options.wait_time_min;
                    let t = if range > 0.0 {
                        (wait_val - options.wait_time_min) / range
                    } else {
                        0.5
                    };
                    let wait_color = Color::viridis_gradient(t);
                    
                    match vector.vector_type {
                        VectorType::Hatch => {
                            // Star at endpoint (p1) where the laser waits
                            let p1 = &vector.points[1];
                            add_star_marker(&mut self.wait_marker_batch, p1, star_radius, &wait_color);
                        }
                        _ => {
                            // For polylines, star at the last point
                            if let Some(last) = vector.points.last() {
                                add_star_marker(&mut self.wait_marker_batch, last, star_radius, &wait_color);
                            }
                        }
                    }
                }
            }
        }

        self.contour_batch.set_line_width(options.line_width);
        self.boundary_batch.set_line_width(options.line_width * 1.5);
        self.hatch_batch.set_line_width(options.line_width * 0.8);
        self.arrow_batch.set_line_width(options.line_width * 0.8);
        self.wait_marker_batch.set_line_width(options.line_width * 1.5);

        let hatch_count = self.hatch_batch.vertex_count() / 2;
        if hatch_count != self.last_hatch_count {
            info!(
                "Prepared layer: {} contour verts, {} boundary verts, {} hatch lines | show_hatches={}",
                self.contour_batch.vertex_count(),
                self.boundary_batch.vertex_count(),
                hatch_count,
                options.show_hatches
            );
            self.last_hatch_count = hatch_count;
        }
    }

    /// Render the prepared layer
    fn render_prepared(&mut self, view: &ViewState) -> RenderResult<()> {
        let shader = self.shader.as_ref().ok_or_else(|| {
            RenderError::InvalidState("Shader not initialized".to_string())
        })?;

        shader.use_program();

        // Set up projection matrix in logical pixels (physical / scale_factor)
        // This keeps the projection in the same coordinate system as egui and ViewState
        let logical_w = self.width as f32 / self.scale_factor;
        let logical_h = self.height as f32 / self.scale_factor;
        let half_w = logical_w / 2.0;
        let half_h = logical_h / 2.0;
        let off_x = self.view_offset_x;
        let off_y = self.view_offset_y;
        let projection = ortho(-half_w + off_x, half_w + off_x, -half_h + off_y, half_h + off_y, -1.0, 1.0);
        shader.set_mat4("uProjection", &projection);

        // Set up view matrix (pan and zoom)
        let view_mat = view_matrix(view);
        shader.set_mat4("uView", &view_mat);

        // One-time diagnostic: verify shader pipeline is working
        if !self.logged_once {
            let proj_loc = shader.get_uniform_location("uProjection");
            let view_loc = shader.get_uniform_location("uView");
            info!("Shader program={}, uProjection loc={}, uView loc={}", shader.id, proj_loc, view_loc);
            info!("View: center=({:.3}, {:.3}), zoom={:.4}", view.center.x, view.center.y, view.zoom);
            info!("Projection: L={:.1}, R={:.1}, B={:.1}, T={:.1}",
                -half_w + off_x, half_w + off_x, -half_h + off_y, half_h + off_y);
            info!("Renderer: {}x{} (logical {:.0}x{:.0}), scale={:.2}, offset_x={}, offset_y={}", self.width, self.height, logical_w, logical_h, self.scale_factor, off_x, off_y);
            unsafe {
                let err = gl::GetError();
                if err != gl::NO_ERROR {
                    info!("GL error before batch render: 0x{:X}", err);
                }
            }
            self.logged_once = true;
        }

        // Render batches in order (back to front)
        self.hatch_batch.render();
        self.contour_batch.render();
        self.boundary_batch.render();
        self.arrow_batch.render();
        if self.show_wait_markers {
            self.wait_marker_batch.render();
        }

        Ok(())
    }
}

impl Default for GlRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for GlRenderer {
    fn initialize(&mut self, width: u32, height: u32) -> RenderResult<()> {
        self.width = width;
        self.height = height;
        
        if !self.initialized {
            self.init_gl()?;
        }
        
        self.resize(width, height)
    }

    fn resize(&mut self, width: u32, height: u32) -> RenderResult<()> {
        self.width = width;
        self.height = height;
        
        unsafe {
            gl::Viewport(0, 0, width as i32, height as i32);
        }
        
        Ok(())
    }

    fn clear(&mut self, color: &Color) -> RenderResult<()> {
        unsafe {
            gl::ClearColor(color.r, color.g, color.b, color.a);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
        Ok(())
    }

    fn begin_frame(&mut self) -> RenderResult<()> {
        // Comprehensive GL state reset — egui_glow modifies many states
        // and only restores scissor test after painting.
        // It leaves: blend func=(ONE, ONE_MINUS_SRC_ALPHA), its VBO bound
        // to GL_ARRAY_BUFFER, its program active, depth/cull/stencil disabled.
        unsafe {
            // Unbind any leftover objects from egui
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::BindVertexArray(0);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
            gl::UseProgram(0);

            // Disable tests that could reject fragments
            gl::Disable(gl::SCISSOR_TEST);
            gl::Disable(gl::DEPTH_TEST);
            gl::Disable(gl::STENCIL_TEST);
            gl::Disable(gl::CULL_FACE);

            // Ensure all color channels are writable
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);

            // Set our rendering state
            gl::Viewport(0, 0, self.width as i32, self.height as i32);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::LINE_SMOOTH);
        }
        Ok(())
    }

    fn end_frame(&mut self) -> RenderResult<()> {
        unsafe {
            gl::Flush();
        }
        Ok(())
    }

    fn render_layer(
        &mut self,
        layer: &Layer,
        view: &ViewState,
        options: &DisplayOptions,
    ) -> RenderResult<()> {
        self.prepare_layer(layer, options);
        self.render_prepared(view)
    }

    fn render_vector(
        &mut self,
        vector: &Vector,
        color: &Color,
        line_width: f32,
    ) -> RenderResult<()> {
        // For single vector rendering, use a temporary batch
        let mut batch = LineBatch::new();
        batch.add_polyline(&vector.points, color);
        batch.set_line_width(line_width);
        batch.render();
        Ok(())
    }

    fn viewport_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
