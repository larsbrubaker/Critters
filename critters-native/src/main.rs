//! # Native Shell for Critter Stack
//!
//! Thinnest possible desktop shim: the winit window, event loop, wgpu
//! surface, input forwarding and frame painting are all `agg-gui-shell`'s.
//! This crate only names the window, opens the audio device, picks the
//! settings file and hands over the shared app built by `critters-core`.

mod audio;
mod settings;

use std::rc::Rc;

use agg_gui_shell::{NoHost, RedrawPolicy, ShellConfig, ShellError};
use critters_core::{build_app_with, DebugOptions, Fonts};

fn main() -> Result<(), ShellError> {
    let fonts = Fonts::load().expect("bundled fonts parse");
    let audio = Rc::new(audio::CpalAudio::new());
    let settings = settings::FileSettingsStore::into_shared();

    // The original's app column is 560 CSS px wide; a phone-like portrait
    // window shows it best. The game re-lays out at any size.
    // `CRITTERS_WINDOW=WxH` overrides the initial logical window size (debug).
    let (win_w, win_h) = std::env::var("CRITTERS_WINDOW")
        .ok()
        .and_then(|v| {
            let (w, h) = v.split_once('x')?;
            Some((w.parse().ok()?, h.parse().ok()?))
        })
        .unwrap_or((560.0, 900.0));
    let mut config = ShellConfig::new("Critter Stack")
        .with_logical_size(win_w, win_h)
        .with_min_logical_size(320.0, 480.0)
        // The scene animates every frame (idle critters, camera easing).
        .with_redraw_policy(RedrawPolicy::Continuous)
        .with_device_label("critters-native");
    // `CRITTERS_SCREENSHOT=path.png` captures a frame after the scene has
    // settled and exits — used for the README hero image and for eyeballing
    // the port against the original without a person at the keyboard.
    if let Ok(path) = std::env::var("CRITTERS_SCREENSHOT") {
        let frames = std::env::var("CRITTERS_SCREENSHOT_FRAMES")
            .ok()
            .and_then(|f| f.parse().ok())
            .unwrap_or(30);
        config = config.with_screenshot(path, frames);
    }

    // `CRITTERS_MOBILE=1` previews the touch layout (full-screen button).
    if std::env::var("CRITTERS_MOBILE").is_ok() {
        agg_gui::input_profile::set_input_profile(
            agg_gui::input_profile::InputProfile::MobileAndroid,
        );
    }
    let env = |k: &str| std::env::var(k).is_ok();
    let options = DebugOptions {
        autoplay: env("CRITTERS_AUTOPLAY"),
        gallery: env("CRITTERS_GALLERY"),
        stress: std::env::var("CRITTERS_STRESS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        stress_x: std::env::var("CRITTERS_STRESS_X")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        stress_physics: env("CRITTERS_STRESS_PHYSICS"),
        stats: env("CRITTERS_STATS"),
    };
    agg_gui_shell::run(config, move |_init| {
        Ok((build_app_with(fonts, audio, settings, options), NoHost))
    })
}
