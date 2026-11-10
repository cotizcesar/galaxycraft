#!/usr/bin/env python3
"""Diff retail function inventory against addresses cited by the reconstruction.

`functions.tsv` is a Ghidra dump with `address<TAB>name<TAB>size<TAB>thunk`.
`--source-root engine-rust` extracts every 0x7a...... runtime address and every
0x004...... PE address from Rust/Markdown/test sources, normalises the runtime
addresses to the PE image base, and reports which addresses fall inside a
discovered function body.

The historical evidence ledger records the 32-bit image mapped at runtime base
`0x79fa0000`.  The current Wine capture maps the same module at `0x400000`, so
the normalisation delta is `0x79ba0000` (not `0x79c00000`).  The delta was
checked against the acquisition routine: `0x7a150d80 -> 0x5b0d80`, whose
operands reference the runtime tables at `0x7ab2f580 -> 0xf8f580` etc.
"""
from __future__ import annotations

import argparse
import bisect
import json
import re
from pathlib import Path

RUNTIME_BASE = 0x79FA0000
PE_BASE = 0x00400000


def normalise(addr: int) -> int:
    if RUNTIME_BASE <= addr < RUNTIME_BASE + 0x1000000:
        return addr - (RUNTIME_BASE - PE_BASE)
    return addr


def load_functions(path: Path):
    rows = []
    with path.open(encoding="utf-8") as f:
        for line in f:
            if line.startswith("address\t"):
                continue
            fields = line.rstrip("\n").split("\t")
            if len(fields) < 4:
                continue
            rows.append((int(fields[0], 16), fields[1], int(fields[2]), fields[3] == "1"))
    rows.sort()
    return rows


def cited_addresses(root: Path):
    addrs = {}
    for path in sorted(root.rglob("*")):
        if path.suffix not in {".rs", ".md"}:
            continue
        try:
            text = path.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        for match in re.finditer(r"0x([0-9a-fA-F]{6,8})\b", text):
            addr = normalise(int(match.group(1), 16))
            if 0x00401000 <= addr < 0x00D30000:
                addrs.setdefault(addr, set()).add(str(path))
    return addrs


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--functions", required=True, type=Path)
    parser.add_argument("--source-root", type=Path, default=Path("engine-rust"))
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    rows = load_functions(args.functions)
    starts = [row[0] for row in rows]
    cited = cited_addresses(args.source_root)
    covered = set()
    missing = []
    for addr in cited:
        i = bisect.bisect_right(starts, addr) - 1
        hit = None
        if i >= 0:
            start, _name, size, _thunk = rows[i]
            if start <= addr < start + max(size, 1):
                hit = start
        if hit is None:
            missing.append(addr)
        else:
            covered.add(hit)
    report = {
        "inventory_functions": len(rows),
        "inventory_thunks": sum(1 for row in rows if row[3]),
        "cited_addresses": len(cited),
        "cited_inside_discovered_body": len(cited) - len(missing),
        "cited_unique_functions": len(covered),
        "cited_outside_partial_inventory": len(missing),
        "coverage_of_cited_addresses": round(
            (len(cited) - len(missing)) / len(cited), 4
        )
        if cited
        else None,
    }
    if args.json:
        print(json.dumps(report, indent=2))
    else:
        for key, value in report.items():
            print(f"{key}: {value}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
