# Scrolling performance

How the client's frame time is measured and what it was on 2026-11-13.

## Tools

- `GC_BENCH=N` (with `--shot … --shot-screen game --shot-map M`): N frames
  of update + draw, headless.  `GC_BENCH_SCROLL=1` holds the pointer at a
  screen edge, `GC_BENCH_SCROLL=pad` the controller's right stick (both
  reverse every 120 frames); `GC_BENCH_SLOW=1` lists the frames over
  16.7 ms; `GC_PROFILE=1` prints the draw phases of every frame.
- `GC_FRAME_LOG=path`: the live window loop writes one line per frame
  (`ms camera_x camera_y edge in_menu`), for measuring a real session.

## Real window, 3440x1440 at 60 Hz (Hyprland, keyboard arrows)

Game started with `--start-screen game --shot-map 4`, the arrow keys held
5 s each through `wtype`.

| mode | scrolling | camera still |
|---|---|---|
| HD | mean 14.2 ms (70 fps), p99 18.0 ms, worst 23.6 ms | mean 13.7 ms, p99 17.1 ms |
| SD | mean 12.0 ms (83 fps), p99 14.8 ms | mean 11.8 ms |

Mouse edge scrolling could not be driven this way: a cursor warped with
`hyprctl dispatch movecursor` does not reach the window as pointer motion.

## Hitches removed

| cause | cost | fix |
|---|---|---|
| Minimap terrain redrawn with the fog | 41 ms every 8 ticks | terrain cached, redrawn only where creep changed (`sc_view/minimap_cache.rs`) |
| Lit HD sprites re-baked every frame of a scroll (eye bucket) | ~1.5 ms each, every frame | no re-bakes while the view moves; at most 2 a frame when still (`hd_sprites.rs`) |
| HD terrain megatiles decoded on first sight | up to 5 ms a frame at 1440p | all of a map's megatiles decoded on its first frame |
| Portrait clips / stills decoded on the frame | ~55 ms | worker thread in live games (`sc_view/portrait_panel.rs`) |
| Next music track decoded on the frame | ~400 ms at 32 s, then every track | decoded ahead on a worker thread (`sc_view/sounds.rs`) |
