//! Toolpath Entity
//!
//! High-level representation of a complete toolpath with metadata.

use crate::domain::entities::SliceStack;
use std::collections::HashMap;

/// Metadata about the toolpath
#[derive(Debug, Clone, Default)]
pub struct ToolpathMetadata {
    /// Original file format
    pub format: String,
    /// Creation timestamp
    pub created: Option<String>,
    /// Software that generated the file
    pub generator: Option<String>,
    /// Material information
    pub material: Option<String>,
    /// Machine/printer name
    pub machine: Option<String>,
    /// Custom key-value properties
    pub properties: HashMap<String, String>,
}

/// A complete toolpath with slice data and metadata
#[derive(Debug, Clone)]
pub struct Toolpath {
    /// The slice stack containing all layer data
    pub slice_stack: SliceStack,
    /// Toolpath metadata
    pub metadata: ToolpathMetadata,
    /// Part names for multi-part files
    pub parts: Vec<String>,
}

impl Toolpath {
    /// Create a new toolpath from a slice stack
    pub fn new(slice_stack: SliceStack) -> Self {
        Self {
            slice_stack,
            metadata: ToolpathMetadata::default(),
            parts: Vec::new(),
        }
    }

    /// Create a toolpath with metadata
    pub fn with_metadata(slice_stack: SliceStack, metadata: ToolpathMetadata) -> Self {
        Self {
            slice_stack,
            metadata,
            parts: Vec::new(),
        }
    }
}

impl Default for Toolpath {
    fn default() -> Self {
        Self::new(SliceStack::default())
    }
}
