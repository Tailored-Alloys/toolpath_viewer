//! File Port
//!
//! Interface for file loading operations.

use crate::domain::entities::Toolpath;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during file operations
#[derive(Debug, Error)]
pub enum FileError {
    #[error("File not found: {0}")]
    NotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Invalid file format: {0}")]
    InvalidFormat(String),
    
    #[error("Decompression error: {0}")]
    DecompressionError(String),
    
    #[error("Parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}

/// Result type for file operations
pub type FileResult<T> = Result<T, FileError>;

/// Port for file loading operations
pub trait FileLoader: Send + Sync {
    /// Load a toolpath from a file
    fn load(&self, path: &Path) -> FileResult<Toolpath>;
    
    /// Get supported file extensions
    fn supported_extensions(&self) -> &[&str];
    
    /// Check if this loader can handle the given file
    fn can_load(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                return self.supported_extensions()
                    .iter()
                    .any(|e| e.eq_ignore_ascii_case(ext_str));
            }
        }
        false
    }

    /// Load all toolpath resources from a file.
    /// For formats that contain multiple toolpath datasets (e.g. 3MF),
    /// each is returned as a separate `(name, Toolpath)` entry.
    /// Default implementation wraps `load()` in a single-element vec.
    fn load_all(&self, path: &Path) -> FileResult<Vec<(String, Toolpath)>> {
        let name = path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        let toolpath = self.load(path)?;
        Ok(vec![(name, toolpath)])
    }
}
