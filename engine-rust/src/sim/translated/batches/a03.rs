#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent03.rs
// ===========================================================================
// Translation batch 03.  Only this file is edited by the batch.
//
// Ten recovered routines from the `CUnit` method block
// `0x005b0000..=0x005b2fff`.
//
// Retail keeps every `CUnit` on one of several intrusive doubly-linked
// lists.  New units are inserted at the *head* (`FUN_005b0fe0` tail-links
// `CUnit+0x6c`/`+0x68` and stores the record in `0x00fa36bc[owner]`;
// `FUN_00598297` shows the same insert-at-head splice), and a dying unit is
// unlinked from its lists before it joins the corpse list (`FUN_005b083a`).
// [`World::units`] is kept in spawn order and never removes records, so a
// head query is the last matching element and a tail query the first.
// Dead units stay in the vector but are no longer in any retail list, which
// is why every accessor below filters on [`SimUnit::alive`].

/// Unit ids kept on the retail temporary-unit list (`0x00fa36b8`).
///
/// `FUN_005b0086` moves a unit from the normal list to this list when
/// `FUN_005b6270` (type `0x21`, Scanner Sweep) or `FUN_005b6220` (type `0x65`,
/// Map Revealer) matches.
const A03_TEMPORARY_UNIT_TYPES: [u16; 2] = [0x21, 0x65];

/// A live unit whose sprite is not currently drawn belongs to the
/// sprite-hidden list: it is loaded in a transport or inside a Bunker, so
/// the renderer and the target scans skip it until it is unloaded.
fn a03_sprite_hidden(unit: &SimUnit) -> bool {
    unit.in_transport.is_some() || unit.in_bunker
}

impl World {
    /// `0x005b0d30`: head of the per-player unit list (`0x00fa36bc[owner]`).
    ///
    /// The constructor links each new unit at the head of its owner's list
    /// (`FUN_005b0fe0` writes `CUnit+0x6c = head`, points the old head back at
    /// the new record and replaces the head), so the head is the most
    /// recently spawned live unit of that player.  The death cleanup
    /// `FUN_005b083a` unlinks the unit, matching the `alive` filter here.
    pub fn a03_first_unit_of_owner(&self, owner: u8) -> Option<u32> {
        self.units
            .iter()
            .rev()
            .find(|unit| unit.alive && unit.owner == owner)
            .map(|unit| unit.id)
    }

    /// `0x005b0d40`: head of the temporary-unit list (`0x00fa36b8`).
    ///
    /// `FUN_005b0086` files Scanner Sweeps (`0x21`) and Map Revealers
    /// (`0x65`) here as they are created; insertion is at the head, so this
    /// is the newest live temporary unit.
    pub fn a03_first_temporary_unit(&self) -> Option<u32> {
        self.units
            .iter()
            .rev()
            .find(|unit| unit.alive && A03_TEMPORARY_UNIT_TYPES.contains(&unit.type_id))
            .map(|unit| unit.id)
    }

    /// `0x005b16c0`: tail of the temporary-unit list (`0x00fa3708`).
    ///
    /// `FUN_005771a0` leaves the tail pointer on the oldest node, so this is
    /// the first temporary unit that is still alive.
    pub fn a03_last_temporary_unit(&self) -> Option<u32> {
        self.units
            .iter()
            .find(|unit| unit.alive && A03_TEMPORARY_UNIT_TYPES.contains(&unit.type_id))
            .map(|unit| unit.id)
    }

    /// `0x005b0d50`: head of the sprite-hidden unit list (`0x00fa36b4`).
    ///
    /// `FUN_005a5f20` unlinks a unit from the visible list (`FUN_005b2f10`) and
    /// re-inserts it here (`FUN_005b16f0`) when its sprite is no longer drawn;
    /// `FUN_005b2f00` removes it again when it is shown.  In the Rust model a
    /// live unit stops rendering while it is loaded into a transport
    /// ([`SimUnit::in_transport`]) or a Bunker ([`SimUnit::in_bunker`]).
    pub fn a03_first_hidden_unit(&self) -> Option<u32> {
        self.units
            .iter()
            .rev()
            .find(|unit| unit.alive && a03_sprite_hidden(unit))
            .map(|unit| unit.id)
    }

    /// `0x005b16d0`: tail of the sprite-hidden unit list (`0x00fa3704`).
    pub fn a03_last_hidden_unit(&self) -> Option<u32> {
        self.units
            .iter()
            .find(|unit| unit.alive && a03_sprite_hidden(unit))
            .map(|unit| unit.id)
    }

    /// `0x005b0d70`: head of the visible unit list (`0x00fa36a4`).
    ///
    /// `FUN_005c51f0` shows a hidden unit by unlinking it from the hidden list
    /// (`FUN_005b2f00`) and inserting it here (`FUN_005b1700`).  The renderer and
    /// the unit searches (`FUN_00769880`, `FUN_0076a330`, `FUN_00769920`) walk
    /// this list, so it holds every live unit that is currently drawn.
    pub fn a03_first_visible_unit(&self) -> Option<u32> {
        self.units
            .iter()
            .rev()
            .find(|unit| unit.alive && !a03_sprite_hidden(unit))
            .map(|unit| unit.id)
    }

    /// `0x005b16e0`: tail of the visible unit list (`0x00fa36fc`).
    pub fn a03_last_visible_unit(&self) -> Option<u32> {
        self.units
            .iter()
            .find(|unit| unit.alive && !a03_sprite_hidden(unit))
            .map(|unit| unit.id)
    }

    /// `0x005b2ef0`: byte table lookup by unit type.
    ///
    /// The table at `0x00f92c78` is filled once at startup by `FUN_005b0d80`
    /// and never changes.  It is non-zero for the types whose sprite draw at
    /// `FUN_00794400` uses the damage-state frame path (buildings, resources
    /// and organic units), and zero for types `>= 0xe4`.  The exact id set
    /// below is the one `FUN_005b0d80` writes.
    pub fn a03_unit_damage_frame_flag(&self, type_id: u16) -> u8 {
        u8::from(matches!(
            type_id,
            0x23..=0x39
                | 0x3b
                | 0x3e
                | 0x3f
                | 0x44
                | 0x4c
                | 0x59..=0x61
                | 0x67
                | 0x82..=0x98
                | 0xad
                | 0xb0..=0xb2
                | 0xbb
                | 0xbc
                | 0xc2
                | 0xc5
                | 0xc9
                | 0xd8
                | 0xdc
                | 0xe2
                | 0xe3
        ))
    }
}

impl SimUnit {
    /// `0x005b1621`: initialise the cached build time before construction
    /// bookkeeping.
    ///
    /// Retail reads the `units.dat` build-time column through `FUN_0058df30`
    /// (the 456-byte table loaded at `0x00f91c58`; the UI divides it by 15
    /// to print whole seconds at `FUN_00659270`) and writes it to
    /// `CUnit+0xac`.  The getter `FUN_005b49e0` exposes that word, which the
    /// Rust model already has as [`SimUnit::field_0ac`].  The companion
    /// fields are *derived* construction rates and are recomputed by the
    /// production code rather than stored: `FUN_005b3910` fills
    /// `CUnit+0xa8 = (hp - hp/10 - 1 + build_time) / build_time` and
    /// `FUN_005b3980` does the shield analogue at `CUnit+0xaa`; the engine's
    /// `update_construction` computes the same `(max - max/10)/build_total`
    /// gain.  A zero build time is clamped to one, exactly like retail.
    pub fn a03_init_build_time_field(&mut self, data: &GameData, type_id: u16) {
        self.field_0ac = data.unit(type_id).build_time.max(1);
    }

    /// `0x005b0ba9`: finish construction of this record.
    ///
    /// Retail copies the type's maximum hit points out of the table at
    /// `0x00f8eab0` into `CUnit+0x8` (current HP, 1/256 points) and sets
    /// `CUnit+0xdc` bit 0, the "ready" bit cleared again by the removal
    /// paths `FUN_005b083a`/`FUN_005b0b4b` and tested by the order validator
    /// `FUN_00534700`.  In the Rust model that ready state is
    /// [`SimUnit::completed`], so the whole-point HP and the completion flag
    /// are updated together.
    pub fn a03_finish_construction(&mut self, data: &GameData, type_id: u16) {
        self.hp = data.unit(type_id).hit_points_raw as i32;
        self.completed = true;
    }
}

#[cfg(test)]
mod tests_agent03 {
    use super::*;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(crate::data::GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    /// `0x005b0d30`: the per-player head is the newest live unit of the
    /// player, and dying removes the unit from that view.
    #[test]
    fn player_list_head_is_the_newest_live_unit() {
        let mut w = world();
        let first = w.spawn(0, uid::MARINE, 10, 10).unwrap();
        let second = w.spawn(0, uid::MARINE, 20, 10).unwrap();
        let other = w.spawn(1, uid::MARINE, 30, 10).unwrap();

        assert_eq!(w.a03_first_unit_of_owner(0), Some(w.units[second].id));
        assert_eq!(w.a03_first_unit_of_owner(1), Some(w.units[other].id));
        assert_eq!(w.a03_first_unit_of_owner(2), None);

        w.units[second].alive = false;
        assert_eq!(w.a03_first_unit_of_owner(0), Some(w.units[first].id));
    }

    /// `0x005b0d40`/`0x005b16c0`: the temporary list keeps Scanner Sweeps
    /// and Map Revealers, newest at the head, oldest at the tail.
    #[test]
    fn temporary_unit_list_ends() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 10, 10).unwrap();
        let sweep = w.spawn(0, uid::SCANNER_SWEEP, 20, 10).unwrap();
        let revealer = w.spawn(0, 0x65, 30, 10).unwrap();

        assert_eq!(w.a03_first_temporary_unit(), Some(w.units[revealer].id));
        assert_eq!(w.a03_last_temporary_unit(), Some(w.units[sweep].id));
        assert_ne!(w.a03_first_temporary_unit(), Some(w.units[marine].id));

        w.units[revealer].alive = false;
        assert_eq!(w.a03_first_temporary_unit(), Some(w.units[sweep].id));
        assert_eq!(w.a03_last_temporary_unit(), Some(w.units[sweep].id));

        w.units[sweep].alive = false;
        assert_eq!(w.a03_first_temporary_unit(), None);
        assert_eq!(w.a03_last_temporary_unit(), None);
    }

    /// `0x005b0d50`/`0x005b16d0`/`0x005b0d70`/`0x005b16e0`: loaded units
    /// leave the visible list and join the hidden one; unloading and death
    /// are reflected at both ends.
    #[test]
    fn hidden_and_visible_unit_lists_follow_loading_state() {
        let mut w = world();
        let first = w.spawn(0, uid::MARINE, 10, 10).unwrap();
        let transport = w.spawn(0, uid::DROPSHIP, 40, 10).unwrap();
        let second = w.spawn(0, uid::MARINE, 20, 10).unwrap();

        assert_eq!(w.a03_first_visible_unit(), Some(w.units[second].id));
        assert_eq!(w.a03_last_visible_unit(), Some(w.units[first].id));
        assert_eq!(w.a03_first_hidden_unit(), None);
        assert_eq!(w.a03_last_hidden_unit(), None);

        w.units[second].in_transport = Some(w.units[transport].id);
        assert_eq!(w.a03_first_hidden_unit(), Some(w.units[second].id));
        assert_eq!(w.a03_last_hidden_unit(), Some(w.units[second].id));
        assert_eq!(w.a03_first_visible_unit(), Some(w.units[transport].id));
        assert_eq!(w.a03_last_visible_unit(), Some(w.units[first].id));

        w.units[first].in_bunker = true;
        assert_eq!(w.a03_last_hidden_unit(), Some(w.units[first].id));

        w.units[first].alive = false;
        assert_eq!(w.a03_last_hidden_unit(), Some(w.units[second].id));
        assert_eq!(w.a03_first_visible_unit(), Some(w.units[transport].id));
    }

    /// `0x005b2ef0`: the table built by `FUN_005b0d80` returns one for the
    /// damage-frame types and zero for everything else, including ids at or
    /// above `0xe4`.
    #[test]
    fn damage_frame_table_matches_recovered_ids() {
        let w = world();
        for type_id in [
            0x23, 0x39, 0x3b, 0x3e, 0x3f, 0x44, 0x4c, 0x59, 0x61, 0x67, 0x82, 0x98, 0xad, 0xb0,
            0xb2, 0xbb, 0xbc, 0xc2, 0xc5, 0xc9, 0xd8, 0xdc, 0xe2, 0xe3,
        ] {
            assert_eq!(w.a03_unit_damage_frame_flag(type_id), 1, "{type_id:#x}");
        }
        for type_id in [
            0x00, 0x01, 0x22, 0x3a, 0x3c, 0x3d, 0x40, 0x43, 0x4d, 0x58, 0x62, 0x66, 0x68, 0x81,
            0x99, 0xac, 0xae, 0xaf, 0xb3, 0xba, 0xbd, 0xc1, 0xc3, 0xc4, 0xc6, 0xc8, 0xca, 0xd7,
            0xd9, 0xdb, 0xdd, 0xe1, 0xe4, 0xff,
        ] {
            assert_eq!(w.a03_unit_damage_frame_flag(type_id), 0, "{type_id:#x}");
        }
    }

    /// `0x005b1621`: `CUnit+0xac` receives the `units.dat` build time, with
    /// the retail clamp to one for a zero row.
    #[test]
    fn build_time_field_is_cached_with_clamp() {
        let mut w = world();
        let barracks = w.spawn(0, uid::BARRACKS, 20, 20).unwrap();
        let build_time = w.data.unit(uid::BARRACKS).build_time;
        assert!(build_time > 1);
        w.units[barracks].field_0ac = 0;
        w.units[barracks].a03_init_build_time_field(&w.data, uid::BARRACKS);
        assert_eq!(w.units[barracks].field_0ac, build_time);

        // Type 150 has a zero build-time row in `units.dat`; retail stores 1.
        let zero = w.spawn(0, 150, 40, 20).unwrap();
        assert_eq!(w.data.unit(150).build_time, 0);
        w.units[zero].a03_init_build_time_field(&w.data, 150);
        assert_eq!(w.units[zero].field_0ac, 1);
    }

    /// `0x005b0ba9`: finishing construction restores full hit points and
    /// marks the record ready.
    #[test]
    fn finish_construction_restores_full_hp() {
        let mut w = world();
        let cc = w.spawn(0, uid::COMMAND_CENTER, 32, 32).unwrap();
        w.units[cc].completed = false;
        w.units[cc].hp = 1;

        w.units[cc].a03_finish_construction(&w.data, uid::COMMAND_CENTER);
        assert!(w.units[cc].completed);
        assert_eq!(
            w.units[cc].hp,
            w.data.unit(uid::COMMAND_CENTER).hit_points_raw as i32
        );
    }
}
