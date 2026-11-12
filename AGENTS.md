# AGENTS.md — GalaxyCraft

Rust reimplementation of StarCraft 1.23.10 (gameplay + menus).
The main goal is to **translate the decompiled retail corpus into Rust and
wire it into the engine**.  Read this file first, then
`engine-rust/docs/TRANSLATION-GUIDE.md` before you start a wave.

## Token budget rules (read before any tool call)

1. **Never open a big module whole.**  `engine-rust/docs/MODULES.md` lists
   every file with its size and purpose; wave output lives one block per
   file (`src/runtime/w31/a07.rs`, `src/client/...`, `src/sim/translated/...`)
   and `World` / `App` methods are split by subsystem (`src/sim/*.rs`,
   `src/bin/sc_view/*.rs`).  Still use `rg -n PATTERN` and read a 40–80
   line range.  Hand-written files stay under 1 500 lines
   (`scripts/check_file_sizes.py`; the files above it today are listed in
   `docs/large-files.txt` and may only shrink).
2. **Never read the ledger or triage whole** (56k / 52k rows).  Use
   `rg '^0xADDR' engine-rust/docs/decomp-ledger.tsv` or the scripts.
3. Read only the pseudocode files listed in your assignment.  One body at a
   time; most are < 150 bytes of machine code.
4. Quiet commands: `cargo test -q ...`, `cargo clippy -q ...`, pipe long
   output through `tail -20`.  Never paste pseudocode back in a report.
5. Do not re-derive state.  The numbers below and in
   `engine-rust/docs/WAVES.md` are authoritative; rerun a script instead of
   reasoning about the corpus by hand.
6. A translation agent runs only its own harness
   (`cargo test -q --test waveW_agentNN`); the full suite runs once, after
   the merge.

## Current state (2026-11-11, after wave 34)

| item | value |
|---|---|
| ledger `translated` | 5736 of 56128 functions |
| `decompiled_uncited` | 49500 (only ~0.05k worth translating, see below) |
| `decompile_failed` | 622 — known ceiling, do not retry |
| tests | `cargo test --lib` 4822 pass; `cargo test` 5099 pass |
| next wave | 35 (tier 3 down to 46 rows, then tier 4: measured yield 47% per row; see yield note) |

Triage of the 49500 uncited rows (`scripts/triage_corpus.py`):

| category | rows | action |
|---|---|---|
| `clean` tier 1 (next to gameplay code) | 0 | **translate first** |
| `clean` tier 2 (two hops from gameplay) | 0 | translate next |
| `clean` tier 3 (next to client/render/audio) | 46 | translate next |
| `clean` tier 4 (unrelated engine/library glue) | 12793 | only on demand |
| `unwind` (compiler exception funclets) | 15305 | never |
| `fragment` (`unaff_*`/`in_*` mid-function blocks) | 6187 | never |
| `library` (OpenGL loader, FMOD, OpenSSL, png, zlib) | 3039 | never |
| `obfuscated` (anti-tamper, bad instruction data) | 2136 | never |
| `trivial` (thunks, ≤ 8 bytes) | 2057 | never |
| `hub` (CRT/allocator helpers with > 150 callers) | 38 | never |
| `bnet` (Battle.net / online services: login, friends, presence, matchmaking, telemetry, web, TACT/CASC downloader; propagated through the call graph) | 704 | **never implement, only marked** |
| `skipped` (assigned to a wave, not translatable yet) | 7195 | never (`skiplist.tsv`) |

## Local data (not in git, must exist)

| path | what |
|---|---|
| `~/.cache/galaxycraft/decomp_full/` | 56128 Ghidra pseudocode bodies |
| `~/.cache/galaxycraft/functions_full.tsv` | inventory: address, name, size, thunk |
| `~/.cache/galaxycraft/sc32_runtime_pe.bin` | analysable retail image |
| `~/galaxycraft-work/ghidra_full/` | Ghidra project (only to re-decompile) |
| `engine-rust/fixtures/`, `engine-rust/assets/` | generated from the user's install |

StarCraft install: `~/Games/battlenet/drive_c/Program Files (x86)/StarCraft`
(export it as `SC_INSTALL`).  Restore/regen steps: `RESUME.md`.

## Commands

```bash
# one-time per wave
python3 scripts/triage_corpus.py                       # refresh priorities
python3 scripts/prepare_wave.py --wave W --queue engine-rust/docs/triage.tsv
# ... translation agents work (see TRANSLATION-GUIDE.md) ...
python3 scripts/validate_wave.py --wave W
python3 scripts/merge_wave.py --wave W --map auto   # writes src/<module>/wNN/aNN.rs
cd engine-rust && cargo fmt && cargo test -q --lib && \
  cargo clippy -q --all-targets -- -D warnings && cargo fmt --check && cd ..
python3 scripts/decomp_ledger.py \
  --functions ~/.cache/galaxycraft/functions_full.tsv \
  --decomp ~/.cache/galaxycraft/decomp_full --source-root engine-rust \
  --tsv engine-rust/docs/decomp-ledger.tsv \
  --json ~/.cache/galaxycraft/decomp_ledger.json
python3 scripts/record_skips.py --wave W   # untranslated rows -> skiplist.tsv

# lookups
python3 scripts/query_decomp.py --address 0xADDR       # body of one function
python3 scripts/query_decomp.py --pattern REGEX --limit 10
rg -n 'w14a03_' engine-rust/src                        # find a merged helper
```

Waves are driven by hand: prepare, launch one agent per assignment (Agent
batches of 20 or Workflow shards of 30), reconcile, merge (see the guide).

Yield note: wave 34 +105 (60 agents; tier 3 72% of rows, tier 4 47% of rows), wave 33 +119 (73 agents, tier 3), wave 32 +228 (150 agents, 5 Workflow shards of 30, tier 3), wave 31 +596 (148 agents, 5 workflows; tier 2 emptied), wave 15 gave +72 (36%), wave 16 +84 (42%), wave 17 +93 (47%), wave 18 +80 (40%), wave 19 +86 (43%), wave 20 +92 (46%), wave 21 +124 (31%, 20 agents), wave 30 +51 (20 agents; the queue had reached the TACT/CDN downloader, so triage now propagates `bnet` through the call graph), wave 29 +81 (20 agents; D3D/TACT/CDN code is mostly bnet or glue), wave 28 +159 (20 agents; libvpx SAD/variance/loop-filter/intra-predictor kernels), wave 27 +115 (20 agents; rapidjson, FMOD DSP, vector-growth helpers), wave 26 +95 (20 agents; ~5 each; MD5/SHA-1/RC4/bignum with test vectors), wave 25 +223 (59 agents launched of 100 prepared; ~4 per agent, 19% yield), wave 24 +251 (25%, 50 agents; ~5 per agent), wave 23 +141 (35%, 20 agents), wave 22 +140 (23%, 30 agents; the harness caps concurrent subagents at 20). Yield is falling as tier 1 empties: most remaining rows are STL/SEH glue ending in non-returning calls, so more waves buy little engine value per token.  Most
skips end in non-returning calls or depend on unmodelled callees; that is
expected.  Do not retry skiplist rows until a callee they need is translated.

## Hard rules

- **Battle.net is out of scope.**  Anything that talks to Battle.net or
  online services (login, friends, presence, matchmaking, ladder, telemetry,
  HTTP/SSL services) is never translated or implemented: it stays marked
  `bnet` in `triage.tsv` and the queue skips it.  If a translator meets such
  a function anyway, skip it (`FUN_` form, reason "bnet").
- **Ledger credit = literal `0x` + address in any `.rs`/`.md` under
  `engine-rust/`.**  Cite only addresses you implemented.  Skipped or
  mentioned addresses are written `FUN_00xxxxxx` or bare hex.  In docs use
  placeholders like `0xADDR`, never a real uncited address.
- Never commit: `engine-rust/src/waveNN/`, `engine-rust/tests/waveNN_agentNN.rs`,
  the corpus, `sc32_runtime_pe.bin`, the Ghidra project, fixtures, assets,
  captures.  `.gitignore` covers most of it; check `git status` anyway.
- Retail install is read-only.  No game assets or pseudocode bodies in git.
- Do not translate rows outside your assignment; do not edit another
  agent's staging file, `lib.rs`, `Cargo.toml` or the ledger by hand.
- Translations are pure functions (`wWaNN_` prefix) with tests.  Wiring into
  `World`/client is a separate **integration** task (guide §5) that must keep
  determinism/replay tests green.
- Toolchain is pinned (Rust 1.99.1).  Every commit: fmt, clippy `-D
  warnings`, `cargo test --lib` green.
- Git: work on a branch, commit per wave, end commit messages with the
  co-author line the user asks for.  Never force-push `main`.

## Repository map

| path | contents |
|---|---|
| `engine-rust/src/sim/` | deterministic `World`: types in `mod.rs`, methods by subsystem (`spawn`, `step`, `orders`, `combat`, `damage`, `movement`, `spells`, `production`, ...), translated batches in `translated/` |
| `engine-rust/src/{combat,orders,iscript,aiscript,ai,path,contour,vision,trigger,handlers,map,data}.rs` | rules subsystems |
| `engine-rust/src/ai_recovered/` | translated AI-script / command dispatch helpers (one file per wave block) |
| `engine-rust/src/runtime/` | translated STL/containers/JSON/runtime helpers (one file per wave block) |
| `engine-rust/src/client/`, `render.rs`, `audio.rs`, `menu.rs` | client |
| `engine-rust/src/bin/sc_view.rs` + `sc_view/` | playable client; `impl App` split by screen; `sc_assets` = data tool |
| `engine-rust/docs/` | ledger, triage, pipeline docs, RE evidence |
| `scripts/` | corpus/wave pipeline, packaging, LAN and timeline tools |

## Docs index

- `engine-rust/docs/TRANSLATION-GUIDE.md` — **the playbook** (waves, prompts,
  integration, troubleshooting).
- `engine-rust/docs/WAVES.md` — one line per wave; append after each merge.
- `engine-rust/docs/WAVE-AGENT-INSTRUCTIONS.md` — prompt for one agent.
- `engine-rust/docs/TRANSLATED-INTEGRATION.md` — what is wired where.
- `engine-rust/docs/TRANSLATION-PIPELINE.md`, `RESUME.md` — restore/regen.
- `engine-rust/docs/evidence-recovery.md`, `IMPLEMENTATION-PHASES.md` — RE notes.
- `engine-rust/docs/MODULES.md` — generated map of every source file
  (`scripts/gen_modules_doc.py`).
- `engine-rust/docs/MODULARIZATION-PLAN.md` — how the big files were split.
- `engine-rust/docs/UNIT-FIDELITY-METHOD.md` — how one unit is audited against
  retail and fixed (reference tool `scripts/unit_ref.py`, tests, scripted
  HD / SD screenshots); results in `FIDELITY.md`, open doubts in `pending.md`.
