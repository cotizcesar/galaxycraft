#!/usr/bin/env python3
"""Query the decompiled retail corpus by address, function name or pseudocode.

Examples:
  # What retail function contains this address, and what does Ghidra think?
  python3 scripts/query_decomp.py --address 0x5b0d80

  # Every function whose name or pseudocode mentions a symbol/constant.
  python3 scripts/query_decomp.py --pattern "Unit order queue exceeded"

  # Functions in an address range, with their decompiled file paths.
  python3 scripts/query_decomp.py --range 0x74d400 0x74d600

Default paths are the persistent cache:
  ~/.cache/galaxycraft/functions_full.tsv
  ~/.cache/galaxycraft/decomp_full
"""
from __future__ import annotations

import argparse
import bisect
import re
from pathlib import Path

DEFAULT_FUNCTIONS = Path.home() / ".cache/galaxycraft/functions_full.tsv"
DEFAULT_DECOMP = Path.home() / ".cache/galaxycraft/decomp_full"
HEADER_RE = re.compile(r"^// ([0-9a-fA-F]+) (\S+) size=(\d+)")


def load(path: Path):
    rows = []
    with path.open(encoding="utf-8") as f:
        for line in f:
            if line.startswith("address\t"):
                continue
            fields = line.rstrip("\n").split("\t")
            if len(fields) < 4:
                continue
            rows.append((int(fields[0], 16), fields[1], int(fields[2])))
    rows.sort()
    return rows


def decomp_path(root: Path, addr: int) -> Path | None:
    for pattern in (f"*_{addr:08x}_*.c", f"*_{addr:x}_*.c"):
        for path in root.glob(pattern):
            try:
                first = path.open(encoding="utf-8", errors="ignore").readline()
            except OSError:
                continue
            if "DECOMPILE_FAILED" not in first:
                return path
    return None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--functions", type=Path, default=DEFAULT_FUNCTIONS)
    parser.add_argument("--decomp", type=Path, default=DEFAULT_DECOMP)
    parser.add_argument("--address", type=lambda s: int(s, 0))
    parser.add_argument("--range", nargs=2, type=lambda s: int(s, 0), metavar=("START", "END"))
    parser.add_argument("--name", help="regex on the Ghidra function name")
    parser.add_argument("--pattern", help="regex on decompiled pseudocode")
    parser.add_argument("--limit", type=int, default=20)
    args = parser.parse_args()

    rows = load(args.functions)
    starts = [r[0] for r in rows]

    def containing(addr: int):
        i = bisect.bisect_right(starts, addr) - 1
        if i < 0:
            return None
        row = rows[i]
        if row[0] <= addr < row[0] + max(row[2], 1):
            return row
        return None

    if args.address is not None:
        row = containing(args.address)
        if row is None:
            print(f"no function contains 0x{args.address:x}")
            return 1
        path = decomp_path(args.decomp, row[0])
        print(f"0x{args.address:x} -> 0x{row[0]:x} {row[1]} size={row[2]}")
        if path:
            print(f"== {path}")
            print(path.read_text(encoding="utf-8", errors="ignore"))
        else:
            print(f"no pseudocode file under {args.decomp}")
        return 0

    if args.range:
        start, end = args.range
        picked = [r for r in rows if start <= r[0] < end]
        for addr, name, size in picked[: args.limit]:
            path = decomp_path(args.decomp, addr)
            print(f"0x{addr:x}\t{name}\t{size}\t{path or '-'}")
        print(f"{len(picked)} functions in range")
        return 0

    if args.name:
        regex = re.compile(args.name, re.I)
        picked = [r for r in rows if regex.search(r[1])]
        for addr, name, size in picked[: args.limit]:
            path = decomp_path(args.decomp, addr)
            print(f"0x{addr:x}\t{name}\t{size}\t{path or '-'}")
        print(f"{len(picked)} matching functions")
        return 0

    if args.pattern:
        regex = re.compile(args.pattern, re.I)
        hits = 0
        for path in sorted(args.decomp.glob("*.c")):
            text = path.read_text(encoding="utf-8", errors="ignore")
            if not regex.search(text):
                continue
            hits += 1
            first = text.splitlines()[0] if text else ""
            print(f"{path}\t{first}")
            if hits >= args.limit:
                break
        print(f"{hits} matching pseudocode files (limit {args.limit})")
        return 0

    parser.error("choose --address, --range, --name or --pattern")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
