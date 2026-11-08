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
use crate::sim::World;

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
    Some(data.sprite(flingy.sprite)?.image)
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
            self.step(world);
            self.step_damage(world, lo);
            self.step_construction(world);
        }
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
        for u in world.units.iter().filter(|u| u.alive) {
            alive_ids.push(u.id);
            let ty = data.unit(u.type_id);
            let Some(image) = unit_image(data, u.type_id) else {
                continue;
            };
            let building = ty.is_building();
            let seen = self.units.entry(u.id).or_insert_with(|| UnitSeen {
                track: None,
                glow: None,
                cooldown: u.cooldown,
                x: u.x,
                y: u.y,
                facing: u.facing,
                image,
                building,
            });
            let fired = u.cooldown > seen.cooldown;
            let moving = !u.path.is_empty();
            let air = ty.ground_weapon == 255 && ty.air_weapon != 255
                || u.target
                    .and_then(|t| world.unit(t))
                    .is_some_and(|t| data.unit(t.type_id).is_flyer());
            if !building {
                let current = seen.track.as_ref().map(|t| t.anim);
                let ended = seen
                    .track
                    .as_ref()
                    .is_none_or(|t| t.state.ended || t.state.ignore_rest);
                let want = if fired {
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
            // Instant weapons (`appear at target unit / pos`, `appear at
            // source unit`, `self destruct`) never become a travelling
            // projectile: show their bullet image where it appears.
            if fired {
                let weapon = if air { ty.air_weapon } else { ty.ground_weapon };
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
        // Units that went away play their Death animation where they were.
        let gone: Vec<u32> = self
            .units
            .keys()
            .copied()
            .filter(|id| !alive_ids.contains(id))
            .collect();
        for id in gone {
            if let Some(s) = self.units.remove(&id)
                && let Some(track) = Track::start(data, s.image, anim::DEATH, s.facing)
            {
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

    /// The engine glow a moving unit shows on top (image), if any; it is
    /// drawn with the unit's own frame.
    pub fn unit_glow(&self, id: u32) -> Option<u16> {
        self.units.get(&id)?.glow
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
}
