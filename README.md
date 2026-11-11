# GalaxyCraft

[![Release](https://img.shields.io/github/v/release/cotizcesar/galaxycraft)](https://github.com/cotizcesar/galaxycraft/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A deterministic Rust reimplementation of StarCraft 1.23 gameplay and menus,
with a playable client (`sc_view`) that reads the player's own
StarCraft: Remastered installation at runtime.

This repository contains **no game assets**. No art, audio, maps, fonts,
extracted tables, retail binaries, decompiled code bodies or verbatim retail
prose are distributed here. See [NOTICE.md](NOTICE.md).

## Download

Prebuilt packages are published on the
[releases page](https://github.com/cotizcesar/galaxycraft/releases):

- Linux AppImage
- Linux Flatpak
- Windows x86_64 zip
- macOS universal app

No game data is included. The client finds a StarCraft: Remastered install in
the platform's default folder, or takes `--install DIR` / `SC_INSTALL`.

```bash
galaxycraft --install "/path/to/StarCraft"
# or
SC_INSTALL="/path/to/StarCraft" galaxycraft

# Pick a shipped locale (`enus`, `frfr`, `dede`, `eses`, `esmx`, `ptbr`,
# `zhcn`, `zhtw`, `kokr`, `jajp`, `plpl`, `ruru`):
galaxycraft --install "/path/to/StarCraft" --language esmx
# or GALAXYCRAFT_LOCALE=esmx galaxycraft
```

## Build from source

Requirements:

- Rust **1.99.1** (pinned by `rust-toolchain.toml`; minimum 1.98.0)
- A C compiler (the build compiles the vendored PKWARE DCL decoder)
- `ffmpeg` for video playback (override with `GALAXYCRAFT_FFMPEG`)
- Your own StarCraft: Remastered installation to play

```bash
git clone https://github.com/cotizcesar/galaxycraft.git
cd galaxycraft

cargo build --release
cargo run --release --bin sc_view -- --install "/path/to/StarCraft"
```

The workspace root builds the `galaxycraft` crate in `engine-rust/`. Build
output stays in `engine-rust/target/`.

Headless frame capture (no window, no audio):

```bash
SC_NO_AUDIO=1 cargo run --release --bin sc_view -- \
  --install "/path/to/StarCraft" \
  --shot /tmp/menu.bmp --shot-screen menu
```

## Features

- Deterministic fixed-tick simulation: units, weapons, orders, movement,
  economy, production, tech, spells, triggers and campaign AI with the
  retail region-state defense/attack graph.
- Complete race command cards: transport load/unload, nuclear strikes,
  cloak/decloak, Archon merges, Zerg morphs and Spawn Larva, Shield Battery,
  Carrier/Reaver hangars and every shipped cancel button.  Transport cargo
  packs 1-, 2- and 4-space units into the shipped selection-panel slots.
- Optional Game Timer and APM plates from the shipped `console_time` /
  `console_apm` art, plus speech/acknowledgment, scroll-speed, HD,
  environment particles, smooth unit turning, shadow stacking, lighting,
  FPS-cap, VSync, pillarbox, SD filtering, HD/SD color cycling, minimap
  position, mouse sensitivity/scaling, hardware cursor, bonus skins, LAN
  port/latency, friend-presence text, hidden ShowFPS, a Linux js fallback
  for physical pads and mature-language filtering wired into the client.
- Playable Remastered-style front end: menus, campaign flow, briefings,
  race-specific Terran/Zerg/Protoss ready rooms, HUD per race, minimap,
  hotkeys, fog of war and score screens.
- Collections: the shipped Carbot unit and structure set can be equipped
  from the Collections screen and applies to every unit, with base art as
  the fallback for HD images the set does not cover.
- Campaign, skirmish against the AI, saves and replays; Greed, Slaughter,
  Sudden Death and Capture the Flag lobby rules; single-player cheat codes.
- Controller play through `gilrs` (virtual cursor, smart commands, camera,
  menus) and 13-language install support via `--language`/`--locale`,
  including the Remastered CEF chrome from `gluall.json`.
- LAN lockstep with `scripts/lan_two_machines.sh`; timeline gate against the
  available retail recordings and a reusable recording-onset extractor for
  new (including later-campaign) footage.
- LAN multiplayer: command-only lockstep over TCP or UDP, discovery, lobby
  and resync.
- Window sizes from 1280x720 through 4K, 16:9 and ultrawide pillarboxing.
- Retail data is parsed at runtime: the client never writes to the install.

## Tests

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# Retail recording timeline gate (Boot Camp, Wasteland, Backwater,
# Desperate Alliance) and the generic onset extractor for new recordings:
python3 scripts/timeline_gate.py --install "$SC_INSTALL"
python3 scripts/recording_text_onsets.py RECORDING.webm
# Later-campaign footage: map index -> [speaker, text prefix, seconds]
python3 scripts/timeline_gate.py --install "$SC_INSTALL" \
  --recordings-json my-recordings.json

# Two-machine LAN check (run host on A, join on B):
scripts/lan_two_machines.sh host
scripts/lan_two_machines.sh join 192.168.1.20

# Controller backend: a physical pad, or a temporary uinput pad:
python3 scripts/virtual_gamepad_test.py --physical
python3 scripts/virtual_gamepad_test.py
```

The full suite reads retail-derived fixtures, which are gitignored and must
be generated locally from your own installation. Tests that need data you do
not have print `skipping: ...` and pass. Setup is documented in
[`engine-rust/fixtures/README.md`](engine-rust/fixtures/README.md):

```bash
export SC_INSTALL="/path/to/StarCraft"
cd engine-rust

cargo run --release --bin sc_assets -- --install "$SC_INSTALL" fixtures --out fixtures
python3 tools/extract_damage_table.py "$SC_INSTALL/x86/StarCraft.exe" fixtures
python3 tools/oracle.py fixtures

cd ..
cargo test
```

## Packaging

The release workflow builds every package from a clean staged tree, so local
fixtures, extracted assets and captures cannot leak into a release. It runs
on tag pushes (`v*`) or manually from GitHub Actions.

```bash
scripts/package_appimage.sh dist
scripts/package_flatpak.sh dist
scripts/package_windows.sh dist   # needs mingw-w64 and the Windows target
scripts/package_macos.sh dist     # runs on macOS
```

See [`.github/workflows/release.yml`](.github/workflows/release.yml) for the
exact toolchain and dependency setup used by CI.

## Decompilation translation

The rules engine is being cross-checked against the retail 1.23.10 code:
Ghidra pseudocode of the player's own executable (kept locally, never
committed) is translated function by function into tested Rust, and the
results are wired into the engine.

| | |
|---|---|
| functions in the retail image | 56,128 |
| translated and cited (waves 1–31) | 5,284 (9.4%) |
| worth translating next (clean tier 3) | 516 |
| clean tier 4 (engine/library glue, only on demand) | 13,170 |
| assigned to a wave and skipped (non-returning calls, unmodelled callees) | 6,800 |
| not worth translating (unwind funclets, fragments, libraries, anti-tamper, thunks, hubs) | ~28,800 |
| Battle.net / online services (marked, deliberately not implemented) | 704 |
| not decompilable with the current toolchain | 622 |

Per-bucket tables: [`engine-rust/docs/CORPUS-STATUS.md`](engine-rust/docs/CORPUS-STATUS.md)
(`python3 scripts/corpus_audit.py` checks that the books balance).
Project status and what is missing: [`engine-rust/docs/STATUS.md`](engine-rust/docs/STATUS.md).

Work goes in waves of 20–150 parallel agents fed from a call-graph priority
queue (`scripts/triage_corpus.py`).  Start with [AGENTS.md](AGENTS.md), then
[`engine-rust/docs/TRANSLATION-GUIDE.md`](engine-rust/docs/TRANSLATION-GUIDE.md);
[`RESUME.md`](RESUME.md) restores the local corpus on a new machine.

```bash
python3 scripts/triage_corpus.py
python3 scripts/prepare_wave.py --wave 32 --queue engine-rust/docs/triage.tsv
# translators (Agent batches or Workflow shards), then:
python3 scripts/validate_wave.py --wave 32
python3 scripts/merge_wave.py --wave 32 --map auto
```

## Repository layout

| Path | Contents |
|---|---|
| `engine-rust/src/` | Rules engine, renderer, client and command-line tools |
| `engine-rust/tests/` | Unit and integration tests |
| `engine-rust/third_party/` | Vendored minifb and PKWARE DCL decoder |
| `engine-rust/packaging/` | Desktop entry, AppStream metadata, Flatpak manifest and macOS plist |
| `engine-rust/fixtures/` | Fixture generator documentation; only synthetic data is tracked |
| `engine-rust/docs/` | Status, corpus status, translation guide, wave log, reverse-engineering evidence and change notes |
| `engine-rust/tools/` | Fixture, oracle and table-inspection helpers |
| `engine-rust/docs/decomp-ledger.tsv`, `triage.tsv` | Per-function translation status and priority queue (addresses only) |
| `scripts/` | Decompilation/wave pipeline, packaging, two-machine LAN and retail-recording timeline tools |
| `AGENTS.md`, `.opencode/` | Instructions, subagent and `/wave` command for coding agents |

## Data and copyright

The project code is independent work. It reads the player's own StarCraft
installation at runtime and never modifies it.

Local files generated from a retail install (fixtures, extracted art, menu
masks, captures) are gitignored. Do not commit or redistribute them. Binaries
built on a machine where those local fixtures exist embed them and are for
personal use only; release packages are built without them.

StarCraft, Brood War and Blizzard Entertainment are trademarks or registered
trademarks of Blizzard Entertainment, Inc. This project is not affiliated
with or endorsed by Blizzard.

## License

Project code is licensed under the [MIT License](LICENSE). Vendored
components keep their own licenses under `engine-rust/third_party/`. FFmpeg,
when bundled in the AppImage or Windows zip, is an LGPL build shipped with
its license as described in [NOTICE.md](NOTICE.md).

StarCraft game data is **not** covered by this license and is not
distributed by this repository.
