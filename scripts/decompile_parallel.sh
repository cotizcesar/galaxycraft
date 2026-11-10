#!/usr/bin/env bash
# Decompile the whole inventory with several Ghidra workers.
#
# Each worker gets a private copy of the analysed project (Ghidra locks a local
# project exclusively) and a contiguous index range balanced by total function
# bytes.  Output files are named by global function index, so workers never
# collide and a rerun skips functions that already have a successful .c file.
#
# Usage:
#   scripts/decompile_parallel.sh <base-project-dir> [project-name] \
#       [program-name] [output-dir] [jobs] [timeout-seconds] [functions.tsv]
#
# Ghidra refuses project paths containing a hidden element, so the base
# project and the worker copies must live under a visible directory such as
# ~/galaxycraft-work/.
#
# Worker count is capped to the host's RAM (8 GiB per worker by default) so a
# batch cannot fill swap and trigger systemd-oomd, which kills the whole
# terminal tab and takes fish down with it.  Override the budget with
# DECOMPILE_MEM_PER_JOB_GIB if needed.
set -euo pipefail

BASE_PROJECT="${1:?Ghidra project directory}"
PROJECT_NAME="${2:-sc32}"
PROGRAM="${3:-sc32_runtime_pe.bin}"
OUT="${4:-$HOME/.cache/galaxycraft/decomp_full}"
TIMEOUT="${6:-1}"
FUNCTIONS="${7:-$HOME/.cache/galaxycraft/functions_full.tsv}"
HERE="$(cd "$(dirname "$0")" && pwd)"

MEM_PER_JOB_GIB="${DECOMPILE_MEM_PER_JOB_GIB:-8}"
mem_gib=$(( $(awk '/^MemTotal:/ {print $2}' /proc/meminfo) / 1024 / 1024 ))
max_jobs=$(( mem_gib / MEM_PER_JOB_GIB ))
(( max_jobs < 1 )) && max_jobs=1
JOBS="${5:-$max_jobs}"
if [[ -n "${5:-}" ]] && (( JOBS > max_jobs )); then
  echo "warning: $JOBS workers requested but only ${mem_gib} GiB RAM;" \
    "capping to $max_jobs (set DECOMPILE_MEM_PER_JOB_GIB to override)" >&2
  JOBS=$max_jobs
fi
WORKROOT="$(dirname "$BASE_PROJECT")"
mkdir -p "$OUT"

# Stale lock from an interrupted run would be copied to every worker.
rm -f "$BASE_PROJECT"/*.lock "$BASE_PROJECT"/*.lock~

# Balanced contiguous ranges: target = total bytes / jobs.
mapfile -t RANGES < <(python3 - "$FUNCTIONS" "$JOBS" <<'PY'
import sys
rows = []
for line in open(sys.argv[1], encoding="utf-8"):
    if line.startswith("address\t"):
        continue
    f = line.rstrip("\n").split("\t")
    if len(f) < 3:
        continue
    rows.append(int(f[2]))
jobs = int(sys.argv[2])
total = sum(rows)
target = max(1, total // jobs)
ranges = []
start = 0
acc = 0
for i, size in enumerate(rows):
    acc += size
    if acc >= target and len(ranges) < jobs - 1:
        ranges.append((start, i + 1 - start, acc))
        start = i + 1
        acc = 0
ranges.append((start, len(rows) - start, acc))
for i, (s, c, b) in enumerate(ranges):
    print(f"{i} {s} {c} {b}")
PY
)

echo "== ranges (worker start count bytes)"
printf '%s\n' "${RANGES[@]}"

# Copy the project in parallel, one private copy per worker.
echo "== copying project to $WORKROOT/ghidra_worker*"
for line in "${RANGES[@]}"; do
  read -r i _ _ _ <<<"$line"
  rm -rf "$WORKROOT/ghidra_worker$i"
  cp -a "$BASE_PROJECT" "$WORKROOT/ghidra_worker$i" &
done
wait

pids=()
for line in "${RANGES[@]}"; do
  read -r i start count _ <<<"$line"
  echo "== worker $i: $start..$((start + count))"
  GHIDRA_HEADLESS_MAXMEM="${GHIDRA_HEADLESS_MAXMEM:-6G}" \
    /opt/ghidra/support/analyzeHeadless "$WORKROOT/ghidra_worker$i" "$PROJECT_NAME" \
    -process "$PROGRAM" -noanalysis \
    -scriptPath "$HERE/ghidra" \
    -postScript DecompileAll.java "$OUT" "$start" "$count" "$TIMEOUT" \
    > "$OUT/worker-$i.log" 2>&1 &
  pids+=($!)
done

status=0
for pid in "${pids[@]}"; do
  if ! wait "$pid"; then
    status=1
  fi
done

echo "== worker results"
grep -h 'DecompileAll ' "$OUT"/worker-*.log 2>/dev/null || true
echo "c files: $(find "$OUT" -maxdepth 1 -name '*.c' | wc -l)"
echo "manifests: $(find "$OUT" -maxdepth 1 -name 'manifest-*.tsv' | wc -l)"
exit "$status"
