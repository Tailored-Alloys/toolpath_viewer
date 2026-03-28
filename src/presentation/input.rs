//! Input Handling
//!
//! User input state and event processing.

use crate::domain::value_objects::Point2D;

/// Mouse button state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

/// Input action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    /// Navigate to next layer
    NextLayer,
    /// Navigate to previous layer
    PrevLayer,
    /// Jump forward 10 layers
    JumpForward,
    /// Jump backward 10 layers
    JumpBackward,
    /// Go to first layer
    FirstLayer,
    /// Go to last layer
    LastLayer,
    /// Reset view (fit to content)
    ResetView,
    /// Toggle boundary visibility
    ToggleBoundaries,
    /// Toggle contour visibility
    ToggleContours,
    /// Toggle hatch visibility
    ToggleHatches,
    /// Toggle direction arrows
    ToggleArrows,
    /// Toggle wait time markers
    ToggleWaitMarkers,
    /// Toggle scale bar
    ToggleScaleBar,
    /// Open file dialog
    OpenFile,
    /// Take a snapshot (screenshot)
    Snapshot,
    /// Toggle background grid
    ToggleGrid,
    /// Zoom in
    ZoomIn,
    /// Zoom out
    ZoomOut,
    /// Toggle zoom selection mode
    ToggleZoomSelect,
    /// Toggle ruler (measure) tool
    ToggleRuler,
    /// Clear ruler measurements
    ClearMeasurements,
    /// Set parameter color mode: None
    ParamModeNone,
    /// Set parameter color mode: Power
    ParamModePower,
    /// Set parameter color mode: Speed
    ParamModeSpeed,
    /// Set parameter color mode: WaitTime
    ParamModeWaitTime,
    /// Toggle file info panel
    ToggleFileInfo,
    /// Toggle controls popup
    ToggleControls,
    /// Toggle vector-by-vector view
    ToggleVectorView,
    /// Next vector (when vector view active)
    NextVector,
    /// Previous vector (when vector view active)
    PrevVector,
    /// Toggle vector playback (play/pause)
    ToggleVectorPlayback,
    /// Quit application
    Quit,
}

/// Input state tracker
#[derive(Debug, Default)]
pub struct InputState {
    /// Current mouse position in screen coordinates
    pub mouse_pos: Point2D,
    /// Previous mouse position (for delta calculation)
    pub prev_mouse_pos: Point2D,
    /// Left mouse button pressed
    pub left_pressed: bool,
    /// Middle mouse button pressed
    pub middle_pressed: bool,
    /// Right mouse button pressed
    pub right_pressed: bool,
    /// Ctrl key held
    pub ctrl_held: bool,
    /// Shift key held
    pub shift_held: bool,
    /// Alt key held
    pub alt_held: bool,
}

impl InputState {
    /// Create a new input state
    pub fn new() -> Self {
        Self::default()
    }

    /// Update mouse position
    pub fn update_mouse(&mut self, x: f32, y: f32) {
        self.prev_mouse_pos = self.mouse_pos;
        self.mouse_pos = Point2D::new(x, y);
    }

    /// Get mouse delta since last update
    pub fn mouse_delta(&self) -> Point2D {
        Point2D::new(
            self.mouse_pos.x - self.prev_mouse_pos.x,
            self.mouse_pos.y - self.prev_mouse_pos.y,
        )
    }

    /// Check if any mouse button is pressed
    pub fn any_mouse_pressed(&self) -> bool {
        self.left_pressed || self.middle_pressed || self.right_pressed
    }

    /// Check if dragging (mouse moved while button pressed)
    pub fn is_dragging(&self) -> bool {
        self.any_mouse_pressed() && self.mouse_delta().distance_to(&Point2D::zero()) > 1.0
    }
}

/// Parse a keyboard event into an action
pub fn key_to_action(key: &str, ctrl: bool, shift: bool) -> Option<InputAction> {
    match (key.to_lowercase().as_str(), ctrl, shift) {
        // Navigation
        ("up" | "w", false, false) => Some(InputAction::NextLayer),
        ("down" | "s", false, false) => Some(InputAction::PrevLayer),
        ("pageup", false, false) => Some(InputAction::JumpForward),
        ("pagedown", false, false) => Some(InputAction::JumpBackward),
        ("home", false, false) => Some(InputAction::FirstLayer),
        ("end", false, false) => Some(InputAction::LastLayer),
        
        // View
        ("f" | "r", false, false) => Some(InputAction::ResetView),
        
        // Toggles
        ("b", false, false) => Some(InputAction::ToggleBoundaries),
        ("c", false, false) => Some(InputAction::ToggleContours),
        ("h", false, false) => Some(InputAction::ToggleHatches),
        ("a", false, false) => Some(InputAction::ToggleArrows),
        ("t", false, false) => Some(InputAction::ToggleWaitMarkers),
        ("v", false, false) => Some(InputAction::ToggleScaleBar),
        ("g", false, false) => Some(InputAction::ToggleGrid),
        ("i", false, false) => Some(InputAction::ToggleFileInfo),
        ("f1", false, false) => Some(InputAction::ToggleControls),

        // Zoom
        ("+" | "=", false, false) => Some(InputAction::ZoomIn),
        ("-", false, false) => Some(InputAction::ZoomOut),
        ("z", false, false) => Some(InputAction::ToggleZoomSelect),

        // Vector view
        ("n", false, false) => Some(InputAction::ToggleVectorView),
        ("right", false, false) => Some(InputAction::NextVector),
        ("left", false, false) => Some(InputAction::PrevVector),
        ("space", false, false) => Some(InputAction::ToggleVectorPlayback),

        // Tools
        ("m", false, false) => Some(InputAction::ToggleRuler),
        ("x", false, false) => Some(InputAction::ClearMeasurements),

        // Color modes
        ("1", false, false) => Some(InputAction::ParamModeNone),
        ("2", false, false) => Some(InputAction::ParamModePower),
        ("3", false, false) => Some(InputAction::ParamModeSpeed),
        ("4", false, false) => Some(InputAction::ParamModeWaitTime),

        // File operations
        ("o", true, false) => Some(InputAction::OpenFile),

        // Snapshot
        ("p", true, false) => Some(InputAction::Snapshot),
        
        // Application
        ("q", true, false) | ("escape", _, _) => Some(InputAction::Quit),
        
        _ => None,
    }
}
