#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave16/agent08.rs
// ===========================================================================
// Wave 16 translation batch 08.
//
// Skipped: FUN_005da6c0 (bare call), FUN_005dc750 (noreturn branch),
// FUN_00633970 (bare call pair), FUN_006515e0 (ends in noreturn memset),
// FUN_00656850 / FUN_006568e0 / FUN_00656970 (list walk, noreturn),
// FUN_006876f0 (noreturn), FUN_006afe30 (noreturn), FUN_006e38c0 (SEH,
// noreturn), FUN_006e3ea0 / FUN_006e4de0 (global-gated call chains),
// FUN_006f2bb0 (allocator / noreturn), FUN_00712c20 (bare list loop).

/// Action chosen by [`w16a08_countdown_tick`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W16a08Tick {
    /// Counter was already zero: nothing happens.
    Idle,
    /// The 0x57_8d90-style callee path (bit 3 of counter, or expired with flag bit 3).
    Primary,
    /// The 0x57_ccd0-style callee path.
    Secondary,
}

/// 0x005d9580: per-tick decrement of the byte at +0xf of an object.
/// `mask` is the byte at +0xc, `flags` the byte at +0xe, `global_mask` the
/// DAT global; if `mask & global_mask == 0` the counter is cleared.
pub fn w16a08_countdown_tick(counter: &mut u8, mask: u8, flags: u8, global_mask: u8) -> W16a08Tick {
    if *counter == 0 {
        return W16a08Tick::Idle;
    }
    *counter = counter.wrapping_sub(1);
    if mask & global_mask == 0 {
        *counter = 0;
    }
    let c = *counter;
    if c & 8 != 0 || (c == 0 && flags & 8 != 0) {
        W16a08Tick::Primary
    } else {
        W16a08Tick::Secondary
    }
}

/// 0x00687080: OR `bits` into a `w` x `h` rectangle at (`x`,`y`) of a u32
/// grid whose row stride is `stride` (the u16 at map-state +0xe4).
pub fn w16a08_grid_or_rect(
    grid: &mut [u32],
    stride: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    bits: u32,
) {
    for row in 0..h {
        let base = (y + row) * stride + x;
        for cell in grid.iter_mut().skip(base).take(w) {
            *cell |= bits;
        }
    }
}

/// 0x006877d0: AND-NOT `bits` over a `w` x `h` rectangle of a u32 grid
/// (same layout as [`w16a08_grid_or_rect`]).
pub fn w16a08_grid_clear_rect(
    grid: &mut [u32],
    stride: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    bits: u32,
) {
    for row in 0..h {
        let base = (y + row) * stride + x;
        for cell in grid.iter_mut().skip(base).take(w) {
            *cell &= !bits;
        }
    }
}

/// 0x006c1f70: scan a zero-terminated list; return the first element for
/// which `pred(elem, ctx)` is nonzero, else 0.  With `pred == None` the
/// retail code returns the first element unconditionally.
pub fn w16a08_find_in_list(list: &[i32], pred: Option<&dyn Fn(i32) -> i32>) -> i32 {
    let mut it = list.iter().copied();
    let first = it.next().unwrap_or(0);
    let Some(pred) = pred else { return first };
    let mut cur = first;
    while cur != 0 {
        if pred(cur) != 0 {
            return cur;
        }
        cur = it.next().unwrap_or(0);
    }
    0
}

/// 0x006bf480: collect the active player ids (< 8) from `players` and pick
/// one using the random value `rand`; `None` when the list is empty.
pub fn w16a08_pick_random_player(players: &[u32], rand: u32) -> Option<u32> {
    let v: Vec<u32> = players.iter().copied().filter(|&p| p < 8).collect();
    if v.is_empty() {
        None
    } else {
        Some(v[(rand as usize) % v.len()])
    }
}

/// 0x00728190: for each node `(link_nonnull, type_byte, owner_byte)` return
/// the indices that pass the retail filter and trigger the callee.
/// `type_enabled[type_byte]` and `owner_table[owner_byte]` must be nonzero.
pub fn w16a08_filter_nodes(
    nodes: &[(bool, u8, u8)],
    type_enabled: &[u8; 256],
    owner_table: &[u8; 256],
) -> Vec<usize> {
    nodes
        .iter()
        .enumerate()
        .filter(|&(_, &(link, t, o))| {
            link && type_enabled[t as usize] != 0 && owner_table[o as usize] != 0
        })
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests_wave16_agent08 {
    use super::*;

    #[test]
    fn tick() {
        let mut c = 0;
        assert_eq!(w16a08_countdown_tick(&mut c, 1, 0, 1), W16a08Tick::Idle);
        let mut c = 5;
        assert_eq!(
            w16a08_countdown_tick(&mut c, 1, 0, 1),
            W16a08Tick::Secondary
        );
        assert_eq!(c, 4);
        let mut c = 9;
        assert_eq!(w16a08_countdown_tick(&mut c, 1, 0, 1), W16a08Tick::Primary);
        let mut c = 9;
        assert_eq!(w16a08_countdown_tick(&mut c, 0, 8, 1), W16a08Tick::Primary);
        assert_eq!(c, 0);
        let mut c = 1;
        assert_eq!(
            w16a08_countdown_tick(&mut c, 1, 0, 1),
            W16a08Tick::Secondary
        );
    }

    #[test]
    fn rects() {
        let mut g = vec![0u32; 16];
        w16a08_grid_or_rect(&mut g, 4, 1, 1, 2, 2, 0x5);
        assert_eq!(g[5], 5);
        assert_eq!(g[10], 5);
        assert_eq!(g[0], 0);
        assert_eq!(g[7], 0);
        w16a08_grid_clear_rect(&mut g, 4, 1, 1, 1, 2, 0x1);
        assert_eq!(g[5], 4);
        assert_eq!(g[9], 4);
        assert_eq!(g[6], 5);
    }

    #[test]
    fn find() {
        let l = [3, 4, 5, 0];
        assert_eq!(w16a08_find_in_list(&l, Some(&|v| (v == 4) as i32)), 4);
        assert_eq!(w16a08_find_in_list(&l, Some(&|_| 0)), 0);
        assert_eq!(w16a08_find_in_list(&l, None), 3);
    }

    #[test]
    fn pick() {
        assert_eq!(w16a08_pick_random_player(&[], 3), None);
        assert_eq!(w16a08_pick_random_player(&[1, 9, 4], 3), Some(4));
    }

    #[test]
    fn filter() {
        let mut t = [0u8; 256];
        let mut o = [0u8; 256];
        t[2] = 1;
        o[3] = 1;
        let n = [(true, 2, 3), (false, 2, 3), (true, 1, 3)];
        assert_eq!(w16a08_filter_nodes(&n, &t, &o), vec![0]);
    }
}
