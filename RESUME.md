# Resume after formatting the OS

The Rust code, ledger and translation pipeline are in this git repository.
The decompiled corpus is a local cache and is **not** in git.

## 1. Copy the cache backup before formatting

The current backup is created at:

```text
/home/cotizcesar/Documents/galaxycraft-work-backup-2026-11-11.tar.zst
```

Copy that file (and its `.sha256`) to an external disk or cloud storage.
It contains:

```text
.cache/galaxycraft/decomp_full/         55,502 decompiled bodies
.cache/galaxycraft/functions_full.tsv   function inventory
.cache/galaxycraft/sc32_runtime_pe.bin  analysable retail image
.cache/galaxycraft/decomp_failures_final.tsv
galaxycraft-work/ghidra_full/           Ghidra project
```

No retail game data is committed to the repository.

## 2. Restore on the new machine

```bash
git clone git@github.com:cotizcesar/galaxycraft.git
cd galaxycraft
scripts/restore_work_cache.sh /path/to/galaxycraft-work-backup-2026-11-11.tar.zst
cargo test --lib
```

If the backup was lost, the corpus can be regenerated from the player's own
StarCraft installation; see `engine-rust/docs/evidence-recovery.md` and
`engine-rust/docs/TRANSLATION-PIPELINE.md`.  That is a multi-hour Ghidra
pass, so keep the backup.

## 3. Continue the translation

Current stop point:

```text
ledger translated: 2801
next window:       0x00684000
next wave:         15
lib tests:         2802
```

```bash
python3 scripts/prepare_wave.py --wave 15 --start 0x684000
# launch ten agents using engine-rust/docs/WAVE-AGENT-INSTRUCTIONS.md
python3 scripts/validate_wave.py --wave 15 --start 0x684000
python3 scripts/merge_wave.py --wave 15 --map 1=client.rs,...
cd engine-rust && cargo fmt && cargo test --lib && cargo clippy --all-targets -- -D warnings
```

Then regenerate the ledger exactly as documented in
`engine-rust/docs/TRANSLATION-PIPELINE.md`.

## Important repository rules

- Never commit `engine-rust/src/waveNN/` or `tests/waveNN_agentNN.rs`; they
  are temporary staging.
- Never commit the decompiled corpus, `sc32_runtime_pe.bin` or the Ghidra
  project.
- Skipped addresses in committed files must be `FUN_xxxxxxxx` or bare hex,
  never `0x...`, or the ledger falsely credits them.
- Retail game files are read-only from the user's own installation.
