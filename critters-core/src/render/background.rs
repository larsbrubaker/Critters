//! Parallax backdrop — `drawSky`, `drawSpace`, `drawMountains`,
//! `drawFarTrees`, `drawLeafyTree`, `drawMidTrees`, `drawNearTrees`,
//! `drawBirdsAndClouds` and `drawWind` from `reference/game.js`, drawn in
//! logical stage units before the camera translation is applied.

use agg_gui::color::Color;

use crate::config::{hex, rgba, FLOOR_Y, SKY, W};
use crate::game::Game;
use crate::render::canvas::{mix, Cx};
use crate::render::View;
use crate::scenery::LeafyTree;

fn sky_colors(view: &View) -> (Color, Color) {
    let a = view.alt_m();
    let mut i = 0;
    while i < SKY.len() - 2 && a > SKY[i + 1].0 {
        i += 1;
    }
    let (a0, t0, b0) = SKY[i];
    let (a1, t1, b1) = SKY[i + 1];
    let t = ((a - a0) / (a1 - a0)).clamp(0.0, 1.0);
    (mix(t0, t1, t), mix(b0, b1, t))
}

pub fn draw_sky(c: &mut Cx, view: &View, _game: &Game) {
    let (top, bottom) = sky_colors(view);
    c.fill_linear_gradient(0.0, 0.0, 0.0, view.h, &[(0.0, top), (1.0, bottom)]);
    c.fill_rect(0.0, 0.0, W, view.h);

    // sun: sits low at the start and drifts up the sky as you climb, fading out into space
    let a = view.alt_m();
    let sun_alpha = (1.0 - ((a - 22.0).max(0.0)) / 8.0).max(0.0);
    if sun_alpha > 0.0 {
        c.global_alpha(sun_alpha);
        c.fill_style(hex(0xffd166));
        c.begin_path();
        c.arc(
            W - 70.0,
            150.0 - a.min(20.0) * 4.0,
            34.0,
            0.0,
            std::f64::consts::TAU,
        );
        c.fill();
        c.global_alpha(1.0);
    }
}

pub fn draw_space(c: &mut Cx, view: &View, game: &Game) {
    let a = view.alt_m();
    let star_alpha = ((a - 12.0) / 10.0).clamp(0.0, 1.0);
    if star_alpha > 0.0 {
        for s in &game.scenery.stars {
            let y = ((s.y + view.rise() * 0.08) % view.h + view.h) % view.h;
            let tw = 0.6 + 0.4 * (view.time / 600.0 + s.tw).sin();
            c.fill_style(Color::rgba(1.0, 1.0, 1.0, (star_alpha * tw) as f32));
            c.begin_path();
            c.arc(s.x, y, s.r, 0.0, std::f64::consts::TAU);
            c.fill();
        }
    }
    for p in &game.scenery.planets {
        let y = view.sky_y(p.a, p.p);
        if y < -80.0 || y > view.h + 80.0 {
            continue;
        }
        c.fill_style(p.color);
        c.begin_path();
        c.arc(p.x, y, p.r, 0.0, std::f64::consts::TAU);
        c.fill();
        c.fill_style(rgba(0, 0, 0, 0.12));
        c.begin_path();
        c.arc(
            p.x - p.r * 0.3,
            y - p.r * 0.2,
            p.r * 0.25,
            0.0,
            std::f64::consts::TAU,
        );
        c.fill();
        c.begin_path();
        c.arc(
            p.x + p.r * 0.35,
            y + p.r * 0.3,
            p.r * 0.18,
            0.0,
            std::f64::consts::TAU,
        );
        c.fill();
        if p.ring {
            c.stroke_style(rgba(240, 220, 180, 0.8));
            c.line_width(6.0);
            c.begin_path();
            c.ellipse(p.x, y, p.r * 1.8, p.r * 0.45, -0.3);
            c.stroke();
        }
    }
}

pub fn draw_mountains(c: &mut Cx, view: &View, game: &Game) {
    let fade = 1.0 - ((view.alt_m() - 14.0) / 10.0).clamp(0.0, 1.0); // gone by ~24 m
    if fade <= 0.0 {
        return;
    }
    let base = view.layer_y(FLOOR_Y, 0.2);
    c.save();
    c.global_alpha(fade);
    for m in &game.scenery.mountains {
        c.fill_style(rgba(110, 140, 165, 0.7));
        c.begin_path();
        c.move_to(m.x - m.w / 2.0, base);
        c.line_to(m.x, base - m.h);
        c.line_to(m.x + m.w / 2.0, base);
        c.close_path();
        c.fill();
        c.fill_style(rgba(255, 255, 255, 0.75));
        c.begin_path();
        c.move_to(m.x - m.w * 0.12, base - m.h * 0.76);
        c.line_to(m.x, base - m.h);
        c.line_to(m.x + m.w * 0.12, base - m.h * 0.76);
        c.close_path();
        c.fill();
    }
    c.restore();
}

pub fn draw_far_trees(c: &mut Cx, view: &View, game: &Game) {
    let base = view.layer_y(FLOOR_Y, 0.15);
    c.fill_style(hex(0x4d8f6f));
    for t in &game.scenery.far_trees {
        let tw = t.h * t.w;
        c.begin_path();
        c.move_to(t.x - tw / 2.0, base);
        c.line_to(t.x, base - t.h);
        c.line_to(t.x + tw / 2.0, base);
        c.close_path();
        c.fill();
    }
    c.fill_rect(0.0, base, W, view.h);
}

fn draw_leafy_tree(c: &mut Cx, t: &LeafyTree, base: f64, trunk: Color, leaf: Color) {
    c.fill_style(trunk);
    c.fill_rect(t.x - t.t / 2.0, base - t.h, t.t, t.h);
    c.fill_style(leaf);
    let top = base - t.h;
    c.begin_path();
    c.arc(t.x, top, t.r, 0.0, std::f64::consts::TAU);
    c.arc(
        t.x - t.r * 0.8,
        top + t.r * 0.5,
        t.r * 0.75,
        0.0,
        std::f64::consts::TAU,
    );
    c.arc(
        t.x + t.r * 0.8,
        top + t.r * 0.5,
        t.r * 0.75,
        0.0,
        std::f64::consts::TAU,
    );
    c.arc(t.x, top + t.r * 0.9, t.r * 0.8, 0.0, std::f64::consts::TAU);
    c.fill();
}

pub fn draw_mid_trees(c: &mut Cx, view: &View, game: &Game) {
    let base = view.layer_y(FLOOR_Y, 0.35);
    for t in &game.scenery.mid_trees {
        draw_leafy_tree(c, t, base, hex(0x7a5a3c), hex(0x4f9a5b));
    }
    c.fill_style(hex(0x3f7a45));
    c.fill_rect(0.0, base, W, view.h);
}

pub fn draw_near_trees(c: &mut Cx, view: &View, game: &Game) {
    let base = view.layer_y(FLOOR_Y, 0.65);
    for t in &game.scenery.near_trees {
        draw_leafy_tree(c, t, base, hex(0x4e3a2a), hex(0x3a8a4a));
    }
    c.fill_style(hex(0x356a3c));
    c.fill_rect(0.0, base, W, view.h);
}

pub fn draw_birds_and_clouds(c: &mut Cx, view: &View, game: &Game) {
    let cloud_alpha = ((view.alt_m() - 4.0) / 3.0).clamp(0.0, 1.0) * 0.88;
    c.fill_style(Color::rgba(1.0, 1.0, 1.0, cloud_alpha as f32));
    for cl in &game.scenery.clouds {
        let y = view.sky_y(cl.a, 0.5);
        if y < -80.0 || y > view.h + 80.0 {
            continue;
        }
        c.begin_path();
        c.arc(cl.x, y, 22.0 * cl.s, 0.0, std::f64::consts::TAU);
        c.arc(
            cl.x + 26.0 * cl.s,
            y - 10.0 * cl.s,
            26.0 * cl.s,
            0.0,
            std::f64::consts::TAU,
        );
        c.arc(
            cl.x + 54.0 * cl.s,
            y,
            20.0 * cl.s,
            0.0,
            std::f64::consts::TAU,
        );
        c.fill();
    }
    let bird_alpha = ((view.alt_m() - 2.5) / 2.0).clamp(0.0, 1.0);
    if bird_alpha <= 0.0 {
        return;
    }
    c.stroke_style(rgba(40, 40, 40, (0.75 * bird_alpha) as f32));
    c.line_width(2.0);
    c.line_cap_butt();
    for b in &game.scenery.birds {
        let y = view.sky_y(b.a, 0.4);
        if y < -20.0 || y > view.h + 20.0 {
            continue;
        }
        let flap = (view.time / 120.0 + b.ph).sin() * 4.0;
        c.begin_path();
        c.move_to(b.x - 8.0, y + flap);
        c.quad_to(b.x - 4.0, y - 3.0, b.x, y);
        c.quad_to(b.x + 4.0, y - 3.0, b.x + 8.0, y + flap);
        c.stroke();
    }
}

pub fn draw_wind(c: &mut Cx, view: &View, game: &Game) {
    let wind = &game.wind;
    if wind.strength <= 0.0 {
        return;
    }
    c.save();
    c.stroke_style(Color::rgba(1.0, 1.0, 1.0, (0.55 * wind.strength) as f32));
    c.line_width(2.0);
    c.line_cap_round();
    let travel = (view.time - wind.start) * 0.9 * wind.dir;
    for st in &game.scenery.streaks {
        let span = W + 300.0;
        let x = (((st.x + travel * st.v) % span) + span) % span - 150.0;
        let y = st.y * view.h + ((view.time / 400.0) + st.x).sin() * 4.0;
        c.begin_path();
        c.move_to(x, y);
        c.line_to(x - st.len * wind.dir, y);
        c.stroke();
    }
    c.restore();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sky_interpolates_between_altitude_stops() {
        let v = |alt: f64| View {
            h: 720.0,
            cam_y: -530.0 - alt * 100.0,
            cam_y0: -530.0,
            time: 0.0,
        };
        fn close(a: Color, b: Color) -> bool {
            (a.r - b.r).abs() < 1e-5 && (a.g - b.g).abs() < 1e-5 && (a.b - b.b).abs() < 1e-5
        }
        let (top0, _) = sky_colors(&v(0.0));
        assert!(close(top0, SKY[0].1));
        let (top2, _) = sky_colors(&v(2.0));
        assert!(close(top2, mix(SKY[0].1, SKY[1].1, 0.5)));
        // beyond the last stop the last pair is held
        let (top_far, bottom_far) = sky_colors(&v(100.0));
        assert!(close(top_far, SKY[5].1));
        assert!(close(bottom_far, SKY[5].2));
    }
}
