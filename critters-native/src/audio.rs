//! Native audio output: a cpal output stream with a tiny software mixer.
//! `critters-core` renders every effect to PCM ([`AudioClip`]); this sink
//! schedules each clip at its `delay` and sums the active voices in the
//! device callback. Falls back to silence if no output device is available.

use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use critters_core::{AudioClip, AudioSink};

struct Voice {
    samples: Vec<f32>,
    /// Mixer clock (frames) at which sample 0 plays.
    start: u64,
}

struct Mixer {
    voices: Vec<Voice>,
    clock: u64,
    channels: usize,
}

impl Mixer {
    fn fill<T: cpal::SizedSample + cpal::FromSample<f32>>(&mut self, out: &mut [T]) {
        for frame in out.chunks_mut(self.channels.max(1)) {
            let mut sum = 0.0f32;
            for v in &self.voices {
                if self.clock >= v.start {
                    let i = (self.clock - v.start) as usize;
                    if let Some(s) = v.samples.get(i) {
                        sum += *s;
                    }
                }
            }
            let sample = T::from_sample(sum.clamp(-1.0, 1.0));
            for ch in frame.iter_mut() {
                *ch = sample;
            }
            self.clock += 1;
        }
        let clock = self.clock;
        self.voices
            .retain(|v| v.start + v.samples.len() as u64 > clock);
    }
}

pub struct CpalAudio {
    _stream: Option<cpal::Stream>,
    mixer: Arc<Mutex<Mixer>>,
    sample_rate: u32,
}

impl CpalAudio {
    /// Open the default output device. Returns a silent sink when audio is
    /// unavailable (headless CI, no sound card) rather than failing the app.
    pub fn new() -> Self {
        let mixer = Arc::new(Mutex::new(Mixer {
            voices: Vec::new(),
            clock: 0,
            channels: 2,
        }));
        let mut sample_rate = 48_000;
        let stream = (|| {
            let host = cpal::default_host();
            let device = host.default_output_device()?;
            let supported = device.default_output_config().ok()?;
            let config = supported.config();
            sample_rate = config.sample_rate;
            if let Ok(mut m) = mixer.lock() {
                m.channels = config.channels as usize;
            }
            let err = |e| eprintln!("critters audio: stream error: {e}");
            let m = Arc::clone(&mixer);
            let stream = match supported.sample_format() {
                cpal::SampleFormat::F32 => device.build_output_stream(
                    &config,
                    move |data: &mut [f32], _| {
                        if let Ok(mut m) = m.lock() {
                            m.fill(data);
                        }
                    },
                    err,
                    None,
                ),
                cpal::SampleFormat::I16 => device.build_output_stream(
                    &config,
                    move |data: &mut [i16], _| {
                        if let Ok(mut m) = m.lock() {
                            m.fill(data);
                        }
                    },
                    err,
                    None,
                ),
                cpal::SampleFormat::U16 => device.build_output_stream(
                    &config,
                    move |data: &mut [u16], _| {
                        if let Ok(mut m) = m.lock() {
                            m.fill(data);
                        }
                    },
                    err,
                    None,
                ),
                other => {
                    eprintln!("critters audio: unsupported sample format {other:?}");
                    return None;
                }
            }
            .ok()?;
            stream.play().ok()?;
            Some(stream)
        })();
        if stream.is_none() {
            eprintln!("critters audio: no output device, running silent");
        }
        Self {
            _stream: stream,
            mixer,
            sample_rate,
        }
    }
}

impl Default for CpalAudio {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioSink for CpalAudio {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn play(&self, clip: AudioClip) {
        let Ok(mut m) = self.mixer.lock() else {
            return;
        };
        let start = m.clock + (clip.delay.max(0.0) * clip.sample_rate as f32) as u64;
        m.voices.push(Voice {
            samples: clip.samples,
            start,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixer_sums_voices_and_drops_finished_ones() {
        let mut m = Mixer {
            voices: vec![
                Voice {
                    samples: vec![0.5; 4],
                    start: 0,
                },
                Voice {
                    samples: vec![0.25; 2],
                    start: 2,
                },
            ],
            clock: 0,
            channels: 2,
        };
        let mut out = vec![0.0f32; 12];
        m.fill(&mut out);
        assert_eq!(&out[..4], &[0.5, 0.5, 0.5, 0.5]);
        assert_eq!(&out[4..8], &[0.75, 0.75, 0.75, 0.75]);
        assert_eq!(&out[8..], &[0.0; 4]);
        assert!(m.voices.is_empty());
        assert_eq!(m.clock, 6);
    }
}
