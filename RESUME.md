# Resume after formatting the OS

The Rust code, ledger and translation pipeline are in this git repository.
The decompiled corpus is a local cache and is **not** in git.

The 2026-11-11 backups live in `~/Documents/galaxycraft-backup/`
(`galaxycraft-decompilation-only-*.tar.zst` with the 56,128-body corpus,
`optional-full-backup/` with the Ghidra project and runtime PE).  Restore
the full one with `scripts/restore_work_cache.sh`, then run the
decompilation-only `restore_to_home.sh` on top.

## 1. Copy the cache backup before formatting

The current backup is created at:

```text
/home/cotizcesar/Documents/galaxycraft-work-backup-2026-11-11.tar.zst
```

The archive is about **87 MB** compressed.  Copy it and its `.sha256` to an
external disk or cloud storage; verify it with
`sha256sum -c galaxycraft-work-backup-2026-11-11.tar.zst.sha256`.
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
next wave:         15 (queue mode)
lib tests:         2802 (full suite 3078)
```

Fixture-backed tests need the generated fixtures and assets
(`engine-rust/fixtures/README.md`); without them ~760 lib tests fail with
`ShortFile ... units.dat`.

Follow [AGENTS.md](AGENTS.md) and
`engine-rust/docs/TRANSLATION-GUIDE.md`:

```bash
python3 scripts/triage_corpus.py
python3 scripts/prepare_wave.py --wave 15 --queue engine-rust/docs/triage.tsv
# launch ten translators (opencode: /wave 15)
python3 scripts/validate_wave.py --wave 15
python3 scripts/merge_wave.py --wave 15 --map auto
cd engine-rust && cargo fmt && cargo test --lib && cargo clippy --all-targets -- -D warnings
```

Then regenerate the ledger exactly as documented in `AGENTS.md`.

## Important repository rules

- Never commit `engine-rust/src/waveNN/` or `tests/waveNN_agentNN.rs`; they
  are temporary staging.
- Never commit the decompiled corpus, `sc32_runtime_pe.bin` or the Ghidra
  project.
- Skipped addresses in committed files must be `FUN_xxxxxxxx` or bare hex,
  never `0x...`, or the ledger falsely credits them.
- Retail game files are read-only from the user's own installation.
