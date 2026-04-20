//! Configuration Adapter
//!
//! File-based configuration management implementation.

use crate::application::ports::{AppConfig, ConfigError, ConfigManager, ConfigResult};
use std::fs;
use std::path::PathBuf;
use log::{debug, info};

/// JSON file-based configuration manager
pub struct JsonConfigManager {
    config_path: PathBuf,
}

impl JsonConfigManager {
    /// Create a new config manager with default path
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("toolpath_viewer");
        
        Self {
            config_path: config_dir.join("config.json"),
        }
    }

    /// Get the config directory
    pub fn config_dir(&self) -> PathBuf {
        self.config_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    }

    /// Ensure config directory exists
    fn ensure_dir(&self) -> ConfigResult<()> {
        let dir = self.config_dir();
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
            debug!("Created config directory: {:?}", dir);
        }
        Ok(())
    }
}

impl Default for JsonConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigManager for JsonConfigManager {
    fn load(&self) -> ConfigResult<AppConfig> {
        if !self.config_path.exists() {
            info!("Config file not found, using defaults");
            return Ok(AppConfig::default());
        }

        let content = fs::read_to_string(&self.config_path)?;
        
        serde_json::from_str(&content).map_err(|e| {
            ConfigError::InvalidFormat(format!("Failed to parse config: {}", e))
        })
    }

    fn save(&self, config: &AppConfig) -> ConfigResult<()> {
        self.ensure_dir()?;

        let content = serde_json::to_string_pretty(config).map_err(|e| {
            ConfigError::SerializationError(format!("Failed to serialize config: {}", e))
        })?;

        fs::write(&self.config_path, content)?;
        info!("Saved config to {:?}", self.config_path);
        
        Ok(())
    }
}

// Platform-specific config directory helper
#[cfg(target_os = "windows")]
mod dirs {
    use std::path::PathBuf;
    use std::env;

    pub fn config_dir() -> Option<PathBuf> {
        env::var("APPDATA").ok().map(PathBuf::from)
    }
}

#[cfg(not(target_os = "windows"))]
mod dirs {
    use std::path::PathBuf;
    use std::env;

    pub fn config_dir() -> Option<PathBuf> {
        env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config"))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let manager = JsonConfigManager::new();
        let config = manager.default_config();
        assert_eq!(config.window.width, 1280);
    }
}
