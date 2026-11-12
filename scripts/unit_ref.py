#!/usr/bin/env python3
"""Retail reference data for one unit, read from engine-rust/fixtures.

Prints what the unit-fidelity audits compare the engine against:

  unit_ref.py --unit 0            # Marine: stats, weapons, image chain, iscript
  unit_ref.py --iscript 312       # disassemble one iscript by id
  unit_ref.py --image 421         # images.dat row (iscript, draw function, remap)
  unit_ref.py --weapon 25         # one weapons.dat row
  unit_ref.py --sounds 'marine'   # sfxdata.tbl entries whose path matches a regex
  unit_ref.py --images-tbl PATH --find 'heal|cure'
                                  # images whose GRP path matches (images.tbl is
                                  # not a fixture: dump it with
                                  # `sc_dump --include arr/images.tbl`)

See engine-rust/docs/UNIT-FIDELITY-METHOD.md.
"""
import argparse
import csv
import re
import struct
from pathlib import Path

FIX = Path(__file__).resolve().parent.parent / "engine-rust" / "fixtures"
N_IMAGES = 999

OPLEN = [2, 2, 1, 1, 2, 1, 2, 2, 4, 4, 2, 2, 0, 4, 4, 4, 4, 4, 2, 4, 4, 3, 0, 1, 2, 0, 4, 0,
         0, 0, 3, 1, 1, 0, 1, 1, 1, 1, 0, 0, 1, 1, 0, 1, 1, 0, 0, 0, 0, 1, 0, 0, 1, 2, 0, 2,
         1, 2, 4, 6, 6, 2, 0, 2, 2, 1, 4, 0, 0]
OPNAME = ("playfram playframtile sethorpos setvertpos setpos wait waitrand goto imgol imgul "
          "imgolorig switchul __0c imgoluselo imguluselo sprol highsprol lowsprul uflunstable "
          "spruluselo sprul sproluselo end setflipstate playsnd playsndrand playsndbtwn "
          "domissiledmg attackmelee followmaingraphic randcondjmp turnccwise turncwise "
          "turn1cwise turnrand setspawnframe sigorder attackwith attack castspell useweapon "
          "move gotorepeatattk engframe engset __2d nobrkcodestart nobrkcodeend ignorerest "
          "attkshiftproj tmprmgraphicstart tmprmgraphicend setfldirect call return setflspeed "
          "creategasoverlays pwrupcondjmp trgtrangecondjmp trgtarccondjmp curdirectcondjmp "
          "imgulnextid __3e liftoffcondjmp warpoverlay orderdone grdsprol __43 "
          "dogrddamage").split()
ANIMS = ("Init Death GndAttkInit AirAttkInit Unused1 GndAttkRpt AirAttkRpt CastSpell "
         "GndAttkToIdle AirAttkToIdle Unused2 Walking WalkingToIdle SpecialState1 "
         "SpecialState2 AlmostBuilt Built Landing LiftOff IsWorking WorkingToIdle WarpIn "
         "Unused3 StarEditInit Disable Burrow UnBurrow Enable").split()
# Animation count by iscript header type.
ANIM_COUNT = {0: 2, 1: 2, 2: 4, 12: 14, 13: 14, 14: 16, 15: 16, 20: 22, 21: 22, 23: 24,
              24: 26, 26: 28, 27: 28, 28: 28, 29: 28}


def u16(buf, off):
    return struct.unpack_from("<H", buf, off)[0]


def u32(buf, off):
    return struct.unpack_from("<I", buf, off)[0]


def tsv_row(name, row_id):
    with open(FIX / name, newline="") as f:
        for row in csv.DictReader(f, delimiter="\t"):
            if int(row["id"]) == row_id:
                return row
    return None


def image_row(image):
    d = (FIX / "images.dat").read_bytes()
    n = N_IMAGES
    return {
        "grp": u32(d, 4 * image),
        "directional": d[4 * n + image],
        "draw_function": d[8 * n + image],
        "remap": d[9 * n + image],
        "iscript": u32(d, 10 * n + 4 * image),
    }


def unit_chain(unit):
    units = (FIX / "units.dat").read_bytes()
    flingy = units[unit]
    sprite = u16((FIX / "flingy.dat").read_bytes(), 2 * flingy)
    image = u16((FIX / "sprites.dat").read_bytes(), 2 * sprite)
    return flingy, sprite, image


def disassemble(script_id):
    d = (FIX / "iscript.bin").read_bytes()
    p = u16(d, 0)
    header = None
    while True:
        sid, off = struct.unpack_from("<HH", d, p)
        p += 4
        if sid == 0xFFFF:
            break
        if sid == script_id:
            header = off
    if header is None:
        print(f"iscript {script_id}: not found")
        return
    assert d[header:header + 4] == b"SCPE"
    kind = d[header + 4]
    count = ANIM_COUNT.get(kind, 16)
    offsets = struct.unpack_from(f"<{count}H", d, header + 8)
    print(f"iscript {script_id} (header type {kind})")
    seen = set()

    def walk(o):
        while o and o not in seen:
            seen.add(o)
            op = d[o]
            if op >= len(OPNAME):
                print(f"    {o:04x} ?? {op}")
                return
            if op in (0x19, 0x1C):
                n = d[o + 1]
                args = struct.unpack_from(f"<{n}H", d, o + 2)
                size = 2 + 2 * n
            else:
                length = OPLEN[op]
                raw = d[o + 1:o + 1 + length]
                size = 1 + length
                if op in (0x07, 0x35):
                    args = (f"0x{u16(raw, 0):04x}",)
                elif op == 0x1E:
                    args = (raw[0], f"0x{u16(raw, 1):04x}")
                elif length == 2:
                    args = (u16(raw, 0),)
                elif length == 4 and op in (8, 9, 13, 14, 15, 16, 17, 19, 20):
                    args = struct.unpack("<Hbb", raw)
                else:
                    args = tuple(raw)
            print(f"    {o:04x} {OPNAME[op]} {' '.join(map(str, args))}".rstrip())
            if op == 0x07:
                o = int(args[0], 16)
                print("      ->")
                continue
            if op in (0x1E, 0x35):
                walk(int(args[-1], 16))
            if op in (0x16, 0x36):
                return
            o += size
        if o in seen and o:
            print(f"      (continues at {o:04x}, shown above)")

    for k, off in enumerate(offsets):
        if off == 0:
            continue
        name = ANIMS[k] if k < len(ANIMS) else str(k)
        print(f"  {name} @ {off:04x}")
        walk(off)


def tbl_strings(path):
    t = Path(path).read_bytes()
    n = u16(t, 0)
    offs = struct.unpack_from(f"<{n}H", t, 2)
    return [t[o:t.index(b"\0", o)].decode("latin1") for o in offs]


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--unit", type=int)
    ap.add_argument("--iscript", type=int)
    ap.add_argument("--image", type=int)
    ap.add_argument("--weapon", type=int)
    ap.add_argument("--sounds")
    ap.add_argument("--images-tbl")
    ap.add_argument("--find")
    a = ap.parse_args()

    if a.unit is not None:
        row = tsv_row("oracle_units.tsv", a.unit)
        print(f"unit {a.unit}")
        for k, v in row.items():
            print(f"  {k}: {v}")
        flingy, sprite, image = unit_chain(a.unit)
        fl = tsv_row("oracle_flingy.tsv", flingy)
        print(f"  flingy {flingy}: {fl}")
        img = image_row(image)
        print(f"  sprite {sprite} -> image {image}: {img}")
        for key in ("ground_weapon", "air_weapon"):
            wid = int(row[key])
            if wid != 130:
                print(f"  {key} {wid}: {tsv_row('oracle_weapons.tsv', wid)}")
        disassemble(img["iscript"])
    if a.iscript is not None:
        disassemble(a.iscript)
    if a.image is not None:
        print(f"image {a.image}: {image_row(a.image)}")
    if a.weapon is not None:
        print(f"weapon {a.weapon}: {tsv_row('oracle_weapons.tsv', a.weapon)}")
    if a.sounds:
        names = tbl_strings(FIX / "sfxdata.tbl")
        d = (FIX / "sfxdata.dat").read_bytes()
        rx = re.compile(a.sounds, re.I)
        for i in range(len(d) // 4):
            g = u32(d, 4 * i)
            if 0 < g <= len(names) and rx.search(names[g - 1]):
                print(i, names[g - 1])
            if 4 * (i + 1) >= len(d) or i > 1200:
                break
    if a.images_tbl and a.find:
        names = tbl_strings(a.images_tbl)
        rx = re.compile(a.find, re.I)
        for image in range(N_IMAGES):
            row = image_row(image)
            g = row["grp"]
            if 0 < g <= len(names) and rx.search(names[g - 1]):
                print(image, names[g - 1], row)


if __name__ == "__main__":
    main()
