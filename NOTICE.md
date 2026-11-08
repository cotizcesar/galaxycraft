# Notice — no copyrighted game content

This repository is an independent reimplementation of StarCraft 1.23
gameplay and menu behaviour. The source code is original work. It contains
**no StarCraft game assets and no copied game content**: no art, sprites,
icons, audio, music, voices, video, maps, fonts, screenshots, extracted data
files, retail binaries, decompiled code bodies or verbatim retail text.

The client reads the player's own StarCraft installation at runtime through
`--install` / `SC_INSTALL`, and the installation is never modified. Text,
tables and assets displayed by the client come either from that installation
or from original placeholder strings in this repository.

Everything derived from retail data is generated locally from the player's
own install and is gitignored. That covers the developer fixtures in
`engine-rust/fixtures/` (only its `README.md`, `SHA256SUMS` and a synthetic
`audio/tone.ogg` are tracked), the text coverage masks in
`engine-rust/src/menu_masks/`, the calibration layers, the extracted assets,
memory dumps and local reverse-engineering evidence. `SHA256SUMS` only lists
hashes so you can check your locally generated fixtures.

**Do not publish locally built binaries that embed fixtures.** `build.rs`
embeds the gitignored fixtures into the binary when they are present. Such a
binary is for personal use only and must not be uploaded or distributed.
Release packages must be built from a clean checkout without fixtures; they
contain no game data. The client reads the command-card table and the
Hotkeys panel data from the player's install at run time. The AppImage and
the Windows zip may bundle an LGPL build of FFmpeg (https://ffmpeg.org,
source at https://github.com/BtbN/FFmpeg-Builds) for video playback, shipped
with its licence as `ffmpeg-LICENSE.txt`; the Flatpak uses the runtime's
FFmpeg.

StarCraft, Brood War and Blizzard Entertainment are trademarks or registered
trademarks of Blizzard Entertainment, Inc. This project is not affiliated
with or endorsed by Blizzard.

Code is licensed under the MIT License (see `LICENSE`).

## minifb (patched copy)

`engine-rust/third_party/minifb` is minifb 0.29.0 by Daniel Collin and
contributors (https://github.com/emoon/rust_minifb), MIT OR Apache-2.0 (its
`LICENSE-MIT` and `LICENSE-APACHE` are kept beside it), with a small
fullscreen patch for Wayland and X11.
