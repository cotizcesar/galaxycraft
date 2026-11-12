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
