# Project status (2026-11-11, release 0.1.18)

## What works (shipped, tested)

- **Rules engine** (`sim.rs`, `combat.rs`, `orders.rs`, `iscript.rs`, `path.rs`, `contour.rs`, `vision.rs`, `trigger.rs`, `handlers.rs`): deterministic fixed-tick simulation of units, weapons, orders, movement/pathfinding, economy, production, tech, spells and status effects, map triggers.
- **AI** (`ai.rs`, `aiscript.rs`, `ai_recovered.rs`): campaign AI scripts with the retail region-state defense/attack graph; skirmish against the AI.
- **Game modes**: campaign flow and briefings, skirmish, saves, replays, Greed / Slaughter / Sudden Death / Capture the Flag rules, cheat codes.
- **Multiplayer**: command-only lockstep over TCP/UDP LAN with discovery, lobby and resync (`net.rs`, `multiplayer.rs`, `bin/sc_view/mp.rs`).
- **Client** (`client.rs`, `render.rs`, `audio.rs`, `menu.rs`, `sc_view`): Remastered-style menus, race-specific HUD and ready rooms, minimap, fog of war, score screens, Collections, 13 languages, controller play, options (timer/APM, FPS, VSync, HD/SD, cursor, ...).
- **Packaging**: AppImage, Flatpak, Windows zip, macOS app built by CI on tag.
- **Translation pipeline**: ledger, triage, wave scripts, Workflow/Agent fan-out (see `TRANSLATION-GUIDE.md`).

## Translation progress

5,284 of 56,128 functions (9.4%) translated, tested and cited; 3,147 wave helpers, of which only 385 have a non-test caller.  Breakdown by bucket and module: `CORPUS-STATUS.md`.  History: `WAVES.md`.

## What is missing

1. **Integration of translated helpers.**  Most of the ~3,100 wave helpers are pure reference code (STL containers, CRT, video/audio kernels, UI records) with no call site, by design: the engine keeps one authoritative state.  Wiring a helper is only worthwhile when it replaces an approximation, with determinism/replay tests green (guide §5).
2. **Remaining queue**: 516 clean tier-3 rows (client/render/audio neighbours); 13,170 tier-4 rows only on demand.  Yield is low: most rows are glue ending in non-returning calls.
3. **Skiplist (6,800 rows)**: skipped because of non-returning calls or unmodelled callees; retry only after a needed callee is translated.  Per-row skip reasons were not recorded.
4. **622 non-decompilable functions**: need manual assembly reconstruction or another decompiler.
5. **Battle.net / online services (704 marked rows)**: deliberately out of scope, never implemented.
6. **Phase 4 (front end/campaign data paths)** and **Phase 5 (helpers/CRT/thunks)** of `IMPLEMENTATION-PHASES.md` are open-ended; there is no exhaustive list of gameplay gaps against retail.  Known verification debt: in-game Remastered check of per-unit hotkey letters (`XBOX-CONTROLS.md`), later-campaign recordings for the timeline gate.
7. **Release hygiene**: releases 0.1.3 and 0.1.4 have no packages; 0.1.18 packages depend on CI finishing.
