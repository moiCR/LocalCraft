use gpui::{
    Bounds, Corners, PathBuilder, Pixels, Point, Size, Window, fill, point, px, rgba, size,
};
use services::topology::{NodePosition, PinSide};
use ui::theme::Palette;

pub const NODE_WIDTH: f32 = 264.0;
pub const NODE_HEIGHT: f32 = 112.0;
pub const MIN_ZOOM: f32 = 0.35;
pub const MAX_ZOOM: f32 = 2.0;
pub const GRID_STEP: f32 = 28.0;

#[derive(Clone, Copy)]
pub struct PaintRope {
    pub from: Point<Pixels>,
    pub from_side: PinSide,
    pub to: Point<Pixels>,
    pub to_side: PinSide,
    pub active: bool,
    pub hovered: bool,
}

#[derive(Clone, Copy)]
pub struct DraftRope {
    pub from: Point<Pixels>,
    pub from_side: PinSide,
    pub cursor: Point<Pixels>,
    pub snap_side: Option<PinSide>,
}

pub fn world_to_screen(
    world: Point<f32>,
    pan_x: f32,
    pan_y: f32,
    zoom: f32,
    origin: Point<Pixels>,
) -> Point<Pixels> {
    point(
        px(f32::from(origin.x) + pan_x + world.x * zoom),
        px(f32::from(origin.y) + pan_y + world.y * zoom),
    )
}

#[cfg(test)]
pub fn screen_to_world(
    screen: Point<Pixels>,
    pan_x: f32,
    pan_y: f32,
    zoom: f32,
    origin: Point<Pixels>,
) -> Point<f32> {
    let safe_zoom = zoom.max(0.01);
    point(
        (f32::from(screen.x) - f32::from(origin.x) - pan_x) / safe_zoom,
        (f32::from(screen.y) - f32::from(origin.y) - pan_y) / safe_zoom,
    )
}

pub fn pin_world_position(pos: NodePosition, side: PinSide) -> Point<f32> {
    let x = match side {
        PinSide::Left => pos.x,
        PinSide::Right => pos.x + NODE_WIDTH,
    };
    point(x, pos.y + NODE_HEIGHT * 0.5)
}

pub fn pin_screen_position(
    pos: NodePosition,
    side: PinSide,
    pan_x: f32,
    pan_y: f32,
    zoom: f32,
    origin: Point<Pixels>,
) -> Point<Pixels> {
    world_to_screen(pin_world_position(pos, side), pan_x, pan_y, zoom, origin)
}

pub fn fit_viewport<'a>(
    positions: impl Iterator<Item = &'a NodePosition>,
    viewport_size: Size<Pixels>,
) -> (f32, f32, f32) {
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut count = 0usize;

    for pos in positions {
        min_x = min_x.min(pos.x);
        max_x = max_x.max(pos.x + NODE_WIDTH);
        min_y = min_y.min(pos.y);
        max_y = max_y.max(pos.y + NODE_HEIGHT);
        count += 1;
    }

    if count == 0 {
        return (0.0, 0.0, 1.0);
    }

    let content_w = (max_x - min_x).max(NODE_WIDTH);
    let content_h = (max_y - min_y).max(NODE_HEIGHT);
    let padding = 72.0;
    let avail_w = (f32::from(viewport_size.width) - padding * 2.0).max(160.0);
    let avail_h = (f32::from(viewport_size.height) - padding * 2.0).max(120.0);
    let zoom = (avail_w / content_w)
        .min(avail_h / content_h)
        .clamp(MIN_ZOOM, 1.15);
    let pan_x = (f32::from(viewport_size.width) - content_w * zoom) * 0.5 - min_x * zoom;
    let pan_y = (f32::from(viewport_size.height) - content_h * zoom) * 0.5 - min_y * zoom;
    (pan_x, pan_y, zoom)
}

pub fn rope_cubic(
    from: Point<Pixels>,
    from_side: PinSide,
    to: Point<Pixels>,
    to_side: Option<PinSide>,
    zoom: f32,
) -> (Point<Pixels>, Point<Pixels>, Point<Pixels>, Point<Pixels>) {
    let fx = f32::from(from.x);
    let fy = f32::from(from.y);
    let tx = f32::from(to.x);
    let ty = f32::from(to.y);
    let dx = tx - fx;
    let dy = ty - fy;
    let dist = (dx * dx + dy * dy).sqrt();
    let sag = (dist * 0.24 + 26.0 * zoom).clamp(18.0 * zoom, 150.0 * zoom);
    let tangent = (dist * 0.16).clamp(16.0 * zoom, 64.0 * zoom);
    let dir_a = side_direction(from_side);
    let c1 = point(px(fx + dir_a * tangent), px(fy + sag));
    let c2 = match to_side {
        Some(side) => {
            let dir_b = side_direction(side);
            point(px(tx + dir_b * tangent), px(ty + sag))
        }
        None => point(px(tx), px(ty + sag * 0.5)),
    };
    (from, c1, c2, to)
}

fn side_direction(side: PinSide) -> f32 {
    match side {
        PinSide::Left => -1.0,
        PinSide::Right => 1.0,
    }
}

pub fn hit_test_rope(
    cursor: Point<Pixels>,
    p0: Point<Pixels>,
    c1: Point<Pixels>,
    c2: Point<Pixels>,
    p3: Point<Pixels>,
    threshold: f32,
) -> bool {
    let cx = f32::from(cursor.x);
    let cy = f32::from(cursor.y);
    let threshold_sq = threshold * threshold;
    let segments = 24usize;
    let mut prev = eval_cubic(p0, c1, c2, p3, 0.0);
    for step in 1..=segments {
        let t = step as f32 / segments as f32;
        let curr = eval_cubic(p0, c1, c2, p3, t);
        if point_segment_dist_sq(cx, cy, prev.0, prev.1, curr.0, curr.1) <= threshold_sq {
            return true;
        }
        prev = curr;
    }
    false
}

fn eval_cubic(
    p0: Point<Pixels>,
    c1: Point<Pixels>,
    c2: Point<Pixels>,
    p3: Point<Pixels>,
    t: f32,
) -> (f32, f32) {
    let u = 1.0 - t;
    let tt = t * t;
    let uu = u * u;
    let uuu = uu * u;
    let ttt = tt * t;
    let x = uuu * f32::from(p0.x)
        + 3.0 * uu * t * f32::from(c1.x)
        + 3.0 * u * tt * f32::from(c2.x)
        + ttt * f32::from(p3.x);
    let y = uuu * f32::from(p0.y)
        + 3.0 * uu * t * f32::from(c1.y)
        + 3.0 * u * tt * f32::from(c2.y)
        + ttt * f32::from(p3.y);
    (x, y)
}

fn point_segment_dist_sq(px_pt: f32, py_pt: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let abx = bx - ax;
    let aby = by - ay;
    let len_sq = abx * abx + aby * aby;
    if len_sq <= f32::EPSILON {
        let dx = px_pt - ax;
        let dy = py_pt - ay;
        return dx * dx + dy * dy;
    }
    let t = (((px_pt - ax) * abx + (py_pt - ay) * aby) / len_sq).clamp(0.0, 1.0);
    let proj_x = ax + t * abx;
    let proj_y = ay + t * aby;
    let dx = px_pt - proj_x;
    let dy = py_pt - proj_y;
    dx * dx + dy * dy
}

#[allow(clippy::too_many_arguments)]
pub fn paint_canvas(
    bounds: Bounds<Pixels>,
    pan_x: f32,
    pan_y: f32,
    zoom: f32,
    palette: Palette,
    ropes: &[PaintRope],
    draft: Option<DraftRope>,
    window: &mut Window,
) {
    paint_dot_grid(bounds, pan_x, pan_y, zoom, palette, window);

    for rope in ropes {
        let (p0, c1, c2, p3) =
            rope_cubic(rope.from, rope.from_side, rope.to, Some(rope.to_side), zoom);
        let shadow_offset = px(3.0 * zoom);
        paint_cubic_stroke(
            point(p0.x, p0.y + shadow_offset),
            point(c1.x, c1.y + shadow_offset),
            point(c2.x, c2.y + shadow_offset),
            point(p3.x, p3.y + shadow_offset),
            (3.5 * zoom).max(2.0),
            rgba(0x00000044),
            window,
        );

        let color = if rope.hovered {
            palette.error
        } else if rope.active {
            palette.running
        } else {
            palette.accent.opacity(0.8)
        };
        let width = if rope.hovered {
            (3.2 * zoom).max(2.0)
        } else {
            (2.5 * zoom).max(1.5)
        };
        paint_cubic_stroke(p0, c1, c2, p3, width, color, window);

        let knot_radius = (3.5 * zoom).clamp(2.5, 5.5);
        paint_circle(p0, knot_radius, color, window);
        paint_circle(p3, knot_radius, color, window);
    }

    if let Some(draft) = draft {
        let (p0, c1, c2, p3) = rope_cubic(
            draft.from,
            draft.from_side,
            draft.cursor,
            draft.snap_side,
            zoom,
        );
        let dash = px((6.0 * zoom).max(3.0));
        let gap = px((4.0 * zoom).max(2.0));
        let mut builder = PathBuilder::stroke(px((2.5 * zoom).max(1.5))).dash_array(&[dash, gap]);
        builder.move_to(p0);
        builder.cubic_bezier_to(p3, c1, c2);
        if let Ok(path) = builder.build() {
            window.paint_path(path, palette.accent);
        }
        let knot_radius = (4.0 * zoom).clamp(3.0, 6.0);
        paint_circle(p0, knot_radius, palette.accent, window);
        paint_circle(p3, knot_radius, palette.accent, window);
    }
}

fn paint_dot_grid(
    bounds: Bounds<Pixels>,
    pan_x: f32,
    pan_y: f32,
    zoom: f32,
    palette: Palette,
    window: &mut Window,
) {
    let mut step = GRID_STEP * zoom;
    while step < 10.0 {
        step *= 2.0;
    }

    let origin_x = f32::from(bounds.origin.x);
    let origin_y = f32::from(bounds.origin.y);
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let offset_x = pan_x.rem_euclid(step);
    let offset_y = pan_y.rem_euclid(step);
    let dot_radius = (1.35 * zoom.sqrt()).clamp(1.0, 2.5);
    let dot_color = palette.border.opacity(0.65);

    let mut x = offset_x;
    while x <= width {
        let mut y = offset_y;
        while y <= height {
            paint_circle(
                point(px(origin_x + x), px(origin_y + y)),
                dot_radius,
                dot_color,
                window,
            );
            y += step;
        }
        x += step;
    }
}

fn paint_circle(
    center: Point<Pixels>,
    radius: f32,
    color: impl Into<gpui::Background>,
    window: &mut Window,
) {
    let diameter = radius * 2.0;
    let quad_bounds = Bounds::new(
        point(
            px(f32::from(center.x) - radius),
            px(f32::from(center.y) - radius),
        ),
        size(px(diameter), px(diameter)),
    );
    let mut quad = fill(quad_bounds, color);
    quad.corner_radii = Corners::all(px(radius));
    window.paint_quad(quad);
}

fn paint_cubic_stroke(
    p0: Point<Pixels>,
    c1: Point<Pixels>,
    c2: Point<Pixels>,
    p3: Point<Pixels>,
    width: f32,
    color: impl Into<gpui::Background>,
    window: &mut Window,
) {
    let mut builder = PathBuilder::stroke(px(width));
    builder.move_to(p0);
    builder.cubic_bezier_to(p3, c1, c2);
    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_and_screen_coordinates_round_trip() {
        let origin = point(px(120.0), px(48.0));
        let world = point(340.0_f32, -95.5_f32);
        let screen = world_to_screen(world, 64.0, -18.0, 1.35, origin);
        let recovered = screen_to_world(screen, 64.0, -18.0, 1.35, origin);
        assert!((recovered.x - world.x).abs() < 0.01);
        assert!((recovered.y - world.y).abs() < 0.01);
    }

    #[test]
    fn hit_test_rope_detects_curve_hits_and_misses() {
        let p0 = point(px(100.0), px(100.0));
        let p3 = point(px(400.0), px(100.0));
        let (p0, c1, c2, p3) = rope_cubic(p0, PinSide::Right, p3, Some(PinSide::Left), 1.0);
        let mid = eval_cubic(p0, c1, c2, p3, 0.5);
        assert!(hit_test_rope(
            point(px(mid.0), px(mid.1 + 4.0)),
            p0,
            c1,
            c2,
            p3,
            9.0
        ));
        assert!(!hit_test_rope(
            point(px(mid.0), px(mid.1 - 80.0)),
            p0,
            c1,
            c2,
            p3,
            9.0
        ));
    }

    #[test]
    fn fit_viewport_centers_nodes_within_bounds() {
        let positions = [
            NodePosition { x: 0.0, y: 0.0 },
            NodePosition { x: 600.0, y: 240.0 },
        ];
        let (pan_x, pan_y, zoom) = fit_viewport(positions.iter(), size(px(1000.0), px(640.0)));
        assert!((MIN_ZOOM..=1.15).contains(&zoom));
        assert!(pan_x > 0.0);
        assert!(pan_y > 0.0);
    }
}
