#!/usr/bin/env python3
"""Merge clean sequential-recovery bodies over their failure markers.

The one-by-one driver writes `clean/`, `warning/` and `garbage/` buckets.
This script replaces the matching `DECOMPILE_FAILED` file in
`decomp_full` with the recovered body, preserving the inventory filename and
index.  Garbage (`halt_baddata`) is never merged.

Usage:
  scripts/merge_recovered.py \
    --recovered ~/.cache/galaxycraft/decomp_one_by_one \
    --decomp ~/.cache/galaxycraft/decomp_full
"""
from __future__ import annotations

import argparse
import re
from pathlib import Path

HEADER = re.compile(r"^// ([0-9a-fA-F]+) (\S+)")


def address_of(path: Path) -> str | None:
    with path.open(encoding="utf-8", errors="ignore") as f:
        first = f.readline()
    return first.split()[1].lower() if first.startswith("// ") and len(first.split()) > 1 else None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--recovered", required=True, type=Path)
    parser.add_argument("--decomp", required=True, type=Path)
    parser.add_argument("--include-warning", action="store_true",
                        help="also merge bodies with Ghidra's bad-data warning")
    args = parser.parse_args()

    buckets = [args.recovered / "clean"]
    if args.include_warning:
        buckets.append(args.recovered / "warning")
    merged = 0
    missing = 0
    for bucket in buckets:
        if not bucket.is_dir():
            continue
        for src in sorted(bucket.glob("*.c")):
            addr = address_of(src)
            if not addr:
                continue
            target = None
            for candidate in args.decomp.glob(f"*_{addr}_*.c"):
                first = candidate.open(encoding="utf-8", errors="ignore").readline()
                if "DECOMPILE_FAILED" in first:
                    target = candidate
                    break
            if target is None:
                missing += 1
                continue
            target.write_text(src.read_text(encoding="utf-8", errors="ignore"),
                              encoding="utf-8")
            merged += 1
    print(f"merged {merged} recovered bodies; {missing} had no failure marker")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
