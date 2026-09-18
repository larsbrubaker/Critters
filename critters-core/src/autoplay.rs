//! Autoplay — a debug driver in the spirit of the original's `?debug`
//! hook: with `CRITTERS_AUTOPLAY=1` (native) or `?autoplay` (web) the game
//! plays itself, picking slots and dropping pieces near the stump centre,
//! so the physics, camera, tiers, wind and game-over flow can be watched or
//! screenshotted without a person at the controls.

use crate::config::W;
use crate::game::{Game, State};

/// Delay between autoplay actions, ms.
const ACTION_GAP: f64 = 500.0;

impl Game {
    /// Advance the autoplay driver; call once per frame after `step`.
    pub fn autoplay_step(&mut self) {
        if self.time - self.input.autoplay_last < ACTION_GAP {
            return;
        }
        self.input.autoplay_last = self.time;
        match self.state {
            State::Start => self.begin_play(),
            State::Over => self.init(true),
            State::Play => {
                if self.held.is_none() {
                    if let Some(i) = (0..3).find(|&i| self.slots[i].is_some()) {
                        self.select_slot(i);
                        self.held_x = W / 2.0 + self.rng.range(-20.0, 20.0);
                    }
                } else if self.can_drop() {
                    self.drop_piece();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autoplay_drives_a_round_to_completion_or_a_tower() {
        let mut g = Game::new(720.0, 0, false);
        let mut max_blocks = 0;
        for _ in 0..60 * 120 {
            g.step(1000.0 / 60.0);
            g.autoplay_step();
            max_blocks = max_blocks.max(g.blocks);
        }
        assert!(max_blocks >= 3, "autoplay only placed {max_blocks} pieces");
    }
}
