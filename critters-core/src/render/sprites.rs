//! Sprite cache — the performance layer under `render/world.rs`.
//!
//! A piece is ~60 vector paths (wood grain, rings, hollow, a clip layer, a
//! hand-drawn face). Re-tessellating that every frame for every piece cost
//! about 1.5 ms per piece on a desktop and made tall towers crawl on phones.
//! None of it changes from frame to frame: a piece's wood body is fixed, and
//! a face only changes with its expression and blink — the idle animation,
//! landing squash, wind lean and body rotation are all plain transforms.
//!
//! So each body (per shape) and each face (per critter × expression × blink)
//! is rasterised once with agg-gui's software `GfxCtx`, at the current
//! pixels-per-logical-unit, and then drawn as a GPU textured quad through
//! `DrawCtx::draw_image_rgba_corners`, which caches the texture by `Arc`
//! identity. The same drawing code (`world::paint_body`, `draw_critter`) is
//! used for the sprites and for the live gallery, so the pixels match.

use std::collections::HashMap;
use std::sync::Arc;

use agg_gui::draw_ctx::DrawCtx;
use agg_gui::framebuffer::unpremultiply_rgba_inplace;
use agg_gui::{Framebuffer, GfxCtx};

use crate::config::{Critter, ShapeDef, ShapeKind};
use crate::fonts::Fonts;
use crate::piece::Piece;
use crate::render::canvas::Cx;
use crate::render::critters::{draw_critter, Expression};
use crate::render::world::paint_body;

/// Margin (logical units) around a body for its anti-aliased edge.
const BODY_MARGIN: f64 = 3.0;
/// Sprites are rasterised at this multiple of the screen density, so a
/// rotated or squashed sprite is minified (bilinear ≈ box filter) rather
/// than magnified, keeping AGG's exact-coverage edges crisp.
const SUPERSAMPLE: f64 = 2.0;
/// Stale chrome (old scores) is dropped once this many have accumulated.
const MAX_CHROME_SPRITES: usize = 24;
/// Sprites are never rasterised finer than this many pixels per unit.
const MAX_PIXELS_PER_UNIT: f64 = 6.0;

/// A rasterised image and the logical rectangle it covers, relative to the
/// piece centre (y down).
pub struct Sprite {
    pub data: Arc<Vec<u8>>,
    pub w: u32,
    pub h: u32,
    pub x0: f64,
    pub y0: f64,
    pub lw: f64,
    pub lh: f64,
}

impl Sprite {
    /// Rasterise `draw` over the logical rect `(x0, y0, lw, lh)` at
    /// `ppu` pixels per unit. `draw` sees the usual y-down logical space.
    pub fn render(
        fonts: &Fonts,
        ppu: f64,
        x0: f64,
        y0: f64,
        lw: f64,
        lh: f64,
        draw: impl FnOnce(&mut Cx),
    ) -> Self {
        let w = (lw * ppu).ceil().max(1.0) as u32;
        let h = (lh * ppu).ceil().max(1.0) as u32;
        let mut fb = Framebuffer::new(w, h);
        {
            let mut gfx = GfxCtx::new(&mut fb);
            let ctx: &mut dyn DrawCtx = &mut gfx;
            // y-up framebuffer → y-down logical space with (x0, y0) at the top-left
            ctx.translate(0.0, h as f64);
            ctx.scale(ppu, -ppu);
            ctx.translate(-x0, -y0);
            let mut c = Cx::new(ctx, fonts);
            draw(&mut c);
        }
        let mut px = fb.pixels_flipped();
        unpremultiply_rgba_inplace(&mut px);
        bleed_edge_colour(&mut px, w as usize, h as usize);
        Self {
            data: Arc::new(px),
            w,
            h,
            x0,
            y0,
            // the rect actually covered once the size was rounded up to whole pixels
            lw: w as f64 / ppu,
            lh: h as f64 / ppu,
        }
    }

    /// Draw the sprite with its origin at the current transform's origin.
    pub fn blit(&self, c: &mut Cx) {
        let (l, t, r, b) = (self.x0, self.y0, self.x0 + self.lw, self.y0 + self.lh);
        // bottom-left, bottom-right, top-right, top-left of the image
        c.ctx
            .draw_image_rgba_corners(&self.data, self.w, self.h, [(l, b), (r, b), (r, t), (l, t)]);
    }
}

/// Number of quantised stability-tint steps above "no tint".
///
/// The tint only moves the already-dark hollow by a colour level or two,
/// so a handful of steps is indistinguishable from the continuous ramp — and
/// every extra level is another full set of body textures (20 levels ran a
/// phone up to 200 sprites).
pub const TINT_LEVELS: u8 = 4;

/// Tint level for a stability multiplier: the tint alpha is
/// `min(0.3, (mult - 1) * 0.03)`, so it saturates at mult = 11.
pub fn tint_level(stability_mult: f64) -> u8 {
    let t = ((stability_mult - 1.0) / 10.0).clamp(0.0, 1.0);
    (t * TINT_LEVELS as f64).round() as u8
}

/// The multiplier a tint level stands for.
pub fn tint_level_mult(level: u8) -> f64 {
    1.0 + 10.0 * level as f64 / TINT_LEVELS as f64
}

/// Give fully transparent texels the colour of their opaque neighbours.
///
/// The texture is straight (un-premultiplied) alpha and is sampled with
/// linear filtering, so an edge sample interpolates between a wood-coloured
/// texel and a transparent one. Un-premultiplying leaves transparent texels
/// black, which would drag that interpolation toward black and outline every
/// sprite with a dark fringe. Two dilation passes cover the filter footprint.
pub fn bleed_edge_colour(px: &mut [u8], w: usize, h: usize) {
    for _ in 0..2 {
        let src = px.to_vec();
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 4;
                if src[i + 3] != 0 || (src[i] | src[i + 1] | src[i + 2]) != 0 {
                    continue;
                }
                let (mut r, mut g, mut b, mut n) = (0u32, 0u32, 0u32, 0u32);
                for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let j = (ny as usize * w + nx as usize) * 4;
                    if src[j + 3] != 0 || (src[j] | src[j + 1] | src[j + 2]) != 0 {
                        r += src[j] as u32;
                        g += src[j + 1] as u32;
                        b += src[j + 2] as u32;
                        n += 1;
                    }
                }
                // alpha stays 0: only the colour bleeds
                if let (Some(r), Some(g), Some(b)) =
                    (r.checked_div(n), g.checked_div(n), b.checked_div(n))
                {
                    px[i] = r as u8;
                    px[i + 1] = g as u8;
                    px[i + 2] = b as u8;
                }
            }
        }
    }
}

/// World rect covered by the ground sprite: the full stage width, from just
/// above the moss on the stump to below the lowest the camera ever looks
/// (`cam_y0 + H` = 190).
pub const GROUND_RECT: (f64, f64, f64, f64) = (0.0, -24.0, crate::config::W, 224.0);

#[derive(Default)]
pub struct SpriteCache {
    ppu: f64,
    /// Screen density without supersampling (for sprites that never rotate).
    screen_ppu: f64,
    ground: Option<Sprite>,
    cloud: Option<Sprite>,
    /// Device pixels per CSS pixel, for the chrome sprites.
    css_ppu: f64,
    chrome: HashMap<String, Sprite>,
    bodies: HashMap<(&'static str, u8), Sprite>,
    faces: HashMap<(Critter, Expression, bool), Sprite>,
}

impl SpriteCache {
    /// Set the rasterisation density (stage scale × device scale); a change
    /// of more than 2 % drops the cache so sprites stay crisp after a resize.
    pub fn set_pixels_per_unit(&mut self, ppu: f64) {
        let ppu = (ppu * SUPERSAMPLE).clamp(0.5, MAX_PIXELS_PER_UNIT);
        if self.ppu == 0.0 || (ppu - self.ppu).abs() / self.ppu > 0.02 {
            self.ppu = ppu;
            self.screen_ppu = (ppu / SUPERSAMPLE).max(0.5);
            self.bodies.clear();
            self.faces.clear();
            self.ground = None;
            self.cloud = None;
        }
    }

    /// Device scale for sprites drawn in CSS pixels (HUD, tray chrome).
    pub fn set_device_scale(&mut self, scale: f64) {
        let scale = scale.clamp(0.5, MAX_PIXELS_PER_UNIT);
        if (scale - self.css_ppu).abs() > 1e-6 {
            self.css_ppu = scale;
            self.chrome.clear();
        }
    }

    /// A piece of static chrome (HUD boxes, a tray slot's frame or badge),
    /// `w × h` CSS pixels with its origin at the top-left, keyed by whatever
    /// it displays. Text and rounded rects cost ~3 ms a frame on a phone when
    /// redrawn live, and they only change when the score or a slot changes.
    pub fn chrome(
        &mut self,
        fonts: &Fonts,
        key: String,
        w: f64,
        h: f64,
        draw: impl FnOnce(&mut Cx),
    ) -> &Sprite {
        let ppu = self.css_ppu.max(0.5);
        if !self.chrome.contains_key(&key) && self.chrome.len() >= MAX_CHROME_SPRITES {
            self.chrome.clear();
        }
        self.chrome
            .entry(key)
            .or_insert_with(|| Sprite::render(fonts, ppu, 0.0, 0.0, w, h, draw))
    }

    pub fn len(&self) -> usize {
        self.bodies.len()
            + self.faces.len()
            + usize::from(self.ground.is_some())
            + usize::from(self.cloud.is_some())
            + self.chrome.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The forest floor and the stump (`drawFloor` + `drawStump`): ~70 static
    /// paths, in world coordinates. It never rotates or scales, so it is
    /// rasterised at screen density without supersampling.
    pub fn ground(&mut self, fonts: &Fonts, game: &crate::game::Game) -> &Sprite {
        let ppu = self.screen_ppu.max(0.5);
        self.ground.get_or_insert_with(|| {
            let (x, y, w, h) = GROUND_RECT;
            Sprite::render(fonts, ppu, x, y, w, h, |c| {
                crate::render::world::draw_floor(c, game);
                crate::render::world::draw_stump(c);
            })
        })
    }

    /// One opaque white cloud at scale 1 (the three circles of
    /// `drawBirdsAndClouds`); the caller scales it and sets the opacity.
    pub fn cloud(&mut self, fonts: &Fonts) -> &Sprite {
        let ppu = self.ppu.max(0.5);
        self.cloud.get_or_insert_with(|| {
            Sprite::render(fonts, ppu, -25.0, -39.0, 102.0, 64.0, |c| {
                c.fill_style(agg_gui::color::Color::white());
                c.begin_path();
                c.arc(0.0, 0.0, 22.0, 0.0, std::f64::consts::TAU);
                c.arc(26.0, -10.0, 26.0, 0.0, std::f64::consts::TAU);
                c.arc(54.0, 0.0, 20.0, 0.0, std::f64::consts::TAU);
                c.fill();
            })
        })
    }

    /// The wood body of a shape: fill, clipped interior, stability tint and
    /// hollow rim. The tint darkens continuously with depth in the original;
    /// it is quantised to [`TINT_LEVELS`] steps (each under 1/255 of a
    /// channel apart on the hollow) so a deep piece is still one cached quad.
    /// Drawing it live cost 8 ms a frame on a phone with a 50-piece tower.
    pub fn body(&mut self, fonts: &Fonts, spec: ShapeDef, stability_mult: f64) -> &Sprite {
        let ppu = self.ppu.max(0.5);
        let level = tint_level(stability_mult);
        self.bodies.entry((spec.name, level)).or_insert_with(|| {
            let mut piece = Piece::new(spec, 0.0, 0.0, 0.0);
            piece.stability_mult = tint_level_mult(level);
            let (hw, hh) = match spec.kind {
                ShapeKind::Round { r } => (r, r),
                _ => {
                    let e = piece.local_extents();
                    (e.max_x.max(-e.min_x), e.max_y.max(-e.min_y))
                }
            };
            let (hw, hh) = (hw + BODY_MARGIN, hh + BODY_MARGIN);
            Sprite::render(fonts, ppu, -hw, -hh, hw * 2.0, hh * 2.0, |c| {
                paint_body(c, &piece);
            })
        })
    }

    /// A critter's face for one expression / blink state.
    pub fn face(
        &mut self,
        fonts: &Fonts,
        spec: ShapeDef,
        expr: Expression,
        blink: bool,
    ) -> &Sprite {
        let ppu = self.ppu.max(0.5);
        // `relieved` eyes ignore blinking and `scared` never blinks
        let blink = blink && !matches!(expr, Expression::Relieved | Expression::Scared);
        self.faces
            .entry((spec.critter, expr, blink))
            .or_insert_with(|| {
                let size = spec.face_size * 1.05;
                let u = size / 2.0;
                // ears, antlers, spikes and tails reach to about 1.5u sideways
                // and 1.7u up; brows and the scared mouth stay inside that
                Sprite::render(fonts, ppu, -1.7 * u, -1.9 * u, 3.4 * u, 3.1 * u, |c| {
                    draw_critter(c, spec.critter, size, expr, blink);
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SHAPES;

    fn alpha_at(s: &Sprite, lx: f64, ly: f64) -> u8 {
        let ppu = s.w as f64 / s.lw;
        let x = (((lx - s.x0) * ppu) as u32).min(s.w - 1);
        let y = (((ly - s.y0) * ppu) as u32).min(s.h - 1);
        s.data[((y * s.w + x) * 4 + 3) as usize]
    }

    #[test]
    fn body_sprite_covers_the_shape_and_nothing_else() {
        let fonts = Fonts::load().unwrap();
        let mut cache = SpriteCache::default();
        cache.set_pixels_per_unit(2.0);
        let log = cache.body(&fonts, SHAPES[0], 1.0); // 120 × 36
                                                      // 120 wide + 3 margin each side, at 2 px per unit × 2 supersampling
        assert!((504..=505).contains(&log.w), "width {}", log.w);
        assert_eq!(alpha_at(log, 0.0, 0.0), 255, "centre is opaque wood");
        assert_eq!(alpha_at(log, 50.0, 10.0), 255);
        assert_eq!(alpha_at(log, -62.5, -20.5), 0, "margin is transparent");
        // the wedge is clipped to its triangle: top corners are empty
        let wedge = cache.body(&fonts, SHAPES[3], 1.0);
        assert_eq!(alpha_at(wedge, -40.0, -35.0), 0);
        assert_eq!(alpha_at(wedge, 0.0, 10.0), 255);
    }

    #[test]
    fn tint_is_quantised_and_baked_into_the_body() {
        assert_eq!(tint_level(1.0), 0);
        assert_eq!(tint_level(0.5), 0);
        assert_eq!(tint_level(11.0), TINT_LEVELS);
        assert_eq!(tint_level(12.0), TINT_LEVELS, "alpha saturates at 0.3");
        assert!((tint_level_mult(tint_level(6.0)) - 6.0).abs() <= 1.25);
        let fonts = Fonts::load().unwrap();
        let mut cache = SpriteCache::default();
        cache.set_pixels_per_unit(1.0);
        // the hollow's inner ellipse sits just below the centre of a block
        let at = |s: &Sprite| {
            let ppu = s.w as f64 / s.lw;
            let x = ((0.0 - s.x0) * ppu) as u32;
            let y = ((14.0 - s.y0) * ppu) as u32;
            let i = ((y * s.w + x) * 4) as usize;
            (s.data[i], s.data[i + 1], s.data[i + 2])
        };
        let plain = at(cache.body(&fonts, SHAPES[1], 1.0));
        let deep = at(cache.body(&fonts, SHAPES[1], 11.0));
        // rgba(40,25,10,.3) over the #22160d hollow: a shift of a level or two
        assert_ne!(deep, plain, "the tint is baked in");
        assert_eq!(cache.len(), 2);
        cache.body(&fonts, SHAPES[1], 11.2);
        assert_eq!(cache.len(), 2, "same level reuses the sprite");
    }

    #[test]
    fn chrome_sprites_are_keyed_and_bounded() {
        let fonts = Fonts::load().unwrap();
        let mut cache = SpriteCache::default();
        cache.set_device_scale(2.0);
        let mut draws = 0;
        for _ in 0..3 {
            let s = cache.chrome(&fonts, "score 10".into(), 50.0, 20.0, |c| {
                draws += 1;
                c.fill_style(agg_gui::color::Color::white());
                c.fill_rect(0.0, 0.0, 50.0, 20.0);
            });
            assert_eq!((s.w, s.h), (100, 40));
        }
        assert_eq!(draws, 1, "drawn once, then reused");
        for i in 0..MAX_CHROME_SPRITES + 5 {
            cache.chrome(&fonts, format!("k{i}"), 4.0, 4.0, |_| {});
        }
        assert!(cache.len() <= MAX_CHROME_SPRITES);
        cache.set_device_scale(3.0);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn ground_sprite_has_stump_and_dirt() {
        let fonts = Fonts::load().unwrap();
        let game = crate::game::Game::new(720.0, 0, false);
        let mut cache = SpriteCache::default();
        cache.set_pixels_per_unit(1.0);
        let g = cache.ground(&fonts, &game);
        assert_eq!(g.w, 480, "screen density, no supersampling");
        assert_eq!(alpha_at(g, 240.0, 70.0), 255, "stump bark");
        assert_eq!(alpha_at(g, 20.0, 180.0), 255, "dirt");
        assert_eq!(
            alpha_at(g, 20.0, 60.0),
            0,
            "air beside the stump stays transparent"
        );
    }

    #[test]
    fn transparent_texels_next_to_the_edge_take_the_edge_colour() {
        // 4 × 1: opaque orange, then three transparent black texels
        let mut px = vec![200, 100, 50, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        bleed_edge_colour(&mut px, 4, 1);
        assert_eq!(
            &px[4..8],
            &[200, 100, 50, 0],
            "first neighbour bleeds, alpha stays 0"
        );
        assert_eq!(
            &px[8..12],
            &[200, 100, 50, 0],
            "second pass reaches one texel further"
        );
        assert_eq!(
            &px[12..16],
            &[0, 0, 0, 0],
            "beyond the filter footprint is untouched"
        );
        assert_eq!(&px[0..4], &[200, 100, 50, 255]);
    }

    #[test]
    fn faces_are_cached_per_expression_and_rescale_drops_the_cache() {
        let fonts = Fonts::load().unwrap();
        let mut cache = SpriteCache::default();
        cache.set_pixels_per_unit(1.5);
        let a = Arc::as_ptr(&cache.face(&fonts, SHAPES[1], Expression::Happy, false).data);
        let b = Arc::as_ptr(&cache.face(&fonts, SHAPES[1], Expression::Happy, false).data);
        assert_eq!(a, b, "same sprite reused");
        cache.face(&fonts, SHAPES[1], Expression::Scared, true);
        cache.face(&fonts, SHAPES[1], Expression::Scared, false);
        assert_eq!(cache.len(), 2, "scared never blinks, so one sprite");
        let bunny = cache.face(&fonts, SHAPES[1], Expression::Happy, false);
        assert_eq!(alpha_at(bunny, 0.0, 0.0), 255, "head is opaque");
        cache.set_pixels_per_unit(1.51);
        assert_eq!(cache.len(), 2, "a sub-2% change keeps the cache");
        cache.set_pixels_per_unit(2.5);
        assert!(cache.is_empty());
    }
}
