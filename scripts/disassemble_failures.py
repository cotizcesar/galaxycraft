#!/usr/bin/env python3
"""Disassembly evidence for functions no decompiler could reconstruct.

Reads `decomp_failures_final.tsv`, disassembles the first instructions of each
function from the rebuilt runtime PE (file offset == RVA), and writes a TSV
with the provider outcomes plus the raw first bytes and mnemonics.  This is the
fallback record for functions whose control flow Ghidra, Hopper and
Rizin/rz-ghidra all refused.

Usage:
  scripts/disassemble_failures.py \
    --failures ~/.cache/galaxycraft/decomp_failures_final.tsv \
    --image ~/.cache/galaxycraft/sc32_runtime_pe.bin \
    --out ~/.cache/galaxycraft/decomp_failures_disassembly.tsv
"""
from __future__ import annotations

import argparse
import csv
from pathlib import Path

import capstone


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--failures", required=True, type=Path)
    parser.add_argument("--image", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--instructions", type=int, default=10)
    args = parser.parse_args()

    image = args.image.read_bytes()
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
    md.detail = False
    rows = []
    with args.failures.open(encoding="utf-8") as f:
        for row in csv.DictReader(f, delimiter="\t"):
            addr = int(row["address"], 16)
            size = int(row.get("size", "0") or 0)
            offset = addr - 0x00400000
            length = max(1, min(size or 16, 64))
            code = image[offset:offset + length] if 0 <= offset < len(image) else b""
            insns = []
            for insn in md.disasm(code, addr):
                insns.append(f"{insn.address:08x}: {insn.mnemonic} {insn.op_str}".rstrip())
                if len(insns) >= args.instructions:
                    break
            rows.append({
                "address": row["address"],
                "name": row["name"],
                "size": row.get("size", ""),
                "attempt_status": row.get("attempt_status", ""),
                "detail": row.get("detail", ""),
                "bytes": code.hex(),
                "instructions": " | ".join(insns) if insns else "UNDECODABLE",
            })
    with args.out.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=[
            "address", "name", "size", "attempt_status", "detail",
            "bytes", "instructions",
        ], delimiter="\t", lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    undecodable = sum(1 for row in rows if row["instructions"] == "UNDECODABLE")
    print(f"wrote {len(rows)} rows; undecodable first bytes: {undecodable}")
    for row in rows[:3]:
        print(row["address"], row["size"], row["instructions"][:160])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
