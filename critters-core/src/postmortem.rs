//! What ended the round, kept so the player can see it. The original only
//! put a sentence on the game-over card; on a tall tower that is not enough
//! to tell *which* critter counted as fallen or why (a resting piece whose
//! centre has drifted past the stump's edge loses the round without anything
//! visibly falling). So the port records the culprit, rings it on screen,
//! points the camera at it, lets the card be dismissed to scroll the tower,
//! and writes the full rule inputs to the debug log.

use crate::config::{BELOW_TOP, EDGE_TOLERANCE, FALL_DROP, STUMP_W, W};
use crate::game::Game;
use crate::piece::FallReason;

/// The piece and rule that ended the round.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Postmortem {
    /// Index into `Game::placed`.
    pub piece: usize,
    pub reason: FallReason,
    /// Game time of the game over (drives the ring's pulse).
    pub at: f64,
}

impl Game {
    /// End the round because `placed[piece]` tripped `reason`.
    pub fn end_round(&mut self, piece: usize, reason: FallReason) {
        let Some(b) = self.placed.get(piece) else {
            return;
        };
        let critter = b.spec.critter.name();
        // the numbers each rule in `Piece::fall_reason` looks at
        crate::debug::log(&format!(
            "critters gameover: {critter} #{piece} of {} — {} | x {:.1} (|dx| {:.1}, edge {:.1}+{EDGE_TOLERANCE}) y {:.1} bottom {:.1} (below-top limit {BELOW_TOP}) | tower_top {:.1} | highest_rest_y {:?} (drop limit {FALL_DROP}) | locked {} landed {} static {} sleeping {} speed {:.3} angle {:.3}",
            self.placed.len(),
            reason.text(),
            b.x,
            (b.x - W / 2.0).abs(),
            STUMP_W / 2.0,
            b.y,
            b.bounds.max.y,
            self.tower_top,
            b.highest_rest_y,
            b.locked,
            b.has_landed,
            b.is_static,
            b.is_sleeping,
            b.speed,
            b.angle,
        ));
        let culprit_y = b.y;
        self.postmortem = Some(Postmortem {
            piece,
            reason,
            at: self.time,
        });
        self.game_over(Some(format!("The {critter} {}!", reason.text())));
        self.look_at(culprit_y);
    }

    /// Scroll the view so world y is about mid-screen (within the limits the
    /// camera allows: never above the automatic position, never below ground).
    pub fn look_at(&mut self, world_y: f64) {
        let focus = self.tower_top - crate::config::HOVER_GAP;
        let auto_target = self.cam_y0.min(focus - self.h * 0.3);
        let wanted_cam = world_y - self.h * 0.5;
        self.view_offset =
            (wanted_cam - auto_target).clamp(0.0, (self.cam_y0 - auto_target).max(0.0));
    }

    /// The culprit piece, while the round is over.
    pub fn culprit(&self) -> Option<(&crate::piece::Piece, Postmortem)> {
        let pm = self.postmortem?;
        (self.state == crate::game::State::Over)
            .then(|| self.placed.get(pm.piece).map(|b| (b, pm)))
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::State;

    #[test]
    fn end_round_records_the_culprit_and_points_the_camera_at_it() {
        let mut g = Game::new(650.0, 0, false);
        g.stress_tower(40, false);
        g.step(16.0);
        assert_eq!(g.state, State::Play);
        g.end_round(3, FallReason::WentOverTheSide);
        assert_eq!(g.state, State::Over);
        let (b, pm) = g.culprit().expect("culprit recorded");
        assert_eq!(pm.piece, 3);
        assert_eq!(pm.reason, FallReason::WentOverTheSide);
        assert!(g.overlay.text.contains("went over the side"));
        // piece 3 is near the bottom of a ~20 m tower: the view scrolled down to it
        let focus = g.tower_top - crate::config::HOVER_GAP;
        let cam = g.cam_y0.min(focus - g.h * 0.3) + g.view_offset;
        assert!(
            b.y > cam && b.y < cam + g.h,
            "culprit y {} not in view {cam}..{}",
            b.y,
            cam + g.h
        );
        // a new round forgets it
        g.init(true);
        assert!(g.culprit().is_none());
    }
}
