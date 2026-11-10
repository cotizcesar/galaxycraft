# Game-type rules recovered from the shipped `triggers/*.trg`

The retail client does not hard-code Greed/Slaughter/Sudden Death in a
single C switch: it loads a small trigger file for the selected type.  The
files are in the installation's `triggers/` CASC directory; they are
standard 2,400-byte `TRIG` records, so the same
`crate::trigger::TriggerSystem` can read them.

Extracted with `sc_dump --include triggers/...` from the local install
(Blizzard data, not committed).  The relevant fields, decoded with the
trigger layout in `src/trigger.rs`:

## `Greed10000.trg` (4 records)

| Trigger | Condition | Action |
|---|---|---|
| 0 | Command at most 0 (any unit) for all players | Defeat |
| 1 | Opponents at most 1 | Victory |
| 2 | Always | Set resources to 50 (minerals) for all players; Leaderboard Resources 10000 |
| 3 | Accumulate **minerals at least 10000** and **gas at least 10000** | Victory |

So Greed's goal is 10,000 of **each** resource, not 10,000 combined.  The
2,500/5,000/7,500 variants are the same file with the two quantities
changed.

## `Slaughter30.trg` (4 records)

| Trigger | Condition | Action |
|---|---|---|
| 0 | Elapsed time at least 0 | Set countdown 1800 s; leaderboard goal (kills, any unit); set switch |
| 1 | Countdown at most 0; kill most (any unit); switch set | Victory |
| 2 | Command at most 0 | Defeat |
| 3 | Opponents at most 1 | Victory |

The default variant is 30 minutes; `Slaughter15/45/60.trg` change the
countdown.  The comparison is **kills**, not kills plus razings.

## `SuddenDeath.trg` (6 records)

Five triggers match `Deaths at least 1` of Hatchery (131), Lair (132),
Hive (133), Command Center (106) and Nexus (154).  Each action kills every
unit of the player and defeats it.  The sixth trigger is the usual
"opponents at most 1 -> Victory".

The reconstruction applies the same rule in `World::kill_unit` when
`World::sudden_death` is set; the death counter is not used because the
simulation marks the owner defeated directly.

## `CTF.trg` (4 records)

| Trigger | Condition | Action |
|---|---|---|
| 0 | Command at most 0 (any unit) | Defeat |
| 1 | Opponents at most 1 | Victory |
| 2 | Elapsed time at least 601 s; command at most 0 **Flag** (unit 215) | Kill all units; Defeat |
| 3 | Elapsed time at least 0 | Set countdown 600 s |

The Flag unit is `units.dat` id 215 (`stat_txt` "Flag"), and the beacons
are ids 197-199 (`Zerg/Terran/Protoss Flag Beacon`).  The countdown is the
10-minute window in which a beacon may be relocated.  The reconstruction
models the flags as `CtfFlag` state, uses workers for pickup, and draws a
pennant over the carrier; the beacon move button is `0x74d5a0`
(`Act::PlaceBeacon`).

## Evidence limits

The `.trg` files describe the rules but the engine-side flag pickup and
carrier handling are not in them (the trigger VM only observes the Flag
unit).  The reconstruction therefore implements the behaviour described by
the retail documentation and the trigger conditions, not a decompiled
`CUnit` flag routine.
