//! # Critters Core
//!
//! Target-agnostic game core for *Critter Stack*, a Rust port of the
//! JavaScript physics stacking game kept under `reference/`. Every visible
//! pixel renders through agg-gui's [`DrawCtx`] — the native and WASM shells
//! in sibling crates only own the OS window/canvas, the event loop, audio
//! output and settings storage.
//!
//! Module map (each mirrors a part of the original):
//!
//! - [`config`] — the constant tables at the top of `game.js`
//! - [`geometry`], [`physics`], [`piece`] — Matter.js bodies on box2d-rust
//! - [`game`], [`input`], [`wind`] — game flow, simulation step, pointer/keyboard, gusts
//! - [`render`] — `draw()` plus the HUD/tray/overlay that were DOM elements
//! - [`audio`], [`synth`] — the synthesized sound effects
//! - [`settings`], [`fonts`], [`widget`] — persistence, bundled fonts, the
//!   agg-gui widget that ties it together
//!
//! The crate is `wasm32`-clean: no `tokio`, no `winit`, no direct `wgpu`
//! calls.

pub mod audio;
pub mod autoplay;
pub mod config;
pub mod debug;
pub mod fonts;
pub mod game;
pub mod geometry;
pub mod input;
pub mod physics;
pub mod piece;
pub mod postmortem;
pub mod render;
pub mod rng;
pub mod scenery;
pub mod settings;
pub mod synth;
pub mod tray;
pub mod widget;
pub mod wind;

use std::rc::Rc;
use std::sync::Arc;

use agg_gui::draw_ctx::DrawCtx;
use agg_gui::App;

pub use audio::{AudioClip, AudioSink, NullAudio};
pub use debug::DebugOptions;
pub use fonts::Fonts;
pub use settings::{InMemorySettingsStore, Settings, SettingsStore};
pub use widget::GameWidget;

/// Build the shared app: one full-window [`GameWidget`]. Both shells call
/// this and forward platform input into the returned [`App`].
pub fn build_app(fonts: Fonts, audio: Rc<dyn AudioSink>, settings: Arc<dyn SettingsStore>) -> App {
    build_app_with(fonts, audio, settings, DebugOptions::default())
}

/// `build_app` with debug switches (see `debug.rs`).
pub fn build_app_with(
    fonts: Fonts,
    audio: Rc<dyn AudioSink>,
    settings: Arc<dyn SettingsStore>,
    options: DebugOptions,
) -> App {
    App::new(Box::new(
        GameWidget::new(fonts, audio, settings).with_debug(options),
    ))
}

/// Keep the `DrawCtx` import referenced for the crate docs above.
#[allow(dead_code)]
fn _draw_ctx_marker(_: &dyn DrawCtx) {}
