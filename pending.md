# Pending

Open items found while working on something else.  Remove an entry when it
is fixed.

## Cheats

- **`show me the money` loses its minerals when typed on the first frame of
  a custom game.**  Seen 2026-11-12 with a scripted session on a melee map
  (`sc_view --game-script "cheat:show me the money,ticks:1"`): minerals go
  10050 -> 50 after one tick, gas stays at 10000.  Probable cause (not
  confirmed): the melee start-up trigger (`fixtures/melee.trg`, "set ore to
  50") runs on tick 1 and overwrites the stock.  A cheat typed later keeps
  its resources (checked at tick 50: 10050 minerals still there 200 ticks
  later), so only the first frame is affected.
  Expected: in single player / Play Custom the cheat resources never
  disappear.
- **Cheats must not work in multiplayer.**  Not checked whether the client
  already refuses them in LAN games (`apply_cheat_code` in
  `engine-rust/src/bin/sc_view.rs`).  Expected: enabled in single player and
  Play Custom, ignored in multiplayer, as in retail.

## Infantry (Marine, Firebat, Ghost, Medic), 2026-11-12

Checked against the OpenBW source (`bwgame.h`, `actions.h`) on 2026-11-12:
Stim Packs, range upgrades, Bunker +64, cloak cost and drain, starting
energy, Medic heal rules and search square, Ghost paint range, the nuke's
90 / 45 frame timers and 320 px drop, splash rings measured to unit boxes,
`attkshiftproj`, Lockdown 131, blind / restore.  What is still open:

- **Cloak look.**  Retail distorts cloaked units through the palette; the
  client fades them (`render::CLOAK_KEEP`) in SD and HD.
- **Annoyed-line threshold.**  The client plays three "what?" lines and
  the annoyed ones from the fourth click (`voice::WHAT_CLICKS`, an
  inference); a fan wiki says "five or more" clicks.  Needs a check against
  the real game.
- **Voice lines** for selection and orders are picked at random, from
  memory of the retail behaviour.  OpenBW has no voice logic and a web
  search (2026-11-12) found no source for the exact rule; the annoyed lines
  after repeated clicks are confirmed to exist (Blizzard, "The Sounds of
  Koprulu").

## Vehicles and air units, 2026-11-12

Rules read from the OpenBW source; these parts are simplified or not checked
on screen.

- **Turrets are folded into the base unit.**  The base fires the turret's
  weapon; there is no separate turret unit, so the turret has no turn rate
  of its own (it is drawn pointing at the target at once), its attack
  animation and muzzle overlays (iscript 93 and friends) are not played,
  and the hull is not held still while it fires.
- **Siege Mode.**  The tank does not turn to its default direction before
  deploying, is not destroyed when a building occupies its spot, and the
  sieged turret's unfolding frames are not drawn (only the base's).
- **Spider Mine.**  The hover list is a hardcoded type list rather than the
  `units.dat` movement flags; the burrow / unburrow frames and sounds are
  not drawn; a mine is nudged by collision while it waits.
- **Dropship.**  No "unload at a point" order (`order_MoveUnload`), units
  board from 8 px instead of 1 and the transport does not come to fetch
  them, and unloaded units all appear at the exit position (collision
  spreads them).
- **Valkyrie.**  The near-target offset index is global, not per Valkyrie,
  and the 80-bullet cap on Halo Rockets is not enforced.
- **Yamato Gun.**  Energy is spent when the blast leaves, not when the
  charge starts; the raw order row (113) casts without the charge.
- **Irradiate** on a unit inside a transport or burrowed follows the old
  rule (OpenBW: only the transport, or only the unit itself).
- **Air repulse.**  The map-edge turn and the "cannot be interrupted"
  exemptions are simplified.
- **Not drawn.**  The hover bob of idle flyers (`setvertpos`), and the SD
  tank hull shows a dotted blue outline (its shadow image).
- **Checked on screen only in outline** (scripted HD and SD sessions, small
  crops): the siege deploy, a mine chasing and exploding, the Yamato charge
  and blast, and the Matrix / Irradiate / EMP casts being accepted.  Not
  looked at: the Matrix and Irradiate overlays on their targets, and a
  Dropship unloading (tests only).
