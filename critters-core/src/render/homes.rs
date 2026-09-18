//! Shape interiors, all wood — a port of `reference/homes.js`. Every piece
//! is a cut of timber with a hollow or knot for its critter: logs side-on
//! with grain and end rings, planks and trunks with bark grain and knots,
//! stumps with a cut surface on top, a split-log wedge, and the hexagon and
//! circle as log ends seen face-on with growth rings. Drawn in the piece's
//! local coordinates (origin at the body centre, unrotated) with the shape's
//! outline already clipped by `render/world.rs`.

use agg_gui::color::Color;

use crate::config::{hex, rgba};
use crate::piece::Extents;
use crate::render::canvas::Cx;

const GRAIN: Color = rgba(60, 35, 15, 0.26);
const RING: Color = rgba(80, 50, 30, 0.5);

fn ell(c: &mut Cx, x: f64, y: f64, rx: f64, ry: f64, fill: Color) {
    c.begin_path();
    c.ellipse(x, y, rx, ry, 0.0);
    c.fill_style(fill);
    c.fill();
}

/// Horizontal grain (logs).
fn grain_h(c: &mut Cx, l: &Extents, rows: usize, amp: f64) {
    c.stroke_style(GRAIN);
    c.line_width(1.5);
    c.line_cap_round();
    for i in 1..=rows {
        let y = l.min_y + (l.h * i as f64) / (rows as f64 + 1.0);
        c.begin_path();
        let mut x = l.min_x;
        let mut first = true;
        while x <= l.max_x {
            let py = y + (x * 0.15 + i as f64).sin() * amp;
            if first {
                c.move_to(x, py);
                first = false;
            } else {
                c.line_to(x, py);
            }
            x += 6.0;
        }
        c.stroke();
    }
}

/// Vertical bark grain (planks, trunks, stumps).
fn grain_v(c: &mut Cx, l: &Extents, cols: usize, amp: f64) {
    c.stroke_style(GRAIN);
    c.line_width(2.0);
    c.line_cap_round();
    for i in 1..=cols {
        let x = l.min_x + (l.w * i as f64) / (cols as f64 + 1.0);
        c.begin_path();
        let mut y = l.min_y;
        let mut first = true;
        while y <= l.max_y {
            let px = x + (y * 0.12 + i as f64 * 2.0).sin() * amp;
            if first {
                c.move_to(px, y);
                first = false;
            } else {
                c.line_to(px, y);
            }
            y += 6.0;
        }
        c.stroke();
    }
}

/// End-grain discs at both ends of a side-on log.
fn end_rings(c: &mut Cx, l: &Extents, color: Color) {
    let rx = l.h * 0.22;
    let ry = l.h * 0.42;
    for x in [l.min_x + rx * 0.9, l.max_x - rx * 0.9] {
        ell(c, x, 0.0, rx, ry, color);
        c.stroke_style(RING);
        c.line_width(1.2);
        for k in [0.65, 0.35] {
            c.begin_path();
            c.ellipse(x, 0.0, rx * k, ry * k, 0.0);
            c.stroke();
        }
    }
}

/// Growth rings filling a cross-cut face.
fn cross_cut(c: &mut Cx, big_r: f64, rings: usize) {
    c.stroke_style(RING);
    c.line_width(1.3);
    for i in 1..=rings {
        let r = (big_r * i as f64) / rings as f64;
        c.begin_path();
        let mut a = 0.0;
        let mut first = true;
        while a <= std::f64::consts::TAU + 0.1 {
            let x = a.cos() * r * (1.0 + (a * 3.0 + i as f64).sin() * 0.03);
            let y = a.sin() * r * (1.0 + (a * 2.0 + i as f64).cos() * 0.03);
            if first {
                c.move_to(x, y);
                first = false;
            } else {
                c.line_to(x, y);
            }
            a += 0.25;
        }
        c.stroke();
    }
}

/// Lighter cut surface along the top of a stump, with a hint of rings.
fn cut_top(c: &mut Cx, l: &Extents, color: Color) {
    let h = l.h * 0.22;
    c.fill_style(color);
    c.fill_rect(l.min_x - 6.0, l.min_y - 6.0, l.w + 12.0, h + 6.0);
    c.stroke_style(RING);
    c.line_width(1.2);
    for k in [0.75, 0.45, 0.18] {
        c.begin_path();
        c.ellipse(0.0, l.min_y + h * 0.5, (l.w / 2.0) * k, h * 0.32 * k, 0.0);
        c.stroke();
    }
    c.stroke_style(rgba(60, 35, 15, 0.35));
    c.line_width(1.5);
    c.begin_path();
    c.move_to(l.min_x - 6.0, l.min_y + h);
    c.line_to(l.max_x + 6.0, l.min_y + h);
    c.stroke();
}

/// Small dark knots.
fn knots(c: &mut Cx, pts: &[(f64, f64, f64)]) {
    for &(x, y, r) in pts {
        ell(c, x, y, r, r * 0.7, rgba(60, 35, 15, 0.4));
        ell(c, x, y, r * 0.5, r * 0.35, rgba(40, 22, 10, 0.5));
    }
}

/// The hollow the critter lives in.
fn hollow(c: &mut Cx, rx: f64, ry: f64, dy: f64) {
    ell(c, 0.0, dy, rx * 1.08, ry * 1.08, rgba(60, 35, 15, 0.45));
    ell(c, 0.0, dy, rx, ry, hex(0x3b2a1e));
    ell(c, 0.0, dy + ry * 0.15, rx * 0.85, ry * 0.8, hex(0x22160d));
}

/// `drawHome(ctx, shapeName, L, faceSize)`.
pub fn draw_home(c: &mut Cx, name: &str, l: &Extents, f: f64) {
    match name {
        "log" => {
            // squirrel: side-on log with a knot hole
            grain_h(c, l, 3, 1.5);
            end_rings(c, l, hex(0xd2a679));
            hollow(c, f * 0.72, f * 0.5, 0.0);
        }
        "block" => {
            // bunny: squared-off chunk of timber
            grain_v(c, l, 4, 1.5);
            knots(
                c,
                &[
                    (l.min_x + l.w * 0.2, l.min_y + l.h * 0.18, 4.0),
                    (l.max_x - l.w * 0.18, l.max_y - l.h * 0.2, 3.5),
                ],
            );
            hollow(c, f * 0.62, f * 0.62, 0.0);
        }
        "plank" => {
            // owl: bark trunk with a hollow
            grain_v(c, l, 3, 2.0);
            knots(c, &[(l.min_x + l.w * 0.3, l.max_y - l.h * 0.12, 3.0)]);
            hollow(c, f * 0.62, f * 0.72, 0.0);
        }
        "wedge" => {
            // fox: split-log wedge, bark along the slanted sides
            grain_h(c, l, 4, 1.2);
            c.stroke_style(rgba(60, 35, 15, 0.45));
            c.line_width(6.0);
            c.begin_path();
            c.move_to(l.min_x, l.max_y);
            c.line_to(0.0, l.min_y);
            c.line_to(l.max_x, l.max_y);
            c.stroke();
            hollow(c, f * 0.66, f * 0.5, f * 0.08);
        }
        "stump" => {
            // frog: tree stump, cut surface on top
            grain_v(c, l, 5, 2.0);
            cut_top(c, l, hex(0xd9b48a));
            hollow(c, f * 0.62, f * 0.5, f * 0.12);
        }
        "log end" => {
            // raccoon: hollow log seen end-on
            cross_cut(c, l.w * 0.55, 5);
            hollow(c, f * 0.62, f * 0.62, 0.0);
        }
        "round log" => {
            // hedgehog: round log end-on
            cross_cut(c, l.w * 0.5, 4);
            hollow(c, f * 0.6, f * 0.6, 0.0);
        }
        "great log" => {
            // bear: big hollow log
            grain_h(c, l, 4, 2.0);
            end_rings(c, l, hex(0xc99a67));
            hollow(c, f * 0.9, f * 0.72, 0.0);
        }
        "trunk" => {
            // wolf: thick trunk section with a big hollow
            grain_v(c, l, 6, 2.5);
            knots(
                c,
                &[
                    (l.min_x + l.w * 0.18, l.min_y + l.h * 0.15, 5.0),
                    (l.max_x - l.w * 0.15, l.max_y - l.h * 0.22, 4.0),
                ],
            );
            hollow(c, f * 0.82, f * 0.74, f * 0.04);
        }
        "great stump" => {
            // deer: wide stump
            grain_v(c, l, 7, 2.5);
            cut_top(c, l, hex(0xdcb890));
            hollow(c, f * 0.7, f * 0.58, f * 0.1);
        }
        _ => {}
    }
}
