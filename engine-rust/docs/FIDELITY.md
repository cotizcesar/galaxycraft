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
