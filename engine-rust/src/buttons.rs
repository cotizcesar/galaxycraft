//! Command card of a unit: the retail button set ([`crate::button_sets`])
//! filtered by its condition functions.
//!
//! The condition addresses are classified by how the table uses them
//! (INFERENCE, 2026-11-03; each line names the evidence).  Addresses not
//! classified below are shown enabled, the conservative choice for a
//! button that exists in the set.  Falsifier for any line: the retail card
//! of a unit in that state showing/hiding the button differently.

use crate::button_sets::{Button, set_of};
use crate::sim::{SimUnit, World};

/// Semantic class of a retail button condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cond {
    /// `0x65f590`: Move/Stop/Attack/Patrol/Hold of mobile units (318 uses).
    Always,
    /// `0x780480`: make the unit `cond_var` (train/build/morph; 87 uses,
    /// all with `cond_var == act_var` = a unit id); greyed while its
    /// requirements are unmet (Lair before a Spawning Pool, 134905).
    MakeUnit,
    /// `0x780690`: Set Rally Point of producing buildings.
    Rally,
    /// Cancel buttons (icon 236): only while something is in progress.
    Cancel,
    /// `0x780820`: research tech `cond_var` (Burrowing at the Hatchery).
    Research,
    /// `0x780890`: upgrade `cond_var` at its building.
    Upgrade,
    /// `0x780830`: use tech `cond_var` (Stim Packs on Marines).
    UseTech,
    /// `0x780b50`: Move/Stop of a lifted Terran building.
    WhenLifted,
    /// `0x780b70`: Land (icon 283).
    Land,
    /// `0x780ba0`: Lift Off (icon 282).
    LiftOff,
    /// `0x780be0`: Burrow (icon 259, tech 11) of an unburrowed unit.
    Burrow,
    /// `0x780c50`: Unburrow (icon 260).
    Unburrow,
    /// `0x781320`: the SCV's Return Cargo (icon 233), only while it carries
    /// minerals or gas.  FACT (Boot Camp recording, 95 s / 110 s): slot 6
    /// shows while the SCV carries and slot 5 (Gather) is empty; an empty
    /// SCV shows Gather and no Return Cargo.
    Carrying,
    /// `0x7812d0`: Gather (icon 231), only while the worker carries nothing.
    NotCarrying,
    /// `0x781440`: Build Advanced Structures, only when one of card 242's
    /// buildings has its requirements (hidden all through Boot Camp, which
    /// has no Barracks).
    AdvancedBuild,
    /// `0x7807b0`: Load, while the transport/Bunker has cargo space.
    Load,
    /// `0x7807f0`: Unload, while the transport/Bunker carries units.
    Unload,
    /// `0x7804d0`: build one Interceptor / Scarab.
    Hangar,
    /// `0x780b10`: Nuclear Strike, while a completed Nuclear Silo is
    /// attached to a Command Center.
    Nuke,
    /// `0x7808a0` / `0x780960`: Cloak, while the tech is researched and
    /// the unit is visible.
    Cloak,
    /// `0x780900` / `0x7809d0`: Decloak, while the unit is cloaked.
    Decloak,
    /// `0x781030` / `0x781050`: Archon Warp (two High Templars).
    MergeArchon,
    /// `0x781090`: Dark Archon Meld (two Dark Templars).
    MergeDarkArchon,
    Unclassified,
}

impl Cond {
    pub fn of(addr: u32) -> Cond {
        match addr {
            0x65f590 => Cond::Always,
            0x780480 => Cond::MakeUnit,
            0x780690 | 0x7806a0 => Cond::Rally,
            0x780620 | 0x780630 | 0x780640 | 0x7806f0 | 0x780710 | 0x780720 | 0x781270 => {
                Cond::Cancel
            }
            0x780820 => Cond::Research,
            0x780890 => Cond::Upgrade,
            // 0x780840: the Vulture's Spider Mines (tech 3); the hero
            // Vulture's copy uses 0x780830.  Both: researched, or a hero.
            0x780830 | 0x780840 => Cond::UseTech,
            0x780b50 => Cond::WhenLifted,
            0x780b70 => Cond::Land,
            0x780ba0 => Cond::LiftOff,
            0x780be0 => Cond::Burrow,
            0x780c50 => Cond::Unburrow,
            0x781320 => Cond::Carrying,
            0x7812d0 => Cond::NotCarrying,
            0x781440 => Cond::AdvancedBuild,
            0x7807b0 => Cond::Load,
            0x7807f0 => Cond::Unload,
            0x7804d0 => Cond::Hangar,
            0x780b10 => Cond::Nuke,
            0x7808a0 | 0x780960 => Cond::Cloak,
            0x780900 | 0x7809d0 => Cond::Decloak,
            0x781030 | 0x781050 => Cond::MergeArchon,
            0x781090 => Cond::MergeDarkArchon,
            _ => Cond::Unclassified,
        }
    }
}

/// Semantic class of a retail button action (`act`).  FACT: classified
/// from the 1.23.10.13515 table (`sc_dump --buttons`) by the icon and the
/// `stat_txt` tooltip each address is used with (2026-11-05): e.g.
/// `0x74d4c0` is the 94 Move buttons (icon 228, hotkey `m`), `0x74d520` the
/// Terran "Build ..." buttons of card 239, `0x784d50` the card switches
/// (`act_var` 239 basic / 242 advanced build card, 228 back to the unit's
/// own card).  Falsifier: a retail button whose effect differs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Move,
    Stop,
    Attack,
    Patrol,
    Hold,
    Repair,
    Gather,
    ReturnCargo,
    /// Show the card `act_var` (228 = the unit's own card).
    Card,
    /// Place the building `act_var` (Terran SCV, Protoss Probe).
    Build,
    /// Drone mutation into the building `act_var`.
    DroneBuild,
    Train,
    Addon,
    Rally,
    StimPacks,
    Research,
    Upgrade,
    LiftOff,
    Land,
    CancelLast,
    CancelAddon,
    CancelConstruction,
    CancelMorph,
    /// Esc of the target / placement cards (229, 230, 235, 237).
    CancelTarget,
    Siege,
    Unsiege,
    /// Spell with a target (`act_var` tech).
    TargetTech,
    MorphUnit,
    MorphBuilding,
    Burrow,
    Unburrow,
    Cloak,
    Decloak,
    /// `0x74d850`: Stop for ordinary units, Archon Warp / Dark Archon Meld
    /// when the button's `act_var` names the merge tech (23 / 28).
    StopOrMerge,
    /// `0x74d870`: merge two templars into an Archon.
    MergeArchon,
    /// `0x74d880`: merge two templars into a Dark Archon.
    MergeDarkArchon,
    /// `0x74d3b0`: load nearby units into the selected transport/bunker.
    Load,
    /// `0x74dba0`: unload the selected transport/bunker.
    Unload,
    /// `0x74d4d0`: Ghost nuclear strike.
    Nuke,
    /// `0x74d410`: Medic Heal.
    Heal,
    /// `0x74d4e0`: Queen Infestation.
    Infest,
    /// `0x74d680`: Hatchery / Lair / Hive Spawn Larva.
    SpawnLarva,
    /// `0x74d630`: Shield Battery recharge.
    RechargeShields,
    /// `0x74d260`: build an Interceptor (Carrier) or Scarab (Reaver).
    TrainHangar,
    /// `0x74d5a0`: flag-beacon placement (campaign map units).
    PlaceBeacon,
    /// The ATS/ATA laser battery test units (`0x75c2c0`, `0x75c370`,
    /// `0x75c420`); no player-facing rule in the reconstruction.
    Special,
    Unclassified,
}

impl Act {
    pub fn of(addr: u32) -> Act {
        match addr {
            0x74d1d0 | 0x74d230 | 0x74d2c0 | 0x74d620 | 0x74db80 => Act::Attack,
            0x74d4c0 | 0x74d2d0 => Act::Move,
            0x74d2e0 | 0x74db90 => Act::Stop,
            0x74d500 => Act::Patrol,
            0x74d430 => Act::Hold,
            0x74d640 => Act::Repair,
            0x74d400 => Act::Gather,
            0x74d650 => Act::ReturnCargo,
            0x784d50 => Act::Card,
            0x74d520 | 0x74d5d0 => Act::Build,
            0x74dc20 => Act::DroneBuild,
            0x74d250 => Act::Train,
            0x74d260 => Act::TrainHangar,
            0x74d560 => Act::Addon,
            0x74d610 => Act::Rally,
            0x74d790 => Act::StimPacks,
            0x74d670 => Act::Research,
            0x74dbd0 => Act::Upgrade,
            0x74d240 => Act::LiftOff,
            0x74d460 => Act::Land,
            0x74d860 => Act::CancelLast,
            0x74d280 | 0x74d2a0 => Act::CancelAddon,
            0x74d290 => Act::CancelMorph,
            0x74d440 => Act::CancelConstruction,
            0x74d450 => Act::CancelMorph,
            0x6cbec0 | 0x7814e0 | 0x74d2b0 | 0x533f30 => Act::CancelTarget,
            0x74d780 => Act::Siege,
            0x74d890 => Act::Unsiege,
            0x74d8a0 => Act::TargetTech,
            0x74d480 => Act::MorphUnit,
            0x74dbe0 => Act::MorphBuilding,
            0x74d270 => Act::Burrow,
            0x74d390 => Act::Unburrow,
            0x74d2f0 => Act::Cloak,
            0x74d3a0 => Act::Decloak,
            0x74d850 => Act::StopOrMerge,
            0x74d870 => Act::MergeArchon,
            0x74d880 => Act::MergeDarkArchon,
            0x74d3b0 => Act::Load,
            0x74dba0 => Act::Unload,
            0x74d4d0 => Act::Nuke,
            0x74d410 => Act::Heal,
            0x74d4e0 => Act::Infest,
            0x74d680 => Act::SpawnLarva,
            0x74d630 => Act::RechargeShields,
            0x74d5a0 => Act::PlaceBeacon,
            0x75c2c0 | 0x75c370 | 0x75c420 => Act::Special,
            _ => Act::Unclassified,
        }
    }

    /// Page of the controller command wheel this action belongs to.  The
    /// wheel mirrors the retail command card, so the classification only
    /// groups the same buttons; it never invents an order.
    pub fn wheel_category(self) -> WheelCategory {
        match self {
            Act::Build | Act::DroneBuild | Act::Addon => WheelCategory::Build,
            Act::Train | Act::TrainHangar | Act::MorphUnit => WheelCategory::Train,
            Act::Research
            | Act::Upgrade
            | Act::StimPacks
            | Act::TargetTech
            | Act::MorphBuilding => WheelCategory::Research,
            Act::Stop
            | Act::StopOrMerge
            | Act::Hold
            | Act::Patrol
            | Act::Burrow
            | Act::Unburrow
            | Act::Siege
            | Act::Unsiege
            | Act::Cloak
            | Act::Decloak => WheelCategory::Stances,
            _ => WheelCategory::Commands,
        }
    }
}

/// Context pages of the controller command wheel, in display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WheelCategory {
    /// Move, attack, repair, gather, rally, transport, cancel, ...
    Commands,
    /// Buildings available from the worker's retail build cards.
    Build,
    /// Unit production and morphs.
    Train,
    /// Research, upgrades and targeted tech.
    Research,
    /// Stop / hold / patrol / siege / burrow / cloak.
    Stances,
}

impl WheelCategory {
    /// Every page, in display order.
    pub const ALL: [WheelCategory; 5] = [
        WheelCategory::Commands,
        WheelCategory::Build,
        WheelCategory::Train,
        WheelCategory::Research,
        WheelCategory::Stances,
    ];

    /// Heading drawn in the middle of the wheel.
    pub fn title(self) -> &'static str {
        match self {
            WheelCategory::Commands => "COMMANDS",
            WheelCategory::Build => "BUILD",
            WheelCategory::Train => "TRAIN",
            WheelCategory::Research => "RESEARCH / UPGRADES",
            WheelCategory::Stances => "STANCES",
        }
    }
}

/// Card shown for `unit`: its own set, or the set a [`Act::Card`] button
/// switched to (`set` 228 or `None` = the unit's own card).
pub fn card_of_set(
    world: &World,
    unit: &SimUnit,
    set: Option<u16>,
) -> [Option<(Button, ButtonState)>; 9] {
    let index = match set {
        Some(s) if s != 228 => s as usize,
        _ => unit.type_id as usize,
    };
    let mut out = [None; 9];
    for b in set_of(index) {
        let i = (b.pos as usize).wrapping_sub(1);
        if i < 9
            && out[i].is_none()
            && let Some(s) = state(world, unit, b)
        {
            out[i] = Some((*b, s));
        }
    }
    out
}

/// How a card slot is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonState {
    Enabled,
    /// Shown greyed (its disabled tooltip applies).
    Disabled,
}

/// Visibility of one button for `unit` (`None` = hidden).
pub fn state(world: &World, unit: &SimUnit, b: &Button) -> Option<ButtonState> {
    let player = world.players.get(unit.owner as usize)?;
    let busy = !unit.completed
        || !unit.production.is_empty()
        || unit.morph_to.is_some()
        || unit
            .addon
            .and_then(|id| world.unit(id))
            .is_some_and(|a| !a.completed);
    let on = |shown: bool| shown.then_some(ButtonState::Enabled);
    match Cond::of(b.cond) {
        Cond::Always | Cond::Rally | Cond::Upgrade | Cond::Unclassified => on(true),
        Cond::MakeUnit => Some(if world.has_requirements(unit.owner, b.cond_var) {
            ButtonState::Enabled
        } else {
            ButtonState::Disabled
        }),
        Cond::Cancel => on(busy),
        Cond::Research => on(!player.has_tech(b.cond_var as u8)),
        Cond::UseTech => on(
            player.has_tech(b.cond_var as u8) || world.data.unit(unit.type_id).flags & 0x40 != 0
        ),
        Cond::WhenLifted | Cond::Land => on(unit.lifted),
        Cond::LiftOff => on(!unit.lifted),
        Cond::Burrow => on(player.has_tech(b.cond_var as u8) && !unit.status.burrowed),
        Cond::Unburrow => on(unit.status.burrowed),
        Cond::Carrying => on(unit.carry > 0),
        Cond::NotCarrying => on(unit.carry == 0),
        Cond::AdvancedBuild => on(set_of(242).iter().any(|a| {
            Cond::of(a.cond) == Cond::MakeUnit && world.has_requirements(unit.owner, a.cond_var)
        })),
        Cond::Load => {
            let used: i32 = unit
                .cargo
                .iter()
                .filter_map(|id| world.unit(*id))
                .map(|u| world.data.unit(u.type_id).space_required as i32)
                .sum();
            on(world.data.unit(unit.type_id).space_provided as i32 > used)
        }
        Cond::Unload => on(!unit.cargo.is_empty()),
        Cond::Hangar => on(true),
        Cond::Nuke => on(world.units.iter().any(|u| {
            u.alive
                && u.completed
                && u.owner == unit.owner
                && u.type_id == crate::ids::unit::NUCLEAR_SILO
                && u.addon_attached
                // Only with a missile built and waiting in the silo.
                && u.nuke_ready
        })),
        Cond::Cloak => on(!unit.status.cloaked
            && (player.has_tech(b.cond_var as u8)
                || world.data.unit(unit.type_id).flags & 0x40 != 0)),
        Cond::Decloak => on(unit.status.cloaked),
        Cond::MergeArchon => on(unit.type_id == crate::ids::unit::HIGH_TEMPLAR),
        Cond::MergeDarkArchon => on(unit.type_id == crate::ids::unit::DARK_TEMPLAR),
    }
}

/// The nine card slots (position 1..=9 -> index 0..=8): the first visible
/// button of each position, as retail stacks alternatives (Lift Off /
/// Land / Cancel all live in position 9 of the Command Center).
pub fn card(world: &World, unit: &SimUnit) -> [Option<(Button, ButtonState)>; 9] {
    card_of_set(world, unit, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::{Race, World};
    use std::sync::Arc;

    fn world_with(race: Race, town: u16) -> (World, usize) {
        let data = Arc::new(crate::data::GameData::bundled().expect("tables"));
        let mut w = World::new(data, Terrain::open(64, 64), 2);
        w.players[0].race = race;
        let i = w.spawn_px(0, town, 512, 512).expect("spawn");
        w.units[i].completed = true;
        (w, i)
    }

    #[test]
    fn act_addresses_follow_the_retail_table() {
        // `sc_dump --buttons` on 1.23.10.13515 (SCV card set 7, CC set 106,
        // Terran build card 239).
        assert_eq!(Act::of(0x74d4c0), Act::Move);
        assert_eq!(Act::of(0x74d850), Act::StopOrMerge);
        assert_eq!(Act::of(0x74d1d0), Act::Attack);
        assert_eq!(Act::of(0x74d640), Act::Repair);
        assert_eq!(Act::of(0x74d400), Act::Gather);
        assert_eq!(Act::of(0x74d650), Act::ReturnCargo);
        assert_eq!(Act::of(0x784d50), Act::Card);
        assert_eq!(Act::of(0x74d520), Act::Build);
        assert_eq!(Act::of(0x74d250), Act::Train);
        assert_eq!(Act::of(0x74d610), Act::Rally);
        assert_eq!(Act::of(0x74d240), Act::LiftOff);
        assert_eq!(Act::of(0x12345), Act::Unclassified);
    }

    #[test]
    fn gather_and_return_cargo_swap_with_the_load() {
        // Boot Camp recording: an empty SCV shows Gather in slot 5, a loaded
        // one Return Cargo in slot 6, never both.
        let (mut w, i) = world_with(Race::Terran, uid::COMMAND_CENTER);
        let scv = w.spawn_px(0, uid::SCV, 600, 600).expect("scv");
        let _ = i;
        let gather = Button {
            pos: 5,
            icon: 231,
            cond: 0x7812d0,
            act: 0x74d400,
            cond_var: 0,
            act_var: 0,
            enabled_string: 0,
            disabled_string: 0,
        };
        let ret = Button {
            pos: 6,
            icon: 233,
            cond: 0x781320,
            act: 0x74d650,
            ..gather
        };
        assert!(state(&w, &w.units[scv], &gather).is_some());
        assert!(state(&w, &w.units[scv], &ret).is_none());
        w.units[scv].carry = 8;
        assert!(state(&w, &w.units[scv], &gather).is_none());
        assert!(state(&w, &w.units[scv], &ret).is_some());
    }

    #[test]
    fn nexus_at_game_start_shows_probe_and_rally_only() {
        // 135051: Probe in slot 1, Set Rally Point in slot 6, nothing else.
        let (w, i) = world_with(Race::Protoss, uid::NEXUS);
        let c = card(&w, &w.units[i]);
        let shown: Vec<(usize, u16, ButtonState)> = c
            .iter()
            .enumerate()
            .filter_map(|(k, s)| s.map(|(b, st)| (k, b.icon, st)))
            .collect();
        assert_eq!(
            shown,
            vec![
                (0, 64, ButtonState::Enabled),
                (5, 286, ButtonState::Enabled)
            ]
        );
    }

    #[test]
    fn hatchery_greys_lair_until_a_spawning_pool_exists() {
        // 134905: Larva, Rally, Burrow research on top; Lair greyed in slot 7.
        let (mut w, i) = world_with(Race::Zerg, uid::HATCHERY);
        let c = card(&w, &w.units[i]);
        let icons: Vec<Option<u16>> = c.iter().map(|s| s.map(|(b, _)| b.icon)).collect();
        assert_eq!(icons[0..3], [Some(35), Some(286), Some(259)]);
        assert_eq!(
            c[6].map(|(b, s)| (b.icon, s)),
            Some((132, ButtonState::Disabled))
        );
        assert!(c[8].is_none(), "no cancel while idle");
        let pool = w.spawn_px(0, uid::SPAWNING_POOL, 900, 900).expect("pool");
        w.units[pool].completed = true;
        let c = card(&w, &w.units[i]);
        assert_eq!(c[6].map(|(_, s)| s), Some(ButtonState::Enabled));
    }

    #[test]
    fn controller_wheel_groups_card_actions_without_changing_them() {
        // Move / attack stay commands; build and train are their own pages;
        // tech and stance buttons keep the same `Act` used by the card.
        assert_eq!(Act::Move.wheel_category(), WheelCategory::Commands);
        assert_eq!(Act::Attack.wheel_category(), WheelCategory::Commands);
        assert_eq!(Act::Gather.wheel_category(), WheelCategory::Commands);
        assert_eq!(Act::Load.wheel_category(), WheelCategory::Commands);
        assert_eq!(Act::Build.wheel_category(), WheelCategory::Build);
        assert_eq!(Act::DroneBuild.wheel_category(), WheelCategory::Build);
        assert_eq!(Act::Addon.wheel_category(), WheelCategory::Build);
        assert_eq!(Act::Train.wheel_category(), WheelCategory::Train);
        assert_eq!(Act::TrainHangar.wheel_category(), WheelCategory::Train);
        assert_eq!(Act::MorphUnit.wheel_category(), WheelCategory::Train);
        assert_eq!(Act::Research.wheel_category(), WheelCategory::Research);
        assert_eq!(Act::Upgrade.wheel_category(), WheelCategory::Research);
        assert_eq!(Act::StimPacks.wheel_category(), WheelCategory::Research);
        assert_eq!(Act::MorphBuilding.wheel_category(), WheelCategory::Research);
        assert_eq!(Act::Stop.wheel_category(), WheelCategory::Stances);
        assert_eq!(Act::Hold.wheel_category(), WheelCategory::Stances);
        assert_eq!(Act::Patrol.wheel_category(), WheelCategory::Stances);
        assert_eq!(Act::Burrow.wheel_category(), WheelCategory::Stances);
        assert_eq!(Act::Siege.wheel_category(), WheelCategory::Stances);
        assert_eq!(Act::Cloak.wheel_category(), WheelCategory::Stances);
        // The five page headings are distinct and in the documented order.
        let titles: Vec<&str> = WheelCategory::ALL.iter().map(|p| p.title()).collect();
        assert_eq!(
            titles,
            vec![
                "COMMANDS",
                "BUILD",
                "TRAIN",
                "RESEARCH / UPGRADES",
                "STANCES"
            ]
        );
    }

    #[test]
    fn every_shipped_button_action_is_classified() {
        // No command-card button may fall into the client's
        // "not implemented yet" fallback.
        for uid in 0..crate::button_sets::slots() {
            for b in set_of(uid) {
                assert_ne!(
                    Act::of(b.act),
                    Act::Unclassified,
                    "unit {uid} icon {} act {:#x}",
                    b.icon,
                    b.act
                );
            }
        }
    }

    #[test]
    fn the_remaining_card_actions_use_their_retail_addresses() {
        // Addresses recovered from the 1.23.10 button table
        // (`fixtures/button_sets.in`); the tooltips and the disassembly in
        // `docs/command-card-re.md` name each one.
        assert_eq!(Act::of(0x74d2d0), Act::Move);
        assert_eq!(Act::of(0x74d2e0), Act::Stop);
        assert_eq!(Act::of(0x74d3b0), Act::Load);
        assert_eq!(Act::of(0x74dba0), Act::Unload);
        assert_eq!(Act::of(0x74d4d0), Act::Nuke);
        assert_eq!(Act::of(0x74d410), Act::Heal);
        assert_eq!(Act::of(0x74d4e0), Act::Infest);
        assert_eq!(Act::of(0x74d680), Act::SpawnLarva);
        assert_eq!(Act::of(0x74d630), Act::RechargeShields);
        assert_eq!(Act::of(0x74d260), Act::TrainHangar);
        assert_eq!(Act::of(0x74d2f0), Act::Cloak);
        assert_eq!(Act::of(0x74d3a0), Act::Decloak);
        assert_eq!(Act::of(0x74dbe0), Act::MorphBuilding);
        assert_eq!(Act::of(0x74d870), Act::MergeArchon);
        assert_eq!(Act::of(0x74d880), Act::MergeDarkArchon);
    }

    #[test]
    fn every_set_uses_card_positions_one_to_nine() {
        for uid in 0..crate::button_sets::slots() {
            for b in set_of(uid) {
                assert!((1..=9).contains(&b.pos), "unit {uid}: pos {}", b.pos);
            }
        }
    }
}
