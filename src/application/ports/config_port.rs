//! Config Port
//!
//! Interface for configuration management.

use serde::{Deserialize, Serialize};
use crate::domain::value_objects::Color;
use thiserror::Error;

/// Errors that can occur during config operations
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    NotFound(String),
    
    #[error("Invalid config format: {0}")]
    InvalidFormat(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Result type for config operations
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Window settings
    pub window: WindowConfig,
    /// Display settings
    pub display: DisplayConfig,
    /// Color settings
    pub colors: ColorConfig,
    /// Recent files list
    pub recent_files: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window: WindowConfig::default(),
            display: DisplayConfig::default(),
            colors: ColorConfig::default(),
            recent_files: Vec::new(),
        }
    }
}

/// Window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Initial window width
    pub width: u32,
    /// Initial window height
    pub height: u32,
    /// Start maximized
    pub maximized: bool,
    /// Window title
    pub title: String,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            maximized: false,
            title: "Toolpath Viewer".to_string(),
        }
    }
}

/// Display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    /// Show slice boundaries by default
    pub show_slices: bool,
    /// Show contours by default
    pub show_contours: bool,
    /// Show hatches by default
    pub show_hatches: bool,
    /// Show direction arrows by default
    pub show_arrows: bool,
    /// Default line width
    pub line_width: f32,
    /// Anti-aliasing enabled
    pub antialiasing: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            show_slices: true,
            show_contours: true,
            show_hatches: true,
            show_arrows: false,
            line_width: 1.5,
            antialiasing: true,
        }
    }
}

/// Color configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    /// Background color (hex)
    pub background: String,
    /// Boundary color (hex)
    pub boundary: String,
    /// Contour color (hex)
    pub contour: String,
    /// Hatch color (hex)
    pub hatch: String,
    /// Depth contour color (hex)
    pub depth_contour: String,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            background: "#1E1E1E".to_string(),
            boundary: "#388E3C".to_string(),
            contour: "#2196F3".to_string(),
            hatch: "#E65100".to_string(),
            depth_contour: "#E91E63".to_string(),
        }
    }
}

/// Port for configuration management
pub trait ConfigManager: Send + Sync {
    /// Load configuration
    fn load(&self) -> ConfigResult<AppConfig>;
    
    /// Save configuration
    fn save(&self, config: &AppConfig) -> ConfigResult<()>;
    
    /// Get default configuration
    fn default_config(&self) -> AppConfig {
        AppConfig::default()
    }
}
