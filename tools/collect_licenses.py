#!/usr/bin/env python3
"""Collect dependency notices for native binary distribution (Cargo + Python)."""
import json
from pathlib import Path
import shutil
import subprocess
import urllib.request

root = Path(__file__).resolve().parents[1]
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--locked", "--manifest-path", str(root / "native/Cargo.toml"), "--format-version", "1"], text=True))
packages = {p['name']: p for p in metadata['packages']}
out = root / "licenses"
out.mkdir(exist_ok=True)
urllib.request.urlretrieve('https://www.mozilla.org/media/MPL/2.0/index.txt', out / 'MPL-2.0.txt')
index = ['# Native dependency licences', '', 'Generated from the locked Cargo dependency graph.', '', '| Package | Licence | Source |', '|---|---|---|']
for package in packages.values():
    if package['source'] is None:
        continue
    name, version = package['name'], package['version']
    folder = Path(package['manifest_path']).parent
    files = [p for p in folder.iterdir() if p.is_file() and p.name.lower().startswith(('license', 'copying', 'notice'))]
    if name == 'nanoserde-derive' and not files:
        files = [Path(packages['nanoserde']['manifest_path']).parent / 'LICENSE-MIT']
    if not files and package['license'] != 'MPL-2.0':
        raise SystemExit(f'Missing licence text for {name}; inspect before distributing.')
    target = out / 'dependencies' / f'{name}-{version}'
    target.mkdir(parents=True, exist_ok=True)
    for path in files:
        shutil.copy2(path, target / path.name)
    if package['license'] == 'MPL-2.0':
        shutil.copy2(out / 'MPL-2.0.txt', target / 'LICENSE')
    index.append(f'| {name} {version} | {package["license"]} | https://crates.io/crates/{name}/{version} |')
(out / 'README.md').write_text('\n'.join(index) + '\n', encoding='utf-8')
print('Collected dependency licence texts in licenses/.')
