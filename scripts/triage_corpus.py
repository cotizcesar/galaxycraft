#!/usr/bin/env python3
"""Rank every `decompiled_uncited` function by translation value.

Sequential address windows waste most of a wave on code that has no value
for the Rust engine: compiler exception funclets, anti-tamper (obfuscated)
blocks, mid-function fragments, thunks and third-party libraries (OpenGL
loaders, FMOD, OpenSSL ...).  This script classifies the corpus once and
writes a priority queue that `prepare_wave.py --queue` consumes.

Usage:
  python3 scripts/triage_corpus.py
  python3 scripts/triage_corpus.py --summary      # print counts only

Output: `engine-rust/docs/triage.tsv` (addresses, sizes and labels only; no
pseudocode, so it is safe to commit).

Columns:
  address   0x-prefixed virtual address
  size      bytes
  category  clean | trivial | fragment | obfuscated | unwind | library | hub
  tier      1..4 for `clean` rows (1 = gameplay core), `-` otherwise
  module    suggested Rust module for the merge (from cited neighbours)
  score     gameplay-neighbour count used for ordering inside a tier

`hub` marks CRT/allocator/security-cookie helpers called from more than
HUB_CALLERS places (`__security_check_cookie`, `_alldiv`, `operator new`
...).  Rust already provides them; do not translate.

Tiers (clean rows only):
  1  calls or is called by a function already cited in a gameplay module
  2  two hops from a gameplay-module citation
  3  one hop from a client/render/audio/runtime citation
  4  everything else that decompiled cleanly (mostly engine/library glue)

Rerun after every ledger regeneration: freshly translated functions become
new seeds, so their neighbours rise to tier 1.
"""
from __future__ import annotations

import argparse
import collections
import csv
import os
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ENGINE = ROOT / "engine-rust"
LEDGER = ENGINE / "docs/decomp-ledger.tsv"
OUT = ENGINE / "docs/triage.tsv"

GAMEPLAY = {
    "sim.rs", "combat.rs", "orders.rs", "iscript.rs", "aiscript.rs",
    "ai_recovered.rs", "ai.rs", "path.rs", "contour.rs", "trigger.rs",
    "handlers.rs", "vision.rs", "map.rs", "data.rs", "replay.rs",
}
CLIENT = {
    "client.rs", "render.rs", "audio.rs", "buttons.rs", "button_sets.rs",
    "menu.rs", "runtime.rs", "profile.rs", "text.rs", "ui_art.rs",
}
REF = re.compile(r"\b(?:FUN|thunk_FUN|LAB|SUB)_([0-9a-f]{8})\b")
CITE = re.compile(r"0x([0-9a-fA-F]{6,8})\b")
LIB = re.compile(
    r'"(?:gl[A-Z]\w+|wgl\w+|egl\w+|FMOD|fmod\w*|SSL\w*|X509\w*|EVP_\w+|BIO_\w+|'
    r'curl\w*|png_\w+|inflate|deflate)|fmod|openssl|protobuf|libcurl',
    re.I,
)
HUB_CALLERS = 150
FRAGMENT = re.compile(r"\b(?:unaff_\w+|in_(?!ECX)\w+)")


def classify(name: str, size: int, text: str) -> str:
    if name.startswith(("Unwind@", "Catch@", "Catch_All@")):
        return "unwind"
    if ("bad instruction data" in text or "overlaps instruction" in text
            or "halt_baddata" in text):
        return "obfuscated"
    if name.startswith("thunk_") or size <= 8:
        return "trivial"
    if len(FRAGMENT.findall(text)) >= 3:
        return "fragment"
    if LIB.search(text):
        return "library"
    return "clean"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--decomp", type=Path,
                    default=Path.home() / ".cache/galaxycraft/decomp_full")
    ap.add_argument("--out", type=Path, default=OUT)
    ap.add_argument("--summary", action="store_true")
    args = ap.parse_args()

    rows = list(csv.DictReader(LEDGER.open(encoding="utf-8"), delimiter="\t"))
    status = {int(r["address"], 16): r for r in rows}
    files = {}
    for f in os.listdir(args.decomp):
        if f.endswith(".c"):
            parts = f.split("_")
            if len(parts) > 2:
                files[int(parts[1], 16)] = args.decomp / f

    # Which Rust module cites each translated address.
    module_of: dict[int, str] = {}
    for path in (ENGINE / "src").rglob("*.rs"):
        if re.search(r"/wave\d+/", str(path)):
            continue
        for m in CITE.findall(path.read_text(encoding="utf-8", errors="ignore")):
            a = int(m, 16)
            if status.get(a, {}).get("status") == "translated":
                module_of.setdefault(a, path.name)

    callees: dict[int, set[int]] = {}
    texts_cat: dict[int, str] = {}
    for a, r in status.items():
        p = files.get(a)
        if p is None:
            continue
        text = p.read_text(encoding="utf-8", errors="ignore")
        callees[a] = {int(x, 16) for x in REF.findall(text)} - {a}
        if r["status"] == "decompiled_uncited":
            texts_cat[a] = classify(r["name"], int(r["size"]), text)
    callers: dict[int, set[int]] = collections.defaultdict(set)
    for a, cs in callees.items():
        for c in cs:
            callers[c].add(a)

    def neighbours(a: int) -> set[int]:
        return callees.get(a, set()) | callers.get(a, set())

    game_seed = {a for a, m in module_of.items() if m in GAMEPLAY}
    client_seed = {a for a, m in module_of.items() if m in CLIENT}
    game1 = {n for a in game_seed for n in neighbours(a)}
    game2 = {n for a in game1 for n in neighbours(a)}
    client1 = {n for a in client_seed for n in neighbours(a)}

    out = []
    for a, cat in texts_cat.items():
        if cat == "clean" and len(callers.get(a, ())) > HUB_CALLERS:
            cat = "hub"
        tier = "-"
        score = 0
        if cat == "clean":
            nb = neighbours(a)
            score = len(nb & game_seed)
            if a in game1:
                tier = 1
            elif a in game2:
                tier = 2
            elif a in client1:
                tier = 3
            else:
                tier = 4
        votes = collections.Counter(module_of[n] for n in neighbours(a) if n in module_of)
        module = votes.most_common(1)[0][0] if votes else "runtime.rs"
        out.append((a, int(status[a]["size"]), cat, tier, module, score))

    order = {1: 0, 2: 1, 3: 2, 4: 3, "-": 9}
    out.sort(key=lambda t: (order[t[3]], -t[5], t[0]))

    cats = collections.Counter(t[2] for t in out)
    tiers = collections.Counter(t[3] for t in out if t[2] == "clean")
    print("categories:", dict(cats))
    print("clean tiers:", {k: tiers[k] for k in sorted(tiers)})
    if args.summary:
        return 0
    with args.out.open("w", encoding="utf-8") as f:
        f.write("address\tsize\tcategory\ttier\tmodule\tscore\n")
        for a, size, cat, tier, module, score in out:
            f.write(f"0x{a:08x}\t{size}\t{cat}\t{tier}\t{module}\t{score}\n")
    print(f"wrote {args.out} ({len(out)} rows)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
