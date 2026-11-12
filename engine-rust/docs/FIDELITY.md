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
| 3 | Unit tags after the first in-game creation | Retail allocates tags from one global slot list (recycled slots bump the generation); the engine numbers units sequentially and the Zerg AI interleaves its own creations, so tags of units created after the start cannot be mapped by formula.  Tags with generation 9 and 13 (recycled slots) occur from frame ~17 400. | **Open.**  Needs a context binder (bind a selected unknown tag to the engine unit that fits the command: producer for train, worker for build/gather) or an engine free list that reproduces retail's. |
| 4 | Commands after ~frame 3 300 | Selections resolve to the wrong unit (`train Marine in Zerg Drone`), a consequence of #3, not an independent rule difference. | Open (blocked by #3). |
| 5 | Computer opponent | The engine's `SimpleAi` is not the retail melee AI, so the Zerg side differs from the first creation on.  Nothing in the replay constrains it. | Open (out of reach with replays alone). |

## Not verified

Combat, pathfinding and spells during the game: the replay offers no state to
compare against, and #3 blocks executing the later commands.  Actions decoded
but not executed: `0x20` cancel train, `0x26` siege, `0x28`, `0x2b` hold,
`0x2f` lift, `0x30` research, `0x32` upgrade, `0x33` cancel upgrade, `0x62`.

## Next steps

1. Context binder for unknown tags (fixes #3/#4).
2. Execute the remaining decoded actions.
3. More replays: short, controlled ones (combat, tanks, pathfinding) record far
   fewer AI interleavings and isolate rule differences better than a 17-minute
   game.
