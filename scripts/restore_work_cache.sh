#!/usr/bin/env bash
# Restore the local decompilation cache and Ghidra project from a backup.
#
# Usage:
#   scripts/restore_work_cache.sh /path/to/galaxycraft-work-backup.tar.zst
#
# The archive is expected to contain paths relative to $HOME:
#   .cache/galaxycraft/decomp_full/
#   .cache/galaxycraft/functions_full.tsv
#   .cache/galaxycraft/sc32_runtime_pe.bin
#   galaxycraft-work/ghidra_full/
set -euo pipefail

archive="${1:-}"
if [[ -z "$archive" ]]; then
    echo "usage: $0 /path/to/galaxycraft-work-backup.tar.zst" >&2
    exit 2
fi
if [[ ! -f "$archive" ]]; then
    echo "missing archive: $archive" >&2
    exit 1
fi

cd "$HOME"
case "$archive" in
    *.tar.zst)  tar -I zstd -xf "$archive" ;;
    *.tar.xz)   tar -xJf "$archive" ;;
    *.tar.gz|*.tgz) tar -xzf "$archive" ;;
    *)
        echo "unsupported archive extension: $archive" >&2
        echo "expected .tar.zst, .tar.xz, .tar.gz or .tgz" >&2
        exit 2
        ;;
esac

test -d "$HOME/.cache/galaxycraft/decomp_full" || {
    echo "archive did not contain .cache/galaxycraft/decomp_full" >&2
    exit 1
}
test -f "$HOME/.cache/galaxycraft/functions_full.tsv" || {
    echo "archive did not contain functions_full.tsv" >&2
    exit 1
}

echo "restored decompilation cache:"
du -sh "$HOME/.cache/galaxycraft/decomp_full" \
       "$HOME/.cache/galaxycraft/functions_full.tsv" 2>/dev/null || true
if [[ -d "$HOME/galaxycraft-work/ghidra_full" ]]; then
    echo "restored Ghidra project: $HOME/galaxycraft-work/ghidra_full"
fi
echo
echo "next:"
echo "  python3 scripts/prepare_wave.py --wave 15 --start 0x684000"
echo "  python3 scripts/decomp_ledger.py --functions ~/.cache/galaxycraft/functions_full.tsv \\"
echo "    --decomp ~/.cache/galaxycraft/decomp_full --source-root engine-rust \\"
echo "    --tsv engine-rust/docs/decomp-ledger.tsv"
