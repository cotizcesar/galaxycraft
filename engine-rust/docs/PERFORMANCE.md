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

`scripts/real_scroll.py mouse|pad|keys LOG [--sd]` opens the client on
map 4 and scrolls for ~30 s with a uinput virtual mouse (pushed to every
edge and corner), a virtual Xbox 360 pad (right stick held each way) or
the arrow keys (`wtype`); the game sees them as real devices.

| input | HD scrolling | SD scrolling |
|---|---|---|
| mouse at the edge | mean 13.9 ms (72 fps), p99 16.7 ms, worst 20.8 ms | 11.6 ms (86 fps), p99 14.4 ms |
| controller stick | mean 14.0 ms (71 fps), p99 17.3 ms, worst 22.0 ms | 12.1 ms (83 fps), p99 16.0 ms |
| arrow keys | mean 14.2 ms (70 fps), p99 18.0 ms, worst 23.6 ms | 12.0 ms (83 fps), p99 14.8 ms |

With the camera still: 13.6-13.8 ms in HD, 11.7-11.8 ms in SD, so
scrolling costs no more than standing still.  Before 0.1.36 HD scrolling
ran at 51 fps (19.5 ms) and every game froze ~400 ms at 32 s.

## Hitches removed

| cause | cost | fix |
|---|---|---|
| Minimap terrain redrawn with the fog | 41 ms every 8 ticks | terrain cached, redrawn only where creep changed (`sc_view/minimap_cache.rs`) |
| Lit HD sprites re-baked every frame of a scroll (eye bucket) | ~1.5 ms each, every frame | no re-bakes while the view moves; at most 2 a frame when still (`hd_sprites.rs`) |
| HD terrain megatiles decoded on first sight | up to 5 ms a frame at 1440p | all of a map's megatiles decoded on its first frame |
| Portrait clips / stills decoded on the frame | ~55 ms | worker thread in live games (`sc_view/portrait_panel.rs`) |
| Next music track decoded on the frame | ~400 ms at 32 s, then every track | decoded ahead on a worker thread (`sc_view/sounds.rs`) |
