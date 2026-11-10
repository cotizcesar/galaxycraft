#!/usr/bin/env bash
# Targeted recovery for functions that exhausted the normal decompiler pass.
#
# Each worker gets a private project copy and a shard of the failure list.
# Only "timeout" failures are retried by default: the low-level flow/varnode
# errors are structural and do not improve with a larger budget.
#
# Usage:
#   scripts/decompile_recovery.sh <base-project-dir> <failure-tsv> [jobs] \
#       [timeout-seconds] [max-instructions] [max-payload-mb] [output-dir]
#
# The failure TSV is the report written by the inventory pass
# (`~/.cache/galaxycraft/decomp_failures.tsv`).  Successful bodies land in
# <output-dir> as `<address>_<name>.c`; merge them over the failure markers
# only after checking the file header.
#
# Worker count is capped to the host's RAM (8 GiB per worker by default) so a
# batch cannot fill swap and trigger systemd-oomd, which kills the whole
# terminal tab and takes fish down with it.  Override the budget with
# DECOMPILE_MEM_PER_JOB_GIB if needed.
set -euo pipefail

BASE_PROJECT="${1:?Ghidra project directory}"
FAILURES="${2:?failure report TSV}"
TIMEOUT="${4:-180}"
MAX_INSTRUCTIONS="${5:-1000000}"
MAX_PAYLOAD="${6:-128}"
OUT="${7:-$HOME/.cache/galaxycraft/decomp_recovered}"
PROJECT_NAME="${PROJECT_NAME:-sc32}"
PROGRAM="${PROGRAM:-sc32_runtime_pe.bin}"
HERE="$(cd "$(dirname "$0")" && pwd)"
WORKROOT="$(dirname "$BASE_PROJECT")"
LISTS="$WORKROOT/recovery_lists"
mkdir -p "$OUT" "$LISTS"

MEM_PER_JOB_GIB="${DECOMPILE_MEM_PER_JOB_GIB:-8}"
mem_gib=$(( $(awk '/^MemTotal:/ {print $2}' /proc/meminfo) / 1024 / 1024 ))
max_jobs=$(( mem_gib / MEM_PER_JOB_GIB ))
(( max_jobs < 1 )) && max_jobs=1
JOBS="${3:-$max_jobs}"
if [[ -n "${3:-}" ]] && (( JOBS > max_jobs )); then
  echo "warning: $JOBS workers requested but only ${mem_gib} GiB RAM;" \
    "capping to $max_jobs (set DECOMPILE_MEM_PER_JOB_GIB to override)" >&2
  JOBS=$max_jobs
fi

python3 - "$FAILURES" "$JOBS" "$LISTS" <<'PY'
import csv
import sys
from pathlib import Path

source, jobs, out = Path(sys.argv[1]), int(sys.argv[2]), Path(sys.argv[3])
rows = []
with source.open(encoding="utf-8") as f:
    reader = csv.DictReader(f, delimiter="\t")
    for row in reader:
        if row.get("reason") != "timeout":
            continue
        rows.append((row["address"], row["name"], row["size"]))
shards = [[] for _ in range(jobs)]
for i, row in enumerate(rows):
    shards[i % jobs].append(row)
for i, shard in enumerate(shards):
    with (out / f"recovery-{i}.tsv").open("w", encoding="utf-8") as f:
        for address, name, size in shard:
            f.write(f"{address}\t{name}\t{size}\n")
    print(f"shard {i}: {len(shard)} functions")
print(f"total timeout failures: {len(rows)}")
PY

pids=()
for i in $(seq 0 $((JOBS - 1))); do
  list="$LISTS/recovery-$i.tsv"
  [ -s "$list" ] || continue
  project="$WORKROOT/ghidra_worker$i"
  if [ ! -d "$project" ]; then
    rm -rf "$project"
    cp -a "$BASE_PROJECT" "$project"
  fi
  echo "== recovery worker $i: $(wc -l < "$list") functions"
  GHIDRA_HEADLESS_MAXMEM="${GHIDRA_HEADLESS_MAXMEM:-6G}" \
    /opt/ghidra/support/analyzeHeadless "$project" "$PROJECT_NAME" \
    -process "$PROGRAM" -noanalysis \
    -scriptPath "$HERE/ghidra" \
    -postScript DecompileList.java "$OUT" "$list" "$TIMEOUT" \
      "$MAX_INSTRUCTIONS" "$MAX_PAYLOAD" \
    > "$OUT/worker-$i.log" 2>&1 &
  pids+=($!)
done

status=0
for pid in "${pids[@]}"; do
  wait "$pid" || status=1
done

echo "== recovery results"
grep -h 'DecompileList ' "$OUT"/worker-*.log 2>/dev/null || true
echo "recovered files: $(find "$OUT" -maxdepth 1 -name '*.c' | wc -l)"
exit "$status"
