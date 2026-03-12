//! Load Toolpath Use Case
//!
//! Handles loading toolpath files from various formats.

use crate::application::ports::{FileLoader, FileResult};
use crate::domain::entities::Toolpath;
use std::path::Path;
use std::sync::Arc;
use log::info;

/// Use case for loading toolpath files
pub struct LoadToolpathUseCase {
    loaders: Vec<Arc<dyn FileLoader>>,
}

impl LoadToolpathUseCase {
    /// Create a new load toolpath use case with the given loaders
    pub fn new(loaders: Vec<Arc<dyn FileLoader>>) -> Self {
        Self { loaders }
    }

    /// Load a toolpath from a file
    pub fn execute(&self, path: &Path) -> FileResult<Toolpath> {
        info!("Loading toolpath from: {:?}", path);

        // Find a loader that can handle this file
        for loader in &self.loaders {
            if loader.can_load(path) {
                info!("Using loader for extensions: {:?}", loader.supported_extensions());
                return loader.load(path);
            }
        }

        Err(crate::application::ports::FileError::UnsupportedFormat(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_string(),
        ))
    }

    /// Get all supported extensions
    pub fn supported_extensions(&self) -> Vec<&str> {
        self.loaders
            .iter()
            .flat_map(|l| l.supported_extensions().iter().copied())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::SliceStack;

    struct MockLoader;

    impl FileLoader for MockLoader {
        fn load(&self, _path: &Path) -> FileResult<Toolpath> {
            Ok(Toolpath::new(SliceStack::new("Mock")))
        }

        fn supported_extensions(&self) -> &[&str] {
            &["ilt", "cli"]
        }
    }

    #[test]
    fn test_load_with_supported_extension() {
        let use_case = LoadToolpathUseCase::new(vec![Arc::new(MockLoader)]);
        let result = use_case.execute(Path::new("test.ilt"));
        assert!(result.is_ok());
    }
}
