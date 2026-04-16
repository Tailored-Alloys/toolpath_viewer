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
    /// Theme settings
    #[serde(default)]
    pub theme: ThemeConfig,
    /// Canvas rendering settings
    #[serde(default)]
    pub canvas: CanvasConfig,
    /// Recent files list
    pub recent_files: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window: WindowConfig::default(),
            display: DisplayConfig::default(),
            colors: ColorConfig::default(),
            theme: ThemeConfig::default(),
            canvas: CanvasConfig::default(),
            recent_files: Vec::new(),
        }
    }
}

/// Canvas rendering settings (persisted)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasConfig {
    /// Base line width in pixels
    pub line_width: f32,
    /// Boundary line width multiplier (relative to base)
    pub boundary_width_multiplier: f32,
    /// Contour line width multiplier (relative to base)
    pub contour_width_multiplier: f32,
    /// Hatch line width multiplier (relative to base)
    pub hatch_width_multiplier: f32,
    /// Arrow size multiplier (scales arrow arm length)
    pub arrow_size_multiplier: f32,
    /// Wait marker (dot) size multiplier (scales circle radius)
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
}

impl Default for CanvasConfig {
    fn default() -> Self {
        Self {
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
        }
    }
}

/// Theme and palette configuration (persisted)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Theme mode: light / dark / system
    pub mode: ThemeMode,
    /// Palette id for light mode
    pub light_palette: PaletteId,
    /// Palette id for dark mode
    pub dark_palette: PaletteId,
    /// Custom palette overrides for light mode
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_light: Option<CustomPaletteConfig>,
    /// Custom palette overrides for dark mode
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_dark: Option<CustomPaletteConfig>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            mode: ThemeMode::default(),
            light_palette: PaletteId::default(),
            dark_palette: PaletteId::default(),
            custom_light: None,
            custom_dark: None,
        }
    }
}

/// Overall application appearance mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Light,
    Dark,
    System,
}

impl Default for ThemeMode {
    fn default() -> Self {
        ThemeMode::System
    }
}

impl ThemeMode {
    pub fn label(&self) -> &'static str {
        match self {
            ThemeMode::Light => "Light",
            ThemeMode::Dark => "Dark",
            ThemeMode::System => "System",
        }
    }
}

/// Identifies a curated palette within a given mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaletteId {
    Default,
    Professional,
    Vibrant,
    Ocean,
    Warm,
    ColorblindRG,
    ColorblindBY,
    HighContrast,
    Custom,
}

impl Default for PaletteId {
    fn default() -> Self {
        PaletteId::Default
    }
}

impl PaletteId {
    /// Human-readable display name.
    pub fn label(&self) -> &'static str {
        match self {
            PaletteId::Default => "Default",
            PaletteId::Professional => "Professional",
            PaletteId::Vibrant => "Vibrant",
            PaletteId::Ocean => "Ocean",
            PaletteId::Warm => "Warm",
            PaletteId::ColorblindRG => "Colorblind Safe (R/G)",
            PaletteId::ColorblindBY => "Colorblind Safe (B/Y)",
            PaletteId::HighContrast => "High Contrast",
            PaletteId::Custom => "Custom",
        }
    }

    /// Accessibility badge text (if any).
    pub fn badge(&self) -> Option<&'static str> {
        match self {
            PaletteId::ColorblindRG => Some("\u{267F} Deuteranopia safe"),
            PaletteId::ColorblindBY => Some("\u{267F} Tritanopia safe"),
            PaletteId::HighContrast => Some("\u{267F} WCAG AAA"),
            _ => None,
        }
    }
}

/// Ordered list of all palette IDs.
pub const ALL_PALETTE_IDS: &[PaletteId] = &[
    PaletteId::Default,
    PaletteId::Professional,
    PaletteId::Vibrant,
    PaletteId::Ocean,
    PaletteId::Warm,
    PaletteId::ColorblindRG,
    PaletteId::ColorblindBY,
    PaletteId::HighContrast,
    PaletteId::Custom,
];

/// Per-color hex overrides for the Custom palette.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPaletteConfig {
    pub background: String,
    pub boundary: String,
    pub contour: String,
    pub hatch: String,
    pub accent: String,
}

impl Default for CustomPaletteConfig {
    fn default() -> Self {
        Self {
            background: "#FAFAFA".into(),
            boundary: "#2C3E50".into(),
            contour: "#00BCD4".into(),
            hatch: "#E91E63".into(),
            accent: "#1976D2".into(),
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
            title: format!("Toolpath Viewer [{}]", crate::APP_VERSION),
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
