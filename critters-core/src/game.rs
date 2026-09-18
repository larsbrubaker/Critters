//! The game state machine and per-frame simulation, a line-for-line port of
//! the "Game flow" and "Simulation" sections of `reference/game.js`:
//! `init`, `beginPlay`, `selectSlot`, `drop`, `gameOver`, `step`,
//! `updateWind`, `windLean`, the camera, hints, toasts and the overlay card.
//! Pointer and keyboard handling lives in `input.rs`, the wind gusts in
//! `wind.rs`, the tray's shape offering in `tray.rs`, drawing in `render/`.
//! Sound effects are queued as [`Sfx`] events for the widget to hand to the
//! platform audio sink, and settings changes are flagged for persistence.

use crate::config::{
    ShapeDef, BELOW_TOP, DROP_FALLBACK, HOVER_GAP, HOVER_MARGIN, PX_PER_M, TIERS, W, WIND,
};
use crate::physics::{Physics, GROUND_USER_DATA};
use crate::piece::Piece;
use crate::rng::Rng;
use crate::scenery::Scenery;
pub use crate::wind::WindState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Start,
    Play,
    Over,
}

/// A sound effect requested by the game this frame (the `sfx` table).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sfx {
    Pick,
    Swap,
    Drop,
    /// Landing thud; strength 0..1 (impact speed / 9).
    Land(f64),
    Tier,
    Best,
    Wait,
    Over,
    Wind,
}

/// `#toast`: text plus the time its CSS animation (re)started.
#[derive(Clone, Debug)]
pub struct Toast {
    pub text: String,
    pub shown_at: f64,
}

/// `#overlay` and its card.
#[derive(Clone, Debug)]
pub struct Overlay {
    pub visible: bool,
    /// `.over`: the game-over variant (button shown, dismiss line hidden).
    pub over: bool,
    pub title: String,
    pub text: String,
    /// Game over only: the card has been dismissed so the tower can be
    /// inspected (not in the original, whose card blocked the stage).
    pub peek: bool,
    /// Game over only: the sentence naming the critter and the rule.
    pub reason: String,
}

pub const INTRO_TITLE: &str = "Critter Stack";
pub const INTRO_TEXT: &str =
    "Stack the forest critters as high as you can without letting anyone fall off the stump.";
pub const INTRO_SMALL: [&str; 3] = [
    "Each critter is worth points, multiplied by how high it sits. Bigger animals score more but need bigger shapes.",
    "Tap a critter below, move it over the tower, then tap or release to drop.",
    "Keyboard: 1 / 2 / 3 pick, \u{2190} \u{2192} move, Space drop.",
];
pub const TOAST_DURATION: f64 = 2700.0;
pub const TOAST_ANIMATION: f64 = 2600.0;

pub struct Game {
    pub rng: Rng,
    pub scenery: Scenery,
    /// Logical stage height (`H`), derived from the aspect ratio.
    pub h: f64,
    pub physics: Physics,
    pub placed: Vec<Piece>,
    pub slots: [Option<ShapeDef>; 3],
    /// Preview bodies for the tray (`slotBodies`).
    pub slot_pieces: [Option<Piece>; 3],
    /// Time each slot's `.pop` animation started.
    pub slot_pop_at: [f64; 3],
    pub held: Option<Piece>,
    pub held_x: f64,
    /// Hover height captured when the piece was picked up (only ever ratchets up).
    pub held_base_y: f64,
    /// Tray slot the held piece came from; it stays empty until the piece is dropped.
    pub held_slot: Option<usize>,
    pub last_dropped: Option<usize>,
    pub last_drop_time: f64,
    /// World y of the highest settled piece (<= 0).
    pub tower_top: f64,
    pub score: u32,
    pub round_best: u32,
    pub round_best_height: f64,
    pub blocks: u32,
    pub tier_reached: usize,
    pub beat_best: bool,
    pub best_score: u32,
    pub cam_y: f64,
    pub cam_y0: f64,
    pub state: State,
    pub time: f64,
    pub acc: f64,
    /// Debug only: pin the camera at a world y.
    pub cam_lock: Option<f64>,
    /// How far (px) the player has scrolled the view down from the automatic camera position.
    pub view_offset: f64,
    pub wind: WindState,
    pub hint: String,
    /// A shaken hint message stays until this time.
    pub hint_until: f64,
    /// When the hint's shake animation started.
    pub hint_shake_at: Option<f64>,
    pub toast: Option<Toast>,
    pub overlay: Overlay,
    pub muted: bool,
    /// Sounds requested since the last drain.
    pub sfx: Vec<Sfx>,
    /// Best score / mute changed since the last save.
    pub settings_dirty: bool,
    /// What ended the round (see `postmortem.rs`).
    pub postmortem: Option<crate::postmortem::Postmortem>,
    /// Transient pointer state (see `input.rs`).
    pub input: crate::input::InputState,
}

impl Game {
    /// `resize(); init();` with the persisted best score and mute state.
    pub fn new(h: f64, best_score: u32, muted: bool) -> Self {
        let mut rng = Rng::from_time();
        let scenery = Scenery::generate(&mut rng);
        let mut game = Self {
            rng,
            scenery,
            h,
            physics: Physics::new(),
            placed: Vec::new(),
            slots: [None, None, None],
            slot_pieces: [None, None, None],
            slot_pop_at: [-1000.0; 3],
            held: None,
            held_x: W / 2.0,
            held_base_y: 0.0,
            held_slot: None,
            last_dropped: None,
            last_drop_time: 0.0,
            tower_top: 0.0,
            score: 0,
            round_best: 0,
            round_best_height: 0.0,
            blocks: 0,
            tier_reached: 0,
            beat_best: false,
            best_score,
            cam_y: 0.0,
            cam_y0: 0.0,
            state: State::Start,
            time: 0.0,
            acc: 0.0,
            cam_lock: None,
            view_offset: 0.0,
            wind: WindState::default(),
            hint: String::new(),
            hint_until: 0.0,
            hint_shake_at: None,
            toast: None,
            overlay: Overlay {
                visible: true,
                over: false,
                title: INTRO_TITLE.to_string(),
                text: INTRO_TEXT.to_string(),
                peek: false,
                reason: String::new(),
            },
            muted,
            sfx: Vec::new(),
            settings_dirty: false,
            postmortem: None,
            input: crate::input::InputState::default(),
        };
        game.init(false);
        game
    }

    // ---------- Helpers ----------

    pub fn clamp_x(x: f64) -> f64 {
        x.clamp(30.0, W - 30.0)
    }

    pub fn fmt_m(m: f64) -> String {
        // `Math.max(0, -0)` is +0 in JS; avoid printing "-0.00 m"
        let m = if m == 0.0 { 0.0 } else { m };
        format!("{m:.2} m")
    }

    /// Highest point of any piece that has actually landed on the tower
    /// (ignores the piece still falling).
    pub fn landed_top(&self) -> f64 {
        let mut top = 0.0_f64;
        for b in &self.placed {
            if b.has_landed && b.bounds.min.y < top {
                top = b.bounds.min.y;
            }
        }
        top
    }

    /// The held piece keeps the hover height it was picked up at, unless that
    /// would bring it within HOVER_MARGIN of a landed piece, in which case it
    /// is lifted just enough (and stays lifted).
    pub fn held_y(&mut self) -> f64 {
        let Some(held) = &self.held else {
            return self.tower_top - HOVER_GAP;
        };
        let half_below = held.bounds.max.y - held.y;
        let clear = self.landed_top() - HOVER_MARGIN - half_below;
        if clear < self.held_base_y {
            self.held_base_y = clear;
        }
        self.held_base_y
    }

    pub fn tower_settled(&self) -> bool {
        self.placed
            .iter()
            .all(|b| b.locked || b.bounds.max.y > BELOW_TOP)
    }

    pub fn can_drop(&self) -> bool {
        self.tower_settled() || self.time - self.last_drop_time > DROP_FALLBACK
    }

    pub fn set_hint(&mut self, text: &str, shake: bool) {
        if self.hint != text {
            self.hint = text.to_string();
        }
        self.hint_shake_at = None;
        if shake {
            self.hint_shake_at = Some(self.time);
            self.hint_until = self.time + 1400.0;
        }
    }

    pub fn show_toast(&mut self, text: String) {
        self.toast = Some(Toast {
            text,
            shown_at: self.time,
        });
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        self.settings_dirty = true;
    }

    // ---------- Game flow ----------

    pub fn init(&mut self, skip_intro: bool) {
        self.physics = Physics::new();
        self.placed.clear();
        self.postmortem = None;
        self.held = None;
        self.last_dropped = None;
        self.blocks = 0;
        self.score = 0;
        self.round_best = 0;
        self.round_best_height = 0.0;
        self.tier_reached = 0;
        self.beat_best = false;
        self.view_offset = 0.0;
        self.wind = WindState {
            active: false,
            dir: 1.0,
            start: 0.0,
            next_at: self.time + WIND.min_gap,
            strength: 0.0,
        };
        self.tower_top = 0.0;
        // stump top sits near the bottom of the view; the floor just below it
        self.cam_y0 = -(self.h - 190.0);
        self.cam_y = self.cam_y0;
        self.held_x = W / 2.0;
        self.held_slot = None;
        let a = self.flat_spec();
        let b = self.flat_spec();
        let c = self.new_spec();
        self.slots = [Some(a), Some(b), Some(c)];
        for i in 0..3 {
            self.render_slot(i);
        }
        self.state = State::Start;
        self.overlay = Overlay {
            visible: true,
            over: false,
            title: INTRO_TITLE.to_string(),
            text: INTRO_TEXT.to_string(),
            peek: false,
            reason: String::new(),
        };
        self.toast = None;
        self.set_hint("Pick a critter below to start", false);
        if skip_intro {
            self.begin_play();
        }
    }

    pub fn begin_play(&mut self) {
        if self.state != State::Start {
            return;
        }
        self.state = State::Play;
        self.overlay.visible = false;
        self.set_hint("Pick a critter below", false);
    }

    pub fn select_slot(&mut self, i: usize) {
        if self.state == State::Over {
            return;
        }
        let Some(spec) = self.slots[i] else {
            return;
        };
        self.begin_play();
        if let Some(held) = &self.held {
            // swap: the held critter goes back into this slot (hover height kept)
            self.slots[i] = Some(held.spec);
            self.sfx.push(Sfx::Swap);
        } else {
            // the slot stays empty until the piece is dropped, so you cannot fish for easier shapes
            self.slots[i] = None;
            self.held_slot = Some(i);
            self.held_base_y = self.tower_top - HOVER_GAP;
            self.sfx.push(Sfx::Pick);
        }
        let piece = self.make_piece(spec, self.held_x, self.held_base_y);
        self.held = Some(piece);
        self.view_offset = 0.0;
        self.render_slot(i);
        self.slot_pop_at[i] = self.time;
        self.set_hint("Move it over the tower, then tap or release to drop", false);
    }

    /// `drop()`: release the held piece into the world.
    pub fn drop_piece(&mut self) {
        if self.held.is_none() || self.state != State::Play {
            return;
        }
        if !self.can_drop() {
            self.set_hint("Wait for the tower to settle\u{2026}", true);
            self.sfx.push(Sfx::Wait);
            return;
        }
        self.view_offset = 0.0;
        self.sfx.push(Sfx::Drop);
        let y = self.held_y();
        let Some(mut held) = self.held.take() else {
            return;
        };
        held.set_position(self.held_x, y);
        let index = self.placed.len();
        held.add_to_world(&mut self.physics, index);
        self.placed.push(held);
        self.last_dropped = Some(index);
        self.last_drop_time = self.time;
        self.blocks += 1;
        if let Some(slot) = self.held_slot.take() {
            let spec = self.new_spec();
            self.slots[slot] = Some(spec);
            self.render_slot(slot);
        }
        self.set_hint("Pick the next critter", false);
    }

    pub fn game_over(&mut self, reason: Option<String>) {
        self.state = State::Over;
        self.held = None;
        self.sfx.push(Sfx::Over);
        let reason_text = reason.clone().unwrap_or_default();
        let plural = if self.blocks == 1 { "" } else { "s" };
        let mut text = format!(
            "{}Score {} \u{B7} {} tall \u{B7} {} critter{}.",
            reason.map(|r| r + " ").unwrap_or_default(),
            self.round_best,
            Self::fmt_m(self.round_best_height),
            self.blocks,
            plural
        );
        if self.round_best >= self.best_score && self.round_best > 0 {
            text.push_str(" New best!");
        }
        self.overlay = Overlay {
            visible: true,
            over: true,
            title: "Timber!".to_string(),
            text,
            peek: false,
            reason: reason_text,
        };
        self.set_hint("", false);
    }

    /// `resize()`: the stage's logical height changed.
    pub fn resize(&mut self, h: f64) {
        self.h = h;
        self.cam_y0 = -(self.h - 190.0);
        if self.state == State::Start {
            self.cam_y = self.cam_y0;
        }
        for i in 0..3 {
            self.render_slot(i);
        }
    }

    // ---------- Simulation ----------

    /// One physics step plus the per-step bookkeeping the original did inside
    /// `Engine.update` (collision events) and right after it (`trackStillness`).
    fn physics_step(&mut self) {
        let prev_speed: Vec<f64> = self.placed.iter().map(|b| b.speed).collect();
        self.physics.step();
        for b in &mut self.placed {
            b.sync_from(&self.physics);
        }
        for (a, b) in self.physics.begin_contacts() {
            for user in [a, b] {
                if user == GROUND_USER_DATA {
                    continue;
                }
                let i = user as usize;
                if let Some(piece) = self.placed.get_mut(i) {
                    if !piece.has_landed {
                        piece.has_landed = true;
                        piece.land_at = self.time;
                        let v = prev_speed.get(i).copied().unwrap_or(piece.speed);
                        self.sfx.push(Sfx::Land(v / 9.0));
                    }
                }
            }
        }
        for b in &mut self.placed {
            b.track_stillness();
        }
    }

    /// `step(dt)`, dt in ms.
    pub fn step(&mut self, dt: f64) {
        self.time += dt;
        if self.state != State::Start {
            self.acc = (self.acc + dt).min(100.0);
            while self.acc >= 1000.0 / 60.0 {
                self.physics_step();
                self.acc -= 1000.0 / 60.0;
            }
        }

        let mut top = 0.0_f64;
        let mut s = 0.0;
        for b in &mut self.placed {
            b.track_lock();
            if !b.locked {
                continue;
            }
            if let Some(rest) = b.rest {
                if rest.top < top {
                    top = rest.top;
                }
            }
            s += b.score();
        }
        self.tower_top = top;
        self.score = s.round() as u32;
        if self.state == State::Play {
            let tower_top = self.tower_top;
            for b in &mut self.placed {
                b.apply_stability(&mut self.physics, tower_top);
            }
        }

        if self.state == State::Play {
            let h = -self.tower_top / PX_PER_M;
            if h > self.round_best_height {
                self.round_best_height = h;
            }
            if self.score > self.round_best {
                self.round_best = self.score;
            }
            if self.round_best > self.best_score {
                if self.best_score > 0 && !self.beat_best {
                    self.beat_best = true;
                    self.sfx.push(Sfx::Best);
                }
                self.best_score = self.round_best;
                self.settings_dirty = true;
            }
            // tier toasts
            let mut i = TIERS.len() - 1;
            while i > self.tier_reached {
                if self.round_best_height >= TIERS[i].at {
                    self.tier_reached = i;
                    self.show_toast(format!("{} {}", TIERS[i].icon, TIERS[i].name));
                    self.sfx.push(Sfx::Tier);
                    break;
                }
                i -= 1;
            }
            let fallen = self
                .placed
                .iter()
                .enumerate()
                .find_map(|(i, b)| b.fall_reason(self.tower_top).map(|why| (i, why)));
            if let Some((piece, why)) = fallen {
                self.end_round(piece, why);
            }
        }

        if self.held.is_some() {
            let y = self.held_y();
            let x = self.held_x;
            if let Some(held) = &mut self.held {
                held.set_position(x, y);
            }
        }

        if self.state == State::Play {
            self.update_wind();
            if self.held.is_some() && self.time > self.hint_until {
                let text = if self.can_drop() {
                    "Move it over the tower, then tap or release to drop"
                } else {
                    "Waiting for the tower to settle\u{2026}"
                };
                self.set_hint(text, false);
            }
        }

        let focus = match &self.held {
            Some(held) => held.y,
            None => self.tower_top - HOVER_GAP,
        };
        let auto_target = self.cam_y0.min(focus - self.h * 0.3);
        self.view_offset = self
            .view_offset
            .clamp(0.0, (self.cam_y0 - auto_target).max(0.0));
        let target = auto_target + self.view_offset;
        self.cam_y = match self.cam_lock {
            Some(lock) => lock,
            None => self.cam_y + (target - self.cam_y) * 0.08,
        };

        for c in &mut self.scenery.clouds {
            c.x += c.v * dt / 1000.0;
            if c.x > W + 140.0 {
                c.x = -140.0;
            }
        }
        for b in &mut self.scenery.birds {
            b.x += b.v * b.dir * dt / 1000.0;
            if b.x > W + 40.0 {
                b.x = -40.0;
            }
            if b.x < -40.0 {
                b.x = W + 40.0;
            }
        }

        if let Some(t) = &self.toast {
            if self.time - t.shown_at > TOAST_DURATION {
                self.toast = None;
            }
        }
    }

    /// Take the sounds requested since the last call.
    pub fn take_sfx(&mut self) -> Vec<Sfx> {
        std::mem::take(&mut self.sfx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ShapeKind, STUMP_W, TIERS};

    fn game() -> Game {
        Game::new(720.0, 0, false)
    }

    /// Run frames of 1000/60 ms.
    fn run(g: &mut Game, frames: usize) {
        for _ in 0..frames {
            g.step(1000.0 / 60.0);
        }
    }

    #[test]
    fn starts_on_the_intro_with_two_flat_slots() {
        let g = game();
        assert_eq!(g.state, State::Start);
        assert!(g.overlay.visible && !g.overlay.over);
        assert_eq!(g.hint, "Pick a critter below to start");
        for slot in &g.slots[..2] {
            let s = slot.unwrap();
            assert!(!s.big);
            assert!(matches!(
                s.kind,
                ShapeKind::Rect { .. } | ShapeKind::Trap { .. }
            ));
        }
        assert!(g.slots[2].is_some());
        assert_eq!(g.cam_y0, -(720.0 - 190.0));
    }

    #[test]
    fn selecting_a_slot_starts_play_and_empties_the_slot() {
        let mut g = game();
        g.select_slot(1);
        assert_eq!(g.state, State::Play);
        assert!(!g.overlay.visible);
        assert!(g.slots[1].is_none());
        assert_eq!(g.held_slot, Some(1));
        assert_eq!(g.held_base_y, -HOVER_GAP);
        assert_eq!(g.take_sfx(), vec![Sfx::Pick]);
        // swapping with another slot keeps the hover height and refills the slot
        let other = g.slots[0].unwrap();
        let held = g.held.as_ref().unwrap().spec;
        g.select_slot(0);
        assert_eq!(g.slots[0].unwrap(), held);
        assert_eq!(g.held.as_ref().unwrap().spec, other);
        assert_eq!(g.held_slot, Some(1));
        assert_eq!(g.take_sfx(), vec![Sfx::Swap]);
    }

    #[test]
    fn dropped_piece_lands_scores_and_raises_the_tower() {
        let mut g = game();
        g.select_slot(0);
        let spec = g.held.as_ref().unwrap().spec;
        g.drop_piece();
        assert!(g.held.is_none());
        assert_eq!(g.placed.len(), 1);
        assert!(g.slots[0].is_some(), "the slot refills after the drop");
        assert_eq!(g.hint, "Pick the next critter");
        run(&mut g, 240);
        assert_eq!(g.state, State::Play);
        let p = &g.placed[0];
        assert!(
            p.has_landed && p.locked,
            "landed={} locked={}",
            p.has_landed,
            p.locked
        );
        assert!(g.tower_top < 0.0);
        let expected = (spec.points as f64 * (1.0 + -p.rest.unwrap().y / PX_PER_M)).round() as u32;
        assert_eq!(g.score, expected);
        assert_eq!(g.best_score, g.score);
        assert!(g.settings_dirty);
        assert!(g.take_sfx().iter().any(|s| matches!(s, Sfx::Land(_))));
    }

    #[test]
    fn cannot_drop_while_the_tower_is_unsettled() {
        let mut g = game();
        g.select_slot(0);
        g.drop_piece();
        run(&mut g, 3);
        g.select_slot(1);
        g.take_sfx();
        g.drop_piece();
        assert!(
            g.held.is_some(),
            "second drop must wait for the first to settle"
        );
        assert_eq!(g.hint, "Wait for the tower to settle\u{2026}");
        assert!(g.hint_shake_at.is_some());
        assert_eq!(g.take_sfx(), vec![Sfx::Wait]);
    }

    #[test]
    fn piece_off_the_stump_ends_the_round_with_a_reason() {
        let mut g = game();
        g.select_slot(0);
        g.held_x = W / 2.0 + STUMP_W / 2.0 + 25.0;
        g.step(16.0);
        g.drop_piece();
        let critter = g.placed[0].spec.critter.name().to_string();
        run(&mut g, 300);
        assert_eq!(g.state, State::Over);
        assert!(g.overlay.visible && g.overlay.over);
        assert_eq!(g.overlay.title, "Timber!");
        assert!(
            g.overlay.text.starts_with(&format!("The {critter} ")),
            "{}",
            g.overlay.text
        );
        assert!(g.overlay.text.contains("1 critter."));
        assert!(g.take_sfx().contains(&Sfx::Over));
        // play again restarts straight into play
        g.init(true);
        assert_eq!(g.state, State::Play);
        assert!(g.placed.is_empty());
        assert_eq!(g.blocks, 0);
    }

    #[test]
    fn tier_toast_and_wind_only_at_altitude() {
        let mut g = game();
        g.begin_play();
        // pretend a settled tower 8.5 m tall
        g.tower_top = -850.0;
        g.round_best_height = 8.5;
        // tier toasts key off round_best_height in step(); force one pass
        g.tier_reached = 0;
        let mut i = TIERS.len() - 1;
        while i > g.tier_reached {
            if g.round_best_height >= TIERS[i].at {
                g.tier_reached = i;
                break;
            }
            i -= 1;
        }
        assert_eq!(g.tier_reached, 2);
        assert_eq!(g.current_tier(), 2);
        g.wind.next_at = 0.0;
        g.time = 1.0;
        g.update_wind();
        assert!(g.wind.active);
        assert_eq!(g.take_sfx(), vec![Sfx::Wind]);
        g.time = g.wind.start + WIND.duration / 2.0;
        g.update_wind();
        assert!((g.wind.strength - 1.0).abs() < 1e-9);
        // on the forest floor the gust never fires
        let mut low = game();
        low.begin_play();
        low.wind.next_at = 0.0;
        low.time = 1.0;
        low.update_wind();
        assert!(!low.wind.active);
        assert!(low.wind.next_at >= low.time + WIND.min_gap);
    }

    #[test]
    fn camera_follows_the_tower_and_view_offset_is_clamped() {
        // a short stage so even one block pushes the camera up
        let mut g = Game::new(300.0, 0, false);
        g.select_slot(0);
        g.drop_piece();
        run(&mut g, 240);
        assert!(g.placed[0].locked);
        let auto_target = g.cam_y0.min(g.tower_top - HOVER_GAP - g.h * 0.3);
        assert!(
            auto_target < g.cam_y0,
            "camera target climbs toward the tower top"
        );
        assert!(g.cam_y < g.cam_y0);
        g.view_offset = 10.0;
        g.step(16.0);
        assert_eq!(g.view_offset, 10.0);
        g.view_offset = 1e9;
        g.step(16.0);
        assert_eq!(g.view_offset, g.cam_y0 - auto_target);
        g.view_offset = -50.0;
        g.step(16.0);
        assert_eq!(g.view_offset, 0.0);
    }

    #[test]
    fn toast_expires_after_its_duration() {
        let mut g = game();
        g.show_toast("hi".into());
        g.step(TOAST_DURATION - 1.0);
        assert!(g.toast.is_some());
        g.step(2.0);
        assert!(g.toast.is_none());
    }
}
