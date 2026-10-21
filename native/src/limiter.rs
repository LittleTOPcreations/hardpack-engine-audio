//! Ignition cuts on the audio mixer clock. The scene thread only publishes
//! control changes; it never advances the envelope or touches mixer state.
use godot::classes::native::AudioFrame;
use godot::classes::{
    AudioEffect, AudioEffectInstance, AudioServer, IAudioEffect, IAudioEffectInstance,
};
use godot::prelude::*;
use godot_core::meta::RawPtr;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

struct Control {
    command: AtomicU64,
    gain: AtomicU32,
}

impl Default for Control {
    fn default() -> Self {
        Self {
            command: AtomicU64::new(0),
            gain: AtomicU32::new(1.0f32.to_bits()),
        }
    }
}

#[derive(Default)]
struct Envelope {
    sample: u32,
    depth: f64,
    command: u64,
}

// Listening controls: a small fade into the ignition cut, a coast-down
// dwell suggesting ~500 RPM of flywheel fall, then an almost immediate pickup.
// This shapes audio only; it does not alter native vehicle RPM or torque.
const CUT_FADE_SECONDS: f64 = 0.005;
const COAST_SECONDS: f64 = 0.050;
const PICKUP_SECONDS: f64 = 0.001;

fn cut(phase: f64, period_seconds: f64) -> f64 {
    fn smooth(x: f64) -> f64 {
        x * x * (3.0 - 2.0 * x)
    }
    let pickup_start = 1.0 - PICKUP_SECONDS / period_seconds;
    let coast_start = pickup_start - COAST_SECONDS.min(period_seconds * 0.6) / period_seconds;
    let fade_start = coast_start - CUT_FADE_SECONDS / period_seconds;
    if phase < fade_start {
        1.0
    } else if phase < coast_start {
        1.0 - 0.92 * smooth((phase - fade_start) / (coast_start - fade_start))
    } else if phase < pickup_start {
        0.08
    } else {
        0.08 + 0.92 * smooth((phase - pickup_start) / (1.0 - pickup_start))
    }
}

impl Envelope {
    fn next(&mut self, command: u64, rate: f64) -> f32 {
        let period = (command as u32).max(1);
        let mode = (command >> 32) & 3;
        // The generation catches resets even when a car switch and re-entry
        // occur between mixer blocks. Ordinary repeated updates never reset.
        if command >> 34 != self.command >> 34 || period != self.command as u32 {
            self.sample = 0;
            self.depth = 0.0;
        }
        self.command = command;
        if mode == 2 {
            self.sample = 0;
            self.depth = 0.0;
            return 1.0;
        }
        if mode == 1 {
            self.depth = (self.depth + 1.0 / (rate * 0.06)).min(1.0);
        } else {
            self.depth = (self.depth - 1.0 / (rate * 0.10)).max(0.0);
        }
        let gain = (1.0
            + (cut(self.sample as f64 / period as f64, period as f64 / rate) - 1.0) * self.depth)
            as f32;
        if self.depth > 0.0 {
            self.sample = (self.sample + 1) % period;
        } else {
            self.sample = 0;
        }
        gain
    }
}

#[derive(GodotClass)]
#[class(base=AudioEffect)]
pub struct EngineAudioLimiterEffect {
    base: Base<AudioEffect>,
    control: Arc<Control>,
    rate: f64,
    generation: u64,
}

#[godot_api]
impl IAudioEffect for EngineAudioLimiterEffect {
    fn init(base: Base<AudioEffect>) -> Self {
        Self {
            base,
            control: Arc::default(),
            rate: AudioServer::singleton().get_mix_rate() as f64,
            generation: 0,
        }
    }

    fn instantiate(&mut self) -> Option<Gd<AudioEffectInstance>> {
        let control = self.control.clone();
        let rate = self.rate;
        Some(
            Gd::from_init_fn(|base| EngineAudioLimiterInstance {
                base,
                control,
                envelope: Envelope::default(),
                rate,
            })
            .upcast(),
        )
    }
}

#[godot_api]
impl EngineAudioLimiterEffect {
    #[func]
    fn configure(&mut self, active: bool, period_seconds: f64, reset: bool) {
        if reset {
            self.generation = self.generation.wrapping_add(1) & ((1 << 30) - 1);
        }
        let period = (period_seconds.clamp(0.05, 1.0) * self.rate).round() as u64;
        let mode = if reset {
            2
        } else if active {
            1
        } else {
            0
        };
        self.control.command.store(
            period | (mode << 32) | (self.generation << 34),
            Ordering::Release,
        );
    }

    /// Diagnostic only; the main thread must never apply this sampled value
    /// to the players, which would double-cut and reintroduce frame stepping.
    #[func]
    fn current_gain(&self) -> f64 {
        f32::from_bits(self.control.gain.load(Ordering::Relaxed)) as f64
    }
}

#[derive(GodotClass)]
#[class(base=AudioEffectInstance, no_init)]
struct EngineAudioLimiterInstance {
    base: Base<AudioEffectInstance>,
    control: Arc<Control>,
    envelope: Envelope,
    rate: f64,
}

#[godot_api]
impl IAudioEffectInstance for EngineAudioLimiterInstance {
    unsafe fn process_rawptr(
        &mut self,
        src_buffer: RawPtr<*const c_void>,
        r_dst_buffer: RawPtr<*mut AudioFrame>,
        frame_count: i32,
    ) {
        let command = self.control.command.load(Ordering::Acquire);
        let src = src_buffer.ptr().cast::<AudioFrame>();
        let dst = r_dst_buffer.ptr();
        let mut gain = 1.0;
        for i in 0..frame_count.max(0) as usize {
            gain = self.envelope.next(command, self.rate);
            // SAFETY: Godot supplies frame_count valid AudioFrames in each
            // buffer. Copy each input before writing, allowing in-place use.
            unsafe {
                let left = (*src.add(i)).left;
                let right = (*src.add(i)).right;
                (*dst.add(i)).left = left * gain;
                (*dst.add(i)).right = right * gain;
            }
        }
        self.control.gain.store(gain.to_bits(), Ordering::Relaxed);
    }

    fn process_silence(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiny_fade_coast_pause_and_sharp_pickup() {
        // At 180 ms: powered to 124 ms, fade to 129, coast to 179,
        // then recover in the final millisecond without a discontinuity.
        assert_eq!(cut(0.123 / 0.180, 0.180), 1.0);
        let fading = cut(0.1265 / 0.180, 0.180);
        assert!((0.53..0.55).contains(&fading));
        for ms in 130..179 {
            assert_eq!(cut(ms as f64 / 180.0, 0.180), 0.08);
        }
        assert!((0.53..0.55).contains(&cut(0.1795 / 0.180, 0.180)));
        assert!((cut(1.0, 0.180) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn sample_clock_has_exact_period_at_every_output_rate() {
        for rate in [44100.0, 48000.0, 96000.0] {
            let period = (rate * 0.180_f64).round() as u64;
            let command = period | (1 << 32);
            let mut env = Envelope::default();
            let mut previous = 1.0;
            let mut cuts = Vec::new();
            // No frame updates at all: this also exercises a scene-thread stall.
            for i in 0..(rate * 3.0) as usize {
                let gain = env.next(command, rate);
                assert!((0.0799..=1.0).contains(&gain));
                if gain < 0.2 && previous >= 0.2 {
                    cuts.push(i);
                }
                previous = gain;
            }
            assert_eq!(cuts.len(), 16);
            assert!(cuts.windows(2).all(|w| w[1] - w[0] == period as usize));
        }
    }

    #[test]
    fn release_and_reset_restore_full_note_without_stale_phase() {
        let rate = 48000.0;
        let period = 8640;
        let active = period | (1 << 32);
        let mut env = Envelope::default();
        for _ in 0..7300 {
            env.next(active, rate);
        }
        assert!(env.next(active, rate) < 0.1);
        for _ in 0..4801 {
            env.next(period, rate);
        }
        assert_eq!(env.depth, 0.0);
        assert_eq!(env.sample, 0);
        assert_eq!(env.next(active, rate), 1.0);
        for _ in 0..7300 {
            env.next(active, rate);
        }
        assert_eq!(env.next(period | (2 << 32) | (1 << 34), rate), 1.0);
        assert_eq!(env.sample, 0);
        assert_eq!(env.next(active | (1 << 34), rate), 1.0);
        // Reset then re-entry between blocks still carries a new generation.
        assert_eq!(env.next(active | (2 << 34), rate), 1.0);
        assert_eq!(env.sample, 1);
    }
}
