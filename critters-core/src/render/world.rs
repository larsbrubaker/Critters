//! The world layer — `drawFloor`, `drawStump`, `drawRuler`, `critterAnim`,
//! `expressionFor` and `drawBody` from `reference/game.js`. Drawn in
//! logical units after the camera translation, so world y = 0 is the stump
//! top. `draw_body` is also used for the tray previews (with `on_stage`
//! false, which disables wind lean and expressions like the original's
//! `c === ctx` checks).

use agg_gui::color::Color;

use crate::config::{hex, rgba, Critter, ShapeKind, FLOOR_Y, PX_PER_M, STUMP_H, STUMP_W, W};
use crate::game::Game;
use crate::piece::Piece;
use crate::render::canvas::Cx;
use crate::render::critters::{draw_critter, Expression};
use crate::render::homes::{draw_home, last_home_path};

pub fn draw_floor(c: &mut Cx, game: &Game) {
    // dirt, with a grass edge
    c.fill_style(hex(0x5b4431));
    c.fill_rect(-W, FLOOR_Y, W * 3.0, 600.0);
    c.fill_style(hex(0x4f8a3f));
    c.fill_rect(-W, FLOOR_Y - 6.0, W * 3.0, 10.0);
    for f in &game.scenery.floor_bits {
        let x = f.x;
        let y = FLOOR_Y - 4.0;
        if f.kind < 0.45 {
            // grass tuft
            c.fill_style(hex(0x6a994e));
            c.begin_path();
            c.move_to(x - 6.0 * f.s, y);
            c.line_to(x - 2.0 * f.s, y - 14.0 * f.s);
            c.line_to(x, y);
            c.fill();
            c.begin_path();
            c.move_to(x, y);
            c.line_to(x + 5.0 * f.s, y - 18.0 * f.s);
            c.line_to(x + 8.0 * f.s, y);
            c.fill();
        } else if f.kind < 0.7 {
            // mushroom
            c.fill_style(hex(0xf1e4c8));
            c.fill_rect(x - 2.0 * f.s, y - 10.0 * f.s, 4.0 * f.s, 10.0 * f.s);
            c.fill_style(hex(0xc1666b));
            c.begin_path();
            c.arc(x, y - 10.0 * f.s, 7.0 * f.s, std::f64::consts::PI, 0.0);
            c.fill();
        } else if f.kind < 0.85 {
            // rock
            c.fill_style(hex(0x8d8a80));
            c.begin_path();
            c.ellipse(x, y - 3.0 * f.s, 9.0 * f.s, 5.0 * f.s, 0.0);
            c.fill();
        } else {
            // fern
            c.stroke_style(hex(0x3f7d4f));
            c.line_width(2.0);
            c.line_cap_butt();
            for k in -2..=2 {
                let k = k as f64;
                c.begin_path();
                c.move_to(x, y);
                c.quad_to(
                    x + k * 6.0 * f.s,
                    y - 12.0 * f.s,
                    x + k * 10.0 * f.s,
                    y - 16.0 * f.s,
                );
                c.stroke();
            }
        }
    }
}

pub fn draw_stump(c: &mut Cx) {
    let x = W / 2.0 - STUMP_W / 2.0;
    // roots
    c.fill_style(hex(0x5a3f2b));
    c.begin_path();
    c.move_to(x, FLOOR_Y - 40.0);
    c.quad_to(x - 10.0, FLOOR_Y, x - 50.0, FLOOR_Y);
    c.line_to(x + 10.0, FLOOR_Y);
    c.fill();
    c.begin_path();
    c.move_to(x + STUMP_W, FLOOR_Y - 40.0);
    c.quad_to(x + STUMP_W + 10.0, FLOOR_Y, x + STUMP_W + 50.0, FLOOR_Y);
    c.line_to(x + STUMP_W - 10.0, FLOOR_Y);
    c.fill();
    // bark
    c.fill_style(hex(0x6b4b35));
    c.fill_rect(x, 0.0, STUMP_W, STUMP_H);
    c.fill_style(rgba(0, 0, 0, 0.12));
    for i in 0..6 {
        c.fill_rect(x + 20.0 + i as f64 * 42.0, 16.0, 10.0, STUMP_H - 16.0);
    }
    // top face with rings
    c.fill_style(hex(0xc69c6d));
    c.fill_rect(x, -4.0, STUMP_W, 16.0);
    c.stroke_style(rgba(90, 60, 40, 0.5));
    c.line_width(2.0);
    let mut r = 20.0;
    while r < STUMP_W / 2.0 {
        c.begin_path();
        c.ellipse(W / 2.0, 4.0, r, 5.0, 0.0);
        c.stroke();
        r += 24.0;
    }
    // moss
    c.fill_style(hex(0x6a994e));
    c.begin_path();
    c.move_to(x - 6.0, 0.0);
    c.line_to(x + 10.0, -18.0);
    c.line_to(x + 16.0, 0.0);
    c.fill();
    c.begin_path();
    c.move_to(x + STUMP_W - 16.0, 0.0);
    c.line_to(x + STUMP_W - 10.0, -18.0);
    c.line_to(x + STUMP_W + 6.0, 0.0);
    c.fill();
}

pub fn draw_ruler(c: &mut Cx, game: &Game) {
    let top_visible = game.cam_y - 20.0;
    let bottom_visible = game.cam_y + game.h;
    let mut m = 1.0;
    while m * PX_PER_M < -top_visible {
        let y = -m * PX_PER_M;
        m += 1.0;
        if y > bottom_visible {
            continue;
        }
        c.stroke_style(rgba(255, 255, 255, 0.35));
        c.line_width(1.0);
        c.line_cap_butt();
        c.begin_path();
        c.move_to(8.0, y);
        c.line_to(38.0, y);
        c.stroke();
        c.fill_style(rgba(255, 255, 255, 0.7));
        let baseline = c.middle_baseline(y, 12.0, true);
        c.fill_text(&format!("{} m", m - 1.0), 42.0, baseline, 12.0, true);
    }
}

/// Idle animation per critter, plus a squash when the piece lands and a lean
/// in the wind: `(dx, dy, rot, sx, sy)`.
pub fn critter_anim(game: &Game, b: &Piece, held: bool) -> (f64, f64, f64, f64, f64) {
    let time = game.time;
    let s = (time + b.anim_phase) / 1000.0;
    let (mut dx, mut dy, mut rot, mut sx, mut sy) = (0.0, 0.0, 0.0, 1.0, 1.0);
    match b.spec.critter {
        Critter::Bunny => {
            let h = (s * 2.4).sin().max(0.0);
            dy = -h * h * 6.0;
            sy = 1.0 + h * 0.08;
            sx = 1.0 - h * 0.05;
        }
        Critter::Squirrel => {
            let burst = if (s * 0.7).sin() > 0.7 { 1.0 } else { 0.0 };
            rot = (s * 16.0).sin() * 0.12 * burst;
        }
        Critter::Owl => rot = (s * 1.1).sin() * 0.14,
        Critter::Fox => {
            let br = (s * 2.2).sin();
            sy = 1.0 + br * 0.05;
            sx = 1.0 - br * 0.025;
        }
        Critter::Frog => {
            let h = (((s * 1.3).sin() - 0.7) / 0.3).max(0.0);
            dy = -h * 7.0;
            sx = 1.0 + h * 0.12;
            sy = 1.0 - h * 0.08;
        }
        Critter::Raccoon => {
            dx = (s * 1.5).sin() * 4.0;
            rot = (s * 1.5).sin() * 0.06;
        }
        Critter::Hedgehog => rot = (s * 2.0).sin() * 0.3,
        Critter::Bear => {
            let br = (s * 1.1).sin();
            sy = 1.0 + br * 0.05;
            dy = -br;
        }
        Critter::Wolf => {
            rot = (s * 0.8).sin() * 0.08;
            dy = (s * 1.6).sin() * 1.5;
        }
        Critter::Deer => dy = (s * 2.6).sin() * 2.0,
    }
    if b.land_at >= 0.0 {
        let e = (time - b.land_at) / 1000.0;
        if e < 0.6 {
            let q = (-e * 7.0).exp() * (e * 22.0).sin();
            sy *= 1.0 - q * 0.35;
            sx *= 1.0 + q * 0.25;
        }
    }
    if held {
        dy += (time / 260.0).sin() * 3.0;
    } else if game.wind.strength > 0.0 && !b.is_static {
        dx += game.wind.dir * game.wind.strength * 6.0;
        rot += game.wind.dir * game.wind.strength * 0.2;
    }
    (dx, dy, rot, sx, sy)
}

/// Which face a critter makes right now.
pub fn expression_for(game: &Game, b: &Piece, held: bool) -> Expression {
    if held {
        return Expression::Curious;
    }
    if !b.has_landed {
        return Expression::Scared;
    }
    if b.speed > 0.4 || b.angular_speed > 0.03 {
        return Expression::Scared;
    }
    if b.land_at >= 0.0 && game.time - b.land_at < 1100.0 {
        return Expression::Relieved;
    }
    if b.has_landed && !b.locked && !b.is_static {
        return Expression::Worried;
    }
    if game.wind.strength > 0.3 && !b.is_static {
        return Expression::Worried;
    }
    Expression::Happy
}

/// `drawBody(c, b, ghost, held)`; `on_stage` is the original's `c === ctx`.
pub fn draw_body(c: &mut Cx, game: &Game, b: &Piece, ghost: bool, held: bool, on_stage: bool) {
    let shape = b.spec;
    c.save();
    if ghost {
        c.global_alpha(0.55);
    }
    if !held && on_stage {
        let (dx, rot) = game.wind_lean(b);
        if dx != 0.0 || rot != 0.0 {
            c.translate(b.x + dx, b.y);
            c.rotate(rot);
            c.translate(-b.x, -b.y);
        }
    }
    c.begin_path();
    if let ShapeKind::Round { r } = shape.kind {
        c.arc(b.x, b.y, r, 0.0, std::f64::consts::TAU);
    } else {
        for (i, v) in b.vertices.iter().enumerate() {
            if i == 0 {
                c.move_to(v.x, v.y);
            } else {
                c.line_to(v.x, v.y);
            }
        }
        c.close_path();
    }
    c.fill_style(shape.color);
    c.fill();
    // paint the critter's home inside the shape
    c.save();
    c.clip();
    c.translate(b.x, b.y);
    c.rotate(b.angle);
    let extents = b.local_extents();
    draw_home(c, shape.name, &extents, shape.face_size);
    c.restore();
    // In the original, `restore()` does not restore the canvas path, so the
    // tint fill and the two strokes below act on the last path `drawHome`
    // built — the hollow's innermost ellipse — which gives the hole its
    // rim. Rebuild that path in the same frame to reproduce it exactly.
    c.save();
    c.translate(b.x, b.y);
    c.rotate(b.angle);
    last_home_path(c, shape.name, shape.face_size);
    if b.stability_mult > 1.0 {
        let a = ((b.stability_mult - 1.0) * 0.03).min(0.3);
        c.fill_style(rgba(40, 25, 10, a as f32));
        c.fill();
    }
    c.line_width(3.0);
    c.stroke_style(rgba(0, 0, 0, 0.28));
    c.line_join_round();
    c.stroke();
    c.line_width(1.5);
    c.stroke_style(rgba(255, 255, 255, 0.25));
    c.stroke();
    c.restore();

    let (dx, dy, rot, sx, sy) = critter_anim(game, b, held);
    c.translate(b.x, b.y);
    c.rotate(b.angle);
    c.translate(dx, dy);
    c.rotate(rot);
    c.scale(sx, sy);
    let expr = if on_stage {
        expression_for(game, b, held)
    } else {
        Expression::Happy
    };
    let blink = expr != Expression::Scared && ((game.time + b.anim_phase * 7.0) % 3900.0) < 120.0;
    draw_critter(c, shape.critter, shape.face_size * 1.05, expr, blink);
    c.restore();
}

/// Tint used by the tray for the slot background — kept here so the palette
/// stays in one place with the other shape colours.
pub const fn slot_background(big: bool, empty: bool) -> Color {
    if empty {
        hex(0x4a3527)
    } else if big {
        hex(0x7a5a3a)
    } else {
        hex(0x6b4b35)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SHAPES;

    fn game() -> Game {
        Game::new(720.0, 0, false)
    }

    #[test]
    fn expressions_follow_the_original_priority() {
        let mut g = game();
        let mut p = Piece::new(SHAPES[1], W / 2.0, -30.0, 0.0);
        assert_eq!(expression_for(&g, &p, true), Expression::Curious);
        assert_eq!(expression_for(&g, &p, false), Expression::Scared);
        p.has_landed = true;
        p.land_at = 0.0;
        g.time = 500.0;
        assert_eq!(expression_for(&g, &p, false), Expression::Relieved);
        p.speed = 1.0;
        assert_eq!(expression_for(&g, &p, false), Expression::Scared);
        p.speed = 0.0;
        g.time = 2000.0;
        assert_eq!(expression_for(&g, &p, false), Expression::Worried);
        p.locked = true;
        assert_eq!(expression_for(&g, &p, false), Expression::Happy);
        g.wind.strength = 0.5;
        assert_eq!(expression_for(&g, &p, false), Expression::Worried);
        p.is_static = true;
        assert_eq!(expression_for(&g, &p, false), Expression::Happy);
    }

    #[test]
    fn landing_squash_decays_and_held_pieces_bob() {
        let mut g = game();
        let mut p = Piece::new(SHAPES[3], 0.0, 0.0, 0.0); // fox
        p.land_at = 0.0;
        g.time = 50.0;
        let (_, _, _, sx, sy) = critter_anim(&g, &p, false);
        assert!(sx > 1.0 && sy < 1.0, "squash right after landing");
        g.time = 5000.0;
        let (_, dy_still, _, _, _) = critter_anim(&g, &p, false);
        let (_, dy_held, _, _, _) = critter_anim(&g, &p, true);
        assert!((dy_held - dy_still - (g.time / 260.0).sin() * 3.0).abs() < 1e-9);
    }
}
