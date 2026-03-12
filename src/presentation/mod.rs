//! Presentation Layer - User Interface
//!
//! This layer contains:
//! - Application: Main application orchestration
//! - Window: Windowing and event handling
//! - Input: User input handling
//! - UI: egui-based overlay UI
//!
//! This is the outermost layer that connects everything together.

mod app;
mod window;
mod input;
mod ui;

pub use app::*;
pub use window::*;
pub use input::*;
pub use ui::*;
