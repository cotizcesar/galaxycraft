#!/usr/bin/env python3
"""Sequential chunked recovery for the small bad-control-flow functions.

One Ghidra process at a time, one function after another inside that process
(`DecompileList.java`).  Each function gets its own short decompiler timeout;
the process group is watched and killed above the RSS cap.  This avoids one
Ghidra startup per tiny function while preserving the one-at-a-time rule.

Usage:
  scripts/decompile_small_chunks.py \
    --project ~/galaxycraft-work/ghidra_full \
    --failures ~/.cache/galaxycraft/decomp_failures_current.tsv \
    --out ~/.cache/galaxycraft/decomp_small \
    --max-size 1999 --timeout 3 --max-instructions 100000 \
    --max-rss-gb 8 --chunk-size 50
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


def classify(path: Path) -> str:
    text = path.read_text(encoding="utf-8", errors="ignore")
    if "halt_baddata" in text:
        return "garbage"
    if "Control flow encountered bad instruction data" in text:
        return "warning"
    return "clean"


def load_done(path: Path) -> set[str]:
    if not path.exists():
        return set()
    with path.open(encoding="utf-8") as f:
        return {row["address"] for row in csv.DictReader(f, delimiter="\t")}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--project", required=True, type=Path)
    parser.add_argument("--project-name", default="sc32")
    parser.add_argument("--program", default="sc32_runtime_pe.bin")
    parser.add_argument("--failures", required=True, type=Path)
    parser.add_argument("--out", type=Path, default=CACHE / "decomp_small")
    parser.add_argument("--max-size", type=int, default=1999)
    parser.add_argument("--timeout", type=int, default=3)
    parser.add_argument("--max-instructions", type=int, default=100_000)
    parser.add_argument("--max-payload", type=int, default=64)
    parser.add_argument("--chunk-size", type=int, default=50)
    parser.add_argument("--max-rss-gb", type=float, default=8.0)
    parser.add_argument("--maxmem", default="2G")
    parser.add_argument("--nice", type=int, default=10)
    parser.add_argument("--script-path", type=Path,
                        default=Path(__file__).resolve().parent / "ghidra")
    args = parser.parse_args()

    for sub in ("clean", "warning", "garbage", "logs"):
        (args.out / sub).mkdir(parents=True, exist_ok=True)
    done_path = args.out / "done.tsv"
    done = load_done(done_path)

    rows = []
    with args.failures.open(encoding="utf-8") as f:
        for row in csv.DictReader(f, delimiter="\t"):
            size = int(row.get("size", "0") or 0)
            if size > args.max_size:
                continue
            if row["address"].lower() in done:
                continue
            rows.append(row)
    rows.sort(key=lambda r: (-int(r.get("size", "0") or 0), r["address"]))
    print(f"small recovery: {len(rows)} functions in chunks of {args.chunk_size}", flush=True)

    fields = ["address", "name", "size", "status", "detail"]
    new_done = not done_path.exists()
    counts: dict[str, int] = {}
    started_all = time.time()
    with done_path.open("a", newline="", encoding="utf-8") as done_file:
        writer = csv.DictWriter(done_file, fieldnames=fields, delimiter="\t",
                                lineterminator="\n")
        if new_done:
            writer.writeheader()
        for start in range(0, len(rows), args.chunk_size):
            chunk = rows[start:start + args.chunk_size]
            n = start // args.chunk_size
            list_path = args.out / f"chunk-{n:03}.tsv"
            with list_path.open("w", encoding="utf-8") as f:
                f.write("address\tname\tsize\n")
                for row in chunk:
                    f.write(f"{row['address'].lower()}\t{row['name']}\t{row.get('size','0')}\n")
            log = args.out / "logs" / f"chunk-{n:03}.log"
            cmd = [
                "nice", "-n", str(args.nice),
                str(HEADLESS), str(args.project), args.project_name,
                "-process", args.program, "-noanalysis", "-max-cpu", "2",
                "-scriptPath", str(args.script_path),
                "-postScript", "DecompileList.java", str(args.out),
                str(list_path), str(args.timeout),
                str(args.max_instructions), str(args.max_payload),
            ]
            env = dict(os.environ, GHIDRA_HEADLESS_MAXMEM=args.maxmem)
            cap_kb = int(args.max_rss_gb * 1024 * 1024)
            started = time.time()
            stop = None
            with log.open("w", encoding="utf-8", errors="ignore") as lf:
                proc = subprocess.Popen(cmd, env=env, stdout=lf, stderr=subprocess.STDOUT,
                                        start_new_session=True)
                pgid = os.getpgid(proc.pid)
                deadline = started + len(chunk) * (args.timeout + 8) + 240
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
                        stop = "chunk timeout"
                        try:
                            os.killpg(pgid, signal.SIGKILL)
                        except OSError:
                            pass
                        proc.wait()
                        break
                    proc.poll()
                    time.sleep(1)
            elapsed = round(time.time() - started, 1)
            for row in chunk:
                addr = row["address"].lower()
                produced = args.out / f"{addr}_{sanitize(row['name'])}.c"
                if produced.exists():
                    status = classify(produced)
                    shutil.move(str(produced), str(args.out / status / produced.name))
                    detail = produced.name
                else:
                    status = "failed"
                    detail = stop or "no output"
                counts[status] = counts.get(status, 0) + 1
                writer.writerow({"address": addr, "name": row["name"],
                                 "size": row.get("size", "0"), "status": status,
                                 "detail": detail})
                done.add(addr)
            done_file.flush()
            total_done = start + len(chunk)
            print(f"chunk {n}: {total_done}/{len(rows)} done in {elapsed:.0f}s "
                  f"({stop or 'ok'}); counts={counts}", flush=True)
    print(f"finished {len(rows)} small functions in "
          f"{(time.time()-started_all)/60:.1f} min; counts={counts}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
