# Third-party notices

## Engine Simulator — Ange Yaghi / AngeTheGreat

https://github.com/ange-yaghi/engine-sim

Hardpack Rally's older JavaScript synth credited Engine Simulator's audio approach. The optional Rust voice generator descends from that JavaScript synth. Shared techniques include pressure modulation with filtered noise, a derivative component, DC removal and exhaust convolution. Our simplified pulse generator and generated pipe response replace the upstream combustion model and supplied response samples. Credit is retained conservatively for that ancestry; no claim of an independent invention is made.

Copyright 2022 AngeTheGreat (Ange Yaghi)

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## Native extension dependencies

The optional native library uses **godot-rust 0.5.5**, MPL-2.0. No changes have been made to its source files. Exact dependency versions are recorded in `native/Cargo.lock`.

- Source: https://github.com/godot-rust/gdext/tree/v0.5.5
- Versioned source packages: https://crates.io/crates/godot/0.5.5 and the corresponding packages named in the lockfile.
- Licence text: `licenses/MPL-2.0.txt`.

The native release also contains Rust dependencies under MIT, Apache-2.0, BSD-2-Clause or Unicode-3.0. Their supplied licence texts are collected in `licenses/dependencies/` by `tools/collect_licenses.py`. This source archive and those versioned source packages provide the source corresponding to the distributed extension.

## Demo audio

Every WAV in `demo/audio` is generated from the included engine/pop generator with seed 7. No supplied game recordings, commercial sound packs, voices or music are included. These generated demonstration assets are distributed under this repository's MIT licence, with the ancestry notice above retained.
