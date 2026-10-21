#!/usr/bin/env python3
"""Install a host-built optional limiter into the demo project (Python stdlib)."""
from pathlib import Path
import platform
import shutil

root = Path(__file__).resolve().parents[1]
names = {"Windows": "engine_audio_native.dll", "Linux": "libengine_audio_native.so", "Darwin": "libengine_audio_native.dylib"}
name = names[platform.system()]
source = root / "native/target/release" / name
if not source.is_file():
    raise SystemExit("Build first: cargo build --release --locked --manifest-path native/Cargo.toml")
dest = root / "addons/engine_audio_native"
(dest / "bin").mkdir(parents=True, exist_ok=True)
shutil.copy2(source, dest / "bin" / name)
shutil.copy2(root / "native/engine_audio.gdextension", dest / "engine_audio.gdextension")
print(f"Installed {name}. Restart Godot to load it.")
