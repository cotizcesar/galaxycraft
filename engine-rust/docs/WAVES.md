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
- Wave 21 (2026-11-11, queue, tier 2, 20 agents): +124 translated (ledger 3308 -> 3432), lib tests 3192, full suite 3469; 276 rows to skiplist.tsv (969 total).  Agent 19 got 20 destructor stubs (0 translated).
- Wave 22 (2026-11-11, queue, tier 2, 30 agents requested, 20 concurrent max): +140 translated (ledger 3432 -> 3572), lib tests 3287, full suite 3564; 460 rows to skiplist.tsv (1429 total).  Tier 1 is now empty; yield 23%.
- Wave 23 (2026-11-11, queue, tiers 2-3, 20 agents): +141 translated (ledger 3572 -> 3713), lib tests 3380, full suite 3657; 259 rows to skiplist.tsv (1688 total).
- Wave 24 (2026-11-11, queue, tiers 2-3, 50 agents in batches of <=20 concurrent): +251 translated (ledger 3713 -> 3964), lib tests 3533, full suite 3810; 749 rows to skiplist.tsv (2437 total).
- Wave 25 (2026-11-11, queue, tiers 2-3): 59 of 100 prepared agents launched (max 20 concurrent), +223 translated (ledger 3964 -> 4187), lib tests 3691, full suite 3968; 957 rows to skiplist.tsv (3394 total).  The 41 unlaunched assignments were left in the queue (not skiplisted).  Yield ~19%.
- Wave 26 (2026-11-11, queue, tiers 2-3, 20 agents): +95 translated (ledger 4187 -> 4282), lib tests 3764, full suite 4041; 305 rows to skiplist.tsv (3699 total).
- Wave 27 (2026-11-11, queue, tiers 2-3, 20 agents): +115 translated (ledger 4282 -> 4397), lib tests 3835, full suite 4112; 285 rows to skiplist.tsv (3984 total).
- Wave 28 (2026-11-11, queue, tiers 2-3, 20 agents): +159 translated (ledger 4397 -> 4556), lib tests 3921, full suite 4198; 241 rows to skiplist.tsv (4225 total).
- Wave 29 (2026-11-11, queue, tiers 2-3, 20 agents): +81 translated (ledger 4556 -> 4637), lib tests 3986, full suite 4263; 319 rows to skiplist.tsv (4544 total).  Many rows were Blizzard TACT/CDN code (bnet).
- Wave 30 (2026-11-11, queue, tiers 2-3, 20 agents): +51 translated (ledger 4637 -> 4688), lib tests 4034, full suite 4311; 349 rows to skiplist.tsv (4893 total).  Six agents got only TACT/CDN rows (0 translated); triage_corpus.py now propagates the bnet mark (704 rows).
- Wave 31 (2026-11-11, queue, tiers 2-3, 148 agents: Workflow shards of 28–30 agents plus a relaunch of 11 stalled agents): +596 translated (ledger 4688 -> 5284), lib tests 4395, full suite 4672; 1907 rows to skiplist.tsv (6800 total).  27 agents returned only skips; 3 agent tests and 10 clippy lints fixed by hand after merge.  Tier 2 is now empty.
