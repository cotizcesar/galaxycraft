# Trash salvage (2026-11-09)

The desktop Trash held no decompilation and no Ghidra project.  It held one
failed Craft Anything attempt, 26 retail UI screenshots, one retail UI
recording and a few unrelated user files.  Only the screenshots and the
recording were unique; they are reference material, not source, so they stay
outside the repository in the cache.  Everything else was either a duplicate
of `~/Documents/starcraft-re-recomp` or unrelated.

## Kept in the cache (not committed)

| Path | Content |
|---|---|
| `~/.cache/galaxycraft/reference/multiplayer-2026-11-06/` | 26 PNGs, 3440x1440, retail 1.23.10 UI on 2026-11-06 14:14–14:17 and 14:56 |
| `~/.cache/galaxycraft/reference/Screencast_20261106_141301.webm` | 14.4 MB retail UI recording from the same session |

`engine-rust/docs/reference/multiplayer-2026-11-06.manifest.tsv` records every
file's path, size and SHA-256.  The 2026-11-06 (147) session-log entry used
these 25 multiplayer screenshots for the menu comparison; the extra 14:56
image is the Options screen.

## What was deleted

The Trash had **40 top-level entries, 9,795 files, 6.77 GB**:

* `Craft Anything` (6.33 GB): Elgato Studio and Command & Conquer app payloads,
  unrelated to StarCraft.
* `Craft Anything (1)` (109 MB): app checks; only its StarCraft journal was
  read, and it contained two `core_target_triage` calls.
* `Elgato.Studio_2.0.0.3364_x64.msix`, `Lente-0.4.0.flatpak`, the ArtCraft
  launcher, `camhub-linux`, personal assembly PDFs: unrelated.
* `PROGRESS.md`, `AGENTS.md`, `README.md`, `opencode.json`: byte-identical to
  `~/Documents/starcraft-re-recomp`; `CHANGELOG.md` was an older prefix.
* 13 tiny journals under `starcraft*`, `investigation.cany*`, `x86-64` and
  `Craft Anything (1)/starcraft`: project-delete and target-triage events,
  no decompiled code.
* `starcraft-2` (392 KB): the stopped Craft Anything project.  Its own
  `feed.json` says “Couldn't find the program to decompile, so Ghidra can't be
  used”; `build-activity.log` runs for eight seconds and stops.  It adds no
  retail evidence, so it is not committed.

The audit files are
`~/.cache/galaxycraft/reference/trash-before-2026-11-09.tsv` (per-entry size
and disposition) and `trash-salvage-manifest-2026-11-09.tsv` (SHA-256 of every
file copied out before deletion).

## What this means for the reconstruction

The real evidence tree was never in the Trash; it is still at
`~/Documents/starcraft-re-recomp` (18 GB).  The salvage changes nothing about
the decompilation workflow:

* `engine-rust/docs/evidence-recovery.md` explains the indexed evidence and
  the `0x79fa0000 -> 0x400000` address mapping.
* `scripts/evidence_index.py` extracts the 257 cited code addresses.
* `scripts/function_coverage.py` / `scripts/decomp_index.py` join them to the
  Ghidra inventory and pseudocode.
* The 26 screenshots are for visual parity checks, not for code recovery.
