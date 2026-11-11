#!/usr/bin/env python3
"""Prepare a ten-agent translation wave from the decompilation ledger.

Usage:
  python3 scripts/prepare_wave.py --wave 15 --start 0x684000
  python3 scripts/prepare_wave.py --wave 15 --start 0x684000 --out /tmp/opencode

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
    parser.add_argument("--start", type=lambda s: int(s, 0), required=True)
    parser.add_argument("--count", type=int, default=10)
    parser.add_argument("--span", type=lambda s: int(s, 0), default=0x2000)
    parser.add_argument("--out", type=Path, default=Path("/tmp/opencode"))
    parser.add_argument("--force", action="store_true",
                        help="overwrite non-placeholder staging files")
    args = parser.parse_args()

    rows = list(csv.DictReader(LEDGER.open(encoding="utf-8"), delimiter="\t"))
    wave_dir = args.out / f"wave{args.wave}"
    wave_dir.mkdir(parents=True, exist_ok=True)
    src = ROOT / "engine-rust/src" / f"wave{args.wave}"
    src.mkdir(parents=True, exist_ok=True)
    tests = ROOT / "engine-rust/tests"
    tests.mkdir(parents=True, exist_ok=True)

    summary = {}
    for i in range(args.count):
        a = args.start + i * args.span
        b = a + args.span
        n = i + 1
        sel = [r for r in rows if a <= int(r["address"], 16) < b]
        cand = [r for r in sel if r["status"] == "decompiled_uncited"]
        skip = [r for r in sel if r["status"] in ("decompile_failed", "thunk")]
        done = [r for r in sel if r["status"] == "translated"]

        lines = [
            f"# Wave {args.wave} agent {n:02d}",
            "",
            f"Window: `0x{a:08x}..0x{b - 1:08x}`",
            "",
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
                f"| `{r['address']}` | `{r['name']}` | {r['size']} | `{r['body']}` |"
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
//! Window: `0x{a:08x}..0x{b - 1:08x}`.
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
        summary[f"agent{n:02d}"] = {
            "window": f"0x{a:08x}-0x{b - 1:08x}",
            "candidates": len(cand),
            "skip": len(skip),
            "done": len(done),
        }

    (wave_dir / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))
    print(f"assignments: {wave_dir}")
    print("next: read engine-rust/docs/WAVE-AGENT-INSTRUCTIONS.md and launch one agent per assignment")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
