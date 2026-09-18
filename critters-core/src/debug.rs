//! Debug options shared by the shells — the counterparts of the original's
//! `?debug` hook: autoplay, the shape gallery, a stress tower for profiling
//! and per-frame timing statistics. None of it is reachable from normal play.

use crate::config::{ShapeKind, SHAPES, W};
use crate::game::Game;
use crate::piece::{Piece, Rest};

/// Build-time debug switches (`CRITTERS_*` env vars natively, query
/// parameters on the web).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DebugOptions {
    /// The game plays itself (`autoplay.rs`).
    pub autoplay: bool,
    /// Show every shape at 2.6× instead of the game (`render/gallery.rs`).
    pub gallery: bool,
    /// Start with a tower of this many pieces already stacked.
    pub stress: usize,
    /// Shift the (frozen) stress tower sideways by this many units, to look
    /// at the sideways-building zoom.
    pub stress_x: i32,
    /// Give the stress tower live physics bodies instead of frozen ones.
    pub stress_physics: bool,
    /// Print physics/draw timings to the console once a second.
    pub stats: bool,
}

/// Rolling per-second timing accumulator for `DebugOptions::stats`.
#[derive(Default)]
pub struct FrameStats {
    frames: u32,
    step_ms: f64,
    draw_ms: f64,
    max_draw_ms: f64,
    since: f64,
}

impl FrameStats {
    /// Record one frame; returns a summary line every 60 frames.
    pub fn record(&mut self, dt_ms: f64, step_ms: f64, draw_ms: f64, info: &str) -> Option<String> {
        self.frames += 1;
        self.step_ms += step_ms;
        self.draw_ms += draw_ms;
        self.max_draw_ms = self.max_draw_ms.max(draw_ms);
        self.since += dt_ms;
        if self.frames < 60 {
            return None;
        }
        let n = self.frames as f64;
        let line = format!(
            "critters stats: {info} | frame {:.1} ms | step {:.2} ms | draw {:.2} ms (max {:.2}) || {}",
            self.since / n,
            self.step_ms / n,
            self.draw_ms / n,
            self.max_draw_ms,
            take_sections(n)
        );
        *self = Self::default();
        Some(line)
    }
}

thread_local! {
    static SECTIONS: std::cell::RefCell<Vec<(&'static str, f64)>> =
        const { std::cell::RefCell::new(Vec::new()) };
    static SECTIONS_ON: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Turn the per-section render timers on (they cost two clock reads each).
pub fn enable_sections(on: bool) {
    SECTIONS_ON.with(|c| c.set(on));
}

/// Times the sections of `render::draw` when stats are enabled.
pub struct SectionTimer {
    last: Option<web_time::Instant>,
}

impl SectionTimer {
    pub fn start() -> Self {
        let on = SECTIONS_ON.with(|c| c.get());
        Self {
            last: on.then(web_time::Instant::now),
        }
    }

    /// Attribute the time since the previous mark to `name`.
    pub fn mark(&mut self, name: &'static str) {
        let Some(last) = self.last else {
            return;
        };
        let now = web_time::Instant::now();
        let ms = (now - last).as_secs_f64() * 1000.0;
        self.last = Some(now);
        SECTIONS.with(|s| {
            let mut s = s.borrow_mut();
            match s.iter_mut().find(|(n, _)| *n == name) {
                Some(entry) => entry.1 += ms,
                None => s.push((name, ms)),
            }
        });
    }
}

/// Take the accumulated section times as `name ms/frame` pairs.
pub fn take_sections(frames: f64) -> String {
    SECTIONS.with(|s| {
        let mut s = s.borrow_mut();
        let out = s
            .iter()
            .map(|(n, ms)| format!("{n} {:.2}", ms / frames))
            .collect::<Vec<_>>()
            .join(" | ");
        s.clear();
        out
    })
}

thread_local! {
    static REPORT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Web only: also POST every debug line to `/__stats` on the page's origin
/// (`?report`), so a phone on the LAN can be profiled from the machine that
/// serves it (`demo/debug-server.ts` appends the lines to a log).
pub fn set_report(on: bool) {
    REPORT.with(|c| c.set(on));
}

/// Whether the section timers are running.
pub fn sections_on() -> bool {
    SECTIONS_ON.with(|c| c.get())
}

/// Add time to a named section from code that is not on the main
/// `SectionTimer` spine (e.g. work inside the per-piece loop).
pub fn add_section(name: &'static str, ms: f64) {
    SECTIONS.with(|s| {
        let mut s = s.borrow_mut();
        match s.iter_mut().find(|(n, _)| *n == name) {
            Some(entry) => entry.1 += ms,
            None => s.push((name, ms)),
        }
    });
}

/// Print a debug line: stderr natively, the console on the web.
pub fn log(line: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::console::log_1(&line.into());
        if REPORT.with(|c| c.get()) {
            if let Some(window) = web_sys::window() {
                let _ = window
                    .navigator()
                    .send_beacon_with_opt_str("/__stats", Some(line));
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        eprintln!("{line}");
    }
}

impl Game {
    /// Stack `n` flat pieces on the stump for profiling. Frozen pieces skip
    /// the physics world entirely (render cost only); with `physics` they
    /// are live dynamic bodies resting on each other.
    pub fn stress_tower(&mut self, n: usize, physics: bool) {
        self.stress_tower_at(n, physics, 0.0);
    }

    /// `stress_tower` with the column centred `dx` units off the stump centre.
    pub fn stress_tower_at(&mut self, n: usize, physics: bool, dx: f64) {
        self.begin_play();
        let flat: Vec<_> = SHAPES
            .iter()
            .filter(|s| !s.big && matches!(s.kind, ShapeKind::Rect { .. } | ShapeKind::Trap { .. }))
            .copied()
            .collect();
        let mut top = 0.0;
        for i in 0..n {
            let spec = flat[i % flat.len()];
            // place by the real bounds: a trapezoid's centroid is not mid-height
            let mut piece = Piece::new(spec, W / 2.0 + dx, 0.0, self.rng.random() * 1000.0);
            let shift = top - piece.bounds.max.y;
            piece.set_position(W / 2.0 + dx, shift);
            piece.has_landed = true;
            piece.land_at = 0.0;
            if physics {
                let index = self.placed.len();
                piece.add_to_world(&mut self.physics, index);
            } else {
                piece.is_static = true;
                piece.is_sleeping = true;
                piece.locked = true;
                piece.rest = Some(Rest {
                    x: piece.x,
                    y: piece.y,
                    angle: 0.0,
                    top: piece.bounds.min.y,
                });
                piece.highest_rest_y = Some(piece.y);
            }
            top = piece.bounds.min.y;
            self.placed.push(piece);
            self.blocks += 1;
        }
        self.tower_top = top;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stress_tower_stacks_pieces_edge_to_edge() {
        let mut g = Game::new(720.0, 0, false);
        g.stress_tower(30, false);
        assert_eq!(g.placed.len(), 30);
        for pair in g.placed.windows(2) {
            assert!((pair[0].bounds.min.y - pair[1].bounds.max.y).abs() < 1e-9);
        }
        g.step(16.0);
        assert!(g.tower_top < -1000.0, "30 pieces are taller than 10 m");
        assert_eq!(g.state, crate::game::State::Play);
        let mut p = Game::new(720.0, 0, false);
        p.stress_tower(10, true);
        for _ in 0..120 {
            p.step(1000.0 / 60.0);
        }
        assert_eq!(
            p.state,
            crate::game::State::Play,
            "a live stress tower must stand"
        );
    }

    #[test]
    fn stats_summarise_every_sixty_frames() {
        let mut s = FrameStats::default();
        for i in 0..59 {
            assert!(s.record(16.0, 1.0, 2.0, "").is_none());
            let _ = i;
        }
        let line = s.record(16.0, 1.0, 2.0, "5 pieces").unwrap();
        assert!(line.contains("5 pieces"), "{line}");
        assert!(line.contains("step 1.00 ms"), "{line}");
    }
}
