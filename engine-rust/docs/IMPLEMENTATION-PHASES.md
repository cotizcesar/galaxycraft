# Implementing the decompiled corpus by phases

> Status 2026-11-11 (release 0.1.18): 5,284 of 56,128 functions translated by queue-mode waves 15–31 (see `WAVES.md`, `CORPUS-STATUS.md`, `STATUS.md`).  The phase tables below record the early hand-ported work; Phase 5 is largely covered by the wave helpers.

The runtime image has **56,128 functions**; **55,502** have pseudocode
(55,501 clean + 1 warning) and **626** are documented non-decompilable
(`decomp_failures_final.tsv`, with disassembly in
`decomp_failures_disassembly.tsv`).  This is the working plan for turning the
corpus into Rust, in small verifiable phases.  Every phase names the evidence
addresses it consumes and the tests/captures that prove it.

## Phase 0 — inventory and search tools (done)

- Ghidra inventory and per-function corpus in
  `~/.cache/galaxycraft/decomp_full`.
- `scripts/evidence_index.py`: 257 cited code addresses from the old research
  tree, normalized to the current image base.
- `scripts/function_coverage.py`, `scripts/decomp_index.py`,
  `scripts/query_decomp.py`: address → containing function → pseudocode.
- `scripts/decomp_status.py`: inventory, corpus and attempt counts.

## Phase 1 — hidden/config/debug routines (done)

The retail config constructor at `0x006e2653` registers the hidden option keys.
Each key is a small, self-contained behavior with a clear acceptance check.

| Key | Status | Evidence / test |
|---|---|---|
| `ShowFPS` | done | `FPS <n>` overlay; `--show-fps`; `fps_label` test |
| `ShowTurnRate` | done | `TURN <n>` overlay for the first selected flingy; `--show-turn-rate`; `turn_rate_label` test; capture `/tmp/opencode/turn_rate2.png` |
| `ColorCycle` | done | terrain palette cycling (existing test) |
| `MinimapPosition` | done | minimap corner/standard (existing capture) |
| `BonusSkins` | done | PreSale set |
| `MouseScaling`, `HardwareCursor` | done | Speed tab |
| `OriginalAspectRatio`, `MatureLanguageFilter` | done | pillarbox / text filter |
| `ShowFoliage` | done | environment particles |
| `sound_background` | done | mixer pauses when the window loses focus; `AudioMessage::Background` and mixer test |
| `building_sounds` | done | gates structure voices; `building_sound_option_gates_only_structures` |
| `original_1998_campaign` | done | prefers optional `voices/sd/...` classic bank with fallback; `classic_campaign_prefers_the_sd_bank_and_falls_back` |

Phase 1 is complete.  The 1.23.10 CASC install has no classic SD voices, so
`original_1998_campaign` falls back to the current bank there; a classic
install that ships `locales/<locale>/assets/sd/sound/` gets copied to
`voices/sd/` and used automatically.

## Phase 2 — evidence functions not yet cited

At the start of Phase 2 a fresh join of the evidence index against every
`0x…` address in `engine-rust/**` left **129 functions with pseudocode whose
address the Rust tree did not spell out** (five more evidence addresses are
among the 626 non-decompilable bodies).  The table below records the functions
ported since then and the ones already implemented under a different name;
the remaining addresses are triaged in the same way.  Use
`query_decomp.py --address` for the body and `decomp_index.py` for coverage;
add a test and/or capture per routine.

| Evidence | Behavior | Rust status |
|---|---|---|
| `0x6a2e60` | damage-type × unit-size multiplier lookup | implemented: `combat::size_multiplier`, table test |
| `0x5b3be0` | Defensive Matrix pool accessor (`CUnit+0x112`) | implemented: `Status::matrix` |
| `0x59cf90`, `0x59ce30` | damage pipeline (spores, floor, matrix, shields, armour, size, HP) | implemented: `combat::resolve_damage_status` + new spore test |
| `0x5b3ba0` | Acid Spore count (`CUnit+0x126`) | **ported now**: nine independent `0x96`-update timers (one update every 8 ticks), oldest-stack refresh, expiry removes one stack; Corrosive Acid damages one target and spores the 128x128 px box; Restoration clears it and active spores reveal cloaking |
| `0x569a70` | direction-table velocity | implemented: `direction_vector` |
| `0x574890` | `playfram` plus direction offset | implemented: `AnimEvent::Frame` + renderer pose |
| `0x5a0750` | iscript `attack` with an explicit weapon | implemented: `AnimEvent::Attack` |
| `0x5b6900` | Stim/Ensnare speed clamp and hero speeds | implemented: `World::modified_speed` |
| `0x5b6a60/80/90/a0` | four directional `units.dat` box extents read by the path initialiser `FUN_007654a0` | **ported**: `Box16::extents`/`rect_at`; blocker rectangles use `[-left, -top, +right, +bottom]`.  The moving-unit `PathRequest` and build-candidate footprint still use symmetric maxima; that is the next path step |
| `0x5d2300` | Defensive Matrix absorption, break and hit cues | **ported now**: `Status::apply_matrix`/`absorb_matrix`, `matrix_ticks` (`CUnit+0x114`, `0xa8` updates) and `World::matrix_cues`; partial hits keep the timer, the last point or the 0xa8-update expiry clears it and emits the break cue |
| `0x5c7460` | Restoration target heuristic | **ported now**: `World::restoration_worthwhile` and `Status::has_removable_effect` (same owner, >60 whole HP+shields, removable status); Restoration also clears Parasite |
| `0x655140` | order-dispatcher periodic hook | **triaged**: called on the 8-tick carry boundary; its `FUN_0058b650 → FUN_00576ce0` body is one of the 626 non-decompilable routines.  The gameplay effect (Acid Spore expiry) is already covered by `0x5dc940` |
| `0x984770` | trigger/string bounds assertion | **triaged**: the helper aborts on an out-of-range string index; Rust's `StringTable::get`/`text` already return `None`/`""` for the same case |
| `0x75d6b0`, `0x75d660` | region neighbour list and centre lookup | **ported now**: `RegionMap::neighbours_of` and `RegionMap::region_centre_px`; the A* consumes the neighbour lookup and the test checks reciprocal adjacency and `centre_cell * 8` |
| `0x5b5b00` | door types `0xcd..0xd0` | **ported now**: `is_door_type`; doors are excluded from automatic targeting and ground-unit separation |
| `0x7609f0` | contour qsort comparator on the first field | **already covered**: `Contours` sorts by `(pos, start)` |
| `0x75db50` | one-pixel segment normalisation | **already covered**: the contour segment builder widens span ends before filing them |
| `0x7600f0` | swept-rectangle corner test | **already covered**: `Contours::sweep` implements the corner candidates |

The Acid Spore box itself is `0x5dc1c0`, which calls
`0x6c1ee0(x, y, 0x40, 0x5dc360, owner)`: the query builds the rectangle
`[x-0x40, y-0x40, x+0x40, y+0x40]` and invokes the stack callback for each
unit.  `0x5dc940` is the periodic timer update and `0x5dc4a0` clears the
stack on removal.

Next Phase 2 candidates: the remaining trigger/map section readers.  The
short-search helpers `0x7600f0`, `0x75db50`, `0x7609f0` are already covered
by `contour.rs`; `0x655140` and `0x984770` are triaged above.  The raw list
is reproducible from `~/.cache/galaxycraft/phase2_uncited.tsv`.

## Phase 3 — game rules from the pseudocode

Work through the recovered routines that map to `sim.rs`, `handlers.rs`,
`orders.rs`, `iscript.rs`, `aiscript.rs`, `combat.rs`, `path.rs` and
`trigger.rs`.  Prioritize routines that change observable simulation results;
keep the existing determinism/replay tests green after every batch.

| Evidence | Behavior | Rust status |
|---|---|---|
| `0x5d91e0` → `0x5cf290` | per-tick/eighth-tick `CUnit` counters | **ported**: `+0x122` is the 0..7 status counter; `+0x114` matrix 168 updates; `+0x116` Ensnare 75; `+0x117` Lockdown 131; `+0x118` Irradiate 75; `+0x119` Stasis 131; `+0x11a` Plague 75; `+0x124` Maelstrom 22.  `+0x54..0x57`, `+0xe3` and `+0x110` are order/AI/visual retry counters (set by `0x5b6810`, `0x5d9030`, the order handlers), not status durations |
| `0x5d2840` | Defensive Matrix cast: pool 64000 and duration `0xa8` | implemented: `Status::apply_matrix` |
| `0x5d25e3` | Irradiate damage `(250 << 8) / 75` every status update | implemented: `IRRADIATE_UPDATES`, the `weapon(34)` calculation, and the hit-type-9 aura: any non-building unit can carry it, only biological units within 32 px of the source box take damage, burrowed units only take the direct damage, larvae/eggs/buildings are immune |
| `0x5dc2c0` | Plague damage `(300 << 8) / 0x4c`, skipped when it would reach current HP | implemented: `PLAGUE_UPDATES` and the guarded damage tick |
| `0x5b6810` / `+0x110` | temporary-unit lifetime / removal countdown | **mapped**: `SimUnit::expires_at`; `spawn_temporary` now keeps the smaller expiry like the retail setter |
| `+0x54..0x57` | order-state retry timers (7/15/30/60) | **triaged**: they belong to typed order state machines already represented by `Order`, `order_carry`, `gather_timer` and the production timers, not status durations |
| hit types 10/11/12/22 | Ensnare/Plague/Stasis/Maelstrom target filters | **ported now**: area spells skip burrowed units; Stasis and Maelstrom skip buildings; Maelstrom also requires organic; Ensnare affects air but skips buildings, loaded units and the caster |
| `0x5d42c0` | Carrier Attack row 20 launch phase (`CUnit+0x55 = 0x3c`) | **ported now**: `special_cooldown` gates the first interceptor launch for 60 ticks |
| `0x5dfa06` | Hatchery/Lair/Hive larva interval (`0x25` updates) | **ported now**: `LARVA_SPAWN_TICKS = 37 * 8`; the duplicate Hatchery-only decrement was removed |
| orders.dat field 7 vs highlight | queue eligibility vs queue-slot count | **ported now**: `can_be_queued` decides replace-vs-queue; `highlight != -1` decides the eight-slot count |
| `STRx` | Remastered 32-bit string table | **ported now**: `Scenario::parse` prefers `STRx` (u32 count/offsets) over the classic 16-bit `STR`; 48 ladder maps that only carry `STRx` now resolve titles/trigger text |
| transport/Bunker death | loaded cargo must die with the carrier | **ported now**: `kill_unit` marks hidden cargo dead, clears `in_transport` and empties the cargo list; the Bunker reconstruction test pins HP 0 and unlink |

## Phase 4 — front end, campaign and data paths

Port remaining menu/campaign routines (dialogs, score, campaign progress,
CEF data) and verify with headless captures and the existing visual gates.

## Phase 5 — helpers, CRT and thunks

The corpus also contains CRT, thunk and utility routines.  Port only what the
client actually exercises; record the rest as inventory, not dead Rust.

## Excluded

The 626 non-decompilable routines are not silently recreated.  Their
disassembly and provider outcomes are recorded; they need manual assembly
reconstruction or a commercial decompiler.
