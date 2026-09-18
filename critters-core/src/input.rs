//! Pointer and keyboard handling — the "Input" section of `reference/game.js`
//! (canvas pointer drag/scroll/drop, the window-level pointer follow, tray
//! slot pick-and-drag, the overlay button, the mute button and the keydown
//! table). Coordinates are "app CSS pixels": y down, relative to the app
//! column's top-left corner, exactly like the original's `clientX/clientY`
//! minus the canvas offset. `widget.rs` converts agg-gui events into this
//! space; `render/hud.rs` publishes the overlay button's rectangle.

use crate::config::{APP_MAX_WIDTH, TRAY_HEIGHT, W};
use crate::game::{Game, Sfx, State};

/// The on-screen geometry the input code shares with the renderer.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Layout {
    /// Width of the app column (`#app`, max 560 px).
    pub app_w: f64,
    /// Height of the stage (`#stage`), i.e. the widget height minus the tray.
    pub stage_h: f64,
    /// CSS px per logical stage px (`rect.width / W`).
    pub scale: f64,
}

/// A rectangle in app CSS px.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
    }
}

impl Layout {
    /// Fit the app column into a widget of the given size (CSS px).
    pub fn fit(widget_w: f64, widget_h: f64) -> Self {
        let app_w = widget_w.clamp(1.0, APP_MAX_WIDTH);
        let stage_h = (widget_h - TRAY_HEIGHT).max(1.0);
        Self {
            app_w,
            stage_h,
            scale: app_w / W,
        }
    }

    /// Logical stage height `H`.
    pub fn stage_logical_h(&self) -> f64 {
        self.stage_h / self.scale
    }

    /// `.slot` rectangle i (tray padding 10, gap 10, 5 px top border).
    pub fn slot_rect(&self, i: usize) -> Rect {
        let inner = self.app_w - 20.0;
        let w = (inner - 20.0) / 3.0;
        Rect {
            x: 10.0 + i as f64 * (w + 10.0),
            y: self.stage_h + 5.0 + 10.0,
            w,
            h: 112.0,
        }
    }

    /// `#mute`: 40 × 40, 10 px from the stage's bottom-right corner.
    pub fn mute_rect(&self) -> Rect {
        Rect {
            x: self.app_w - 50.0,
            y: self.stage_h - 50.0,
            w: 40.0,
            h: 40.0,
        }
    }

    /// Full-screen toggle (touch devices only): a second round button to
    /// the left of the mute button. Not part of the original, which relied
    /// on the browser chrome; on phones the game is far better full screen.
    pub fn fullscreen_rect(&self) -> Rect {
        Rect {
            x: self.app_w - 100.0,
            y: self.stage_h - 50.0,
            w: 40.0,
            h: 40.0,
        }
    }

    /// Whether the full-screen button is shown (mobile / touch profile).
    pub fn shows_fullscreen_button() -> bool {
        agg_gui::input_profile::is_mobile_touch()
    }

    pub fn slot_at(&self, x: f64, y: f64) -> Option<usize> {
        (0..3).find(|&i| self.slot_rect(i).contains(x, y))
    }
}

/// Keys the game reacts to (`keydown` table).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameKey {
    Digit(u8),
    Left,
    Right,
    Up,
    Down,
    Space,
    Enter,
    Escape,
}

/// Transient pointer state (`dragStart`, `dragLastY`, `scrolling`, …).
#[derive(Clone, Debug, Default)]
pub struct InputState {
    pub dragging: bool,
    pub drag_start: Option<(f64, f64)>,
    pub drag_last_y: f64,
    pub scrolling: bool,
    /// A press that started on a tray slot: its start y (drag straight up to drop).
    pub slot_drag: Option<f64>,
    /// Slot currently pressed (`:active` scale).
    pub slot_active: Option<usize>,
    /// The overlay button was pressed and a release inside it counts as a click.
    pub button_armed: bool,
    /// Last pointer position, for hover styling.
    pub pointer: Option<(f64, f64)>,
    /// Overlay "Play again" button rectangle as last painted.
    pub overlay_button: Option<Rect>,
    /// Time of the last autoplay action (see `autoplay.rs`).
    pub autoplay_last: f64,
}

impl Game {
    /// `toX(e)`: pointer x → clamped logical stage x.
    pub fn to_x(&self, layout: &Layout, x: f64) -> f64 {
        Self::clamp_x(x / layout.scale)
    }

    pub fn pointer_down(&mut self, layout: &Layout, x: f64, y: f64) {
        self.input.pointer = Some((x, y));
        if Layout::shows_fullscreen_button() && layout.fullscreen_rect().contains(x, y) {
            // handled by the platform shell on the next frame, inside the gesture
            agg_gui::fullscreen::request_toggle();
            return;
        }
        if layout.mute_rect().contains(x, y) {
            // muteBtn pointerdown stops propagation
            let muted = !self.muted;
            self.set_muted(muted);
            if !muted {
                self.sfx.push(Sfx::Pick);
            }
            return;
        }
        if y < layout.stage_h {
            if self.state == State::Over {
                let on_button = self
                    .input
                    .overlay_button
                    .map(|r| r.contains(x, y))
                    .unwrap_or(false);
                if on_button {
                    self.input.button_armed = true;
                } else if !self.overlay.peek {
                    // a tap off the button puts the card away so the tower,
                    // and the ringed culprit, can be looked at
                    self.overlay.peek = true;
                    let hint = format!("{} Drag to look around", self.overlay.reason);
                    self.set_hint(hint.trim(), false);
                } else {
                    // peeking: drags scroll the view
                    self.input.drag_start = Some((x, y));
                    self.input.drag_last_y = y;
                    self.input.scrolling = false;
                    self.input.dragging = true;
                }
                return;
            }
            if self.state == State::Start {
                self.begin_play();
                return;
            }
            self.input.drag_start = Some((x, y));
            self.input.drag_last_y = y;
            self.input.scrolling = false;
            self.input.dragging = true;
            if self.held.is_some() {
                self.held_x = self.to_x(layout, x);
            }
            return;
        }
        if let Some(i) = layout.slot_at(x, y) {
            if self.slots[i].is_none() || self.state == State::Over {
                return;
            }
            self.input.slot_active = Some(i);
            self.select_slot(i);
            self.held_x = self.to_x(layout, x);
            // drag straight up onto the stage and release to drop
            self.input.slot_drag = Some(y);
        }
    }

    pub fn pointer_move(&mut self, layout: &Layout, x: f64, y: f64) {
        self.input.pointer = Some((x, y));
        if self.input.dragging {
            if let Some((sx, sy)) = self.input.drag_start {
                let dx = x - sx;
                let dy = y - sy;
                if !self.input.scrolling && dy.abs() > 14.0 && dy.abs() > dx.abs() * 1.5 {
                    self.input.scrolling = true;
                }
                if self.input.scrolling {
                    self.view_offset -= (y - self.input.drag_last_y) / layout.scale;
                    self.input.drag_last_y = y;
                    return;
                }
            }
        }
        self.input.drag_last_y = y;
        // The held piece follows the pointer wherever it is, including over the tray.
        if self.held.is_some() && !self.input.scrolling {
            self.held_x = self.to_x(layout, x);
        }
    }

    pub fn pointer_up(&mut self, layout: &Layout, x: f64, y: f64) {
        self.input.pointer = Some((x, y));
        self.input.slot_active = None;
        if let Some(start_y) = self.input.slot_drag.take() {
            if start_y - y > 24.0 && y < layout.stage_h {
                self.held_x = self.to_x(layout, x);
                self.drop_piece();
            }
        }
        if self.input.button_armed {
            self.input.button_armed = false;
            let hit = self
                .input
                .overlay_button
                .map(|r| r.contains(x, y))
                .unwrap_or(false);
            if hit && self.state == State::Over {
                self.init(true);
            }
            return;
        }
        if !self.input.dragging {
            return;
        }
        self.input.dragging = false;
        let was_scroll = self.input.scrolling;
        self.input.scrolling = false;
        self.input.drag_start = None;
        if was_scroll {
            return;
        }
        if self.held.is_some() {
            self.held_x = self.to_x(layout, x);
            self.drop_piece();
        } else if self.state == State::Play {
            self.set_hint("Pick a critter from the tray first", true);
        }
    }

    pub fn pointer_cancel(&mut self) {
        self.input.dragging = false;
        self.input.scrolling = false;
        self.input.drag_start = None;
        self.input.slot_drag = None;
        self.input.slot_active = None;
        self.input.button_armed = false;
    }

    pub fn pointer_leave(&mut self) {
        self.input.pointer = None;
    }

    /// `wheel`: `delta_css_px` is the browser's `deltaY` (positive scrolls down the tower).
    pub fn wheel(&mut self, layout: &Layout, delta_css_px: f64) {
        self.view_offset += delta_css_px / layout.scale;
    }

    pub fn key_down(&mut self, key: GameKey) {
        match key {
            GameKey::Digit(d) if (1..=3).contains(&d) => self.select_slot(d as usize - 1),
            GameKey::Digit(_) => {}
            GameKey::Left => self.held_x = Self::clamp_x(self.held_x - 12.0),
            GameKey::Right => self.held_x = Self::clamp_x(self.held_x + 12.0),
            GameKey::Down => self.view_offset += 60.0,
            GameKey::Up => self.view_offset -= 60.0,
            GameKey::Space | GameKey::Enter => match self.state {
                State::Over => self.init(true),
                State::Start => self.begin_play(),
                State::Play => self.drop_piece(),
            },
            GameKey::Escape => {
                if self.state == State::Start {
                    self.begin_play();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Game, Layout) {
        let layout = Layout::fit(560.0, 900.0);
        let game = Game::new(layout.stage_logical_h(), 0, false);
        (game, layout)
    }

    #[test]
    fn layout_matches_the_css_boxes() {
        let l = Layout::fit(800.0, 900.0);
        assert_eq!(l.app_w, 560.0);
        assert_eq!(l.stage_h, 900.0 - 137.0);
        assert!((l.scale - 560.0 / 480.0).abs() < 1e-12);
        let s0 = l.slot_rect(0);
        let s2 = l.slot_rect(2);
        assert_eq!(s0.x, 10.0);
        assert_eq!(s0.y, l.stage_h + 15.0);
        assert!((s2.x + s2.w - (560.0 - 10.0)).abs() < 1e-9);
        assert_eq!(l.mute_rect().x, 510.0);
        let narrow = Layout::fit(360.0, 640.0);
        assert_eq!(narrow.app_w, 360.0);
    }

    #[test]
    fn tap_on_stage_dismisses_intro_then_drags_and_drops() {
        let (mut g, l) = setup();
        g.pointer_down(&l, 100.0, 100.0);
        assert_eq!(g.state, State::Play);
        assert!(
            !g.input.dragging,
            "the dismissing tap does not start a drag"
        );
        g.pointer_up(&l, 100.0, 100.0);
        // tapping the stage with nothing held shakes the hint
        g.pointer_down(&l, 100.0, 100.0);
        g.pointer_up(&l, 100.0, 100.0);
        assert_eq!(g.hint, "Pick a critter from the tray first");
        // pick from the tray, move over the stage, release to drop
        let slot = l.slot_rect(1);
        g.pointer_down(&l, slot.x + 5.0, slot.y + 5.0);
        assert!(g.held.is_some());
        g.pointer_up(&l, slot.x + 5.0, slot.y + 5.0);
        assert!(g.held.is_some(), "a plain tap on the slot keeps holding");
        g.pointer_move(&l, 300.0, 200.0);
        assert!((g.held_x - 300.0 / l.scale).abs() < 1e-9);
        g.pointer_down(&l, 310.0, 200.0);
        g.pointer_up(&l, 310.0, 200.0);
        assert!(g.held.is_none());
        assert_eq!(g.placed.len(), 1);
    }

    #[test]
    fn vertical_drag_scrolls_instead_of_dropping() {
        let (mut g, l) = setup();
        g.begin_play();
        g.select_slot(0);
        g.pointer_down(&l, 200.0, 300.0);
        g.pointer_move(&l, 202.0, 340.0);
        assert!(g.input.scrolling);
        assert!((g.view_offset + 40.0 / l.scale).abs() < 1e-9);
        g.pointer_up(&l, 202.0, 340.0);
        assert!(g.held.is_some(), "a scroll release must not drop");
        g.wheel(&l, 100.0);
        assert!((g.view_offset - (-40.0 + 100.0) / l.scale).abs() < 1e-9);
    }

    #[test]
    fn drag_up_from_a_slot_drops_on_release() {
        let (mut g, l) = setup();
        let slot = l.slot_rect(0);
        g.pointer_down(&l, slot.x + 20.0, slot.y + 60.0);
        g.pointer_move(&l, slot.x + 20.0, slot.y - 100.0);
        g.pointer_up(&l, slot.x + 20.0, slot.y - 100.0);
        assert!(g.held.is_none());
        assert_eq!(g.placed.len(), 1);
    }

    #[test]
    fn mute_button_toggles_and_swallows_the_press() {
        let (mut g, l) = setup();
        let m = l.mute_rect();
        g.pointer_down(&l, m.x + 20.0, m.y + 20.0);
        assert!(g.muted);
        assert_eq!(g.state, State::Start, "the press never reached the stage");
        assert!(g.settings_dirty);
        g.pointer_down(&l, m.x + 20.0, m.y + 20.0);
        assert!(!g.muted);
        assert_eq!(g.take_sfx(), vec![Sfx::Pick]);
    }

    #[test]
    fn fullscreen_button_only_on_touch_profiles() {
        let (mut g, l) = setup();
        let r = l.fullscreen_rect();
        assert_eq!(r.x, l.mute_rect().x - 50.0);
        // desktop: the spot is plain stage, so the tap dismisses the intro
        agg_gui::input_profile::set_input_profile(agg_gui::input_profile::InputProfile::Desktop);
        g.pointer_down(&l, r.x + 20.0, r.y + 20.0);
        assert_eq!(g.state, State::Play);
        // mobile: the tap asks the shell for full screen and goes no further
        let (mut m, l) = setup();
        agg_gui::input_profile::set_input_profile(
            agg_gui::input_profile::InputProfile::MobileAndroid,
        );
        let _ = agg_gui::fullscreen::take_request();
        m.pointer_down(&l, r.x + 20.0, r.y + 20.0);
        assert_eq!(m.state, State::Start);
        assert!(agg_gui::fullscreen::take_request());
        agg_gui::input_profile::set_input_profile(agg_gui::input_profile::InputProfile::Desktop);
    }

    #[test]
    fn play_again_button_needs_press_and_release_inside() {
        let (mut g, l) = setup();
        g.begin_play();
        g.game_over(None);
        g.input.overlay_button = Some(Rect {
            x: 200.0,
            y: 400.0,
            w: 120.0,
            h: 40.0,
        });
        g.pointer_down(&l, 210.0, 410.0);
        g.pointer_up(&l, 10.0, 10.0);
        assert_eq!(g.state, State::Over);
        g.pointer_down(&l, 210.0, 410.0);
        g.pointer_up(&l, 250.0, 420.0);
        assert_eq!(g.state, State::Play);
    }

    #[test]
    fn after_game_over_the_card_can_be_put_away_and_the_tower_scrolled() {
        let (mut g, l) = setup();
        g.stress_tower(30, false);
        g.step(16.0);
        g.end_round(2, crate::piece::FallReason::WentOverTheSide);
        g.input.overlay_button = Some(Rect {
            x: 200.0,
            y: 400.0,
            w: 120.0,
            h: 40.0,
        });
        // first tap off the button dismisses the card
        g.pointer_down(&l, 50.0, 50.0);
        g.pointer_up(&l, 50.0, 50.0);
        assert!(g.overlay.peek);
        assert_eq!(g.state, State::Over);
        assert!(g.hint.contains("went over the side"), "{}", g.hint);
        // then a vertical drag scrolls
        let before = g.view_offset;
        g.pointer_down(&l, 100.0, 300.0);
        g.pointer_move(&l, 100.0, 240.0);
        g.pointer_up(&l, 100.0, 240.0);
        assert!((g.view_offset - (before + 60.0 / l.scale)).abs() < 1e-9);
        assert_eq!(g.state, State::Over);
        // the button still restarts
        g.pointer_down(&l, 210.0, 410.0);
        g.pointer_up(&l, 210.0, 410.0);
        assert_eq!(g.state, State::Play);
        assert!(!g.overlay.peek);
    }

    #[test]
    fn keyboard_table() {
        let (mut g, _l) = setup();
        g.key_down(GameKey::Escape);
        assert_eq!(g.state, State::Play);
        g.key_down(GameKey::Digit(2));
        assert!(g.held.is_some());
        let x = g.held_x;
        g.key_down(GameKey::Right);
        assert_eq!(g.held_x, x + 12.0);
        g.key_down(GameKey::Down);
        assert_eq!(g.view_offset, 60.0);
        g.key_down(GameKey::Space);
        assert!(g.held.is_none());
        g.game_over(None);
        g.key_down(GameKey::Enter);
        assert_eq!(g.state, State::Play);
        assert!(g.placed.is_empty());
    }
}
