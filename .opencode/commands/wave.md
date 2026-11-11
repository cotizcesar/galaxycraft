---
description: Run one full GalaxyCraft translation wave (triage, 10 parallel translators, validate, merge, verify, ledger, log). Argument = wave number.
---

Run translation wave $ARGUMENTS following `engine-rust/docs/TRANSLATION-GUIDE.md` §2.
Read `AGENTS.md` first and obey its token budget.

1. `python3 scripts/triage_corpus.py`
2. `python3 scripts/prepare_wave.py --wave $ARGUMENTS --queue engine-rust/docs/triage.tsv`
   (if it refuses because staging exists, inspect it: finish that wave or pass `--force`).
3. Launch the `translator` subagent 10 times **in parallel** (one message,
   10 task calls), agent numbers 01..10, each told: "wave $ARGUMENTS, agent NN".
4. `python3 scripts/validate_wave.py --wave $ARGUMENTS`; re-prompt only
   failing agents once with the validator lines; drop an agent that fails twice
   (reset its file to the placeholder text and leave its rows for the next wave).
5. `python3 scripts/merge_wave.py --wave $ARGUMENTS --map auto`
6. In `engine-rust/`: `cargo fmt`, `cargo test -q --lib 2>&1 | tail -5`,
   `cargo clippy -q --all-targets -- -D warnings 2>&1 | tail -20`,
   `cargo fmt --check`.  Fix failures locally in the merged code.
7. Regenerate the ledger (command in AGENTS.md), run `python3 scripts/record_skips.py --wave $ARGUMENTS`, check `git status --short`
   has no `src/wave$ARGUMENTS/` or `tests/wave$ARGUMENTS_*`.
8. Append the wave line to `engine-rust/docs/WAVES.md`, update the state
   table in `AGENTS.md`, commit on the current branch.
9. Report in 10 lines: translated count, ledger before/after, tests, dropped agents.
   Every second wave, propose an integration wave (guide §5).
