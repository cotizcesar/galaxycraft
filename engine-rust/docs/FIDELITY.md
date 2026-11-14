# Engine fidelity audit (2026-11-11)

Method: play a real Remastered replay through the engine and note where the
engine stops agreeing with the recorded game.  Tool: `sc_retailrep`
(`cargo run --bin sc_retailrep -- REPLAY.rep --install "$SC_INSTALL"`), built
on the `retail_replay` reader.

A retail replay holds the human players' commands, a header and the map; it has
**no per-tick state and no AI commands**.  The oracle is therefore weak:
commands that cannot be executed, economy that does not allow a recorded
purchase, and the game length.  Replays also log commands the game rejected
(the player spamming "train SCV" with 0 minerals), so such failures are noise.

Replay used: `Terran vs Zerg (BW).rep` — map Challenger 96x128, LinKa (Terran,
human) against a Zerg computer, 25 437 frames, 621 action blocks.

## What matches

- **Container and log decoding.**  Header (633 B), map and action log decode
  to exactly the announced lengths; frames are monotonic and end inside the
  header length.
- **Action sizes** (bytes after `[player][id]`), all consistent with the log
  ending exactly at its last byte: build `0x0c` 7, train `0x1f` 2, cancel
  train `0x20` 2, hotkey `0x13` 2, hold `0x2b` 1, siege `0x26` 1, research
  `0x30` 1, upgrade `0x32` 1, lift `0x2f` 4, select/add/remove `0x63/0x64/0x65`
  `1 + 4n`, right click `0x60` 11, targeted order `0x61` 12, `0x62` 4.
  A frame block can hold several actions (deselect followed by select).
- **Unit tags.**  `tag = 11595 - creation index`, where the index counts every
  record of the map's UNIT section (start locations included) and then the
  melee starting units in player order.  Confirmed by cross-checking six
  right-click targets against the UNIT positions, and by the first selection
  (the four starting SCVs).
- **Start locations are assigned at game start** (random in this replay): the
  human started at the bottom location although the map lists it second.  The
  driver gives player 0 the location nearest its first command.
- **Opening economy**: the first SCV training and mining orders are accepted.

## Differences found

| # | Where | What | Status |
|---|---|---|---|
| 1 | `World::find_build_spot` (gas buildings) | Returned the *first* free geyser in the unit list, not the one at the cursor, so a refinery could only be placed on that one geyser; an under-construction refinery did not occupy its geyser. | **Fixed** (nearest free geyser; incomplete refinery occupies it), test `refinery_placement_tests`. |
| 2 | `retail_replay` | The legacy container reader returned a map 2 747 bytes too long and no commands for container 0x8909. | **Fixed** (structured reader, PR #25). |
| 3 | Unit tags after the first in-game creation | Retail allocates tags from one global slot list (recycled slots bump the generation) that the Zerg AI also consumes, so tags of units created after the start cannot be derived. Mitigation in `sc_retailrep`: **context binding** (an unknown tag in a selection is bound to the engine unit that fits the next command: producer for train, worker for build/gather, any mobile unit for a move; enemy targets by click position).  17 tags bound in this replay. | Mitigated; not exact (tags with generation 9 and 13 occur from frame ~17 400). |
| 4 | Engine economy vs the recorded game | The recorded Barracks (frame 1974, 150 minerals) and Supply Depot cannot start in the engine: the SCV arrives with 114 minerals and `order_build` drops the order silently.  By the first builds the engine is behind by **118 minerals** in total (the driver tops up builds and counts the gap).  Mining is calibrated to recordings (Wasteland -1.7 %), so the gap may come from which SCV trains the player's spam got accepted (the log also holds rejected commands), not from the mining model. | Open; the replay alone cannot tell income from acceptance. |
| 4b | Worker patch hopping | Traced SCV #68 from the opening mining order (frame 78): it retargets its mineral patch three times (62, 64, 63, 65) because the nearer patches are occupied and only starts mining at frame ~240; the engine has mined 40 minerals by frame 500 with four workers.  The rule in `sim.rs` (`free_mineral_near`) is OpenBW's `find_mineral_field(.., 32 * 8, true)`, so this is **probably faithful** and not the cause of #4; the retail order itself (`MoveToMinerals`, inside the non-decompilable range around `0x005ac85d`) has not been read. | Unresolved; do not change without evidence. |
| 5 | Silent build abort | `order_build` ends the order without any error when the money is short on arrival; retail shows "Not enough minerals" and keeps the SCV. | Open (behaviour to verify on a recording). |
| 6 | Requirements | After the topped-up builds, `train Firebat/Medic in Barracks` fails with `MissingRequirements` at frame ~7 900 and ~12 200 (Academy not finished or not built in the engine) and `research`/`upgrade` fail the same way (frames 6 485, 11 515). | Open: needs the build state at those frames. |
| 7 | Supply | `train Marine` fails with `NotEnoughSupply` from frame 4 527: the recorded depots do not all exist in the engine. | Open (consequence of #4/#5). |
| 8 | Computer opponent | `SimpleAi` is not the retail melee AI, so the Zerg side differs from the first creation on.  Nothing in the replay constrains it. | Open (out of reach with replays alone). |

Result of the last run: 17 tags bound by context, 238 failed commands of ~1 000 executed or tried
(most are cascades of #4-#7), the replay's last action at frame 25 387 and the
header length 25 437 are reached, no winner.

## Not verified

Combat, pathfinding and spells during the game: the replay offers no state to
compare against, and #3 blocks executing the later commands.  Executed: select/add/remove, hotkeys, build, train, cancel train, right
click, targeted order (move / attack-move), hold, siege, lift, research,
upgrade.  Decoded but not executed: `0x28`, `0x33` cancel upgrade, `0x62`.

## Next steps

1. Check #4/#5 against a short retail recording or replay with a known mineral
   count at the first build (a replay of the first 3 minutes is enough).
2. Make `order_build` report "not enough minerals" like retail once verified.
3. More replays: short, controlled ones (combat, tanks, pathfinding) record far
   fewer AI interleavings and isolate rule differences better than a 17-minute
   game.

## Unit by unit: Terran SCV (2026-11-12)

Checked against the retail data (iscript 84, `weapons.dat` rows 13 / 14,
`flingy.dat`, `images.dat`) and OpenBW's worker rules; tests in
`tests/scv_retail.rs`.

| aspect | retail | engine |
|---|---|---|
| walking | Walking anim, `imgol 249` thruster glow; 32 directions, the western half mirrored | HD matches (`tests/unit_glow.rs`); **fixed in SD**: the 17 direction frames were spread over the whole circle without mirroring (a unit heading west showed a wrong direction), and the glow was not drawn |
| mining minerals | AlmostBuilt loop: frame 34, `useweapon 14`, frame 17; spark image 526 appears 10 px ahead; cutter sounds 35..39 | **fixed**: the animator runs AlmostBuilt while `gather_timer > 0`; the worker faces its field |
| cargo in hands | overlay image: OreChunk 397 / 398, GasTank 403 / 404 (Orb 399, Sac 401 for the other races) until delivery | **fixed**: `Animator::carry_overlays`, drawn in SD and HD |
| gas | worker hidden for 37 frames inside the building | **fixed**: `World::inside_gas_building` (not drawn, not selectable, not visible to enemies, no collision) |
| building / repair | AlmostBuilt loop with sparks, facing the structure | **fixed**: same animation path (replaces the HD-only fake weld) |
| attack | Fusion Cutter: 5 damage, cooldown 15, range 10 px, GndAttkInit/Rpt | **fixed** (three engine bugs, all units): weapon range was measured between centres instead of between unit boxes, so a 10 px weapon never reached; scripts that end a swing with `gotorepeatattk` and an idle loop (SCV, Zergling) never attacked twice; a unit already in reach kept walking into its target |
| speed | flingy top speed 1280, acceleration 67, turn 40; no speed upgrade | **fixed**: Metabolic Boost was applied to every worker; it is Zergling-only |
| sight | 7 tiles | matches |
| collision | a gathering worker collides only with buildings | **fixed**: `World::is_gathering` units are skipped by `resolve_collisions` |


| aspect | retail | engine |
|---|---|---|
| gas exit | the worker reappears on the building's edge towards its depot | **fixed**: `World::place_at_gas_exit` |
| building stages | scaffold frames 0 / 1 / 2 then AlmostBuilt; the Refinery uses its own frames 1 / 2 / 3 / 4 | **fixed**: SD now draws the stages too, and the Refinery (construction image 0) runs them on its own image |

Checked on screen (HD and `--sd`, scripted sessions on a melee map): mining
pose and spark, mineral chunk and gas tank, worker hidden inside the
Refinery and its exit side, welding with sparks, the construction stages.

Measured rather than derived: the mining dwell is the 93 frames recorded
from the retail game (75-frame timer plus the wait around it,
`tests/economy_original.rs`), and the welding walk follows the Boot Camp
recording (the SCV works the left, front and right edges, changing spot);
retail draws those spots from its RNG, so the exact sequence differs.

## Unit by unit: Marine, Firebat, Ghost, Medic (2026-11-12)

Method: `UNIT-FIDELITY-METHOD.md`.  Tests: `tests/infantry_retail.rs`.
Checked on screen in HD and SD: Marine gunfire, Firebat flame, Medic heal
glow, Ghost cloak fade.

| unit | aspect | retail | engine |
|---|---|---|---|
| all | stats | units.dat / weapons.dat | matches (asserted) |
| all | weapon range | between unit boxes; U-238 Shells +32 (also Grooved Spines, Singularity Charge, Charon Boosters) | **fixed**: the upgrades did nothing; acquisition and attack both use boxes |
| all | Bunker | +64 px from an occupant at the centre; SCVs, Civilians and heroes may enter; an SCV does not fire | **fixed** (+32 from centre, last target scanned, four types only) |
| all | voices | 1 ready, 4 what, 4 yes, 7 annoyed lines (Ghost 4); annoyed lines after repeated clicks | matches (asserted against sfxdata.tbl) |
| Marine / Firebat | Stim Pack | 10 hp, refused at 10 hp or fewer, 37 x 8 frames, heroes too, sound 278 / 279 | **fixed** (allowed at any hp, 336 frames, no heroes, no sound, bypassed the network queue) |
| Firebat | attack | `attkshiftproj` 24 / 52 / 80: three flames, each an 8 damage enemy-only splash 15 / 20 / 25 px from the victim's box; flame overlay `imgol 421` | **fixed**: the opcode was ignored (one hit at the target); the invisible `circle14` bullet was drawn as a white disc; a stale path hid the flame |
| Ghost | cloak | 25 energy to cloak, free to decloak, hidden from enemies without a detector | **fixed** (was free; enemies saw it with fog off); drawn faded for players who can see it |
| Ghost | Lockdown | missile (weapons.dat row 32), 131 updates on landing, mechanical only | **fixed** (was instant); field overlay 361..363 |
| Ghost | Nuclear Strike | silo builds the missile (200 / 200, build time 1500); Ghost paints from its sight range; damage max(500, 2/3 hp) in rings 128 / 192 / 256 | **fixed** (missile was free and instant for players, no range, flat 128 px) |
| Medic | heal | 200/256 hp for 100/256 energy per frame, patient within 30 px, one Medic per patient, never itself, not from a transport or Bunker, walks to patients, stops on attack-move | **fixed** (64 px centre range, healed itself, never walked, instant +20 Heal command) |
| Medic | orders | `orderdone` clears a signal bit | **fixed**: the Medic dropped every order when it started to walk |
| Medic | Optical Flare | permanent, sight 2, no detection, any non-building unit | **fixed** (600 frames, no effect on sight, refused air units) |
| Medic | Restoration | removes lockdown, flare, irradiate, plague, ensnare, parasite, acid spores | matches; effect and sound added |
| casters | energy | trained units start with a quarter of the maximum | **fixed** (started full) |
| all (melee) | attack loop | repeat after `gotorepeatattk`; stand still in reach | **fixed** with the SCV |
| all | instant bullets | damage on the bullet script's `domissiledmg`: 2 frames after firing for the Gauss Rifle, C-10 and Fusion Cutter, 7 for the Flame Thrower | **fixed** (`World::instant_hit_delay`, was immediate) |
| Ghost | acquisition | a cloaked Ghost does not auto-attack unless on hold position | **fixed** |
| all | voices | "what" and "yes" lines picked at random, annoyed lines in order | **fixed** (was cycling) |

| Medic | Optical Flare delivery | a bullet that follows its target (weapons.dat row 107) | **fixed** (was instant) |
| Ghost | nuclear missile | a unit that leaves the silo, climbs for 90 frames, is off the map for 250 (WarpIn), falls on the target and explodes (`imgol 428`, `sprol 267`); 8 supply while stored | **fixed** (was a 90-frame countdown and a fast bullet: the bomb landed about 10 s early, with no missile and no explosion) |
| Ghost | nuclear missile motion | starts its climb and its fall from rest, acceleration 33, top speed 8533 (flingy 109) | **fixed** |
| all (SD) | glow images (draw function 9) | drawn through the tileset's remap table (`ofire` / `gfire` / `bfire` / `bexpl`) | **fixed**: SD reads the tables from the install (was a tint approximation) |
| Medic | Restoration delivery | lands with its bullet, 7 frames after the cast | **fixed** (was instant) |
| other | Psionic Storm | 48 px from the victim's box | **fixed** (was centre distance) |

The rules marked OpenBW were read from its source (`bwgame.h`,
`actions.h`) on 2026-11-12, not recalled.

Open doubts are in `pending.md`.

## Unit by unit: Vulture, Siege Tank, Goliath, Wraith, Dropship, Valkyrie, Science Vessel, Battlecruiser (2026-11-12)

Method: `UNIT-FIDELITY-METHOD.md`; rules read from the OpenBW source.
Tests: `tests/vehicles_retail.rs`, `tests/air_retail.rs`.  Checked on screen
in HD and SD: turrets and the deployed tank, the air units fighting.

| unit | aspect | retail / OpenBW | engine |
|---|---|---|---|
| Siege Tank, Goliath | weapons | carried by the turret unit (`units.dat` subunit) | **fixed**: they could not attack at all; the base now uses its turret row's weapons and acquisition range |
| Siege Tank | Siege Mode | deploys for 64 frames, packs up for 63 (iscript 92), takes no orders meanwhile | **fixed** (was instant) |
| Siege Tank | Arclite cannon | minimum range 64 px between boxes; a target inside it is not acquired; radial splash hits own units | **fixed** (the tank locked on a target it could not shoot) |
| Vulture and others | speed upgrades | `update_unit_speed_upgrades`: speed x1.5, acceleration and turn rate x2 (Ion Thrusters, Metabolic Boost, Muscular Augments, Leg Enhancements, Gravitic Drive / Boosters / Thrusters, Pneumatized Carapace, Anabolic Synthesis); Adrenal Glands halves the Zergling's cooldown | **fixed** (none of them did anything) |
| Vulture | laying a mine | drives to within 20 px of the point (`order_PlaceMine`) | **fixed** (10 px by spell, instant at any distance by raw order) |
| Spider Mine | behaviour | 60 frames in the open, 4 to burrow; watches a 96 px square; ignores grounded buildings, flyers and hovering units; chases up to 576 px; explodes within 30 px | **fixed** (armed in 24 frames, 32 px, any ground unit, no chase) |
| all | `attackwith 2` | fires the air weapon | **fixed**: it fired the ground weapon (the Valkyrie never shot; the Wraith lasered air units) |
| Valkyrie | Halo Rockets | 8 per volley around the target (14 offsets); air splash: one full hit, others half within 50 px / a quarter within 100 px, own units spared | **fixed** |
| Dropship | unload | one unit per 15 frames, weapons on cooldown; a Bunker empties at once | **fixed** (everything at once) |
| Dropship | boarding | the transport flies over to fetch a unit that asks to board (`order_PickupTransport`); they board when the boxes touch | **fixed** (the unit walked all the way; 8 px) |
| client | boarding | a unit that boards just disappears | **fixed**: it played its death animation |
| Dropship, Bunker | unload placement | each unit on the nearest free spot to the container (`find_unit_placement`) | **fixed** (all on one exit point) |
| Siege Tank, Goliath | turret heading | turns at the turret's flingy turn rate (13 / 17), fires within the weapon's attack angle | **fixed** (pointed at the target at once) |
| client | Defensive Matrix | front and back shells | **fixed** (none, then front only) |
| Dropship | unload at a point | `order_MoveUnload`: fly there, unload within 16 px; the Unload button asks for the spot | **fixed** (the order did not exist) |
| Battlecruiser | Yamato Gun | charges until its overlay's `castspell`, then a travelling blast, 260 on arrival | **fixed** (instant damage) |
| Science Vessel | EMP Shockwave | a missile; drains the 64 px square at the impact, not the caster or units in stasis; kills hallucinations | **fixed** (instant, circle, caster included) |
| Science Vessel | Irradiate | lands with its bullet | **fixed** (instant) |
| Science Vessel | Defensive Matrix | cast from sight range (10 tiles) | **fixed** (9) |
| Science Vessel, all | detection | a cloaked / burrowed unit under Matrix, Lockdown, Maelstrom, Irradiate, Ensnare, Stasis, Plague or Acid Spores is seen by everyone; detectors must be complete and not disabled; range between boxes | **fixed** |
| Wraith | hero cloak | Tom Kazansky drains 13, the Ghost heroes 10 | **fixed** (free) |
| air units | stacking | units sharing a 48 px cell drift apart (`apply_repulse_field`) | **fixed** (they stacked on one pixel) |
| client | turrets | drawn on top of the base, pointing at the target; the sieged base unfolds | **fixed** in HD and SD (tank without a barrel, Goliath without a torso) |

## Terran buildings and the in-game client (2026-11-12)

Method: `UNIT-FIDELITY-METHOD.md`; rules from the OpenBW source, timings
from the iscripts, placement from `units.dat`.  Tests:
`tests/terran_buildings_retail.rs`.  Checked on screen in HD and SD:
construction of the three scaffold sizes, a Barracks lifting off and
landing, a Factory building, abandoning and re-taking its Machine Shop,
working animations, cursors, cloak, fog memory.

| subject | aspect | retail / OpenBW | engine |
|---|---|---|---|
| lifting buildings | lift-off | rises 42 px at 1 px per tick, iscript `LiftOff` frames, shadow left on the ground, takes no orders until the move and the script's `sigorder` are both done | **fixed** (instant, no animation, no height) |
| lifting buildings | landing | flies to 42 px over the spot, checks the tiles, sets down with `Landing` (Barracks 48 ticks) | **fixed** (instant) |
| lifting buildings | flight speed | 1 px per tick (`flingy_top_speed` is set to 1 and only restored for non-buildings) | **fixed** (5 px per tick) |
| add-ons | position | `units.dat` add-on position: 128, 32 px from the parent's placement box | **fixed** (dimension boxes side by side) |
| add-ons | lift-off | the add-on is abandoned (neutral, no buttons, no research); one under construction is cancelled | **fixed** (kept by the owner, still researching) |
| add-ons | landing | re-attaches only on the exact tiles, for whoever lands | **fixed** (any add-on within 96 px, owner unchanged) |
| production | add-on per producer | Siege Tank: Machine Shop on that Factory; Dropship, Science Vessel, Valkyrie, Battlecruiser: Control Tower on that Starport; nothing trains while the add-on is built | **fixed** (not checked; any add-on of the player counted) |
| command card | add-on buttons | hidden once the building has one; unit buttons greyed without the producer's add-on | **fixed** |
| placement | occupancy | tiles (placement boxes) | **fixed** (sprite boxes: a Factory could not land next to its own Machine Shop) |
| working buildings | `IsWorking` | Barracks door frames, Machine Shop / Physics Lab frames, overlays with blinking lights (`tmprmgraphicstart/end`) while training, researching or upgrading | **fixed** (never played) |
| construction | scaffolds | images 325-330 by stage, Refinery on its own image | matches (checked on screen) |
| cursor | shapes | `MagG/R/Y` over units, `TargN/G/R/Y` while targeting, `Drag` for the box, `Scroll*` at the edges; hotspots from the executable's table | **fixed** (targeting cursor on hover, no drag / scroll cursors, minerals green) |
| cursor | SD | the classic GRP cursors, 128 px canvas centred on the pointer | **fixed** (HD art in SD) |
| camera | edge scroll | the pointer on a screen edge scrolls | **fixed** (missing) |
| SD | selection | ovals 561+ in the relation colour, blinking target, ground marker | **fixed** (a green circle; no marker) |
| status | wireframe | sections turn yellow and red as hit points fall | **fixed** (always green); which section turns first is an INFERENCE |
| selection | other players | a click on an enemy or neutral unit shows its stats | **fixed** |
| fog | memory | scouted enemy buildings stay drawn, darkened, and on the minimap | **fixed** (they vanished) |
| cloak | look | the background shows through, displaced | **changed** from a fade; the shift and tint are tuned by eye |
| cheats | first tick | resources survive the melee start-up trigger | **fixed** |

### Follow-up (2026-11-12)

| subject | aspect | retail / OpenBW | engine |
|---|---|---|---|
| SD | Command Center, Control Tower art | `images.tbl`: `terran\control.grp` is the Command Center, `terran\DryDocks.grp` the Control Tower | **fixed** (CC drawn with `neutral\starbase.grp`, Control Tower with the CC's art) |
| iscript | Burrow / UnBurrow entries | header entries 25 / 26 (23 / 24 are StarEditInit / Disable) | **fixed** constants; the client plays them (Spider Mine dig-in frames 8-11, dirt, sound 354) |
| client | non-directional images | show the iscript frame as is | **fixed** (always frame 0) |
| Spider Mine | hovering units | `units.dat` movement flags 0xC1 | now read from the data (the old list matched) |
| Yamato Gun | energy | spent when the charge starts (`order_CastSpell`); row 113 charges too | **fixed** |
| Valkyrie | rocket spread | per-shooter near-target index | **fixed** (global counter) |
| bullets | caps | 100 bullets; no Halo Rockets once 80 are active | **fixed** (80 for everything) |
| Irradiate | burrowed / loaded | burrowed: only itself; loaded: only its transport | **fixed** |
| SD | team colour | `tunit.pcx` shades per slot colour | **fixed** for slot colours |
| lifting buildings | sounds and dust | iscript `playsnd` 471 / 472; dust at the lift-off / landing LO offsets (`create_dust_sprite`) | **fixed** (none); the mirrored half is not flipped |
| all Terran buildings | construction | scaffold stages 1-3, AlmostBuilt, finished | checked on screen for all 18 in HD and SD (`site:` script step) |
| all Terran buildings | IsWorking | frames (Barracks, Physics Lab, Machine Shop) or overlays (the other nine with one) while working, cleared by WorkingToIdle | tested for all 12 (`every_terran_building_with_is_working_animates_while_it_works`) |
| modularization | equivalence | same state logs; screenshots identical outside the wall-clock portrait (0 differing pixels) | checked across the file-move commits |
| cursor | scale and rate | `FUN_0069df10`: scale `height / 1920`, at least 0.25; every cursor built with the same frame time (10) | **fixed** floor (was 0.02); rate matches |
| lifting buildings | dust mirror | offsets 16-24 drawn mirrored | **fixed** |
| fog | memory | a scouted building stays as last seen; one destroyed out of sight stays as a ghost until its spot is seen | **fixed** (live state drawn; destroyed ones vanished) |
| add-ons | abandoned | neutral: not auto-attacked | **fixed** (enemies attacked it) |
| cursor | scroll arrows | eight edge cursors with edge hotspots | checked on screen in HD and SD (`edge:x:y` script step) |
| Bunker | boarding | within 1 px of the box (`order_EnterTransport`) | **fixed** (8 px): the walker closes the last pixels straight to the box; tested from 8 sides |

## Controls measured in the live retail client (2026-11-13)

StarCraft: Remastered 1.23.10.13515 (Battle.net, Wine/Proton) at
3440x1440, driven with uinput virtual mouse and keyboard devices, read with
screenshots and the game audio (PipeWire monitor).  Results now in the
engine:

| item | method | result | where |
|---|---|---|---|
| Edge / key scroll speed | pointer at an edge or an arrow key held; the minimap view box timed every 0.06-0.2 s; view 853 game px wide, 3244 px of travel = 303 minimap px | 16, 24, 24, 32, 40, 48, 64 game px per 1/60 s for slider notches 1-7 (default 32), mouse and keys alike; real time (Slowest and Fastest game speed agree); no acceleration, starts ~0.1 s after reaching the edge | `sc_view/cursor.rs::scroll_rate` |
| Annoyed lines | an SCV and a Probe clicked 7-18 times 2-4.5 s apart; each recorded line matched (envelope correlation 0.9-1.0) against the unit's `wht` / `pss` files | after selecting the unit: clicks 1-4 "what?", then every annoyed line in order, then 4 "what?" again; a click while the unit is still talking plays nothing and does not count; a 100 s pause keeps the count, selecting another unit resets it | `voice::WHAT_CLICKS`, `sc_view/sounds.rs::play_selected` |
| "What?" take | 80 clicks alternating two SCVs (each click a new selection, so every line is a "what?"), all 80 lines matched (score >= 0.95) | random; the line heard never repeats the one before it (0 of 79) while each SCV's own sequence repeats ~25% (11 and 8 of 39): the take avoided is the last one played, whichever unit said it | `voice::pick_what`, `sc_view/sounds.rs::play_selected` |
| Cloak (HD and SD) | a Dark Templar of the player's photographed undetected, on bare ground, and detected by the player's own Observer at the same spot; then the same in SD (HD Graphics off) | own cloaked units are see-through and undistorted in both modes, the same every frame, whether the player's detectors reach them or not; alpha from the team red over the measured ground: 0.46-0.52 in HD, 0.35-0.45 in SD; the Observer itself (a detector) is drawn opaque | `render::CLOAK_HD_KEEP` / `CLOAK_SD_KEEP`, `App::own_see_through`, `sim` permanent cloak (`units.dat` 0x400000) |

Finding on the way: the simulation did not cloak Observers and Dark
Templar at all (the `units.dat` permanent-cloak flag was not applied), so
enemies could attack them without a detector.

## Rate of fire and damage per hit, every unit (2026-11-14)

Stats, costs, weapons and the damage matrix were already checked against an
independent oracle (`tests/original_parity.rs`: every `units.dat`,
`weapons.dat`, `flingy.dat`, `orders.dat` row and the executable's damage
table).  What the simulation does with them is now checked for every armed
unit (`tests/rate_of_fire_retail.rs`): one unit attacks a held target for
2000 frames.

Three rules from OpenBW were missing, so units fired slower than retail:

| OpenBW | rule | before |
|---|---|---|
| `attack_unit_fire_weapon` | the cooldown starts with the attack animation, not at the iscript's `attackwith` | the animation's lead-in was added to every shot |
| `attack_unit_fire_weapon` | the order timer is cut to `cooldown - 1` while the weapon reloads | the next shot waited for the 9-frame order pass: a Marine fired every 18 frames, not 15 |
| `order_AttackUnit` / `iscript_run_anim` | a ready weapon restarts the attack animation even mid-swing | a long swing set the pace (Ultralisk every 18 frames, Zergling 9.2) |
| `attack_unit_fire_weapon` | each shot adds a random -1..=+2 frames to the cooldown | none |

Now every armed ground and air unit fires every `weapons.dat` cooldown plus
0.5 frames on average (gaps cooldown-1 to cooldown+2); the Valkyrie lands
four rockets per cooldown, the Zealot and Firebat two hits per attack, and the
Devouring One has Adrenal Glands (half cooldown, at least 5).  Every
single-hit weapon deals the executable's damage per hit against a large,
armour-1 target; the flame weapons release three splash flames, the first
in the 50 % ring of a Supply Depot.

A consequence that matches retail: a full-health Zergling now beats a Marine
one on one.

## Spell effects, production times and mining (2026-11-14)

`tests/effects_openbw.rs` casts each spell in the simulation and compares
what it leaves with OpenBW's numbers (status timers count 8-frame updates in
both): Lockdown 131, Stasis Field 131, Ensnare 75, Plague 75 with the weapon
damage / 76 per update (never killing), Maelstrom 22, Defensive Matrix 250
hit points for 168, Irradiate the Irradiate weapon's cooldown, EMP energy and
shields to 0, Hallucination two copies owned by the caster for 1350 frames,
Broodlings 1800 frames, Dark Swarm 900 frames, Disruption Web 360 frames.
Three were wrong and are fixed:

- Broodlings lived 1350 frames: the disassembled retail `0x005cc810` returns
  0x546 (1350) for a hallucination and 0x708 (1800) for a Broodling
  (type 0x28), as OpenBW's `default_remove_timer`;
- Hallucination made one copy, owned by the target's player, that never
  expired (OpenBW `order_CastHallucination`: two, the caster's, 1350 frames);
- Dark Swarm lasted 360 frames, the Disruption Web's time (OpenBW
  `dark_swarm`: 900).

Also checked: Psionic Storm deals eight hits of its weapon (112), Feedback
damage equal to the target's energy and empties it, Consume kills its
target, Restoration clears every effect.  Two more were wrong and are fixed:

- Consume gave twice the victim's hit points in energy (OpenBW
  `consume_unit`: a flat 50, none from a hallucination);
- Restoration refused targets under 61 hit points plus shields, other
  players' units and units with nothing to remove, for players too: that is
  the computer's target choice (`FUN_005c7460`); OpenBW
  `unit_tech_target_valid` only refuses buildings.  Players could not
  restore a Marine.

Second pass (0.1.57), also in `tests/effects_openbw.rs`:

- Recall was reversed: it sent the units around the Arbiter to the point.
  OpenBW `order_Recall` brings the caster's own units inside a 64 px square
  of the point to the Arbiter (not buildings, larvae, eggs, hallucinations,
  burrowed or invincible units), 22 frames after the cast, and they arrive
  idle with no unit still targeting them (0.1.58);
- Mind Control left the Dark Archon's shields; OpenBW empties them, zeroes a
  Dark Archon target's energy and kills a hallucination;
- Consume now takes any own unit that is not a building, Terran or a Larva
  (OpenBW `unit_tech_target_valid`), not the weapon's organic-only flags.

Checked and matching: Optical Flare blinds, Parasite marks, hallucinations
take double damage before armour, Yamato deals the executable's 259 to a
large armour-1 target.  `tests/build_times_retail.rs` also checks every
research (`techdata.dat` time) and the first level of every upgrade
(`upgrades.dat` base time).

`tests/build_times_retail.rs`: every unit trained in a building appears
after its `units.dat` build time (within 2 frames).

Mining: 8 minerals and 8 gas per trip and 37 frames inside a refinery, as
OpenBW.  The mineral dwell is 93 frames, not OpenBW's 75: it is fitted to
the user's retail recordings (Wasteland income within -1.7 %, see
`MINING_FRAMES`).

Third pass (0.1.58):

- Scanner Sweep is now the retail unit (33: sight 10, detector, invincible
  here) that its iscript removes after 156 frames; it revealed a 96 px circle
  for 400 frames.  Tested: a cloaked Ghost is detected during the sweep and
  not after.
- Spider Mines: 3 per Vulture, one laid per order, the blast deals the
  executable's 124 to a large armour-1 target (tested).
- A depleted geyser gives 2 gas per trip forever (OpenBW
  `gather_resource`); ours gave the rest and then nothing, and the workers
  stopped as if a mineral field had run out (tested).
