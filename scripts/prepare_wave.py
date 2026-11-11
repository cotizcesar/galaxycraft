#!/usr/bin/env python3
"""Prepare a ten-agent translation wave from the decompilation ledger.

Usage:
  # priority queue (recommended): next best rows from triage.tsv
  python3 scripts/triage_corpus.py
  python3 scripts/prepare_wave.py --wave 15 --queue engine-rust/docs/triage.tsv
  # legacy contiguous windows
  python3 scripts/prepare_wave.py --wave 15 --start 0x684000

In queue mode each agent receives an explicit address list (grouped by the
suggested Rust module and sorted by address) and `summary.json` records the
lists, so `validate_wave.py` and `merge_wave.py --map auto` need no window
arguments.

The script reads `engine-rust/docs/decomp-ledger.tsv` and creates one
assignment table per agent, a placeholder Rust batch and an isolated
integration-test harness:

  /tmp/opencode/waveNN/agentNN.md
  engine-rust/src/waveNN/agentNN.rs
  engine-rust/tests/waveNN_agentNN.rs

The placeholders are temporary.  A wave is finished only after
`scripts/merge_wave.py` has moved every batch into a natural module and the
staging directory and harnesses have been deleted.
"""
from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEDGER = ROOT / "engine-rust/docs/decomp-ledger.tsv"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--wave", type=int, required=True)
    parser.add_argument("--start", type=lambda s: int(s, 0))
    parser.add_argument("--queue", type=Path,
                        help="triage.tsv from scripts/triage_corpus.py")
    parser.add_argument("--per-agent", type=int, default=20)
    parser.add_argument("--max-tier", type=int, default=2)
    parser.add_argument("--count", type=int, default=10)
    parser.add_argument("--span", type=lambda s: int(s, 0), default=0x2000)
    parser.add_argument("--out", type=Path, default=Path("/tmp/opencode"))
    parser.add_argument(
        "--decomp",
        type=Path,
        default=Path.home() / ".cache/galaxycraft/decomp_full",
        help="decompiled-body directory; resolved by address so a moved home still works",
    )
    parser.add_argument("--force", action="store_true",
                        help="overwrite non-placeholder staging files")
    args = parser.parse_args()

    if args.start is None and args.queue is None:
        parser.error("pass --queue engine-rust/docs/triage.tsv or --start ADDR")
    rows = list(csv.DictReader(LEDGER.open(encoding="utf-8"), delimiter="\t"))
    by_addr = {int(r["address"], 16): r for r in rows}
    decomp = args.decomp.expanduser()

    def body_path(row: dict) -> str:
        if decomp.is_dir():
            matches = sorted(decomp.glob(f"*_{row['address'][2:]}_*.c"))
            if matches:
                return str(matches[0])
        return row["body"]
    wave_dir = args.out / f"wave{args.wave}"
    wave_dir.mkdir(parents=True, exist_ok=True)
    src = ROOT / "engine-rust/src" / f"wave{args.wave}"
    src.mkdir(parents=True, exist_ok=True)
    tests = ROOT / "engine-rust/tests"
    tests.mkdir(parents=True, exist_ok=True)

    plans = []  # (n, window_label, cand, skip, done, module, addresses|None)
    if args.queue:
        picked = []
        with args.queue.open(encoding="utf-8") as f:
            for q in csv.DictReader(f, delimiter="\t"):
                if q["category"] != "clean" or q["tier"] == "-":
                    continue
                if int(q["tier"]) > args.max_tier:
                    continue
                a = int(q["address"], 16)
                if by_addr.get(a, {}).get("status") != "decompiled_uncited":
                    continue
                picked.append((q["module"], a))
                if len(picked) >= args.count * args.per_agent:
                    break
        picked.sort()
        for i in range(args.count):
            chunk = picked[i * args.per_agent:(i + 1) * args.per_agent]
            if not chunk:
                break
            mods = {}
            for m, _ in chunk:
                mods[m] = mods.get(m, 0) + 1
            module = max(mods, key=mods.get)
            addrs = sorted(a for _, a in chunk)
            cand = [by_addr[a] for a in addrs]
            plans.append((i + 1, f"queue: {len(addrs)} addresses, module `{module}`",
                          cand, [], [], module, addrs))
    else:
        for i in range(args.count):
            a = args.start + i * args.span
            b = a + args.span
            sel = [r for r in rows if a <= int(r["address"], 16) < b]
            cand = [r for r in sel if r["status"] == "decompiled_uncited"]
            skip = [r for r in sel if r["status"] in ("decompile_failed", "thunk")]
            done = [r for r in sel if r["status"] == "translated"]
            plans.append((i + 1, f"`0x{a:08x}..0x{b - 1:08x}`", cand, skip, done,
                          None, None))

    summary = {"mode": "queue" if args.queue else "window", "agents": {}}
    for n, label, cand, skip, done, module, addrs in plans:
        lines = [
            f"# Wave {args.wave} agent {n:02d}",
            "",
            f"Window: {label}",
            "",
            *([f"Merge target: `src/{module}` (write code that fits there).", ""]
              if module else []),
            f"- Candidate `decompiled_uncited` rows: **{len(cand)}**",
            f"- Already translated in this window: {len(done)}",
            f"- Decompile-failed / thunk rows: {len(skip)}",
            "",
            "## Candidate rows (translate these)",
            "",
            "| address | name | size | decompiled body |",
            "|---|---|---|---|",
        ]
        for r in cand:
            lines.append(
                f"| `{r['address']}` | `{r['name']}` | {r['size']} | `{body_path(r)}` |"
            )
        if skip:
            lines += [
                "",
                "## Known-unrecoverable rows (skip; list the reason)",
                "",
                "| address | name | size | status |",
                "|---|---|---|---|",
            ]
            for r in skip:
                lines.append(
                    f"| `{r['address']}` | `{r['name']}` | {r['size']} | {r['status']} |"
                )
        (wave_dir / f"agent{n:02d}.md").write_text("\n".join(lines) + "\n")

        batch = src / f"agent{n:02d}.rs"
        if batch.exists() and "Replace this placeholder" not in batch.read_text(encoding="utf-8") and not args.force:
            print(f"refusing to overwrite {batch}; use --force")
            return 1
        batch.write_text(
            f"""//! Wave {args.wave} translation batch {n:02d} (staging).
//!
//! Window: {label}.
//! Replace this placeholder with the audited translations.  Use
//! `galaxycraft::` paths so the file compiles from the integration-test
//! harness.  Every implemented address must appear verbatim in a doc comment.
//! Skipped addresses must use `FUN_xxxxxxxx` or bare hex, never `0x`.
"""
        )
        harness = tests / f"wave{args.wave}_agent{n:02d}.rs"
        harness.write_text(
            f"""//! Temporary isolated harness for wave {args.wave} batch {n:02d}.
#[path = "../src/wave{args.wave}/agent{n:02d}.rs"]
mod batch;
"""
        )
        entry = {
            "window": label,
            "candidates": len(cand),
            "skip": len(skip),
            "done": len(done),
        }
        if addrs is not None:
            entry["module"] = module
            entry["addresses"] = [f"0x{a:08x}" for a in addrs]
        summary["agents"][f"agent{n:02d}"] = entry

    (wave_dir / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    for name, e in summary["agents"].items():
        print(f"{name}: {e['candidates']} candidates  {e['window']}")
    print(f"assignments: {wave_dir}")
    print("next: read engine-rust/docs/WAVE-AGENT-INSTRUCTIONS.md and launch one agent per assignment")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
