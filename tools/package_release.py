#!/usr/bin/env python3
"""Package tracked files only; optionally add the built Windows native addon."""
import argparse
from pathlib import Path
import subprocess
import zipfile

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--windows', action='store_true')
args = parser.parse_args()
files = subprocess.check_output(['git', 'ls-files', '-z'], cwd=root).decode().split('\0')
dest = root / 'dist'
dest.mkdir(exist_ok=True)
name = 'hardpack-engine-audio-windows-x64.zip' if args.windows else 'hardpack-engine-audio-portable.zip'
extra = []
if args.windows:
    extra = ['addons/engine_audio_native/engine_audio.gdextension', 'addons/engine_audio_native/bin/engine_audio_native.dll']
    if not all((root / p).is_file() for p in extra):
        raise SystemExit('Build and install the Windows native extension first.')
with zipfile.ZipFile(dest / name, 'w', zipfile.ZIP_DEFLATED) as archive:
    for path in files + extra:
        if path:
            archive.write(root / path, 'hardpack-engine-audio/' + path)
print(dest / name)
