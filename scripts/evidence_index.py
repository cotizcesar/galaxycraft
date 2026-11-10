#!/usr/bin/env python3
"""Index retail addresses and function names cited by historical evidence.

The pre-existing `starcraft-re-recomp` research tree records decompiled
behaviour as prose plus `0x7a......` addresses and `FUN_......` names.  Those
notes are the human-readable half of the Ghidra work; this script turns them
into a machine-readable index so every implementation task can start from a
retail address instead of a deterministic re-creation.

The historical 32-bit image was mapped at `0x79fa0000`; the current Wine
capture is mapped at `0x400000`.  The normalisation delta is `0x79ba0000`.
It was verified against the recovered acquisition routine
`0x7a150d80 -> 0x5b0d80`, whose operands point at the runtime tables
`0x7ab2f580 -> 0xf8f580`.

Usage:
    python3 scripts/evidence_index.py \
        --source ~/Documents/starcraft-re-recomp/engine-rust/docs/EVIDENCE.md \
        --source ~/Documents/starcraft-re-recomp/research/docs/session-log.md \
        --json ~/.cache/galaxycraft/evidence_index.json \
        --tsv ~/.cache/galaxycraft/evidence_index.tsv
"""
from __future__ import annotations

import argparse
import json
import re
from collections import Counter, OrderedDict
from pathlib import Path

RUNTIME_BASE = 0x79FA0000
PE_BASE = 0x00400000
CODE_START = 0x00401000
CODE_END = 0x00D30000

ADDR_RE = re.compile(r"0x([0-9a-fA-F]{6,8})\b")
FUN_RE = re.compile(r"FUN_([0-9a-fA-F]{6,10})\b")
HEADING_RE = re.compile(r"^#{1,6}\s+(.*\S)\s*$")

IMAGE_END = 0x010CE000


def normalise(value: int) -> int | None:
    """Map either historical convention to the current PE image base.

    `0x79fa0000`-based runtime addresses and `0x400000`-based menu-dump
    addresses both occur in the evidence tree.
    """
    if RUNTIME_BASE <= value < RUNTIME_BASE + 0x010CE000:
        return value - (RUNTIME_BASE - PE_BASE)
    if PE_BASE + 0x1000 <= value < IMAGE_END:
        return value
    return None


def kind_of(new: int) -> str:
    if CODE_START <= new < CODE_END:
        return "code"
    if new >= CODE_END:
        return "data"
    return "other"


def scan_source(path: Path) -> list[dict]:
    entries: "OrderedDict[tuple[int, str], dict]" = OrderedDict()
    section = ""
    text = path.read_text(encoding="utf-8", errors="ignore")
    for lineno, line in enumerate(text.splitlines(), 1):
        heading = HEADING_RE.match(line)
        if heading:
            section = heading.group(1)
            continue
        raw_addrs = {int(m.group(1), 16) for m in ADDR_RE.finditer(line)}
        raw_addrs |= {int(m.group(1), 16) for m in FUN_RE.finditer(line)}
        resolved = {raw: normalise(raw) for raw in raw_addrs}
        resolved = {raw: new for raw, new in resolved.items() if new is not None}
        if not resolved:
            continue
        context = " ".join(line.strip().split())[:400]
        for raw, new in sorted(resolved.items()):
            key = (new, section)
            entry = entries.get(key)
            if entry is None:
                entries[key] = {
                    "source": str(path),
                    "line": lineno,
                    "section": section,
                    "cited_address": f"0x{raw:x}",
                    "pe_address": f"0x{new:x}",
                    "kind": kind_of(new),
                    "context": context,
                }
    return list(entries.values())


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", action="append", required=True, type=Path)
    parser.add_argument("--json", type=Path)
    parser.add_argument("--tsv", type=Path)
    args = parser.parse_args()

    entries: list[dict] = []
    for source in args.source:
        found = scan_source(source)
        entries.extend(found)
        codes = sum(1 for e in found if e["kind"] == "code")
        print(f"{source}: {len(found)} entries ({codes} code, {len(found) - codes} data)")

    unique_code = {e["pe_address"] for e in entries if e["kind"] == "code"}
    unique_data = {e["pe_address"] for e in entries if e["kind"] == "data"}
    unique_contexts = Counter(e["section"] for e in entries)
    summary = {
        "runtime_base": f"0x{RUNTIME_BASE:x}",
        "pe_base": f"0x{PE_BASE:x}",
        "normalisation_delta": f"0x{RUNTIME_BASE - PE_BASE:x}",
        "entries": entries,
        "unique_code_addresses": sorted(unique_code),
        "unique_data_addresses": sorted(unique_data),
        "section_counts": dict(unique_contexts.most_common()),
    }
    print(
        f"total: {len(entries)} entries, {len(unique_code)} unique code addresses, "
        f"{len(unique_data)} unique data addresses"
    )

    if args.json:
        args.json.parent.mkdir(parents=True, exist_ok=True)
        args.json.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    if args.tsv:
        args.tsv.parent.mkdir(parents=True, exist_ok=True)
        with args.tsv.open("w", encoding="utf-8") as f:
            f.write("pe_address\tcited_address\tkind\tsource\tline\tsection\tcontext\n")
            for e in entries:
                f.write(
                    f"{e['pe_address']}\t{e['cited_address']}\t{e['kind']}\t"
                    f"{e['source']}\t{e['line']}\t{e['section']}\t{e['context']}\n"
                )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
