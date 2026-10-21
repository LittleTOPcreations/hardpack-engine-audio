#!/usr/bin/env python3
"""Create a Godot bank resource from <rpm>.wav/.ogg files inside this project.

python tools/make_bank.py demo/audio/vulcan demo/banks/vulcan.tres --name 'Vulcan / five cylinders' --pops demo/audio/pops
Uses Python's standard library only. Run Godot's import once after adding audio.
"""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def resource(path):
    return "res://" + path.resolve().relative_to(ROOT).as_posix()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("folder", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--name", default="Engine")
    parser.add_argument("--idle", type=float, default=900)
    parser.add_argument("--redline", type=float, default=7000)
    parser.add_argument("--pops", type=Path)
    args = parser.parse_args()
    loops = sorted((int(p.stem), p) for p in args.folder.iterdir()
                   if p.suffix.lower() in (".wav", ".ogg") and p.stem.isdigit())
    if not loops or len({r for r, _ in loops}) != len(loops) or loops[0][0] <= 0:
        parser.error("Provide audio files with positive, unique RPM names, e.g. 900.wav and 3000.wav")
    if not 0 < args.idle < args.redline:
        parser.error("Use 0 < idle < redline")
    pops = sorted(p for p in args.pops.iterdir() if p.suffix.lower() in (".wav", ".ogg")) if args.pops else []
    lines = [f'[gd_resource type="Resource" script_class="EngineAudioBank" load_steps={len(loops) + len(pops) + 2} format=3]',
             '', '[ext_resource type="Script" path="res://addons/engine_audio/engine_audio_bank.gd" id="bank"]']
    for i, path in enumerate([p for _, p in loops] + pops):
        lines.append(f'[ext_resource type="AudioStream" path={json.dumps(resource(path))} id="s{i}"]')
    refs = lambda start, stop: ', '.join(f'ExtResource("s{i}")' for i in range(start, stop))
    lines += ['', '[resource]', 'script = ExtResource("bank")', f'label = {json.dumps(args.name)}',
              f'idle_rpm = {args.idle}', f'redline_rpm = {args.redline}',
              'rpms = PackedFloat32Array(' + ', '.join(str(r) for r, _ in loops) + ')',
              'loops = Array[AudioStream]([' + refs(0, len(loops)) + '])',
              'pops = Array[AudioStream]([' + refs(len(loops), len(loops) + len(pops)) + '])']
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text('\n'.join(lines) + '\n', encoding='utf-8')
    print(f'{args.output}: {len(loops)} loops, {len(pops)} pops')


if __name__ == '__main__':
    main()
