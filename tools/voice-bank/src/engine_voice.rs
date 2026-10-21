//! Procedural voice generator extracted from Hardpack Rally.
//! Inspired by Ange Yaghi's Engine Simulator; see THIRD_PARTY_NOTICES.md.
use std::collections::BTreeMap;
use std::f64::consts::PI;
use std::fs;
use std::path::Path;

use crate::engine_loops::{cut_loop, normalize, write_wav, RATE};

/// A voice: the old VOICES table.
#[derive(Clone, Copy)]
pub struct Voice {
    pub name: &'static str,
    pub cylinders: usize,
    pub unevenness: f64,
    pub pipe: f64,
    pub openness: f64,
    pub damping: f64,
    pub decay: f64,
    pub rumble: f64,
    pub noise_mix: f64,
    pub jitter: f64,
    pub cut_base: f64,
    pub cut_track: f64,
    pub drive: f64,
}

/// "an even-fire four barks, an odd-fire five warbles, a flat six smooths
/// out"; the vulcan carried all four of the old cars.
pub const VOICES: [Voice; 5] = [
    Voice { name: "comet", cylinders: 4, unevenness: 0.00, pipe: 1.35, openness: 0.60, damping: 0.45, decay: 0.62, rumble: 0.35, noise_mix: 0.50, jitter: 0.14, cut_base: 520.0, cut_track: 4.6, drive: 7.0 },
    Voice { name: "vulcan", cylinders: 5, unevenness: 0.22, pipe: 2.10, openness: 0.38, damping: 0.62, decay: 0.48, rumble: 0.62, noise_mix: 0.62, jitter: 0.20, cut_base: 380.0, cut_track: 3.6, drive: 11.0 },
    Voice { name: "briar", cylinders: 4, unevenness: 0.08, pipe: 0.65, openness: 0.85, damping: 0.28, decay: 0.72, rumble: 0.24, noise_mix: 0.70, jitter: 0.26, cut_base: 640.0, cut_track: 5.4, drive: 9.0 },
    Voice { name: "wisp", cylinders: 6, unevenness: 0.00, pipe: 0.85, openness: 0.78, damping: 0.30, decay: 0.80, rumble: 0.16, noise_mix: 0.42, jitter: 0.10, cut_base: 760.0, cut_track: 6.6, drive: 5.0 },
    Voice { name: "goliath", cylinders: 8, unevenness: 0.30, pipe: 2.70, openness: 0.30, damping: 0.70, decay: 0.42, rumble: 0.88, noise_mix: 0.55, jitter: 0.18, cut_base: 300.0, cut_track: 3.0, drive: 13.0 },
];

/// Exhaust gas is hot, so sound moves faster in it.
const C_SOUND: f64 = 343.0 * 1.6;

/// The old game's Math.random, made repeatable: xorshift32.
struct Rng(u32);

impl Rng {
    fn new(seed: u64) -> Self {
        let s = (seed as u32) ^ ((seed >> 32) as u32) ^ 0x9E37_79B9;
        Rng(if s == 0 { 1 } else { s })
    }

    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f64 / (u32::MAX as f64 + 1.0)
    }
}

/// The pipe's impulse response (the old exhaust-ir.js, its left channel):
/// a wave takes 2L/c per round trip in a pipe open at one end, so a train
/// of echoes at that spacing, each quieter (the open end radiates) and
/// duller (a widening smear), the spacing a little irregular, turbulent
/// hiss on the tail, the peak normalized so the length is not a volume.
fn exhaust_ir(rng: &mut Rng, length_m: f64, openness: f64, damping: f64, seconds: f64) -> Vec<f32> {
    let rate = RATE as f64;
    let n = ((rate * seconds).floor() as usize).max(64);
    let mut d = vec![0.0f64; n];
    let round_trip = 2.0 * length_m / C_SOUND;
    let gap = ((round_trip * rate).round() as i64).max(4);
    let bounce = 0.30 + 0.55 * (1.0 - openness);
    let mut amp = 1.0;
    let mut sign = 1.0;
    let mut pos: i64 = 0;
    let mut k = 0;
    while (pos as usize) < n && amp > 0.0015 && k < 220 {
        let i = pos as usize;
        let width = ((k as f64 * damping * 5.0 + 1.0).round() as usize).max(1);
        for w in 0..width {
            if i + w >= n {
                break;
            }
            let taper = 1.0 - w as f64 / width as f64;
            d[i + w] += sign * amp * taper / width as f64;
        }
        amp *= bounce;
        sign = -sign;
        k += 1;
        pos += gap + ((rng.next() - 0.5) * gap as f64 * 0.12).round() as i64;
    }
    for (i, s) in d.iter_mut().enumerate() {
        let t = i as f64 / n as f64;
        *s += (rng.next() * 2.0 - 1.0) * (1.0 - t).powf(2.6) * 0.05;
    }
    let peak = d.iter().fold(0.0f64, |m, s| m.max(s.abs()));
    if peak > 0.0 {
        let g = 0.92 / peak;
        for s in d.iter_mut() {
            *s *= g;
        }
    }
    d.iter().map(|s| *s as f32).collect()
}

/// The pressure signal (the old engine-processor.js) at a steady rpm and
/// load, `n` samples.
fn pressure(rng: &mut Rng, v: &Voice, rpm: f64, load: f64, jitter_scale: f64, noise_scale: f64, n: usize) -> Vec<f32> {
    let rate = RATE as f64;
    let cyl = v.cylinders.clamp(1, 12);
    let step = 720.0 / cyl as f64;
    let firing: Vec<f64> = (0..cyl)
        .map(|i| {
            let skew = if i % 2 == 1 { 1.0 } else { -1.0 } * v.unevenness * step * 0.5;
            i as f64 * step + skew
        })
        .collect();
    // no two cylinders are identical
    let trim: Vec<f64> = (0..cyl).map(|_| 0.86 + rng.next() * 0.28).collect();
    let jitter = v.jitter * jitter_scale;
    let noise_mix = v.noise_mix * noise_scale;
    let deg_per_sample = rpm * 360.0 / (60.0 * rate);
    // a hotter, harder-working engine empties its cylinder faster: the note
    // sharpens under load rather than just getting louder
    let decay = (-(1.0 / rate) * (140.0 + 900.0 * v.decay + load * 700.0)).exp();
    let mut angle = 0.0;
    let mut fired = vec![false; cyl];
    let mut p = 0.0;
    let mut lp = 0.0;
    let mut dc = 0.0;
    let mut prev = 0.0;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        angle += deg_per_sample;
        if angle >= 720.0 {
            angle -= 720.0;
            fired.iter_mut().for_each(|f| *f = false);
        }
        for c in 0..cyl {
            if !fired[c] && angle >= firing[c] {
                fired[c] = true;
                let j = 1.0 + (rng.next() * 2.0 - 1.0) * jitter;
                p += trim[c] * j * (0.55 + load * 0.75);
            }
        }
        p *= decay;
        // turbulent gas multiplies the pressure; adding would just lay hiss on it
        let white = rng.next() * 2.0 - 1.0;
        lp += (white - lp) * 0.22;
        let m = noise_mix * lp + (1.0 - noise_mix);
        let mut s = p * m;
        s += (p - prev) * 6.0 * (0.3 + load * 0.7);
        prev = p;
        dc += (s - dc) * 0.0016;
        s -= dc;
        out.push(((s * (1.0 + v.rumble)).tanh() * 0.42) as f32);
    }
    out
}

/// Direct convolution: the signal through the pipe.
fn convolve(x: &[f32], ir: &[f32]) -> Vec<f32> {
    let mut y = vec![0.0f32; x.len()];
    for (i, xi) in x.iter().enumerate() {
        if *xi == 0.0 {
            continue;
        }
        let end = ir.len().min(x.len() - i);
        let slice = &mut y[i..i + end];
        for (yj, hj) in slice.iter_mut().zip(ir.iter()) {
            *yj += xi * hj;
        }
    }
    y
}

/// A second-order low-pass (the cookbook biquad the browser's is).
struct LowPass {
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

impl LowPass {
    fn new(hz: f64, q: f64) -> Self {
        let w0 = 2.0 * PI * hz.min(RATE as f64 * 0.45) / RATE as f64;
        let alpha = w0.sin() / (2.0 * q);
        let c = w0.cos();
        let a0 = 1.0 + alpha;
        LowPass {
            b0: (1.0 - c) / 2.0 / a0,
            b1: (1.0 - c) / a0,
            b2: (1.0 - c) / 2.0 / a0,
            a1: -2.0 * c / a0,
            a2: (1.0 - alpha) / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
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

/// The settings the old config carried live.
pub struct Pipe {
    pub scale: f64,
    pub mix: f64,
    pub gas_noise: f64,
    pub jitter: f64,
}

/// One steady note: the pressure through the pipe, wet and dry, the tone
/// by the firing rate and the load, the soft clip. `seconds` of it.
pub fn render(v: &Voice, rpm: f64, load: f64, seconds: f64, pipe: &Pipe, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let ir = exhaust_ir(&mut rng, v.pipe * pipe.scale, v.openness, v.damping, 0.42);
    let n = (seconds * RATE as f64) as usize;
    let x = pressure(&mut rng, v, rpm, load, pipe.jitter, pipe.gas_noise, n);
    let wet = convolve(&x, &ir);
    let firing = rpm * v.cylinders as f64 / 120.0;
    let cut = (v.cut_base + firing * v.cut_track * (0.55 + load * 0.75)).min(12000.0);
    let mut tone = LowPass::new(cut, 0.7);
    let norm = v.drive.tanh();
    x.iter()
        .zip(wet.iter())
        .map(|(d, w)| {
            let mixed = (*w as f64) * pipe.mix + (*d as f64) * (1.0 - pipe.mix);
            let t = tone.run(mixed);
            ((t * v.drive).tanh() / norm) as f32
        })
        .collect()
}

fn num(opts: &BTreeMap<String, String>, key: &str, default: f64) -> f64 {
    opts.get(key).and_then(|s| s.parse().ok()).unwrap_or(default)
}

/// The command: a bank per voice.
pub fn run(opts: &BTreeMap<String, String>) -> Result<(), String> {
    let want = opts.get("voice").map(String::as_str).unwrap_or("vulcan");
    let out_dir = opts.get("out").ok_or("--out DIR is needed")?;
    let idle = num(opts, "idle", 900.0);
    let redline = num(opts, "redline", 7000.0);
    let loops = num(opts, "loops", 8.0).max(2.0) as usize;
    let seconds = num(opts, "seconds", 1.0);
    let seed = num(opts, "seed", 7.0) as u64;
    let pipe = Pipe {
        scale: num(opts, "pipe-scale", 1.0),
        mix: num(opts, "pipe-mix", 0.72),
        gas_noise: num(opts, "gas-noise", 1.0),
        jitter: num(opts, "jitter", 1.0),
    };
    let voices: Vec<&Voice> = if want == "all" {
        VOICES.iter().collect()
    } else {
        match VOICES.iter().find(|v| v.name == want) {
            Some(v) => vec![v],
            None => return Err(format!("no voice {want}; the old game's: {}", VOICES.iter().map(|v| v.name).collect::<Vec<_>>().join(", "))),
        }
    };
    let fade = (0.04 * RATE as f64) as usize;
    for v in voices {
        let dir = if want == "all" { Path::new(out_dir).join(v.name) } else { Path::new(out_dir).to_path_buf() };
        fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let mut manifest = format!(
            "# the old game's {} voice rendered by `rally engine-voice`: {} cylinders, unevenness {}, pipe {} m x {}, mix {}\n# rpm file\n",
            v.name, v.cylinders, v.unevenness, v.pipe, pipe.scale, pipe.mix
        );
        // the idle at its own load, then the range log-spaced to just under
        // the redline, each under more load as the revs climb
        let mut targets = vec![(idle, 0.05)];
        let top = redline * 0.97;
        let lo = idle * 1.18;
        for i in 0..loops {
            let t = i as f64 / (loops - 1) as f64;
            targets.push((lo * (top / lo).powf(t), 0.15 + 0.75 * t));
        }
        let mut written = 0;
        for (i, (rpm, load)) in targets.iter().enumerate() {
            let take = render(v, *rpm, *load, seconds + 1.6, &pipe, seed.wrapping_mul(31).wrapping_add(i as u64));
            // the loop from the settled middle, a whole number of 720° cycles
            let center = take.len() * 3 / 5;
            let cycle_hz = rpm / 120.0;
            let Some(mut lp) = cut_loop(&take, RATE, center, cycle_hz, seconds, fade) else {
                println!("  {rpm:.0} rpm: the take was too short for a loop, skipped");
                continue;
            };
            normalize(&mut lp, -18.0);
            let name = format!("{}.wav", rpm.round() as i64);
            write_wav(&dir.join(&name), RATE, &lp)?;
            manifest.push_str(&format!("{} {name}\n", rpm.round() as i64));
            println!("  {}: {rpm:.0} rpm at load {load:.2}: {name}, {:.2} s", v.name, lp.len() as f64 / RATE as f64);
            written += 1;
        }
        fs::write(dir.join("bank.txt"), manifest).map_err(|e| format!("{}: {e}", dir.display()))?;
        println!("{written} loops of the {} voice in {}; name the folder in the car file under [sound] engine", v.name, dir.display());
    }
    Ok(())
}
