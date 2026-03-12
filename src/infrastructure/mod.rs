//! Infrastructure Layer - External Dependencies Implementation
//!
//! This layer contains:
//! - File Adapters: Implementations of file loading ports
//! - Rendering: OpenGL renderer implementation
//! - Config: File-based configuration management
//!
//! This layer implements the ports defined in the Application layer.

pub mod file_adapters;
pub mod rendering;
pub mod config;
