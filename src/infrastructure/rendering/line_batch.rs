//! Line Batch Renderer
//!
//! Efficient batched line rendering using VBOs.
//! Similar to the Python GL2DLineItem but in Rust.

use gl::types::*;
use crate::domain::value_objects::{Color, Point2D};
use log::info;
use std::mem;
use std::ptr;

/// Vertex data for a line point
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LineVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

impl LineVertex {
    pub fn new(x: f32, y: f32, color: &Color) -> Self {
        Self {
            position: [x, y],
            color: color.to_array(),
        }
    }

    pub fn from_point(point: &Point2D, color: &Color) -> Self {
        Self {
            position: [point.x, point.y],
            color: color.to_array(),
        }
    }
}

/// A segment of a line batch (start index, count)
#[derive(Debug, Clone, Copy)]
pub struct LineSegment {
    pub start: usize,
    pub count: usize,
}

/// Batched line renderer using VBOs
pub struct LineBatch {
    vao: GLuint,
    vbo: GLuint,
    vertices: Vec<LineVertex>,
    segments: Vec<LineSegment>,
    dirty: bool,
    line_width: f32,
}

impl LineBatch {
    /// Create a new line batch
    pub fn new() -> Self {
        let mut vao = 0;
        let mut vbo = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);

            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

            // Position attribute
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<LineVertex>() as GLsizei,
                ptr::null(),
            );
            gl::EnableVertexAttribArray(0);

            // Color attribute
            gl::VertexAttribPointer(
                1,
                4,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<LineVertex>() as GLsizei,
                (2 * mem::size_of::<f32>()) as *const _,
            );
            gl::EnableVertexAttribArray(1);

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }

        Self {
            vao,
            vbo,
            vertices: Vec::new(),
            segments: Vec::new(),
            dirty: true,
            line_width: 1.5,
        }
    }

    /// Clear all line data
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.segments.clear();
        self.dirty = true;
    }

    /// Add a polyline to the batch
    pub fn add_polyline(&mut self, points: &[Point2D], color: &Color) {
        if points.len() < 2 {
            return;
        }

        let start = self.vertices.len();
        
        for point in points {
            self.vertices.push(LineVertex::from_point(point, color));
        }

        self.segments.push(LineSegment {
            start,
            count: points.len(),
        });

        self.dirty = true;
    }

    /// Add a polyline with per-vertex colors
    pub fn add_polyline_colored(&mut self, points: &[Point2D], colors: &[Color]) {
        if points.len() < 2 || colors.len() != points.len() {
            return;
        }

        let start = self.vertices.len();

        for (point, color) in points.iter().zip(colors.iter()) {
            self.vertices.push(LineVertex::from_point(point, color));
        }

        self.segments.push(LineSegment {
            start,
            count: points.len(),
        });

        self.dirty = true;
    }

    /// Add a single line segment
    pub fn add_line(&mut self, p1: &Point2D, p2: &Point2D, color: &Color) {
        let start = self.vertices.len();

        self.vertices.push(LineVertex::from_point(p1, color));
        self.vertices.push(LineVertex::from_point(p2, color));

        self.segments.push(LineSegment { start, count: 2 });

        self.dirty = true;
    }

    /// Add a single line segment with per-vertex colors (gradient)
    pub fn add_line_gradient(&mut self, p1: &Point2D, c1: &Color, p2: &Point2D, c2: &Color) {
        let start = self.vertices.len();

        self.vertices.push(LineVertex::from_point(p1, c1));
        self.vertices.push(LineVertex::from_point(p2, c2));

        self.segments.push(LineSegment { start, count: 2 });

        self.dirty = true;
    }

    /// Set line width
    pub fn set_line_width(&mut self, width: f32) {
        self.line_width = width;
    }

    /// Get mutable access to internal vertices (for custom geometry like triangle fans).
    pub fn vertices_mut(&mut self) -> &mut Vec<LineVertex> {
        self.dirty = true;
        &mut self.vertices
    }

    /// Push a raw segment (start index and vertex count).
    pub fn push_segment(&mut self, start: usize, count: usize) {
        self.segments.push(LineSegment { start, count });
        self.dirty = true;
    }

    /// Upload data to GPU
    fn upload(&mut self) {
        if !self.dirty || self.vertices.is_empty() {
            return;
        }

        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (self.vertices.len() * mem::size_of::<LineVertex>()) as GLsizeiptr,
                self.vertices.as_ptr() as *const _,
                gl::DYNAMIC_DRAW,
            );
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }

        self.dirty = false;
    }

    /// Render all line segments
    pub fn render(&mut self) {
        if self.vertices.is_empty() {
            return;
        }

        self.upload();

        unsafe {
            gl::LineWidth(self.line_width);
            gl::Enable(gl::LINE_SMOOTH);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

            // Bind VAO, then re-bind VBO and re-establish vertex attrib pointers.
            // This guards against VAO state corruption from egui_glow's painter.
            gl::BindVertexArray(self.vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);

            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<LineVertex>() as GLsizei,
                ptr::null(),
            );
            gl::EnableVertexAttribArray(0);

            gl::VertexAttribPointer(
                1,
                4,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<LineVertex>() as GLsizei,
                (2 * mem::size_of::<f32>()) as *const _,
            );
            gl::EnableVertexAttribArray(1);

            for segment in &self.segments {
                gl::DrawArrays(
                    gl::LINE_STRIP,
                    segment.start as GLint,
                    segment.count as GLsizei,
                );
            }

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
            gl::Disable(gl::LINE_SMOOTH);
        }
    }

    /// Render all segments as filled triangle fans.
    /// Each segment must have a center vertex first, followed by perimeter vertices.
    pub fn render_as_fans(&mut self) {
        if self.vertices.is_empty() {
            return;
        }

        self.upload();

        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

            gl::BindVertexArray(self.vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);

            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<LineVertex>() as GLsizei,
                ptr::null(),
            );
            gl::EnableVertexAttribArray(0);

            gl::VertexAttribPointer(
                1,
                4,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<LineVertex>() as GLsizei,
                (2 * mem::size_of::<f32>()) as *const _,
            );
            gl::EnableVertexAttribArray(1);

            for segment in &self.segments {
                gl::DrawArrays(
                    gl::TRIANGLE_FAN,
                    segment.start as GLint,
                    segment.count as GLsizei,
                );
            }

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }
    }

    /// Get vertex count
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Get segment count
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }
}

impl Default for LineBatch {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LineBatch {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.vao);
            gl::DeleteBuffers(1, &self.vbo);
        }
    }
}


