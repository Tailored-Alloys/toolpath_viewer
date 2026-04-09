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
mod cursors;
pub mod tab_state;

pub use app::*;
pub use window::*;
pub use input::*;
pub use ui::*;
pub use layout::TOOLBAR_HEIGHT;
pub use layout::TOOLBAR_BOTTOM;
pub use layout::SIDEBAR_WIDTH;
pub use layout::ACTIVITY_BAR_WIDTH;
pub use layout::STATUS_BAR_HEIGHT;
pub use layout::VECTOR_PLAYER_HEIGHT;
pub use layout::TAB_BAR_HEIGHT;
pub use tab_state::{TabManager, TabState};
