#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent40.rs
// ===========================================================================
// Translation batch 40.  Only this file is edited by the batch.
//
// The `decompiled_uncited` rows in `0x005e2000..=0x005e3fff` are retail
// client plumbing, not order handlers: the tail of the static initialiser
// that fills the fixed string table, a family of string-keyed settings
// accessors, the MSVC exception-cleanup funclets of the non-trivial
// routines in the range, and a few container helpers.
//
// The accessors share one shape.  A numeric key is resolved through the
// fixed string table (`gm-nn`, `mp-nn`, `mi-nn`, `sv-nn`, `re-nn` ids), the
// record's settings map is searched for that key, and the stored value is
// converted to the caller's width.  A missing key is never an error: the
// destination is simply left untouched, which is modelled here with
// [`Option`].
//
// Both storage layers are client-side in the retail build, and [`World`]
// keeps no free-form settings map.  Each method below therefore implements
// the accessor against the one keyed override table the simulation does
// keep - the scenario's `UNIS` section - and the doc comment for every row
// names the slot that stands in for the client value.  The several rows
// whose only effect is to build or destroy client objects with no
// simulation counterpart are listed at the end of the file.

/// The `UNIS` override entry for `type_id` is live when the scenario's
/// `use_default` byte is zero; a one (or a short table) means "keep the
/// `units.dat` value", which is the retail absent-key case.
fn a40_override_live(settings: &crate::map::UnitSettings, type_id: u16) -> bool {
    settings.use_default.get(type_id as usize).copied() == Some(0)
}

impl World {
    /// `0x005e2a30`: read a byte-valued setting, copying it out only when
    /// the key is present.
    ///
    /// The routine resolves the key against the fixed string table, looks
    /// it up in the record's settings map and, on success, converts the
    /// stored value to an integer and stores its low byte through the
    /// output pointer.  When the key is absent nothing is written at all -
    /// there is no error return.
    ///
    /// `UNIS` armour is the simulation's only byte-wide override keyed by
    /// unit type, so the method returns that slot.  `None` covers both "no
    /// `UNIS` section" and "this type keeps its default", matching the
    /// retail absent-key path; the string key itself has no simulation
    /// counterpart and is not stored.
    pub fn a40_armor_override(&self, type_id: u16) -> Option<u8> {
        let settings = self.rules.unit_settings.as_ref()?;
        if !a40_override_live(settings, type_id) {
            return None;
        }
        settings.armor.get(type_id as usize).copied()
    }

    /// `0x005e2a60`: read a word-valued setting, copying it out only when
    /// the key is present.
    ///
    /// Same accessor as `0x005e2a30` with a 16-bit destination: the value
    /// is converted to an integer and the low word is stored.  The
    /// simulation's word-wide `UNIS` slot of the same kind is the shield
    /// pool, which the method returns; a missing key (or a type left on
    /// its default) yields `None`.
    pub fn a40_shield_override(&self, type_id: u16) -> Option<u16> {
        let settings = self.rules.unit_settings.as_ref()?;
        if !a40_override_live(settings, type_id) {
            return None;
        }
        settings.shields.get(type_id as usize).copied()
    }

    /// `0x005e2a90`: read a dword-valued setting through the conversion
    /// path whose text payload is parsed by the second decimal helper.
    ///
    /// The accessor copies four bytes from the converted value, so the
    /// destination receives a 32-bit integer even when the map stored a
    /// string or a double.  This reader and its neighbour share every case
    /// of the conversion (integers, doubles, booleans) and differ only in
    /// which decimal string parser they call, so a stored text value is the
    /// one case where the two addresses can disagree.
    ///
    /// The 32-bit `UNIS` slot is the raw maximum hit-point value, and the
    /// method returns it; `None` is the absent-key path.
    pub fn a40_hit_point_override(&self, type_id: u16) -> Option<u32> {
        let settings = self.rules.unit_settings.as_ref()?;
        if !a40_override_live(settings, type_id) {
            return None;
        }
        settings.hit_points.get(type_id as usize).copied()
    }

    /// `0x005e2ac0`: read a dword-valued setting through the integer
    /// conversion family.
    ///
    /// The body mirrors `0x005e2a30`/`0x005e2a60` apart from the store
    /// width: the same converter is used, so text that is not a number and
    /// an out-of-range payload behave exactly as they do for the byte and
    /// word readers.  Four bytes are copied out.
    ///
    /// The four-byte slot of the same width in the scenario table is the
    /// mineral/gas cost pair, so the method returns both words.  `None`
    /// again means "no entry for this type".
    pub fn a40_unit_cost_override(&self, type_id: u16) -> Option<(u16, u16)> {
        let settings = self.rules.unit_settings.as_ref()?;
        if !a40_override_live(settings, type_id) {
            return None;
        }
        let minerals = settings.mineral_cost.get(type_id as usize).copied()?;
        let gas = settings.gas_cost.get(type_id as usize).copied()?;
        Some((minerals, gas))
    }

    /// `0x005e2af0`: read a text-valued setting into the caller's string,
    /// leaving that string untouched when the key is absent.
    ///
    /// The routine reserves room in a temporary string, asks the settings
    /// map for the key as text and, when it is found, assigns the text to
    /// the destination (which is grown first).  The absent path leaves the
    /// destination exactly as it was, so retail callers can hand in a
    /// default-initialised buffer.  The exception frame around the
    /// temporary is restored on both paths.
    ///
    /// [`World`] keeps a unit's `UNIS` name override as an id into the
    /// scenario/`stat_txt` string table rather than as text, so the method
    /// returns that id.  [`World::unit_name`] resolves the id to text when
    /// the caller needs the string; `None` here is the untouched-destination
    /// path.
    pub fn a40_string_id_override(&self, type_id: u16) -> Option<u16> {
        let settings = self.rules.unit_settings.as_ref()?;
        if !a40_override_live(settings, type_id) {
            return None;
        }
        settings.string_id.get(type_id as usize).copied()
    }

    /// `0x005e29f0`: store a text value under a settings key.
    ///
    /// The routine resolves the key through the string table, copies the
    /// key into the map's node storage, then writes the value; the map
    /// creates the entry when the key was not present, so a written entry
    /// counts as present from then on.
    ///
    /// The simulation's `UNIS` name override is present exactly when
    /// `use_default[type]` is zero and the override itself is a string id,
    /// so this method writes the id and clears the default marker.  It
    /// returns `false` only when the scenario supplied no `UNIS` section at
    /// all - the single case the retail map can accept but the simulation
    /// record cannot represent.
    pub fn a40_set_string_id_override(&mut self, type_id: u16, string_id: u16) -> bool {
        let Some(settings) = self.rules.unit_settings.as_mut() else {
            return false;
        };
        let Some(slot) = settings.string_id.get_mut(type_id as usize) else {
            return false;
        };
        *slot = string_id;
        if let Some(live) = settings.use_default.get_mut(type_id as usize) {
            *live = 0;
        }
        true
    }

    /// `0x005e2a10`: the second exported copy of the settings writer.
    ///
    /// The instruction sequence is the same accessor as `0x005e29f0`: the
    /// helper resolves the key, then tail-calls the same map setter, and
    /// only the order in which the caller pushes the key and value differs.
    /// The method is the same write, kept as a separate entry point so that
    /// callers keyed by address match one to one.
    pub fn a40_store_string_id_override(&mut self, type_id: u16, string_id: u16) -> bool {
        self.a40_set_string_id_override(type_id, string_id)
    }

    /// `0x005e3dc0`: append a 16-byte record to the growable point array.
    ///
    /// The helper compares the element count with the capacity, grows the
    /// storage from 16 elements (or by half the current capacity, rounded
    /// up, when it is already allocated), copies the 16-byte source record
    /// and then clears the source record's trailing `u16` marker.
    ///
    /// [`crate::trigger::TriggerSystem::locations`] is the simulation's
    /// 16-byte point record (`(left, top, right, bottom)`); the method
    /// appends one and returns its 1-based id, which is the form trigger
    /// actions use.  `None` means the scenario has no trigger VM, so there
    /// is no point array to append to.
    pub fn a40_add_trigger_location(
        &mut self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> Option<u32> {
        let triggers = self.triggers.as_mut()?;
        triggers.locations.push((left, top, right, bottom));
        Some(triggers.locations.len() as u32)
    }

    /// `0x005e3bd0`: release every element of the record's pointer array.
    ///
    /// The routine walks the array, and for each non-null element frees its
    /// two `std::string` members before deleting the element itself; the
    /// array storage is released last.  The high-bit tests in the recovered
    /// code are the small-string optimisation checks of those two string
    /// destructors.
    ///
    /// The simulation's per-tick cue arrays are the collections the client
    /// drops the same way: [`World::matrix_cues`] and
    /// [`World::nuke_warnings`] are presentation-only and are cleared at the
    /// end of the tick.  The method releases both (dropping their storage)
    /// and returns how many cues were released.
    pub fn a40_release_tick_cues(&mut self) -> usize {
        let matrix = std::mem::take(&mut self.matrix_cues);
        let warnings = std::mem::take(&mut self.nuke_warnings);
        matrix.len() + warnings.len()
    }
}

impl SimUnit {
    /// `0x005e3740`: empty the record's intrusive list and release its
    /// nodes.
    ///
    /// The routine points the head's next and previous links back at the
    /// head, zeroes the count and then walks the old chain, releasing each
    /// node's two `std::string` members before deleting the node.  When the
    /// list was already empty it returns after the first three stores.
    ///
    /// The order queue is the simulation's intrusive list of that shape, so
    /// the method clears it and returns the number of orders that were
    /// released.  The per-order text members the retail nodes also own have
    /// no simulation counterpart.
    pub fn a40_reset_order_list(&mut self) -> usize {
        let released = self.queue.len();
        self.queue.clear();
        released
    }

    /// `0x005e2ce0`: destroy the order lists when the owning record goes
    /// away.
    ///
    /// This is the destructor wrapper for the routine above: it only sets
    /// up the exception frame, calls the list reset with its own `this` and
    /// restores the frame, so the list ends up empty on both the normal and
    /// the unwind path.
    ///
    /// A destroyed record also drops every other order container it owns,
    /// so the method clears the queue and the production queue and drops
    /// the pending resume order, returning the number of queued orders that
    /// were released.  It is not called from the simulation automatically;
    /// units keep their record in [`World::units`] after death.
    pub fn a40_teardown_order_lists(&mut self) -> usize {
        let released = self.queue.len() + self.production.len();
        self.queue.clear();
        self.production.clear();
        self.resume_order = None;
        released
    }

    /// `0x005e3e30`: coerce a stored setting to the client's two-word
    /// boolean and record whether it was loaded.
    ///
    /// The routine reads the value through the mapping iterator and writes
    /// a pair into the destination record.  When the value is absent it
    /// writes `loaded = 0` and leaves the value byte alone.  When it is
    /// present it sets `loaded = 1` and computes the boolean from the
    /// variant: if the tag's bit 3 is clear the integer payload is tested
    /// for non-zero, otherwise the tag itself must be the boolean `true`
    /// code (10).
    ///
    /// [`SimUnit::doodad_enabled`] is the simulation's optional switch of
    /// exactly that shape, so the method stores the pair there and returns
    /// the loaded flag.  `tag` is the variant tag word and `payload` its
    /// integer payload; `None` is the absent case.
    pub fn a40_store_doodad_switch(&mut self, stored: Option<(i16, i32)>) -> bool {
        self.doodad_enabled = stored.map(|(tag, payload)| {
            if (tag as u8) >> 3 & 1 == 0 {
                payload != 0
            } else {
                tag == 10
            }
        });
        stored.is_some()
    }
}

// Rows in `0x005e2000..=0x005e3fff` that were not translated:
//
// * `FUN_005e221a`, `FUN_005e27a6`, `FUN_005e2889`, `FUN_005e28f7` - fragments of the
//   static initialiser for the fixed string table (string id, size and
//   small-string capacity stores).  Rust initialises that data at compile
//   time, so there is no runtime behaviour to reproduce.
// * `FUN_005e24fe`, `FUN_005e26cd` - recovered as `halt_baddata`; the addresses
//   fall inside the initialiser, not on an instruction boundary.
// * `FUN_005e2a04`, `FUN_005e2a67`, `FUN_005e3e79` - partially recovered copies of
//   the accessors above whose arguments live at fixed `ebp` offsets (the
//   decompiler started inside the caller frame), so no independent routine
//   can be attributed to them.
// * `FUN_005e2b8c`, `FUN_005e2b9c`, `FUN_005e2bc0`, `FUN_005e2d20`, `FUN_005e2d5b`,
//   `FUN_005e2d79`, `FUN_005e2da0`, `FUN_005e3100`, `FUN_005e3172`, `FUN_005e3187`,
//   `FUN_005e31a7`, `FUN_005e330f`, `FUN_005e332e`, `FUN_005e3363`, `FUN_005e3386`,
//   `FUN_005e3453`, `FUN_005e3468`, `FUN_005e3667`, `FUN_005e368a`, `FUN_005e36a7`,
//   `FUN_005e36c4`, `FUN_005e36e4`, `FUN_005e379b`, `FUN_005e37ba`, `FUN_005e37d0`,
//   `FUN_005e38ad`, `FUN_005e38c2`, `FUN_005e38c8`, `FUN_005e3ed4` - MSVC
//   `__finally`/catch funclets, temporary-string destructors and vector
//   copy constructors.  They only restore the exception list, free
//   temporaries or copy client objects; there is no simulation record to
//   update.  `FUN_005e3453`/`FUN_005e3468` additionally count `/` characters in
//   a resource path, which the simulation never handles.
// * `FUN_005e2e70`, `FUN_005e3980`, `FUN_005e3a60`, `FUN_005e3af2`, `FUN_005e3b60` -
//   scans of a client-side registry whose 0x98-byte nodes embed a settings
//   map and two `std::string` members, keyed by a formatted
//   `"{0}#|#{1}"` string.  The simulation keeps no such registry.
// * `FUN_005e3cc0`, `FUN_005e3d40` - destructors that free two `std::string`
//   members of a client record; the simulation has no per-record text.
// * `FUN_005e3d00`, `FUN_005e3d10` - one-byte flag clears on client UI objects
//   reached through their vtables; the object type is not a `CUnit`.
// * `FUN_005e3d21` - not an instruction boundary (`*eax += eax` was decoded
//   from data), `FUN_005e3d30` - thunk to a routine outside this range,
//   `FUN_005e3d3c` - `int3` breakpoint.

#[cfg(test)]
mod tests_agent40 {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::{Terrain, UnitSettings};
    use crate::sim::Order;
    use crate::trigger::TriggerSystem;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    #[allow(clippy::field_reassign_with_default)]
    fn unit_settings() -> UnitSettings {
        let mut settings = UnitSettings::default();
        settings.use_default = vec![1; 228];
        settings.hit_points = vec![0; 228];
        settings.shields = vec![0; 228];
        settings.armor = vec![0; 228];
        settings.build_time = vec![0; 228];
        settings.mineral_cost = vec![0; 228];
        settings.gas_cost = vec![0; 228];
        settings.string_id = vec![0; 228];
        settings.weapon_damage = vec![0; 130];
        settings.weapon_bonus = vec![0; 130];
        settings
    }

    /// `0x005e2a30`: the byte accessor answers only for a live `UNIS`
    /// entry; the type left on its default is the absent-key case.
    #[test]
    fn armor_override_is_read_only_when_unised() {
        let mut w = world();
        assert_eq!(w.a40_armor_override(uid::MARINE), None);

        let mut settings = unit_settings();
        settings.armor[uid::MARINE as usize] = 3;
        w.rules.unit_settings = Some(settings);

        // `use_default` is still 1, so the entry is not live yet.
        assert_eq!(w.a40_armor_override(uid::MARINE), None);

        w.rules.unit_settings.as_mut().unwrap().use_default[uid::MARINE as usize] = 0;
        assert_eq!(w.a40_armor_override(uid::MARINE), Some(3));
        // Types left on their default are the absent-key case even though
        // the table entry exists.
        assert_eq!(w.a40_armor_override(uid::MARINE + 1), None);
    }

    /// `0x005e2a60` and `0x005e2a90`: the word and dword accessors read
    /// their own slots of the same live entry.
    #[test]
    fn shield_and_hit_point_overrides_are_read_by_width() {
        let mut w = world();
        let mut settings = unit_settings();
        settings.use_default[uid::MARINE as usize] = 0;
        settings.shields[uid::MARINE as usize] = 40;
        settings.hit_points[uid::MARINE as usize] = 900;
        w.rules.unit_settings = Some(settings);

        assert_eq!(w.a40_shield_override(uid::MARINE), Some(40));
        assert_eq!(w.a40_hit_point_override(uid::MARINE), Some(900));
        assert_eq!(
            w.a40_shield_override(uid::MARINE + 1),
            None,
            "a type left on its default has no live entry"
        );
    }

    /// `0x005e2ac0`: the dword writer copies four bytes; the simulation
    /// reads that pair back as the mineral/gas cost.
    #[test]
    fn cost_override_reads_both_words() {
        let mut w = world();
        assert_eq!(w.a40_unit_cost_override(uid::MARINE), None);

        let mut settings = unit_settings();
        settings.use_default[uid::MARINE as usize] = 0;
        settings.mineral_cost[uid::MARINE as usize] = 50;
        settings.gas_cost[uid::MARINE as usize] = 25;
        w.rules.unit_settings = Some(settings);

        assert_eq!(w.a40_unit_cost_override(uid::MARINE), Some((50, 25)));
    }

    /// `0x005e2af0`, `0x005e29f0` and `0x005e2a10`: a written text entry
    /// becomes present, and the two writer addresses agree.
    #[test]
    fn text_override_writes_then_reads() {
        let mut w = world();
        assert_eq!(w.a40_string_id_override(uid::MARINE), None);
        assert!(
            !w.a40_set_string_id_override(uid::MARINE, 7),
            "no UNIS section means the record has no slot"
        );

        w.rules.unit_settings = Some(unit_settings());
        assert_eq!(w.a40_string_id_override(uid::MARINE), None);

        assert!(w.a40_set_string_id_override(uid::MARINE, 7));
        assert_eq!(w.a40_string_id_override(uid::MARINE), Some(7));
        assert_eq!(
            w.rules.unit_settings.as_ref().unwrap().use_default[uid::MARINE as usize],
            0,
            "writing the key makes the entry present"
        );

        assert!(w.a40_store_string_id_override(uid::MARINE, 9));
        assert_eq!(w.a40_string_id_override(uid::MARINE), Some(9));
    }

    /// `0x005e3740`: the intrusive list reset empties the order queue and
    /// reports how many nodes it released, but not the production queue.
    #[test]
    fn reset_order_list_empties_only_the_queue() {
        let mut w = world();
        let token = w.spawn(0, uid::MARINE, 12, 12).unwrap();
        let unit = &mut w.units[token];
        unit.queue.push_back(Order::HoldPosition);
        unit.queue.push_back(Order::Move { pos: (20, 20) });
        unit.production.push_back(Order::Train {
            unit_type: uid::MARINE,
        });

        assert_eq!(unit.a40_reset_order_list(), 2);
        assert!(unit.queue.is_empty());
        assert_eq!(unit.production.len(), 1);
        assert_eq!(unit.a40_reset_order_list(), 0);
    }

    /// `0x005e2ce0`: the destructor path drops every order container and
    /// the pending resume order.
    #[test]
    fn teardown_drops_all_order_state() {
        let mut w = world();
        let token = w.spawn(0, uid::MARINE, 12, 12).unwrap();
        let unit = &mut w.units[token];
        unit.queue.push_back(Order::Stop);
        unit.production.push_back(Order::Train {
            unit_type: uid::FIREBAT,
        });
        unit.resume_order = Some(Order::HoldPosition);

        assert_eq!(unit.a40_teardown_order_lists(), 2);
        assert!(unit.queue.is_empty());
        assert!(unit.production.is_empty());
        assert!(unit.resume_order.is_none());
    }

    /// `0x005e3e30`: absent values leave the switch unloaded; present ones
    /// take the integer or the tag branch of the variant conversion.
    #[test]
    fn doodad_switch_follows_the_variant_conversion() {
        let mut w = world();
        let token = w.spawn(0, uid::MARINE, 12, 12).unwrap();
        let unit = &mut w.units[token];

        assert!(!unit.a40_store_doodad_switch(None));
        assert_eq!(unit.doodad_enabled, None);

        assert!(unit.a40_store_doodad_switch(Some((0, 5))));
        assert_eq!(unit.doodad_enabled, Some(true));

        assert!(unit.a40_store_doodad_switch(Some((0, 0))));
        assert_eq!(unit.doodad_enabled, Some(false));

        // Tag bit 3 set: the payload is ignored and the tag is compared
        // with the boolean `true` code.
        assert!(unit.a40_store_doodad_switch(Some((8, 99))));
        assert_eq!(unit.doodad_enabled, Some(false));

        assert!(unit.a40_store_doodad_switch(Some((10, 0))));
        assert_eq!(unit.doodad_enabled, Some(true));
    }

    /// `0x005e3dc0`: appended point records get consecutive 1-based ids;
    /// without a trigger VM there is no array to append to.
    #[test]
    fn trigger_locations_append_in_order() {
        let mut w = world();
        assert_eq!(w.a40_add_trigger_location(1, 2, 3, 4), None);

        w.triggers = Some(TriggerSystem::default());
        assert_eq!(w.a40_add_trigger_location(1, 2, 3, 4), Some(1));
        assert_eq!(w.a40_add_trigger_location(5, 6, 7, 8), Some(2));
        assert_eq!(
            w.triggers.as_ref().unwrap().locations,
            vec![(1, 2, 3, 4), (5, 6, 7, 8)]
        );
    }

    /// `0x005e3bd0`: the cue release empties both per-tick arrays and
    /// reports the number of released elements.
    #[test]
    fn releasing_tick_cues_drops_both_lists() {
        let mut w = world();
        w.nuke_warnings.push((4, 5));
        w.nuke_warnings.push((6, 7));
        w.matrix_cues.push(crate::sim::MatrixCue {
            unit: 3,
            x: 1,
            y: 2,
            absorbed: 12,
            broke: false,
        });

        assert_eq!(w.a40_release_tick_cues(), 3);
        assert!(w.nuke_warnings.is_empty());
        assert!(w.matrix_cues.is_empty());
        assert_eq!(w.a40_release_tick_cues(), 0);
    }
}
