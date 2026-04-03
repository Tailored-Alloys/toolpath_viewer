//! Layout Region System
//!
//! Centralizes all layout constants and computes non-overlapping screen regions
//! for each UI section. Regions are recomputed each frame from the current screen
//! size and visibility flags (immediate-mode compatible).

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

/// Width of the left gradient scale panel
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

/// Compact toolbar width threshold
pub const COMPACT_TOOLBAR_THRESHOLD: f32 = 900.0;

/// Minimum height for layer slider content to show goto section
pub const GOTO_SECTION_MIN_HEIGHT: f32 = 250.0;

/// Minimum height for gradient ticks to be visible
pub const GRADIENT_TICKS_MIN_HEIGHT: f32 = 350.0;

// ── Visibility flags ─────────────────────────────────────────────────────

/// Flags controlling which optional panels are visible, affecting region computation.
#[derive(Debug, Clone, Copy)]
pub struct VisibilityFlags {
    /// Whether a file is loaded (controls layer slider visibility)
    pub has_layers: bool,
    /// Whether a parameter mode is active (controls gradient panel visibility)
    pub show_gradient: bool,
    /// Whether wait markers are active (controls wait gradient panel visibility)
    pub show_wait_gradient: bool,
    /// Whether the scale bar is visible (affects gradient bottom margin)
    pub show_scale_bar: bool,
    /// Whether file info popup is open
    pub show_file_info: bool,
    /// Whether controls popup is open
    pub show_controls: bool,
    /// Whether the grid is enabled (affects grid label overlay)
    pub show_grid: bool,
    /// Whether vector-by-vector view is active (controls vector slider visibility)
    pub vector_view_active: bool,
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

    /// Tool panel — top-right, horizontal strip left of slider
    pub tool_panel: ToolPanelRegion,

    /// Right layer slider — full height right side, below tool panel
    pub layer_slider: Option<LayerSliderRegion>,

    /// Vector slider — right side, left of layer slider (visible when vector view active)
    pub vector_slider: Option<VectorSliderRegion>,

    /// Left gradient scale — full height left side
    pub gradient_panel: Option<GradientRegion>,

    /// Wait time gradient scale — next to gradient panel (or same position if no param gradient)
    pub wait_gradient_panel: Option<GradientRegion>,

    /// Scale bar — bottom-left corner
    pub scale_bar: Option<Rect>,

    /// Viewport — the central area where GL content renders
    pub viewport: Rect,
}

/// Tool panel positioning info
#[derive(Debug, Clone)]
pub struct ToolPanelRegion {
    /// Position for the egui Area (anchor point, right-top pivot)
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

/// Gradient panel positioning info
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

/// Vector slider positioning info
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
    /// Regions are computed relative to each other to prevent overlap:
    /// - Toolbar is always at the top
    /// - Tool panel is below toolbar, right-aligned, left of slider
    /// - Layer slider is right side, below tool panel
    /// - Gradient panel is left side, below toolbar
    /// - Scale bar is bottom-left
    /// - Viewport fills the remaining central area
    pub fn compute(screen: Rect, flags: &VisibilityFlags) -> Self {
        let screen_w = screen.width();
        let screen_h = screen.height();
        let compact_toolbar = screen_w < COMPACT_TOOLBAR_THRESHOLD;

        // ── Toolbar ──
        let toolbar = Rect::from_min_size(
            screen.left_top(),
            egui::vec2(screen_w, TOOLBAR_HEIGHT),
        );

        // ── Tool panel ──
        // Positioned top-right, left of the layer slider area
        let right_panels_width = if flags.vector_view_active && flags.has_layers {
            LAYER_SLIDER_WIDTH + PANEL_GAP + LAYER_SLIDER_WIDTH
        } else {
            LAYER_SLIDER_WIDTH
        };
        let tool_panel_right_offset = right_panels_width + PANEL_MARGIN + PANEL_GAP;
        let tool_panel_top = TOOLBAR_BOTTOM + PANEL_GAP;
        let tool_panel = ToolPanelRegion {
            anchor_pos: egui::pos2(screen.right() - tool_panel_right_offset, tool_panel_top),
            top: tool_panel_top,
        };

        // ── Layer slider ──
        let layer_slider = if flags.has_layers {
            let slider_top = TOOLBAR_BOTTOM + PANEL_MARGIN;
            let available_h = (screen_h - slider_top - BOTTOM_MARGIN).max(120.0);
            let frame_overhead = 16.0; // inner_margin(8,8)
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

        // ── Vector slider ──
        let vector_slider = if flags.has_layers && flags.vector_view_active {
            let slider_top = TOOLBAR_BOTTOM + PANEL_MARGIN;
            let available_h = (screen_h - slider_top - BOTTOM_MARGIN).max(120.0);
            let frame_overhead = 16.0;
            let content_h = available_h - frame_overhead;
            let show_goto = content_h > GOTO_SECTION_MIN_HEIGHT;

            Some(VectorSliderRegion {
                pos: egui::pos2(
                    screen.right() - PANEL_MARGIN - LAYER_SLIDER_WIDTH - PANEL_GAP - LAYER_SLIDER_WIDTH,
                    slider_top,
                ),
                content_height: content_h,
                width: LAYER_SLIDER_WIDTH,
                show_goto,
            })
        } else {
            None
        };

        // ── Gradient panel ──
        let gradient_panel = if flags.show_gradient {
            let grad_top = TOOLBAR_BOTTOM + PANEL_MARGIN;
            let grad_bottom_margin = BOTTOM_MARGIN;
            let available_h = (screen_h - grad_top - grad_bottom_margin).max(200.0);
            let frame_overhead = 16.0; // inner_margin(12h, 8v) → 16px vertical
            let content_h = available_h - frame_overhead;
            let show_ticks = available_h > GRADIENT_TICKS_MIN_HEIGHT;

            Some(GradientRegion {
                pos: egui::pos2(PANEL_MARGIN, grad_top),
                content_height: content_h,
                width: GRADIENT_PANEL_WIDTH,
                show_ticks,
            })
        } else {
            None
        };

        // ── Wait gradient panel ──
        let wait_gradient_panel = if flags.show_wait_gradient {
            let grad_top = TOOLBAR_BOTTOM + PANEL_MARGIN;
            let grad_bottom_margin = BOTTOM_MARGIN;
            let available_h = (screen_h - grad_top - grad_bottom_margin).max(200.0);
            let frame_overhead = 16.0;
            let content_h = available_h - frame_overhead;
            let show_ticks = available_h > GRADIENT_TICKS_MIN_HEIGHT;
            // Position to the right of the param gradient panel if it's visible
            let x = if flags.show_gradient {
                PANEL_MARGIN + GRADIENT_PANEL_WIDTH + PANEL_GAP
            } else {
                PANEL_MARGIN
            };

            Some(GradientRegion {
                pos: egui::pos2(x, grad_top),
                content_height: content_h,
                width: GRADIENT_PANEL_WIDTH,
                show_ticks,
            })
        } else {
            None
        };

        // ── Scale bar ──
        let scale_bar = if flags.show_scale_bar {
            Some(Rect::from_min_size(
                egui::pos2(SCALE_BAR_MARGIN, screen_h - SCALE_BAR_MARGIN - 30.0),
                egui::vec2(220.0, 30.0),
            ))
        } else {
            None
        };

        // ── Viewport ──
        // The viewport is the full area below the toolbar.
        // Floating panels overlay it but don't reduce it.
        let viewport = Rect::from_min_max(
            egui::pos2(screen.left(), toolbar.bottom()),
            screen.right_bottom(),
        );

        let regions = Self {
            screen,
            toolbar,
            compact_toolbar,
            tool_panel,
            layer_slider,
            vector_slider,
            gradient_panel,
            wait_gradient_panel,
            scale_bar,
            viewport,
        };

        #[cfg(debug_assertions)]
        regions.validate_no_overlap();

        regions
    }

    /// In debug builds, validate that no two solid panels overlap.
    #[cfg(debug_assertions)]
    fn validate_no_overlap(&self) {
        let mut rects: Vec<(&str, Rect)> = vec![("toolbar", self.toolbar)];

        if let Some(ref slider) = self.layer_slider {
            let r = Rect::from_min_size(slider.pos, egui::vec2(slider.width, slider.content_height + 16.0));
            rects.push(("layer_slider", r));
        }
        if let Some(ref vs) = self.vector_slider {
            let r = Rect::from_min_size(vs.pos, egui::vec2(vs.width, vs.content_height + 16.0));
            rects.push(("vector_slider", r));
        }
        if let Some(ref grad) = self.gradient_panel {
            let r = Rect::from_min_size(grad.pos, egui::vec2(grad.width, grad.content_height + 16.0));
            rects.push(("gradient_panel", r));
        }
        if let Some(ref wg) = self.wait_gradient_panel {
            let r = Rect::from_min_size(wg.pos, egui::vec2(wg.width, wg.content_height + 16.0));
            rects.push(("wait_gradient_panel", r));
        }

        // Check pairwise (skip toolbar vs floating panels since they overlay the viewport)
        for i in 1..rects.len() {
            for j in (i + 1)..rects.len() {
                let (name_a, rect_a) = &rects[i];
                let (name_b, rect_b) = &rects[j];
                debug_assert!(
                    !rect_a.intersects(*rect_b),
                    "Layout overlap detected between {} and {}: {:?} vs {:?}",
                    name_a, name_b, rect_a, rect_b
                );
            }
        }
    }
}
