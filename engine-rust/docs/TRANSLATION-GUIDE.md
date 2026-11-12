# Translation guide — finish the corpus and wire it in

Playbook for the agent (orchestrator) that continues the decompilation
translation.  Goal, in order of value:

1. Translate every **gameplay-relevant** function (triage tiers 1–2).
2. **Integrate**: make the engine call the translated logic, or prove with a
   test that the existing engine already matches it.
3. Translate client-side tier 3, then tier 4 only when a feature needs it.

> This file lives under `engine-rust/`, so every literal `0x` + 6–8 hex
> digits in it is credited by the ledger.  It uses `0xADDR` placeholders on
> purpose.  Keep it that way when you edit it.

## 1. Why queue mode instead of address windows

Waves 1–14 walked contiguous windows.  Above the gameplay code the binary is
mostly compiler funclets, anti-tamper blocks and third-party libraries, so
windows would burn hundreds of waves on code the Rust engine never needs.
`scripts/triage_corpus.py` classifies all `decompiled_uncited` rows once:

- **skip forever** (≈ 29k rows): `unwind`, `fragment`, `library`,
  `obfuscated`, `trivial`, `hub`, and `bnet` (Battle.net / online services:
  a product decision, marked only, never implemented).
- **clean** rows get a tier from the call graph of the pseudocode
  (`FUN_xxxxxxxx` references) relative to addresses already cited in
  gameplay modules (`sim`, `combat`, `orders`, `iscript`, `aiscript`,
  `ai_recovered`, `path`, `trigger`, `handlers`, `map`, `data` …).
- each row carries a suggested `module` (majority vote of its translated
  neighbours), so `merge_wave.py --map auto` needs no human decision.

Rerun the triage after every ledger regeneration: new translations become
seeds and pull their neighbours up to tier 1.  Expect tier 2 to shrink as
tier 1 refills; that is the frontier advancing through the gameplay graph.

Budget estimate: tiers 1–3 ≈ 7.9k functions ≈ 40 waves of 10 × 20.

## 2. One wave, step by step (orchestrator)

```bash
git switch -c wave-W                          # or keep one branch per batch of waves
python3 scripts/triage_corpus.py              # ~1 min, prints counts only
python3 scripts/prepare_wave.py --wave W --queue engine-rust/docs/triage.tsv
#   options: --count 10 --per-agent 20 --max-tier 2  (the owner asked for
#   --count 20, then 30, agents per wave; merge/validate scale with it)
```

This writes, per agent NN:

- `/tmp/opencode/waveW/agentNN.md` — the address list, sizes, body paths
  and the merge target module;
- `engine-rust/src/waveW/agentNN.rs` — placeholder staging file;
- `engine-rust/tests/waveW_agentNN.rs` — isolated harness;
- `/tmp/opencode/waveW/summary.json` — lists and modules (used by validate
  and merge).

Launch the ten translators **in parallel**, each with the prompt in §3.
Then:

```bash
python3 scripts/validate_wave.py --wave W     # must print "problems: 0"
python3 scripts/merge_wave.py --wave W --map auto
cd engine-rust
cargo fmt
cargo test -q --lib 2>&1 | tail -5
cargo clippy -q --all-targets -- -D warnings 2>&1 | tail -20
cargo fmt --check
cd ..
python3 scripts/decomp_ledger.py \
  --functions ~/.cache/galaxycraft/functions_full.tsv \
  --decomp ~/.cache/galaxycraft/decomp_full --source-root engine-rust \
  --tsv engine-rust/docs/decomp-ledger.tsv \
  --json ~/.cache/galaxycraft/decomp_ledger.json
python3 scripts/record_skips.py --wave W      # keep skipped rows out of the queue
git status --short          # no src/waveW/, no tests/waveW_*
```

`merge_wave.py` drops inner attributes (`#![allow(...)]`) from staging files.
If clippy then fails (`too_many_arguments`, `collapsible_if`), add item-level
`#[allow(...)]` on the flagged functions in the merged module.  Translators
must also cite no address outside their list, even in tests or data tables:
write such constants as `0x5b_b5e0` (underscore) so the ledger ignores them.

Append one line to `engine-rust/docs/WAVES.md` and commit:

```text
Wave W: +N translated (ledger A -> B), lib tests T, tier mix 1:x 2:y
```

If an agent returns a placeholder or fails validation: re-prompt only that
agent with the validator output, then `merge_wave.py --wave W --map auto
--replace`.  If it fails twice, drop it: delete its staging file content
back to the placeholder, merge the rest, and let the next triage re-queue
its rows.

Fix compile/clippy errors after the merge yourself, locally (usually name
clashes or unused imports).  Do not re-launch a whole wave for them.

## 3. Translator prompt (copy, fill W and NN)

```text
You are translator NN of wave W in /home/cotizcesar/Documents/galaxycraft.
Read AGENTS.md (rules) and engine-rust/docs/WAVE-AGENT-INSTRUCTIONS.md (task).
Your assignment: /tmp/opencode/waveW/agentNN.md.
Write only engine-rust/src/waveW/agentNN.rs.  Verify with
  cd engine-rust && rustfmt --edition 2024 src/waveW/agentNN.rs &&
  cargo test -q --test waveW_agentNN 2>&1 | tail -15
Return the 4-part report (translated / skipped / tests / notes), max 40 lines.
```

Pass this prompt (with W and NN filled in) to each agent.

## 4. Reading Ghidra pseudocode fast

Ignore (boilerplate, never translate):

- `ExceptionList = &local_10; puStack_c = &DAT_...;` — SEH frame setup.
- `DAT_00f4e578 ^ (uint)&stack...` and a final call that compares it — the
  stack-cookie check.
- `guard_check_icall()` — CFG check before an indirect call.
- `WARNING: Removing unreachable block` comments.

Map to Rust:

| pseudocode | Rust |
|---|---|
| `int in_ECX` / `param_1` used as base pointer | `this`: pass `&[u8]`/`&mut [u8]` or a typed struct |
| `*(int *)(p + 0x20)` | `read_i32(buf, 0x20)` / `i32::from_le_bytes` / struct field |
| `*(ushort *)(p + i * 0xc + 4)` | `rows[i].field` with stride 0xc documented |
| `(DAT_xxxxxxxx ^ 0xKEY) + off` | Remastered encrypted global pointer: take the decoded table as a parameter |
| `CONCAT31(a, b)` | `(a << 8) \| b` (top 3 bytes from `a`) |
| `SUB41(x, 0)` | `x as u8` |
| `(code *)` call through table | caller-supplied closure `impl FnMut(...)` |
| `FUN_xxxxxxxx(...)` not yet translated | closure parameter, or call the existing Rust helper if `rg` finds its address |
| loops over `DAT_` arrays of fixed length | slice parameter + documented length |

Arithmetic must stay bit-exact: use `wrapping_*`, explicit `as i16/u16`
casts and signed/unsigned shifts exactly as the pseudocode shows.  Gameplay
uses fixed point (`<< 8`) — keep it.

Known layouts and constants are in `evidence-recovery.md`,
`IMPLEMENTATION-PHASES.md` and the doc comments of neighbours: `rg -n
'0xADDR' engine-rust/src` for each `FUN_` callee before guessing a struct.

## 5. Integration (the "implement it" half)

A translated helper is only reference code until the engine uses it or a
test proves the engine already agrees.  Run **one integration wave after
every two translation waves**.  Work on a single subsystem at a time.

Pick targets:

```bash
# helpers merged into gameplay modules that no non-test code calls yet
rg -o 'pub fn (w[0-9]+a[0-9]+_[a-z0-9_]+)' -r '$1' engine-rust/src/sim.rs \
  engine-rust/src/ai_recovered.rs | sort -u > /tmp/helpers.txt
```

For each candidate (smallest observable effect first: timers, counters,
predicates, order transitions, damage/AI decisions):

1. Find the engine code that models the same behaviour (`rg` for the field,
   order or constant).
2. **Differential test**: drive both the helper and the `World` path with
   the same inputs (including edge values) and assert equal results.
3. If they agree → keep the engine code, add the test, cite the address
   in the engine function's doc comment.  Done.
4. If they disagree → the retail helper wins.  Fix the engine (or call the
   helper from it), keep the test, and run the determinism/replay tests:
   `cargo test -q --lib replay` and `cargo test -q --lib determin`.
5. Never add a second copy of state (`CUnit` lists, raw order nodes) to
   `World`; translate the *logic*, not the retail memory layout.  Reasons
   are in `TRANSLATED-INTEGRATION.md`.

Record each wired helper in the table of `TRANSLATED-INTEGRATION.md`
(address, helper, call site, test).  Before closing an integration wave run
the full `cargo test -q` and, if `SC_INSTALL` is set, a visual smoke test:

```bash
SC_NO_AUDIO=1 cargo run -q --release --bin sc_view -- \
  --install "$SC_INSTALL" --shot /tmp/ingame.bmp --shot-screen ingame
```

## 6. Troubleshooting

| symptom | fix |
|---|---|
| hundreds of tests fail with `ShortFile ... units.dat` | fixtures missing: `engine-rust/fixtures/README.md` |
| test panics `bundled map` / `NotFound` under `assets/` | run `sc_assets extract --out assets` + the symlinks in the fixtures README |
| harness error in another agent's file | ignore, retry later; never edit it |
| validator: `outside` | file cites an address not in the list → change it to `FUN_` form |
| ledger count jumped by more than the wave | a doc/comment cited an uncited address with `0x`; `git diff` + fix |
| `prepare_wave` refuses to overwrite | staging from a previous run; finish/merge it or `--force` |
| tier 1–2 empty | lower priority: `--max-tier 3`, then tier 4 only for features in progress |
| a body needs a callee in the 622 failures | skip it (`FUN_` form, reason "depends on failed body") |

## 7. Definition of done

- Tiers 1–3 empty in `triage.tsv` (everything translated or explicitly
  skipped with a reason in the batch notes).
- Every gameplay helper either wired or covered by a differential test, and
  listed in `TRANSLATED-INTEGRATION.md`.
- `cargo test`, clippy and fmt clean; README, `RESUME.md` and the state
  table in `AGENTS.md` updated with the final numbers.
