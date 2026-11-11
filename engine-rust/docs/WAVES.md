# Wave log

One line per merged wave.  Append; do not rewrite history.  Format:
`Wave W (date, mode): +N translated (ledger A -> B), lib tests T, notes`.

- Wave 13 (2026-11-10, window): +174 translated (ledger 2444 -> 2620), modules render/runtime/ai_recovered/sim/trigger/client.
- Wave 14 (2026-11-11, window): +181 translated (ledger 2620 -> 2801), lib tests 2802.
- 2026-11-11: switched to triage queue mode (`scripts/triage_corpus.py`); next wave 15 starts from tier 1.
- Wave 15 (2026-11-11, queue, tier 1): +72 translated (ledger 2801 -> 2873), lib tests 2856, full suite 3132; 128 rows to skiplist.tsv.
- Wave 16 (2026-11-11, queue, tier 1): +84 translated (ledger 2873 -> 2957), lib tests 2908, full suite 3184; 116 rows to skiplist.tsv (244 total).
- Wave 17 (2026-11-11, queue, tiers 1-2): +93 translated (ledger 2957 -> 3050), lib tests 2958, full suite 3235; 107 rows to skiplist.tsv (351 total).
- Wave 18 (2026-11-11, queue, tier 2): +80 translated (ledger 3050 -> 3130), lib tests 3005, full suite 3282; 120 rows to skiplist.tsv (471 total).  Agent 04 got 20 static-initialiser stubs (0 translated).
- Wave 19 (2026-11-11, queue, tier 2): +86 translated (ledger 3130 -> 3216), lib tests 3060, full suite 3337; 114 rows to skiplist.tsv (585 total).  Two network-provider helpers (Battle.net ids) were dropped per the bnet rule.
- Wave 20 (2026-11-11, queue, tier 2): +92 translated (ledger 3216 -> 3308), lib tests 3111, full suite 3388; 108 rows to skiplist.tsv (693 total).
