//! Deterministic StarCraft rules simulation.
//!
//! The world is a fixed 24 Hz logical simulation.  It implements the recovered
//! data tables (units, weapons, flingy, orders, tech, upgrades), the damage
//! pipeline, production/research, the hard-coded build tree, worker mining,
//! grid pathfinding, unit orders, spells and transport loading.
//!
//! Every deterministic source of randomness (spell scatter, starting larva)
//! uses a small seeded LCG so replays are reproducible.  The retail engine's
//! pathfinding tie-breaking is recovered from the `0x7a305590` open-list heap
//! (see `docs/evidence/pathfinding_astar.txt`); other ordering choices are
//! exposed through `RulesProfile` where they are visible.

#![allow(clippy::result_unit_err)]

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;

use crate::combat::{Defense, StatusDamage, fp, resolve_damage_status};
use crate::data::{GameData, UnitType, WeaponTarget, WeaponType};
use crate::ids::{tech as tech_id, unit as uid, upgrade, weapon as wid};
use crate::iscript::{
    ANIM_AIR_ATTACK_INIT, ANIM_AIR_ATTACK_REPEAT, ANIM_CAST_SPELL, ANIM_GND_ATTACK_INIT,
    ANIM_GND_ATTACK_REPEAT, ANIM_INIT, ANIM_WALKING, ANIM_WALKING_TO_IDLE, AnimContext, AnimEvent,
    AnimState,
};
use crate::map::{Scenario, TILE_HIGH, TILE_PX, TILE_VERY_HIGH, Terrain, UnitProperties};
use crate::path::{Obstacles, PathRequest, Rect, RegionMap, find_path_with_regions};
use crate::projectile::{BulletState, PERSIST_DAMAGE_INTERVAL, Projectile, behavior, hit};
use crate::replay::Recorder;
use crate::tables::{TechType, UpgradeType};
use crate::tech as tree;
use crate::vision::Vision;

pub const TICKS_PER_SECOND: u32 = 24;
/// Real duration of one tick at the retail Fastest speed.  FACT (Desperate
/// Alliance recording): the 1800-second countdown (28,800 ticks of 16, see
/// `trigger::GAME_SECOND_TICKS`) ran from the first game frame (127.25 s)
/// to "Sons of Korhal" (1337.0 s), 1209.75 s, i.e. 42.0 ms per tick; the
/// score screen's "Elapsed Time: 20:20" is that real time.
pub const TICK_MS: u64 = 42;

/// Real seconds of `ticks` at Fastest (the score screen's Elapsed Time).
pub fn real_seconds(ticks: u64) -> u64 {
    ticks * TICK_MS / 1000
}
pub const MAX_ENERGY: i32 = 200;
/// `0x005cc6d0`: heroes and reactor-upgraded casters cap at 250.
pub const MAX_ENERGY_UPGRADED: i32 = 250;
/// Retail stores energy in 1/256 points (`CUnit+0xa2`), like shields and HP.
/// Ability costs in `techdata.dat` are whole points and are scaled by this.
pub const ENERGY_SCALE: i32 = 256;
/// Stim Pack duration in frames: retail sets the stim timer to 37 and
/// counts it down once every 8 frames.
pub const STIM_FRAMES: i32 = 37 * 8;
pub const MINERALS_PER_TRIP: i32 = 8;
pub const GAS_PER_TRIP: i32 = 8;
/// Retail MiningMinerals timer: `CUnit+0x54 = 0x4b` (75), written by
/// `FUN_005abe7f` and decremented every tick by `FUN_005d91e0`.
/// `0x5a0520`: Bunker Guard reloads `CUnit+0x54` with 0xf when it has no
/// target and only reacquires when the counter reaches zero.
pub const BUNKER_GUARD_RETRY_TICKS: u8 = 15;
/// Units firing from a Bunker get 64 px of weapon range (OpenBW
/// `weapon_max_range`: `if (u_in_bunker(u)) r += 64`).  The occupant
/// stands at the Bunker's centre, so from the Bunker's edge this is the
/// familiar "+1 range".
pub const BUNKER_GUARD_RANGE_BONUS_PX: i32 = 64;
/// `0x005cc810`: Broodlings (and dying units) receive `0x546` = 1350
/// frames from `0x005b5e20`.  Earlier revisions used 1800 from a secondary
/// source; the retail constant is 1350.
pub const BROODLING_LIFETIME_TICKS: i64 = 1350;

pub const MINING_TIMER_FRAMES: u16 = 75;
/// This engine starts at box contact and does not model the facing/entry and
/// gather-to-idle frames retail counts inside the field occupancy window.
pub const MINING_DWELL_EXTRA_FRAMES: u16 = 18;
/// Effective field dwell.  Measured Boot Camp 99-115 frames versus 88-92
/// with 75; 93 keeps Wasteland income within -1.7 % of its recording.
pub const MINING_FRAMES: u16 = MINING_TIMER_FRAMES + MINING_DWELL_EXTRA_FRAMES;
/// Frames a worker spends inside a refinery (OpenBW `order_HarvestGas`).
pub const GAS_MINING_FRAMES: u16 = 37;
pub const MINERAL_CAPACITY: i32 = 1500;
pub const GAS_CAPACITY: i32 = 5000;
pub const MAX_ORDER_QUEUE: usize = 8;
/// Ticks a unit may fail to approach an intermediate waypoint before it is
/// skipped (see `move_towards`).
pub const PATH_STALL_TICKS: u8 = 8;
/// Frames an SCV welds from one spot of its building before moving on.
pub const CONSTRUCT_HOP_TICKS: u64 = 48;
/// Gap between a worker's and a resource's / depot's units.dat boxes at
/// which mining or the hand-in happens (INFERENCE: contact, as transports).
pub const HARVEST_CONTACT_PX: i32 = 4;
/// A unit stalled within this distance of its final destination counts as
/// arrived (INFERENCE: 3 tiles; falsifier: a retail group move whose outer
/// units keep walking).
pub const ARRIVAL_CROWD_PX: i32 = 96;
pub const MAX_PRODUCTION_QUEUE: usize = 5;
/// The retail engine caps active Halo Rockets at 80; use the same hard cap for
/// every projectile so a pathological custom map cannot grow without bound.
pub const MAX_PROJECTILES: usize = 80;
/// Spider Mine (OpenBW `order_SpiderMine`): frames it waits before it
/// burrows, frames of its Burrow / UnBurrow animation (iscript 87), and so
/// the frames from laying to armed.
pub const SPIDER_MINE_WAIT_FRAMES: i64 = 60;
pub const SPIDER_MINE_BURROW_FRAMES: i64 = 4;
pub const SPIDER_MINE_ARM_TICKS: i64 = SPIDER_MINE_WAIT_FRAMES + SPIDER_MINE_BURROW_FRAMES;
/// Half side of the square an armed mine watches (3 tiles), how far it
/// follows a victim, and the box distance at which it explodes.
pub const SPIDER_MINE_SEEK_PX: i32 = 96;
pub const SPIDER_MINE_CHASE_PX: i32 = 576;
pub const SPIDER_MINE_BLAST_PX: i32 = 30;
/// Hatchery/Lair/Hive larva interval: 37 status updates (`0x25` reloaded by
/// `0x5dfa06`), one status update every 8 logical ticks.
pub const LARVA_SPAWN_TICKS: i32 = 37 * 8;
/// Carrier/Reaver hangar limits before upgrades.
pub const CARRIER_HANGAR: u8 = 4;
pub const REAVER_HANGAR: u8 = 5;
/// Detector building sight radius, in pixels, recovered from the retail
/// `unit_calculate_detected_flags` path.
pub const DETECTOR_BUILDING_RANGE: i32 = 7 * 32;
pub const SHIELD_REGEN_PER_TICK: i32 = 7;
pub const ZERG_REGEN_PER_TICK: i32 = 4;
/// Per-tick energy regeneration in 1/256 points (`FUN_7a16f...`: `+8` on
/// `CUnit+0xa2` every logical tick, clamped to the unit's maximum).
pub const ENERGY_REGEN_PER_TICK: i32 = 8;
/// Personal-cloak drain in 1/256 points per tick (Ghost 10, Wraith 13).
/// Cloaking suppresses the +8 regen, so the effective net drain is 18/21.
pub const GHOST_CLOAK_COST_PER_TICK: i32 = 10;
pub const WRAITH_CLOAK_COST_PER_TICK: i32 = 13;
/// Medic healing: 200/256 HP restored for 100/256 energy per tick.
pub const MEDIC_HEAL_PER_TICK: i32 = 200;
pub const MEDIC_HEAL_COST_PER_TICK: i32 = 100;
/// A Medic heals a patient whose box is within 30 px of its own (OpenBW
/// `order_MedicHeal`: `unit_target_in_range(u, 30)`).
pub const MEDIC_REACH_PX: i32 = 30;
/// An idle Medic looks for patients in the square of 160 px around it
/// (OpenBW `find_medic_target`: `square_at(position, 160)`).
pub const MEDIC_SEARCH_PX: i32 = 160;
/// Update count of one Acid Spore stack.  `FUN_005dc360` writes `0x96` into
/// the chosen one of the nine `CUnit+0x127..+0x12f` timer slots and
/// `FUN_005dc940` decrements them once per status update.  The spore timer
/// runs every eighth logical tick, so 150 updates last ~50 s at 24 Hz.
pub const ACID_SPORE_TICKS: u8 = 0x96;
/// Logical ticks between two Acid Spore timer updates (`FUN_005dc940`).
pub const ACID_SPORE_UPDATE_INTERVAL: u64 = 8;
/// Defensive Matrix duration in status updates: `FUN_005d2840` writes `0xa8`
/// to `CUnit+0x114` and `FUN_005cf290` decrements it every eighth tick.
pub const DEFENSIVE_MATRIX_UPDATES: u8 = 0xa8;
/// Ensnare/Plague timer `0x4b` at `CUnit+0x116`/`+0x11a`.
pub const ENSNARE_UPDATES: i32 = 0x4b;
pub const PLAGUE_UPDATES: i32 = 0x4b;
/// Irradiate uses the weapon cooldown (`weapons.dat` 34) as its timer and
/// damages biological units within 32 px of the irradiated unit's box.
pub const IRRADIATE_UPDATES: i32 = 0x4b;
pub const IRRADIATE_RADIUS_PX: i32 = 32;
/// Lockdown (`CUnit+0x117`) and Stasis Field (`CUnit+0x119`) timers `0x83`.
pub const LOCKDOWN_UPDATES: i32 = 0x83;
pub const STASIS_UPDATES: i32 = 0x83;
/// Maelstrom timer `0x16` at `CUnit+0x124`.
pub const MAELSTROM_UPDATES: i32 = 0x16;
/// Creep spreads/recedes one maturity step every 8 logical ticks.
pub const CREEP_GROW_INTERVAL: u32 = 8;
/// `OWNR` value of a "Rescue Passive" player.
pub const OWNR_RESCUE_PASSIVE: u8 = 3;
/// Ticks between two rescue checks (retail order 160 handler resets its
/// timer at `+0x54` to 15).
pub const RESCUE_INTERVAL: u64 = 15;
/// Centre distance at which a rescuable unit is rescued.  Retail Wasteland
/// recording (2026-11-05, `Screencast_20261105_074011`): Raynor changes
/// hands at 15.7 s with the nearest Marine ~55 px away, not at ~65-75 px
/// half a second earlier; 2 tiles is the INFERENCE that fits.
pub const RESCUE_RANGE_PX: i32 = 64;

/// `units.dat` build times, `techdata.dat` research times and
/// `upgrades.dat` times are game frames, i.e. simulation ticks.  FACT
/// (Backwater Station recording, 2026-11-05): the Command Center's SCV
/// progress bar runs 220.2 s -> 232.8 s = 12.6 s = 300 frames x 42 ms
/// (`TICK_MS`), and a Barracks Marine fills at 16.4 px/s over the 240 px
/// bar = 14.6 s ~ 360 x 42 ms.  The old 1/15 s and 1/60 s scalings made
/// production 1.6x and upgrades 0.4x the retail duration.
pub fn build_ticks(build_time: u16) -> i32 {
    build_time as i32
}

pub fn research_ticks(research_time: u16) -> i32 {
    research_time as i32
}

pub fn upgrade_ticks(time: u32) -> i32 {
    time as i32
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Race {
    Zerg,
    Terran,
    Protoss,
    Neutral,
}

impl Race {
    pub fn from_unit(unit: &UnitType) -> Self {
        match unit.group_flags & 0x0f {
            9 => Race::Zerg,
            10 => Race::Terran,
            12 => Race::Protoss,
            _ => Race::Neutral,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CarryKind {
    None,
    Minerals,
    Gas,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellTarget {
    None,
    Unit(u32),
    Point(i32, i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spell {
    StimPacks,
    Lockdown,
    EmpShockwave,
    SpiderMines,
    ScannerSweep,
    TankSiegeMode,
    DefensiveMatrix,
    Irradiate,
    YamatoGun,
    CloakingField,
    PersonnelCloaking,
    Burrowing,
    SpawnBroodlings,
    DarkSwarm,
    Plague,
    Consume,
    Ensnare,
    Parasite,
    Infestation,
    PsionicStorm,
    Hallucination,
    Recall,
    StasisField,
    ArchonWarp,
    Restoration,
    DisruptionWeb,
    MindControl,
    DarkArchonMeld,
    Feedback,
    OpticalFlare,
    Maelstrom,
    LurkerAspect,
    Healing,
}

impl Order {
    /// `orders.dat` row used by this high-level order.  Only the queue/UI
    /// metadata is read from the row; the simulation still keeps its typed
    /// order state.
    pub fn order_id(&self) -> u8 {
        match self {
            Order::Idle => 0,
            Order::Stop => 1,
            Order::Move { .. } => 6,
            Order::AttackMove { .. } => 13,
            Order::AttackUnit { .. } => 8,
            Order::HoldPosition => 106,
            Order::Patrol { .. } => 151,
            Order::Gather { .. } => 78,
            Order::ReturnCargo { .. } => 85,
            Order::Build { .. } => 15,
            Order::Repair { .. } => 34,
            Order::Construct { .. } => 33,
            Order::Train { .. } => 37,
            Order::Research { .. } => 74,
            Order::Upgrade { .. } => 75,
            Order::Cast { .. } => 80,
            Order::EnterTransport { .. } => 90,
            Order::PickupTransport { .. } => 94,
            Order::Unload => 110,
            Order::MoveUnload { .. } => 111,
            Order::Burrow => 115,
            Order::Unburrow => 117,
            Order::Siege => 97,
            Order::Unsiege => 98,
            Order::Lift => 73,
            Order::Land { .. } => 70,
            Order::Die => 0,
        }
    }
}

impl Spell {
    /// Every spell, for [`Self::from_tech`].
    pub const ALL: [Spell; 33] = [
        Spell::StimPacks,
        Spell::Lockdown,
        Spell::EmpShockwave,
        Spell::SpiderMines,
        Spell::ScannerSweep,
        Spell::TankSiegeMode,
        Spell::DefensiveMatrix,
        Spell::Irradiate,
        Spell::YamatoGun,
        Spell::CloakingField,
        Spell::PersonnelCloaking,
        Spell::Burrowing,
        Spell::SpawnBroodlings,
        Spell::DarkSwarm,
        Spell::Plague,
        Spell::Consume,
        Spell::Ensnare,
        Spell::Parasite,
        Spell::Infestation,
        Spell::PsionicStorm,
        Spell::Hallucination,
        Spell::Recall,
        Spell::StasisField,
        Spell::ArchonWarp,
        Spell::Restoration,
        Spell::DisruptionWeb,
        Spell::MindControl,
        Spell::DarkArchonMeld,
        Spell::Feedback,
        Spell::OpticalFlare,
        Spell::Maelstrom,
        Spell::LurkerAspect,
        Spell::Healing,
    ];

    /// The spell a `techdata.dat` id casts (a card button's `act_var`).
    pub fn from_tech(tech: u8) -> Option<Spell> {
        Self::ALL.into_iter().find(|s| s.tech_id() == tech)
    }

    pub fn tech_id(self) -> u8 {
        match self {
            Spell::StimPacks => tech_id::STIM_PACKS,
            Spell::Lockdown => tech_id::LOCKDOWN,
            Spell::EmpShockwave => tech_id::EMP_SHOCKWAVE,
            Spell::SpiderMines => tech_id::SPIDER_MINES,
            Spell::ScannerSweep => tech_id::SCANNER_SWEEP,
            Spell::TankSiegeMode => tech_id::TANK_SIEGE_MODE,
            Spell::DefensiveMatrix => tech_id::DEFENSIVE_MATRIX,
            Spell::Irradiate => tech_id::IRRADIATE,
            Spell::YamatoGun => tech_id::YAMATO_GUN,
            Spell::CloakingField => tech_id::CLOAKING_FIELD,
            Spell::PersonnelCloaking => tech_id::PERSONNEL_CLOAKING,
            Spell::Burrowing => tech_id::BURROWING,
            Spell::SpawnBroodlings => tech_id::SPAWN_BROODLINGS,
            Spell::DarkSwarm => tech_id::DARK_SWARM,
            Spell::Plague => tech_id::PLAGUE,
            Spell::Consume => tech_id::CONSUME,
            Spell::Ensnare => tech_id::ENSNARE,
            Spell::Parasite => tech_id::PARASITE,
            Spell::Infestation => tech_id::INFESTATION,
            Spell::PsionicStorm => tech_id::PSIONIC_STORM,
            Spell::Hallucination => tech_id::HALLUCINATION,
            Spell::Recall => tech_id::RECALL,
            Spell::StasisField => tech_id::STASIS_FIELD,
            Spell::ArchonWarp => tech_id::ARCHON_WARP,
            Spell::Restoration => tech_id::RESTORATION,
            Spell::DisruptionWeb => tech_id::DISRUPTION_WEB,
            Spell::MindControl => tech_id::MIND_CONTROL,
            Spell::DarkArchonMeld => tech_id::DARK_ARCHON_MELD,
            Spell::Feedback => tech_id::FEEDBACK,
            Spell::OpticalFlare => tech_id::OPTICAL_FLARE,
            Spell::Maelstrom => tech_id::MAELSTROM,
            Spell::LurkerAspect => tech_id::LURKER_ASPECT,
            Spell::Healing => tech_id::HEALING,
        }
    }
}

impl Spell {
    /// `orders.dat` weapon column for this ability, when the order has one.
    /// The generic cast handler validates unit targets with the same
    /// `FUN_0059ca70` predicate used by weapons (`0x005cbea0`).
    pub fn weapon_id(self) -> Option<u8> {
        Some(match self {
            Spell::Lockdown => wid::LOCKDOWN,
            Spell::EmpShockwave => wid::EMP_SHOCKWAVE,
            Spell::SpiderMines => wid::SPIDER_MINES,
            Spell::Irradiate => wid::IRRADIATE,
            Spell::YamatoGun => wid::YAMATO_GUN,
            Spell::SpawnBroodlings => wid::SPAWN_BROODLINGS,
            Spell::DarkSwarm => wid::DARK_SWARM,
            Spell::Plague => wid::PLAGUE,
            Spell::Consume => wid::CONSUME,
            Spell::Ensnare => wid::ENSNARE,
            Spell::Parasite => wid::PARASITE,
            Spell::PsionicStorm => wid::PSIONIC_STORM,
            Spell::StasisField => wid::STASIS_FIELD,
            Spell::Restoration => wid::RESTORATION,
            Spell::DisruptionWeb => wid::DISRUPTION_WEB,
            Spell::MindControl => wid::MIND_CONTROL,
            Spell::Feedback => wid::FEEDBACK,
            Spell::OpticalFlare => wid::OPTICAL_FLARE,
            Spell::Maelstrom => wid::MAELSTROM,
            _ => return None,
        })
    }

    /// Spells whose order carries a unit target and therefore use the
    /// recovered weapon target predicate.
    pub fn takes_unit_target(self) -> bool {
        matches!(
            self,
            Spell::Lockdown
                | Spell::Irradiate
                | Spell::YamatoGun
                | Spell::SpawnBroodlings
                | Spell::Consume
                | Spell::Parasite
                | Spell::Restoration
                | Spell::MindControl
                | Spell::Feedback
                | Spell::OpticalFlare
                | Spell::DefensiveMatrix
                | Spell::Hallucination
                | Spell::Healing
                | Spell::Infestation
        )
    }
}

pub use crate::orders::{OrderClass, OrderLane, OrderSpec, lane, order_class, spec};

/// True when the recovered order dispatcher runs this row's handler on the
/// current logical tick.  Retail keeps a per-unit carry counter at
/// `CUnit+0x85`: the four-case fast switch and the 36-case worker/attack
/// switch run every tick, while the remaining 128 direct cases wait for the
/// counter to reach zero (every 8th tick).  `carry` is that counter before it
/// is decremented for the tick; it is `0` on the first tick after spawn.
pub fn order_runs_this_tick(id: u8, carry: u8) -> bool {
    match lane(id) {
        OrderLane::EveryTick => true,
        OrderLane::Carry8 => carry == 0,
        OrderLane::Alias(target) => order_runs_this_tick(target, carry),
        OrderLane::Production | OrderLane::Engine => true,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Order {
    Idle,
    Stop,
    Move {
        pos: (i32, i32),
    },
    AttackMove {
        pos: (i32, i32),
    },
    AttackUnit {
        target: u32,
    },
    HoldPosition,
    Patrol {
        a: (i32, i32),
        b: (i32, i32),
        forward: bool,
    },
    Gather {
        resource: u32,
    },
    ReturnCargo {
        depot: u32,
    },
    Build {
        unit_type: u16,
        pos: (i32, i32),
    },
    Repair {
        target: u32,
    },
    /// A Terran SCV building `building` (orders.dat row 33,
    /// ConstructingBuilding): construction only advances while an SCV holds
    /// this order on it.
    Construct {
        building: u32,
    },
    Train {
        unit_type: u16,
    },
    Research {
        tech_id: u8,
    },
    Upgrade {
        upgrade_id: u8,
        level: u8,
    },
    Cast {
        spell: Spell,
        target: SpellTarget,
    },
    EnterTransport {
        transport: u32,
    },
    /// A transport flying over to pick up a unit that asked to board
    /// (orders.dat row 94, OpenBW `order_PickupTransport`).
    PickupTransport {
        unit: u32,
    },
    Unload,
    /// Fly to a point and unload there (orders.dat row 111, OpenBW
    /// `order_MoveUnload`).
    MoveUnload {
        pos: (i32, i32),
    },
    Burrow,
    Unburrow,
    Siege,
    Unsiege,
    /// Lift a Terran building off the ground (row 73).
    Lift,
    /// Land a lifted building at a position (row 70).
    Land {
        pos: (i32, i32),
    },
    Die,
}

#[derive(Clone, Debug, Default)]
pub struct Status {
    /// Defensive Matrix pool in 1/256 points.
    pub matrix: i32,
    /// `CUnit+0x114`: remaining Defensive Matrix status updates.  Set to
    /// `0xa8` by `FUN_005d2840`, decremented every eighth tick by
    /// `FUN_005cf290`, and cleared when the pool breaks (`FUN_005d2300`).
    pub matrix_ticks: u8,
    /// Number of active Acid Spore stacks (`CUnit+0x126`).  Kept in sync with
    /// [`Status::acid_spore_ticks`] by the stack helpers.
    pub acid_spores: u8,
    /// One independent lifetime per Acid Spore stack (`CUnit+0x127..+0x12f`);
    /// zero means the slot is free.
    pub acid_spore_ticks: [u8; 9],
    pub hallucination: bool,
    pub stim_ticks: i32,
    pub lockdown_ticks: i32,
    pub stasis_ticks: i32,
    pub irradiate_ticks: i32,
    pub plague_ticks: i32,
    pub ensnare_ticks: i32,
    pub maelstrom_ticks: i32,
    pub disruption_ticks: i32,
    pub parasite: bool,
    /// `CUnit+0x121`: per-unit 8-bit mask, OR-merged on morph/merge
    /// (`0x005c1fe0`/`0x005c206d`).
    pub status_mask_121: u8,
    /// `CUnit+0x123`: second per-unit 8-bit mask, merged the same way.
    pub status_mask_123: u8,
    pub optical_flare_ticks: i32,
    pub cloaked: bool,
    pub burrowed: bool,
    pub sieged: bool,
    /// Scanner Sweep / detector reveal timer.
    pub revealed_ticks: i32,
    /// `CUnit+0x11b`: one-update guard that prevents Psionic Storm from
    /// hitting the same unit twice in one damage cadence (`0x005b4c20`
    /// reads it, `0x005b67d0` sets it to 1).
    pub psi_storm_guard: u8,
}

impl Status {
    /// Apply one Acid Spore stack the way `FUN_005dc360` does: fill the first
    /// free timer slot; when all nine are busy, refresh the stack closest to
    /// expiring and keep the count at nine.
    pub fn add_acid_spore(&mut self) {
        let slot = match self.acid_spore_ticks.iter().position(|&ticks| ticks == 0) {
            Some(slot) => slot,
            None => self
                .acid_spore_ticks
                .iter()
                .enumerate()
                .min_by_key(|&(_, &ticks)| ticks)
                .map(|(slot, _)| slot)
                .unwrap_or(0),
        };
        self.acid_spore_ticks[slot] = ACID_SPORE_TICKS;
        self.sync_acid_spore_count();
    }

    /// One logical tick of the nine independent spore timers
    /// (`FUN_005dc940`): every expiry removes one stack.
    pub fn tick_acid_spores(&mut self) {
        for ticks in &mut self.acid_spore_ticks {
            *ticks = ticks.saturating_sub(1);
        }
        self.sync_acid_spore_count();
    }

    /// Restoration and unit removal wipe every stack (`FUN_005dc4a0`).
    pub fn clear_acid_spores(&mut self) {
        self.acid_spores = 0;
        self.acid_spore_ticks = [0; 9];
    }

    /// Start (or refresh) a Defensive Matrix.  `0x005d2840` writes 64000
    /// (250 points in 1/256) and the duration `0xa8` to `CUnit+0x114`.
    pub fn apply_matrix(&mut self, points: i32) {
        self.matrix = points.max(0);
        self.matrix_ticks = if self.matrix > 0 {
            DEFENSIVE_MATRIX_UPDATES
        } else {
            0
        };
    }

    /// `0x005c1fe0`/`0x005c206d`: merge a source unit's status into this
    /// one.  Each timer keeps the larger value; the matrix pool follows a
    /// longer matrix timer and the larger acid-spore stack wins.
    pub fn merge_from(&mut self, other: &Status) {
        macro_rules! keep_max {
            ($field:ident) => {
                if other.$field > self.$field {
                    self.$field = other.$field;
                }
            };
        }
        if other.matrix_ticks > self.matrix_ticks {
            self.matrix_ticks = other.matrix_ticks;
            self.matrix = other.matrix;
        }
        keep_max!(stim_ticks);
        keep_max!(lockdown_ticks);
        keep_max!(stasis_ticks);
        keep_max!(irradiate_ticks);
        keep_max!(plague_ticks);
        keep_max!(ensnare_ticks);
        keep_max!(maelstrom_ticks);
        keep_max!(optical_flare_ticks);
        keep_max!(disruption_ticks);
        if other.acid_spores > self.acid_spores {
            self.acid_spores = other.acid_spores;
            self.acid_spore_ticks = other.acid_spore_ticks;
        }
        self.parasite |= other.parasite;
        self.status_mask_121 |= other.status_mask_121;
        self.status_mask_123 |= other.status_mask_123;
    }

    /// `0x005d21b0`/`0x005d21bb`/`0x005d227a`: a dying unit drops the
    /// removable status timers (ensnare, plague, irradiate, lockdown,
    /// maelstrom) and every acid spore stack.
    pub fn clear_on_death(&mut self) {
        self.ensnare_ticks = 0;
        self.plague_ticks = 0;
        self.irradiate_ticks = 0;
        self.lockdown_ticks = 0;
        self.maelstrom_ticks = 0;
        self.clear_acid_spores();
    }

    /// `0x005d2900`: lockdown refresh keeps the larger of the current and
    /// incoming timer (`CUnit+0x117`).
    pub fn apply_lockdown(&mut self, ticks: i32) {
        self.lockdown_ticks = self.lockdown_ticks.max(ticks);
    }

    /// `0x005d2620`/`0x005d27a0`: expire lockdown (`CUnit+0x117 = 0`).
    pub fn clear_lockdown(&mut self) {
        self.lockdown_ticks = 0;
    }

    /// `0x005bb640`: stasis refresh keeps the larger timer (`CUnit+0x119`).
    pub fn apply_stasis(&mut self, ticks: i32) {
        self.stasis_ticks = self.stasis_ticks.max(ticks);
    }

    /// `0x005bb5e0`: expire stasis (`CUnit+0x119 = 0`).
    pub fn clear_stasis(&mut self) {
        self.stasis_ticks = 0;
    }

    /// `0x005d2a80`: maelstrom refresh keeps the larger timer (`CUnit+0x124`).
    pub fn apply_maelstrom(&mut self, ticks: i32) {
        self.maelstrom_ticks = self.maelstrom_ticks.max(ticks);
    }

    /// `0x005d2660`/`0x005d27f0`: expire maelstrom (`CUnit+0x124 = 0`).
    pub fn clear_maelstrom(&mut self) {
        self.maelstrom_ticks = 0;
    }

    /// `0x005dc6d0`: plague writes the fixed `0x4b` update count to
    /// `CUnit+0x11a`.
    pub fn apply_plague(&mut self) {
        self.plague_ticks = PLAGUE_UPDATES;
    }

    /// `0x005dc300`: expire plague (`CUnit+0x11a = 0`).
    pub fn clear_plague(&mut self) {
        self.plague_ticks = 0;
    }

    /// `0x005dc710`: ensnare writes the fixed `0x4b` update count to
    /// `CUnit+0x116`.
    pub fn apply_ensnare(&mut self) {
        self.ensnare_ticks = ENSNARE_UPDATES;
    }

    /// `0x005dc4e0`: expire ensnare (`CUnit+0x116 = 0`).
    pub fn clear_ensnare(&mut self) {
        self.ensnare_ticks = 0;
    }

    /// `0x005d28a0`: irradiate writes the duration table value to
    /// `CUnit+0x118`.
    pub fn apply_irradiate(&mut self, ticks: i32) {
        self.irradiate_ticks = ticks;
    }

    /// `0x005d2540`: expire irradiate (`CUnit+0x118 = 0`).
    pub fn clear_irradiate(&mut self) {
        self.irradiate_ticks = 0;
    }

    /// True while the matrix pool and its duration timer are both live.
    pub fn matrix_active(&self) -> bool {
        self.matrix_ticks > 0 && self.matrix > 0
    }

    /// One status update for the matrix duration (`FUN_005cf290`).  When the
    /// last update expires, the pool is cleared and the consumed amount is
    /// returned so the caller can emit the break cue.
    pub fn tick_matrix(&mut self) -> Option<i32> {
        if self.matrix_ticks == 0 {
            return None;
        }
        self.matrix_ticks -= 1;
        if self.matrix_ticks == 0 {
            let pool = self.matrix.max(0);
            self.matrix = 0;
            Some(pool)
        } else {
            None
        }
    }

    /// True when Restoration has something to remove (`FUN_005c7460`):
    /// parasite, optical flare, ensnare, plague, irradiate, lockdown,
    /// maelstrom or acid spores.  Stasis Field is deliberately excluded.
    pub fn has_removable_effect(&self) -> bool {
        self.parasite
            || self.lockdown_ticks > 0
            || self.plague_ticks > 0
            || self.irradiate_ticks > 0
            || self.maelstrom_ticks > 0
            || self.ensnare_ticks > 0
            || self.optical_flare_ticks > 0
            || self.acid_spores > 0
    }

    /// Consume a hit from the matrix exactly as `FUN_005d2300`: a hit smaller
    /// than the pool leaves the overlay active; a hit that reaches or exceeds
    /// the pool consumes it, clears the flag and returns `broke = true`.
    pub fn absorb_matrix(&mut self, amount: i32) -> (i32, bool) {
        let amount = amount.max(0);
        let pool = self.matrix.max(0);
        if amount < pool {
            self.matrix = pool - amount;
            (amount, false)
        } else {
            self.matrix = 0;
            self.matrix_ticks = 0;
            (pool, pool > 0)
        }
    }

    fn sync_acid_spore_count(&mut self) {
        self.acid_spores = self.acid_spore_ticks.iter().filter(|&&t| t > 0).count() as u8;
    }
}

/// A Defensive Matrix cue emitted by `FUN_005d2300`: `broke` selects the
/// shield-down cue, otherwise the matrix hit cue.  The simulation stores the
/// position and absorbed amount so the client can play/draw the cue without
/// re-reading unit state after a later tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatrixCue {
    pub unit: u32,
    pub x: i32,
    pub y: i32,
    /// 1/256 points consumed by this hit.
    pub absorbed: i32,
    pub broke: bool,
}

/// One staged nuclear launch (`0x5d3823` Ghidra entry; real prologue at
/// `0x5d3890`): the Ghost paints the target,
/// waits 90 ticks, emits the 45-tick warning, then the missile travels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NukeStrike {
    pub owner: u8,
    pub ghost: u32,
    pub silo: u32,
    pub target: (i32, i32),
    /// Ticks left before the missile is created.
    pub timer: i32,
    /// Target dot is still painted.
    pub dot: bool,
    /// Warning cue already emitted.
    pub warned: bool,
    /// The missile unit climbing out of the silo (0 before it launches).
    pub missile: u32,
}

/// One instant-weapon hit that lands when its bullet's script says so.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingHit {
    /// Tick on which the damage is dealt.
    pub due: u64,
    pub owner: u8,
    pub source: u32,
    pub target: Option<u32>,
    pub weapon: u8,
    /// Impact point in 1/256 px.
    pub pos: (i32, i32),
}

/// What a [`SpellCue`] announces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellCueKind {
    /// A unit used a Stim Pack.
    Stim,
    /// The Lockdown missile disabled its target.
    LockdownHit,
    /// Restoration cleansed the unit.
    Restoration,
    /// A Medic fired Optical Flare (the cue's unit is the caster).
    OpticalFlareCast,
    /// Optical Flare blinded the unit.
    OpticalFlareHit,
}

/// One spell event for the renderer (see [`World::spell_cues`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpellCue {
    pub kind: SpellCueKind,
    pub unit: u32,
}

/// Frames the nuclear missile spends off the map between its climb and its
/// fall: its WarpIn animation (iscript 131: two `wait 125`).
pub const NUKE_WARP_FRAMES: i32 = 250;
/// The nuclear missile's flingy (`flingy.dat` row 109): acceleration 33 and
/// top speed 8533, in 1/256 px per frame.  It starts both its climb and its
/// fall from rest.
pub const NUKE_ACCELERATION: i32 = 33;
pub const NUKE_TOP_SPEED: i32 = 8533;

/// Frames a tank is busy deploying into Siege Mode and packing up again
/// (iscript 92: Init and SpecialState2 up to their `sigorder 1`).
pub const SIEGE_FRAMES: u8 = 64;
/// `units.dat` default directions x 8: the tank (and its turret) before
/// sieging, and the sieged cannon at rest (iscript 93 ends its unfolding
/// with `setfldirect 28`).
pub const SIEGE_HEADING: i32 = 96;
pub const SIEGED_TURRET_HEADING: i32 = 224;
pub const UNSIEGE_FRAMES: u8 = 63;

/// Frames between two units leaving a transport (OpenBW `unit_unload`:
/// `container->main_order_timer = 15`).
pub const TRANSPORT_UNLOAD_FRAMES: u8 = 15;

/// The Yamato Gun's charge overlay (`images.dat` 543, iscript 314): its
/// `castspell` releases the shot.
pub const YAMATO_CHARGE_IMAGE: u16 = 543;

/// Spell cues kept for the renderer.
pub const SPELL_CUE_KEEP: usize = 64;

/// `units.dat` group flags (OpenBW `GroupFlags`): Men and Building.
pub const GROUP_MEN: u8 = 0x08;
pub const GROUP_BUILDING: u8 = 0x10;

#[derive(Clone, Debug)]
pub struct SimUnit {
    pub id: u32,
    pub owner: u8,
    pub type_id: u16,
    pub hp: i32,
    pub shields: i32,
    pub energy: i32,
    /// Sub-point energy in 1/256, matching retail's `CUnit+0xa2` fixed-point
    /// storage.  `energy` remains the whole-point view used by ability costs.
    pub energy_frac: i32,
    /// `0x005cc6d0` maximum energy in 1/256 points (200 or 250).
    pub energy_cap: i32,
    pub x: i32,
    pub y: i32,
    pub facing: u8,
    pub alive: bool,
    /// False while a building is under construction.
    pub completed: bool,
    /// The unit's current completion has been credited to its owner's
    /// score (`World::credit_scores`); cleared when a morph or construction
    /// starts so the finished form is credited again.
    pub credited: bool,
    /// True while a Terran building is lifted off the ground (`orders.dat`
    /// rows 69-73).  Lifted buildings move as air units and cannot produce.
    pub lifted: bool,
    /// Lift-off or landing in progress: `(landing, ticks elapsed)`.  The
    /// building rises (or sets down) [`crate::unit_anim::LIFT_HEIGHT`] px at
    /// 1 px per tick and takes no orders until the phase ends (OpenBW
    /// `order_LiftingOff`, `order_BuildingLand`).
    pub lift_phase: Option<(bool, u16)>,
    /// Add-on attachment state.  Lifted parents leave their add-on grounded
    /// and inactive; landing on it re-attaches it.
    pub addon_attached: bool,
    /// Remaining construction/training/research ticks.
    pub build_remaining: i32,
    pub build_total: i32,
    pub cooldown: u32,
    pub order: Order,
    /// Order to resume when a temporary attack (from attack-move) ends.
    pub resume_order: Option<Order>,
    pub queue: VecDeque<Order>,
    pub target: Option<u32>,
    pub path: VecDeque<(i32, i32)>,
    pub path_goal: Option<(i32, i32)>,
    /// Ticks without getting closer to the current waypoint (crowds).
    pub path_stall: u8,
    /// Current speed of a flingy-controlled unit (1/256 px per tick) and
    /// the tick it last moved (a unit that skipped a tick starts from 0).
    pub speed: i32,
    pub moved_tick: u64,
    /// Distance to the current waypoint last tick (1/256 px).
    pub path_last_dist: i32,
    pub rally: Option<(i32, i32)>,
    pub carry: i32,
    pub carry_kind: CarryKind,
    /// Frames left mining the patch (or inside the refinery) it is
    /// gathering; 0 when not mining.  The mineral timer is 75 frames plus
    /// the unmodelled entry/exit dwell; gas is 37 frames.
    pub gather_timer: u16,
    pub resource_target: Option<u32>,
    pub depot: Option<u32>,
    pub production: VecDeque<Order>,
    pub addon: Option<u32>,
    /// Units currently loaded into this transport/bunker.
    pub cargo: Vec<u32>,
    pub in_transport: Option<u32>,
    pub status: Status,
    /// Larva spawn timer for Zerg hatcheries.
    pub larva_timer: i32,
    /// Resource capacity for mineral fields/geysers.
    pub resource_amount: i32,
    /// For Zerg eggs: the unit that will hatch when construction finishes.
    pub morph_to: Option<u16>,
    /// Last damage tick, for damage overlays/telemetry.
    pub last_damaged: u64,
    /// Trigger action 43 (Set Invincibility) or a UPRP invincible slot.
    pub invincible: bool,
    /// Unit that last dealt damage, used to credit kills and razings.
    pub last_damage_source: u32,
    /// SCV constructing this (Terran) building, 0 when none.
    pub builder: u32,
    /// Units and buildings this unit destroyed (the selection panel's
    /// "Kills:" line).
    pub kills: u32,
    /// Trigger action 42 (Set Doodad State); `None` for ordinary units.
    pub doodad_enabled: Option<bool>,
    /// A completed Nuclear Silo holds one armed missile (`CUnit+0xd4`).
    pub nuke_ready: bool,
    /// Paired Nydus Canal exit, when the map linked the two records by serial.
    pub nydus_exit: Option<u32>,
    /// Tick at which a temporary spell unit (Dark Swarm, Disruption Web,
    /// Scanner Sweep, Nuclear Missile) disappears.  Zero means permanent.
    pub expires_at: i64,
    /// Tick at which a Spider Mine finishes arming.
    pub mine_armed_at: i64,
    /// Air-unit repulse state (OpenBW `repulse_direction` / `repulse_flags`):
    /// the direction a flyer drifts when it shares a 48 px cell with other
    /// flyers, a 3-bit countdown before it picks a new one (low bits) and
    /// its row of the adjust table (high nibble, 0 = not initialised).
    pub repulse_dir: u8,
    pub repulse_flags: u8,
    /// Heading of the unit's turret (Siege Tank, Goliath), which turns on
    /// its own at the turret's flingy turn rate.
    pub turret_facing: u8,
    /// Fractional mineral debt for SCV repair, in 1/256 minerals.
    pub repair_debt: i32,
    /// Interceptors (Carrier) or scarabs (Reaver) currently in the hangar.
    pub hangar: u8,
    pub hangar_capacity: u8,
    /// Personal weapon cooldown used by carriers and reavers, whose attack is
    /// driven by a launched unit rather than a normal weapon row.
    pub special_cooldown: i32,
    /// Per-unit order-carry counter (`CUnit+0x85`).  Initialised to zero so
    /// the carry-gated handlers run on the first tick, then reset to 8.
    pub order_carry: u8,
    /// `CUnit+0x8a`: remaining unit-voice timer, decremented every logic
    /// tick (`0x005b65a0`).  `speech_id` is the `CUnit+0x8b` sound/line id.
    pub speech_ticks: u8,
    pub speech_id: u8,
    /// `CUnit+0x87`: generic countdown timer set by `0x005b6800`,
    /// decremented by `0x005b3bc0` and tested by `0x005b62a0`.
    pub timer_087: u8,
    /// `CUnit+0x90`: byte initialised to 8 by the unit constructor;
    /// purpose not recovered, getter `0x005b4ad0`.
    pub field_090: u8,
    /// `CUnit+0x94`: unrecovered u16 field, getter `0x005b4cb0`.
    pub field_094: u16,
    /// `CUnit+0xac`: unrecovered u16 field, getter `0x005b49e0`.
    pub field_0ac: u16,
    /// `CUnit+0xe1`: per-unit variation byte, getter `0x005b4c30`.
    pub field_0e1: u8,
    /// `CUnit+0xdc` bit 10, set by `0x005b3b00`/cleared by `0x005b6750`.
    pub flag_10: bool,
    /// `CUnit+0xdc` bit 14, read by `0x005b6b50`.
    pub flag_14: bool,
    /// `CUnit+0xdc` bit 15, set by `0x005b6700`/cleared by `0x005b3ae0`
    /// and read by `0x005b5ad0`.
    pub flag_15: bool,
    /// `CUnit+0xdc` bit 29, set by `0x005b66b0`.
    pub flag_29: bool,
    /// `CUnit+0xdc` bit 16, set by `0x005d45e0`.
    pub flag_16: bool,
    /// `CUnit+0xdc` bit 17, set by `0x005d45e0` for non-subunits.
    pub flag_17: bool,
    /// `CUnit+0x107`: set when a Shield Battery recharged this unit during
    /// the current tick (`0x005d4700`), cleared by the status update
    /// (`0x005d91e0`).
    pub shield_recharged: bool,
    /// `CUnit+0xdc` bit 18, written by `0x005bd460`.
    pub flag_18: bool,
    /// `CUnit+0xdc` bit 19, written by `0x005bd4c0`.
    pub flag_19: bool,
    /// `CUnit+0x100`: opaque pointer slot (`0x005bd4e0` setter,
    /// `0x005bb910`/`0x005bb920` getters).
    pub field_100: u32,
    /// `CUnit+0xdc` bit 21, read by `0x005bb930`.
    pub flag_21: bool,
    /// `CUnit+0xdc` bit 22, read by `0x005bb940`.
    pub flag_22: bool,
    /// `CUnit+0x91`: byte cleared by `0x005b0115`.
    pub field_091: u8,
    /// `CUnit+0x93`: order/state flags byte; `0x005b6860` writes bit 1.
    pub flags_93: u8,
    /// `CUnit+0x96`: byte cleared by `0x005b3b50`.
    pub field_096: u8,
    /// `CUnit+0x105`: two low state bits read by
    /// `0x005bb7a0`/`0x005bb7b0`.
    pub flags_105: u8,
    /// `CUnit+0xe0`: overlay/state bits set by `0x005b31c0`.
    pub overlay_flags: u8,
    /// `CUnit+0xcf`: companion byte written by `0x005b31c0`.
    pub field_0cf: u8,
    /// `CUnit+0xdc` bit 26: capability bit retained only for types with
    /// `units.dat` flag 0x20000000 (`0x005b66c0`).
    pub flag_26: bool,
    /// `CUnit+0xdc` bit 6: loaded inside a Bunker (distinct from the
    /// transport bit 5).
    pub in_bunker: bool,
    /// `CUnit+0xe4`: per-player visibility bitmask.  Initialised to all
    /// visible and recomputed from the fog grid every tick.
    pub visibility_mask: u32,
    /// `CUnit+0x8f`: kill-rank counter, capped at 255 by
    /// `0x005a2ec0`/`0x005a2f5a`.
    pub rank_kills: u8,
    /// `CUnit+0x110`: corpse/death animation timer.  `0x005b6810` takes the
    /// minimum of the current value and the incoming one; `0x005b4cc0`
    /// reads it.
    pub death_timer: u16,
    /// `CUnit+0x54`: per-order wait counter (Bunker Guard's 15-tick retry).
    pub order_wait: u8,
    /// `CUnit+0x80`: the Hatchery/Lair/Hive that owns this larva.
    pub connected_unit: Option<u32>,
    /// `0x5dfe20` spawn-side scores: `[left, top, right, bottom]`.
    pub larva_spawn_side_values: [i32; 4],
    /// `orders.dat` row that produced the current typed order.  This keeps
    /// the exact recovered row (including internal rows) available to the
    /// handler dispatch layer.
    pub order_row: u8,
    /// Tick counter into the Walking iscript cycle.
    pub anim_tick: u32,
    /// Current iscript animation state, when the unit has one.
    pub anim: Option<AnimState>,
    /// Number of weapon events fired, used to select the repeat-attack
    /// animation instead of the initial attack animation.
    pub attacks_fired: u32,
}

impl SimUnit {
    /// A finished add-on with no building attached.  Retail gives it to
    /// the neutral player (OpenBW `building_abandon_addon`): it trains,
    /// researches and upgrades nothing until a building lands next to it.
    pub fn is_detached_addon(&self) -> bool {
        self.completed && !self.addon_attached && addon_parent(self.type_id).is_some()
    }

    pub fn is_completed_building(&self, data: &GameData) -> bool {
        self.completed && data.unit(self.type_id).is_building()
    }

    /// `0x005b51f0`: type currently at the head of the build queue, or the
    /// unit's own type when nothing is queued.
    pub fn current_build_unit(&self) -> u16 {
        match &self.order {
            Order::Train { unit_type } => *unit_type,
            _ => match self.production.front() {
                Some(Order::Train { unit_type }) => *unit_type,
                _ => self.type_id,
            },
        }
    }

    /// `0x005b4a00`: CUnit+0x126 getter (acid spore count).
    pub fn acid_spore_count(&self) -> u8 {
        self.status.acid_spores
    }

    /// `0x005b4b60`: raw `CUnit+0xc0` linked target.
    pub fn linked_target(&self) -> Option<u32> {
        self.order_unit_target()
    }

    /// Unit id carried by the order's `CUnit+0x5c` target slot, when the
    /// row targets a unit rather than a point.
    pub fn order_unit_target(&self) -> Option<u32> {
        match &self.order {
            Order::AttackUnit { target }
            | Order::Gather { resource: target }
            | Order::ReturnCargo { depot: target }
            | Order::Repair { target }
            | Order::Construct { building: target }
            | Order::EnterTransport { transport: target } => Some(*target),
            Order::Cast {
                target: SpellTarget::Unit(id),
                ..
            } => Some(*id),
            _ => None,
        }
    }

    /// `0x005a3cd0`: drop the order's unit target when that unit is gone.
    pub fn clear_order_target(&mut self, dead_id: u32) -> bool {
        if self.order_unit_target() == Some(dead_id) {
            self.order = Order::Idle;
            true
        } else {
            false
        }
    }

    /// `0x005a4030`: write `CUnit+0x85` from `param - 1` and report
    /// success to the order dispatcher.
    pub fn set_order_carry_from(&mut self, param: u8) -> bool {
        self.order_carry = param.wrapping_sub(1);
        true
    }

    /// `0x005a3180`: true when the unit has no sprite (hidden/dead) or its
    /// primary order id is zero.
    pub fn is_idle_or_hidden(&self) -> bool {
        !self.alive || matches!(self.order, Order::Idle)
    }

    /// `0x005b2d00`: true when `CUnit+0x4c` equals the supplied owner.
    pub fn owner_is(&self, owner: u8) -> bool {
        self.owner == owner
    }

    /// `0x005b3ad0`: clear the current target (`CUnit+0x7c = 0`).
    pub fn clear_target(&mut self) {
        self.target = None;
    }

    /// `0x005b4900`: set the current target (`CUnit+0x7c`); zero clears it.
    pub fn set_target(&mut self, id: u32) {
        self.target = (id != 0).then_some(id);
    }

    /// `0x005b6800`: write the `CUnit+0x87` countdown.
    pub fn set_timer_087(&mut self, ticks: u8) {
        self.timer_087 = ticks;
    }

    /// `0x005b3bc0`: decrement `CUnit+0x87` when nonzero.
    pub fn tick_timer_087(&mut self) {
        self.timer_087 = self.timer_087.saturating_sub(1);
    }

    /// `0x005b62a0`: true when `CUnit+0x87` is zero.
    pub fn timer_087_is_zero(&self) -> bool {
        self.timer_087 == 0
    }

    /// `0x005b6790`: queue a unit voice line: `CUnit+0x8a` lifetime and
    /// `CUnit+0x8b` sound id.
    pub fn say(&mut self, ticks: u8, sound_id: u8) {
        self.speech_ticks = ticks;
        self.speech_id = sound_id;
    }

    /// `0x005b65a0`: one tick of the voice timer; returns the previous
    /// nonzero value (the caller plays the line when it sees one).
    pub fn tick_speech(&mut self) -> Option<u8> {
        let ticks = self.speech_ticks;
        if ticks == 0 {
            return None;
        }
        self.speech_ticks = ticks - 1;
        Some(ticks)
    }

    /// `0x005b4ae0`: `CUnit+0x8b` sound/line id.
    pub fn speech_sound(&self) -> u8 {
        self.speech_id
    }

    /// `0x005b31c0`: OR `CUnit+0xe0` with 1 or 2 depending on `mode`, and
    /// write the companion byte `CUnit+0xcf`.
    pub fn set_overlay_state(&mut self, mode: bool, companion: u8) {
        self.overlay_flags |= if mode { 2 } else { 1 };
        self.field_0cf = companion;
    }

    /// `0x005b5620`: `CUnit+0xe0` bit 0.
    pub fn overlay_bit0(&self) -> bool {
        self.overlay_flags & 1 != 0
    }

    /// `0x005b5630`: `CUnit+0xe0` bit 1.
    pub fn overlay_bit1(&self) -> bool {
        self.overlay_flags >> 1 & 1 != 0
    }

    /// `0x005b3b00`: set `CUnit+0xdc` bit 10.
    pub fn set_flag_10(&mut self) {
        self.flag_10 = true;
    }

    /// `0x005b6750`: clear `CUnit+0xdc` bit 10.
    pub fn clear_flag_10(&mut self) {
        self.flag_10 = false;
    }

    /// `0x005b6b50`: `CUnit+0xdc` bit 14.
    pub fn flag_14(&self) -> bool {
        self.flag_14
    }

    /// `0x005b6700`: set `CUnit+0xdc` bit 15.
    pub fn set_flag_15(&mut self) {
        self.flag_15 = true;
    }

    /// `0x005b3ae0`: clear `CUnit+0xdc` bit 15.
    pub fn clear_flag_15(&mut self) {
        self.flag_15 = false;
    }

    /// `0x005b5ad0`: `CUnit+0xdc` bit 15.
    pub fn flag_15(&self) -> bool {
        self.flag_15
    }

    /// `0x005b66b0`: set `CUnit+0xdc` bit 29.
    pub fn set_flag_29(&mut self) {
        self.flag_29 = true;
    }

    /// `0x005b0115`: clear `CUnit+0x91`.
    pub fn clear_field_091(&mut self) {
        self.field_091 = 0;
    }

    /// `0x005b6860`: set or clear `CUnit+0x93` bit 1.
    pub fn set_flag_93_bit1(&mut self, set: bool) {
        if set {
            self.flags_93 |= 2;
        } else {
            self.flags_93 &= !2;
        }
    }

    /// `0x005b3b50`: clear `CUnit+0x96`.
    pub fn clear_field_096(&mut self) {
        self.field_096 = 0;
    }

    /// `0x005b48c0`: set `CUnit+0xdc` bit 5 (in transport) and link the
    /// carrier.  Bit 17 is not part of the Rust model.
    pub fn mark_in_transport(&mut self, transport: u32) {
        self.in_transport = Some(transport);
    }

    /// `0x005b48e0`: clear the `CUnit+0xdc` bit-5 transport link.
    pub fn clear_transport_link(&mut self) {
        self.in_transport = None;
    }

    /// `0x005bd460`: set or clear `CUnit+0xdc` bit 18.
    pub fn set_flag_18(&mut self, set: bool) {
        self.flag_18 = set;
    }

    /// `0x005bd4c0`: set or clear `CUnit+0xdc` bit 19.
    pub fn set_flag_19(&mut self, set: bool) {
        self.flag_19 = set;
    }

    /// `0x005bd4e0`: write the `CUnit+0x100` pointer slot.
    pub fn set_field_100(&mut self, value: u32) {
        self.field_100 = value;
    }

    /// `0x005bb910`: `CUnit+0x100` value.
    pub fn field_100(&self) -> u32 {
        self.field_100
    }

    /// `0x005bb920`: true when `CUnit+0x100` is nonzero.
    pub fn field_100_nonzero(&self) -> bool {
        self.field_100 != 0
    }

    /// `0x005d45e0`: set bit 16 and set/clear bit 17 from the subunit flag.
    /// The retail routine also clears status bit 1; that bit is not part of
    /// the Rust model.
    pub fn set_subunit_state(&mut self, is_subunit: bool) {
        self.flag_16 = true;
        self.flag_17 = !is_subunit;
    }

    /// `0x005b38f0`: `CUnit+0x107` shield-recharge flag.
    pub fn shield_recharge_flag(&self) -> bool {
        self.shield_recharged
    }

    /// `0x005bb930`: `CUnit+0xdc` bit 21.
    pub fn flag_21(&self) -> bool {
        self.flag_21
    }

    /// `0x005bb940`: `CUnit+0xdc` bit 22.
    pub fn flag_22(&self) -> bool {
        self.flag_22
    }

    /// `0x005bb7a0`: `CUnit+0x105` bit 0.
    pub fn flag_105_bit0(&self) -> bool {
        self.flags_105 & 1 != 0
    }

    /// `0x005bb7b0`: inverted `CUnit+0x105` bit 1.
    pub fn flag_105_bit1_clear(&self) -> bool {
        (self.flags_105 >> 1) & 1 == 0
    }

    /// `0x005b3abe`: `CUnit+0xdc` bit 26.
    pub fn flag_26(&self) -> bool {
        self.flag_26
    }

    /// `0x005b3aa0`: either burrow-state bit (`CUnit+0xdc & 0x3000`) set.
    pub fn is_burrowed_flags(&self) -> bool {
        self.status.burrowed
    }

    /// `0x005b5840`: `CUnit+0xdc` bit 4, the completed flag.
    pub fn is_completed_flag(&self) -> bool {
        self.completed
    }

    /// `0x005b5690`: `CUnit+0xdc` bit 6, set while loaded in a Bunker.
    pub fn is_in_bunker(&self) -> bool {
        self.in_bunker
    }

    /// `0x005b6840`: set or clear the Bunker bit 6.
    pub fn set_in_bunker(&mut self, flag: bool) {
        self.in_bunker = flag;
    }

    /// `0x005b5680`: `CUnit+0xdc` bit 5, set while loaded in a transport or
    /// Bunker (`0x00631c10` clears it during unload).
    pub fn is_loaded(&self) -> bool {
        self.in_transport.is_some()
    }

    /// `0x005b5940`: `CUnit+0xdc` bit 8, the first cloak-state bit.
    pub fn cloak_bit(&self) -> bool {
        self.cloak_state() != 0
    }

    /// `0x005b4ad0`: `CUnit+0x90` byte field.
    pub fn field_090(&self) -> u8 {
        self.field_090
    }

    /// `0x005b4cb0`: `CUnit+0x94` u16 field.
    pub fn field_094(&self) -> u16 {
        self.field_094
    }

    /// `0x005b49e0`: `CUnit+0xac` u16 field.
    pub fn field_0ac(&self) -> u16 {
        self.field_0ac
    }

    /// `0x005b4c30`: `CUnit+0xe1` variation byte.
    pub fn field_0e1(&self) -> u8 {
        self.field_0e1
    }

    /// `0x005b4a10`: `CUnit+0xe4` visibility mask.
    pub fn visibility_mask(&self) -> u32 {
        self.visibility_mask
    }

    /// `0x005b5a40`: `CUnit+0xe4 & mask`.
    pub fn visibility_mask_and(&self, mask: u32) -> u32 {
        self.visibility_mask & mask
    }

    /// `0x005b4b70`: `CUnit+0x123` mask getter.
    pub fn mask_123(&self) -> u8 {
        self.status.status_mask_123
    }

    /// `0x005b65d0`: clear the `CUnit+0x123` mask.
    pub fn clear_mask_123(&mut self) {
        self.status.status_mask_123 = 0;
    }

    /// `0x005b65e0`: set one bit (`1 << (bit & 31)`) in `CUnit+0x123`.
    pub fn set_mask_123_bit(&mut self, bit: u8) {
        self.status.status_mask_123 |= 1u8 << (bit & 31);
    }

    /// `0x005b4bc0`: `CUnit+0x121` mask getter.
    pub fn mask_121(&self) -> u8 {
        self.status.status_mask_121
    }

    /// `0x005b6640`: clear the `CUnit+0x121` mask.
    pub fn clear_mask_121(&mut self) {
        self.status.status_mask_121 = 0;
    }

    /// `0x005b6650`: set one bit (`1 << (bit & 31)`) in `CUnit+0x121`.
    pub fn set_mask_121_bit(&mut self, bit: u8) {
        self.status.status_mask_121 |= 1u8 << (bit & 31);
    }

    /// `0x005a2ec0`/`0x005a2f5a`: increment the kill-rank counter unless it
    /// is already 255.
    pub fn credit_rank_kill(&mut self) {
        self.rank_kills = self.rank_kills.saturating_add(1);
    }

    /// `0x005b55c0`: `CUnit+0xc8` as a dword (0 when no research is active).
    pub fn pending_tech_raw(&self) -> u32 {
        self.pending_tech().map(u32::from).unwrap_or(0)
    }

    /// `0x005b4b90`: byte sum of the active upgrade id (`CUnit+0xc9`) and
    /// research id (`CUnit+0xc8`), used by the status-panel icon lookup.
    pub fn pending_research_sum(&self) -> u8 {
        self.pending_upgrade()
            .unwrap_or(0)
            .wrapping_add(self.pending_tech().unwrap_or(0))
    }

    /// `0x005b4970`: number of queued production orders (`CUnit+0xc4`).
    pub fn queued_count(&self) -> u16 {
        self.production.len().min(u16::MAX as usize) as u16
    }

    /// `0x005b6540`: `CUnit+0x8f` kill-rank counter.
    pub fn rank_kills(&self) -> u8 {
        self.rank_kills
    }

    /// `0x005d4d60`: clear the production queue (`CUnit+0xb0..+0xbc`,
    /// `CUnit+0x132`).
    pub fn clear_production_queue(&mut self) {
        self.production.clear();
    }

    /// `0x005b4ce0`: current queue unit for the Zerg queue rows (0 when
    /// the queue is empty), or the unit's own type for every other row.
    pub fn current_queue_or_self(&self) -> Option<u16> {
        if is_zerg_queue_building(self.type_id) {
            match self.production.front() {
                Some(Order::Train { unit_type }) => Some(*unit_type),
                _ => None,
            }
        } else {
            Some(self.type_id)
        }
    }

    /// `0x005b4c60`: remaining research/upgrade time (`CUnit+0xc6`).
    /// The Rust order stores that timer in `build_remaining`.
    pub fn research_remaining(&self) -> u16 {
        match self.order {
            Order::Research { .. } | Order::Upgrade { .. } => {
                self.build_remaining.clamp(0, u16::MAX as i32) as u16
            }
            _ => 0,
        }
    }

    /// `0x005b6810`: lower the death timer to the incoming value.
    pub fn set_death_timer_min(&mut self, ticks: u16) {
        if self.death_timer == 0 || ticks < self.death_timer {
            self.death_timer = ticks;
        }
    }

    /// `0x005b4cc0`: `CUnit+0x110` death timer.
    pub fn death_timer(&self) -> u16 {
        self.death_timer
    }

    /// `0x005b4c20`: Psionic Storm one-update hit guard (`CUnit+0x11b`).
    pub fn storm_guard(&self) -> u8 {
        self.status.psi_storm_guard
    }

    /// `0x005b67d0`: write the Psionic Storm hit guard (`CUnit+0x11b`).
    pub fn set_storm_guard(&mut self, value: u8) {
        self.status.psi_storm_guard = value;
    }

    /// `0x005b3b10`: clear the linked hatchery/parent (`CUnit+0x80 = 0`).
    pub fn clear_connected_unit(&mut self) {
        self.connected_unit = None;
    }

    /// `0x005b3af0`: clear the active research id (`CUnit+0xc8 = 0`).
    /// The Rust order holds the same state, so an active research order is
    /// dropped; other orders are untouched.
    pub fn clear_pending_tech(&mut self) {
        if matches!(self.order, Order::Research { .. }) {
            self.order = Order::Idle;
        }
    }

    /// `0x005b6160`: true when an upgrade is active (`CUnit+0xc8 != 0x2c`).
    pub fn has_pending_upgrade(&self) -> bool {
        self.pending_upgrade().is_some()
    }

    /// `0x005b6470`: true when a technology is active (`CUnit+0xc9 != 0x3d`).
    pub fn has_pending_tech(&self) -> bool {
        self.pending_tech().is_some()
    }

    /// `0x005b6630`: CUnit+0x124 getter (Maelstrom timer).
    pub fn maelstrom_ticks(&self) -> i32 {
        self.status.maelstrom_ticks
    }

    /// `0x005b6a30`: CUnit+0x7c getter (has a target).
    pub fn has_target(&self) -> bool {
        self.target.is_some()
    }

    /// `0x005b6550`: CUnit+0x117 getter (lockdown timer).
    pub fn lockdown_ticks(&self) -> i32 {
        self.status.lockdown_ticks
    }

    /// `0x005b67c0`: CUnit+0x80 setter (hatchery/parent link).
    pub fn set_connected_unit(&mut self, id: Option<u32>) {
        self.connected_unit = id;
    }

    /// `0x005b4b80`: CUnit+0xc8 getter (active research tech).
    pub fn pending_tech(&self) -> Option<u8> {
        match &self.order {
            Order::Research { tech_id } => Some(*tech_id),
            _ => None,
        }
    }

    /// `0x005b5380`: CUnit+0xc9 getter (active upgrade).
    pub fn pending_upgrade(&self) -> Option<u8> {
        match &self.order {
            Order::Upgrade { upgrade_id, .. } => Some(*upgrade_id),
            _ => None,
        }
    }

    /// `0x005b4bd0`: CUnit+0x80 getter (the linked hatchery/parent).
    pub fn connected_unit_id(&self) -> Option<u32> {
        self.connected_unit
    }

    /// `0x005b6620`: CUnit+0xd4 getter (Nuclear Silo armed).
    pub fn nuke_ready_flag(&self) -> bool {
        self.nuke_ready
    }

    /// `0x005b5910`: bits 8..9 of `CUnit+0xdc` are the cloak state
    /// (0 = none, 2 = fully cloaked).  Rust cloaks atomically, so there is
    /// no intermediate 1.
    pub fn cloak_state(&self) -> u8 {
        if self.status.cloaked { 2 } else { 0 }
    }

    /// `0x005b3a00`: true when the unit is neither hidden (transport/mine
    /// hidden flag 0x1000) nor burrowed (0x2000).
    pub fn is_exposed(&self) -> bool {
        self.in_transport.is_none() && !self.status.burrowed
    }

    pub fn is_resource(&self) -> bool {
        is_resource_type(self.type_id)
    }

    /// Energy in retail 1/256 points.
    pub fn energy_fixed(&self) -> i32 {
        self.energy * ENERGY_SCALE + self.energy_frac
    }

    /// Write energy from a 1/256-point value, clamping at both ends.
    pub fn set_energy_fixed(&mut self, value: i32) {
        let v = value.clamp(0, self.energy_cap.max(0));
        self.energy = v / ENERGY_SCALE;
        self.energy_frac = v % ENERGY_SCALE;
    }

    pub fn add_energy_fixed(&mut self, delta: i32) {
        self.set_energy_fixed(self.energy_fixed() + delta);
    }

    pub fn add_energy_points(&mut self, points: i32) {
        self.add_energy_fixed(points.saturating_mul(ENERGY_SCALE));
    }

    /// `0x005b6760`: set energy from whole points, clamped to the type's
    /// maximum (`FUN_005cc800`) and stored as 1/256 points.
    pub fn set_energy_points(&mut self, points: i32) {
        let max_points = self.energy_cap.max(0) / ENERGY_SCALE;
        self.set_energy_fixed(points.min(max_points) * ENERGY_SCALE);
    }

    /// `0x005cc8e0`: true when the unit's fixed-point energy
    /// (`CUnit+0xa2`) is below the requested cost.  Retail short-circuits to
    /// false while cheat flag 0x800 is set; callers here gate that with
    /// [`Cheats::infinite_energy`].
    pub fn energy_below(&self, cost: i32) -> bool {
        self.energy_fixed() < cost
    }

    /// Spend 1/256-point energy.  Returns false (and changes nothing) when
    /// the pool cannot pay the whole cost.
    pub fn spend_energy_fixed(&mut self, cost: i32) -> bool {
        let current = self.energy_fixed();
        if self.energy_below(cost) {
            return false;
        }
        self.set_energy_fixed(current - cost);
        true
    }
}

/// Single-player cheat toggles (one set per player; the client applies them
/// to the local player only).  Retail's codes are handled in
/// [`World::apply_cheat`].
/// Ids of the single-player cheat codes in [`World::apply_cheat`].  They are
/// recorded in the command log so saves and replays keep their effects.
pub mod cheat {
    pub const SHOW_ME_THE_MONEY: u8 = 0;
    pub const WHATS_MINE_IS_MINE: u8 = 1;
    pub const BREATHE_DEEP: u8 = 2;
    pub const POWER_OVERWHELMING: u8 = 3;
    pub const THE_GATHERING: u8 = 4;
    pub const FOOD_FOR_THOUGHT: u8 = 5;
    pub const BLACK_SHEEP_WALL: u8 = 6;
    pub const WAR_AINT_WHAT_IT_USED_TO_BE: u8 = 7;
    pub const OPERATION_CWAL: u8 = 8;
    pub const SOMETHING_FOR_NOTHING: u8 = 9;
    pub const MEDIEVAL_MAN: u8 = 10;
    pub const MODIFY_THE_PHASE_VARIANCE: u8 = 11;
    pub const THERE_IS_NO_COW_LEVEL: u8 = 12;
    pub const GAME_OVER_MAN: u8 = 13;
    pub const STAYING_ALIVE: u8 = 14;
    pub const NOGLUES: u8 = 15;
}

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Cheats {
    pub invincible: bool,
    pub fast_build: bool,
    pub infinite_energy: bool,
    pub no_supply: bool,
    pub reveal: bool,
    pub free_tech: bool,
    pub free_upgrades: bool,
    pub build_anything: bool,
    pub stay_alive: bool,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub race: Race,
    pub team: u8,
    /// This side shares its victory with same-team players.  Lobby team
    /// games and action 57 status 2 set it; a plain ally (status 1) does not.
    pub allied_victory: bool,
    pub minerals: i32,
    pub gas: i32,
    pub techs: [bool; 44],
    pub upgrades: [u8; 61],
    pub defeated: bool,
    pub is_computer: bool,
    /// `OWNR` 3 "Rescue Passive": the player's units are nobody's enemy
    /// and change hands when another player's unit reaches them.
    pub rescuable: bool,
    /// Resources ever gathered, used by the "accumulate" trigger condition.
    pub minerals_gathered: i32,
    pub gas_gathered: i32,
    /// Lifetime statistics used by trigger conditions and the score screen.
    pub kills: u32,
    pub deaths: u32,
    pub razings: u32,
    /// Per-unit-type death counters (condition type 8).
    pub death_counts: [u32; 228],
    /// Per-unit-type kill counters (condition types 5/8/18).
    pub kill_counts: [u32; 228],
    /// Trigger score categories, indexed by the CHK score type: total,
    /// units, buildings, units+buildings, kills, razings, kills+razings,
    /// custom.  Units/buildings accumulate the `units.dat` build score of
    /// what the player produces, kills/razings the destroy score of what
    /// it kills (see `credit_scores` / `kill_unit`).
    pub scores: [i32; 8],
    /// Score-screen tallies: units and structures lost, resources spent.
    pub units_lost: u32,
    pub buildings_lost: u32,
    pub minerals_spent: i32,
    pub gas_spent: i32,
    /// Resources the player started with; the score screen's Resources
    /// column counts them as gathered (BWAPI `gatheredMinerals`).
    pub start_minerals: i32,
    pub start_gas: i32,
    /// Players whose vision this player receives (bit per player; its own
    /// bit is always implied).  Team games share vision among allies.
    pub vision_from: u16,
    /// Units and structures completed (starting ones included), the score
    /// screen's Produced / Constructed counts; credited with the score.
    pub units_produced: u32,
    pub structures_built: u32,
    /// Lobby colour (`multiplayer::COLOR_CHOICES` index); `None` keeps the
    /// classic slot colour (`render::PLAYER_COLORS[player]`).  Drawn by
    /// [`crate::render::player_rgb`].
    pub color: Option<u8>,
    /// Single-player cheat toggles (unused by computer players).
    pub cheats: Cheats,
}

impl Player {
    pub fn new(race: Race, team: u8, minerals: i32, gas: i32) -> Self {
        Self {
            race,
            team,
            allied_victory: false,
            minerals,
            gas,
            techs: [false; 44],
            upgrades: [0; 61],
            defeated: false,
            is_computer: false,
            rescuable: false,
            minerals_gathered: 0,
            gas_gathered: 0,
            kills: 0,
            deaths: 0,
            razings: 0,
            death_counts: [0; 228],
            kill_counts: [0; 228],
            scores: [0; 8],
            units_lost: 0,
            buildings_lost: 0,
            minerals_spent: 0,
            gas_spent: 0,
            start_minerals: minerals,
            start_gas: gas,
            vision_from: 0,
            units_produced: 0,
            structures_built: 0,
            color: None,
            cheats: Cheats::default(),
        }
    }

    /// Pay minerals (counted as spent for the score screen).
    pub fn pay_minerals(&mut self, amount: i32) {
        self.minerals -= amount;
        self.minerals_spent += amount;
    }

    /// Pay gas (counted as spent for the score screen).
    pub fn pay_gas(&mut self, amount: i32) {
        self.gas -= amount;
        self.gas_spent += amount;
    }

    pub fn has_tech(&self, id: u8) -> bool {
        self.cheats.free_tech || self.techs.get(id as usize).copied().unwrap_or(false)
    }

    pub fn upgrade_level(&self, id: u8) -> u8 {
        self.upgrades.get(id as usize).copied().unwrap_or(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimError {
    BadUnit,
    BadOwner,
    NotEnoughMinerals,
    NotEnoughGas,
    NotEnoughSupply,
    MissingRequirements,
    MissingTech,
    ProducerBusy,
    QueueFull,
    InvalidTarget,
    OutOfRange,
    NotEnoughEnergy,
    /// A Protoss structure is outside every Pylon power field.
    Unpowered,
    Unsupported,
}

impl std::fmt::Display for SimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for SimError {}

/// One player's flag in Capture the Flag.  `pos` is world pixels; the flag
/// follows its worker while `carrier` is set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CtfFlag {
    pub owner: u8,
    pub home: (i32, i32),
    pub pos: (i32, i32),
    pub carrier: Option<u32>,
    /// Tick at which a dropped flag returns home (None while carried/home).
    pub return_at: Option<u64>,
    pub released: bool,
}

#[derive(Clone, Debug, Default)]
pub struct CtfState {
    pub flags: Vec<CtfFlag>,
    pub beacons: [Option<u32>; 8],
    /// Tick at which the flags appear (`start + 3 game minutes`).
    pub release_at: u64,
    /// A beacon may be moved once until this tick (`start + 10 minutes`).
    pub move_until: u64,
    pub beacon_moved: [bool; 8],
}

pub struct World {
    pub data: Arc<GameData>,
    pub terrain: Terrain,
    /// Connected-component map of the walk grid, built once per terrain.
    pub regions: RegionMap,
    /// Retail campaign-AI region states, one byte per `regions` id and
    /// player: 0 unused, 1 continue attack, 2 attacking, 3 enemy spotted,
    /// 4 defend, 5 town, 6 actively defending, 7 chase, 8 preparing attack,
    /// 9 attack unreachable.
    pub ai_region_states: Vec<Vec<u8>>,
    pub players: Vec<Player>,
    pub units: Vec<SimUnit>,
    pub next_id: u32,
    pub tick: u64,
    pub winner: Option<u8>,
    /// Sudden Death: destroying any town hall defeats its owner.
    pub sudden_death: bool,
    /// Greed: first player to gather this many minerals + gas wins.
    pub greed_goal: Option<i32>,
    /// Slaughter: at this tick the player with the most kills wins.
    pub slaughter_deadline: Option<u64>,
    /// `noglues`: computer players cannot cast spells.
    pub cheat_no_enemy_spells: bool,
    /// Capture the Flag state (`None` for every other game type).
    pub ctf: Option<CtfState>,
    /// Per-map `UNIS`/`UPGS`/`TECS`/`PUNI` overrides.
    pub rules: crate::map::ScenarioRules,
    /// Victory and defeat come only from the scenario's triggers (a mission
    /// or use-map scenario with placed forces), never from the melee
    /// "last player standing" rule, which would stop the world before the
    /// map's closing transmissions and `Victory` action run.
    pub trigger_victory: bool,
    /// Rescues so far, in order: (unit id, new owner).  Append-only so the
    /// client can follow it like the trigger cues (rescue sound).
    pub rescues: Vec<(u32, u8)>,
    /// Defensive Matrix hit/break cues produced during the current tick.
    pub matrix_cues: Vec<MatrixCue>,
    /// The last [`SPELL_CUE_KEEP`] spell cues (Stim, Lockdown hit,
    /// Restoration, Optical Flare) for the renderer's effects and sounds;
    /// `spell_cue_seq` counts every cue ever pushed, so a consumer knows
    /// which ones it has not seen yet.  Presentation only.
    pub spell_cues: std::collections::VecDeque<SpellCue>,
    pub spell_cue_seq: u64,
    /// Hits of instant weapons waiting for their bullet image to reach
    /// `domissiledmg` (see [`World::instant_hit`]).
    pub pending_hits: Vec<PendingHit>,
    /// Staged nuclear launches.
    pub nuke_strikes: Vec<NukeStrike>,
    /// `(x, y)` warning cues produced during the current tick.
    pub nuke_warnings: Vec<(i32, i32)>,
    /// Deterministic LCG state for effects that use randomness.
    pub rng: u64,
    /// Scenario trigger VM, when the map has a `TRIG` section.
    pub triggers: Option<crate::trigger::TriggerSystem>,
    /// Travelling bullets and missiles.
    pub projectiles: Vec<Projectile>,
    /// Per-player visible/explored masks.  Present when a scenario has an
    /// extent; `fog_of_war` controls whether targeting consults it.
    pub vision: Option<Vision>,
    pub fog_of_war: bool,
    /// Zerg creep, one flag per 32-pixel tile.  Rebuilt when a creep source
    /// completes or dies.
    creep: Vec<bool>,
    /// Creep maturity per tile (0..=3).  Sources seed their footprint at 3;
    /// creep spreads one step every `CREEP_GROW_INTERVAL` ticks and decays
    /// when no source supports the tile.
    creep_strength: Vec<u8>,
    /// Tiles covered by a creep source's full radius.
    creep_supported: Vec<bool>,
    creep_dirty: bool,
    next_projectile_id: u32,
    /// Scratch obstacle list rebuilt when a path is requested.
    obstacles: Obstacles,
    index: HashMap<u32, usize>,
    /// How many times each recovered order handler routine was entered.
    /// The 137 fixed targets are ordered by VA in `crate::handlers`.
    pub handler_hits: [u64; crate::handlers::HANDLER_COUNT],
    /// Optional command recorder (save games / replays / LAN). Pushed by
    /// [`World::issue`] only; zero cost when `None`.
    pub recorder: Option<Recorder>,
}

impl World {
    /// `0x005b66c0`: clear status bit 26 unless the type is below 0xe4 and
    /// carries `units.dat` flag 0x20000000.
    pub fn clear_unsupported_flag_26(&mut self, i: usize) {
        let ty = self.data.unit(self.units[i].type_id);
        if self.units[i].type_id >= 0xe4 || ty.flags & 0x2000_0000 == 0 {
            self.units[i].flag_26 = false;
        }
    }
}

fn direction_vector(direction: i32) -> (i32, i32) {
    // The engine's 256-entry direction table is a cosine/sine pair scaled by
    // 256, measured clockwise from north (0 = up, 64 = right).  Four
    // cardinal directions are exact; intermediate angles are rounded the
    // same way the recovered helper does.
    let angle = (direction as f32) * std::f32::consts::TAU / 256.0;
    (
        (angle.sin() * 256.0).round() as i32,
        (-angle.cos() * 256.0).round() as i32,
    )
}

/// Hovering units (`units.dat` movement flags 0xC1): they do not set off
/// Spider Mines.  Vulture, SCV, Spider Mine, Raynor's Vulture (19), Drone,
/// Dark Archon (63), Probe, Archon, Tassadar/Zeratul (76).
pub fn is_hover_type(type_id: u16) -> bool {
    matches!(
        type_id,
        uid::VULTURE | uid::SCV | uid::SPIDER_MINE | 19 | uid::DRONE | 63 | uid::PROBE | 68 | 76
    )
}

/// Marine and its hero (20 = Jim Raynor).
pub fn is_marine_type(type_id: u16) -> bool {
    matches!(type_id, uid::MARINE | 20)
}

/// Firebat and its hero (10 = Gui Montag).
pub fn is_firebat_type(type_id: u16) -> bool {
    matches!(type_id, uid::FIREBAT | 10)
}

/// Ghost and its heroes (16 Sarah Kerrigan, 99 Samir Duran, 100 Alexei
/// Stukov, 104 Infested Duran).
pub fn is_ghost_type(type_id: u16) -> bool {
    matches!(type_id, uid::GHOST | 16 | 99 | 100 | 104)
}

fn caster_can_cast(type_id: u16, spell: Spell) -> bool {
    match spell {
        Spell::StimPacks => is_marine_type(type_id) || is_firebat_type(type_id),
        Spell::Lockdown => is_ghost_type(type_id),
        Spell::EmpShockwave | Spell::DefensiveMatrix | Spell::Irradiate => {
            type_id == uid::SCIENCE_VESSEL
        }
        // 19 = Jim Raynor's Vulture (Wasteland, Backwater Station).
        Spell::SpiderMines => matches!(type_id, uid::VULTURE | 19),
        Spell::ScannerSweep => type_id == uid::COMSAT_STATION,
        Spell::TankSiegeMode => matches!(type_id, uid::SIEGE_TANK | uid::SIEGE_MODE),
        Spell::YamatoGun => type_id == uid::BATTLECRUISER,
        Spell::CloakingField => type_id == uid::WRAITH,
        Spell::PersonnelCloaking => is_ghost_type(type_id),
        Spell::Burrowing => matches!(
            type_id,
            uid::DRONE
                | uid::ZERGLING
                | uid::HYDRALISK
                | uid::LURKER
                | uid::DEFILER
                | uid::INFESTED_TERRAN
        ),
        Spell::SpawnBroodlings | Spell::Ensnare | Spell::Parasite | Spell::Infestation => {
            type_id == uid::QUEEN
        }
        Spell::DarkSwarm | Spell::Plague | Spell::Consume => type_id == uid::DEFILER,
        Spell::PsionicStorm | Spell::Hallucination | Spell::ArchonWarp => {
            type_id == uid::HIGH_TEMPLAR
        }
        Spell::Recall | Spell::StasisField => type_id == uid::ARBITER,
        Spell::Restoration | Spell::OpticalFlare | Spell::Healing => type_id == uid::MEDIC,
        Spell::DisruptionWeb => type_id == uid::CORSAIR,
        Spell::MindControl | Spell::Feedback | Spell::Maelstrom => type_id == uid::DARK_ARCHON,
        Spell::DarkArchonMeld => type_id == uid::DARK_TEMPLAR,
        Spell::LurkerAspect => type_id == uid::HYDRALISK,
    }
}

fn default_hangar_capacity(type_id: u16) -> u8 {
    match type_id {
        uid::CARRIER => CARRIER_HANGAR,
        uid::REAVER => REAVER_HANGAR,
        // The Vulture's three Spider Mines reuse the hangar counter (19 =
        // Jim Raynor's Vulture).
        uid::VULTURE | 19 => 3,
        _ => 0,
    }
}

fn default_hangar(type_id: u16) -> u8 {
    // 19 = Jim Raynor's Vulture.
    if matches!(type_id, uid::VULTURE | 19) {
        3
    } else {
        0
    }
}

fn weapon_can_target(weapon: &WeaponType, target: &UnitType) -> bool {
    weapon_can_target_unit(weapon, target, target.is_flyer())
}

/// `FUN_0059ca70` with the runtime airborne state, so a lifted Terran
/// building counts as air even though its unit type is a building.
fn weapon_can_target_unit(weapon: &WeaponType, target: &UnitType, air: bool) -> bool {
    if air {
        if !weapon.can_target_air() {
            return false;
        }
    } else if !weapon.can_target_ground() {
        return false;
    }
    if weapon.requires_mechanical() && !target.is_mechanical() {
        return false;
    }
    if weapon.requires_non_robotic() && target.is_robotic() {
        return false;
    }
    if weapon.requires_organic() && !target.is_organic() {
        return false;
    }
    if !weapon.can_target_buildings() && target.is_building() {
        return false;
    }
    if weapon.requires_organic_or_mechanical() && !target.is_organic() && !target.is_mechanical() {
        return false;
    }
    true
}

pub fn is_resource_type(type_id: u16) -> bool {
    // Translation batch 02 (`0x005ada00`): the four resource node rows.
    a02_is_resource_node_type(type_id)
}

/// Map-only door units (`FUN_005b5b00`, types `0xcd..0xd0`: left/right upper
/// level doors and left/right pit doors).  They are not normal targets and
/// must not participate in ground-unit separation.
pub fn is_door_type(type_id: u16) -> bool {
    matches!(type_id, 0xcd..=0xd0)
}

/// `0x005b5c40`: floor trap types (`0xcb`, `0xd1`).
pub fn is_floor_trap_type(type_id: u16) -> bool {
    matches!(type_id, 0xcb | 0xd1)
}

/// `0x005b5d50`: explosive/trigger trap types (`0xcb`, `0xd1..=0xd5`),
/// excluding the `0xcc` doodad and the four doors.
pub fn is_trap_trigger_type(type_id: u16) -> bool {
    matches!(type_id, 0xcb | 0xd1..=0xd5)
}

/// Floor/wall traps (`0x5b56a0`).
pub fn is_trap_type(type_id: u16) -> bool {
    is_trap_trigger_type(type_id) || type_id == 0xcc
}

/// `0x005b5ae0`: `(type - 0xcb) < 0xb`, i.e. doors and floor/wall traps.
pub fn is_doodad_type(type_id: u16) -> bool {
    is_door_type(type_id) || is_trap_type(type_id)
}

/// `0x005b5e10`: unit type 0x9c (Pylon).
pub fn is_pylon_type(type_id: u16) -> bool {
    type_id == uid::PYLON
}

/// `0x005b5870`: Carrier/Gantrithor rows (0x48/0x52).
pub fn is_carrier_type(type_id: u16) -> bool {
    matches!(type_id, 0x48 | 0x52)
}

/// `0x005b5a10`: Defiler/Unclean One rows (0x2e/0x34).
pub fn is_defiler_family_type(type_id: u16) -> bool {
    matches!(type_id, 0x2e | 0x34)
}

/// `0x005b5f60`: Zerg Nydus Canal row (0x86).
pub fn is_nydus_canal_type(type_id: u16) -> bool {
    type_id == uid::NYDUS_CANAL
}

/// `0x005b6220`: Map Revealer row (0x65).
pub fn is_map_revealer_type(type_id: u16) -> bool {
    type_id == 0x65
}

/// `0x005b6270`: Scanner Sweep row (0x21).
pub fn is_scanner_sweep_type(type_id: u16) -> bool {
    type_id == 0x21
}

/// `0x005b6290`: Nuclear Silo row (0x6c).
pub fn is_nuclear_silo_type(type_id: u16) -> bool {
    type_id == 0x6c
}

/// `0x005b3a80`: Infested Terran row (0x32).
pub fn is_infested_terran_type(type_id: u16) -> bool {
    type_id == 0x32
}

/// `0x005b6250`: suicide units (Scourge 0x2f and Infested Terran 0x32).
pub fn is_suicide_unit_type(type_id: u16) -> bool {
    matches!(type_id, 0x2f | 0x32)
}

/// `0x005b6310`: beacon rows 0xc2..=0xc7 plus the unused Floor Hatch 0xcc.
pub fn is_beacon_type(type_id: u16) -> bool {
    (0xc2..=0xc7).contains(&type_id) || type_id == 0xcc
}

/// `0x005b63d0`: Ultralisk rows (0x27 and hero Torrasque 0x30).
pub fn is_ultralisk_type(type_id: u16) -> bool {
    matches!(type_id, 0x27 | 0x30)
}

/// `0x005b64a0`: Vulture rows (2 and hero Jim Raynor 0x13).
pub fn is_vulture_type(type_id: u16) -> bool {
    matches!(type_id, 0x02 | 0x13)
}

/// `0x005b6500`: Zealot rows (0x41 and hero Fenix 0x4d).
pub fn is_zealot_type(type_id: u16) -> bool {
    matches!(type_id, 0x41 | 0x4d)
}

/// `0x005b6520`: Zergling rows (0x25 and hero Devouring One 0x36).
pub fn is_zergling_type(type_id: u16) -> bool {
    matches!(type_id, 0x25 | 0x36)
}

/// `0x005dbe40`: Zerg rows whose `CUnit+0x98` queue is exposed by
/// `0x005b4ce0` (Lair, Hive, Greater Spire, Spore Colony, Sunken Colony).
pub fn is_zerg_queue_building(type_id: u16) -> bool {
    matches!(type_id, 0x84 | 0x85 | 0x89 | 0x90 | 0x92)
}

/// `0x005b59c0`: wildlife/critter rows 0x59, 0x5a and 0x5d..=0x60.
pub fn is_critter_type(type_id: u16) -> bool {
    matches!(type_id, 0x59 | 0x5a | 0x5d | 0x5e | 0x5f | 0x60)
}

/// `0x005b5e20`: true for Broodling row 0x28 (and for dying units, whose
/// `CUnit+0xdc` bit 30 this engine does not retain).
pub fn is_broodling_or_dying(type_id: u16) -> bool {
    type_id == 0x28
}

/// `0x005b6010`: Queen rows (normal 0x2d and hero Matriarch 0x31).
pub fn is_queen_type(type_id: u16) -> bool {
    matches!(type_id, 0x2d | 0x31)
}

/// `0x005b5c80`: Valkyrie row (0x3a).
pub fn is_valkyrie_type(type_id: u16) -> bool {
    type_id == 0x3a
}

/// `0x005b5e90`: Medic row (0x22).
pub fn is_medic_type(type_id: u16) -> bool {
    type_id == 0x22
}

/// `0x005b5890`: Carrier/Gantrithor (0x48/0x52) and Reaver/Warbringer
/// (0x53/0x51), the hangar-unit families.
pub fn is_carrier_reaver_family(type_id: u16) -> bool {
    matches!(type_id, 0x48 | 0x52 | 0x53 | 0x51)
}

/// `0x005b5c90`: Ghost-family rows (Ghost, Kerrigan, Duran, Stukov,
/// Infested Duran).
pub fn is_ghost_family_type(type_id: u16) -> bool {
    matches!(type_id, 1 | 16 | 99 | 100 | 104)
}

/// `0x005b5be0`: Interceptor (0x49) or Scarab (0x55).
pub fn is_hangar_unit_type(type_id: u16) -> bool {
    matches!(type_id, 0x49 | 0x55)
}

/// `0x005b5c00`: Firebat (0x20) or Gui Montag (0x0a).
pub fn is_firebat_family_type(type_id: u16) -> bool {
    matches!(type_id, 0x20 | 0x0a)
}

/// `0x005b58e0`: inverted `units.dat` building flag; ids above 0xe3 are
/// treated as non-buildings.
pub fn is_non_building_type(data: &GameData, type_id: u16) -> bool {
    type_id > 0xe3 || !data.unit(type_id).is_building()
}

/// `0x005b5cc0`: Goliath turret rows (`0x04`, `0x12`).
pub fn is_goliath_turret_type(type_id: u16) -> bool {
    matches!(type_id, uid::GOLIATH_TURRET | 0x12)
}

/// `0x005b57c0`: the Arbiter row and its hero variant (`0x47`, `0x56`).
pub fn is_arbiter_type(type_id: u16) -> bool {
    matches!(type_id, uid::ARBITER | 0x56)
}

/// `0x005b6030`: town halls and unit-producing structures that can own a
/// rally point (Command Center, Barracks, Factory, Starport, Hatchery/
/// Lair/Hive, Nexus, Robotics Facility, Gateway, Stargate).
pub fn is_production_building(type_id: u16) -> bool {
    matches!(
        type_id,
        uid::COMMAND_CENTER
            | uid::BARRACKS
            | uid::FACTORY
            | uid::STARPORT
            | uid::INFESTED_COMMAND_CENTER
            | uid::HATCHERY
            | uid::LAIR
            | uid::HIVE
            | uid::NEXUS
            | uid::ROBOTICS_FACILITY
            | uid::GATEWAY
            | uid::STARGATE
    )
}

/// Refinery/Assimilator/Extractor: the gas building that replaces a geyser.
/// `0x005a4610`: rotate an 8-bit direction value right by 6
/// (`v >> 6 | v << 2`).
pub fn rotate_byte_6(value: u8) -> u8 {
    value.rotate_left(2)
}

/// `0x005b63a0`: Refinery 0x6e, Extractor 0x95 and Assimilator 0x9d.
pub fn is_gas_building(type_id: u16) -> bool {
    matches!(type_id, uid::REFINERY | uid::ASSIMILATOR | uid::EXTRACTOR)
}

fn is_gas_resource(type_id: u16) -> bool {
    type_id == uid::VESPENE_GEYSER || is_gas_building(type_id)
}

/// Terran buildings occupy 106..=125 (Command Center through Bunker).
fn is_terran_building(type_id: u16) -> bool {
    (106..=125).contains(&type_id)
}

/// Zerg buildings are the contiguous block 130..=150 in `units.dat`
/// (Infested Command Center through Extractor).
fn is_zerg_building(type_id: u16) -> bool {
    (130..=150).contains(&type_id)
}

/// Protoss buildings occupy 154..=172 (Nexus through Shield Battery).
fn is_protoss_building(type_id: u16) -> bool {
    (154..=172).contains(&type_id)
}

/// Upgraded Zerg buildings satisfy their base building's requirements
/// (Lair/Hive count as Hatchery, Greater Spire as Spire, colonies as Creep
/// Colony).
fn requirement_equivalent(required: u16, actual: u16) -> bool {
    actual == required
        || matches!(
            (required, actual),
            (uid::HATCHERY, uid::LAIR)
                | (uid::HATCHERY, uid::HIVE)
                | (uid::LAIR, uid::HIVE)
                | (uid::SPIRE, uid::GREATER_SPIRE)
                | (uid::CREEP_COLONY, uid::SUNKEN_COLONY)
                | (uid::CREEP_COLONY, uid::SPORE_COLONY)
        )
}

/// Source building for each Zerg building morph.
/// The Pylon psi field: 16 x 10 tiles around the Pylon, rounded at the
/// corners.  INFERENCE: this is the field table BWAPI documents for the
/// retail game (`bPsiFieldMask`), indexed by the tile offset of the point
/// from the Pylon (C-style truncating division), not measured here.
pub const PSI_FIELD: [[bool; 16]; 10] = {
    const O: bool = false;
    const X: bool = true;
    [
        [O, O, O, O, O, X, X, X, X, X, X, O, O, O, O, O],
        [O, O, X, X, X, X, X, X, X, X, X, X, X, X, O, O],
        [O, X, X, X, X, X, X, X, X, X, X, X, X, X, X, O],
        [X, X, X, X, X, X, X, X, X, X, X, X, X, X, X, X],
        [X, X, X, X, X, X, X, X, X, X, X, X, X, X, X, X],
        [X, X, X, X, X, X, X, X, X, X, X, X, X, X, X, X],
        [X, X, X, X, X, X, X, X, X, X, X, X, X, X, X, X],
        [O, X, X, X, X, X, X, X, X, X, X, X, X, X, X, O],
        [O, O, X, X, X, X, X, X, X, X, X, X, X, X, O, O],
        [O, O, O, O, O, X, X, X, X, X, X, O, O, O, O, O],
    ]
};

/// Whether a point `(dx, dy)` px from a Pylon's centre is in its field.
pub fn psi_field_covers(dx: i32, dy: i32) -> bool {
    let (col, row) = (dx / 32 + 8, dy / 32 + 5);
    (0..16).contains(&col) && (0..10).contains(&row) && PSI_FIELD[row as usize][col as usize]
}

pub fn building_morph_source(target: u16) -> Option<u16> {
    Some(match target {
        uid::LAIR => uid::HATCHERY,
        uid::HIVE => uid::LAIR,
        uid::GREATER_SPIRE => uid::SPIRE,
        uid::SUNKEN_COLONY | uid::SPORE_COLONY => uid::CREEP_COLONY,
        _ => return None,
    })
}

/// One miss roll: `chance` is in 1/256 units, so `roll < chance` misses
/// exactly `chance` times out of 256.  `0` never misses.
fn dodge_roll(roll: u32, chance: u32) -> bool {
    chance > 0 && (roll & 0xff) < chance
}

/// Shipped `orders.dat` rows that are direct ability casts.
fn row_spell(order_id: u8) -> Option<Spell> {
    use crate::ids::order;
    Some(match order_id {
        order::FIRE_YAMATO_GUN => Spell::YamatoGun,
        order::CAST_LOCKDOWN => Spell::Lockdown,
        order::CAST_DARK_SWARM => Spell::DarkSwarm,
        order::CAST_PARASITE => Spell::Parasite,
        order::CAST_SPAWN_BROODLINGS => Spell::SpawnBroodlings,
        order::CAST_EMP_SHOCKWAVE => Spell::EmpShockwave,
        order::CAST_RECALL => Spell::Recall,
        order::CAST_SCANNER_SWEEP => Spell::ScannerSweep,
        order::CAST_DEFENSIVE_MATRIX => Spell::DefensiveMatrix,
        order::CAST_PSIONIC_STORM => Spell::PsionicStorm,
        order::CAST_IRRADIATE => Spell::Irradiate,
        order::CAST_PLAGUE => Spell::Plague,
        order::CAST_CONSUME => Spell::Consume,
        order::CAST_ENSNARE => Spell::Ensnare,
        order::CAST_STASIS_FIELD => Spell::StasisField,
        order::CAST_HALLUCINATION => Spell::Hallucination,
        order::MEDIC_HEAL => Spell::Healing,
        order::CAST_RESTORATION => Spell::Restoration,
        order::CAST_DISRUPTION_WEB => Spell::DisruptionWeb,
        order::CAST_MIND_CONTROL => Spell::MindControl,
        order::DARK_ARCHON_MELD => Spell::DarkArchonMeld,
        order::CAST_FEEDBACK => Spell::Feedback,
        order::CAST_OPTICAL_FLARE => Spell::OpticalFlare,
        order::CAST_MAELSTROM => Spell::Maelstrom,
        _ => return None,
    })
}

/// `techdata.dat` energy cost, in whole points, for a spell.
pub fn spell_energy_cost(data: &GameData, spell: Spell) -> i32 {
    data.tech(spell.tech_id())
        .map(|t| t.energy_cost as i32)
        .unwrap_or(0)
}

/// Public row -> spell lookup for tests and command-card code.
pub fn spell_for_order_row(order_id: u8) -> Option<Spell> {
    row_spell(order_id)
}

/// Terran structures that can lift off in the retail game.
fn can_lift_building(type_id: u16) -> bool {
    matches!(
        type_id,
        uid::COMMAND_CENTER
            | uid::BARRACKS
            | uid::FACTORY
            | uid::STARPORT
            | uid::SCIENCE_FACILITY
            | uid::ENGINEERING_BAY
            | uid::INFESTED_COMMAND_CENTER
    )
}

/// Parent building for each Terran add-on.
fn addon_parent(addon: u16) -> Option<u16> {
    Some(match addon {
        uid::COMSAT_STATION | uid::NUCLEAR_SILO => uid::COMMAND_CENTER,
        uid::MACHINE_SHOP => uid::FACTORY,
        uid::CONTROL_TOWER => uid::STARPORT,
        uid::COVERT_OPS | uid::PHYSICS_LAB => uid::SCIENCE_FACILITY,
        _ => return None,
    })
}

/// The buildings that generate creep.
/// Creep reach of every creep source (px, retail `get_max_creep_bb`).
const CREEP_RX: i32 = 320;
const CREEP_RY: i32 = 200;

fn is_creep_source(type_id: u16) -> bool {
    matches!(
        type_id,
        uid::HATCHERY
            | uid::LAIR
            | uid::HIVE
            | uid::CREEP_COLONY
            | uid::SPORE_COLONY
            | uid::SUNKEN_COLONY
    )
}

/// Bunker occupancy is four slots for Terran units that take one transport
/// space: Marine, Ghost, Firebat, Medic, the SCV (it does not fire from
/// inside), the Civilian (15) and the infantry heroes.
fn is_bunker_infantry(type_id: u16) -> bool {
    matches!(
        type_id,
        uid::MARINE | uid::GHOST | uid::FIREBAT | uid::MEDIC | uid::SCV | 15
    ) || is_marine_type(type_id)
        || is_firebat_type(type_id)
        || is_ghost_type(type_id)
}

/// `0x005cc6d0`'s unit switch: the upgrade that raises this unit's
/// maximum energy.  `None` for every other unit (including Shield Battery,
/// which stays at 200).
pub fn energy_upgrade(type_id: u16) -> Option<u8> {
    Some(match type_id {
        uid::GHOST => upgrade::MOEBIUS_REACTOR,
        uid::WRAITH => upgrade::APOLLO_REACTOR,
        uid::SCIENCE_VESSEL => upgrade::TITAN_REACTOR,
        uid::BATTLECRUISER => upgrade::COLOSSUS_REACTOR,
        uid::MEDIC => upgrade::CADUCEUS_REACTOR,
        uid::QUEEN => upgrade::GAMETE_MEIOSIS,
        uid::DEFILER => upgrade::METASYNAPTIC_NODE,
        uid::CORSAIR => upgrade::ARGUS_JEWEL,
        uid::DARK_ARCHON => upgrade::ARGUS_TALISMAN,
        uid::HIGH_TEMPLAR => upgrade::KHAYDARIN_AMULET,
        uid::ARBITER => upgrade::KHAYDARIN_CORE,
        _ => return None,
    })
}

/// `0x005cc800`: maximum energy in whole points (`FUN_005cc6d0` >> 8).
pub fn unit_max_energy_points(data: &GameData, upgrades: &[u8; 61], type_id: u16) -> i32 {
    unit_max_energy_fixed(data, upgrades, type_id) >> 8
}

/// `0x005cc6d0`: maximum energy in 1/256 points.
pub fn unit_max_energy_fixed(data: &GameData, upgrades: &[u8; 61], type_id: u16) -> i32 {
    let upgraded = data.unit(type_id).is_hero()
        || energy_upgrade(type_id).is_some_and(|upgrade_id| upgrades[upgrade_id as usize] > 0);
    if upgraded {
        MAX_ENERGY_UPGRADED * ENERGY_SCALE
    } else {
        MAX_ENERGY * ENERGY_SCALE
    }
}

fn spell_range(spell: Spell) -> i32 {
    let tiles = match spell {
        Spell::PsionicStorm
        | Spell::EmpShockwave
        | Spell::Irradiate
        | Spell::StasisField
        | Spell::Maelstrom
        | Spell::Plague
        | Spell::Ensnare
        | Spell::DisruptionWeb
        | Spell::Recall
        | Spell::OpticalFlare
        | Spell::Restoration
        | Spell::Healing
        | Spell::Feedback
        | Spell::MindControl
        | Spell::SpawnBroodlings
        | Spell::Parasite
        | Spell::Consume
        | Spell::YamatoGun
        | Spell::Lockdown
        | Spell::Hallucination => 9,
        // OpenBW `order_CastDefensiveMatrix`: the Science Vessel casts it
        // from its sight range (10 tiles).
        Spell::DefensiveMatrix => 10,
        Spell::ScannerSweep | Spell::DarkSwarm => 12,
        _ => 1,
    };
    tiles * 32 * fp(1)
}

fn direction_byte(dx: i32, dy: i32) -> u8 {
    // Screen compass: 0 = north/up, 64 = east/right, 128 = south/down,
    // 192 = west/left.  `atan2(dx, -dy)` is the clockwise angle from up.
    let angle = (dx as f32).atan2(-(dy as f32));
    let mut a = (angle * 256.0 / std::f32::consts::TAU).round() as i32;
    a = (a + 256) % 256;
    a as u8
}

fn isqrt(n: i64) -> i64 {
    if n <= 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Pixel rectangle around a unit centre using the four directional
/// `units.dat` half-extents (`FUN_005b6a60/80/90/a0`).
/// `0x005b52f0`: build the unit's `units.dat` rectangle from its centre
/// (`x-left`, `y-top`, `x+right`, `y+bottom`).
/// The tiles a building occupies: its `units.dat` placement box around
/// `centre` (right / bottom exclusive).
fn placement_rect(ty: &UnitType, centre: (i32, i32)) -> Rect {
    let (w, h) = (ty.placement.x.max(1) as i32, ty.placement.y.max(1) as i32);
    let (l, t) = (centre.0 - w / 2, centre.1 - h / 2);
    Rect::new(l, t, l + w, t + h)
}

fn unit_box_rect(ty: &UnitType, centre: (i32, i32)) -> Rect {
    let (l, t, r, b) = ty.dimensions.rect_at(centre);
    Rect::new(l, t, r, b)
}

/// `0x005b6ad0`: full `units.dat` box width in pixels (`left + right + 1`).
#[allow(dead_code)]
fn unit_box_width(ty: &UnitType) -> i32 {
    ty.dimensions.left as i32 + ty.dimensions.right as i32 + 1
}

/// Larger half extent of a unit's `units.dat` box, in pixels.
fn half_extent_px(ty: &UnitType) -> i32 {
    let d = &ty.dimensions;
    (d.left.max(d.right).max(d.top).max(d.bottom)).max(0) as i32
}

#[cfg(test)]
mod tests;

mod translated;
pub use translated::*;

// `impl World` split by subsystem (scripts/split_impl.py).
mod animation;
mod combat;
mod creep_power;
mod damage;
mod movement;
mod nuke;
mod orders;
mod production;
mod projectiles;
mod spawn;
mod spells;
mod step;
mod transport;
mod unit_queries;
mod workers;
