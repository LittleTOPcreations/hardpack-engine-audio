# Design and provenance

The public package takes the current game's recorded-loop playback as its starting point. Its goal is a small Godot component that a developer can understand without learning Hardpack Rally's car model.

## Extraction map

| Hardpack Rally source | Public component |
|---|---|
| `godot/scripts/audio.gd`: `_bank_pair`, `_bank_weights`, `_bank_update`, loop loading | Bank resource and player: log-RPM weights, equal-power/Gaussian overlap, pitch ratio, throttle low-pass, 40 ms limiter RPM smoothing |
| `ext/src/limiter_audio.rs` | `native/src/limiter.rs`: same sample-clock envelope and atomic controls; renamed classes |
| `harness/src/engine_voice.rs` | Offline generator: same pressure/pipe/render implementation and five voice presets |
| `harness/src/engine_loops.rs` | Selected WAV writer, cycle-aligned cutter and normalization functions |
| `harness/src/pops.rs` | Offline synthetic pop generator |

## Deliberate changes

- Actual RPM is the public input, rather than the game's normalized rev value.
- Explicit bank resources replace car configuration paths. This is easier to inspect and makes export dependencies explicit.
- A resource and a node replace the large combined driving-audio script. No physics objects, replay singletons or game-specific bus names are required.
- The native limiter is optional; a clearly labelled frame-timed version allows a source ZIP to run without compilation.
- Exhaust playback is an explicit event, rather than reading counters from the game's physics. This release does not automatically reproduce the game's overrun timing or grouped limiter pops.
- A separate mix bus per player keeps filters and safety limiting local to a car. The default demo levels leave headroom.
- Generated demonstration sounds replace the game's supplied recordings. No game recording rights are inferred from source-code licensing.

## Limits

This is an audio player, not a combustion or drivetrain simulator. The generated voices are stylized; realistic results depend strongly on source recordings and loop preparation. The fallback limiter is frame-timed and cannot preserve short cuts through scene stalls. Native Windows x64 is tested locally; Linux is checked by GitHub Actions. See [validation notes](validation.md) and the current workflow result. macOS, mobile, Web and older Godot releases are not claimed as tested.

The core is intentionally small. Adding a car-physics dependency or a large editor plugin would make the first-use experience harder. More advanced automatic exhaust behaviour can be added as a separate component later, without widening the player's required inputs.
