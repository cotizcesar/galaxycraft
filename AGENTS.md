# AGENTS.md — GalaxyCraft

Rust reimplementation of StarCraft 1.23.10 (gameplay + menus).
The main goal is to **translate the decompiled retail corpus into Rust and
wire it into the engine**.  Read this file first, then
`engine-rust/docs/TRANSLATION-GUIDE.md` before you start a wave.

## Token budget rules (read before any tool call)

1. **Never open a big module whole.**  `client.rs` is 60k lines, `runtime.rs`
   33k, `sim.rs` 31k, `ai_recovered.rs` 12k.  Use `rg -n PATTERN FILE` and
   then read a 40–80 line range.
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

## Current state (2026-11-11, after wave 22)

| item | value |
|---|---|
| ledger `translated` | 3572 of 56128 functions |
| `decompiled_uncited` | 51664 (only ~6.5k worth translating, see below) |
| `decompile_failed` | 622 — known ceiling, do not retry |
| tests | `cargo test --lib` 3287 pass; `cargo test` 3564 pass |
| next wave | 23 (queue mode; tier 1 is empty, see yield note) |

Triage of the 52435 uncited rows (`scripts/triage_corpus.py`):

| category | rows | action |
|---|---|---|
| `clean` tier 1 (next to gameplay code) | 0 | **translate first** |
| `clean` tier 2 (two hops from gameplay) | 5893 | translate next |
| `clean` tier 3 (next to client/render/audio) | 676 | after tier 2 |
| `clean` tier 4 (unrelated engine/library glue) | 14382 | only on demand |
| `unwind` (compiler exception funclets) | 15305 | never |
| `fragment` (`unaff_*`/`in_*` mid-function blocks) | 6392 | never |
| `library` (OpenGL loader, FMOD, OpenSSL, png, zlib) | 3041 | never |
| `obfuscated` (anti-tamper, bad instruction data) | 2136 | never |
| `trivial` (thunks, ≤ 8 bytes) | 2058 | never |
| `hub` (CRT/allocator helpers with > 150 callers) | 38 | never |
| `bnet` (Battle.net / online services: login, friends, presence, matchmaking, telemetry, web) | 465 | **never implement, only marked** |
| `skipped` (assigned to a wave, not translatable yet) | 1429 | never (`skiplist.tsv`) |

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
python3 scripts/merge_wave.py --wave W --map auto
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

In opencode: `/wave W` runs the whole cycle with the `translator` subagent
(`.opencode/commands/wave.md`, `.opencode/agents/translator.md`).

Yield note: wave 15 gave +72 (36%), wave 16 +84 (42%), wave 17 +93 (47%), wave 18 +80 (40%), wave 19 +86 (43%), wave 20 +92 (46%), wave 21 +124 (31%, 20 agents), wave 22 +140 (23%, 30 agents; the harness caps concurrent subagents at 20). Yield is falling as tier 1 empties: most remaining rows are STL/SEH glue ending in non-returning calls, so more waves buy little engine value per token.  Most
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
| `engine-rust/src/sim.rs` | deterministic `World`, units, orders, combat glue |
| `engine-rust/src/{combat,orders,iscript,aiscript,ai,path,contour,vision,trigger,handlers,map,data}.rs` | rules subsystems |
| `engine-rust/src/ai_recovered.rs` | translated AI-script / command dispatch helpers |
| `engine-rust/src/runtime.rs` | translated STL/containers/JSON/runtime helpers |
| `engine-rust/src/client.rs`, `render.rs`, `audio.rs`, `menu.rs` | client |
| `engine-rust/src/bin/sc_view.rs` | playable client; `sc_assets` = data tool |
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
