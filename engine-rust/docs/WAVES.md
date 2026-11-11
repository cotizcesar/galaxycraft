# Wave log

One line per merged wave.  Append; do not rewrite history.  Format:
`Wave W (date, mode): +N translated (ledger A -> B), lib tests T, notes`.

- Wave 13 (2026-11-10, window): +174 translated (ledger 2444 -> 2620), modules render/runtime/ai_recovered/sim/trigger/client.
- Wave 14 (2026-11-11, window): +181 translated (ledger 2620 -> 2801), lib tests 2802.
- 2026-11-11: switched to triage queue mode (`scripts/triage_corpus.py`); next wave 15 starts from tier 1.
- Wave 15 (2026-11-11, queue, tier 1): +72 translated (ledger 2801 -> 2873), lib tests 2856, full suite 3132; 128 rows to skiplist.tsv.
- Wave 16 (2026-11-11, queue, tier 1): +84 translated (ledger 2873 -> 2957), lib tests 2908, full suite 3184; 116 rows to skiplist.tsv (244 total).
