//! Web Audio output. The core renders effects to PCM; this sink wraps each
//! clip in an `AudioBuffer` and schedules an `AudioBufferSourceNode` at
//! `currentTime + delay`, matching the original's node graphs' timing. The
//! `AudioContext` is created on the first user gesture (`unlock`) because
//! browsers refuse to start audio before one.

use std::cell::RefCell;

use critters_core::{AudioClip, AudioSink};
use web_sys::{AudioContext, AudioContextState};

pub struct WebAudio {
    ctx: RefCell<Option<AudioContext>>,
}

impl WebAudio {
    pub fn new() -> Self {
        Self {
            ctx: RefCell::new(None),
        }
    }

    /// `audio()` in the original: create lazily, resume if suspended.
    fn context(&self) -> Option<AudioContext> {
        let mut slot = self.ctx.borrow_mut();
        if slot.is_none() {
            match AudioContext::new() {
                Ok(ctx) => *slot = Some(ctx),
                Err(err) => {
                    web_sys::console::warn_1(&err);
                    return None;
                }
            }
        }
        let ctx = slot.as_ref()?;
        if ctx.state() == AudioContextState::Suspended {
            let _ = ctx.resume();
        }
        Some(ctx.clone())
    }
}

impl Default for WebAudio {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioSink for WebAudio {
    fn sample_rate(&self) -> u32 {
        self.ctx
            .borrow()
            .as_ref()
            .map(|c| c.sample_rate() as u32)
            .unwrap_or(48_000)
    }

    fn play(&self, clip: AudioClip) {
        let Some(ctx) = self.context() else {
            return;
        };
        let Ok(buffer) = ctx.create_buffer(1, clip.samples.len() as u32, clip.sample_rate as f32)
        else {
            return;
        };
        let mut samples = clip.samples;
        if buffer.copy_to_channel(&mut samples, 0).is_err() {
            return;
        }
        let Ok(source) = ctx.create_buffer_source() else {
            return;
        };
        source.set_buffer(Some(&buffer));
        if source.connect_with_audio_node(&ctx.destination()).is_err() {
            return;
        }
        let _ = source.start_with_when(ctx.current_time() + clip.delay as f64);
    }

    fn unlock(&self) {
        let _ = self.context();
    }
}
