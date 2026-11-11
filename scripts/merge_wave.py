#!/usr/bin/env python3
"""Merge a prepared translation wave into natural Rust modules.

Usage:
  python3 scripts/merge_wave.py --wave 15 \
      --map 1=client.rs,2=client.rs,3=render.rs,4=runtime.rs,5=trigger.rs,\
6=audio.rs,7=runtime.rs,8=client.rs,9=client.rs,10=client.rs

The staging files are converted as follows:
  * leading `//!` doc comments become ordinary `//` comments;
  * inner attributes (`#![...]`) are dropped, because the batch becomes items
    inside an existing module;
  * `galaxycraft::` paths become `crate::`.

The script appends each batch under a `Merged from src/waveNN/agentNN.rs`
header.  With `--replace`, existing waveNN blocks at the end of each target
are removed first, which is how a batch is re-merged after a late correction.
"""
from __future__ import annotations

import argparse
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ENGINE = ROOT / "engine-rust"


def convert(text: str) -> str:
    out = []
    for line in text.splitlines():
        if line.startswith("//!"):
            out.append("//" + line[3:])
        elif line.startswith("#!["):
            continue
        else:
            out.append(line)
    return "\n".join(out).replace("galaxycraft::", "crate::").rstrip() + "\n"


def parse_map(value: str) -> dict[int, str]:
    mapping = {}
    for item in value.split(","):
        if not item.strip():
            continue
        number, target = item.split("=", 1)
        mapping[int(number)] = target.strip()
    return mapping


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--wave", type=int, required=True)
    parser.add_argument("--map", required=True)
    parser.add_argument("--keep", action="store_true",
                        help="keep staging files and harnesses")
    parser.add_argument("--replace", action="store_true",
                        help="remove existing wave blocks at the end of each target")
    args = parser.parse_args()
    mapping = parse_map(args.map)
    stage = ENGINE / "src" / f"wave{args.wave}"
    harness = ENGINE / "tests"

    batches = []
    placeholders = []
    for n in sorted(mapping):
        path = stage / f"agent{n:02d}.rs"
        if not path.exists():
            print(f"missing {path}")
            return 1
        text = path.read_text(encoding="utf-8")
        if "Replace this placeholder" in text:
            placeholders.append(n)
            continue
        body = convert(text)
        addresses = sorted({
            int(a, 16)
            for a in re.findall(r"0x([0-9a-fA-F]{6,8})", body)
            if 0x00401000 <= int(a, 16) < 0x00D30000
        })
        if not addresses:
            print(f"agent{n:02d}: no address citations; refusing to merge")
            return 1
        batches.append((n, mapping[n], body, len(addresses)))

    if not batches:
        print("nothing to merge")
        return 1

    for target in sorted({target for _, target, _, _ in batches}):
        path = ENGINE / "src" / target
        text = path.read_text(encoding="utf-8")
        selected = [b for b in batches if b[1] == target]
        if args.replace:
            first = f"// Merged from src/wave{args.wave}/agent{selected[0][0]:02d}.rs"
            idx = text.find(first)
            if idx != -1:
                cut = text.rfind(
                    "// ===========================================================================",
                    0,
                    idx,
                )
                if cut != -1:
                    text = text[:cut].rstrip() + "\n"
        with path.open("w", encoding="utf-8") as f:
            f.write(text)
            for n, _, body, _ in selected:
                f.write(
                    "\n// ===========================================================================\n"
                    f"// Merged from src/wave{args.wave}/agent{n:02d}.rs\n"
                    "// ===========================================================================\n"
                )
                f.write(body)
                f.write("\n")
        print(f"{target}: merged {', '.join(f'agent{n:02d}' for n, _, _, _ in selected)}")

    for n, target, _, count in batches:
        print(f"  agent{n:02d} -> {target}: {count} cited addresses")

    if placeholders:
        print(f"placeholders left untouched: {placeholders}")
        print("staging kept because some batches are still placeholders")
    elif args.keep:
        print("staging kept because --keep was passed")
    else:
        for path in stage.glob("agent*.rs"):
            path.unlink()
        stage.rmdir()
        for n in mapping:
            h = harness / f"wave{args.wave}_agent{n:02d}.rs"
            if h.exists():
                h.unlink()
        print("removed staging directory and harnesses")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
