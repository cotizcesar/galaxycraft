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
