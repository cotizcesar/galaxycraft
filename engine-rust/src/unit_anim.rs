//! Render-side iscript animation: unit poses (walking, attacking), bullets in
//! flight and the death / impact effects they leave.
//!
//! The rules engine (`World`) moves units and resolves shots; it does not
//! keep per-image animation state.  This module watches the world after each
//! tick and runs the shipped `iscript.bin` animations with
//! [`crate::iscript::Iscript::tick_anim`] so the client can draw what retail
//! draws: the Walking cycle while a unit has a path, GndAttkInit /
//! AirAttkInit when it fires (its weapon cooldown restarts), the bullet
//! image while a projectile flies, and the Death animation (with the
//! explosions its `imgol` / `sprol` opcodes create) when a unit or a bullet
//! goes away.  It never mutates the world, so lockstep, saves and replays are
//! unaffected.
//!
//! INFERENCE: retail drives these animations from the unit's order state
//! inside the engine; reading the cooldown restart as "fired" and the path
//! as "walking" reproduces the visible result, not the exact frame timing.

use std::collections::HashMap;

use crate::data::GameData;
use crate::iscript::{AnimContext, AnimEvent, AnimState};
use crate::sim::{Order, World};

/// iscript animation entries (`iscript.bin` header order).
pub mod anim {
    pub const INIT: usize = 0;
    pub const DEATH: usize = 1;
    pub const GND_ATTK_INIT: usize = 2;
    pub const AIR_ATTK_INIT: usize = 3;
    pub const GND_ATTK_RPT: usize = 5;
    pub const AIR_ATTK_RPT: usize = 6;
    pub const GND_ATTK_TO_IDLE: usize = 8;
    pub const AIR_ATTK_TO_IDLE: usize = 9;
    pub const WALKING: usize = 11;
    pub const WALKING_TO_IDLE: usize = 12;
    /// Workers loop this one while they mine, build or repair (OpenBW
    /// `order_MiningMinerals` / `order_ConstructingBuilding` /
    /// `order_Repair` run `iscript_anims::AlmostBuilt`); the SCV's fires
    /// `useweapon 14`, the harvest Fusion Cutter spark.
    pub const ALMOST_BUILT: usize = 15;
    /// Spell casters play this once while the simulation runs their cast.
    pub const CAST_SPELL: usize = 7;
    /// The Medic loops this while it heals (iscript 360: frames 68 / 85 and
    /// the heal sound, `playsnd 1018`).
    pub const SPECIAL_STATE_1: usize = 13;
}

/// Spell overlay images (`images.dat`), small / medium / large by the
/// target's `units.dat` size: Lockdown field 361..=363 (`eldSmall/Med/
/// Large`), Restoration 967..=969 (`tCure*`), Heal 970..=972 (`tmeHeal*`),
/// Optical Flare hit 976..=978 (`TmeFlsh*`).
pub const LOCKDOWN_FIELD_IMAGE: u16 = 361;
/// Defensive Matrix front shell 371..=373 (`edmFronS/M/L`) and Irradiate
/// cloud 380..=382 (`eradLrg`), by the target's size.
pub const MATRIX_FRONT_IMAGE: u16 = 371;
pub const IRRADIATE_IMAGE: u16 = 380;
pub const RESTORATION_IMAGE: u16 = 967;
pub const HEAL_IMAGE: u16 = 970;
pub const OPTICAL_FLARE_HIT_IMAGE: u16 = 976;
/// `sfxdata.dat`: Stim Pack 278 / 279 (`TMaSti00/01`), Optical Flare cast
/// 1016 (`TMedflsh`), its hit 1019 (`Miopia1`), Restoration 1017
/// (`TMedRest`).
pub const STIM_SOUNDS: [u16; 2] = [278, 279];
pub const OPTICAL_FLARE_CAST_SOUND: u16 = 1016;
pub const OPTICAL_FLARE_HIT_SOUND: u16 = 1019;
pub const RESTORATION_SOUND: u16 = 1017;

/// Offset (0..=2) of a size-keyed overlay image for a unit type.
fn size_slot(data: &GameData, type_id: u16) -> u16 {
    match data.unit(type_id).size {
        crate::data::UnitSize::Medium => 1,
        crate::data::UnitSize::Large => 2,
        _ => 0,
    }
}

/// Carried-resource images (`images.dat`): `OreChunk` 397 / 398, `GasOrb`
/// 399 / 400 (Protoss), `GasSac` 401 / 402 (Zerg), `GasTank` 403 / 404
/// (Terran).  The second of each pair is the small load of a depleted
/// source (OpenBW `order_MiningMinerals` / `order_HarvestGas`: type 2 when
/// fewer than 8 units were taken).
pub fn carry_image(type_id: u16, kind: crate::sim::CarryKind, amount: i32) -> Option<u16> {
    use crate::ids::unit as uid;
    use crate::sim::CarryKind;
    let small = u16::from(amount < 8);
    match kind {
        CarryKind::None => None,
        CarryKind::Minerals => Some(397 + small),
        CarryKind::Gas => Some(
            match type_id {
                uid::PROBE => 399,
                uid::DRONE => 401,
                _ => 403,
            } + small,
        ),
    }
}

/// Ticks one script may lag behind before it is fast-forwarded in one go.
const MAX_CATCH_UP: u64 = 8;
/// First damage-overlay image (`images.dat` 450): LO slot `k` shows image
/// `450 + k`.  FACT: images 450..471 are the fire / blood overlay GRPs
/// (`thingy\\oFire*`, `thingy\\bblood*`), 22 of them, and a building's
/// damage LO (`.lof`) has 22 slots of which it uses a few (Supply Depot: slot
/// 0; Zerg buildings use the blood slots), so the slot picks the image.
pub const DAMAGE_OVERLAY_BASE: u16 = 450;
/// Unused LO slot marker.
const LO_UNUSED: i8 = 127;

/// Longest an effect may run (about 20 s at Fastest): a looping overlay
/// whose script never reaches `end` is dropped then.
const MAX_EFFECT_TICKS: u32 = 480;

/// One running animation and the last frame it showed.
#[derive(Clone, Debug)]
struct Track {
    image: u16,
    anim: usize,
    state: AnimState,
    /// Last `playfram` operand (before the direction offset).
    frame: Option<u16>,
    /// `playsnd` / `playsndrand` / `playsndbtwn` ids since the last drain.
    sounds: Vec<u16>,
    /// `useweapon` operands since the last drain.
    weapons: Vec<u8>,
}

/// What the animator remembers about a unit between ticks.
#[derive(Clone, Debug)]
struct UnitSeen {
    track: Option<Track>,
    /// Engine glow attached by the walking animation (`imgol` of an image
    /// whose iscript is [`ENGINE_GLOW_ISCRIPT`], e.g. the SCV's `tscGlow`,
    /// image 249): it follows the unit and shows its frame (`engframe`)
    /// until the unit stops.
    glow: Option<u16>,
    /// Attack animation of the unit's turret image (tank barrel, Goliath
    /// torso): its frames, firing sound and muzzle overlays.
    turret: Option<Track>,
    cooldown: u32,
    x: i32,
    y: i32,
    facing: u8,
    image: u16,
    building: bool,
}

/// A free-standing image: a death animation, an explosion or an impact.
#[derive(Clone, Debug)]
pub struct Effect {
    pub image: u16,
    /// Position in pixels.
    pub x: i32,
    pub y: i32,
    pub facing: u8,
    pub frame: u16,
    track: Track,
    /// Ticks run so far (looping overlays are dropped at [`MAX_EFFECT_TICKS`]).
    age: u32,
}

/// A bullet in flight, ready to draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BulletPose {
    pub image: u16,
    pub x: i32,
    pub y: i32,
    pub facing: u8,
    pub frame: u16,
}

#[derive(Clone, Debug)]
struct BulletSeen {
    track: Track,
    x: i32,
    y: i32,
    facing: u8,
}

/// A `.lo?` overlay offset file: `u32 frames, u32 overlays, u32
/// offsets[frames]`, each offset pointing at `overlays` `(i8 x, i8 y)` pairs.
/// Returns frame 0's used slots as `(slot, x, y)`; `None` for a malformed
/// file (sizes are checked before anything is read).
pub fn parse_lo_slots(bytes: &[u8]) -> Option<Vec<(u8, i8, i8)>> {
    let u32_at = |o: usize| {
        bytes
            .get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    let frames = u32_at(0)?;
    let overlays = u32_at(4)?;
    if frames == 0 || frames > 4096 || overlays == 0 || overlays > 255 {
        return None;
    }
    let first = u32_at(8)?;
    let pairs = bytes.get(first..first + overlays * 2)?;
    Some(
        pairs
            .as_chunks::<2>()
            .0
            .iter()
            .enumerate()
            .filter(|(_, p)| p[0] as i8 != LO_UNUSED || p[1] as i8 != LO_UNUSED)
            .map(|(k, p)| (k as u8, p[0] as i8, p[1] as i8))
            .collect(),
    )
}

/// How many of a building's `used` damage slots burn at hit-point fraction
/// `frac`: none above two thirds, about half below, all below one third.
/// INFERENCE: retail's exact thresholds were not measured; these follow its
/// visible "lightly / badly damaged" steps.
pub fn burning_slots(used: usize, frac: f32) -> usize {
    if used == 0 || frac >= 2.0 / 3.0 {
        0
    } else if frac >= 1.0 / 3.0 {
        used.div_ceil(2)
    } else {
        used
    }
}

/// A damage overlay to draw: image, building centre plus offset, frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverlayPose {
    pub image: u16,
    pub x: i32,
    pub y: i32,
    pub frame: u16,
}

#[derive(Clone, Debug, Default)]
pub struct Animator {
    /// Sounds the animations played (`sfxdata.dat` id, position in px)
    /// since the last [`Animator::take_sounds`]: weapon fire, deaths,
    /// explosions, as retail's iscript `playsnd` opcodes do.
    sounds: Vec<(u16, i32, i32)>,
    /// Zerg buildings forming from a drone: the construction image's
    /// animation (`units.dat` construction graphic, `zerg\\zbuild`).
    construction: HashMap<u32, Track>,
    /// Burning / bleeding overlays per (building id, LO slot).
    damage: HashMap<(u32, u8), (Track, i32, i32)>,
    /// Damage LO slots per building image, from the caller's lookup.
    damage_slots: HashMap<u16, Vec<(u8, i8, i8)>>,
    units: HashMap<u32, UnitSeen>,
    bullets: HashMap<u32, BulletSeen>,
    effects: Vec<Effect>,
    last_tick: Option<u64>,
    rng: u32,
    /// Overlays that stay on a unit while a state lasts and restart when
    /// their script ends: the Lockdown field and the Heal glow on a
    /// patient, keyed by (unit id, base image).
    status: HashMap<(u32, u16), Track>,
    /// [`World::spell_cue_seq`] already turned into effects and sounds.
    cue_seq: u64,
}

/// iscript of the engine-glow overlays (`tscGlow`, `tbaGlow`, `tdrGlow`,
/// `tbmGlow`, `pabGlow`: images 249 / 220 / 225 / 941 / 132 all use it).
/// FACT (2026-11-07 probe): the SCV's walking animation (iscript 84,
/// anim 11) runs `imgol 249`.
pub const ENGINE_GLOW_ISCRIPT: u32 = 291;

/// Main image of a unit type (`units.dat -> flingy -> sprite -> image`).
pub fn unit_image(data: &GameData, type_id: u16) -> Option<u16> {
    let flingy = data.flingy(data.unit(type_id).flingy)?;
    Some(data.sprite(flingy.sprite)?.image)
}

/// Image a weapon's bullet shows (`weapons.dat -> flingy -> sprite -> image`).
pub fn bullet_image(data: &GameData, weapon: u8) -> Option<u16> {
    let w = data.weapon(weapon)?;
    let flingy = data.flingy(u16::try_from(w.flingy).ok()?)?;
    // An invisible sprite (`sprites.dat` visible = 0) is never drawn: the
    // Flame Thrower's bullet is the white `circle14` placeholder, and the
    // flame the player sees is the Firebat's own `imgol 421`.
    data.sprite(flingy.sprite)
        .filter(|s| s.visible)
        .map(|s| s.image)
}

/// Facing (0..256) of a vector in game pixels, 0 = up, clockwise.
fn facing_of(dx: i32, dy: i32) -> u8 {
    if dx == 0 && dy == 0 {
        return 0;
    }
    let a = (dx as f64).atan2(-(dy as f64));
    ((a / std::f64::consts::TAU * 256.0).round() as i32).rem_euclid(256) as u8
}

impl Track {
    fn start(data: &GameData, image: u16, anim: usize, facing: u8) -> Option<Self> {
        let script = data.image(image)?.iscript_id as u16;
        let state = data.iscript.start_animation(script, anim, facing)?;
        Some(Self {
            image,
            anim,
            state,
            frame: None,
            sounds: Vec::new(),
            weapons: Vec::new(),
        })
    }

    /// Run one tick; returns the overlays it created.
    fn tick(&mut self, data: &GameData, facing: u8, rng: &mut u32) -> Vec<(u16, i8, i8)> {
        if self.state.ended {
            return Vec::new();
        }
        let mut ctx = AnimContext {
            direction: facing,
            rng: *rng,
            ..AnimContext::default()
        };
        let mut spawned = Vec::new();
        for e in data.iscript.tick_anim(&mut self.state, &mut ctx) {
            match e {
                AnimEvent::Frame { raw, .. } => self.frame = Some(raw),
                AnimEvent::Sound(id) | AnimEvent::RandomSound(id) => self.sounds.push(id),
                AnimEvent::Attack {
                    weapon: Some(w), ..
                } => self.weapons.push(w),
                AnimEvent::Overlay {
                    id,
                    x,
                    y,
                    below,
                    sprite,
                } if id != 0 && !below => {
                    // `sprol` & co. name a sprite; `imgol` & co. an image.
                    let image = if sprite {
                        data.sprite(id).map(|s| s.image)
                    } else {
                        Some(id)
                    };
                    if let Some(image) = image {
                        spawned.push((image, x, y));
                    }
                }
                _ => {}
            }
        }
        *rng = ctx.rng;
        spawned
    }
}

impl Animator {
    /// Catch up with the world: one animation step per simulated tick since
    /// the last call (at most [`MAX_CATCH_UP`]).
    pub fn advance(&mut self, world: &World) {
        self.advance_with(world, &mut |_| Vec::new());
    }

    /// [`Self::advance`] with a lookup for building damage overlays: the
    /// used slots of an image's damage LO (`images.dat` damage file, see
    /// [`parse_lo_slots`]), asked once per image.
    pub fn advance_with(&mut self, world: &World, lo: &mut dyn FnMut(u16) -> Vec<(u8, i8, i8)>) {
        let ticks = match self.last_tick {
            Some(t) if world.tick >= t => (world.tick - t).min(MAX_CATCH_UP),
            // First sight or a reload / rewind: start over (the LO cache
            // stays, it only depends on the data).
            _ => {
                let slots = std::mem::take(&mut self.damage_slots);
                *self = Self {
                    damage_slots: slots,
                    ..Self::default()
                };
                1
            }
        };
        self.last_tick = Some(world.tick);
        for _ in 0..ticks {
            self.step_spells(world);
            self.step(world);
            self.step_damage(world, lo);
            self.step_construction(world);
        }
    }

    /// Spell effects and sounds: the cues the simulation queued since the
    /// last step, and the overlays that last as long as a state does.
    fn step_spells(&mut self, world: &World) {
        use crate::sim::SpellCueKind as K;
        let data = &*world.data;
        let mut rng = self.rng ^ 0x5bd1_e995;
        // New cues (at most the ones the world still keeps).
        let fresh =
            (world.spell_cue_seq.saturating_sub(self.cue_seq) as usize).min(world.spell_cues.len());
        self.cue_seq = world.spell_cue_seq;
        let start = world.spell_cues.len() - fresh;
        for cue in world.spell_cues.iter().skip(start) {
            let Some(u) = world.unit(cue.unit) else {
                continue;
            };
            let (x, y) = (u.x / 256, u.y / 256);
            let slot = size_slot(data, u.type_id);
            let (image, sound) = match cue.kind {
                K::Stim => {
                    rng = rng.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                    (None, Some(STIM_SOUNDS[(rng >> 16) as usize % 2]))
                }
                K::LockdownHit => (None, None),
                K::Restoration => (Some(RESTORATION_IMAGE + slot), Some(RESTORATION_SOUND)),
                K::OpticalFlareCast => (None, Some(OPTICAL_FLARE_CAST_SOUND)),
                K::OpticalFlareHit => (
                    Some(OPTICAL_FLARE_HIT_IMAGE + slot),
                    Some(OPTICAL_FLARE_HIT_SOUND),
                ),
            };
            if let Some(id) = sound {
                self.sounds.push((id, x, y));
            }
            if let Some(image) = image
                && let Some(track) = Track::start(data, image, anim::INIT, 0)
            {
                self.effects.push(Effect {
                    image,
                    x,
                    y,
                    facing: 0,
                    frame: 0,
                    track,
                    age: 0,
                });
            }
        }
        // Lasting overlays: the Lockdown field on a disabled unit, the Heal
        // glow on a patient a Medic is working on.
        let mut wanted: Vec<(u32, u16)> = Vec::new();
        for (i, u) in world.units.iter().enumerate().filter(|(_, u)| u.alive) {
            if u.status.lockdown_ticks > 0 {
                wanted.push((u.id, LOCKDOWN_FIELD_IMAGE + size_slot(data, u.type_id)));
            }
            // Defensive Matrix shell and Irradiate cloud, while they last.
            if u.status.matrix_ticks > 0 {
                wanted.push((u.id, MATRIX_FRONT_IMAGE + size_slot(data, u.type_id)));
            }
            if u.status.irradiate_ticks > 0 {
                wanted.push((u.id, IRRADIATE_IMAGE + size_slot(data, u.type_id)));
            }
            if world.medic_is_healing(i)
                && let Some(p) = u.target.and_then(|t| world.unit(t))
            {
                let key = (p.id, HEAL_IMAGE + size_slot(data, p.type_id));
                if !wanted.contains(&key) {
                    wanted.push(key);
                }
            }
        }
        self.status
            .retain(|k, t| wanted.contains(k) || !t.state.ended);
        for key in wanted {
            let restart = self.status.get(&key).is_none_or(|t| t.state.ended);
            if restart && let Some(t) = Track::start(data, key.1, anim::INIT, 0) {
                self.status.insert(key, t);
            }
        }
        for t in self.status.values_mut() {
            let _ = t.tick(data, 0, &mut rng);
            t.sounds.clear();
        }
        self.status
            .retain(|(id, _), _| world.unit(*id).is_some_and(|u| u.alive));
    }

    /// The Lockdown fields and Heal glows showing a frame, on their units.
    pub fn status_overlays(&self, world: &World) -> Vec<BulletPose> {
        self.status
            .iter()
            .filter_map(|((id, image), t)| {
                let u = world.unit(*id)?;
                Some(BulletPose {
                    image: *image,
                    x: u.x / 256,
                    y: u.y / 256,
                    facing: 0,
                    frame: t.frame?,
                })
            })
            .collect()
    }

    /// Fire on burning Terran / Protoss buildings, blood on Zerg ones: the
    /// image's damage LO slots light up as hit points fall.
    fn step_damage(&mut self, world: &World, lo: &mut dyn FnMut(u16) -> Vec<(u8, i8, i8)>) {
        let data = &*world.data;
        let mut rng = self.rng ^ 0x9e37_79b9;
        let mut wanted: Vec<(u32, u8)> = Vec::new();
        for u in world.units.iter().filter(|u| u.alive && u.completed) {
            let ty = data.unit(u.type_id);
            if !ty.is_building() || ty.hit_points_raw == 0 {
                continue;
            }
            let frac = u.hp as f32 / ty.hit_points_raw as f32;
            if frac >= 2.0 / 3.0 {
                continue;
            }
            let Some(image) = unit_image(data, u.type_id) else {
                continue;
            };
            let slots = self
                .damage_slots
                .entry(image)
                .or_insert_with(|| lo(image))
                .clone();
            for &(slot, dx, dy) in slots.iter().take(burning_slots(slots.len(), frac)) {
                let key = (u.id, slot);
                wanted.push(key);
                let entry = match self.damage.entry(key) {
                    std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
                    std::collections::hash_map::Entry::Vacant(e) => {
                        let img = DAMAGE_OVERLAY_BASE + slot as u16;
                        let Some(track) = Track::start(data, img, anim::INIT, 0) else {
                            continue;
                        };
                        e.insert((track, dx as i32, dy as i32))
                    }
                };
                let _ = entry.0.tick(data, 0, &mut rng);
            }
        }
        // Repaired or destroyed buildings stop burning.
        self.damage.retain(|k, _| wanted.contains(k));
    }

    /// Zerg buildings forming from a drone show their construction image
    /// (`zerg\\zbuild`, sized by `units.dat`) animating until complete.
    /// A building morph (Hatchery to Lair, Creep Colony to Sunken...) is not
    /// a drone construction and keeps the building's own art.
    fn step_construction(&mut self, world: &World) {
        let data = &*world.data;
        let mut rng = self.rng ^ 0x5bd1_e995;
        let mut forming: Vec<u32> = Vec::new();
        for u in world.units.iter().filter(|u| u.alive && !u.completed) {
            let ty = data.unit(u.type_id);
            if !ty.is_building()
                || ty.construction_image == 0
                || world.players.get(u.owner as usize).map(|p| p.race)
                    != Some(crate::sim::Race::Zerg)
                || crate::sim::building_morph_source(u.type_id).is_some()
            {
                continue;
            }
            forming.push(u.id);
            let t = match self.construction.entry(u.id) {
                std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
                std::collections::hash_map::Entry::Vacant(e) => {
                    let Some(t) = Track::start(data, ty.construction_image, anim::INIT, 0) else {
                        continue;
                    };
                    e.insert(t)
                }
            };
            let _ = t.tick(data, 0, &mut rng);
            t.sounds.clear();
        }
        self.construction.retain(|id, _| forming.contains(id));
    }

    /// The construction image and frame a forming Zerg building shows.
    pub fn construction_pose(&self, id: u32) -> Option<(u16, u16)> {
        let t = self.construction.get(&id)?;
        Some((t.image, t.frame?))
    }

    /// Sounds the animations played since the last call.
    pub fn take_sounds(&mut self) -> Vec<(u16, i32, i32)> {
        std::mem::take(&mut self.sounds)
    }

    /// Building fire / blood overlays with their current frame.
    pub fn damage_overlays(&self, world: &World) -> Vec<OverlayPose> {
        self.damage
            .iter()
            .filter_map(|((id, _), (t, dx, dy))| {
                let u = world.unit(*id)?;
                Some(OverlayPose {
                    image: t.image,
                    x: u.x / 256 + dx,
                    y: u.y / 256 + dy,
                    frame: t.frame?,
                })
            })
            .collect()
    }

    fn step(&mut self, world: &World) {
        let data = &*world.data;
        let mut rng = self.rng.wrapping_add(world.tick as u32);
        let mut alive_ids = Vec::with_capacity(world.units.len());
        let mut spawned: Vec<Effect> = Vec::new();
        for (idx, u) in world.units.iter().enumerate().filter(|(_, u)| u.alive) {
            alive_ids.push(u.id);
            let ty = data.unit(u.type_id);
            let Some(image) = unit_image(data, u.type_id) else {
                continue;
            };
            let building = ty.is_building();
            let seen = self.units.entry(u.id).or_insert_with(|| UnitSeen {
                track: None,
                glow: None,
                turret: None,
                cooldown: u.cooldown,
                x: u.x,
                y: u.y,
                facing: u.facing,
                image,
                building,
            });
            let moving = !u.path.is_empty();
            // Mining a field, welding a structure or repairing: the worker
            // loops AlmostBuilt.  Inside a gas building it is hidden.
            let hidden = world.inside_gas_building(idx);
            let working = ty.is_worker()
                && !hidden
                && match u.order {
                    Order::Gather { .. } => u.gather_timer > 0,
                    Order::Construct { building } => {
                        !moving
                            && world
                                .unit(building)
                                .is_some_and(|b| b.alive && !b.completed && b.builder == u.id)
                    }
                    Order::Repair { target } => {
                        !moving && world.unit(target).is_some_and(|t| t.alive)
                    }
                    _ => false,
                };
            let healing = !hidden && world.medic_is_healing(idx);
            // The simulation is running this unit's cast animation.
            let casting = u
                .anim
                .as_ref()
                .is_some_and(|a| a.anim as usize == anim::CAST_SPELL);
            let fired = !working && !healing && !casting && u.cooldown > seen.cooldown;
            let (gw, aw) = (data.ground_weapon_id(ty), data.air_weapon_id(ty));
            let air = gw >= 130 && aw < 130
                || u.target
                    .and_then(|t| world.unit(t))
                    .is_some_and(|t| data.unit(t.type_id).is_flyer());
            if !building {
                let current = seen.track.as_ref().map(|t| t.anim);
                let ended = seen
                    .track
                    .as_ref()
                    .is_none_or(|t| t.state.ended || t.state.ignore_rest);
                let special = if working {
                    Some(anim::ALMOST_BUILT)
                } else if healing {
                    Some(anim::SPECIAL_STATE_1)
                } else if casting {
                    Some(anim::CAST_SPELL)
                } else {
                    None
                };
                let in_special = matches!(
                    current,
                    Some(anim::ALMOST_BUILT | anim::SPECIAL_STATE_1 | anim::CAST_SPELL)
                );
                let want = if let Some(a) = special {
                    (current != Some(a)).then_some(a)
                } else if in_special {
                    Some(if moving {
                        anim::WALKING
                    } else {
                        anim::WALKING_TO_IDLE
                    })
                } else if fired {
                    let attacking = matches!(
                        current,
                        Some(anim::GND_ATTK_INIT | anim::GND_ATTK_RPT)
                            | Some(anim::AIR_ATTK_INIT | anim::AIR_ATTK_RPT)
                    );
                    Some(match (air, attacking) {
                        (false, false) => anim::GND_ATTK_INIT,
                        (false, true) => anim::GND_ATTK_RPT,
                        (true, false) => anim::AIR_ATTK_INIT,
                        (true, true) => anim::AIR_ATTK_RPT,
                    })
                } else if moving {
                    (current != Some(anim::WALKING)).then_some(anim::WALKING)
                } else {
                    match current {
                        Some(anim::WALKING) => Some(anim::WALKING_TO_IDLE),
                        Some(
                            anim::GND_ATTK_INIT
                            | anim::GND_ATTK_RPT
                            | anim::AIR_ATTK_INIT
                            | anim::AIR_ATTK_RPT,
                        ) if ended => Some(if air {
                            anim::AIR_ATTK_TO_IDLE
                        } else {
                            anim::GND_ATTK_TO_IDLE
                        }),
                        None => Some(anim::INIT),
                        _ => None,
                    }
                };
                if let Some(a) = want {
                    // A unit without that entry keeps its current pose.
                    if let Some(mut t) = Track::start(data, image, a, u.facing) {
                        t.frame = seen.track.as_ref().and_then(|o| o.frame);
                        seen.track = Some(t);
                    }
                    if a != anim::WALKING {
                        seen.glow = None;
                    }
                }
                if let Some(t) = seen.track.as_mut() {
                    let spawned_now = t.tick(data, u.facing, &mut rng);
                    for id in t.sounds.drain(..) {
                        self.sounds.push((id, u.x / 256, u.y / 256));
                    }
                    // `useweapon`: the weapon's bullet image where its
                    // behaviour puts it (the harvest cutter appears at the
                    // source unit, `forward_offset` px ahead of it).
                    for w in t.weapons.drain(..) {
                        let Some(wd) = data.weapon(w) else { continue };
                        let Some(img) = bullet_image(data, w) else {
                            continue;
                        };
                        let a = u.facing as f64 / 256.0 * std::f64::consts::TAU;
                        let fwd = wd.forward_offset as f64;
                        let (ox, oy) = (
                            (a.sin() * fwd).round() as i32,
                            (-a.cos() * fwd).round() as i32,
                        );
                        if let Some(track) = Track::start(data, img, anim::INIT, u.facing) {
                            spawned.push(Effect {
                                image: img,
                                x: u.x / 256 + ox,
                                y: u.y / 256 + oy - wd.vertical_offset as i32,
                                facing: u.facing,
                                frame: 0,
                                track,
                                age: 0,
                            });
                        }
                    }
                    for (img, dx, dy) in spawned_now {
                        if data
                            .image(img)
                            .is_some_and(|i| i.iscript_id == ENGINE_GLOW_ISCRIPT)
                        {
                            seen.glow = Some(img);
                            continue;
                        }
                        if let Some(track) = Track::start(data, img, anim::INIT, u.facing) {
                            spawned.push(Effect {
                                image: img,
                                x: u.x / 256 + dx as i32,
                                y: u.y / 256 + dy as i32,
                                facing: u.facing,
                                frame: 0,
                                track,
                                age: 0,
                            });
                        }
                    }
                }
            }
            // The turret fires: its own image runs the attack animation
            // (iscript 93 for the sieged cannon: sound 96 and the muzzle
            // flash; the Goliath's guns likewise).
            if fired
                && data.turret(ty).is_some()
                && let Some(timage) = unit_image(data, ty.turret)
            {
                let a = if air {
                    anim::AIR_ATTK_INIT
                } else {
                    anim::GND_ATTK_INIT
                };
                if let Some(t) = Track::start(data, timage, a, u.facing) {
                    seen.turret = Some(t);
                }
            }
            if let Some(t) = seen.turret.as_mut() {
                let spawned_now = t.tick(data, u.facing, &mut rng);
                for id in t.sounds.drain(..) {
                    self.sounds.push((id, u.x / 256, u.y / 256));
                }
                t.weapons.clear();
                for (img, dx, dy) in spawned_now {
                    if let Some(track) = Track::start(data, img, anim::INIT, u.facing) {
                        spawned.push(Effect {
                            image: img,
                            x: u.x / 256 + dx as i32,
                            y: u.y / 256 + dy as i32,
                            facing: u.facing,
                            frame: 0,
                            track,
                            age: 0,
                        });
                    }
                }
                if t.state.ended || t.state.ignore_rest {
                    seen.turret = None;
                }
            }
            // Instant weapons (`appear at target unit / pos`, `appear at
            // source unit`, `self destruct`) never become a travelling
            // projectile: show their bullet image where it appears.
            if fired {
                let weapon = if air { aw } else { gw };
                let at = data.weapon(weapon).and_then(|w| {
                    use crate::projectile::behavior as b;
                    let target = u.target.and_then(|t| world.unit(t));
                    match w.bullet_type {
                        b::APPEAR_AT_TARGET_UNIT | b::APPEAR_AT_TARGET_POS => {
                            target.map(|t| (t.x / 256, t.y / 256))
                        }
                        b::APPEAR_AT_SOURCE_UNIT | b::SELF_DESTRUCT => Some((u.x / 256, u.y / 256)),
                        _ => None,
                    }
                });
                if let Some((x, y)) = at
                    && let Some(img) = bullet_image(data, weapon)
                    && let Some(track) = Track::start(data, img, anim::INIT, u.facing)
                {
                    spawned.push(Effect {
                        image: img,
                        x,
                        y,
                        facing: u.facing,
                        frame: 0,
                        track,
                        age: 0,
                    });
                }
            }
            seen.cooldown = u.cooldown;
            seen.x = u.x;
            seen.y = u.y;
            seen.facing = u.facing;
            seen.image = image;
            seen.building = building;
        }
        // Units that went away play their Death animation where they were;
        // one that boarded a transport or entered a Bunker (kept with
        // `alive == false`) just disappears.
        let loaded: Vec<u32> = world
            .units
            .iter()
            .filter(|u| u.in_transport.is_some())
            .map(|u| u.id)
            .collect();
        self.units.retain(|id, _| !loaded.contains(id));
        let gone: Vec<u32> = self
            .units
            .keys()
            .copied()
            .filter(|id| !alive_ids.contains(id))
            .collect();
        // A nuclear missile that climbs off the map (heading up) just
        // disappears; one that was falling (heading down, 128) has landed
        // and runs its SpecialState1: the explosion (`imgol 428`,
        // `sprol 267`).
        let missile_image = unit_image(data, crate::ids::unit::NUCLEAR_MISSILE);
        for id in gone {
            let Some(s) = self.units.remove(&id) else {
                continue;
            };
            let last = if Some(s.image) == missile_image {
                if s.facing != 128 {
                    continue;
                }
                anim::SPECIAL_STATE_1
            } else {
                anim::DEATH
            };
            if let Some(track) = Track::start(data, s.image, last, s.facing) {
                spawned.push(Effect {
                    image: s.image,
                    x: s.x / 256,
                    y: s.y / 256,
                    facing: s.facing,
                    frame: 0,
                    track,
                    age: 0,
                });
            }
        }
        // Bullets: their image animates while they fly; on impact (the
        // projectile is gone) their Death animation is the hit effect.
        let mut live = Vec::with_capacity(world.projectiles.len());
        for p in &world.projectiles {
            live.push(p.id);
            let facing = facing_of(p.target_pos.0 - p.x, p.target_pos.1 - p.y);
            let b = match self.bullets.entry(p.id) {
                std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
                std::collections::hash_map::Entry::Vacant(e) => {
                    let Some(image) = bullet_image(data, p.weapon) else {
                        continue;
                    };
                    let Some(track) = Track::start(data, image, anim::INIT, facing) else {
                        continue;
                    };
                    e.insert(BulletSeen {
                        track,
                        x: p.x,
                        y: p.y,
                        facing,
                    })
                }
            };
            b.x = p.x;
            b.y = p.y;
            b.facing = facing;
            let _ = b.track.tick(data, facing, &mut rng);
            for id in b.track.sounds.drain(..) {
                self.sounds.push((id, p.x / 256, p.y / 256));
            }
        }
        let landed: Vec<u32> = self
            .bullets
            .keys()
            .copied()
            .filter(|id| !live.contains(id))
            .collect();
        for id in landed {
            if let Some(b) = self.bullets.remove(&id)
                && let Some(track) = Track::start(data, b.track.image, anim::DEATH, b.facing)
            {
                spawned.push(Effect {
                    image: b.track.image,
                    x: b.x / 256,
                    y: b.y / 256,
                    facing: b.facing,
                    frame: b.track.frame.unwrap_or(0),
                    track,
                    age: 0,
                });
            }
        }
        // Effects run to their `end`; the overlays they create join them.
        let mut next = Vec::with_capacity(self.effects.len() + spawned.len());
        for mut e in std::mem::take(&mut self.effects).into_iter().chain(spawned) {
            let spawned_now = e.track.tick(data, e.facing, &mut rng);
            for id in e.track.sounds.drain(..) {
                self.sounds.push((id, e.x, e.y));
            }
            for (img, dx, dy) in spawned_now {
                if let Some(track) = Track::start(data, img, anim::INIT, e.facing) {
                    next.push(Effect {
                        image: img,
                        x: e.x + dx as i32,
                        y: e.y + dy as i32,
                        facing: e.facing,
                        frame: 0,
                        track,
                        age: 0,
                    });
                }
            }
            if let Some(f) = e.track.frame {
                e.frame = f;
            }
            e.age += 1;
            if !e.track.state.ended && e.age < MAX_EFFECT_TICKS {
                next.push(e);
            }
        }
        // A safety cap: long-lived loops (a burning effect that never ends)
        // cannot pile up without bound.
        if next.len() > 512 {
            next.drain(..next.len() - 512);
        }
        self.effects = next;
        self.rng = rng;
    }

    /// The `playfram` operand of a unit's current animation, `None` while
    /// it has none (buildings, units without scripts).
    pub fn unit_frame(&self, id: u32) -> Option<u16> {
        self.units.get(&id)?.track.as_ref()?.frame
    }

    /// The `playfram` operand of the unit's turret while it runs its attack
    /// animation, `None` otherwise (the renderer then shows frame 0).
    pub fn turret_frame(&self, id: u32) -> Option<u16> {
        self.units.get(&id)?.turret.as_ref()?.frame
    }

    /// The engine glow a moving unit shows on top (image), if any; it is
    /// drawn with the unit's own frame.
    pub fn unit_glow(&self, id: u32) -> Option<u16> {
        self.units.get(&id)?.glow
    }

    /// The mineral chunk / gas container each loaded worker holds: an
    /// overlay on the worker's own position, showing the first frame of its
    /// Init script (retail attaches it with `create_image` above the main
    /// image when the load is taken and removes it at the depot).
    pub fn carry_overlays(&self, world: &World) -> Vec<BulletPose> {
        let data = &*world.data;
        world
            .units
            .iter()
            .enumerate()
            .filter(|(i, u)| u.alive && u.carry > 0 && !world.inside_gas_building(*i))
            .filter_map(|(_, u)| {
                let image = carry_image(u.type_id, u.carry_kind, u.carry)?;
                let mut track = Track::start(data, image, anim::INIT, u.facing)?;
                let mut rng = 0;
                let _ = track.tick(data, u.facing, &mut rng);
                Some(BulletPose {
                    image,
                    x: u.x / 256,
                    y: u.y / 256,
                    facing: u.facing,
                    frame: track.frame?,
                })
            })
            .collect()
    }

    /// Bullets in flight with their current frame.
    pub fn bullets(&self) -> Vec<BulletPose> {
        self.bullets
            .values()
            .filter_map(|b| {
                Some(BulletPose {
                    image: b.track.image,
                    x: b.x / 256,
                    y: b.y / 256,
                    facing: b.facing,
                    frame: b.track.frame?,
                })
            })
            .collect()
    }

    /// Death animations, explosions and impacts that are showing a frame.
    pub fn effects(&self) -> impl Iterator<Item = &Effect> {
        self.effects.iter().filter(|e| e.track.frame.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lo_files_give_their_used_slots_and_refuse_garbage() {
        // Supply Depot shape: 2 frames, 22 slots, only slot 0 used.
        let mut d = Vec::new();
        d.extend_from_slice(&2u32.to_le_bytes());
        d.extend_from_slice(&22u32.to_le_bytes());
        d.extend_from_slice(&16u32.to_le_bytes());
        d.extend_from_slice(&60u32.to_le_bytes());
        for frame in 0..2 {
            let _ = frame;
            d.extend_from_slice(&[(-23i8) as u8, (-26i8) as u8]);
            for _ in 1..22 {
                d.extend_from_slice(&[127, 127]);
            }
        }
        assert_eq!(parse_lo_slots(&d), Some(vec![(0, -23, -26)]));
        assert_eq!(parse_lo_slots(&d[..20]), None, "cut short");
        assert_eq!(parse_lo_slots(b"garbage"), None);
        let mut huge = d.clone();
        huge[4..8].copy_from_slice(&100_000u32.to_le_bytes());
        assert_eq!(parse_lo_slots(&huge), None, "slot count capped");
    }

    #[test]
    fn more_slots_burn_as_hit_points_fall() {
        assert_eq!(burning_slots(5, 1.0), 0);
        assert_eq!(burning_slots(5, 0.7), 0);
        assert_eq!(burning_slots(5, 0.5), 3);
        assert_eq!(burning_slots(5, 0.2), 5);
        assert_eq!(burning_slots(1, 0.5), 1, "the depot's single fire");
        assert_eq!(burning_slots(0, 0.1), 0);
    }

    #[test]
    fn facings_follow_the_screen_compass() {
        assert_eq!(facing_of(0, -10), 0, "up");
        assert_eq!(facing_of(10, 0), 64, "right");
        assert_eq!(facing_of(0, 10), 128, "down");
        assert_eq!(facing_of(-10, 0), 192, "left");
    }

    /// Differential test against the retail direction routine
    /// (`0x005692b0` with the arctangent search `0x00569120`): the engine's
    /// floating-point `facing_of` must agree with the retail table lookup to
    /// within one facing unit.  The retail table (64 ascending `tan * 256`
    /// bounds) is rebuilt analytically, so no retail data is needed.
    #[test]
    fn facing_matches_the_retail_direction_routine() {
        use crate::sim::{w15a04_direction, w16a04_signed_table_search};
        let table: Vec<u32> = (0..0x40)
            .map(|k| ((k as f64 * std::f64::consts::PI / 128.0).tan() * 256.0).round() as u32)
            .collect();
        let mut worst = 0;
        for dy in (-300..=300).step_by(7) {
            for dx in (-300..=300).step_by(7) {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let retail =
                    w15a04_direction(0, 0, dx, dy, |v| w16a04_signed_table_search(v, &table));
                let retail = retail.rem_euclid(256);
                let mine = facing_of(dx, dy) as i32;
                let diff = (retail - mine).rem_euclid(256);
                let diff = diff.min(256 - diff);
                worst = worst.max(diff);
                assert!(diff <= 1, "({dx},{dy}): retail {retail} engine {mine}");
            }
        }
        assert!(worst <= 1);
    }
}
