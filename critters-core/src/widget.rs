//! `GameWidget` — the one agg-gui widget that is the whole app. It owns the
//! [`Game`], drives the frame loop from `paint` (the original's
//! `requestAnimationFrame` callback: clamp the elapsed time to 50 ms, step,
//! draw), converts agg-gui events into the app-CSS-pixel space `input.rs`
//! expects, plays queued sound effects through the platform [`AudioSink`]
//! and persists settings through the [`SettingsStore`].

use std::rc::Rc;
use std::sync::Arc;

use agg_gui::draw_ctx::DrawCtx;
use agg_gui::event::{Event, EventResult, Key};
use agg_gui::geometry::{Rect, Size};
use agg_gui::widget::Widget;
use web_time::Instant;

use crate::audio::{render_sfx, AudioSink};
use crate::fonts::Fonts;
use crate::game::Game;
use crate::input::{GameKey, Layout};
use crate::settings::{Settings, SettingsStore};

pub struct GameWidget {
    bounds: Rect,
    children: Vec<Box<dyn Widget>>,
    game: Game,
    fonts: Fonts,
    audio: Rc<dyn AudioSink>,
    settings: Arc<dyn SettingsStore>,
    layout: Layout,
    last_paint: Option<Instant>,
    unlocked: bool,
    autoplay: bool,
    gallery: bool,
}

impl GameWidget {
    pub fn new(fonts: Fonts, audio: Rc<dyn AudioSink>, settings: Arc<dyn SettingsStore>) -> Self {
        let saved = settings.load();
        let layout = Layout::fit(560.0, 900.0);
        let game = Game::new(layout.stage_logical_h(), saved.best_score, saved.muted);
        Self {
            bounds: Rect::default(),
            children: Vec::new(),
            game,
            fonts,
            audio,
            settings,
            layout,
            last_paint: None,
            unlocked: false,
            autoplay: false,
            gallery: false,
        }
    }

    /// Show the shape gallery instead of the game (debug).
    pub fn with_gallery(mut self) -> Self {
        self.gallery = true;
        self
    }

    /// Enable the autoplay debug driver.
    pub fn with_autoplay(mut self) -> Self {
        self.autoplay = true;
        self
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    pub fn game_mut(&mut self) -> &mut Game {
        &mut self.game
    }

    /// Widget-local (y up) → app CSS px (y down, relative to the app column).
    fn to_css(&self, pos: agg_gui::geometry::Point) -> (f64, f64) {
        let app_x = ((self.bounds.width - self.layout.app_w) / 2.0).max(0.0);
        (pos.x - app_x, self.bounds.height - pos.y)
    }

    fn unlock_audio(&mut self) {
        if !self.unlocked {
            self.unlocked = true;
            self.audio.unlock();
        }
    }

    fn flush_side_effects(&mut self) {
        let muted = self.game.muted;
        for sfx in self.game.take_sfx() {
            if muted {
                continue;
            }
            for clip in render_sfx(sfx, self.audio.sample_rate()) {
                self.audio.play(clip);
            }
        }
        if self.game.settings_dirty {
            self.game.settings_dirty = false;
            self.settings.save(&Settings {
                best_score: self.game.best_score,
                muted: self.game.muted,
            });
        }
    }

    fn map_key(key: &Key) -> Option<GameKey> {
        Some(match key {
            Key::Char(c) if c.is_ascii_digit() => GameKey::Digit(*c as u8 - b'0'),
            Key::Char(' ') => GameKey::Space,
            Key::ArrowLeft => GameKey::Left,
            Key::ArrowRight => GameKey::Right,
            Key::ArrowUp => GameKey::Up,
            Key::ArrowDown => GameKey::Down,
            Key::Enter => GameKey::Enter,
            Key::Escape => GameKey::Escape,
            _ => return None,
        })
    }
}

impl Widget for GameWidget {
    fn type_name(&self) -> &'static str {
        "GameWidget"
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }

    fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut Vec<Box<dyn Widget>> {
        &mut self.children
    }

    fn layout(&mut self, available: Size) -> Size {
        let layout = Layout::fit(available.width, available.height);
        if layout != self.layout {
            self.layout = layout;
            self.game.resize(layout.stage_logical_h());
        }
        available
    }

    fn paint(&mut self, ctx: &mut dyn DrawCtx) {
        let now = Instant::now();
        let dt = match self.last_paint {
            Some(last) => (now - last).as_secs_f64() * 1000.0,
            None => 1000.0 / 60.0,
        }
        .min(50.0);
        self.last_paint = Some(now);
        self.game.step(dt);
        if self.autoplay {
            self.game.autoplay_step();
        }
        self.flush_side_effects();
        let (w, h) = (self.bounds.width, self.bounds.height);
        let layout = self.layout;
        if self.gallery {
            crate::render::gallery::draw_gallery(ctx, &self.fonts, &self.game, w, h);
            return;
        }
        crate::render::draw(ctx, &self.fonts, &mut self.game, &layout, w, h);
    }

    fn needs_draw(&self) -> bool {
        true
    }

    fn on_event(&mut self, event: &Event) -> EventResult {
        let layout = self.layout;
        match event {
            Event::MouseDown { pos, .. } => {
                self.unlock_audio();
                let (x, y) = self.to_css(*pos);
                self.game.pointer_down(&layout, x, y);
            }
            Event::MouseMove { pos } => {
                let (x, y) = self.to_css(*pos);
                self.game.pointer_move(&layout, x, y);
            }
            Event::MouseUp { pos, .. } => {
                let (x, y) = self.to_css(*pos);
                self.game.pointer_up(&layout, x, y);
            }
            Event::MouseWheel { delta_y, .. } => {
                // agg-gui: positive = content above; browser deltaY: positive = scroll down
                self.game.wheel(&layout, -delta_y * 100.0);
            }
            Event::KeyDown { key, .. } => {
                self.unlock_audio();
                if let Some(k) = Self::map_key(key) {
                    self.game.key_down(k);
                } else {
                    return EventResult::Ignored;
                }
            }
            Event::FocusLost => self.game.pointer_cancel(),
            _ => return EventResult::Ignored,
        }
        self.flush_side_effects();
        EventResult::Consumed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::NullAudio;
    use crate::settings::InMemorySettingsStore;
    use agg_gui::event::{Modifiers, MouseButton};
    use agg_gui::geometry::Point;

    fn widget() -> GameWidget {
        let store = Arc::new(InMemorySettingsStore::default());
        store.save(&Settings {
            best_score: 4321,
            muted: true,
        });
        let mut w = GameWidget::new(Fonts::load().unwrap(), Rc::new(NullAudio), store);
        w.set_bounds(Rect::new(0.0, 0.0, 800.0, 900.0));
        w.layout(Size::new(800.0, 900.0));
        w
    }

    #[test]
    fn loads_persisted_settings_and_lays_out_the_app_column() {
        let w = widget();
        assert_eq!(w.game().best_score, 4321);
        assert!(w.game().muted);
        assert_eq!(w.layout.app_w, 560.0);
        assert_eq!(w.layout.stage_h, 900.0 - 137.0);
        assert!((w.game().h - (900.0 - 137.0) / (560.0 / 480.0)).abs() < 1e-9);
    }

    #[test]
    fn events_are_mapped_into_app_css_space() {
        let mut w = widget();
        // widget is 800 wide, app column 560 → app_x = 120; y up → flip
        let down = Event::MouseDown {
            pos: Point {
                x: 120.0 + 100.0,
                y: 900.0 - 100.0,
            },
            button: MouseButton::Left,
            modifiers: Modifiers::default(),
        };
        assert_eq!(w.on_event(&down), EventResult::Consumed);
        assert_eq!(w.game().state, crate::game::State::Play);
        // press a tray slot: stage_h + 15 + 10 from the top
        let slot_y = 900.0 - (763.0 + 25.0);
        let down = Event::MouseDown {
            pos: Point {
                x: 120.0 + 30.0,
                y: slot_y,
            },
            button: MouseButton::Left,
            modifiers: Modifiers::default(),
        };
        w.on_event(&down);
        assert!(w.game().held.is_some());
        let key = Event::KeyDown {
            key: Key::Char(' '),
            modifiers: Modifiers::default(),
        };
        w.on_event(&key);
        assert!(w.game().held.is_none());
        assert_eq!(w.game().placed.len(), 1);
    }

    #[test]
    fn best_score_is_saved_through_the_store() {
        let store: Arc<InMemorySettingsStore> = Arc::new(InMemorySettingsStore::default());
        let shared: Arc<dyn SettingsStore> = Arc::clone(&store) as Arc<dyn SettingsStore>;
        let mut w = GameWidget::new(Fonts::load().unwrap(), Rc::new(NullAudio), shared);
        w.game_mut().best_score = 99;
        w.game_mut().settings_dirty = true;
        w.flush_side_effects();
        assert_eq!(store.load().best_score, 99);
    }
}
