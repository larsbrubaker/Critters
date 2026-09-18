//! Rendering entry point. `draw` reproduces `draw()` in `reference/game.js`
//! and the DOM chrome around the canvas (HUD, hint, toast, overlay card,
//! tray, mute button) inside one agg-gui paint call:
//!
//! 1. The agg-gui context (y up, origin bottom-left) is flipped into the
//!    original's CSS-pixel space (y down, origin at the app column's top-left).
//! 2. The stage is scaled by `layout.scale` so 480 logical units span the
//!    app width, and the scene is drawn in logical units.
//! 3. Chrome is drawn back in CSS pixels, exactly where the stylesheet put it.
//!
//! `View` bundles the camera helpers (`rise`, `altM`, `layerY`, `skyY`).

pub mod background;
pub mod batch;
pub mod canvas;
pub mod critters;
pub mod gallery;
pub mod homes;
pub mod hud;
pub mod sprites;
pub mod world;

use agg_gui::draw_ctx::DrawCtx;

use crate::config::{hex, PX_PER_M};
use crate::fonts::Fonts;
use crate::game::Game;
use crate::input::Layout;
use canvas::Cx;
use sprites::SpriteCache;

/// Camera state needed by the parallax helpers.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub h: f64,
    pub cam_y: f64,
    pub cam_y0: f64,
    pub time: f64,
}

impl View {
    pub fn of(game: &Game) -> Self {
        Self {
            h: game.h,
            cam_y: game.cam_y,
            cam_y0: game.cam_y0,
            time: game.time,
        }
    }

    /// px the camera has climbed (>= 0).
    pub fn rise(&self) -> f64 {
        self.cam_y0 - self.cam_y
    }

    /// Camera altitude in metres.
    pub fn alt_m(&self) -> f64 {
        self.rise() / PX_PER_M
    }

    /// Screen y of a world y with parallax `p`.
    pub fn layer_y(&self, world_y: f64, p: f64) -> f64 {
        (world_y - self.cam_y0) - (self.cam_y - self.cam_y0) * p
    }

    /// Screen y of a sky object centred when rise == a.
    pub fn sky_y(&self, a: f64, p: f64) -> f64 {
        self.h / 2.0 + (a - self.rise()) * p
    }
}

/// Paint the whole widget. `widget_w`/`widget_h` are the widget's size in
/// CSS px; the app column is centred within it.
pub fn draw(
    ctx: &mut dyn DrawCtx,
    fonts: &Fonts,
    game: &mut Game,
    sprites: &mut SpriteCache,
    layout: &Layout,
    widget_w: f64,
    widget_h: f64,
) {
    let app_x = ((widget_w - layout.app_w) / 2.0).max(0.0);
    let mut timer = crate::debug::SectionTimer::start();
    let mut c = Cx::new(ctx, fonts);

    // page background behind the app column
    c.fill_style(hex(0x14281c));
    c.fill_rect(0.0, 0.0, widget_w, widget_h);

    // flip into CSS-pixel space: y down, origin at the app column's top-left
    c.save();
    c.translate(app_x, widget_h);
    c.scale(1.0, -1.0);

    // ── stage (logical units) ──
    c.save();
    c.ctx.clip_rect(0.0, 0.0, layout.app_w, layout.stage_h);
    c.scale(layout.scale, layout.scale);
    c.ppu = layout.scale * agg_gui::device_scale();
    let view = View::of(game);
    background::draw_sky(&mut c, &view, game);
    timer.mark("sky");
    background::draw_space(&mut c, &view, game);
    timer.mark("space");
    background::draw_mountains(&mut c, &view, game);
    timer.mark("mountains");
    background::draw_far_trees(&mut c, &view, game);
    timer.mark("far");
    background::draw_birds_and_clouds(&mut c, &view, game, sprites);
    timer.mark("clouds");
    background::draw_mid_trees(&mut c, &view, game);
    timer.mark("mid");
    background::draw_near_trees(&mut c, &view, game);
    timer.mark("near");
    background::draw_wind(&mut c, &view, game);
    timer.mark("wind");
    c.save();
    c.translate(0.0, -game.cam_y);
    // the floor and stump scroll out of view once the camera has climbed
    if game.cam_y + game.h > sprites::GROUND_RECT.1 {
        sprites.ground(fonts, game).blit(&mut c);
    }
    world::draw_ruler(&mut c, game);
    timer.mark("floor+stump+ruler");
    // Tall towers: only the pieces the camera can see are drawn.
    for b in &game.placed {
        if world::piece_visible(b, game.cam_y, game.h) {
            world::draw_body(&mut c, game, sprites, b, false, false, true);
        }
    }
    if let Some(held) = &game.held {
        c.save();
        c.set_line_dash(&[6.0, 8.0]);
        c.stroke_style(crate::config::rgba(255, 255, 255, 0.5));
        c.line_width(2.0);
        c.begin_path();
        c.move_to(held.x, held.bounds.max.y);
        c.line_to(held.x, 0.0);
        c.stroke();
        c.restore();
        world::draw_body(&mut c, game, sprites, held, !game.can_drop(), true, true);
    }
    c.restore();
    c.restore();

    timer.mark("pieces");
    // ── chrome (CSS px) ──
    hud::draw_stage_chrome(&mut c, game, layout);
    timer.mark("hud");
    hud::draw_tray(&mut c, game, sprites, layout);
    timer.mark("tray");

    c.restore();
}
