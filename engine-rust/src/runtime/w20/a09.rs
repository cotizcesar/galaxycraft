#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave20/agent09.rs
// ===========================================================================
// Wave 20 agent 09: small vector/list helpers and unit-target filters.

/// `0x00565c30`: source index of the pushed element when the argument
/// aliases the vector storage (stride `0x24`): `(arg - base) / 0x24`; the
/// element is re-read from the vector only when that index is `< len`.
pub fn w20a09_push_alias_source(base: u32, len: u32, arg: u32) -> Option<u32> {
    let idx = (arg.wrapping_sub(base) as i32).wrapping_div(0x24) as u32;
    if idx < len { Some(idx) } else { None }
}

/// `0x00565c30`: growth step of the capacity word (low 31 bits capacity,
/// top bit preserved).  Returns `None` when the vector is not full
/// (`len != cap`) or on overflow; otherwise the new capacity word.
pub fn w20a09_push_grow(len: u32, cap_word: u32) -> Option<u32> {
    let cap = cap_word & 0x7fff_ffff;
    if len != cap || cap.checked_add(1).is_none() {
        return None;
    }
    let half = (cap_word >> 1) & 0x3fff_ffff;
    let step = if cap.checked_add(half).is_some_and(|s| cap < s) {
        half
    } else {
        1
    };
    let new_cap = step.wrapping_add(cap);
    Some(((new_cap ^ cap_word) & 0x7fff_ffff) ^ cap_word)
}

/// `0x00566340`: circular list sentinel reset.  Node = `[next, prev, size]`
/// words; sets prev=self, next=self, size=0 and returns the old prev link
/// (nodes to free when it differs from self).
pub fn w20a09_list_reset_a(node: &mut [u32; 3], self_addr: u32) -> u32 {
    let old = node[1];
    node[1] = self_addr;
    node[0] = self_addr;
    node[2] = 0;
    old
}

/// `0x00567210`: same sentinel reset as `0x00566340`, followed by zeroing
/// the `0xc0` bytes at offset `0xc`.  Returns the old prev link.
pub fn w20a09_list_reset_clear(obj: &mut [u8], self_addr: u32) -> u32 {
    let old = u32::from_le_bytes(obj[4..8].try_into().unwrap());
    obj[0..4].copy_from_slice(&self_addr.to_le_bytes());
    obj[4..8].copy_from_slice(&self_addr.to_le_bytes());
    obj[8..12].copy_from_slice(&0u32.to_le_bytes());
    obj[0xc..0xc + 0xc0].fill(0);
    old
}

/// `0x005668f0`: slot guard; true when `row < 12`, `col < 4` (signed) and
/// the owner's field at `+8` is non-zero (the following lookup is opaque).
pub fn w20a09_slot_guard_rc(row: u32, col: i32, owner_field8: u32) -> bool {
    row < 0xc && col < 4 && owner_field8 != 0
}

/// `0x005670a0`: slot guard without the column test.
pub fn w20a09_slot_guard_r(row: u32, owner_field8: u32) -> bool {
    row < 0xc && owner_field8 != 0
}

/// `0x00566f40`: for mode 1/2 returns the column (0/1) and the byte offsets
/// written in the owner: `0xd0 + (col + row*4)*4` and `400 + (col+row*4)*4`.
pub fn w20a09_mode_slot(mode: i32, row: i32) -> Option<(i32, usize, usize)> {
    let col = match mode {
        1 => 0,
        2 => 1,
        _ => return None,
    };
    let k = (col + row.wrapping_mul(4)).wrapping_mul(4);
    Some((col, (0xd0 + k) as usize, (400 + k) as usize))
}

/// `0x00567be0`: candidate collection predicate.  `cand`/`owner` hold the
/// fields read at `+0x4c`, `+0x60`, `+0x5c`, `+100`.  `can_target(kind)`,
/// `check_a`, `check_b` model the opaque callees.  Pushes into `list`;
/// returns true when the owner's own target is chosen (list reset to it).
#[allow(clippy::too_many_arguments)]
pub fn w20a09_collect_candidate(
    cand: u32,
    cand_byte4c: u8,
    owner_byte4c: u8,
    owner_kind: u8,
    owner_f5c: u32,
    owner_f64: u32,
    can_target: impl Fn(u8, u32) -> bool,
    check_a: impl Fn(u32) -> bool,
    check_b: impl Fn(u32) -> bool,
    list: &mut Vec<u32>,
) -> bool {
    if cand == owner_f64 {
        return false;
    }
    if cand_byte4c == owner_byte4c && cand != owner_f5c {
        return false;
    }
    if !can_target(owner_kind, cand) || !check_a(cand) {
        return false;
    }
    if check_b(cand) && cand == owner_f5c {
        list.clear();
        list.push(cand);
        return true;
    }
    list.push(cand);
    false
}

/// `0x00567d40`: byte copy of the four fields at `+0x60..+0x63` from the
/// source object to the destination (other calls are opaque).
pub fn w20a09_copy_state_bytes(src: &[u8], dst: &mut [u8]) {
    dst[0x60..0x64].copy_from_slice(&src[0x60..0x64]);
}

/// Outcome of `0x00567dc0`.
#[derive(Debug, PartialEq, Eq)]
pub enum W20a09Move {
    None,
    /// order 5 toward unit (`+0x5c`), then `(1,1)` follow-up
    Unit(u32),
    /// order 5 toward the point at `+0x58/+0x5a`
    Point(i16, i16),
}

/// `0x00567dc0`: step counter.  `f28/f2a` = current pos, `f10/f12` = prior
/// pos, `f58/f5a` = destination, `f5c` = target unit, `counter` = byte at
/// `+0x61`, `w/h` = map limits.  Returns the new counter and the decision.
#[allow(clippy::too_many_arguments)]
pub fn w20a09_step_move(
    f28: i16,
    f2a: i16,
    f10: i16,
    f12: i16,
    f58: i16,
    f5a: i16,
    f5c: u32,
    counter: u8,
    w: u16,
    h: u16,
) -> (u8, W20a09Move) {
    let mut c = counter;
    if f28 < 0 || (w as i32) <= f28 as i32 || f2a < 0 || (h as i32) <= f2a as i32 {
        c = 0;
    }
    let old = c;
    c = c.wrapping_sub(1);
    if old == 0 || (f10 == f28 && f12 == f2a) {
        if f5c != 0 {
            return (c, W20a09Move::Unit(f5c));
        }
        return (c, W20a09Move::Point(f58, f5a));
    }
    (c, W20a09Move::None)
}

#[cfg(test)]
mod tests_wave20_agent09 {
    use super::*;

    #[test]
    fn push_alias_and_grow() {
        assert_eq!(w20a09_push_alias_source(1000, 4, 1000 + 0x24 * 2), Some(2));
        assert_eq!(w20a09_push_alias_source(1000, 4, 1000 + 0x24 * 4), None);
        assert_eq!(w20a09_push_alias_source(1000, 4, 5000), None);
        assert_eq!(w20a09_push_grow(3, 4), None);
        assert_eq!(w20a09_push_grow(0, 0), Some(1));
        assert_eq!(w20a09_push_grow(4, 4), Some(6));
        assert_eq!(w20a09_push_grow(4, 0x8000_0004), Some(0x8000_0006));
    }

    #[test]
    fn list_resets() {
        let mut n = [1, 2, 3];
        assert_eq!(w20a09_list_reset_a(&mut n, 77), 2);
        assert_eq!(n, [77, 77, 0]);
        let mut o = vec![0xffu8; 0xc + 0xc0 + 4];
        o[4..8].copy_from_slice(&9u32.to_le_bytes());
        assert_eq!(w20a09_list_reset_clear(&mut o, 5), 9);
        assert_eq!(&o[0..4], &5u32.to_le_bytes());
        assert_eq!(o[8], 0);
        assert!(o[0xc..0xcc].iter().all(|&b| b == 0));
        assert_eq!(o[0xcc], 0xff);
    }

    #[test]
    fn slot_guards() {
        assert!(w20a09_slot_guard_rc(11, 3, 1));
        assert!(!w20a09_slot_guard_rc(12, 0, 1));
        assert!(!w20a09_slot_guard_rc(0, 4, 1));
        assert!(!w20a09_slot_guard_rc(0, 0, 0));
        assert!(w20a09_slot_guard_r(0, 1));
        assert!(!w20a09_slot_guard_r(12, 1));
        assert_eq!(w20a09_mode_slot(1, 2), Some((0, 0xd0 + 32, 400 + 32)));
        assert_eq!(w20a09_mode_slot(2, 0), Some((1, 0xd4, 404)));
        assert_eq!(w20a09_mode_slot(0, 0), None);
    }

    #[test]
    fn collect_candidate() {
        let mut l = vec![];
        let t = |_k: u8, _c: u32| true;
        assert!(!w20a09_collect_candidate(
            5,
            1,
            2,
            0,
            9,
            7,
            t,
            |_| true,
            |_| false,
            &mut l
        ));
        assert_eq!(l, vec![5]);
        assert!(!w20a09_collect_candidate(
            7,
            1,
            2,
            0,
            9,
            7,
            t,
            |_| true,
            |_| true,
            &mut l
        ));
        assert!(!w20a09_collect_candidate(
            6,
            2,
            2,
            0,
            9,
            7,
            t,
            |_| true,
            |_| true,
            &mut l
        ));
        assert!(w20a09_collect_candidate(
            9,
            2,
            2,
            0,
            9,
            7,
            t,
            |_| true,
            |_| true,
            &mut l
        ));
        assert_eq!(l, vec![9]);
    }

    #[test]
    fn copy_and_step() {
        let mut s = vec![0u8; 0x70];
        s[0x60..0x64].copy_from_slice(&[1, 2, 3, 4]);
        let mut d = vec![0u8; 0x70];
        w20a09_copy_state_bytes(&s, &mut d);
        assert_eq!(&d[0x60..0x64], &[1, 2, 3, 4]);
        assert_eq!(
            w20a09_step_move(1, 1, 0, 0, 8, 9, 0, 3, 10, 10),
            (2, W20a09Move::None)
        );
        assert_eq!(
            w20a09_step_move(1, 1, 1, 1, 8, 9, 0, 3, 10, 10),
            (2, W20a09Move::Point(8, 9))
        );
        assert_eq!(
            w20a09_step_move(1, 1, 0, 0, 8, 9, 4, 0, 10, 10),
            (255, W20a09Move::Unit(4))
        );
        assert_eq!(
            w20a09_step_move(-1, 1, 0, 0, 8, 9, 0, 3, 10, 10).1,
            W20a09Move::Point(8, 9)
        );
    }
}
