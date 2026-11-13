#!/usr/bin/env python3
"""Census of campaign missions from their scenario.chk files.

    python3 scripts/campaign_census.py --cache ~/.cache/galaxycraft/retail-XXXX \\
        --episodes terran expterran > engine-rust/docs/CAMPAIGN-CENSUS.md

For every mission of the given episodes (in `campaign/manifest.tsv` order):
the seats (human / computers / rescuable / neutral), the forces and their
alliance flags, the AI scripts the triggers start (actions 15 / 16), the
trigger and briefing-trigger counts, the action kinds used and the mission
objectives text.  Read-only; nothing from the install is copied into the
output except the objective strings' first line.
"""
import argparse
import struct
from pathlib import Path

OWNR = {3: "rescuable", 5: "computer", 6: "human", 7: "neutral"}


def sections(raw: bytes) -> dict:
    out, i = {}, 0
    while i + 8 <= len(raw):
        tag = raw[i : i + 4]
        n = struct.unpack_from("<i", raw, i + 4)[0]
        out[tag] = out.get(tag, b"") + raw[i + 8 : i + 8 + max(n, 0)]
        i += 8 + max(n, 0)
    return out


def strings(sec: dict) -> list:
    s = sec.get(b"STRx") or sec.get(b"STR ", b"")
    if not s:
        return []
    wide = b"STRx" in sec
    count = struct.unpack_from("<I" if wide else "<H", s, 0)[0]
    step = 4 if wide else 2
    out = []
    for k in range(count):
        off = struct.unpack_from("<I" if wide else "<H", s, step * (k + 1))[0]
        end = s.find(b"\0", off)
        out.append(s[off:end].decode("latin-1", "replace"))
    return out


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cache", type=Path, required=True)
    ap.add_argument("--episodes", nargs="+", default=["terran", "expterran"])
    a = ap.parse_args()
    manifest = (a.cache / "campaign" / "manifest.tsv").read_text().splitlines()[1:]
    print("| # | mission | seats | forces (flags) | AI scripts | triggers | objectives |")
    print("|---|---|---|---|---|---|---|")
    for line in manifest:
        f = line.split("\t")
        if f[0] not in a.episodes:
            continue
        sec = sections((a.cache / "campaign" / f[6]).read_bytes())
        strs = strings(sec)
        owners = list(sec.get(b"OWNR", bytes(12))[:8])
        forc = sec.get(b"FORC", b"")
        forces = list(forc[:8]) if forc else [0] * 8
        flags = list(forc[16:20]) if len(forc) >= 20 else [0] * 4
        seats = ", ".join(f"P{p + 1} {OWNR[o]}" for p, o in enumerate(owners) if o in OWNR)
        used = sorted({forces[p] for p, o in enumerate(owners) if o in (5, 6)})
        force_txt = ", ".join(f"F{k + 1}{' allied' if flags[k] & 2 else ''}" for k in used)
        trig = sec.get(b"TRIG", b"")
        scripts, objectives = set(), []
        for t in range(len(trig) // 2400):
            for k in range(64):
                o = t * 2400 + 320 + k * 32
                kind = trig[o + 26]
                if kind in (15, 16):
                    scripts.add(trig[o + 20 : o + 24].decode("latin-1"))
                if kind == 12:
                    sid = struct.unpack_from("<I", trig, o + 4)[0]
                    if 0 < sid <= len(strs):
                        text = strs[sid - 1].replace("\r", " ").replace("\n", " ")
                        text = "".join(c for c in text if c >= " ")
                        objectives.append(text.strip())
        obj = objectives[0][:90] + ("…" if objectives and len(objectives[0]) > 90 else "") if objectives else ""
        print(
            f"| {f[2]} | {f[4]} | {seats} | {force_txt} | {', '.join(sorted(scripts)) or '-'} "
            f"| {len(trig) // 2400} | {obj.replace('|', '/')} |"
        )


if __name__ == "__main__":
    main()
