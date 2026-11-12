#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave16/agent10.rs
// ===========================================================================
// Wave 16 translation batch 10.
//
// Skipped rows (reason in each line):
// FUN_0061b5e0 vector buffer release, compiler EH glue
// FUN_00665de0 bare call into noreturn
// FUN_00666230 bare call into noreturn
// FUN_00679410 std::map erase with unresolved rotations
// FUN_006c52e0 range check then noreturn
// FUN_0078931a mid-function fragment, noreturn snprintf
// FUN_00789dc0 call chain ending noreturn
// FUN_0078a470 call chain ending noreturn
// FUN_0078a577 call chain of unresolved callees
// FUN_0078b2a0 ends in noreturn snprintf
// FUN_0078c440 call chain ending noreturn
// FUN_0079ec90 ends in noreturn call
// FUN_007a9f30 loops ending in noreturn calls
// FUN_007ab790 global reset ending noreturn
// FUN_0084b900 single virtual call thunk
// FUN_00c5dea0 std::map insert rebalance, runtime.rs territory
// FUN_00c5e000 duplicate of the map insert rebalance

/// Predicate at 0x0078d650.  Parameters are the already evaluated inputs:
/// `is_local` = (global flag != 0) || (player byte == local player id);
/// `c`..`h` are the results of the five/six unresolved callee predicates in
/// call order (c=005d5c80, d=005b5810, e=005b5970, f=005b5980, g=005b6160,
/// h=005b6470).
pub fn w16a10_0078d650_predicate(
    is_local: bool,
    c: bool,
    d: bool,
    e: bool,
    f: bool,
    g: bool,
    h: bool,
) -> bool {
    if !is_local {
        return false;
    }
    if c {
        return !e;
    }
    if !d {
        return !e || f;
    }
    !e || f || g || h
}

/// Slot permutation computed at the top of 0x00794820.  `flags` is the byte
/// returned by the first callee; returns the four-entry `local_18` array.
pub fn w16a10_00794820_slot_permutation(flags: u8) -> [u32; 4] {
    let b9 = (flags & 1) == 0;
    let mut l = [!b9 as u32, b9 as u32, 2, 3];
    let mut u4 = b9 as u32;
    if flags & 2 != 0 {
        l[1] = 2;
        u4 = 2;
        l[2] = b9 as u32;
    }
    let prev2 = l[2];
    if flags & 4 != 0 {
        l[2] = 3;
        l[3] = prev2;
    }
    let mut u5 = !b9 as u32;
    if flags & 8 != 0 {
        u5 = l[3];
        l[0] = l[3];
        l[3] = !b9 as u32;
    }
    let (mut u6, mut u8) = (u5, u4);
    if flags & 0x10 != 0 {
        u6 = u4;
        u8 = u5;
        l[0] = u4;
        l[1] = u5;
    }
    let (u4b, u5b) = (u6, u8);
    let (mut u6, mut u8) = (u4b, u5b);
    if flags & 0x20 != 0 {
        u6 = u5b;
        u8 = u4b;
        l[0] = u5b;
        l[1] = u4b;
    }
    // bit 6
    let (u4c, u5c) = (u6, u8);
    let (mut u6c, mut u8c) = (u4c, u5c);
    if flags & 0x40 != 0 {
        u6c = u5c;
        u8c = u4c;
        l[0] = u5c;
        l[1] = u4c;
    }
    if flags & 0x80 != 0 {
        l[0] = u8c;
        l[1] = u6c;
    }
    l
}

/// Tail of 0x00794820: color-table lookup.  `a`/`b` are the two ratio inputs
/// (callees 00588d60 and 005b4b30; `b` must be nonzero), `flags` the 005b4c30
/// byte, `map_even`/`map_odd` the two string tables (s_00f308d0 / s_00f308a8,
/// chosen by `flags & 1`), `remap` the 011b20b8 byte table and `palette` the
/// 00fa9068 dword table.  Returns the dword written to the callee 00a8fc10.
#[allow(clippy::too_many_arguments)]
pub fn w16a10_00794820_color_lookup(
    a: i32,
    b: i32,
    flags: u8,
    index: usize,
    map_even: &[u8],
    map_odd: &[u8],
    remap: &[u8],
    palette: &[u32],
) -> Option<u32> {
    let perm = w16a10_00794820_slot_permutation(flags);
    let table = if flags & 1 != 0 { map_odd } else { map_even };
    let row = a.checked_mul(9)?.checked_div(b)?;
    let pos = (row as usize)
        .checked_mul(4)?
        .checked_add(*perm.get(index)? as usize)?;
    let c = *table.get(pos)?;
    let r = *remap.get(c as usize)?;
    palette.get(r as usize).copied()
}

/// Player-state update, first half of 0x007ab4f0.  `state` is the byte table
/// at 011b8704 (indexed by player), `input` the dword 01165d00, `local` the
/// local player id, `alt_mode` = (0119f7bc != 0), `members` the sequence
/// yielded by the 0065b0c0/0065b0e0 iterator, `present` the byte table at
/// 011b8734.  Returns true when the local slot was promoted 0 -> 2 (the
/// caller then stores 0x2d into 011b86fc).
pub fn w16a10_007ab4f0_update_states(
    state: &mut [u8; 8],
    input: u32,
    local: usize,
    alt_mode: bool,
    members: &[usize],
    present: &[bool; 8],
) -> bool {
    let mut local_promoted = false;
    if !alt_mode {
        if input & 0x80 != 0 {
            state[local] = 3;
        } else if input & 0x40 != 0 {
            state[local] = 2;
        }
        if input & 0x100 != 0 && state[local] > 1 {
            state[local] = 0;
        }
    }
    if !members.iter().any(|&u| state[u] > 2) {
        return false;
    }
    for &u in members {
        if state[u] == 0 {
            state[u] = 2;
            if u == local {
                local_promoted = true;
            }
        }
    }
    let mut total = 0u8;
    let mut high = 0u8;
    for u in 0..8 {
        if present[u] {
            total = total.wrapping_add(1);
            if state[u] > 2 {
                high = high.wrapping_add(1);
            }
        }
    }
    if total == high {
        for u in 0..8 {
            if present[u] {
                state[u] = 5;
            }
        }
    }
    local_promoted
}

#[cfg(test)]
mod tests_wave16_agent10 {
    use super::*;

    #[test]
    fn predicate_paths() {
        assert!(!w16a10_0078d650_predicate(
            false, true, true, false, false, false, false
        ));
        assert!(w16a10_0078d650_predicate(
            true, true, false, false, false, false, false
        ));
        assert!(!w16a10_0078d650_predicate(
            true, true, false, true, false, false, false
        ));
        assert!(w16a10_0078d650_predicate(
            true, false, false, true, true, false, false
        ));
        assert!(!w16a10_0078d650_predicate(
            true, false, true, true, false, false, false
        ));
        assert!(w16a10_0078d650_predicate(
            true, false, true, true, false, false, true
        ));
    }

    #[test]
    fn permutation_identity_and_swap() {
        assert_eq!(w16a10_00794820_slot_permutation(0), [0, 1, 2, 3]);
        assert_eq!(w16a10_00794820_slot_permutation(1), [1, 0, 2, 3]);
        assert_eq!(w16a10_00794820_slot_permutation(2)[1], 2);
    }

    #[test]
    fn color_lookup() {
        let m = [0u8, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3];
        let remap = [0u8, 1, 2, 3];
        let pal = [10u32, 20, 30, 40];
        let v = w16a10_00794820_color_lookup(0, 1, 0, 1, &m, &m, &remap, &pal);
        assert_eq!(v, Some(20));
        assert_eq!(
            w16a10_00794820_color_lookup(0, 0, 0, 1, &m, &m, &remap, &pal),
            None
        );
    }

    #[test]
    fn update_states() {
        let mut s = [0u8; 8];
        let present = [true, true, false, false, false, false, false, false];
        let p = w16a10_007ab4f0_update_states(&mut s, 0x80, 0, false, &[0, 1], &present);
        assert!(!p);
        assert_eq!(s[0], 3);
        assert_eq!(s[1], 2);
        let mut s = [0u8; 8];
        let p = w16a10_007ab4f0_update_states(&mut s, 0x40, 0, true, &[0], &present);
        assert!(!p);
        let mut s = [0, 0, 0, 0, 0, 0, 0, 3];
        let p = w16a10_007ab4f0_update_states(&mut s, 0, 1, true, &[0, 1, 7], &present);
        assert!(p);
        assert_eq!(s[1], 2);
    }
}
