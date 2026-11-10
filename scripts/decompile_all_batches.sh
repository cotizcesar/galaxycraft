#!/usr/bin/env bash
# Decompile every function in an already-analyzed Ghidra project, in bounded
# batches.  The project is not re-imported or re-analyzed; each batch only
# runs DecompileAll.java over a slice and resumes by skipping existing files.
#
# Usage:
#   scripts/decompile_all_batches.sh <project-dir> [project-name] \
#       [program-name] [batch-size] [total] [output-dir] [timeout-seconds]
#
# The output defaults to ~/.cache/galaxycraft/decomp_full so a /tmp reset does
# not lose hours of decompilation.  Ghidra refuses project paths containing a
# hidden element (for example `~/.cache`), so keep the project under a visible
# directory such as `~/galaxycraft-work/`.
set -euo pipefail

PROJECT_DIR="${1:?Ghidra project directory}"
PROJECT_NAME="${2:-sc32}"
PROGRAM="${3:-sc32_runtime_pe.bin}"
BATCH="${4:-5000}"
TOTAL="${5:-57000}"
OUT="${6:-$HOME/.cache/galaxycraft/decomp_full}"
TIMEOUT="${7:-3}"
HERE="$(cd "$(dirname "$0")" && pwd)"
export GHIDRA_HEADLESS_MAXMEM="${GHIDRA_HEADLESS_MAXMEM:-10G}"
mkdir -p "$OUT"

start=0
while [ "$start" -lt "$TOTAL" ]; do
  echo "== batch $start..$((start+BATCH)) -> $OUT"
  /opt/ghidra/support/analyzeHeadless "$PROJECT_DIR" "$PROJECT_NAME" \
    -process "$PROGRAM" -noanalysis \
    -scriptPath "$HERE/ghidra" \
    -postScript DecompileAll.java "$OUT" "$start" "$BATCH" "$TIMEOUT" || true
  start=$((start+BATCH))
done

echo "decompiled .c files: $(find "$OUT" -maxdepth 1 -name '*.c' | wc -l)"
echo "manifest batches:    $(find "$OUT" -maxdepth 1 -name 'manifest-*.tsv' | wc -l)"
echo "output directory:    $OUT"
