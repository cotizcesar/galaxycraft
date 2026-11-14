# Scrolling performance

How the client's frame time is measured and what it was on 2026-11-13.

## Tools

- `GC_BENCH=N` (with `--shot … --shot-screen game --shot-map M`): N frames
  of update + draw, headless.  `GC_BENCH_SCROLL=1` holds the pointer at a
  screen edge, `GC_BENCH_SCROLL=pad` the controller's right stick (both
  reverse every 120 frames); `GC_BENCH_SLOW=1` lists the frames over
  16.7 ms; `GC_PROFILE=1` prints the draw phases of every frame.
- `GC_FRAME_LOG=path`: the live window loop writes one line per frame
  (`ms camera_x camera_y edge in_menu`), for measuring a real session
  (`scripts/real_scroll.py` drives one and summarises it).

## Real window, 3440x1440 at 60 Hz (Hyprland)

`scripts/real_scroll.py mouse|pad|keys LOG [--float WxH] [--sd]` opens the client on
map 4 and scrolls for ~30 s with a uinput virtual mouse (pushed to every
edge and corner), a virtual Xbox 360 pad (right stick held each way) or
the arrow keys (`wtype`); the game sees them as real devices.

| input | HD scrolling | SD scrolling |
|---|---|---|
| mouse at the edge | mean 13.9 ms (72 fps), p99 16.7 ms, worst 20.8 ms | 11.6 ms (86 fps), p99 14.4 ms |
| controller stick | mean 14.0 ms (71 fps), p99 17.3 ms, worst 22.0 ms | 12.1 ms (83 fps), p99 16.0 ms |
| arrow keys | mean 14.2 ms (70 fps), p99 18.0 ms, worst 23.6 ms | 12.0 ms (83 fps), p99 14.8 ms |

With the camera still: 13.6-13.8 ms in HD, 11.7-11.8 ms in SD, so
scrolling costs no more than standing still.

## Real window, 2560x1440 HD (0.1.38)

`scripts/real_scroll.py MODE LOG --float 2560x1440` (a floating window of
exactly that size; the pointer stays on it, as Hyprland's focus follows
the mouse):

| input | scrolling | p99 | worst |
|---|---|---|---|
| mouse at the window edge | 11.1 ms (90 fps) | 13.0 ms | 14.8 ms |
| controller stick | 9.0 ms (111 fps) | 13.3 ms | 17.9 ms |
| arrow keys | 7.6 ms (131 fps) | 13.7 ms | 21.9 ms (one frame) |  Before 0.1.36 HD scrolling
ran at 51 fps (19.5 ms) and every game froze ~400 ms at 32 s.

## Hitches removed

| cause | cost | fix |
|---|---|---|
| Minimap terrain redrawn with the fog | 41 ms every 8 ticks | terrain cached, redrawn only where creep changed (`sc_view/minimap_cache.rs`) |
| Lit HD sprites re-baked every frame of a scroll (eye bucket) | ~1.5 ms each, every frame | no re-bakes while the view moves; at most 2 a frame when still (`hd_sprites.rs`) |
| HD terrain megatiles decoded on first sight | up to 5 ms a frame at 1440p | all of a map's megatiles decoded on its first frame |
| Portrait clips / stills decoded on the frame | ~55 ms | worker thread in live games (`sc_view/portrait_panel.rs`) |
| Next music track decoded on the frame | ~400 ms at 32 s, then every track | decoded ahead on a worker thread (`sc_view/sounds.rs`) |

## 8-player skirmish simulation (2026-11-14)

`sc_match --install "$SC_INSTALL" --map Homeworld --races
terran,protoss,zerg,terran,protoss,zerg,terran,protoss --verbose` (eight
computers, 128x128 map).  Before: 600 ticks took 11 s at the start (55
ticks/s) and 51 s by tick 6000 (12 ticks/s), under the 24 ticks/s of
Fastest; a 20 000-tick game did not finish in 20 minutes.  All of it was
the workers' `Gather` order walking to their mineral field:

- every path search scanned every building and mineral field for each grid
  cell it tested (`Obstacles::blocks_rect`); a 128 px spatial index made a
  search ~3x faster;
- the free cell next to a field is often walled in by the other fields and
  the town hall, so the search flooded 120 000 cells before failing; the
  budget now grows with the distance (8000 + 400 per cell, at most 120 000)
  and a failed search still falls back to a direct move;
- a route that ends within 96 px of a goal inside an obstacle is finished in
  a straight line instead of replanned every frame.

After: the first 3000 ticks in 5 s (the economy identical tick for tick), a
20 000-tick game with up to ~540 units in 90 s: ~220 ticks/s on average and
~166 ticks/s at the busiest stretch, 7x the speed Fastest needs.

### Frame spikes in the 8-player game (2026-11-14)

`sc_match --verbose` now lists the ticks over 30 ms.  After the fixes above
the same game had 182 of them, the worst 481 ms, mostly on the ticks after an
AI attack wave (every 1200 ticks): each unit of the wave searched its own
~8 ms cross-map route on one frame.  Three changes:

- a path search remembers each grid cell's footprint test (each cell was
  tested up to eight times against terrain and nearby buildings): the whole
  20 000-tick game went from 90 s to 40 s, economy unchanged;
- routes longer than 512 px are shared within a tick by units of one size
  leaving the same 128 px square for the same goal;
- at most 6 new long searches start per tick; a unit over the limit asks again
  next tick (INFERENCE: spreading the work, not a retail rule).

Now 12-14 ticks over 30 ms in a 14-minute game, the worst ~52 ms; the game
simulates in 38 s.
