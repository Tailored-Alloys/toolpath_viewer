//! Custom cursor rendering.
//!
//! egui 0.27 has no custom-cursor-image API, so we hide the OS cursor with
//! `CursorIcon::None` and paint the cursor directly onto the `Tooltip` layer.
//!
//! Cursor catalogue:
//!  - **Pan idle**     — 4-way move arrows (think Inkscape selector)
//!  - **Pan drag**     — horizontal 2-headed arrow + vertical grip lines
//!  - **ZoomSelect idle** — magnifier circle with cross inside + handle
//!  - **ZoomSelect drag** — precision crosshair (thin, with centre gap)
//!  - **Ruler**        — precision crosshair + small diagonal ruler icon

use egui::{Color32, Id, LayerId, Order, Painter, Pos2, Stroke, Vec2};

use super::ui::ToolMode;

// ── palette ──────────────────────────────────────────────────────────────────

const SHADOW: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 210);
const FG: Color32 = Color32::WHITE;

// ── public entry point ────────────────────────────────────────────────────────

/// Hide the OS cursor and paint the appropriate custom cursor for the current
/// tool mode.  Call this only while the pointer is over the viewport (i.e.
/// `!ctx.wants_pointer_input()` is true).
pub fn draw_custom_cursor(ctx: &egui::Context, mode: ToolMode, is_dragging: bool) {
    ctx.set_cursor_icon(egui::CursorIcon::None);

    let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else {
        return;
    };

    let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new("__custom_cursor")));

    match mode {
        ToolMode::Pan => {
            if is_dragging {
                draw_grab_cursor(&painter, pos);
            } else {
                draw_move_cursor(&painter, pos);
            }
        }
        ToolMode::ZoomSelect => {
            if is_dragging {
                draw_precise_crosshair(&painter, pos);
            } else {
                draw_zoom_cursor(&painter, pos);
            }
        }
        ToolMode::Ruler => draw_ruler_cursor(&painter, pos),
    }
}

// ── internal helpers ─────────────────────────────────────────────────────────

fn line_s(painter: &Painter, p1: Pos2, p2: Pos2, color: Color32, width: f32) {
    painter.line_segment([p1, p2], Stroke::new(width, color));
}

/// Draw one shadow pass then one foreground pass of a line.
fn line2(painter: &Painter, p1: Pos2, p2: Pos2) {
    line_s(painter, p1, p2, SHADOW, 3.0);
    line_s(painter, p1, p2, FG, 1.5);
}

/// Filled arrowhead triangle at `tip` pointing in direction `(dx, dy)`.
fn arrowhead(painter: &Painter, tip: Pos2, dx: f32, dy: f32, color: Color32) {
    let sz = 4.5_f32;
    let depth = 5.5_f32;
    let (px, py) = (-dy, dx); // perpendicular to (dx,dy)
    let pts = vec![
        tip,
        Pos2::new(tip.x - dx * depth + px * sz, tip.y - dy * depth + py * sz),
        Pos2::new(tip.x - dx * depth - px * sz, tip.y - dy * depth - py * sz),
    ];
    painter.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
}

// ── cursor shapes ─────────────────────────────────────────────────────────────

/// 4-way move cursor — used for **Pan idle**.
fn draw_move_cursor(painter: &Painter, pos: Pos2) {
    let gap = 4.5_f32;
    let arm = 13.0_f32;
    let dirs: [(f32, f32); 4] = [(0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0)];

    // Shadow pass first so white sits on top
    for &(dx, dy) in &dirs {
        let start = pos + Vec2::new(dx * gap, dy * gap);
        let tip = pos + Vec2::new(dx * arm, dy * arm);
        line_s(painter, start, tip, SHADOW, 3.0);
        arrowhead(painter, tip, dx, dy, SHADOW);
    }
    for &(dx, dy) in &dirs {
        let start = pos + Vec2::new(dx * gap, dy * gap);
        let tip = pos + Vec2::new(dx * arm, dy * arm);
        line_s(painter, start, tip, FG, 1.5);
        arrowhead(painter, tip, dx, dy, FG);
    }

    // Centre dot
    painter.circle_filled(pos, 2.5, SHADOW);
    painter.circle_filled(pos, 1.5, FG);
}

/// Horizontal two-headed arrow + vertical grip stripes — used for **Pan drag**.
fn draw_grab_cursor(painter: &Painter, pos: Pos2) {
    let gap = 3.5_f32;
    let arm = 10.0_f32;

    // Arrow shafts & heads
    for &(dx, dy) in &[(-1.0_f32, 0.0_f32), (1.0, 0.0)] {
        let start = pos + Vec2::new(dx * gap, dy * gap);
        let tip = pos + Vec2::new(dx * arm, dy * arm);
        line_s(painter, start, tip, SHADOW, 3.0);
        arrowhead(painter, tip, dx, dy, SHADOW);
    }
    for &(dx, dy) in &[(-1.0_f32, 0.0_f32), (1.0, 0.0)] {
        let start = pos + Vec2::new(dx * gap, dy * gap);
        let tip = pos + Vec2::new(dx * arm, dy * arm);
        line_s(painter, start, tip, FG, 1.5);
        arrowhead(painter, tip, dx, dy, FG);
    }

    // Vertical grip stripes (shadow then white)
    for &x in &[-3.0_f32, 0.0, 3.0] {
        let top = pos + Vec2::new(x, -5.0);
        let bot = pos + Vec2::new(x, 5.0);
        line_s(painter, top, bot, SHADOW, 2.5);
    }
    for &x in &[-3.0_f32, 0.0, 3.0] {
        let top = pos + Vec2::new(x, -5.0);
        let bot = pos + Vec2::new(x, 5.0);
        line_s(painter, top, bot, FG, 1.5);
    }
}

/// Magnifier with cross inside — used for **ZoomSelect idle**.
fn draw_zoom_cursor(painter: &Painter, pos: Pos2) {
    // Hotspot (pos) is the pointer tip; glass centre is offset top-left
    let center = pos + Vec2::new(-3.0, -3.0);
    let r = 7.5_f32;
    let ci = r * 0.58;
    let ha = center + Vec2::splat(r * 0.68);
    let hb = center + Vec2::splat(r * 0.68 + 7.0);

    // Shadow pass
    painter.circle_stroke(center, r, Stroke::new(3.0, SHADOW));
    line_s(painter, ha, hb, SHADOW, 3.5);
    line_s(painter, center + Vec2::new(-ci, 0.0), center + Vec2::new(ci, 0.0), SHADOW, 2.5);
    line_s(painter, center + Vec2::new(0.0, -ci), center + Vec2::new(0.0, ci), SHADOW, 2.5);

    // White pass
    painter.circle_stroke(center, r, Stroke::new(1.5, FG));
    line_s(painter, ha, hb, FG, 2.0);
    line_s(painter, center + Vec2::new(-ci, 0.0), center + Vec2::new(ci, 0.0), FG, 1.5);
    line_s(painter, center + Vec2::new(0.0, -ci), center + Vec2::new(0.0, ci), FG, 1.5);
}

/// Precision crosshair with centre-gap circle — used for **ZoomSelect drag** and
/// as the base for the ruler cursor.
fn draw_precise_crosshair(painter: &Painter, pos: Pos2) {
    let arm = 13.0_f32;
    let gap = 3.5_f32;

    let arms: [(Pos2, Pos2); 4] = [
        (pos + Vec2::new(-arm, 0.0), pos + Vec2::new(-gap, 0.0)),
        (pos + Vec2::new(gap, 0.0), pos + Vec2::new(arm, 0.0)),
        (pos + Vec2::new(0.0, -arm), pos + Vec2::new(0.0, -gap)),
        (pos + Vec2::new(0.0, gap), pos + Vec2::new(0.0, arm)),
    ];

    for &(a, b) in &arms {
        line_s(painter, a, b, SHADOW, 3.0);
    }
    for &(a, b) in &arms {
        line_s(painter, a, b, FG, 1.5);
    }
    painter.circle_stroke(pos, gap - 0.5, Stroke::new(1.5, SHADOW));
    painter.circle_stroke(pos, gap - 0.5, Stroke::new(1.5, FG));
}

/// Precision crosshair + small diagonal ruler icon — used for **Ruler** mode.
fn draw_ruler_cursor(painter: &Painter, pos: Pos2) {
    draw_precise_crosshair(painter, pos);

    // Ruler body: rotated rectangle at 45° offset to bottom-right
    let base = pos + Vec2::new(9.0, 9.0);
    let len = 10.0_f32;
    let half_w = 2.5_f32;
    let a45 = std::f32::consts::FRAC_PI_4;
    let (sa, ca) = a45.sin_cos();
    let along = Vec2::new(ca, sa);
    let perp = Vec2::new(-sa, ca);

    let c = [
        base - perp * half_w,
        base - perp * half_w + along * len,
        base + perp * half_w + along * len,
        base + perp * half_w,
    ];

    // Shadow outline
    for i in 0..4 {
        line_s(painter, c[i], c[(i + 1) % 4], SHADOW, 2.5);
    }
    // White outline
    for i in 0..4 {
        line_s(painter, c[i], c[(i + 1) % 4], FG, 1.5);
    }

    // Tick marks along the ruler body
    for &t in &[0.25_f32, 0.5, 0.75] {
        let mid = base + along * (len * t);
        let tick = if (t - 0.5).abs() < 0.01 { 2.5_f32 } else { 1.5_f32 };
        line2(painter, mid - perp * tick, mid + perp * tick);
    }
}
