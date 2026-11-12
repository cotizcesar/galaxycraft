# Translation pipeline and resume guide

This document is the hand-off point for continuing the decompilation
translation after a machine reset.  The Rust reconstruction, ledger and
pipeline scripts live in git; the decompiled corpus is a local, read-only
cache derived from the player's own StarCraft installation and is **not**
committed.

## Current stop point (2026-11-11)

| item | value |
|---|---|
| last merged wave | wave 34 (release 0.1.21) |
| ledger translated | **5736** functions |
| ledger uncited | 49500 functions |
| ledger decompile-failed | 622 functions |
| lib tests | **4822 passed** (full suite 5099) |
| next wave number | 35 (queue mode, tier 3: 46 rows, then tier 4) |

Wave 13 added 174 exact ledger functions; wave 14 added 181.  The wave-13
and wave-14 staging files and harnesses were deleted after merging.  The
batches were placed in natural modules:

| window content | module |
|---|---|
| tile pages, surface/render queue, texture factory | `src/render.rs` |
| C++ containers, hashes, key/value tables, JSON parser | `src/runtime.rs` |
| sound mixer, voices, limiter | `src/audio.rs` |
| text triggers, localisation, level smoother | `src/trigger.rs` |
| keys/input, minimap scroll, settings, mission-end UI, chat dialogs | `src/client.rs` |
| command dispatch, AI-script opcodes, regions | `src/ai_recovered.rs` |
| order/shared-amount accounting | `src/sim.rs` |

## Prerequisites after reinstall

- Rust: the repository pins the toolchain in `rust-toolchain.toml`
  (1.99.1 at the last run).
- A StarCraft: Remastered installation.  The client reads it read-only:
  `--install DIR`, `SC_INSTALL`, `GALAXYCRAFT_INSTALL` or `STARCRAFT_HOME`.
- Ghidra 12.1.2 only if the decompiled corpus must be rebuilt.
- `zstd` for the optional cache backup.

```bash
git clone git@github.com:cotizcesar/galaxycraft.git
cd galaxycraft
cargo test --lib
```

## Restoring the decompiled corpus (recommended)

The corpus archive contains `decomp_full/`, `functions_full.tsv`,
`sc32_runtime_pe.bin`, the failure reports and the Ghidra project.  It is
kept outside the repository.  Copy the archive to the new machine, then:

```bash
scripts/restore_work_cache.sh /path/to/galaxycraft-work-backup.tar.zst
```

The script restores:

```text
~/.cache/galaxycraft/decomp_full/
~/.cache/galaxycraft/functions_full.tsv
~/.cache/galaxycraft/sc32_runtime_pe.bin
~/.cache/galaxycraft/decomp_failures_final.tsv
~/.cache/galaxycraft/decomp_attempts_final.tsv
~/galaxycraft-work/ghidra_full/
```

If the archive is lost, the corpus can be regenerated from the retail
executable, but that is a multi-hour Ghidra pass.  The exact evidence and
Ghidra settings are in `engine-rust/docs/evidence-recovery.md` and
`engine-rust/docs/CHANGES-2026-11-08.md`.  The generic pass is:

```bash
# after importing sc32_runtime_pe.bin into ~/galaxycraft-work/ghidra_full
scripts/decompile_parallel.sh \
  ~/galaxycraft-work/ghidra_full sc32 sc32_runtime_pe.bin \
  ~/.cache/galaxycraft/decomp_full 8 1 \
  ~/.cache/galaxycraft/functions_full.tsv
```

The 622 hard failures are a known ceiling with the installed toolchain; do
not spend a wave on them again.

## Running one ten-agent wave

> Since wave 15 waves use **queue mode** (`scripts/triage_corpus.py` +
> `prepare_wave.py --queue`, `validate_wave.py --wave W`, `merge_wave.py
> --map auto`).  See `TRANSLATION-GUIDE.md`.  The window-mode steps below
> still work and are kept for reference.

1. Prepare the assignments and isolated harnesses:

   ```bash
   python3 scripts/prepare_wave.py --wave 15 --start 0x684000
   ```

   This writes `/tmp/opencode/wave15/agentNN.md`,
   `engine-rust/src/wave15/agentNN.rs` placeholders and
   `engine-rust/tests/wave15_agentNN.rs`.

2. Launch agents in parallel (20 per Agent batch, or Workflow shards of ~30), one per assignment.  Use
   `engine-rust/docs/WAVE-AGENT-INSTRUCTIONS.md` as the prompt; set `W=15`
   and `NN` to the agent number.  Each agent writes only its own staging
   file and runs `cargo test --test wave15_agentNN`.

3. Validate the staging before merging:

   ```bash
   python3 scripts/validate_wave.py --wave 15 --start 0x684000
   ```

   This checks window ownership, ledger presence, duplicates and
   placeholders.

4. Merge into natural modules.  The map is chosen from the window content;
   example:

   ```bash
   python3 scripts/merge_wave.py --wave 15 \
     --map 1=client.rs,2=client.rs,3=render.rs,4=runtime.rs,5=trigger.rs,\
6=audio.rs,7=runtime.rs,8=client.rs,9=client.rs,10=client.rs
   ```

   Use `--replace` to re-merge a batch after a late correction.  The merge
   script deletes `src/wave15/` and the harnesses when every batch is real.

5. Verify:

   ```bash
   cd engine-rust
   cargo fmt
   cargo test --lib
   cargo clippy --all-targets -- -D warnings
   cargo fmt --check
   ```

6. Regenerate the ledger:

   ```bash
   cd ..
   python3 scripts/decomp_ledger.py \
     --functions ~/.cache/galaxycraft/functions_full.tsv \
     --decomp ~/.cache/galaxycraft/decomp_full \
     --source-root engine-rust \
     --tsv engine-rust/docs/decomp-ledger.tsv \
     --json ~/.cache/galaxycraft/decomp_ledger.json
   ```

7. Record the wave in `engine-rust/docs/CHANGES-2026-11-09.md`, confirm the
   staging directory is gone, commit and push.

## Ledger rules

- A function is `translated` only when its exact address appears as `0x...`
  in a `.rs`/`.md` file under `engine-rust/`.
- Never write a skipped address with a `0x` prefix in a committed file; use
  `FUN_xxxxxxxx` or bare hex.  Otherwise the ledger falsely credits it.
- Do not commit `engine-rust/src/waveNN/` or
  `engine-rust/tests/waveNN_agentNN.rs`.
- Do not commit the decompiled corpus, the runtime PE or the Ghidra
  project.

## Current repository state

At the time of writing:

```text
cargo test --lib:                    4822 passed
cargo clippy --all-targets -- -D warnings: clean
cargo fmt --check:                    clean
ledger translated:                   5736
next wave:                           35 (queue mode)
```
