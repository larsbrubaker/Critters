//! A canvas-2D shaped wrapper over agg-gui's `DrawCtx` so the drawing code
//! in the other `render/` modules can be transcribed from the JavaScript
//! almost call for call (`beginPath`/`arc`/`ellipse`/`clip`/`fillText`, …).
//! The caller has already put the context into the original's coordinate
//! system (y down, logical stage units) — see `render/mod.rs` — so text is
//! re-flipped locally before it is drawn.

use agg_gui::color::Color;
use agg_gui::draw_ctx::{DrawCtx, GradientSpread, GradientStop, LinearGradientPaint};
use agg_rust::math_stroke::{LineCap, LineJoin};
use agg_rust::trans_affine::TransAffine;

use crate::fonts::Fonts;

pub struct Cx<'a> {
    pub ctx: &'a mut dyn DrawCtx,
    pub fonts: &'a Fonts,
    /// Device pixels per unit of the space being drawn in (set by the caller
    /// when it changes scale); `render/batch.rs` sizes its AA fringe from it.
    pub ppu: f64,
    /// Canvas state the backend's own save/restore does not cover: agg-gui's
    /// wgpu context only stacks the transform and clip, whereas canvas-2D
    /// also restores `globalAlpha` and the line dash. Tracked here so a
    /// dimmed held piece or a faded cloud cannot leak into later drawing.
    alpha: f64,
    dash: Vec<f64>,
    saved: Vec<(f64, Vec<f64>)>,
}

impl<'a> Cx<'a> {
    pub fn new(ctx: &'a mut dyn DrawCtx, fonts: &'a Fonts) -> Self {
        Self {
            ctx,
            fonts,
            ppu: 1.0,
            alpha: 1.0,
            dash: Vec::new(),
            saved: Vec::new(),
        }
    }

    // ── style ─────────────────────────────────────────────────────────────

    pub fn fill_style(&mut self, c: Color) {
        self.ctx.set_fill_color(c);
    }

    pub fn stroke_style(&mut self, c: Color) {
        self.ctx.set_stroke_color(c);
    }

    pub fn line_width(&mut self, w: f64) {
        self.ctx.set_line_width(w);
    }

    pub fn line_cap_round(&mut self) {
        self.ctx.set_line_cap(LineCap::Round);
    }

    pub fn line_cap_butt(&mut self) {
        self.ctx.set_line_cap(LineCap::Butt);
    }

    pub fn line_join_round(&mut self) {
        self.ctx.set_line_join(LineJoin::Round);
    }

    pub fn global_alpha(&mut self, a: f64) {
        self.alpha = a;
        self.ctx.set_global_alpha(a);
    }

    pub fn set_line_dash(&mut self, dashes: &[f64]) {
        self.dash = dashes.to_vec();
        self.ctx.set_line_dash(dashes, 0.0);
    }

    /// `createLinearGradient(x1, y1, x2, y2)` + `addColorStop` as the fill style.
    pub fn fill_linear_gradient(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        stops: &[(f64, Color)],
    ) {
        if self.ctx.supports_fill_linear_gradient() {
            self.ctx.set_fill_linear_gradient(LinearGradientPaint {
                x1,
                y1,
                x2,
                y2,
                transform: TransAffine::new(),
                spread: GradientSpread::Pad,
                stops: stops
                    .iter()
                    .map(|&(offset, color)| GradientStop { offset, color })
                    .collect(),
            });
        } else if let Some(&(_, c)) = stops.first() {
            self.ctx.set_fill_color(c);
        }
    }

    // ── paths ─────────────────────────────────────────────────────────────

    pub fn begin_path(&mut self) {
        self.ctx.begin_path();
    }

    pub fn move_to(&mut self, x: f64, y: f64) {
        self.ctx.move_to(x, y);
    }

    pub fn line_to(&mut self, x: f64, y: f64) {
        self.ctx.line_to(x, y);
    }

    pub fn quad_to(&mut self, cx: f64, cy: f64, x: f64, y: f64) {
        self.ctx.quad_to(cx, cy, x, y);
    }

    pub fn close_path(&mut self) {
        self.ctx.close_path();
    }

    /// Canvas `arc(x, y, r, a0, a1)` with the default (increasing-angle) sweep.
    pub fn arc(&mut self, x: f64, y: f64, r: f64, a0: f64, a1: f64) {
        self.ctx.arc_to(x, y, r, a0, a1, true);
    }

    /// Canvas `ellipse(x, y, rx, ry, rotation, 0, 2π)`.
    pub fn ellipse(&mut self, x: f64, y: f64, rx: f64, ry: f64, rotation: f64) {
        self.ctx
            .ellipse(x, y, rx, ry, rotation, 0.0, std::f64::consts::TAU, false);
    }

    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.ctx.rect(x, y, w, h);
    }

    pub fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.ctx.begin_path();
        self.ctx.rect(x, y, w, h);
        self.ctx.fill();
    }

    pub fn stroke_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.ctx.begin_path();
        self.ctx.rect(x, y, w, h);
        self.ctx.stroke();
    }

    pub fn fill(&mut self) {
        self.ctx.fill();
    }

    pub fn stroke(&mut self) {
        self.ctx.stroke();
    }

    /// Canvas `clip()`: everything until the matching `restore` is masked by
    /// the current path.
    pub fn clip(&mut self) {
        self.ctx.clip_path();
    }

    // ── transform ─────────────────────────────────────────────────────────

    pub fn save(&mut self) {
        self.saved.push((self.alpha, self.dash.clone()));
        self.ctx.save();
    }

    pub fn restore(&mut self) {
        self.ctx.restore();
        if let Some((alpha, dash)) = self.saved.pop() {
            if alpha != self.alpha {
                self.alpha = alpha;
                self.ctx.set_global_alpha(alpha);
            }
            if dash != self.dash {
                self.ctx.set_line_dash(&dash, 0.0);
                self.dash = dash;
            }
        }
    }

    pub fn translate(&mut self, x: f64, y: f64) {
        self.ctx.translate(x, y);
    }

    pub fn rotate(&mut self, a: f64) {
        self.ctx.rotate(a);
    }

    pub fn scale(&mut self, sx: f64, sy: f64) {
        self.ctx.scale(sx, sy);
    }

    // ── text ──────────────────────────────────────────────────────────────

    fn select_font(&mut self, size: f64, bold: bool) {
        let font = if bold {
            &self.fonts.bold
        } else {
            &self.fonts.regular
        };
        self.ctx.set_font(std::sync::Arc::clone(font));
        self.ctx.set_font_size(size);
    }

    /// Advance width of `text` at `size`.
    pub fn measure(&mut self, text: &str, size: f64, bold: bool) -> f64 {
        self.select_font(size, bold);
        self.ctx.measure_text(text).map(|m| m.width).unwrap_or(0.0)
    }

    /// Ascent and descent (positive) of the face at `size`.
    pub fn metrics(&mut self, size: f64, bold: bool) -> (f64, f64) {
        self.select_font(size, bold);
        self.ctx
            .measure_text("Hg")
            .map(|m| (m.ascent, m.descent))
            .unwrap_or((size * 0.8, size * 0.2))
    }

    /// `fillText(text, x, baseline_y)` in the y-down space.
    pub fn fill_text(&mut self, text: &str, x: f64, baseline_y: f64, size: f64, bold: bool) {
        self.select_font(size, bold);
        self.ctx.save();
        self.ctx.translate(x, baseline_y);
        self.ctx.scale(1.0, -1.0);
        self.ctx.fill_text(text, 0.0, 0.0);
        self.ctx.restore();
    }

    /// A Font Awesome glyph centred on `(cx, cy)`.
    pub fn icon_centered(&mut self, glyph: &str, cx: f64, cy: f64, size: f64) {
        self.ctx.set_font(std::sync::Arc::clone(&self.fonts.icons));
        self.ctx.set_font_size(size);
        let m = self.ctx.measure_text(glyph);
        let (w, a, d) =
            m.map(|m| (m.width, m.ascent, m.descent))
                .unwrap_or((size, size * 0.8, 0.0));
        self.ctx.save();
        self.ctx.translate(cx - w * 0.5, cy + (a - d) * 0.5);
        self.ctx.scale(1.0, -1.0);
        self.ctx.fill_text(glyph, 0.0, 0.0);
        self.ctx.restore();
    }

    /// `textAlign = 'center'` variant.
    pub fn fill_text_centered(
        &mut self,
        text: &str,
        cx: f64,
        baseline_y: f64,
        size: f64,
        bold: bool,
    ) {
        let w = self.measure(text, size, bold);
        self.fill_text(text, cx - w * 0.5, baseline_y, size, bold);
    }

    /// Baseline for `textBaseline = 'middle'` at centre `cy`.
    pub fn middle_baseline(&mut self, cy: f64, size: f64, bold: bool) -> f64 {
        let (ascent, descent) = self.metrics(size, bold);
        cy + (ascent - descent) * 0.5
    }
}

/// `mix(a, b, t)`: blend two colours component-wise (the original rounds to
/// 8-bit channels, which is invisible at these gradients).
pub fn mix(a: Color, b: Color, t: f64) -> Color {
    a.lerp(b, t as f32)
}

/// Greedy word wrap for the overlay card paragraphs.
pub fn wrap_lines(cx: &mut Cx, text: &str, max_w: f64, size: f64, bold: bool) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        let candidate = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && cx.measure(&candidate, size, bold) > max_w {
            lines.push(std::mem::take(&mut line));
            line = word.to_string();
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use agg_gui::{Framebuffer, GfxCtx};

    /// Canvas semantics: `restore()` brings back the `globalAlpha` that was
    /// current at `save()`. A faded sprite must not dim what is drawn next.
    #[test]
    fn restore_brings_back_global_alpha() {
        let fonts = Fonts::load().unwrap();
        let mut fb = Framebuffer::new(8, 8);
        {
            let mut gfx = GfxCtx::new(&mut fb);
            let mut c = Cx::new(&mut gfx, &fonts);
            c.save();
            c.global_alpha(0.25);
            c.set_line_dash(&[2.0, 2.0]);
            c.restore();
            assert_eq!(c.alpha, 1.0);
            assert!(c.dash.is_empty());
            c.fill_style(Color::from_rgb8(255, 0, 0));
            c.fill_rect(0.0, 0.0, 8.0, 8.0);
        }
        let px = fb.pixels();
        let i = (4 * 8 + 4) * 4;
        assert_eq!(px[i + 3], 255, "the fill after restore is fully opaque");
        assert_eq!(px[i], 255);
    }
}
