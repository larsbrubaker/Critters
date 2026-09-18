//! Hand-drawn critter faces with expressions — a port of
//! `reference/critters.js`. `draw_critter(c, critter, size, expression,
//! blink)` paints one face centred on the origin, `size` being the
//! approximate face height. Everything is vector: ears, tails, spikes and
//! antlers behind the head, then markings, eyes, brows, nose and mouth.

use agg_gui::color::Color;

use crate::config::{hex, rgba, Critter};
use crate::render::canvas::Cx;

const OUTLINE: Color = hex(0x3a2718);

/// Which face a critter makes right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Expression {
    Happy,
    Curious,
    Scared,
    Worried,
    Relieved,
}

struct Palette {
    body: Color,
    inner: Color,
    muzzle: Color,
    nose: Color,
    tail: Color,
    iris: Option<Color>,
    ring: Color,
    mask: Color,
    spikes: Color,
    antler: Color,
}

fn palette(name: Critter) -> Palette {
    let base = Palette {
        body: Color::white(),
        inner: Color::white(),
        muzzle: Color::white(),
        nose: OUTLINE,
        tail: OUTLINE,
        iris: None,
        ring: OUTLINE,
        mask: OUTLINE,
        spikes: OUTLINE,
        antler: OUTLINE,
    };
    match name {
        Critter::Bunny => Palette {
            body: hex(0xf4f1ea),
            inner: hex(0xf5a6b8),
            muzzle: hex(0xffffff),
            nose: hex(0xe77c98),
            ..base
        },
        Critter::Squirrel => Palette {
            body: hex(0xc2743a),
            inner: hex(0xe9b68a),
            muzzle: hex(0xf0cfa5),
            nose: hex(0x3a2718),
            tail: hex(0xa85a25),
            ..base
        },
        Critter::Owl => Palette {
            body: hex(0x8a6a4a),
            inner: hex(0xd9b48a),
            muzzle: hex(0xefe0c4),
            nose: hex(0xf39c2b),
            iris: Some(hex(0xf2b632)),
            ring: hex(0x5a4030),
            ..base
        },
        Critter::Fox => Palette {
            body: hex(0xec7f2f),
            inner: hex(0x3a2718),
            muzzle: hex(0xfff5e8),
            nose: hex(0x3a2718),
            ..base
        },
        Critter::Frog => Palette {
            body: hex(0x63b552),
            inner: hex(0x8fd67a),
            muzzle: hex(0xa6dd8f),
            nose: hex(0x2f6b2a),
            ..base
        },
        Critter::Raccoon => Palette {
            body: hex(0x9a9a96),
            inner: hex(0xc8c8c4),
            muzzle: hex(0xe8e8e4),
            nose: hex(0x3a2718),
            mask: hex(0x3f3a36),
            ..base
        },
        Critter::Hedgehog => Palette {
            body: hex(0xd5aa7c),
            inner: hex(0xf0d6b8),
            muzzle: hex(0xf4e2cc),
            nose: hex(0x3a2718),
            spikes: hex(0x6b4a2e),
            ..base
        },
        Critter::Bear => Palette {
            body: hex(0x8f5b2c),
            inner: hex(0xc99a67),
            muzzle: hex(0xd9b17e),
            nose: hex(0x3a2718),
            ..base
        },
        Critter::Wolf => Palette {
            body: hex(0x7d8a8c),
            inner: hex(0xc5cdcf),
            muzzle: hex(0xe3e8e8),
            nose: hex(0x3a2718),
            iris: Some(hex(0xe8c03a)),
            ..base
        },
        Critter::Deer => Palette {
            body: hex(0xc68d55),
            inner: hex(0xefd2ad),
            muzzle: hex(0xf3e1c4),
            nose: hex(0x3a2718),
            antler: hex(0x7a5230),
            ..base
        },
    }
}

fn line(c: &mut Cx, x1: f64, y1: f64, x2: f64, y2: f64) {
    c.begin_path();
    c.move_to(x1, y1);
    c.line_to(x2, y2);
    c.stroke();
}

fn ellipse(c: &mut Cx, x: f64, y: f64, rx: f64, ry: f64, fill: Option<Color>, stroke: bool) {
    c.begin_path();
    c.ellipse(x, y, rx, ry, 0.0);
    if let Some(f) = fill {
        c.fill_style(f);
        c.fill();
    }
    if stroke {
        c.stroke();
    }
}

#[allow(clippy::too_many_arguments)]
fn tri(
    c: &mut Cx,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    x3: f64,
    y3: f64,
    fill: Option<Color>,
    stroke: bool,
) {
    c.begin_path();
    c.move_to(x1, y1);
    c.line_to(x2, y2);
    c.line_to(x3, y3);
    c.close_path();
    if let Some(f) = fill {
        c.fill_style(f);
        c.fill();
    }
    if stroke {
        c.stroke();
    }
}

/// Things drawn behind the head: ears, tails, spikes, antlers, tufts.
fn draw_behind(c: &mut Cx, name: Critter, u: f64, p: &Palette) {
    match name {
        Critter::Bunny => {
            // long ears, the right one flopped over
            for s in [-1.0, 1.0] {
                c.save();
                c.translate(s * 0.4 * u, -0.98 * u);
                c.rotate(if s == -1.0 { -0.14 } else { 0.55 });
                ellipse(c, 0.0, 0.0, 0.23 * u, 0.7 * u, Some(p.body), true);
                ellipse(c, 0.0, 0.06 * u, 0.11 * u, 0.5 * u, Some(p.inner), false);
                c.restore();
            }
            // fur tuft on top
            for (x, y, r) in [
                (-0.18, -0.86, 0.14),
                (0.02, -0.94, 0.15),
                (0.2, -0.86, 0.13),
            ] {
                ellipse(c, x * u, y * u, r * u, r * u, Some(p.body), true);
            }
        }
        Critter::Fox | Critter::Wolf => {
            for s in [-1.0, 1.0] {
                tri(
                    c,
                    s * 0.3 * u,
                    -0.55 * u,
                    s * 0.95 * u,
                    -0.45 * u,
                    s * 0.7 * u,
                    -1.25 * u,
                    Some(p.body),
                    true,
                );
                tri(
                    c,
                    s * 0.45 * u,
                    -0.62 * u,
                    s * 0.82 * u,
                    -0.57 * u,
                    s * 0.68 * u,
                    -1.05 * u,
                    Some(p.inner),
                    false,
                );
            }
        }
        Critter::Squirrel => {
            c.save();
            c.translate(0.55 * u, 0.1 * u);
            ellipse(
                c,
                0.35 * u,
                -0.25 * u,
                0.42 * u,
                0.85 * u,
                Some(p.tail),
                true,
            );
            ellipse(
                c,
                0.42 * u,
                -0.35 * u,
                0.2 * u,
                0.55 * u,
                Some(p.inner),
                false,
            );
            c.restore();
            for s in [-1.0, 1.0] {
                ellipse(
                    c,
                    s * 0.62 * u,
                    -0.7 * u,
                    0.24 * u,
                    0.26 * u,
                    Some(p.body),
                    true,
                );
                ellipse(
                    c,
                    s * 0.62 * u,
                    -0.68 * u,
                    0.12 * u,
                    0.14 * u,
                    Some(p.inner),
                    false,
                );
            }
        }
        Critter::Bear | Critter::Raccoon => {
            for s in [-1.0, 1.0] {
                ellipse(
                    c,
                    s * 0.68 * u,
                    -0.6 * u,
                    0.3 * u,
                    0.3 * u,
                    Some(p.body),
                    true,
                );
                ellipse(
                    c,
                    s * 0.68 * u,
                    -0.58 * u,
                    0.16 * u,
                    0.16 * u,
                    Some(p.inner),
                    false,
                );
            }
        }
        Critter::Owl => {
            for s in [-1.0, 1.0] {
                tri(
                    c,
                    s * 0.25 * u,
                    -0.72 * u,
                    s * 0.8 * u,
                    -0.62 * u,
                    s * 0.62 * u,
                    -1.25 * u,
                    Some(p.body),
                    true,
                );
                tri(
                    c,
                    s * 0.5 * u,
                    -0.7 * u,
                    s * 0.95 * u,
                    -0.55 * u,
                    s * 0.9 * u,
                    -1.05 * u,
                    Some(p.body),
                    true,
                );
            }
        }
        Critter::Frog => {
            for s in [-1.0, 1.0] {
                ellipse(
                    c,
                    s * 0.48 * u,
                    -0.72 * u,
                    0.34 * u,
                    0.34 * u,
                    Some(p.body),
                    true,
                );
            }
        }
        Critter::Hedgehog => {
            let mut a = -165.0;
            while a <= -15.0 {
                let r = a * std::f64::consts::PI / 180.0;
                let bx = r.cos() * 0.78 * u;
                let by = r.sin() * 0.7 * u;
                let tx = r.cos() * 1.35 * u;
                let ty = r.sin() * 1.25 * u;
                let px = -r.sin() * 0.16 * u;
                let py = r.cos() * 0.16 * u;
                tri(
                    c,
                    bx - px,
                    by - py,
                    bx + px,
                    by + py,
                    tx,
                    ty,
                    Some(p.spikes),
                    true,
                );
                a += 18.0;
            }
        }
        Critter::Deer => {
            c.stroke_style(p.antler);
            c.line_cap_round();
            c.save();
            c.line_width((u * 0.11).max(1.5));
            for s in [-1.0, 1.0] {
                line(c, s * 0.4 * u, -0.7 * u, s * 0.55 * u, -1.35 * u);
                line(c, s * 0.48 * u, -1.05 * u, s * 0.85 * u, -1.25 * u);
                line(c, s * 0.52 * u, -1.2 * u, s * 0.3 * u, -1.45 * u);
            }
            c.restore();
            c.stroke_style(OUTLINE);
            for s in [-1.0, 1.0] {
                c.save();
                c.translate(s * 0.85 * u, -0.45 * u);
                c.rotate(s * 0.9);
                ellipse(c, 0.0, 0.0, 0.2 * u, 0.42 * u, Some(p.body), true);
                ellipse(c, 0.0, 0.0, 0.09 * u, 0.26 * u, Some(p.inner), false);
                c.restore();
            }
        }
    }
}

fn draw_markings(c: &mut Cx, name: Critter, u: f64, p: &Palette) {
    match name {
        Critter::Fox | Critter::Wolf => {
            ellipse(c, 0.0, 0.32 * u, 0.62 * u, 0.48 * u, Some(p.muzzle), false);
        }
        Critter::Bear | Critter::Deer | Critter::Squirrel => {
            ellipse(c, 0.0, 0.36 * u, 0.48 * u, 0.36 * u, Some(p.muzzle), false);
        }
        Critter::Raccoon => {
            ellipse(c, 0.0, 0.4 * u, 0.5 * u, 0.36 * u, Some(p.muzzle), false);
            ellipse(c, 0.0, -0.1 * u, 0.86 * u, 0.3 * u, Some(p.mask), false);
            for s in [-1.0, 1.0] {
                ellipse(
                    c,
                    s * 0.34 * u,
                    -0.1 * u,
                    0.24 * u,
                    0.2 * u,
                    Some(p.inner),
                    false,
                );
            }
        }
        Critter::Owl => {
            // heart-shaped facial disc: two overlapping cream circles, one around each eye
            for s in [-1.0, 1.0] {
                ellipse(
                    c,
                    s * 0.36 * u,
                    -0.04 * u,
                    0.46 * u,
                    0.5 * u,
                    Some(p.muzzle),
                    false,
                );
            }
            c.save();
            c.line_width((u * 0.06).max(1.0));
            c.stroke_style(p.ring);
            for s in [-1.0, 1.0] {
                c.begin_path();
                c.ellipse(s * 0.36 * u, -0.04 * u, 0.46 * u, 0.5 * u, 0.0);
                c.stroke();
            }
            // chest feathers along the bottom of the head
            c.stroke_style(rgba(60, 40, 25, 0.55));
            c.line_width((u * 0.07).max(1.0));
            for x in [-0.42, -0.14, 0.14, 0.42] {
                c.begin_path();
                c.arc(
                    x * u,
                    0.62 * u,
                    0.14 * u,
                    std::f64::consts::PI * 0.1,
                    std::f64::consts::PI * 0.9,
                );
                c.stroke();
            }
            c.restore();
        }
        Critter::Hedgehog => {
            ellipse(c, 0.0, 0.3 * u, 0.55 * u, 0.42 * u, Some(p.muzzle), false);
        }
        Critter::Bunny | Critter::Frog => {}
    }
}

fn draw_eye(c: &mut Cx, x: f64, y: f64, r: f64, expr: Expression, blink: bool, p: &Palette) {
    use std::f64::consts::PI;
    c.save();
    c.line_width((r * 0.32).max(1.2));
    c.line_cap_round();
    c.stroke_style(OUTLINE);
    if expr == Expression::Relieved {
        c.begin_path();
        c.arc(x, y + r * 0.45, r * 0.85, PI * 1.15, PI * 1.85);
        c.stroke();
        c.restore();
        return;
    }
    if blink {
        line(c, x - r * 0.8, y, x + r * 0.8, y);
        c.restore();
        return;
    }
    let k = match expr {
        Expression::Scared => 1.3,
        Expression::Curious => 1.15,
        _ => 1.0,
    };
    let pr = match expr {
        Expression::Scared => 0.4,
        Expression::Worried => 0.52,
        _ => 0.6,
    };
    let py = match expr {
        Expression::Curious => -r * 0.18,
        Expression::Worried => r * 0.1,
        _ => 0.0,
    };
    ellipse(c, x, y, r * k, r * k, Some(Color::white()), false);
    c.line_width((r * 0.18).max(1.0));
    c.stroke();
    ellipse(
        c,
        x,
        y + py,
        r * k * pr,
        r * k * pr,
        Some(p.iris.unwrap_or(OUTLINE)),
        false,
    );
    if p.iris.is_some() {
        ellipse(
            c,
            x,
            y + py,
            r * k * pr * 0.55,
            r * k * pr * 0.55,
            Some(OUTLINE),
            false,
        );
    }
    ellipse(
        c,
        x - r * k * pr * 0.35,
        y + py - r * k * pr * 0.35,
        r * k * 0.16,
        r * k * 0.16,
        Some(Color::white()),
        false,
    );
    ellipse(
        c,
        x + r * k * pr * 0.3,
        y + py + r * k * pr * 0.3,
        r * k * 0.07,
        r * k * 0.07,
        Some(Color::white()),
        false,
    );
    c.restore();
}

fn draw_brows(c: &mut Cx, ex: f64, ey: f64, r: f64, expr: Expression) {
    if expr != Expression::Scared && expr != Expression::Worried {
        return;
    }
    c.save();
    c.line_width((r * 0.3).max(1.2));
    c.line_cap_round();
    c.stroke_style(OUTLINE);
    let lift = if expr == Expression::Scared { 2.1 } else { 1.8 };
    for s in [-1.0, 1.0] {
        line(
            c,
            s * ex - s * r * 0.9,
            ey - r * 1.35,
            s * ex + s * r * 0.5,
            ey - r * lift,
        );
    }
    c.restore();
}

fn draw_mouth(c: &mut Cx, name: Critter, u: f64, expr: Expression) {
    use std::f64::consts::PI;
    c.save();
    c.line_width((u * 0.09).max(1.2));
    c.line_cap_round();
    c.stroke_style(OUTLINE);
    let my = if name == Critter::Frog {
        0.28 * u
    } else {
        0.42 * u
    };
    let w = if name == Critter::Frog {
        0.5 * u
    } else {
        0.26 * u
    };
    if name == Critter::Bunny && expr == Expression::Happy {
        // split "ω" lip under the nose with two buck teeth
        c.line_width((u * 0.08).max(1.2));
        line(c, 0.0, 0.27 * u, 0.0, 0.36 * u);
        c.begin_path();
        c.arc(-0.11 * u, 0.33 * u, 0.11 * u, PI * 0.1, PI * 0.9);
        c.stroke();
        c.begin_path();
        c.arc(0.11 * u, 0.33 * u, 0.11 * u, PI * 0.1, PI * 0.9);
        c.stroke();
        c.fill_style(Color::white());
        c.fill_rect(-0.085 * u, 0.36 * u, 0.08 * u, 0.17 * u);
        c.fill_rect(0.005 * u, 0.36 * u, 0.08 * u, 0.17 * u);
        c.line_width((u * 0.045).max(1.0));
        c.stroke_rect(-0.085 * u, 0.36 * u, 0.17 * u, 0.17 * u);
        line(c, 0.0, 0.36 * u, 0.0, 0.53 * u);
        c.restore();
        return;
    }
    match expr {
        Expression::Happy => {
            c.begin_path();
            c.arc(0.0, my - w * 0.5, w, PI * 0.15, PI * 0.85);
            c.stroke();
        }
        Expression::Relieved => {
            c.begin_path();
            c.arc(0.0, my - w * 0.3, w * 1.25, PI * 0.1, PI * 0.9);
            c.stroke();
            for s in [-1.0, 1.0] {
                ellipse(
                    c,
                    s * 0.58 * u,
                    0.22 * u,
                    0.14 * u,
                    0.09 * u,
                    Some(rgba(240, 120, 140, 0.45)),
                    false,
                );
            }
        }
        Expression::Curious => {
            ellipse(
                c,
                0.0,
                my + 0.05 * u,
                0.09 * u,
                0.1 * u,
                Some(OUTLINE),
                false,
            );
        }
        Expression::Scared => {
            ellipse(
                c,
                0.0,
                my + 0.08 * u,
                w * 0.75,
                w * 0.62,
                Some(hex(0x4a2323)),
                true,
            );
            ellipse(
                c,
                0.0,
                my + 0.25 * u,
                w * 0.4,
                w * 0.2,
                Some(hex(0xe07a8a)),
                false,
            );
        }
        Expression::Worried => {
            c.begin_path();
            c.move_to(-w, my + 0.04 * u);
            c.quad_to(-w * 0.5, my - 0.12 * u, 0.0, my + 0.04 * u);
            c.quad_to(w * 0.5, my + 0.2 * u, w, my + 0.04 * u);
            c.stroke();
        }
    }
    c.restore();
}

fn draw_nose(c: &mut Cx, name: Critter, u: f64, p: &Palette) {
    if name == Critter::Bunny {
        // tiny pink nose, slightly heart-shaped
        c.begin_path();
        c.move_to(-0.1 * u, 0.16 * u);
        c.line_to(0.1 * u, 0.16 * u);
        c.line_to(0.0, 0.27 * u);
        c.close_path();
        c.fill_style(p.nose);
        c.fill();
        ellipse(
            c,
            -0.05 * u,
            0.16 * u,
            0.055 * u,
            0.045 * u,
            Some(p.nose),
            false,
        );
        ellipse(
            c,
            0.05 * u,
            0.16 * u,
            0.055 * u,
            0.045 * u,
            Some(p.nose),
            false,
        );
        return;
    }
    if name == Critter::Owl {
        tri(
            c,
            -0.11 * u,
            0.1 * u,
            0.11 * u,
            0.1 * u,
            0.0,
            0.42 * u,
            Some(p.nose),
            true,
        );
        return;
    }
    if name == Critter::Frog {
        for s in [-1.0, 1.0] {
            ellipse(
                c,
                s * 0.12 * u,
                0.02 * u,
                0.04 * u,
                0.03 * u,
                Some(p.nose),
                false,
            );
        }
        return;
    }
    ellipse(c, 0.0, 0.14 * u, 0.1 * u, 0.075 * u, Some(p.nose), false);
}

/// `drawCritter(ctx, name, size, expression, blink)`.
pub fn draw_critter(c: &mut Cx, name: Critter, size: f64, expr: Expression, blink: bool) {
    let p = palette(name);
    let u = size / 2.0;
    c.save();
    c.line_width((u * 0.085).max(1.2));
    c.stroke_style(OUTLINE);
    c.line_join_round();
    draw_behind(c, name, u, &p);
    // head
    if name == Critter::Bunny {
        ellipse(c, 0.0, 0.03 * u, 0.98 * u, 0.86 * u, Some(p.body), true);
    } else {
        ellipse(c, 0.0, 0.0, 0.9 * u, 0.82 * u, Some(p.body), true);
    }
    draw_markings(c, name, u, &p);
    if name == Critter::Bunny {
        // rosy cheeks and whiskers
        for s in [-1.0, 1.0] {
            ellipse(
                c,
                s * 0.6 * u,
                0.32 * u,
                0.17 * u,
                0.1 * u,
                Some(rgba(245, 150, 170, 0.55)),
                false,
            );
            c.save();
            c.stroke_style(rgba(90, 70, 60, 0.5));
            c.line_width((u * 0.04).max(1.0));
            c.line_cap_round();
            line(c, s * 0.5 * u, 0.3 * u, s * 0.96 * u, 0.2 * u);
            line(c, s * 0.5 * u, 0.36 * u, s * 0.96 * u, 0.42 * u);
            c.restore();
        }
    }
    // eyes
    let (mut ex, mut ey, mut er) = (0.33 * u, -0.12 * u, 0.13 * u);
    match name {
        Critter::Bunny => {
            ex = 0.37 * u;
            ey = -0.04 * u;
            er = 0.165 * u;
        }
        Critter::Owl => {
            ex = 0.36 * u;
            ey = -0.06 * u;
            er = 0.25 * u;
        }
        Critter::Frog => {
            ex = 0.48 * u;
            ey = -0.72 * u;
            er = 0.17 * u;
        }
        _ => {}
    }
    draw_eye(c, -ex, ey, er, expr, blink, &p);
    draw_eye(c, ex, ey, er, expr, blink, &p);
    draw_brows(c, ex, ey, er, expr);
    draw_nose(c, name, u, &p);
    draw_mouth(c, name, u, expr);
    c.restore();
}
