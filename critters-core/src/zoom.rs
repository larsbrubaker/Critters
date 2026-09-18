//! Sideways building and the zoom that goes with it — a deliberate change
//! from the original. There, a piece whose centre drifted 8 px past the
//! stump's edge lost the round even while it rested happily on the tower.
//! Here anything that can be balanced is legal, as far out as the player
//! dares; only a piece that hits the ground ends the round
//! (`Piece::fall_reason`). To keep a wide tower in view the world layer
//! zooms out around the focus point, and the range the held piece can be
//! moved over widens with it.
//!
//! World → stage mapping (logical stage units, `z` = zoom ≤ 1, pivot `P` =
//! 30 % of the stage height, where the camera keeps its focus):
//!
//! ```text
//! sx = W/2 + (wx - W/2)·z
//! sy = P   + (wy - cam_y - P)·z
//! ```

use crate::config::W;
use crate::game::Game;

/// Never zoom out further than this.
pub const MIN_ZOOM: f64 = 0.35;
/// Clear space kept between the widest landed piece and the stage edge.
const SIDE_MARGIN: f64 = 70.0;
/// The held piece can be moved to within this much of the visible edge.
const HELD_INSET: f64 = 30.0;

impl Game {
    /// Screen y (at zoom 1) the zoom is centred on: the camera's focus line.
    pub fn zoom_pivot(&self) -> f64 {
        self.h * 0.3
    }

    /// Zoom that keeps every landed piece on stage with a margin.
    pub fn zoom_target(&self) -> f64 {
        let mut half = 0.0_f64;
        for b in self.placed.iter().filter(|b| b.has_landed && !b.hit_ground) {
            half = half
                .max((b.bounds.min.x - W / 2.0).abs())
                .max((b.bounds.max.x - W / 2.0).abs());
        }
        ((W / 2.0) / (half + SIDE_MARGIN)).clamp(MIN_ZOOM, 1.0)
    }

    /// Ease the zoom toward its target (called once per frame from `step`).
    pub fn update_zoom(&mut self) {
        let target = self.zoom_target();
        self.zoom += (target - self.zoom) * 0.05;
        if (self.zoom - target).abs() < 1e-4 {
            self.zoom = target;
        }
    }

    /// Half the world width currently on stage.
    pub fn visible_half_width(&self) -> f64 {
        W / (2.0 * self.zoom)
    }

    /// World y span `(top, bottom)` currently on stage.
    pub fn visible_y_span(&self) -> (f64, f64) {
        let p = self.zoom_pivot();
        (
            self.cam_y + p - p / self.zoom,
            self.cam_y + p + (self.h - p) / self.zoom,
        )
    }

    /// Logical stage x (0..W) → world x under the current zoom.
    pub fn stage_to_world_x(&self, stage_x: f64) -> f64 {
        W / 2.0 + (stage_x - W / 2.0) / self.zoom
    }

    /// `clampX`, widened by the zoom: the held piece may go anywhere on stage.
    pub fn clamp_held_x(&self, world_x: f64) -> f64 {
        let reach = (self.visible_half_width() - HELD_INSET).max(0.0);
        world_x.clamp(W / 2.0 - reach, W / 2.0 + reach)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SHAPES;
    use crate::piece::Piece;

    fn landed(x: f64) -> Piece {
        let mut p = Piece::new(SHAPES[0], x, -30.0, 0.0); // 120-wide log
        p.has_landed = true;
        p
    }

    #[test]
    fn no_zoom_until_the_tower_nears_the_stage_edge() {
        let mut g = Game::new(650.0, 0, false);
        assert_eq!(g.zoom_target(), 1.0);
        g.placed.push(landed(W / 2.0 + 100.0)); // reaches 160 from centre
        assert_eq!(g.zoom_target(), 1.0, "160 + 70 margin still fits in 240");
        assert_eq!(
            g.clamp_held_x(10_000.0),
            W - 30.0,
            "original clampX at zoom 1"
        );
        assert_eq!(g.clamp_held_x(-10_000.0), 30.0);
    }

    #[test]
    fn a_wide_tower_zooms_out_and_widens_the_reach() {
        let mut g = Game::new(650.0, 0, false);
        g.placed.push(landed(W / 2.0 + 400.0)); // reaches 460 from centre
        let target = g.zoom_target();
        assert!((target - 240.0 / 530.0).abs() < 1e-9);
        for _ in 0..400 {
            g.update_zoom();
        }
        assert_eq!(g.zoom, target);
        assert!((g.visible_half_width() - 530.0).abs() < 1e-9);
        assert!((g.clamp_held_x(10_000.0) - (W / 2.0 + 500.0)).abs() < 1e-9);
        // the stage centre maps to the world centre, the edge to the far reach
        assert_eq!(g.stage_to_world_x(W / 2.0), W / 2.0);
        assert!((g.stage_to_world_x(W) - (W / 2.0 + 530.0)).abs() < 1e-9);
        // pieces on the ground or still falling do not drive the zoom
        let mut fallen = landed(W / 2.0 + 5000.0);
        fallen.hit_ground = true;
        g.placed.push(fallen);
        assert_eq!(g.zoom_target(), target);
        // and it never goes below the floor
        g.placed.push(landed(W / 2.0 + 50_000.0));
        assert_eq!(g.zoom_target(), MIN_ZOOM);
    }

    /// The case from the field: a block resting on the overhanging end of a
    /// great log, its centre past the stump's edge. The original ended the
    /// round ("went over the side"); here it stands, and only a piece that
    /// reaches the forest floor loses.
    #[test]
    fn a_balanced_overhang_is_legal_and_the_ground_is_not() {
        use crate::config::STUMP_W;
        use crate::game::State;
        let mut g = Game::new(650.0, 0, false);
        let place = |g: &mut Game, shape: usize, x: f64| {
            g.slots[0] = Some(SHAPES[shape]);
            g.select_slot(0);
            g.held_x = x;
            g.step(16.0);
            g.drop_piece();
            for _ in 0..360 {
                g.step(1000.0 / 60.0);
            }
        };
        place(&mut g, 7, 320.0); // great log, 180 wide: spans 230..410 over a stump ending at 370
        place(&mut g, 1, 385.0); // block on the overhang
        let block = &g.placed[1];
        assert!(block.locked, "the block came to rest");
        assert!(
            (block.x - W / 2.0).abs() > STUMP_W / 2.0 + 8.0,
            "its centre ({}) is past the original's edge tolerance",
            block.x
        );
        assert_eq!(
            g.state,
            State::Play,
            "a balanced overhang does not end the round"
        );
        assert!(g.score > 0);
        // now drop one straight past everything, onto the forest floor
        place(&mut g, 1, W / 2.0 + 215.0);
        assert_eq!(g.state, State::Over);
        assert!(
            g.overlay.text.contains("hit the ground"),
            "{}",
            g.overlay.text
        );
        assert!(
            g.placed[2].hit_ground,
            "reported by the floor contact event"
        );
    }

    #[test]
    fn visible_span_grows_around_the_pivot() {
        let mut g = Game::new(600.0, 0, false);
        let (t1, b1) = g.visible_y_span();
        assert!((b1 - t1 - 600.0).abs() < 1e-9);
        g.zoom = 0.5;
        let (t2, b2) = g.visible_y_span();
        assert!((b2 - t2 - 1200.0).abs() < 1e-9);
        // the focus line (30 % down) stays put
        assert!(((t1 + 0.3 * 600.0) - (t2 + 0.3 * 1200.0)).abs() < 1e-9);
    }
}
