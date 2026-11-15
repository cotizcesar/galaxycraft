//! Free-for-all rules: the retail melee trigger (`fixtures/melee.trg`,
//! "Command at most 0 Buildings" -> Defeat) and the skirmish AI's target.

use galaxycraft::data::GameData;
use galaxycraft::ids::unit as uid;
use galaxycraft::map::Terrain;
use galaxycraft::sim::World;
use std::sync::Arc;

fn world(players: usize) -> World {
    World::new(
        Arc::new(GameData::bundled().unwrap()),
        Terrain::open(128, 128),
        players,
    )
}

#[test]
fn a_player_left_without_buildings_is_defeated() {
    let mut w = world(2);
    let a = w.spawn_px(0, uid::COMMAND_CENTER, 400, 400).unwrap();
    let b = w.spawn_px(1, uid::COMMAND_CENTER, 3000, 3000).unwrap();
    w.units[a].completed = true;
    w.units[b].completed = true;
    // Player 1 keeps a Marine but loses its only building.
    w.spawn_px(1, uid::MARINE, 2800, 3000).unwrap();
    let id = w.units[b].id;
    let hp = w.units[b].hp + 1;
    w.damage_unit(id, hp).unwrap();
    for _ in 0..5 {
        w.step();
    }
    assert!(w.players[1].defeated, "no buildings left: defeated");
    assert_eq!(w.winner, Some(0));
}

#[test]
fn the_skirmish_ai_targets_the_nearest_enemy_town_hall() {
    let mut w = world(3);
    for (p, x) in [(0, 400), (1, 1200), (2, 3600)] {
        let h = w.spawn_px(p, uid::COMMAND_CENTER, x, 400).unwrap();
        w.units[h].completed = true;
    }
    assert_eq!(w.nearest_enemy_town_hall(2), Some((1200, 400)));
    assert_eq!(w.nearest_enemy_town_hall(0), Some((1200, 400)));
}

#[test]
fn an_eight_player_free_for_all_ends_with_the_last_player_standing() {
    let mut w = world(8);
    let mut halls = Vec::new();
    for p in 0..8u8 {
        let x = 300 + (p as i32 % 4) * 900;
        let y = 400 + (p as i32 / 4) * 2400;
        let i = w.spawn_px(p as usize, uid::COMMAND_CENTER, x, y).unwrap();
        w.units[i].completed = true;
        halls.push(w.units[i].id);
    }
    // Players 1..=7 lose their only building one after the other.
    for p in 1..8u8 {
        assert_eq!(w.winner, None, "{} players left", 9 - p);
        let id = halls[p as usize];
        let hp = w.unit(id).unwrap().hp + 1;
        w.damage_unit(id, hp).unwrap();
        for _ in 0..5 {
            w.step();
        }
        assert!(w.players[p as usize].defeated, "player {p} defeated");
    }
    assert_eq!(w.winner, Some(0));
}

#[test]
fn a_player_whose_last_building_changes_owner_is_defeated() {
    // An infested Command Center changes owner without dying, so
    // `buildings_lost` stays 0; the player must still fall (retail melee:
    // "commands at most 0 buildings").
    let mut w = world(2);
    let a = w.spawn_px(0, uid::HATCHERY, 400, 400).unwrap();
    let b = w.spawn_px(1, uid::COMMAND_CENTER, 3000, 3000).unwrap();
    w.units[a].completed = true;
    w.units[b].completed = true;
    w.spawn_px(1, uid::MARINE, 2800, 3000).unwrap();
    w.step();
    w.units[b].owner = 0;
    w.units[b].type_id = uid::INFESTED_COMMAND_CENTER;
    for _ in 0..5 {
        w.step();
    }
    assert_eq!(w.players[1].buildings_lost, 0);
    assert!(w.players[1].defeated, "no building left: defeated");
    assert_eq!(w.winner, Some(0));
}
