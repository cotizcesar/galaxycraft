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

Not confirmed against retail; each is marked `INFERENCE` in the code or
described here so it can be checked with a recording or the binary.

- **Nuclear Strike timing.**  The engine waits 90 frames after the Ghost
  starts painting, warns at 45, and the missile falls for about 32 more.
  The retail sequence (launch from the silo, flight, the Ghost holding the
  dot) was not measured.  The missile also should use 8 supply while it
  sits in the silo; the engine does not count it.  Computer players still
  get their missile on demand when their script orders a strike.
- **Ghost paint range.**  The Ghost walks until the target is within its
  sight range (9 tiles, 11 with Ocular Implants); taken from memory of
  OpenBW `order_NukeGround`.
- **Medic search range.**  An idle Medic walks to patients whose box is
  within 160 px; from memory of OpenBW, not measured.
- **Flame Thrower timing.**  The retail flame bullet deals its damage about
  7 frames after it appears (`wait 1, wait 1, playfram, wait 5,
  domissiledmg` in iscript 292); the engine applies it on the frame the
  Firebat's script fires.  Same for the SCV's and Marine's instant bullets
  (`sigorder`, `domissiledmg` in their bullet scripts).
- **Psionic Storm radius** is still measured to unit centres (an older test
  pins 48 px inside / 49 px outside); every other splash now measures to the
  victim's box.
- **Cloak look.**  Retail distorts cloaked units through the palette; the
  client fades them (`render::CLOAK_KEEP`).  Glow images (draw function 9)
  are tinted in SD instead of using the `ofire` / `gfire` / `bfire` tables.
- **Voice order.**  The client cycles through the "what" / "yes" lines in
  order; retail picks them at random.
- **Spell effects in SD** use the palette-free glow approximation; the
  Lockdown hit and Optical Flare projectiles have no travelling image.
