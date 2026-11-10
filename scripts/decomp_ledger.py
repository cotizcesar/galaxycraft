#!/usr/bin/env python3
"""Build a per-function translation ledger for the decompiled retail corpus.

The ledger is deliberately conservative: a function is marked `translated`
only when it has a real decompiled body *and* its address is cited by the
Rust/markdown reconstruction sources.  A decompiled body with no citation is
`decompiled_uncited`; a missing/failed body is `decompile_failed`.

Usage:
  scripts/decomp_ledger.py \
    --functions ~/.cache/galaxycraft/functions_full.tsv \
    --decomp ~/.cache/galaxycraft/decomp_full \
    --source-root engine-rust \
    --tsv engine-rust/docs/decomp-ledger.tsv \
    --json ~/.cache/galaxycraft/decomp_ledger.json
"""
from __future__ import annotations

import argparse
import bisect
import json
import re
from collections import Counter
from pathlib import Path

RUNTIME_BASE = 0x79FA0000
PE_BASE = 0x00400000


def normalise(addr: int) -> int:
    if RUNTIME_BASE <= addr < RUNTIME_BASE + 0x1000000:
        return addr - (RUNTIME_BASE - PE_BASE)
    return addr


def load_inventory(path: Path):
    rows = []
    with path.open(encoding="utf-8") as f:
        for line in f:
            if line.startswith("address\t") or not line.strip():
                continue
            fields = line.rstrip("\n").split("\t")
            if len(fields) < 4:
                continue
            rows.append(
                {
                    "address": int(fields[0], 16),
                    "name": fields[1],
                    "size": int(fields[2]),
                    "thunk": fields[3] == "1",
                }
            )
    rows.sort(key=lambda r: r["address"])
    return rows


def load_bodies(root: Path):
    """address -> (path, ok, first_line)."""
    bodies = {}
    for path in root.glob("*.c"):
        name = path.name
        match = re.match(r"^\d+_([0-9a-fA-F]{8})_", name)
        if not match:
            continue
        addr = int(match.group(1), 16)
        try:
            first = path.open(encoding="utf-8", errors="ignore").readline()
        except OSError:
            first = ""
        ok = first.startswith("// ") and "DECOMPILE_FAILED" not in first
        bodies[addr] = (path, ok)
    return bodies


def cited_addresses(root: Path):
    cited = {}
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
                cited.setdefault(addr, set()).add(str(path))
    return cited


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--functions", required=True, type=Path)
    parser.add_argument("--decomp", required=True, type=Path)
    parser.add_argument("--source-root", type=Path, default=Path("engine-rust"))
    parser.add_argument("--tsv", type=Path, required=True)
    parser.add_argument("--json", type=Path)
    args = parser.parse_args()

    inventory = load_inventory(args.functions)
    bodies = load_bodies(args.decomp)
    cited = cited_addresses(args.source_root)
    starts = [row["address"] for row in inventory]

    rows = []
    counts: Counter[str] = Counter()
    for row in inventory:
        addr = row["address"]
        body = bodies.get(addr)
        body_ok = bool(body and body[1])
        evidence = cited.get(addr, set())
        if row["thunk"]:
            status = "thunk"
        elif not body_ok:
            status = "decompile_failed"
        elif evidence:
            status = "translated"
        else:
            status = "decompiled_uncited"
        counts[status] += 1
        rows.append(
            {
                "address": f"0x{addr:08x}",
                "name": row["name"],
                "size": row["size"],
                "thunk": int(row["thunk"]),
                "body": str(body[0]) if body else "",
                "evidence_count": len(evidence),
                "evidence": ";".join(sorted(evidence)),
                "status": status,
            }
        )

    args.tsv.parent.mkdir(parents=True, exist_ok=True)
    with args.tsv.open("w", encoding="utf-8") as f:
        f.write(
            "address\tname\tsize\tthunk\tbody\tevidence_count\tevidence\tstatus\n"
        )
        for row in rows:
            f.write(
                "{address}\t{name}\t{size}\t{thunk}\t{body}\t{evidence_count}\t"
                "{evidence}\t{status}\n".format(**row)
            )

    # Functions whose cited address lands inside a body but not at its start
    # are still evidence for that body; report both the exact-start count and
    # the containing-body count.
    containing = set()
    for addr in cited:
        i = bisect.bisect_right(starts, addr) - 1
        if i >= 0:
            start = starts[i]
            size = inventory[i]["size"]
            if start <= addr < start + max(size, 1):
                containing.add(start)

    summary = {
        "inventory_functions": len(inventory),
        "decompiled_ok": sum(1 for row in inventory if bodies.get(row["address"], (None, False))[1]),
        "decompile_failed": counts["decompile_failed"],
        "translated_exact_citation": counts["translated"],
        "decompiled_uncited": counts["decompiled_uncited"],
        "thunks": counts["thunk"],
        "cited_unique_functions_containing": len(containing),
        "status_counts": dict(counts),
        "tsv": str(args.tsv),
    }
    text = json.dumps(summary, indent=2)
    print(text)
    if args.json:
        args.json.write_text(text + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
