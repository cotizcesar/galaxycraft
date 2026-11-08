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
  economy, production, tech, spells, triggers and campaign AI.
- Playable Remastered-style front end: menus, campaign flow, briefings,
  ready room, HUD per race, minimap, hotkeys, fog of war and score screens.
- Campaign, skirmish against the AI, saves and replays.
- LAN multiplayer: command-only lockstep over TCP or UDP, discovery, lobby
  and resync.
- Window sizes from 1280x720 through 4K, 16:9 and ultrawide pillarboxing.
- Retail data is parsed at runtime: the client never writes to the install.

## Tests

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
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

## Repository layout

| Path | Contents |
|---|---|
| `engine-rust/src/` | Rules engine, renderer, client and command-line tools |
| `engine-rust/tests/` | Unit and integration tests |
| `engine-rust/third_party/` | Vendored minifb and PKWARE DCL decoder |
| `engine-rust/packaging/` | Desktop entry, AppStream metadata, Flatpak manifest and macOS plist |
| `engine-rust/fixtures/` | Fixture generator documentation; only synthetic data is tracked |
| `engine-rust/tools/` | Fixture, oracle and table-inspection helpers |
| `scripts/` | Clean staging and release packaging scripts |

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
