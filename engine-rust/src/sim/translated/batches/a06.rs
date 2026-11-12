#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent06.rs
// ===========================================================================
// Translation batch 06.  Only this file is edited by the batch.
//
// The ledger rows recovered for `0x005b7000..=0x005b9fff` are the order-list
// helpers of `CUnit`.  The list head is `CUnit+0x74` (the current order), the
// tail is `CUnit+0x78`, and `CUnit+0x84` is the order-count byte.  The typed
// model splits that list into [`SimUnit::order`] (head) and
// [`SimUnit::queue`] (the remaining nodes), so a node's `+8` order-type byte
// is `Order::order_id()` and the `DAT_00f8ad50[type]` lookup is the
// `orders.dat` highlight column.

/// `DAT_00f8ad50`: `orders.dat` highlight column.  Every order-list helper
/// tests `highlight != -1` before touching `CUnit+0x84`; rows without a
/// highlight are linked but do not consume a queue slot.
fn a06_uses_queue_slot(data: &GameData, order: &Order) -> bool {
    data.order(order.order_id())
        .is_some_and(|row| row.highlight != -1)
}

/// Number of nodes in the retail order list: the current order plus queued
/// orders.  `Order::Idle` is the typed model's empty-list sentinel, not a
/// linked node.
fn a06_order_list_len(unit: &SimUnit) -> usize {
    usize::from(!matches!(unit.order, Order::Idle)) + unit.queue.len()
}

/// One current order plus the eight pending slots (`CUnit+0x84 < 9` in
/// `0x005b7520`, matching [`crate::sim::MAX_ORDER_QUEUE`]).
const A06_ORDER_LIST_CAPACITY: usize = crate::sim::MAX_ORDER_QUEUE + 1;

impl SimUnit {
    /// `0x005b7140`: unlink every order node from the list at `CUnit+0x74`.
    /// For each node whose `DAT_00f8ad50[type]` entry is not `-1` the
    /// `CUnit+0x84` byte is decremented (clamped at zero), then the node is
    /// removed with `FUN_00577210`/`FUN_005772d0`.
    ///
    /// The typed model derives the count byte from the list, so clearing the
    /// current order and the queue is equivalent.  Returns the number of
    /// slot-consuming nodes removed.  `order_row` is the Rust cache of the
    /// head node's order id and is cleared with the list.
    pub fn a06_clear_order_list(&mut self, data: &GameData) -> u16 {
        let mut removed = 0u16;
        if !matches!(self.order, Order::Idle) && a06_uses_queue_slot(data, &self.order) {
            removed += 1;
        }
        for order in &self.queue {
            if a06_uses_queue_slot(data, order) {
                removed += 1;
            }
        }
        self.order = Order::Idle;
        self.order_row = 0;
        self.queue.clear();
        removed
    }

    /// `0x005b7c20`: unlink the head node (`CUnit+0x74`, the current order)
    /// and decrement `CUnit+0x84` when that order's highlight is not `-1`.
    /// The next queued node becomes the new head.  The retail helper assumes
    /// a non-empty list; the typed version reports an empty list as `None`.
    ///
    /// Returns the removed order and whether it consumed a queue slot.
    pub fn a06_pop_order_head(&mut self, data: &GameData) -> Option<(Order, bool)> {
        if matches!(self.order, Order::Idle) {
            return None;
        }
        let removed = std::mem::replace(&mut self.order, Order::Idle);
        let used_slot = a06_uses_queue_slot(data, &removed);
        if let Some(next) = self.queue.pop_front() {
            self.order = next;
        }
        self.order_row = self.order.order_id();
        Some((removed, used_slot))
    }

    /// `0x005b7410`: unlink the order node passed in `param_1`, decrementing
    /// `CUnit+0x84` when its `DAT_00f8ad50[type]` entry is not `-1`.  The
    /// caller guarantees a linked node; the typed equivalent searches the
    /// current order first and then the queue.
    ///
    /// Returns `Some(used_slot)` when an equal order was unlinked, or `None`
    /// when it was not in the list.  Removing the current node promotes the
    /// next queued order, matching the head-pointer update performed by
    /// `FUN_00577210`.
    pub fn a06_remove_order_node(&mut self, target: &Order, data: &GameData) -> Option<bool> {
        if matches!(target, Order::Idle) {
            return None;
        }
        if &self.order == target {
            let used_slot = a06_uses_queue_slot(data, target);
            self.order = self.queue.pop_front().unwrap_or(Order::Idle);
            self.order_row = self.order.order_id();
            return Some(used_slot);
        }
        let pos = self.queue.iter().position(|order| order == target)?;
        let removed = self.queue.remove(pos)?;
        Some(a06_uses_queue_slot(data, &removed))
    }

    /// `0x005b7520`: allocate a node for `order` and insert it at the head
    /// of the list, making it the current order.  `CUnit+0x84` is incremented
    /// only for rows whose highlight is not `-1`.  Allocation failure takes
    /// the retail error path; the typed queue cannot fail to allocate.
    ///
    /// The decompiled guard also accepts every insertion when `CUnit+0x134`
    /// is null.  That field is not modelled, so this translation enforces the
    /// `count < 9` arm ([`A06_ORDER_LIST_CAPACITY`]).  Returns whether the
    /// inserted row consumes a queue slot, or [`SimError::QueueFull`] when
    /// the head insert would exceed nine nodes.
    pub fn a06_insert_order_at_head(
        &mut self,
        order: Order,
        data: &GameData,
    ) -> Result<bool, SimError> {
        if matches!(order, Order::Idle) {
            return Ok(false);
        }
        if a06_order_list_len(self) >= A06_ORDER_LIST_CAPACITY {
            return Err(SimError::QueueFull);
        }
        let used_slot = a06_uses_queue_slot(data, &order);
        let old = std::mem::replace(&mut self.order, order);
        if !matches!(old, Order::Idle) {
            self.queue.push_front(old);
        }
        self.order_row = self.order.order_id();
        Ok(used_slot)
    }

    /// `0x005b7460`: wrapper around `0x005b7520` that supplies the fixed
    /// serialized tag `0xe4` and forwards the caller's list anchor.  The
    /// typed order has no raw tag/anchor, so this is the same head insertion
    /// with its capacity and slot accounting unchanged.
    pub fn a06_insert_order_head_default(
        &mut self,
        order: Order,
        data: &GameData,
    ) -> Result<bool, SimError> {
        self.a06_insert_order_at_head(order, data)
    }

    /// `0x005b71f0`: issue `order` at the list head.  When the list is empty
    /// the decompiled path uses `FUN_005b6cd0` (list initialisation); when it
    /// is non-empty it uses `0x005b7520` (head insertion).  Both make the new
    /// order the head, so an idle unit starts the order directly and a busy
    /// unit defers to [`Self::a06_insert_order_at_head`].
    pub fn a06_issue_order_or_start(
        &mut self,
        order: Order,
        data: &GameData,
    ) -> Result<bool, SimError> {
        if a06_order_list_len(self) == 0 {
            if matches!(order, Order::Idle) {
                return Ok(false);
            }
            let used_slot = a06_uses_queue_slot(data, &order);
            self.order = order;
            self.order_row = self.order.order_id();
            Ok(used_slot)
        } else {
            self.a06_insert_order_at_head(order, data)
        }
    }
}

impl World {
    /// `0x005b76a0`: reject a non-null second argument, then forward
    /// `(order, 0, 0, 0xe4, flag)` to `FUN_005b76f0`.  In the typed model the
    /// absent argument is `target: None, pos: None`; the row goes through the
    /// recovered dispatcher.  The final `FUN_005b76f0` flag selects which
    /// queued orders are trimmed before issuing and has no separate typed
    /// state of its own.
    pub fn a06_issue_order_without_argument(
        &mut self,
        unit_id: u32,
        order_id: u8,
    ) -> Result<(), SimError> {
        self.issue_order_id(unit_id, order_id, None, None)
    }

    /// `0x005b7b90`: forward an order row, its target and its position to
    /// `FUN_005b76f0` with the fixed tag byte `0xe4`.
    pub fn a06_issue_order_row(
        &mut self,
        unit_id: u32,
        order_id: u8,
        target: Option<u32>,
        pos: Option<(i32, i32)>,
    ) -> Result<(), SimError> {
        self.issue_order_id(unit_id, order_id, target, pos)
    }

    /// `0x005b7d40`: set bit 0 of `CUnit+0x93`, issue the row through
    /// `FUN_005b76f0` with the position and the queue flag set, then call
    /// `FUN_005b8090`.  The `FUN_005b8090` decompilation failed, so only the
    /// recovered flag write and order dispatch are modelled here.
    pub fn a06_issue_order_set_flags_93_bit0(
        &mut self,
        unit_id: u32,
        order_id: u8,
        pos: Option<(i32, i32)>,
    ) -> Result<(), SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        self.units[i].flags_93 |= 1;
        self.issue_order_id(unit_id, order_id, None, pos)
    }

    /// `0x005b7e60`: command-card dispatcher.  The selector chooses which
    /// caller fields become the serialized position and target:
    ///
    /// * `0`: position `(0, 0)`, no target.
    /// * `1`: delegate to `FUN_005b7dd0`, which rejects a non-null second
    ///   argument and issues with no arguments.
    /// * `2`, `3`: the caller position, no target (they differ only in the
    ///   `0xe4` tag byte, which has no typed equivalent).
    /// * `4`: the caller position and target.
    ///
    /// Every recognized case sets `CUnit+0x93` bit 0 before `FUN_005b76f0`
    /// and then calls `FUN_005b8090` (not decompiled).  Returns `false` for an
    /// unknown selector, which the decompiled default path returns from
    /// without changing the unit.
    pub fn a06_issue_order_variant(
        &mut self,
        unit_id: u32,
        variant: u8,
        order_id: u8,
        target: Option<u32>,
        pos: Option<(i32, i32)>,
    ) -> Result<bool, SimError> {
        let (target, pos) = match variant {
            0 => (None, Some((0, 0))),
            1 => {
                if target.is_some() {
                    // `FUN_0058b640` is the retail fatal path for a non-null
                    // second argument; the typed wrapper reports it instead.
                    return Err(SimError::InvalidTarget);
                }
                (None, None)
            }
            2 | 3 => (None, pos),
            4 => (target, pos),
            _ => return Ok(false),
        };
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        self.units[i].flags_93 |= 1;
        self.issue_order_id(unit_id, order_id, target, pos)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests_agent06 {
    use super::*;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use std::sync::Arc;

    fn a06_world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn a06_marine(world: &mut World) -> u32 {
        let index = world.spawn(0, uid::MARINE, 320, 320).unwrap();
        world.units[index].id
    }

    #[test]
    fn a06_clear_order_list_removes_current_and_queued_orders() {
        let mut world = a06_world();
        let data = world.data.clone();
        let id = a06_marine(&mut world);
        let index = world.index_of(id).unwrap();
        world.issue(id, Order::Move { pos: (400, 400) }).unwrap();
        world.queue_order(id, Order::Stop).unwrap();
        // A row with highlight -1 is linked but does not consume a slot.
        assert_eq!(data.order(106).unwrap().highlight, -1);
        world.units[index].queue.push_back(Order::HoldPosition);

        let removed = world.units[index].a06_clear_order_list(data.as_ref());
        assert_eq!(removed, 2, "Move and Stop consume slots, Hold does not");
        assert_eq!(world.units[index].order, Order::Idle);
        assert_eq!(world.units[index].order_row, 0);
        assert!(world.units[index].queue.is_empty());
    }

    #[test]
    fn a06_pop_order_head_promotes_the_next_queued_order() {
        let mut world = a06_world();
        let data = world.data.clone();
        let id = a06_marine(&mut world);
        let index = world.index_of(id).unwrap();
        let first = Order::Move { pos: (400, 400) };
        world.issue(id, first.clone()).unwrap();
        world.queue_order(id, Order::Stop).unwrap();
        world.units[index].queue.push_back(Order::HoldPosition);

        let (removed, used_slot) = world.units[index]
            .a06_pop_order_head(data.as_ref())
            .expect("head node");
        assert_eq!(removed, first);
        assert!(used_slot);
        assert_eq!(world.units[index].order, Order::Stop);
        assert_eq!(world.units[index].order_row, 1);

        let (removed, used_slot) = world.units[index]
            .a06_pop_order_head(data.as_ref())
            .expect("Stop node");
        assert_eq!(removed, Order::Stop);
        assert!(used_slot);
        assert_eq!(world.units[index].order, Order::HoldPosition);
        assert_eq!(world.units[index].order_row, 106);

        let (removed, used_slot) = world.units[index]
            .a06_pop_order_head(data.as_ref())
            .expect("Hold node");
        assert_eq!(removed, Order::HoldPosition);
        assert!(!used_slot, "row 106 has highlight -1");
        assert_eq!(world.units[index].order, Order::Idle);
        assert_eq!(world.units[index].order_row, 0);

        assert!(
            world.units[index]
                .a06_pop_order_head(data.as_ref())
                .is_none()
        );
    }

    #[test]
    fn a06_remove_order_node_handles_queued_current_and_missing_nodes() {
        let mut world = a06_world();
        let data = world.data.clone();
        let id = a06_marine(&mut world);
        let index = world.index_of(id).unwrap();
        let current = Order::Move { pos: (400, 400) };
        let queued = Order::Move { pos: (500, 500) };
        world.issue(id, current.clone()).unwrap();
        world.queue_order(id, Order::Stop).unwrap();
        world.units[index].queue.push_back(queued.clone());
        world.units[index].queue.push_back(Order::HoldPosition);

        assert_eq!(
            world.units[index].a06_remove_order_node(&Order::Stop, data.as_ref()),
            Some(true),
            "row 1 consumes a slot"
        );
        assert_eq!(
            world.units[index].a06_remove_order_node(&Order::HoldPosition, data.as_ref()),
            Some(false),
            "row 106 is linked without a slot"
        );
        assert_eq!(
            world.units[index].a06_remove_order_node(&current, data.as_ref()),
            Some(true)
        );
        assert_eq!(
            world.units[index].order, queued,
            "removing the head promotes its successor"
        );
        assert_eq!(
            world.units[index].a06_remove_order_node(&Order::Stop, data.as_ref()),
            None
        );
        assert_eq!(world.units[index].order_row, 6);
    }

    #[test]
    fn a06_insert_order_at_head_caps_the_list_at_nine_nodes() {
        let mut world = a06_world();
        let data = world.data.clone();
        let id = a06_marine(&mut world);
        let index = world.index_of(id).unwrap();

        for i in 0..A06_ORDER_LIST_CAPACITY {
            let used_slot = world.units[index]
                .a06_insert_order_at_head(
                    Order::Move {
                        pos: (400 + i as i32, 400),
                    },
                    data.as_ref(),
                )
                .unwrap();
            assert!(used_slot, "Move row 6 consumes a slot");
        }
        assert_eq!(world.units[index].queue.len(), 8);
        assert_eq!(world.units[index].order, Order::Move { pos: (408, 400) });
        let full = world.units[index]
            .a06_insert_order_at_head(Order::Move { pos: (900, 900) }, data.as_ref());
        assert_eq!(full, Err(SimError::QueueFull));
        assert_eq!(world.units[index].queue.len(), 8);

        let other = a06_marine(&mut world);
        let other_index = world.index_of(other).unwrap();
        let used_slot = world.units[other_index]
            .a06_insert_order_at_head(Order::HoldPosition, data.as_ref())
            .unwrap();
        assert!(!used_slot, "row 106 does not consume a slot");
        assert_eq!(world.units[other_index].order, Order::HoldPosition);
    }

    #[test]
    fn a06_insert_order_head_default_matches_the_delegated_wrapper() {
        let mut world = a06_world();
        let data = world.data.clone();
        let id = a06_marine(&mut world);
        let index = world.index_of(id).unwrap();

        assert_eq!(
            world.units[index].a06_insert_order_head_default(Order::Stop, data.as_ref()),
            Ok(true)
        );
        assert_eq!(world.units[index].order, Order::Stop);
        assert_eq!(world.units[index].order_row, 1);

        assert_eq!(
            world.units[index]
                .a06_insert_order_head_default(Order::Move { pos: (400, 400) }, data.as_ref()),
            Ok(true)
        );
        assert_eq!(world.units[index].order, Order::Move { pos: (400, 400) });
        assert_eq!(world.units[index].queue.len(), 1);
        assert_eq!(world.units[index].queue.front(), Some(&Order::Stop));
    }

    #[test]
    fn a06_issue_order_or_start_uses_the_empty_list_path_then_queues() {
        let mut world = a06_world();
        let data = world.data.clone();
        let id = a06_marine(&mut world);
        let index = world.index_of(id).unwrap();

        assert_eq!(
            world.units[index]
                .a06_issue_order_or_start(Order::Move { pos: (400, 400) }, data.as_ref()),
            Ok(true)
        );
        assert_eq!(world.units[index].order, Order::Move { pos: (400, 400) });
        assert!(world.units[index].queue.is_empty());
        assert_eq!(world.units[index].order_row, 6);

        assert_eq!(
            world.units[index].a06_issue_order_or_start(Order::Stop, data.as_ref()),
            Ok(true)
        );
        assert_eq!(world.units[index].order, Order::Stop);
        assert_eq!(
            world.units[index].queue.front(),
            Some(&Order::Move { pos: (400, 400) })
        );
        assert_eq!(world.units[index].order_row, 1);
    }

    #[test]
    fn a06_issue_order_without_argument_dispatches_the_recovered_row() {
        let mut world = a06_world();
        let id = a06_marine(&mut world);
        world.a06_issue_order_without_argument(id, 106).unwrap();
        let index = world.index_of(id).unwrap();
        assert_eq!(world.units[index].order, Order::HoldPosition);
        assert_eq!(world.units[index].order_row, 106);

        assert_eq!(
            world.a06_issue_order_without_argument(9999, 106),
            Err(SimError::BadUnit)
        );
    }

    #[test]
    fn a06_issue_order_row_forwards_target_and_position() {
        let mut world = a06_world();
        let id = a06_marine(&mut world);
        world
            .a06_issue_order_row(id, 6, None, Some((400, 400)))
            .unwrap();
        let index = world.index_of(id).unwrap();
        assert_eq!(world.units[index].order, Order::Move { pos: (400, 400) });
        assert_eq!(world.units[index].order_row, 6);

        assert_eq!(
            world.a06_issue_order_row(9999, 6, None, Some((400, 400))),
            Err(SimError::BadUnit)
        );
    }

    #[test]
    fn a06_issue_order_set_flags_93_bit0_sets_the_flag_before_dispatch() {
        let mut world = a06_world();
        let id = a06_marine(&mut world);
        let index = world.index_of(id).unwrap();
        world.units[index].flags_93 = 0;

        world
            .a06_issue_order_set_flags_93_bit0(id, 6, Some((400, 400)))
            .unwrap();
        assert_eq!(world.units[index].flags_93 & 1, 1);
        assert_eq!(world.units[index].order, Order::Move { pos: (400, 400) });
        assert_eq!(world.units[index].order_row, 6);

        assert_eq!(
            world.a06_issue_order_set_flags_93_bit0(9999, 6, None),
            Err(SimError::BadUnit)
        );
    }

    #[test]
    fn a06_issue_order_variant_selects_the_recovered_argument_shape() {
        let mut world = a06_world();
        let id = a06_marine(&mut world);
        let index = world.index_of(id).unwrap();

        assert_eq!(
            world.a06_issue_order_variant(id, 9, 6, None, Some((400, 400))),
            Ok(false),
            "the default selector returns without issuing"
        );
        assert_eq!(world.units[index].order, Order::Idle);

        assert_eq!(
            world.a06_issue_order_variant(id, 0, 106, None, Some((900, 900))),
            Ok(true),
            "selector 0 issues the row with a zero position"
        );
        assert_eq!(world.units[index].flags_93 & 1, 1);
        assert_eq!(world.units[index].order, Order::HoldPosition);

        let other = a06_marine(&mut world);
        assert_eq!(
            world.a06_issue_order_variant(other, 1, 106, Some(123), None),
            Err(SimError::InvalidTarget),
            "selector 1 rejects a second argument"
        );
        let other_index = world.index_of(other).unwrap();
        assert_eq!(world.units[other_index].order, Order::Idle);
        assert_eq!(world.units[other_index].flags_93 & 1, 0);

        let third = a06_marine(&mut world);
        assert_eq!(
            world.a06_issue_order_variant(third, 4, 6, None, Some((400, 400))),
            Ok(true)
        );
        let third_index = world.index_of(third).unwrap();
        assert_eq!(
            world.units[third_index].order,
            Order::Move { pos: (400, 400) }
        );
        assert_eq!(world.units[third_index].flags_93 & 1, 1);
    }
}
