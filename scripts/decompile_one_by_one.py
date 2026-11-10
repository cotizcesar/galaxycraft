#!/usr/bin/env python3
"""Sequential recovery decompiler: one function per Ghidra process.

This is the low-impact path for the functions that failed the bulk pass.  It
runs a single headless Ghidra process at a time (``nice``d, low heap, two CPU
cores), writes a per-function log, and records every attempt in ``done.tsv`` so
a restart never repeats a function that already has a result.

Quality split:
  clean    decompiled with no control-flow corruption
  warning  decompiled but contains Ghidra's bad-instruction warning
  garbage  contains ``halt_baddata`` (not useful as pseudocode)
  failed   no file or the harness timed out

Usage:
  scripts/decompile_one_by_one.py \
    --project ~/galaxycraft-work/ghidra_full \
    --failures ~/.cache/galaxycraft/decomp_failures_current.tsv \
    --min-size 2000 --timeout 180 --max-instructions 1000000 --max-functions 40
"""
from __future__ import annotations

import argparse
import csv
import os
import re
import shutil
import signal
import subprocess
import time
from pathlib import Path

CACHE = Path.home() / ".cache/galaxycraft"
HEADLESS = Path("/opt/ghidra/support/analyzeHeadless")


def sanitize(name: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]", "_", name)


def load_rows(path: Path) -> list[dict]:
    with path.open(encoding="utf-8") as f:
        return list(csv.DictReader(f, delimiter="\t"))


def load_done(path: Path) -> set[str]:
    if not path.exists():
        return set()
    with path.open(encoding="utf-8") as f:
        return {row["address"] for row in csv.DictReader(f, delimiter="\t")}


def classify(path: Path) -> str:
    text = path.read_text(encoding="utf-8", errors="ignore")
    if "halt_baddata" in text:
        return "garbage"
    if "Control flow encountered bad instruction data" in text:
        return "warning"
    return "clean"


def rss_kb(pid: int) -> int:
    try:
        with open(f"/proc/{pid}/status", encoding="utf-8") as f:
            for line in f:
                if line.startswith("VmRSS:"):
                    return int(line.split()[1])
    except OSError:
        pass
    return 0


def group_pids(pgid: int) -> list[int]:
    """Every live process in the Popen session/process group."""
    out = []
    for entry in os.listdir("/proc"):
        if not entry.isdigit():
            continue
        pid = int(entry)
        try:
            if os.getpgid(pid) == pgid:
                out.append(pid)
        except OSError:
            pass
    return out


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--project", required=True, type=Path)
    parser.add_argument("--project-name", default="sc32")
    parser.add_argument("--program", default="sc32_runtime_pe.bin")
    parser.add_argument("--failures", required=True, type=Path)
    parser.add_argument("--out", type=Path, default=CACHE / "decomp_one_by_one")
    parser.add_argument("--timeout", type=int, default=60)
    parser.add_argument("--max-instructions", type=int, default=500_000)
    parser.add_argument("--small-size", type=int, default=2000,
                        help="functions below this size use the small budget")
    parser.add_argument("--small-timeout", type=int, default=15)
    parser.add_argument("--small-max-instructions", type=int, default=150_000)
    parser.add_argument("--max-payload", type=int, default=128)
    parser.add_argument("--max-functions", type=int, default=0)
    parser.add_argument("--min-size", type=int, default=0)
    parser.add_argument("--max-size", type=int, default=0)
    parser.add_argument("--maxmem", default="3G")
    parser.add_argument("--max-rss-gb", type=float, default=10.0,
                        help="kill the Ghidra process tree above this RSS")
    parser.add_argument("--nice", type=int, default=10)
    parser.add_argument("--script-path", type=Path,
                        default=Path(__file__).resolve().parent / "ghidra")
    args = parser.parse_args()

    for sub in ("clean", "warning", "garbage", "logs"):
        (args.out / sub).mkdir(parents=True, exist_ok=True)
    done_path = args.out / "done.tsv"
    done = load_done(done_path)
    rows = load_rows(args.failures)
    picked = []
    for row in rows:
        size = int(row.get("size", "0") or 0)
        if args.min_size and size < args.min_size:
            continue
        if args.max_size and size >= args.max_size:
            continue
        if row["address"].lower() in done:
            continue
        picked.append(row)
    picked.sort(key=lambda r: (-int(r.get("size", "0") or 0), r["address"]))
    if args.max_functions:
        picked = picked[: args.max_functions]
    print(f"sequential recovery: {len(picked)} functions, timeout={args.timeout}s, "
          f"maxInstructions={args.max_instructions}", flush=True)

    done_fields = ["address", "name", "size", "status", "seconds", "detail"]
    new_done = not done_path.exists()
    started_all = time.time()
    counts: dict[str, int] = {}
    with done_path.open("a", newline="", encoding="utf-8") as done_file:
        writer = csv.DictWriter(done_file, fieldnames=done_fields,
                                delimiter="\t", lineterminator="\n")
        if new_done:
            writer.writeheader()
        for i, row in enumerate(picked, 1):
            address = row["address"].lower()
            name = row["name"]
            size = row.get("size", "0")
            size_int = int(size or 0)
            if size_int < args.small_size:
                timeout = args.small_timeout
                max_instructions = args.small_max_instructions
            else:
                timeout = args.timeout
                max_instructions = args.max_instructions
            current = args.out / "current.tsv"
            current.write_text(f"{address}\t{name}\t{size}\n", encoding="utf-8")
            log = args.out / "logs" / f"{address}.log"
            cmd = [
                "nice", "-n", str(args.nice),
                str(HEADLESS), str(args.project), args.project_name,
                "-process", args.program, "-noanalysis", "-max-cpu", "2",
                "-scriptPath", str(args.script_path),
                "-postScript", "DecompileList.java", str(args.out),
                str(current), str(timeout),
                str(max_instructions), str(args.max_payload),
            ]
            env = dict(os.environ, GHIDRA_HEADLESS_MAXMEM=args.maxmem)
            started = time.time()
            stop = None
            with log.open("w", encoding="utf-8", errors="ignore") as lf:
                proc = subprocess.Popen(cmd, env=env, stdout=lf, stderr=subprocess.STDOUT,
                                        start_new_session=True)
                pgid = os.getpgid(proc.pid)
                cap_kb = int(args.max_rss_gb * 1024 * 1024)
                deadline = started + timeout + 240
                while True:
                    members = group_pids(pgid)
                    if not members:
                        break
                    if sum(rss_kb(pid) for pid in members) > cap_kb:
                        stop = f"memory cap {args.max_rss_gb:.0f} GB"
                        try:
                            os.killpg(pgid, signal.SIGKILL)
                        except OSError:
                            pass
                        proc.wait()
                        break
                    if time.time() > deadline:
                        stop = "harness timeout"
                        try:
                            os.killpg(pgid, signal.SIGKILL)
                        except OSError:
                            pass
                        proc.wait()
                        break
                    proc.poll()
                    time.sleep(1)
            elapsed = round(time.time() - started, 1)
            produced = args.out / f"{address}_{sanitize(name)}.c"
            if stop:
                status = "failed"
                detail = stop
                produced.unlink(missing_ok=True)
            elif produced.exists():
                status = classify(produced)
                detail = produced.name
                dest = args.out / status / produced.name
                shutil.move(str(produced), str(dest))
            else:
                status = "failed"
                detail = "no output; see logs"
            counts[status] = counts.get(status, 0) + 1
            writer.writerow({"address": address, "name": name, "size": size,
                             "status": status, "seconds": elapsed,
                             "detail": detail})
            done_file.flush()
            done.add(address)
            elapsed_all = time.time() - started_all
            eta = (elapsed_all / i) * (len(picked) - i) if i else 0
            print(f"[{i}/{len(picked)}] {address} {name} size={size} "
                  f"budget={timeout}s/{max_instructions} -> {status} "
                  f"({elapsed:.0f}s; eta {eta/60:.0f}m) counts={counts}", flush=True)
            time.sleep(1)
    print(f"finished {len(picked)} functions in {(time.time()-started_all)/60:.1f} min; "
          f"counts={counts}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
