//! Gallery debug view — the counterpart of the original's `?debug`
//! `window.__cs.drawPiece`: every shape from `SHAPES` drawn unrotated at
//! 2.6× on a plain background so the port can be compared with the
//! JavaScript rendering side by side. Enabled with `CRITTERS_GALLERY=1`
//! (native); never reachable from normal play.

use agg_gui::draw_ctx::DrawCtx;

use crate::config::{hex, SHAPES};
use crate::fonts::Fonts;
use crate::game::Game;
use crate::piece::Piece;
use crate::render::canvas::Cx;
use crate::render::world::draw_body_live;

/// Positions matching the reference gallery script (logical px at 2.6×).
const POSITIONS: [(f64, f64); 10] = [
    (70.0, 40.0),
    (190.0, 40.0),
    (260.0, 60.0),
    (80.0, 120.0),
    (190.0, 120.0),
    (260.0, 120.0),
    (70.0, 190.0),
    (190.0, 190.0),
    (270.0, 200.0),
    (130.0, 220.0),
];

pub fn draw_gallery(
    ctx: &mut dyn DrawCtx,
    fonts: &Fonts,
    game: &Game,
    widget_w: f64,
    widget_h: f64,
) {
    let mut c = Cx::new(ctx, fonts);
    c.fill_style(hex(0x555566));
    c.fill_rect(0.0, 0.0, widget_w, widget_h);
    c.save();
    c.translate(0.0, widget_h);
    c.scale(2.6, -2.6);
    for (spec, (x, y)) in SHAPES.iter().zip(POSITIONS) {
        let piece = Piece::new(*spec, x, y, 0.0);
        draw_body_live(&mut c, game, &piece, false, false, false);
    }
    c.restore();
}
