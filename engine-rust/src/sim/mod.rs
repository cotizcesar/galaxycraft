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
    pub fn new(data: Arc<GameData>, terrain: Terrain, player_count: usize) -> Self {
        let regions = RegionMap::from_terrain(&terrain);
        let region_count = regions.count() as usize;
        let creep_tiles = terrain.width * terrain.height;
        let creep = vec![false; creep_tiles];
        let creep_strength = vec![0u8; creep_tiles];
        let creep_supported = vec![false; creep_tiles];
        Self {
            data,
            terrain,
            regions,
            ai_region_states: vec![vec![0; region_count]; player_count],
            players: (0..player_count)
                .map(|i| Player::new(Race::Terran, i as u8, 50, 0))
                .collect(),
            units: Vec::new(),
            next_id: 1,
            tick: 0,
            winner: None,
            sudden_death: false,
            greed_goal: None,
            slaughter_deadline: None,
            cheat_no_enemy_spells: false,
            ctf: None,
            rules: crate::map::ScenarioRules::default(),
            rescues: Vec::new(),
            matrix_cues: Vec::new(),
            spell_cues: std::collections::VecDeque::new(),
            spell_cue_seq: 0,
            pending_hits: Vec::new(),
            nuke_strikes: Vec::new(),
            nuke_warnings: Vec::new(),
            trigger_victory: false,
            triggers: None,
            projectiles: Vec::new(),
            vision: None,
            fog_of_war: false,
            creep,
            creep_strength,
            creep_supported,
            creep_dirty: true,
            next_projectile_id: 1,
            rng: 0x1234_5678_9abc_def0,
            obstacles: Obstacles::new(),
            index: HashMap::new(),
            handler_hits: [0; crate::handlers::HANDLER_COUNT],
            recorder: None,
        }
    }

    pub fn from_scenario(
        data: Arc<GameData>,
        scenario: &Scenario,
        tiles: &crate::map::TileData,
        races: &[Race],
    ) -> Self {
        Self::from_scenario_occupied(data, scenario, tiles, races, None)
    }

    /// [`Self::from_scenario`] for a lobby game: `occupied[p]` false (an
    /// Open or Closed slot) gets no melee starting force and is defeated
    /// from the first tick, as retail leaves empty start locations empty.
    pub fn from_scenario_occupied(
        data: Arc<GameData>,
        scenario: &Scenario,
        tiles: &crate::map::TileData,
        races: &[Race],
        occupied: Option<&[bool]>,
    ) -> Self {
        let terrain = Terrain::from_scenario(scenario, tiles);
        let mut world = Self::new(data, terrain, races.len());
        world.vision = Some(Vision::from_mask(
            scenario.width,
            scenario.height,
            &scenario.mask,
        ));
        world.rules = scenario.rules();
        for (i, race) in races.iter().enumerate() {
            world.players[i].race = *race;
        }
        let rules = world.rules.clone();
        for p in 0..world.players.len() {
            for tech in 0..44u8 {
                if rules
                    .tech_restrictions
                    .as_ref()
                    .is_some_and(|r| r.researched(p, tech))
                {
                    world.players[p].techs[tech as usize] = true;
                }
            }
            for upgrade in 0..61u8 {
                if let Some(restrictions) = &rules.upgrade_restrictions {
                    let level = restrictions.start_level(p, upgrade);
                    if level > 0 {
                        world.players[p].upgrades[upgrade as usize] = level;
                    }
                }
            }
        }
        for (i, p) in world.players.iter_mut().enumerate() {
            p.rescuable = scenario.owners.get(i) == Some(&OWNR_RESCUE_PASSIVE);
        }
        world.trigger_victory = scenario.has_placed_force() && !scenario.triggers.is_empty();
        let mut serial_to_index: HashMap<u32, usize> = HashMap::new();
        for mu in &scenario.units {
            // Start-location markers are metadata, not units.
            if mu.unit_id == uid::START_LOCATION {
                continue;
            }
            let resource = is_resource_type(mu.unit_id);
            let owner = mu.owner as usize;
            if !resource && owner >= world.players.len() {
                continue;
            }
            // Neutral resources are placed under player 0 for ownership
            // bookkeeping; every combat/economy path treats them as neutral.
            let owner = if resource { 0 } else { owner };
            let idx = world.spawn(owner, mu.unit_id, mu.x, mu.y);
            if let Ok(i) = idx {
                serial_to_index.insert(mu.serial, i);
                let ty = world.unit_stats(mu.unit_id);
                let u = &mut world.units[i];
                if mu.fields_set & 0x02 != 0 {
                    let pct = (mu.hit_point_percent as i32).min(100);
                    // 0% leaves the retail minimum of 1/256 HP.
                    u.hp = ((ty.hit_points_raw as i64 * pct as i64 / 100) as i32).max(1);
                }
                if mu.fields_set & 0x04 != 0 && ty.has_shields {
                    let pct = (mu.shield_percent as i32).min(100);
                    u.shields = (fp(ty.shield_points as i32) * pct / 100).max(0);
                }
                if mu.fields_set & 0x08 != 0 {
                    let pct = (mu.energy_percent as i32).min(100);
                    u.set_energy_fixed(MAX_ENERGY * ENERGY_SCALE * pct / 100);
                }
                if mu.fields_set & 0x20 != 0 {
                    u.hangar = mu.hangar.min(u.hangar_capacity as u16) as u8;
                }
                if is_resource_type(u.type_id) {
                    let capacity = if u.type_id == uid::VESPENE_GEYSER {
                        GAS_CAPACITY
                    } else {
                        MINERAL_CAPACITY
                    };
                    u.resource_amount = if mu.fields_set & 0x10 != 0 || mu.resource_amount > 0 {
                        (mu.resource_amount as u16) as i32
                    } else {
                        capacity
                    };
                }
                let states = mu.states & mu.states_set;
                if states & 0x01 != 0 {
                    u.status.cloaked = true;
                }
                if states & 0x02 != 0 {
                    u.status.burrowed = true;
                }
                if states & 0x04 != 0 && can_lift_building(u.type_id) {
                    u.lifted = true;
                }
                if states & 0x08 != 0 {
                    u.status.hallucination = true;
                }
                if states & 0x10 != 0 {
                    u.invincible = true;
                }
            }
        }
        // `0x70fc40`: THG2 unit records become doodad CUnits.  Static
        // sprites (`0x1000`) are never simulated.
        for th in &scenario.thingies {
            if th.flags & 0x1000 != 0 || !is_doodad_type(th.unit_id) {
                continue;
            }
            let owner = if is_door_type(th.unit_id) {
                0
            } else if (th.player as usize) < world.players.len() {
                th.player as usize
            } else {
                0
            };
            // `0x00711730`: realize the THG2 record as a unit, then apply
            // the disabled bit from `0x005b56a0`.
            if let Ok(i) = world.spawn(owner, th.unit_id, th.x, th.y) {
                world.units[i].doodad_enabled = Some(th.flags & 0x8000 == 0);
            }
        }

        // Link add-ons and Nydus pairs by the serial numbers StarEdit writes.
        // The link flags at offset 10 are ignored by the retail engine; only
        // the two records naming each other matter.
        for mu in &scenario.units {
            if mu.link_serial == 0 {
                continue;
            }
            let (Some(&i), Some(&j)) = (
                serial_to_index.get(&mu.serial),
                serial_to_index.get(&mu.link_serial),
            ) else {
                continue;
            };
            if i == j || !world.units[i].alive || !world.units[j].alive {
                continue;
            }
            if is_nydus_canal_type(world.units[i].type_id)
                && is_nydus_canal_type(world.units[j].type_id)
            {
                world.units[i].nydus_exit = Some(world.units[j].id);
                world.units[j].nydus_exit = Some(world.units[i].id);
            }
            // Add-on -> parent, or parent -> add-on.
            if let Some(parent_type) = addon_parent(world.units[i].type_id) {
                if world.units[j].type_id == parent_type && world.units[j].addon.is_none() {
                    let addon_id = world.units[i].id;
                    world.units[j].a14_set_addon_link(addon_id);
                    world.units[i].addon_attached = true;
                }
            } else if let Some(child_parent) = addon_parent(world.units[j].type_id)
                && world.units[i].type_id == child_parent
                && world.units[i].addon.is_none()
            {
                let addon_id = world.units[j].id;
                world.units[i].a14_set_addon_link(addon_id);
                world.units[j].addon_attached = true;
            }
        }
        // Legacy/protected maps with no usable serial link: attach an add-on
        // that sits beside its parent.
        let placed_addons: Vec<(usize, u16, (i32, i32))> = world
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| {
                addon_parent(u.type_id).is_some() && u.alive && u.completed && !u.addon_attached
            })
            .map(|(i, u)| (i, u.type_id, (u.x, u.y)))
            .collect();
        for (ai, addon_type, (ax, ay)) in placed_addons {
            let Some(parent_type) = addon_parent(addon_type) else {
                continue;
            };
            let parent = world.units.iter().position(|u| {
                u.alive
                    && u.completed
                    && u.type_id == parent_type
                    && u.addon.is_none()
                    && (u.x - ax).abs() <= fp(96)
                    && (u.y - ay).abs() <= fp(96)
            });
            if let Some(p) = parent {
                world.units[p].addon = Some(world.units[ai].id);
                world.units[ai].addon_attached = true;
            }
        }
        // Melee maps contain only start locations and resources; create the
        // stock starting town hall and four workers for every player.  A
        // scenario that already places a force for any active player is a
        // campaign/use-map map and must not receive free bases.
        let has_placed_force = scenario.units.iter().any(|mu| {
            mu.unit_id != uid::START_LOCATION
                && !is_resource_type(mu.unit_id)
                && (mu.owner as usize) < world.players.len()
        });
        let starts = scenario.start_locations();
        for p in 0..world.players.len() {
            if has_placed_force {
                break;
            }
            if occupied.is_some_and(|o| !o.get(p).copied().unwrap_or(false)) {
                world.players[p].defeated = true;
                continue;
            }
            let race = world.players[p].race;
            let start = scenario
                .start_location_of(p as u8)
                .or_else(|| starts.get(p).copied())
                .or_else(|| starts.first().copied())
                .unwrap_or((16 + p as i32 * 16, 16));
            let (sx, sy) = (start.0, start.1);
            let has_base = world.units.iter().any(|u| {
                u.alive
                    && u.owner == p as u8
                    && u.completed
                    && matches!(u.type_id, uid::COMMAND_CENTER | uid::NEXUS | uid::HATCHERY)
            });
            if has_base {
                continue;
            }
            let (town, worker) = match race {
                Race::Terran => (uid::COMMAND_CENTER, uid::SCV),
                Race::Protoss => (uid::NEXUS, uid::PROBE),
                Race::Zerg => (uid::HATCHERY, uid::DRONE),
                Race::Neutral => (uid::COMMAND_CENTER, uid::SCV),
            };
            let town_idx = world.spawn_px(p, town, sx, sy).ok();
            // Start-unit offsets measured on the retail captures of
            // (8)The Hunters (135051: probes in a row under the Nexus;
            // 134905: drones around the Hatchery, whose larvae take the
            // spots under it, and the Overlord to the lower left).
            // INFERENCE: retail runs its placement search; these are its
            // results for an unobstructed start location.
            let workers: [(i32, i32); 4] = if race == Race::Zerg {
                [(-49, 45), (11, 45), (39, 45), (54, 0)]
            } else {
                [(-48, 53), (-24, 53), (0, 53), (24, 53)]
            };
            for (dx, dy) in workers {
                let _ = world.spawn_px(p, worker, sx + dx, sy + dy);
            }
            // A melee Zerg start also includes one Overlord; it is the
            // starting supply provider (the Hatchery alone provides 1).
            // Retail `create_starting_units` (OpenBW): the Overlord sits the
            // town hall's full extent + 1 px from the start location, toward
            // the map centre on each axis; the Hatchery spawns two larvae at
            // its first larva spot, 10 px below its box.
            if race == Race::Zerg {
                let d = world.data.unit(town).dimensions;
                let add_x = d.left as i32 + d.right as i32 + 1;
                let add_y = d.top as i32 + d.bottom as i32 + 1;
                let (mw, mh) = (
                    world.terrain.width as i32 * 32,
                    world.terrain.height as i32 * 32,
                );
                let ox = if sx >= mw / 2 { sx - add_x } else { sx + add_x };
                let oy = if sy >= mh / 2 { sy - add_y } else { sy + add_y };
                let _ = world.spawn_px(p, uid::OVERLORD, ox, oy);
                let town_id = town_idx.map(|i| world.units[i].id);
                for _ in 0..2 {
                    if let (Some(parent), Ok(l)) = (
                        town_id,
                        world.spawn_px(p, uid::LARVA, sx, sy + d.bottom as i32 + 10),
                    ) {
                        world.units[l].connected_unit = Some(parent);
                    }
                }
            }
            world.players[p].minerals = 50;
        }
        if !has_placed_force {
            world.mature_creep();
        }
        if !scenario.triggers.is_empty() {
            // `MRGN` rectangles are already in pixels (Boot Camp's location
            // 1 is 0..1280 x 0..2048 on a 64x64-tile = 2048 px map); the
            // old x32 made every location cover the whole map.
            let locations: Vec<(i32, i32, i32, i32)> = scenario
                .locations
                .iter()
                .map(|l| (l.left, l.top, l.right, l.bottom))
                .collect();
            world.triggers = Some(crate::trigger::TriggerSystem::parse(
                &scenario.triggers,
                &locations,
            ));
            if let Some(t) = world.triggers.as_mut() {
                t.unit_properties = scenario.unit_properties.clone();
                t.forces = scenario.forces;
                for (p, present) in t.present.iter_mut().enumerate() {
                    *present = scenario.owners[p] != 0;
                }
                t.mission_objectives = vec![None; world.players.len()];
                t.preserved = vec![vec![false; t.triggers.len()]; world.players.len()];
            }
        }
        world
    }

    /// `0x005b4fa0`: display name from the `UNIS`/`UNIx` string id, with
    /// `stat_txt.tbl` as the fallback.
    pub fn unit_name(&self, type_id: u16) -> &str {
        if let Some(settings) = &self.rules.unit_settings
            && settings.use_default.get(type_id as usize).copied() == Some(0)
            && let Some(&id) = settings.string_id.get(type_id as usize)
            && id != 0
        {
            return self.data.strings.text(id);
        }
        self.data.unit_name(type_id)
    }

    /// `0x005b3a50`: nonzero when the type carries `units.dat` flag 0x80.
    pub fn unit_regenerates(&self, type_id: u16) -> bool {
        self.data.unit(type_id).regenerates()
    }

    /// `0x005b57b0`: nonzero when the type carries `units.dat` flag 0x2.
    pub fn unit_is_addon(&self, type_id: u16) -> bool {
        self.data.unit(type_id).is_addon()
    }

    /// `0x005b5950`: nonzero when the type carries `units.dat` flag 0x200.
    pub fn unit_can_cloak(&self, type_id: u16) -> bool {
        self.data.unit(type_id).can_cloak()
    }

    /// `0x005b66c0`: clear status bit 26 unless the type is below 0xe4 and
    /// carries `units.dat` flag 0x20000000.
    pub fn clear_unsupported_flag_26(&mut self, i: usize) {
        let ty = self.data.unit(self.units[i].type_id);
        if self.units[i].type_id >= 0xe4 || ty.flags & 0x2000_0000 == 0 {
            self.units[i].flag_26 = false;
        }
    }

    /// `0x005b61d0`: nonzero when the type carries `units.dat` flag 0x2000.
    pub fn unit_is_resource_container(&self, type_id: u16) -> bool {
        self.data.unit(type_id).is_resource_container()
    }

    /// `0x005b53a0`: `CUnit+0xd4` resource amount for types with the
    /// `units.dat` powerup/resource flag 0x800; all other types report 0.
    pub fn unit_resource_amount(&self, i: usize) -> i32 {
        let u = &self.units[i];
        if self.data.unit(u.type_id).is_powerup_flag() {
            u.resource_amount
        } else {
            0
        }
    }

    /// `0x005c1880`: true when the terrain at a placement point accepts a
    /// building (retail checks the tile's build/walk flag word).
    pub fn terrain_accepts_placement(&self, x: i32, y: i32) -> bool {
        self.terrain.buildable_px(x, y)
    }

    /// `0x005c1d60`: retail's placement pre-check.  The point must satisfy
    /// the type's `units.dat` box against the map bounds and the terrain
    /// check above; the pathfinder occupancy test (`0x00761210`) follows in
    /// [`Self::placement_spot`].
    pub fn placement_corner_valid(&self, type_id: u16, x: i32, y: i32) -> bool {
        let d = self.data.unit(type_id).dimensions;
        let width = self.terrain.width as i32 * 32;
        let height = self.terrain.height as i32 * 32;
        d.left as i32 <= x
            && d.right as i32 + x < width
            && d.top as i32 <= y
            && d.bottom as i32 + y < height
            && self.terrain_accepts_placement(x, y)
    }

    /// `0x005b6ac0`: `units.dat` placement X (pixels).
    pub fn unit_placement_x(&self, type_id: u16) -> i32 {
        self.data.unit(type_id).placement.x as i32
    }

    /// `0x005b6ab0`: `units.dat` placement Y (pixels).
    pub fn unit_placement_y(&self, type_id: u16) -> i32 {
        self.data.unit(type_id).placement.y as i32
    }

    /// `0x005a55e0`: living unit ids within one 32 px search half-extent
    /// of a world-pixel point.
    pub fn units_near_point(&self, point: (i32, i32)) -> Vec<u32> {
        self.units_in_rect(point, 32)
    }

    /// `0x005c9620`: living unit ids whose centres lie inside the
    /// axis-aligned box `centre ± half` (world pixels).
    pub fn units_in_rect(&self, centre: (i32, i32), half: i32) -> Vec<u32> {
        self.units
            .iter()
            .filter(|u| {
                u.alive
                    && (u.x / fp(1) - centre.0).abs() <= half
                    && (u.y / fp(1) - centre.1).abs() <= half
            })
            .map(|u| u.id)
            .collect()
    }

    /// `0x005b0cf0`: 1-based CUnit slot number (retail derives it from the
    /// pointer distance to the unit array base).
    pub fn unit_slot_number(&self, i: usize) -> u32 {
        i as u32 + 1
    }

    /// `0x005b4be0`: worker-only `CUnit+0xc0` linked target.
    pub fn unit_worker_target(&self, i: usize) -> Option<u32> {
        let u = &self.units[i];
        if self.data.unit(u.type_id).is_worker() {
            u.linked_target()
        } else {
            None
        }
    }

    /// `0x005b55d0`: the worker's linked target is the Flag unit row 0xd7.
    pub fn unit_worker_target_is_flag(&self, i: usize) -> bool {
        self.unit_worker_target(i)
            .and_then(|id| self.index_of(id))
            .is_some_and(|j| self.units[j].type_id == 0xd7)
    }

    /// `0x005b4b00`: `units.dat` destroy score used by the score screen.
    pub fn unit_destroy_score(&self, type_id: u16) -> u16 {
        self.data.unit(type_id).destroy_score
    }

    /// `0x005b4b20`: `units.dat` maximum shield points.
    pub fn unit_shield_points(&self, type_id: u16) -> i32 {
        self.data.unit(type_id).shield_points as i32
    }

    /// `0x005b5250`: score-screen race class from `group_flags`
    /// (Zerg = 0, Terran = 1, Protoss = 2, other = 4).
    pub fn unit_race_class(&self, type_id: u16) -> u8 {
        let group = self.data.unit(type_id).group_flags;
        if group & 1 != 0 {
            0
        } else if group & 4 != 0 {
            2
        } else if group & 2 != 0 {
            1
        } else {
            4
        }
    }

    /// `0x005b59f0`: true while current HP is below the type's raw maximum.
    pub fn unit_is_damaged(&self, i: usize) -> bool {
        let u = &self.units[i];
        (u.hp as u32) < self.data.unit(u.type_id).hit_points_raw
    }

    /// `0x005a3df0`: true when the id is below 0xe4 and `units.dat` flag
    /// 0x20000000 is clear.
    pub fn unit_flag_29_clear(&self, type_id: u16) -> bool {
        type_id < 0xe4 && self.data.unit(type_id).flags & 0x2000_0000 == 0
    }

    /// `0x005b6b20`: energy flag 0x200000 on a live unit (retail also
    /// requires `CUnit+0xdc` bit 30, the dying flag, to be clear).
    pub fn unit_has_energy(&self, i: usize) -> bool {
        let u = &self.units[i];
        u.alive && self.data.unit(u.type_id).has_energy()
    }

    /// `0x005b68a0`: nonzero when `units.dat` marks the type as shielded.
    pub fn unit_has_shields(&self, type_id: u16) -> bool {
        self.data.unit(type_id).has_shields
    }

    /// `0x005b4aa0`: current HP in whole points plus the shield pool in
    /// whole points for shielded types (`FUN_00588d60` + `CUnit+0x60 >> 8`).
    pub fn unit_current_total_points(&self, i: usize) -> i32 {
        let u = &self.units[i];
        let hp = (u.hp + 0xff) >> 8;
        if self.data.unit(u.type_id).has_shields {
            hp + (u.shields >> 8)
        } else {
            hp
        }
    }

    /// `0x005b6710`: set the shield pool from whole points, clamped to the
    /// type's `units.dat` maximum and stored in 1/256 points.
    pub fn set_unit_shields_points(&mut self, i: usize, points: i32) {
        let max = self.data.unit(self.units[i].type_id).shield_points as i32;
        self.units[i].shields = points.min(max).max(0) << 8;
    }

    /// `0x005b4a50`: one third of the raw maximum-HP table value.
    pub fn unit_max_hp_third(&self, type_id: u16) -> i32 {
        self.data.unit(type_id).hit_points_raw as i32 / 3
    }

    /// `0x005b4b30`: maximum HP in whole points (`raw >> 8`).  Types whose
    /// raw HP is below one point fall back to the current HP rounded up, or
    /// one when that is also zero.
    pub fn unit_max_hp_points(&self, i: usize) -> i32 {
        let u = &self.units[i];
        let points = self.data.unit(u.type_id).hit_points_raw as i32 >> 8;
        if points != 0 {
            points
        } else {
            ((u.hp + 0xff) >> 8).max(1)
        }
    }

    /// `0x005b6560`: whole-point HP plus shields for shielded types.
    pub fn unit_total_hit_points(&self, i: usize) -> i32 {
        let u = &self.units[i];
        let ty = self.data.unit(u.type_id);
        let shields = if ty.has_shields {
            ty.shield_points as i32
        } else {
            0
        };
        self.unit_max_hp_points(i) + shields
    }

    /// `0x005b5820`: `units.dat` building flag 0x1, but only for ids below
    /// 0xe4; higher ids are not buildings.
    pub fn unit_is_building_flagged(&self, type_id: u16) -> bool {
        type_id < 0xe4 && self.data.unit(type_id).is_building()
    }

    /// `0x005b58e0`: inverse of `0x005b5820`, treating every id >= 0xe4 as
    /// non-building.
    pub fn unit_is_not_building_flagged(&self, type_id: u16) -> bool {
        type_id >= 0xe4 || !self.data.unit(type_id).is_building()
    }

    /// `0x005b5ea0`: `units.dat` flying-building flag 0x20.
    pub fn unit_is_flying_building(&self, type_id: u16) -> bool {
        self.data.unit(type_id).is_flying_building()
    }

    /// `0x005b6a80`: units.dat box left extent.    /// `0x005b6a80`: units.dat box left extent.
    pub fn unit_box_left(&self, type_id: u16) -> i32 {
        self.data.unit(type_id).dimensions.left as i32
    }

    /// `0x005b6a90`: units.dat box right extent.
    pub fn unit_box_right(&self, type_id: u16) -> i32 {
        self.data.unit(type_id).dimensions.right as i32
    }

    /// `0x005b6aa0`: units.dat box top extent.
    pub fn unit_box_top(&self, type_id: u16) -> i32 {
        self.data.unit(type_id).dimensions.top as i32
    }

    /// `0x005b6a60`: units.dat box bottom extent.
    pub fn unit_box_bottom(&self, type_id: u16) -> i32 {
        self.data.unit(type_id).dimensions.bottom as i32
    }

    /// `0x005b6ad0`: total box width including the centre column
    /// (`right + 1 + left`).
    pub fn unit_box_width(&self, type_id: u16) -> i32 {
        let d = self.data.unit(type_id).dimensions;
        d.right as i32 + 1 + d.left as i32
    }

    /// `0x005b6a40`: total box height including the centre row
    /// (`top + 1 + bottom`).
    pub fn unit_box_height(&self, type_id: u16) -> i32 {
        let d = self.data.unit(type_id).dimensions;
        d.top as i32 + 1 + d.bottom as i32
    }

    /// `0x007601c0`: pathfinder box extents.  Types below 0x8000 read
    /// `units.dat` dimensions; types with bit 15 set use the low 15 bits as
    /// an all-round radius.  The stored order is
    /// `[top, -right, -bottom, left, top+1, ~right, ~bottom, left+1]`.
    pub fn unit_path_extents(&self, type_id: u16) -> [i32; 8] {
        let (top, right, bottom, left) = if type_id < 0x8000 {
            let d = self.data.unit(type_id).dimensions;
            (d.top as i32, d.right as i32, d.bottom as i32, d.left as i32)
        } else {
            let r = (type_id & 0x7fff) as i32;
            (r, r, r, r)
        };
        [
            top,
            -right,
            -bottom,
            left,
            top + 1,
            !right,
            !bottom,
            left + 1,
        ]
    }

    /// `0x005b6670`: supply provided by a unit type.
    pub fn unit_supply_provided(&self, type_id: u16) -> u8 {
        self.data.unit(type_id).supply_provided
    }

    /// `0x005b4a40`: maximum HP table lookup for a unit type.
    pub fn unit_max_hp_raw(&self, type_id: u16) -> i32 {
        self.unit_stats(type_id).hit_points_raw as i32
    }

    /// Unit stats with the scenario's `UNIS`/`UNIx` overrides applied.
    pub fn unit_stats(&self, type_id: u16) -> UnitType {
        let mut ty = self.data.unit(type_id).clone();
        if let Some(settings) = &self.rules.unit_settings
            && settings.use_default.get(type_id as usize).copied() == Some(0)
            && let Some(&hp) = settings.hit_points.get(type_id as usize)
        {
            ty.hit_points_raw = hp;
            if let Some(&shields) = settings.shields.get(type_id as usize) {
                ty.shield_points = shields;
                ty.has_shields = shields > 0;
            }
            if let Some(&armor) = settings.armor.get(type_id as usize) {
                ty.armor = armor;
            }
            if let Some(&build) = settings.build_time.get(type_id as usize) {
                ty.build_time = build;
            }
            if let Some(&minerals) = settings.mineral_cost.get(type_id as usize) {
                ty.mineral_cost = minerals;
            }
            if let Some(&gas) = settings.gas_cost.get(type_id as usize) {
                ty.gas_cost = gas;
            }
        }
        ty
    }

    /// Technology stats with `TECS`/`TECx` overrides.
    pub fn tech_stats(&self, tech_id: u8) -> Option<TechType> {
        let mut tech = self.data.tech(tech_id)?.clone();
        if let Some(settings) = &self.rules.tech_settings
            && settings.use_default.get(tech_id as usize).copied() == Some(0)
            && let Some(&minerals) = settings.mineral_cost.get(tech_id as usize)
        {
            tech.mineral_cost = minerals;
            if let Some(&gas) = settings.gas_cost.get(tech_id as usize) {
                tech.gas_cost = gas;
            }
            if let Some(&time) = settings.research_time.get(tech_id as usize) {
                tech.research_time = time;
            }
            if let Some(&energy) = settings.energy_cost.get(tech_id as usize) {
                tech.energy_cost = energy;
            }
        }
        Some(tech)
    }

    /// Upgrade stats with `UPGS`/`UPGx` overrides.
    pub fn upgrade_stats(&self, upgrade_id: u8) -> Option<UpgradeType> {
        let mut upgrade = self.data.upgrade(upgrade_id)?.clone();
        if let Some(settings) = &self.rules.upgrade_settings
            && settings.use_default.get(upgrade_id as usize).copied() == Some(0)
            && let Some(&base) = settings.mineral_base.get(upgrade_id as usize)
        {
            upgrade.mineral_cost_base = base;
            if let Some(&v) = settings.mineral_factor.get(upgrade_id as usize) {
                upgrade.mineral_cost_factor = v;
            }
            if let Some(&v) = settings.gas_base.get(upgrade_id as usize) {
                upgrade.gas_cost_base = v;
            }
            if let Some(&v) = settings.gas_factor.get(upgrade_id as usize) {
                upgrade.gas_cost_factor = v;
            }
            if let Some(&v) = settings.time_base.get(upgrade_id as usize) {
                upgrade.time_cost_base = v;
            }
            if let Some(&v) = settings.time_factor.get(upgrade_id as usize) {
                upgrade.time_cost_factor = v;
            }
        }
        Some(upgrade)
    }

    /// `PUNI` unit availability for a player.
    pub fn unit_available(&self, owner: u8, type_id: u16) -> bool {
        self.rules
            .unit_availability
            .as_ref()
            .is_none_or(|a| a.available(owner as usize, type_id))
    }

    /// `PTEC`/`PTEx` technology availability.
    pub fn tech_available(&self, owner: u8, tech_id: u8) -> bool {
        self.rules
            .tech_restrictions
            .as_ref()
            .is_none_or(|r| r.available(owner as usize, tech_id))
    }

    /// `PTEC`/`PTEx` already-researched bit.
    pub fn tech_researched(&self, owner: u8, tech_id: u8) -> bool {
        self.rules
            .tech_restrictions
            .as_ref()
            .is_some_and(|r| r.researched(owner as usize, tech_id))
    }

    /// `UPGR`/`PUPx` maximum level, falling back to `upgrades.dat`.
    pub fn upgrade_max_level(&self, owner: u8, upgrade_id: u8) -> u8 {
        self.rules.upgrade_restrictions.as_ref().map_or_else(
            || {
                self.data
                    .upgrade(upgrade_id)
                    .map(|u| u.max_level)
                    .unwrap_or(0)
            },
            |r| r.max_level(owner as usize, upgrade_id),
        )
    }

    /// `UPGR`/`PUPx` starting level for a player.
    pub fn upgrade_start_level(&self, owner: u8, upgrade_id: u8) -> u8 {
        self.rules
            .upgrade_restrictions
            .as_ref()
            .map_or(0, |r| r.start_level(owner as usize, upgrade_id))
    }

    /// `FUN_005cc6d0` maximum energy in 1/256 points for a unit type and
    /// owner (hero or researched reactor upgrade).
    pub fn max_energy_fixed(&self, type_id: u16, owner: u8) -> i32 {
        self.players
            .get(owner as usize)
            .map_or(MAX_ENERGY * ENERGY_SCALE, |p| {
                unit_max_energy_fixed(&self.data, &p.upgrades, type_id)
            })
    }

    /// Retail sight getter (`0x5a6670`).  Ocular Implants, Antennae,
    /// Sensor Array and Apial Sensors *replace* the unit's sight with 11
    /// tiles; they do not add two.  The Scout's `units.dat` sight is 8, so
    /// the retail switch jumps it straight to 11.
    /// Sight of one unit in tiles: [`Self::sight_range`], or 2 while it is
    /// blinded by Optical Flare.
    pub fn unit_sight_range(&self, i: usize) -> i32 {
        let u = &self.units[i];
        if u.status.optical_flare_ticks > 0 {
            2
        } else {
            self.sight_range(u.owner, u.type_id)
        }
    }

    pub fn sight_range(&self, owner: u8, type_id: u16) -> i32 {
        let base = self.data.unit(type_id).sight_range as i32;
        let upgrade_id = match type_id {
            uid::GHOST => upgrade::OCULAR_IMPLANTS,
            uid::OVERLORD => upgrade::ANTENNAE,
            uid::SCOUT => upgrade::APIAL_SENSORS,
            uid::OBSERVER => upgrade::SENSOR_ARRAY,
            _ => return base,
        };
        if self
            .players
            .get(owner as usize)
            .is_some_and(|p| p.upgrade_level(upgrade_id) > 0)
        {
            11
        } else {
            base
        }
    }

    /// Recompute the cached caps after an upgrade changes (including every
    /// existing unit of that type) and clamp current energy down if needed.
    pub fn refresh_energy_caps(&mut self) {
        for i in 0..self.units.len() {
            let type_id = self.units[i].type_id;
            let owner = self.units[i].owner;
            let cap = self.max_energy_fixed(type_id, owner);
            self.units[i].energy_cap = cap;
            if self.units[i].energy_fixed() > cap {
                self.units[i].set_energy_fixed(cap);
            }
        }
    }

    /// Turn on rules-level fog of war.  A scenario gets a vision grid when it
    /// is loaded; for a synthetic `World::new` the grid is created lazily from
    /// the terrain extent.
    pub fn enable_fog(&mut self) {
        if self.vision.is_none() {
            self.vision = Some(Vision::new(self.terrain.width, self.terrain.height));
        }
        self.fog_of_war = true;
        self.update_vision();
    }

    pub fn index_of(&self, id: u32) -> Option<usize> {
        self.index.get(&id).copied()
    }

    pub fn unit(&self, id: u32) -> Option<&SimUnit> {
        self.index_of(id).map(|i| &self.units[i])
    }

    pub fn unit_mut(&mut self, id: u32) -> Option<&mut SimUnit> {
        self.index_of(id).map(|i| &mut self.units[i])
    }

    /// `0x005b0fe0`: retail unit constructor.  Its `0x00585350` sub-init
    /// (owner, order/target zeroing) is mirrored by the `SimUnit`
    /// initializer.  This path clears the same runtime state (target, order
    /// timers, energy, hp) and, because the engine spawns a completed unit,
    /// takes the finished-HP branch.
    pub fn spawn(&mut self, owner: usize, type_id: u16, x: i32, y: i32) -> Result<usize, SimError> {
        if owner >= self.players.len() {
            return Err(SimError::BadOwner);
        }
        let ty = self.unit_stats(type_id);
        let shields = if ty.has_shields {
            fp(ty.shield_points as i32)
        } else {
            0
        };
        let energy_cap = self.max_energy_fixed(type_id, owner as u8);
        let energy = if ty.flags & 0x0020_0000 != 0 {
            energy_cap / ENERGY_SCALE
        } else {
            0
        };
        let id = self.next_id;
        self.next_id += 1;
        let unit = SimUnit {
            id,
            kills: 0,
            builder: 0,
            owner: owner as u8,
            type_id,
            hp: ty.hit_points_raw as i32,
            shields,
            energy,
            energy_frac: 0,
            energy_cap,
            x: x.saturating_mul(fp(1)),
            y: y.saturating_mul(fp(1)),
            facing: 0,
            alive: true,
            completed: true,
            credited: false,
            lifted: false,
            addon_attached: false,
            build_remaining: 0,
            build_total: 0,
            cooldown: 0,
            order: Order::Idle,
            resume_order: None,
            queue: VecDeque::new(),
            target: None,
            path: VecDeque::new(),
            path_goal: None,
            path_stall: 0,
            speed: 0,
            moved_tick: 0,
            path_last_dist: 0,
            rally: None,
            carry: 0,
            carry_kind: CarryKind::None,
            gather_timer: 0,
            resource_target: None,
            depot: None,
            production: VecDeque::new(),
            addon: None,
            cargo: Vec::new(),
            in_transport: None,
            status: Status::default(),
            larva_timer: 0,
            resource_amount: 0,
            morph_to: None,
            last_damaged: 0,
            invincible: false,
            last_damage_source: 0,
            doodad_enabled: None,
            nuke_ready: false,
            nydus_exit: None,
            expires_at: 0,
            mine_armed_at: 0,
            repulse_dir: 0,
            repulse_flags: 0,
            turret_facing: 0,
            repair_debt: 0,
            hangar: default_hangar(type_id),
            hangar_capacity: default_hangar_capacity(type_id),
            special_cooldown: 0,
            order_carry: 0,
            speech_ticks: 0,
            speech_id: 0,
            death_timer: 0,
            rank_kills: 0,
            visibility_mask: u32::MAX,
            in_bunker: false,
            flag_26: false,
            overlay_flags: 0,
            flag_10: false,
            flag_14: false,
            flag_15: false,
            flag_29: false,
            flag_21: false,
            flag_18: false,
            shield_recharged: false,
            flag_16: false,
            flag_17: false,
            flag_19: false,
            field_100: 0,
            flag_22: false,
            flags_105: 0,
            flags_93: 0,
            field_091: 0,
            field_096: 0,
            field_0cf: 0,
            field_090: 8,
            field_094: 0,
            field_0ac: 0,
            field_0e1: 0,
            timer_087: 0,
            order_wait: 0,
            connected_unit: None,
            larva_spawn_side_values: [0; 4],
            order_row: 0,
            anim_tick: 0,
            anim: None,
            attacks_fired: 0,
        };
        let idx = self.units.len();
        self.index.insert(id, idx);
        self.units.push(unit);
        if is_creep_source(type_id) {
            self.creep_dirty = true;
        }
        self.start_animation(idx, ANIM_INIT);
        Ok(idx)
    }

    /// Spawn using pixel coordinates rather than tiles.
    pub fn spawn_px(
        &mut self,
        owner: usize,
        type_id: u16,
        x: i32,
        y: i32,
    ) -> Result<usize, SimError> {
        let ty = self.unit_stats(type_id);
        let shields = if ty.has_shields {
            fp(ty.shield_points as i32)
        } else {
            0
        };
        let energy_cap = self.max_energy_fixed(type_id, owner as u8);
        let energy = if ty.flags & 0x0020_0000 != 0 {
            energy_cap / ENERGY_SCALE
        } else {
            0
        };
        let id = self.next_id;
        self.next_id += 1;
        let unit = SimUnit {
            id,
            kills: 0,
            builder: 0,
            owner: owner as u8,
            type_id,
            hp: ty.hit_points_raw as i32,
            shields,
            energy,
            energy_frac: 0,
            energy_cap,
            x: x.saturating_mul(fp(1)),
            y: y.saturating_mul(fp(1)),
            facing: 0,
            alive: true,
            completed: true,
            credited: false,
            lifted: false,
            addon_attached: false,
            build_remaining: 0,
            build_total: 0,
            cooldown: 0,
            order: Order::Idle,
            resume_order: None,
            queue: VecDeque::new(),
            target: None,
            path: VecDeque::new(),
            path_goal: None,
            path_stall: 0,
            speed: 0,
            moved_tick: 0,
            path_last_dist: 0,
            rally: None,
            carry: 0,
            carry_kind: CarryKind::None,
            gather_timer: 0,
            resource_target: None,
            depot: None,
            production: VecDeque::new(),
            addon: None,
            cargo: Vec::new(),
            in_transport: None,
            status: Status::default(),
            larva_timer: 0,
            resource_amount: 0,
            morph_to: None,
            last_damaged: 0,
            invincible: false,
            last_damage_source: 0,
            doodad_enabled: None,
            nuke_ready: false,
            nydus_exit: None,
            expires_at: 0,
            mine_armed_at: 0,
            repulse_dir: 0,
            repulse_flags: 0,
            turret_facing: 0,
            repair_debt: 0,
            hangar: default_hangar(type_id),
            hangar_capacity: default_hangar_capacity(type_id),
            special_cooldown: 0,
            order_carry: 0,
            speech_ticks: 0,
            speech_id: 0,
            death_timer: 0,
            rank_kills: 0,
            visibility_mask: u32::MAX,
            in_bunker: false,
            flag_26: false,
            overlay_flags: 0,
            flag_10: false,
            flag_14: false,
            flag_15: false,
            flag_29: false,
            flag_21: false,
            flag_18: false,
            shield_recharged: false,
            flag_16: false,
            flag_17: false,
            flag_19: false,
            field_100: 0,
            flag_22: false,
            flags_105: 0,
            flags_93: 0,
            field_091: 0,
            field_096: 0,
            field_0cf: 0,
            field_090: 8,
            field_094: 0,
            field_0ac: 0,
            field_0e1: 0,
            timer_087: 0,
            order_wait: 0,
            connected_unit: None,
            larva_spawn_side_values: [0; 4],
            order_row: 0,
            anim_tick: 0,
            anim: None,
            attacks_fired: 0,
        };
        let idx = self.units.len();
        self.index.insert(id, idx);
        self.units.push(unit);
        if is_creep_source(type_id) {
            self.creep_dirty = true;
        }
        self.start_animation(idx, ANIM_INIT);
        Ok(idx)
    }

    /// Spawn a unit and apply one `UPRP` property slot (trigger actions 11 and
    /// 44).  Only fields whose flag bit is set are applied; percentages clamp
    /// to 1..=100 and resources/hangar keep the low 16/8 bits, matching the
    /// retail loader.
    /// `0x00712ce0`: apply a `UPRP`/UNIT property slot.  The owner byte
    /// (field flag 0x01) is not read by retail; HP is raw fixed-point,
    /// shields/energy are percentages of the cap, resources only apply to
    /// `units.dat` flag 0x2000, and hangar is clamped to the unit's capacity.
    pub fn spawn_with_properties(
        &mut self,
        owner: usize,
        type_id: u16,
        x: i32,
        y: i32,
        props: &UnitProperties,
    ) -> Result<usize, SimError> {
        let idx = self.spawn_px(owner, type_id, x, y)?;
        let ty = self.unit_stats(type_id);
        let energy_cap = self.max_energy_fixed(type_id, self.units[idx].owner);
        let u = &mut self.units[idx];
        if props.field_flags & 0x02 != 0 {
            let pct = props.hit_points_percent.min(100) as i32;
            u.hp = ((ty.hit_points_raw as i64 * pct as i64 / 100) as i32).max(1);
        }
        if props.field_flags & 0x04 != 0 && ty.has_shields {
            let pct = props.shields_percent.min(100) as i32;
            let max = ty.shield_points as i32;
            u.shields = fp((max * pct / 100).min(max));
        }
        if props.field_flags & 0x08 != 0 {
            let pct = props.energy_percent.min(100) as i32;
            u.set_energy_fixed(energy_cap * pct / 100);
        }
        if props.field_flags & 0x10 != 0 && ty.flags & 0x2000 != 0 {
            u.resource_amount = (props.resource_amount as u16) as i32;
        }
        if props.field_flags & 0x20 != 0 {
            let capacity = u.hangar_capacity as u16;
            let count = props.hangar.min(capacity);
            if capacity != 0 && count != 0 {
                u.hangar = count as u8;
            }
        }
        let states = props.states & props.state_flags;
        if states & 0x01 != 0 {
            u.status.cloaked = true;
        }
        if states & 0x02 != 0 {
            u.status.burrowed = true;
        }
        if states & 0x04 != 0 && can_lift_building(u.type_id) {
            u.lifted = true;
        }
        if states & 0x08 != 0 {
            u.status.hallucination = true;
        }
        if states & 0x10 != 0 {
            u.invincible = true;
        }
        Ok(idx)
    }

    /// Ticks of progress per simulation tick (Operation CWAL runs at 20x).
    fn build_step(&self, owner: u8) -> i32 {
        if self
            .players
            .get(owner as usize)
            .is_some_and(|p| p.cheats.fast_build)
        {
            20
        } else {
            1
        }
    }

    pub fn supply_cap(&self, owner: u8) -> i32 {
        if self
            .players
            .get(owner as usize)
            .is_some_and(|p| p.cheats.no_supply)
        {
            return 400;
        }
        // Supply is stored in half-points, so the retail 200 cap is 400 raw.
        let cap: i32 = self
            .units
            .iter()
            .filter(|u| u.alive && u.owner == owner && u.completed)
            .map(|u| self.data.unit(u.type_id).supply_provided as i32)
            .sum();
        cap.min(400)
    }

    /// Recompute the creep mask from completed Zerg creep sources.  Called
    /// lazily; sources are Hatcheries/Lairs/Hives and the three colony
    /// buildings.  Creep covers walkable tiles within three tiles of the
    /// building footprint.
    fn rebuild_creep(&mut self) {
        if !self.creep_dirty {
            return;
        }
        let w = self.terrain.width as i32;
        let h = self.terrain.height as i32;
        self.creep_supported.iter_mut().for_each(|s| *s = false);
        let mut seed = vec![false; self.creep.len()];
        // Tiles under buildings and resources never take creep (retail
        // `flag_occupied`); the source's own footprint is seeded anyway.
        let mut occupied = vec![false; self.creep.len()];
        let footprint = |data: &GameData, u: &SimUnit| {
            let ty = data.unit(u.type_id);
            let (pw, ph) = (ty.placement.x as i32 / 32, ty.placement.y as i32 / 32);
            let (x0, y0) = (u.x / fp(1) / 32 - pw / 2, u.y / fp(1) / 32 - ph / 2);
            (x0, y0, pw, ph)
        };
        for u in &self.units {
            if !u.alive || !(self.data.unit(u.type_id).is_building() || is_resource_type(u.type_id))
            {
                continue;
            }
            let (x0, y0, pw, ph) = footprint(&self.data, u);
            for ty in y0.max(0)..(y0 + ph).min(h) {
                for tx in x0.max(0)..(x0 + pw).min(w) {
                    occupied[(ty * w + tx) as usize] = true;
                }
            }
        }
        for u in &self.units {
            if !u.alive || !u.completed || !is_creep_source(u.type_id) {
                continue;
            }
            // Retail `spread_creep_completely` (OpenBW): seed the footprint,
            // then grow 8-connected over tiles that can have creep, are not
            // occupied and lie in the ellipse 25 dx^2 + 64 dy^2 <= 25 * 320^2
            // (tile centre to building position) inside pos +- (320, 200).
            let (cx, cy) = (u.x / fp(1), u.y / fp(1));
            let (x0, y0, pw, ph) = footprint(&self.data, u);
            let mut stack = Vec::new();
            for ty in y0.max(0)..(y0 + ph).min(h) {
                for tx in x0.max(0)..(x0 + pw).min(w) {
                    let i = (ty * w + tx) as usize;
                    seed[i] = true;
                    if self.terrain.tile_can_have_creep(tx, ty) && !self.creep_supported[i] {
                        self.creep_supported[i] = true;
                        stack.push((tx, ty));
                    }
                }
            }
            let bx0 = ((cx - CREEP_RX).max(0)) / 32;
            let by0 = ((cy - CREEP_RY).max(0)) / 32;
            let bx1 = ((cx + CREEP_RX) / 32).min(w - 1);
            let by1 = ((cy + CREEP_RY) / 32).min(h - 1);
            let in_area = |tx: i32, ty: i32| {
                if tx < bx0 || ty < by0 || tx > bx1 || ty > by1 {
                    return false;
                }
                let dx = (tx * 32 + 16 - cx) as i64;
                let dy = (ty * 32 + 16 - cy) as i64;
                dx * dx * 25 + dy * dy * 64 <= (CREEP_RX as i64).pow(2) * 25
            };
            while let Some((tx, ty)) = stack.pop() {
                for (dx, dy) in [
                    (-1, -1),
                    (0, -1),
                    (1, -1),
                    (-1, 0),
                    (1, 0),
                    (-1, 1),
                    (0, 1),
                    (1, 1),
                ] {
                    let (nx, ny) = (tx + dx, ty + dy);
                    if !in_area(nx, ny) {
                        continue;
                    }
                    let i = (ny * w + nx) as usize;
                    if self.creep_supported[i]
                        || occupied[i]
                        || !self.terrain.tile_can_have_creep(nx, ny)
                    {
                        continue;
                    }
                    self.creep_supported[i] = true;
                    stack.push((nx, ny));
                }
            }
        }
        for (i, &is_seed) in seed.iter().enumerate() {
            if self.creep_supported[i] {
                if is_seed && self.creep_strength[i] == 0 {
                    self.creep_strength[i] = 3;
                }
            } else {
                // A source died: the tile recedes one step per rebuild.
                self.creep_strength[i] = self.creep_strength[i].saturating_sub(1);
            }
            self.creep[i] = self.creep_strength[i] > 0;
        }
        self.creep_dirty = false;
    }

    /// Spread/recede creep one maturity step.  Runs every
    /// `CREEP_GROW_INTERVAL` logical ticks.
    fn grow_creep(&mut self) {
        self.rebuild_creep();
        let w = self.terrain.width as i32;
        let h = self.terrain.height as i32;
        let mut next = self.creep_strength.clone();
        for tile_y in 0..h {
            for tile_x in 0..w {
                let i = (tile_y * w + tile_x) as usize;
                if !self.creep_supported[i] {
                    next[i] = self.creep_strength[i].saturating_sub(1);
                    continue;
                }
                if self.creep_strength[i] >= 3 {
                    continue;
                }
                let mut adjacent = false;
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = tile_x + dx;
                    let ny = tile_y + dy;
                    if nx >= 0
                        && ny >= 0
                        && nx < w
                        && ny < h
                        && self.creep_strength[(ny * w + nx) as usize] > 0
                    {
                        adjacent = true;
                        break;
                    }
                }
                if adjacent {
                    next[i] = (self.creep_strength[i] + 1).min(3);
                }
            }
        }
        self.creep_strength = next;
        for i in 0..self.creep.len() {
            self.creep[i] = self.creep_strength[i] > 0;
        }
    }

    pub fn has_creep_tile(&mut self, tx: i32, ty: i32) -> bool {
        self.rebuild_creep();
        let w = self.terrain.width as i32;
        let h = self.terrain.height as i32;
        if tx < 0 || ty < 0 || tx >= w || ty >= h {
            return false;
        }
        self.creep[(ty * w + tx) as usize]
    }

    /// Per-tile creep maturity (`0..=3`) for renderers.  Rebuilds the mask
    /// first so a source that appeared or died this tick is reflected without
    /// waiting for the next `CREEP_GROW_INTERVAL` boundary.
    /// Spread every source's creep to its full extent at once: a melee
    /// game starts with the start Hatchery's creep already grown (134905).
    pub fn mature_creep(&mut self) {
        self.creep_dirty = true;
        self.rebuild_creep();
        for i in 0..self.creep.len() {
            if self.creep_supported[i] {
                self.creep_strength[i] = 3;
                self.creep[i] = true;
            }
        }
    }

    pub fn creep_grid(&mut self) -> &[u8] {
        self.rebuild_creep();
        &self.creep_strength
    }

    /// Creep maturity at one tile, or zero outside the map.
    pub fn creep_strength_at(&mut self, tx: i32, ty: i32) -> u8 {
        self.rebuild_creep();
        let w = self.terrain.width as i32;
        let h = self.terrain.height as i32;
        if tx < 0 || ty < 0 || tx >= w || ty >= h {
            return 0;
        }
        self.creep_strength[(ty * w + tx) as usize]
    }

    pub fn has_creep_px(&mut self, x: i32, y: i32) -> bool {
        self.has_creep_tile(x.div_euclid(32), y.div_euclid(32))
    }

    /// Minimap dots for one observer: own units always, enemies only when the
    /// rules-level fog can currently see them.  Each unit is tested by its own
    /// index; using one shared index would leak every enemy once any unit was
    /// visible.
    pub fn minimap_units(&self, owner: u8, fog: bool) -> Vec<crate::render::MinimapDot> {
        self.units
            .iter()
            .enumerate()
            .filter(|(i, u)| u.alive && (u.owner == owner || !fog || self.can_see_unit(owner, *i)))
            .map(|(_, u)| {
                let ty = self.data.unit(u.type_id);
                crate::render::MinimapDot {
                    x: u.x / 256,
                    y: u.y / 256,
                    owner: u.owner,
                    placement: (ty.placement.x as i32, ty.placement.y as i32),
                    building: ty.is_building(),
                    resource: is_resource_type(u.type_id),
                    color: crate::render::player_rgb(self, u.owner),
                }
            })
            .collect()
    }

    /// Protoss power field: a point is powered by one of `owner`'s
    /// completed Pylons when it falls in that Pylon's psi field
    /// ([`psi_field_covers`]).  It was a 12 x 12 tile square that any
    /// player's Pylon could provide.
    pub fn has_power_px(&self, owner: u8, x: i32, y: i32) -> bool {
        self.units.iter().any(|u| {
            u.alive
                && u.completed
                && u.owner == owner
                && is_pylon_type(u.type_id)
                && psi_field_covers(x - u.x / fp(1), y - u.y / fp(1))
        })
    }

    /// A Protoss building functions only inside a power field.  Nexus and
    /// Pylon are exempt (they are the field providers themselves).
    pub fn building_powered(&self, i: usize) -> bool {
        let u = &self.units[i];
        if !is_protoss_building(u.type_id) || matches!(u.type_id, uid::NEXUS | uid::PYLON) {
            return true;
        }
        self.has_power_px(u.owner, u.x / fp(1), u.y / fp(1))
    }

    pub fn supply_used(&self, owner: u8) -> i32 {
        let mut used: i32 = self
            .units
            .iter()
            // Units inside a transport/bunker are hidden (`alive == false`)
            // but still consume supply.
            .filter(|u| (u.alive || u.in_transport.is_some()) && u.owner == owner)
            .map(|u| self.data.unit(u.type_id).supply_required as i32)
            .sum();
        // Only the unit in training counts; queued ones take their supply
        // when they start (FACT, Backwater Station recording 83.5-121.5 s:
        // four SCVs queued at once, 200 minerals paid at once, supply 6 -> 7
        // at once and +1 at each start, every 12.5-13.0 s).
        for u in &self.units {
            if u.alive
                && u.owner == owner
                && let Order::Train { unit_type } = u.order
            {
                used += self.data.unit(unit_type).supply_required as i32;
            }
            // A missile waiting in its silo is a unit too (8 supply).
            if u.alive && u.owner == owner && u.type_id == uid::NUCLEAR_SILO && u.nuke_ready {
                used += self.data.unit(uid::NUCLEAR_MISSILE).supply_required as i32;
            }
        }
        used
    }

    pub fn can_afford(&self, owner: u8, type_id: u16) -> bool {
        let p = &self.players[owner as usize];
        let ty = self.data.unit(type_id);
        p.minerals >= ty.mineral_cost as i32 && p.gas >= ty.gas_cost as i32
    }

    pub fn has_requirements(&self, owner: u8, type_id: u16) -> bool {
        let Some(info) = tree::build_info(type_id) else {
            return false;
        };
        let p = &self.players[owner as usize];
        if p.cheats.build_anything {
            return true;
        }
        if let Some(t) = info.required_tech
            && !p.has_tech(t)
        {
            return false;
        }
        let attached_addons: std::collections::HashSet<u32> =
            self.units.iter().filter_map(|u| u.addon).collect();
        info.required_units.iter().all(|&(required, count)| {
            self.units
                .iter()
                .filter(|u| {
                    u.alive
                        && u.owner == owner
                        && u.completed
                        && requirement_equivalent(required, u.type_id)
                        && (addon_parent(u.type_id).is_none() || attached_addons.contains(&u.id))
                })
                .count()
                >= count as usize
        })
    }

    pub fn train(&mut self, building_id: u32, unit_type: u16) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        let owner = self.units[i].owner;
        let building_type = self.units[i].type_id;
        if !self.units[i].completed || self.units[i].lifted {
            return Err(SimError::MissingRequirements);
        }
        if !self.building_powered(i) {
            return Err(SimError::Unpowered);
        }
        let info = tree::build_info(unit_type).ok_or(SimError::MissingRequirements)?;
        if info.producer != building_type {
            return Err(SimError::MissingRequirements);
        }
        if !self.has_requirements(owner, unit_type) {
            return Err(SimError::MissingRequirements);
        }
        if !self.unit_available(owner, unit_type) {
            return Err(SimError::MissingRequirements);
        }
        let ty = self.unit_stats(unit_type);
        {
            let p = &self.players[owner as usize];
            if p.minerals < ty.mineral_cost as i32 {
                return Err(SimError::NotEnoughMinerals);
            }
            if p.gas < ty.gas_cost as i32 {
                return Err(SimError::NotEnoughGas);
            }
        }
        let starts_now = self.units[i].production.is_empty() && self.units[i].order == Order::Idle;
        if starts_now
            && self.supply_used(owner) + ty.supply_required as i32 > self.supply_cap(owner)
        {
            return Err(SimError::NotEnoughSupply);
        }
        if self.units[i].production.len() >= MAX_PRODUCTION_QUEUE {
            return Err(SimError::QueueFull);
        }
        {
            let p = &mut self.players[owner as usize];
            p.pay_minerals(ty.mineral_cost as i32);
            p.pay_gas(ty.gas_cost as i32);
        }
        let order = Order::Train { unit_type };
        self.units[i].production.push_back(order);
        if self.units[i].order == Order::Idle {
            self.start_next_production(i);
        }
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::Train {
                    building: building_id,
                    unit: unit_type,
                },
            );
        }
        Ok(())
    }

    /// Morph a Zerg larva into a unit.  The larva becomes an egg and hatches
    /// after the target's build time.  Supply is checked and the cost is paid
    /// immediately, matching the retail build queue.
    /// Container unit shown while a Zerg morph is in progress.  Retail uses
    /// the generic Egg for a larva, the Lurker Egg for Hydralisk -> Lurker,
    /// and the Cocoon for Mutalisk -> Guardian/Devourer.
    fn morph_container(source: u16, target: u16) -> u16 {
        match (source, target) {
            (uid::HYDRALISK, uid::LURKER) => uid::LURKER_EGG,
            (uid::MUTALISK, uid::GUARDIAN | uid::DEVOURER) => uid::COCOON,
            _ => uid::EGG,
        }
    }

    pub fn morph(&mut self, larva_id: u32, unit_type: u16) -> Result<(), SimError> {
        let i = self.index_of(larva_id).ok_or(SimError::BadUnit)?;
        if self.units[i].type_id != uid::LARVA {
            return Err(SimError::MissingRequirements);
        }
        let info = tree::build_info(unit_type).ok_or(SimError::MissingRequirements)?;
        if info.producer != uid::LARVA {
            return Err(SimError::MissingRequirements);
        }
        let owner = self.units[i].owner;
        if !self.has_requirements(owner, unit_type) {
            return Err(SimError::MissingRequirements);
        }
        if !self.unit_available(owner, unit_type) {
            return Err(SimError::MissingRequirements);
        }
        let ty = self.unit_stats(unit_type);
        // Supply first, then the cost: a refused morph used to keep the
        // money, and an Overlord (no supply needed) was refused while supply
        // blocked, so a blocked Zerg could never recover (2026-11-06).
        if ty.supply_required > 0
            && self.supply_used(owner) + ty.supply_required as i32 > self.supply_cap(owner)
        {
            return Err(SimError::NotEnoughSupply);
        }
        {
            let p = &mut self.players[owner as usize];
            if p.minerals < ty.mineral_cost as i32 {
                return Err(SimError::NotEnoughMinerals);
            }
            if p.gas < ty.gas_cost as i32 {
                return Err(SimError::NotEnoughGas);
            }
            p.pay_minerals(ty.mineral_cost as i32);
            p.pay_gas(ty.gas_cost as i32);
        }
        let container = Self::morph_container(uid::LARVA, unit_type);
        self.units[i].type_id = container;
        self.units[i].morph_to = Some(unit_type);
        // Translation batch 13 (`0x005b0b4b`): a morph starts uncompleted.
        self.units[i].a13_clear_completed_flag();
        self.units[i].credited = false;
        self.units[i].build_total = build_ticks(ty.build_time);
        self.units[i].build_remaining = self.units[i].build_total;
        self.units[i].hp = self.data.unit(container).hit_points_raw as i32;
        self.units[i].order = Order::Idle;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::Morph {
                    larva: larva_id,
                    unit: unit_type,
                },
            );
        }
        Ok(())
    }

    /// All living unit ids owned by `owner`.
    pub fn unit_ids(&self, owner: u8) -> Vec<u32> {
        self.units
            .iter()
            .filter(|u| u.alive && u.owner == owner)
            .map(|u| u.id)
            .collect()
    }

    /// A player's placement: the building goes exactly where the cursor
    /// puts it or nowhere.  The units.dat placement box (`placement`, px) is
    /// snapped to the 32 px tile grid around `cursor` (retail placement
    /// preview), then checked like [`Self::find_build_spot`] without
    /// searching further.  Gas buildings need the cursor on a free geyser.
    /// FACT (2026-11-06): the client used `find_build_spot` for the player
    /// too, which moved a depot ~96 px away from the click.
    pub fn placement_spot(&mut self, type_id: u16, cursor: (i32, i32)) -> Option<(i32, i32)> {
        if is_gas_building(type_id) {
            let on_geyser = self.units.iter().any(|u| {
                u.alive
                    && u.type_id == uid::VESPENE_GEYSER
                    && (u.x / fp(1) - cursor.0).abs() <= 64
                    && (u.y / fp(1) - cursor.1).abs() <= 32
            });
            return on_geyser
                .then(|| self.find_build_spot(type_id, cursor))
                .flatten()
                .filter(|g| (g.0 - cursor.0).abs() <= 64 && (g.1 - cursor.1).abs() <= 32);
        }
        let ty = self.data.unit(type_id);
        let (pw, ph) = (ty.placement.x.max(32) as i32, ty.placement.y.max(32) as i32);
        let left = ((cursor.0 - pw / 2) as f32 / 32.0).round() as i32 * 32;
        let top = ((cursor.1 - ph / 2) as f32 / 32.0).round() as i32 * 32;
        let centre = (left + pw / 2, top + ph / 2);
        self.build_spot_search(type_id, centre, 1)
    }

    /// Placement preview at `cursor`: (snapped centre, placement box size
    /// in px, valid).  Same snapping as [`Self::placement_spot`].
    pub fn placement_preview(
        &mut self,
        type_id: u16,
        cursor: (i32, i32),
    ) -> ((i32, i32), (i32, i32), bool) {
        let ty = self.data.unit(type_id);
        let (pw, ph) = (ty.placement.x.max(32) as i32, ty.placement.y.max(32) as i32);
        let left = ((cursor.0 - pw / 2) as f32 / 32.0).round() as i32 * 32;
        let top = ((cursor.1 - ph / 2) as f32 / 32.0).round() as i32 * 32;
        let centre = (left + pw / 2, top + ph / 2);
        let spot = self.placement_spot(type_id, cursor);
        (spot.unwrap_or(centre), (pw, ph), spot.is_some())
    }

    /// Per-tile validity of a placement box at `cursor` (the build and Land
    /// previews): (snapped centre, box size px, row-major tiles, true =
    /// free).  A tile is free when its terrain is buildable and no structure
    /// or resource other than `ignore` overlaps it (FACT, Desperate Alliance
    /// recording 64 s: landing a Barracks, only the blocked tiles of the box
    /// turn red).
    pub fn placement_tiles(
        &self,
        type_id: u16,
        cursor: (i32, i32),
        ignore: Option<u32>,
    ) -> ((i32, i32), (i32, i32), Vec<bool>) {
        let ty = self.data.unit(type_id);
        let (pw, ph) = (ty.placement.x.max(32) as i32, ty.placement.y.max(32) as i32);
        let left = ((cursor.0 - pw / 2) as f32 / 32.0).round() as i32 * 32;
        let top = ((cursor.1 - ph / 2) as f32 / 32.0).round() as i32 * 32;
        let blockers: Vec<Rect> = self
            .units
            .iter()
            .filter(|u| u.alive && !u.lifted && Some(u.id) != ignore)
            .filter_map(|u| {
                let ut = self.data.unit(u.type_id);
                if !(ut.is_building() || is_resource_type(u.type_id)) {
                    return None;
                }
                let (ux, uy) = (u.x / fp(1), u.y / fp(1));
                Some(unit_box_rect(ut, (ux, uy)))
            })
            .collect();
        let mut tiles = Vec::new();
        for ty_ in 0..ph / 32 {
            for tx in 0..pw / 32 {
                let (x0, y0) = (left + tx * 32, top + ty_ * 32);
                let free = x0 >= 0
                    && y0 >= 0
                    && self.terrain.footprint_buildable(x0, y0, x0 + 32, y0 + 32)
                    && !blockers
                        .iter()
                        .any(|r| Rect::new(x0 + 1, y0 + 1, x0 + 31, y0 + 31).intersects(r));
                tiles.push(free);
            }
        }
        ((left + pw / 2, top + ph / 2), (pw, ph), tiles)
    }

    /// Find a buildable spot near `near` (pixel coordinates).
    pub fn find_build_spot(&mut self, type_id: u16, near: (i32, i32)) -> Option<(i32, i32)> {
        self.rebuild_creep();
        // Gas buildings are not free-standing: they are placed on a geyser
        // and occupy the geyser tile.
        if is_gas_building(type_id) {
            // The Extractor is the one Zerg structure exempt from creep
            // (it lives off the geyser itself).
            let zerg = is_zerg_building(type_id) && type_id != uid::EXTRACTOR;
            // The free geyser nearest to the requested spot (an under-construction
            // refinery already occupies its geyser).
            let geyser = self
                .units
                .iter()
                .filter(|u| u.alive && u.type_id == uid::VESPENE_GEYSER)
                .filter(|u| {
                    !self.units.iter().any(|b| {
                        b.alive
                            && is_gas_building(b.type_id)
                            && (b.x - u.x).abs() < fp(64)
                            && (b.y - u.y).abs() < fp(64)
                    })
                })
                .map(|u| (u.x / fp(1), u.y / fp(1)))
                .min_by_key(|g| (g.0 - near.0).pow(2) as i64 + (g.1 - near.1).pow(2) as i64);
            if let Some((gx, gy)) = geyser {
                if zerg && !self.has_creep_px(gx, gy) {
                    return None;
                }
                return Some((gx, gy));
            }
            return None;
        }
        self.build_spot_search(type_id, near, 20)
    }

    /// `0x00750b20`: search the candidate rectangle around `near` in 8 px
    /// steps for a free footprint, clamped to the retail map margins.
    /// `rings` is the Rust search radius in 32 px tiles.
    fn build_spot_search(
        &mut self,
        type_id: u16,
        near: (i32, i32),
        rings: i32,
    ) -> Option<(i32, i32)> {
        self.rebuild_creep();
        let ty = self.data.unit(type_id).clone();
        let half_w = ty.dimensions.right.max(0).max(ty.dimensions.left.max(0)) as i32;
        let half_h = ty.dimensions.bottom.max(0).max(ty.dimensions.top.max(0)) as i32;
        let need_creep = is_zerg_building(type_id) && type_id != uid::HATCHERY;
        let need_power =
            is_protoss_building(type_id) && !matches!(type_id, uid::NEXUS | uid::PYLON);
        // Cache the Pylon field and the blocking footprint list once instead
        // of rescanning every unit for each candidate position.
        let pylons: Vec<(i32, i32)> = if need_power {
            self.units
                .iter()
                .filter(|u| u.alive && u.completed && is_pylon_type(u.type_id))
                .map(|u| (u.x / fp(1), u.y / fp(1)))
                .collect()
        } else {
            Vec::new()
        };
        let blockers: Vec<Rect> = self
            .units
            .iter()
            // Unfinished structures block too (two depots could overlap).
            .filter(|u| u.alive && !u.lifted)
            .filter_map(|u| {
                let ut = self.data.unit(u.type_id);
                if !(ut.is_building() || is_resource_type(u.type_id)) {
                    return None;
                }
                let ux = u.x / fp(1);
                let uy = u.y / fp(1);
                Some(unit_box_rect(ut, (ux, uy)))
            })
            .collect();
        let step = 16;
        for ring in 0..rings {
            for dy in -ring..=ring {
                for dx in -ring..=ring {
                    if ring > 0 && dx.abs() != ring && dy.abs() != ring {
                        continue;
                    }
                    let x = near.0 + dx * step * 2;
                    let y = near.1 + dy * step * 2;
                    let left = x - half_w;
                    let top = y - half_h;
                    let right = x + half_w;
                    let bottom = y + half_h;
                    if left < 0 || top < 0 {
                        continue;
                    }
                    if !self.terrain.footprint_buildable(left, top, right, bottom) {
                        continue;
                    }
                    let rect = Rect::new(left, top, right, bottom);
                    if blockers.iter().any(|r| rect.intersects(r)) {
                        continue;
                    }
                    if need_creep && !self.has_creep_px(x, y) {
                        continue;
                    }
                    if need_power
                        && !pylons
                            .iter()
                            .any(|&(px, py)| (px - x).abs() <= 6 * 32 && (py - y).abs() <= 6 * 32)
                    {
                        continue;
                    }
                    return Some((x, y));
                }
            }
        }
        None
    }

    /// Build a Terran add-on on the right edge of its parent building.  The
    /// recovered build tree names the parent as the add-on's producer
    /// (Command Center → Comsat/Nuclear Silo, Factory → Machine Shop,
    /// Starport → Control Tower, Science Facility → Covert Ops/Physics Lab).
    /// Recorded mutator (`RecAct`): saves, replays and LAN replay it.
    pub fn build_addon(&mut self, parent_id: u32, addon_type: u16) -> Result<(), SimError> {
        self.build_addon_unrecorded(parent_id, addon_type)?;
        let tick = self.tick;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                tick,
                crate::replay::RecAct::Addon {
                    parent: parent_id,
                    unit: addon_type,
                },
            );
        }
        Ok(())
    }

    fn build_addon_unrecorded(&mut self, parent_id: u32, addon_type: u16) -> Result<(), SimError> {
        let p = self.index_of(parent_id).ok_or(SimError::BadUnit)?;
        if !self.units[p].alive || !self.units[p].completed || self.units[p].lifted {
            return Err(SimError::MissingRequirements);
        }
        if addon_parent(addon_type) != Some(self.units[p].type_id) {
            return Err(SimError::InvalidTarget);
        }
        if self.units[p].addon.is_some() {
            return Err(SimError::ProducerBusy);
        }
        let addon_owner = self.units[p].owner;
        if !self.has_requirements(addon_owner, addon_type) {
            return Err(SimError::MissingRequirements);
        }
        let owner = self.units[p].owner;
        let ty = self.unit_stats(addon_type);
        if self.players[owner as usize].minerals < ty.mineral_cost as i32 {
            return Err(SimError::NotEnoughMinerals);
        }
        if self.players[owner as usize].gas < ty.gas_cost as i32 {
            return Err(SimError::NotEnoughGas);
        }
        let parent_ty = self.data.unit(self.units[p].type_id).clone();
        let addon_hw = ty.dimensions.right.max(0).max(ty.dimensions.left.max(0)) as i32;
        let parent_hw = parent_ty
            .dimensions
            .right
            .max(0)
            .max(parent_ty.dimensions.left.max(0)) as i32;
        let px = self.units[p].x + (parent_hw + addon_hw).max(8) * fp(1);
        let py = self.units[p].y;
        if !self.terrain.footprint_buildable(
            px / fp(1) - addon_hw,
            py / fp(1) - addon_hw,
            px / fp(1) + addon_hw,
            py / fp(1) + addon_hw,
        ) {
            return Err(SimError::InvalidTarget);
        }
        {
            let pl = &mut self.players[owner as usize];
            pl.pay_minerals(ty.mineral_cost as i32);
            pl.pay_gas(ty.gas_cost as i32);
        }
        let idx = self.spawn_px(owner as usize, addon_type, px / fp(1), py / fp(1))?;
        self.units[idx].a13_clear_completed_flag();
        self.units[idx].credited = false;
        self.units[idx].build_total = build_ticks(ty.build_time);
        self.units[idx].build_remaining = self.units[idx].build_total;
        self.units[idx].hp = (ty.hit_points_raw as i32 / 10).max(1);
        let addon_id = self.units[idx].id;
        // Translation batch 14 (`0x005b66a0`): install the parent link.
        self.units[p].a14_set_addon_link(addon_id);
        self.units[idx].addon_attached = true;
        Ok(())
    }

    /// Morph one Zerg unit into another (Hydralisk → Lurker, Mutalisk →
    /// Guardian/Devourer).  The unit becomes an egg until the morph time
    /// elapses, exactly like the larva morph path.
    pub fn morph_unit(&mut self, unit_id: u32, target_type: u16) -> Result<(), SimError> {
        self.morph_unit_unrecorded(unit_id, target_type)?;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::MorphUnit {
                    unit: unit_id,
                    unit_type: target_type,
                },
            );
        }
        Ok(())
    }

    fn morph_unit_unrecorded(&mut self, unit_id: u32, target_type: u16) -> Result<(), SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        if !self.units[i].alive || !self.units[i].completed {
            return Err(SimError::MissingRequirements);
        }
        let source_type = self.units[i].type_id;
        let valid = matches!(
            (source_type, target_type),
            (uid::HYDRALISK, uid::LURKER)
                | (uid::MUTALISK, uid::GUARDIAN)
                | (uid::MUTALISK, uid::DEVOURER)
        );
        if !valid {
            return Err(SimError::InvalidTarget);
        }
        let container = Self::morph_container(source_type, target_type);
        let owner = self.units[i].owner;
        if !self.has_requirements(owner, target_type) {
            return Err(SimError::MissingRequirements);
        }
        let ty = self.unit_stats(target_type);
        {
            let p = &mut self.players[owner as usize];
            if p.minerals < ty.mineral_cost as i32 {
                return Err(SimError::NotEnoughMinerals);
            }
            if p.gas < ty.gas_cost as i32 {
                return Err(SimError::NotEnoughGas);
            }
            p.pay_minerals(ty.mineral_cost as i32);
            p.pay_gas(ty.gas_cost as i32);
        }
        self.units[i].type_id = container;
        self.units[i].morph_to = Some(target_type);
        self.units[i].a13_clear_completed_flag();
        self.units[i].credited = false;
        self.units[i].build_total = build_ticks(ty.build_time).max(1);
        self.units[i].build_remaining = self.units[i].build_total;
        self.units[i].hp = self.data.unit(container).hit_points_raw as i32;
        self.units[i].order = Order::Idle;
        Ok(())
    }

    /// Place one Spider Mine from a Vulture.  Rows 18/19/131 use this path;
    /// the Vulture carries three mines (`hangar`).
    pub fn place_spider_mine(&mut self, unit_id: u32, pos: (i32, i32)) -> Result<(), SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        if !matches!(self.units[i].type_id, uid::VULTURE | 19) {
            return Err(SimError::Unsupported);
        }
        let owner = self.units[i].owner;
        if !self.players[owner as usize].has_tech(tech_id::SPIDER_MINES) {
            return Err(SimError::MissingTech);
        }
        if self.units[i].hangar == 0 {
            return Err(SimError::QueueFull);
        }
        // The Vulture drives to the point and lays the mine there
        // (`order_PlaceMine`), like the Spider Mines button.
        self.issue(
            unit_id,
            Order::Cast {
                spell: Spell::SpiderMines,
                target: SpellTarget::Point(pos.0, pos.1),
            },
        )
    }

    /// Find an owned completed parent building with a free add-on slot.
    /// `0x005b5c70`: dynamic air bit (`CUnit+0xdc` bit 2).  True when the
    /// unit is in the air, either natively or as a lifted Terran building.
    pub fn is_air_unit(&self, i: usize) -> bool {
        self.units[i].lifted || self.data.unit(self.units[i].type_id).is_flyer()
    }

    /// Lift a Terran building off the ground.  Add-ons are left behind and
    /// destroyed, as in retail.
    pub fn lift_building(&mut self, building_id: u32) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        if !self.units[i].alive || !self.units[i].completed {
            return Err(SimError::MissingRequirements);
        }
        if !can_lift_building(self.units[i].type_id) {
            return Err(SimError::InvalidTarget);
        }
        if self.units[i].lifted {
            return Err(SimError::ProducerBusy);
        }
        if let Some(addon_id) = self.units[i].addon.take()
            && let Some(a) = self.index_of(addon_id)
        {
            // Add-ons stay grounded and become inactive, not destroyed.
            self.units[a].addon_attached = false;
        }
        self.units[i].lifted = true;
        self.units[i].order = Order::Idle;
        self.units[i].path.clear();
        self.units[i].path_goal = None;
        Ok(())
    }

    /// Land a lifted building at `pos` (pixels).  The footprint must be
    /// buildable and free of other structures/units.
    pub fn land_building(&mut self, building_id: u32, pos: (i32, i32)) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        if !self.units[i].alive || !self.units[i].lifted {
            return Err(SimError::InvalidTarget);
        }
        let ty = self.data.unit(self.units[i].type_id).clone();
        let hw = ty.dimensions.right.max(0).max(ty.dimensions.left.max(0)) as i32;
        let hh = ty.dimensions.bottom.max(0).max(ty.dimensions.top.max(0)) as i32;
        let left = pos.0 - hw;
        let top = pos.1 - hh;
        if left < 0
            || top < 0
            || !self
                .terrain
                .footprint_buildable(left, top, pos.0 + hw, pos.1 + hh)
        {
            return Err(SimError::InvalidTarget);
        }
        let rect = Rect::new(left, top, pos.0 + hw, pos.1 + hh);
        let blocked = self.units.iter().any(|u| {
            if !u.alive || u.id == building_id || !u.completed || u.lifted {
                return false;
            }
            let ut = self.data.unit(u.type_id);
            if !(ut.is_building() || is_resource_type(u.type_id)) {
                return false;
            }
            let ux = u.x / fp(1);
            let uy = u.y / fp(1);
            rect.intersects(&unit_box_rect(ut, (ux, uy)))
        });
        if blocked {
            return Err(SimError::InvalidTarget);
        }
        self.units[i].x = pos.0 * fp(1);
        self.units[i].y = pos.1 * fp(1);
        self.units[i].lifted = false;
        self.units[i].order = Order::Idle;
        self.units[i].path.clear();
        self.units[i].path_goal = None;
        // Landing on/near a grounded add-on of the right type re-attaches it.
        let parent_type = self.units[i].type_id;
        let (px, py) = (pos.0 * fp(1), pos.1 * fp(1));
        let found = self.units.iter().position(|u| {
            u.alive
                && u.completed
                && !u.addon_attached
                && addon_parent(u.type_id) == Some(parent_type)
                && (u.x - px).abs() <= fp(96)
                && (u.y - py).abs() <= fp(96)
        });
        if let Some(a) = found {
            self.units[a].addon_attached = true;
            let addon_id = self.units[a].id;
            self.units[i].a14_set_addon_link(addon_id);
        }
        Ok(())
    }

    pub fn addon_parent_unit(&self, owner: u8, addon: u16) -> Option<u32> {
        let parent = addon_parent(addon)?;
        self.units
            .iter()
            .find(|u| {
                u.alive
                    && u.completed
                    && u.owner == owner
                    && u.type_id == parent
                    && u.addon.is_none()
            })
            .map(|u| u.id)
    }

    /// Find the owned completed building that can morph into `target`.
    pub fn morph_source_unit(&self, owner: u8, target: u16) -> Option<u32> {
        let source = building_morph_source(target)?;
        self.units
            .iter()
            .find(|u| u.alive && u.completed && u.owner == owner && u.type_id == source)
            .map(|u| u.id)
    }

    /// Morph an existing Zerg building into its upgrade (Hatchery → Lair,
    /// Lair → Hive, Spire → Greater Spire, Creep Colony → Sunken/Spore
    /// Colony).  The building keeps its position and identity and becomes a
    /// construction site for the target type.
    pub fn morph_building(&mut self, building_id: u32, target_type: u16) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        if !self.units[i].alive || !self.units[i].completed {
            return Err(SimError::MissingRequirements);
        }
        if building_morph_source(target_type) != Some(self.units[i].type_id) {
            return Err(SimError::InvalidTarget);
        }
        let owner = self.units[i].owner;
        if !self.has_requirements(owner, target_type) {
            return Err(SimError::MissingRequirements);
        }
        let ty = self.unit_stats(target_type);
        {
            let p = &mut self.players[owner as usize];
            if p.minerals < ty.mineral_cost as i32 {
                return Err(SimError::NotEnoughMinerals);
            }
            if p.gas < ty.gas_cost as i32 {
                return Err(SimError::NotEnoughGas);
            }
            p.pay_minerals(ty.mineral_cost as i32);
            p.pay_gas(ty.gas_cost as i32);
        }
        let old_max = self.data.unit(self.units[i].type_id).hit_points_raw.max(1) as i64;
        let old_hp = self.units[i].hp.max(1) as i64;
        let new_max = ty.hit_points_raw.max(1) as i64;
        self.units[i].type_id = target_type;
        self.units[i].a13_clear_completed_flag();
        self.units[i].credited = false;
        self.units[i].build_total = build_ticks(ty.build_time);
        self.units[i].build_remaining = self.units[i].build_total;
        // Preserve the current health fraction across the morph.
        self.units[i].hp = ((new_max * old_hp) / old_max).clamp(1, new_max) as i32;
        self.units[i].order = Order::Idle;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::MorphBuilding {
                    source: building_id,
                    unit: target_type,
                },
            );
        }
        Ok(())
    }

    /// Recorded mutator (`RecAct`): saves, replays and LAN replay it.
    pub fn cancel_production(&mut self, building_id: u32) -> Result<(), SimError> {
        self.cancel_production_unrecorded(building_id)?;
        let tick = self.tick;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                tick,
                crate::replay::RecAct::Cancel {
                    building: building_id,
                },
            );
        }
        Ok(())
    }

    /// Cancel an unfinished building (the Cancel button of a structure
    /// under construction, `0x74d440`).  Recorded (`RecAct::CancelBuild`).
    /// Retail refunds 75 % of the cost (FACT: `stat_txt` / OpenBW
    /// `cancel_building_unit`), removes the structure without counting a
    /// loss, frees the Terran SCV and gives a Zerg Drone back.
    pub fn cancel_construction(&mut self, building_id: u32) -> Result<(), SimError> {
        self.cancel_construction_unrecorded(building_id)?;
        let tick = self.tick;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                tick,
                crate::replay::RecAct::CancelBuild {
                    building: building_id,
                },
            );
        }
        Ok(())
    }

    /// Cancel the add-on being built on `building_id` (the Cancel button of
    /// a Command Center / Factory / Starport / Science Facility).  Recorded
    /// (`RecAct::CancelAddon`, save tag 18).
    pub fn cancel_addon(&mut self, building_id: u32) -> Result<(), SimError> {
        self.cancel_addon_unrecorded(building_id)?;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::CancelAddon {
                    building: building_id,
                },
            );
        }
        Ok(())
    }

    fn cancel_addon_unrecorded(&mut self, building_id: u32) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        let addon = self.units[i].addon.ok_or(SimError::InvalidTarget)?;
        self.cancel_construction_unrecorded(addon)?;
        self.units[i].addon = None;
        Ok(())
    }

    /// Cancel an in-progress Zerg morph: an egg returns to its larva or the
    /// original unit, and an upgrading building returns to its source type.
    /// Recorded (`RecAct::CancelMorph`, save tag 19).
    pub fn cancel_morph(&mut self, building_id: u32) -> Result<(), SimError> {
        self.cancel_morph_unrecorded(building_id)?;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::CancelMorph {
                    building: building_id,
                },
            );
        }
        Ok(())
    }

    fn cancel_morph_unrecorded(&mut self, unit_id: u32) -> Result<(), SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        if !self.units[i].alive {
            return Err(SimError::BadUnit);
        }
        let target = self.units[i].morph_to.ok_or(SimError::Unsupported)?;
        let source = match target {
            uid::LURKER => uid::HYDRALISK,
            uid::GUARDIAN | uid::DEVOURER => uid::MUTALISK,
            _ => building_morph_source(target).unwrap_or(uid::LARVA),
        };
        let ty = self.data.unit(target).clone();
        let source_ty = self.data.unit(source).clone();
        let owner = self.units[i].owner as usize;
        if owner < self.players.len() {
            self.players[owner].minerals += ty.mineral_cost as i32 * 3 / 4;
            self.players[owner].gas += ty.gas_cost as i32 * 3 / 4;
        }
        self.units[i].type_id = source;
        self.units[i].completed = true;
        self.units[i].credited = true;
        self.units[i].morph_to = None;
        self.units[i].build_remaining = 0;
        self.units[i].build_total = 0;
        self.units[i].hp = source_ty.hit_points_raw as i32;
        self.units[i].shields = if source_ty.has_shields {
            fp(source_ty.shield_points as i32)
        } else {
            0
        };
        self.units[i].order = Order::Idle;
        self.units[i].order_row = 0;
        Ok(())
    }

    fn cancel_construction_unrecorded(&mut self, building_id: u32) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        let u = &self.units[i];
        if !u.alive || u.completed || u.morph_to.is_some() {
            return Err(SimError::Unsupported);
        }
        let type_id = u.type_id;
        let ty = self.data.unit(type_id).clone();
        if !ty.is_building() {
            return Err(SimError::Unsupported);
        }
        let owner = u.owner as usize;
        let (x, y) = (u.x / fp(1), u.y / fp(1));
        let builder = u.builder;
        if owner < self.players.len() {
            self.players[owner].minerals += ty.mineral_cost as i32 * 3 / 4;
            self.players[owner].gas += ty.gas_cost as i32 * 3 / 4;
        }
        self.units[i].alive = false;
        self.units[i].order = Order::Die;
        // `0x005b02df`: cancel clears both per-unit masks before the
        // construction is removed.
        self.units[i].clear_mask_121();
        self.units[i].clear_mask_123();
        if let Some(b) = self.index_of(builder)
            && matches!(self.units[b].order, Order::Construct { building } if building == building_id)
        {
            self.units[b].order = Order::Idle;
            self.units[b].order_row = 0;
        }
        if is_zerg_building(type_id) {
            let _ = self.spawn_px(owner, uid::DRONE, x, y);
        }
        Ok(())
    }

    /// `0x0058dcb9`: clear the active build unit and queue state.  The Rust
    /// path also refunds the current block (train/research/upgrade).
    fn cancel_production_unrecorded(&mut self, building_id: u32) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        let owner = self.units[i].owner as usize;
        // `0x5d9f80` (research) and `0x5da010` (upgrade) refund the full
        // current-level price.  Train keeps the pre-existing full refund.
        let (minerals, gas, resume_queue) = match self.units[i].order.clone() {
            Order::Train { unit_type } => {
                let ty = self.data.unit(unit_type);
                (ty.mineral_cost as i32, ty.gas_cost as i32, true)
            }
            Order::Research { tech_id } => {
                let t = self.data.tech(tech_id).ok_or(SimError::BadUnit)?;
                (t.mineral_cost as i32, t.gas_cost as i32, false)
            }
            Order::Upgrade { upgrade_id, .. } => {
                let up = self.data.upgrade(upgrade_id).ok_or(SimError::BadUnit)?;
                let level = self.players[owner].upgrade_level(upgrade_id);
                (
                    up.mineral_cost(level) as i32,
                    up.gas_cost(level) as i32,
                    false,
                )
            }
            Order::Idle => return Ok(()),
            _ => return Err(SimError::Unsupported),
        };
        self.units[i].order = Order::Idle;
        self.units[i].build_remaining = 0;
        self.units[i].build_total = 0;
        self.players[owner].minerals += minerals;
        self.players[owner].gas += gas;
        if resume_queue {
            self.start_next_production(i);
        }
        Ok(())
    }

    /// Recorded mutator (`RecAct::Dequeue`): refund and remove the newest
    /// queued production item.  Returns false when the queue is empty; the
    /// item currently being built is untouched.
    pub fn dequeue_production(&mut self, building_id: u32) -> Result<bool, SimError> {
        let removed = self.dequeue_production_unrecorded(building_id)?;
        if removed {
            let tick = self.tick;
            if let Some(r) = self.recorder.as_mut() {
                r.record_econ(
                    tick,
                    crate::replay::RecAct::Dequeue {
                        building: building_id,
                    },
                );
            }
        }
        Ok(removed)
    }

    fn dequeue_production_unrecorded(&mut self, building_id: u32) -> Result<bool, SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        let owner = self.units[i].owner as usize;
        let Some(order) = self.units[i].production.pop_back() else {
            return Ok(false);
        };
        let (minerals, gas) = match order {
            Order::Train { unit_type } => {
                let ty = self.data.unit(unit_type);
                (ty.mineral_cost as i32, ty.gas_cost as i32)
            }
            _ => (0, 0),
        };
        self.players[owner].minerals += minerals;
        self.players[owner].gas += gas;
        Ok(true)
    }

    /// Recorded mutator (`RecAct`): saves, replays and LAN replay it.
    pub fn research(&mut self, building_id: u32, tech_id: u8) -> Result<(), SimError> {
        self.research_unrecorded(building_id, tech_id)?;
        let tick = self.tick;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                tick,
                crate::replay::RecAct::Research {
                    building: building_id,
                    tech: tech_id,
                },
            );
        }
        Ok(())
    }

    fn research_unrecorded(&mut self, building_id: u32, tech_id: u8) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        let owner = self.units[i].owner;
        if self.players[owner as usize].cheats.free_tech {
            self.players[owner as usize].techs[tech_id as usize] = true;
            return Ok(());
        }
        if !self.units[i].completed || self.units[i].lifted {
            return Err(SimError::MissingRequirements);
        }
        if !self.building_powered(i) {
            return Err(SimError::Unpowered);
        }
        if !tree::tech_buildings(tech_id)
            .iter()
            .any(|&b| requirement_equivalent(b, self.units[i].type_id))
        {
            return Err(SimError::MissingRequirements);
        }
        if !self.tech_available(owner, tech_id) {
            return Err(SimError::MissingTech);
        }
        if self.players[owner as usize].has_tech(tech_id) {
            return Err(SimError::MissingTech);
        }
        let t = self.tech_stats(tech_id).ok_or(SimError::BadUnit)?;
        {
            let p = &mut self.players[owner as usize];
            if p.minerals < t.mineral_cost as i32 {
                return Err(SimError::NotEnoughMinerals);
            }
            if p.gas < t.gas_cost as i32 {
                return Err(SimError::NotEnoughGas);
            }
            p.pay_minerals(t.mineral_cost as i32);
            p.pay_gas(t.gas_cost as i32);
        }
        if self.units[i].order != Order::Idle {
            return Err(SimError::ProducerBusy);
        }
        self.units[i].build_remaining = research_ticks(t.research_time);
        self.units[i].build_total = self.units[i].build_remaining;
        self.units[i].order = Order::Research { tech_id };
        Ok(())
    }

    /// Recorded mutator (`RecAct`): saves, replays and LAN replay it.
    pub fn upgrade(&mut self, building_id: u32, upgrade_id: u8) -> Result<(), SimError> {
        self.upgrade_unrecorded(building_id, upgrade_id)?;
        let tick = self.tick;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                tick,
                crate::replay::RecAct::Upgrade {
                    building: building_id,
                    upgrade: upgrade_id,
                },
            );
        }
        Ok(())
    }

    /// `0x005da550`: begin an upgrade.  Retail checks the producer, its
    /// requirements and the player's resources, writes the upgrade id to
    /// `CUnit+0xc8` and the duration to `CUnit+0xc6`; the Rust order carries
    /// the same target level.
    fn upgrade_unrecorded(&mut self, building_id: u32, upgrade_id: u8) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        let owner = self.units[i].owner;
        if self.players[owner as usize].cheats.free_upgrades {
            let up = self
                .data
                .upgrade(upgrade_id)
                .ok_or(SimError::BadUnit)?
                .clone();
            let level = &mut self.players[owner as usize].upgrades[upgrade_id as usize];
            if *level < up.max_level {
                *level += 1;
            }
            return Ok(());
        }
        if !self.units[i].completed || self.units[i].lifted {
            return Err(SimError::MissingRequirements);
        }
        if !self.building_powered(i) {
            return Err(SimError::Unpowered);
        }
        if !tree::upgrade_buildings(upgrade_id)
            .iter()
            .any(|&b| requirement_equivalent(b, self.units[i].type_id))
        {
            return Err(SimError::MissingRequirements);
        }
        let up = self.upgrade_stats(upgrade_id).ok_or(SimError::BadUnit)?;
        let level = self.players[owner as usize].upgrade_level(upgrade_id);
        if level >= self.upgrade_max_level(owner, upgrade_id) {
            return Err(SimError::MissingTech);
        }
        let next = level + 1;
        let extra = tree::upgrade_level_requirement(upgrade_id, level);
        for required in extra {
            if !self
                .units
                .iter()
                .any(|u| u.alive && u.owner == owner && u.completed && u.type_id == *required)
            {
                return Err(SimError::MissingRequirements);
            }
        }
        if self.units[i].order != Order::Idle {
            return Err(SimError::ProducerBusy);
        }
        {
            let p = &mut self.players[owner as usize];
            // `0x5da5d0` charges base + factor * current_level; the
            // target level is stored separately in the order.
            let m = up.mineral_cost(level);
            let g = up.gas_cost(level);
            if p.minerals < m as i32 {
                return Err(SimError::NotEnoughMinerals);
            }
            if p.gas < g as i32 {
                return Err(SimError::NotEnoughGas);
            }
            p.pay_minerals(m as i32);
            p.pay_gas(g as i32);
        }
        self.units[i].build_remaining = upgrade_ticks(up.time_cost(level));
        self.units[i].build_total = self.units[i].build_remaining;
        self.units[i].order = Order::Upgrade {
            upgrade_id,
            level: next,
        };
        Ok(())
    }

    fn start_next_production(&mut self, i: usize) {
        // A queued unit starts only when its supply is free; otherwise it
        // waits at the front of the queue (retried every frame).
        if let Some(Order::Train { unit_type }) = self.units[i].production.front() {
            let owner = self.units[i].owner;
            let need = self.unit_stats(*unit_type).supply_required as i32;
            if need > 0 && self.supply_used(owner) + need > self.supply_cap(owner) {
                return;
            }
        }
        let next = self.units[i].production.pop_front();
        if let Some(Order::Train { unit_type }) = next {
            let ty = self.unit_stats(unit_type);
            self.units[i].build_remaining = build_ticks(ty.build_time);
            self.units[i].build_total = self.units[i].build_remaining;
            self.units[i].order = Order::Train { unit_type };
        }
    }

    pub fn issue(&mut self, unit_id: u32, new_order: Order) -> Result<(), SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        if !self.units[i].alive {
            return Err(SimError::BadUnit);
        }
        // A tank in the middle of its siege / unsiege animation takes no
        // orders (retail `status_flag_order_not_interruptible`).
        if self.is_transforming(i) {
            return Err(SimError::ProducerBusy);
        }
        // A new order drops whatever the old one was counting (a Yamato
        // charge, an unload in progress).
        self.units[i].order_wait = 0;
        self.cancel_nuke_for(unit_id);
        match &new_order {
            Order::AttackUnit { target } => {
                if self.unit(*target).is_none() {
                    return Err(SimError::InvalidTarget);
                }
            }
            Order::Gather { resource } => {
                let target = self.unit(*resource).ok_or(SimError::InvalidTarget)?;
                if !target.is_resource() && !is_gas_building(target.type_id) {
                    return Err(SimError::InvalidTarget);
                }
            }
            _ => {}
        }
        if matches!(new_order, Order::AttackUnit { .. }) && self.units[i].type_id == uid::CARRIER {
            // Carrier Attack row 20 starts with a 60-tick launch phase
            // (`CUnit+0x55 = 0x3c`); the Pulse Cannon cycle follows it.
            self.units[i].special_cooldown = 0x3c;
        }
        self.units[i].order = new_order;
        self.units[i].order_row = self.units[i].order.order_id();
        self.units[i].resume_order = None;
        self.units[i].target = None;
        self.units[i].path.clear();
        self.units[i].path_goal = None;
        if !matches!(self.units[i].order, Order::Idle | Order::HoldPosition) {
            self.units[i].queue.clear();
        }
        // Record successful commands for save games / replays / LAN sync.
        if self.recorder.is_some() {
            let tick = self.tick;
            let logged = self.units[i].order.clone();
            if let Some(r) = self.recorder.as_mut() {
                r.cmds.push(crate::replay::RecCmd {
                    tick,
                    act: crate::replay::RecAct::Issue {
                        unit: unit_id,
                        order: logged,
                    },
                });
            }
        }
        Ok(())
    }

    /// Issue any of the 189 `orders.dat` rows by id.  This is the front
    /// door to the recovered handler table: every row with a direct case in
    /// `FUN_7a178340` calls its own transcribed Rust routine in
    /// [`crate::handlers`], and rows resolved by an alias, a production timer
    /// or the engine fall back to the typed state machine.
    pub fn issue_order_id(
        &mut self,
        unit_id: u32,
        order_id: u8,
        target: Option<u32>,
        pos: Option<(i32, i32)>,
    ) -> Result<(), SimError> {
        crate::handlers::dispatch_row(self, unit_id, order_id, target, pos)
    }

    /// Apply a row through the typed order state machine.  This is the body
    /// called by the transcribed handler routines and by rows that are
    /// resolved before the recovered switch.
    pub(crate) fn apply_order_row_typed(
        &mut self,
        unit_id: u32,
        order_id: u8,
        target: Option<u32>,
        pos: Option<(i32, i32)>,
    ) -> Result<(), SimError> {
        let result = self.apply_order_row_typed_inner(unit_id, order_id, target, pos);
        if result.is_ok()
            && let Some(i) = self.index_of(unit_id)
        {
            self.units[i].order_row = order_id;
        }
        result
    }

    fn apply_order_row_typed_inner(
        &mut self,
        unit_id: u32,
        order_id: u8,
        target: Option<u32>,
        pos: Option<(i32, i32)>,
    ) -> Result<(), SimError> {
        // Internal-but-player-visible rows with a concrete API.
        match order_id {
            // Critters / Junk Yard Dog wander around their current point.
            154 | 155 | 178 => {
                let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
                let a = (self.units[i].x / fp(1), self.units[i].y / fp(1));
                let dx = (self.world_rand() % 128) as i32 - 64;
                let dy = (self.world_rand() % 128) as i32 - 64;
                return self.issue(
                    unit_id,
                    Order::Patrol {
                        a,
                        b: (a.0 + dx, a.1 + dy),
                        forward: true,
                    },
                );
            }
            // AI harass uses attack-move; guard/tower/passive rows hold.
            157 => {
                let p = pos.ok_or(SimError::Unsupported)?;
                return self.issue(unit_id, Order::AttackMove { pos: p });
            }
            156 | 159 | 160 | 161 | 163 | 166 | 61 => {
                return self.issue(unit_id, Order::HoldPosition);
            }
            // Computer Return heads for the owner's town hall.
            162 => {
                let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
                let owner = self.units[i].owner;
                let home = self
                    .units
                    .iter()
                    .find(|u| {
                        u.alive
                            && u.completed
                            && u.owner == owner
                            && matches!(
                                u.type_id,
                                uid::COMMAND_CENTER
                                    | uid::NEXUS
                                    | uid::HATCHERY
                                    | uid::LAIR
                                    | uid::HIVE
                            )
                    })
                    .map(|u| (u.x / fp(1), u.y / fp(1)));
                return match home {
                    Some(p) => self.issue(unit_id, Order::Move { pos: p }),
                    None => self.issue(unit_id, Order::Stop),
                };
            }
            // Self-destructing rows (Scourge-style campaign units).
            164 | 174 => {
                let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
                let hp = self.units[i].hp;
                self.damage_unit(unit_id, hp)?;
                return Ok(());
            }
            // Reaver/Scarab combat rows.
            56 | 58 | 59 | 60 | 64 | 65 => {
                if let Some(t) = target {
                    return self.issue(unit_id, Order::AttackUnit { target: t });
                }
                let p = pos.ok_or(SimError::Unsupported)?;
                return self.issue(unit_id, Order::AttackMove { pos: p });
            }
            18 | 19 | 131 => {
                let p = pos.ok_or(SimError::Unsupported)?;
                self.place_spider_mine(unit_id, p)?;
                return Ok(());
            }
            49 | 62 => {
                self.build_hangar_unit(unit_id)?;
                return Ok(());
            }
            51 | 52 | 55 => return self.issue(unit_id, Order::Stop),
            68 => {
                let p = pos.ok_or(SimError::Unsupported)?;
                let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
                self.units[i].rally = Some(p);
                return Ok(());
            }
            104 | 105 => {
                return self.cast(unit_id, Spell::ArchonWarp, SpellTarget::None);
            }
            183 => return self.cast(unit_id, Spell::DarkArchonMeld, SpellTarget::None),
            138 => {
                let target = pos
                    .map(|(x, y)| SpellTarget::Point(x, y))
                    .unwrap_or(SpellTarget::None);
                return self.cast(unit_id, Spell::ScannerSweep, target);
            }
            _ => {}
        }
        // Rows whose behaviour is a direct ability call rather than a typed
        // order state.  These are the shipped cast/cloak/shield rows.
        if let Some(spell) = row_spell(order_id) {
            let target = match target {
                Some(t) => SpellTarget::Unit(t),
                None => match pos {
                    Some((x, y)) => SpellTarget::Point(x, y),
                    None => SpellTarget::None,
                },
            };
            // Parameter rows must fail loudly when the replay/AI omitted
            // their target instead of falling through to a no-op cast.
            if spell.takes_unit_target() && !matches!(target, SpellTarget::Unit(_)) {
                return Err(SimError::Unsupported);
            }
            return self.cast(unit_id, spell, target);
        }
        match order_id {
            57 | 107 | 108 | 136 => return self.set_cloak(unit_id, true),
            109 => return self.set_cloak(unit_id, false),
            66 | 67 => {
                let t = target.ok_or(SimError::Unsupported)?;
                self.shield_battery(unit_id, t)?;
                return Ok(());
            }
            124 | 127 => {
                let p = pos.ok_or(SimError::Unsupported)?;
                return self.launch_nuke(unit_id, p);
            }
            126 => {
                let t = target.ok_or(SimError::Unsupported)?;
                let u = self.unit(t).ok_or(SimError::InvalidTarget)?;
                let p = (u.x / fp(1), u.y / fp(1));
                return self.launch_nuke(unit_id, p);
            }
            _ => {}
        }
        // Terran lift/land rows (69-73) have their own typed states.
        match order_id {
            71..=73 => return self.issue(unit_id, Order::Lift),
            69 | 70 => {
                let p = pos.ok_or(SimError::Unsupported)?;
                return self.issue(unit_id, Order::Land { pos: p });
            }
            _ => {}
        }
        let order = match order_class(order_id) {
            OrderClass::Idle => Order::Stop,
            OrderClass::Move => Order::Move {
                pos: pos.ok_or(SimError::Unsupported)?,
            },
            OrderClass::Attack => match target {
                Some(t) => Order::AttackUnit { target: t },
                None => Order::AttackMove {
                    pos: pos.ok_or(SimError::Unsupported)?,
                },
            },
            OrderClass::AttackMove => Order::AttackMove {
                pos: pos.ok_or(SimError::Unsupported)?,
            },
            OrderClass::Hold => Order::HoldPosition,
            OrderClass::Patrol => {
                let a = pos.ok_or(SimError::Unsupported)?;
                Order::Patrol {
                    a,
                    b: a,
                    forward: true,
                }
            }
            OrderClass::Harvest => Order::Gather {
                resource: target.ok_or(SimError::Unsupported)?,
            },
            OrderClass::ReturnCargo => Order::ReturnCargo {
                depot: target.ok_or(SimError::Unsupported)?,
            },
            OrderClass::Build => return Err(SimError::Unsupported),
            OrderClass::Train => return Err(SimError::Unsupported),
            OrderClass::Research => return Err(SimError::Unsupported),
            OrderClass::Upgrade => return Err(SimError::Unsupported),
            OrderClass::Cast => return Err(SimError::Unsupported),
            OrderClass::Transport => Order::EnterTransport {
                transport: target.ok_or(SimError::Unsupported)?,
            },
            OrderClass::Unload => Order::Unload,
            OrderClass::Burrow => Order::Burrow,
            OrderClass::Siege => Order::Siege,
            OrderClass::Morph => return Err(SimError::Unsupported),
            OrderClass::Special => Order::Stop,
        };
        self.issue(unit_id, order)
    }

    pub fn stop(&mut self, unit_id: u32) {
        if let Some(i) = self.index_of(unit_id) {
            self.units[i].order = Order::Stop;
            self.units[i].resume_order = None;
            self.units[i].queue.clear();
            self.units[i].path.clear();
            self.units[i].path_goal = None;
            self.units[i].target = None;
        }
    }

    pub fn queue_order(&mut self, unit_id: u32, o: Order) -> Result<(), SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        // `orders.dat` uses two independent bits here: `can_be_queued`
        // decides replace-vs-queue, while a non-`-1` highlight decides
        // whether the order consumes one of the eight queue slots.
        let row = self.data.order(o.order_id());
        let can_queue = row.map(|r| r.can_be_queued).unwrap_or(false);
        let uses_slot = row.map(|r| r.highlight != -1).unwrap_or(false);
        if can_queue && uses_slot && self.units[i].queue.len() >= MAX_ORDER_QUEUE {
            return Err(SimError::QueueFull);
        }
        if self.units[i].order == Order::Idle || !can_queue {
            self.units[i].order_row = o.order_id();
            self.units[i].order = o;
            if !can_queue {
                self.units[i].queue.clear();
            }
        } else {
            self.units[i].queue.push_back(o);
        }
        Ok(())
    }

    /// Recorded mutator (`RecAct`): append a waypoint order without
    /// replacing the current one.  Used by Shift/LT right-click.
    pub fn queue_order_recorded(&mut self, unit_id: u32, o: Order) -> Result<(), SimError> {
        self.queue_order(unit_id, o.clone())?;
        if let Some(r) = self.recorder.as_mut() {
            r.cmds.push(crate::replay::RecCmd {
                tick: self.tick,
                act: crate::replay::RecAct::QueueOrder {
                    unit: unit_id,
                    order: o,
                },
            });
        }
        Ok(())
    }

    /// Recorded mutator (`RecAct`): saves, replays and LAN replay it.
    pub fn set_rally(&mut self, building_id: u32, pos: (i32, i32)) -> Result<(), SimError> {
        self.set_rally_unrecorded(building_id, pos)?;
        let tick = self.tick;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                tick,
                crate::replay::RecAct::Rally {
                    building: building_id,
                    pos,
                },
            );
        }
        Ok(())
    }

    fn set_rally_unrecorded(&mut self, building_id: u32, pos: (i32, i32)) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        // `0x005b6030`: only production/town-hall structures own a rally.
        if !is_production_building(self.units[i].type_id) {
            return Err(SimError::MissingRequirements);
        }
        self.units[i].rally = Some(pos);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Tick
    // ------------------------------------------------------------------

    pub fn step(&mut self) {
        // The world keeps running after Victory / Defeat: retail units move
        // behind the end dialog (Wasteland recording 302.5-307.5 s) until
        // the player leaves for the score screen.
        self.tick += 1;
        for u in &mut self.units {
            if !u.alive && u.death_timer > 0 {
                u.death_timer -= 1;
            }
        }
        self.matrix_cues.clear();
        self.nuke_warnings.clear();
        let n = self.units.len();
        for i in 0..n {
            if !self.units[i].alive {
                continue;
            }
            self.update_timers(i);
            self.update_construction(i);
            self.update_production(i);
        }
        for i in 0..n {
            if !self.units[i].alive {
                continue;
            }
            self.tick_animation(i);
            if !self.units[i].alive {
                continue;
            }
            let order = self.units[i].order.clone();
            // Recovered `0x7a178340` dispatch cadence: the carry counter is
            // decremented every tick and reset to 8 when it reaches zero, so
            // the carry-gated handlers run on ticks 0, 8, 16, ... per unit.
            let carry = self.units[i].order_carry;
            // Translation batch 35 (`0x005d834c`): decrement/reset carry.
            self.units[i].a35_advance_order_carry();
            let think = order_runs_this_tick(order.order_id(), carry);
            if think {
                // Enter the recovered handler routine for this row.  The
                // typed order state machine below remains the gameplay
                // authority; this is the recovered dispatcher boundary and
                // records the hit in `World::handler_hits`.
                crate::handlers::tick_row(self, self.units[i].id, order.order_id());
            }
            if !matches!(order, Order::Die) {
                self.execute_order(i, order, think);
            }
        }
        self.resolve_collisions();
        self.apply_air_repulse();
        self.turn_turrets();
        if self.tick.is_multiple_of(CREEP_GROW_INTERVAL as u64) {
            self.grow_creep();
        }
        self.update_specials();
        self.update_pending_hits();
        self.update_projectiles();
        self.advance_nuke_strikes();
        self.update_larva();
        self.update_vision();
        self.resolve_deaths();
        // The first check runs on the first tick (the order's timer starts
        // at 0), before the triggers: Backwater Station's "Glad to see you,
        // boys" (needs Raynor rescued) shows on the first game frame of the
        // retail recording.  Then every 15 ticks.
        if (self.tick - 1).is_multiple_of(RESCUE_INTERVAL) {
            self.update_rescues();
        }
        self.credit_scores();
        self.update_ctf();
        self.check_victory();
        if self.triggers.is_some() {
            let mut triggers = self.triggers.take();
            if let Some(t) = triggers.as_mut() {
                t.process(self);
            }
            self.triggers = triggers;
        }
    }

    fn update_timers(&mut self, i: usize) {
        let ty = self.data.unit(self.units[i].type_id).clone();
        if self.units[i].status.stasis_ticks > 0 {
            // Stasis freezes the unit; its own timer still runs on the
            // recovered eighth-tick status cadence.
            if self.tick.is_multiple_of(ACID_SPORE_UPDATE_INTERVAL) {
                self.units[i].status.stasis_ticks -= 1;
            }
            return;
        }
        self.units[i].anim_tick = self.units[i].anim_tick.wrapping_add(1);
        if self.units[i].cooldown > 0 {
            self.units[i].cooldown -= 1;
        }
        self.units[i].tick_speech();
        self.units[i].tick_timer_087();
        // `0x005d91e0`: the recharge flag only survives the tick that set it.
        self.units[i].shield_recharged = false;
        if ty.has_shields {
            let max = fp(ty.shield_points as i32);
            if self.units[i].shields < max {
                self.units[i].shields = (self.units[i].shields + SHIELD_REGEN_PER_TICK).min(max);
            }
        }
        if ty.regenerates() {
            let max = ty.hit_points_raw as i32;
            if self.units[i].hp < max && self.units[i].hp > 0 {
                self.units[i].hp = (self.units[i].hp + ZERG_REGEN_PER_TICK).min(max);
            }
        }
        // Terran structures below one third HP burn at the recovered rate of
        // 20/256 HP per frame (1.875 HP/s at 24 Hz).
        if self.units[i].completed && is_terran_building(self.units[i].type_id) {
            let max = ty.hit_points_raw as i32;
            if self.units[i].hp > 0 && self.units[i].hp * 3 < max {
                self.units[i].hp -= 20;
                if self.units[i].hp <= 0 {
                    self.kill_unit(i);
                    return;
                }
            }
        }
        if self.units[i].special_cooldown > 0 {
            self.units[i].special_cooldown -= 1;
        }
        // `0x005d93a0` decrements CUnit+0x110 and kills the temporary unit
        // when it reaches zero (Dark Swarm, Disruption Web, Scanner Sweep,
        // Nuclear Missile).
        if self.units[i].expires_at > 0 && self.tick as i64 >= self.units[i].expires_at {
            self.kill_unit(i);
            return;
        }
        if ty.flags & 0x0020_0000 != 0 {
            // Personal cloaking drains 10 (Ghost) or 13 (Wraith) energy in
            // 1/256 points per tick and suppresses the +8 regeneration, for
            // the documented net 18/256 and 21/256.  Every other energy user
            // regenerates 8/256 per tick up to its maximum.
            let cloak_drain = if self.units[i].status.cloaked && !self.units[i].status.hallucination
            {
                match self.units[i].type_id {
                    // OpenBW `unit_cloak_energy_cost`: the Ghost heroes
                    // (and Infested Kerrigan, 51) pay 10, Tom Kazansky's
                    // Wraith (21) pays 13.
                    t if is_ghost_type(t) || t == 51 => GHOST_CLOAK_COST_PER_TICK,
                    uid::WRAITH | 21 => WRAITH_CLOAK_COST_PER_TICK,
                    _ => 0,
                }
            } else {
                0
            };
            if cloak_drain > 0 {
                let owner = self.units[i].owner as usize;
                if !self.players[owner].cheats.infinite_energy
                    && !self.units[i].spend_energy_fixed(cloak_drain)
                {
                    self.units[i].status.cloaked = false;
                }
            } else {
                self.units[i].add_energy_fixed(ENERGY_REGEN_PER_TICK);
            }
        }
        let status_update = self.tick.is_multiple_of(ACID_SPORE_UPDATE_INTERVAL);
        let (irradiate, plague, matrix_expired) = {
            let s = &mut self.units[i].status;
            // `FUN_005cf290` runs every eighth tick.  Its byte timers are
            // update counts, not logical ticks: Ensnare/Plague/Irradiate 75,
            // Lockdown/Stasis 131, Maelstrom 22, Matrix 168.
            let matrix_expired = if status_update { s.tick_matrix() } else { None };
            // Damage is applied on the same update that decrements the last
            // point of the timer (`FUN_005cf290` calls the damage routine
            // before testing for zero).
            let irradiate_due = s.irradiate_ticks;
            let plague_due = s.plague_ticks;
            if status_update {
                s.tick_acid_spores();
                s.lockdown_ticks = (s.lockdown_ticks - 1).max(0);
                s.irradiate_ticks = (s.irradiate_ticks - 1).max(0);
                s.plague_ticks = (s.plague_ticks - 1).max(0);
                s.ensnare_ticks = (s.ensnare_ticks - 1).max(0);
                s.maelstrom_ticks = (s.maelstrom_ticks - 1).max(0);
            }
            s.stim_ticks = (s.stim_ticks - 1).max(0);
            s.psi_storm_guard = s.psi_storm_guard.saturating_sub(1);
            s.disruption_ticks = (s.disruption_ticks - 1).max(0);
            s.revealed_ticks = (s.revealed_ticks - 1).max(0);
            (irradiate_due, plague_due, matrix_expired)
        };
        if let Some(absorbed) = matrix_expired {
            self.matrix_cues.push(MatrixCue {
                unit: self.units[i].id,
                x: self.units[i].x / fp(1),
                y: self.units[i].y / fp(1),
                absorbed,
                broke: true,
            });
        }
        if status_update && irradiate > 0 {
            // `FUN_005d25e3`: (Irradiate damage << 8) / cooldown, weapon 34
            // (250 * 256 / 75 = 853) every eighth tick.  The aura damages
            // every biological unit within 32 px of the irradiated unit's
            // collision box, including itself; burrowed units only take the
            // direct damage when they are the irradiated unit.
            let damage = self
                .data
                .weapon(wid::IRRADIATE)
                .map(|w| (w.damage as i32) * fp(1) / (w.cooldown.max(1) as i32))
                .unwrap_or(0);
            if damage > 0 {
                let mut targets = Vec::new();
                for j in 0..self.units.len() {
                    let u = &self.units[j];
                    if !u.alive || u.in_transport.is_some() {
                        continue;
                    }
                    let ty = self.data.unit(u.type_id);
                    if !ty.is_organic()
                        || ty.is_building()
                        || matches!(
                            u.type_id,
                            uid::LARVA | uid::EGG | uid::COCOON | uid::LURKER_EGG
                        )
                    {
                        continue;
                    }
                    if j != i && u.status.burrowed {
                        continue;
                    }
                    if self.box_gap_px(i, j) <= IRRADIATE_RADIUS_PX {
                        targets.push(j);
                    }
                }
                for j in targets {
                    self.damage_hp(j, damage, 0);
                }
            }
        }
        if status_update && plague > 0 {
            // `FUN_005dc2c0`: plague damage is weapon 60's 300 * 256 / 0x4c
            // per update, and is skipped when it would reach the current HP
            // (so the spell cannot kill by itself).
            let damage = self
                .data
                .weapon(wid::PLAGUE)
                .map(|w| (w.damage as i32) * fp(1) / 0x4c)
                .unwrap_or(0);
            let hp = self.units[i].hp;
            if damage > 0 && damage < hp {
                self.damage_hp(i, damage, 0);
            }
        }
    }

    fn update_construction(&mut self, i: usize) {
        if self.units[i].completed {
            return;
        }
        // A Terran building that was started by an SCV only advances while
        // an SCV is constructing it (stopped SCV = paused construction), and
        // its hit points grow with the progress like retail.
        if self.units[i].builder != 0 {
            let bid = self.units[i].id;
            let busy = self.index_of(self.units[i].builder).is_some_and(|j| {
                self.units[j].alive
                    && matches!(self.units[j].order, Order::Construct { building } if building == bid)
            });
            if !busy {
                return;
            }
        }
        if self.units[i].build_remaining > 0 {
            if self.units[i].builder != 0 && self.units[i].build_total > 0 {
                let max = self.data.unit(self.units[i].type_id).hit_points_raw as i32;
                let gain = (max - max / 10) / self.units[i].build_total;
                self.units[i].hp = (self.units[i].hp + gain.max(1)).min(max);
            }
            self.units[i].build_remaining -= self.build_step(self.units[i].owner);
            if self.units[i].build_remaining <= 0 {
                if let Some(target) = self.units[i].morph_to.take() {
                    self.units[i].type_id = target;
                }
                // Translation batch 03 (`0x005b0ba9`): mark the structure
                // complete and restore its full raw HP.
                let type_id = self.units[i].type_id;
                self.units[i].a03_finish_construction(&self.data, type_id);
                let ty = self.data.unit(self.units[i].type_id).clone();
                if ty.has_shields {
                    self.units[i].shields = fp(ty.shield_points as i32);
                }
                if is_creep_source(self.units[i].type_id) {
                    self.creep_dirty = true;
                }
                // A completed Refinery/Assimilator/Extractor takes over the
                // geyser underneath it: the gas pool moves to the building
                // and the geyser is hidden.
                if is_gas_building(self.units[i].type_id) {
                    let (bx, by) = (self.units[i].x, self.units[i].y);
                    let geyser = self.units.iter().position(|u| {
                        u.alive
                            && u.type_id == uid::VESPENE_GEYSER
                            && (u.x - bx).abs() < fp(64)
                            && (u.y - by).abs() < fp(64)
                    });
                    if let Some(g) = geyser {
                        let pool = self.units[g].resource_amount;
                        self.units[i].resource_amount = if pool > 0 { pool } else { GAS_CAPACITY };
                        self.units[g].resource_amount = 0;
                        self.units[g].alive = false;
                    }
                }
            }
        }
    }

    fn update_production(&mut self, i: usize) {
        if !self.units[i].alive || !self.units[i].completed || self.units[i].lifted {
            return;
        }
        // Production and research pause while a Protoss structure is outside
        // a Pylon field; it resumes when power returns.
        if !self.building_powered(i) {
            return;
        }
        if self.units[i].order == Order::Idle && !self.units[i].production.is_empty() {
            self.start_next_production(i);
        }
        if self.units[i].build_remaining > 0 {
            self.units[i].build_remaining -= self.build_step(self.units[i].owner);
            if self.units[i].build_remaining > 0 {
                return;
            }
        }
        let order = self.units[i].order.clone();
        match order {
            // A Nuclear Silo "trains" its missile: it stays inside, armed.
            Order::Train { unit_type }
                if unit_type == uid::NUCLEAR_MISSILE
                    && self.units[i].type_id == uid::NUCLEAR_SILO =>
            {
                self.units[i].nuke_ready = true;
                self.units[i].order = Order::Idle;
                self.start_next_production(i);
            }
            Order::Train { unit_type } => {
                let owner = self.units[i].owner;
                let spawn_pos = self.exit_position(i);
                let rally = self.units[i].rally;
                if let Ok(new_idx) = self.spawn_px(
                    owner as usize,
                    unit_type,
                    spawn_pos.0 / fp(1),
                    spawn_pos.1 / fp(1),
                ) {
                    // A trained caster starts with a quarter of its maximum
                    // energy: 50, or 62.5 with its reactor upgrade (retail
                    // `finish_building_unit`: `energy = max_energy / 4`).
                    if self.units[new_idx].energy_cap > 0 {
                        let quarter = self.units[new_idx].energy_cap / 4;
                        self.units[new_idx].set_energy_fixed(quarter);
                    }
                    if let Some(rally) = rally {
                        self.units[new_idx].order = Order::Move { pos: rally };
                    }
                }
                self.units[i].order = Order::Idle;
                self.start_next_production(i);
            }
            Order::Research { tech_id } => {
                let owner = self.units[i].owner as usize;
                if let Some(t) = self.data.tech(tech_id) {
                    let _ = t;
                }
                self.players[owner].techs[tech_id as usize] = true;
                self.units[i].order = Order::Idle;
            }
            Order::Upgrade { upgrade_id, level } => {
                let owner = self.units[i].owner as usize;
                self.players[owner].upgrades[upgrade_id as usize] = level;
                self.units[i].order = Order::Idle;
                self.refresh_energy_caps();
            }
            _ => {}
        }
    }

    fn update_larva(&mut self) {
        let mut spawns: Vec<(usize, (i32, i32), u32)> = Vec::new();
        for i in 0..self.units.len() {
            let (alive, completed, hatch) = {
                let u = &self.units[i];
                (
                    u.alive,
                    u.completed,
                    matches!(u.type_id, uid::HATCHERY | uid::LAIR | uid::HIVE),
                )
            };
            if !alive || !completed || !hatch {
                continue;
            }
            if self.larva_count_for(i) >= 3 {
                // Retail returns before touching CUnit+0xca while capped.
                continue;
            }
            if self.units[i].larva_timer <= 1 {
                self.units[i].larva_timer = LARVA_SPAWN_TICKS;
                if let Some(pos) = self.larva_spawn_point(i) {
                    spawns.push((self.units[i].owner as usize, pos, self.units[i].id));
                }
            } else {
                self.units[i].larva_timer -= 1;
            }
        }
        for (owner, (x, y), parent) in spawns {
            if let Ok(l) = self.spawn_px(owner, uid::LARVA, x, y) {
                self.units[l].set_connected_unit(Some(parent));
            }
        }
    }

    /// `0x005dfe20`: choose the next larva spot from the bottom/left/right/
    /// top side scores, requiring creep and a walkable/creep neighbour.
    fn larva_spawn_point(&mut self, hatch_idx: usize) -> Option<(i32, i32)> {
        let (hx, hy, type_id, values) = {
            let h = &self.units[hatch_idx];
            (
                h.x / fp(1),
                h.y / fp(1),
                h.type_id,
                h.larva_spawn_side_values,
            )
        };
        let d = self.data.unit(type_id).dimensions;
        let candidates = [
            (3usize, (hx, hy + d.bottom as i32 + 10), (0, 22)),
            (0, (hx - d.left as i32 - 10, hy), (-22, 0)),
            (2, (hx + d.right as i32 + 10, hy), (22, 0)),
            (1, (hx, hy - d.top as i32 - 10), (0, -22)),
        ];
        let mut best = None;
        let mut best_score = 101;
        for (side, pos, off) in candidates {
            let score = values[side];
            if score < best_score && self.larva_spawn_clear(pos, (pos.0 + off.0, pos.1 + off.1)) {
                best_score = score;
                best = Some(pos);
            }
        }
        best.or(Some((hx, hy + d.bottom as i32 + 10)))
    }

    fn larva_spawn_clear(&mut self, pos: (i32, i32), neighbour: (i32, i32)) -> bool {
        let max_x = self.terrain.width as i32 * 32;
        let max_y = self.terrain.height as i32 * 32;
        if pos.0 < 0 || pos.1 < 0 || pos.0 >= max_x || pos.1 >= max_y {
            return false;
        }
        if neighbour.0 < 0 || neighbour.1 < 0 || neighbour.0 >= max_x || neighbour.1 >= max_y {
            return false;
        }
        self.has_creep_px(pos.0, pos.1)
            && (self.terrain.walkable_px(neighbour.0, neighbour.1)
                || self.has_creep_px(neighbour.0, neighbour.1))
    }

    /// Retail `0x5e0290`: larvae are counted per linked hatchery inside a
    /// +/-256 px square, not per owner.
    fn larva_count_for(&self, hatch_idx: usize) -> usize {
        let h = &self.units[hatch_idx];
        let (hx, hy) = (h.x, h.y);
        self.units
            .iter()
            .filter(|l| {
                l.alive
                    && l.type_id == uid::LARVA
                    && l.connected_unit == Some(h.id)
                    && (l.x - hx).abs() <= fp(256)
                    && (l.y - hy).abs() <= fp(256)
            })
            .count()
    }

    /// OpenBW `find_unit_placement`: the spot nearest to `center` (1/256
    /// px), within 128 px, where unit `j`'s box is on walkable ground and
    /// overlaps no other ground unit or building.  Searched on a 4 px grid,
    /// ring by ring.
    fn free_spot_near(&self, j: usize, center: (i32, i32)) -> Option<(i32, i32)> {
        let ty = self.data.unit(self.units[j].type_id);
        let (cx, cy) = (center.0 / fp(1), center.1 / fp(1));
        let fits = |x: i32, y: i32| {
            let r = ty.dimensions.rect_at((x, y));
            if r.0 < 0 || r.1 < 0 {
                return false;
            }
            let corners = [(r.0, r.1), (r.2, r.1), (r.0, r.3), (r.2, r.3), (x, y)];
            if !corners
                .iter()
                .all(|&(px, py)| self.terrain.walkable_px(px, py))
            {
                return false;
            }
            !self.units.iter().enumerate().any(|(k, o)| {
                k != j && o.alive && o.in_transport.is_none() && !self.is_air_unit(k) && {
                    let b = self
                        .data
                        .unit(o.type_id)
                        .dimensions
                        .rect_at((o.x / fp(1), o.y / fp(1)));
                    r.0 <= b.2 && b.0 <= r.2 && r.1 <= b.3 && b.1 <= r.3
                }
            })
        };
        for ring in 0..=32 {
            let d = ring * 4;
            let mut best: Option<(i64, (i32, i32))> = None;
            for k in -ring..=ring {
                for (dx, dy) in [(k * 4, -d), (k * 4, d), (-d, k * 4), (d, k * 4)] {
                    let (x, y) = (cx + dx, cy + dy);
                    if fits(x, y) {
                        let dist = (dx as i64).pow(2) + (dy as i64).pow(2);
                        if best.is_none_or(|(bd, _)| dist < bd) {
                            best = Some((dist, (x, y)));
                        }
                    }
                }
            }
            if let Some((_, (x, y))) = best {
                return Some((x * fp(1), y * fp(1)));
            }
        }
        None
    }

    fn exit_position(&self, i: usize) -> (i32, i32) {
        let u = &self.units[i];
        let ty = self.data.unit(u.type_id);
        let w = ty.dimensions.right.max(0) as i32 * fp(1);
        let h = ty.dimensions.bottom.max(0) as i32 * fp(1);
        (u.x + w / 2 + fp(8), u.y + h / 2 + fp(8))
    }

    /// Advance toward a hostile without starting a new attack.  Used on the
    /// ticks between carry passes: retail keeps moving the flingy every tick
    /// while the order handler only re-validates the target every 8th tick.
    fn advance_attack(&mut self, i: usize, target: u32) {
        if let Some(tidx) = self.index_of(target)
            && self.units[tidx].alive
        {
            // Already in weapon reach: stand still (walking on into the
            // target shoved both fighters across the map).
            if let Some((_, w)) = self.weapon_for(i, tidx)
                && self.units_distance_px(i, tidx) <= self.weapon_max_range_px(i, tidx, &w)
            {
                self.units[i].path.clear();
                self.units[i].path_goal = None;
                return;
            }
            // Game pixels (see `attack_target`).
            let p = (self.units[tidx].x / fp(1), self.units[tidx].y / fp(1));
            self.move_towards(i, p, fp(2));
        }
    }

    fn execute_order(&mut self, i: usize, order: Order, think: bool) {
        if self.units[i].status.stasis_ticks > 0
            || self.units[i].status.lockdown_ticks > 0
            || self.units[i].status.maelstrom_ticks > 0
            || (self.units[i].status.burrowed && !matches!(order, Order::Unburrow))
        {
            return;
        }
        match order {
            Order::Idle => self.order_idle(i),
            Order::Stop => {
                self.units[i].target = None;
                self.units[i].path.clear();
                self.units[i].path_goal = None;
                self.units[i].order = Order::Idle;
            }
            Order::Move { pos } => {
                if self.move_towards(i, pos, fp(4)) {
                    self.order_done(i);
                }
            }
            Order::AttackMove { pos } => {
                // Retail `HealMove`: a Medic on attack-move stops for a
                // patient; `medic_healing` walks it there and heals.
                if self.units[i].type_id == uid::MEDIC
                    && self.units[i].energy_fixed() >= MEDIC_HEAL_COST_PER_TICK
                    && self.medic_pick_target(i, &[]).is_some()
                {
                    return;
                }
                if think && let Some(target) = self.acquire_target(i) {
                    self.units[i].resume_order = Some(Order::AttackMove { pos });
                    if self.units[i].type_id == uid::CARRIER {
                        self.units[i].special_cooldown = 0x3c;
                    }
                    self.units[i].order = Order::AttackUnit { target };
                    self.attack_target(i, target);
                    return;
                }
                if self.move_towards(i, pos, fp(4)) {
                    self.order_done(i);
                }
            }
            Order::AttackUnit { target } => {
                if think {
                    self.attack_target(i, target);
                } else {
                    self.advance_attack(i, target);
                }
            }
            Order::HoldPosition => {
                if let Some(target) = self.acquire_target(i) {
                    self.attack_target_no_move(i, target);
                }
            }
            Order::Patrol { a, b, forward } => {
                if let Some(target) = self.acquire_target(i) {
                    self.attack_target(i, target);
                    return;
                }
                let dest = if forward { b } else { a };
                if self.move_towards(i, dest, fp(4)) {
                    let next = Order::Patrol {
                        a,
                        b,
                        forward: !forward,
                    };
                    self.units[i].order = next;
                }
            }
            Order::Gather { resource } => self.order_gather(i, resource),
            Order::ReturnCargo { depot } => self.order_return(i, depot),
            Order::Build { unit_type, pos } => {
                if think {
                    self.order_build(i, unit_type, pos);
                } else {
                    // The walk to the site advances every tick; only the
                    // placement decision runs on the 8-tick order cadence
                    // (the SCV crawled at 1/8 of its speed, Wasteland).
                    let pos_fp = (pos.0 * fp(1), pos.1 * fp(1));
                    if self.distance_to_point(i, pos_fp) > fp(64) {
                        self.move_towards(i, pos, fp(32));
                    }
                }
            }
            Order::Repair { target } => self.order_repair(i, target),
            Order::Construct { building } => self.order_construct(i, building),
            Order::Train { .. } | Order::Research { .. } | Order::Upgrade { .. } => {
                // Handled by update_production.
            }
            Order::Cast { spell, target } => {
                // Yamato Gun charging: the shot leaves when the charge
                // overlay (image 543, iscript 314) reaches `castspell`.
                if spell == Spell::YamatoGun && self.units[i].order_wait > 0 {
                    self.units[i].order_wait -= 1;
                    if self.units[i].order_wait == 0 {
                        let id = self.units[i].id;
                        let _ = self.cast_unrecorded(id, spell, target);
                        self.order_done(i);
                    }
                    return;
                }
                if think {
                    self.order_cast(i, spell, target);
                } else {
                    // The walk into casting range advances every tick (it
                    // ran at 1/8 speed on the order cadence, like the build
                    // walk).
                    let range = self
                        .spell_range_px(spell)
                        .unwrap_or_else(|| spell_range(spell));
                    let pos = match target {
                        SpellTarget::Unit(id) => self.unit(id).map(|u| (u.x, u.y)),
                        SpellTarget::Point(x, y) => Some((x * fp(1), y * fp(1))),
                        SpellTarget::None => None,
                    };
                    if let Some(p) = pos
                        && self.distance_to_point(i, p) > range
                    {
                        self.move_towards(i, (p.0 / fp(1), p.1 / fp(1)), range / fp(1));
                    }
                }
            }
            Order::EnterTransport { transport } => self.order_enter_transport(i, transport),
            Order::PickupTransport { unit } => self.order_pickup_transport(i, unit),
            Order::Unload => self.order_unload(i),
            // OpenBW `order_MoveUnload`: within 16 px of the point the
            // transport starts unloading.
            Order::MoveUnload { pos } => {
                if self.distance_to_point(i, (pos.0 * fp(1), pos.1 * fp(1))) <= fp(16) {
                    self.units[i].path.clear();
                    self.units[i].path_goal = None;
                    self.units[i].order = Order::Unload;
                    self.units[i].order_wait = 0;
                } else {
                    self.move_towards(i, pos, fp(4));
                }
            }
            Order::Burrow => {
                if think {
                    self.order_burrow(i, true);
                }
            }
            Order::Unburrow => {
                if think {
                    self.order_burrow(i, false);
                }
            }
            // The transformation counts frames, so it runs every tick.
            Order::Siege => self.order_siege(i, true),
            Order::Unsiege => self.order_siege(i, false),
            Order::Lift => {
                let id = self.units[i].id;
                let _ = self.lift_building(id);
            }
            Order::Land { pos } => {
                // Fly to the grid-snapped spot (the preview's), then land
                // (Desperate Alliance recording: the Barracks flies over and
                // sets down in the green box).
                let id = self.units[i].id;
                let ty = self.units[i].type_id;
                let ((cx, cy), _, tiles) = self.placement_tiles(ty, pos, Some(id));
                if !tiles.iter().all(|t| *t) {
                    self.order_done(i);
                } else if self.distance_to_point(i, (cx * fp(1), cy * fp(1))) > fp(2) {
                    self.move_towards(i, (cx, cy), fp(1));
                } else if self.land_building(id, (cx, cy)).is_err() {
                    self.order_done(i);
                }
            }
            Order::Die => {}
        }
    }

    fn order_done(&mut self, i: usize) {
        if self.units[i].anim.as_ref().is_some_and(|a| {
            matches!(
                a.anim as usize,
                ANIM_GND_ATTACK_INIT
                    | ANIM_AIR_ATTACK_INIT
                    | ANIM_GND_ATTACK_REPEAT
                    | ANIM_AIR_ATTACK_REPEAT
                    | ANIM_CAST_SPELL
            )
        }) {
            self.units[i].anim = None;
        }
        if let Some(resume) = self.units[i].resume_order.take() {
            // Finishing the temporary attack returns the unit to its
            // attack-move destination instead of dropping to Idle.
            self.units[i].order = resume;
            self.units[i].target = None;
            self.units[i].path.clear();
            self.units[i].path_goal = None;
            return;
        }
        self.units[i].target = None;
        self.units[i].path.clear();
        self.units[i].path_goal = None;
        // Translation batch 06 (`0x005b7c20`): replace the finished order
        // with `Idle` (then promote the next queued node) and refresh the
        // `orders.dat` row.
        self.units[i].a06_pop_order_head(&self.data);
    }

    fn order_idle(&mut self, i: usize) {
        let ty = self.data.unit(self.units[i].type_id).clone();
        if ty.is_building() {
            // Defensive buildings (Missile Turret, Photon Cannon, Sunken and
            // Spore Colony) acquire and fire while stationary; economic
            // buildings have no weapon and stay idle.
            if let Some(target) = self.acquire_target(i) {
                self.attack_target_no_move(i, target);
            }
            return;
        }
        if self.units[i].type_id == uid::SCV
            || self.units[i].type_id == uid::PROBE
            || self.units[i].type_id == uid::DRONE
            // The Spider Mine runs its own state machine
            // (`update_specials`), not the generic acquire-and-attack.
            || self.units[i].type_id == uid::SPIDER_MINE
        {
            return;
        }
        if let Some(target) = self.acquire_target(i) {
            self.attack_target(i, target);
        }
    }

    fn attack_target(&mut self, i: usize, target: u32) {
        let Some(tidx) = self.index_of(target) else {
            self.order_done(i);
            return;
        };
        if !self.units[tidx].alive {
            self.order_done(i);
            return;
        }
        self.units[i].target = Some(target);
        if self.attack_target_no_move(i, target) {
            // In range: the unit stands and fires.  A path left over from
            // the approach made the renderer play the walk animation over
            // the attack (the Firebat's flame never showed).
            self.units[i].path.clear();
            self.units[i].path_goal = None;
            return;
        }
        // `move_towards` takes game pixels: passing the fixed-point position
        // sent every chasing unit toward a point 256x farther away, off the
        // map (Wasteland: Marines at x 3700-4200 on a 2048 px map).
        let (tx, ty) = (self.units[tidx].x, self.units[tidx].y);
        self.move_towards(i, (tx / fp(1), ty / fp(1)), fp(2));
    }

    /// Fire if in range.  Returns true when the unit is in range (whether or
    /// not the cooldown allowed a shot this tick).
    fn attack_target_no_move(&mut self, i: usize, target: u32) -> bool {
        let Some(tidx) = self.index_of(target) else {
            return false;
        };
        if !self.units[tidx].alive {
            return false;
        }
        self.units[i].target = Some(target);
        let Some((weapon_id, weapon)) = self.weapon_for(i, tidx) else {
            return self.special_attack(i, tidx);
        };
        // Weapon range is measured between the units' boxes, not their
        // centres (OpenBW `units_distance`): a melee weapon (SCV 10 px,
        // Zergling / Zealot 15 px) reaches a target whose box it touches.
        // Translation batch 21 (`0x005a0836`): maximum range plus the
        // `CUnit+0x105` bit-1 gate, in whole pixels.
        let distance = self.units_distance_px(i, tidx);
        // Translation batch 23 (`0x005a4835` / `0x005a485f`): the matching
        // ground/air weapon must clear its `weapons.dat` minimum range.
        let min_ok = if self.is_air_unit(tidx) {
            self.a23_air_weapon_min_range_ok(&self.units[i], distance)
        } else {
            self.a23_ground_weapon_min_range_ok(&self.units[i], distance)
        };
        let max_range = self.weapon_max_range_px(i, tidx, &weapon);
        if !self.units[i].a21_attack_range_max_gate(distance, max_range) || !min_ok {
            return false;
        }
        if self.units[i].status.disruption_ticks > 0 {
            return false;
        }
        // The attack animation supplies the release timing: the weapon fires
        // on its `attack`/`attackwith`/`attackmelee` opcode, not on the tick
        // the order is issued.
        // `gotorepeatattk` ends the swing: scripts that go on to an idle
        // loop instead of `ignorerest` (SCV, Zergling) may attack again.
        let attacking = self.units[i].anim.as_ref().is_some_and(|a| {
            !a.repeat_attack
                && matches!(
                    a.anim as usize,
                    ANIM_GND_ATTACK_INIT
                        | ANIM_AIR_ATTACK_INIT
                        | ANIM_GND_ATTACK_REPEAT
                        | ANIM_AIR_ATTACK_REPEAT
                )
        });
        if attacking {
            return true;
        }
        // A turret fires only once it points at the target, within the
        // weapon's attack angle (OpenBW `unit_target_in_attack_angle` on
        // the turret's heading).
        if self
            .data
            .turret(self.data.unit(self.units[i].type_id))
            .is_some()
        {
            let (dx, dy) = (
                self.units[tidx].x - self.units[i].x,
                self.units[tidx].y - self.units[i].y,
            );
            let want = direction_byte(dx, dy) as i32;
            let diff = (want - self.units[i].turret_facing as i32).rem_euclid(256);
            if diff.min(256 - diff) > weapon.attack_angle as i32 {
                return true;
            }
        }
        if self.units[i].cooldown == 0 {
            let flying = self.is_air_unit(tidx);
            if !self.start_attack_animation(i, flying) {
                self.fire(i, tidx, weapon_id, &weapon);
            }
        }
        true
    }

    fn weapon_for(&self, attacker: usize, target: usize) -> Option<(u8, WeaponType)> {
        let a_ty = self.data.unit(self.units[attacker].type_id);
        if self.is_air_unit(target) {
            let id = self.data.air_weapon_id(a_ty);
            self.data.air_weapon(a_ty).cloned().map(|w| (id, w))
        } else {
            let id = self.data.ground_weapon_id(a_ty);
            self.data.ground_weapon(a_ty).cloned().map(|w| (id, w))
        }
    }

    /// Whether units of `a` and `b` fight: enemies ([`World::is_enemy`],
    /// allies never) and neither side a rescuable player, whose units
    /// nobody targets (they wait to be rescued).
    pub fn hostile(&self, a: u8, b: u8) -> bool {
        let rescuable = |p: u8| self.players.get(p as usize).is_some_and(|p| p.rescuable);
        self.is_enemy(a, b) && !rescuable(a) && !rescuable(b)
    }

    /// `Rescue Passive` (order 160): every [`RESCUE_INTERVAL`] ticks a unit
    /// of a rescuable player joins the first human (not computer, not
    /// rescuable) player with a unit within [`RESCUE_RANGE_PX`].  The
    /// cadence is the retail handler's (`+0x54 = 0xf`, FACT); range and
    /// rescuer rule are INFERENCE (the Wasteland enemy sits near Raynor and
    /// must not take him; falsifier: a retail recording of that rescue).
    fn update_rescues(&mut self) {
        if !self.players.iter().any(|p| p.rescuable) {
            return;
        }
        let range = (RESCUE_RANGE_PX * fp(1)) as i64;
        for i in 0..self.units.len() {
            let u = &self.units[i];
            if !u.alive
                || !self
                    .players
                    .get(u.owner as usize)
                    .is_some_and(|p| p.rescuable)
            {
                continue;
            }
            let (x, y) = (u.x as i64, u.y as i64);
            let rescuer = self.units.iter().find(|o| {
                o.alive
                    && (o.owner as usize) < 8
                    && !self
                        .players
                        .get(o.owner as usize)
                        .is_none_or(|p| p.rescuable || p.defeated || p.is_computer)
                    && !is_resource_type(o.type_id)
                    && o.type_id != uid::START_LOCATION
                    && (o.x as i64 - x).pow(2) + (o.y as i64 - y).pow(2) <= range * range
            });
            if let Some(owner) = rescuer.map(|o| o.owner) {
                self.units[i].owner = owner;
                self.rescues.push((self.units[i].id, owner));
            }
        }
    }

    fn acquire_target(&self, i: usize) -> Option<u32> {
        let u = &self.units[i];
        let ty = self.data.unit(u.type_id);
        let mut acq_range = self.data.target_acquisition_px(ty);
        if is_carrier_reaver_family(u.type_id) {
            acq_range = acq_range.max(if u.type_id == uid::CARRIER {
                8 * 32
            } else {
                10 * 32
            });
        }
        if acq_range == 0 {
            return None;
        }
        // OpenBW `unit_target_acquisition_range`: a cloaked Ghost does not
        // pick targets on its own unless it is holding position.
        if is_ghost_type(u.type_id) && u.status.cloaked && !matches!(u.order, Order::HoldPosition) {
            return None;
        }
        let max_fp = acq_range * fp(1);
        let mut best: Option<(i32, u32)> = None;
        for (other_idx, other) in self.units.iter().enumerate() {
            if !other.alive || !self.hostile(u.owner, other.owner) {
                continue;
            }
            if u.in_transport == Some(other.id) {
                continue;
            }
            let t_ty = self.data.unit(other.type_id);
            if is_resource_type(other.type_id)
                || other.type_id == uid::START_LOCATION
                || is_door_type(other.type_id)
            {
                continue;
            }
            if !self.can_see_unit(u.owner, other_idx) {
                continue;
            }
            let Some(w) = (if self.is_air_unit(other_idx) {
                self.data.air_weapon(ty)
            } else {
                self.data.ground_weapon(ty)
            }) else {
                if !is_carrier_reaver_family(u.type_id) {
                    continue;
                }
                // Carriers and Reavers attack through their hangar units.
                if u.hangar == 0 {
                    continue;
                }
                let d = self.distance_ids(i, other.id);
                if d <= max_fp && best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, other.id));
                }
                continue;
            };
            if !weapon_can_target(w, t_ty) {
                continue;
            }
            // Box-to-box distance, and the weapon's range with its upgrade
            // (U-238 Shells, ...), as the attack itself measures it.
            let d = self.units_distance_px(i, other_idx) * fp(1);
            let limit = max_fp.max(self.weapon_max_range_px(i, other_idx, w) * fp(1));
            // A target inside the weapon's minimum range (the sieged tank's
            // 64 px) is not a candidate.
            if d < w.min_range as i32 * fp(1) {
                continue;
            }
            if d <= limit && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, other.id));
            }
        }
        best.map(|(_, id)| id)
    }

    fn distance_ids(&self, a: usize, b: u32) -> i32 {
        let ui = &self.units[a];
        let Some(bi) = self.index_of(b) else {
            return i32::MAX;
        };
        let uj = &self.units[bi];
        let dx = (ui.x - uj.x) as i64;
        let dy = (ui.y - uj.y) as i64;
        isqrt(dx * dx + dy * dy) as i32
    }

    /// Range upgrades (OpenBW `weapon_max_range`): U-238 Shells +32 for
    /// Marines, Grooved Spines +32 for Hydralisks, Singularity Charge +64
    /// for Dragoons, Charon Boosters +96 for the Goliath's air weapon.  The
    /// hero versions (Fenix Dragoon, Alan Schezar) always have theirs.
    pub fn weapon_range_bonus_px(&self, i: usize, target: usize) -> i32 {
        let u = &self.units[i];
        let has = |id: u8| {
            self.players
                .get(u.owner as usize)
                .is_some_and(|p| p.upgrade_level(id) > 0)
        };
        match u.type_id {
            uid::MARINE if has(upgrade::U238_SHELLS) => 32,
            uid::HYDRALISK if has(upgrade::GROOVED_SPINES) => 32,
            uid::DRAGOON if has(upgrade::SINGULARITY_CHARGE) => 64,
            // 75 = Fenix (Dragoon).
            75 => 64,
            uid::GOLIATH | uid::GOLIATH_TURRET
                if self.is_air_unit(target) && has(upgrade::CHARON_BOOSTERS) =>
            {
                96
            }
            // 17 / 18 = Alan Schezar (Goliath) and his turret.
            17 | 18 if self.is_air_unit(target) => 96,
            _ => 0,
        }
    }

    /// Maximum range in pixels of `weapon` fired by unit `i` at `target`.
    pub fn weapon_max_range_px(&self, i: usize, target: usize, weapon: &WeaponType) -> i32 {
        weapon.max_range as i32 + self.weapon_range_bonus_px(i, target)
    }

    /// Box-to-box distance in pixels from a Bunker occupant (standing at
    /// the Bunker's centre `(bx, by)`, 1/256 px) to `target`.
    fn bunker_distance_px(&self, o: usize, target: usize, bx: i32, by: i32) -> i32 {
        let a0 = self
            .data
            .unit(self.units[o].type_id)
            .dimensions
            .rect_at((bx / fp(1), by / fp(1)));
        let t = &self.units[target];
        let b0 = self
            .data
            .unit(t.type_id)
            .dimensions
            .rect_at((t.x / fp(1), t.y / fp(1)));
        let dx = (b0.0 - a0.2).max(a0.0 - b0.2).max(0) as i64;
        let dy = (b0.1 - a0.3).max(a0.1 - b0.3).max(0) as i64;
        isqrt(dx * dx + dy * dy) as i32
    }

    /// Distance in pixels between the `units.dat` boxes of two units (0 when
    /// they touch or overlap), as retail measures weapon range.
    pub fn units_distance_px(&self, a: usize, b: usize) -> i32 {
        let bx = |i: usize| {
            let u = &self.units[i];
            self.data
                .unit(u.type_id)
                .dimensions
                .rect_at((u.x / fp(1), u.y / fp(1)))
        };
        let (a0, b0) = (bx(a), bx(b));
        let dx = (b0.0 - a0.2).max(a0.0 - b0.2).max(0) as i64;
        let dy = (b0.1 - a0.3).max(a0.1 - b0.3).max(0) as i64;
        isqrt(dx * dx + dy * dy) as i32
    }

    fn distance(&self, a: usize, b: usize) -> i32 {
        let dx = (self.units[a].x - self.units[b].x) as i64;
        let dy = (self.units[a].y - self.units[b].y) as i64;
        isqrt(dx * dx + dy * dy) as i32
    }

    fn fire(&mut self, attacker: usize, target: usize, weapon_id: u8, weapon: &WeaponType) {
        // Translation batch 01 (`0x005a0720` / `0x005a077a`): a damage
        // factor of 2 runs the effect dispatcher twice.
        let shots = a01_weapon_effect_repetitions(weapon);
        let owner = self.units[attacker].owner;
        if self.units[attacker].status.hallucination {
            // Hallucinations play the attack animation and cooldown, but do
            // no damage.
            self.units[attacker].cooldown = self.modified_cooldown(attacker, weapon);
            return;
        }
        if weapon.bullet_type == behavior::SELF_DESTRUCT {
            self.kill_unit(attacker);
            return;
        }
        if Projectile::weapon_is_travelling(weapon) {
            for _ in 0..shots {
                self.spawn_projectile(attacker, target, weapon_id, weapon);
            }
        } else {
            let missed = weapon.bullet_type == behavior::APPEAR_AT_TARGET_UNIT
                && self.target_dodges(attacker, target);
            if !missed {
                let target_id = self.units[target].id;
                let pos = (self.units[target].x, self.units[target].y);
                for _ in 0..shots {
                    self.instant_hit(owner, Some(target_id), attacker, weapon_id, weapon, pos);
                }
            }
        }
        self.units[attacker].cooldown = self.modified_cooldown(attacker, weapon);
    }

    /// Frames between an instant weapon firing and its damage: the bullet
    /// image (weapon -> flingy -> sprite -> image) runs its Init script and
    /// deals the damage on `domissiledmg`.  The Gauss Rifle, C-10 and Fusion
    /// Cutter sparks take 2 frames, the Flame Thrower's bullet 7.
    pub fn instant_hit_delay(&self, weapon: &WeaponType) -> u32 {
        u16::try_from(weapon.flingy)
            .ok()
            .and_then(|f| self.data.flingy(f))
            .and_then(|f| self.data.sprite(f.sprite))
            .and_then(|s| self.data.image(s.image))
            .and_then(|i| self.data.iscript.frames_until_damage(i.iscript_id as u16))
            .unwrap_or(0)
    }

    /// Deal an instant weapon's hit when its bullet script reaches
    /// `domissiledmg`: now when there is no delay, else queued.
    fn instant_hit(
        &mut self,
        owner: u8,
        target: Option<u32>,
        source: usize,
        weapon_id: u8,
        weapon: &WeaponType,
        pos: (i32, i32),
    ) {
        let delay = self.instant_hit_delay(weapon);
        if delay == 0 {
            self.resolve_hit_type(owner, target, source, weapon_id, weapon, pos, 1, 1);
            return;
        }
        self.pending_hits.push(PendingHit {
            due: self.tick + delay as u64,
            owner,
            source: self.units[source].id,
            target,
            weapon: weapon_id,
            pos,
        });
    }

    /// Land the queued instant hits whose time has come.
    fn update_pending_hits(&mut self) {
        if self.pending_hits.is_empty() {
            return;
        }
        let tick = self.tick;
        let (due, later): (Vec<PendingHit>, Vec<PendingHit>) =
            std::mem::take(&mut self.pending_hits)
                .into_iter()
                .partition(|h| h.due <= tick);
        self.pending_hits = later;
        for h in due {
            // The shooter may have died since; its bullet still lands.
            let Some(source) = self.index_of(h.source) else {
                continue;
            };
            let Some(weapon) = self.data.weapon(h.weapon).cloned() else {
                continue;
            };
            // An "appear on target" bullet follows its target; the others
            // (the flame) stay where they appeared.
            let pos = match h.target.and_then(|t| self.index_of(t)) {
                Some(t) if weapon.bullet_type == behavior::APPEAR_AT_TARGET_UNIT => {
                    (self.units[t].x, self.units[t].y)
                }
                _ => h.pos,
            };
            self.resolve_hit_type(h.owner, h.target, source, h.weapon, &weapon, pos, 1, 1);
        }
    }

    /// Cooldown modifiers recovered from the retail `get_modified_weapon_cooldown`
    /// helper: acid spores lengthen it, Stim/cooldown upgrades halve it and
    /// Ensnare adds 25%.  The result is clamped to `[5, 250]`.
    fn modified_cooldown(&self, attacker: usize, weapon: &WeaponType) -> u32 {
        let u = &self.units[attacker];
        let mut cooldown = weapon.cooldown as i32;
        if u.status.acid_spores > 0 {
            cooldown += (cooldown / 8).max(3) * u.status.acid_spores as i32;
        }
        let mut mods = 0;
        if u.status.stim_ticks > 0 {
            mods += 1;
        }
        // OpenBW `update_unit_speed_upgrades`: Adrenal Glands for the
        // Zergling; the Devouring One (54) always has it.
        if self.data.unit(u.type_id).flags & 0x2000_0000 != 0
            || u.type_id == 54
            || (u.type_id == uid::ZERGLING
                && self
                    .players
                    .get(u.owner as usize)
                    .is_some_and(|p| p.upgrade_level(upgrade::ADRENAL_GLANDS) > 0))
        {
            mods += 1;
        }
        if u.status.ensnare_ticks > 0 {
            mods -= 1;
        }
        if mods > 0 {
            cooldown /= 2;
        } else if mods < 0 {
            cooldown += cooldown / 4;
        }
        cooldown = cooldown.clamp(5, 250);
        cooldown as u32
    }

    /// Create one travelling bullet.  The same routine handles every weapon
    /// behaviour; `Projectile::state` selects the per-tick movement rule.
    fn spawn_projectile(
        &mut self,
        attacker: usize,
        target: usize,
        weapon_id: u8,
        weapon: &WeaponType,
    ) {
        if self.projectiles.len() >= MAX_PROJECTILES {
            return;
        }
        let a = &self.units[attacker];
        let (ax, ay) = (a.x, a.y);
        let facing = a.facing as i32;
        let (dx, dy) = direction_vector(facing);
        let forward = weapon.forward_offset as i32 * fp(1);
        let mut px = ax + dx * forward / 256;
        let mut py = ay + dy * forward / 256;
        let target_unit = self.units.get(target).map(|u| (u.id, u.x, u.y));
        let Some((target_id, tx, ty)) = target_unit else {
            return;
        };
        let mut state = match weapon.bullet_type {
            behavior::FLY => BulletState::Fly,
            behavior::FOLLOW_TARGET => BulletState::Follow,
            behavior::BOUNCE => BulletState::Bounce,
            behavior::ATTACK_TARGET_POS => BulletState::HitNearTarget,
            behavior::EXTEND_TO_MAX_RANGE => BulletState::Extend,
            behavior::PERSIST_AT_TARGET_POS => BulletState::Persist,
            behavior::APPEAR_AT_SOURCE_UNIT => BulletState::Fly,
            _ => return,
        };
        let mut target_pos = (tx, ty);
        let mut hit_flags = 0u8;
        if self.units[attacker].status.hallucination {
            hit_flags |= 2;
        }
        // Miss chance.  Ground units under Dark Swarm are almost impossible
        // to hit; a target on higher ground or behind cover also has a chance
        // to dodge.  The roll is deterministic (the world's LCG).
        if matches!(
            weapon.bullet_type,
            behavior::FLY | behavior::FOLLOW_TARGET | behavior::BOUNCE
        ) && self.target_dodges(attacker, target)
        {
            hit_flags |= 1;
            target_pos.0 -= dx * 30 / 256;
            target_pos.1 -= dy * 30 / 256;
        }
        match weapon.bullet_type {
            behavior::PERSIST_AT_TARGET_POS => {
                px = tx;
                py = ty;
                state = BulletState::Persist;
            }
            behavior::ATTACK_TARGET_POS => {
                let off = self.projectile_near_offset(attacker);
                px = ax;
                py = ay;
                target_pos = (tx + off.0, ty + off.1);
            }
            behavior::EXTEND_TO_MAX_RANGE => {
                let range = weapon.max_range as i32 + 20;
                target_pos = (ax + dx * range * fp(1) / 256, ay + dy * range * fp(1) / 256);
            }
            behavior::APPEAR_AT_SOURCE_UNIT => {
                px = ax;
                py = ay;
            }
            _ => {}
        }
        let bounces = if weapon.bullet_type == behavior::BOUNCE {
            3
        } else {
            0
        };
        let speed = self.data.weapon_speed(weapon).max(256);
        let _ = state;
        self.projectiles.push(Projectile {
            id: self.next_projectile_id,
            owner: self.units[attacker].owner,
            source: self.units[attacker].id,
            weapon: weapon_id,
            behavior: weapon.bullet_type,
            hit_type: weapon.explosion,
            state: match weapon.bullet_type {
                behavior::FLY => BulletState::Fly,
                behavior::FOLLOW_TARGET => BulletState::Follow,
                behavior::BOUNCE => BulletState::Bounce,
                behavior::ATTACK_TARGET_POS => BulletState::HitNearTarget,
                behavior::EXTEND_TO_MAX_RANGE => BulletState::Extend,
                behavior::PERSIST_AT_TARGET_POS => BulletState::Persist,
                behavior::APPEAR_AT_SOURCE_UNIT => BulletState::Fly,
                _ => BulletState::Fly,
            },
            x: px,
            y: py,
            target: Some(target_id),
            target_pos,
            speed,
            remaining_time: weapon.lifetime.max(1) as i32,
            bounces_left: bounces,
            hit_flags,
            previous_target: None,
            last_dot_tick: self.tick as i64,
        });
        self.next_projectile_id = self.next_projectile_id.wrapping_add(1).max(1);
    }

    /// Create a stationary `bullet_type` 3 field at a point.  Psionic Storm
    /// uses this path (`0x568a5a`); `0x568aa5` re-applies its radial hit
    /// every seven updates for the weapon's 60-update lifetime.
    /// A bullet that flies from `source` to a point on the ground and goes
    /// off there (EMP Shockwave).
    fn spawn_point_bullet(
        &mut self,
        source: usize,
        weapon_id: u8,
        weapon: &WeaponType,
        target: (i32, i32),
    ) {
        if self.projectiles.len() >= MAX_PROJECTILES {
            return;
        }
        let (owner, source_id, x, y) = {
            let u = &self.units[source];
            (u.owner, u.id, u.x, u.y)
        };
        self.projectiles.push(Projectile {
            id: self.next_projectile_id,
            owner,
            source: source_id,
            weapon: weapon_id,
            behavior: behavior::FLY,
            hit_type: weapon.explosion,
            state: BulletState::Fly,
            x,
            y,
            target: None,
            target_pos: target,
            speed: self.data.weapon_speed(weapon).max(256),
            remaining_time: 300,
            bounces_left: 0,
            hit_flags: 0,
            previous_target: None,
            last_dot_tick: self.tick as i64,
        });
        self.next_projectile_id = self.next_projectile_id.wrapping_add(1).max(1);
    }

    fn spawn_persistent_field(
        &mut self,
        source: usize,
        weapon_id: u8,
        weapon: &WeaponType,
        pos: (i32, i32),
    ) {
        if self.projectiles.len() >= MAX_PROJECTILES {
            return;
        }
        let (owner, source_id) = {
            let u = &self.units[source];
            (u.owner, u.id)
        };
        let bullet = Projectile::persistent(
            self.next_projectile_id,
            owner,
            source_id,
            weapon_id,
            weapon,
            pos,
            self.tick as i64,
        );
        self.projectiles.push(bullet);
        self.next_projectile_id = self.next_projectile_id.wrapping_add(1).max(1);
    }

    fn projectile_near_offset(&mut self, _attacker: usize) -> (i32, i32) {
        // OpenBW `hit_near_target_positions`: bullets of type "attack
        // target position" (Halo Rockets, Corrosive Acid) aim at the target
        // plus the next of these 14 offsets, so a volley spreads around it.
        // (Retail keeps the index per shooter; the bullet counter cycles
        // through the same table.)
        const NEAR: [(i32, i32); 14] = [
            (-1, -1),
            (-51, -31),
            (-7, -12),
            (36, 52),
            (49, -25),
            (-40, 46),
            (4, 19),
            (-28, -50),
            (-50, 15),
            (54, 13),
            (-11, 53),
            (-15, 9),
            (19, -50),
            (17, -8),
        ];
        let (x, y) = NEAR[(self.next_projectile_id % 14) as usize];
        (x * fp(1), y * fp(1))
    }

    pub fn world_rand(&mut self) -> u32 {
        // 32-bit LCG used by the retail engine for miss rolls and scatter.
        self.rng = self
            .rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.rng >> 32) as u32
    }

    /// Miss chance in 1/256 units for one ranged attack.  Retail uses
    /// 255/256 against ground units inside Dark Swarm and 119/256 when the
    /// target stands on higher terrain.  Melee attacks (`fire_melee`) never
    /// consult this, and air units ignore both rules.
    fn dodge_chance(&self, attacker: usize, target: usize) -> u32 {
        let a = &self.units[attacker];
        let t = &self.units[target];
        if !self.is_air_unit(target) && self.is_under_dark_swarm(target) {
            255
        } else if !self.is_air_unit(attacker) && !self.is_air_unit(target) {
            let ah = self.terrain.elevation_px(a.x / fp(1), a.y / fp(1));
            let th = self.terrain.elevation_px(t.x / fp(1), t.y / fp(1));
            if th > ah { 119 } else { 0 }
        } else {
            0
        }
    }

    fn target_dodges(&mut self, attacker: usize, target: usize) -> bool {
        let chance = self.dodge_chance(attacker, target);
        dodge_roll(self.world_rand(), chance)
    }

    fn update_projectiles(&mut self) {
        if self.projectiles.is_empty() {
            return;
        }
        let bullets = std::mem::take(&mut self.projectiles);
        let mut kept = Vec::with_capacity(bullets.len());
        for mut b in bullets {
            if self.step_projectile(&mut b) {
                kept.push(b);
            }
        }
        self.projectiles = kept;
    }

    /// Advance one bullet.  Returns false when it has been consumed.
    fn step_projectile(&mut self, b: &mut Projectile) -> bool {
        let Some(weapon) = self.data.weapon(b.weapon).cloned() else {
            return false;
        };
        if b.state == BulletState::Persist {
            b.remaining_time -= 1;
            if b.remaining_time <= 0 {
                return false;
            }
            if self.tick as i64 - b.last_dot_tick >= PERSIST_DAMAGE_INTERVAL {
                b.last_dot_tick = self.tick as i64;
                self.resolve_hit_type(
                    b.owner,
                    b.target,
                    self.index_of(b.source).unwrap_or(usize::MAX),
                    b.weapon,
                    &weapon,
                    b.target_pos,
                    1,
                    1,
                );
            }
            return true;
        }
        if b.state == BulletState::Follow
            && let Some(tid) = b.target
        {
            if let Some(t) = self.unit(tid) {
                if t.alive {
                    b.target_pos = (t.x, t.y);
                } else {
                    b.target = None;
                    b.state = BulletState::Fly;
                }
            } else {
                b.target = None;
                b.state = BulletState::Fly;
            }
        }
        if b.state == BulletState::Bounce
            && let Some(tid) = b.target
            && let Some(t) = self.unit(tid)
            && t.alive
        {
            b.target_pos = (t.x, t.y);
        }
        b.remaining_time -= 1;
        let dx = b.target_pos.0 - b.x;
        let dy = b.target_pos.1 - b.y;
        let dist = isqrt((dx as i64) * (dx as i64) + (dy as i64) * (dy as i64)) as i32;
        if dist <= b.speed || b.remaining_time <= 0 {
            b.x = b.target_pos.0;
            b.y = b.target_pos.1;
            return self.projectile_impact(b, &weapon);
        }
        b.x += (dx as i64 * b.speed as i64 / dist as i64) as i32;
        b.y += (dy as i64 * b.speed as i64 / dist as i64) as i32;
        true
    }

    fn projectile_impact(&mut self, b: &mut Projectile, weapon: &WeaponType) -> bool {
        // A missed shot still produces its hit animation in the retail game
        // but deals no damage.  Hallucinated attackers likewise produce a fake
        // impact with no damage.
        if b.hit_flags & 3 != 0 {
            return false;
        }
        let source_idx = self.index_of(b.source).unwrap_or(usize::MAX);
        if b.state == BulletState::Bounce {
            if b.bounces_left > 0
                && b.hit_flags & 1 == 0
                && let Some(tid) = b.target
            {
                let divisor = 3u32.saturating_pow(3 - b.bounces_left as u32);
                self.resolve_hit_type(
                    b.owner,
                    Some(tid),
                    source_idx,
                    b.weapon,
                    weapon,
                    (b.x, b.y),
                    divisor,
                    1,
                );
                if b.bounces_left > 1
                    && let Some(next) = self.find_bounce_target(b, tid)
                {
                    b.previous_target = Some(tid);
                    b.target = Some(next);
                    b.bounces_left -= 1;
                    b.remaining_time = b.remaining_time.max(30);
                    b.state = BulletState::Bounce;
                    return true;
                }
            }
            return false;
        }
        let divisor = 1;
        self.resolve_hit_type(
            b.owner,
            b.target,
            source_idx,
            b.weapon,
            weapon,
            (b.x, b.y),
            divisor,
            1,
        );
        false
    }

    fn find_bounce_target(&self, b: &Projectile, current: u32) -> Option<u32> {
        let mut best: Option<(i32, u32)> = None;
        for u in &self.units {
            if !u.alive
                || !self.is_enemy(b.owner, u.owner)
                || u.id == current
                || Some(u.id) == b.previous_target
            {
                continue;
            }
            if !self.can_see_unit(b.owner, self.index_of(u.id)?) {
                continue;
            }
            let dx = (u.x - b.x) as i64;
            let dy = (u.y - b.y) as i64;
            let d = isqrt(dx * dx + dy * dy) as i32;
            if d <= 96 * fp(1) && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, u.id));
            }
        }
        best.map(|(_, id)| id)
    }

    /// Apply a weapon's `hit_type` at `pos`.  `target` is the intended unit
    /// (used by normal hits and unit-targeted spells); `divisor` is the splash
    /// ring divisor.
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    fn resolve_hit_type(
        &mut self,
        owner: u8,
        target: Option<u32>,
        source: usize,
        weapon_id: u8,
        weapon: &WeaponType,
        pos: (i32, i32),
        divisor: u32,
        bullet_count: u32,
    ) {
        let _ = bullet_count;
        let target_idx = target.and_then(|id| self.index_of(id));
        match weapon.explosion {
            hit::NONE => {}
            hit::NORMAL | hit::YAMATO | hit::UNUSED_23 => {
                if let Some(t) = target_idx
                    && self.can_see_unit(owner, t)
                {
                    self.apply_hit_from(owner, source, t, weapon, divisor);
                }
            }
            hit::RADIAL_SPLASH | hit::ENEMY_SPLASH | hit::AIR_SPLASH => {
                self.apply_splash(
                    owner,
                    source,
                    weapon_id,
                    weapon,
                    target,
                    pos,
                    matches!(weapon.explosion, hit::ENEMY_SPLASH | hit::AIR_SPLASH),
                    weapon.explosion == hit::AIR_SPLASH,
                );
            }
            hit::NUCLEAR => {
                self.apply_nuke(owner, source, pos);
            }
            hit::LOCKDOWN => {
                if let Some(t) = target_idx {
                    self.units[t].status.apply_lockdown(LOCKDOWN_UPDATES);
                    let id = self.units[t].id;
                    self.push_spell_cue(SpellCueKind::LockdownHit, id);
                }
            }
            hit::PARASITE => {
                if let Some(t) = target_idx {
                    self.units[t].status.parasite = true;
                    self.units[t].status.revealed_ticks = 1200;
                }
            }
            hit::BROODLINGS => {
                if let Some(t) = target_idx
                    && self.data.unit(self.units[t].type_id).is_organic()
                {
                    let (x, y) = (self.units[t].x / fp(1), self.units[t].y / fp(1));
                    self.kill_unit(t);
                    self.spawn_broodling(owner as usize, x, y);
                    self.spawn_broodling(owner as usize, x + 16, y);
                }
            }
            hit::EMP => {
                // OpenBW `emp_shockwave`: every unit whose box touches the
                // square of the weapon's inner splash radius (64 px) around
                // the impact loses its energy and shields, except the
                // caster and units in stasis; hallucinations die.
                let half = weapon.inner_splash as i32;
                let (ix, iy) = (pos.0 / fp(1), pos.1 / fp(1));
                for j in 0..self.units.len() {
                    if !self.units[j].alive || j == source {
                        continue;
                    }
                    let rect = self
                        .data
                        .unit(self.units[j].type_id)
                        .dimensions
                        .rect_at((self.units[j].x / fp(1), self.units[j].y / fp(1)));
                    if rect.2 < ix - half
                        || rect.0 > ix + half
                        || rect.3 < iy - half
                        || rect.1 > iy + half
                    {
                        continue;
                    }
                    if self.units[j].status.hallucination {
                        self.kill_unit(j);
                        continue;
                    }
                    if self.units[j].status.stasis_ticks > 0 {
                        continue;
                    }
                    self.units[j].shields = 0;
                    self.units[j].set_energy_fixed(0);
                }
            }
            hit::IRRADIATE => {
                if let Some(t) = target_idx
                    && !self.data.unit(self.units[t].type_id).is_building()
                {
                    // Any non-building unit can carry the aura; only
                    // biological units in range take damage.
                    self.units[t].status.apply_irradiate(IRRADIATE_UPDATES);
                }
            }
            hit::ENSNARE => {
                for j in 0..self.units.len() {
                    if j == source
                        || !self.units[j].alive
                        || self.units[j].in_transport.is_some()
                        || self.units[j].status.burrowed
                        || self.data.unit(self.units[j].type_id).is_building()
                        || self.distance_to_point(j, pos) > fp(64)
                    {
                        continue;
                    }
                    self.units[j].status.apply_ensnare();
                }
            }
            hit::PLAGUE => {
                for j in 0..self.units.len() {
                    if self.units[j].alive
                        && !self.units[j].status.burrowed
                        && self.distance_to_point(j, pos) <= fp(64)
                    {
                        self.units[j].status.apply_plague();
                    }
                }
            }
            hit::STASIS => {
                for j in 0..self.units.len() {
                    if self.units[j].alive
                        && !self.units[j].status.burrowed
                        && !self.data.unit(self.units[j].type_id).is_building()
                        && self.distance_to_point(j, pos) <= fp(64)
                    {
                        self.units[j].status.apply_stasis(STASIS_UPDATES);
                    }
                }
            }
            hit::DARK_SWARM => {
                self.spawn_dark_swarm(owner, pos);
            }
            hit::CONSUME => {
                if let Some(t) = target_idx
                    && t != source
                {
                    let gained =
                        self.data.unit(self.units[t].type_id).hit_points_raw as i32 / fp(1) * 2;
                    if source < self.units.len() {
                        self.units[source].add_energy_points(gained);
                    }
                    self.kill_unit(t);
                }
            }
            hit::RESTORATION => {
                if let Some(t) = target_idx {
                    self.restore_unit(t);
                }
            }
            hit::DISRUPTION_WEB => {
                let _ = self.spawn_temporary(owner, uid::DISRUPTION_WEB, pos, 360);
            }
            hit::CORROSIVE_ACID => {
                if let Some(t) = target_idx {
                    self.apply_hit_from(owner, source, t, weapon, divisor);
                }
                // `FUN_005dc1c0` asks `FUN_006c1ee0` for every unit in a
                // 128 x 128 px box (`0x40` half-extent) and calls
                // `FUN_005dc360` on it: the damage is single-target, but the
                // spore debuff is an area effect.  The callback skips
                // buildings, loaded units and cloaked units the attacker
                // cannot see.
                let mut spored = Vec::new();
                for j in 0..self.units.len() {
                    let u = &self.units[j];
                    if !u.alive || u.owner == owner || u.in_transport.is_some() {
                        continue;
                    }
                    if self.data.unit(u.type_id).is_building() {
                        continue;
                    }
                    if (u.x - pos.0).abs() > fp(64) || (u.y - pos.1).abs() > fp(64) {
                        continue;
                    }
                    if u.status.cloaked && !self.can_see_unit(owner, j) {
                        continue;
                    }
                    spored.push(j);
                }
                for j in spored {
                    self.units[j].status.add_acid_spore();
                }
            }
            hit::MIND_CONTROL => {
                if let Some(t) = target_idx {
                    self.units[t].owner = owner;
                }
            }
            hit::FEEDBACK => {
                if let Some(t) = target_idx {
                    self.feedback_damage(t, source as u32);
                }
            }
            hit::OPTICAL_FLARE => {
                if let Some(t) = target_idx {
                    self.blind_unit(t);
                }
            }
            hit::MAELSTROM => {
                for j in 0..self.units.len() {
                    if self.units[j].alive
                        && !self.units[j].status.burrowed
                        && self.distance_to_point(j, pos) <= fp(64)
                        && self.data.unit(self.units[j].type_id).is_organic()
                        && !self.data.unit(self.units[j].type_id).is_building()
                    {
                        self.units[j].status.apply_maelstrom(MAELSTROM_UPDATES);
                    }
                }
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_splash(
        &mut self,
        owner: u8,
        source: usize,
        weapon_id: u8,
        weapon: &WeaponType,
        target: Option<u32>,
        impact: (i32, i32),
        enemy_only: bool,
        air_only: bool,
    ) {
        let inner = weapon.inner_splash as i64 * fp(1) as i64;
        let middle = weapon.middle_splash as i64 * fp(1) as i64;
        let outer = weapon.outer_splash as i64 * fp(1) as i64;
        if air_only {
            self.apply_air_splash(owner, source, weapon, target, impact);
            return;
        }
        for j in 0..self.units.len() {
            if !self.units[j].alive {
                continue;
            }
            // `0x00567aa0`: the bullet's source is never a splash victim,
            // except weapon 0x54 (Psionic Storm), which may hit its caster.
            if j == source && weapon_id != wid::PSIONIC_STORM {
                continue;
            }
            // `0x005b4c20`/`0x005b67d0`: a Psionic Storm victim can only be
            // hit once per damage cadence.
            if weapon_id == wid::PSIONIC_STORM && self.units[j].status.psi_storm_guard != 0 {
                continue;
            }
            let air = self.is_air_unit(j);
            if air_only && !air {
                continue;
            }
            if enemy_only && self.units[j].owner == owner && Some(self.units[j].id) != target {
                continue;
            }
            // Target flags apply to every splash victim, not just the
            // acquired target: ground-only Shock Cannon splash must not hit
            // air and Psionic Storm's 0x10 bit must not hit buildings.
            let victim_ty = self.data.unit(self.units[j].type_id).clone();
            if !weapon_can_target_unit(weapon, &victim_ty, air) {
                continue;
            }
            // Distance from the impact point to the victim's box, not to its
            // centre (OpenBW `unit_distance_to`): a large unit is hit when
            // any part of it is inside the ring.
            let rect = victim_ty
                .dimensions
                .rect_at((self.units[j].x / fp(1), self.units[j].y / fp(1)));
            let (ix, iy) = (impact.0 / fp(1), impact.1 / fp(1));
            let d = {
                let dx = (rect.0 - ix).max(ix - rect.2).max(0) as i64 * fp(1) as i64;
                let dy = (rect.1 - iy).max(iy - rect.3).max(0) as i64 * fp(1) as i64;
                isqrt(dx * dx + dy * dy)
            };
            if d > outer {
                continue;
            }
            // `FUN_00576ce0` (hit types 2/3/24): burrowed units only take
            // the immediate/inner ring; middle and outer miss them.
            if self.units[j].status.burrowed && d > inner {
                continue;
            }
            let divisor = if d <= inner {
                1
            } else if d <= middle {
                2
            } else {
                4
            };
            if weapon_id == wid::PSIONIC_STORM {
                self.units[j].status.psi_storm_guard = 1;
            }
            self.apply_hit_from(owner, source, j, weapon, divisor);
        }
    }

    /// Air splash (Halo Rockets; OpenBW `bullet_deal_splash_damage` with
    /// `is_air_splash`).  One unit takes the full hit: the bullet's target
    /// if the rocket landed within the inner radius of it, else one valid
    /// air unit in the outer radius picked at random.  Every other valid
    /// unit in reach, except the bullet's target and Interceptors, takes
    /// half within the middle radius and a quarter within the outer one.
    /// The shooter's own units are spared unless they are the target.
    fn apply_air_splash(
        &mut self,
        owner: u8,
        source: usize,
        weapon: &WeaponType,
        target: Option<u32>,
        impact: (i32, i32),
    ) {
        let (inner, middle, outer) = (
            weapon.inner_splash as i32,
            weapon.middle_splash as i32,
            weapon.outer_splash as i32,
        );
        let (ix, iy) = (impact.0 / fp(1), impact.1 / fp(1));
        // (unit, distance from the impact to its box)
        let mut reach: Vec<(usize, i32)> = Vec::new();
        for j in 0..self.units.len() {
            let u = &self.units[j];
            if !u.alive || j == source || !self.is_air_unit(j) {
                continue;
            }
            if u.owner == owner && Some(u.id) != target {
                continue;
            }
            let ty = self.data.unit(u.type_id);
            if !weapon_can_target_unit(weapon, ty, true) {
                continue;
            }
            let rect = ty.dimensions.rect_at((u.x / fp(1), u.y / fp(1)));
            let dx = (rect.0 - ix).max(ix - rect.2).max(0) as i64;
            let dy = (rect.1 - iy).max(iy - rect.3).max(0) as i64;
            let d = isqrt(dx * dx + dy * dy) as i32;
            if d <= outer {
                reach.push((j, d));
            }
        }
        if reach.is_empty() {
            return;
        }
        let on_target = reach
            .iter()
            .find(|(j, d)| Some(self.units[*j].id) == target && *d <= inner)
            .map(|(j, _)| *j);
        let main = on_target.unwrap_or_else(|| {
            if reach.len() == 1 {
                reach[0].0
            } else {
                reach[self.world_rand() as usize % reach.len()].0
            }
        });
        self.apply_hit_from(owner, source, main, weapon, 1);
        for (j, d) in reach {
            let u = &self.units[j];
            if !u.alive
                || Some(u.id) == target
                || u.type_id == uid::INTERCEPTOR
                || u.status.burrowed
            {
                continue;
            }
            let divisor = if d <= middle { 2 } else { 4 };
            self.apply_hit_from(owner, source, j, weapon, divisor);
        }
    }

    /// The retail nuclear missile does `max(500, 2/3 * max_hp_plus_shields)`
    /// damage to every unit in its splash radius, ignoring the weapon's stored
    /// per-shot damage.
    fn apply_nuke(&mut self, owner: u8, source: usize, impact: (i32, i32)) {
        let Some(mut nuke) = self.data.weapon(wid::NUCLEAR_STRIKE).cloned() else {
            return;
        };
        // weapons.dat row 31: full damage within 128 px of the victim's
        // box, half within 192, a quarter within 256 (the splash rings of
        // every radial weapon).
        let (inner, middle, outer) = (
            nuke.inner_splash as i32,
            nuke.middle_splash as i32,
            nuke.outer_splash as i32,
        );
        let (ix, iy) = (impact.0 / fp(1), impact.1 / fp(1));
        let mut targets = Vec::new();
        for j in 0..self.units.len() {
            if !self.units[j].alive {
                continue;
            }
            let rect = self
                .data
                .unit(self.units[j].type_id)
                .dimensions
                .rect_at((self.units[j].x / fp(1), self.units[j].y / fp(1)));
            let dx = (rect.0 - ix).max(ix - rect.2).max(0) as i64;
            let dy = (rect.1 - iy).max(iy - rect.3).max(0) as i64;
            let d = isqrt(dx * dx + dy * dy) as i32;
            let divisor = if d <= inner {
                1
            } else if d <= middle {
                2
            } else if d <= outer {
                4
            } else {
                continue;
            };
            targets.push((j, divisor));
        }
        nuke.damage_bonus = 0;
        for (j, divisor) in targets {
            let ty = self.data.unit(self.units[j].type_id);
            let max = ty.hit_points_raw as i32
                + if ty.has_shields {
                    fp(ty.shield_points as i32)
                } else {
                    0
                };
            let damage = (max * 2 / 3).max(fp(500));
            nuke.damage = (damage / fp(1)).max(1) as u16;
            self.apply_hit_from(owner, source, j, &nuke, divisor);
        }
    }

    fn apply_hit(&mut self, attacker: usize, target: usize, weapon: &WeaponType, divisor: u32) {
        let owner = self.units.get(attacker).map(|u| u.owner).unwrap_or(0);
        self.apply_hit_from(owner, attacker, target, weapon, divisor);
    }

    fn apply_hit_from(
        &mut self,
        owner: u8,
        _source: usize,
        target: usize,
        weapon: &WeaponType,
        divisor: u32,
    ) {
        if owner as usize >= self.players.len() || !self.units.get(target).is_some_and(|u| u.alive)
        {
            return;
        }
        let t_owner = self.units[target].owner;
        if t_owner as usize >= self.players.len() {
            return;
        }
        let t_ty = self.data.unit(self.units[target].type_id).clone();
        let a_up = self.players[owner as usize].upgrade_level(weapon.damage_upgrade);
        let t_armor_up = self.players[t_owner as usize].upgrade_level(t_ty.armor_upgrade);
        let plating = crate::combat::chitinous_plating_bonus(
            self.units[target].type_id,
            t_ty.flags,
            self.players[t_owner as usize].upgrade_level(upgrade::CHITINOUS_PLATING) > 0,
        );
        let defense = Defense {
            size: t_ty.size,
            armor_level: t_ty.armor as u32 + t_armor_up as u32 + plating,
            has_shields: t_ty.has_shields,
            shield_points: self.units[target].shields,
            shield_upgrade: self.players[t_owner as usize]
                .upgrade_level(upgrade::PROTOSS_PLASMA_SHIELDS),
        };
        let status = StatusDamage {
            matrix: self.units[target].status.matrix,
            acid_spores: self.units[target].status.acid_spores,
            hallucination: self.units[target].status.hallucination,
        };
        let result = resolve_damage_status(weapon, a_up, defense, divisor, status);
        let matrix_was_active = self.units[target].status.matrix_active();
        if matrix_was_active {
            let (absorbed, broke) = self.units[target]
                .status
                .absorb_matrix(result.matrix_absorbed);
            let cue = MatrixCue {
                unit: self.units[target].id,
                x: self.units[target].x / fp(1),
                y: self.units[target].y / fp(1),
                absorbed,
                broke,
            };
            self.matrix_cues.push(cue);
        }
        if result.damage.shield_damage > 0 {
            self.units[target].shields =
                (self.units[target].shields - result.damage.shield_damage).max(0);
        }
        if result.damage.hp_damage > 0 {
            // `_source` is the attacker's index (`usize::MAX` once it died);
            // `last_damage_source` stores unit ids, which is what
            // `kill_unit` resolves to credit the kill.  Passing the index
            // credited kills to whichever unit had that id.
            let source_id = self.units.get(_source).map(|u| u.id).unwrap_or(0);
            self.damage_hp(target, result.damage.hp_damage, source_id);
        }
    }

    /// Apply raw hit-point damage to a unit (scripting/test entry point).
    pub fn damage_unit(&mut self, id: u32, amount: i32) -> Result<(), SimError> {
        let target = self.index_of(id).ok_or(SimError::BadUnit)?;
        self.damage_hp(target, amount, 0);
        Ok(())
    }

    /// Set a unit's plasma shields in whole points (scripting/test entry
    /// point for damage and Shield Battery scenarios).
    pub fn set_unit_shields(&mut self, id: u32, points: i32) -> Result<(), SimError> {
        let target = self.index_of(id).ok_or(SimError::BadUnit)?;
        self.units[target].shields = fp(points.max(0));
        Ok(())
    }

    /// Feedback drains every 1/256 energy point and deals exactly that many
    /// 1/256 HP of damage.  Defensive Matrix absorbs the damage first; shields
    /// and armour are bypassed, as in the retail spell.
    fn feedback_damage(&mut self, target: usize, source: u32) {
        let energy = self.units[target].energy_fixed();
        if energy <= 0 {
            return;
        }
        self.units[target].set_energy_fixed(0);
        let matrix_was_active = self.units[target].status.matrix_active();
        let absorbed = if matrix_was_active {
            let (absorbed, broke) = self.units[target].status.absorb_matrix(energy);
            self.matrix_cues.push(MatrixCue {
                unit: self.units[target].id,
                x: self.units[target].x / fp(1),
                y: self.units[target].y / fp(1),
                absorbed,
                broke,
            });
            absorbed
        } else {
            0
        };
        let hp_damage = energy - absorbed;
        if hp_damage > 0 {
            self.damage_hp(target, hp_damage, source);
        }
    }

    fn damage_hp(&mut self, target: usize, amount: i32, source: u32) {
        if !self.units[target].alive || self.units[target].invincible {
            return;
        }
        let target_owner = self.units[target].owner as usize;
        if self
            .players
            .get(target_owner)
            .is_some_and(|p| p.cheats.invincible)
        {
            return;
        }
        if source != 0 {
            self.units[target].last_damage_source = source;
        }
        self.units[target].hp -= amount;
        self.units[target].last_damaged = self.tick;
        if self.units[target].hp <= 0 {
            self.kill_unit(target);
        } else if source != 0 {
            self.retaliate(target, source);
        }
    }

    /// An idle unit that takes damage attacks its attacker when one of its
    /// weapons can hit it; idle units of a computer player near the victim
    /// join in.  FACT (Wasteland, 2026-11-05): without this, Zerglings shot
    /// by Marines (4-tile range against their 3-tile acquisition) stayed idle
    /// until dead.  INFERENCE: the helper radius (8 tiles) and that only
    /// computer players call for help; falsifier: a retail recording of a
    /// human player's idle unit next to an attacked one.
    fn retaliate(&mut self, victim: usize, source: u32) {
        let Some(a) = self.index_of(source) else {
            return;
        };
        let (owner, attacker_owner) = (self.units[victim].owner, self.units[a].owner);
        if !self.units[a].alive || !self.hostile(owner, attacker_owner) {
            return;
        }
        let air = self.is_air_unit(a);
        let attacker_ty = self.data.unit(self.units[a].type_id).clone();
        let can_hit = |w: &World, i: usize| {
            let ty = w.data.unit(w.units[i].type_id);
            // (Spider Mines run their own state machine.)
            if ty.is_worker()
                || w.units[i].type_id == uid::SPIDER_MINE
                || !matches!(w.units[i].order, Order::Idle | Order::Stop)
            {
                return false;
            }
            let weapon = if air {
                w.data.air_weapon(ty)
            } else {
                w.data.ground_weapon(ty)
            };
            weapon.is_some_and(|wt| weapon_can_target(wt, &attacker_ty))
        };
        let mut responders = Vec::new();
        if can_hit(self, victim) {
            responders.push(victim);
        }
        if self
            .players
            .get(owner as usize)
            .is_some_and(|p| p.is_computer)
        {
            let r = (8 * 32 * fp(1)) as i64;
            let (vx, vy) = (self.units[victim].x as i64, self.units[victim].y as i64);
            for j in 0..self.units.len() {
                if j != victim
                    && self.units[j].alive
                    && self.units[j].owner == owner
                    && (self.units[j].x as i64 - vx).pow(2) + (self.units[j].y as i64 - vy).pow(2)
                        <= r * r
                    && !self.data.unit(self.units[j].type_id).is_building()
                    && can_hit(self, j)
                {
                    responders.push(j);
                }
            }
        }
        for j in responders {
            self.units[j].order = Order::AttackUnit { target: source };
            self.units[j].order_row = self.units[j].order.order_id();
        }
    }

    /// `0x005b2b70`: clear references to a dying unit from every other
    /// unit's target/order fields, plus the nuke link.  Rust stores the
    /// same links in typed orders instead of the retail `CUnit` offsets.
    fn remove_target_references(&mut self, dead_id: u32) {
        let mut cleared_orders = Vec::new();
        for (i, u) in self.units.iter_mut().enumerate() {
            if u.target == Some(dead_id) {
                u.target = None;
            }
            if u.order_unit_target() == Some(dead_id) {
                cleared_orders.push(i);
            }
            if u.connected_unit == Some(dead_id) {
                u.connected_unit = None;
            }
            if u.rally.is_some() {
                // Rally points store only the ground position in this
                // engine; no unit id remains to clear.
            }
        }
        // `0x005a3cd0`: the order target is gone; finish the temporary
        // attack so `resume_order` (attack-move/patrol) can take over.
        for i in cleared_orders {
            self.order_done(i);
        }
        // Once the bomb is falling nothing calls it off.
        self.nuke_strikes
            .retain(|s| s.timer <= -NUKE_WARP_FRAMES || (s.ghost != dead_id && s.silo != dead_id));
    }

    /// `0x005b083a`: retail unit-death cleanup.  The Rust path marks the
    /// unit dead, clears target/nuke references, tears down add-ons/cargo,
    /// credits the kill and refreshes creep exactly once.
    fn kill_unit(&mut self, i: usize) {
        if !self.units[i].alive {
            return;
        }
        self.units[i].alive = false;
        self.units[i].hp = 0;
        // Translation batch 10 (`0x005d22bf`): removal wipes acid-spore
        // stacks; the rest of the death status cleanup follows.
        self.units[i].a10_clear_acid_spores_if_present();
        self.units[i].status.clear_on_death();
        self.units[i].clear_production_queue();
        // `0x005b06aa`: a destroyed Nuclear Silo drops its armed missile
        // and the parent link (the generic add-on cleanup below removes the
        // parent's `addon` slot).
        if self.units[i].type_id == uid::NUCLEAR_SILO {
            self.units[i].nuke_ready = false;
        }
        // `0x005cc810` returns 0x546 for Broodlings and dying units;
        // `0x005b6810` stores it in CUnit+0x110.
        self.units[i].set_death_timer_min(BROODLING_LIFETIME_TICKS as u16);
        let owner = self.units[i].owner as usize;
        // `0x005b6160`/`0x005da2e0`: a producer destroyed while upgrading
        // refunds 75% of the current level's mineral and gas cost.  An
        // active technology is only cancelled (`0x005da390`).
        if let Order::Upgrade { upgrade_id, .. } = self.units[i].order
            && let Some(up) = self.data.upgrade(upgrade_id)
            && owner < self.players.len()
        {
            let level = self.players[owner].upgrade_level(upgrade_id);
            let minerals = up.mineral_cost(level) as i32 * 3 / 4;
            let gas = up.gas_cost(level) as i32 * 3 / 4;
            self.players[owner].minerals += minerals;
            self.players[owner].gas += gas;
        }
        self.units[i].order = Order::Die;
        let type_id = self.units[i].type_id;
        let dead_id = self.units[i].id;
        self.cancel_nuke_for(dead_id);
        self.remove_target_references(dead_id);
        // Translation batch 29 (`0x005b05a0`): carrier/reaver hangars empty
        // on removal; the ghost-hero arm has no extra Rust state.
        let _ = self.units[i].a29_apply_removal_route();
        let source_id = self.units[i].last_damage_source;
        self.units[i].last_damage_source = 0;
        let source_owner = self
            .index_of(source_id)
            .map(|j| self.units[j].owner as usize)
            .filter(|&s| s != owner && s < self.players.len());
        if is_creep_source(type_id) {
            self.creep_dirty = true;
        }
        // An add-on dies with its parent, and a destroyed add-on clears the
        // parent's slot.
        if let Some(addon_id) = self.units[i].addon.take()
            && let Some(j) = self.index_of(addon_id)
            && self.units[j].alive
        {
            self.kill_unit(j);
        }
        let dead_id = self.units[i].id;
        for u in &mut self.units {
            if u.addon == Some(dead_id) {
                u.addon = None;
            }
        }
        let building = self.data.unit(type_id).is_building();
        if self.sudden_death
            && owner < self.players.len()
            && matches!(
                type_id,
                uid::COMMAND_CENTER | uid::NEXUS | uid::HATCHERY | uid::LAIR | uid::HIVE
            )
        {
            self.players[owner].defeated = true;
        }
        if owner < self.players.len() {
            self.players[owner].deaths += 1;
            if building {
                self.players[owner].buildings_lost += 1;
            } else {
                self.players[owner].units_lost += 1;
            }
            if (type_id as usize) < self.players[owner].death_counts.len() {
                self.players[owner].death_counts[type_id as usize] += 1;
            }
        }
        if let Some(killer) = source_owner {
            if let Some(j) = self.index_of(source_id) {
                self.units[j].kills += 1;
                self.units[j].credit_rank_kill();
            }
            self.players[killer].kills += 1;
            if (type_id as usize) < self.players[killer].kill_counts.len() {
                self.players[killer].kill_counts[type_id as usize] += 1;
            }
            if building {
                self.players[killer].razings += 1;
            }
            // Kill score (Men) and razing score (Building): the victim's
            // `units.dat` destroy score.
            let ty = self.data.unit(type_id);
            let destroy = ty.destroy_score as i32;
            if ty.group_flags & GROUP_MEN != 0 {
                self.players[killer].scores[4] += destroy;
            } else if ty.group_flags & GROUP_BUILDING != 0 {
                self.players[killer].scores[5] += destroy;
            }
        }
        // Loaded units die with their transport/bunker (retail behaviour);
        // they are not ejected alive.
        let cargo = std::mem::take(&mut self.units[i].cargo);
        for id in cargo {
            if let Some(j) = self.index_of(id) {
                // Loaded units are kept with `alive == false` and
                // `in_transport = Some(transport)`, so `kill_unit(j)` would
                // early-return without touching them.  Mark the cargo dead
                // directly and clear the link.
                self.units[j].hp = 0;
                self.units[j].alive = false;
                self.units[j].clear_transport_link();
                self.units[j].set_in_bunker(false);
                self.units[j].order = Order::Die;
                self.units[j].path.clear();
                self.units[j].path_goal = None;
            }
        }
    }

    fn resolve_deaths(&mut self) {
        // Dead units stay in the vector (ids are stable) but are skipped by
        // every system.  Transports drop their cargo on death in kill_unit.
    }

    fn check_victory(&mut self) {
        if self.players.iter().any(|p| p.cheats.stay_alive) {
            return;
        }
        // Special melee rules run before the trigger-driven victory gate:
        // they are lobby rules, not map objectives.
        if let Some(goal) = self.greed_goal
            && let Some(p) = self
                .players
                .iter()
                .position(|p| !p.defeated && p.minerals_gathered >= goal && p.gas_gathered >= goal)
        {
            self.winner = Some(p as u8);
            return;
        }
        if let Some(deadline) = self.slaughter_deadline
            && self.tick >= deadline
        {
            let best = self
                .players
                .iter()
                .enumerate()
                .filter(|(_, p)| !p.defeated)
                .max_by_key(|(i, p)| (p.kills, std::cmp::Reverse(*i)))
                .map(|(i, _)| i as u8);
            self.winner = best;
            return;
        }
        if self.trigger_victory {
            return;
        }
        // A sandbox with a single owner (unit tests, spell labs) has no
        // opposing side and therefore no victory condition.
        let owners: std::collections::HashSet<u8> = self.units.iter().map(|u| u.owner).collect();
        if owners.len() < 2 {
            return;
        }
        let alive_players: Vec<u8> = (0..self.players.len() as u8)
            .filter(|&p| {
                !self.players[p as usize].defeated
                    && self.units.iter().any(|u| {
                        u.owner == p
                            && !is_resource_type(u.type_id)
                            && (u.alive || u.in_transport.is_some())
                    })
            })
            .collect();
        // The game ends when everyone still standing is on one side (a lone
        // survivor, or one team in a team game); `winner` names the side's
        // first player and `has_won` answers for the whole side.
        let one_side = alive_players
            .iter()
            .all(|&p| alive_players.first().is_none_or(|&f| self.same_side(p, f)));
        if one_side {
            self.winner = alive_players.first().copied();
        }
    }

    /// `0x005b5ba3`: allied lookup in the retail alliance table.  Players
    /// are on the same side when identical or equal `team`, which the lobby
    /// and the "Set Alliance Status" trigger both write.
    pub fn same_side(&self, a: u8, b: u8) -> bool {
        a == b
            || matches!(
                (self.players.get(a as usize), self.players.get(b as usize)),
                (Some(pa), Some(pb)) if pa.team == pb.team
            )
    }

    /// `0x005b5b90`: retail alliance-table lookup.  Two owners are enemies
    /// when neither the same player nor an ally.
    pub fn is_enemy(&self, a: u8, b: u8) -> bool {
        !self.same_side(a, b)
    }

    /// Town hall an AI of `player` attacks: `prefer`'s (the local player in
    /// the client, as before teams existed) when `prefer` is an enemy, else
    /// the first enemy player's by player order.  Allies are never chosen.
    pub fn enemy_town_hall(&self, player: u8, prefer: u8) -> Option<(i32, i32)> {
        let hall = |p: u8| {
            self.units
                .iter()
                .find(|u| {
                    u.alive
                        && u.owner == p
                        && matches!(
                            u.type_id,
                            uid::COMMAND_CENTER
                                | uid::NEXUS
                                | uid::HATCHERY
                                | uid::LAIR
                                | uid::HIVE
                        )
                })
                .map(|u| (u.x / 256, u.y / 256))
        };
        if self.is_enemy(player, prefer)
            && let Some(at) = hall(prefer)
        {
            return Some(at);
        }
        (0..self.players.len() as u8)
            .filter(|&p| self.is_enemy(player, p) && !self.players[p as usize].defeated)
            .find_map(hall)
    }

    /// Whether `p` won: the game has a winner and `p` is on its side.
    pub fn has_won(&self, p: u8) -> bool {
        self.winner.is_some_and(|w| {
            w == p
                || (self.same_side(w, p)
                    && self
                        .players
                        .get(w as usize)
                        .is_some_and(|winner| winner.allied_victory))
        })
    }

    /// Engine score bookkeeping (OpenBW `add_completed_unit` and the kill
    /// path): the first tick a unit is complete, its `units.dat` build score
    /// goes to its owner's unit score (Men) or building score (Building).
    /// Starting and pre-placed units count, which is why a fresh Protoss
    /// melee start already shows Units 200 / Structures 400 (retail score
    /// screen, 2026-11-04 21:14:04).  Hallucinations do not (OpenBW also
    /// skips turret subunits, which this engine does not create as units).
    pub fn credit_scores(&mut self) {
        for i in 0..self.units.len() {
            let u = &self.units[i];
            if !u.alive || !u.completed || u.credited {
                continue;
            }
            let owner = u.owner as usize;
            let resource = is_resource_type(u.type_id);
            let hallucination = u.status.hallucination;
            self.units[i].credited = true;
            if resource || hallucination || owner >= self.players.len() {
                continue;
            }
            let ty = self.data.unit(self.units[i].type_id);
            let score = ty.build_score as i32;
            if ty.group_flags & GROUP_MEN != 0 {
                self.players[owner].scores[1] += score;
                self.players[owner].units_produced += 1;
            } else if ty.group_flags & GROUP_BUILDING != 0 {
                self.players[owner].scores[2] += score;
                self.players[owner].structures_built += 1;
            }
        }
    }

    // ------------------------------------------------------------------
    // Movement
    // ------------------------------------------------------------------

    fn build_obstacles(&mut self) {
        self.obstacles.clear();
        for u in &self.units {
            if !u.alive || !u.completed {
                continue;
            }
            let ty = self.data.unit(u.type_id);
            if is_door_type(u.type_id) {
                if u.doodad_enabled.unwrap_or(false) {
                    self.obstacles
                        .push(unit_box_rect(ty, (u.x / fp(1), u.y / fp(1))));
                }
                continue;
            }
            if !u.lifted && (is_resource_type(u.type_id) || ty.is_building()) {
                let (l, t, r, b) = ty.dimensions.extents();
                if l + r > 0 || t + b > 0 {
                    let rect = unit_box_rect(ty, (u.x / fp(1), u.y / fp(1)));
                    self.obstacles.push(rect);
                }
            }
        }
    }

    /// Retail hides a worker while it is inside a Refinery / Assimilator /
    /// Extractor (OpenBW `order_HarvestGas` calls `hide_unit` for the 37
    /// frames): it is not drawn, cannot be selected or targeted and takes no
    /// part in collision.
    pub fn inside_gas_building(&self, i: usize) -> bool {
        let Some(u) = self.units.get(i) else {
            return false;
        };
        if !u.alive || u.gather_timer == 0 {
            return false;
        }
        let Order::Gather { resource } = u.order else {
            return false;
        };
        self.index_of(resource)
            .is_some_and(|r| is_gas_building(self.units[r].type_id))
    }

    /// Retail `status_flag_gathering`: set while a worker walks to a
    /// resource, mines it or carries cargo back.  Such a worker collides
    /// only with buildings (OpenBW `unit_can_collide_with`), which is why
    /// mineral lines do not jam.
    pub fn is_gathering(&self, i: usize) -> bool {
        self.units.get(i).is_some_and(|u| {
            u.alive && matches!(u.order, Order::Gather { .. } | Order::ReturnCargo { .. })
        })
    }

    /// Half extents of a ground unit's collision box in 1/256 pixels.
    fn collision_extents(&self, i: usize) -> Option<(i32, i32)> {
        let u = &self.units[i];
        if !u.alive || u.in_transport.is_some() || u.status.burrowed {
            return None;
        }
        if self.is_gathering(i) {
            return None;
        }
        let ty = self.data.unit(u.type_id);
        if ty.is_building() || ty.is_flyer() || is_door_type(u.type_id) {
            return None;
        }
        // `units.dat` dimensions are in pixels.  Every stock ground unit has
        // a non-zero box; the floor keeps tiny doodads from collapsing.
        let hw = ty.dimensions.right.max(ty.dimensions.left.max(0)) as i32;
        let hh = ty.dimensions.bottom.max(ty.dimensions.top.max(0)) as i32;
        Some(((hw.max(5)) * fp(1), (hh.max(5)) * fp(1)))
    }

    /// Air units do not collide, but they do not stack either (OpenBW
    /// `apply_repulse_field`).  The map is cut into 48 px cells; a flyer
    /// that shares its cell with another drifts half a pixel per frame
    /// along its repulse direction, faster the more crowded the cell when
    /// it is standing still, with a small per-unit bias (the adjust table)
    /// so a stack fans out instead of sliding as one.  Buildings,
    /// Interceptors and units that cannot be interrupted are left alone.
    fn apply_air_repulse(&mut self) {
        const ADJUST: [i32; 32] = [
            -5, -5, -5, 5, 5, -5, 5, 5, 5, -5, 5, 5, -5, -5, -5, 5, -5, 5, 5, -5, -5, -5, 5, 5, -5,
            5, 5, -5, 5, 5, -5, -5,
        ];
        let movers: Vec<usize> = (0..self.units.len())
            .filter(|&i| {
                let u = &self.units[i];
                u.alive
                    && u.in_transport.is_none()
                    && u.type_id != uid::INTERCEPTOR
                    && u.type_id != uid::NUCLEAR_MISSILE
                    && !self.data.unit(u.type_id).is_building()
                    && self.is_air_unit(i)
            })
            .collect();
        if movers.len() < 2 {
            return;
        }
        let cell = |u: &SimUnit| (u.x / fp(48), u.y / fp(48));
        let mut field: HashMap<(i32, i32), i32> = HashMap::new();
        for &i in &movers {
            *field.entry(cell(&self.units[i])).or_default() += 1;
        }
        let (map_w, map_h) = (
            self.terrain.width as i32 * 32,
            self.terrain.height as i32 * 32,
        );
        for i in movers {
            let strength = field[&cell(&self.units[i])].min(255);
            if self.units[i].repulse_flags & 0xf0 == 0 {
                let v = self.world_rand();
                self.units[i].repulse_dir = (v & 0xff) as u8;
                let row = ((v >> 8) & 0xf0) as u8;
                self.units[i].repulse_flags = if row == 0 { 0xf0 } else { row };
            }
            if strength <= 1 {
                self.units[i].repulse_flags &= !7;
                continue;
            }
            if self.is_transforming(i)
                || self.units[i].status.lockdown_ticks > 0
                || self.units[i].status.stasis_ticks > 0
                || self.units[i].status.maelstrom_ticks > 0
            {
                continue;
            }
            let still = self.units[i].path.is_empty();
            if self.units[i].repulse_flags & 7 != 0 {
                self.units[i].repulse_flags -= 1;
            } else {
                self.units[i].repulse_flags |= 7;
                // `update_repulse_direction`: while moving, keep the drift
                // to the side of the travel direction; near a map edge,
                // turn it back inwards.
                let mut dir = self.units[i].repulse_dir;
                if let Some(&(wx, wy)) = self.units[i].path.front() {
                    let target = direction_byte(wx - self.units[i].x, wy - self.units[i].y) as i32;
                    let index = (dir as i32 - target).rem_euclid(256);
                    let side = index % 32 + if index < 128 { 32 } else { -64 };
                    dir = (target + side).rem_euclid(256) as u8;
                }
                let (x, y) = (self.units[i].x / fp(1), self.units[i].y / fp(1));
                // Directions 1..=127 point east, 129..=255 west; 65..=191
                // south, the rest north.
                let east = (1..128).contains(&dir);
                let south = (65..192).contains(&dir);
                if (x < 48 && !east && dir != 0 && dir != 128) || (x > map_w - 48 && east) {
                    dir = 0u8.wrapping_sub(dir);
                }
                if (y < 48 && !south) || (y > map_h - 48 && south) {
                    dir = 128u8.wrapping_sub(dir);
                }
                self.units[i].repulse_dir = dir;
            }
            let (vx, vy) = direction_vector(self.units[i].repulse_dir as i32);
            let (mut rx, mut ry) = (vx / 2, vy / 2);
            if still {
                rx = rx * (strength / 4 + 1) / 2;
                ry = ry * (strength / 4 + 1) / 2;
            }
            let row = (self.units[i].repulse_flags >> 4) as usize;
            let (ax, ay) = (ADJUST[row * 2], ADJUST[row * 2 + 1]);
            // Pull a strong component back by the bias and push a weak one
            // further (OpenBW's four-way compare, folded).
            let bias = |v: i32, a: i32| match v {
                0 => 0,
                v if v > 0 => {
                    if v > a * 4 {
                        v - a
                    } else {
                        v + a
                    }
                }
                v => {
                    if v < -a * 4 {
                        v + a
                    } else {
                        v - a
                    }
                }
            };
            rx = bias(rx, ax);
            ry = bias(ry, ay);
            let nx = (self.units[i].x + rx).clamp(fp(8), fp(map_w - 8));
            let ny = (self.units[i].y + ry).clamp(fp(8), fp(map_h - 8));
            self.units[i].x = nx;
            self.units[i].y = ny;
        }
    }

    /// Turrets (Siege Tank, Goliath) turn towards the unit's target, or
    /// back along the hull without one, by their flingy turn rate per
    /// frame (17 for the Goliath's guns, 13 for the tank cannons).  A
    /// freshly spawned or just-sieged turret starts along the hull.
    fn turn_turrets(&mut self) {
        for i in 0..self.units.len() {
            let u = &self.units[i];
            if !u.alive {
                continue;
            }
            let ty = self.data.unit(u.type_id);
            let Some(turret) = self.data.turret(ty) else {
                continue;
            };
            let rate = self
                .data
                .flingy(turret.flingy)
                .map_or(255, |f| f.turn_rate as i32)
                .max(1);
            // At rest: along the hull, or the sieged cannon's own rest
            // heading once it has unfolded.
            let rest = if u.type_id == uid::SIEGE_MODE
                && !(matches!(u.order, Order::Siege) && u.order_wait > 0)
            {
                SIEGED_TURRET_HEADING
            } else {
                u.facing as i32
            };
            let want = u
                .target
                .and_then(|t| self.index_of(t))
                .filter(|&t| self.units[t].alive)
                .map_or(rest, |t| {
                    direction_byte(self.units[t].x - u.x, self.units[t].y - u.y) as i32
                });
            let cur = u.turret_facing as i32;
            let mut diff = (want - cur).rem_euclid(256);
            if diff > 128 {
                diff -= 256;
            }
            let step = diff.clamp(-rate, rate);
            self.units[i].turret_facing = (cur + step).rem_euclid(256) as u8;
        }
    }

    fn try_shift(&mut self, i: usize, dx: i32, dy: i32) -> bool {
        if dx == 0 && dy == 0 {
            return true;
        }
        let nx = self.units[i].x + dx;
        let ny = self.units[i].y + dy;
        if self.terrain.walkable_px(nx / fp(1), ny / fp(1)) {
            self.units[i].x = nx;
            self.units[i].y = ny;
            true
        } else {
            false
        }
    }

    /// Push overlapping ground units apart.  Retail units cannot share the
    /// same ground position; ignoring this made armies collapse into a single
    /// point.  The separation is the minimum translation along the smaller
    /// overlap axis, split between both bodies, with a few relaxation passes.
    fn resolve_collisions(&mut self) {
        let mut movable: Vec<usize> = (0..self.units.len())
            .filter(|&i| self.collision_extents(i).is_some())
            .collect();
        if movable.len() < 2 {
            return;
        }
        movable.sort_unstable();
        let cell = 32 * fp(1);
        for _pass in 0..2 {
            let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
            for &i in &movable {
                let key = (
                    self.units[i].x.div_euclid(cell),
                    self.units[i].y.div_euclid(cell),
                );
                grid.entry(key).or_default().push(i);
            }
            let mut pairs: Vec<(usize, usize)> = Vec::new();
            for (&(cx, cy), list) in &grid {
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        let Some(other) = grid.get(&(cx + dx, cy + dy)) else {
                            continue;
                        };
                        for &i in list {
                            for &j in other {
                                if i < j {
                                    pairs.push((i, j));
                                }
                            }
                        }
                    }
                }
            }
            pairs.sort_unstable();
            pairs.dedup();
            for (i, j) in pairs {
                let (Some((hwi, hhi)), Some((hwj, hhj))) =
                    (self.collision_extents(i), self.collision_extents(j))
                else {
                    continue;
                };
                let dx = self.units[j].x - self.units[i].x;
                let dy = self.units[j].y - self.units[i].y;
                let ox = hwi + hwj - dx.abs();
                let oy = hhi + hhj - dy.abs();
                if ox <= 0 || oy <= 0 {
                    continue;
                }
                if ox < oy {
                    let sign = if dx >= 0 { 1 } else { -1 };
                    let half = (ox / 2).max(1) * sign;
                    if !self.try_shift(i, -half, 0) {
                        let _ = self.try_shift(j, half * 2, 0);
                    } else {
                        let _ = self.try_shift(j, half, 0);
                    }
                } else {
                    let sign = if dy >= 0 { 1 } else { -1 };
                    let half = (oy / 2).max(1) * sign;
                    if !self.try_shift(i, 0, -half) {
                        let _ = self.try_shift(j, 0, half * 2);
                    } else {
                        let _ = self.try_shift(j, 0, half);
                    }
                }
            }
        }
    }

    /// Move toward `goal` in pixel coordinates.  Returns true when the unit
    /// has arrived.
    fn move_towards(&mut self, i: usize, goal: (i32, i32), stop_distance: i32) -> bool {
        let ty = self.data.unit(self.units[i].type_id).clone();
        if ty.is_building() && !self.units[i].status.sieged && !self.units[i].lifted {
            return true;
        }
        if !ty.is_flyer() && !self.units[i].lifted {
            self.ensure_walking_animation(i);
        }
        let goal_fp = (goal.0.saturating_mul(fp(1)), goal.1.saturating_mul(fp(1)));
        if self.units[i].path_goal != Some(goal_fp) {
            self.build_obstacles();
            let speed = self.speed_of(i);
            let half_w = ty.dimensions.right.max(0).max(ty.dimensions.left.max(0)) as i32;
            let half_h = ty.dimensions.bottom.max(0).max(ty.dimensions.top.max(0)) as i32;
            let req = PathRequest {
                start: (self.units[i].x / fp(1), self.units[i].y / fp(1)),
                goal: (goal.0, goal.1),
                half_width: half_w.max(4),
                half_height: half_h.max(4),
                flying: ty.is_flyer() || self.units[i].lifted || speed <= 0,
            };
            let path = if speed <= 0 {
                None
            } else {
                find_path_with_regions(&self.terrain, &self.obstacles, &req, Some(&self.regions))
            };
            self.units[i].path.clear();
            self.units[i].path_goal = Some(goal_fp);
            if let Some(p) = path {
                for w in p.waypoints {
                    // find_path works in pixels; convert to fp.
                    self.units[i].path.push_back((w.0 * fp(1), w.1 * fp(1)));
                }
                if self.units[i].path.is_empty() {
                    self.units[i].path.push_back(goal_fp);
                }
            } else {
                // No route: fall back to a direct move.  The retail engine
                // reports "unreachable" and keeps the unit in place; a direct
                // fallback keeps the simulation usable on partially walkable
                // custom maps without hiding the failure from the caller.
                self.units[i].path.push_back(goal_fp);
            }
        }
        let top = self.speed_of(i);
        if top <= 0 {
            return true;
        }
        // Flingy-controlled units (SCV, Vulture, Drone, Overlord, lifted
        // buildings; flingy.dat move control 0) accelerate by `acceleration`
        // per frame up to their top speed and brake inside `halt_distance`
        // of the goal, as retail.  FACT (Boot Camp recording, 3 SCVs on gas):
        // a delivery every 1.97 s against our 1.80 s with SCVs at full speed
        // from a standstill.  Iscript-moved walkers (Marines, Zerglings)
        // keep their animation speeds.
        let speed = match self.data.flingy(ty.flingy) {
            Some(f)
                if f.movement_type == crate::data::MovementType::Flingy && f.acceleration > 0 =>
            {
                let acc = self.modified_rate(i, f.acceleration as i32);
                let cur = if self.units[i].moved_tick + 1 == self.tick {
                    self.units[i].speed
                } else {
                    0
                };
                // Brake when the stopping distance at the current speed,
                // cur^2 / (2 acc) in 1/256 px, reaches the goal: at top
                // speed that is exactly flingy.dat's `halt_distance` (SCV
                // 1280^2 / 134 = 12227).
                let stopping = (cur as i64 * cur as i64 / (2 * acc as i64)) as i32;
                let near = self.distance_to_point(i, goal_fp) <= stopping.max(acc);
                let next = if near {
                    (cur - acc).max(acc)
                } else {
                    (cur + acc).min(top)
                };
                self.units[i].speed = next;
                self.units[i].moved_tick = self.tick;
                next
            }
            _ => top,
        };
        // Flingy units turn at flingy.dat `turn_rate` (256 = a full turn)
        // per frame and do not advance until they face their waypoint
        // within one step (INFERENCE from BW's flingy model; FACT: Boot Camp
        // gas with instant turning ran 1.82 s per delivery against the
        // recording's 1.97 s, two reversals per trip).
        let turn_rate = self
            .data
            .flingy(ty.flingy)
            .filter(|f| f.movement_type == crate::data::MovementType::Flingy && f.turn_rate > 0)
            .map(|f| f.turn_rate as i32)
            .map(|r| self.modified_rate(i, r));
        if let (Some(rate), Some(&(wx, wy))) = (turn_rate, self.units[i].path.front()) {
            let (dx, dy) = (wx - self.units[i].x, wy - self.units[i].y);
            if dx != 0 || dy != 0 {
                let want = direction_byte(dx, dy) as i32;
                let cur = self.units[i].facing as i32;
                let diff = (want - cur + 128).rem_euclid(256) - 128;
                if diff != 0 {
                    if diff.abs() > rate {
                        self.units[i].facing = (cur + rate * diff.signum()).rem_euclid(256) as u8;
                        self.units[i].speed = 0;
                        return false;
                    }
                    // Within one turn step: face the waypoint exactly before
                    // advancing, or the sprite keeps a small permanent skew.
                    self.units[i].facing = want as u8;
                }
            }
        }
        let mut remaining = speed;
        // Intermediate waypoints are reached within twice the unit's half
        // size, and given up after `PATH_STALL_TICKS` ticks without getting
        // closer: a group shares the same path corners, and requiring each
        // unit to stand on one froze the whole group once the others
        // occupied it (Wasteland: nine units jammed for 1,500+ ticks).
        let reach = fp((2 * half_extent_px(&ty)).max(16));
        if let Some(&(wx, wy)) = self.units[i].path.front() {
            let d = self.distance_to_point(i, (wx, wy));
            if d >= self.units[i].path_last_dist {
                self.units[i].path_stall = self.units[i].path_stall.saturating_add(1);
            } else {
                self.units[i].path_stall = 0;
            }
            self.units[i].path_last_dist = d;
            if self.units[i].path_stall >= PATH_STALL_TICKS {
                self.units[i].path_stall = 0;
                self.units[i].path_last_dist = i32::MAX;
                if self.units[i].path.len() > 1 {
                    self.units[i].path.pop_front();
                } else if self.distance_to_point(i, goal_fp) <= fp(ARRIVAL_CROWD_PX) {
                    // The goal is taken by the rest of the group: stop here,
                    // as retail units around a crowded destination do.
                    self.units[i].path.clear();
                    self.finish_move_animation(i);
                    return true;
                }
            }
        }
        while remaining > 0 {
            if self.units[i].path.len() > 1
                && let Some(&(wx, wy)) = self.units[i].path.front()
                && self.distance_to_point(i, (wx, wy)) <= reach
            {
                self.units[i].path.pop_front();
                self.units[i].path_last_dist = i32::MAX;
                continue;
            }
            let Some(&(wx, wy)) = self.units[i].path.front() else {
                if self.distance_to_point(i, goal_fp) > stop_distance {
                    // Replan next tick instead of freezing.
                    self.units[i].path_goal = None;
                }
                let done = self.distance_to_point(i, goal_fp) <= stop_distance;
                if done {
                    self.finish_move_animation(i);
                }
                return done;
            };
            let dx = wx - self.units[i].x;
            let dy = wy - self.units[i].y;
            let dist = isqrt((dx as i64) * (dx as i64) + (dy as i64) * (dy as i64)) as i32;
            if dist <= remaining {
                self.units[i].x = wx;
                self.units[i].y = wy;
                self.units[i].path.pop_front();
                remaining -= dist.max(0);
            } else if dist > 0 {
                self.units[i].x += (dx as i64 * remaining as i64 / dist as i64) as i32;
                self.units[i].y += (dy as i64 * remaining as i64 / dist as i64) as i32;
                self.units[i].facing = direction_byte(dx, dy);
                remaining = 0;
            } else {
                self.units[i].path.pop_front();
            }
        }
        self.distance_to_point(i, goal_fp) <= stop_distance && self.units[i].path.is_empty()
    }

    fn finish_move_animation(&mut self, i: usize) {
        if self.units[i]
            .anim
            .as_ref()
            .is_some_and(|a| a.anim as usize == ANIM_WALKING)
        {
            self.start_animation(i, ANIM_WALKING_TO_IDLE);
        }
    }

    fn distance_to_point(&self, i: usize, p: (i32, i32)) -> i32 {
        let dx = (self.units[i].x - p.0) as i64;
        let dy = (self.units[i].y - p.1) as i64;
        isqrt(dx * dx + dy * dy) as i32
    }

    /// Speed clamp recovered from `FUN_005b6900`: a positive status count
    /// applies the +50% speed (minimum 0x355) and three special hero types
    /// are pinned at 0x6ab; a negative count halves it; both cancel.
    #[cfg(test)]
    fn modified_speed(type_id: u16, base: i32, stim: bool, ensnare: bool) -> i32 {
        let mods = i32::from(stim) - i32::from(ensnare);
        Self::modified_speed_for_mods(type_id, base, mods)
    }

    /// The clamp itself, driven by the recovered status-modifier count
    /// (translation batch 04, `0x005b49a0`: Stim +1, `CUnit+0xdc` bit 29
    /// +1, Ensnare -1).
    fn modified_speed_for_mods(type_id: u16, base: i32, mods: i32) -> i32 {
        if mods > 0 {
            if matches!(type_id, 0x46 | 0x50 | 0x58) {
                return 0x6ab;
            }
            (base + base / 2).max(0x355)
        } else if mods < 0 {
            base / 2
        } else {
            base
        }
    }

    fn speed_of(&self, i: usize) -> i32 {
        let u = &self.units[i];
        let ty = self.data.unit(u.type_id);
        if u.status.lockdown_ticks > 0 || u.status.stasis_ticks > 0 {
            return 0;
        }
        if u.status.sieged || u.status.burrowed {
            return 0;
        }
        let mut speed = if let Some(anim) = &u.anim {
            if anim.fling_speed > 0 {
                anim.fling_speed
            } else if anim.anim as usize == ANIM_WALKING && anim.move_speed > 0 {
                anim.move_speed
            } else {
                let cycle = self.data.unit_walk_cycle(ty);
                if cycle.is_empty() {
                    self.data.unit_speed(ty)
                } else {
                    cycle[(u.anim_tick as usize) % cycle.len()]
                }
            }
        } else {
            let cycle = self.data.unit_walk_cycle(ty);
            if cycle.is_empty() {
                self.data.unit_speed(ty)
            } else {
                cycle[(u.anim_tick as usize) % cycle.len()]
            }
        };
        speed = Self::modified_speed_for_mods(u.type_id, speed, self.movement_mods(i));
        // Lifted Terran buildings use their flingy speed when present;
        // otherwise they drift at the standard building air speed.
        if u.lifted && speed <= 0 {
            speed = 1280;
        }
        speed
    }

    /// OpenBW `update_unit_speed_upgrades`: whether this unit moves with
    /// its speed upgrade (Ion Thrusters for the Vulture, Metabolic Boost
    /// for the Zergling, ...) or is one of the units that always have it.
    pub fn has_speed_upgrade(&self, i: usize) -> bool {
        let u = &self.units[i];
        let id = match u.type_id {
            // 19 = Jim Raynor (Vulture).
            uid::VULTURE | 19 => upgrade::ION_THRUSTERS,
            uid::OVERLORD => upgrade::PNEUMATIZED_CARAPACE,
            uid::ZERGLING => upgrade::METABOLIC_BOOST,
            uid::HYDRALISK => upgrade::MUSCULAR_AUGMENTS,
            uid::ZEALOT => upgrade::LEG_ENHANCEMENTS,
            uid::SCOUT => upgrade::GRAVITIC_THRUSTERS,
            uid::SHUTTLE => upgrade::GRAVITIC_DRIVE,
            uid::OBSERVER => upgrade::GRAVITIC_BOOSTERS,
            uid::ULTRALISK => upgrade::ANABOLIC_SYNTHESIS,
            // Hunter Killer 53, Yggdrasill 57, Fenix (Zealot) 77, Mojo 80,
            // Artanis 88 and every Lurker.
            53 | 57 | 77 | 80 | 88 | uid::LURKER => return true,
            _ => return false,
        };
        self.players
            .get(u.owner as usize)
            .is_some_and(|p| p.upgrade_level(id) > 0)
    }

    /// Movement modifier count (OpenBW `get_modified_unit_speed`): +1 for
    /// Stim, +1 for a speed upgrade, -1 for Ensnare.  Positive: speed x1.5
    /// (at least 3 1/3 px per frame), acceleration and turn rate x2;
    /// negative: speed / 2, acceleration and turn rate - 1/4.
    pub fn movement_mods(&self, i: usize) -> i32 {
        let u = &self.units[i];
        i32::from(u.status.stim_ticks > 0) + i32::from(u.flag_29 || self.has_speed_upgrade(i))
            - i32::from(u.status.ensnare_ticks > 0)
    }

    /// Flingy acceleration or turn rate under the movement modifiers.
    fn modified_rate(&self, i: usize, base: i32) -> i32 {
        match self.movement_mods(i) {
            m if m > 0 => base * 2,
            m if m < 0 => base - base / 4,
            _ => base,
        }
    }

    // ------------------------------------------------------------------
    // iscript animation VM integration
    // ------------------------------------------------------------------

    /// Start an animation and execute the script's first tick immediately.
    /// The retail engine does the same from `FUN_7a1148f0`, which is why a
    /// Walking script's first `move` takes effect on the tick the order is
    /// issued.
    fn start_animation(&mut self, i: usize, anim: usize) {
        let ty = self.data.unit(self.units[i].type_id).clone();
        let Some(iscript) = self.data.unit_iscript_id(&ty) else {
            return;
        };
        let direction = self.units[i].facing;
        if let Some(st) = self.data.iscript.start_animation(iscript, anim, direction) {
            self.units[i].anim = Some(st);
            self.tick_animation(i);
        }
    }

    /// Advance the current animation by one logical tick and apply gameplay
    /// events (weapon release, spell release, order signals, turns).
    fn tick_animation(&mut self, i: usize) {
        let Some(mut st) = self.units[i].anim.take() else {
            return;
        };
        let direction = self.units[i].facing;
        let (target_distance, target_angle) = match self.units[i].target {
            Some(id) => match self.index_of(id) {
                Some(tidx) => {
                    let dx = self.units[tidx].x - self.units[i].x;
                    let dy = self.units[tidx].y - self.units[i].y;
                    let d = crate::sim::isqrt((dx as i64) * (dx as i64) + (dy as i64) * (dy as i64))
                        as i32
                        / fp(1);
                    let a = direction_byte(dx, dy) as i32 * 8;
                    (Some(d), Some(a))
                }
                None => (None, None),
            },
            None => (None, None),
        };
        let mut ctx = AnimContext {
            direction,
            tileset: 0,
            rng: self.rng as u32,
            target_distance,
            target_angle,
            lifted: false,
            powerup_picked: false,
            hallucination: self.units[i].status.hallucination,
        };
        let events = self.data.iscript.tick_anim(&mut st, &mut ctx);
        self.rng = (self.rng & 0xffff_ffff_0000_0000) | ctx.rng as u64;
        let finished = st.ended || st.ignore_rest;
        if finished {
            self.units[i].anim = None;
        } else {
            self.units[i].anim = Some(st);
        }
        for ev in events {
            self.handle_anim_event(i, ev);
        }
    }

    fn handle_anim_event(&mut self, i: usize, ev: AnimEvent) {
        match ev {
            AnimEvent::Attack {
                weapon,
                melee,
                ground,
            } => self.animation_attack(i, weapon, melee, ground),
            AnimEvent::AttackShift(distance) => self.animation_attack_shift(i, distance),
            AnimEvent::CastSpell => self.animation_cast(i),
            // `orderdone <flag>` only clears bits of the unit's order
            // signal (the counterpart of `sigorder`); it does not finish the
            // order.  Treating it as completion made the Medic, whose
            // Walking script starts with `orderdone 1`, drop every order
            // the moment it started to walk.
            AnimEvent::OrderDone(_) => {}
            AnimEvent::SetDirection(d) => {
                self.units[i].facing = d.clamp(0, 255) as u8;
            }
            AnimEvent::Turn(d) => {
                self.units[i].facing = (self.units[i].facing as i32 + d).rem_euclid(256) as u8;
            }
            AnimEvent::Flip(_)
            | AnimEvent::Frame { .. }
            | AnimEvent::Sound(_)
            | AnimEvent::RandomSound(_)
            | AnimEvent::Move(_)
            | AnimEvent::SetFlingSpeed(_)
            | AnimEvent::Signal(_)
            | AnimEvent::Overlay { .. }
            | AnimEvent::End
            | AnimEvent::IgnoreRest => {}
        }
    }

    fn animation_attack(&mut self, i: usize, weapon: Option<u8>, melee: bool, ground: bool) {
        let Some(target) = self.units[i].target else {
            return;
        };
        let Some(tidx) = self.index_of(target) else {
            return;
        };
        if !self.units[tidx].alive {
            return;
        }
        let ty = self.data.unit(self.units[i].type_id).clone();
        let selected = if let Some(id) = weapon {
            self.data.weapon(id).cloned().map(|w| (id, w))
        } else if ground {
            self.data
                .ground_weapon(&ty)
                .cloned()
                .map(|w| (self.data.ground_weapon_id(&ty), w))
        } else {
            self.weapon_for(i, tidx)
        };
        let Some((weapon_id, w)) = selected else {
            self.special_attack(i, tidx);
            return;
        };
        if melee {
            self.fire_melee(i, tidx, weapon_id, &w);
        } else {
            self.fire(i, tidx, weapon_id, &w);
        }
        self.units[i].attacks_fired = self.units[i].attacks_fired.saturating_add(1);
    }

    /// `attkshiftproj`: one bullet of the ground weapon `distance` px ahead
    /// of the attacker, along the line to its target.  The Firebat's attack
    /// runs it three times (24, 52 and 80 px): each flame is an enemy-only
    /// splash of the full weapon damage, so a target next to the Firebat is
    /// burnt by the first two.
    fn animation_attack_shift(&mut self, i: usize, distance: u8) {
        let Some(tidx) = self.units[i].target.and_then(|t| self.index_of(t)) else {
            return;
        };
        if !self.units[tidx].alive {
            return;
        }
        let ty = self.data.unit(self.units[i].type_id).clone();
        let Some(w) = self.data.ground_weapon(&ty).cloned() else {
            return;
        };
        let weapon_id = self.data.ground_weapon_id(&ty);
        self.units[i].cooldown = self.modified_cooldown(i, &w);
        self.units[i].attacks_fired = self.units[i].attacks_fired.saturating_add(1);
        if self.units[i].status.hallucination {
            return;
        }
        let (ax, ay) = (self.units[i].x, self.units[i].y);
        let (dx, dy) = (
            (self.units[tidx].x - ax) as i64,
            (self.units[tidx].y - ay) as i64,
        );
        let len = isqrt(dx * dx + dy * dy).max(1);
        let reach = distance as i64 * fp(1) as i64;
        let impact = (
            ax + (dx * reach / len) as i32,
            ay + (dy * reach / len) as i32,
        );
        let owner = self.units[i].owner;
        let target_id = self.units[tidx].id;
        self.instant_hit(owner, Some(target_id), i, weapon_id, &w, impact);
    }

    fn animation_cast(&mut self, i: usize) {
        let order = self.units[i].order.clone();
        if let Order::Cast { spell, target } = order {
            let id = self.units[i].id;
            // Unrecorded: the `Order::Cast` that led here is the logged act.
            let _ = self.cast_unrecorded(id, spell, target);
            self.order_done(i);
        }
    }

    fn fire_melee(&mut self, attacker: usize, target: usize, weapon_id: u8, weapon: &WeaponType) {
        if self.units[attacker].status.hallucination {
            self.units[attacker].cooldown = self.modified_cooldown(attacker, weapon);
            return;
        }
        let owner = self.units[attacker].owner;
        let pos = (self.units[target].x, self.units[target].y);
        self.resolve_hit_type(
            owner,
            Some(self.units[target].id),
            attacker,
            weapon_id,
            weapon,
            pos,
            1,
            1,
        );
        self.units[attacker].cooldown = self.modified_cooldown(attacker, weapon);
    }

    fn ensure_walking_animation(&mut self, i: usize) {
        // Any resting state starts the walk.  Only Init and the *ToIdle
        // states used to qualify, so a unit whose script sat in Init (map
        // placed units) or IsWorking (an SCV back from mining) kept
        // `move_speed` 0 and crawled on the walk-cycle fallback (Wasteland,
        // 2026-11-06: an SCV ordered to build moved ~0.5 px/tick instead of
        // 5; a Marine group stood still for 1,500 ticks).
        let current = self.units[i].anim.as_ref().map(|a| a.anim as usize);
        let busy = matches!(
            current,
            Some(
                crate::iscript::ANIM_WALKING
                    | crate::iscript::ANIM_DEATH
                    | crate::iscript::ANIM_GND_ATTACK_INIT
                    | crate::iscript::ANIM_AIR_ATTACK_INIT
                    | crate::iscript::ANIM_GND_ATTACK_REPEAT
                    | crate::iscript::ANIM_AIR_ATTACK_REPEAT
                    | crate::iscript::ANIM_CAST_SPELL
                    | crate::iscript::ANIM_BURROW
                    | crate::iscript::ANIM_UNBURROW
                    | crate::iscript::ANIM_LANDING
                    | crate::iscript::ANIM_LIFT_OFF
                    | crate::iscript::ANIM_WARP_IN
            )
        );
        if !busy {
            self.start_animation(i, ANIM_WALKING);
        }
    }

    fn start_attack_animation(&mut self, i: usize, flying_target: bool) -> bool {
        let repeat = self.units[i].attacks_fired > 0;
        let anim = match (flying_target, repeat) {
            (true, false) => ANIM_AIR_ATTACK_INIT,
            (true, true) => ANIM_AIR_ATTACK_REPEAT,
            (false, false) => ANIM_GND_ATTACK_INIT,
            (false, true) => ANIM_GND_ATTACK_REPEAT,
        };
        let ty = self.data.unit(self.units[i].type_id);
        let Some(iscript) = self.data.unit_iscript_id(ty) else {
            return false;
        };
        if !self.data.iscript.animation_has_attack(iscript, anim) {
            return false;
        }
        self.start_animation(i, anim);
        true
    }

    // ------------------------------------------------------------------
    // Economy
    // ------------------------------------------------------------------

    /// Another worker is mining `resource` right now.
    fn resource_busy(&self, i: usize, resource: u32) -> bool {
        self.units.iter().enumerate().any(|(j, u)| {
            j != i && u.alive && u.gather_timer > 0 && u.order == Order::Gather { resource }
        })
    }

    /// Nearest mineral field within 8 tiles of the worker that nobody is
    /// mining (OpenBW `find_mineral_field(.., 32 * 8, true)`).
    fn free_mineral_near(&self, i: usize, current: usize) -> Option<u32> {
        let (x, y) = (self.units[i].x, self.units[i].y);
        self.units
            .iter()
            .enumerate()
            .filter(|(j, u)| {
                *j != current
                    && u.alive
                    && is_resource_type(u.type_id)
                    && !is_gas_resource(u.type_id)
                    && u.type_id != uid::VESPENE_GEYSER
                    && u.resource_amount > 0
                    && (u.x - x).abs() <= fp(256)
                    && (u.y - y).abs() <= fp(256)
                    && !self.resource_busy(i, u.id)
            })
            .min_by_key(|(j, u)| {
                let (dx, dy) = ((u.x - x) as i64 / 256, (u.y - y) as i64 / 256);
                (dx * dx + dy * dy, *j)
            })
            .map(|(_, u)| u.id)
    }

    /// Nearest live mineral field within 8 tiles of worker `i` other than
    /// `current`, busy or not.
    fn nearest_mineral_near(&self, i: usize, current: usize) -> Option<u32> {
        let (x, y) = (self.units[i].x, self.units[i].y);
        self.units
            .iter()
            .enumerate()
            .filter(|(j, u)| {
                *j != current
                    && u.alive
                    && is_resource_type(u.type_id)
                    && !is_gas_resource(u.type_id)
                    && u.type_id != uid::VESPENE_GEYSER
                    && u.resource_amount > 0
                    && (u.x - x).abs() <= fp(256)
                    && (u.y - y).abs() <= fp(256)
            })
            .min_by_key(|(j, u)| {
                let (dx, dy) = ((u.x - x) as i64 / 256, (u.y - y) as i64 / 256);
                (dx * dx + dy * dy, *j)
            })
            .map(|(_, u)| u.id)
    }

    fn order_gather(&mut self, i: usize, resource: u32) {
        let Some(ridx) = self.index_of(resource) else {
            self.order_done(i);
            return;
        };
        let mined_out = !self.units[ridx].alive || self.units[ridx].resource_amount <= 0;
        if mined_out {
            // Retail `order_MoveToMinerals`: a worker whose field is gone
            // takes the nearest other field within 8 tiles (FACT, Backwater
            // Station 2026-11-06: without it every SCV went idle once the
            // small base fields ran out, freezing income at tick ~3,500).
            let was_mineral = !is_gas_resource(self.units[ridx].type_id)
                && self.units[ridx].type_id != uid::VESPENE_GEYSER
                && !is_gas_building(self.units[ridx].type_id);
            match was_mineral
                .then(|| self.nearest_mineral_near(i, ridx))
                .flatten()
            {
                Some(next) => {
                    self.units[i].order = Order::Gather { resource: next };
                    self.units[i].resource_target = Some(next);
                }
                None => {
                    self.units[i].resource_target = None;
                    self.order_done(i);
                }
            }
            return;
        }
        // Raw geysers require a completed Refinery/Assimilator/Extractor.
        // The worker then gathers from that building, exactly as in the
        // retail game.
        if self.units[ridx].type_id == uid::VESPENE_GEYSER {
            let (gx, gy) = (self.units[ridx].x, self.units[ridx].y);
            let refinery = self.units.iter().position(|u| {
                u.alive
                    && u.completed
                    && is_gas_building(u.type_id)
                    && (u.x - gx).abs() < fp(64)
                    && (u.y - gy).abs() < fp(64)
            });
            if let Some(r) = refinery {
                self.issue(
                    self.units[i].id,
                    Order::Gather {
                        resource: self.units[r].id,
                    },
                )
                .ok();
            } else {
                self.order_done(i);
            }
            return;
        }
        // Mining starts on contact: the units.dat boxes within
        // `HARVEST_CONTACT_PX` (the 80 px centre distance shortened every
        // trip; Wasteland recording: 5 SCVs bring 3.98 minerals/s, ours 4.6).
        // Refineries too: an 80 px centre range left a queued SCV standing
        // 81.6 px away, held off by the footprint, idle until nudged (Boot
        // Camp: deliveries every 35 / 71 frames instead of the recording's
        // three-SCV rhythm).
        // Once mining has started it runs to the end: a worker nudged off
        // contact by another one (it is inside the Refinery, or busy at its
        // field) restarted its 37 / 75 frames and the queue stalled.
        if self.units[i].gather_timer == 0 && self.box_gap_px(i, ridx) > HARVEST_CONTACT_PX {
            let (rx, ry) = (self.units[ridx].x, self.units[ridx].y);
            // Walk on until the boxes touch (a 64 px stop left workers
            // short of contact).
            self.move_towards(i, (rx / fp(1), ry / fp(1)), fp(4));
            return;
        }
        // One worker at a time per patch / refinery, and mining takes time
        // (OpenBW `try_gather_resource`, `order_MiningMinerals`: 75 frames;
        // `order_HarvestGas`: 37).  Without this every worker harvested on
        // arrival and income ran ~2.5x retail (session log 2026-11-04 (86)).
        let gas = is_gas_resource(self.units[ridx].type_id);
        if self.units[i].gather_timer == 0 {
            if self.resource_busy(i, resource) {
                // `order_MoveToMinerals`: try a free field within 8 tiles,
                // else wait for this one.
                if !gas && let Some(free) = self.free_mineral_near(i, ridx) {
                    self.units[i].order = Order::Gather { resource: free };
                }
                return;
            }
            // `0x005abe7f` writes 75 to CUnit+0x54 at MiningMinerals start;
            // this contact-based model uses the calibrated 93-frame dwell.
            self.units[i].gather_timer = if gas {
                GAS_MINING_FRAMES
            } else {
                MINING_FRAMES
            };
            self.units[i].resource_target = Some(resource);
            // The worker faces what it mines (its Fusion Cutter spark appears
            // 10 px ahead of it, weapons.dat row 14).
            let (dx, dy) = (
                self.units[ridx].x - self.units[i].x,
                self.units[ridx].y - self.units[i].y,
            );
            if dx != 0 || dy != 0 {
                self.units[i].facing = direction_byte(dx, dy);
            }
            self.units[i].path.clear();
            self.units[i].path_goal = None;
            return;
        }
        if self.units[i].gather_timer > 1 {
            self.units[i].gather_timer -= 1;
            return;
        }
        self.units[i].gather_timer = 0;
        // Harvest.
        let kind = if gas {
            CarryKind::Gas
        } else {
            CarryKind::Minerals
        };
        let amount = if kind == CarryKind::Gas {
            GAS_PER_TRIP
        } else {
            MINERALS_PER_TRIP
        };
        let taken = amount.min(self.units[ridx].resource_amount);
        self.units[ridx].resource_amount -= taken;
        // A mineral field mined out disappears (retail removes the sprite and
        // its footprint); geysers / refineries stay and yield less gas.
        if !gas && self.units[ridx].resource_amount <= 0 {
            self.units[ridx].alive = false;
            self.units[ridx].order = Order::Die;
        }
        self.units[i].carry = taken;
        self.units[i].carry_kind = kind;
        self.units[i].resource_target = Some(resource);
        self.units[i].order = Order::ReturnCargo { depot: 0 };
        if gas {
            self.place_at_gas_exit(i, ridx);
        }
    }

    /// Retail `order_HarvestGas`: the worker that was hidden inside the gas
    /// building reappears on the building's edge, on the side of the depot it
    /// will return to, already facing it.
    fn place_at_gas_exit(&mut self, i: usize, ridx: usize) {
        let Some(d) = self.nearest_depot(i) else {
            return;
        };
        let (rx, ry) = (self.units[ridx].x / fp(1), self.units[ridx].y / fp(1));
        let (dx, dy) = (
            (self.units[d].x / fp(1) - rx) as i64,
            (self.units[d].y / fp(1) - ry) as i64,
        );
        if dx == 0 && dy == 0 {
            return;
        }
        let rd = self.data.unit(self.units[ridx].type_id).dimensions;
        let wd = self.data.unit(self.units[i].type_id).dimensions;
        // Half extents of the building's box grown by the worker's own.
        let hw = (if dx >= 0 {
            rd.right + wd.left
        } else {
            rd.left + wd.right
        }) as i64
            + 1;
        let hh = (if dy >= 0 {
            rd.bottom + wd.top
        } else {
            rd.top + wd.bottom
        }) as i64
            + 1;
        // Where the line to the depot leaves that box.
        let (ex, ey) = if dx.abs() * hh >= dy.abs() * hw {
            (hw * dx.signum(), dy * hw / dx.abs())
        } else {
            (dx * hh / dy.abs(), hh * dy.signum())
        };
        let (nx, ny) = (rx + ex as i32, ry + ey as i32);
        if self.terrain.walkable_px(nx, ny) {
            self.units[i].x = nx * fp(1);
            self.units[i].y = ny * fp(1);
            self.units[i].path.clear();
            self.units[i].path_goal = None;
        }
        self.units[i].facing = direction_byte(dx as i32, dy as i32);
    }

    fn order_return(&mut self, i: usize, depot: u32) {
        let kind = self.units[i].carry_kind;
        if kind == CarryKind::None {
            if let Some(res) = self.units[i].resource_target {
                self.units[i].order = Order::Gather { resource: res };
            } else {
                self.order_done(i);
            }
            return;
        }
        let depot_idx = if depot != 0 {
            self.index_of(depot)
        } else {
            self.nearest_depot(i)
        };
        let Some(didx) = depot_idx else {
            self.order_done(i);
            return;
        };
        // Cargo is handed in on contact with the depot's box (it was a
        // 96 px centre distance; see `HARVEST_CONTACT_PX`).
        if self.box_gap_px(i, didx) > HARVEST_CONTACT_PX {
            let (dx, dy) = (self.units[didx].x, self.units[didx].y);
            self.move_towards(i, (dx / fp(1), dy / fp(1)), fp(4));
            return;
        }
        let owner = self.units[i].owner as usize;
        match kind {
            CarryKind::Minerals => {
                self.players[owner].minerals += self.units[i].carry;
                self.players[owner].minerals_gathered += self.units[i].carry;
            }
            CarryKind::Gas => {
                self.players[owner].gas += self.units[i].carry;
                self.players[owner].gas_gathered += self.units[i].carry;
            }
            CarryKind::None => {}
        }
        self.units[i].carry = 0;
        self.units[i].carry_kind = CarryKind::None;
        if let Some(res) = self.units[i].resource_target {
            self.units[i].order = Order::Gather { resource: res };
        } else {
            self.order_done(i);
        }
    }

    fn nearest_depot(&self, i: usize) -> Option<usize> {
        let owner = self.units[i].owner;
        self.units
            .iter()
            .enumerate()
            .filter(|(_, u)| {
                u.alive
                    && u.owner == owner
                    && u.completed
                    && (u.type_id == uid::COMMAND_CENTER
                        || u.type_id == uid::NEXUS
                        || u.type_id == uid::HATCHERY
                        || u.type_id == uid::LAIR
                        || u.type_id == uid::HIVE)
            })
            .min_by_key(|(_, u)| {
                let dx = (u.x - self.units[i].x) as i64;
                let dy = (u.y - self.units[i].y) as i64;
                dx * dx + dy * dy
            })
            .map(|(idx, _)| idx)
    }

    fn order_build(&mut self, i: usize, unit_type: u16, pos: (i32, i32)) {
        // Add-ons are parent-built, never worker-built; building morphs keep
        // the source building and never create a second structure.
        if addon_parent(unit_type).is_some() || building_morph_source(unit_type).is_some() {
            self.order_done(i);
            return;
        }
        // Zerg structures (except the Hatchery, which creates creep) must be
        // placed on creep; Protoss structures must be inside a Pylon field.
        if is_zerg_building(unit_type)
            && !matches!(unit_type, uid::HATCHERY | uid::EXTRACTOR)
            && !self.has_creep_px(pos.0, pos.1)
        {
            self.order_done(i);
            return;
        }
        if is_protoss_building(unit_type)
            && !matches!(unit_type, uid::NEXUS | uid::PYLON)
            && !self.has_power_px(self.units[i].owner, pos.0, pos.1)
        {
            self.order_done(i);
            return;
        }
        let ty = self.data.unit(unit_type).clone();
        let pos_fp = (pos.0 * fp(1), pos.1 * fp(1));
        if self.distance_to_point(i, pos_fp) > fp(64) {
            self.move_towards(i, pos, fp(32));
            return;
        }
        let owner = self.units[i].owner as usize;
        if !self.has_requirements(owner as u8, unit_type) {
            self.order_done(i);
            return;
        }
        if !self.can_afford(owner as u8, unit_type) {
            self.order_done(i);
            return;
        }
        {
            let p = &mut self.players[owner];
            p.pay_minerals(ty.mineral_cost as i32);
            p.pay_gas(ty.gas_cost as i32);
        }
        let worker_is_drone = self.units[i].type_id == uid::DRONE;
        let Ok(bidx) = self.spawn_px(owner, unit_type, pos.0, pos.1) else {
            self.order_done(i);
            return;
        };
        self.units[bidx].a13_clear_completed_flag();
        self.units[bidx].credited = false;
        self.units[bidx].build_total = build_ticks(ty.build_time);
        self.units[bidx].build_remaining = self.units[bidx].build_total;
        self.units[bidx].hp = (ty.hit_points_raw as i32 / 10).max(1);
        if worker_is_drone && is_zerg_building(unit_type) {
            // The Drone is consumed by the structure it starts.
            self.kill_unit(i);
        } else if self.units[i].type_id == uid::SCV && !is_protoss_building(unit_type) {
            // Retail Terran construction: the SCV stays on the building
            // (`Order::Construct`) until it completes.
            let bid = self.units[bidx].id;
            self.units[bidx].builder = self.units[i].id;
            self.units[i].order = Order::Construct { building: bid };
            self.units[i].order_row = 33;
        } else {
            self.order_done(i);
        }
    }

    /// SCV on a building it constructs: hold the order until the building
    /// completes or dies, then go idle (resume it by right-clicking the
    /// unfinished building).
    fn order_construct(&mut self, i: usize, building: u32) {
        let Some(b) = self.index_of(building) else {
            self.order_done(i);
            return;
        };
        if !self.units[b].alive || self.units[b].completed {
            if self.units[b].builder == self.units[i].id {
                self.units[b].builder = 0;
            }
            self.order_done(i);
            return;
        }
        // The SCV works its way around the structure: a spot on the edge of
        // the building's box, changing every `CONSTRUCT_HOP_TICKS` (Boot Camp
        // recording 175-199 s: the SCV welds the depot from its left, front
        // and right sides, never from its centre).  Derived from the tick and
        // the unit id, so no extra state and no RNG draw.
        let ty = self.data.unit(self.units[b].type_id);
        let (hw, hh) = (
            ty.dimensions.left.max(ty.dimensions.right).max(8) as i64,
            ty.dimensions.top.max(ty.dimensions.bottom).max(8) as i64,
        );
        let hop = self.tick / CONSTRUCT_HOP_TICKS;
        let h = (hop as i64)
            .wrapping_mul(2654435761)
            .wrapping_add(self.units[i].id as i64 * 40503)
            .rem_euclid(1 << 20);
        let (ex, ey) = match h % 3 {
            0 => (-hw, (h / 3).rem_euclid(2 * hh) - hh),
            1 => (hw, (h / 3).rem_euclid(2 * hh) - hh),
            _ => ((h / 3).rem_euclid(2 * hw) - hw, hh),
        };
        let (bx, by) = (self.units[b].x / fp(1), self.units[b].y / fp(1));
        let spot = (bx + ex as i32, by + ey as i32);
        if self.distance(i, b) > fp(96) {
            self.move_towards(i, spot, fp(8));
            return;
        }
        self.units[b].builder = self.units[i].id;
        if self.distance_to_point(i, (spot.0 * fp(1), spot.1 * fp(1))) > fp(6) {
            self.move_towards(i, spot, fp(4));
        } else {
            self.units[i].path.clear();
            self.units[i].path_goal = None;
            // Welding: the SCV faces the structure.
            let (dx, dy) = (
                self.units[b].x - self.units[i].x,
                self.units[b].y - self.units[i].y,
            );
            if dx != 0 || dy != 0 {
                self.units[i].facing = direction_byte(dx, dy);
            }
        }
    }

    /// SCV repair.  Rate is the documented `0.9 * maxHP / build_time` per
    /// second; cost is about one third of the unit's mineral cost over the
    /// full hit-point pool, accumulated in fractional minerals per SCV.
    fn order_repair(&mut self, i: usize, target: u32) {
        let Some(t) = self.index_of(target) else {
            self.order_done(i);
            return;
        };
        if !self.units[t].alive {
            self.order_done(i);
            return;
        }
        let ty = self.data.unit(self.units[t].type_id).clone();
        if !ty.is_mechanical() && !ty.is_building() {
            self.order_done(i);
            return;
        }
        let max = ty.hit_points_raw as i32;
        if self.units[t].hp >= max {
            self.order_done(i);
            return;
        }
        if t == i {
            self.order_done(i);
            return;
        }
        let dist = self.distance(i, t);
        if dist > fp(48) {
            let (tx, ty) = (self.units[t].x, self.units[t].y);
            self.move_towards(i, (tx / fp(1), ty / fp(1)), fp(32));
            return;
        }
        let owner = self.units[i].owner as usize;
        // Retail repair rate: 0.9 * maxHP / build_time_seconds per SCV.
        // `build_time` is in 1/15 s and the logical rate is 24 Hz, so the
        // per-tick raw HP is maxHP * 27 / (build_time * 48).
        let build_time = ty.build_time.max(1) as i64;
        let max_raw = max.max(1) as i64;
        let mut heal = (max_raw * 27 / (build_time * 48)).max(1) as i32;
        heal = heal.min(max - self.units[t].hp);
        if heal <= 0 {
            self.order_done(i);
            return;
        }
        // Repair costs about one third of the unit's mineral cost over its
        // whole hit-point pool.  Fractional minerals are kept per SCV.
        let debt = (ty.mineral_cost as i64 * heal as i64 * 256) / (3 * max_raw);
        self.units[i].repair_debt = self.units[i].repair_debt.saturating_add(debt as i32);
        let payable = self.units[i].repair_debt / 256;
        if payable > self.players[owner].minerals {
            self.order_done(i);
            return;
        }
        if payable > 0 {
            self.players[owner].pay_minerals(payable);
            self.units[i].repair_debt %= 256;
        }
        self.units[t].hp += heal;
    }

    // ------------------------------------------------------------------
    // Transports
    // ------------------------------------------------------------------

    /// Gap in px between the units.dat boxes of units `a` and `b` (0 when
    /// they touch or overlap): the larger of the x and y separations.
    fn box_gap_px(&self, a: usize, b: usize) -> i32 {
        let bx = |i: usize| {
            let u = &self.units[i];
            let (x, y) = (u.x / fp(1), u.y / fp(1));
            self.data.unit(u.type_id).dimensions.rect_at((x, y))
        };
        let (a0, b0) = (bx(a), bx(b));
        let dx = (b0.0 - a0.2).max(a0.0 - b0.2).max(0);
        let dy = (b0.1 - a0.3).max(a0.1 - b0.3).max(0);
        dx.max(dy)
    }

    /// OpenBW `order_PickupTransport`: fly to a unit that wants to board
    /// until the boxes touch; done when it is aboard or gave up.
    fn order_pickup_transport(&mut self, i: usize, unit: u32) {
        let waiting = self.index_of(unit).filter(|&p| {
            self.units[p].alive
                && self.units[p].in_transport.is_none()
                && self.units[p].order
                    == Order::EnterTransport {
                        transport: self.units[i].id,
                    }
        });
        let Some(p) = waiting else {
            self.units[i].path.clear();
            self.units[i].path_goal = None;
            self.order_done(i);
            return;
        };
        if self.box_gap_px(i, p) > 1 {
            let (px, py) = (self.units[p].x / fp(1), self.units[p].y / fp(1));
            self.move_towards(i, (px, py), fp(1));
        } else {
            self.units[i].path.clear();
            self.units[i].path_goal = None;
        }
    }

    fn order_enter_transport(&mut self, i: usize, transport: u32) {
        let Some(tidx) = self.index_of(transport) else {
            self.order_done(i);
            return;
        };
        if !self.units[tidx].alive
            || !self.units[tidx].completed
            || tidx == i
            || self.units[tidx].owner != self.units[i].owner
            || self.data.unit(self.units[i].type_id).is_building()
        {
            self.order_done(i);
            return;
        }
        // A transport that can move comes to fetch its passenger (OpenBW
        // `order_EnterTransport` queues `PickupTransport` on it) and the
        // two board when their boxes touch (1 px).  A Bunker stays put and
        // takes units within 8 px of its box (a centre distance of 48 px
        // left a Marine stuck on the corner of a Bunker it approached
        // diagonally).
        let mobile = !self.data.unit(self.units[tidx].type_id).is_building();
        if mobile && matches!(self.units[tidx].order, Order::Idle) {
            self.units[tidx].order = Order::PickupTransport {
                unit: self.units[i].id,
            };
            self.units[tidx].order_row = 94;
        }
        // OpenBW `order_EnterTransport`: board when the boxes are within
        // 1 px.  A Bunker keeps 8 px: the engine's building footprint holds
        // walkers a few pixels off its box, and at 1 px Marines stalled on
        // its corners.
        let reach = if mobile { 1 } else { 8 };
        if self.box_gap_px(i, tidx) > reach {
            let (tx, ty) = (self.units[tidx].x, self.units[tidx].y);
            let stop = if mobile { fp(1) } else { fp(24) };
            self.move_towards(i, (tx / fp(1), ty / fp(1)), stop);
            return;
        }
        let target_type = self.units[tidx].type_id;
        let owner = self.units[i].owner;

        // Nydus Canal: the unit travels to the paired canal instead of being
        // stored.  Order 45 (Enter Nydus Canal) uses this path.
        if is_nydus_canal_type(target_type) {
            let paired = self.units[tidx]
                .nydus_exit
                .and_then(|id| self.index_of(id))
                .filter(|&e| self.units[e].alive && self.units[e].completed);
            let exit = paired.or_else(|| {
                self.units.iter().position(|u| {
                    u.alive
                        && u.completed
                        && u.owner == owner
                        && is_nydus_canal_type(u.type_id)
                        && u.id != transport
                })
            });
            if let Some(e) = exit {
                let (ex, ey) = self.exit_position(e);
                self.units[i].x = ex;
                self.units[i].y = ey;
                self.units[i].path.clear();
                self.units[i].path_goal = None;
                self.order_done(i);
            } else {
                self.order_done(i);
            }
            return;
        }

        let moving_id = self.units[i].id;
        if target_type == uid::BUNKER {
            // Four infantry slots, no vehicles or workers.
            if !is_bunker_infantry(self.units[i].type_id) || self.units[tidx].cargo.len() >= 4 {
                self.order_done(i);
                return;
            }
        } else {
            let capacity = self.data.unit(target_type).space_provided as i32;
            // Translation batch 33 (`0x005d5ea0`): cargo space already used.
            let used = self.a33_cargo_used_space(transport);
            let need = self.data.unit(self.units[i].type_id).space_required as i32;
            if capacity - used < need {
                self.order_done(i);
                return;
            }
        }
        self.units[tidx].cargo.push(moving_id);
        self.units[i].mark_in_transport(transport);
        self.units[i].set_in_bunker(target_type == uid::BUNKER);
        self.units[i].alive = false; // hidden/loaded
        if target_type == uid::BUNKER {
            self.units[i].x = self.units[tidx].x;
            self.units[i].y = self.units[tidx].y;
        }
        self.units[i].order = Order::Idle;
    }

    /// Unload one unit from a Bunker / transport (retail: a click on its
    /// cargo slot).  Recorded (`RecAct::UnloadOne`, save tag 11).
    pub fn unload_unit(&mut self, transport_id: u32, unit_id: u32) -> Result<(), SimError> {
        let i = self.index_of(transport_id).ok_or(SimError::BadUnit)?;
        // Translation batch 27 (`0x005ad780`): detach the cargo node and
        // clear the unit's containment back-link.
        if !self.a27_remove_cargo(transport_id, unit_id) {
            return Err(SimError::InvalidTarget);
        }
        let (x, y) = self.exit_position(i);
        if let Some(j) = self.index_of(unit_id) {
            self.units[j].alive = true;
            self.units[j].clear_transport_link();
            self.units[j].set_in_bunker(false);
            self.units[j].target = None;
            self.units[j].order_wait = 0;
            self.units[j].x = x;
            self.units[j].y = y;
        }
        let tick = self.tick;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                tick,
                crate::replay::RecAct::UnloadOne {
                    transport: transport_id,
                    unit: unit_id,
                },
            );
        }
        Ok(())
    }

    /// Unload (OpenBW `order_Unload` / `unit_unload`).  A Bunker empties at
    /// once.  A transport lets one unit out every 15 frames; each one
    /// lands with its weapon on cooldown, so a drop does not fire the
    /// instant it touches the ground.
    fn order_unload(&mut self, i: usize) {
        let building = self.data.unit(self.units[i].type_id).is_building();
        if !building && self.units[i].order_wait > 0 {
            self.units[i].order_wait -= 1;
            return;
        }
        let (cx, cy) = (self.units[i].x, self.units[i].y);
        let count = if building {
            self.units[i].cargo.len()
        } else {
            1.min(self.units[i].cargo.len())
        };
        for _ in 0..count {
            let id = self.units[i].cargo.remove(0);
            let Some(j) = self.index_of(id) else {
                continue;
            };
            // OpenBW `unit_unload`: the nearest free spot to the container
            // (`find_unit_placement`), not one shared exit point.
            let (x, y) = self
                .free_spot_near(j, (cx, cy))
                .unwrap_or_else(|| self.exit_position(i));
            self.units[j].alive = true;
            self.units[j].clear_transport_link();
            self.units[j].set_in_bunker(false);
            self.units[j].x = x;
            self.units[j].y = y;
            if !building {
                let ty = self.data.unit(self.units[j].type_id).clone();
                let cooldown = [self.data.ground_weapon(&ty), self.data.air_weapon(&ty)]
                    .into_iter()
                    .flatten()
                    .map(|w| self.modified_cooldown(j, w))
                    .max()
                    .unwrap_or(0);
                self.units[j].cooldown = self.units[j].cooldown.max(cooldown);
            }
        }
        if building || self.units[i].cargo.is_empty() {
            self.units[i].order_wait = 0;
            self.order_done(i);
        } else {
            self.units[i].order_wait = TRANSPORT_UNLOAD_FRAMES;
        }
    }

    // ------------------------------------------------------------------
    // Spells and modes
    // ------------------------------------------------------------------

    /// `0x005a3c90` / `0x005b7370`: unburrow clears the hidden/burrow state
    /// and the sprite visibility bit; the Rust model clears
    /// `status.burrowed`.
    fn order_burrow(&mut self, i: usize, burrow: bool) {
        if burrow {
            if !self.players[self.units[i].owner as usize].has_tech(tech_id::BURROWING) {
                self.order_done(i);
                return;
            }
            self.units[i].status.burrowed = true;
        } else {
            self.units[i].status.burrowed = false;
        }
        self.order_done(i);
    }

    /// Siege Mode and back (OpenBW `order_Sieging` / `order_Unsieging`).
    /// Sieging: the tank stops, becomes the sieged type at once and is busy
    /// for the 64 frames of its deploy animation (iscript 92 Init: `wait 1`,
    /// five frames of 5, `wait 38`, `sigorder 1`).  Unsieging: it stays
    /// sieged for the 63 frames of SpecialState2 (`wait 38`, five frames of
    /// 5, `sigorder 1`) and only then becomes the tank again.  It cannot
    /// fire or take another order meanwhile (`order_wait` counts down; 1
    /// marks the last frame).
    fn order_siege(&mut self, i: usize, siege: bool) {
        if !self.players[self.units[i].owner as usize].has_tech(tech_id::TANK_SIEGE_MODE) {
            self.units[i].order_wait = 0;
            self.order_done(i);
            return;
        }
        let ty = self.units[i].type_id;
        let wait = self.units[i].order_wait;
        if siege {
            if ty == uid::SIEGE_TANK {
                self.units[i].path.clear();
                self.units[i].path_goal = None;
                self.units[i].target = None;
                // OpenBW `order_Sieging`: hull and turret first turn to the
                // type's default direction (`units.dat` 12, heading 96).
                let rate = self
                    .data
                    .flingy(self.data.unit(ty).flingy)
                    .map_or(255, |f| f.turn_rate as i32)
                    .max(1);
                let cur = self.units[i].facing as i32;
                let mut diff = (SIEGE_HEADING - cur).rem_euclid(256);
                if diff > 128 {
                    diff -= 256;
                }
                self.units[i].facing = (cur + diff.clamp(-rate, rate)).rem_euclid(256) as u8;
                if self.units[i].facing as i32 != SIEGE_HEADING
                    || self.units[i].turret_facing as i32 != SIEGE_HEADING
                {
                    return;
                }
                // A tank that deploys on top of a building is destroyed.
                if self.overlaps_grounded_building(i) {
                    self.kill_unit(i);
                    return;
                }
                self.units[i].status.sieged = true;
                self.units[i].type_id = uid::SIEGE_MODE;
                self.units[i].order_wait = SIEGE_FRAMES + 1;
                return;
            }
            if ty == uid::SIEGE_MODE && wait > 1 {
                self.units[i].order_wait = wait - 1;
                return;
            }
        } else if ty == uid::SIEGE_MODE {
            if wait == 0 {
                self.units[i].target = None;
                // The cannon swings back to its rest heading first.
                if self.units[i].turret_facing as i32 != SIEGED_TURRET_HEADING {
                    return;
                }
                self.units[i].order_wait = UNSIEGE_FRAMES + 1;
                return;
            }
            if wait > 1 {
                self.units[i].order_wait = wait - 1;
                return;
            }
            self.units[i].status.sieged = false;
            self.units[i].type_id = uid::SIEGE_TANK;
        }
        self.units[i].order_wait = 0;
        self.order_done(i);
    }

    /// Whether unit `i`'s box overlaps a building on the ground.
    fn overlaps_grounded_building(&self, i: usize) -> bool {
        let r = self
            .data
            .unit(self.units[i].type_id)
            .dimensions
            .rect_at((self.units[i].x / fp(1), self.units[i].y / fp(1)));
        self.units.iter().enumerate().any(|(k, b)| {
            k != i && b.alive && !b.lifted && self.data.unit(b.type_id).is_building() && {
                let o = self
                    .data
                    .unit(b.type_id)
                    .dimensions
                    .rect_at((b.x / fp(1), b.y / fp(1)));
                r.0 <= o.2 && o.0 <= r.2 && r.1 <= o.3 && o.1 <= r.3
            }
        })
    }

    /// True while a tank is in the middle of its siege / unsiege animation.
    pub fn is_transforming(&self, i: usize) -> bool {
        self.units.get(i).is_some_and(|u| {
            u.alive && matches!(u.order, Order::Siege | Order::Unsiege) && u.order_wait > 0
        })
    }

    /// `0x005d4640`: Shield Battery target pre-check (different unit, same
    /// side and shielded).  A full pool is handled by the caller, which
    /// returns zero restored.
    pub fn can_shield_battery_target(&self, battery_id: u32, target_id: u32) -> bool {
        let Some(b) = self.index_of(battery_id) else {
            return false;
        };
        let Some(t) = self.index_of(target_id) else {
            return false;
        };
        if b == t || !self.units[b].alive || !self.units[t].alive {
            return false;
        }
        if self.units[b].owner != self.units[t].owner {
            return false;
        }
        self.data.unit(self.units[t].type_id).has_shields
    }

    /// Shield Battery recharge (orders 66/67, `0x005d4700`).  The classic ability converts
    /// one energy point into two plasma-shield points and can be repeated
    /// until the battery is empty.  Returns the number of shield points
    /// restored.
    pub fn shield_battery(&mut self, battery_id: u32, target_id: u32) -> Result<i32, SimError> {
        let restored = self.shield_battery_unrecorded(battery_id, target_id)?;
        if restored > 0
            && let Some(r) = self.recorder.as_mut()
        {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::ShieldBattery {
                    battery: battery_id,
                    target: target_id,
                },
            );
        }
        Ok(restored)
    }

    fn shield_battery_unrecorded(
        &mut self,
        battery_id: u32,
        target_id: u32,
    ) -> Result<i32, SimError> {
        let b = self.index_of(battery_id).ok_or(SimError::BadUnit)?;
        let t = self.index_of(target_id).ok_or(SimError::BadUnit)?;
        if self.units[b].type_id != uid::SHIELD_BATTERY || !self.units[b].completed {
            return Err(SimError::InvalidTarget);
        }
        if !self.units[b].alive || !self.units[t].alive {
            return Err(SimError::BadUnit);
        }
        if self.units[b].owner != self.units[t].owner {
            return Err(SimError::InvalidTarget);
        }
        if !self.can_shield_battery_target(battery_id, target_id) {
            return Err(SimError::InvalidTarget);
        }
        let ty = self.data.unit(self.units[t].type_id).clone();
        let max = fp(ty.shield_points as i32);
        if self.units[t].shields >= max {
            return Ok(0);
        }
        if self.units[b].energy <= 0 {
            return Err(SimError::NotEnoughEnergy);
        }
        let need = max - self.units[t].shields;
        let restore_fp = need.min(self.units[b].energy * fp(2));
        let energy_used = (restore_fp + fp(2) - 1) / fp(2);
        self.units[t].shields += restore_fp;
        self.units[b].energy -= energy_used;
        // `0x005d4700`: mark the target as recharged for this tick.
        self.units[t].shield_recharged = true;
        Ok(restore_fp / fp(1))
    }

    /// Recorded mutator (`RecAct::Cast`).
    pub fn cast(
        &mut self,
        caster_id: u32,
        spell: Spell,
        target: SpellTarget,
    ) -> Result<(), SimError> {
        self.cast_unrecorded(caster_id, spell, target)?;
        let tick = self.tick;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                tick,
                crate::replay::RecAct::Cast {
                    caster: caster_id,
                    spell,
                    target,
                },
            );
        }
        Ok(())
    }

    fn cast_unrecorded(
        &mut self,
        caster_id: u32,
        spell: Spell,
        target: SpellTarget,
    ) -> Result<(), SimError> {
        let i = self.index_of(caster_id).ok_or(SimError::BadUnit)?;
        let owner = self.units[i].owner as usize;
        if self.cheat_no_enemy_spells && self.players[owner].is_computer {
            return Err(SimError::MissingTech);
        }
        // A detached Comsat/Nuclear Silo is inactive.
        if matches!(
            self.units[i].type_id,
            uid::COMSAT_STATION | uid::NUCLEAR_SILO
        ) && !self.units[i].addon_attached
        {
            return Err(SimError::InvalidTarget);
        }
        let tech_id = spell.tech_id();
        // Heroes (units.dat flag 0x40, Jim Raynor) use their abilities
        // without the research (INFERENCE from BW; falsifier: a hero card
        // without the button before the research).
        let hero = self.data.unit(self.units[i].type_id).is_hero();
        // Scanner Sweep, Archon Warp and Dark Archon Meld have no research
        // building; they are implicit abilities of their caster.
        let implicit = matches!(
            spell,
            Spell::Healing | Spell::ScannerSweep | Spell::ArchonWarp | Spell::DarkArchonMeld
        );
        if !self.players[owner].has_tech(tech_id) && !implicit && !hero {
            return Err(SimError::MissingTech);
        }
        if spell == Spell::SpiderMines && self.units[i].hangar == 0 {
            return Err(SimError::QueueFull);
        }
        if !caster_can_cast(self.units[i].type_id, spell) {
            return Err(SimError::Unsupported);
        }
        // Stim Packs cost 10 hit points and are refused at 10 or fewer
        // (OpenBW `Stimpacks`: `if (u->hp <= 10_fp8) return`).
        if spell == Spell::StimPacks && self.units[i].hp <= fp(10) {
            return Err(SimError::InvalidTarget);
        }
        // `0x005cbea0` validates the target through `FUN_0059ca70` before
        // reading and spending the techdata energy.  A rejected target must
        // therefore leave the energy pool untouched.
        if spell.takes_unit_target() {
            let SpellTarget::Unit(id) = target else {
                return Err(SimError::InvalidTarget);
            };
            let Some(t) = self.index_of(id) else {
                return Err(SimError::InvalidTarget);
            };
            if matches!(spell, Spell::SpawnBroodlings | Spell::Consume) && id == caster_id {
                return Err(SimError::InvalidTarget);
            }
            if spell == Spell::Restoration && !self.restoration_worthwhile(caster_id, id) {
                return Err(SimError::InvalidTarget);
            }
            if spell == Spell::Healing {
                let ty = self.data.unit(self.units[t].type_id);
                if self.units[t].owner != self.units[i].owner
                    || ty.is_building()
                    || !ty.is_organic()
                    || self.units[t].hp >= ty.hit_points_raw as i32
                {
                    return Err(SimError::InvalidTarget);
                }
            }
            // Optical Flare blinds any unit, air included (its weapons.dat
            // row says ground only, but the tech is not gated by it: the
            // classic use is on Observers and Overlords); only buildings
            // are refused.
            if spell == Spell::OpticalFlare && self.data.unit(self.units[t].type_id).is_building() {
                return Err(SimError::InvalidTarget);
            }
            if let Some(weapon) = spell.weapon_id()
                && spell != Spell::OpticalFlare
            {
                let info = WeaponTarget {
                    unit: self.data.unit(self.units[t].type_id),
                    owner: self.units[t].owner,
                    flying: self.is_air_unit(t),
                    untargetable: !self.units[t].alive
                        || self.units[t].invincible
                        || self.units[t].status.stasis_ticks > 0,
                };
                if !self
                    .data
                    .weapon_target_ok(weapon, self.units[i].owner, Some(info))
                {
                    return Err(SimError::InvalidTarget);
                }
            }
        }
        if let Some(range) = self.spell_range_px(spell) {
            let pos = match target {
                SpellTarget::Unit(id) => self.unit(id).map(|u| (u.x, u.y)),
                SpellTarget::Point(x, y) => Some((x * fp(1), y * fp(1))),
                SpellTarget::None => None,
            };
            if let Some(p) = pos
                && self.distance_to_point(i, p) > range
            {
                return Err(SimError::InvalidTarget);
            }
        }
        // Lurker Aspect is a unit morph, not an energy ability; the techdata
        // energy column is unused for it.
        // Switching a cloak off is free; only cloaking costs energy.
        let decloak = matches!(spell, Spell::CloakingField | Spell::PersonnelCloaking)
            && self.units[i].status.cloaked;
        let cost = if spell == Spell::LurkerAspect || decloak {
            0
        } else {
            spell_energy_cost(&self.data, spell)
        };
        if !self.players[owner].cheats.infinite_energy
            && !self.units[i].spend_energy_fixed(cost * ENERGY_SCALE)
        {
            return Err(SimError::NotEnoughEnergy);
        }
        self.apply_spell(i, spell, target);
        Ok(())
    }

    /// Order-weapon maximum range in 1/256 pixels.  Spells whose order row
    /// has weapon 130 (Scanner Sweep, Recall, Meld, ...) have no weapon
    /// range and fall back to [`spell_range`].
    fn spell_range_px(&self, spell: Spell) -> Option<i32> {
        // OpenBW `order_PlaceMine`: the Vulture lays the mine once it is
        // within 20 px of the point.
        if spell == Spell::SpiderMines {
            return Some(20 * fp(1));
        }
        spell
            .weapon_id()
            .and_then(|id| self.data.weapon(id))
            .map(|w| w.max_range as i32 * fp(1))
    }

    fn order_cast(&mut self, i: usize, spell: Spell, target: SpellTarget) {
        let current = self.units[i].anim.as_ref().map(|a| a.anim as usize);
        if current == Some(ANIM_CAST_SPELL) {
            return;
        }
        // Spells are released on the iscript `castspell` opcode.  The order
        // waits at casting range until the animation reaches that frame.
        let range = self
            .spell_range_px(spell)
            .unwrap_or_else(|| spell_range(spell));
        let pos = match target {
            SpellTarget::Unit(id) => self.unit(id).map(|u| (u.x, u.y)),
            SpellTarget::Point(x, y) => Some((x * fp(1), y * fp(1))),
            SpellTarget::None => Some((self.units[i].x, self.units[i].y)),
        };
        if let Some(p) = pos
            && self.distance_to_point(i, p) > range
        {
            self.move_towards(i, (p.0 / fp(1), p.1 / fp(1)), range / fp(1));
            return;
        }
        if spell == Spell::YamatoGun {
            // Stop and charge; the Battlecruiser's CastSpell animation
            // shows the charge overlay.
            let charge = self
                .data
                .image(YAMATO_CHARGE_IMAGE)
                .and_then(|im| {
                    self.data
                        .iscript
                        .frames_until(im.iscript_id as u16, 0, 0x27)
                })
                .unwrap_or(0);
            if charge > 0 {
                self.units[i].path.clear();
                self.units[i].path_goal = None;
                self.units[i].order_wait = charge.min(255) as u8;
                self.start_animation(i, ANIM_CAST_SPELL);
                return;
            }
        }
        let has_anim = self
            .data
            .unit_iscript_id(self.data.unit(self.units[i].type_id))
            .is_some_and(|id| self.data.iscript.animation_has_cast(id, ANIM_CAST_SPELL));
        if has_anim {
            self.start_animation(i, ANIM_CAST_SPELL);
        } else {
            // Unrecorded: the order itself is the logged command.
            let _ = self.cast_unrecorded(self.units[i].id, spell, target);
            self.order_done(i);
        }
    }

    fn apply_spell(&mut self, caster: usize, spell: Spell, target: SpellTarget) {
        let c_owner = self.units[caster].owner;
        let (cx, cy) = (self.units[caster].x, self.units[caster].y);
        let target_idx = match target {
            SpellTarget::Unit(id) => self.index_of(id),
            _ => None,
        };
        let point = match target {
            SpellTarget::Unit(id) => self.unit(id).map(|u| (u.x, u.y)),
            SpellTarget::Point(x, y) => Some((x * fp(1), y * fp(1))),
            SpellTarget::None => Some((cx, cy)),
        };
        match spell {
            Spell::StimPacks => {
                let hp = self.units[caster].hp;
                self.units[caster].hp = (hp - fp(10)).max(1);
                self.units[caster].status.stim_ticks = STIM_FRAMES;
                let id = self.units[caster].id;
                self.push_spell_cue(SpellCueKind::Stim, id);
            }
            Spell::TankSiegeMode => {
                // The spell only starts the transformation order.
                if !self.is_transforming(caster) {
                    self.units[caster].order = if self.units[caster].type_id == uid::SIEGE_TANK {
                        Order::Siege
                    } else {
                        Order::Unsiege
                    };
                }
            }
            Spell::CloakingField | Spell::PersonnelCloaking => {
                self.units[caster].status.cloaked = !self.units[caster].status.cloaked;
            }
            Spell::Burrowing => {
                self.units[caster].status.burrowed = true;
            }
            Spell::YamatoGun => {
                // The blast travels (weapons.dat row 30, a flying bullet)
                // and deals its 260 on arrival.
                if let Some(t) = target_idx
                    && let Some(w) = self.data.weapon(wid::YAMATO_GUN).cloned()
                {
                    if Projectile::weapon_is_travelling(&w) {
                        self.spawn_projectile(caster, t, wid::YAMATO_GUN, &w);
                    } else {
                        self.apply_hit(caster, t, &w, 1);
                    }
                }
            }
            Spell::DefensiveMatrix => {
                if let Some(t) = target_idx {
                    self.units[t].status.apply_matrix(fp(250));
                }
            }
            Spell::EmpShockwave => {
                // The EMP missile flies to the point (weapons.dat row 33)
                // and goes off there (`hit::EMP`).
                if let Some(p) = point
                    && let Some(w) = self.data.weapon(wid::EMP_SHOCKWAVE).cloned()
                {
                    self.spawn_point_bullet(caster, wid::EMP_SHOCKWAVE, &w, p);
                }
            }
            Spell::Irradiate => {
                // Irradiate lands with its bullet (weapons.dat row 34,
                // "appear on target"), a few frames after the cast.
                if let Some(t) = target_idx
                    && !self.data.unit(self.units[t].type_id).is_building()
                {
                    let tid = self.units[t].id;
                    let pos = (self.units[t].x, self.units[t].y);
                    match self.data.weapon(wid::IRRADIATE).cloned() {
                        Some(w) if w.explosion == hit::IRRADIATE => {
                            self.instant_hit(c_owner, Some(tid), caster, wid::IRRADIATE, &w, pos);
                        }
                        _ => self.units[t].status.apply_irradiate(IRRADIATE_UPDATES),
                    }
                }
            }
            Spell::PsionicStorm => {
                if let Some((tx, ty)) = point
                    && let Some(w) = self.data.weapon(wid::PSIONIC_STORM).cloned()
                {
                    self.spawn_persistent_field(caster, wid::PSIONIC_STORM, &w, (tx, ty));
                }
            }
            Spell::Hallucination => {
                if let Some(t) = target_idx {
                    let ty = self.units[t].type_id;
                    let owner = self.units[t].owner;
                    if let Ok(new) = self.spawn_px(
                        owner as usize,
                        ty,
                        self.units[t].x / fp(1),
                        self.units[t].y / fp(1),
                    ) {
                        self.units[new].status.hallucination = true;
                        self.units[new].status.revealed_ticks = 600;
                    }
                }
            }
            Spell::StasisField => {
                if let Some((tx, ty)) = point {
                    for j in 0..self.units.len() {
                        if self.units[j].alive
                            && !self.units[j].status.burrowed
                            && !self.data.unit(self.units[j].type_id).is_building()
                            && self.distance_to_point(j, (tx, ty)) <= fp(64)
                        {
                            self.units[j].status.apply_stasis(STASIS_UPDATES);
                        }
                    }
                }
            }
            Spell::ArchonWarp | Spell::DarkArchonMeld => {
                let (a, b) = (uid::HIGH_TEMPLAR, uid::DARK_TEMPLAR);
                let (wanted, result) = if spell == Spell::ArchonWarp {
                    (a, uid::ARCHON)
                } else {
                    (b, uid::DARK_ARCHON)
                };
                let count = self
                    .units
                    .iter()
                    .filter(|u| u.alive && u.owner == c_owner && u.type_id == wanted)
                    .count();
                if count >= 2 {
                    let mut removed = 0;
                    let mut merged = Vec::new();
                    for j in 0..self.units.len() {
                        if removed == 2 {
                            break;
                        }
                        if self.units[j].alive
                            && self.units[j].owner == c_owner
                            && self.units[j].type_id == wanted
                        {
                            merged.push(self.units[j].status.clone());
                            self.units[j].alive = false;
                            removed += 1;
                        }
                    }
                    if let Ok(new) = self.spawn_px(c_owner as usize, result, cx / fp(1), cy / fp(1))
                    {
                        // `0x005c1fe0`/`0x005c206d`: the merged unit inherits
                        // the longer timers from every consumed unit.
                        for status in &merged {
                            self.units[new].status.merge_from(status);
                        }
                    }
                }
            }
            Spell::Feedback => {
                if let Some(t) = target_idx {
                    self.feedback_damage(t, caster as u32);
                }
            }
            Spell::Maelstrom => {
                if let Some((tx, ty)) = point {
                    for j in 0..self.units.len() {
                        if self.units[j].alive
                            && !self.units[j].status.burrowed
                            && self.distance_to_point(j, (tx, ty)) <= fp(64)
                            && self.data.unit(self.units[j].type_id).is_organic()
                            && !self.data.unit(self.units[j].type_id).is_building()
                        {
                            self.units[j].status.apply_maelstrom(MAELSTROM_UPDATES);
                        }
                    }
                }
            }
            Spell::MindControl => {
                if let Some(t) = target_idx {
                    self.units[t].owner = c_owner;
                }
            }
            Spell::SpawnBroodlings => {
                if let Some(t) = target_idx {
                    let tx = self.units[t].x / fp(1);
                    let ty = self.units[t].y / fp(1);
                    if self.data.unit(self.units[t].type_id).is_organic() {
                        self.kill_unit(t);
                        self.spawn_broodling(c_owner as usize, tx, ty);
                        self.spawn_broodling(c_owner as usize, tx + 16, ty);
                    }
                }
            }
            Spell::Parasite => {
                if let Some(t) = target_idx {
                    self.units[t].status.parasite = true;
                    self.units[t].status.revealed_ticks = 1200;
                }
            }
            Spell::Infestation => {
                if let Some(t) = target_idx {
                    let max = self.data.unit(uid::COMMAND_CENTER).hit_points_raw as i32;
                    if self.units[t].type_id == uid::COMMAND_CENTER && self.units[t].hp * 2 < max {
                        if let Some(addon) = self.units[t].addon.take()
                            && let Some(a) = self.index_of(addon)
                        {
                            self.kill_unit(a);
                        }
                        self.units[t].type_id = uid::INFESTED_COMMAND_CENTER;
                        self.units[t].owner = c_owner;
                        self.units[t].shields = 0;
                        self.units[t].status.parasite = false;
                    }
                }
            }
            Spell::Plague => {
                if let Some((tx, ty)) = point {
                    for j in 0..self.units.len() {
                        if self.units[j].alive
                            && !self.units[j].status.burrowed
                            && self.distance_to_point(j, (tx, ty)) <= fp(64)
                        {
                            self.units[j].status.apply_plague();
                        }
                    }
                }
            }
            Spell::Consume => {
                if let Some(t) = target_idx
                    && t != caster
                {
                    let gained =
                        self.data.unit(self.units[t].type_id).hit_points_raw as i32 / fp(1) * 2;
                    self.units[caster].add_energy_points(gained);
                    self.kill_unit(t);
                }
            }
            Spell::Ensnare => {
                if let Some((tx, ty)) = point {
                    for j in 0..self.units.len() {
                        if j == caster
                            || !self.units[j].alive
                            || self.units[j].in_transport.is_some()
                            || self.units[j].status.burrowed
                            || self.data.unit(self.units[j].type_id).is_building()
                            || self.distance_to_point(j, (tx, ty)) > fp(64)
                        {
                            continue;
                        }
                        self.units[j].status.apply_ensnare();
                    }
                }
            }
            Spell::Recall => {
                if let Some((tx, ty)) = point {
                    for j in 0..self.units.len() {
                        if self.units[j].alive
                            && self.units[j].owner == c_owner
                            && j != caster
                            && self.distance_to_point(j, (cx, cy)) <= fp(96)
                        {
                            self.units[j].x = tx;
                            self.units[j].y = ty;
                        }
                    }
                }
            }
            Spell::Restoration => {
                // Restoration lands when its bullet (weapons.dat row 102,
                // "appear on target") reaches `domissiledmg`.
                if let Some(t) = target_idx {
                    let tid = self.units[t].id;
                    let pos = (self.units[t].x, self.units[t].y);
                    match self.data.weapon(wid::RESTORATION).cloned() {
                        Some(w) if w.explosion == hit::RESTORATION => {
                            self.instant_hit(c_owner, Some(tid), caster, wid::RESTORATION, &w, pos);
                        }
                        _ => self.restore_unit(t),
                    }
                }
            }
            Spell::DisruptionWeb => {
                if let Some((tx, ty)) = point {
                    for j in 0..self.units.len() {
                        if self.units[j].alive && self.distance_to_point(j, (tx, ty)) <= fp(64) {
                            self.units[j].status.disruption_ticks = 240;
                        }
                    }
                    self.spawn_temporary(c_owner, uid::DISRUPTION_WEB, (tx, ty), 360);
                }
            }
            Spell::OpticalFlare => {
                // The Medic throws the flare (weapons.dat row 107, a bullet
                // that follows its target); the unit is blinded when it
                // lands.
                if let Some(t) = target_idx {
                    let cid = self.units[caster].id;
                    self.push_spell_cue(SpellCueKind::OpticalFlareCast, cid);
                    match self.data.weapon(wid::OPTICAL_FLARE).cloned() {
                        Some(w) if Projectile::weapon_is_travelling(&w) => {
                            self.spawn_projectile(caster, t, wid::OPTICAL_FLARE, &w);
                        }
                        _ => self.blind_unit(t),
                    }
                }
            }
            Spell::Healing => {
                // The Heal command only names the patient: the Medic then
                // walks to it and heals it at the normal rate.
                if let Some(t) = target_idx {
                    self.units[caster].target = Some(self.units[t].id);
                }
            }
            Spell::Lockdown => {
                // The Ghost fires the Lockdown missile (weapons.dat row 32,
                // a bullet that follows its target); the unit is disabled
                // when it lands (`hit::LOCKDOWN`).
                if let Some(t) = target_idx
                    && self.data.unit(self.units[t].type_id).is_mechanical()
                {
                    match self.data.weapon(wid::LOCKDOWN).cloned() {
                        Some(w) if Projectile::weapon_is_travelling(&w) => {
                            self.spawn_projectile(caster, t, wid::LOCKDOWN, &w);
                        }
                        _ => self.units[t].status.apply_lockdown(LOCKDOWN_UPDATES),
                    }
                }
            }
            Spell::DarkSwarm => {
                let point = match target {
                    SpellTarget::Unit(id) => self.unit(id).map(|u| (u.x, u.y)),
                    SpellTarget::Point(x, y) => Some((x * fp(1), y * fp(1))),
                    SpellTarget::None => None,
                };
                if let Some((tx, ty)) = point {
                    self.spawn_dark_swarm(c_owner, (tx, ty));
                }
            }
            Spell::ScannerSweep => {
                let point = match target {
                    SpellTarget::Unit(id) => self.unit(id).map(|u| (u.x, u.y)),
                    SpellTarget::Point(x, y) => Some((x * fp(1), y * fp(1))),
                    SpellTarget::None => None,
                };
                if let Some((tx, ty)) = point {
                    for j in 0..self.units.len() {
                        if self.units[j].alive
                            && self.is_enemy(c_owner, self.units[j].owner)
                            && self.distance_to_point(j, (tx, ty)) <= fp(96)
                        {
                            self.units[j].status.revealed_ticks = 400;
                        }
                    }
                    self.reveal_point(c_owner, tx / fp(1), ty / fp(1), 8);
                }
            }
            Spell::SpiderMines => {
                let point = match target {
                    SpellTarget::Unit(id) => self.unit(id).map(|u| (u.x, u.y)),
                    SpellTarget::Point(x, y) => Some((x * fp(1), y * fp(1))),
                    SpellTarget::None => None,
                };
                // One mine per order from the Vulture's three (`hangar`),
                // as `place_spider_mine`; the old version laid all three.
                if let Some((tx, ty)) = point
                    && self.units[caster].hangar > 0
                    && let Some(idx) = self.spawn_temporary(c_owner, uid::SPIDER_MINE, (tx, ty), 0)
                {
                    // The mine takes the Vulture's heading and starts in
                    // the open (`update_specials` runs its state machine).
                    self.units[idx].facing = self.units[caster].facing;
                    self.units[idx].mine_armed_at = self.tick as i64 + SPIDER_MINE_WAIT_FRAMES;
                    self.units[caster].hangar -= 1;
                }
            }
            Spell::LurkerAspect => {
                if let Some(t) = target_idx {
                    let _ = self.morph_unit_unrecorded(self.units[t].id, uid::LURKER);
                }
            }
        }
    }

    /// `FUN_005c7460`: whether a Medic's Restoration is worthwhile on
    /// `target` for `caster`.  The recovered routine requires the same owner,
    /// at least 61 whole HP+shields and a removable status.
    pub fn restoration_worthwhile(&self, caster: u32, target: u32) -> bool {
        let (Some(c), Some(t)) = (self.index_of(caster), self.index_of(target)) else {
            return false;
        };
        let (cu, tu) = (&self.units[c], &self.units[t]);
        if !tu.alive || tu.in_transport.is_some() || self.data.unit(tu.type_id).is_building() {
            return false;
        }
        if cu.owner != tu.owner {
            return false;
        }
        if tu.hp / fp(1) + tu.shields / fp(1) < 0x3d {
            return false;
        }
        tu.status.has_removable_effect()
    }

    /// Toggle a Ghost's/Wraith's personal cloak.  The tech gate is the same
    /// `Cloaking Field` / `Personnel Cloaking` research used by the ability.
    pub fn set_cloak(&mut self, unit_id: u32, cloaked: bool) -> Result<(), SimError> {
        self.set_cloak_unrecorded(unit_id, cloaked)?;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::Cloak {
                    unit: unit_id,
                    cloaked,
                },
            );
        }
        Ok(())
    }

    fn set_cloak_unrecorded(&mut self, unit_id: u32, cloaked: bool) -> Result<(), SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        let owner = self.units[i].owner as usize;
        let (tech_id, spell) = match self.units[i].type_id {
            t if is_ghost_type(t) => (tech_id::PERSONNEL_CLOAKING, Spell::PersonnelCloaking),
            uid::WRAITH | 21 => (tech_id::CLOAKING_FIELD, Spell::CloakingField),
            _ => return Err(SimError::Unsupported),
        };
        let hero = self.data.unit(self.units[i].type_id).flags & 0x40 != 0;
        if cloaked && !self.players[owner].has_tech(tech_id) && !hero {
            return Err(SimError::MissingTech);
        }
        // Cloaking costs the techdata energy (25); decloaking is free.
        if cloaked
            && !self.units[i].status.cloaked
            && !self.players[owner].cheats.infinite_energy
            && !self.units[i]
                .spend_energy_fixed(spell_energy_cost(&self.data, spell) * ENERGY_SCALE)
        {
            return Err(SimError::NotEnoughEnergy);
        }
        self.units[i].status.cloaked = cloaked;
        Ok(())
    }

    /// Apply one single-player cheat for `player` (see [`cheat`]).  Recorded
    /// (`RecAct::Cheat`) so a save or replay keeps the resources, upgrades
    /// and toggles it changed.
    pub fn apply_cheat(&mut self, player: u8, code: u8) -> Result<(), SimError> {
        self.apply_cheat_unrecorded(player, code)?;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(self.tick, crate::replay::RecAct::Cheat { player, code });
        }
        Ok(())
    }

    fn apply_cheat_unrecorded(&mut self, player: u8, code: u8) -> Result<(), SimError> {
        let idx = player as usize;
        if idx >= self.players.len() {
            return Err(SimError::BadOwner);
        }
        match code {
            cheat::SHOW_ME_THE_MONEY => {
                self.players[idx].minerals += 10_000;
                self.players[idx].gas += 10_000;
            }
            cheat::WHATS_MINE_IS_MINE => self.players[idx].minerals += 500,
            cheat::BREATHE_DEEP => self.players[idx].gas += 500,
            cheat::POWER_OVERWHELMING => {
                self.players[idx].cheats.invincible = !self.players[idx].cheats.invincible
            }
            cheat::THE_GATHERING => {
                self.players[idx].cheats.infinite_energy = !self.players[idx].cheats.infinite_energy
            }
            cheat::FOOD_FOR_THOUGHT => {
                self.players[idx].cheats.no_supply = !self.players[idx].cheats.no_supply
            }
            cheat::BLACK_SHEEP_WALL | cheat::WAR_AINT_WHAT_IT_USED_TO_BE => {
                self.players[idx].cheats.reveal = !self.players[idx].cheats.reveal
            }
            cheat::OPERATION_CWAL => {
                self.players[idx].cheats.fast_build = !self.players[idx].cheats.fast_build
            }
            cheat::SOMETHING_FOR_NOTHING => {
                for level in &mut self.players[idx].upgrades {
                    *level = (*level + 1).min(3);
                }
            }
            cheat::MEDIEVAL_MAN => {
                let on = !self.players[idx].cheats.free_tech;
                self.players[idx].cheats.free_tech = on;
                self.players[idx].cheats.free_upgrades = on;
            }
            cheat::MODIFY_THE_PHASE_VARIANCE => {
                self.players[idx].cheats.build_anything = !self.players[idx].cheats.build_anything
            }
            cheat::THERE_IS_NO_COW_LEVEL => self.winner = Some(player),
            cheat::GAME_OVER_MAN => self.players[idx].defeated = true,
            cheat::STAYING_ALIVE => {
                self.players[idx].cheats.stay_alive = !self.players[idx].cheats.stay_alive
            }
            cheat::NOGLUES => self.cheat_no_enemy_spells = !self.cheat_no_enemy_spells,
            _ => return Err(SimError::Unsupported),
        }
        Ok(())
    }

    /// Start Capture the Flag: one beacon per active player near its town
    /// hall, flags hidden for three game minutes, beacons movable for ten.
    pub fn start_ctf(&mut self) {
        if self.ctf.is_some() {
            return;
        }
        let mut beacons = [None; 8];
        let mut flags = Vec::new();
        let active = self.players.len().min(8);
        for (p, beacon_slot) in beacons.iter_mut().enumerate().take(active) {
            if self.players[p].defeated {
                continue;
            }
            let hall = self.units.iter().find(|u| {
                u.alive
                    && u.owner as usize == p
                    && u.completed
                    && matches!(
                        u.type_id,
                        uid::COMMAND_CENTER | uid::NEXUS | uid::HATCHERY | uid::LAIR | uid::HIVE
                    )
            });
            let (x, y) = hall
                .map(|u| (u.x / fp(1) + 64, u.y / fp(1) + 48))
                .or_else(|| {
                    self.units
                        .iter()
                        .find(|u| u.alive && u.owner as usize == p)
                        .map(|u| (u.x / fp(1) + 48, u.y / fp(1) + 48))
                })
                .unwrap_or((16 + p as i32 * 48, 16));
            let beacon_type = match self.players[p].race {
                Race::Terran => uid::TERRAN_FLAG_BEACON,
                Race::Zerg => uid::ZERG_FLAG_BEACON,
                Race::Protoss => uid::PROTOSS_FLAG_BEACON,
                Race::Neutral => uid::TERRAN_FLAG_BEACON,
            };
            if let Ok(i) = self.spawn_px(p, beacon_type, x, y) {
                self.units[i].invincible = true;
                self.units[i].completed = true;
                *beacon_slot = Some(self.units[i].id);
            }
            flags.push(CtfFlag {
                owner: p as u8,
                home: (x, y),
                pos: (x, y),
                carrier: None,
                return_at: None,
                released: false,
            });
        }
        let secs = TICKS_PER_SECOND as u64;
        self.ctf = Some(CtfState {
            flags,
            beacons,
            release_at: self.tick + 3 * 60 * secs,
            move_until: self.tick + 10 * 60 * secs,
            beacon_moved: [false; 8],
        });
    }

    /// Move a flag beacon once during the first ten game minutes.
    pub fn move_beacon(&mut self, player: u8, pos: (i32, i32)) -> Result<(), SimError> {
        let idx = player as usize;
        let Some(beacon) = self
            .ctf
            .as_ref()
            .and_then(|c| c.beacons.get(idx).copied().flatten())
        else {
            return Err(SimError::Unsupported);
        };
        if self
            .ctf
            .as_ref()
            .is_none_or(|c| self.tick > c.move_until || c.beacon_moved[idx])
        {
            return Err(SimError::Unsupported);
        }
        let Some(i) = self.index_of(beacon) else {
            return Err(SimError::BadUnit);
        };
        self.units[i].x = pos.0 * fp(1);
        self.units[i].y = pos.1 * fp(1);
        if let Some(c) = self.ctf.as_mut() {
            c.beacon_moved[idx] = true;
            if let Some(flag) = c.flags.iter_mut().find(|f| f.owner == player) {
                flag.home = pos;
                if flag.carrier.is_none() {
                    flag.pos = pos;
                }
            }
        }
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(self.tick, crate::replay::RecAct::Beacon { player, pos });
        }
        Ok(())
    }

    fn update_ctf(&mut self) {
        let Some(ctf) = self.ctf.as_ref() else {
            return;
        };
        let now = self.tick;
        let release_at = ctf.release_at;
        let mut drops: Vec<(usize, Option<u32>, (i32, i32))> = Vec::new();
        let mut captures: Vec<u8> = Vec::new();
        let mut returns: Vec<usize> = Vec::new();
        let mut pickups: Vec<(usize, u32, (i32, i32))> = Vec::new();
        let mut return_home: Vec<usize> = Vec::new();
        for (fi, flag) in ctf.flags.iter().enumerate() {
            if now < release_at || !flag.released {
                continue;
            }
            if let Some(carrier) = flag.carrier {
                let Some(ci) = self.index_of(carrier) else {
                    drops.push((fi, None, flag.pos));
                    continue;
                };
                let u = &self.units[ci];
                if !u.alive || u.in_transport.is_some() {
                    drops.push((fi, None, (u.x / fp(1), u.y / fp(1))));
                    continue;
                }
                if u.owner != flag.owner {
                    let home = ctf
                        .flags
                        .iter()
                        .find(|f| f.owner == u.owner)
                        .map(|f| f.home)
                        .unwrap_or(flag.pos);
                    if (u.x / fp(1) - home.0).abs() < 64 && (u.y / fp(1) - home.1).abs() < 64 {
                        captures.push(flag.owner);
                    }
                }
            } else if let Some(at) = flag.return_at {
                if now >= at {
                    returns.push(fi);
                }
            } else {
                for (i, u) in self.units.iter().enumerate() {
                    if !u.alive
                        || u.in_transport.is_some()
                        || !self.data.unit(u.type_id).is_worker()
                    {
                        continue;
                    }
                    let (ux, uy) = (u.x / fp(1), u.y / fp(1));
                    if (ux - flag.pos.0).abs() >= 40 || (uy - flag.pos.1).abs() >= 40 {
                        continue;
                    }
                    if u.owner == flag.owner {
                        if flag.pos != flag.home {
                            return_home.push(fi);
                        }
                    } else {
                        pickups.push((fi, u.id, (ux, uy)));
                        let _ = i;
                        break;
                    }
                }
            }
        }
        for (fi, carrier, pos) in drops {
            if let Some(c) = self.ctf.as_mut() {
                c.flags[fi].carrier = carrier;
                c.flags[fi].pos = pos;
                c.flags[fi].return_at = Some(now + 30 * TICKS_PER_SECOND as u64);
            }
        }
        for fi in returns {
            if let Some(c) = self.ctf.as_mut() {
                c.flags[fi].pos = c.flags[fi].home;
                c.flags[fi].return_at = None;
                c.flags[fi].released = true;
            }
        }
        for fi in return_home {
            if let Some(c) = self.ctf.as_mut() {
                c.flags[fi].pos = c.flags[fi].home;
                c.flags[fi].return_at = None;
            }
        }
        for (fi, unit, pos) in pickups {
            if let Some(c) = self.ctf.as_mut() {
                c.flags[fi].carrier = Some(unit);
                c.flags[fi].return_at = None;
                c.flags[fi].pos = pos;
            }
        }
        for owner in captures {
            self.defeat_player(owner);
        }
        // Release and carrier-follow updates are computed against the
        // immutable world, then applied to the flag state.
        let updates: Vec<(usize, (i32, i32), bool)> = {
            let Some(c) = self.ctf.as_ref() else {
                return;
            };
            c.flags
                .iter()
                .enumerate()
                .map(|(fi, flag)| {
                    let pos = flag
                        .carrier
                        .and_then(|id| self.unit(id))
                        .map(|u| (u.x / fp(1), u.y / fp(1)))
                        .unwrap_or(flag.pos);
                    (fi, pos, flag.released || now >= release_at)
                })
                .collect()
        };
        if let Some(c) = self.ctf.as_mut() {
            for (fi, pos, released) in updates {
                c.flags[fi].pos = pos;
                c.flags[fi].released = released;
            }
        }
    }

    /// Remove a player from a Capture the Flag match (the flag owner is
    /// "destroyed" when an enemy flag reaches the capturer's beacon).
    fn defeat_player(&mut self, player: u8) {
        let idx = player as usize;
        if idx >= self.players.len() {
            return;
        }
        self.players[idx].defeated = true;
        for u in &mut self.units {
            if u.owner == player && !is_resource_type(u.type_id) {
                u.alive = false;
                u.order = Order::Die;
            }
        }
    }

    /// Launch a nuclear strike from a Ghost.  Requires a completed Nuclear
    /// Silo and pays the 200/200 cost.  The missile is a travelling projectile
    /// with the retail nuclear hit type; impact damage is
    /// `max(500, 2/3 * max_hp_plus_shields)` over the large splash radius.
    /// Arm a completed, attached Nuclear Silo (`0x5d1e80`).  The missile
    /// costs its `units.dat` 200/200 when armed, and the silo holds exactly
    /// one until a Ghost launches it.
    pub fn arm_nuke(&mut self, silo_id: u32) -> Result<(), SimError> {
        let i = self.index_of(silo_id).ok_or(SimError::BadUnit)?;
        if self.units[i].type_id != uid::NUCLEAR_SILO
            || !self.units[i].alive
            || !self.units[i].completed
            || !self.units[i].addon_attached
        {
            return Err(SimError::MissingRequirements);
        }
        if self.units[i].nuke_ready {
            return Ok(());
        }
        let owner = self.units[i].owner as usize;
        let p = &mut self.players[owner];
        if p.minerals < 200 {
            return Err(SimError::NotEnoughMinerals);
        }
        if p.gas < 200 {
            return Err(SimError::NotEnoughGas);
        }
        p.pay_minerals(200);
        p.pay_gas(200);
        self.units[i].nuke_ready = true;
        Ok(())
    }

    pub fn launch_nuke(&mut self, ghost_id: u32, pos: (i32, i32)) -> Result<(), SimError> {
        self.launch_nuke_unrecorded(ghost_id, pos)?;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::Nuke {
                    ghost: ghost_id,
                    pos,
                },
            );
        }
        Ok(())
    }

    fn launch_nuke_unrecorded(&mut self, ghost_id: u32, pos: (i32, i32)) -> Result<(), SimError> {
        let i = self.index_of(ghost_id).ok_or(SimError::BadUnit)?;
        if !self.units[i].alive || !is_ghost_type(self.units[i].type_id) {
            return Err(SimError::Unsupported);
        }
        if self.nuke_strikes.iter().any(|s| s.ghost == ghost_id) {
            return Err(SimError::ProducerBusy);
        }
        let owner = self.units[i].owner;
        let is_silo = |u: &SimUnit| {
            u.alive
                && u.completed
                && u.owner == owner
                && u.type_id == uid::NUCLEAR_SILO
                && u.addon_attached
        };
        // A silo that already holds a missile first.
        let Some(silo) = self
            .units
            .iter()
            .position(|u| is_silo(u) && u.nuke_ready)
            .or_else(|| self.units.iter().position(is_silo))
        else {
            return Err(SimError::MissingRequirements);
        };
        let silo_id = self.units[silo].id;
        if !self.units[silo].nuke_ready {
            // The missile has to be built in the silo first (200 / 200,
            // build time 1500).  A computer player whose script asks for a
            // strike without one starts building it and strikes on a later
            // request, like a player would.
            if self.players[owner as usize].is_computer {
                let _ = self.train(silo_id, uid::NUCLEAR_MISSILE);
            }
            return Err(SimError::MissingRequirements);
        }
        self.units[silo].nuke_ready = false;
        // `0x005b4950`: the launched missile links back to its silo
        // (`CUnit+0xd0`), and the silo records the live missile.
        self.nuke_strikes.push(NukeStrike {
            owner,
            ghost: ghost_id,
            silo: silo_id,
            target: (pos.0 * fp(1), pos.1 * fp(1)),
            timer: 90,
            dot: true,
            warned: false,
            missile: 0,
        });
        self.units[i].cooldown = 5;
        Ok(())
    }

    fn advance_nuke_strikes(&mut self) {
        // The Ghost paints the target from within its sight range: until it
        // is that close it walks towards the point and the countdown waits.
        // OpenBW `order_CastNuclearStrike`: centre distance to the point
        // greater than `unit_sight_range` keeps the Ghost walking.
        let mut waiting = Vec::new();
        for k in 0..self.nuke_strikes.len() {
            let (ghost, target) = (self.nuke_strikes[k].ghost, self.nuke_strikes[k].target);
            let Some(g) = self.index_of(ghost) else {
                continue;
            };
            let range = self.unit_sight_range(g) * 32 * fp(1);
            if self.distance_to_point(g, target) > range {
                self.move_towards(g, (target.0 / fp(1), target.1 / fp(1)), fp(8));
                waiting.push(ghost);
            } else {
                self.units[g].path.clear();
                self.units[g].path_goal = None;
            }
        }
        // The countdown has started: the missile leaves its silo and climbs
        // off the top of the map (OpenBW `order_NukeLaunch`: 90 frames,
        // warning at 45).  It is invincible and only seen, never fought.
        for k in 0..self.nuke_strikes.len() {
            let st = &self.nuke_strikes[k];
            if st.missile != 0 || waiting.contains(&st.ghost) {
                continue;
            }
            let (owner, silo, ghost) = (st.owner, st.silo, st.ghost);
            let Some(si) = self.index_of(silo) else {
                continue;
            };
            let (sx, sy) = (self.units[si].x / fp(1), self.units[si].y / fp(1));
            if let Ok(m) = self.spawn_px(owner as usize, uid::NUCLEAR_MISSILE, sx, sy) {
                self.units[m].invincible = true;
                self.units[m].connected_unit = Some(ghost);
                self.units[m].order = Order::Idle;
                self.units[m].facing = 0;
                self.units[m].speed = 0;
                self.nuke_strikes[k].missile = self.units[m].id;
            }
        }
        let mut launched = Vec::new();
        for strike in &mut self.nuke_strikes {
            if waiting.contains(&strike.ghost) {
                continue;
            }
            strike.timer -= 1;
            if strike.timer <= 45 && !strike.warned {
                strike.warned = true;
                self.nuke_warnings
                    .push((strike.target.0 / fp(1), strike.target.1 / fp(1)));
            }
            // The missile is out of sight for its WarpIn animation
            // (iscript 131: `wait 125, wait 125, sigorder 2`) before it
            // reappears above the target.
            if strike.timer == -NUKE_WARP_FRAMES {
                launched.push((strike.owner, strike.ghost, strike.target));
            }
        }
        for (owner, ghost, target) in launched {
            self.spawn_nuke_missile(owner, ghost, target);
        }
        // The strike stays on the books until the bomb has landed.
        let falling = |w: &World, ghost: u32| {
            w.projectiles
                .iter()
                .find(|p| p.weapon == wid::NUCLEAR_STRIKE && p.source == ghost)
                .map(|p| (p.x, p.y))
        };
        let live: Vec<bool> = self
            .nuke_strikes
            .iter()
            .map(|s| s.timer > -NUKE_WARP_FRAMES || falling(self, s.ghost).is_some())
            .collect();
        let mut keep = live.iter();
        self.nuke_strikes.retain(|_| *keep.next().unwrap_or(&false));
        // The missile unit: climbing during the countdown, hidden while it
        // is off the map, then the same unit 320 px above the target,
        // heading down (OpenBW `order_NukeLaunch` states 2 to 5), carried
        // by the nuclear bullet that deals the damage.
        for k in 0..self.nuke_strikes.len() {
            let (timer, ghost, missile) = {
                let s = &self.nuke_strikes[k];
                (s.timer, s.ghost, s.missile)
            };
            let Some(m) = self.index_of(missile) else {
                continue;
            };
            if timer > 0 {
                // Climbing straight up from rest with the missile's flingy
                // acceleration (33/256 px per frame per frame, top speed
                // 8533/256).
                let v = (self.units[m].speed + NUKE_ACCELERATION).min(NUKE_TOP_SPEED);
                self.units[m].speed = v;
                self.units[m].y -= v;
                continue;
            }
            // The falling bomb accelerates the same way.
            if let Some(p) = self
                .projectiles
                .iter_mut()
                .find(|p| p.weapon == wid::NUCLEAR_STRIKE && p.source == ghost)
            {
                p.speed = (p.speed + NUKE_ACCELERATION).min(NUKE_TOP_SPEED);
            }
            match falling(self, ghost) {
                Some((x, y)) => {
                    self.units[m].alive = true;
                    self.units[m].x = x;
                    self.units[m].y = y;
                    self.units[m].facing = 128;
                    self.units[m].order = Order::Idle;
                    self.units[m].path.clear();
                    self.units[m].path_goal = None;
                }
                None => self.units[m].alive = false,
            }
        }
        // A missile whose strike landed or was called off leaves the map.
        for i in 0..self.units.len() {
            let u = &self.units[i];
            if u.alive
                && u.type_id == uid::NUCLEAR_MISSILE
                && u.invincible
                && !self.nuke_strikes.iter().any(|s| s.missile == u.id)
            {
                self.units[i].alive = false;
                self.units[i].order = Order::Die;
            }
        }
    }

    fn spawn_nuke_missile(&mut self, owner: u8, ghost: u32, target: (i32, i32)) {
        let target_unit = self
            .units
            .iter()
            .position(|u| {
                u.alive
                    && u.owner != owner
                    && (u.x - target.0).abs() < fp(4)
                    && (u.y - target.1).abs() < fp(4)
            })
            .map(|i| self.units[i].id);
        self.projectiles.push(Projectile {
            id: self.next_projectile_id,
            owner,
            source: ghost,
            weapon: wid::NUCLEAR_STRIKE,
            behavior: behavior::FLY,
            hit_type: hit::NUCLEAR,
            state: BulletState::Fly,
            x: target.0,
            y: target.1 - 320 * fp(1),
            target: target_unit,
            target_pos: target,
            // From rest; `advance_nuke_strikes` accelerates it each frame,
            // so the 320 px take about 70 frames.
            speed: NUKE_ACCELERATION,
            remaining_time: 300,
            bounces_left: 0,
            hit_flags: 0,
            previous_target: None,
            last_dot_tick: self.tick as i64,
        });
        self.next_projectile_id = self.next_projectile_id.wrapping_add(1).max(1);
    }

    /// `0x005b4950`: record the launched missile as linked to its silo
    /// (`CUnit+0xd0`, with `CUnit+0xd4 = 1`).
    pub fn link_nuke_missile(&mut self, silo_id: u32) -> bool {
        self.units
            .iter()
            .any(|u| u.id == silo_id && u.alive && u.type_id == uid::NUCLEAR_SILO)
    }

    /// `0x005b5a30`: true when the silo has no linked missile
    /// (`CUnit+0xd0` is zero).
    pub fn has_no_linked_missile(&self, silo_id: u32) -> bool {
        !self.nuke_strikes.iter().any(|s| s.silo == silo_id)
    }

    fn cancel_nuke_for(&mut self, unit_id: u32) {
        self.nuke_strikes
            .retain(|s| s.timer <= -NUKE_WARP_FRAMES || (s.ghost != unit_id && s.silo != unit_id));
    }

    /// Apply a simple AI order: move/attack-move.
    pub fn command_move(&mut self, unit_ids: &[u32], pos: (i32, i32)) {
        for &id in unit_ids {
            let _ = self.issue(id, Order::Move { pos });
        }
    }

    pub fn command_attack_move(&mut self, unit_ids: &[u32], pos: (i32, i32)) {
        for &id in unit_ids {
            let _ = self.issue(id, Order::AttackMove { pos });
        }
    }

    // ------------------------------------------------------------------
    // Carriers, Reavers, Spider Mines and other special units
    // ------------------------------------------------------------------

    /// Add one interceptor/scarab to a carrier/reaver hangar.
    pub fn build_hangar_unit(&mut self, unit_id: u32) -> Result<(), SimError> {
        self.build_hangar_unit_unrecorded(unit_id)?;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(self.tick, crate::replay::RecAct::Hangar { unit: unit_id });
        }
        Ok(())
    }

    fn build_hangar_unit_unrecorded(&mut self, unit_id: u32) -> Result<(), SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        let owner = self.units[i].owner;
        let type_id = self.units[i].type_id;
        let (child, cost) = match type_id {
            uid::CARRIER => (uid::INTERCEPTOR, 15),
            uid::REAVER => (uid::SCARAB, 15),
            _ => return Err(SimError::Unsupported),
        };
        let max = self.hangar_capacity(i);
        if self.units[i].hangar >= max {
            return Err(SimError::QueueFull);
        }
        if self.players[owner as usize].minerals < cost {
            return Err(SimError::NotEnoughMinerals);
        }
        self.players[owner as usize].pay_minerals(cost);
        self.units[i].hangar += 1;
        let _ = child;
        Ok(())
    }

    /// The Hatchery / Lair / Hive "Spawn Larva" button.  Recorded
    /// (`RecAct::SpawnLarva`, save tag 17).
    /// Spawn a Broodling with the `0x005b5e20`/`0x005cc810` 1350-frame
    /// lifetime.
    fn spawn_broodling(&mut self, owner: usize, x: i32, y: i32) {
        if let Ok(i) = self.spawn_px(owner, uid::BROODLING, x, y) {
            self.units[i].expires_at = self.tick as i64 + BROODLING_LIFETIME_TICKS;
        }
    }

    pub fn spawn_larva(&mut self, hatchery_id: u32) -> Result<(), SimError> {
        self.spawn_larva_unrecorded(hatchery_id)?;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::SpawnLarva {
                    hatchery: hatchery_id,
                },
            );
        }
        Ok(())
    }

    fn spawn_larva_unrecorded(&mut self, hatchery_id: u32) -> Result<(), SimError> {
        let i = self.index_of(hatchery_id).ok_or(SimError::BadUnit)?;
        let u = &self.units[i];
        if !u.alive || !u.completed || !matches!(u.type_id, uid::HATCHERY | uid::LAIR | uid::HIVE) {
            return Err(SimError::MissingRequirements);
        }
        let owner = u.owner;
        if self.larva_count_for(i) >= 3 {
            return Err(SimError::QueueFull);
        }
        let parent = self.units[i].id;
        let (sx, sy) = self.larva_spawn_spot(i);
        if let Ok(l) = self.spawn_px(owner as usize, uid::LARVA, sx, sy) {
            self.units[l].connected_unit = Some(parent);
        }
        Ok(())
    }

    /// Retail larva placement (`0x005dfabc`..`0x005dffe0`): probe the four
    /// Hatchery sides (translation batch 38), keep the lowest side score with
    /// the batch-39 pair fold, then try the bottom-right half fallback.  A
    /// map without creep falls back to the historical bottom point.
    fn larva_spawn_spot(&mut self, i: usize) -> (i32, i32) {
        let id = self.units[i].id;
        let hx = self.units[i].x / fp(1);
        let hy = self.units[i].y / fp(1);
        let bottom = self.data.unit(self.units[i].type_id).dimensions.bottom as i32;
        let mut candidates: Vec<(u32, i32, i32, u32)> = Vec::with_capacity(4);
        if let Some((x, y, score)) = self.a38_larva_spot_bottom(id, 0x65) {
            candidates.push((0, x, y, score as u32));
        }
        if let Some((x, y, score)) = self.a38_larva_spot_left(id, 0x65) {
            candidates.push((1, x, y, score as u32));
        }
        if let Some((x, y, score)) = self.a38_larva_spot_right(id, 0x65) {
            candidates.push((2, x, y, score as u32));
        }
        if let Some((x, y, score)) = self.a38_larva_spot_top(id, 0x65) {
            candidates.push((3, x, y, score as u32));
        }
        let mut incumbent = (0u32, 0x65u32);
        let mut chosen = None;
        for (order, x, y, score) in candidates {
            let next = self.units[i].a39_keep_smaller_pair(incumbent, (order, score));
            if next != incumbent {
                chosen = Some((x, y));
                incumbent = next;
            }
        }
        if let Some(pos) = chosen {
            return pos;
        }
        if let Some((x, y, _)) = self.a38_larva_spot_bottom_right_half(id) {
            return (x, y);
        }
        (hx, hy + bottom + 10)
    }

    pub fn hangar_capacity(&self, i: usize) -> u8 {
        let u = &self.units[i];
        let owner = u.owner as usize;
        match u.type_id {
            uid::CARRIER => {
                if self.players[owner].upgrade_level(upgrade::CARRIER_CAPACITY) > 0 {
                    8
                } else {
                    CARRIER_HANGAR
                }
            }
            uid::REAVER => {
                if self.players[owner].upgrade_level(upgrade::REAVER_CAPACITY) > 0 {
                    10
                } else {
                    REAVER_HANGAR
                }
            }
            _ => 0,
        }
    }

    fn special_attack(&mut self, attacker: usize, target: usize) -> bool {
        let ty = self.units[attacker].type_id;
        let dist = self.distance(attacker, target);
        match ty {
            uid::CARRIER => {
                if self.units[attacker].special_cooldown > 0 {
                    return true;
                }
                if self.units[attacker].hangar == 0 || dist > 8 * 32 * fp(1) {
                    return false;
                }
                if self.units[attacker].cooldown == 0 {
                    let w = self.data.weapon(wid::PULSE_CANNON).cloned();
                    if let Some(w) = w {
                        self.spawn_projectile(attacker, target, wid::PULSE_CANNON, &w);
                        self.units[attacker].cooldown = 9;
                    }
                }
                true
            }
            uid::REAVER => {
                if self.units[attacker].special_cooldown > 0 {
                    return true;
                }
                if self.units[attacker].hangar == 0 || dist > 10 * 32 * fp(1) {
                    return false;
                }
                if self.units[attacker].cooldown == 0 {
                    let w = self.data.weapon(wid::SCARAB).cloned();
                    if let Some(w) = w {
                        let before = self.projectiles.len();
                        self.spawn_projectile(attacker, target, wid::SCARAB, &w);
                        if self.projectiles.len() > before {
                            self.units[attacker].hangar -= 1;
                            self.units[attacker].special_cooldown = 0x3c;
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn is_under_dark_swarm(&self, target: usize) -> bool {
        let t = &self.units[target];
        if self.is_air_unit(target) || self.data.unit(t.type_id).is_building() {
            return false;
        }
        self.units.iter().any(|u| {
            u.alive
                && u.type_id == uid::DARK_SWARM
                && (u.x - t.x).abs() < fp(96)
                && (u.y - t.y).abs() < fp(96)
        })
    }

    fn spawn_dark_swarm(&mut self, owner: u8, pos: (i32, i32)) {
        // Translation batch 37 (`0x005dc1e0`): clamp with the `0xca` margin
        // row, set the hidden status bit and the removal timer, then keep the
        // sim's 360-tick temporary-unit expiry.
        if let Ok(id) = self.a37_spawn_dark_swarm(owner as usize, (pos.0 / fp(1), pos.1 / fp(1)))
            && let Some(idx) = self.index_of(id)
        {
            let expiry = self.tick as i64 + 360;
            self.units[idx].expires_at = if self.units[idx].expires_at > 0 {
                self.units[idx].expires_at.min(expiry)
            } else {
                expiry
            };
        }
    }

    fn spawn_temporary(
        &mut self,
        owner: u8,
        type_id: u16,
        pos: (i32, i32),
        lifetime: i64,
    ) -> Option<usize> {
        let idx = self
            .spawn_px(owner as usize, type_id, pos.0 / fp(1), pos.1 / fp(1))
            .ok()?;
        if lifetime > 0 {
            // `CUnit+0x110` is the temporary-unit lifetime (`FUN_005b6810`
            // keeps the smaller of the current and new value).
            let expiry = self.tick as i64 + lifetime;
            self.units[idx].expires_at = if self.units[idx].expires_at > 0 {
                self.units[idx].expires_at.min(expiry)
            } else {
                expiry
            };
        }
        self.units[idx].completed = true;
        Some(idx)
    }

    /// Spider Mines (OpenBW `order_SpiderMine`).  A new mine sits in the
    /// open for 60 frames, takes 4 to burrow, and is then hidden and armed.
    /// It wakes for the nearest enemy inside the 96 px square around it
    /// (3 tiles of target acquisition) that is not invincible, a grounded
    /// building, a flyer or a hovering unit; unburrows (4 frames), chases
    /// it at 16 px per frame while it stays within 576 px, and explodes
    /// when their boxes are within 30 px (weapons.dat row 6: 125 explosive,
    /// radial splash 50 / 75 / 100).  A mine that loses its target digs in
    /// again.  `mine_armed_at` is the tick of the next state change.
    fn update_specials(&mut self) {
        let now = self.tick as i64;
        let mut detonations: Vec<usize> = Vec::new();
        for i in 0..self.units.len() {
            if !self.units[i].alive || self.units[i].type_id != uid::SPIDER_MINE {
                continue;
            }
            if self.units[i].status.lockdown_ticks > 0 || self.units[i].status.stasis_ticks > 0 {
                continue;
            }
            // Fresh mine: start the 60-frame wait.
            if self.units[i].mine_armed_at == 0 {
                self.units[i].mine_armed_at = now + SPIDER_MINE_WAIT_FRAMES;
            }
            let armed_at = self.units[i].mine_armed_at;
            let target = self.units[i].target.and_then(|t| self.index_of(t));
            if self.units[i].status.burrowed {
                // Armed: look for a victim.
                let (mx, my, owner) = (self.units[i].x, self.units[i].y, self.units[i].owner);
                let half = SPIDER_MINE_SEEK_PX * fp(1);
                let victim = (0..self.units.len())
                    .filter(|&n| {
                        let v = &self.units[n];
                        n != i
                            && v.alive
                            && self.hostile(owner, v.owner)
                            && !v.invincible
                            && !self.data.unit(v.type_id).is_building()
                            && !self.is_air_unit(n)
                            && !is_hover_type(v.type_id)
                            && (v.x - mx).abs() <= half
                            && (v.y - my).abs() <= half
                    })
                    .map(|n| (self.distance(i, n), n))
                    .min()
                    .map(|(_, n)| n);
                if let Some(v) = victim {
                    self.units[i].status.burrowed = false;
                    self.units[i].target = Some(self.units[v].id);
                    self.units[i].mine_armed_at = now + SPIDER_MINE_BURROW_FRAMES;
                }
                continue;
            }
            match target {
                // Waiting in the open, then burrowing.
                None => {
                    if now >= armed_at + SPIDER_MINE_BURROW_FRAMES {
                        self.units[i].status.burrowed = true;
                        self.units[i].path.clear();
                        self.units[i].path_goal = None;
                    }
                }
                // Unburrowing, then the chase.
                Some(_) if now < armed_at => {}
                Some(v) => {
                    let alive = self.units[v].alive && !self.is_air_unit(v);
                    let dist = self.units_distance_px(i, v);
                    if !alive || dist > SPIDER_MINE_CHASE_PX {
                        self.units[i].target = None;
                        self.units[i].path.clear();
                        self.units[i].path_goal = None;
                        self.units[i].mine_armed_at = now + SPIDER_MINE_WAIT_FRAMES;
                    } else if dist <= SPIDER_MINE_BLAST_PX {
                        detonations.push(i);
                    } else {
                        let (vx, vy) = (self.units[v].x / fp(1), self.units[v].y / fp(1));
                        self.move_towards(i, (vx, vy), fp(4));
                    }
                }
            }
        }
        for i in detonations {
            let pos = (self.units[i].x, self.units[i].y);
            if let Some(w) = self.data.weapon(wid::SPIDER_MINES).cloned() {
                let owner = self.units[i].owner;
                self.resolve_hit_type(owner, None, i, wid::SPIDER_MINES, &w, pos, 1, 1);
            }
            self.kill_unit(i);
        }
        self.bunker_fire();
        self.medic_healing();
    }

    /// Infantry inside a Bunker fire from the building's position.  Each
    /// occupant keeps its own weapon, cooldown and upgrades; the bunker only
    /// supplies the location and protection.
    fn bunker_fire(&mut self) {
        for b in 0..self.units.len() {
            if !self.units[b].alive
                || !self.units[b].completed
                || self.units[b].type_id != uid::BUNKER
                || self.units[b].cargo.is_empty()
            {
                continue;
            }
            let (bx, by) = (self.units[b].x, self.units[b].y);
            let cargo = self.units[b].cargo.clone();
            for id in cargo {
                let Some(o) = self.index_of(id) else {
                    continue;
                };
                if self.units[o].hp <= 0 {
                    continue;
                }
                // An SCV sheltering in the Bunker does not fire.
                if self.data.unit(self.units[o].type_id).is_worker() {
                    continue;
                }
                // `0x5b5aa0`: disabled occupants do not enter Bunker Guard.
                if self.units[o].status.lockdown_ticks > 0
                    || self.units[o].status.stasis_ticks > 0
                    || self.units[o].status.maelstrom_ticks > 0
                {
                    continue;
                }
                if self.units[o].order_wait > 0 {
                    self.units[o].order_wait -= 1;
                }
                // `CUnit+0x7c`: keep the current target while it is legal.
                if let Some(t) = self.units[o].target
                    && !self.bunker_target_valid(o, t)
                {
                    self.units[o].target = None;
                }
                if let Some(t) = self.units[o].target {
                    self.bunker_fire_at(o, t, bx, by);
                    continue;
                }
                // `0x5a0581`: still waiting out the 15-tick retry.
                if self.units[o].order_wait != 0 {
                    continue;
                }
                self.units[o].order_wait = BUNKER_GUARD_RETRY_TICKS;
                let owner = self.units[o].owner;
                let mut best: Option<(i32, u32)> = None;
                for j in 0..self.units.len() {
                    if !self.units[j].alive
                        || !self.hostile(owner, self.units[j].owner)
                        || self.units[j].invincible
                        || self.units[j].status.stasis_ticks > 0
                    {
                        continue;
                    }
                    let ty = self.data.unit(self.units[j].type_id);
                    let Some((_, w)) = self.weapon_for(o, j) else {
                        continue;
                    };
                    if !weapon_can_target(&w, ty) {
                        continue;
                    }
                    let dist = self.bunker_distance_px(o, j, bx, by);
                    let max_range =
                        self.weapon_max_range_px(o, j, &w) + BUNKER_GUARD_RANGE_BONUS_PX;
                    // The nearest legal target, not the last one scanned.
                    if dist <= max_range
                        && dist >= w.min_range as i32
                        && best.is_none_or(|(bd, _)| dist < bd)
                    {
                        best = Some((dist, self.units[j].id));
                    }
                }
                if let Some((_, t)) = best {
                    self.units[o].target = Some(t);
                    self.bunker_fire_at(o, t, bx, by);
                }
            }
        }
    }

    fn bunker_target_valid(&self, o: usize, target: u32) -> bool {
        let owner = self.units[o].owner;
        self.index_of(target).is_some_and(|ti| {
            let t = &self.units[ti];
            t.alive
                && t.hp > 0
                && !t.invincible
                && t.status.stasis_ticks == 0
                && self.hostile(owner, t.owner)
                && self.can_see_unit(owner, ti)
                && self
                    .weapon_for(o, ti)
                    .is_some_and(|(_, w)| weapon_can_target(&w, self.data.unit(t.type_id)))
        })
    }

    fn bunker_fire_at(&mut self, o: usize, target: u32, bx: i32, by: i32) {
        if self.units[o].cooldown > 0 || self.units[o].status.disruption_ticks > 0 {
            return;
        }
        let Some(ti) = self.index_of(target) else {
            return;
        };
        if !self.units[ti].alive {
            return;
        }
        let Some((wid, w)) = self.weapon_for(o, ti) else {
            return;
        };
        if !weapon_can_target(&w, self.data.unit(self.units[ti].type_id)) {
            return;
        }
        let dist = self.bunker_distance_px(o, ti, bx, by);
        let max_range = self.weapon_max_range_px(o, ti, &w) + BUNKER_GUARD_RANGE_BONUS_PX;
        if dist > max_range || dist < w.min_range as i32 {
            return;
        }
        let (dx, dy) = (self.units[ti].x - bx, self.units[ti].y - by);
        self.units[o].facing = direction_byte(dx, dy);
        self.fire(o, ti, wid, &w);
    }

    /// OpenBW `restore_unit`: a hallucination dies; anything else loses
    /// parasite, blindness, ensnare, plague, irradiate, lockdown, maelstrom
    /// and acid spores (`0x005d2620` / `0x005dc300` / `0x005d2540` /
    /// `0x005d2660` clear the timers).
    fn restore_unit(&mut self, t: usize) {
        if self.units[t].status.hallucination {
            self.kill_unit(t);
            return;
        }
        let s = &mut self.units[t].status;
        s.clear_lockdown();
        s.clear_plague();
        s.clear_irradiate();
        s.clear_maelstrom();
        s.clear_ensnare();
        s.optical_flare_ticks = 0;
        s.parasite = false;
        s.clear_acid_spores();
        let id = self.units[t].id;
        self.push_spell_cue(SpellCueKind::Restoration, id);
    }

    /// OpenBW `blind_unit`: Optical Flare lands.  A hallucination dies;
    /// anything else is blinded until Restoration (any non-zero value of
    /// the timer means blind; it does not count down).
    fn blind_unit(&mut self, t: usize) {
        if self.units[t].status.hallucination {
            self.kill_unit(t);
            return;
        }
        self.units[t].status.optical_flare_ticks = 600;
        let id = self.units[t].id;
        self.push_spell_cue(SpellCueKind::OpticalFlareHit, id);
    }

    fn push_spell_cue(&mut self, kind: SpellCueKind, unit: u32) {
        if self.spell_cues.len() >= SPELL_CUE_KEEP {
            self.spell_cues.pop_front();
        }
        self.spell_cues.push_back(SpellCue { kind, unit });
        self.spell_cue_seq += 1;
    }

    /// Whether `medic` may heal `j` (OpenBW `medic_can_heal_target`): a
    /// living, wounded, organic, non-building ally that is not the Medic
    /// itself and is not loaded in a transport or Bunker.
    fn medic_can_heal(&self, medic: usize, j: usize) -> bool {
        let t = &self.units[j];
        let ty = self.data.unit(t.type_id);
        j != medic
            && t.alive
            && t.in_transport.is_none()
            && !t.in_bunker
            && !t.invincible
            && !self.is_enemy(self.units[medic].owner, t.owner)
            && ty.is_organic()
            && !ty.is_building()
            // OpenBW `medic_can_heal_target`: not a flyer, not a
            // hallucination, not disabled (lockdown, stasis, maelstrom).
            && !self.is_air_unit(j)
            && !t.status.hallucination
            && t.status.lockdown_ticks == 0
            && t.status.stasis_ticks == 0
            && t.status.maelstrom_ticks == 0
            && t.hp < ty.hit_points_raw as i32
    }

    /// The unit a Medic is (or will be) healing: its current target while
    /// that stays healable, else the nearest healable unit whose box is
    /// within [`MEDIC_SEARCH_PX`] and that no other Medic has taken.
    fn medic_pick_target(&self, i: usize, taken: &[usize]) -> Option<usize> {
        if let Some(t) = self.units[i].target.and_then(|t| self.index_of(t))
            && self.medic_can_heal(i, t)
            && !taken.contains(&t)
        {
            return Some(t);
        }
        // OpenBW `find_medic_target`: the nearest healable unit inside the
        // square of 160 px around the Medic.
        let (mx, my) = (self.units[i].x, self.units[i].y);
        let half = MEDIC_SEARCH_PX * fp(1);
        (0..self.units.len())
            .filter(|&j| self.medic_can_heal(i, j) && !taken.contains(&j))
            .filter(|&j| {
                (self.units[j].x - mx).abs() <= half && (self.units[j].y - my).abs() <= half
            })
            .map(|j| (self.distance(i, j), j))
            .min()
            .map(|(_, j)| j)
    }

    /// True while Medic `i` stands in reach of its heal target with the
    /// energy to heal it (the renderer plays the heal animation then).
    pub fn medic_is_healing(&self, i: usize) -> bool {
        let Some(u) = self.units.get(i) else {
            return false;
        };
        u.alive
            && u.type_id == uid::MEDIC
            && u.energy_fixed() >= MEDIC_HEAL_COST_PER_TICK
            && u.target.and_then(|t| self.index_of(t)).is_some_and(|t| {
                self.medic_can_heal(i, t) && self.units_distance_px(i, t) <= MEDIC_REACH_PX
            })
    }

    /// Medics heal wounded organic allies: 200/256 hit points for 100/256
    /// energy per frame, one Medic per patient.  An idle Medic (or one on
    /// attack-move, retail `HealMove`) walks to a patient within
    /// [`MEDIC_SEARCH_PX`]; on hold position it only heals what it can
    /// reach.  A Medic inside a transport or Bunker does nothing.
    fn medic_healing(&mut self) {
        let mut taken: Vec<usize> = Vec::new();
        for i in 0..self.units.len() {
            if !self.units[i].alive
                || self.units[i].type_id != uid::MEDIC
                || self.units[i].in_transport.is_some()
                || self.units[i].in_bunker
                || self.units[i].status.lockdown_ticks > 0
                || self.units[i].status.stasis_ticks > 0
                || self.units[i].status.maelstrom_ticks > 0
            {
                continue;
            }
            let hold = self.units[i].a08_is_hold_position_idle();
            let roaming = matches!(self.units[i].order, Order::Idle | Order::AttackMove { .. });
            if !hold && !roaming {
                continue;
            }
            if self.units[i].energy_fixed() < MEDIC_HEAL_COST_PER_TICK {
                self.units[i].target = None;
                continue;
            }
            let Some(target) = self.medic_pick_target(i, &taken) else {
                self.units[i].target = None;
                continue;
            };
            let dist = self.units_distance_px(i, target);
            if dist > MEDIC_REACH_PX {
                if hold {
                    self.units[i].target = None;
                    continue;
                }
                // Walk up to the patient.
                self.units[i].target = Some(self.units[target].id);
                taken.push(target);
                let (tx, ty) = (self.units[target].x / fp(1), self.units[target].y / fp(1));
                self.move_towards(i, (tx, ty), fp(8));
                continue;
            }
            self.units[i].target = Some(self.units[target].id);
            taken.push(target);
            self.units[i].path.clear();
            self.units[i].path_goal = None;
            let (dx, dy) = (
                self.units[target].x - self.units[i].x,
                self.units[target].y - self.units[i].y,
            );
            if dx != 0 || dy != 0 {
                self.units[i].facing = direction_byte(dx, dy);
            }
            let max = self.data.unit(self.units[target].type_id).hit_points_raw as i32;
            // OpenBW `medic_try_heal`: up to 200/256 hp for half as much
            // energy (the last, smaller step costs less).
            let healed = MEDIC_HEAL_PER_TICK.min(max - self.units[target].hp);
            if healed > 0 && self.units[i].spend_energy_fixed(healed / 2) {
                self.units[target].hp += healed;
            }
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
mod tests {
    use super::*;

    #[test]
    fn the_psi_field_is_wide_rounded_and_only_the_owners() {
        // 16 x 10 tiles: 8 tiles to each side, 5 up and down (rounded).
        assert!(psi_field_covers(0, 0));
        assert!(psi_field_covers(7 * 32 + 31, 0), "eight tiles to the right");
        assert!(!psi_field_covers(8 * 32 + 31, 0));
        assert!(psi_field_covers(0, 4 * 32 + 31), "five tiles down");
        assert!(!psi_field_covers(0, 5 * 32 + 31));
        assert!(!psi_field_covers(7 * 32, 4 * 32), "the corners are cut");
        let mut w = World::new(
            Arc::new(GameData::bundled().unwrap()),
            crate::map::Terrain::open(64, 64),
            2,
        );
        let pylon = w.spawn_px(1, uid::PYLON, 640, 640).unwrap();
        w.units[pylon].completed = true;
        assert!(w.has_power_px(1, 700, 640));
        assert!(
            !w.has_power_px(0, 700, 640),
            "an enemy Pylon powers nothing of ours"
        );
    }

    fn open_world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    #[test]
    fn facing_follows_the_screen_compass_while_walking() {
        // A script walker (Marine) and a flingy (SCV, turn rate 40).
        for type_id in [uid::MARINE, uid::SCV] {
            for ((tx, ty), want) in [
                ((960, 640), 64),  // east
                ((640, 960), 128), // south
                ((320, 640), 192), // west
                ((640, 320), 0),   // north
            ] {
                let mut w = open_world();
                let i = w.spawn_px(0, type_id, 640, 640).unwrap();
                let id = w.units[i].id;
                w.issue(id, Order::Move { pos: (tx, ty) }).unwrap();
                for _ in 0..60 {
                    w.step();
                    let f = w.units[i].facing as i32;
                    let diff = (f - want + 128).rem_euclid(256) - 128;
                    if diff.abs() <= 2 && w.units[i].x != 640 * 256 {
                        break;
                    }
                }
                let f = w.units[i].facing as i32;
                let diff = (f - want + 128).rem_euclid(256) - 128;
                assert!(
                    diff.abs() <= 2,
                    "type {type_id} target {tx},{ty}: facing {f}, want {want}"
                );
            }
        }
    }

    #[test]
    fn cardinal_direction_vectors_point_north_east_south_west() {
        assert_eq!(direction_vector(0), (0, -256));
        assert_eq!(direction_vector(64), (256, 0));
        assert_eq!(direction_vector(128), (0, 256));
        assert_eq!(direction_vector(192), (-256, 0));
    }

    #[test]
    fn rescuable_units_are_nobodys_target_and_join_the_first_comer() {
        let mut w = World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            3,
        );
        w.players[2].rescuable = true;
        w.players[1].is_computer = true;
        // Positions in pixels: Raynor 20 px from the Marine, the other
        // rescuable unit ~2500 px away.
        let marine = w.spawn(0, uid::MARINE, 100, 100).unwrap();
        // units.dat 20: Jim Raynor (Marine), as placed by `terran04`.
        let raynor = w.spawn(2, 20, 120, 100).unwrap();
        let far = w.spawn(2, uid::MARINE, 1900, 1900).unwrap();
        // A computer enemy next to a rescuable unit does not take it.
        let held = w.spawn(2, uid::MARINE, 1000, 1000).unwrap();
        w.spawn(1, uid::MARINE, 1020, 1000).unwrap();
        let hp = w.units[raynor].hp;
        assert!(!w.hostile(0, 2) && !w.hostile(2, 1) && w.hostile(0, 1));
        // The retail handler polls on the first tick (Backwater Station's
        // Raynor line is on the first game frame), then every 15 ticks.
        w.step();
        assert_eq!(w.units[raynor].owner, 0, "rescued by the adjacent Marine");
        assert_eq!(w.units[marine].target, None);
        // One placed next to the Marine after the first poll waits for the
        // next one (tick 16) without being shot.
        let late = w.spawn(2, uid::MARINE, 100, 120).unwrap();
        for _ in 0..14 {
            w.step();
        }
        assert_eq!(w.units[late].owner, 2, "tick 15: not polled yet");
        w.step();
        assert_eq!(w.units[late].owner, 0, "tick 16: rescued");
        assert_eq!(w.units[raynor].hp, hp, "never attacked");
        assert_eq!(w.units[far].owner, 2, "nobody within reach");
        assert_eq!(w.units[held].owner, 2, "computers never rescue");
        assert_eq!(
            w.rescues,
            vec![(w.units[raynor].id, 0), (w.units[late].id, 0)]
        );
    }

    #[test]
    fn speed_clamp_matches_the_recovered_status_rule() {
        // FUN_005b6900: +50% with a 0x355 floor, hero types 0x46/0x50/0x58
        // pinned at 0x6ab, ensnare halves, and both statuses cancel.
        assert_eq!(World::modified_speed(uid::MARINE, 1024, true, false), 1536);
        assert_eq!(World::modified_speed(uid::MARINE, 100, true, false), 0x355);
        assert_eq!(World::modified_speed(0x46, 100, true, false), 0x6ab);
        assert_eq!(World::modified_speed(0x50, 100, true, false), 0x6ab);
        assert_eq!(World::modified_speed(0x58, 100, true, false), 0x6ab);
        assert_eq!(World::modified_speed(uid::MARINE, 1024, false, true), 512);
        assert_eq!(World::modified_speed(uid::MARINE, 1024, true, true), 1024);
        assert_eq!(World::modified_speed(uid::MARINE, 1024, false, false), 1024);
    }

    #[test]
    fn ticks_are_42_ms_of_real_time() {
        // Retail Desperate Alliance: game start 127.25 s, victory dialog
        // 1347.37 s, score "Elapsed Time: 20:20" (1220 s).
        let ticks = ((1347.37 - 127.25) * 1000.0 / TICK_MS as f64) as u64;
        assert_eq!(real_seconds(ticks), 1220);
        assert_eq!(real_seconds(28_800), 1209, "the 30-minute countdown");
    }

    #[test]
    fn score_tallies_follow_production_spending_and_kills() {
        let mut w = open_world();
        w.players[0].minerals = 1000;
        let cc = w.spawn(0, uid::COMMAND_CENTER, 300, 300).unwrap();
        let before = w.players[0].clone();
        w.train(w.units[cc].id, uid::SCV).expect("train SCV");
        for _ in 0..24 * 30 {
            w.step();
        }
        let p = &w.players[0];
        let scv = w.data.unit(uid::SCV).clone();
        assert_eq!(p.units_produced, 1, "one SCV produced");
        assert_eq!(
            p.scores[1], scv.build_score as i32,
            "units score = build score"
        );
        assert_eq!(
            p.minerals_spent - before.minerals_spent,
            scv.mineral_cost as i32
        );
        // Placed and starting units count too (OpenBW `add_completed_unit`;
        // a fresh retail melee start shows Structures 400).
        assert_eq!(p.structures_built, 1, "the placed CC counts");
        // A kill credits the destroy score and the loss to the victim.
        let enemy = w.spawn(1, uid::MARINE, 1500, 1500).unwrap();
        let marine_destroy = w.data.unit(uid::MARINE).destroy_score as i32;
        let shooter = w.spawn(0, uid::MARINE, 1800, 1800).unwrap();
        w.units[enemy].last_damage_source = w.units[shooter].id;
        let id = w.units[enemy].id;
        w.damage_unit(id, 100_000).expect("damage");
        w.step();
        assert_eq!(w.players[0].scores[4], marine_destroy);
        assert_eq!(w.players[1].units_lost, 1);
    }

    #[test]
    fn mission_worlds_leave_victory_to_the_triggers() {
        for trigger_victory in [false, true] {
            let mut w = open_world();
            w.trigger_victory = trigger_victory;
            w.spawn(0, uid::MARINE, 10, 10).unwrap();
            let enemy = w.spawn(1, uid::MARINE, 50, 50).unwrap();
            w.units[enemy].alive = false;
            w.step();
            let want = if trigger_victory { None } else { Some(0) };
            assert_eq!(w.winner, want, "trigger_victory = {trigger_victory}");
        }
    }

    #[test]
    fn miss_chances_are_exact_fractions_of_256() {
        // 255/256 under Dark Swarm, 119/256 on higher terrain, never 256/256.
        assert!(dodge_roll(0, 255));
        assert!(dodge_roll(254, 255));
        assert!(!dodge_roll(255, 255));
        assert!(dodge_roll(0, 119));
        assert!(dodge_roll(118, 119));
        assert!(!dodge_roll(119, 119));
        assert!(!dodge_roll(0, 0));
        // Over a full 256-value sweep exactly `chance` rolls miss.
        let misses = (0u32..256).filter(|r| dodge_roll(*r, 119)).count();
        assert_eq!(misses, 119);
        let misses = (0u32..256).filter(|r| dodge_roll(*r, 255)).count();
        assert_eq!(misses, 255);
    }

    #[test]
    fn dark_swarm_only_affects_ground_targets() {
        let mut w = open_world();
        let shooter = w.spawn(0, uid::MARINE, 10, 10).unwrap();
        let ground = w.spawn(1, uid::MARINE, 20, 10).unwrap();
        let air = w.spawn(1, uid::WRAITH, 20, 11).unwrap();
        assert_eq!(w.dodge_chance(shooter, ground), 0);
        // A target standing one elevation class higher misses 119/256.
        w.terrain.set_elevation_cell(20 / 8, 10 / 8, 2);
        assert_eq!(w.dodge_chance(shooter, ground), 119);
        w.terrain.set_elevation_cell(20 / 8, 10 / 8, 0);
        // A Dark Swarm cloud at the target's position.
        w.spawn(0, uid::DARK_SWARM, 20, 10).unwrap();
        assert_eq!(w.dodge_chance(shooter, ground), 255);
        assert_eq!(w.dodge_chance(shooter, air), 0);
    }

    #[test]
    fn marines_fight_and_damage_matches_the_pipeline() {
        let mut w = open_world();
        let a = w.spawn(0, uid::MARINE, 10, 10).unwrap();
        let b = w.spawn(1, uid::MARINE, 12, 10).unwrap();
        let start_hp = w.units[b].hp;
        for _ in 0..16 {
            w.step();
            if w.units[b].hp < start_hp {
                break;
            }
        }
        assert!(w.units[b].hp < start_hp);
        // The cooldown started when the Marine fired; its bullet (iscript
        // 238) dealt the damage two frames later.
        assert_eq!(w.units[a].cooldown, 15 - 2);
    }

    #[test]
    fn attack_animation_supplies_the_first_shot_timing() {
        let mut w = open_world();
        let _a = w.spawn(0, uid::MARINE, 10, 10).unwrap();
        let b = w.spawn(1, uid::MARINE, 12, 10).unwrap();
        let start = w.units[b].hp;
        let mut fired = None;
        for t in 1..=10 {
            w.step();
            if w.units[b].hp < start {
                fired = Some(t);
                break;
            }
        }
        assert_eq!(
            fired,
            Some(6),
            "the Marine attackwith opcode is reached on the fourth logical tick \
             and its bullet's domissiledmg two frames later"
        );
    }

    #[test]
    fn worker_mines_and_returns() {
        let mut w = open_world();
        let cc = w.spawn(0, uid::COMMAND_CENTER, 4, 4).unwrap();
        let mf = w.spawn_px(0, uid::MINERAL_FIELD, 12 * 32, 12 * 32).unwrap();
        w.units[mf].resource_amount = 1500;
        let scv = w.spawn(0, uid::SCV, 8, 8).unwrap();
        w.issue(
            w.units[scv].id,
            Order::Gather {
                resource: w.units[mf].id,
            },
        )
        .unwrap();
        for _ in 0..2000 {
            w.step();
            if w.players[0].minerals > 50 {
                break;
            }
        }
        assert!(w.players[0].minerals > 50, "worker should deposit minerals");
        let _ = cc;
    }

    #[test]
    fn production_requires_the_right_producer_and_supply() {
        let mut w = open_world();
        let rax = w.spawn(0, uid::BARRACKS, 10, 10).unwrap();
        // A Barracks has no supply of its own without a Command Center, so a
        // Marine cannot be queued even though the producer matches.
        assert_eq!(
            w.train(w.units[rax].id, uid::MARINE),
            Err(SimError::NotEnoughSupply)
        );
        let cc = w.spawn(0, uid::COMMAND_CENTER, 4, 4).unwrap();
        let _ = cc;
        assert_eq!(
            w.train(w.units[rax].id, uid::SCV),
            Err(SimError::MissingRequirements)
        );
    }

    #[test]
    fn barracks_trains_a_marine() {
        let mut w = open_world();
        let cc = w.spawn(0, uid::COMMAND_CENTER, 4, 4).unwrap();
        let depot = w.spawn(0, uid::SUPPLY_DEPOT, 8, 4).unwrap();
        let rax = w.spawn(0, uid::BARRACKS, 12, 4).unwrap();
        w.players[0].minerals = 1000;
        w.train(w.units[rax].id, uid::MARINE).unwrap();
        for _ in 0..1000 {
            w.step();
            if w.units.iter().any(|u| u.alive && u.type_id == uid::MARINE) {
                break;
            }
        }
        assert!(w.units.iter().any(|u| u.alive && u.type_id == uid::MARINE));
        let _ = (cc, depot);
    }

    #[test]
    fn tech_and_upgrade_are_tracked_per_player() {
        let mut w = open_world();
        let academy = w.spawn(0, uid::ACADEMY, 8, 8).unwrap();
        w.players[0].minerals = 1000;
        w.players[0].gas = 1000;
        w.research(w.units[academy].id, tech_id::STIM_PACKS)
            .unwrap();
        for _ in 0..3000 {
            w.step();
            if w.players[0].has_tech(tech_id::STIM_PACKS) {
                break;
            }
        }
        assert!(w.players[0].has_tech(tech_id::STIM_PACKS));
    }

    #[test]
    fn upgrade_costs_and_times_use_the_current_level() {
        let mut w = open_world();
        let bay = w.spawn(0, uid::ENGINEERING_BAY, 8, 8).unwrap();
        let bay_id = w.units[bay].id;
        w.players[0].minerals = 1000;
        w.players[0].gas = 1000;

        w.upgrade(bay_id, upgrade::TERRAN_INFANTRY_WEAPONS).unwrap();
        assert_eq!((w.players[0].minerals, w.players[0].gas), (900, 900));
        assert_eq!(w.units[bay].build_remaining, 4000);
        assert_eq!(w.units[bay].build_total, 4000);

        w.units[bay].build_remaining = 1;
        w.step();
        assert_eq!(
            w.players[0].upgrade_level(upgrade::TERRAN_INFANTRY_WEAPONS),
            1
        );

        w.spawn(0, uid::SCIENCE_FACILITY, 16, 8).unwrap();
        w.upgrade(bay_id, upgrade::TERRAN_INFANTRY_WEAPONS).unwrap();
        assert_eq!((w.players[0].minerals, w.players[0].gas), (725, 725));
        assert_eq!(w.units[bay].build_remaining, 4480);
        assert_eq!(w.units[bay].build_total, 4480);
    }

    #[test]
    fn energy_caps_follow_the_retail_reactor_table_and_hero_flag() {
        const BASE: i32 = MAX_ENERGY * ENERGY_SCALE;
        const CAP: i32 = MAX_ENERGY_UPGRADED * ENERGY_SCALE;
        let reactors = [
            (uid::GHOST, upgrade::MOEBIUS_REACTOR),
            (uid::WRAITH, upgrade::APOLLO_REACTOR),
            (uid::SCIENCE_VESSEL, upgrade::TITAN_REACTOR),
            (uid::BATTLECRUISER, upgrade::COLOSSUS_REACTOR),
            (uid::MEDIC, upgrade::CADUCEUS_REACTOR),
            (uid::QUEEN, upgrade::GAMETE_MEIOSIS),
            (uid::DEFILER, upgrade::METASYNAPTIC_NODE),
            (uid::CORSAIR, upgrade::ARGUS_JEWEL),
            (uid::DARK_ARCHON, upgrade::ARGUS_TALISMAN),
            (uid::HIGH_TEMPLAR, upgrade::KHAYDARIN_AMULET),
            (uid::ARBITER, upgrade::KHAYDARIN_CORE),
        ];
        for (unit, upgrade_id) in reactors {
            let mut w = open_world();
            let i = w.spawn(0, unit, 10, 10).unwrap();
            w.units[i].set_energy_fixed(CAP);
            assert_eq!(w.units[i].energy_fixed(), BASE, "unit {unit}");
            w.players[0].upgrades[upgrade_id as usize] = 1;
            w.refresh_energy_caps();
            w.units[i].set_energy_fixed(CAP);
            assert_eq!(w.units[i].energy_fixed(), CAP, "unit {unit}");
        }
        let mut w = open_world();
        let hero = w
            .data
            .units
            .iter()
            .position(|u| u.is_hero() && u.flags & 0x0020_0000 != 0)
            .expect("a spellcasting hero") as u16;
        let i = w.spawn(0, hero, 10, 10).unwrap();
        w.units[i].set_energy_fixed(CAP);
        assert_eq!(w.units[i].energy_fixed(), CAP, "hero energy cap");
    }

    #[test]
    fn siege_splash_skips_air_and_spares_its_source() {
        let mut w = open_world();
        let siege = w.spawn(0, uid::SIEGE_MODE_TURRET, 320, 320).unwrap();
        let target = w.spawn(1, uid::MARINE, 420, 320).unwrap();
        let wraith = w.spawn(1, uid::WRAITH, 440, 320).unwrap();
        let friendly = w.spawn(0, uid::MARINE, 430, 320).unwrap();
        let weapon = w.data.weapon(wid::ARCLITE_SHOCK_CANNON).cloned().unwrap();
        let (siege_hp, friendly_hp, wraith_hp) =
            (w.units[siege].hp, w.units[friendly].hp, w.units[wraith].hp);
        let pos = (w.units[target].x, w.units[target].y);
        w.resolve_hit_type(
            0,
            Some(w.units[target].id),
            siege,
            wid::ARCLITE_SHOCK_CANNON,
            &weapon,
            pos,
            1,
            1,
        );
        assert_eq!(w.units[siege].hp, siege_hp, "splash never hits its source");
        assert_eq!(
            w.units[wraith].hp, wraith_hp,
            "ground-only splash skips air"
        );
        assert!(
            w.units[friendly].hp < friendly_hp,
            "same-owner units still take radial splash"
        );
    }

    #[test]
    fn siege_mode_switches_unit_type() {
        let mut w = open_world();
        let tank = w.spawn(0, uid::SIEGE_TANK, 8, 8).unwrap();
        w.players[0].techs[tech_id::TANK_SIEGE_MODE as usize] = true;
        w.issue(w.units[tank].id, Order::Siege).unwrap();
        // It first turns to heading 96 (13 per frame from 0: 8 frames).
        for _ in 0..8 {
            w.step();
            assert_eq!(w.units[tank].type_id, uid::SIEGE_TANK);
        }
        w.step();
        assert_eq!(w.units[tank].type_id, uid::SIEGE_MODE);
        // Busy for the deploy animation, then idle again.
        assert!(w.is_transforming(tank));
        for _ in 0..=SIEGE_FRAMES {
            w.step();
        }
        assert!(!w.is_transforming(tank));
        assert_eq!(w.units[tank].order, Order::Idle);
    }

    #[test]
    fn travelling_weapons_create_a_bullet_and_hit_on_arrival() {
        let mut w = open_world();
        // Missile Turret air weapons are BulletType 1 (follow target), unlike
        // the Marine's instant appear-at-target rifle.
        let a = w.spawn(0, uid::MISSILE_TURRET, 320, 320).unwrap();
        let b = w.spawn(1, uid::DROPSHIP, 480, 320).unwrap();
        let start = w.units[b].hp;
        for _ in 0..16 {
            w.step();
            if !w.projectiles.is_empty() {
                break;
            }
        }
        assert!(!w.projectiles.is_empty(), "the turret missile must travel");
        assert_eq!(w.units[b].hp, start, "damage is not applied at fire time");
        for _ in 0..120 {
            w.step();
            if w.units[b].hp < start {
                break;
            }
        }
        assert!(
            w.units[b].hp < start,
            "the bullet must deal damage on impact"
        );
        let _ = a;
    }

    #[test]
    fn instant_weapons_still_hit_on_the_firing_tick() {
        let mut w = open_world();
        let a = w.spawn(0, uid::MARINE, 10, 10).unwrap();
        let b = w.spawn(1, uid::MARINE, 12, 10).unwrap();
        let start = w.units[b].hp;
        for _ in 0..16 {
            w.step();
            if w.units[b].hp < start {
                break;
            }
        }
        assert!(w.projectiles.is_empty(), "Gauss Rifle is instant");
        assert!(w.units[b].hp < start);
        let _ = a;
    }

    #[test]
    fn cloak_requires_a_detector() {
        let mut w = open_world();
        let ghost = w.spawn(0, uid::GHOST, 10, 10).unwrap();
        w.players[0].techs[tech_id::PERSONNEL_CLOAKING as usize] = true;
        w.set_cloak(w.units[ghost].id, true).unwrap();
        let marine = w.spawn(1, uid::MARINE, 12, 10).unwrap();
        for _ in 0..20 {
            w.step();
        }
        assert_eq!(
            w.units[ghost].hp,
            w.data.unit(uid::GHOST).hit_points_raw as i32
        );
        // A Missile Turret is a detector and makes the Ghost targetable.
        let turret = w.spawn(1, uid::MISSILE_TURRET, 14, 10).unwrap();
        let start = w.units[ghost].hp;
        for _ in 0..50 {
            w.step();
            if w.units[ghost].hp < start {
                break;
            }
        }
        assert!(
            w.units[ghost].hp < start,
            "detector should reveal the Ghost"
        );
        let _ = (marine, turret);
    }

    #[test]
    fn detector_radius_is_seven_tiles_in_fixed_point_space() {
        // The cloaked unit's position is in 1/256 pixels while the detector
        // radius is in pixels; comparing them directly made the radius less
        // than one pixel.  The recovered building radius is seven tiles.
        let mut w = open_world();
        let ghost = w.spawn(0, uid::GHOST, 320, 320).unwrap();
        w.players[0].techs[tech_id::PERSONNEL_CLOAKING as usize] = true;
        w.set_cloak(w.units[ghost].id, true).unwrap();
        let turret = w.spawn(1, uid::MISSILE_TURRET, 320 + 6 * 32, 320).unwrap();
        assert!(w.can_see_unit(1, ghost), "six tiles is inside the radius");
        w.units[turret].x = (320 + 7 * 32) * fp(1); // exactly seven tiles
        assert!(w.can_see_unit(1, ghost));
        w.units[turret].x = (320 + 8 * 32) * fp(1); // eight tiles
        assert!(!w.can_see_unit(1, ghost));
    }

    #[test]
    fn energy_regenerates_at_eight_256ths_per_tick() {
        // `CUnit+0xa2` is fixed point and the update routine adds 8 each
        // logical tick, so one whole energy point takes 32 ticks.
        let mut w = open_world();
        let ghost = w.spawn(0, uid::GHOST, 10, 10).unwrap();
        w.units[ghost].set_energy_fixed(0);
        for _ in 0..32 {
            w.step();
        }
        assert_eq!(w.units[ghost].energy_fixed(), ENERGY_SCALE);
    }

    #[test]
    fn personal_cloak_drains_its_256ths_and_suppresses_regen() {
        let mut w = open_world();
        let ghost = w.spawn(0, uid::GHOST, 10, 10).unwrap();
        w.players[0].techs[tech_id::PERSONNEL_CLOAKING as usize] = true;
        w.units[ghost].set_energy_fixed(MAX_ENERGY * ENERGY_SCALE);
        w.set_cloak(w.units[ghost].id, true).unwrap();
        for _ in 0..25 {
            w.step();
        }
        // 25 ticks * 10/256 drained, no +8 regen while cloaked.
        assert_eq!(
            w.units[ghost].energy_fixed(),
            // 25 energy to cloak (techdata), then the drain.
            MAX_ENERGY * ENERGY_SCALE - 25 * ENERGY_SCALE - 25 * GHOST_CLOAK_COST_PER_TICK
        );
        assert!(w.units[ghost].status.cloaked);

        let wraith = w.spawn(0, uid::WRAITH, 20, 20).unwrap();
        w.players[0].techs[tech_id::CLOAKING_FIELD as usize] = true;
        // Under the 25-energy cost the cloak is refused.
        w.units[wraith].set_energy_fixed(1000);
        assert_eq!(
            w.set_cloak(w.units[wraith].id, true),
            Err(SimError::NotEnoughEnergy)
        );
        w.units[wraith].set_energy_fixed(1000 + 25 * ENERGY_SCALE);
        w.set_cloak(w.units[wraith].id, true).unwrap();
        w.step();
        assert_eq!(
            w.units[wraith].energy_fixed(),
            1000 - WRAITH_CLOAK_COST_PER_TICK
        );
    }

    #[test]
    fn medic_heals_200_256ths_for_100_256ths_energy() {
        let mut w = open_world();
        let medic = w.spawn(0, uid::MEDIC, 320, 320).unwrap();
        let marine = w.spawn(0, uid::MARINE, 340, 320).unwrap();
        w.units[marine].hp = fp(10);
        w.units[medic].set_energy_fixed(ENERGY_SCALE);
        let before = w.units[marine].hp;
        w.step();
        assert_eq!(w.units[marine].hp - before, MEDIC_HEAL_PER_TICK);
        // +8 regen then -100 heal cost.
        assert_eq!(
            w.units[medic].energy_fixed(),
            ENERGY_SCALE + ENERGY_REGEN_PER_TICK - MEDIC_HEAL_COST_PER_TICK
        );
    }

    #[test]
    fn acid_spores_stack_nine_independent_150_update_timers() {
        // `FUN_005dc360` increments `CUnit+0x126` up to nine and writes
        // `0x96` into one of the nine `CUnit+0x127..+0x12f` slots;
        // `FUN_005dc940` expires each slot independently every 8th tick.
        let mut status = Status::default();
        for _ in 0..12 {
            status.add_acid_spore();
        }
        assert_eq!(status.acid_spores, 9);
        assert!(status.acid_spore_ticks.iter().all(|&t| t > 0));
        for _ in 0..ACID_SPORE_TICKS - 1 {
            status.tick_acid_spores();
        }
        assert_eq!(status.acid_spores, 9, "stacks last 150 updates");
        status.tick_acid_spores();
        assert_eq!(status.acid_spores, 0);
        assert_eq!(status.acid_spore_ticks, [0; 9]);
    }

    #[test]
    fn acid_spore_refreshes_the_stack_closest_to_expiring_when_full() {
        // With all nine slots busy the apply routine scans for the smallest
        // remaining timer and refreshes that slot, not the first one.
        let mut status = Status {
            acid_spore_ticks: [150, 10, 90, 120, 60, 30, 140, 80, 20],
            acid_spores: 9,
            ..Status::default()
        };
        status.add_acid_spore();
        assert_eq!(status.acid_spores, 9);
        assert_eq!(status.acid_spore_ticks[1], ACID_SPORE_TICKS);
        assert_eq!(status.acid_spore_ticks[0], 150);
    }

    #[test]
    fn acid_spore_timers_drop_one_stack_per_expiry() {
        let mut status = Status {
            acid_spore_ticks: [1, 2, 0, 0, 0, 0, 0, 0, 0],
            acid_spores: 2,
            ..Status::default()
        };
        status.tick_acid_spores();
        assert_eq!(status.acid_spores, 1);
        assert_eq!(status.acid_spore_ticks[0], 0);
        assert_eq!(status.acid_spore_ticks[1], 1);
        status.tick_acid_spores();
        assert_eq!(status.acid_spores, 0);
    }

    #[test]
    fn corrosive_acid_spores_the_128_px_box_but_damages_one_target() {
        // `FUN_005dc1c0` passes a 0x40 half-extent to `FUN_006c1ee0`, which
        // calls `FUN_005dc360` for every unit in the resulting 128 x 128 box.
        // Weapon 104's damage splash radii are all zero, so only the target
        // takes the 25 damage while nearby air units only gain a stack.
        let mut w = open_world();
        let queen = w.spawn(0, uid::QUEEN, 320, 320).unwrap();
        let a = w.spawn(1, uid::WRAITH, 360, 320).unwrap();
        let b = w.spawn(1, uid::WRAITH, 380, 320).unwrap();
        let friendly = w.spawn(0, uid::WRAITH, 380, 340).unwrap();
        let far = w.spawn(1, uid::WRAITH, 460, 320).unwrap();
        let weapon = w.data.weapon(104).cloned().expect("corrosive acid");
        assert_eq!(weapon.explosion, crate::projectile::hit::CORROSIVE_ACID);
        assert_eq!(
            (
                weapon.inner_splash,
                weapon.middle_splash,
                weapon.outer_splash
            ),
            (0, 0, 0)
        );
        let (hp_a, hp_b) = (w.units[a].hp, w.units[b].hp);
        let pos = (w.units[a].x, w.units[a].y);
        w.resolve_hit_type(0, Some(w.units[a].id), queen, 104, &weapon, pos, 1, 1);
        assert!(w.units[a].hp < hp_a, "the struck unit takes damage");
        assert_eq!(w.units[b].hp, hp_b, "no splash damage");
        assert_eq!(w.units[a].status.acid_spores, 1);
        assert_eq!(w.units[b].status.acid_spores, 1);
        assert_eq!(
            w.units[friendly].status.acid_spores, 0,
            "same owner as the attacker"
        );
        assert_eq!(w.units[far].status.acid_spores, 0);
    }

    #[test]
    fn acid_spore_stacks_expire_in_the_world_tick() {
        let mut w = open_world();
        let queen = w.spawn(0, uid::QUEEN, 320, 320).unwrap();
        let bc = w.spawn(1, uid::BATTLECRUISER, 360, 320).unwrap();
        let weapon = w.data.weapon(104).cloned().expect("corrosive acid");
        let pos = (w.units[bc].x, w.units[bc].y);
        w.resolve_hit_type(0, Some(w.units[bc].id), queen, 104, &weapon, pos, 1, 1);
        assert_eq!(w.units[bc].status.acid_spores, 1);
        let expire_ticks = u64::from(ACID_SPORE_TICKS) * ACID_SPORE_UPDATE_INTERVAL;
        for _ in 0..expire_ticks - 1 {
            w.step();
        }
        assert_eq!(w.units[bc].status.acid_spores, 1);
        w.step();
        assert_eq!(w.units[bc].status.acid_spores, 0);
    }

    #[test]
    fn restoration_clears_acid_spores_and_parasite() {
        let mut w = open_world();
        let medic = w.spawn(0, uid::MEDIC, 320, 320).unwrap();
        let bc = w.spawn(0, uid::BATTLECRUISER, 340, 320).unwrap();
        w.units[bc].status.add_acid_spore();
        w.units[bc].status.add_acid_spore();
        w.units[bc].status.parasite = true;
        assert_eq!(w.units[bc].status.acid_spores, 2);
        w.apply_spell(medic, Spell::Restoration, SpellTarget::Unit(w.units[bc].id));
        // Its bullet (iscript 292) lands seven frames after the cast.
        assert_eq!(w.units[bc].status.acid_spores, 2);
        for _ in 0..8 {
            w.step();
        }
        assert_eq!(w.units[bc].status.acid_spores, 0);
        assert_eq!(w.units[bc].status.acid_spore_ticks, [0; 9]);
        assert!(!w.units[bc].status.parasite);
    }

    #[test]
    fn restoration_worthwhile_matches_fun_005c7460() {
        let mut w = open_world();
        let medic = w.spawn(0, uid::MEDIC, 320, 320).unwrap();
        let bc = w.spawn(0, uid::BATTLECRUISER, 340, 320).unwrap();
        let (medic_id, bc_id) = (w.units[medic].id, w.units[bc].id);
        assert!(
            !w.restoration_worthwhile(medic_id, bc_id),
            "nothing to cure"
        );
        w.units[bc].status.plague_ticks = 100;
        assert!(w.restoration_worthwhile(medic_id, bc_id));

        // 40 HP + 30 shields = 70 whole points passes the 0x3d threshold;
        // 40 + 20 = 60 fails it.
        let zealot = w.spawn(0, uid::ZEALOT, 360, 320).unwrap();
        w.units[zealot].status.ensnare_ticks = 100;
        w.units[zealot].hp = fp(40);
        w.units[zealot].shields = fp(30);
        assert!(w.restoration_worthwhile(medic_id, w.units[zealot].id));
        w.units[zealot].shields = fp(20);
        assert!(!w.restoration_worthwhile(medic_id, w.units[zealot].id));

        let enemy = w.spawn(1, uid::ZEALOT, 380, 320).unwrap();
        w.units[enemy].status.plague_ticks = 100;
        assert!(!w.restoration_worthwhile(medic_id, w.units[enemy].id));

        let cc = w.spawn(0, uid::COMMAND_CENTER, 400, 320).unwrap();
        w.units[cc].status.plague_ticks = 100;
        assert!(!w.restoration_worthwhile(medic_id, w.units[cc].id));
    }

    #[test]
    fn acid_spores_reveal_a_cloaked_unit() {
        let mut w = open_world();
        let ghost = w.spawn(1, uid::GHOST, 320, 320).unwrap();
        w.units[ghost].status.cloaked = true;
        w.units[ghost].status.revealed_ticks = 0;
        assert!(!w.can_see_unit(0, ghost), "no detector");
        w.units[ghost].status.add_acid_spore();
        assert!(w.can_see_unit(0, ghost), "spores reveal cloaking");
    }

    #[test]
    fn air_units_do_not_block_ground_paths() {
        let mut w = open_world();
        let wraith = w.spawn(0, uid::WRAITH, 320, 320).unwrap();
        w.units[wraith].completed = true;
        w.build_obstacles();
        assert!(w.obstacles.is_empty(), "flyers are not ground obstacles");
    }

    #[test]
    fn door_units_are_not_targets_or_colliders() {
        // `FUN_005b5b00` returns true for the four door types 0xcd..0xd0.
        let mut w = open_world();
        for ty in 0xcd..=0xd0 {
            assert!(super::is_door_type(ty));
        }
        assert!(!super::is_door_type(uid::MARINE));
        let door = w.spawn(1, 0xcd, 340, 320).unwrap();
        w.units[door].completed = true;
        assert!(w.collision_extents(door).is_none());
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        assert!(
            w.acquire_target(marine).is_none(),
            "doors are excluded from automatic targeting"
        );
    }

    #[test]
    fn closed_door_is_a_path_obstacle_and_open_door_is_not() {
        let mut w = open_world();
        let door = w.spawn(0, 0xcd, 340, 320).unwrap();
        w.units[door].completed = true;
        w.units[door].doodad_enabled = Some(true);
        w.build_obstacles();
        assert!(w.collision_extents(door).is_none());
        assert!(w.obstacles.blocks_point(340, 320));

        w.units[door].doodad_enabled = Some(false);
        w.build_obstacles();
        assert!(w.obstacles.is_empty());
        assert!(!w.obstacles.blocks_point(340, 320));
    }

    #[test]
    fn path_obstacles_use_the_asymmetric_units_dat_box() {
        // Barracks extents are left 48, top 40, right 56, bottom 32.  The
        // old symmetric `max` made it block 56/40 px on every side; the
        // recovered path context keeps the four directions separate.
        let mut w = open_world();
        let barracks = w.spawn(0, uid::BARRACKS, 320, 320).unwrap();
        w.units[barracks].completed = true;
        w.build_obstacles();
        assert_eq!(w.obstacles.len(), 1);
        assert!(w.obstacles.blocks_point(272, 320));
        assert!(!w.obstacles.blocks_point(271, 320));
        assert!(w.obstacles.blocks_point(375, 320));
        assert!(!w.obstacles.blocks_point(376, 320));
        assert!(w.obstacles.blocks_point(320, 280));
        assert!(!w.obstacles.blocks_point(320, 279));
        assert!(w.obstacles.blocks_point(320, 351));
        assert!(!w.obstacles.blocks_point(320, 352));
    }

    #[test]
    fn feedback_deals_the_full_drained_energy_through_matrix() {
        let mut w = open_world();
        w.players[0].techs[tech_id::FEEDBACK as usize] = true;
        let archon = w.spawn(0, uid::DARK_ARCHON, 320, 320).unwrap();
        let bc = w.spawn(1, uid::BATTLECRUISER, 340, 320).unwrap();
        w.units[bc].set_energy_fixed(200 * ENERGY_SCALE);
        let before = w.units[bc].hp;
        w.cast(
            w.units[archon].id,
            Spell::Feedback,
            SpellTarget::Unit(w.units[bc].id),
        )
        .unwrap();
        assert_eq!(w.units[bc].hp, before - 200 * fp(1));
        assert_eq!(w.units[bc].energy_fixed(), 0);

        // Defensive Matrix absorbs the feedback damage first.
        let bc2 = w.spawn(1, uid::BATTLECRUISER, 360, 320).unwrap();
        w.units[bc2].set_energy_fixed(100 * ENERGY_SCALE);
        w.units[bc2].status.apply_matrix(fp(40));
        let before = w.units[bc2].hp;
        w.cast(
            w.units[archon].id,
            Spell::Feedback,
            SpellTarget::Unit(w.units[bc2].id),
        )
        .unwrap();
        assert_eq!(w.units[bc2].status.matrix, 0);
        assert_eq!(w.units[bc2].hp, before - 60 * fp(1));
    }

    #[test]
    fn defensive_matrix_absorbs_and_breaks_like_fun_005d2300() {
        let mut status = Status::default();
        status.apply_matrix(1000);
        assert!(status.matrix_active());
        assert_eq!(status.matrix_ticks, DEFENSIVE_MATRIX_UPDATES);
        assert_eq!(status.absorb_matrix(400), (400, false));
        assert_eq!(status.matrix, 600);
        assert!(status.matrix_active());
        assert_eq!(status.matrix_ticks, DEFENSIVE_MATRIX_UPDATES);
        assert_eq!(status.absorb_matrix(600), (600, true));
        assert_eq!(status.matrix, 0);
        assert!(!status.matrix_active());
        assert_eq!(status.matrix_ticks, 0);
        // Overkill consumes only what is left, even if the hit is larger.
        status.apply_matrix(250);
        assert_eq!(status.absorb_matrix(10_000), (250, true));
        assert_eq!(status.matrix, 0);
        assert!(!status.matrix_active());
    }

    #[test]
    fn defensive_matrix_expires_after_168_status_updates() {
        let mut status = Status::default();
        status.apply_matrix(64000);
        for _ in 0..DEFENSIVE_MATRIX_UPDATES - 1 {
            assert_eq!(status.tick_matrix(), None);
        }
        assert_eq!(status.matrix, 64000);
        assert_eq!(status.tick_matrix(), Some(64000));
        assert_eq!(status.matrix, 0);
        assert!(!status.matrix_active());
        assert_eq!(status.tick_matrix(), None);
    }

    #[test]
    fn defensive_matrix_expires_in_the_world_tick() {
        let mut w = open_world();
        let bc = w.spawn(0, uid::BATTLECRUISER, 320, 320).unwrap();
        w.units[bc].status.apply_matrix(fp(250));
        let expire_ticks = u64::from(DEFENSIVE_MATRIX_UPDATES) * ACID_SPORE_UPDATE_INTERVAL;
        for _ in 0..expire_ticks - 1 {
            w.step();
        }
        assert!(w.units[bc].status.matrix_active());
        w.step();
        assert!(!w.units[bc].status.matrix_active());
        assert_eq!(w.units[bc].status.matrix, 0);
        assert_eq!(w.matrix_cues.len(), 1);
        assert!(w.matrix_cues[0].broke);
        assert_eq!(w.matrix_cues[0].absorbed, fp(250));
    }

    #[test]
    fn matrix_cues_record_partial_hits_and_the_break() {
        let mut w = open_world();
        let vessel = w.spawn(0, uid::SCIENCE_VESSEL, 320, 320).unwrap();
        let bc = w.spawn(0, uid::BATTLECRUISER, 340, 320).unwrap();
        w.apply_spell(
            vessel,
            Spell::DefensiveMatrix,
            SpellTarget::Unit(w.units[bc].id),
        );
        assert_eq!(w.units[bc].status.matrix, fp(250));
        assert!(w.units[bc].status.matrix_active());
        assert_eq!(w.units[bc].status.matrix_ticks, DEFENSIVE_MATRIX_UPDATES);

        let marine = w.spawn(1, uid::MARINE, 360, 320).unwrap();
        let rifle = w.data.weapon(wid::GAUSS_RIFLE).cloned().unwrap();
        let pos = (w.units[bc].x, w.units[bc].y);
        w.resolve_hit_type(
            1,
            Some(w.units[bc].id),
            marine,
            wid::GAUSS_RIFLE,
            &rifle,
            pos,
            1,
            1,
        );
        assert_eq!(w.matrix_cues.len(), 1);
        assert_eq!(w.matrix_cues[0].absorbed, fp(6));
        assert!(!w.matrix_cues[0].broke, "the matrix survives one rifle hit");
        assert_eq!(w.units[bc].status.matrix, fp(244));
        assert!(w.units[bc].status.matrix_active());

        // A hit larger than the remaining pool consumes the pool, clears the
        // overlay flag and emits the break cue.
        let mut heavy = rifle.clone();
        heavy.damage = 300;
        w.resolve_hit_type(
            1,
            Some(w.units[bc].id),
            marine,
            wid::GAUSS_RIFLE,
            &heavy,
            pos,
            1,
            1,
        );
        assert_eq!(w.matrix_cues.len(), 2);
        assert_eq!(w.matrix_cues[1].absorbed, fp(244));
        assert!(w.matrix_cues[1].broke);
        assert_eq!(w.units[bc].status.matrix, 0);
        assert!(!w.units[bc].status.matrix_active());

        // Without an active matrix no further cue is produced.
        w.resolve_hit_type(
            1,
            Some(w.units[bc].id),
            marine,
            wid::GAUSS_RIFLE,
            &rifle,
            pos,
            1,
            1,
        );
        assert_eq!(w.matrix_cues.len(), 2);

        // Cues belong to the tick that produced them.
        w.step();
        assert!(w.matrix_cues.is_empty());
    }

    #[test]
    fn status_timers_use_the_recovered_eight_tick_updates() {
        let mut w = open_world();
        let marine = w.spawn(1, uid::MARINE, 320, 320).unwrap();
        {
            let s = &mut w.units[marine].status;
            s.lockdown_ticks = 2;
            s.ensnare_ticks = 2;
            s.maelstrom_ticks = 2;
        }
        for _ in 0..7 {
            w.step();
        }
        assert_eq!(w.units[marine].status.lockdown_ticks, 2);
        assert_eq!(w.units[marine].status.ensnare_ticks, 2);
        assert_eq!(w.units[marine].status.maelstrom_ticks, 2);
        w.step();
        assert_eq!(w.units[marine].status.lockdown_ticks, 1);
        assert_eq!(w.units[marine].status.ensnare_ticks, 1);
        assert_eq!(w.units[marine].status.maelstrom_ticks, 1);
    }

    #[test]
    fn ensnare_affects_air_but_not_buildings_burrowed_or_the_caster() {
        let mut w = open_world();
        let caster = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let air = w.spawn(1, uid::MUTALISK, 350, 320).unwrap();
        let building = w.spawn(1, uid::HATCHERY, 340, 340).unwrap();
        w.units[building].completed = true;
        let burrowed = w.spawn(1, uid::ZERGLING, 330, 320).unwrap();
        w.units[burrowed].status.burrowed = true;
        let ground = w.spawn(1, uid::ZERGLING, 360, 320).unwrap();
        w.apply_spell(caster, Spell::Ensnare, SpellTarget::Point(340, 320));
        assert_eq!(w.units[air].status.ensnare_ticks, ENSNARE_UPDATES);
        assert_eq!(w.units[ground].status.ensnare_ticks, ENSNARE_UPDATES);
        assert_eq!(w.units[building].status.ensnare_ticks, 0);
        assert_eq!(w.units[burrowed].status.ensnare_ticks, 0);
        assert_eq!(w.units[caster].status.ensnare_ticks, 0);
    }

    #[test]
    fn area_status_spells_skip_burrowed_and_stasis_maelstrom_skip_buildings() {
        let mut w = open_world();
        let caster = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let buried = w.spawn(1, uid::ZERGLING, 360, 320).unwrap();
        w.units[buried].status.burrowed = true;
        let building = w.spawn(1, uid::HATCHERY, 380, 320).unwrap();
        w.units[building].completed = true;
        let normal = w.spawn(1, uid::ZERGLING, 340, 320).unwrap();
        let point = SpellTarget::Point(360, 320);

        w.apply_spell(caster, Spell::Ensnare, point);
        assert_eq!(w.units[buried].status.ensnare_ticks, 0);
        assert_eq!(w.units[normal].status.ensnare_ticks, ENSNARE_UPDATES);

        w.apply_spell(caster, Spell::Plague, point);
        assert_eq!(w.units[buried].status.plague_ticks, 0);
        assert_eq!(w.units[normal].status.plague_ticks, PLAGUE_UPDATES);

        w.apply_spell(caster, Spell::StasisField, point);
        assert_eq!(w.units[buried].status.stasis_ticks, 0);
        assert_eq!(w.units[building].status.stasis_ticks, 0);
        assert_eq!(w.units[normal].status.stasis_ticks, STASIS_UPDATES);

        w.apply_spell(caster, Spell::Maelstrom, point);
        assert_eq!(w.units[buried].status.maelstrom_ticks, 0);
        assert_eq!(w.units[building].status.maelstrom_ticks, 0);
        assert_eq!(w.units[normal].status.maelstrom_ticks, MAELSTROM_UPDATES);
    }

    #[test]
    fn stasis_expires_on_the_eighth_tick_and_freezes_the_unit() {
        let mut w = open_world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let target = (400, 320);
        w.issue(w.units[marine].id, Order::Move { pos: target })
            .unwrap();
        w.units[marine].status.stasis_ticks = 2;
        let start = (w.units[marine].x, w.units[marine].y);
        for _ in 0..7 {
            w.step();
        }
        assert_eq!(w.units[marine].status.stasis_ticks, 2);
        assert_eq!((w.units[marine].x, w.units[marine].y), start);
        w.step();
        assert_eq!(w.units[marine].status.stasis_ticks, 1);
        for _ in 0..8 {
            w.step();
        }
        assert_eq!(w.units[marine].status.stasis_ticks, 0);
        let frozen = (w.units[marine].x, w.units[marine].y);
        for _ in 0..8 {
            w.step();
        }
        assert_ne!((w.units[marine].x, w.units[marine].y), frozen);
    }

    #[test]
    fn irradiate_deals_weapon_34_damage_every_eight_ticks() {
        let mut w = open_world();
        let marine = w.spawn(1, uid::MARINE, 320, 320).unwrap();
        let before = w.units[marine].hp;
        w.units[marine].status.irradiate_ticks = 1;
        w.step();
        assert_eq!(w.units[marine].hp, before);
        for _ in 0..7 {
            w.step();
        }
        // 250 * 256 / 75 = 853 in 1/256 points.
        assert_eq!(w.units[marine].hp, before - 853);
        assert_eq!(w.units[marine].status.irradiate_ticks, 0);
        let after = w.units[marine].hp;
        for _ in 0..8 {
            w.step();
        }
        assert_eq!(w.units[marine].hp, after, "expired Irradiate stops");
    }

    #[test]
    fn irradiate_mechanical_source_splashes_only_biological_units_in_32_px() {
        let mut w = open_world();
        let source = w.spawn(0, uid::VULTURE, 320, 320).unwrap();
        let near = w.spawn(0, uid::MARINE, 340, 320).unwrap();
        let burrowed = w.spawn(0, uid::MARINE, 330, 320).unwrap();
        w.units[burrowed].status.burrowed = true;
        let building = w.spawn(0, uid::HATCHERY, 340, 340).unwrap();
        w.units[building].completed = true;
        let far = w.spawn(0, uid::MARINE, 500, 320).unwrap();
        w.units[source].status.irradiate_ticks = 1;
        let (source_hp, near_hp, burrowed_hp, building_hp, far_hp) = (
            w.units[source].hp,
            w.units[near].hp,
            w.units[burrowed].hp,
            w.units[building].hp,
            w.units[far].hp,
        );
        for _ in 0..8 {
            w.step();
        }
        assert_eq!(w.units[source].hp, source_hp, "mechanical source is immune");
        assert_eq!(w.units[near].hp, near_hp - 853);
        assert_eq!(
            w.units[burrowed].hp, burrowed_hp,
            "splash skips burrowed units"
        );
        assert_eq!(w.units[building].hp, building_hp);
        assert_eq!(w.units[far].hp, far_hp);
    }

    #[test]
    fn state_hash_covers_future_deciding_unit_fields() {
        use crate::replay::state_hash;
        let data = Arc::new(GameData::bundled().unwrap());
        let base = || {
            let mut w = World::new(data.clone(), Terrain::open(8, 8), 2);
            w.spawn(0, uid::MARINE, 100, 100).unwrap();
            w
        };
        let h0 = state_hash(&base());
        let mut w = base();
        w.units[0].energy_frac = 1;
        assert_ne!(h0, state_hash(&w), "energy fraction");
        let mut w = base();
        w.units[0].larva_timer = 1;
        assert_ne!(h0, state_hash(&w), "larva timer");
        let mut w = base();
        w.units[0].expires_at = 100;
        assert_ne!(h0, state_hash(&w), "temporary-unit expiry");
        let mut w = base();
        w.units[0].order_carry = 3;
        assert_ne!(h0, state_hash(&w), "order carry");
        let mut w = base();
        w.units[0].path.push_back((7, 9));
        assert_ne!(h0, state_hash(&w), "path queue");
    }

    #[test]
    fn transports_reject_enemy_and_unfinished_carriers() {
        let mut w = open_world();
        let dropship = w.spawn(0, uid::DROPSHIP, 320, 320).unwrap();
        let dropship_id = w.units[dropship].id;
        let enemy = w.spawn(1, uid::MARINE, 330, 320).unwrap();
        let enemy_id = w.units[enemy].id;
        w.issue(
            enemy_id,
            Order::EnterTransport {
                transport: dropship_id,
            },
        )
        .unwrap();
        for _ in 0..12 {
            w.step();
        }
        assert!(w.units[enemy].alive);
        assert_eq!(w.units[enemy].in_transport, None);

        let marine = w.spawn(0, uid::MARINE, 330, 320).unwrap();
        let marine_id = w.units[marine].id;
        w.units[dropship].completed = false;
        w.issue(
            marine_id,
            Order::EnterTransport {
                transport: dropship_id,
            },
        )
        .unwrap();
        for _ in 0..12 {
            w.step();
        }
        assert_eq!(w.units[marine].in_transport, None);
        w.units[dropship].completed = true;
        w.issue(
            marine_id,
            Order::EnterTransport {
                transport: dropship_id,
            },
        )
        .unwrap();
        for _ in 0..12 {
            w.step();
        }
        assert_eq!(w.units[marine].in_transport, Some(dropship_id));
    }

    #[test]
    fn temporary_units_expire_on_the_recovered_lifetime_timer() {
        let mut w = open_world();
        let id = w
            .spawn_temporary(0, uid::DISRUPTION_WEB, (320, 320), 8)
            .unwrap();
        for _ in 0..7 {
            w.step();
        }
        assert!(w.units[id].alive);
        w.step();
        assert!(!w.units[id].alive, "CUnit+0x110 lifetime reached zero");
    }

    #[test]
    fn plague_deals_weapon_60_damage_and_stops_before_killing() {
        let mut w = open_world();
        let marine = w.spawn(1, uid::MARINE, 320, 320).unwrap();
        w.units[marine].hp = 1500;
        w.units[marine].status.plague_ticks = 10;
        for _ in 0..8 {
            w.step();
        }
        // 300 * 256 / 0x4c = 1010 in 1/256 points.
        assert_eq!(w.units[marine].hp, 1500 - 1010);
        for _ in 0..8 {
            w.step();
        }
        assert_eq!(
            w.units[marine].hp, 490,
            "the next 1010-point tick would reach 0, so it is skipped"
        );
        assert_eq!(w.units[marine].status.plague_ticks, 8);
    }

    #[test]
    fn spell_handlers_set_the_recovered_status_durations() {
        let mut w = open_world();
        let caster = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let target = w.spawn(1, uid::MARINE, 360, 320).unwrap();
        let point = (w.units[target].x / fp(1), w.units[target].y / fp(1));
        let id = w.units[target].id;
        let idx = w.index_of(id).unwrap();
        for (spell, field) in [
            (Spell::Ensnare, 0),
            (Spell::Plague, 1),
            (Spell::StasisField, 2),
            (Spell::Maelstrom, 3),
            (Spell::Irradiate, 4),
        ] {
            let target = match spell {
                Spell::Irradiate => SpellTarget::Unit(id),
                _ => SpellTarget::Point(point.0, point.1),
            };
            w.apply_spell(caster, spell, target);
            if spell == Spell::Irradiate {
                // It lands with its bullet, a few frames after the cast.
                assert_eq!(w.units[idx].status.irradiate_ticks, 0);
                for _ in 0..10 {
                    w.step();
                    if w.units[idx].status.irradiate_ticks > 0 {
                        break;
                    }
                }
            }
            let s = &w.units[idx].status;
            let value = match field {
                0 => s.ensnare_ticks,
                1 => s.plague_ticks,
                2 => s.stasis_ticks,
                3 => s.maelstrom_ticks,
                _ => s.irradiate_ticks,
            };
            let expected = match field {
                0 => ENSNARE_UPDATES,
                1 => PLAGUE_UPDATES,
                2 => STASIS_UPDATES,
                3 => MAELSTROM_UPDATES,
                _ => IRRADIATE_UPDATES,
            };
            assert_eq!(value, expected, "{spell:?}");
        }

        let vulture = w.spawn(1, uid::VULTURE, 400, 320).unwrap();
        let vulture_id = w.units[vulture].id;
        w.apply_spell(caster, Spell::Lockdown, SpellTarget::Unit(vulture_id));
        // The Lockdown missile has to reach the Vulture first.
        assert_eq!(w.units[vulture].status.lockdown_ticks, 0);
        let mut landed = None;
        for _ in 0..60 {
            w.step();
            if w.units[vulture].status.lockdown_ticks > 0 {
                landed = Some(w.units[vulture].status.lockdown_ticks);
                break;
            }
        }
        assert_eq!(landed, Some(LOCKDOWN_UPDATES));
    }

    #[test]
    fn fog_of_war_hides_distant_units_from_targeting() {
        let mut w = open_world();
        w.enable_fog();
        let a = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let b = w.spawn(1, uid::MARINE, 600, 600).unwrap();
        w.step();
        assert!(w.is_visible_to(0, w.units[a].x / 256, w.units[a].y / 256));
        assert!(!w.is_visible_to(0, w.units[b].x / 256, w.units[b].y / 256));
        assert!(w.acquire_target(a).is_none());
        let _ = b;
    }

    #[test]
    fn spider_mines_arm_and_detonate() {
        let mut w = open_world();
        let vulture = w.spawn(0, uid::VULTURE, 10, 10).unwrap();
        w.players[0].techs[tech_id::SPIDER_MINES as usize] = true;
        w.units[vulture].energy = 200;
        w.cast(
            w.units[vulture].id,
            Spell::SpiderMines,
            SpellTarget::Point(14, 10),
        )
        .unwrap();
        assert!(
            w.units
                .iter()
                .filter(|u| u.type_id == uid::SPIDER_MINE)
                .count()
                == 1,
            "one mine per order (the Vulture carries three)"
        );
        // Let the mines arm, then walk an enemy onto them.
        for _ in 0..SPIDER_MINE_ARM_TICKS as usize + 2 {
            w.step();
        }
        let enemy = w.spawn(1, uid::MARINE, 14, 10).unwrap();
        let start = w.units[enemy].hp;
        for _ in 0..20 {
            w.step();
        }
        assert!(w.units[enemy].hp < start, "spider mine should detonate");
    }

    #[test]
    fn scv_repairs_a_mechanical_unit() {
        let mut w = open_world();
        let scv = w.spawn(0, uid::SCV, 320, 320).unwrap();
        let tank = w.spawn(0, uid::SIEGE_TANK, 340, 320).unwrap();
        w.units[tank].hp = fp(50);
        w.players[0].minerals = 100;
        w.issue(
            w.units[scv].id,
            Order::Repair {
                target: w.units[tank].id,
            },
        )
        .unwrap();
        for _ in 0..30 {
            w.step();
        }
        assert!(w.units[tank].hp > fp(50));
    }

    #[test]
    fn medics_automatically_heal_nearby_organic_units() {
        let mut w = open_world();
        let medic = w.spawn(0, uid::MEDIC, 320, 320).unwrap();
        let marine = w.spawn(0, uid::MARINE, 340, 320).unwrap();
        w.units[marine].hp = fp(10);
        w.units[medic].energy = 200;
        for _ in 0..20 {
            w.step();
        }
        assert!(w.units[marine].hp > fp(10));
        let _ = medic;
    }

    #[test]
    fn point_target_spells_hit_the_ground_location() {
        let mut w = open_world();
        let templar = w.spawn(0, uid::HIGH_TEMPLAR, 320, 320).unwrap();
        w.players[0].techs[tech_id::PSIONIC_STORM as usize] = true;
        w.units[templar].energy = 200;
        let a = w.spawn(1, uid::DROPSHIP, 480, 320).unwrap();
        let b = w.spawn(1, uid::DROPSHIP, 528, 320).unwrap();
        // Psionic Storm reaches 48 px from the victim's box (OpenBW
        // `bullet_deal_splash_damage`: `unit_distance_to`).  Dropship box:
        // 24 px up from its centre, so 73 px below the storm is just out.
        let c = w.spawn(1, uid::DROPSHIP, 480, 320 + 48 + 25).unwrap();
        for i in [a, b, c] {
            w.units[i].hp = fp(500);
        }
        let (ha, hb, hc) = (w.units[a].hp, w.units[b].hp, w.units[c].hp);
        w.cast(
            w.units[templar].id,
            Spell::PsionicStorm,
            SpellTarget::Point(480, 320),
        )
        .unwrap();
        assert_eq!(w.units[a].hp, ha, "the cast tick does not deal damage");
        let bullet = w.projectiles.last().expect("persistent storm field");
        assert_eq!(bullet.weapon, wid::PSIONIC_STORM);
        assert_eq!(bullet.state, BulletState::Persist);
        for _ in 0..60 {
            w.step();
        }
        assert_eq!(w.units[a].hp, ha - fp(14) * 8);
        assert_eq!(w.units[b].hp, hb - fp(14) * 8, "48 px is inside");
        assert_eq!(w.units[c].hp, hc, "49 px from its box is outside");
        assert!(w.projectiles.is_empty());
    }

    #[test]
    fn order_queue_uses_the_recovered_queueable_metadata() {
        let mut w = open_world();
        let idx = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let id = w.units[idx].id;
        w.issue(id, Order::Move { pos: (400, 400) }).unwrap();
        for i in 0..MAX_ORDER_QUEUE {
            w.queue_order(
                id,
                Order::Move {
                    pos: (400 + i as i32, 400),
                },
            )
            .unwrap();
        }
        assert_eq!(
            w.units[w.index_of(id).unwrap()].queue.len(),
            MAX_ORDER_QUEUE
        );
        assert_eq!(
            w.queue_order(id, Order::Move { pos: (1, 1) }),
            Err(SimError::QueueFull)
        );
        // Build is not queueable in the shipped table; it replaces the
        // current order and clears the queue.
        w.queue_order(
            id,
            Order::Build {
                unit_type: uid::BARRACKS,
                pos: (400, 400),
            },
        )
        .unwrap();
        let u = &w.units[w.index_of(id).unwrap()];
        assert!(matches!(u.order, Order::Build { .. }));
        assert!(u.queue.is_empty());
    }

    #[test]
    fn order_done_uses_the_batch_06_queue_pop_and_refreshes_the_row() {
        let mut w = open_world();
        let idx = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let id = w.units[idx].id;
        w.issue(id, Order::Move { pos: (400, 400) }).unwrap();
        w.queue_order(id, Order::Move { pos: (500, 500) }).unwrap();
        w.order_done(idx);
        let queued = Order::Move { pos: (500, 500) };
        let u = &w.units[idx];
        assert_eq!(u.order, queued);
        assert_eq!(u.order_row, queued.order_id());
        assert!(u.queue.is_empty());
    }

    #[test]
    fn status_modifier_count_drives_the_speed_clamp() {
        let mut w = open_world();
        let idx = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        assert_eq!(w.units[idx].a04_status_modifier_count(), 0);
        w.units[idx].set_flag_29();
        let mods = w.units[idx].a04_status_modifier_count();
        assert_eq!(mods, 1);
        assert_eq!(
            World::modified_speed_for_mods(uid::MARINE, 1024, mods),
            1536
        );
    }

    #[test]
    fn queue_order_uses_can_be_queued_not_highlight() {
        let mut w = open_world();
        let idx = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let id = w.units[idx].id;
        // Row 115 Burrow has highlight 240 but `can_be_queued = false`; it
        // must replace the current order and clear the queue.
        w.issue(id, Order::Move { pos: (400, 400) }).unwrap();
        assert!(w.data.order(115).is_some_and(|r| !r.can_be_queued));
        assert!(w.data.order(115).is_some_and(|r| r.highlight != -1));
        w.queue_order(id, Order::Burrow).unwrap();
        let u = &w.units[w.index_of(id).unwrap()];
        assert!(matches!(u.order, Order::Burrow));
        assert!(u.queue.is_empty());
        // Row 21 Turret Attack is the opposite: queueable but highlight -1.
        assert!(
            w.data
                .order(21)
                .is_some_and(|r| r.can_be_queued && r.highlight == -1)
        );
    }

    #[test]
    fn carrier_attack_waits_for_the_60_tick_launch_phase() {
        let mut w = open_world();
        let carrier = w.spawn(0, uid::CARRIER, 320, 320).unwrap();
        w.players[0].minerals = 1000;
        w.build_hangar_unit(w.units[carrier].id).unwrap();
        let enemy = w.spawn(1, uid::MARINE, 480, 320).unwrap();
        let start = w.units[enemy].hp;
        w.issue_order_id(w.units[carrier].id, 20, Some(w.units[enemy].id), None)
            .unwrap();
        for _ in 0..59 {
            w.step();
        }
        assert_eq!(w.units[enemy].hp, start, "launch phase is still running");
        for _ in 0..15 {
            w.step();
        }
        assert!(
            w.units[enemy].hp < start,
            "the interceptor launches after CUnit+0x55 reaches zero"
        );
    }

    #[test]
    fn reaver_scarab_damages_and_consumes_one_hangar() {
        let mut w = open_world();
        let reaver = w.spawn(0, uid::REAVER, 320, 320).unwrap();
        w.units[reaver].hangar = 1;
        let enemy = w.spawn(1, uid::BARRACKS, 480, 320).unwrap();
        w.units[enemy].completed = true;
        let start = w.units[enemy].hp;
        w.issue(
            w.units[reaver].id,
            Order::AttackUnit {
                target: w.units[enemy].id,
            },
        )
        .unwrap();
        for _ in 0..300 {
            w.step();
            if w.units[enemy].hp < start {
                break;
            }
        }
        assert!(
            w.units[enemy].hp < start,
            "the scarab projectile must damage the target (projectiles={}, reaver hangar={})",
            w.projectiles.len(),
            w.units[reaver].hangar
        );
        assert_eq!(w.units[reaver].hangar, 0);
    }

    #[test]
    fn carrier_launches_interceptors_from_its_hangar() {
        let mut w = open_world();
        let carrier = w.spawn(0, uid::CARRIER, 320, 320).unwrap();
        w.players[0].minerals = 1000;
        w.build_hangar_unit(w.units[carrier].id).unwrap();
        assert_eq!(w.units[carrier].hangar, 1);
        let enemy = w.spawn(1, uid::MARINE, 480, 320).unwrap();
        let start = w.units[enemy].hp;
        for _ in 0..120 {
            w.step();
            if w.units[enemy].hp < start {
                break;
            }
        }
        assert!(
            w.units[enemy].hp < start,
            "interceptor should damage the target"
        );
    }

    #[test]
    fn nuclear_strike_uses_the_nuclear_damage_floor() {
        let mut w = open_world();
        let ghost = w.spawn(0, uid::GHOST, 320, 320).unwrap();
        let silo = w.spawn(0, uid::NUCLEAR_SILO, 128, 128).unwrap();
        w.units[silo].addon_attached = true;
        let target = w.spawn(1, uid::MARINE, 700, 320).unwrap();
        w.players[0].minerals = 1000;
        w.players[0].gas = 1000;
        w.arm_nuke(w.units[silo].id).unwrap();
        let start = w.units[target].hp;
        w.launch_nuke(w.units[ghost].id, (700, 320)).unwrap();
        // Walk into range, 90 frames of countdown, 250 off the map, the
        // fall.
        for _ in 0..700 {
            w.step();
            if w.units[target].hp < start {
                break;
            }
        }
        assert!(
            w.units[target].hp < start,
            "nuke should hit the target area"
        );
    }

    #[test]
    fn nuke_waits_90_ticks_warns_at_45_and_consumes_the_silo() {
        let mut w = open_world();
        // Within the Ghost's sight (9 tiles) of the target point.
        let ghost = w.spawn(0, uid::GHOST, 450, 320).unwrap();
        let silo = w.spawn(0, uid::NUCLEAR_SILO, 128, 128).unwrap();
        w.units[silo].addon_attached = true;
        let target = w.spawn(1, uid::MARINE, 700, 320).unwrap();
        w.players[0].minerals = 1000;
        w.players[0].gas = 1000;
        w.arm_nuke(w.units[silo].id).unwrap();
        assert_eq!((w.players[0].minerals, w.players[0].gas), (800, 800));
        assert!(w.units[silo].nuke_ready);
        let hp = w.units[target].hp;
        w.launch_nuke(w.units[ghost].id, (700, 320)).unwrap();
        assert!(!w.units[silo].nuke_ready);
        assert_eq!(w.nuke_strikes.len(), 1);
        assert!(w.nuke_strikes[0].dot);
        assert!(w.projectiles.is_empty());
        for _ in 0..44 {
            w.step();
        }
        assert!(w.nuke_warnings.is_empty(), "warning is at 45 ticks left");
        w.step();
        assert_eq!(w.nuke_warnings, vec![(700, 320)]);
        for _ in 0..45 {
            w.step();
        }
        assert!(
            w.nuke_strikes.iter().all(|s| s.timer <= 0),
            "countdown over; the strike stays until the bomb lands"
        );
        // The missile is off the map for its WarpIn animation (250
        // frames) before the bomb appears above the target.
        assert!(w.projectiles.is_empty(), "nothing falls yet");
        for _ in 0..NUKE_WARP_FRAMES {
            w.step();
        }
        assert_eq!(w.projectiles.len(), 1, "the bomb is falling");
        assert_eq!(w.units[target].hp, hp, "impact is still travelling");
    }

    #[test]
    fn nuke_aborts_on_ghost_death_and_on_a_new_order() {
        let mut w = open_world();
        let ghost = w.spawn(0, uid::GHOST, 320, 320).unwrap();
        let silo = w.spawn(0, uid::NUCLEAR_SILO, 128, 128).unwrap();
        w.units[silo].addon_attached = true;
        let target = w.spawn(1, uid::MARINE, 700, 320).unwrap();
        w.players[0].minerals = 2000;
        w.players[0].gas = 2000;
        let ghost_id = w.units[ghost].id;
        w.arm_nuke(w.units[silo].id).unwrap();
        w.launch_nuke(ghost_id, (700, 320)).unwrap();
        for _ in 0..10 {
            w.step();
        }
        w.damage_unit(ghost_id, 1_000_000).unwrap();
        assert!(w.nuke_strikes.is_empty(), "ghost death cancels the strike");
        let hp = w.units[target].hp;
        for _ in 0..200 {
            w.step();
        }
        assert_eq!(w.units[target].hp, hp, "canceled strike deals no damage");

        let ghost2 = w.spawn(0, uid::GHOST, 320, 320).unwrap();
        let ghost2_id = w.units[ghost2].id;
        w.arm_nuke(w.units[silo].id).unwrap();
        w.launch_nuke(ghost2_id, (700, 320)).unwrap();
        assert_eq!(w.nuke_strikes.len(), 1);
        w.issue(ghost2_id, Order::Move { pos: (400, 400) }).unwrap();
        assert!(w.nuke_strikes.is_empty(), "a new order cancels the strike");
    }

    #[test]
    fn spawn_larva_adds_one_and_stops_at_three() {
        let mut w = open_world();
        let hatch = w.spawn(0, uid::HATCHERY, 320, 320).unwrap();
        let id = w.units[hatch].id;
        let larvae = |w: &World| {
            w.units
                .iter()
                .filter(|u| u.alive && u.owner == 0 && u.type_id == uid::LARVA)
                .count()
        };
        w.spawn_larva(id).unwrap();
        assert_eq!(larvae(&w), 1);
        w.spawn_larva(id).unwrap();
        w.spawn_larva(id).unwrap();
        assert_eq!(larvae(&w), 3);
        assert_eq!(w.spawn_larva(id), Err(SimError::QueueFull));
    }

    #[test]
    fn larva_spawn_uses_the_lowest_side_score() {
        let mut w = open_world();
        let hatch = w.spawn(0, uid::HATCHERY, 320, 320).unwrap();
        let id = w.units[hatch].id;
        w.mature_creep();
        let (hx, hy) = (w.units[hatch].x / fp(1), w.units[hatch].y / fp(1));
        let (left, _, _, bottom) = w.data.unit(uid::HATCHERY).dimensions.rect_at((hx, hy));

        w.units[hatch].larva_spawn_side_values = [40, 40, 40, 7];
        w.spawn_larva(id).unwrap();
        let larva = w
            .units
            .iter()
            .find(|u| u.type_id == uid::LARVA)
            .expect("first larva");
        assert_eq!((larva.x / fp(1), larva.y / fp(1)), (hx, bottom + 10));

        w.units[hatch].larva_spawn_side_values = [1, 40, 40, 40];
        w.spawn_larva(id).unwrap();
        let larva = w
            .units
            .iter()
            .rfind(|u| u.type_id == uid::LARVA)
            .expect("second larva");
        assert_eq!((larva.x / fp(1), larva.y / fp(1)), (left - 10, hy));
    }

    #[test]
    fn killing_a_unit_clears_every_reference_to_it() {
        let mut w = open_world();
        let attacker = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let victim = w.spawn(1, uid::MARINE, 360, 320).unwrap();
        let (a_id, v_id) = (w.units[attacker].id, w.units[victim].id);
        w.issue(a_id, Order::AttackUnit { target: v_id }).unwrap();
        w.step();
        assert_eq!(w.units[attacker].target, Some(v_id));
        w.damage_unit(v_id, 1_000_000).unwrap();
        assert_eq!(w.units[attacker].target, None);
    }

    #[test]
    fn larva_cap_is_per_connected_hatchery() {
        let mut w = open_world();
        let a = w.spawn(0, uid::HATCHERY, 320, 320).unwrap();
        let b = w.spawn(0, uid::HATCHERY, 400, 320).unwrap();
        let (aid, bid) = (w.units[a].id, w.units[b].id);
        for k in 0..3 {
            let l = w.spawn(0, uid::LARVA, 320 + k, 320).unwrap();
            w.units[l].connected_unit = Some(aid);
        }
        assert_eq!(w.spawn_larva(aid), Err(SimError::QueueFull));
        w.spawn_larva(bid).unwrap();
        assert_eq!(
            w.units
                .iter()
                .filter(|u| u.alive && u.type_id == uid::LARVA && u.connected_unit == Some(bid))
                .count(),
            1
        );
    }

    #[test]
    fn hatchery_larva_interval_is_37_status_updates() {
        let mut w = open_world();
        let hatch = w.spawn(0, uid::HATCHERY, 320, 320).unwrap();
        w.units[hatch].completed = true;
        w.units[hatch].larva_timer = LARVA_SPAWN_TICKS;
        let larvae = |w: &World| {
            w.units
                .iter()
                .filter(|u| u.alive && u.owner == 0 && u.type_id == uid::LARVA)
                .count()
        };
        for _ in 0..LARVA_SPAWN_TICKS - 1 {
            w.step();
        }
        assert_eq!(larvae(&w), 0);
        w.step();
        assert_eq!(larvae(&w), 1);
        assert_eq!(w.units[hatch].larva_timer, LARVA_SPAWN_TICKS);
    }

    #[test]
    fn cancel_morph_returns_an_egg_to_its_larva() {
        let mut w = open_world();
        let hatch = w.spawn(0, uid::HATCHERY, 320, 320).unwrap();
        w.units[hatch].completed = true;
        let larva = w.spawn(0, uid::LARVA, 320, 320).unwrap();
        let id = w.units[larva].id;
        w.spawn(0, uid::SPAWNING_POOL, 512, 512).unwrap();
        w.players[0].minerals = 1000;
        w.morph(id, uid::ZERGLING).unwrap();
        assert_eq!(w.units[larva].type_id, uid::EGG);
        w.cancel_morph(id).unwrap();
        assert_eq!(w.units[larva].type_id, uid::LARVA);
        assert!(w.units[larva].alive);
    }

    #[test]
    fn cancel_addon_refunds_and_unlinks_the_addon() {
        let mut w = open_world();
        let cc = w.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
        w.units[cc].completed = true;
        w.spawn(0, uid::ACADEMY, 300, 320).unwrap();
        w.players[0].minerals = 1000;
        w.players[0].gas = 1000;
        w.build_addon(w.units[cc].id, uid::COMSAT_STATION).unwrap();
        let before = w.players[0].minerals;
        w.cancel_addon(w.units[cc].id).unwrap();
        assert!(w.units[cc].addon.is_none());
        assert!(w.players[0].minerals > before, "75 % refund");
    }

    #[test]
    fn addons_require_academy_and_covert_ops() {
        let mut w = open_world();
        w.players[0].minerals = 2000;
        w.players[0].gas = 2000;
        let cc = w.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
        let cc_id = w.units[cc].id;
        let before = w.players[0].minerals;
        assert_eq!(
            w.build_addon(cc_id, uid::COMSAT_STATION),
            Err(SimError::MissingRequirements)
        );
        assert_eq!(w.players[0].minerals, before);
        w.spawn(0, uid::ACADEMY, 300, 320).unwrap();
        w.build_addon(cc_id, uid::COMSAT_STATION).unwrap();
    }

    #[test]
    fn cancel_research_and_upgrade_refunds_the_full_price() {
        let mut w = open_world();
        let academy = w.spawn(0, uid::ACADEMY, 8, 8).unwrap();
        let id = w.units[academy].id;
        w.players[0].minerals = 1000;
        w.players[0].gas = 1000;

        let tech = tech_id::STIM_PACKS;
        let t = w.data.tech(tech).unwrap().clone();
        let before = (w.players[0].minerals, w.players[0].gas);
        w.research(id, tech).unwrap();
        w.step();
        assert_eq!(
            (w.players[0].minerals, w.players[0].gas),
            (
                before.0 - t.mineral_cost as i32,
                before.1 - t.gas_cost as i32
            )
        );
        w.cancel_production(id).unwrap();
        assert_eq!((w.players[0].minerals, w.players[0].gas), before);
        assert_eq!(w.units[academy].build_remaining, 0);
        assert_eq!(w.units[academy].build_total, 0);
        assert!(!w.players[0].has_tech(tech));

        let up_id = upgrade::U238_SHELLS;
        let up = w.data.upgrade(up_id).unwrap().clone();
        let before = (w.players[0].minerals, w.players[0].gas);
        w.upgrade(id, up_id).unwrap();
        w.step();
        assert_eq!(
            (w.players[0].minerals, w.players[0].gas),
            (
                before.0 - up.mineral_cost(0) as i32,
                before.1 - up.gas_cost(0) as i32
            )
        );
        w.cancel_production(id).unwrap();
        assert_eq!((w.players[0].minerals, w.players[0].gas), before);
        assert_eq!(w.units[academy].build_remaining, 0);
        assert_eq!(w.units[academy].build_total, 0);
        assert_eq!(w.players[0].upgrade_level(up_id), 0);
    }

    #[test]
    fn capture_the_flag_pickup_and_capture_eliminate_the_owner() {
        let mut w = open_world();
        let cc0 = w.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
        let cc1 = w.spawn(1, uid::COMMAND_CENTER, 1200, 320).unwrap();
        w.units[cc0].completed = true;
        w.units[cc1].completed = true;
        w.start_ctf();
        assert!(w.ctf.is_some());
        // Force the flags out without simulating three game minutes.
        {
            let c = w.ctf.as_mut().unwrap();
            c.release_at = 0;
            for f in &mut c.flags {
                f.released = true;
            }
        }
        let enemy_flag = w.ctf.as_ref().unwrap().flags[1].pos;
        let worker = w.spawn_px(0, uid::SCV, enemy_flag.0, enemy_flag.1).unwrap();
        w.update_ctf();
        assert_eq!(
            w.ctf.as_ref().unwrap().flags[1].carrier,
            Some(w.units[worker].id)
        );
        // Carry it to our own beacon: the enemy is eliminated.
        let home = w.ctf.as_ref().unwrap().flags[0].home;
        w.units[worker].x = home.0 * fp(1);
        w.units[worker].y = home.1 * fp(1);
        w.update_ctf();
        assert!(w.players[1].defeated);
    }

    #[test]
    fn a_beacon_can_move_once_before_the_deadline() {
        let mut w = open_world();
        let cc = w.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
        w.units[cc].completed = true;
        w.start_ctf();
        w.move_beacon(0, (700, 700)).unwrap();
        assert_eq!(w.ctf.as_ref().unwrap().flags[0].home, (700, 700));
        assert_eq!(w.move_beacon(0, (800, 800)), Err(SimError::Unsupported));
    }

    #[test]
    fn cheat_codes_apply_and_are_recorded() {
        let mut w = open_world();
        w.recorder = Some(crate::replay::Recorder::new());
        let before = w.players[0].minerals;
        w.apply_cheat(0, cheat::SHOW_ME_THE_MONEY).unwrap();
        assert_eq!(w.players[0].minerals, before + 10_000);
        assert!(matches!(
            w.recorder.as_ref().unwrap().cmds.last().map(|c| &c.act),
            Some(crate::replay::RecAct::Cheat { .. })
        ));
        w.apply_cheat(0, cheat::POWER_OVERWHELMING).unwrap();
        let marine = w.spawn(0, uid::MARINE, 100, 100).unwrap();
        let hp = w.units[marine].hp;
        w.damage_hp(marine, 1000, 0);
        assert_eq!(w.units[marine].hp, hp);
        w.apply_cheat(0, cheat::MEDIEVAL_MAN).unwrap();
        assert!(w.players[0].has_tech(tech_id::STIM_PACKS));
        assert!(w.players[0].cheats.free_upgrades);
        w.apply_cheat(0, cheat::FOOD_FOR_THOUGHT).unwrap();
        assert_eq!(w.supply_cap(0), 400);
        w.apply_cheat(0, cheat::THERE_IS_NO_COW_LEVEL).unwrap();
        assert_eq!(w.winner, Some(0));
    }

    #[test]
    fn sudden_death_ends_the_owner_of_a_destroyed_town_hall() {
        let mut w = open_world();
        let cc = w.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
        let nexus = w.spawn(1, uid::NEXUS, 800, 800).unwrap();
        w.units[cc].completed = true;
        w.units[nexus].completed = true;
        w.sudden_death = true;
        let id = w.units[cc].id;
        w.units[cc].hp = 0;
        w.kill_unit(w.index_of(id).unwrap());
        w.check_victory();
        assert!(w.players[0].defeated);
        assert_eq!(w.winner, Some(1));
    }

    #[test]
    fn greed_wins_at_the_gathered_resource_goal() {
        let mut w = open_world();
        w.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
        w.spawn(1, uid::NEXUS, 800, 800).unwrap();
        w.greed_goal = Some(1000);
        w.players[0].minerals_gathered = 1000;
        w.players[0].gas_gathered = 999;
        w.check_victory();
        assert_eq!(w.winner, None, "both resources are required");
        w.players[0].gas_gathered = 1000;
        w.check_victory();
        assert_eq!(w.winner, Some(0));
    }

    #[test]
    fn slaughter_wins_by_kills_at_the_deadline() {
        let mut w = open_world();
        w.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
        w.spawn(1, uid::NEXUS, 800, 800).unwrap();
        w.players[0].kills = 3;
        w.players[1].kills = 7;
        w.players[1].razings = 2;
        w.slaughter_deadline = Some(10);
        w.tick = 10;
        w.check_victory();
        assert_eq!(w.winner, Some(1));
    }

    #[test]
    fn direct_abilities_enter_the_command_log() {
        let mut w = open_world();
        w.recorder = Some(crate::replay::Recorder::new());
        let hatch = w.spawn(0, uid::HATCHERY, 320, 320).unwrap();
        let id = w.units[hatch].id;
        w.spawn_larva(id).unwrap();
        let cloak = w.spawn(0, uid::GHOST, 380, 380).unwrap();
        let cid = w.units[cloak].id;
        w.players[0].techs[crate::ids::tech::PERSONNEL_CLOAKING as usize] = true;
        w.set_cloak(cid, true).unwrap();
        let acts: Vec<_> = w
            .recorder
            .as_ref()
            .unwrap()
            .cmds
            .iter()
            .map(|c| c.act.clone())
            .collect();
        assert!(matches!(
            acts.first(),
            Some(crate::replay::RecAct::SpawnLarva { .. })
        ));
        assert!(matches!(
            acts.last(),
            Some(crate::replay::RecAct::Cloak { cloaked: true, .. })
        ));
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn scenario_rules_override_unit_tech_upgrade_and_availability() {
        use crate::map::{TechSettings, UnitAvailability, UnitSettings, UpgradeSettings};
        let mut w = open_world();

        let mut units = UnitSettings::default();
        units.use_default = vec![1; 228];
        units.use_default[0] = 0;
        units.hit_points = vec![0; 228];
        units.hit_points[0] = 5120; // 20 HP
        units.shields = vec![0; 228];
        units.armor = vec![0; 228];
        units.armor[0] = 2;
        units.build_time = vec![0; 228];
        units.build_time[0] = 300;
        units.mineral_cost = vec![0; 228];
        units.mineral_cost[0] = 25;
        units.gas_cost = vec![0; 228];
        units.string_id = vec![0; 228];
        units.weapon_damage = vec![0; 130];
        units.weapon_bonus = vec![0; 130];
        w.rules.unit_settings = Some(units);

        let mut avail = UnitAvailability::default();
        avail.player = vec![1; 12 * 228];
        avail.use_global = vec![0; 12 * 228];
        avail.global = vec![1; 228];
        avail.player[37] = 0; // player 0 cannot train Zergling
        w.rules.unit_availability = Some(avail);

        let mut techs = TechSettings::default();
        techs.use_default = vec![1; 44];
        techs.use_default[0] = 0;
        techs.mineral_cost = vec![0; 44];
        techs.mineral_cost[0] = 150;
        techs.gas_cost = vec![0; 44];
        techs.research_time = vec![0; 44];
        techs.research_time[0] = 100;
        techs.energy_cost = vec![0; 44];
        w.rules.tech_settings = Some(techs);

        let mut upgrades = UpgradeSettings::default();
        upgrades.use_default = vec![1; 61];
        upgrades.use_default[7] = 0;
        upgrades.mineral_base = vec![0; 61];
        upgrades.mineral_base[7] = 100;
        upgrades.mineral_factor = vec![0; 61];
        upgrades.mineral_factor[7] = 10;
        upgrades.gas_base = vec![0; 61];
        upgrades.gas_factor = vec![0; 61];
        upgrades.time_base = vec![0; 61];
        upgrades.time_base[7] = 4000;
        upgrades.time_factor = vec![0; 61];
        w.rules.upgrade_settings = Some(upgrades);

        let marine = w.spawn(0, uid::MARINE, 10, 10).unwrap();
        assert_eq!(w.units[marine].hp, 5120);
        assert_eq!(w.units[marine].shields, 0);
        assert_eq!(w.unit_stats(uid::MARINE).armor, 2);
        assert_eq!(w.unit_stats(uid::MARINE).mineral_cost, 25);
        assert!(!w.unit_available(0, uid::ZERGLING));
        assert!(w.unit_available(1, uid::ZERGLING));
        assert_eq!(w.tech_stats(0).unwrap().mineral_cost, 150);
        assert_eq!(w.tech_stats(0).unwrap().research_time, 100);
        assert_eq!(w.upgrade_stats(7).unwrap().mineral_cost(0), 100);
        assert_eq!(w.upgrade_stats(7).unwrap().time_cost(0), 4000);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn property_slot_applies_retail_limits_and_ignores_owner() {
        use crate::map::UnitProperties;
        let mut w = open_world();
        let props = UnitProperties {
            state_flags: 0x14,
            field_flags: 0x3f,
            owner: 1,
            hit_points_percent: 50,
            shields_percent: 25,
            energy_percent: 10,
            resource_amount: 0x1_2345,
            hangar: 2,
            states: 0x14,
        };
        let i = w
            .spawn_with_properties(0, uid::HIGH_TEMPLAR, 10, 10, &props)
            .unwrap();
        let ty = w.data.unit(uid::HIGH_TEMPLAR).clone();
        assert_eq!(w.units[i].hp, (ty.hit_points_raw as i32 * 50 / 100).max(1));
        assert_eq!(w.units[i].shields, fp(ty.shield_points as i32 * 25 / 100));
        assert_eq!(
            w.units[i].energy_fixed(),
            w.max_energy_fixed(uid::HIGH_TEMPLAR, 0) * 10 / 100
        );
        assert_eq!(w.units[i].owner, 0, "owner byte is unused");
        assert_eq!(w.units[i].resource_amount, 0);
        assert!(w.units[i].invincible);

        let mut mineral = UnitProperties::default();
        mineral.field_flags = 0x10;
        mineral.resource_amount = 1234;
        let m = w
            .spawn_with_properties(0, uid::MINERAL_FIELD, 40, 40, &mineral)
            .unwrap();
        assert_eq!(w.units[m].resource_amount, 1234);

        let mut hangar = UnitProperties::default();
        hangar.field_flags = 0x20;
        hangar.hangar = 8;
        let c = w
            .spawn_with_properties(0, uid::CARRIER, 60, 60, &hangar)
            .unwrap();
        assert_eq!(w.units[c].hangar, 4, "clamped to the Carrier's capacity");

        let mut lift = UnitProperties::default();
        lift.state_flags = 0x04;
        lift.states = 0x04;
        let cc = w
            .spawn_with_properties(0, uid::COMMAND_CENTER, 80, 80, &lift)
            .unwrap();
        assert!(w.units[cc].lifted);
    }

    #[test]
    fn burrow_and_unburrow_toggle_the_hidden_state() {
        let mut w = open_world();
        let ling = w.spawn(0, uid::ZERGLING, 320, 320).unwrap();
        w.players[0].techs[tech_id::BURROWING as usize] = true;
        let id = w.units[ling].id;
        w.issue(id, Order::Burrow).unwrap();
        w.step();
        assert!(w.units[ling].status.burrowed);
        w.issue(id, Order::Unburrow).unwrap();
        for _ in 0..10 {
            w.step();
        }
        assert!(!w.units[ling].status.burrowed);
    }

    #[test]
    fn exposed_predicate_rejects_hidden_and_burrowed_units() {
        let mut w = open_world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        assert!(w.units[marine].is_exposed());
        w.units[marine].status.burrowed = true;
        assert!(!w.units[marine].is_exposed());
        w.units[marine].status.burrowed = false;
        w.units[marine].in_transport = Some(1);
        assert!(!w.units[marine].is_exposed());
    }

    #[test]
    fn only_production_buildings_accept_a_rally_point() {
        let mut w = open_world();
        let pylon = w.spawn(0, uid::PYLON, 320, 320).unwrap();
        assert_eq!(
            w.set_rally(w.units[pylon].id, (400, 400)),
            Err(SimError::MissingRequirements)
        );
        let barracks = w.spawn(0, uid::BARRACKS, 420, 320).unwrap();
        w.set_rally(w.units[barracks].id, (400, 400)).unwrap();
        assert_eq!(w.units[barracks].rally, Some((400, 400)));
    }

    #[test]
    fn worker_flag_matches_the_recovered_predicate() {
        let w = open_world();
        for (id, expected) in [
            (uid::SCV, true),
            (uid::DRONE, true),
            (uid::PROBE, true),
            (uid::MARINE, false),
            (uid::ZERGLING, false),
        ] {
            assert_eq!(w.data.unit(id).is_worker(), expected, "unit {id}");
        }
    }

    #[test]
    fn arbiter_predicate_matches_the_recovered_rows() {
        assert!(is_arbiter_type(uid::ARBITER));
        assert!(is_arbiter_type(0x56));
        assert!(!is_arbiter_type(uid::CARRIER));
    }

    #[test]
    fn cloak_state_matches_the_recovered_bits() {
        let mut w = open_world();
        let ghost = w.spawn(0, uid::GHOST, 320, 320).unwrap();
        assert_eq!(w.units[ghost].cloak_state(), 0);
        w.units[ghost].status.cloaked = true;
        assert_eq!(w.units[ghost].cloak_state(), 2);
    }

    #[test]
    fn unit_box_rect_uses_each_asymmetric_extent() {
        let w = open_world();
        let ty = w.data.unit(uid::BARRACKS);
        let rect = unit_box_rect(ty, (100, 100));
        assert_eq!(rect.left, 100 - ty.dimensions.left as i32);
        assert_eq!(rect.top, 100 - ty.dimensions.top as i32);
        assert_eq!(rect.right, 100 + ty.dimensions.right as i32);
        assert_eq!(rect.bottom, 100 + ty.dimensions.bottom as i32);
    }

    #[test]
    fn unit_box_width_matches_left_plus_right_plus_one() {
        let w = open_world();
        for id in [uid::MARINE, uid::BARRACKS, uid::COMMAND_CENTER] {
            let ty = w.data.unit(id);
            assert_eq!(
                unit_box_width(ty),
                ty.dimensions.left as i32 + ty.dimensions.right as i32 + 1,
                "unit {id}"
            );
        }
    }

    #[test]
    fn current_build_unit_reads_the_queue_head() {
        let mut w = open_world();
        let barracks = w.spawn(0, uid::BARRACKS, 320, 320).unwrap();
        w.units[barracks].completed = true;
        let cc = w.spawn(0, uid::COMMAND_CENTER, 240, 320).unwrap();
        w.units[cc].completed = true;
        w.players[0].minerals = 500;
        assert_eq!(w.units[barracks].current_build_unit(), uid::BARRACKS);
        w.train(w.units[barracks].id, uid::MARINE).unwrap();
        assert_eq!(w.units[barracks].current_build_unit(), uid::MARINE);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn unit_name_uses_the_scenario_string_override() {
        let mut w = open_world();
        let mut units = crate::map::UnitSettings::default();
        units.use_default = vec![1; 228];
        units.use_default[0] = 0;
        units.string_id = vec![0; 228];
        units.string_id[0] = 5;
        w.rules.unit_settings = Some(units);
        assert_eq!(w.unit_name(uid::MARINE), w.data.strings.text(5));
        assert_eq!(w.unit_name(uid::ZERGLING), w.data.unit_name(uid::ZERGLING));
    }

    #[test]
    fn doodad_range_matches_the_recovered_predicate() {
        for ty in 0xcb..=0xd5 {
            assert!(is_doodad_type(ty), "type {ty:#x}");
        }
        assert!(!is_doodad_type(0xca));
        assert!(!is_doodad_type(0xd6));
    }

    #[test]
    fn organic_predicate_matches_the_recovered_flag() {
        let w = open_world();
        assert!(w.data.unit(uid::MARINE).is_organic());
        assert!(!w.data.unit(uid::VULTURE).is_organic());
        assert!(!w.data.unit(uid::BARRACKS).is_organic());
    }

    #[test]
    fn robotic_and_hero_flags_match_the_recovered_predicates() {
        let w = open_world();
        assert!(w.data.unit(uid::PROBE).is_robotic());
        assert!(!w.data.unit(uid::MARINE).is_robotic());
        assert!(!w.data.unit(uid::MARINE).is_hero());
        assert!(
            w.data.units.iter().any(|u| u.is_hero()),
            "the table has at least one hero"
        );
    }

    #[test]
    fn air_state_matches_the_dynamic_flag() {
        let mut w = open_world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let wraith = w.spawn(0, uid::WRAITH, 360, 320).unwrap();
        let cc = w.spawn(0, uid::COMMAND_CENTER, 400, 320).unwrap();
        assert!(!w.is_air_unit(marine));
        assert!(w.is_air_unit(wraith));
        assert!(!w.is_air_unit(cc));
        w.units[cc].lifted = true;
        assert!(w.is_air_unit(cc), "lifted building is dynamically air");
    }

    #[test]
    fn pylon_predicate_matches_type_0x9c() {
        assert!(is_pylon_type(uid::PYLON));
        assert!(!is_pylon_type(uid::NEXUS));
    }

    #[test]
    fn broodlings_expire_after_1350_frames() {
        assert!(is_broodling_or_dying(uid::BROODLING));
        assert!(!is_broodling_or_dying(uid::ZERGLING));
        let mut w = open_world();
        w.spawn_broodling(0, 320, 320);
        let i = w
            .units
            .iter()
            .position(|u| u.type_id == uid::BROODLING)
            .unwrap();
        assert!(w.units[i].alive);
        for _ in 0..BROODLING_LIFETIME_TICKS {
            w.step();
        }
        assert!(!w.units[i].alive);
    }

    #[test]
    fn floor_trap_predicate_matches_0xcb_and_0xd1() {
        assert!(is_floor_trap_type(0xcb));
        assert!(is_floor_trap_type(0xd1));
        assert!(!is_floor_trap_type(0xcc));
        assert!(!is_floor_trap_type(0xd2));
    }

    #[test]
    fn trap_trigger_predicate_excludes_doors_and_0xcc() {
        for ty in [0xcb, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5] {
            assert!(is_trap_trigger_type(ty), "type {ty:#x}");
        }
        assert!(!is_trap_trigger_type(0xcc));
        assert!(!is_trap_trigger_type(0xcd));
        assert!(!is_trap_trigger_type(0xd0));
    }

    #[test]
    fn goliath_turret_predicate_matches_both_rows() {
        assert!(is_goliath_turret_type(uid::GOLIATH_TURRET));
        assert!(is_goliath_turret_type(0x12));
        assert!(!is_goliath_turret_type(uid::GOLIATH));
    }

    #[test]
    fn non_building_predicate_inverts_the_units_dat_flag() {
        let w = open_world();
        assert!(is_non_building_type(&w.data, uid::MARINE));
        assert!(!is_non_building_type(&w.data, uid::BARRACKS));
        assert!(is_non_building_type(&w.data, 0xff));
    }

    #[test]
    fn special_family_predicates_match_the_recovered_rows() {
        for ty in [1, 16, 99, 100, 104] {
            assert!(is_ghost_family_type(ty), "ghost {ty}");
        }
        assert!(!is_ghost_family_type(0));
        assert!(is_hangar_unit_type(0x49));
        assert!(is_hangar_unit_type(0x55));
        assert!(!is_hangar_unit_type(uid::CARRIER));
        assert!(is_firebat_family_type(uid::FIREBAT));
        assert!(is_firebat_family_type(10));
        assert!(!is_firebat_family_type(uid::MARINE));
    }

    #[test]
    fn zerg_flag_predicate_matches_the_units_dat_bit() {
        let w = open_world();
        assert!(w.data.unit(uid::ZERGLING).is_zerg_flag());
        assert!(w.data.unit(uid::DRONE).is_zerg_flag());
        assert!(!w.data.unit(uid::MARINE).is_zerg_flag());
    }

    #[test]
    fn carrier_reaver_family_predicate_matches_all_four_rows() {
        for ty in [0x48, 0x52, 0x53, 0x51] {
            assert!(is_carrier_reaver_family(ty), "type {ty:#x}");
        }
        assert!(!is_carrier_reaver_family(uid::SCOUT));
    }

    #[test]
    fn carrier_defiler_and_medic_predicates_match_their_rows() {
        assert!(is_carrier_type(0x48));
        assert!(is_carrier_type(0x52));
        assert!(!is_carrier_type(0x53));
        assert!(is_defiler_family_type(0x2e));
        assert!(is_defiler_family_type(0x34));
        assert!(!is_defiler_family_type(uid::HYDRALISK));
        assert!(is_medic_type(0x22));
        assert!(!is_medic_type(uid::MARINE));
    }

    #[test]
    fn valkyrie_predicate_matches_row_0x3a() {
        assert!(is_valkyrie_type(0x3a));
        assert!(!is_valkyrie_type(uid::WRAITH));
    }

    #[test]
    fn nydus_canal_predicate_matches_row_0x86() {
        assert!(is_nydus_canal_type(uid::NYDUS_CANAL));
        assert!(!is_nydus_canal_type(uid::HATCHERY));
    }

    #[test]
    fn powerup_flag_predicate_matches_the_units_dat_bit() {
        let w = open_world();
        assert!(w.data.unit(128).is_powerup_flag()); // Uraj Crystal
        assert!(w.data.unit(215).is_powerup_flag()); // Flag
        assert!(w.data.unit(220).is_powerup_flag()); // Mineral Chunk
        assert!(!w.data.unit(uid::MARINE).is_powerup_flag());
    }

    #[test]
    fn detector_flag_predicate_matches_units_dat() {
        let w = open_world();
        assert!(w.data.unit(uid::SCIENCE_VESSEL).is_detector());
        assert!(w.data.unit(uid::OBSERVER).is_detector());
        assert!(!w.data.unit(uid::MARINE).is_detector());
    }

    #[test]
    fn enemy_predicate_uses_the_alliance_table() {
        let mut w = open_world();
        assert!(w.is_enemy(0, 1));
        assert!(!w.is_enemy(0, 0));
        w.players[1].team = w.players[0].team;
        assert!(!w.is_enemy(0, 1));
    }

    #[test]
    fn same_side_predicate_matches_the_alliance_table() {
        let mut w = open_world();
        assert!(w.same_side(0, 0));
        assert!(!w.same_side(0, 1));
        w.players[1].team = w.players[0].team;
        assert!(w.same_side(0, 1));
    }

    #[test]
    fn connected_unit_and_nuke_ready_getters_match_their_fields() {
        let mut w = open_world();
        let hatch = w.spawn(0, uid::HATCHERY, 320, 320).unwrap();
        let larva = w.spawn(0, uid::LARVA, 320, 340).unwrap();
        w.units[larva].connected_unit = Some(w.units[hatch].id);
        assert_eq!(w.units[larva].connected_unit_id(), Some(w.units[hatch].id));
        let silo = w.spawn(0, uid::NUCLEAR_SILO, 400, 320).unwrap();
        assert!(!w.units[silo].nuke_ready_flag());
        w.units[silo].nuke_ready = true;
        assert!(w.units[silo].nuke_ready_flag());
    }

    #[test]
    fn pending_tech_and_upgrade_getters_read_the_active_order() {
        let mut w = open_world();
        let building = w.spawn(0, uid::ACADEMY, 320, 320).unwrap();
        assert_eq!(w.units[building].pending_tech(), None);
        w.units[building].order = Order::Research {
            tech_id: tech_id::STIM_PACKS,
        };
        assert_eq!(w.units[building].pending_tech(), Some(tech_id::STIM_PACKS));
        w.units[building].order = Order::Upgrade {
            upgrade_id: upgrade::U238_SHELLS,
            level: 1,
        };
        assert_eq!(
            w.units[building].pending_upgrade(),
            Some(upgrade::U238_SHELLS)
        );
    }

    #[test]
    fn acid_spore_getter_matches_the_status_counter() {
        let mut w = open_world();
        let z = w.spawn(0, uid::ZERGLING, 320, 320).unwrap();
        assert_eq!(w.units[z].acid_spore_count(), 0);
        w.units[z].status.acid_spores = 3;
        assert_eq!(w.units[z].acid_spore_count(), 3);
    }

    #[test]
    fn max_hp_lookup_matches_the_units_dat_table() {
        let w = open_world();
        assert_eq!(
            w.unit_max_hp_raw(uid::MARINE),
            w.data.unit(uid::MARINE).hit_points_raw as i32
        );
        assert_eq!(
            w.unit_max_hp_raw(uid::BARRACKS),
            w.data.unit(uid::BARRACKS).hit_points_raw as i32
        );
    }

    #[test]
    fn special_type_and_state_getters_match_their_rows() {
        assert!(is_map_revealer_type(0x65));
        assert!(is_scanner_sweep_type(0x21));
        assert!(is_nuclear_silo_type(0x6c));
        assert!(!is_nuclear_silo_type(uid::COMMAND_CENTER));

        let mut w = open_world();
        let b = w.spawn(0, uid::ACADEMY, 320, 320).unwrap();
        assert!(!w.units[b].has_pending_upgrade());
        w.units[b].order = Order::Upgrade {
            upgrade_id: upgrade::U238_SHELLS,
            level: 1,
        };
        assert!(w.units[b].has_pending_upgrade());
        assert_eq!(w.units[b].lockdown_ticks(), 0);
        w.units[b].status.lockdown_ticks = 7;
        assert_eq!(w.units[b].lockdown_ticks(), 7);
        let hatch = w.spawn(0, uid::HATCHERY, 400, 320).unwrap();
        let hatch_id = w.units[hatch].id;
        w.units[b].set_connected_unit(Some(hatch_id));
        assert_eq!(w.units[b].connected_unit_id(), Some(hatch_id));
    }

    #[test]
    fn target_maelstrom_and_supply_getters_match_their_fields() {
        let mut w = open_world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        assert!(!w.units[marine].has_target());
        w.units[marine].target = Some(7);
        assert!(w.units[marine].has_target());
        assert_eq!(w.units[marine].maelstrom_ticks(), 0);
        w.units[marine].status.maelstrom_ticks = 5;
        assert_eq!(w.units[marine].maelstrom_ticks(), 5);
        assert_eq!(
            w.unit_supply_provided(uid::OVERLORD),
            w.data.unit(uid::OVERLORD).supply_provided
        );
    }

    #[test]
    fn unit_box_extent_getters_match_units_dat() {
        let w = open_world();
        for id in [uid::MARINE, uid::BARRACKS, uid::COMMAND_CENTER] {
            let d = w.data.unit(id).dimensions;
            assert_eq!(w.unit_box_left(id), d.left as i32, "left {id}");
            assert_eq!(w.unit_box_right(id), d.right as i32, "right {id}");
            assert_eq!(w.unit_box_top(id), d.top as i32, "top {id}");
        }
    }

    #[test]
    fn box_bottom_height_and_path_extents_follow_units_dat() {
        let w = open_world();
        for id in [uid::MARINE, uid::BARRACKS, uid::COMMAND_CENTER] {
            let d = w.data.unit(id).dimensions;
            assert_eq!(w.unit_box_bottom(id), d.bottom as i32, "bottom {id}");
            assert_eq!(
                w.unit_box_height(id),
                d.top as i32 + 1 + d.bottom as i32,
                "height {id}"
            );
            assert_eq!(
                w.unit_path_extents(id),
                [
                    d.top as i32,
                    -(d.right as i32),
                    -(d.bottom as i32),
                    d.left as i32,
                    d.top as i32 + 1,
                    !(d.right as i32),
                    !(d.bottom as i32),
                    d.left as i32 + 1,
                ],
                "path extents {id}"
            );
        }
        // Bit 15 selects the all-round radius form.
        assert_eq!(w.unit_path_extents(0x8005), [5, -5, -5, 5, 6, -6, -6, 6]);
    }

    #[test]
    fn target_and_link_accessors_reproduce_the_cunit_setters() {
        let mut w = open_world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        assert!(!w.units[marine].has_target());
        w.units[marine].set_target(42);
        assert_eq!(w.units[marine].target, Some(42));
        w.units[marine].set_target(0);
        assert!(!w.units[marine].has_target());
        w.units[marine].set_target(42);
        w.units[marine].clear_target();
        assert!(!w.units[marine].has_target());

        let hatch = w.spawn(0, uid::HATCHERY, 400, 320).unwrap();
        let hatch_id = w.units[hatch].id;
        w.units[marine].set_connected_unit(Some(hatch_id));
        w.units[marine].clear_connected_unit();
        assert_eq!(w.units[marine].connected_unit_id(), None);

        let academy = w.spawn(0, uid::ACADEMY, 480, 320).unwrap();
        w.units[academy].order = Order::Research {
            tech_id: tech_id::STIM_PACKS,
        };
        assert_eq!(w.units[academy].pending_tech(), Some(tech_id::STIM_PACKS));
        w.units[academy].clear_pending_tech();
        assert_eq!(w.units[academy].pending_tech(), None);
        assert!(matches!(w.units[academy].order, Order::Idle));
        // A non-research order is not disturbed.
        w.units[academy].order = Order::Train {
            unit_type: uid::MARINE,
        };
        w.units[academy].clear_pending_tech();
        assert!(matches!(
            w.units[academy].order,
            Order::Train { unit_type } if unit_type == uid::MARINE
        ));

        assert!(is_infested_terran_type(0x32));
        assert!(!is_infested_terran_type(uid::MARINE));
    }

    #[test]
    fn units_dat_flags_and_resource_amount_match_their_rows() {
        let mut w = open_world();
        // 0x005b3a50 / 0x005b57b0 / 0x005b5950 / 0x005b61d0.
        assert!(w.unit_regenerates(uid::ZERGLING));
        assert!(!w.unit_regenerates(uid::MARINE));
        assert!(w.unit_is_addon(107)); // Comsat Station.
        assert!(!w.unit_is_addon(uid::COMMAND_CENTER));
        assert!(w.unit_can_cloak(uid::GHOST));
        assert!(w.unit_can_cloak(uid::WRAITH));
        assert!(!w.unit_can_cloak(uid::MARINE));
        assert!(w.unit_is_resource_container(176)); // Mineral Field.
        assert!(!w.unit_is_resource_container(uid::BARRACKS));

        // 0x005b6010: Queen / Matriarch only.
        assert!(is_queen_type(uid::QUEEN));
        assert!(is_queen_type(0x31));
        assert!(!is_queen_type(uid::DEFILER));

        // 0x005b53a0: +0xd4 exposed only for flag-0x800 powerup rows.
        let chunk = w.spawn(0, 220, 300, 300).unwrap(); // Mineral Chunk.
        w.units[chunk].resource_amount = 1500;
        assert_eq!(w.unit_resource_amount(chunk), 1500);
        // A mineral field carries 0x2000, not the powerup bit 0x800.
        let field = w.spawn(0, 176, 340, 300).unwrap();
        w.units[field].resource_amount = 1500;
        assert_eq!(w.unit_resource_amount(field), 0);
        let marine = w.spawn(0, uid::MARINE, 400, 300).unwrap();
        w.units[marine].resource_amount = 99;
        assert_eq!(w.unit_resource_amount(marine), 0);
    }

    #[test]
    fn energy_gate_and_reactor_upgrades_match_the_retail_switch() {
        assert_eq!(energy_upgrade(uid::GHOST), Some(upgrade::MOEBIUS_REACTOR));
        assert_eq!(energy_upgrade(uid::WRAITH), Some(upgrade::APOLLO_REACTOR));
        assert_eq!(
            energy_upgrade(uid::SCIENCE_VESSEL),
            Some(upgrade::TITAN_REACTOR)
        );
        assert_eq!(
            energy_upgrade(uid::BATTLECRUISER),
            Some(upgrade::COLOSSUS_REACTOR)
        );
        assert_eq!(energy_upgrade(uid::MEDIC), Some(upgrade::CADUCEUS_REACTOR));
        assert_eq!(energy_upgrade(uid::QUEEN), Some(upgrade::GAMETE_MEIOSIS));
        assert_eq!(
            energy_upgrade(uid::DEFILER),
            Some(upgrade::METASYNAPTIC_NODE)
        );
        assert_eq!(energy_upgrade(uid::CORSAIR), Some(upgrade::ARGUS_JEWEL));
        assert_eq!(
            energy_upgrade(uid::DARK_ARCHON),
            Some(upgrade::ARGUS_TALISMAN)
        );
        assert_eq!(
            energy_upgrade(uid::HIGH_TEMPLAR),
            Some(upgrade::KHAYDARIN_AMULET)
        );
        assert_eq!(energy_upgrade(uid::ARBITER), Some(upgrade::KHAYDARIN_CORE));
        assert_eq!(energy_upgrade(uid::MARINE), None);

        let w = open_world();
        let mut upgrades = [0u8; 61];
        assert_eq!(
            unit_max_energy_fixed(&w.data, &upgrades, uid::GHOST),
            MAX_ENERGY * ENERGY_SCALE
        );
        upgrades[upgrade::MOEBIUS_REACTOR as usize] = 1;
        assert_eq!(
            unit_max_energy_fixed(&w.data, &upgrades, uid::GHOST),
            MAX_ENERGY_UPGRADED * ENERGY_SCALE
        );

        assert_eq!(
            unit_max_energy_points(&w.data, &upgrades, uid::GHOST),
            MAX_ENERGY_UPGRADED
        );
        assert_eq!(
            unit_max_energy_points(&w.data, &[0u8; 61], uid::MARINE),
            MAX_ENERGY
        );

        let mut w = open_world();
        let ghost = w.spawn(0, uid::GHOST, 320, 320).unwrap();
        w.units[ghost].set_energy_points(50);
        assert_eq!(w.units[ghost].energy_fixed(), 50 * ENERGY_SCALE);
        w.units[ghost].set_energy_points(999);
        assert_eq!(
            w.units[ghost].energy_fixed(),
            unit_max_energy_fixed(&w.data, &w.players[0].upgrades, uid::GHOST)
        );
        w.units[ghost].set_energy_fixed(100);
        assert!(w.units[ghost].energy_below(200));
        assert!(!w.units[ghost].energy_below(100));
        assert!(!w.units[ghost].spend_energy_fixed(200));
        assert_eq!(w.units[ghost].energy_fixed(), 100);
        assert!(w.units[ghost].spend_energy_fixed(100));
        assert_eq!(w.units[ghost].energy_fixed(), 0);
    }

    #[test]
    fn building_and_flying_building_flag_getters_follow_the_retail_guards() {
        let w = open_world();
        // 0x005b5820 / 0x005b58e0.
        assert!(w.unit_is_building_flagged(uid::BARRACKS));
        assert!(!w.unit_is_not_building_flagged(uid::BARRACKS));
        assert!(!w.unit_is_building_flagged(uid::MARINE));
        assert!(w.unit_is_not_building_flagged(uid::MARINE));
        // Ids >= 0xe4 are outside the per-type table and count as non-buildings.
        assert!(!w.unit_is_building_flagged(0xe4));
        assert!(w.unit_is_not_building_flagged(0xe4));
        // 0x005b5ea0.
        assert!(w.unit_is_flying_building(uid::COMMAND_CENTER));
        assert!(!w.unit_is_flying_building(uid::HATCHERY));
        // 0x005b5e70.
        assert!(w.data.unit(uid::SCV).is_mechanical());
        assert!(!w.data.unit(uid::ZERGLING).is_mechanical());
    }

    #[test]
    fn hp_point_conversions_match_the_retail_table_helpers() {
        let mut w = open_world();
        assert_eq!(
            w.unit_max_hp_third(uid::MARINE),
            w.data.unit(uid::MARINE).hit_points_raw as i32 / 3
        );
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        assert_eq!(w.unit_max_hp_points(marine), 40);
        assert_eq!(w.unit_total_hit_points(marine), 40);

        let zealot = w.spawn(1, uid::ZEALOT, 400, 320).unwrap();
        let z = w.data.unit(uid::ZEALOT);
        assert_eq!(
            w.unit_total_hit_points(zealot),
            (z.hit_points_raw as i32 >> 8) + z.shield_points as i32
        );

        // A subunit row with raw HP 1 has zero whole max HP; the helper
        // rounds the current pool up instead (FUN_00588d60).
        let turret = w.spawn(0, uid::GOLIATH_TURRET, 480, 320).unwrap();
        w.units[turret].hp = 300;
        assert_eq!(w.unit_max_hp_points(turret), 2);
        w.units[turret].hp = 1;
        assert_eq!(w.unit_max_hp_points(turret), 1);
    }

    #[test]
    fn psi_storm_guard_blocks_a_second_hit_in_the_same_cadence() {
        let mut w = open_world();
        w.fog_of_war = false;
        let caster = w.spawn(0, uid::HIGH_TEMPLAR, 320, 320).unwrap();
        let victim = w.spawn(1, uid::MARINE, 340, 320).unwrap();
        assert_eq!(w.units[victim].storm_guard(), 0);
        let weapon = w.data.weapon(wid::PSIONIC_STORM).cloned().unwrap();
        let before = w.units[victim].hp;
        w.apply_splash(
            0,
            caster,
            wid::PSIONIC_STORM,
            &weapon,
            None,
            (340 * fp(1), 320 * fp(1)),
            false,
            false,
        );
        let after_first = w.units[victim].hp;
        assert!(after_first < before, "first storm hit must damage");
        assert_eq!(w.units[victim].storm_guard(), 1);
        w.apply_splash(
            0,
            caster,
            wid::PSIONIC_STORM,
            &weapon,
            None,
            (340 * fp(1), 320 * fp(1)),
            false,
            false,
        );
        assert_eq!(
            w.units[victim].hp, after_first,
            "guard must block the same-cadence repeat"
        );
        // Next logical tick clears the guard, so the following cadence hits.
        w.step();
        assert_eq!(w.units[victim].storm_guard(), 0);
        w.apply_splash(
            0,
            caster,
            wid::PSIONIC_STORM,
            &weapon,
            None,
            (340 * fp(1), 320 * fp(1)),
            false,
            false,
        );
        assert!(w.units[victim].hp < after_first);
    }

    #[test]
    fn placement_and_destroy_score_getters_match_units_dat() {
        let w = open_world();
        for id in [uid::BARRACKS, uid::COMMAND_CENTER, uid::HATCHERY] {
            let ty = w.data.unit(id);
            assert_eq!(w.unit_placement_x(id), ty.placement.x as i32, "px {id}");
            assert_eq!(w.unit_placement_y(id), ty.placement.y as i32, "py {id}");
            assert_eq!(
                w.unit_destroy_score(id),
                ty.destroy_score,
                "destroy score {id}"
            );
        }
        assert!(w.unit_destroy_score(uid::MARINE) > 0);
    }

    #[test]
    fn shield_flag_and_current_total_follow_the_retail_helpers() {
        let mut w = open_world();
        assert!(w.unit_has_shields(uid::ZEALOT));
        assert!(!w.unit_has_shields(uid::MARINE));

        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        w.units[marine].hp = 1024; // 4 whole points.
        w.units[marine].shields = 5000; // must be ignored: no shield flag.
        assert_eq!(w.unit_current_total_points(marine), 4);

        let zealot = w.spawn(1, uid::ZEALOT, 400, 320).unwrap();
        let max_shields = w.data.unit(uid::ZEALOT).shield_points as i32;
        w.units[zealot].hp = 2560; // 10 whole points.
        w.set_unit_shields_points(zealot, max_shields + 25);
        assert_eq!(w.units[zealot].shields, max_shields << 8);
        assert_eq!(w.unit_current_total_points(zealot), 10 + max_shields);
        w.set_unit_shields_points(zealot, 7);
        assert_eq!(w.units[zealot].shields, 7 << 8);
        assert_eq!(w.unit_current_total_points(zealot), 17);
    }

    #[test]
    fn destroying_an_upgrading_producer_refunds_seventy_five_percent() {
        let mut w = open_world();
        let academy = w.spawn(0, uid::ACADEMY, 320, 320).unwrap();
        w.units[academy].completed = true;
        w.players[0].minerals = 1000;
        w.players[0].gas = 1000;
        w.upgrade(w.units[academy].id, upgrade::U238_SHELLS)
            .unwrap();
        assert!(w.units[academy].has_pending_upgrade());
        assert!(!w.units[academy].has_pending_tech());
        let up = w.data.upgrade(upgrade::U238_SHELLS).cloned().unwrap();
        let level = w.players[0].upgrade_level(upgrade::U238_SHELLS);
        let refund_m = up.mineral_cost(level) as i32 * 3 / 4;
        let refund_g = up.gas_cost(level) as i32 * 3 / 4;
        let before_m = w.players[0].minerals;
        let before_g = w.players[0].gas;
        w.kill_unit(academy);
        assert_eq!(w.players[0].minerals, before_m + refund_m);
        assert_eq!(w.players[0].gas, before_g + refund_g);
    }

    #[test]
    fn pending_tech_and_upgrade_presence_helpers_match_their_orders() {
        let mut w = open_world();
        let b = w.spawn(0, uid::ACADEMY, 320, 320).unwrap();
        assert!(!w.units[b].has_pending_tech());
        assert!(!w.units[b].has_pending_upgrade());
        w.units[b].order = Order::Research {
            tech_id: tech_id::STIM_PACKS,
        };
        assert!(w.units[b].has_pending_tech());
        assert!(!w.units[b].has_pending_upgrade());
        w.units[b].order = Order::Upgrade {
            upgrade_id: upgrade::U238_SHELLS,
            level: 1,
        };
        assert!(w.units[b].has_pending_upgrade());
        assert!(!w.units[b].has_pending_tech());
    }

    #[test]
    fn shield_race_damage_energy_and_box_width_helpers_match_retail() {
        let mut w = open_world();
        assert_eq!(
            w.unit_shield_points(uid::ZEALOT),
            w.data.unit(uid::ZEALOT).shield_points as i32
        );
        assert_eq!(w.unit_race_class(uid::ZERGLING), 0);
        assert_eq!(w.unit_race_class(uid::MARINE), 1);
        assert_eq!(w.unit_race_class(uid::ZEALOT), 2);
        let ghost = w.spawn(0, uid::GHOST, 300, 300).unwrap();
        let marine = w.spawn(0, uid::MARINE, 360, 300).unwrap();
        assert!(w.unit_has_energy(ghost));
        assert!(!w.unit_has_energy(marine));

        w.units[marine].hp = 100;
        assert!(w.unit_is_damaged(marine));
        w.units[marine].hp = w.data.unit(uid::MARINE).hit_points_raw as i32;
        assert!(!w.unit_is_damaged(marine));

        for id in [uid::MARINE, uid::BARRACKS, uid::HATCHERY] {
            let d = w.data.unit(id).dimensions;
            assert_eq!(
                w.unit_box_width(id),
                d.right as i32 + 1 + d.left as i32,
                "width {id}"
            );
        }
    }

    #[test]
    fn unit_type_predicates_match_the_retail_rows() {
        assert!(is_suicide_unit_type(0x2f));
        assert!(is_suicide_unit_type(0x32));
        assert!(!is_suicide_unit_type(uid::MARINE));
        assert!(is_beacon_type(0xc2));
        assert!(is_beacon_type(0xc7));
        assert!(is_beacon_type(0xcc));
        assert!(!is_beacon_type(uid::MARINE));
        assert!(is_ultralisk_type(0x27));
        assert!(is_ultralisk_type(0x30));
        assert!(!is_ultralisk_type(0x28));
        assert!(is_vulture_type(0x02));
        assert!(is_vulture_type(0x13));
        assert!(!is_vulture_type(uid::MARINE));
        assert!(is_zealot_type(0x41));
        assert!(is_zealot_type(0x4d));
        assert!(!is_zealot_type(uid::DRAGOON));
        assert!(is_zergling_type(0x25));
        assert!(is_zergling_type(0x36));
        assert!(!is_zergling_type(uid::HYDRALISK));
        assert!(is_critter_type(0x59));
        assert!(is_critter_type(0x5a));
        assert!(is_critter_type(0x5d));
        assert!(is_critter_type(0x60));
        assert!(!is_critter_type(0x61));
        assert!(is_gas_building(uid::REFINERY));
        assert!(is_gas_building(uid::EXTRACTOR));
        assert!(is_gas_building(uid::ASSIMILATOR));
        assert!(!is_gas_building(uid::COMMAND_CENTER));
    }

    #[test]
    fn status_refresh_helpers_keep_the_longest_timer_and_clear_on_expiry() {
        let mut s = Status::default();
        s.apply_lockdown(3);
        s.apply_lockdown(1);
        assert_eq!(s.lockdown_ticks, 3);
        s.apply_lockdown(7);
        assert_eq!(s.lockdown_ticks, 7);
        s.clear_lockdown();
        assert_eq!(s.lockdown_ticks, 0);

        s.apply_stasis(2);
        s.apply_stasis(9);
        assert_eq!(s.stasis_ticks, 9);
        s.clear_stasis();
        assert_eq!(s.stasis_ticks, 0);

        s.apply_maelstrom(4);
        s.apply_maelstrom(1);
        assert_eq!(s.maelstrom_ticks, 4);
        s.clear_maelstrom();
        assert_eq!(s.maelstrom_ticks, 0);

        s.apply_plague();
        assert_eq!(s.plague_ticks, PLAGUE_UPDATES);
        s.clear_plague();
        assert_eq!(s.plague_ticks, 0);
        s.apply_lockdown(2);
        s.clear_lockdown();
        assert_eq!(s.lockdown_ticks, 0);
        s.ensnare_ticks = 5;
        s.clear_ensnare();
        assert_eq!(s.ensnare_ticks, 0);
        s.apply_ensnare();
        assert_eq!(s.ensnare_ticks, ENSNARE_UPDATES);
        s.apply_irradiate(IRRADIATE_UPDATES);
        assert_eq!(s.irradiate_ticks, IRRADIATE_UPDATES);
        s.clear_irradiate();
        assert_eq!(s.irradiate_ticks, 0);
    }

    #[test]
    fn death_clears_removable_status_timers_and_acid_spores() {
        let mut w = open_world();
        let z = w.spawn(0, uid::ZERGLING, 320, 320).unwrap();
        let st = &mut w.units[z].status;
        st.ensnare_ticks = 5;
        st.plague_ticks = 5;
        st.irradiate_ticks = 5;
        st.lockdown_ticks = 5;
        st.maelstrom_ticks = 5;
        st.add_acid_spore();
        assert!(st.acid_spores > 0);
        w.kill_unit(z);
        let st = &w.units[z].status;
        assert_eq!(st.ensnare_ticks, 0);
        assert_eq!(st.plague_ticks, 0);
        assert_eq!(st.irradiate_ticks, 0);
        assert_eq!(st.lockdown_ticks, 0);
        assert_eq!(st.maelstrom_ticks, 0);
        assert_eq!(st.acid_spores, 0);
    }

    #[test]
    fn unit_voice_timer_and_sound_id_follow_the_cunit_fields() {
        let mut w = open_world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        assert_eq!(w.units[marine].speech_sound(), 0);
        assert_eq!(w.units[marine].tick_speech(), None);
        w.units[marine].say(2, 0x2a);
        assert_eq!(w.units[marine].speech_sound(), 0x2a);
        assert_eq!(w.units[marine].speech_ticks, 2);
        w.step();
        assert_eq!(w.units[marine].speech_ticks, 1);
        w.step();
        assert_eq!(w.units[marine].speech_ticks, 0);
        // The getter is an atomic read/decrement.
        w.units[marine].say(3, 1);
        assert_eq!(w.units[marine].tick_speech(), Some(3));
        assert_eq!(w.units[marine].tick_speech(), Some(2));
        assert_eq!(w.units[marine].tick_speech(), Some(1));
        assert_eq!(w.units[marine].tick_speech(), None);
    }

    #[test]
    fn completed_loaded_and_cloak_flags_follow_the_unit_state() {
        let mut w = open_world();
        let barracks = w.spawn(0, uid::BARRACKS, 320, 320).unwrap();
        w.units[barracks].completed = false;
        assert!(!w.units[barracks].is_completed_flag());
        w.units[barracks].completed = true;
        assert!(w.units[barracks].is_completed_flag());

        let marine = w.spawn(0, uid::MARINE, 400, 320).unwrap();
        assert!(!w.units[marine].is_loaded());
        w.units[marine].in_transport = Some(w.units[barracks].id);
        assert!(w.units[marine].is_loaded());

        assert!(!w.units[marine].is_burrowed_flags());
        w.units[marine].status.burrowed = true;
        assert!(w.units[marine].is_burrowed_flags());

        let ghost = w.spawn(0, uid::GHOST, 480, 320).unwrap();
        assert!(!w.units[ghost].cloak_bit());
        w.units[ghost].status.cloaked = true;
        assert_eq!(w.units[ghost].cloak_state(), 2);
        assert!(w.units[ghost].cloak_bit());
    }

    #[test]
    fn research_timer_and_death_timer_are_readable_and_tick_down() {
        let mut w = open_world();
        let academy = w.spawn(0, uid::ACADEMY, 320, 320).unwrap();
        w.units[academy].completed = true;
        w.players[0].minerals = 1000;
        w.players[0].gas = 1000;
        w.upgrade(w.units[academy].id, upgrade::U238_SHELLS)
            .unwrap();
        let remaining = w.units[academy].research_remaining();
        assert!(remaining > 0);
        assert_eq!(remaining as i32, w.units[academy].build_remaining);

        let z = w.spawn(0, uid::ZERGLING, 420, 320).unwrap();
        assert_eq!(w.units[z].death_timer(), 0);
        w.kill_unit(z);
        assert_eq!(w.units[z].death_timer(), 1350);
        w.step();
        assert_eq!(w.units[z].death_timer(), 1349);
        // `0x005b6810` keeps the smaller value once it is set.
        w.units[z].set_death_timer_min(2000);
        assert_eq!(w.units[z].death_timer(), 1349);
        w.units[z].set_death_timer_min(10);
        assert_eq!(w.units[z].death_timer(), 10);
    }

    #[test]
    fn rank_kill_counter_increments_and_caps_at_255() {
        let mut w = open_world();
        let a = w.spawn(0, uid::MARINE, 300, 300).unwrap();
        let b = w.spawn(1, uid::ZERGLING, 340, 300).unwrap();
        assert_eq!(w.units[a].rank_kills(), 0);
        w.units[b].last_damage_source = w.units[a].id;
        w.kill_unit(b);
        assert_eq!(w.units[a].rank_kills(), 1);
        assert_eq!(w.units[a].kills, 1);
        w.units[a].rank_kills = 254;
        w.units[a].credit_rank_kill();
        assert_eq!(w.units[a].rank_kills(), 255);
        w.units[a].credit_rank_kill();
        assert_eq!(w.units[a].rank_kills(), 255);
    }

    #[test]
    fn pending_research_raw_sum_and_queue_count_match_the_cunit_fields() {
        let mut w = open_world();
        let academy = w.spawn(0, uid::ACADEMY, 320, 320).unwrap();
        assert_eq!(w.units[academy].pending_tech_raw(), 0);
        assert_eq!(w.units[academy].pending_research_sum(), 0);
        assert_eq!(w.units[academy].queued_count(), 0);
        w.units[academy].order = Order::Research {
            tech_id: tech_id::STIM_PACKS,
        };
        assert_eq!(
            w.units[academy].pending_tech_raw(),
            u32::from(tech_id::STIM_PACKS)
        );
        assert_eq!(w.units[academy].pending_research_sum(), tech_id::STIM_PACKS);
        w.units[academy].order = Order::Upgrade {
            upgrade_id: upgrade::U238_SHELLS,
            level: 1,
        };
        assert_eq!(
            w.units[academy].pending_research_sum(),
            upgrade::U238_SHELLS
        );
        w.units[academy].production.push_back(Order::Train {
            unit_type: uid::MARINE,
        });
        assert_eq!(w.units[academy].queued_count(), 1);
    }

    #[test]
    fn placement_precheck_uses_box_extents_and_terrain_flags() {
        let w = open_world();
        let (wpx, hpx) = (w.terrain.width as i32 * 32, w.terrain.height as i32 * 32);
        // Find a buildable point away from the map border.
        let mut probe = None;
        'outer: for y in (64..hpx - 64).step_by(32) {
            for x in (64..wpx - 64).step_by(32) {
                if w.terrain_accepts_placement(x, y) {
                    probe = Some((x, y));
                    break 'outer;
                }
            }
        }
        let (px, py) = probe.expect("test map needs one buildable point");
        assert!(w.placement_corner_valid(uid::BARRACKS, px, py));
        assert!(!w.placement_corner_valid(uid::BARRACKS, -64, py));
        assert!(!w.placement_corner_valid(uid::BARRACKS, wpx, py));
        assert!(!w.placement_corner_valid(uid::BARRACKS, px, -64));
        assert!(!w.placement_corner_valid(uid::BARRACKS, px, hpx));
    }

    #[test]
    fn status_merge_keeps_the_longest_timers_and_largest_spores() {
        let mut a = Status::default();
        let mut b = Status::default();
        a.lockdown_ticks = 5;
        b.lockdown_ticks = 9;
        a.stasis_ticks = 12;
        b.stasis_ticks = 3;
        a.matrix_ticks = 2;
        a.matrix = 100;
        b.matrix_ticks = 7;
        b.matrix = 500;
        a.acid_spores = 2;
        a.acid_spore_ticks[0] = 4;
        b.acid_spores = 4;
        b.acid_spore_ticks[0] = 9;
        b.parasite = true;
        a.merge_from(&b);
        assert_eq!(a.lockdown_ticks, 9);
        assert_eq!(a.stasis_ticks, 12);
        assert_eq!(a.matrix_ticks, 7);
        assert_eq!(a.matrix, 500);
        assert_eq!(a.acid_spores, 4);
        assert_eq!(a.acid_spore_ticks[0], 9);
        assert!(a.parasite);
    }

    #[test]
    fn archon_merge_inherits_source_statuses() {
        let mut w = open_world();
        let a = w.spawn(0, uid::HIGH_TEMPLAR, 320, 320).unwrap();
        let b = w.spawn(0, uid::HIGH_TEMPLAR, 340, 320).unwrap();
        w.units[a].status.plague_ticks = 40;
        w.units[b].status.ensnare_ticks = 70;
        w.cast(w.units[a].id, Spell::ArchonWarp, SpellTarget::None)
            .unwrap();
        let archon = w
            .units
            .iter()
            .find(|u| u.alive && u.type_id == uid::ARCHON)
            .expect("archon spawned");
        assert_eq!(archon.status.plague_ticks, 40);
        assert_eq!(archon.status.ensnare_ticks, 70);
    }

    #[test]
    fn linked_nuke_missile_state_tracks_launches() {
        let mut w = open_world();
        let cc = w.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
        let silo = w.spawn(0, uid::NUCLEAR_SILO, 352, 320).unwrap();
        w.units[silo].connected_unit = Some(w.units[cc].id);
        let silo_id = w.units[silo].id;
        assert!(w.has_no_linked_missile(silo_id));
        assert!(w.link_nuke_missile(silo_id));
        w.nuke_strikes.push(NukeStrike {
            owner: 0,
            ghost: 0,
            silo: silo_id,
            target: (320, 320),
            timer: 100,
            dot: true,
            warned: false,
            missile: 0,
        });
        assert!(!w.has_no_linked_missile(silo_id));
        let other = w.spawn(0, uid::NUCLEAR_SILO, 420, 320).unwrap();
        assert!(w.has_no_linked_missile(w.units[other].id));
    }

    #[test]
    fn per_unit_masks_set_clear_and_merge_bitwise() {
        let mut w = open_world();
        let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        assert_eq!(w.units[u].mask_121(), 0);
        assert_eq!(w.units[u].mask_123(), 0);
        w.units[u].set_mask_121_bit(0);
        w.units[u].set_mask_121_bit(3);
        w.units[u].set_mask_123_bit(2);
        assert_eq!(w.units[u].mask_121(), 0b0000_1001);
        assert_eq!(w.units[u].mask_123(), 0b0000_0100);
        let other = Status {
            status_mask_121: 0b0000_0010,
            status_mask_123: 0b0000_0001,
            ..Status::default()
        };
        w.units[u].status.merge_from(&other);
        assert_eq!(w.units[u].mask_121(), 0b0000_1011);
        assert_eq!(w.units[u].mask_123(), 0b0000_0101);
        w.units[u].clear_mask_121();
        w.units[u].clear_mask_123();
        assert_eq!(w.units[u].mask_121(), 0);
        assert_eq!(w.units[u].mask_123(), 0);
    }

    #[test]
    fn zerg_queue_helpers_expose_the_front_unit_or_the_type_itself() {
        assert!(is_zerg_queue_building(uid::LAIR));
        assert!(is_zerg_queue_building(uid::HIVE));
        assert!(is_zerg_queue_building(uid::GREATER_SPIRE));
        assert!(is_zerg_queue_building(uid::SPORE_COLONY));
        assert!(is_zerg_queue_building(uid::SUNKEN_COLONY));
        assert!(!is_zerg_queue_building(uid::HATCHERY));

        let mut w = open_world();
        let lair = w.spawn(0, uid::LAIR, 320, 320).unwrap();
        assert_eq!(w.units[lair].current_queue_or_self(), None);
        w.units[lair].production.push_back(Order::Train {
            unit_type: uid::MUTALISK,
        });
        assert_eq!(w.units[lair].current_queue_or_self(), Some(uid::MUTALISK));
        let marine = w.spawn(0, uid::MARINE, 400, 320).unwrap();
        assert_eq!(w.units[marine].current_queue_or_self(), Some(uid::MARINE));
    }

    #[test]
    fn production_queue_clears_on_demand_and_on_death() {
        let mut w = open_world();
        let barracks = w.spawn(0, uid::BARRACKS, 320, 320).unwrap();
        w.units[barracks].production.push_back(Order::Train {
            unit_type: uid::MARINE,
        });
        w.units[barracks].production.push_back(Order::Train {
            unit_type: uid::FIREBAT,
        });
        assert_eq!(w.units[barracks].queued_count(), 2);
        w.units[barracks].clear_production_queue();
        assert_eq!(w.units[barracks].queued_count(), 0);
        w.units[barracks].production.push_back(Order::Train {
            unit_type: uid::MARINE,
        });
        w.kill_unit(barracks);
        assert_eq!(w.units[barracks].queued_count(), 0);
    }

    #[test]
    fn dequeue_removes_and_refunds_the_newest_queued_item() {
        let mut w = open_world();
        let barracks = w.spawn(0, uid::BARRACKS, 320, 320).unwrap();
        let id = w.units[barracks].id;
        w.units[barracks].production.push_back(Order::Train {
            unit_type: uid::MARINE,
        });
        w.units[barracks].production.push_back(Order::Train {
            unit_type: uid::FIREBAT,
        });
        w.players[0].minerals = 0;
        w.players[0].gas = 0;
        let firebat = w.data.unit(uid::FIREBAT).clone();
        assert!(w.dequeue_production(id).unwrap());
        assert_eq!(w.units[barracks].queued_count(), 1);
        assert_eq!(w.players[0].minerals, firebat.mineral_cost as i32);
        assert_eq!(w.players[0].gas, firebat.gas_cost as i32);
        assert!(w.dequeue_production(id).unwrap());
        assert_eq!(w.units[barracks].queued_count(), 0);
        assert!(!w.dequeue_production(id).unwrap());
    }

    #[test]
    fn cancelling_construction_clears_per_unit_masks() {
        let mut w = open_world();
        let b = w.spawn(0, uid::BARRACKS, 320, 320).unwrap();
        w.units[b].completed = false;
        w.units[b].set_mask_121_bit(1);
        w.units[b].set_mask_123_bit(2);
        assert_ne!(w.units[b].mask_121(), 0);
        assert_ne!(w.units[b].mask_123(), 0);
        w.cancel_construction(w.units[b].id).unwrap();
        assert_eq!(w.units[b].mask_121(), 0);
        assert_eq!(w.units[b].mask_123(), 0);
        assert!(!w.units[b].alive);
    }

    #[test]
    fn generic_087_timer_sets_ticks_and_reports_zero() {
        let mut w = open_world();
        let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        assert!(w.units[u].timer_087_is_zero());
        w.units[u].set_timer_087(3);
        assert!(!w.units[u].timer_087_is_zero());
        w.units[u].tick_timer_087();
        assert_eq!(w.units[u].timer_087, 2);
        w.step();
        assert_eq!(w.units[u].timer_087, 1);
        w.step();
        assert!(w.units[u].timer_087_is_zero());
        w.units[u].tick_timer_087();
        assert_eq!(w.units[u].timer_087, 0);
    }

    #[test]
    fn visibility_masks_track_fog_and_the_masked_query() {
        let mut w = open_world();
        let own = w.spawn(0, uid::MARINE, 32, 32).unwrap();
        let far = w.spawn(1, uid::ZERGLING, 2000, 2000).unwrap();
        assert_eq!(w.units[own].visibility_mask(), u32::MAX);
        assert_eq!(w.units[own].visibility_mask_and(1), 1);
        w.enable_fog();
        assert_ne!(w.units[own].visibility_mask() & 1, 0);
        assert_eq!(w.units[far].visibility_mask() & 1, 0);
        assert_ne!(w.units[far].visibility_mask() & 2, 0);
        assert_eq!(w.units[own].visibility_mask_and(1), 1);
        assert_eq!(w.units[far].visibility_mask_and(1), 0);
    }

    #[test]
    fn raw_cunit_getters_return_their_stored_fields() {
        let mut w = open_world();
        let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        assert_eq!(w.units[u].field_090(), 8);
        assert_eq!(w.units[u].field_094(), 0);
        assert_eq!(w.units[u].field_0ac(), 0);
        assert_eq!(w.units[u].field_0e1(), 0);
        w.units[u].field_090 = 3;
        w.units[u].field_094 = 0x1234;
        w.units[u].field_0ac = 0xbeef;
        w.units[u].field_0e1 = 7;
        assert_eq!(w.units[u].field_090(), 3);
        assert_eq!(w.units[u].field_094(), 0x1234);
        assert_eq!(w.units[u].field_0ac(), 0xbeef);
        assert_eq!(w.units[u].field_0e1(), 7);
    }

    #[test]
    fn killing_a_units_order_target_clears_the_order() {
        let mut w = open_world();
        let attacker = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let victim = w.spawn(1, uid::ZERGLING, 360, 320).unwrap();
        let victim_id = w.units[victim].id;
        w.units[attacker].order = Order::AttackUnit { target: victim_id };
        assert_eq!(w.units[attacker].order_unit_target(), Some(victim_id));
        w.kill_unit(victim);
        assert_eq!(w.units[attacker].order_unit_target(), None);
        assert!(matches!(w.units[attacker].order, Order::Idle));
    }

    #[test]
    fn worker_linked_target_helpers_use_the_order_target() {
        let mut w = open_world();
        let scv = w.spawn(0, uid::SCV, 320, 320).unwrap();
        let flag = w.spawn(0, 0xd7, 360, 320).unwrap();
        let flag_id = w.units[flag].id;
        w.units[scv].order = Order::Construct { building: flag_id };
        assert_eq!(w.units[scv].linked_target(), Some(flag_id));
        assert_eq!(w.unit_worker_target(scv), Some(flag_id));
        assert!(w.unit_worker_target_is_flag(scv));

        let marine = w.spawn(0, uid::MARINE, 400, 320).unwrap();
        w.units[marine].order = Order::Construct { building: flag_id };
        assert_eq!(w.units[marine].linked_target(), Some(flag_id));
        assert_eq!(w.unit_worker_target(marine), None);
        assert!(!w.unit_worker_target_is_flag(marine));
    }

    #[test]
    fn bunker_bit_tracks_loading_and_unloading() {
        let mut w = open_world();
        let bunker = w.spawn(0, uid::BUNKER, 320, 320).unwrap();
        let marine = w.spawn(0, uid::MARINE, 340, 320).unwrap();
        assert!(!w.units[marine].is_in_bunker());
        let bunker_id = w.units[bunker].id;
        let marine_id = w.units[marine].id;
        w.order_enter_transport(marine, bunker_id);
        for _ in 0..200 {
            if w.units[marine].in_transport.is_some() {
                break;
            }
            w.step();
        }
        assert!(w.units[marine].is_loaded());
        assert!(w.units[marine].is_in_bunker());
        w.unload_unit(bunker_id, marine_id).unwrap();
        assert!(!w.units[marine].is_loaded());
        assert!(!w.units[marine].is_in_bunker());
    }

    #[test]
    fn status_bit_26_is_cleared_for_unsupported_rows_only() {
        let mut w = open_world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let field = w.spawn(0, 176, 420, 320).unwrap();
        assert!(!w.units[marine].flag_26());
        w.units[marine].flag_26 = true;
        w.units[field].flag_26 = true;
        w.clear_unsupported_flag_26(marine);
        assert!(!w.units[marine].flag_26());
        w.clear_unsupported_flag_26(field);
        assert!(w.units[field].flag_26());
    }

    #[test]
    fn overlay_state_bits_follow_the_two_mode_or() {
        let mut w = open_world();
        let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        assert!(!w.units[u].overlay_bit0());
        assert!(!w.units[u].overlay_bit1());
        w.units[u].set_overlay_state(false, 9);
        assert!(w.units[u].overlay_bit0());
        assert!(!w.units[u].overlay_bit1());
        assert_eq!(w.units[u].field_0cf, 9);
        w.units[u].set_overlay_state(true, 4);
        assert!(w.units[u].overlay_bit0());
        assert!(w.units[u].overlay_bit1());
        assert_eq!(w.units[u].overlay_flags, 3);
        assert_eq!(w.units[u].field_0cf, 4);
    }

    #[test]
    fn raw_status_bits_10_14_15_and_29_toggle_exactly() {
        let mut w = open_world();
        let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        assert!(!w.units[u].flag_10);
        assert!(!w.units[u].flag_14());
        assert!(!w.units[u].flag_15());
        assert!(!w.units[u].flag_29);
        w.units[u].set_flag_10();
        w.units[u].set_flag_15();
        w.units[u].set_flag_29();
        assert!(w.units[u].flag_10);
        assert!(w.units[u].flag_15());
        assert!(w.units[u].flag_29);
        w.units[u].clear_flag_10();
        w.units[u].clear_flag_15();
        assert!(!w.units[u].flag_10);
        assert!(!w.units[u].flag_15());
        w.units[u].flag_14 = true;
        assert!(w.units[u].flag_14());
    }

    #[test]
    fn status_bits_21_22_and_105_read_their_storage() {
        let mut w = open_world();
        let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        assert!(!w.units[u].flag_21());
        assert!(!w.units[u].flag_22());
        assert!(!w.units[u].flag_105_bit0());
        assert!(w.units[u].flag_105_bit1_clear());
        w.units[u].flag_21 = true;
        w.units[u].flag_22 = true;
        w.units[u].flags_105 = 0b11;
        assert!(w.units[u].flag_21());
        assert!(w.units[u].flag_22());
        assert!(w.units[u].flag_105_bit0());
        assert!(!w.units[u].flag_105_bit1_clear());
        w.units[u].flags_105 = 0b01;
        assert!(w.units[u].flag_105_bit0());
        assert!(w.units[u].flag_105_bit1_clear());
    }

    #[test]
    fn order_flags_93_96_and_transport_link_helpers_are_exact() {
        let mut w = open_world();
        let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        w.units[u].set_flag_93_bit1(true);
        assert_eq!(w.units[u].flags_93, 2);
        w.units[u].set_flag_93_bit1(false);
        assert_eq!(w.units[u].flags_93, 0);
        w.units[u].field_096 = 7;
        w.units[u].clear_field_096();
        assert_eq!(w.units[u].field_096, 0);

        assert!(!w.units[u].is_loaded());
        w.units[u].mark_in_transport(42);
        assert!(w.units[u].is_loaded());
        assert_eq!(w.units[u].in_transport, Some(42));
        w.units[u].clear_transport_link();
        assert!(!w.units[u].is_loaded());
    }

    #[test]
    fn status_bits_18_19_and_pointer_100_are_exact() {
        let mut w = open_world();
        let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        assert!(!w.units[u].flag_18);
        assert!(!w.units[u].flag_19);
        assert_eq!(w.units[u].field_100(), 0);
        assert!(!w.units[u].field_100_nonzero());
        w.units[u].set_flag_18(true);
        w.units[u].set_flag_19(true);
        w.units[u].set_field_100(0x1234);
        assert!(w.units[u].flag_18);
        assert!(w.units[u].flag_19);
        assert_eq!(w.units[u].field_100(), 0x1234);
        assert!(w.units[u].field_100_nonzero());
        w.units[u].set_flag_18(false);
        w.units[u].set_flag_19(false);
        w.units[u].set_field_100(0);
        assert!(!w.units[u].flag_18);
        assert!(!w.units[u].flag_19);
        assert!(!w.units[u].field_100_nonzero());
    }

    #[test]
    fn shield_battery_marks_the_target_for_one_tick() {
        let mut w = open_world();
        let battery = w.spawn(0, uid::SHIELD_BATTERY, 320, 320).unwrap();
        let zealot = w.spawn(0, uid::ZEALOT, 360, 320).unwrap();
        w.units[battery].completed = true;
        w.units[battery].energy = 100;
        w.units[zealot].shields = 0;
        assert!(!w.units[zealot].shield_recharge_flag());
        assert!(w.can_shield_battery_target(w.units[battery].id, w.units[zealot].id));
        assert!(!w.can_shield_battery_target(w.units[battery].id, w.units[battery].id));
        w.shield_battery(w.units[battery].id, w.units[zealot].id)
            .unwrap();
        assert!(w.units[zealot].shield_recharge_flag());
        assert!(w.units[zealot].shields > 0);
        w.step();
        assert!(!w.units[zealot].shield_recharge_flag());
    }

    #[test]
    fn subunit_state_sets_bits_16_and_17() {
        let mut w = open_world();
        let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        w.units[u].set_subunit_state(false);
        assert!(w.units[u].flag_16);
        assert!(w.units[u].flag_17);
        w.units[u].set_subunit_state(true);
        assert!(w.units[u].flag_16);
        assert!(!w.units[u].flag_17);
    }

    #[test]
    fn unit_slot_numbers_are_one_based_indices() {
        let mut w = open_world();
        let a = w.spawn(0, uid::MARINE, 300, 300).unwrap();
        let b = w.spawn(0, uid::MARINE, 340, 300).unwrap();
        assert_eq!(w.unit_slot_number(a), a as u32 + 1);
        assert_eq!(w.unit_slot_number(b), b as u32 + 1);
        assert_eq!(w.unit_slot_number(b), w.unit_slot_number(a) + 1);
    }

    #[test]
    fn unit_flag_29_predicate_matches_units_dat() {
        assert!(w_flag29(uid::MARINE));
        assert!(!w_flag29(176)); // Mineral Field carries 0x20000000.
        assert!(!w_flag29(0xe4));
    }

    fn w_flag29(type_id: u16) -> bool {
        let w = open_world();
        w.unit_flag_29_clear(type_id)
    }

    #[test]
    fn rotate_byte_6_matches_the_shift_or_expression() {
        assert_eq!(rotate_byte_6(0b0000_0001), 0b0000_0100);
        assert_eq!(rotate_byte_6(0b0100_0000), 0b0000_0001);
        assert_eq!(rotate_byte_6(0), 0);
        for v in 0u8..=255 {
            assert_eq!(rotate_byte_6(v), v.rotate_left(2));
        }
    }

    #[test]
    fn order_carry_setter_matches_the_retail_minus_one() {
        let mut w = open_world();
        let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        assert!(w.units[u].set_order_carry_from(4));
        assert_eq!(w.units[u].order_carry, 3);
        assert!(w.units[u].set_order_carry_from(0));
        assert_eq!(w.units[u].order_carry, 255);
    }

    #[test]
    fn order_carry_advances_through_the_batch_35_step() {
        let mut w = open_world();
        let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        w.units[u].order_carry = 0;
        assert!(w.units[u].a35_advance_order_carry());
        assert_eq!(w.units[u].order_carry, 8);
        assert!(!w.units[u].a35_advance_order_carry());
        assert_eq!(w.units[u].order_carry, 7);
    }

    #[test]
    fn idle_or_hidden_predicate_matches_order_zero_or_no_sprite() {
        let mut w = open_world();
        let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        assert!(w.units[u].is_idle_or_hidden());
        w.units[u].order = Order::Move { pos: (400, 320) };
        assert!(!w.units[u].is_idle_or_hidden());
        w.units[u].alive = false;
        assert!(w.units[u].is_idle_or_hidden());
    }

    #[test]
    fn units_in_rect_returns_only_centres_inside_the_box() {
        let mut w = open_world();
        let a = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let b = w.spawn(1, uid::ZERGLING, 380, 320).unwrap();
        let c = w.spawn(0, uid::MARINE, 700, 700).unwrap();
        let mut ids = w.units_in_rect((350, 320), 64);
        ids.sort_unstable();
        let mut want = vec![w.units[a].id, w.units[b].id];
        want.sort_unstable();
        assert_eq!(ids, want);
        assert!(!ids.contains(&w.units[c].id));
        let ids = w.units_in_rect((700, 700), 8);
        assert_eq!(ids, vec![w.units[c].id]);
    }

    #[test]
    fn units_near_point_uses_the_32px_half_extent() {
        let mut w = open_world();
        let a = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let b = w.spawn(0, uid::MARINE, 360, 320).unwrap();
        let c = w.spawn(0, uid::MARINE, 380, 320).unwrap();
        let mut ids = w.units_near_point((331, 320));
        ids.sort_unstable();
        let mut want = vec![w.units[a].id, w.units[b].id];
        want.sort_unstable();
        assert_eq!(ids, want);
        assert!(!ids.contains(&w.units[c].id));
    }

    #[test]
    fn destroyed_nuclear_silo_drops_its_missile_and_parent_link() {
        let mut w = open_world();
        let cc = w.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
        let silo = w.spawn(0, uid::NUCLEAR_SILO, 352, 320).unwrap();
        w.units[cc].addon = Some(w.units[silo].id);
        w.units[silo].addon_attached = true;
        w.units[silo].nuke_ready = true;
        w.kill_unit(silo);
        assert!(!w.units[silo].nuke_ready);
        assert_eq!(w.units[cc].addon, None);
    }

    #[test]
    fn field_091_clears_like_the_retail_initializer() {
        let mut w = open_world();
        let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        w.units[u].field_091 = 9;
        w.units[u].clear_field_091();
        assert_eq!(w.units[u].field_091, 0);
    }

    #[test]
    fn owner_predicate_matches_the_retail_assertion() {
        let mut w = open_world();
        let u = w.spawn(1, uid::MARINE, 320, 320).unwrap();
        assert!(w.units[u].owner_is(1));
        assert!(!w.units[u].owner_is(0));
    }
}

mod translated;
pub use translated::*;
