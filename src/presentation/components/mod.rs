//! UI Components
//!
//! Each component is responsible for rendering a single UI section within
//! its assigned layout region. Components receive their region from
//! `LayoutRegions` and return typed output structs.

pub mod toolbar;
pub mod layer_slider;
pub mod vector_slider;
pub mod gradient_scale;
pub mod tool_panel;
pub mod overlays;
pub mod popups;
pub mod preferences;
pub mod sidebar;
pub mod status_bar;
pub mod activity_bar;
pub mod tab_bar;

pub use toolbar::*;
pub use layer_slider::*;
pub use vector_slider::*;
pub use tool_panel::*;
pub use overlays::*;
pub use popups::*;
pub use sidebar::*;
pub use status_bar::*;
pub use activity_bar::*;
pub use tab_bar::*;
