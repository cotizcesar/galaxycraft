#!/usr/bin/env python3
"""Measure scrolling in the live client window with real input devices.

    python3 scripts/real_scroll.py mouse|pad|keys LOG [--float WxH] [sc_view args...]

Opens `sc_view --start-screen game --shot-map 4` (extra args are passed
on, e.g. `--sd` or `--shot-map 0`), focuses it (Hyprland), then scrolls
for ~30 s with a uinput virtual device the game sees as real hardware:

- `mouse`: a relative mouse pushed to every screen edge and corner;
- `pad`:   an Xbox 360 pad (045e:028e) with the right stick held each way;
- `keys`:  the arrow keys held through `wtype`.

`--float WxH` runs the client in a floating window of exactly that size
(Hyprland tiles windows otherwise); the mouse then goes to the window's
edges instead of the screen's.

The client writes one line per frame to LOG (`GC_FRAME_LOG`); the summary
splits frames with the camera moving from frames with it still.  Needs
write access to /dev/uinput and python-evdev.  It takes over the pointer
while it runs.
"""
import os
import statistics
import subprocess
import sys
import time

from evdev import AbsInfo, UInput, ecodes as e

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "engine-rust")


def summary(log: str) -> None:
    rows = [l.split() for l in open(log)]
    rows = [r for r in rows if r[-1] == "false"][10:]
    moving, still, prev = [], [], None
    for r in rows:
        cam = (int(r[1]), int(r[2]))
        (moving if prev and cam != prev else still).append(float(r[0]))
        prev = cam
    for name, v in (("scrolling", moving), ("still", still)):
        if not v:
            print(f"{name}: no frames")
            continue
        s = sorted(v)
        mean = statistics.mean(v)
        print(
            f"{name}: {len(v)} frames, mean {mean:.1f} ms ({1000 / mean:.0f} fps), "
            f"p99 {s[int(len(s) * 0.99)]:.1f} ms, max {s[-1]:.1f} ms, "
            f"over 20 ms {sum(x > 20 for x in v)}"
        )


def main() -> None:
    mode, log, extra = sys.argv[1], sys.argv[2], sys.argv[3:]
    float_size = None
    if extra[:1] == ["--float"]:
        float_size = tuple(int(v) for v in extra[1].split("x"))
        extra = ["--windowed", "--width", str(float_size[0]), "--height", str(float_size[1])] + extra[2:]
    install = os.environ.get("SC_INSTALL") or os.path.expanduser(
        "~/Games/battlenet/drive_c/Program Files (x86)/StarCraft"
    )
    ui = None
    if mode == "mouse":
        ui = UInput({e.EV_REL: [e.REL_X, e.REL_Y], e.EV_KEY: [e.BTN_LEFT, e.BTN_RIGHT]}, name="gc-test-mouse")
    elif mode == "pad":
        stick = AbsInfo(0, -32768, 32767, 16, 128, 0)
        trig = AbsInfo(0, 0, 255, 0, 0, 0)
        hat = AbsInfo(0, -1, 1, 0, 0, 0)
        keys = [e.BTN_SOUTH, e.BTN_EAST, e.BTN_NORTH, e.BTN_WEST, e.BTN_TL, e.BTN_TR,
                e.BTN_SELECT, e.BTN_START, e.BTN_MODE, e.BTN_THUMBL, e.BTN_THUMBR]
        axes = [(e.ABS_X, stick), (e.ABS_Y, stick), (e.ABS_RX, stick), (e.ABS_RY, stick),
                (e.ABS_Z, trig), (e.ABS_RZ, trig), (e.ABS_HAT0X, hat), (e.ABS_HAT0Y, hat)]
        ui = UInput({e.EV_KEY: keys, e.EV_ABS: axes}, name="Microsoft X-Box 360 pad",
                    vendor=0x045E, product=0x028E, version=0x110, bustype=e.BUS_USB)
        time.sleep(1)
    env = dict(os.environ, GC_FRAME_LOG=os.path.abspath(log))
    game = subprocess.Popen(
        ["./target/release/sc_view", "--install", install, "--start-screen", "game", "--shot-map", "4", *extra],
        cwd=ROOT, env=env, stdout=open(log + ".stderr", "w"), stderr=subprocess.STDOUT,
    )
    time.sleep(12)
    hypr = lambda cmd: subprocess.run(["hyprctl", "dispatch", cmd], capture_output=True)
    hypr(f"hl.dsp.focus({{ window = 'pid:{game.pid}' }})")
    rect = None
    if float_size:
        hypr("hl.dsp.window.float({ action = 'enable' })")
        time.sleep(0.5)
        hypr(f"hl.dsp.window.resize({{ x = {float_size[0]}, y = {float_size[1]}, exact = true }})")
        hypr("hl.dsp.window.center()")
        time.sleep(1)
        hypr(f"hl.dsp.focus({{ window = 'pid:{game.pid}' }})")
        time.sleep(1)
        out = subprocess.run(["hyprctl", "clients", "-j"], capture_output=True, text=True).stdout
        import json
        for c in json.loads(out):
            if c["pid"] == game.pid:
                rect = (c["at"][0], c["at"][1], c["size"][0], c["size"][1])
        print("window", rect)
        if rect:
            # Focus follows the mouse: keep the pointer on the game.
            hypr(f"hl.dsp.cursor.move({{ x = {rect[0] + rect[2] // 2}, y = {rect[1] + rect[3] // 2} }})")
            time.sleep(0.5)
    if mode == "mouse" and rect:
        x, y, w, h = rect
        edges = [(x + w // 2, y + h // 2), (x + w - 1, y + h // 2), (x + w // 2, y + h - 1),
                 (x, y + h // 2), (x + w // 2, y), (x + w - 1, y + h - 1), (x, y), (x + w // 2, y + h // 2)]
        for ex, ey in edges:
            hypr(f"hl.dsp.cursor.move({{ x = {ex}, y = {ey} }})")
            for d in (1, -1):
                ui.write(e.EV_REL, e.REL_X, d)
                ui.syn()
                time.sleep(0.02)
            time.sleep(5)
    elif mode == "mouse":
        for dx, dy in [(-6000, -6000), (1700, 700), (6000, 0), (-1700, 6000), (-6000, 0),
                       (1700, -6000), (6000, 6000), (-6000, -6000)]:
            for _ in range(20):
                ui.write(e.EV_REL, e.REL_X, dx // 20)
                ui.write(e.EV_REL, e.REL_Y, dy // 20)
                ui.syn()
                time.sleep(0.01)
            time.sleep(5)
    elif mode == "pad":
        for x, y in [(-32767, 0), (0, -32767), (32767, 0), (0, 32767), (-32767, -32767), (32767, 32767)]:
            ui.write(e.EV_ABS, e.ABS_RX, x)
            ui.write(e.EV_ABS, e.ABS_RY, y)
            ui.syn()
            time.sleep(5)
    else:
        for k in ["Left", "Up", "Right", "Down", "Left", "Up"]:
            subprocess.run(["wtype", "-P", k])
            time.sleep(5)
            subprocess.run(["wtype", "-p", k])
    game.terminate()
    game.wait()
    if ui:
        ui.close()
    summary(log)


if __name__ == "__main__":
    main()
