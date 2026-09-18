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
use critters_core::{build_app, build_app_autoplay, build_app_gallery, Fonts};

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

    let autoplay = std::env::var("CRITTERS_AUTOPLAY").is_ok();
    let gallery = std::env::var("CRITTERS_GALLERY").is_ok();
    agg_gui_shell::run(config, move |_init| {
        let app = if gallery {
            build_app_gallery(fonts, audio, settings)
        } else if autoplay {
            build_app_autoplay(fonts, audio, settings)
        } else {
            build_app(fonts, audio, settings)
        };
        Ok((app, NoHost))
    })
}
