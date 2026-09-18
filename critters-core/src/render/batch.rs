//! Batched discs. The backdrop is full of circles — 120 stars, tree canopies
//! made of four overlapping circles each — and sending every one through the
//! general path tessellator cost several milliseconds a frame. A disc needs
//! no tessellator: `DiscBatch` triangulates them directly (a fan plus a
//! one-pixel anti-aliasing fringe with per-vertex alpha) and submits the
//! whole layer as a single `DrawCtx::draw_triangles_aa` call.
//!
//! Only use it for discs of one colour. Overlapping discs are fine when the
//! colour is opaque (a canopy); translucent overlapping discs would blend
//! twice, so the clouds stay ordinary paths.

use agg_gui::color::Color;

use crate::render::canvas::Cx;

pub struct DiscBatch {
    verts: Vec<[f32; 3]>,
    indices: Vec<u32>,
    /// Width of the anti-aliasing fringe in logical units (one device pixel).
    fringe: f64,
    /// Device pixels per logical unit, for choosing the segment count.
    ppu: f64,
}

impl DiscBatch {
    /// `ppu` is device pixels per logical unit under the current transform.
    pub fn new(ppu: f64) -> Self {
        let ppu = ppu.max(0.25);
        Self {
            verts: Vec::new(),
            indices: Vec::new(),
            fringe: 1.0 / ppu,
            ppu,
        }
    }

    /// Segment count that keeps the chord error under ~¼ device pixel.
    fn segments(&self, r: f64) -> usize {
        let px = (r * self.ppu).max(1.0);
        // chord error r(1 - cos(pi/n)) <= 0.25 px  =>  n >= pi / acos(1 - 0.25/px)
        let n = (std::f64::consts::PI / (1.0 - (0.25 / px).min(1.0)).acos()).ceil();
        (n as usize).clamp(8, 96)
    }

    /// Add a disc with opacity `alpha` (multiplies the batch colour's alpha).
    pub fn add(&mut self, x: f64, y: f64, r: f64, alpha: f64) {
        if r <= 0.0 || alpha <= 0.0 {
            return;
        }
        let n = self.segments(r);
        let a = alpha.min(1.0) as f32;
        let inner = (r - self.fringe * 0.5).max(0.0);
        let outer = r + self.fringe * 0.5;
        let base = self.verts.len() as u32;
        self.verts.push([x as f32, y as f32, a]);
        for i in 0..n {
            let t = i as f64 / n as f64 * std::f64::consts::TAU;
            let (s, c) = t.sin_cos();
            self.verts
                .push([(x + c * inner) as f32, (y + s * inner) as f32, a]);
            self.verts
                .push([(x + c * outer) as f32, (y + s * outer) as f32, 0.0]);
        }
        let n = n as u32;
        for i in 0..n {
            let j = (i + 1) % n;
            let (in_i, out_i) = (base + 1 + i * 2, base + 2 + i * 2);
            let (in_j, out_j) = (base + 1 + j * 2, base + 2 + j * 2);
            // fan
            self.indices.extend_from_slice(&[base, in_i, in_j]);
            // fringe quad
            self.indices
                .extend_from_slice(&[in_i, out_i, out_j, in_i, out_j, in_j]);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Submit every disc added so far in one draw call.
    pub fn flush(self, c: &mut Cx, color: Color) {
        if !self.is_empty() {
            c.ctx.draw_triangles_aa(&self.verts, &self.indices, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disc_geometry_is_a_fan_plus_fringe() {
        let mut b = DiscBatch::new(2.0);
        b.add(10.0, 20.0, 5.0, 0.5);
        let n = b.segments(5.0);
        assert_eq!(b.verts.len(), 1 + 2 * n);
        assert_eq!(b.indices.len(), 9 * n);
        assert!(b.indices.iter().all(|&i| (i as usize) < b.verts.len()));
        // centre and inner ring carry the alpha, the outer ring fades to zero
        assert_eq!(b.verts[0], [10.0, 20.0, 0.5]);
        assert_eq!(b.verts[1][2], 0.5);
        assert_eq!(b.verts[2][2], 0.0);
        let inner_r = ((b.verts[1][0] - 10.0).powi(2) + (b.verts[1][1] - 20.0).powi(2)).sqrt();
        let outer_r = ((b.verts[2][0] - 10.0).powi(2) + (b.verts[2][1] - 20.0).powi(2)).sqrt();
        assert!((inner_r - 4.75).abs() < 1e-4 && (outer_r - 5.25).abs() < 1e-4);
    }

    #[test]
    fn segment_count_tracks_pixel_radius_and_invisible_discs_are_skipped() {
        let b = DiscBatch::new(3.0);
        assert_eq!(b.segments(0.2), 8);
        assert!(b.segments(100.0) > b.segments(10.0));
        assert!(b.segments(10_000.0) <= 96);
        let mut e = DiscBatch::new(1.0);
        e.add(0.0, 0.0, 0.0, 1.0);
        e.add(0.0, 0.0, 4.0, 0.0);
        assert!(e.is_empty());
        let mut two = DiscBatch::new(1.0);
        two.add(0.0, 0.0, 4.0, 1.0);
        two.add(9.0, 0.0, 4.0, 1.0);
        let n = two.segments(4.0) as u32;
        assert_eq!(*two.indices.iter().max().unwrap(), 2 * (1 + 2 * n) - 1);
    }
}
