//! Application Ports (Interfaces)
//!
//! Defines traits that external dependencies must implement.
//! This follows the Dependency Inversion Principle.

mod file_port;
mod renderer_port;
mod config_port;

pub use file_port::*;
pub use renderer_port::*;
pub use config_port::*;
