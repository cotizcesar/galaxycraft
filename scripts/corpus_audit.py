#!/usr/bin/env python3
"""Audit the corpus bookkeeping: every retail function must sit in exactly
one bucket (translated / thunk / decompile_failed / one triage category).

Prints a report (no pseudocode, no addresses) and exits non-zero when the
books do not balance.  Run after `decomp_ledger.py` + `triage_corpus.py`.

  python3 scripts/corpus_audit.py            # human report
  python3 scripts/corpus_audit.py --markdown # tables for CORPUS-STATUS.md
"""
from __future__ import annotations

import argparse
import collections
import csv
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "engine-rust/docs"
SRC = ROOT / "engine-rust/src"
FN = re.compile(r"^\s*pub(?:\([a-z]+\))? (?:const )?fn (w\d+a\d+_\w+)")
TESTMOD = re.compile(r"^mod tests_wave\d+_agent\d+|^mod tests\b|^#\[cfg\(test\)\]")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--markdown", action="store_true")
    args = ap.parse_args()

    ledger = list(csv.DictReader((DOCS / "decomp-ledger.tsv").open(), delimiter="\t"))
    triage = list(csv.DictReader((DOCS / "triage.tsv").open(), delimiter="\t"))
    skip = list(csv.DictReader((DOCS / "skiplist.tsv").open(), delimiter="\t"))
    status = {r["address"]: r for r in ledger}
    problems = []

    st = collections.Counter(r["status"] for r in ledger)
    tri = collections.Counter(r["category"] for r in triage)
    tiers = collections.Counter(r["tier"] for r in triage if r["category"] == "clean")

    # 1. books balance
    if len(status) != len(ledger):
        problems.append("duplicate addresses in the ledger")
    tri_addr = {r["address"] for r in triage}
    uncited = {a for a, r in status.items() if r["status"] == "decompiled_uncited"}
    if tri_addr != uncited:
        problems.append(f"triage != uncited: {len(tri_addr - uncited)} extra, "
                        f"{len(uncited - tri_addr)} missing (rerun triage_corpus.py)")
    # 2. skiplist hygiene
    skip_addr = [r["address"] for r in skip]
    dup = len(skip_addr) - len(set(skip_addr))
    stale = [a for a in set(skip_addr) if status.get(a, {}).get("status") == "translated"]
    unknown = [a for a in set(skip_addr) if a not in status]
    if dup:
        problems.append(f"{dup} duplicate skiplist rows")
    if unknown:
        problems.append(f"{len(unknown)} skiplist rows not in the inventory")
    # 3. evidence quality of translated rows
    code = docs_only = 0
    per_mod = collections.Counter()
    for r in ledger:
        if r["status"] != "translated":
            continue
        ev = [e for e in r["evidence"].split(";") if e]
        rs = [e for e in ev if e.endswith(".rs")]
        if rs:
            code += 1
            per_mod[Path(rs[0]).name] += 1
        else:
            docs_only += 1
    # 4. wiring: wave helpers referenced outside their definition and tests
    defined: dict[str, str] = {}
    nontest = collections.Counter()
    word = re.compile(r"\bw\d+a\d+_\w+")
    for p in sorted(SRC.rglob("*.rs")):
        in_test = False
        for line in p.read_text(encoding="utf-8", errors="ignore").splitlines():
            if TESTMOD.match(line):
                in_test = True
            elif in_test and line.startswith("}"):
                in_test = False
                continue
            m = FN.match(line)
            if m and not in_test:
                defined[m.group(1)] = p.name
                continue
            if not in_test:
                for w in word.findall(line):
                    nontest[w] += 1
    used = {f for f in defined if nontest[f] > 0}
    wired_mod = collections.Counter(defined[f] for f in used)
    def_mod = collections.Counter(defined.values())
    by_wave = collections.Counter(r["wave"] for r in skip)

    def table(title, rows, head):
        if args.markdown:
            print(f"\n### {title}\n\n| " + " | ".join(head) + " |\n|" + "---|" * len(head))
            for r in rows:
                print("| " + " | ".join(str(x) for x in r) + " |")
        else:
            print(f"\n== {title}")
            for r in rows:
                print("  " + "  ".join(str(x) for x in r))

    total = len(ledger)
    table("Ledger status", [(k, v, f"{100*v/total:.1f}%") for k, v in st.most_common()]
          + [("total", total, "100%")], ["status", "functions", "share"])
    table("Translated rows by evidence",
          [("cited in Rust code", code), ("cited only in docs (RE notes)", docs_only)],
          ["evidence", "functions"])
    table("Uncited rows by triage category",
          [(k, v) for k, v in tri.most_common()] + [("total", len(triage))],
          ["category", "rows"])
    table("Clean rows by tier", [(k, tiers[k]) for k in sorted(tiers)], ["tier", "rows"])
    table("Translated functions by module (first Rust citation)",
          per_mod.most_common(), ["module", "functions"])
    table("Wave helpers (`wNNaMM_*`) and wiring",
          [(m, def_mod[m], wired_mod[m]) for m, _ in def_mod.most_common()]
          + [("total", len(defined), len(used))],
          ["module", "helpers defined", "called from non-test code"])
    table("Skiplist", [("rows", len(skip_addr)), ("now translated (stale)", len(stale)),
                       ("waves", f"{min(by_wave, key=int)}-{max(by_wave, key=int)}")],
          ["item", "value"])
    print()
    for p in problems:
        print("PROBLEM:", p)
    print("books balance" if not problems else "books DO NOT balance")
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
