# Unit fidelity method

How one unit is checked against retail StarCraft 1.23.10 and fixed, so the
same audit can be repeated for the next unit.  It was used for the SCV
(`tests/scv_retail.rs`) and for the Marine, Firebat, Ghost and Medic
(`tests/infantry_retail.rs`); the results are in `FIDELITY.md`.

The idea: every claim about retail comes from one of the reference sources
below, every fix gets a test that names its source, and what the player sees
is checked in the real client in both HD and SD.

## 1. Reference sources, in order of trust

| source | what it gives | how to read it |
|---|---|---|
| Retail data in `fixtures/` (`units.dat`, `weapons.dat`, `flingy.dat`, `images.dat`, `sprites.dat`, `iscript.bin`, `sfxdata.*`, `techdata.dat`) | stats, costs, ranges, splash radii, animation scripts, sounds | `python3 scripts/unit_ref.py --unit ID` (below) |
| The user's install (`SC_INSTALL`) | GRP art, `images.tbl`, maps | `sc_dump --install "$SC_INSTALL" --out DIR --include arr/images.tbl` |
| Decompiled corpus (`~/.cache/galaxycraft/decomp_full`) | exact rules when a body exists | `python3 scripts/query_decomp.py --address 0xADDR`; many gameplay functions have no body, so do not count on it |
| OpenBW source (`bwgame.h`, `actions.h`) | order logic that is code, not data (collision of gathering workers, Medic orders, range upgrades, nuke timers, splash) | download it to the scratchpad and `grep` it: `curl -sL -o bwgame.h https://raw.githubusercontent.com/OpenBW/openbw/master/bwgame.h` (same for `actions.h`); cite the function in the code comment as `OpenBW <function>`.  Never copy its code or commit the file; read it, then write the rule |
| The user's recordings / replays | timings that are not in the data (mining dwell, construction stages) | `sc_retailrep`, existing `FACT (… recording)` comments |

Anything you could not confirm goes in `pending.md` at the repository root or
is marked `INFERENCE` in the code with what would prove it wrong.  Never
present a guess as retail behaviour.

## 2. Get the retail picture of the unit

```bash
python3 scripts/unit_ref.py --unit 32          # Firebat: stats, weapon rows, iscript
python3 scripts/unit_ref.py --iscript 312      # an overlay or bullet script
python3 scripts/unit_ref.py --image 421        # draw function, remap, iscript of an image
python3 scripts/unit_ref.py --weapon 25
python3 scripts/unit_ref.py --sounds 'firebat' # sound ids and file names
python3 scripts/unit_ref.py --images-tbl DUMP/arr/images.tbl --find 'heal|cure|eld'
```

Read the iscript animation by animation.  It answers most questions:

- which frames each action shows (`playfram`), and for how long (`wait`);
- when the weapon fires (`attackwith`, `attack`, `useweapon N`,
  `attkshiftproj DIST`, `castspell`) and when the swing ends
  (`gotorepeatattk`, `ignorerest`);
- overlays (`imgol` / `imgul` / `sprol`) and sounds (`playsnd*`);
- whether the unit moves by script (`move N` in Walking) or by its flingy.

Then follow the chain for bullets and overlays: weapon -> flingy -> sprite ->
image -> iscript.  Check `sprites.dat` `visible` (an invisible sprite such as
the Flame Thrower's `circle14` must never be drawn) and the image's draw
function (9 = a remapped glow, not palette colours).

## 3. Map the engine

Ask an Explore agent (or use `rg`) for a table of: where each aspect is
implemented, the constants used, and a verdict (matches / differs / missing).
Give it the retail values so it can compare.  The aspects that mattered so
far:

- stats source (must be `units.dat`, no hardcoded overrides);
- weapon: damage, cooldown, range, range upgrades, splash, damage type;
- abilities: cost, duration, preconditions, heroes;
- orders: idle acquisition, attack, hold, attack-move, special orders;
- collision and what the unit is exempt from;
- visibility: sight, cloak, hidden states;
- renderer: which animation plays for each state, overlays, effects, sounds;
- client: command card buttons, hotkeys, network path of each action;
- voices: ready / what / yes / annoyed lines.

Treat the report as leads, not as findings: confirm each one in the code
before changing anything.

## 4. Fix and test in the simulation

One behaviour at a time: change `sim.rs` (or the module that owns it), then
add a test to `tests/<unit>_retail.rs`.  A good test:

- builds a small `World::new(GameData::bundled(), Terrain::open(64, 64), 2)`;
- spawns the units with `spawn_px`, sets what it needs (`players[0].techs`,
  `upgrades`, `set_energy_fixed`, `hp`);
- issues the order through the public API (`issue`, `cast`, `train`,
  `set_cloak`, `launch_nuke`);
- steps and asserts numbers that come from the data, with the source in a
  comment (`// weapons.dat row 25`, `// iscript 69 GndAttkInit`).

Use an enemy that does not fight back when you measure damage (a Zerg Egg, or
a unit on `HoldPosition` out of its own range), and remember that Zerg units
regenerate.

After each batch run the whole suite; the older tests encode other evidence:

```bash
cd engine-rust
cargo test -q --test infantry_retail
cargo test -q 2>&1 | rg "^---- |panicked|test result: FAILED"
cargo clippy -q --all-targets -- -D warnings && cargo fmt --check
```

When an old test fails, decide which side is evidence.  A test that pins a
recovered retail value (a comment with a function address or a recording)
wins over a general rule; a test that only pinned the old approximation is
updated, with the reason in the commit.

## 5. Test the renderer without a window

`unit_anim::Animator` is what picks animations, overlays, effects and sounds.
Drive it next to the world and assert on what it would draw:

```rust
let mut anim = Animator::default();
for _ in 0..60 {
    w.step();
    anim.advance(&w);
    frames.extend(anim.unit_frame(id));           // playfram operands
    flame |= anim.effects().any(|e| e.image == 421);
    sounds.extend(anim.take_sounds().into_iter().map(|s| s.0));
}
```

`carry_overlays`, `status_overlays` and `unit_glow` cover the other layers.
The animator only advances when the world tick changed, so step the world
before `advance`.

## 6. Look at it in the real client

Tests do not see a wrong palette, a missing mirror or an effect drawn as a
white disc.  `sc_view` runs scripted sessions and writes screenshots:

```bash
export SC_INSTALL="$HOME/Games/battlenet/drive_c/Program Files (x86)/StarCraft"
cargo build -q --release --bin sc_view
GC_WALK_DIR=/tmp/walk ./target/release/sc_view --install "$SC_INSTALL" \
  --races terran,zerg --no-fog --shot-map 80 \
  --game-script "look:420:2640,ticks:3,spawn:0:32:470:2660,spawn:1:39:560:2640,ticks:30,dump-units:a,shot:a,ticks:3,shot:b"
```

- Run every scene twice: as above (HD) and with `--sd`.  The two renderers
  share the simulation and the animator but draw differently, and several
  bugs were in only one of them.
- `--shot-map 80` is a melee map (index 0 is a campaign mission with tech
  restrictions).  Player 0 starts at world (320, 2768) on it.
- Script steps (full list in the comment above `run_game_script` in
  `src/bin/sc_view.rs`): `look:x:y`, `spawn:OWNER:TYPE:x:y`, `ticks:N`,
  `click-unit:NAME`, `rclick-unit:NAME`, `click-world:x:y`,
  `rclick-world:x:y`, `drag:x0:y0:x1:y1`, `key:K`, `cheat:TEXT`,
  `arm-silos` (every Nuclear Silo attached and loaded),
  `dump-units:NAME`, `shot:NAME`.
- `dump-units:NAME` writes `NAME.tsv` (id, owner, type, position, hp, order,
  path, animation); `game.tsv` has one line per step with the camera,
  resources and selection.  Use them to find where a unit is and what it is
  doing before you look at pixels.
- An idle unit that auto-attacks keeps `order=Idle`; watch the target's hit
  points, not the order.
- Screenshots are 1280 x 720 BMPs; the playfield is drawn at 1.5x, so a world
  point is at about `((x - cam_x) * 1.5, (y - cam_y) * 1.5)`.  Crop and zoom
  with ImageMagick and read the PNG:

```bash
magick /tmp/walk/shot-a.bmp -crop 200x160+640+230 +repage -filter point -resize 200% a.png
magick a.png b.png +append strip.png        # several moments side by side
```

- Animations are short.  Take a strip of shots 1 to 3 ticks apart over a
  whole cycle (an attack is 15 to 22 ticks) instead of one shot.
- Cheats typed on the first frame lose their minerals (see `pending.md`);
  let a few ticks pass first.  Scripted `spawn` units start with full
  energy.

When a scene looks wrong, find out which layer is wrong before fixing: the
unit's own frame (`unit_frame`), an overlay, an effect, or the draw code.  A
temporary `eprintln!` in the draw loop, behind an environment variable, is
the quickest way to see which images reach it; remove it afterwards.

## 7. Bugs this method found, as patterns to look for

- Distances measured between centres where retail measures between unit
  boxes (weapon range, splash, heal reach): short-range weapons never hit.
- A script that ends a swing with `gotorepeatattk` and an idle loop instead
  of `ignorerest`: the unit attacked once.
- An iscript opcode with a misleading name (`orderdone` clears a signal bit;
  it does not finish the order): the Medic dropped every order.
- An opcode silently ignored (`attkshiftproj`): the Firebat's three flames
  were one hit.
- Damage applied on the frame the unit fires, where retail waits for the
  bullet image's `domissiledmg` (2 frames for sparks, 7 for the flame).
- A path left over from the approach: the renderer played the walk over the
  attack and the effect never appeared.
- Invisible placeholder sprites drawn, glow images drawn with the palette,
  directions not mirrored in SD.
- Abilities that are free or permanent in the engine and not in retail
  (cloak cost, Optical Flare duration, full energy on trained casters).

## 8. Finish

- Add the unit's table to `FIDELITY.md` (aspect, retail, engine, fixed or
  matches) and what stayed approximate.
- Put open doubts in `pending.md`.
- Commit on a branch, open a PR, and follow the release loop in `AGENTS.md`
  when the user asks to merge.
