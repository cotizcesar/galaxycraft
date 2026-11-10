#!/usr/bin/env python3
"""Join Ghidra function inventory, decompiled pseudocode and evidence addresses.

Inputs:
  --functions   functions_full.tsv (address, name, size, thunk)
  --decomp      directory written by scripts/ghidra/DecompileAll.java
  --evidence    evidence_index.json from scripts/evidence_index.py (optional)

Outputs a JSON report with:
  * inventory and pseudocode completeness;
  * the function that contains every cited evidence address;
  * whether that function has a decompiled .c file.

This is the verification step for "the whole inventory was decompiled": a
cited address is only covered when its containing function has a pseudocode
file, not merely when it appears in the function list.
"""
from __future__ import annotations

import argparse
import bisect
import json
import re
from pathlib import Path

HEADER_RE = re.compile(r"^// ([0-9a-fA-F]+) (\S+) size=(\d+)")
FAILED_RE = re.compile(r"^// ([0-9a-fA-F]+) (\S+) DECOMPILE_FAILED")


def load_functions(path: Path):
    rows = []
    with path.open(encoding="utf-8") as f:
        for line in f:
            if line.startswith("address\t"):
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


def scan_decomp(root: Path):
    files = {}
    failed = 0
    for path in root.glob("*.c"):
        try:
            with path.open(encoding="utf-8", errors="ignore") as f:
                first = f.readline()
        except OSError:
            continue
        if FAILED_RE.match(first):
            failed += 1
            continue
        match = HEADER_RE.match(first)
        if not match:
            continue
        addr = int(match.group(1), 16)
        files[addr] = str(path)
    return files, failed


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--functions", required=True, type=Path)
    parser.add_argument("--decomp", required=True, type=Path)
    parser.add_argument("--evidence", type=Path)
    parser.add_argument("--json", type=Path)
    args = parser.parse_args()

    rows = load_functions(args.functions)
    starts = [r["address"] for r in rows]
    files, failed = scan_decomp(args.decomp)
    covered_rows = [r for r in rows if r["address"] in files]

    def containing(addr: int):
        i = bisect.bisect_right(starts, addr) - 1
        if i < 0:
            return None
        row = rows[i]
        if row["address"] <= addr < row["address"] + max(row["size"], 1):
            return row
        return None

    report = {
        "inventory_functions": len(rows),
        "inventory_thunks": sum(1 for r in rows if r["thunk"]),
        "decompiled_functions": len(covered_rows),
        "decompile_failed_markers": failed,
        "inventory_coverage": round(len(covered_rows) / len(rows), 4) if rows else None,
    }

    if args.evidence and args.evidence.exists():
        evidence = json.loads(args.evidence.read_text(encoding="utf-8"))
        cited = []
        missing = []
        seen = set()
        for entry in evidence.get("entries", []):
            if entry["kind"] != "code":
                continue
            addr = int(entry["pe_address"], 16)
            if addr in seen:
                continue
            seen.add(addr)
            row = containing(addr)
            if row is None:
                missing.append(entry)
                continue
            cited.append(
                {
                    "evidence_address": entry["pe_address"],
                    "function_address": f"0x{row['address']:x}",
                    "function_name": row["name"],
                    "decompiled": row["address"] in files,
                    "section": entry["section"],
                    "context": entry["context"],
                }
            )
        report["evidence_code_addresses"] = len(seen)
        report["evidence_inside_inventory"] = len(cited)
        report["evidence_outside_inventory"] = len(missing)
        report["evidence_with_pseudocode"] = sum(1 for c in cited if c["decompiled"])
        report["evidence_functions"] = sorted(
            {(c["function_address"], c["function_name"]) for c in cited}
        )
        report["evidence_missing"] = missing

    text = json.dumps(report, indent=2)
    if args.json:
        args.json.parent.mkdir(parents=True, exist_ok=True)
        args.json.write_text(text + "\n", encoding="utf-8")
    print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
