//! WAV output and loop cutting extracted from Hardpack Rally.
use std::{fs, path::Path, f64::consts::PI};
pub const RATE: u32 = 44_100;
pub fn write_wav(path: &Path, rate: u32, samples: &[f32]) -> Result<(), String> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    fs::write(path, out).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn cut_loop(signal: &[f32], rate: u32, center: usize, hz: f64, seconds: f64, fade: usize) -> Option<Vec<f32>> {
    let period = rate as f64 / hz;
    let periods = (seconds * hz).round().max(1.0);
    let len = (periods * period).round() as usize;
    let start = center.checked_sub(len / 2)?;
    if start + len + fade > signal.len() {
        return None;
    }
    let s = &signal[start..start + len + fade];
    let mut out = vec![0.0f32; len];
    for i in 0..len {
        if i < fade {
            let w = ((i as f64 + 0.5) / fade as f64 * PI / 2.0).sin();
            let w2 = (1.0 - w * w).sqrt();
            out[i] = (s[i] as f64 * w + s[len + i] as f64 * w2) as f32;
        } else {
            out[i] = s[i];
        }
    }
    Some(out)
}

/// RMS to `target_db`, the peak held under -1 dBFS.
pub fn normalize(samples: &mut [f32], target_db: f64) {
    let rms = (samples.iter().map(|s| (*s as f64) * (*s as f64)).sum::<f64>() / samples.len().max(1) as f64).sqrt();
    if rms <= 1e-9 {
        return;
    }
    let mut gain = 10f64.powf(target_db / 20.0) / rms;
    let peak = samples.iter().fold(0.0f64, |m, s| m.max((*s as f64).abs()));
    if peak * gain > 0.891 {
        gain = 0.891 / peak;
    }
    for s in samples.iter_mut() {
        *s = (*s as f64 * gain) as f32;
    }
}
