#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent28.rs
// ===========================================================================
// Translation batch 28.  Only this file is edited by the batch.
//
// The `decompiled_uncited` entries of `0x005ae000..=0x005affff` are the
// retail unit-record (`CUnit`) copy-constructor and assignment fragments.
// The shipped compiler split one large copy constructor (destination in
// `ESI`, source in `EDI`) and its assignment twin (destination in `EDI`,
// source in `ESI`) into basic-block entry points: each entry copies one
// window of the donor record and then falls into the shared tail, where a
// predicate on the *destination* unit type selects which `+0xc0..=+0xf8`
// union member is cloned.  The calls the decompiler prints as
// non-returning are member-clone helpers, not aborts.
//
// [`SimUnit`] models the record's scalar state, so every method below
// mirrors the modelled subset of its window and documents the offsets it
// cannot represent (owned pointers at `+0xec`, `+0xf0..=+0xfc`, `+0x11c`,
// the tagged pointer at `+0x134`, the per-type queue/union slots).  No
// state is added to [`World`] or [`SimUnit`].  A `bool` return reports
// whether a destination-type-gated branch ran.

impl SimUnit {
    // -----------------------------------------------------------------
    // Modelled sub-block mirrors shared by the entry points below.
    // These are private helpers, not separate recovered routines.
    // -----------------------------------------------------------------

    /// Mirror the modelled bits of the `+0xdc` flag dword.
    ///
    /// Bit 0 is the removal/death latch (closest Rust state: `alive`),
    /// bit 4 the completed latch, bit 6 the Bunker latch and bits 8..9 the
    /// cloak state.  Bits 10, 14, 15, 16, 17, 18, 19, 21, 22 and 26 have
    /// dedicated flags; bit 29 is the capability latch.  The transport bit
    /// (bit 5) and the remaining reserved bits have no scalar field.
    fn a28_mirror_flag_word(&mut self, donor: &SimUnit) {
        self.alive = donor.alive;
        self.completed = donor.completed;
        self.in_bunker = donor.in_bunker;
        self.status.cloaked = donor.status.cloaked;
        self.flag_10 = donor.flag_10;
        self.flag_14 = donor.flag_14;
        self.flag_15 = donor.flag_15;
        self.flag_16 = donor.flag_16;
        self.flag_17 = donor.flag_17;
        self.flag_18 = donor.flag_18;
        self.flag_19 = donor.flag_19;
        self.flag_21 = donor.flag_21;
        self.flag_22 = donor.flag_22;
        self.flag_26 = donor.flag_26;
        self.flag_29 = donor.flag_29;
    }

    /// Mirror the modelled bytes of `+0xe0..=+0xe8`: the overlay flags
    /// (`+0xe0`), their companion byte (`+0xe1`) and the per-player
    /// visibility mask (`+0xe4`).  `+0xe2`, `+0xe3` and `+0xe8` have no
    /// Rust field.
    fn a28_mirror_overlay_and_visibility(&mut self, donor: &SimUnit) {
        self.overlay_flags = donor.overlay_flags;
        self.field_0e1 = donor.field_0e1;
        self.visibility_mask = donor.visibility_mask;
    }

    /// Mirror the status head `+0x110..=+0x11b`: the corpse/death timer, the
    /// Defensive Matrix pool and duration, and the six effect timers plus
    /// the psionic-storm guard.  The irradiate source pointer at `+0x11c`
    /// has no Rust field.
    fn a28_mirror_status_head(&mut self, donor: &SimUnit) {
        self.death_timer = donor.death_timer;
        self.status.matrix = donor.status.matrix;
        self.status.matrix_ticks = donor.status.matrix_ticks;
        self.status.ensnare_ticks = donor.status.ensnare_ticks;
        self.status.lockdown_ticks = donor.status.lockdown_ticks;
        self.status.irradiate_ticks = donor.status.irradiate_ticks;
        self.status.stasis_ticks = donor.status.stasis_ticks;
        self.status.plague_ticks = donor.status.plague_ticks;
        self.status.psi_storm_guard = donor.status.psi_storm_guard;
    }

    /// Mirror the status tail `+0x120..=+0x12f`: hallucination, both
    /// per-unit masks, parasite, Maelstrom, the Acid Spore count and the
    /// nine spore lifetime slots.  `+0x125` has no Rust field.
    fn a28_mirror_status_tail(&mut self, donor: &SimUnit) {
        self.status.hallucination = donor.status.hallucination;
        self.status.status_mask_121 = donor.status.status_mask_121;
        self.status.parasite = donor.status.parasite;
        self.status.status_mask_123 = donor.status.status_mask_123;
        self.status.maelstrom_ticks = donor.status.maelstrom_ticks;
        self.status.acid_spores = donor.status.acid_spores;
        self.status.acid_spore_ticks = donor.status.acid_spore_ticks;
    }

    /// Mirror the header `+0x100..=+0x11b` as far as it is modelled: the
    /// opaque `+0x100` slot, the `+0x105` state bits, the `+0x107`
    /// shield-recharge latch and the status head.  Retail stores `+0x100`
    /// as an offset relative to the module base; the Rust field is already
    /// an opaque value, so the value itself is carried over.  `+0x104`,
    /// `+0x106` and `+0x108..=+0x10e` have no Rust field.
    fn a28_mirror_unit_header(&mut self, donor: &SimUnit) {
        self.field_100 = donor.field_100;
        self.flags_105 = donor.flags_105;
        self.shield_recharged = donor.shield_recharged;
        self.a28_mirror_status_head(donor);
    }

    /// The shared tail's Nuclear Silo union block: a Silo destination takes
    /// the donor's `+0xd4` dword, the armed-missile latch.  Every other
    /// destination type uses `+0xd0..=+0xd6` for a different union member,
    /// so the flag is only mirrored for a Silo.  Returns whether the block
    /// ran.  The missile link at `+0xd0` is a unit pointer with no Rust
    /// field (`World::nuke_strikes` owns the live missile instead).
    fn a28_mirror_silo_slot(&mut self, donor: &SimUnit) -> bool {
        if self.type_id != uid::NUCLEAR_SILO {
            return false;
        }
        self.nuke_ready = donor.nuke_ready;
        true
    }

    // -----------------------------------------------------------------
    // 0x005ae1c3 - destination-side constructor reset
    // -----------------------------------------------------------------

    /// `0x005ae1c3`: constructor prologue for a freshly allocated record.
    ///
    /// After the shared record copy the routine zeroes the 16 bytes at
    /// `+0xc0..=+0xcf`, the dword at `+0xd8`, the pair at `+0xf8..=+0xff`
    /// and the scratch block `+0x110..=+0x133`, then copies the source's
    /// `+0x60` dword and `+0x64`/`+0x66` words (order/target coordinates
    /// with no Rust field).  The modelled effect is the reset: the
    /// `+0xcf` companion byte, the armed-missile latch at `+0xd4` and every
    /// stored status slot in `+0x110..=+0x133` are cleared.
    ///
    /// The zeroed pointer slots at `+0xc0`, `+0xc4`, `+0xc8` and `+0xd0`
    /// are derived from [`Order`]/`production` or owned by other records in
    /// this crate, so they are deliberately not rewritten.  Flags at `+0xdc`
    /// and the overlay/visibility bytes are outside the cleared block and
    /// survive, exactly as in retail.
    pub fn a28_reset_transient_unit_state(&mut self) {
        self.field_0cf = 0; // +0xcf, inside the +0xc0..=+0xcf clear
        self.nuke_ready = false; // +0xd4, inside the +0xd0..=+0xd7 clear
        self.death_timer = 0; // +0x110
        self.status.matrix = 0; // +0x112
        self.status.matrix_ticks = 0; // +0x114
        self.status.ensnare_ticks = 0; // +0x116
        self.status.lockdown_ticks = 0; // +0x117
        self.status.irradiate_ticks = 0; // +0x118
        self.status.stasis_ticks = 0; // +0x119
        self.status.plague_ticks = 0; // +0x11a
        self.status.psi_storm_guard = 0; // +0x11b
        self.status.hallucination = false; // +0x120
        self.status.status_mask_121 = 0; // +0x121
        self.status.parasite = false; // +0x122
        self.status.status_mask_123 = 0; // +0x123
        self.status.maelstrom_ticks = 0; // +0x124
        self.status.acid_spores = 0; // +0x126
        self.status.acid_spore_ticks = [0; 9]; // +0x127..=+0x12f
    }

    // -----------------------------------------------------------------
    // 0x005ae275 - core stat window
    // -----------------------------------------------------------------

    /// `0x005ae275` (copy-constructor form): the core window
    /// `+0x80..=+0xbe`, then the flag dword at `+0xdc` and the overlay /
    /// visibility bytes at `+0xe0..=+0xe8`.
    ///
    /// The incoming register argument is stored at `+0x80`, the
    /// Hatchery/Lair/Hive parent link ([`SimUnit::connected_unit`]); it is
    /// passed explicitly because retail takes it in a register rather than
    /// from the donor.  The packed stat block contributes the order
    /// carry counter, the two voice slots, the generic timers, the rank
    /// counter, the order/state bytes and the fixed-point energy pair.
    /// `+0x88`, `+0x8c`, `+0x92`, `+0x97`, `+0x98`, `+0x9a..=+0xa0`,
    /// `+0xa6..=+0xaa` and `+0xae..=+0xbe` have no Rust field.
    pub fn a28_copy_core_stats_from(&mut self, donor: &SimUnit, connected_unit: Option<u32>) {
        self.connected_unit = connected_unit; // +0x80
        self.order_carry = donor.order_carry; // +0x85
        self.timer_087 = donor.timer_087; // +0x87
        self.speech_ticks = donor.speech_ticks; // +0x8a
        self.speech_id = donor.speech_id; // +0x8b
        self.rank_kills = donor.rank_kills; // +0x8f
        self.field_090 = donor.field_090; // +0x90
        self.field_091 = donor.field_091; // +0x91
        self.flags_93 = donor.flags_93; // +0x93
        self.field_094 = donor.field_094; // +0x94
        self.field_096 = donor.field_096; // +0x96
        self.energy = donor.energy; // +0xa2 fixed-point energy, whole part
        self.energy_frac = donor.energy_frac; // +0xa2 fixed-point energy, fraction
        self.field_0ac = donor.field_0ac; // +0xac
        self.a28_mirror_flag_word(donor); // +0xdc
        self.a28_mirror_overlay_and_visibility(donor); // +0xe0..=+0xe8
    }

    // -----------------------------------------------------------------
    // 0x005ae56f - status window
    // -----------------------------------------------------------------

    /// `0x005ae56f` (copy-constructor form): the status window
    /// `+0x110..=+0x12f`, the following words/longs at `+0x130..=+0x14f`,
    /// and the shared tail that reaches the Nuclear Silo union block.
    ///
    /// The incoming register argument is the cloned irradiate-source
    /// pointer for `+0x11c`; it has no Rust field.  The `+0x134` slot is a
    /// tagged pointer whose clone helper is selected by the tag, and the
    /// words at `+0x130`, `+0x132`, `+0x138`, `+0x13a`, `+0x13c..=+0x148`
    /// and the bytes at `+0x14c..=+0x14f` are unmodelled.  All status slots
    /// in the window are mirrored; the Silo tail copies the armed latch.
    /// Returns whether the destination-type Silo block ran.
    pub fn a28_copy_status_window_from(&mut self, donor: &SimUnit) -> bool {
        self.a28_mirror_status_head(donor); // +0x110..=+0x11b
        self.a28_mirror_status_tail(donor); // +0x120..=+0x12f
        self.a28_mirror_silo_slot(donor) // +0xd4, Silo tail
    }

    // -----------------------------------------------------------------
    // 0x005ae7c2 - order prefix and availability seed
    // -----------------------------------------------------------------

    /// `0x005ae7c2` (copy-constructor entry): the order prefix
    /// `+0xc4..=+0xcd` and the shared tail.
    ///
    /// The packed prefix holds the linked order target and its queue slots,
    /// which the Rust model derives from [`Order`]; only the Silo latch in
    /// the tail is modelled.  The routine additionally has a real branch:
    /// when the donor's active upgrade id (`+0xc9`) is not the `0x3d`
    /// sentinel and the donor's `+0xcd` byte is zero, it looks the upgrade
    /// up in the per-owner availability table and stores `result + 1` back
    /// into `+0xcd`.
    ///
    /// `+0xcd` has no Rust field, so it reads as zero and the branch runs
    /// for every donor carrying an upgrade order.  The returned `bool` is
    /// the recovered branch condition; it tells the caller whether the
    /// availability seed happened.  Returns false for an Idle donor or for
    /// the `0x3d` sentinel id.
    pub fn a28_copy_order_prefix_from(&mut self, donor: &SimUnit) -> bool {
        self.a28_mirror_silo_slot(donor);
        matches!(donor.order, Order::Upgrade { upgrade_id, .. } if upgrade_id != 0x3d)
    }

    // -----------------------------------------------------------------
    // 0x005ae884 - research prefix
    // -----------------------------------------------------------------

    /// `0x005ae884` (copy-constructor entry): the research/counter prefix
    /// `+0xc8` (incoming argument), `+0xcc` word, `+0xce`/`+0xcf` bytes,
    /// then the shared tail.
    ///
    /// The `+0xc8` dword is an owned pointer in the retail record and the
    /// `+0xcc`/`+0xce` bytes are queue bookkeeping; none has a Rust field.
    /// The `+0xcf` companion byte is [`SimUnit::field_0cf`], and the Silo
    /// tail supplies the armed latch.  Returns whether the Silo block ran.
    pub fn a28_copy_research_prefix_from(&mut self, donor: &SimUnit) -> bool {
        self.field_0cf = donor.field_0cf; // +0xcf
        self.a28_mirror_silo_slot(donor)
    }

    // -----------------------------------------------------------------
    // 0x005ae93f - dialog prefix without the companion byte
    // -----------------------------------------------------------------

    /// `0x005ae93f` (copy-constructor entry): the short dialog prefix
    /// `+0xc8` (incoming argument), the `+0xcc` byte and the shared tail.
    ///
    /// Unlike the `0x005ae884` entry this block stops before `+0xce` and
    /// `+0xcf`, so [`SimUnit::field_0cf`] is deliberately left untouched.
    /// The only modelled field in the window is the Silo armed latch from
    /// the tail.  Returns whether that Silo block ran.
    pub fn a28_copy_dialog_prefix_from(&mut self, donor: &SimUnit) -> bool {
        self.a28_mirror_silo_slot(donor)
    }

    // -----------------------------------------------------------------
    // 0x005ae981 - Silo union store
    // -----------------------------------------------------------------

    /// `0x005ae981` (copy-constructor entry): the Nuclear Silo union block.
    ///
    /// The entry stores the cloned missile link (the incoming register
    /// argument, retail `+0xd0`) and the donor's `+0xd4` dword, then falls
    /// through the remaining per-type tail.  Retail only reaches this block
    /// when the destination is a Nuclear Silo, so this helper returns false
    /// and changes nothing for every other type.  The missile link itself
    /// has no Rust field; the `+0xd4` armed latch is
    /// [`SimUnit::nuke_ready`].
    pub fn a28_store_silo_slot_from(&mut self, donor: &SimUnit) -> bool {
        self.a28_mirror_silo_slot(donor)
    }

    // -----------------------------------------------------------------
    // 0x005aeed4 - assignment-operator prefix
    // -----------------------------------------------------------------

    /// `0x005aeed4` (assignment form): the order prefix
    /// `+0xc0..=+0xcd`, then the flag/overlay/visibility tail.
    ///
    /// The source address is in the *destination* register pair here, which
    /// is how the compiler emits the assignment twin.  The routine copies
    /// `+0xc0` (incoming linked target), the `+0xc4`/`+0xc6` words and the
    /// bytes through `+0xcd`, then stores a zero word at `+0xce..=+0xcf`.
    /// Only `+0xcf` ([`SimUnit::field_0cf`]) is a stored Rust field, so this
    /// entry *clears* it rather than copying it.  The tail mirrors the flag
    /// dword, the overlay/visibility bytes and the Silo armed latch.
    /// Returns whether the Silo block ran.
    pub fn a28_assign_order_prefix_from(&mut self, donor: &SimUnit) -> bool {
        self.a28_mirror_flag_word(donor); // +0xdc
        self.a28_mirror_overlay_and_visibility(donor); // +0xe0..=+0xe8
        self.field_0cf = 0; // +0xce..=+0xcf explicitly zeroed
        self.a28_mirror_silo_slot(donor)
    }

    // -----------------------------------------------------------------
    // 0x005af181 - assignment scratch tail
    // -----------------------------------------------------------------

    /// `0x005af181` (assignment form): the scratch bytes
    /// `+0xd4`, `+0xd8`, `+0xd9`, the zero word at `+0xda..=+0xdb`, and the
    /// flag/overlay/visibility tail.
    ///
    /// `+0xd4` receives the incoming register argument before the union
    /// branches; for a Silo destination the tail's Silo block then takes the
    /// donor's `+0xd4` dword, which is the modelled case.  `+0xd8`, `+0xd9`
    /// and the cleared `+0xda` word have no Rust field.  Returns whether the
    /// Silo block ran.
    pub fn a28_assign_scratch_tail_from(&mut self, donor: &SimUnit) -> bool {
        self.a28_mirror_flag_word(donor); // +0xdc
        self.a28_mirror_overlay_and_visibility(donor); // +0xe0..=+0xe8
        self.a28_mirror_silo_slot(donor)
    }

    // -----------------------------------------------------------------
    // 0x005af1e5 - assignment flag tail
    // -----------------------------------------------------------------

    /// `0x005af1e5` (assignment form): the continuation after the
    /// `+0xd8` store.  It mirrors the flag dword at `+0xdc`, the
    /// overlay/visibility bytes at `+0xe0..=+0xe8` and the Silo armed
    /// latch, then clones `+0xec` (an owned member with no Rust field).
    ///
    /// Returns whether the Silo block ran.
    pub fn a28_assign_flag_tail_from(&mut self, donor: &SimUnit) -> bool {
        self.a28_mirror_flag_word(donor); // +0xdc
        self.a28_mirror_overlay_and_visibility(donor); // +0xe0..=+0xe8
        self.a28_mirror_silo_slot(donor)
    }

    // -----------------------------------------------------------------
    // 0x005af294 / 0x005af2c4 / 0x005af31f - unit header entries
    // -----------------------------------------------------------------

    /// `0x005af294` (assignment form): the header entry after the
    /// `+0xf0`/`+0xf4` owner-slot clones.  The modelled part is the
    /// `+0x100..=+0x11b` header (opaque slot, state bits, recharge latch,
    /// status head); the `+0x11c` source pointer and the owned slots at
    /// `+0xf0..=+0xf4` have no Rust field.
    pub fn a28_copy_unit_header_from(&mut self, donor: &SimUnit) {
        self.a28_mirror_unit_header(donor);
    }

    /// `0x005af2c4` (assignment form): the header entry after the
    /// `+0xf8`/`+0xfc` pointer clones.  The `+0x100..=+0x11b` window is the
    /// same modelled header as [`SimUnit::a28_copy_unit_header_from`]; the
    /// owned `+0xf8`/`+0xfc` pair and the trailing `+0x11c` clone are not
    /// modelled.
    pub fn a28_copy_unit_header_with_words_from(&mut self, donor: &SimUnit) {
        self.a28_mirror_unit_header(donor);
    }

    /// `0x005af31f` (assignment form): the header entry after the `+0xfc`
    /// store.  Mirrors the same modelled `+0x100..=+0x11b` header; the
    /// trailing `+0x11c` clone has no Rust field.
    pub fn a28_assign_unit_header_from(&mut self, donor: &SimUnit) {
        self.a28_mirror_unit_header(donor);
    }

    // -----------------------------------------------------------------
    // 0x005af4ae - spore lifetime slots
    // -----------------------------------------------------------------

    /// `0x005af4ae` (assignment form): the Acid Spore lifetime window
    /// `+0x128..=+0x12f`, followed by the words and longs at
    /// `+0x130..=+0x14f`.
    ///
    /// The entry stores the incoming byte at `+0x128` (spore slot 1) and
    /// copies `+0x129..=+0x12f` (slots 2..=8) from the donor.  Slot 0
    /// (`+0x127`) and the Acid Spore count belong to earlier entries, so
    /// they are untouched.  The `+0x130`/`+0x132` words, the tagged
    /// `+0x134` pointer and `+0x138..=+0x14f` have no Rust field.
    pub fn a28_copy_spore_slots_from(&mut self, donor: &SimUnit, slot_128: u8) {
        self.status.acid_spore_ticks[1] = slot_128; // +0x128
        self.status.acid_spore_ticks[2..9].copy_from_slice(&donor.status.acid_spore_ticks[2..9]);
        // +0x129..=+0x12f
    }
}

impl World {
    /// `0x005af680`: construct a unit only when its `units.dat` placement
    /// box lies completely inside the map.
    ///
    /// The recovered body rejects the placement unless
    /// `x - left >= 0`, `x + right < map_width`, `y - top >= 0` and
    /// `y + bottom < map_height`, using the 8-byte `units.dat` box record
    /// and the map's pixel dimensions.  On success it runs the retail unit
    /// constructor, appends the record to the unit list and returns the
    /// record; its remaining arguments (the two constructor words and the
    /// wire tag written by the slot helper) have no Rust equivalent.  The
    /// global unit-count invariant and the "unit list is initialised" gate
    /// are world-level conditions that the Rust [`World`] always satisfies.
    ///
    /// Returns the new unit id, or `None` when the box would cross a map
    /// edge, the owner is invalid, or construction fails.
    pub fn a28_spawn_unit_checked(
        &mut self,
        owner: usize,
        type_id: u16,
        x: i32,
        y: i32,
    ) -> Option<u32> {
        let left = self.unit_box_left(type_id);
        let right = self.unit_box_right(type_id);
        let top = self.unit_box_top(type_id);
        let bottom = self.unit_box_bottom(type_id);
        let map_width = self.terrain.width as i32 * 32;
        let map_height = self.terrain.height as i32 * 32;
        if owner >= self.players.len()
            || x - left < 0
            || x + right >= map_width
            || y - top < 0
            || y + bottom >= map_height
        {
            return None;
        }
        let index = self.spawn_px(owner, type_id, x, y).ok()?;
        Some(self.units[index].id)
    }

    /// `0x005afe5a`: the vertical half of the unit-box map test.
    ///
    /// The decompiled entry lands inside the enclosing bounds routine; the
    /// recovered tail compares the pixel `y` against the `units.dat` top and
    /// bottom extents and the map's pixel height:
    /// `y - top >= 0 && y + bottom < map_height`.  The leading pointer-window
    /// comparison of the enclosing routine is not part of this recovered
    /// predicate and is not modelled.
    pub fn a28_vertical_extent_fits(&self, type_id: u16, y: i32) -> bool {
        let top = self.unit_box_top(type_id);
        let bottom = self.unit_box_bottom(type_id);
        let map_height = self.terrain.height as i32 * 32;
        y - top >= 0 && y + bottom < map_height
    }
}

#[cfg(test)]
mod tests_agent28 {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::World;
    use std::sync::Arc;

    fn open_world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn spawn(world: &mut World, type_id: u16, x: i32, y: i32) -> usize {
        world.spawn_px(0, type_id, x, y).unwrap()
    }

    fn marine_pair(world: &mut World) -> (usize, usize) {
        (
            spawn(world, uid::MARINE, 100, 100),
            spawn(world, uid::MARINE, 200, 200),
        )
    }

    fn set_all_flags(unit: &mut SimUnit) {
        unit.completed = false;
        unit.in_bunker = true;
        unit.status.cloaked = true;
        unit.flag_10 = true;
        unit.flag_14 = true;
        unit.flag_15 = true;
        unit.flag_16 = true;
        unit.flag_17 = true;
        unit.flag_18 = true;
        unit.flag_19 = true;
        unit.flag_21 = true;
        unit.flag_22 = true;
        unit.flag_26 = true;
        unit.flag_29 = true;
    }

    fn assert_all_flags(unit: &SimUnit) {
        assert!(!unit.completed);
        assert!(unit.in_bunker);
        assert!(unit.status.cloaked);
        assert!(unit.flag_10);
        assert!(unit.flag_14);
        assert!(unit.flag_15);
        assert!(unit.flag_16);
        assert!(unit.flag_17);
        assert!(unit.flag_18);
        assert!(unit.flag_19);
        assert!(unit.flag_21);
        assert!(unit.flag_22);
        assert!(unit.flag_26);
        assert!(unit.flag_29);
    }

    #[test]
    fn reset_transient_unit_state_clears_status_but_not_flags() {
        let mut w = open_world();
        let (dest, _) = marine_pair(&mut w);
        {
            let u = &mut w.units[dest];
            u.field_0cf = 9;
            u.nuke_ready = true;
            u.death_timer = 5;
            u.status.matrix = 100;
            u.status.matrix_ticks = 3;
            u.status.ensnare_ticks = 1;
            u.status.lockdown_ticks = 2;
            u.status.irradiate_ticks = 3;
            u.status.stasis_ticks = 4;
            u.status.plague_ticks = 5;
            u.status.psi_storm_guard = 6;
            u.status.hallucination = true;
            u.status.status_mask_121 = 7;
            u.status.parasite = true;
            u.status.status_mask_123 = 8;
            u.status.maelstrom_ticks = 9;
            u.status.acid_spores = 1;
            u.status.acid_spore_ticks[0] = 4;
            u.flags_93 = 0x22;
            u.overlay_flags = 3;
            u.field_0e1 = 4;
            u.visibility_mask = 0xdead_beef;
        }
        w.units[dest].a28_reset_transient_unit_state();
        let u = &w.units[dest];
        assert_eq!(u.field_0cf, 0);
        assert!(!u.nuke_ready);
        assert_eq!(u.death_timer, 0);
        assert_eq!(u.status.matrix, 0);
        assert_eq!(u.status.matrix_ticks, 0);
        assert_eq!(u.status.ensnare_ticks, 0);
        assert_eq!(u.status.lockdown_ticks, 0);
        assert_eq!(u.status.irradiate_ticks, 0);
        assert_eq!(u.status.stasis_ticks, 0);
        assert_eq!(u.status.plague_ticks, 0);
        assert_eq!(u.status.psi_storm_guard, 0);
        assert!(!u.status.hallucination);
        assert_eq!(u.status.status_mask_121, 0);
        assert!(!u.status.parasite);
        assert_eq!(u.status.status_mask_123, 0);
        assert_eq!(u.status.maelstrom_ticks, 0);
        assert_eq!(u.status.acid_spores, 0);
        assert_eq!(u.status.acid_spore_ticks, [0; 9]);
        // Outside the cleared block.
        assert_eq!(u.flags_93, 0x22);
        assert_eq!(u.overlay_flags, 3);
        assert_eq!(u.field_0e1, 4);
        assert_eq!(u.visibility_mask, 0xdead_beef);
        assert!(u.alive);
    }

    #[test]
    fn core_stats_window_copies_modelled_slots() {
        let mut w = open_world();
        let (dest, donor) = marine_pair(&mut w);
        {
            let d = &mut w.units[donor];
            d.connected_unit = Some(1);
            d.order_carry = 7;
            d.timer_087 = 8;
            d.speech_ticks = 9;
            d.speech_id = 0x2a;
            d.rank_kills = 11;
            d.field_090 = 12;
            d.field_091 = 13;
            d.flags_93 = 0x14;
            d.field_094 = 0x1515;
            d.field_096 = 16;
            d.energy = 17;
            d.energy_frac = 18;
            d.field_0ac = 0x1919;
            d.overlay_flags = 0x21;
            d.field_0e1 = 0x22;
            d.visibility_mask = 0x1234_5678;
            set_all_flags(d);
        }
        {
            let d = &mut w.units[dest];
            d.status.maelstrom_ticks = 55;
            d.field_0ac = 1;
        }
        let donor_unit = w.units[donor].clone();
        w.units[dest].a28_copy_core_stats_from(&donor_unit, Some(77));
        let d = &w.units[dest];
        assert_eq!(d.connected_unit, Some(77));
        assert_eq!(d.order_carry, 7);
        assert_eq!(d.timer_087, 8);
        assert_eq!(d.speech_ticks, 9);
        assert_eq!(d.speech_id, 0x2a);
        assert_eq!(d.rank_kills, 11);
        assert_eq!(d.field_090, 12);
        assert_eq!(d.field_091, 13);
        assert_eq!(d.flags_93, 0x14);
        assert_eq!(d.field_094, 0x1515);
        assert_eq!(d.field_096, 16);
        assert_eq!(d.energy, 17);
        assert_eq!(d.energy_frac, 18);
        assert_eq!(d.field_0ac, 0x1919);
        assert_eq!(d.overlay_flags, 0x21);
        assert_eq!(d.field_0e1, 0x22);
        assert_eq!(d.visibility_mask, 0x1234_5678);
        assert_all_flags(d);
        assert_eq!(d.status.maelstrom_ticks, 55);
    }

    #[test]
    fn status_window_copies_effect_block_and_silo_slot() {
        let mut w = open_world();
        let dest = spawn(&mut w, uid::NUCLEAR_SILO, 100, 100);
        let donor = spawn(&mut w, uid::NUCLEAR_SILO, 200, 200);
        {
            let d = &mut w.units[donor];
            d.death_timer = 0x1111;
            d.status.matrix = 640;
            d.status.matrix_ticks = 0xa8;
            d.status.ensnare_ticks = 1;
            d.status.lockdown_ticks = 2;
            d.status.irradiate_ticks = 3;
            d.status.stasis_ticks = 4;
            d.status.plague_ticks = 5;
            d.status.psi_storm_guard = 1;
            d.status.hallucination = true;
            d.status.status_mask_121 = 0x21;
            d.status.parasite = true;
            d.status.status_mask_123 = 0x23;
            d.status.maelstrom_ticks = 0x16;
            d.status.acid_spores = 9;
            d.status.acid_spore_ticks = [1, 2, 3, 4, 5, 6, 7, 8, 9];
            d.nuke_ready = true;
        }
        {
            let d = &mut w.units[dest];
            d.flag_10 = true;
            d.status.matrix = 1;
        }
        let donor_unit = w.units[donor].clone();
        assert!(w.units[dest].a28_copy_status_window_from(&donor_unit));
        let d = &w.units[dest];
        assert_eq!(d.death_timer, 0x1111);
        assert_eq!(d.status.matrix, 640);
        assert_eq!(d.status.matrix_ticks, 0xa8);
        assert_eq!(d.status.ensnare_ticks, 1);
        assert_eq!(d.status.lockdown_ticks, 2);
        assert_eq!(d.status.irradiate_ticks, 3);
        assert_eq!(d.status.stasis_ticks, 4);
        assert_eq!(d.status.plague_ticks, 5);
        assert_eq!(d.status.psi_storm_guard, 1);
        assert!(d.status.hallucination);
        assert_eq!(d.status.status_mask_121, 0x21);
        assert!(d.status.parasite);
        assert_eq!(d.status.status_mask_123, 0x23);
        assert_eq!(d.status.maelstrom_ticks, 0x16);
        assert_eq!(d.status.acid_spores, 9);
        assert_eq!(d.status.acid_spore_ticks, [1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert!(d.nuke_ready);
        // The flag dword is not part of this window.
        assert!(d.flag_10);
    }

    #[test]
    fn order_prefix_reports_pending_upgrade_seed() {
        let mut w = open_world();
        let (dest, donor) = marine_pair(&mut w);
        let donor_unit = w.units[donor].clone();
        assert!(!w.units[dest].a28_copy_order_prefix_from(&donor_unit));

        w.units[donor].order = Order::Upgrade {
            upgrade_id: 5,
            level: 1,
        };
        let donor_unit = w.units[donor].clone();
        assert!(w.units[dest].a28_copy_order_prefix_from(&donor_unit));

        w.units[donor].order = Order::Upgrade {
            upgrade_id: 0x3d,
            level: 1,
        };
        let donor_unit = w.units[donor].clone();
        assert!(!w.units[dest].a28_copy_order_prefix_from(&donor_unit));
    }

    #[test]
    fn research_prefix_copies_counter_and_silo_slot() {
        let mut w = open_world();
        let (dest, donor) = marine_pair(&mut w);
        w.units[donor].field_0cf = 0x44;
        w.units[donor].nuke_ready = true;
        w.units[dest].field_0cf = 0;
        let donor_unit = w.units[donor].clone();
        assert!(!w.units[dest].a28_copy_research_prefix_from(&donor_unit));
        assert_eq!(w.units[dest].field_0cf, 0x44);
        // A non-Silo destination keeps its own armed latch.
        assert!(!w.units[dest].nuke_ready);

        let silo_dest = spawn(&mut w, uid::NUCLEAR_SILO, 300, 300);
        let silo_donor = spawn(&mut w, uid::NUCLEAR_SILO, 400, 400);
        w.units[silo_donor].field_0cf = 0x45;
        w.units[silo_donor].nuke_ready = true;
        let donor_unit = w.units[silo_donor].clone();
        assert!(w.units[silo_dest].a28_copy_research_prefix_from(&donor_unit));
        assert_eq!(w.units[silo_dest].field_0cf, 0x45);
        assert!(w.units[silo_dest].nuke_ready);
    }

    #[test]
    fn dialog_prefix_leaves_companion_byte_alone() {
        let mut w = open_world();
        let (dest, donor) = marine_pair(&mut w);
        w.units[donor].field_0cf = 0x55;
        w.units[dest].field_0cf = 0x66;
        let donor_unit = w.units[donor].clone();
        assert!(!w.units[dest].a28_copy_dialog_prefix_from(&donor_unit));
        assert_eq!(w.units[dest].field_0cf, 0x66);

        let silo_dest = spawn(&mut w, uid::NUCLEAR_SILO, 300, 300);
        let silo_donor = spawn(&mut w, uid::NUCLEAR_SILO, 400, 400);
        w.units[silo_donor].nuke_ready = true;
        let donor_unit = w.units[silo_donor].clone();
        assert!(w.units[silo_dest].a28_copy_dialog_prefix_from(&donor_unit));
        assert!(w.units[silo_dest].nuke_ready);
    }

    #[test]
    fn silo_slot_fragment_copies_the_armed_flag() {
        let mut w = open_world();
        let dest = spawn(&mut w, uid::NUCLEAR_SILO, 100, 100);
        let donor = spawn(&mut w, uid::NUCLEAR_SILO, 200, 200);
        w.units[donor].nuke_ready = true;
        let donor_unit = w.units[donor].clone();
        assert!(w.units[dest].a28_store_silo_slot_from(&donor_unit));
        assert!(w.units[dest].nuke_ready);

        let (marine_dest, marine_donor) = marine_pair(&mut w);
        w.units[marine_dest].nuke_ready = true;
        w.units[marine_donor].nuke_ready = false;
        let donor_unit = w.units[marine_donor].clone();
        assert!(!w.units[marine_dest].a28_store_silo_slot_from(&donor_unit));
        assert!(w.units[marine_dest].nuke_ready);
    }

    #[test]
    fn assignment_prefix_copies_flags_and_clears_counter() {
        let mut w = open_world();
        let (dest, donor) = marine_pair(&mut w);
        {
            let d = &mut w.units[donor];
            set_all_flags(d);
            d.overlay_flags = 0x31;
            d.field_0e1 = 0x32;
            d.visibility_mask = 0x0bad_f00d;
            d.field_0cf = 0x77;
            d.status.matrix = 999;
        }
        {
            let d = &mut w.units[dest];
            d.field_0cf = 0x66;
            d.status.matrix = 123;
        }
        let donor_unit = w.units[donor].clone();
        assert!(!w.units[dest].a28_assign_order_prefix_from(&donor_unit));
        let d = &w.units[dest];
        assert_all_flags(d);
        assert_eq!(d.overlay_flags, 0x31);
        assert_eq!(d.field_0e1, 0x32);
        assert_eq!(d.visibility_mask, 0x0bad_f00d);
        assert_eq!(d.field_0cf, 0);
        assert_eq!(d.status.matrix, 123);
    }

    #[test]
    fn assignment_scratch_tail_copies_flags() {
        let mut w = open_world();
        let (dest, donor) = marine_pair(&mut w);
        {
            let d = &mut w.units[donor];
            set_all_flags(d);
            d.overlay_flags = 0x41;
            d.field_0e1 = 0x42;
            d.visibility_mask = 0x1111_2222;
        }
        w.units[dest].field_0cf = 0x66;
        let donor_unit = w.units[donor].clone();
        assert!(!w.units[dest].a28_assign_scratch_tail_from(&donor_unit));
        assert_all_flags(&w.units[dest]);
        assert_eq!(w.units[dest].overlay_flags, 0x41);
        assert_eq!(w.units[dest].field_0e1, 0x42);
        assert_eq!(w.units[dest].visibility_mask, 0x1111_2222);
        // +0xcf is outside this entry's window.
        assert_eq!(w.units[dest].field_0cf, 0x66);
    }

    #[test]
    fn assignment_flag_tail_copies_flags() {
        let mut w = open_world();
        let (dest, donor) = marine_pair(&mut w);
        {
            let d = &mut w.units[donor];
            set_all_flags(d);
            d.overlay_flags = 0x51;
            d.field_0e1 = 0x52;
            d.visibility_mask = 0x3333_4444;
        }
        w.units[dest].field_0cf = 0x66;
        let donor_unit = w.units[donor].clone();
        assert!(!w.units[dest].a28_assign_flag_tail_from(&donor_unit));
        assert_all_flags(&w.units[dest]);
        assert_eq!(w.units[dest].overlay_flags, 0x51);
        assert_eq!(w.units[dest].field_0e1, 0x52);
        assert_eq!(w.units[dest].visibility_mask, 0x3333_4444);
        assert_eq!(w.units[dest].field_0cf, 0x66);
    }

    fn header_fixture(world: &mut World) -> (usize, usize) {
        let (dest, donor) = marine_pair(world);
        {
            let d = &mut world.units[donor];
            d.field_100 = 0x1234_5678;
            d.flags_105 = 0b11;
            d.shield_recharged = true;
            d.death_timer = 0x2222;
            d.status.matrix = 0x14000;
            d.status.matrix_ticks = 0xa0;
            d.status.ensnare_ticks = 1;
            d.status.lockdown_ticks = 2;
            d.status.irradiate_ticks = 3;
            d.status.stasis_ticks = 4;
            d.status.plague_ticks = 5;
            d.status.psi_storm_guard = 1;
        }
        {
            let d = &mut world.units[dest];
            d.field_100 = 1;
            d.flags_105 = 0;
            d.shield_recharged = false;
            d.death_timer = 1;
            d.status.matrix = 7;
            d.status.maelstrom_ticks = 55;
            d.flag_10 = true;
        }
        (dest, donor)
    }

    fn assert_header(world: &World, dest: usize) {
        let d = &world.units[dest];
        assert_eq!(d.field_100, 0x1234_5678);
        assert_eq!(d.flags_105, 0b11);
        assert!(d.shield_recharged);
        assert_eq!(d.death_timer, 0x2222);
        assert_eq!(d.status.matrix, 0x14000);
        assert_eq!(d.status.matrix_ticks, 0xa0);
        assert_eq!(d.status.ensnare_ticks, 1);
        assert_eq!(d.status.lockdown_ticks, 2);
        assert_eq!(d.status.irradiate_ticks, 3);
        assert_eq!(d.status.stasis_ticks, 4);
        assert_eq!(d.status.plague_ticks, 5);
        assert_eq!(d.status.psi_storm_guard, 1);
        // Fields outside the +0x100..=+0x11b window.
        assert!(d.flag_10);
        assert_eq!(d.status.maelstrom_ticks, 55);
    }

    #[test]
    fn unit_header_entry_points_mirror_the_header() {
        for entry in 0..3 {
            let mut w = open_world();
            let (dest, donor) = header_fixture(&mut w);
            let donor_unit = w.units[donor].clone();
            match entry {
                0 => w.units[dest].a28_copy_unit_header_from(&donor_unit),
                1 => w.units[dest].a28_copy_unit_header_with_words_from(&donor_unit),
                _ => w.units[dest].a28_assign_unit_header_from(&donor_unit),
            }
            assert_header(&w, dest);
        }
    }

    #[test]
    fn spore_slot_window_starts_at_index_one() {
        let mut w = open_world();
        let (dest, donor) = marine_pair(&mut w);
        w.units[donor].status.acid_spore_ticks = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        w.units[donor].status.acid_spores = 9;
        w.units[dest].status.acid_spore_ticks = [0x55; 9];
        w.units[dest].status.acid_spores = 1;
        let donor_unit = w.units[donor].clone();
        w.units[dest].a28_copy_spore_slots_from(&donor_unit, 0xaa);
        let s = &w.units[dest].status;
        assert_eq!(s.acid_spore_ticks, [0x55, 0xaa, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(s.acid_spores, 1);
    }

    #[test]
    fn checked_spawn_requires_inside_footprint() {
        let mut w = open_world();
        let dims = w.data.unit(uid::MARINE).dimensions;
        let (left, top) = (dims.left as i32, dims.top as i32);
        let (right, bottom) = (dims.right as i32, dims.bottom as i32);
        let map_width = w.terrain.width as i32 * 32;
        let map_height = w.terrain.height as i32 * 32;

        let id = w
            .a28_spawn_unit_checked(0, uid::MARINE, 100, 100)
            .expect("centre placement");
        let index = w.units.iter().position(|u| u.id == id).unwrap();
        assert_eq!(w.units[index].x / 256, 100);
        assert_eq!(w.units[index].y / 256, 100);

        assert!(
            w.a28_spawn_unit_checked(0, uid::MARINE, left - 1, 100)
                .is_none()
        );
        assert!(
            w.a28_spawn_unit_checked(0, uid::MARINE, 100, top - 1)
                .is_none()
        );
        assert!(
            w.a28_spawn_unit_checked(0, uid::MARINE, map_width - right, 100)
                .is_none()
        );
        assert!(
            w.a28_spawn_unit_checked(0, uid::MARINE, 100, map_height - bottom)
                .is_none()
        );
        assert!(
            w.a28_spawn_unit_checked(0, uid::MARINE, map_width - right - 1, 100)
                .is_some()
        );
        assert!(
            w.a28_spawn_unit_checked(0, uid::MARINE, 100, map_height - bottom - 1)
                .is_some()
        );
        // Owner 2 does not exist in a two-player world.
        assert!(w.a28_spawn_unit_checked(2, uid::MARINE, 100, 100).is_none());
    }

    #[test]
    fn vertical_extent_predicate_matches_the_map_band() {
        let w = open_world();
        let dims = w.data.unit(uid::MARINE).dimensions;
        let (top, bottom) = (dims.top as i32, dims.bottom as i32);
        let map_height = w.terrain.height as i32 * 32;
        assert!(w.a28_vertical_extent_fits(uid::MARINE, 1000));
        assert!(!w.a28_vertical_extent_fits(uid::MARINE, top - 1));
        assert!(!w.a28_vertical_extent_fits(uid::MARINE, map_height - bottom));
        assert!(w.a28_vertical_extent_fits(uid::MARINE, map_height - bottom - 1));
    }
}
