# Initial release validation

Local environment: Windows x64, Godot 4.7.2, Rust 1.98.1. The native crate requires Rust 1.94+.

Checked before publishing:

- Clean Godot import and standalone demo rendering, with the screenshot inspected for clipping and readability.
- Portable and native addon integration: valid/invalid banks, single-loop banks, blend normalization and continuity across RPM anchors, actual RPM pitch ratios, independent cars/buses, limiter continuity, pop playback, mute, bank replacement, non-finite input handling and bus cleanup.
- Actual Godot mixer capture through a listener: audible engine output, peak headroom, limiter/pop playback, tree pause/resume, mute and bank switching. A steady 3500 RPM Vulcan capture at 70% throttle measured approximately RMS 0.055 and peak 0.15 (linear amplitude; running loop phase varies).
- Native envelope tests at 44.1, 48 and 96 kHz, including release and reset.
- Native mixer timing: 11 consecutive cuts spaced exactly 7,938 samples apart at 44.1 kHz / 180 ms, including a deliberate 550 ms scene-thread stall.
- Demo tones and pops regenerated from the included source. No game recordings were copied.

These are functional and numerical checks, not a substitute for listening on a range of speakers or profiling a large racing grid. The demo screenshot is captured from Godot. No claim is made that generated demo tones match real engines.

A ready-to-use Linux GitHub Actions configuration is supplied in `docs/ci-example.yml`. It is not active: the publishing account's token lacked the `workflow` scope. To enable it with a suitably authorized account, copy it to `.github/workflows/check.yml`. No CI pass is claimed. Linux, macOS, mobile, Web, custom Godot builds and versions before 4.7 are not locally verified.
