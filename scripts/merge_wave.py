#!/usr/bin/env python3
"""Merge a prepared translation wave into natural Rust modules.

Usage:
  python3 scripts/merge_wave.py --wave 15 \
      --map 1=client.rs,2=client.rs,3=render.rs,4=runtime.rs,5=trigger.rs,\
6=audio.rs,7=runtime.rs,8=client.rs,9=client.rs,10=client.rs

  # queue mode: each batch goes to the module chosen by prepare_wave.py
  python3 scripts/merge_wave.py --wave 15 --map auto

The staging files are converted as follows:
  * leading `//!` doc comments become ordinary `//` comments;
  * inner attributes (`#![...]`) are dropped, because the batch becomes items
    inside an existing module;
  * `galaxycraft::` paths become `crate::`.

A module split into a directory (`src/<module>/mod.rs`, see
`scripts/split_merged.py`) gets each batch as `src/<module>/wNN/aNN.rs`.
For a single-file module the script appends each batch under a `Merged from src/waveNN/agentNN.rs`
header.  With `--replace`, existing waveNN blocks at the end of each target
are removed first, which is how a batch is re-merged after a late correction.
"""
from __future__ import annotations

import argparse
import json
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


def add_line(path: Path, line: str, header: str = "") -> None:
    """Append `line` to a module list file, creating it, unless present."""
    text = path.read_text(encoding="utf-8") if path.exists() else header
    if line not in text.splitlines():
        text = text.rstrip("\n") + "\n" + line + "\n" if text else line + "\n"
        path.write_text(text, encoding="utf-8")


def write_split(module_dir: Path, wave: int, selected) -> None:
    """Modules split by `scripts/split_merged.py` (`src/<module>/mod.rs`):
    each batch becomes `src/<module>/wNN/aNN.rs` (under `translated/` for
    `sim`), listed in `wNN/mod.rs` and re-exported."""
    # `sim` keeps its hand-written code in `mod.rs` and its batches under
    # `translated/`; the wave-only modules hold them at the top.
    base = module_dir / "translated" if module_dir.name == "sim" else module_dir
    depth = "super::super::super::*" if base != module_dir else "super::super::*"
    group = f"w{wave:02d}"
    gdir = base / group
    gdir.mkdir(parents=True, exist_ok=True)
    for n, _, body, _ in selected:
        (gdir / f"a{n:02d}.rs").write_text(
            "#![allow(unused_imports)]\n"
            f"use {depth};\n\n"
            "// ===========================================================================\n"
            f"// Merged from src/wave{wave}/agent{n:02d}.rs\n"
            "// ===========================================================================\n"
            + body + "\n",
            encoding="utf-8",
        )
        add_line(gdir / "mod.rs", f"mod a{n:02d};", "#![allow(unused_imports)]\n")
        add_line(gdir / "mod.rs", f"pub use a{n:02d}::*;")
    add_line(base / "mod.rs", f"mod {group};")
    add_line(base / "mod.rs", f"pub use {group}::*;")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--wave", type=int, required=True)
    parser.add_argument("--map", required=True)
    parser.add_argument("--keep", action="store_true",
                        help="keep staging files and harnesses")
    parser.add_argument("--replace", action="store_true",
                        help="remove existing wave blocks at the end of each target")
    args = parser.parse_args()
    if args.map == "auto":
        summary = json.loads(
            (Path("/tmp/opencode") / f"wave{args.wave}/summary.json").read_text()
        )
        mapping = {
            int(name[5:]): entry["module"]
            for name, entry in summary["agents"].items()
            if "module" in entry
        }
        if not mapping:
            print("summary.json has no modules; pass an explicit --map")
            return 1
    else:
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
        module_dir = ENGINE / "src" / Path(target).stem
        if (module_dir / "mod.rs").exists():
            selected = [b for b in batches if b[1] == target]
            write_split(module_dir, args.wave, selected)
            print(f"{target}: merged {', '.join(f'agent{n:02d}' for n, _, _, _ in selected)}")
            continue
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
