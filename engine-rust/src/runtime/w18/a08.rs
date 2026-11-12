#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave18/agent08.rs
// ===========================================================================
// Wave 18 agent 08: intrusive list / hash-bucket / refcount helpers (runtime).
// Doc anchors: 0x0053d150 0x0053d250 0x0053d320 0x0053d420 0x0053d520 0x0053e750 0x0053e7e0 0x0053ead0 0x0053eb70 0x0053ec10 0x00739f30.

/// Hash bucket index used by the bucket tables below (multiplicative mix).
pub fn w18a08_bucket_hash(key: u32, bucket_count: u32) -> u32 {
    let mut h = (key >> 16 ^ key).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    (h >> 16 ^ h) & bucket_count.wrapping_sub(1)
}

/// Bucket node: `next` at +0, `key` at +4, payload at +8.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W18a08Node {
    pub next: Option<usize>,
    pub key: u32,
}

fn w18a08_find(table: &[Option<usize>], nodes: &[W18a08Node], key: u32) -> Option<usize> {
    let mut cur = table[w18a08_bucket_hash(key, table.len() as u32) as usize];
    while let Some(i) = cur {
        if nodes[i].key == key {
            return Some(i);
        }
        cur = nodes[i].next;
    }
    None
}

/// `0x0053d320`: bucket lookup by key; `Some(node)` when found (caller adds 8
/// for the payload).  An empty table triggers allocation in retail (not
/// modelled, returns `None`); a miss falls into the insert path (`None`).
pub fn w18a08_table_find_or_insert(
    table: &[Option<usize>],
    nodes: &[W18a08Node],
    key: u32,
) -> Option<usize> {
    if table.is_empty() {
        return None;
    }
    w18a08_find(table, nodes, key)
}

/// `0x0053e750`: same lookup; a hit goes to the erase path (returned node),
/// a miss or missing table does nothing (retail returns 0 in both cases).
pub fn w18a08_table_find_for_erase(
    table: &[Option<usize>],
    nodes: &[W18a08Node],
    key: u32,
) -> Option<usize> {
    if table.is_empty() {
        return None;
    }
    w18a08_find(table, nodes, key)
}

/// `0x0053ead0`: scans every bucket head; returns the indices of non-empty
/// buckets (each one is cleared by the callee in retail).  Empty table: none.
pub fn w18a08_nonempty_buckets(table: &[Option<usize>]) -> Vec<usize> {
    table
        .iter()
        .enumerate()
        .filter(|(_, h)| h.is_some())
        .map(|(i, _)| i)
        .collect()
}

/// Decodes a tagged pointer word: bit 0 set means the pointer is
/// `word & !1` (0 = null); otherwise the object is inline at the slot.
pub fn w18a08_tagged_target(word: u32) -> Option<u32> {
    if word & 1 != 0 {
        let p = word & !1;
        if p == 0 { None } else { Some(p) }
    } else {
        Some(word)
    }
}

/// `0x0053d150`: if `active`, resets the owned tagged word to 1.  A word
/// whose bit 0 is already set aborts in retail (`Err`).
pub fn w18a08_reset_tagged_slot(slot: &mut u32, active: bool) -> Result<(), ()> {
    if active {
        if *slot & 1 != 0 {
            return Err(());
        }
        *slot = 1;
    }
    Ok(())
}

/// `0x0053d250`: resets the sentinel `[next, prev, owner]` to point at
/// itself with a zeroed third word, returning the old `prev` (`None` if the
/// list was already empty).
pub fn w18a08_list_reset(head: &mut [u32; 3], self_addr: u32) -> Option<u32> {
    let old = head[1];
    head[1] = self_addr;
    head[0] = self_addr;
    head[2] = 0;
    if old == self_addr { None } else { Some(old) }
}

/// `0x0053e7e0`: unlinks node `n` of a doubly linked arena
/// (`links[i] = (next, prev)`): `next.prev = prev; prev.next = next`.
pub fn w18a08_list_unlink(links: &mut [(usize, usize)], n: usize) {
    let (next, prev) = links[n];
    links[next].1 = prev;
    links[prev].0 = next;
}

/// `0x0053ec10`: clears a `[ptr, aux]` pair, invoking `release` on a non-zero `ptr`.
pub fn w18a08_clear_pair(pair: &mut [u32; 2], mut release: impl FnMut(u32)) {
    if pair[0] != 0 {
        release(pair[0]);
    }
    pair[1] = 0;
    pair[0] = 0;
}

/// Actions requested by [`w18a08_release_shared`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct W18a08Release {
    pub dispose: bool,
    pub destroy: bool,
}

/// `0x0053eb70`: drops a strong reference of a shared-pointer control block
/// (`use` at +4, `weak` at +8); the last strong ref disposes (vtable +8) and
/// the last weak ref then destroys (vtable +0xc).  `None` is a null block.
pub fn w18a08_release_shared(block: Option<(&mut i32, &mut i32)>) -> W18a08Release {
    let mut r = W18a08Release::default();
    if let Some((strong, weak)) = block {
        let old = *strong;
        *strong = old.wrapping_sub(1);
        if old == 1 {
            r.dispose = true;
            let w = *weak;
            *weak = w.wrapping_sub(1);
            r.destroy = w == 1;
        }
    }
    r
}

/// `0x0053d420`: bumps the global counter at the `f029c8` slot.
pub fn w18a08_bump_counter_a(counter: &mut u32) {
    *counter = counter.wrapping_add(1);
}

/// `0x0053d520`: identical bump of the same counter.
pub fn w18a08_bump_counter_b(counter: &mut u32) {
    *counter = counter.wrapping_add(1);
}

/// `0x00739f30`: returns `Ok(0)` when `check(0x21)` is true; otherwise the
/// failure label `"GetTurnPackets"` is raised.
pub fn w18a08_get_turn_packets(mut check: impl FnMut(u32) -> bool) -> Result<u32, &'static str> {
    if check(0x21) {
        Ok(0)
    } else {
        Err("GetTurnPackets")
    }
}

#[cfg(test)]
mod tests_wave18_agent08 {
    use super::*;

    #[test]
    fn hash_and_find() {
        assert_eq!(w18a08_bucket_hash(0, 8), 0);
        assert!(w18a08_bucket_hash(12345, 16) < 16);
        let mut table = vec![None; 8];
        let nodes = vec![W18a08Node {
            next: None,
            key: 77,
        }];
        table[w18a08_bucket_hash(77, 8) as usize] = Some(0);
        assert_eq!(w18a08_table_find_or_insert(&table, &nodes, 77), Some(0));
        assert_eq!(w18a08_table_find_for_erase(&table, &nodes, 78), None);
        assert_eq!(w18a08_table_find_for_erase(&[], &nodes, 77), None);
        assert_eq!(w18a08_nonempty_buckets(&table).len(), 1);
    }

    #[test]
    fn tagged_and_lists() {
        assert_eq!(w18a08_tagged_target(1), None);
        assert_eq!(w18a08_tagged_target(0x1001), Some(0x1000));
        assert_eq!(w18a08_tagged_target(0x1000), Some(0x1000));
        let mut s = 0;
        assert!(w18a08_reset_tagged_slot(&mut s, true).is_ok());
        assert_eq!(s, 1);
        assert!(w18a08_reset_tagged_slot(&mut s, true).is_err());
        let mut h = [5, 9, 7];
        assert_eq!(w18a08_list_reset(&mut h, 100), Some(9));
        assert_eq!(h, [100, 100, 0]);
        assert_eq!(w18a08_list_reset(&mut h, 100), None);
        let mut l = vec![(1, 2), (2, 0), (0, 1)];
        w18a08_list_unlink(&mut l, 1);
        assert_eq!(l[2].1, 0);
        assert_eq!(l[0].0, 2);
    }

    #[test]
    fn misc() {
        let mut p = [4, 9];
        let mut got = 0;
        w18a08_clear_pair(&mut p, |v| got = v);
        assert_eq!((p, got), ([0, 0], 4));
        let (mut a, mut b) = (1, 1);
        let r = w18a08_release_shared(Some((&mut a, &mut b)));
        assert!(r.dispose && r.destroy);
        let (mut a, mut b) = (2, 1);
        assert_eq!(
            w18a08_release_shared(Some((&mut a, &mut b))),
            W18a08Release::default()
        );
        assert_eq!(w18a08_release_shared(None), W18a08Release::default());
        let mut c = u32::MAX;
        w18a08_bump_counter_a(&mut c);
        w18a08_bump_counter_b(&mut c);
        assert_eq!(c, 1);
        assert_eq!(w18a08_get_turn_packets(|x| x == 0x21), Ok(0));
        assert_eq!(w18a08_get_turn_packets(|_| false), Err("GetTurnPackets"));
    }
}
