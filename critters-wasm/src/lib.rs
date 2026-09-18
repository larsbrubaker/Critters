//! # WebAssembly Shell for Critter Stack
//!
//! Thinnest possible browser shim: canvas sizing, the wgpu/WebGL2 surface,
//! the rAF loop, DOM pointer / wheel / keyboard listeners and DPR tracking
//! all live in `demo_wgpu::web_shell`. This crate boots the shared app from
//! `critters-core` with a Web Audio sink and a `localStorage` settings store.

#![cfg(target_arch = "wasm32")]

mod audio;
mod settings;

use std::rc::Rc;
use std::sync::Arc;

use critters_core::{build_app_with, DebugOptions, Fonts};
use demo_wgpu::web_shell;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    web_shell::start(
        "critters-canvas",
        || {
            // The web shell applies agg-gui's 1.7× touch "UX scale" on phones
            // for widget-based UIs. This game lays itself out in CSS pixels
            // exactly like the original page, so keep 1:1.
            agg_gui::ux_scale::set_ux_scale(1.0);
            let fonts = Fonts::load().expect("bundled fonts parse");
            let audio = Rc::new(audio::WebAudio::new());
            let settings = Arc::new(settings::LocalStorageSettingsStore);
            // `?autoplay`, `?stats`, `?stress=N`, `?physics` mirror the native env vars.
            let query = web_sys::window()
                .and_then(|w| w.location().search().ok())
                .unwrap_or_default();
            let has = |k: &str| query.contains(k);
            let stress = query
                .split(['?', '&'])
                .find_map(|kv| kv.strip_prefix("stress="))
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let options = DebugOptions {
                autoplay: has("autoplay"),
                gallery: has("gallery"),
                stress,
                stress_physics: has("physics"),
                stats: has("stats") || has("report"),
            };
            critters_core::debug::set_report(has("report"));
            build_app_with(fonts, audio, settings, options)
        },
        || {},
    );
}
