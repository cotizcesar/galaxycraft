# Xbox controller — StarCraft 1 scheme

The controller backend lives in `src/bin/sc_view/gamepad.rs`; the in-game
actions are routed through `App::handle_pad_input` in `src/bin/sc_view.rs`.
The simulation never sees a gamepad: every action enters through the same
selection, command-card and order entry points as the mouse/keyboard.

## Mapping

| Input | Action |
|---|---|
| Left stick | Virtual cursor (mouse pointer) |
| Right stick | Camera pan (arrow keys) |
| A | Select; hold = box selection; double tap = every unit of that type on screen |
| X | Smart order (move / attack / gather) |
| Y | Attack-move |
| B | Cancel; with a production building selected, removes the newest queued item |
| LT | Add/remove from selection; queue waypoints (`Shift`) |
| RT + X / Y / B | Command-card row 1, slots 0 / 1 / 2 |
| RT + D-pad ← / ↑ / → | Command-card row 2, slots 3 / 4 / 5 |
| RT + D-pad ↓ / A / RB | Command-card row 3, slots 6 / 7 / 8 |
| LB + D-pad / A / B / X / Y | Groups 1–8; hold to assign, tap to recall, double tap to centre |
| D-pad ↓ / ← / → | Saved cameras (base / expansion / front) |
| LT + D-pad ↓ / ← / → | Save the current view to that camera |
| D-pad ↑ | Last damage alert (falls back to the last transmission) |
| RB held | Confine the virtual cursor to the minimap; A jumps the camera there |
| Start | Game menu |

The command card is positional, so it works with every unit without
per-unit letter hotkeys: remap the nine card cells to
`Q W E / A S D / Z X C` in Remastered and the controller grid matches the
keyboard grid.

## StarCraft 1 selection rules

- At most **12 mobile units**; a **building is selected alone**.  The rules
  are enforced for mouse click, Shift click, box selection, double click,
  control groups and the controller paths by `App::select_unit`,
  `App::select_box_region` and `App::recall_group`.
- Clicking empty ground clears the selection.
- **LT + command** appends a waypoint instead of replacing the current
  order.  This is `RecAct::QueueOrder` (tag 23), so queued paths are also
  replay/LAN deterministic.
- Groups store at most 12 ids; recalling a group drops dead members and
  never mixes a building with mobile units.
- `B` removes the newest **queued** item and refunds it; the item currently
  being built keeps building.  It is a recorded replay/LAN action
  (`RecAct::Dequeue`, tag 22) so saves, replays and lockstep stay identical.

## Virtual cursor

`App::pad_control` is set on the first controller input.  While it is set:

- `App::system_cursor_visible` hides the operating-system arrow;
- `App::draw_cursor` draws the retail cursor art (or the fallback arrow) at
  `App::pad_cursor`;
- moving the physical mouse takes control back (`App::pad_suppressed`), and
  the next controller input after the pad goes neutral takes over again.

## Integrated translation batches

The gameplay batches are no longer separate `agentNN.rs` files: the 32
gameplay batches were merged directly into `src/sim.rs` (and the shared
weapon-repetition helper is reachable from `combat.rs`).  The remaining
runtime/client translations live in `src/runtime.rs`, `src/ai_recovered.rs`
and `src/client.rs`; `src/translated/` no longer exists.  In particular:

- `a02_is_resource_node_type` (`0x005ada00`) backs `sim::is_resource_type`.
- `a30_cached_whole_hp` (`0x005b2570`) supplies the status-panel HP.

## Verification

```
cargo test --lib:                    2426 passed
cargo test --bin sc_view:              87 passed
cargo test --test reconstruction:      76 passed
cargo test --test replay:               5 passed
cargo clippy --all-targets -- -D warnings: clean
cargo fmt --check:                    clean
python3 scripts/virtual_gamepad_test.py --seconds 7:
  PASS active frames
  PASS menu Enter (A)
  PASS primary A
  PASS smart order X
  PASS command slot RT+X
  PASS group LB+A
  PASS save camera LT+D-pad
  PASS D-pad camera
  PASS minimap RB
  PASS south held
  PASS camera pan
```

The virtual-pad run exercises the real kernel evdev -> gilrs -> `map_state`
path.  The per-unit hotkey letters still need the in-game Remastered check;
the controller grid is positional and does not depend on them.

## Manual acceptance required

The in-game confirmation of each unit's hotkey letter and the Remastered
`Q W E / A S D / Z X C` remap cannot be automated from this environment: it
requires launching the user's retail StarCraft 1 / Remastered and observing
the command card.  The controller implementation does not depend on those
letters because RT addresses the nine card cells positionally; the remaining
check is a user acceptance step, not a code gap.
