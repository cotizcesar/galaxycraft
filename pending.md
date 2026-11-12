# Pending

Open items found while working on something else.  Remove an entry when it
is fixed.

## Infantry (Marine, Firebat, Ghost, Medic), 2026-11-12

Checked against the OpenBW source (`bwgame.h`, `actions.h`) on 2026-11-12:
Stim Packs, range upgrades, Bunker +64, cloak cost and drain, starting
energy, Medic heal rules and search square, Ghost paint range, the nuke's
90 / 45 frame timers and 320 px drop, splash rings measured to unit boxes,
`attkshiftproj`, Lockdown 131, blind / restore.  What is still open:

- **Cloak look.**  The client now shows the background displaced through
  the unit (`Framebuffer::refract_rect`); retail does it through palette
  tables, so the shift and tint are tuned by eye.  In HD the selection
  circle and bar of a cloaked unit are distorted too.
- **Annoyed-line threshold.**  The client plays three "what?" lines and
  the annoyed ones from the fourth click (`voice::WHAT_CLICKS`, an
  inference); a fan wiki says "five or more" clicks.  A second web search
  (2026-11-12) and OpenBW have no number, and the decompiled corpus has no
  readable reference to the units.dat what / annoyed arrays (0x00f903e0,
  0x00f90770): the code that picks them is in the obfuscated part.  Needs
  a check against the real game.
- **Voice lines** for selection and orders are picked at random, from
  memory of the retail behaviour.  OpenBW has no voice logic and a web
  search (2026-11-12) found no source for the exact rule; the annoyed lines
  after repeated clicks are confirmed to exist (Blizzard, "The Sounds of
  Koprulu").

## Vehicles and air units, 2026-11-12

Rules read from the OpenBW source; these parts are simplified or not checked
on screen.

- **Turrets are folded into the base unit.**  The base fires the turret's
  weapon; the turret turns at its own rate, gates the shot and plays its
  firing animation, and the hull holds still while it fires (tested).
- **Spider Mine.**  A mine is nudged by collision while it waits (not
  checked against OpenBW's collision rules for it).
- **Bunker boarding** uses 8 px instead of retail's 1: the engine's
  pathing blocks walkers on a building's tiles (placement box), wider than
  the collision box retail uses, and at 1 px Marines stalled on the
  corners (tested).  Fixing it means modelling building collision by the
  `units.dat` dimensions in the pathfinder.  Transports use 1 px.
- **Air repulse.**  The map-edge turn and the "cannot be interrupted"
  exemptions are simplified.
- **Team colour in SD** uses `tunit.pcx`'s exact shades for the slot
  colours; a lobby-chosen colour still uses median brightness steps.
- **Checked on screen only in outline** (scripted HD and SD sessions, small
  crops): the siege deploy, a mine chasing and exploding, the Yamato charge
  and blast, and the Matrix / Irradiate / EMP casts being accepted; the Matrix
  shell and the Irradiate cloud on their targets (front and back shells).  A
  Dropship loading, flying to a point and unloading was checked on screen.

## Terran buildings and client, 2026-11-12

- **Abandoned add-ons** behave as neutral (no buttons, no research, not
  auto-attacked, yellow circle and cursor, taken by whoever lands on
  them) but keep their owner inside the engine, which has no neutral
  player slot (`World` holds one entry per race); kill credit and score
  may still count them for the old owner.
- **Landing** crushes nothing (OpenBW kills burrowed / immobile units
  under the building and lifts off again if blocked by one).
- **Edge scrolling speed** follows the Mouse Scroll slider by inference.
  (Cursor rates are settled: the loader `FUN_0069df10` gives all eighteen
  cursors the same frame time, 10, the arrow's measured 6 fps.)
- **Wireframe damage**: the order in which sections turn yellow / red is a
  seeded permutation per unit (INFERENCE); retail picks them at random.
