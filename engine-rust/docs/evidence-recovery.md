# Recoverable decompilation evidence

This note answers which evidence survives from the earlier research tree, what
the desktop Trash contained before it was emptied on 2026-11-09, and how to
turn the surviving prose into addresses that drive implementation.  Nothing
here is a substitute for the retail image: every claim below points at a
`0x7a......` runtime address or a `FUN_......` routine recovered from Ghidra
on the unpacked 32-bit client.

## What the Trash contained

`~/.local/share/Trash/files/starcraft-2` was a Craft Anything project, not the
research tree.  It held 10 files (420 KB):

| File | Content |
|---|---|
| `craft.json` | request pointing at `~/Documents/starcraft-re-recomp`, status `stopped` |
| `feed.json` | the attempt stopped after `Couldn't find the program to decompile` |
| `build-activity.log` | 8 seconds from `target triage` to `Paused`; no Ghidra run |
| `agent-session.jsonl` | 20 JSONL records (session setup, no decompilation) |
| `inbox/build-progress.json` | an unchanged-turn counter |
| `capture/README.md` | directory triage of the retail folder |
| `investigation.cany/journal/2026-11-08.jsonl` | two `core_target_triage` calls |

The `investigation.cany*` directories elsewhere in the Trash contain only
journals.  `investigation.cany (3)` records:

```json
{"tool":"project_delete","params":{"confirm":true,
 "project":"/home/cotizcesar/Documents/starcraft-re-recomp/"}}
```

That is a Craft Anything project deletion (2026-11-08T01:39:31Z), not a file
deletion: the 18 GB tree is still present at
`~/Documents/starcraft-re-recomp`.  No Ghidra project, raw dump, rebuilt
`sc_dump.exe`, or decompiled `.c` file existed in the Trash.

The only unique items were 26 retail UI screenshots and a 14 MB UI recording;
they are kept in the cache as visual reference.  The deletion audit and the
file manifest are in [`trash-salvage.md`](trash-salvage.md).

## What actually survives

The knowledge base is `~/Documents/starcraft-re-recomp`:

| Artifact | Size | What it gives |
|---|---:|---|
| `engine-rust/docs/EVIDENCE.md` | 2,127 lines | recovered routines, data tables, damage/order/movement semantics |
| `research/docs/session-log.md` | 7,348 lines | chronological RE notes, falsifications, per-routine algorithms |
| `PROGRESS.md` | 213 lines | feature status and known gaps |
| `research/manifests/` | — | install inventory and SHA-256 manifest |
| `research/tools/` | 21 files | the capture/decompile scripts used for the old dumps |

The old Ghidra database and the old raw image are gone; only the written
evidence survives.  That is why the current work re-captures the live image
and regenerates the complete inventory rather than relying on a saved project.

## Machine-readable index

`scripts/evidence_index.py` extracts every cited address and `FUN_` name with
its section and source line:

```bash
python3 scripts/evidence_index.py \
  --source ~/Documents/starcraft-re-recomp/engine-rust/docs/EVIDENCE.md \
  --source ~/Documents/starcraft-re-recomp/research/docs/session-log.md \
  --json ~/.cache/galaxycraft/evidence_index.json \
  --tsv  ~/.cache/galaxycraft/evidence_index.tsv
```

Current result:

```text
EVIDENCE.md:    174 entries ( 98 code,  76 data)
session-log.md: 337 entries (305 code,  32 data)
total:          511 entries, 257 unique code addresses, 83 unique data addresses
```

Both historical conventions occur and are normalised to the current PE base:

* `EVIDENCE.md` mostly uses the runtime base `0x79fa0000`
  (e.g. `0x7a150d80`).
* The later session log uses the menu dump's flat `0x400000` base
  (e.g. `FUN_00760350`, `0x945050`).

The current Wine capture maps the module at `0x400000`, so the delta is
`0x79ba0000`.  It was verified on the acquisition routine:

```text
0x7a150d80 - 0x79ba0000 = 0x5b0d80
0x5b0d80: movzx eax,WORD PTR [esi*2+0xf8df38]   ; turret table
          movzx eax,BYTE PTR [edx+0xf8f580]     ; ground weapon
          movzx ecx,BYTE PTR [edx+0xf8fd98]     ; acquisition byte
          cmp eax,0x82 / mov eax,[eax*4+0xf93948] / shr eax,0x5
          cmova ecx,eax
```

`0xf8df38`, `0xf8f580`, `0xf8fd98` and `0xf93948` are exactly the runtime
tables listed in `EVIDENCE.md` (`0x7ab2df38`, `0x7ab2f580`, `0x7ab2fd98`,
`0x7ab33948`) after subtracting the same delta.  This fixes the earlier
`0x79c00000` guess, which pointed at unrelated bytes.

The decompiled body of `0x5b0d80` reproduces the recovered rule directly:
the stored acquisition byte is raised to `weapon_max_range >> 5` for the
ground weapon and again for the air weapon (tables `0xf8f580`, `0xf8f750`,
`0xf93948`), with the turret table selecting the unit’s own row.  The
pseudocode is in `~/.cache/galaxycraft/decomp_full/02659_005b0d80_*.c`.

Four addresses from the later `0x400000`-based notes were checked directly
against the current image and all are real function prologues:

```text
0x898c30  push ebp / mov ebp,esp / sub esp,0x9dc   Load Saved Game screen
0x945050  push ebp / mov ebp,esp / sub esp,0x570   main WindowProc
0x760350  push ebp / mov ebp,esp / sub esp,0x3c    contour builder
0x75f290  push ebp / mov ebp,esp / push ecx        free-box helper
```

## Current inventory (2026-11-09)

The live 32-bit client produced a 17,620,992-byte image
(`7dfc23657e770773d6e4d6de52efc241b87ebde5acd06e4892e288caa6307a6e`) and the
analysable PE `~/.cache/galaxycraft/sc32_runtime_pe.bin`
(`70e6779de6b496bd663e10510e159210b48f2c1b0f41049a3c3f6586e5d17738`).
Ghidra 12.1.2, with Function ID, Decompiler Parameter ID, Decompiler Switch
Analysis, Embedded Media and Create Address Tables disabled, finds:

```text
56,128 functions (270 thunks), 56,127 in .text 0x469607..0xd2f390
1 data false positive
```

The project is at `~/galaxycraft-work/ghidra_full`; the inventory is
`~/.cache/galaxycraft/functions_full.tsv`.  The bulk pseudocode pass wrote
**55,501 of 56,128 functions (98.88%)** to
`~/.cache/galaxycraft/decomp_full`; merging the one partial warning body from
the recovery pass gives **55,502 bodies**.  The remaining **626** are the
functions whose control flow cannot be reconstructed.

All **627** original failures were then attempted one at a time:

| Range | Attempted | Warning merged | Failed | Garbage (`halt_baddata`) |
|---|---:|---:|---:|---:|
| large (≥2 KB) | 32 | 1 | 25 | 6 |
| small (<2 KB) | 595 | 0 | 595 | 0 |
| **total** | **627** | **1** | **620** | **6** |

The small functions were run sequentially in chunks of 50 inside one Ghidra
process (`scripts/decompile_small_chunks.py`, 3 s / 100k instructions each):
one decompiler at a time, twelve Ghidra startups instead of 595.  The large
ones used `scripts/decompile_one_by_one.py` with 1M instructions/90 s and a
4–8 GB process-group watchdog.  The watchdog was verified on
`FUN_0057d1e0` (killed at 8 GB after 54 s) and `FUN_008aee30` (timeout at
152 s) without driving the machine above 7 GB RSS.  Per-attempt status is in
`~/.cache/galaxycraft/decomp_attempts_final.tsv`; the 626 remaining failures
and their outcomes are in `~/.cache/galaxycraft/decomp_failures_final.tsv`.

### Alternate decompiler attempts

No available alternate provider produced pseudocode for the 626:

| Provider | Attempt | Result |
|---|---|---|
| Hopper 6.6.2 via REA | full image, 3 GB cgroup, 10 min | analysis did not complete; no document |
| Rizin 0.8.2 + rz-ghidra 0.8 | 5 large + 10 small failures | 0/15 pseudocode; `Symbol var_Nh extends beyond the end of the address space` or no function |
| IDA/Hex-Rays | — | not installed/configured (`REA_IDA_MCP_CONFIG` absent) |

Rizin/rz-ghidra does decompile normal functions (verified on `0x5b0d80`), so
the failure is specific to these routines, not a broken install.  The
disassembly evidence for all 626 is in
`~/.cache/galaxycraft/decomp_failures_disassembly.tsv` (address, size, first
bytes, first ten instructions, provider outcome).  Seven functions do not
even begin with decodable 32-bit code.  This is the practical ceiling with the
installed toolchain; going further requires manual assembly reconstruction or
a commercial decompiler.

The parallel driver copies the project once per worker and balances ranges by
total function bytes:

```bash
scripts/decompile_parallel.sh \
  ~/galaxycraft-work/ghidra_full sc32 sc32_runtime_pe.bin \
  ~/.cache/galaxycraft/decomp_full 8 1 ~/.cache/galaxycraft/functions_full.tsv
```

The per-range manifest records `ok`, `failed` or `skipped`.  Rerun with a
larger timeout (for example `10`) to retry only failed files; successful files
are skipped.  An additional 30-second pass recovered 6 of the 676 first-pass failures;
the final artifact records 627 attempted and 626 remaining hard failures
(`decomp_failures_final.tsv`).

## Coverage workflow

1. `scripts/function_coverage.py --functions functions_full.tsv` reports which
   cited addresses fall inside a discovered function body.
2. `scripts/decompile_all_batches.sh` writes one Ghidra pseudocode file per
   function to `~/.cache/galaxycraft/decomp_full`, resuming by index.
3. `scripts/decomp_index.py` joins the inventory, the pseudocode files and the
   evidence index, so a cited address only counts as covered when its
   containing function has a decompiled `.c` file.
4. `scripts/query_decomp.py --address 0x5b0d80` prints the pseudocode that
   contains an address; `--pattern` searches every decompiled body.  Every
   implementation task therefore starts from a retail routine rather than a
   deterministic re-creation.

## Phase 2 status-stack recovery (2026-11-09)

The Acid Spore count at `CUnit+0x126` is maintained by a nine-slot timer
stack, not a single countdown:

* `FUN_005dc360` increments the count up to nine, picks the first free timer
  slot at `CUnit+0x127..+0x12f`, or the slot closest to expiring when all nine
  are busy, and writes `0x96` (150 updates) into it.
* `FUN_005dc940` decrements every non-zero slot on each status update; each
  expiry decrements the count.  The update runs every eighth logical tick
  (`ACID_SPORE_UPDATE_INTERVAL`), so a stack lasts ~50 s at 24 Hz.
* `FUN_0059ce30` reads the count through `FUN_005b3ba0` and adds
  `count * 0x100` to weapon damage/divisor before the `0x80` floor.
* `FUN_005d22bf` and both Restoration paths clear the whole stack.
* `FUN_005dc1c0` passes a `0x40` half-extent to `FUN_006c1ee0`, which builds
  the rectangle `[x-0x40, y-0x40, x+0x40, y+0x40]` and calls `FUN_005dc360`
  for each unit inside it.  Weapon 104's damage splash radii are all zero, so
  the struck unit takes the damage while the whole 128 x 128 px box gains
  spores.  `FUN_005dc360` skips units of the attacker's own player, buildings,
  loaded units and cloaked units the attacker cannot see; active spores then
  reveal cloaking.

Rust: `Status::add_acid_spore`, `Status::tick_acid_spores`,
`Status::clear_acid_spores`, `ACID_SPORE_TICKS`,
`ACID_SPORE_UPDATE_INTERVAL`, and the `hit::CORROSIVE_ACID` arm in `World`.

### Defensive Matrix (`0x5d2300`)

`FUN_005d2840` casts the spell: it writes 64000 (250 points in 1/256) to
`CUnit+0x112` and `0xa8` to the duration timer at `CUnit+0x114`.  The
eighth-tick status update `FUN_005cf290` decrements that timer; when it
reaches zero it calls `FUN_005d2300(pool)`, which clears the pool and plays
the shield-down cue.  `FUN_005d2300` also takes an already-clamped hit for a
partial absorption; while `+0x114` remains non-zero it plays the matrix hit
cue.  Rust: `Status::apply_matrix` / `Status::absorb_matrix` /
`Status::tick_matrix`, `matrix_ticks`, and `World::matrix_cues`
(`MatrixCue { absorbed, broke }`), cleared at the start of each tick.  The cue
is simulation-side; the client can consume the queue without re-reading a
unit that may have died later in the tick.

### Restoration targeting (`0x5c7460`)

The recovered predicate requires two units of the same owner, a target with
at least `0x3d` (61) whole HP+shields, and at least one removable status.  The
removable set is Parasite, Optical Flare, Ensnare, Plague, Irradiate,
Lockdown, Maelstrom and Acid Spores; Stasis Field is not removed.  The six
`+0x116..+0x124` status offsets are mapped to those timers; the separate
`FUN_0059ca70(0x66, target)` targeting check is approximated by
alive/non-building/non-transport because its exact tech-target flags live in
the data tables.  Rust: `Status::has_removable_effect`,
`World::restoration_worthwhile`, and the Parasite clear in both Restoration
paths.

### CUnit status timers (`0x5d91e0`, `0x5cf290`)

`0x5d91e0` increments the 0..7 counter at `CUnit+0x122`; on the eighth call
it runs `FUN_005cf290`.  That routine decrements byte timers and dispatches
their expiry/periodic handlers:

| Field | Status | Updates | Evidence |
|---|---|---:|---|
| `+0x114` | Defensive Matrix | 168 | `FUN_005d2840` writes `0xa8` |
| `+0x116` | Ensnare | 75 | `FUN_005dc680`/`005dc710` write `0x4b` |
| `+0x117` | Lockdown | 131 | `FUN_005d2900(0x83)` |
| `+0x118` | Irradiate | 75 | `FUN_005d28a0`, `DAT_00f94342` = weapon 34 cooldown |
| `+0x119` | Stasis Field | 131 | `FUN_005bb640(0x83)` |
| `+0x11a` | Plague | 75 | `FUN_005dc6d0` writes `0x4b` |
| `+0x124` | Maelstrom | 22 | `FUN_005d2a80(0x16)` |

Irradiate damage is `(weapon 34 damage << 8) / 75 = 853` in 1/256 points.
Hit type 9 allows any non-building unit to carry the aura, but only
biological non-building, non-larva/egg units within 32 px of the source's
collision box take damage; a mechanical carrier is immune, burrowed units
take only the direct hit, and larvae/eggs/buildings are immune.  Plague
damage is `(weapon 60 damage << 8) / 0x4c = 1010`, skipped when the value is
not below current HP (so the spell alone cannot kill).  The remaining
`+0x54..0x57` are order-state retry timers set by the order handlers
(7/15/30/60), and `+0xe3` is a 15-tick visual/speech timer.  `+0x110` is the
temporary-unit lifetime: `FUN_005b6810` keeps the smaller value and the
expiry path `FUN_005b7f10` removes the unit/orders; Rust maps it to
`SimUnit::expires_at`.

Hit-type filters: Ensnare (10), Plague (11), Stasis (12) and Maelstrom (22)
skip burrowed units; Stasis and Maelstrom skip buildings; Maelstrom also
requires organic.  Rust: the `hit::*` and `Spell::*` area loops.

### Remastered STRx table

`0x7725b0` installs the shared string table; the STRx wrapper `0x70fd90`
selects mode 1, whose layout is a u32 count followed by u32 offsets and
NUL-terminated UTF-8 strings.  Forty-eight cached ladder maps carry `STRx`
with no `STR`, so `Scenario::parse` now prefers STRx and falls back to the
classic 16-bit STR table.  Test: `strx_strings_use_32_bit_offsets`.

### Carrier launch and larva timing

`0x5d42c0` (Carrier Attack row 20) writes `0x3c` to `CUnit+0x55` on entry;
the state waits for it to reach zero before launching the first interceptor.
Rust: `SimUnit::special_cooldown`, set on `Order::AttackUnit` for a Carrier
and checked at the top of `special_attack`.

`0x5dfa06` reloads the Hatchery/Lair/Hive larva timer with `0x25` (37 status
updates, one every eight ticks = 296 logical ticks).  Rust:
`LARVA_SPAWN_TICKS`; the previous 300-tick value and the duplicate Hatchery
decrement in `update_timers` were removed.

### Region lookups (`0x75d6b0`, `0x75d660`)

`FUN_0075d6b0(region, &out)` writes the neighbour-list pointer and returns
its count; the short-path builder walks that list to mark the next region and
its neighbours.  `FUN_0075d660(region, out)` reads the region's fixed-point
centre fields at `+0x44a0c`/`+0x44a10` and returns them shifted right by 8
(pixels).  Rust: `RegionMap::neighbours_of` (recovered append order) and
`RegionMap::region_centre_px` (`centre_cell * 8`); the A* uses the former.

### Door units (`0x5b5b00`)

The predicate returns true for the four map-only door types `0xcd..0xd0`
(left/right upper level doors and left/right pit doors).  Rust:
`is_door_type`; doors are skipped by `collision_extents` (so ground-unit
separation neither pushes nor is pushed by them) and by `acquire_target`.

### Unit box accessors (`0x5b6a60/80/90/a0`)

The path initialiser `FUN_007654a0` reads four `units.dat` words through
`FUN_005b6a80`/`aa0`/`90`/`60` and stores them at path context
`+0x158..+0x15e` as `[-left, -top, right, bottom]`.  `FUN_006c1ee0` and the
build-placement search use those directional extents; a symmetric maximum is
wrong for boxes such as Wraith `(19, 15, 18, 14)` or Bunker `(32, 24, 32,
16)`.  Rust: `Box16::extents`/`Box16::rect_at`, used by `unit_box_rect` in
`World::build_obstacles`, `placement_tiles`, `build_spot_search` and
`land_building`.  The moving-unit `PathRequest` and the build-candidate
footprint still use symmetric maxima and are the next path refinement.

The decompiled bodies stay in the local, gitignored corpus; this ledger keeps
the addresses and the recovered rules.

## Reference captures

The 2026-11-06 multiplayer screenshots and UI recording recovered from the
Trash are listed in
[`reference/multiplayer-2026-11-06.manifest.tsv`](reference/multiplayer-2026-11-06.manifest.tsv)
and live under `~/.cache/galaxycraft/reference/`.  They support the visual
parity checks only; they are not decompilation evidence and are not committed.

The inventory and pseudocode are derived retail metadata and stay outside the
repository; only aggregate counts and the method are committed.
