//! Wind gusts — `updateWind`, `windLean`, `currentTier` and the debug
//! `gust()` hook from `reference/game.js`. Gusts only blow in the Open Sky,
//! Cloud Country and Stratosphere tiers and are purely visual: `game.rs`
//! never feeds them into the physics, so the tower is drawn leaning
//! (`render/world.rs` applies `wind_lean`) but can never be blown over.

use crate::config::{PX_PER_M, TIERS, WIND};
use crate::game::{Game, Sfx};
use crate::piece::Piece;

#[derive(Clone, Debug, Default)]
pub struct WindState {
    pub active: bool,
    pub dir: f64,
    pub start: f64,
    pub next_at: f64,
    pub strength: f64,
}

impl Game {
    pub fn current_tier(&self) -> usize {
        let h = -self.tower_top / PX_PER_M;
        let mut t = 0;
        for (i, tier) in TIERS.iter().enumerate() {
            if h >= tier.at {
                t = i;
            }
        }
        t
    }

    pub(crate) fn update_wind(&mut self) {
        let tier = self.current_tier();
        let eligible = (WIND.min_tier..=WIND.max_tier).contains(&tier);
        if !eligible {
            self.wind.next_at = self.wind.next_at.max(self.time + WIND.min_gap);
        }
        if !self.wind.active && eligible && self.time >= self.wind.next_at {
            self.wind.active = true;
            self.wind.dir = self.rng.sign();
            self.wind.start = self.time;
            self.sfx.push(Sfx::Wind);
        }
        if !self.wind.active {
            return;
        }
        let p = (self.time - self.wind.start) / WIND.duration;
        if p >= 1.0 {
            self.wind.active = false;
            self.wind.strength = 0.0;
            self.wind.next_at =
                self.time + WIND.min_gap + self.rng.random() * (WIND.max_gap - WIND.min_gap);
            return;
        }
        self.wind.strength = (p * std::f64::consts::PI).sin();
    }

    /// Visual lean for a piece during a gust: grows with its height up the
    /// tower (like a bending reed). Returns `(dx, rot)`.
    pub fn wind_lean(&self, b: &Piece) -> (f64, f64) {
        if self.wind.strength <= 0.0 || b.is_static {
            return (0.0, 0.0);
        }
        let span = (-self.tower_top).max(300.0);
        let f = ((-b.y).max(0.0) / span).powf(1.5);
        let shiver = (self.time / 90.0 + b.y * 0.02).sin() * WIND.sway * f;
        (
            self.wind.dir * self.wind.strength * (WIND.lean * f + shiver),
            self.wind.dir * self.wind.strength * WIND.tilt * f,
        )
    }

    /// Debug hook: force a gust like `window.__cs.gust(dir)`.
    pub fn gust(&mut self, dir: f64) {
        self.wind.active = true;
        self.wind.dir = dir;
        self.wind.start = self.time;
    }
}
