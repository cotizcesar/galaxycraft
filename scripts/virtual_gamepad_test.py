#!/usr/bin/env python3
"""End-to-end gamepad backend check through the Linux input stack.

The physical controller is ideal: plug it in and run

    python3 scripts/virtual_gamepad_test.py --physical

which only runs `sc_view --gamepad-probe 4` and reports what the real
gilrs/evdev backend sees.

When no controller is connected, the default mode creates a temporary
uinput gamepad (Xbox button/axis codes) and injects a short sequence:
left stick, A, X, RT+X, LB+A, LT+D-pad, D-pad, RB and the right stick.
That exercises the same kernel evdev -> gilrs -> `gamepad::map_state` path
as a physical pad; it is not a substitute for the final hardware acceptance
run.

Requires write access to /dev/uinput (usually a udev uaccess ACL) for the
virtual mode.  No Python packages beyond the standard library.
"""
from __future__ import annotations

import argparse
import fcntl
import os
import struct
import subprocess
import sys
import time

# linux/uinput.h
UI_SET_EVBIT = 0x40045564
UI_SET_KEYBIT = 0x40045565
UI_SET_ABSBIT = 0x40045567
UI_DEV_CREATE = 0x5501
UI_DEV_DESTROY = 0x5502

EV_SYN = 0
EV_KEY = 1
EV_ABS = 3

ABS_X, ABS_Y, ABS_RX, ABS_RY = 0, 1, 3, 4
ABS_HAT0X, ABS_HAT0Y = 16, 17

BTN_SOUTH = 0x130
BTN_EAST = 0x131
BTN_NORTH = 0x133
BTN_WEST = 0x134
BTN_TL = 0x136
BTN_TR = 0x137
BTN_TL2 = 0x138
BTN_TR2 = 0x139
BTN_SELECT = 0x13A
BTN_START = 0x13B
BTN_THUMBL = 0x13D
BTN_THUMBR = 0x13E
BTN_DPAD_UP = 0x220
BTN_DPAD_DOWN = 0x221
BTN_DPAD_LEFT = 0x222
BTN_DPAD_RIGHT = 0x223

KEYS = [
    BTN_SOUTH,
    BTN_EAST,
    BTN_NORTH,
    BTN_WEST,
    BTN_TL,
    BTN_TR,
    BTN_TL2,
    BTN_TR2,
    BTN_SELECT,
    BTN_START,
    BTN_THUMBL,
    BTN_THUMBR,
    BTN_DPAD_UP,
    BTN_DPAD_DOWN,
    BTN_DPAD_LEFT,
    BTN_DPAD_RIGHT,
]


class VirtualGamepad:
    def __init__(self) -> None:
        self.fd = os.open("/dev/uinput", os.O_WRONLY | os.O_NONBLOCK)
        fcntl.ioctl(self.fd, UI_SET_EVBIT, EV_KEY)
        fcntl.ioctl(self.fd, UI_SET_EVBIT, EV_ABS)
        for key in KEYS:
            fcntl.ioctl(self.fd, UI_SET_KEYBIT, key)
        for axis in (ABS_X, ABS_Y, ABS_RX, ABS_RY):
            fcntl.ioctl(self.fd, UI_SET_ABSBIT, axis)
        for axis in (ABS_HAT0X, ABS_HAT0Y):
            fcntl.ioctl(self.fd, UI_SET_ABSBIT, axis)
        name = b"GalaxyCraft Virtual Gamepad"
        # name[80], bustype/vendor/product/version (u16 x4), ff_effects_max,
        # absmax/min/fuzz/flat (i32 x64 each).
        dev = struct.pack(
            "80sHHHHI64i64i64i64i",
            name,
            0x0003,  # BUS_USB
            0x045E,  # Microsoft
            0x02FD,  # Xbox One S controller (wired)
            0x0111,
            0,
            *( [32767, 32767, 32767, 32767] + [1, 1] + [0] * 58 ),
            *( [-32768, -32768, -32768, -32768] + [-1, -1] + [0] * 58 ),
            *([0] * 64),
            *([0] * 64),
        )
        os.write(self.fd, dev)
        fcntl.ioctl(self.fd, UI_DEV_CREATE)
        time.sleep(0.6)  # let udev/gilrs enumerate it

    def emit(self, ev_type: int, code: int, value: int) -> None:
        now = time.time()
        sec = int(now)
        usec = int((now - sec) * 1_000_000)
        os.write(self.fd, struct.pack("@llHHi", sec, usec, ev_type, code, value))

    def sync(self) -> None:
        self.emit(EV_SYN, 0, 0)

    def close(self) -> None:
        try:
            fcntl.ioctl(self.fd, UI_DEV_DESTROY)
        finally:
            os.close(self.fd)


def find_binary(explicit: str | None) -> str:
    if explicit:
        return explicit
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    for rel in ("target/debug/sc_view", "target/release/sc_view"):
        path = os.path.join(root, "engine-rust", rel)
        if os.path.isfile(path) and os.access(path, os.X_OK):
            return path
    raise SystemExit("build sc_view first: (cd engine-rust && cargo build --bin sc_view)")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--physical", action="store_true", help="only probe connected hardware")
    ap.add_argument("--seconds", type=float, default=4.0)
    ap.add_argument("--binary")
    ap.add_argument("--keep", action="store_true", help="print the raw probe output")
    args = ap.parse_args()

    binary = find_binary(args.binary)
    pad = None if args.physical else VirtualGamepad()
    proc = None
    try:
        proc = subprocess.Popen(
            [binary, "--gamepad-probe", str(args.seconds)],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        if pad is not None:
            time.sleep(0.8)
            # Left stick right + up: virtual cursor moves right and up.
            pad.emit(EV_ABS, ABS_X, 32000)
            pad.emit(EV_ABS, ABS_Y, -32000)
            pad.sync()
            time.sleep(0.25)
            pad.emit(EV_ABS, ABS_X, 0)
            pad.emit(EV_ABS, ABS_Y, 0)
            pad.sync()
            # A (South) press: left click / Enter in menus.
            pad.emit(EV_KEY, BTN_SOUTH, 1)
            pad.sync()
            time.sleep(0.15)
            pad.emit(EV_KEY, BTN_SOUTH, 0)
            pad.sync()
            # X (West): in-game smart order.
            pad.emit(EV_KEY, BTN_WEST, 1)
            pad.sync()
            time.sleep(0.15)
            pad.emit(EV_KEY, BTN_WEST, 0)
            pad.sync()
            time.sleep(0.1)
            # RT + X: command-card slot 0.
            pad.emit(EV_KEY, BTN_TR2, 1)
            pad.emit(EV_KEY, BTN_WEST, 1)
            pad.sync()
            time.sleep(0.2)
            pad.emit(EV_KEY, BTN_WEST, 0)
            pad.emit(EV_KEY, BTN_TR2, 0)
            pad.sync()
            time.sleep(0.1)
            # LB + A: group 5.
            pad.emit(EV_KEY, BTN_TL, 1)
            pad.emit(EV_KEY, BTN_SOUTH, 1)
            pad.sync()
            time.sleep(0.2)
            pad.emit(EV_KEY, BTN_SOUTH, 0)
            pad.emit(EV_KEY, BTN_TL, 0)
            pad.sync()
            time.sleep(0.1)
            # LT + D-pad left: save camera 2.
            pad.emit(EV_KEY, BTN_TL2, 1)
            pad.emit(EV_KEY, BTN_DPAD_LEFT, 1)
            pad.sync()
            time.sleep(0.2)
            pad.emit(EV_KEY, BTN_DPAD_LEFT, 0)
            pad.emit(EV_KEY, BTN_TL2, 0)
            pad.sync()
            time.sleep(0.1)
            # D-pad left: restore camera 2.  Both the hat axis and the
            # digital key are sent so either kernel mapping reaches gilrs.
            pad.emit(EV_ABS, ABS_HAT0X, -1)
            pad.emit(EV_KEY, BTN_DPAD_LEFT, 1)
            pad.sync()
            time.sleep(0.3)
            pad.emit(EV_ABS, ABS_HAT0X, 0)
            pad.emit(EV_KEY, BTN_DPAD_LEFT, 0)
            pad.sync()
            time.sleep(0.1)
            # RB held: confine the cursor to the minimap.
            pad.emit(EV_KEY, BTN_TR, 1)
            pad.sync()
            time.sleep(0.25)
            pad.emit(EV_KEY, BTN_TR, 0)
            pad.sync()
            # Right stick right: camera pan.
            pad.emit(EV_ABS, ABS_RX, 32000)
            pad.sync()
            time.sleep(0.2)
            pad.emit(EV_ABS, ABS_RX, 0)
            pad.sync()
        output, _ = proc.communicate(timeout=args.seconds + 10)
    finally:
        if proc is not None and proc.poll() is None:
            proc.kill()
        if pad is not None:
            pad.close()

    if args.keep:
        print(output, end="")
    ok = proc.returncode == 0
    checks = {
        "active frames": "active frames" in output and "0 active frames" not in output,
    }
    if not args.physical:
        checks.update(
            {
                "menu Enter (A)": "menu Enter" in output,
                "primary A": "primary (A)" in output,
                "smart order X": "smart order (X)" in output,
                "command slot RT+X": "command slot 0" in output,
                "command layer RT": "command layer (RT)" in output,
                "group LB+A": "group 5 (LB)" in output,
                "save camera LT+D-pad": "save camera 2" in output,
                "D-pad camera": "camera 2 (D-pad)" in output,
                "minimap RB": "minimap cursor (RB)" in output,
                "south held": "south held" in output,
                "camera pan": "pan (" in output and "pan (0, 0)" not in output,
            }
        )
    for name, passed in checks.items():
        print(f"{'PASS' if passed else 'FAIL'} {name}")
    if not args.physical:
        print("virtual uinput pad: kernel evdev -> gilrs -> map_state path")
    else:
        print("physical probe: connect a controller and rerun without --physical")
    return 0 if ok and all(checks.values()) else 1


if __name__ == "__main__":
    sys.exit(main())
