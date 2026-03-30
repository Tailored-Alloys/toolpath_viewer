//! Presentation Layer - User Interface
//!
//! This layer contains:
//! - Application: Main application orchestration
//! - Window: Windowing and event handling
//! - Input: User input handling
//! - UI: egui-based overlay UI
//! - Layout: Centralized layout region computation
//! - Theme: Shared colors, fonts, and widget helpers
//! - Components: Individual UI section components
//!
//! This is the outermost layer that connects everything together.

mod app;
mod window;
mod input;
pub mod layout;
pub mod palette;
pub mod theme;
pub mod components;
mod ui;

pub use app::*;
pub use window::*;
pub use input::*;
pub use ui::*;
pub use layout::TOOLBAR_HEIGHT;
pub use layout::TOOLBAR_BOTTOM;
