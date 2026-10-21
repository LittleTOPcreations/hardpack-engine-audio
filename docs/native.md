# Optional audio-clock limiter

Use this when rev-limiter rhythm must remain exact during frame drops. It is the small native effect from Hardpack Rally, separated from the game's extension. The portable player detects it automatically.

The Windows x64 release ZIP already contains `addons/engine_audio_native` and requires no compiler. Copy that folder alongside `addons/engine_audio`, then restart Godot. For other targets build from source; no cross-platform binary compatibility is implied.

## Build from source

Install Rust **1.94 or newer** and the native linker for your operating system. Windows requires the Visual Studio C++ build tools; Linux requires a C linker; macOS requires Xcode command-line tools.

```sh
cargo build --release --locked --manifest-path native/Cargo.toml
python tools/install_native.py
```

Restart Godot after installing. The demo should say **audio-clock native effect**. When copying the addon to another project, include both addon folders. Keep the licence files with your distribution.

`install_native.py` copies the current platform's library and the `.gdextension` descriptor. It expects the host build in `native/target/release`; custom Cargo target paths require manual copying. The descriptor targets standard-precision Godot 4.7+, with Windows/Linux x64 and macOS entries. macOS must be built for the architecture on which it will run. Custom double-precision Godot builds are not supported by the supplied binaries.

For Windows builds that do not require a separate Visual C++ runtime:

```powershell
$env:RUSTFLAGS = '-C target-feature=+crt-static'
cargo build --release --locked --manifest-path native/Cargo.toml
python tools/install_native.py
```

## Verify

```sh
cargo test --locked --manifest-path native/Cargo.toml
godot --headless --editor --path . --import
godot --headless --audio-driver Dummy --path . --script tests/native_clock.gd
```

The mixer test deliberately stalls scene updates for 550 ms and checks exact sample spacing. It exits with failure if the native class is unavailable. Remove `addons/engine_audio_native` and restart Godot to return to the portable limiter; do not leave a descriptor pointing to a missing library.
