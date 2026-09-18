//! Audio primitives used by `audio.rs` to reproduce the original's Web Audio
//! nodes: an oscillator with exponential frequency ramp and the
//! `exponentialRampToValueAtTime` gain envelope (`tone`), a decaying white
//! noise buffer (`noise_burst`), and a biquad filter with the Web Audio
//! spec's lowpass (Q in dB) and bandpass coefficient formulas.

use std::f32::consts::TAU;

/// `OscillatorNode.type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wave {
    Sine,
    Triangle,
    Square,
    Sawtooth,
}

impl Wave {
    /// Sample the waveform at phase `p` in `[0, 1)`.
    fn sample(self, p: f32) -> f32 {
        match self {
            Wave::Sine => (p * TAU).sin(),
            Wave::Triangle => {
                // 0 at p=0, 1 at p=0.25, 0 at p=0.5, -1 at p=0.75
                let t = (p + 0.25).fract();
                1.0 - 4.0 * (t - 0.5).abs()
            }
            Wave::Square => {
                if p < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Wave::Sawtooth => 2.0 * p - 1.0,
        }
    }
}

/// `exponentialRampToValueAtTime` between two automation points.
fn exp_ramp(v0: f32, v1: f32, t0: f32, t1: f32, t: f32) -> f32 {
    if t <= t0 {
        return v0;
    }
    if t >= t1 {
        return v1;
    }
    v0 * (v1 / v0).powf((t - t0) / (t1 - t0))
}

/// One `tone(type, f0, f1, dur, gain)` note: the frequency ramps
/// exponentially from `f0` to `max(1, f1)` over `dur`, the gain rises from
/// 0.0001 to `gain` in 10 ms and decays back to 0.0001 at `dur`; the
/// oscillator stops 20 ms later.
pub fn tone(sample_rate: u32, wave: Wave, f0: f32, f1: f32, dur: f32, gain: f32) -> Vec<f32> {
    let sr = sample_rate as f32;
    let f1 = f1.max(1.0);
    let len = ((dur + 0.02) * sr).ceil() as usize;
    let mut out = Vec::with_capacity(len);
    let mut phase = 0.0f32;
    for i in 0..len {
        let t = i as f32 / sr;
        let f = exp_ramp(f0, f1, 0.0, dur, t);
        let g = if t < 0.01 {
            exp_ramp(0.0001, gain, 0.0, 0.01, t)
        } else {
            exp_ramp(gain, 0.0001, 0.01, dur, t)
        };
        out.push(wave.sample(phase) * g);
        phase = (phase + f / sr).fract();
    }
    out
}

/// White noise of `len` samples shaped by `envelope(i)`.
pub fn noise_burst(len: usize, envelope: impl Fn(usize) -> f32) -> Vec<f32> {
    let mut state: u32 = 0x1234_5678;
    (0..len)
        .map(|i| {
            // xorshift32 → uniform in [-1, 1)
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let r = (state >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0;
            r * envelope(i)
        })
        .collect()
}

/// Direct-form-I biquad with Web Audio's coefficient formulas.
#[derive(Clone, Debug)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    /// `BiquadFilterNode` lowpass; `q_db` is the node's `Q` (in dB for lowpass).
    pub fn lowpass(sample_rate: u32, frequency: f32, q_db: f32) -> Self {
        let mut f = Self::identity();
        f.set_lowpass(sample_rate, frequency, q_db);
        f
    }

    /// `BiquadFilterNode` bandpass; `q` is the ordinary quality factor.
    pub fn bandpass(sample_rate: u32, frequency: f32, q: f32) -> Self {
        let mut f = Self::identity();
        f.set_bandpass(sample_rate, frequency, q);
        f
    }

    fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    pub fn set_lowpass(&mut self, sample_rate: u32, frequency: f32, q_db: f32) {
        let w0 = TAU * frequency / sample_rate as f32;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * 10f32.powf(q_db / 20.0));
        let a0 = 1.0 + alpha;
        self.b0 = (1.0 - cos) / 2.0 / a0;
        self.b1 = (1.0 - cos) / a0;
        self.b2 = (1.0 - cos) / 2.0 / a0;
        self.a1 = -2.0 * cos / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    pub fn set_bandpass(&mut self, sample_rate: u32, frequency: f32, q: f32) {
        let w0 = TAU * frequency / sample_rate as f32;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let a0 = 1.0 + alpha;
        self.b0 = alpha / a0;
        self.b1 = 0.0;
        self.b2 = -alpha / a0;
        self.a1 = -2.0 * cos / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// Convenience for tests: RMS of a slice.
#[cfg(test)]
fn rms(s: &[f32]) -> f32 {
    (s.iter().map(|v| v * v).sum::<f32>() / s.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waveforms_span_minus_one_to_one() {
        for w in [Wave::Sine, Wave::Triangle, Wave::Square, Wave::Sawtooth] {
            let (mut lo, mut hi) = (1.0f32, -1.0f32);
            for i in 0..1000 {
                let v = w.sample(i as f32 / 1000.0);
                lo = lo.min(v);
                hi = hi.max(v);
            }
            assert!(lo <= -0.99 && hi >= 0.99, "{w:?}: {lo}..{hi}");
        }
        assert!((Wave::Triangle.sample(0.0)).abs() < 1e-6);
        assert!((Wave::Triangle.sample(0.25) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn tone_has_attack_decay_and_stop_tail() {
        let sr = 48_000;
        let t = tone(sr, Wave::Sine, 440.0, 440.0, 0.1, 0.5);
        assert_eq!(t.len(), (0.12 * sr as f32).ceil() as usize);
        assert!(t[0].abs() < 0.001, "starts near silent");
        let attack_end = (0.01 * sr as f32) as usize;
        let mid = (0.05 * sr as f32) as usize;
        assert!(rms(&t[attack_end..attack_end + 200]) > rms(&t[mid..mid + 200]));
        let tail = (0.1 * sr as f32) as usize;
        assert!(t[tail..].iter().all(|v| v.abs() < 0.001));
    }

    #[test]
    fn frequency_ramp_reaches_the_target() {
        // count zero crossings in the last 10 ms of a 1 s glide 100 → 800 Hz
        let sr = 48_000;
        let t = tone(sr, Wave::Sine, 100.0, 800.0, 1.0, 1.0);
        let window = &t[(0.98 * sr as f32) as usize..(0.99 * sr as f32) as usize];
        let crossings = window
            .windows(2)
            .filter(|w| (w[0] < 0.0) != (w[1] < 0.0))
            .count();
        // ~800 Hz → 16 crossings per 10 ms (the gain is tiny here but the sign is not)
        assert!((14..=18).contains(&crossings), "crossings {crossings}");
    }

    #[test]
    fn lowpass_attenuates_high_frequencies() {
        let sr = 48_000;
        let mut f = Biquad::lowpass(sr, 500.0, 1.0);
        let mut low = Vec::new();
        let mut high = Vec::new();
        for i in 0..4800 {
            let t = i as f32 / sr as f32;
            low.push(f.process((TAU * 100.0 * t).sin()));
        }
        let mut f = Biquad::lowpass(sr, 500.0, 1.0);
        for i in 0..4800 {
            let t = i as f32 / sr as f32;
            high.push(f.process((TAU * 8000.0 * t).sin()));
        }
        assert!(rms(&low[2400..]) > 0.6);
        assert!(rms(&high[2400..]) < 0.05);
    }

    #[test]
    fn bandpass_passes_its_centre() {
        let sr = 48_000;
        let run = |hz: f32| {
            let mut f = Biquad::bandpass(sr, 600.0, 1.2);
            let out: Vec<f32> = (0..9600)
                .map(|i| f.process((TAU * hz * i as f32 / sr as f32).sin()))
                .collect();
            rms(&out[4800..])
        };
        let centre = run(600.0);
        assert!(centre > run(60.0) * 3.0);
        assert!(centre > run(6000.0) * 3.0);
    }

    #[test]
    fn noise_is_deterministic_and_shaped() {
        let a = noise_burst(100, |i| if i < 50 { 1.0 } else { 0.0 });
        let b = noise_burst(100, |i| if i < 50 { 1.0 } else { 0.0 });
        assert_eq!(a, b);
        assert!(a[..50].iter().any(|v| v.abs() > 0.1));
        assert!(a[50..].iter().all(|v| *v == 0.0));
    }
}
