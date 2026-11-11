#!/usr/bin/env python3
"""Record the rows a merged wave assigned but did not translate.

Run after the ledger is regenerated:
  python3 scripts/record_skips.py --wave 15

Every address of the wave's assignment (`/tmp/opencode/waveW/summary.json`)
that is still `decompiled_uncited` is appended to
`engine-rust/docs/skiplist.tsv`, and `triage_corpus.py` never queues it
again.  The file holds addresses only (no pseudocode), so it is committed.
Delete a line to give a row another chance (for example after a helper it
depends on was translated).
"""
from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEDGER = ROOT / "engine-rust/docs/decomp-ledger.tsv"
SKIP = ROOT / "engine-rust/docs/skiplist.tsv"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--wave", type=int, required=True)
    ap.add_argument("--out", type=Path, default=Path("/tmp/opencode"))
    args = ap.parse_args()
    summary = json.loads((args.out / f"wave{args.wave}/summary.json").read_text())
    status = {r["address"]: r["status"] for r in
              csv.DictReader(LEDGER.open(encoding="utf-8"), delimiter="\t")}
    have = set()
    if SKIP.exists():
        have = {l.split("\t")[0] for l in SKIP.read_text().splitlines()[1:]}
    new = []
    for entry in summary["agents"].values():
        for a in entry.get("addresses", []):
            if status.get(a) == "decompiled_uncited" and a not in have:
                new.append(a)
    if not SKIP.exists():
        SKIP.write_text("address\twave\n")
    with SKIP.open("a", encoding="utf-8") as f:
        for a in sorted(new):
            f.write(f"{a}\t{args.wave}\n")
    print(f"skiplist: +{len(new)} (total {len(have) + len(new)})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
