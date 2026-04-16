//! Layout Region System
//!
//! Centralizes all layout constants and computes non-overlapping screen regions
//! for each UI section. Regions are recomputed each frame from the current screen
//! size and visibility flags (immediate-mode compatible).
//!
//! New VS Code–style layout:
//! - Top: toolbar (full width)
//! - Left: collapsible sidebar (files + gradient legends)
//! - Right: vertical layer slider (inline, displaces viewport)
//! - Bottom: status bar (always) + vector player bar (conditional)
//! - Center: viewport (remaining space)
//! - Floating: tool panel (top-right of viewport)

use egui::Rect;

// ── Global layout constants ──────────────────────────────────────────────

/// Height of the top toolbar in logical pixels (passed to exact_height)
pub const TOOLBAR_HEIGHT: f32 = 52.0;

/// Toolbar frame vertical padding (must match toolbar inner_margin vertical)
pub const TOOLBAR_FRAME_VPAD: f32 = 8.0;

/// Actual rendered toolbar bottom (TOOLBAR_HEIGHT + 2 * TOOLBAR_FRAME_VPAD)
pub const TOOLBAR_BOTTOM: f32 = TOOLBAR_HEIGHT + 2.0 * TOOLBAR_FRAME_VPAD;

/// Standard margin between panels and screen edges
pub const PANEL_MARGIN: f32 = 12.0;

/// Gap between adjacent panels
pub const PANEL_GAP: f32 = 8.0;

/// Width of the right layer slider panel
pub const LAYER_SLIDER_WIDTH: f32 = 80.0;

/// Width of the left gradient scale panel (used inside sidebar sections)
pub const GRADIENT_PANEL_WIDTH: f32 = 130.0;

/// Tool panel button size
pub const TOOL_BTN_SIZE: f32 = 28.0;

/// Tool panel icon size
pub const TOOL_ICON_SIZE: f32 = 14.0;

/// Tool panel estimated height (buttons + frame padding)
pub const TOOL_PANEL_HEIGHT: f32 = 44.0;

/// Bottom margin for panels
pub const BOTTOM_MARGIN: f32 = 16.0;

/// Scale bar height reservation (when visible)
pub const SCALE_BAR_RESERVE: f32 = 48.0;

/// Scale bar drawing margin from screen edge
pub const SCALE_BAR_MARGIN: f32 = 20.0;

/// Width of the left activity bar (icon strip, always visible)
pub const ACTIVITY_BAR_WIDTH: f32 = 44.0;

/// Width of the left sidebar content panel when open (default)
pub const SIDEBAR_WIDTH: f32 = 240.0;

/// Minimum sidebar content panel width (for drag-to-resize)
pub const SIDEBAR_MIN_WIDTH: f32 = 180.0;

/// Maximum sidebar content panel width (for drag-to-resize)
pub const SIDEBAR_MAX_WIDTH: f32 = 400.0;

/// Height of the bottom status bar
pub const STATUS_BAR_HEIGHT: f32 = 24.0;

/// Height of the horizontal vector player bar
pub const VECTOR_PLAYER_HEIGHT: f32 = 36.0;

/// Compact toolbar width threshold
pub const COMPACT_TOOLBAR_THRESHOLD: f32 = 900.0;

/// Minimum height for layer slider content to show goto section
pub const GOTO_SECTION_MIN_HEIGHT: f32 = 250.0;

/// Minimum height for gradient ticks to be visible
pub const GRADIENT_TICKS_MIN_HEIGHT: f32 = 350.0;

/// Minimum window width before auto-collapsing sidebar
pub const SIDEBAR_AUTO_COLLAPSE_WIDTH: f32 = 1000.0;

/// Height of the secondary header tab bar (below toolbar)
pub const TAB_BAR_HEIGHT: f32 = 32.0;

/// Horizontal gap (in logical pixels) between left and right split panes
pub const SPLIT_GAP: f32 = 8.0;

/// Sidebar content panel horizontal inner_margin (must match sidebar frame inner_margin horizontal).
/// In egui 0.27, `exact_width(w)` sets the content min-width; the frame's inner_margin
/// adds to the total panel width claimed. This constant accounts for that.
pub const SIDEBAR_CONTENT_HPAD: f32 = 8.0;

// ── Visibility flags ─────────────────────────────────────────────────────

/// Flags controlling which optional panels are visible, affecting region computation.
#[derive(Debug, Clone, Copy)]
pub struct VisibilityFlags {
    /// Whether a file is loaded (controls layer slider visibility)
    pub has_layers: bool,
    /// Whether a parameter mode is active (controls gradient legend in sidebar)
    pub show_gradient: bool,
    /// Whether wait markers are active (controls wait gradient legend in sidebar)
    pub show_wait_gradient: bool,
    /// Whether the scale bar is visible
    pub show_scale_bar: bool,
    /// Whether file info popup is open
    pub show_file_info: bool,
    /// Whether controls popup is open
    pub show_controls: bool,
    /// Whether the grid is enabled (affects grid label overlay)
    pub show_grid: bool,
    /// Whether vector-by-vector view is active (controls vector player bar)
    pub vector_view_active: bool,
    /// Whether the sidebar is open
    pub sidebar_open: bool,
    /// Whether the tab bar is visible (any open tabs)
    pub has_tab_bar: bool,
    /// Dynamic sidebar content panel width (user-resizable, clamped to min/max)
    pub sidebar_content_width: f32,
}

// ── Layout Regions ───────────────────────────────────────────────────────

/// Pre-computed non-overlapping screen regions for all UI sections.
///
/// Computed once per frame from screen size and visibility flags.
/// All positions are in logical (egui) screen coordinates.
#[derive(Debug, Clone)]
pub struct LayoutRegions {
    /// Full screen rect
    pub screen: Rect,

    /// Top toolbar — full width, fixed height
    pub toolbar: Rect,

    /// Whether compact toolbar mode is active (narrow window)
    pub compact_toolbar: bool,

    /// Left sidebar region (collapsible, displaces viewport)
    pub sidebar: SidebarRegion,

    /// Tool panel — top-right of viewport, floating
    pub tool_panel: ToolPanelRegion,

    /// Right layer slider — inline, displaces viewport right edge
    pub layer_slider: Option<LayerSliderRegion>,

    /// Bottom status bar — full width, always visible
    pub status_bar: StatusBarRegion,

    /// Horizontal vector player bar — above status bar, conditional
    pub vector_player: Option<VectorPlayerRegion>,

    /// Scale bar — bottom-left of viewport
    pub scale_bar: Option<Rect>,

    /// Secondary header tab bar — below toolbar, shown when files are open
    pub tab_bar: Option<Rect>,

    /// Viewport — the central area where GL content renders
    /// Displaces for sidebar, layer slider, toolbar, tab bar, status bar, and vector player.
    pub viewport: Rect,
}

/// Sidebar positioning info (activity bar + optional content panel)
#[derive(Debug, Clone)]
pub struct SidebarRegion {
    /// Whether the content panel is currently visible (sidebar_open && not auto-collapsed)
    pub content_visible: bool,
    /// Width of the activity bar (always visible)
    pub activity_bar_width: f32,
    /// Width of the content panel (0 when collapsed)
    pub content_width: f32,
    /// Total width displaced by sidebar (activity_bar + content)
    pub total_width: f32,
    /// Top position (below toolbar)
    pub top: f32,
    /// Bottom position (above status bar / vector player)
    pub bottom: f32,
    /// Available content height inside the sidebar
    pub content_height: f32,
    /// Whether gradient legend should show ticks
    pub show_gradient_ticks: bool,
}

/// Tool panel positioning info
#[derive(Debug, Clone)]
pub struct ToolPanelRegion {
    /// Position for the egui Area (left-top corner)
    pub anchor_pos: egui::Pos2,
    /// Top of the tool panel in screen coords
    pub top: f32,
}

/// Layer slider positioning info
#[derive(Debug, Clone)]
pub struct LayerSliderRegion {
    /// Position for the egui Area (top-left)
    pub pos: egui::Pos2,
    /// Available content height (inside frame, excluding padding)
    pub content_height: f32,
    /// Width of the panel
    pub width: f32,
    /// Whether to show the goto section
    pub show_goto: bool,
}

/// Gradient panel positioning info (used inside sidebar)
#[derive(Debug, Clone)]
pub struct GradientRegion {
    /// Position for the egui Area (top-left)
    pub pos: egui::Pos2,
    /// Available content height (inside frame, excluding padding)
    pub content_height: f32,
    /// Width of the panel
    pub width: f32,
    /// Whether to show tick labels
    pub show_ticks: bool,
}

/// Status bar positioning info
#[derive(Debug, Clone)]
pub struct StatusBarRegion {
    /// Full rect for the status bar
    pub rect: Rect,
    /// Height of the status bar
    pub height: f32,
}

/// Vector player bar positioning info (horizontal, above status bar)
#[derive(Debug, Clone)]
pub struct VectorPlayerRegion {
    /// Full rect for the vector player bar
    pub rect: Rect,
    /// Height of the bar
    pub height: f32,
}

/// Vector slider positioning info (kept for backward compat, unused in new layout)
#[derive(Debug, Clone)]
pub struct VectorSliderRegion {
    /// Position for the egui Area (top-left)
    pub pos: egui::Pos2,
    /// Available content height (inside frame, excluding padding)
    pub content_height: f32,
    /// Width of the panel
    pub width: f32,
    /// Whether to show the goto section
    pub show_goto: bool,
}

impl LayoutRegions {
    /// Compute all layout regions from current screen size and visibility flags.
    ///
    /// Layout structure (displacing viewport):
    /// ```text
    /// ┌───────────────────────────────────────────┐
    /// │                 Toolbar                    │
    /// ├──────┬──────┬───────────────────┬──────────┤
    /// │      │      │   Tab Bar         │          │
    /// │ Act. │ Side ├───────────────────┤  Layer   │
    /// │ Bar  │ bar  │    Viewport       │  Slider  │
    /// │      │      │                   │          │
    /// ├──────┴──────┴───────────────────┴──────────┤
    /// │           Vector Player (conditional)      │
    /// ├───────────────────────────────────────────│
    /// │              Status Bar                    │
    /// └───────────────────────────────────────────┘
    /// ```
    pub fn compute(screen: Rect, flags: &VisibilityFlags) -> Self {
        let screen_w = screen.width();
        let screen_h = screen.height();
        let compact_toolbar = screen_w < COMPACT_TOOLBAR_THRESHOLD;

        // ── Toolbar ──
        // Note: egui 0.27 exact_height(h) sets the content min-height; the frame's
        // inner_margin (TOOLBAR_FRAME_VPAD top+bottom) adds to the actual panel height.
        let toolbar = Rect::from_min_size(
            screen.left_top(),
            egui::vec2(screen_w, TOOLBAR_BOTTOM),
        );

        // ── Sidebar width (computed early so tab bar can start after it) ──
        let sidebar_auto_collapsed = screen_w < SIDEBAR_AUTO_COLLAPSE_WIDTH;
        let content_visible = flags.sidebar_open && !sidebar_auto_collapsed;
        let content_width = if content_visible {
            flags.sidebar_content_width.clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH)
        } else {
            0.0
        };
        // In egui 0.27, exact_width(w) sets content min-width; the frame's inner_margin
        // adds to the total panel width. Account for this when sidebar is visible.
        let content_margin = if content_visible { 2.0 * SIDEBAR_CONTENT_HPAD } else { 0.0 };
        let sidebar_width = ACTIVITY_BAR_WIDTH + content_width + content_margin;

        // ── Tab bar (secondary header, only spans the canvas area right of sidebar) ──
        let tab_bar = if flags.has_tab_bar {
            Some(Rect::from_min_size(
                egui::pos2(screen.left() + sidebar_width, TOOLBAR_BOTTOM),
                egui::vec2((screen_w - sidebar_width).max(0.0), TAB_BAR_HEIGHT),
            ))
        } else {
            None
        };

        // Both sidebar and canvas start below the tab bar (when visible),
        // since the tab bar is a TopBottomPanel that claims vertical space.
        let content_top = TOOLBAR_BOTTOM + if flags.has_tab_bar { TAB_BAR_HEIGHT } else { 0.0 };

        // ── Status bar (always at bottom) ──
        let status_bar_rect = Rect::from_min_size(
            egui::pos2(screen.left(), screen_h - STATUS_BAR_HEIGHT),
            egui::vec2(screen_w, STATUS_BAR_HEIGHT),
        );
        let status_bar = StatusBarRegion {
            rect: status_bar_rect,
            height: STATUS_BAR_HEIGHT,
        };

        // ── Vector player bar (conditional, above status bar) ──
        let vector_player = if flags.vector_view_active && flags.has_layers {
            let vp_rect = Rect::from_min_size(
                egui::pos2(screen.left(), screen_h - STATUS_BAR_HEIGHT - VECTOR_PLAYER_HEIGHT),
                egui::vec2(screen_w, VECTOR_PLAYER_HEIGHT),
            );
            Some(VectorPlayerRegion {
                rect: vp_rect,
                height: VECTOR_PLAYER_HEIGHT,
            })
        } else {
            None
        };

        // Bottom edge of the viewport area (above vector player / status bar)
        let viewport_bottom = if vector_player.is_some() {
            screen_h - STATUS_BAR_HEIGHT - VECTOR_PLAYER_HEIGHT
        } else {
            screen_h - STATUS_BAR_HEIGHT
        };

        // ── Sidebar (activity bar always visible + collapsible content panel) ──
        let sidebar_bottom = viewport_bottom;
        let sidebar_content_h = (sidebar_bottom - content_top - 16.0).max(100.0);
        let sidebar_show_ticks = sidebar_content_h > GRADIENT_TICKS_MIN_HEIGHT;
        let sidebar = SidebarRegion {
            content_visible,
            activity_bar_width: ACTIVITY_BAR_WIDTH,
            content_width,
            total_width: sidebar_width,
            top: content_top,
            bottom: sidebar_bottom,
            content_height: sidebar_content_h,
            show_gradient_ticks: sidebar_show_ticks,
        };

        // ── Layer slider (right, inline, displaces viewport) ──
        let layer_slider_right_width = if flags.has_layers {
            LAYER_SLIDER_WIDTH + PANEL_MARGIN
        } else {
            0.0
        };

        let layer_slider = if flags.has_layers {
            let slider_top = content_top + PANEL_MARGIN;
            let available_h = (viewport_bottom - slider_top - BOTTOM_MARGIN).max(120.0);
            let frame_overhead = 16.0;
            let content_h = available_h - frame_overhead;
            let show_goto = content_h > GOTO_SECTION_MIN_HEIGHT;

            Some(LayerSliderRegion {
                pos: egui::pos2(
                    screen.right() - PANEL_MARGIN - LAYER_SLIDER_WIDTH,
                    slider_top,
                ),
                content_height: content_h,
                width: LAYER_SLIDER_WIDTH,
                show_goto,
            })
        } else {
            None
        };

        // ── Tool panel (floating, to the left of layer slider) ──
        // Width = button size + inner_margin (4+4) = 36
        let tool_panel_width = TOOL_BTN_SIZE + 8.0;
        let tool_panel_right_offset = layer_slider_right_width + PANEL_GAP;
        let tool_panel_left = screen.right() - tool_panel_right_offset - tool_panel_width;
        let tool_panel_top = content_top + PANEL_MARGIN; // same top as layer slider
        let tool_panel = ToolPanelRegion {
            anchor_pos: egui::pos2(tool_panel_left, tool_panel_top),
            top: tool_panel_top,
        };

        // ── Scale bar ──
        let scale_bar = if flags.show_scale_bar {
            Some(Rect::from_min_size(
                egui::pos2(
                    sidebar_width + SCALE_BAR_MARGIN,
                    viewport_bottom - SCALE_BAR_MARGIN - 30.0,
                ),
                egui::vec2(220.0, 30.0),
            ))
        } else {
            None
        };

        // ── Viewport ──
        // The viewport displaces for sidebar (left), layer slider (right),
        // toolbar (top), vector player + status bar (bottom).
        let viewport = Rect::from_min_max(
            egui::pos2(screen.left() + sidebar_width, content_top),
            egui::pos2(screen.right() - layer_slider_right_width, viewport_bottom),
        );

        Self {
            screen,
            toolbar,
            compact_toolbar,
            sidebar,
            tool_panel,
            layer_slider,
            status_bar,
            vector_player,
            scale_bar,
            tab_bar,
            viewport,
        }
    }

    /// Get the effective sidebar width (activity bar + content when open)
    pub fn sidebar_width(&self) -> f32 {
        self.sidebar.total_width
    }

    /// Get the bottom edge of the viewport (for coordinate transforms)
    pub fn viewport_bottom(&self) -> f32 {
        self.viewport.bottom()
    }
}
