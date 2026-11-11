#!/usr/bin/env python3
"""Validate a prepared translation wave before merging.

Usage:
  python3 scripts/validate_wave.py --wave 15                 # queue mode
  python3 scripts/validate_wave.py --wave 15 --start 0x684000  # window mode

Queue mode reads the per-agent address lists from
`/tmp/opencode/waveNN/summary.json` written by `prepare_wave.py --queue`.

Checks:
  * every staging file exists and is not a placeholder;
  * every cited address belongs to that agent's window;
  * every cited address is present in the decompilation ledger;
  * no address is cited by two agents;
  * no address already marked `translated` is claimed again.
"""
from __future__ import annotations

import argparse
import csv
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ENGINE = ROOT / "engine-rust"
LEDGER = ENGINE / "docs/decomp-ledger.tsv"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--wave", type=int, required=True)
    parser.add_argument("--start", type=lambda s: int(s, 0))
    parser.add_argument("--out", type=Path, default=Path("/tmp/opencode"))
    parser.add_argument("--count", type=int, default=10)
    parser.add_argument("--span", type=lambda s: int(s, 0), default=0x2000)
    args = parser.parse_args()

    ledger = {}
    with LEDGER.open(encoding="utf-8") as f:
        for row in csv.DictReader(f, delimiter="\t"):
            ledger[int(row["address"], 16)] = row

    owned: dict[int, set[int]] = {}
    summary_path = args.out / f"wave{args.wave}/summary.json"
    if args.start is None:
        if not summary_path.exists():
            parser.error(f"no --start and no {summary_path}")
        summary = json.loads(summary_path.read_text())
        for name, entry in summary.get("agents", {}).items():
            if "addresses" in entry:
                owned[int(name[5:])] = {int(a, 16) for a in entry["addresses"]}
        args.count = len(summary.get("agents", {}))
        if not owned:
            parser.error(f"{summary_path} is a window-mode summary; pass --start")

    seen: dict[int, list[int]] = defaultdict(list)
    problems = 0
    stage = ENGINE / "src" / f"wave{args.wave}"
    for i in range(args.count):
        n = i + 1
        if owned:
            mine = owned.get(n, set())
            inside = mine.__contains__
        else:
            start = args.start + i * args.span
            end = start + args.span
            inside = lambda a, s=start, e=end: s <= a < e  # noqa: E731
        path = stage / f"agent{n:02d}.rs"
        if not path.exists():
            print(f"agent{n:02d}: MISSING")
            problems += 1
            continue
        text = path.read_text(encoding="utf-8")
        if "Replace this placeholder" in text:
            print(f"agent{n:02d}: placeholder")
            problems += 1
            continue
        addrs = sorted({
            int(a, 16)
            for a in re.findall(r"0x([0-9a-fA-F]{6,8})", text)
            if 0x00401000 <= int(a, 16) < 0x00D30000
        })
        window = [a for a in addrs if inside(a)]
        outside = [a for a in addrs if not inside(a)]
        unknown = [a for a in window if a not in ledger]
        already = [
            a for a in window
            if ledger.get(a, {}).get("status") == "translated"
        ]
        for a in window:
            seen[a].append(n)
        print(
            f"agent{n:02d}: {len(window)} addresses in window, "
            f"{len(outside)} outside, {len(unknown)} unknown, "
            f"{len(already)} already translated"
        )
        for a in outside[:5]:
            print(f"  outside: 0x{a:08x}")
        for a in unknown[:5]:
            print(f"  unknown: 0x{a:08x}")
        if outside or unknown:
            problems += 1

    duplicates = {a: v for a, v in seen.items() if len(v) > 1}
    if duplicates:
        problems += 1
        print(f"duplicate addresses across batches: {len(duplicates)}")
        for a, v in list(duplicates.items())[:10]:
            print(f"  0x{a:08x}: agents {v}")

    print(f"unique addresses cited: {len(seen)}")
    print(f"problems: {problems}")
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
