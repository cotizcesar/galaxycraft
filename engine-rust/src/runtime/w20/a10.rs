#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave20/agent10.rs
// ===========================================================================
// Wave 20 agent 10 translations (runtime helpers).

/// Selector chosen by `0x00568820`: either a pointer-like handle or a tile pair.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum W20a10Target {
    /// Handle at `+0x5c` was non-zero (callee takes `param, handle`).
    Handle(i32),
    /// Handle was zero (callee takes `param, x, y`, i16 pair at `+0x58/+0x5a`).
    Pos(i16, i16),
}

/// `0x00568820`: picks the handle when `+0x5c` is non-zero, else the i16
/// position at `+0x58`/`+0x5a`.
pub fn w20a10_select_target(handle: i32, x: i16, y: i16) -> W20a10Target {
    if handle != 0 {
        W20a10Target::Handle(handle)
    } else {
        W20a10Target::Pos(x, y)
    }
}

/// Outcome of `0x00568e40`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W20a10Notify {
    /// Direct notification code (5) when `Some`, else the wrapper code.
    pub direct: bool,
    /// Code passed to the target callee (5 when direct, else 1..=6).
    pub code: u32,
    /// Value passed as first arg of the follow-up call (second arg is 1).
    pub follow: u32,
}

/// `0x00568e40`: if bit 0 of the flags byte (`+0x4f`) is set, clears it and
/// maps the class `kind` (table at `+0x60`) to a notification.
pub fn w20a10_flag_notify(flags: &mut u8, kind: u8) -> Option<W20a10Notify> {
    if *flags & 1 == 0 {
        return None;
    }
    *flags &= 0xfe;
    Some(match kind {
        2 | 4 | 5 | 6 => W20a10Notify {
            direct: true,
            code: 5,
            follow: 1,
        },
        1 => W20a10Notify {
            direct: false,
            code: 2,
            follow: 2,
        },
        7 => W20a10Notify {
            direct: false,
            code: 3,
            follow: 2,
        },
        3 => W20a10Notify {
            direct: false,
            code: 4,
            follow: 0xe,
        },
        8 => W20a10Notify {
            direct: false,
            code: 6,
            follow: 2,
        },
        _ => W20a10Notify {
            direct: false,
            code: 1,
            follow: 2,
        },
    })
}

/// `0x00568f20`: clears the global word at the list-state area.
pub fn w20a10_clear_word(slot: &mut u32) {
    *slot = 0;
}

/// `0x00568f9b`: true when `limit(0) <= arg` (unsigned).
pub fn w20a10_le_limit(limit: u32, arg: u32) -> bool {
    limit <= arg
}

/// `0x0058b3f0`: stores a byte at offset `0xf` of the record referenced by `+0xc`.
pub fn w20a10_set_byte_0f(record: &mut [u8], value: u8) {
    record[0xf] = value;
}

/// `0x0058ddb0`: when the target pair at `+0xf8/+0xfa` is non-zero and
/// differs from the current pair at `+0x28/+0x2a`, returns the pair to pass on.
pub fn w20a10_pending_move(target: (i16, i16), current: (i16, i16)) -> Option<(i16, i16)> {
    if (target.0 != 0 || target.1 != 0) && target != current {
        Some(target)
    } else {
        None
    }
}

/// `0x006e90d0`: reads (u16 at `+0x18`, u8 at `+0x10`) for the callee; always returns 1.
pub fn w20a10_read_pair(rec: &[u8]) -> (u16, u8, u32) {
    (u16::from_le_bytes([rec[0x18], rec[0x19]]), rec[0x10], 1)
}

/// `0x006e9b50`: on the sub-record, reports whether a pending handle (`+8`)
/// must be released, then sets `+0xc = 2` and `+0xe = value`.
pub fn w20a10_set_mode_two(sub: &mut [u8], value: u16) -> bool {
    let pending = sub[8..12] != [0; 4];
    sub[0xc..0xe].copy_from_slice(&2u16.to_le_bytes());
    sub[0xe..0x10].copy_from_slice(&value.to_le_bytes());
    pending
}

/// `0x00983dd0`: true when `a*a*100 + b*b*256 > 0x9c_4000` (wrapping i32).
pub fn w20a10_dist_exceeds(a: i32, b: i32) -> bool {
    let v = b
        .wrapping_mul(b)
        .wrapping_mul(0x100)
        .wrapping_add(a.wrapping_mul(a).wrapping_mul(100));
    0x9c4000u32 < v as u32
}

/// Result of `0x0056869b`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W20a10Rebuild {
    pub first_free: Option<usize>,
    pub first_used: Option<usize>,
    pub used_count: u32,
}

/// `0x0056869b`: rebuilds free/used cursors over 0x70-byte records (28 words;
/// word 3 = in-use flag).  `on_release(i)` stands for the per-record callee
/// (called for later free records in pass one and for later used ones in
/// pass two, after words 0 and 1 are zeroed).
#[allow(clippy::needless_range_loop)]
pub fn w20a10_rebuild_lists(
    recs: &mut [[u32; 0x1c]],
    mut on_release: impl FnMut(usize),
) -> W20a10Rebuild {
    let n = recs.len();
    let first_free = (0..n).find(|&i| recs[i][3] == 0);
    if let Some(f) = first_free {
        for j in f + 1..n {
            if recs[j][3] == 0 {
                on_release(j);
            }
        }
    }
    let first_used = (0..n).find(|&i| recs[i][3] != 0);
    let mut used_count = 0;
    if let Some(u) = first_used {
        recs[u][0] = 0;
        recs[u][1] = 0;
        used_count = 1;
        for j in u + 1..n {
            if recs[j][3] != 0 {
                recs[j][0] = 0;
                recs[j][1] = 0;
                on_release(j);
                used_count += 1;
            }
        }
    }
    W20a10Rebuild {
        first_free,
        first_used,
        used_count,
    }
}

#[cfg(test)]
mod tests_wave20_agent10 {
    use super::*;

    #[test]
    fn select_and_flag() {
        assert_eq!(w20a10_select_target(5, 1, 2), W20a10Target::Handle(5));
        assert_eq!(w20a10_select_target(0, 1, 2), W20a10Target::Pos(1, 2));
        let mut f = 0x03;
        let r = w20a10_flag_notify(&mut f, 3).unwrap();
        assert_eq!((f, r.code, r.follow, r.direct), (2, 4, 0xe, false));
        assert!(w20a10_flag_notify(&mut f, 3).is_none());
        let mut f = 1;
        assert!(w20a10_flag_notify(&mut f, 6).unwrap().direct);
        let mut f = 1;
        assert_eq!(w20a10_flag_notify(&mut f, 8).unwrap().code, 6);
        let mut f = 1;
        assert_eq!(w20a10_flag_notify(&mut f, 9).unwrap().code, 1);
    }

    #[test]
    fn small_helpers() {
        let mut w = 7;
        w20a10_clear_word(&mut w);
        assert_eq!(w, 0);
        assert!(w20a10_le_limit(3, 3) && !w20a10_le_limit(4, 3));
        let mut r = [0u8; 16];
        w20a10_set_byte_0f(&mut r, 9);
        assert_eq!(r[15], 9);
        assert_eq!(w20a10_pending_move((0, 0), (1, 1)), None);
        assert_eq!(w20a10_pending_move((2, 3), (2, 3)), None);
        assert_eq!(w20a10_pending_move((0, 3), (2, 3)), Some((0, 3)));
        let mut b = [0u8; 0x20];
        b[0x18] = 0x34;
        b[0x19] = 0x12;
        b[0x10] = 7;
        assert_eq!(w20a10_read_pair(&b), (0x1234, 7, 1));
        let mut s = [0u8; 16];
        s[8] = 1;
        assert!(w20a10_set_mode_two(&mut s, 0xabcd));
        assert_eq!((s[0xc], s[0xe], s[0xf]), (2, 0xcd, 0xab));
        assert!(!w20a10_dist_exceeds(0, 0));
        assert!(!w20a10_dist_exceeds(0, 200));
        assert!(w20a10_dist_exceeds(0, 201));
    }

    #[test]
    fn rebuild() {
        let mut r = vec![[0u32; 0x1c]; 4];
        r[1][3] = 1;
        r[3][3] = 1;
        r[3][0] = 9;
        let mut calls = vec![];
        let out = w20a10_rebuild_lists(&mut r, |i| calls.push(i));
        assert_eq!(
            out,
            W20a10Rebuild {
                first_free: Some(0),
                first_used: Some(1),
                used_count: 2
            }
        );
        assert_eq!(calls, vec![2, 3]);
        assert_eq!(r[3][0], 0);
    }
}
