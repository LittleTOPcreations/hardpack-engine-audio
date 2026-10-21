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

GitHub Actions in `.github/workflows/check.yml` repeats import/integration/mixer checks on Linux and builds/tests the native extension. Consult the repository's Actions status for each run's outcome. macOS, mobile, Web, custom Godot builds and versions before 4.7 are not verified. Windows is tested locally; Linux is exercised by CI.

Native headless import uses `--frame-delay 1000` to allow deferred editor documentation callbacks to finish before shutdown, following the workaround for [Godot issue #111048](https://github.com/godotengine/godot/issues/111048). Playback and sample-clock tests run at normal speed without that delay.
