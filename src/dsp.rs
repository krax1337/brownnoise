//! Noise synthesis. Everything here runs on the real-time audio thread, so it
//! never allocates, locks, or blocks; the UI talks to it only through atomics.

use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};

use cpal::{FromSample, Sample};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Color {
    White,
    Pink,
    Brown,
}

impl Color {
    pub const ALL: [Color; 3] = [Color::White, Color::Pink, Color::Brown];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn next(self) -> Color {
        Color::ALL[(self.index() + 1) % Color::ALL.len()]
    }

    pub fn name(self) -> &'static str {
        match self {
            Color::White => "white",
            Color::Pink => "pink",
            Color::Brown => "brown",
        }
    }
}

/// Tone 0..=100 → high-shelf gain. 50 is flat (pure color); each end is ±18 dB.
pub fn tone_gain(tone: u8) -> f32 {
    8f32.powf((tone as f32 - 50.0) / 50.0)
}

/// Shelf crossover: below this the sound is "body", above it "air".
const SHELF_HZ: f32 = 350.0;

fn one_pole_coef(hz: f32, sample_rate: f32) -> f32 {
    1.0 - (-2.0 * std::f32::consts::PI * hz / sample_rate).exp()
}

fn smoothing_coef(seconds: f32, sample_rate: f32) -> f32 {
    1.0 - (-1.0 / (seconds * sample_rate)).exp()
}

/// xorshift32: fast, allocation-free, plenty random for noise.
struct Rng(u32);

impl Rng {
    #[inline(always)]
    fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x as i32 as f32 / i32::MAX as f32
    }
}

/// One channel's worth of generators. Produces all three colors every sample
/// so color changes can crossfade instead of jumping.
struct Voice {
    rng: Rng,
    pink: [f32; 7],
    brown: f32,
}

impl Voice {
    fn new(seed: u32) -> Self {
        Voice {
            rng: Rng(seed.max(1)),
            pink: [0.0; 7],
            brown: 0.0,
        }
    }

    /// Raw (un-normalized) white, pink, brown samples.
    #[inline(always)]
    fn next(&mut self) -> [f32; 3] {
        let w = self.rng.next();
        // Paul Kellet's refined pink filter (-3 dB/octave).
        let b = &mut self.pink;
        b[0] = 0.99886 * b[0] + w * 0.0555179;
        b[1] = 0.99332 * b[1] + w * 0.0750759;
        b[2] = 0.96900 * b[2] + w * 0.153852;
        b[3] = 0.86650 * b[3] + w * 0.3104856;
        b[4] = 0.55000 * b[4] + w * 0.5329522;
        b[5] = -0.7616 * b[5] - w * 0.0168980;
        let pink = b[0] + b[1] + b[2] + b[3] + b[4] + b[5] + b[6] + w * 0.5362;
        b[6] = w * 0.115926;
        // Leaky integrator (-6 dB/octave) — brown without DC drift.
        self.brown = (self.brown + 0.02 * w) / 1.02;
        [w, pink, self.brown]
    }
}

/// High shelf built from a one-pole split: low + g·high. g = 1 is identity.
#[derive(Default)]
struct Shelf {
    low: f32,
}

impl Shelf {
    #[inline(always)]
    fn process(&mut self, x: f32, coef: f32, gain: f32) -> f32 {
        self.low += coef * (x - self.low);
        self.low + gain * (x - self.low)
    }
}

/// Per-color gains that bring each color, after the tone shelf, to unit RMS,
/// so switching color or tone does not change loudness.
pub fn measure_norms(tone: u8, sample_rate: f32) -> [f32; 3] {
    let gain = tone_gain(tone);
    let coef = one_pole_coef(SHELF_HZ, sample_rate);
    let warmup = (sample_rate * 0.5) as usize;
    let measured = (sample_rate * 1.5) as usize;
    let mut voice = Voice::new(0x1234_5678);
    let mut shelves: [Shelf; 3] = Default::default();
    let mut energy = [0f64; 3];
    for i in 0..warmup + measured {
        let raw = voice.next();
        for c in 0..3 {
            let y = shelves[c].process(raw[c], coef, gain);
            if i >= warmup {
                energy[c] += (y as f64) * (y as f64);
            }
        }
    }
    energy.map(|e| (1.0 / (e / measured as f64).sqrt()) as f32)
}

/// Lock-free control surface shared between the UI thread and the audio thread.
pub struct Params {
    color: AtomicU8,
    tone_gain: AtomicF32,
    norms: [AtomicF32; 3],
    target: AtomicF32,
    fade_secs: AtomicF32,
    fade_seq: AtomicU32,
}

impl Params {
    pub fn new(color: Color, tone: u8, sample_rate: f32) -> Self {
        let norms = measure_norms(tone, sample_rate);
        Params {
            color: AtomicU8::new(color as u8),
            tone_gain: AtomicF32::new(tone_gain(tone)),
            norms: norms.map(AtomicF32::new),
            target: AtomicF32::new(0.0),
            fade_secs: AtomicF32::new(0.0),
            fade_seq: AtomicU32::new(0),
        }
    }

    pub fn set_color(&self, color: Color) {
        self.color.store(color as u8, Ordering::Relaxed);
    }

    pub fn set_tone(&self, tone: u8, sample_rate: f32) {
        let norms = measure_norms(tone, sample_rate);
        for (slot, n) in self.norms.iter().zip(norms) {
            slot.store(n);
        }
        self.tone_gain.store(tone_gain(tone));
    }

    /// Ramp linearly to `amplitude` (output RMS) over `seconds`.
    pub fn fade_to(&self, amplitude: f32, seconds: f32) {
        self.target.store(amplitude);
        self.fade_secs.store(seconds);
        self.fade_seq.fetch_add(1, Ordering::Release);
    }
}

/// Audio-thread state: generators plus smoothed copies of every parameter.
pub struct Synth {
    voices: [Voice; 2],
    shelves: [Shelf; 2],
    shelf_coef: f32,
    fast: f32,
    xfade: f32,
    weights: [f32; 3],
    norms: [f32; 3],
    tone_gain: f32,
    gain: f32,
    target: f32,
    step: f32,
    seen_seq: u32,
    sample_rate: f32,
}

impl Synth {
    /// Starts silent; the next `fade_to` on `params` brings it in.
    pub fn new(params: &Params, sample_rate: f32) -> Self {
        let mut weights = [0.0; 3];
        weights[params.color.load(Ordering::Relaxed) as usize] = 1.0;
        Synth {
            // Different seeds per channel → decorrelated, wide stereo.
            voices: [Voice::new(0x9E37_79B9), Voice::new(0x85EB_CA6B)],
            shelves: Default::default(),
            shelf_coef: one_pole_coef(SHELF_HZ, sample_rate),
            fast: smoothing_coef(0.05, sample_rate),
            xfade: smoothing_coef(0.12, sample_rate),
            weights,
            norms: params.norms.each_ref().map(AtomicF32::load),
            tone_gain: params.tone_gain.load(),
            gain: 0.0,
            target: 0.0,
            step: 0.0,
            seen_seq: params.fade_seq.load(Ordering::Acquire).wrapping_sub(1),
            sample_rate,
        }
    }

    /// Fill an interleaved buffer. Channels beyond the first two copy left.
    pub fn fill<T: Sample + FromSample<f32>>(
        &mut self,
        params: &Params,
        out: &mut [T],
        channels: usize,
    ) {
        let seq = params.fade_seq.load(Ordering::Acquire);
        if seq != self.seen_seq {
            self.seen_seq = seq;
            self.target = params.target.load();
            let samples = (params.fade_secs.load() * self.sample_rate).max(1.0);
            self.step = (self.target - self.gain).abs() / samples;
        }
        let color = params.color.load(Ordering::Relaxed) as usize;
        let norm_targets = params.norms.each_ref().map(AtomicF32::load);
        let tone_target = params.tone_gain.load();

        for frame in out.chunks_mut(channels.max(1)) {
            if self.gain < self.target {
                self.gain = (self.gain + self.step).min(self.target);
            } else if self.gain > self.target {
                self.gain = (self.gain - self.step).max(self.target);
            }
            self.tone_gain += self.fast * (tone_target - self.tone_gain);
            let mut mix_amp = [0f32; 3];
            for c in 0..3 {
                let one_hot = if c == color { 1.0 } else { 0.0 };
                self.weights[c] += self.xfade * (one_hot - self.weights[c]);
                self.norms[c] += self.fast * (norm_targets[c] - self.norms[c]);
                // Weights sum to 1 and the colors are uncorrelated, so sqrt keeps power constant.
                mix_amp[c] = self.weights[c].sqrt() * self.norms[c];
            }
            let mut stereo = [0f32; 2];
            for (ch, out) in stereo.iter_mut().enumerate() {
                let raw = self.voices[ch].next();
                let mixed = raw[0] * mix_amp[0] + raw[1] * mix_amp[1] + raw[2] * mix_amp[2];
                let shaped = self.shelves[ch].process(mixed, self.shelf_coef, self.tone_gain);
                *out = (shaped * self.gain).clamp(-1.0, 1.0);
            }
            for (i, sample) in frame.iter_mut().enumerate() {
                *sample = T::from_sample(stereo[if i == 1 { 1 } else { 0 }]);
            }
        }
    }
}

/// f32 stored as bits in an AtomicU32 (relaxed: each value stands alone).
struct AtomicF32(AtomicU32);

impl AtomicF32 {
    fn new(v: f32) -> Self {
        AtomicF32(AtomicU32::new(v.to_bits()))
    }
    fn load(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
    fn store(&self, v: f32) {
        self.0.store(v.to_bits(), Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    /// Run the synth at full gain and return (rms, high-frequency ratio) of the left channel.
    /// HF ratio = var(first difference) / var(signal): ~2 for white, near 0 for brown.
    fn analyze(color: Color, tone: u8) -> (f32, f32) {
        let params = Params::new(color, tone, SR);
        let mut synth = Synth::new(&params, SR);
        params.fade_to(0.2, 0.0);
        let mut buf = vec![0f32; SR as usize * 4];
        synth.fill(&params, &mut buf, 2); // warm-up second
        synth.fill(&params, &mut buf, 2);
        let left: Vec<f32> = buf.iter().step_by(2).copied().collect();
        let energy: f32 = left.iter().map(|x| x * x).sum::<f32>() / left.len() as f32;
        let diff: f32 =
            left.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum::<f32>() / left.len() as f32;
        (energy.sqrt(), diff / energy)
    }

    #[test]
    fn colors_order_by_brightness() {
        let (_, white) = analyze(Color::White, 50);
        let (_, pink) = analyze(Color::Pink, 50);
        let (_, brown) = analyze(Color::Brown, 50);
        assert!(white > 1.5, "white hf ratio {white}");
        assert!(
            white > pink && pink > brown,
            "white {white} pink {pink} brown {brown}"
        );
        assert!(brown < 0.1, "brown hf ratio {brown}");
    }

    #[test]
    fn tone_moves_brightness_monotonically() {
        let ratios: Vec<f32> = [0, 50, 100]
            .iter()
            .map(|&t| analyze(Color::Brown, t).1)
            .collect();
        assert!(ratios[0] < ratios[1] && ratios[1] < ratios[2], "{ratios:?}");
    }

    #[test]
    fn loudness_is_constant_across_colors_and_tones() {
        for color in Color::ALL {
            for tone in [0, 50, 100] {
                let (rms, _) = analyze(color, tone);
                assert!((rms - 0.2).abs() < 0.02, "{color:?} tone {tone}: rms {rms}");
            }
        }
    }

    #[test]
    fn neutral_tone_is_flat() {
        let mut shelf = Shelf::default();
        let coef = one_pole_coef(SHELF_HZ, SR);
        for x in [0.3, -0.7, 0.1, 0.9] {
            assert!((shelf.process(x, coef, tone_gain(50)) - x).abs() < 1e-6);
        }
    }

    #[test]
    fn fade_reaches_target_and_never_overshoots() {
        let params = Params::new(Color::Brown, 50, SR);
        let mut synth = Synth::new(&params, SR);
        params.fade_to(0.5, 0.1);
        let mut buf = vec![0f32; (SR * 0.2) as usize];
        synth.fill(&params, &mut buf, 1);
        assert_eq!(synth.gain, 0.5);
        params.fade_to(0.0, 0.1);
        synth.fill(&params, &mut buf, 1);
        assert_eq!(synth.gain, 0.0);
        assert!(buf[buf.len() / 2..].iter().all(|&s| s == 0.0));
    }
}
