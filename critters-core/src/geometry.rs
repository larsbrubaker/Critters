//! Polygon helpers ported from Matter.js `Vertices` / `Bodies` so a piece's
//! outline is exactly what the original drew: the chamfered rectangle from
//! `Vertices.chamfer`, the clockwise vertex order from `Bodies.fromVertices`,
//! and the centroid re-centring done by `Body.setVertices`. `piece.rs` keeps
//! these local outlines; `physics.rs` builds the Box2D shapes from the same
//! data so what is drawn is what collides.

use crate::config::ShapeKind;

/// A 2-D point/vector in game space (y down).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct P {
    pub x: f64,
    pub y: f64,
}

impl P {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Axis-aligned bounds, like Matter's `body.bounds`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bounds {
    pub min: P,
    pub max: P,
}

impl Bounds {
    /// `Bounds.update`: extents of the vertices, extended by the velocity so a
    /// fast body's bounds cover where it is heading.
    pub fn of(vertices: &[P], velocity: P) -> Self {
        let mut b = Bounds {
            min: P::new(f64::INFINITY, f64::INFINITY),
            max: P::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        };
        for v in vertices {
            if v.x > b.max.x {
                b.max.x = v.x;
            }
            if v.x < b.min.x {
                b.min.x = v.x;
            }
            if v.y > b.max.y {
                b.max.y = v.y;
            }
            if v.y < b.min.y {
                b.min.y = v.y;
            }
        }
        if velocity.x > 0.0 {
            b.max.x += velocity.x;
        } else {
            b.min.x += velocity.x;
        }
        if velocity.y > 0.0 {
            b.max.y += velocity.y;
        } else {
            b.min.y += velocity.y;
        }
        b
    }
}

fn normalise(x: f64, y: f64) -> P {
    let m = (x * x + y * y).sqrt();
    if m == 0.0 {
        P::new(0.0, 0.0)
    } else {
        P::new(x / m, y / m)
    }
}

/// `Vertices.chamfer` with the default quality (-1 → derived from the radius).
pub fn chamfer(vertices: &[P], radius: f64) -> Vec<P> {
    let n = vertices.len();
    let mut out = Vec::new();
    for i in 0..n {
        let prev = vertices[(i + n - 1) % n];
        let vertex = vertices[i];
        let next = vertices[(i + 1) % n];
        if radius == 0.0 {
            out.push(vertex);
            continue;
        }
        let prev_normal = normalise(vertex.y - prev.y, prev.x - vertex.x);
        let next_normal = normalise(next.y - vertex.y, vertex.x - next.x);
        let diagonal_radius = (2.0 * radius * radius).sqrt();
        let radius_vector = P::new(prev_normal.x * radius, prev_normal.y * radius);
        let mid_normal = normalise(
            (prev_normal.x + next_normal.x) * 0.5,
            (prev_normal.y + next_normal.y) * 0.5,
        );
        let scaled = P::new(
            vertex.x - mid_normal.x * diagonal_radius,
            vertex.y - mid_normal.y * diagonal_radius,
        );
        let mut precision = radius.powf(0.32) * 1.75;
        precision = precision.clamp(2.0, 14.0);
        if precision % 2.0 == 1.0 {
            precision += 1.0;
        }
        let alpha = (prev_normal.x * next_normal.x + prev_normal.y * next_normal.y).acos();
        let theta = alpha / precision;
        let mut j = 0.0;
        while j < precision {
            let (s, c) = (theta * j).sin_cos();
            out.push(P::new(
                radius_vector.x * c - radius_vector.y * s + scaled.x,
                radius_vector.x * s + radius_vector.y * c + scaled.y,
            ));
            j += 1.0;
        }
    }
    out
}

/// `Vertices.area(vertices, signed)`.
pub fn area(vertices: &[P], signed: bool) -> f64 {
    let mut a = 0.0;
    let mut j = vertices.len() - 1;
    for i in 0..vertices.len() {
        a += (vertices[j].x - vertices[i].x) * (vertices[j].y + vertices[i].y);
        j = i;
    }
    if signed {
        a / 2.0
    } else {
        a.abs() / 2.0
    }
}

/// `Vertices.centre`: the polygon centroid.
pub fn centre(vertices: &[P]) -> P {
    let a = area(vertices, true);
    let mut cx = 0.0;
    let mut cy = 0.0;
    for i in 0..vertices.len() {
        let j = (i + 1) % vertices.len();
        let cross = vertices[i].x * vertices[j].y - vertices[i].y * vertices[j].x;
        cx += (vertices[i].x + vertices[j].x) * cross;
        cy += (vertices[i].y + vertices[j].y) * cross;
    }
    P::new(cx / (6.0 * a), cy / (6.0 * a))
}

/// `Vertices.clockwiseSort`: order by angle around the mean point.
pub fn clockwise_sort(vertices: &mut [P]) {
    let n = vertices.len() as f64;
    let mean = vertices
        .iter()
        .fold(P::default(), |m, v| P::new(m.x + v.x / n, m.y + v.y / n));
    let angle = |v: &P| (v.y - mean.y).atan2(v.x - mean.x);
    vertices.sort_by(|a, b| {
        angle(a)
            .partial_cmp(&angle(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// The piece outline in its own frame, centred on the centroid exactly as
/// `Body.setVertices` leaves it. `Round` has no polygon (it is drawn as an
/// arc), so `None` is returned for it.
pub fn local_outline(kind: ShapeKind) -> Option<Vec<P>> {
    let raw: Vec<P> = match kind {
        ShapeKind::Rect { w, h } => chamfer(
            &[
                P::new(0.0, 0.0),
                P::new(w, 0.0),
                P::new(w, h),
                P::new(0.0, h),
            ],
            4.0,
        ),
        ShapeKind::Tri { w, h } => {
            let mut v = vec![
                P::new(-w / 2.0, h / 2.0),
                P::new(w / 2.0, h / 2.0),
                P::new(0.0, -h / 2.0),
            ];
            clockwise_sort(&mut v);
            v
        }
        ShapeKind::Trap { w, top, h } => {
            let mut v = vec![
                P::new(-w / 2.0, h / 2.0),
                P::new(w / 2.0, h / 2.0),
                P::new(top / 2.0, -h / 2.0),
                P::new(-top / 2.0, -h / 2.0),
            ];
            clockwise_sort(&mut v);
            v
        }
        ShapeKind::Hex { r } => {
            let mut v: Vec<P> = (0..6)
                .map(|i| {
                    let a = i as f64 * std::f64::consts::PI / 3.0;
                    P::new(a.cos() * r, a.sin() * r)
                })
                .collect();
            clockwise_sort(&mut v);
            v
        }
        ShapeKind::Round { .. } => return None,
    };
    let c = centre(&raw);
    Some(raw.iter().map(|v| P::new(v.x - c.x, v.y - c.y)).collect())
}

/// Rotate a local outline by `angle` and translate it to `(x, y)`.
pub fn world_vertices(local: &[P], x: f64, y: f64, angle: f64) -> Vec<P> {
    let (s, c) = angle.sin_cos();
    local
        .iter()
        .map(|v| P::new(x + v.x * c - v.y * s, y + v.x * s + v.y * c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chamfered_rectangle_has_three_points_per_corner() {
        // radius 4 → precision 4^0.32 * 1.75 ≈ 2.73 → j = 0, 1, 2
        let v = local_outline(ShapeKind::Rect { w: 120.0, h: 36.0 }).unwrap();
        assert_eq!(v.len(), 12);
        let b = Bounds::of(&v, P::default());
        assert!((b.max.x - b.min.x - 120.0).abs() < 1e-9);
        assert!((b.max.y - b.min.y - 36.0).abs() < 1e-9);
        // centred on the rectangle centre
        assert!((b.max.x + b.min.x).abs() < 1e-9);
        assert!((b.max.y + b.min.y).abs() < 1e-9);
    }

    #[test]
    fn triangle_is_recentred_on_its_centroid() {
        let v = local_outline(ShapeKind::Tri { w: 88.0, h: 64.0 }).unwrap();
        assert_eq!(v.len(), 3);
        let c = centre(&v);
        assert!(c.x.abs() < 1e-9 && c.y.abs() < 1e-9);
        let b = Bounds::of(&v, P::default());
        // apex above the centroid by 2/3 h, base below by 1/3 h
        assert!((b.min.y + 64.0 * 2.0 / 3.0).abs() < 1e-9);
        assert!((b.max.y - 64.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn hexagon_has_flat_top_and_bottom() {
        let v = local_outline(ShapeKind::Hex { r: 36.0 }).unwrap();
        let b = Bounds::of(&v, P::default());
        assert!((b.max.x - 36.0).abs() < 1e-9);
        let top = v.iter().filter(|p| (p.y - b.min.y).abs() < 1e-9).count();
        assert_eq!(top, 2, "two vertices share the top edge");
    }

    #[test]
    fn bounds_extend_along_velocity() {
        let v = [P::new(0.0, 0.0), P::new(2.0, 0.0), P::new(2.0, 2.0)];
        let b = Bounds::of(&v, P::new(-1.0, 3.0));
        assert_eq!(b.min.x, -1.0);
        assert_eq!(b.max.y, 5.0);
        assert_eq!(b.max.x, 2.0);
    }

    #[test]
    fn trapezoid_area_and_order() {
        let v = local_outline(ShapeKind::Trap {
            w: 96.0,
            top: 56.0,
            h: 46.0,
        })
        .unwrap();
        assert!((area(&v, false) - (96.0 + 56.0) / 2.0 * 46.0).abs() < 1e-6);
        // clockwise_sort orders by ascending angle from the mean
        let mean_y = v.iter().map(|p| p.y).sum::<f64>() / 4.0;
        let angles: Vec<f64> = v.iter().map(|p| (p.y - mean_y).atan2(p.x)).collect();
        for w in angles.windows(2) {
            assert!(w[0] <= w[1]);
        }
    }
}
