#!/usr/bin/env python3
"""Measure trigger-text onsets in a retail mission recording.

The reference layouts below are fractions of the 3440x1440 content box
(console strip and the trigger-text strip above it), so 1280x720, 1920x1080
and ultrawide captures work without per-recording constants.

Usage:
    python3 scripts/recording_text_onsets.py RECORDING [--from S] [--duration S]

Then compare the printed onsets with `scripts/timeline_gate.py` or the
trigger cues written by `sc_view --game-script ticks:N`
(`$GC_WALK_DIR/cues.tsv`).
"""
from __future__ import annotations

import argparse
import subprocess
import sys

try:
    import numpy as np
except ImportError:
    print("numpy is required (pip install numpy)", file=sys.stderr)
    sys.exit(2)

REF_W, REF_H = 3440, 1440
FPS = 10


def probe_size(video: str) -> tuple[int, int]:
    out = subprocess.run(
        [
            "ffprobe",
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=s=x:p=0",
            video,
        ],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    w, h = out.split("x")
    return int(w), int(h)


def frames(video: str, t0: float, dur: float, box: tuple[int, int, int, int]) -> np.ndarray:
    x, y, w, h = box
    cmd = [
        "ffmpeg",
        "-loglevel",
        "error",
        "-ss",
        str(t0),
        "-t",
        str(dur),
        "-i",
        video,
        "-vf",
        f"fps={FPS},crop={w}:{h}:{x}:{y},format=rgb24",
        "-f",
        "rawvideo",
        "-",
    ]
    raw = subprocess.run(cmd, capture_output=True, check=True).stdout
    return np.frombuffer(raw, np.uint8).reshape(-1, h, w, 3).astype(int)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("video")
    ap.add_argument("--from", dest="t0", type=float, default=0.0)
    ap.add_argument("--duration", type=float, default=600.0)
    args = ap.parse_args()

    width, height = probe_size(args.video)
    # Console strip (lower middle) and trigger-text strip above it.
    console = (
        round(REF_W * 0.128),
        round(REF_H * 0.861),
        round(REF_W * 0.174),
        round(REF_H * 0.139),
    )
    text = (
        round(REF_W * 0.140),
        round(REF_H * 0.333),
        round(REF_W * 0.407),
        round(REF_H * 0.153),
    )
    con_box = tuple(round(v * (width / REF_W)) for v in console)
    text_box = tuple(round(v * (height / REF_H)) for v in text)
    con = frames(args.video, args.t0, args.duration, con_box).mean(axis=(1, 2, 3))
    txt = frames(args.video, args.t0, args.duration, text_box)
    lav = (
        (txt[..., 2] > 170) & (txt[..., 0] > 150) & (txt[..., 1] > 150)
    ).sum(axis=(1, 2))

    black = np.where(con < 2)[0]
    start_i = None
    for i in range(1, len(con)):
        if con[i - 1] < 2 and con[i] > 8:
            start_i = i
            break
    start = args.t0 + start_i / FPS if start_i is not None else None
    print(f"game start {start:.2f}s" if start is not None else "no start found")

    prev = 0
    for i, v in enumerate(lav):
        if start_i is not None and i < start_i:
            prev = v
            continue
        if (prev < 300 and v >= 800) or (v - prev > 1500):
            t = args.t0 + i / FPS
            rel = f" (+{t - start:.1f}s)" if start is not None else ""
            print(f"text onset {t:.1f}s{rel}  px={v}")
        prev = v
    return 0


if __name__ == "__main__":
    sys.exit(main())
