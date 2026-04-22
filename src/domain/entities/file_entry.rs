//! File Entry Entity
//!
//! Represents a single loaded file in a multi-file workspace.

use std::path::PathBuf;
use crate::domain::entities::Toolpath;
use crate::domain::value_objects::Color;

/// A loaded file with its associated toolpath and display state
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Unique identifier for this file
    pub id: usize,
    /// Display name (file stem)
    pub name: String,
    /// Full file path
    pub path: PathBuf,
    /// Parsed toolpath data
    pub toolpath: Toolpath,
    /// Whether this file is visible in the viewport
    pub visible: bool,
    /// Assigned display color (used in "color by file" mode)
    pub color: Color,
}

/// Predefined palette of distinct colors for auto-assignment to files
const FILE_COLORS: &[Color] = &[
    Color::rgb(0.220, 0.557, 0.886),  // Blue
    Color::rgb(0.910, 0.298, 0.235),  // Red
    Color::rgb(0.180, 0.800, 0.443),  // Green
    Color::rgb(0.608, 0.349, 0.714),  // Purple
    Color::rgb(1.000, 0.596, 0.000),  // Orange
    Color::rgb(0.000, 0.737, 0.831),  // Cyan
    Color::rgb(0.957, 0.263, 0.612),  // Pink
    Color::rgb(0.498, 0.549, 0.553),  // Gray
    Color::rgb(0.827, 0.686, 0.216),  // Gold
    Color::rgb(0.000, 0.588, 0.533),  // Teal
];

impl FileEntry {
    /// Get a color from the palette by index (cycles)
    pub fn palette_color(index: usize) -> Color {
        FILE_COLORS[index % FILE_COLORS.len()]
    }
}
