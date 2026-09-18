//! Synthesized sound effects (no asset files), mirroring the Web Audio graphs
//! in the "Audio" section of `reference/game.js`. Each [`Sfx`] is rendered to
//! PCM on the core side by [`render_sfx`] — oscillators with exponential
//! frequency/gain ramps, filtered noise bursts — and the platform shells only
//! play the resulting buffers through an [`AudioSink`]. `synth.rs` holds the
//! oscillator, envelope and biquad primitives.

use crate::game::Sfx;
use crate::synth::{noise_burst, tone, Biquad, Wave};

/// One buffer to play: mono samples at `sample_rate`, starting `delay`
/// seconds after the request (the original's `delay` argument to `tone`).
#[derive(Clone, Debug, PartialEq)]
pub struct AudioClip {
    pub sample_rate: u32,
    pub samples: Vec<f32>,
    pub delay: f32,
}

/// Platform audio output. Implemented with cpal on native and Web Audio in
/// the browser; `NullAudio` is used in tests.
pub trait AudioSink {
    /// Output sample rate, so effects are rendered without resampling.
    fn sample_rate(&self) -> u32;
    /// Queue a clip (mixed with anything already playing).
    fn play(&self, clip: AudioClip);
    /// First user gesture: browsers only start audio after one. No-op elsewhere.
    fn unlock(&self) {}
}

/// Silent sink.
#[derive(Default)]
pub struct NullAudio;

impl AudioSink for NullAudio {
    fn sample_rate(&self) -> u32 {
        48_000
    }
    fn play(&self, _clip: AudioClip) {}
}

/// Duration of the wind gust sound (`WIND.duration / 1000`).
pub const WIND_SECONDS: f32 = 2.2;

/// Render an effect to the clips that make it up.
pub fn render_sfx(sfx: Sfx, sample_rate: u32) -> Vec<AudioClip> {
    let sr = sample_rate;
    let clip = |samples: Vec<f32>, delay: f32| AudioClip {
        sample_rate: sr,
        samples,
        delay,
    };
    match sfx {
        Sfx::Pick => vec![clip(tone(sr, Wave::Sine, 620.0, 920.0, 0.09, 0.15), 0.0)],
        Sfx::Swap => vec![clip(tone(sr, Wave::Sine, 920.0, 620.0, 0.09, 0.15), 0.0)],
        Sfx::Drop => vec![clip(
            tone(sr, Wave::Triangle, 420.0, 180.0, 0.16, 0.12),
            0.0,
        )],
        Sfx::Land(strength) => thump(sr, strength as f32),
        Sfx::Tier => [523.0, 659.0, 784.0, 1047.0]
            .iter()
            .enumerate()
            .map(|(i, &f)| clip(tone(sr, Wave::Sine, f, f, 0.28, 0.18), i as f32 * 0.11))
            .collect(),
        Sfx::Best => [784.0, 988.0, 1175.0]
            .iter()
            .enumerate()
            .map(|(i, &f)| clip(tone(sr, Wave::Triangle, f, f, 0.2, 0.12), i as f32 * 0.09))
            .collect(),
        Sfx::Wait => vec![clip(tone(sr, Wave::Square, 200.0, 160.0, 0.08, 0.05), 0.0)],
        Sfx::Over => [330.0, 262.0, 208.0, 147.0]
            .iter()
            .enumerate()
            .map(|(i, &f)| {
                clip(
                    tone(sr, Wave::Sawtooth, f, f * 0.94, 0.32, 0.12),
                    i as f32 * 0.22,
                )
            })
            .collect(),
        Sfx::Wind => vec![clip(wind(sr), 0.0)],
    }
}

/// `thump(strength)`: a filtered noise burst plus a low sine.
fn thump(sr: u32, strength: f32) -> Vec<AudioClip> {
    let k = strength.clamp(0.15, 1.0);
    let dur = 0.12 + 0.1 * k;
    let len = (sr as f32 * dur).ceil() as usize;
    // noise fading with (1 - i/len)^2, through a lowpass, at gain 0.5k
    let mut noise = noise_burst(len, |i| {
        let t = 1.0 - i as f32 / len as f32;
        t * t
    });
    let mut lp = Biquad::lowpass(sr, 300.0 + 500.0 * k, 1.0);
    for s in noise.iter_mut() {
        *s = lp.process(*s) * 0.5 * k;
    }
    vec![
        AudioClip {
            sample_rate: sr,
            samples: noise,
            delay: 0.0,
        },
        AudioClip {
            sample_rate: sr,
            samples: tone(sr, Wave::Sine, 110.0 + 40.0 * k, 60.0, dur, 0.35 * k),
            delay: 0.0,
        },
    ]
}

/// `sfx.wind()`: white noise through a sweeping bandpass with a swell envelope.
fn wind(sr: u32) -> Vec<f32> {
    let dur = WIND_SECONDS;
    let len = (sr as f32 * dur).ceil() as usize;
    let mut out = noise_burst(len, |_| 1.0);
    let half = dur / 2.0;
    let mut bp = Biquad::bandpass(sr, 250.0, 1.2);
    for (i, s) in out.iter_mut().enumerate() {
        let t = i as f32 / sr as f32;
        // frequency: linear 250 → 900 → 250
        let f = if t < half {
            250.0 + (900.0 - 250.0) * (t / half)
        } else {
            900.0 - (900.0 - 250.0) * ((t - half) / half)
        };
        bp.set_bandpass(sr, f, 1.2);
        // gain: exponential 0.0001 → 0.25 → 0.0001
        let g = if t < half {
            0.0001 * (0.25f32 / 0.0001).powf(t / half)
        } else {
            0.25 * (0.0001f32 / 0.25).powf((t - half) / half)
        };
        *s = bp.process(*s) * g;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peak(s: &[f32]) -> f32 {
        s.iter().fold(0.0f32, |m, v| m.max(v.abs()))
    }

    #[test]
    fn every_effect_renders_finite_audio_at_the_expected_gain() {
        let sr = 44_100;
        for sfx in [
            Sfx::Pick,
            Sfx::Swap,
            Sfx::Drop,
            Sfx::Land(0.5),
            Sfx::Tier,
            Sfx::Best,
            Sfx::Wait,
            Sfx::Over,
            Sfx::Wind,
        ] {
            let clips = render_sfx(sfx, sr);
            assert!(!clips.is_empty());
            for c in &clips {
                assert_eq!(c.sample_rate, sr);
                assert!(!c.samples.is_empty());
                assert!(c.samples.iter().all(|s| s.is_finite()));
                let p = peak(&c.samples);
                assert!(p > 0.01 && p <= 1.0, "{sfx:?}: peak {p}");
            }
        }
    }

    #[test]
    fn chords_are_staggered_like_the_original() {
        let tier = render_sfx(Sfx::Tier, 48_000);
        let delays: Vec<f32> = tier.iter().map(|c| c.delay).collect();
        assert_eq!(delays.len(), 4);
        assert!((delays[3] - 0.33).abs() < 1e-6);
        let over = render_sfx(Sfx::Over, 48_000);
        assert!((over[1].delay - 0.22).abs() < 1e-6);
    }

    #[test]
    fn thump_scales_with_impact_and_wind_lasts_the_gust() {
        let soft = render_sfx(Sfx::Land(0.0), 48_000);
        let hard = render_sfx(Sfx::Land(1.0), 48_000);
        assert!(peak(&hard[0].samples) > peak(&soft[0].samples));
        assert!(hard[0].samples.len() > soft[0].samples.len());
        let wind = render_sfx(Sfx::Wind, 48_000);
        assert_eq!(wind[0].samples.len(), (48_000.0 * WIND_SECONDS) as usize);
        // swells toward the middle and dies away
        let s = &wind[0].samples;
        let mid = peak(&s[s.len() / 2 - 2000..s.len() / 2 + 2000]);
        let end = peak(&s[s.len() - 2000..]);
        assert!(mid > end * 10.0, "mid {mid} end {end}");
    }
}
