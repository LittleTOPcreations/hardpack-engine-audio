//! Synthetic exhaust one-shots extracted from Hardpack Rally.
use std::collections::BTreeMap;
use std::f64::consts::PI;
use std::fs;
use std::path::Path;

use crate::engine_loops::{write_wav, RATE};

/// xorshift32, the old Math.random made repeatable.
struct Rng(u32);

impl Rng {
    fn new(seed: u64) -> Self {
        let s = (seed as u32) ^ ((seed >> 32) as u32) ^ 0x2545_F491;
        Rng(if s == 0 { 1 } else { s })
    }

    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f64 / (u32::MAX as f64 + 1.0)
    }

    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next()
    }
}

/// A cookbook biquad, band-pass (constant skirt) or high-pass.
struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Biquad {
    fn new() -> Self {
        Biquad { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    fn bandpass(&mut self, hz: f64, q: f64) {
        let w0 = 2.0 * PI * hz.clamp(10.0, RATE as f64 * 0.45) / RATE as f64;
        let alpha = w0.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        self.b0 = alpha / a0;
        self.b1 = 0.0;
        self.b2 = -alpha / a0;
        self.a1 = -2.0 * w0.cos() / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    fn highpass(&mut self, hz: f64, q: f64) {
        let w0 = 2.0 * PI * hz.clamp(10.0, RATE as f64 * 0.45) / RATE as f64;
        let alpha = w0.sin() / (2.0 * q);
        let c = w0.cos();
        let a0 = 1.0 + alpha;
        self.b0 = (1.0 + c) / 2.0 / a0;
        self.b1 = -(1.0 + c) / a0;
        self.b2 = (1.0 + c) / 2.0 / a0;
        self.a1 = -2.0 * c / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    fn run(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// The browser's gain automation: linear to `peak` over `attack`, then
/// exponential to `floor` at `decay`.
fn envelope(t: f64, attack: f64, peak: f64, decay: f64, floor: f64) -> f64 {
    if t < 0.0 {
        0.0
    } else if t < attack {
        peak * t / attack
    } else if t < decay {
        let u = (t - attack) / (decay - attack).max(1e-6);
        peak * (floor / peak).powf(u)
    } else {
        0.0
    }
}

/// One crack (the old `crack()`): its parameters rolled, then rendered.
fn crack(rng: &mut Rng, level: f64) -> Vec<f32> {
    let short = rng.next() < 0.5;
    let decay = if short { rng.range(0.12, 0.18) } else { rng.range(0.20, 0.36) };
    let f_start = rng.range(620.0, 1080.0);
    let f_end = rng.range(72.0, 118.0);
    let q = rng.range(0.7, 1.6);
    let partials = [rng.range(46.0, 62.0), rng.range(78.0, 104.0), rng.range(126.0, 168.0)];
    let partial_decay = if short { rng.range(0.10, 0.16) } else { rng.range(0.18, 0.32) };
    let partial_level = 0.42;
    let drive = rng.range(8.0, 14.0);
    let click_level = if short { 1.15 } else { 0.7 };
    let click_hz = if short { rng.range(3400.0, 6800.0) } else { rng.range(2600.0, 5200.0) };
    let click_decay = if short { 0.013 } else { 0.012 };
    let attack = 0.0007;
    let partial_attack = 0.0009;
    // the partials: each its own detune, level and decay
    let parts: Vec<(f64, f64, f64)> = partials
        .iter()
        .map(|f| (f * rng.range(0.96, 1.05), level * partial_level * rng.range(0.7, 1.3), partial_decay * rng.range(0.7, 1.35)))
        .collect();
    let longest = parts.iter().map(|p| p.2).fold(decay, f64::max) + 0.05;
    let n = (longest * RATE as f64) as usize;
    let mut body = Biquad::new();
    let mut click = Biquad::new();
    click.highpass(click_hz, 0.7);
    let norm = drive.tanh();
    let mut out = Vec::with_capacity(n);
    let mut phases = [0.0f64; 3];
    for i in 0..n {
        let t = i as f64 / RATE as f64;
        // the body: noise through the band-pass, its center sliding down
        // exponentially over 0.8 of the decay (recomputed every 16 samples)
        if i % 16 == 0 {
            let u = (t / (decay * 0.8)).min(1.0);
            let hz = f_start * (f_end / f_start).powf(u);
            body.bandpass(hz, q);
        }
        let white = rng.next() * 2.0 - 1.0;
        let mut v = body.run(white) * envelope(t, attack, level, decay, 0.0008);
        // the click: high-passed noise, gone in a hundredth
        if t < 0.02 {
            let white2 = rng.next() * 2.0 - 1.0;
            let g = if t < click_decay { level * click_level * (0.0005 / (level * click_level).max(1e-6)).powf(t / click_decay) } else { 0.0 };
            v += click.run(white2) * g;
        }
        // the ringing modes: triangles
        for (k, (hz, lv, dk)) in parts.iter().enumerate() {
            phases[k] += hz / RATE as f64;
            if phases[k] >= 1.0 {
                phases[k] -= 1.0;
            }
            let tri = 4.0 * (phases[k] - 0.5).abs() - 1.0;
            v += tri * envelope(t, partial_attack, *lv, *dk, 0.0006);
        }
        out.push(((v * drive).tanh() / norm) as f32);
    }
    out
}

fn num(opts: &BTreeMap<String, String>, key: &str, default: f64) -> f64 {
    opts.get(key).and_then(|s| s.parse().ok()).unwrap_or(default)
}

/// The command: `count` cracks into DIR as `pop-<n>.wav`, each peak at -3 dBFS.
pub fn run(opts: &BTreeMap<String, String>) -> Result<(), String> {
    let out_dir = opts.get("out").ok_or("--out DIR is needed")?;
    let count = num(opts, "count", 12.0).max(1.0) as usize;
    let seed = num(opts, "seed", 7.0) as u64;
    fs::create_dir_all(out_dir).map_err(|e| format!("{out_dir}: {e}"))?;
    let mut rng = Rng::new(seed);
    let mut manifest = String::from("# the old game's exhaust pops rendered by `rally pops`; the shell picks one per pop\n");
    for i in 0..count {
        // the old game's level: 0.55 to 1.05 of a pop times backfireLevel 0.52
        let level = rng.range(0.55, 1.05) * 0.52;
        let mut s = crack(&mut rng, level);
        let peak = s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        if peak > 0.0 {
            let g = 0.708 / peak;
            for v in s.iter_mut() {
                *v *= g;
            }
        }
        let name = format!("pop-{:02}.wav", i + 1);
        write_wav(&Path::new(out_dir).join(&name), RATE, &s)?;
        manifest.push_str(&format!("{name} {:.3} s\n", s.len() as f64 / RATE as f64));
        println!("  {name}: {:.3} s", s.len() as f64 / RATE as f64);
    }
    fs::write(Path::new(out_dir).join("bank.txt"), manifest).map_err(|e| format!("{out_dir}: {e}"))?;
    println!("{count} pops in {out_dir}");
    Ok(())
}
