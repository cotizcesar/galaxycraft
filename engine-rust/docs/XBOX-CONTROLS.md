# Xbox controller — Baldur's Gate 3 pad layout

The controller backend lives in `src/bin/sc_view/gamepad.rs`; the in-game
actions, the radials and the button prompts live in
`src/bin/sc_view/pad_ui.rs` (`App::handle_pad_input`, `draw_pad_radials`,
`draw_pad_prompts`).  The simulation never sees a gamepad: every action
enters through the same selection, command-card and order entry points as
the mouse/keyboard.

The scheme and the on-screen sizes follow the controller interface of
Baldur's Gate 3: bumpers open the action radials, RT the shortcut radial,
LT the party (here: control groups), the D-pad steps through targets.

## Mapping

| Input | Action |
|---|---|
| Left stick | Virtual cursor; picks the slot while a radial is open |
| Right stick | Camera pan; also picks the slot while a radial is open |
| A | Select / confirm; hold = box selection; double tap = every unit of that type on screen |
| B | Cancel / close; with a production building selected, removes the newest queued item |
| X | Context order (move / attack / gather); in the group radial: assign the selection |
| Y | Attack-move |
| LB / RB | Open the action radials; then previous / next radial |
| RT | Toggle the shortcut radial |
| LT tap | Toggle the group radial |
| LT held | Add/remove from selection; queue waypoints (`Shift`) |
| D-pad ← / → | Previous / next control group (recall and centre) |
| D-pad ↑ | Jump to the last alert (falls back to the last transmission) |
| D-pad ↓ | Examine: centre the screen on the selection |
| L3 | Centre on the main base |
| View | Minimap pointer: A jumps the camera there, B / View leaves |
| Menu | Game menu |

## Radials

A radial has the twelve clock slots.  The stick points at one, the D-pad
steps over the filled ones, A uses the focused slot and B closes.

- **Action radials (LB / RB)**: the first radial is the selection's retail
  command card (the commands every selected unit shares).  A lone unit also
  gets one radial per build card its card links to (`Structures`,
  `Advanced Structures`).  The neighbouring radials are drawn dimmed on
  both sides and LB / RB step through them.
- **Shortcut radial (RT)**: game menu, objectives, idle unit, last alert,
  main base, minimap pointer, cameras 1–3 and save camera 1–3.
- **Group radial (LT tap)**: groups 1–9 and 0.  A recalls the group (again
  to centre on it), X stores the current selection in the focused slot.

Geometry, as fractions of the 16:9 content height (measured on a
1920x1080 capture of the reference interface; constants at the top of
`pad_ui.rs`):

| Element | Size |
|---|---|
| Focused radial | outer radius 0.200, inner 0.133 (432 / 288 px wide at 1080p) |
| Neighbour radials | outer 0.128, inner 0.089 (64 %), centres 0.43 apart |
| LB / RB tags | 0.264 left / right of the centre, 0.16 above it |
| Description panel | 0.65 wide, 0.218 below the radial centre |
| Button prompts | right margin 0.051, row pitch 0.048, glyph radius 0.013 |

The reference has no console under the radial, so the radial and its
description panel are lifted until the panel clears the StarCraft console,
and the prompt list rests on top of the command card instead of the
bottom-right corner.

`--game-script` accepts `pad:BUTTON` (`a b x y lb rb rt lt view menu up
down left right`) and `pad:stick:X:Y` (percent), so the radials can be
captured headlessly with `shot:NAME`.

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
