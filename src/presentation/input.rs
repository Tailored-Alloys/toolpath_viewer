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
    /// Set tool mode: Pan
    ToolPan,
    /// Clear ruler measurements
    ClearMeasurements,
    /// Set parameter color mode: None
    ParamModeNone,
    /// Set parameter color mode: Power
    ParamModePower,
    /// Set parameter color mode: Speed
    ParamModeSpeed,

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
    /// Toggle sidebar
    ToggleSidebar,
    /// Switch to next tab (Ctrl+Tab)
    NextTab,
    /// Switch to previous tab (Ctrl+Shift+Tab)
    PrevTab,
    /// Close active tab (Ctrl+W)
    CloseTab,
    /// Jump to first vector (Ctrl+Left)
    FirstVector,
    /// Jump to last vector (Ctrl+Right)
    LastVector,
    /// Increase playback speed
    PlaybackSpeedUp,
    /// Decrease playback speed
    PlaybackSpeedDown,
    /// Undo last measurement (Ctrl+Z)
    UndoMeasurement,
    /// Focus the "Go to layer" input field
    FocusLayerInput,
    /// Cycle view mode (Overlay → Tab → Split)
    CycleViewMode,
    /// Jump to a specific tab by 1-based position (Ctrl+1..9)
    JumpToTab(usize),
    /// Toggle split view on/off
    ToggleSplit,
    /// Focus the left pane in split mode
    FocusLeftPane,
    /// Focus the right pane in split mode
    FocusRightPane,
    /// Set color mode: By File/Part
    ParamModeFile,
    /// Toggle preferences dialog
    TogglePreferences,
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

        // Tool modes
        ("p", false, false) => Some(InputAction::ToolPan),
        ("m", false, false) => Some(InputAction::ToggleRuler),

        // Vector view
        ("n", false, false) => Some(InputAction::ToggleVectorView),
        ("right", false, false) => Some(InputAction::NextVector),
        ("left", false, false) => Some(InputAction::PrevVector),
        ("right", true, false) => Some(InputAction::LastVector),
        ("left", true, false) => Some(InputAction::FirstVector),
        ("space", false, false) => Some(InputAction::ToggleVectorPlayback),
        ("]", false, false) => Some(InputAction::PlaybackSpeedUp),
        ("[", false, false) => Some(InputAction::PlaybackSpeedDown),

        // Tools
        ("x", false, false) => Some(InputAction::ClearMeasurements),
        ("z", true, false) => Some(InputAction::UndoMeasurement),

        // Sidebar
        ("e", false, false) => Some(InputAction::ToggleSidebar),

        // Layer input focus
        ("l", false, false) => Some(InputAction::FocusLayerInput),

        // View mode cycling (Ctrl+Shift+\)
        ("\\", true, true) => Some(InputAction::CycleViewMode),

        // Toggle split view (Ctrl+\)
        ("\\", true, false) => Some(InputAction::ToggleSplit),

        // Tab management
        ("tab", true, false) => Some(InputAction::NextTab),
        ("tab", true, true) => Some(InputAction::PrevTab),
        ("w", true, false) => Some(InputAction::CloseTab),

        // Jump to tab by position (Ctrl+1..9)
        ("1", true, false) => Some(InputAction::JumpToTab(0)),
        ("2", true, false) => Some(InputAction::JumpToTab(1)),
        ("3", true, false) => Some(InputAction::JumpToTab(2)),
        ("4", true, false) => Some(InputAction::JumpToTab(3)),
        ("5", true, false) => Some(InputAction::JumpToTab(4)),
        ("6", true, false) => Some(InputAction::JumpToTab(5)),
        ("7", true, false) => Some(InputAction::JumpToTab(6)),
        ("8", true, false) => Some(InputAction::JumpToTab(7)),
        ("9", true, false) => Some(InputAction::JumpToTab(8)),

        // Split pane focus (Ctrl+Alt uses alt_held check in caller, but here
        // we only receive ctrl+shift — so we use Ctrl+Shift+Left/Right)
        ("left", true, true) => Some(InputAction::FocusLeftPane),
        ("right", true, true) => Some(InputAction::FocusRightPane),

        // Color modes
        ("1", false, false) => Some(InputAction::ParamModeNone),
        ("2", false, false) => Some(InputAction::ParamModePower),
        ("3", false, false) => Some(InputAction::ParamModeSpeed),
        ("4", false, false) => Some(InputAction::ParamModeFile),

        // File operations
        ("o", true, false) => Some(InputAction::OpenFile),

        // Snapshot
        ("p", true, false) => Some(InputAction::Snapshot),

        // Preferences
        (",", true, false) => Some(InputAction::TogglePreferences),

        // Application
        ("q", true, false) | ("escape", _, _) => Some(InputAction::Quit),
        
        _ => None,
    }
}
