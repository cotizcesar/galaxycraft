#!/usr/bin/env python3
"""Final decompilation status: corpus counts plus per-attempt outcomes.

Reads the inventory, the pseudocode corpus and the attempt manifests written
by `decompile_one_by_one.py` / `decompile_small_chunks.py`.

Usage:
  scripts/decomp_status.py \
    --functions ~/.cache/galaxycraft/functions_full.tsv \
    --decomp ~/.cache/galaxycraft/decomp_full \
    --attempt ~/.cache/galaxycraft/decomp_one_by_one \
    --attempt ~/.cache/galaxycraft/decomp_small \
    --json ~/.cache/galaxycraft/decomp_status.json
"""
from __future__ import annotations

import argparse
import csv
import json
from collections import Counter
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--functions", required=True, type=Path)
    parser.add_argument("--decomp", required=True, type=Path)
    parser.add_argument("--attempt", action="append", default=[], type=Path)
    parser.add_argument("--json", type=Path)
    args = parser.parse_args()

    inventory = 0
    with args.functions.open(encoding="utf-8") as f:
        for line in f:
            if line.startswith("address\t"):
                continue
            if line.strip():
                inventory += 1
    ok = 0
    failed_markers = 0
    for path in args.decomp.glob("*.c"):
        first = path.open(encoding="utf-8", errors="ignore").readline()
        if "DECOMPILE_FAILED" in first:
            failed_markers += 1
        elif first.startswith("// "):
            ok += 1

    attempts: Counter[str] = Counter()
    details: Counter[str] = Counter()
    for directory in args.attempt:
        manifest = directory / "done.tsv"
        if not manifest.exists():
            continue
        with manifest.open(encoding="utf-8") as f:
            for row in csv.DictReader(f, delimiter="\t"):
                attempts[row["status"]] += 1
                if row["status"] in {"failed", "garbage"}:
                    details[row.get("detail", "")] += 1

    report = {
        "inventory_functions": inventory,
        "decompiled_ok": ok,
        "failed_markers": failed_markers,
        "coverage": round(ok / inventory, 4) if inventory else None,
        "attempt_statuses": dict(attempts),
        "failed_details": dict(details.most_common(20)),
    }
    text = json.dumps(report, indent=2)
    print(text)
    if args.json:
        args.json.write_text(text + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
