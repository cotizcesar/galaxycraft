# Translation batch integration audit

All 90 batches have been consolidated out of the per-agent staging files.
The 32 gameplay batches live in `src/sim.rs` (with `combat.rs` using the
shared weapon helper), the 20 STL/args/iostream/settings helpers live in
`src/runtime.rs`, the 6 AI-queue/script batches in `src/ai_recovered.rs`, and
the 32 client/UI batches in `src/client.rs`.  The `src/translated/` directory
no longer exists; citations and tests moved with the code.

## Batches physically merged into `src/sim.rs`

01, 02, 03, 04, 05, 06, 07, 08, 09, 10, 11, 12, 13, 14, 18, 19, 20, 21,
22, 23, 27, 28, 29, 30, 31, 32, 33, 35, 36, 37, 38, 39.  Their tests run as
`tests_agentNN` inside `sim.rs`; the ledger citations moved with the code.

## Wired into the general code

| Batch | Address | Translation | Call site |
|---|---|---|---|
| 01 | `0x005a0720` / `0x005a077a` | `a01_weapon_effect_repetitions` | `sim::World::fire`, `combat::Unit::fire` |
| 02 | `0x005ada00` | `a02_is_resource_node_type` | `sim::is_resource_type` / `SimUnit::is_resource` |
| 03 | `0x005b0ba9` | `a03_finish_construction` | structure completion in `World::update_production` |
| 04 | `0x005b49a0` | `a04_status_modifier_count` | `World::speed_of` (Stim / `flag_29` / Ensnare) |
| 06 | `0x005b7c20` | `a06_pop_order_head` | `World::order_done` (current order → next queued order) |
| 08 | `0x005c4da0` | `a08_is_hold_position_idle` | Medic auto-heal idle/hold gate |
| 10 | `0x005d22bf` | `a10_clear_acid_spores_if_present` | `World::kill_unit` removal cleanup |
| 13 | `0x005b0b4b` | `a13_clear_completed_flag` | construction/morph start paths |
| 14 | `0x005b66a0` | `a14_set_addon_link` | add-on parent linking (build, proximity attach, scenario link) |
| 27 | `0x005ad780` | `a27_remove_cargo` | `World::unload_unit` cargo detach |
| 30 | `0x005b2570` | `a30_cached_whole_hp` | status-panel HP readout |
| 33 | `0x005d5ea0` | `a33_cargo_used_space` | transport boarding capacity check |
| 35 | `0x005d834c` | `a35_advance_order_carry` | per-unit order dispatch cadence |
| 37 | `0x005dc1e0` | `a37_spawn_dark_swarm` | `World::spawn_dark_swarm` clamp/status/timer |
| 38 | `0x005dfabc`–`0x005dffe0` | `a38_larva_spot_*`, `a38_larva_spot_bottom_right_half` | `World::larva_spawn_spot` |
| 39 | `0x005e08e0` | `a39_keep_smaller_pair` | side-score fold in `World::larva_spawn_spot` |

The cursor rendering is extracted into `draw_virtual_cursor` and covered by
`cursor_visibility_tests::the_virtual_cursor_renders_its_arrow_at_the_pointer`.

Regression tests that exercise the wired paths from the simulation:

- `sim::tests::order_done_uses_the_batch_06_queue_pop_and_refreshes_the_row`
- `sim::tests::status_modifier_count_drives_the_speed_clamp`
- `sim::tests::larva_spawn_uses_the_lowest_side_score`
- the existing construction, add-on, cargo/unload, medic, resource and combat
  suites.

## Why the remaining gameplay modules are not called

The rules core owns one authoritative representation for each object.  Most
early batches translate a *second* view of the same state (intrusive `CUnit`
lists, `CSprite` fields, raw order nodes, cached status words).  Calling those
functions from the general code would duplicate state and break save/replay
hashing; their recovered arithmetic is already reflected in the model and
the recovered code remains the auditable reference plus regression tests.

| Range | Content | Status |
|---|---|---|
| 01–24 | `CUnit`/`CSprite` lists, orders, attack/selection, overlays | Gameplay helpers; only stateless ones with an authoritative call site are wired (01, 02, 03, 04, 06, 08, 10, 13, 14).  The rest alias fields already owned by `World`/`SimUnit`. |
| 25–29 | Client loader, briefing/dialog objects, copy/destructor fragments | Client/runtime only; `a27_remove_cargo` is wired into `unload_unit`, the rest have no simulation call site. |
| 30–33 | Command-card records, obfuscated order/status tails | `a30_cached_whole_hp` and `a33_cargo_used_space` wired; the rest revisit client card/UI state or fragmented obfuscated blocks. |
| 34–37 | Integrity glue, CRT helpers, placement callbacks, spell/order tails | `a35_advance_order_carry` and `a37_spawn_dark_swarm` wired; the rest are partial or presentation-only. |
| 38–39 | Larva spawn-side scan | Wired. |
| 40 | Settings/string tables and trigger-location records | Client/configuration plumbing. |
| 41–60 | `args`, iostream, STL containers, hash tables, save paths, tilesets | Runtime/UI library code; translated as pure functions and tested, no rules call site. |
| 61–70 | View/zoom, gamma, variant storage, input driver, client decode | Client/UI. |
| 71–80 | Containers, glue refresh, renderer reporting, text/glue caches | Client/UI. |
| 81–83 | AI build queue, client card slots, AI order queue | The rules core does not model these separate persistent records; wiring them would add save/replay state.  Their pure helpers remain available for the campaign-AI layer. |
| 84–85 | Modal UI, anchors, anti-tamper | UI/runtime only. |
| 86–88 | AI script dispatch/regions, script runtime, AI spawn pool | Separate AI-script subsystem; `aiscript.rs` models the script stream with its own tables. |
| 89 | Session-history report | Client report/UI. |
| 90 | Glue text layout, sound lookups | Client/UI. |

## Verification

```
cargo test --lib:                    2426 passed
cargo test --bin sc_view:              87 passed
cargo clippy --all-targets -- -D warnings: clean
cargo fmt --check:                    clean
```

## Wave 13 (2026-11-10)

Ten parallel agents covered `0x0065c000`–`0x0066ffff`.  The 174 exact
ledger additions were merged into natural modules rather than one staging
file:

| window content | module |
|---|---|
| tile pages, DXT1/palette pages, tile-set cache | `src/render.rs` |
| byte/word/record vectors, `TimedScope`, JSON parser, callable/hash helpers | `src/runtime.rs` |
| command dispatch, AI-script opcodes, region queries | `src/ai_recovered.rs` |
| order/shared-amount accounting, target predicates | `src/sim.rs` |
| trigger accumulate-condition selectors and counter tables | `src/trigger.rs` |
| fade state, variant maps, callable/string settings glue, mission-end UI | `src/client.rs` |

All `src/wave13/agentNN.rs` staging files and the isolated harnesses were
deleted after the merge.  The two endpoint getters `0x0066f800` and
`0x0066fd00` landed in a follow-up commit.  Ledger: 2444 → 2620; lib tests:
2613; clippy and fmt clean.
