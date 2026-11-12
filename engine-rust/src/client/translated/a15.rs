#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent15.rs
// ===========================================================================
// Translation batch 15.  Only this file is edited by the batch.
//
// The batch covers the `decompiled_uncited` rows in
// `0x005b8000..=0x005bbfff` whose recovered body is at most 100 bytes and
// whose observable state is already modelled by [`crate::sim::World`] /
// [`crate::sim::SimUnit`].  Almost the whole block is the retail
// **ground-click command handler** plus the target-cursor state machine and
// a run of exception-unwind funclets, so only the click handlers are
// implemented here; the remaining rows are reported as skipped instead of
// being guessed at.
//
// Every click handler looks up a per-unit-type `orders.dat` row in one of
// the shipped byte tables (`0x00f8f2c8`, `0x00f8f3b0`, `0x00f8f498`,
// `0x00f8a450`) and then issues that row either against the clicked unit or
// at the clicked point.  Those tables are not part of
// [`crate::data::GameData`], so each entry point below takes the recovered
// row (and, for the gated variant, the gate byte) from its caller; the
// `CUnit+0x93` bit-0 write and the row dispatch itself are reproduced
// exactly through [`World::issue_order_id`].

impl World {
    /// Shared body of the recovered click handlers: set `CUnit+0x93` bit 0
    /// on the ordering unit and issue the recovered row with the clicked
    /// unit when one was hit, or with the clicked point otherwise.
    ///
    /// Retail performs the flag write before the dispatch in both branches
    /// (the target branch writes it inline, the point branch through the
    /// order-dispatch helper), so the Rust body does the same.
    fn a15_click_order_body(
        &mut self,
        unit_id: u32,
        order_row: u8,
        target: Option<u32>,
        pos: Option<(i32, i32)>,
    ) -> Result<(), SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        self.units[i].flags_93 |= 1;
        self.issue_order_id(unit_id, order_row, target, pos)
    }

    /// `0x005ba060` (`FUN_005ba060`, size 14): click handler for the
    /// per-unit-type row table at `0x00f8f2c8`.
    ///
    /// The body reads `row = table[unit type]` and branches on the clicked
    /// unit pointer.  With a unit target it sets `CUnit+0x93` bit 0 and
    /// issues `row` with both the position and the target; without a target
    /// it forwards the position to the same dispatch path (which performs
    /// the identical flag write).  The table lookup is replaced by the
    /// caller-supplied `order_row`; the remaining arguments are the clicked
    /// unit and point.
    pub fn a15_click_order_table_00f8f2c8(
        &mut self,
        unit_id: u32,
        order_row: u8,
        target: Option<u32>,
        pos: Option<(i32, i32)>,
    ) -> Result<(), SimError> {
        self.a15_click_order_body(unit_id, order_row, target, pos)
    }

    /// `0x005ba08b` (`FUN_005ba08b`, size 14): second click handler,
    /// identical to [`Self::a15_click_order_table_00f8f2c8`] except that it
    /// reads the per-unit-type row from the table at `0x00f8f3b0`.
    ///
    /// Retail keeps one instantiation per table pointer; the Rust port takes
    /// the row from the caller because the table is not part of the data
    /// model.
    pub fn a15_click_order_table_00f8f3b0(
        &mut self,
        unit_id: u32,
        order_row: u8,
        target: Option<u32>,
        pos: Option<(i32, i32)>,
    ) -> Result<(), SimError> {
        self.a15_click_order_body(unit_id, order_row, target, pos)
    }

    /// `0x005ba0b6` (`FUN_005ba0b6`, size 93): third click handler, the
    /// same target/point dispatch with the per-unit-type row read from the
    /// table at `0x00f8f498`.
    ///
    /// The routine is the largest of the four only because the compiler
    /// inlined the flag write and the two dispatch paths; the observable
    /// effect is the `CUnit+0x93` bit-0 write plus the row dispatch.
    pub fn a15_click_order_table_00f8f498(
        &mut self,
        unit_id: u32,
        order_row: u8,
        target: Option<u32>,
        pos: Option<(i32, i32)>,
    ) -> Result<(), SimError> {
        self.a15_click_order_body(unit_id, order_row, target, pos)
    }

    /// `0x005ba0a0` (`FUN_005ba0a0`, size 22): gated click handler.
    ///
    /// Before dispatching, the body compares the caller's row byte with
    /// `table[type]` at `0x00f8f498` and reads the per-type gate byte from
    /// the second table at `0x00f8a450`; an unset gate makes the routine
    /// return without issuing anything.  The comparison's taken branch only
    /// re-reads the unit type, so it has no state effect.  The gate is taken
    /// as a caller-supplied boolean for the same reason as the row.
    ///
    /// Returns whether the order was issued.
    pub fn a15_click_order_gated(
        &mut self,
        unit_id: u32,
        order_row: u8,
        gate: bool,
        target: Option<u32>,
        pos: Option<(i32, i32)>,
    ) -> Result<bool, SimError> {
        if !gate {
            return Ok(false);
        }
        self.a15_click_order_body(unit_id, order_row, target, pos)?;
        Ok(true)
    }

    /// `0x005ba343` (`FUN_005ba343`, size 38): continuation thunk that
    /// forwards four stack arguments (row, target and point) to the command
    /// row dispatcher and then unwinds its exception frame.
    ///
    /// The dispatcher is already modelled (it sets `CUnit+0x93` bit 0 and
    /// issues the row), so the thunk is exposed as a forwarding entry point
    /// with the same flag write; the unwind has no Rust equivalent.
    pub fn a15_forward_click_command(
        &mut self,
        unit_id: u32,
        order_row: u8,
        target: Option<u32>,
        pos: Option<(i32, i32)>,
    ) -> Result<(), SimError> {
        self.a15_click_order_body(unit_id, order_row, target, pos)
    }
}

#[cfg(test)]
mod tests_agent15 {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::Order;
    use std::sync::Arc;

    fn open_world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn spawn(world: &mut World, owner: usize, type_id: u16) -> u32 {
        let i = world.spawn(owner, type_id, 320, 320).unwrap();
        world.units[i].id
    }

    #[test]
    fn click_order_table_00f8f2c8_dispatches_target_and_point() {
        let mut w = open_world();
        let marine = spawn(&mut w, 0, uid::MARINE);
        let zergling = spawn(&mut w, 1, uid::ZERGLING);

        // Clicked unit: the row goes out with the unit target.
        w.a15_click_order_table_00f8f2c8(marine, 8, Some(zergling), Some((400, 400)))
            .unwrap();
        let i = w.index_of(marine).unwrap();
        assert_eq!(w.units[i].order_row, 8);
        assert_eq!(w.units[i].order, Order::AttackUnit { target: zergling });
        assert_eq!(w.units[i].flags_93 & 1, 1, "CUnit+0x93 bit 0");

        // Clicked ground: the same flag write and a position order.
        let marine2 = spawn(&mut w, 0, uid::MARINE);
        w.a15_click_order_table_00f8f2c8(marine2, 6, None, Some((420, 420)))
            .unwrap();
        let j = w.index_of(marine2).unwrap();
        assert_eq!(w.units[j].order, Order::Move { pos: (420, 420) });
        assert_eq!(w.units[j].order_row, 6);
        assert_eq!(w.units[j].flags_93 & 1, 1);

        assert_eq!(
            w.a15_click_order_table_00f8f2c8(0xdead_beef, 6, None, Some((1, 1))),
            Err(SimError::BadUnit)
        );
    }

    #[test]
    fn click_order_table_00f8f3b0_issues_the_same_dispatch() {
        let mut w = open_world();
        let marine = spawn(&mut w, 0, uid::MARINE);
        w.a15_click_order_table_00f8f3b0(marine, 6, None, Some((100, 200)))
            .unwrap();
        let i = w.index_of(marine).unwrap();
        assert_eq!(w.units[i].order, Order::Move { pos: (100, 200) });
        assert_eq!(w.units[i].order_row, 6);
        assert_eq!(w.units[i].flags_93 & 1, 1);
    }

    #[test]
    fn click_order_table_00f8f498_issues_a_hold_row_without_a_point() {
        let mut w = open_world();
        let marine = spawn(&mut w, 0, uid::MARINE);
        w.a15_click_order_table_00f8f498(marine, 1, None, None)
            .unwrap();
        let i = w.index_of(marine).unwrap();
        assert_eq!(w.units[i].order, Order::HoldPosition);
        assert_eq!(w.units[i].order_row, 1);
        assert_eq!(w.units[i].flags_93 & 1, 1);
    }

    #[test]
    fn gated_click_order_needs_its_gate_byte() {
        let mut w = open_world();
        let marine = spawn(&mut w, 0, uid::MARINE);

        assert_eq!(
            w.a15_click_order_gated(marine, 6, false, None, Some((64, 64))),
            Ok(false)
        );
        let i = w.index_of(marine).unwrap();
        assert_eq!(w.units[i].order, Order::Idle);
        assert_eq!(w.units[i].flags_93, 0);

        assert_eq!(
            w.a15_click_order_gated(marine, 6, true, None, Some((64, 64))),
            Ok(true)
        );
        assert_eq!(w.units[i].order, Order::Move { pos: (64, 64) });
        assert_eq!(w.units[i].flags_93 & 1, 1);
    }

    #[test]
    fn forward_click_command_keeps_the_row_and_the_point() {
        let mut w = open_world();
        let marine = spawn(&mut w, 0, uid::MARINE);
        w.a15_forward_click_command(marine, 6, None, Some((96, 128)))
            .unwrap();
        let i = w.index_of(marine).unwrap();
        assert_eq!(w.units[i].order, Order::Move { pos: (96, 128) });
        assert_eq!(w.units[i].order_row, 6);
        // The dispatcher sets the command flag before issuing the row.
        assert_eq!(w.units[i].flags_93 & 1, 1);
    }
}
