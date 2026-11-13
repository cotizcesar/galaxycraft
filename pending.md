# Pending

Open items found while working on something else.  Remove an entry when it
is fixed.

## Infantry (Marine, Firebat, Ghost, Medic), 2026-11-12

Checked against the OpenBW source (`bwgame.h`, `actions.h`) on 2026-11-12:
Stim Packs, range upgrades, Bunker +64, cloak cost and drain, starting
energy, Medic heal rules and search square, Ghost paint range, the nuke's
90 / 45 frame timers and 320 px drop, splash rings measured to unit boxes,
`attkshiftproj`, Lockdown 131, blind / restore.  What is still open:

- **Cloak look, SD only.**  HD is settled (measured in retail 2026-11-13:
  an own cloaked unit that no own detector reaches is see-through and
  undistorted, `CLOAK_HD_KEEP`; a detected one, like an Observer, is drawn
  normally).  The opacity (~half) is estimated from screenshots, and that a
  Dark Templar beside an Observer turns opaque is an INFERENCE from the
  Observer case.  The SD path still displaces the background
  (`refract_rect`, tuned by eye).
- **"What?" take choice.**  Retail never repeated a take back to back in ~30
  recorded lines; `voice::pick_what` excludes the previous take (INFERENCE:
  the sample is small).  The click count itself is measured
  (`voice::WHAT_CLICKS`).

## Vehicles and air units, 2026-11-12

Rules read from the OpenBW source; these parts are simplified or not checked
on screen.

- **Turrets are folded into the base unit.**  The base fires the turret's
  weapon; the turret turns at its own rate, gates the shot and plays its
  firing animation, and the hull holds still while it fires (tested).
- **Spider Mine.**  A mine is nudged by collision while it waits (not
  checked against OpenBW's collision rules for it).
- **Air repulse.**  The map-edge turn and the "cannot be interrupted"
  exemptions are simplified.
- **Team colour in SD** uses `tunit.pcx`'s exact shades for the slot
  colours; a lobby-chosen colour still uses median brightness steps.
- **Checked on screen only in outline** (scripted HD and SD sessions, small
  crops): the siege deploy, a mine chasing and exploding, the Yamato charge
  and blast, and the Matrix / Irradiate / EMP casts being accepted; the Matrix
  shell and the Irradiate cloud on their targets (front and back shells).  A
  Dropship loading, flying to a point and unloading was checked on screen.

## Terran buildings and client, 2026-11-12

- **Abandoned add-ons** behave as neutral (no buttons, no research, not
  auto-attacked, yellow circle and cursor, taken by whoever lands on
  them) but keep their owner inside the engine, which has no neutral
  player slot (`World` holds one entry per race); kill credit and score
  may still count them for the old owner.
- **Landing** crushes nothing (OpenBW kills burrowed / immobile units
  under the building and lifts off again if blocked by one).
- **Wireframe damage**: the order in which sections turn yellow / red is a
  seeded permutation per unit (INFERENCE); retail picks them at random.

## Campaign, 2026-11-13

- **Alliances are one team per player** (transitive); retail keeps them
  per pair.  No Terran mission puts the human in an allied force with a
  computer.  See `engine-rust/docs/CAMPAIGN-TERRAN.md`.
- **The 68 campaign missions (all six episodes) reach victory only through
  `sc_solve`**, which moves or spawns the units a goal asks for and gives
  the player invincibility.  A normal play-through (difficulty, attack
  timing) is not checked.
- **Rescue range** (`RESCUE_RANGE_PX` = 36, per-axis box gap) is fitted to
  the Wasteland recording and to two Brood War missions that require an
  immediate rescue; any value 33-40 fits.  Falsifier: a retail recording
  of a rescue at a measured distance.
- **Trigger pass after a Wait** goes on after the trigger that waited
  instead of restarting at trigger 0 (otherwise Brood War Zerg 1 cannot
  end).  Falsifier: the retail trigger loop in the corpus.
- **Campaign AI defence response** (`CampaignAi::defend`) timing is an
  inference.
