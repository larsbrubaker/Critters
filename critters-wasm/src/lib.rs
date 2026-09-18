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

use critters_core::{build_app, build_app_autoplay, Fonts};
use demo_wgpu::web_shell;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    web_shell::start(
        "critters-canvas",
        || {
            let fonts = Fonts::load().expect("bundled fonts parse");
            let audio = Rc::new(audio::WebAudio::new());
            let settings = Arc::new(settings::LocalStorageSettingsStore);
            let autoplay = web_sys::window()
                .and_then(|w| w.location().search().ok())
                .map(|s| s.contains("autoplay"))
                .unwrap_or(false);
            if autoplay {
                build_app_autoplay(fonts, audio, settings)
            } else {
                build_app(fonts, audio, settings)
            }
        },
        || {},
    );
}
