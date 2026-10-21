//! Optional offline generator. The addon itself does not need Rust.
mod engine_loops;
mod engine_voice;
mod pops;
use std::collections::BTreeMap;

fn main() {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_default();
    if command != "engine" && command != "pops" {
        eprintln!("voice-bank engine --voice vulcan --out DIR [--loops 8] [--seed 7]\nvoice-bank pops --out DIR [--count 6] [--seed 7]");
        std::process::exit(2);
    }
    let mut opts = BTreeMap::new();
    while let Some(key) = args.next() {
        let Some(value) = args.next() else { eprintln!("Missing value for {key}"); std::process::exit(2) };
        if !key.starts_with("--") { eprintln!("Expected --option, got {key}"); std::process::exit(2) }
        let name = key.trim_start_matches("--");
        if !["out", "voice", "idle", "redline", "loops", "seconds", "seed", "pipe-scale", "pipe-mix", "gas-noise", "jitter", "count"].contains(&name) {
            eprintln!("Unknown option: {key}"); std::process::exit(2);
        }
        if name != "out" && name != "voice" {
            let number = value.parse::<f64>().unwrap_or(f64::NAN);
            let (low, high) = match name {
                "idle" | "redline" => (100.0, 20000.0),
                "loops" | "count" => (2.0, 32.0),
                "seconds" => (0.1, 5.0),
                "pipe-scale" => (0.1, 4.0),
                "pipe-mix" => (0.0, 1.0),
                "seed" => (0.0, u32::MAX as f64),
                _ => (0.0, 2.0),
            };
            if !number.is_finite() || !(low..=high).contains(&number) {
                eprintln!("{key} must be between {low} and {high}"); std::process::exit(2);
            }
        }
        opts.insert(name.to_string(), value);
    }
    let idle = opts.get("idle").map(|v| v.parse::<f64>().unwrap()).unwrap_or(900.0);
    let redline = opts.get("redline").map(|v| v.parse::<f64>().unwrap()).unwrap_or(7000.0);
    if redline <= idle * 1.22 { eprintln!("Redline must exceed idle by at least 22% to space the loops."); std::process::exit(2) }
    let result = if command == "engine" { engine_voice::run(&opts) } else { pops::run(&opts) };
    if let Err(error) = result { eprintln!("{error}"); std::process::exit(1) }
}
