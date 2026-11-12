#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave16/agent07.rs
// ===========================================================================
// Wave 16 translation batch 07.
//
// Pure helpers; unresolved callees are closure/bool parameters.
// Skipped: FUN_005bf340 (noreturn tail), FUN_005bfa2a (noreturn call),
// FUN_005c0ed0 (bit-count over unresolved callees), FUN_005c149c (bare thunk),
// FUN_005c4b00 (noreturn exit), FUN_005c6a80 (bare call), FUN_005c6f80
// (noreturn), FUN_005c7350 (noreturn), FUN_005c7740 (noreturn),
// FUN_005c7ea0 (bare call chain), FUN_005c8e40 (STL tree iterator glue),
// FUN_005cc890 (noreturn), FUN_005cea00 (noreturn).

/// Rectangle half-split used by 0x005bf2c0: (left half, right remainder).
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W16a07Split {
    pub x_lo: i16,
    pub y_lo: i16,
    pub x_hi: i16,
    pub y_hi: i16,
}

/// 0x005bd4f0: if flag bit 1 (0x20 byte) is set and the state byte (+0x104)
/// is > 2 and `cond` holds, clear the flag; returns true when `0x58_b460(0xc,1)`
/// must be invoked.
pub fn w16a07_clear_flag_if_ready(flags20: &mut u8, state104: u8, cond: bool) -> bool {
    if *flags20 & 2 != 0 && state104 > 2 && cond {
        *flags20 &= 0xfd;
        return true;
    }
    false
}

/// 0x005bf2c0: when the blocked test is false, split `width`/`height` into
/// halves (low = n/2 truncating, high = n - low).  `None` means return 0.
pub fn w16a07_half_extents(blocked: bool, width: i32, height: i32) -> Option<W16a07Split> {
    if blocked {
        return None;
    }
    let y_lo = (height / 2) as i16;
    let y_hi = (height as i16).wrapping_sub(y_lo);
    let x_lo = (width / 2) as i16;
    let x_hi = (width as i16).wrapping_sub(x_lo);
    Some(W16a07Split {
        x_lo,
        y_lo,
        x_hi,
        y_hi,
    })
}

/// 0x005c4d90: sets the global byte flag; returns the new value.
pub fn w16a07_set_flag() -> u32 {
    1
}

/// Per-entry result of 0x005c5110.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W16a07Entry {
    pub owner: u8,
    pub has_ctrl: bool,
    pub ctrl_flags: u8,
}

/// 0x005c5110: for each list entry matching `local_owner` while `paused == 0`
/// and (ctrl present or `fallback(entry)` true), clear bit 0x20 of the control
/// flags.  Returns indices of entries that need the `0x57_d6e0` refresh.
pub fn w16a07_unmark_entries(
    entries: &mut [W16a07Entry],
    paused: u32,
    local_owner: u8,
    mut fallback: impl FnMut(&W16a07Entry) -> bool,
) -> Vec<usize> {
    let mut out = Vec::new();
    for (i, e) in entries.iter_mut().enumerate() {
        if paused == 0 && e.owner == local_owner && (e.has_ctrl || fallback(e)) {
            e.ctrl_flags &= 0xdf;
            out.push(i);
        }
    }
    out
}

/// 0x005c6aa0: accumulate a cost into the pending-order slot.  Returns the new
/// `(order_id, accum)` pair; the function always returns 0 to the caller.
#[allow(clippy::too_many_arguments)]
pub fn w16a07_accumulate_cost(
    same: bool,
    flag_a: bool,
    can_use: bool,
    ok_b: bool,
    order_id: u32,
    accum: i32,
    cost_special: i32,
    cost_normal: i32,
) -> i32 {
    if same || flag_a || !can_use || !ok_b {
        return accum;
    }
    if order_id == 0x3c {
        accum.wrapping_add(cost_special)
    } else {
        accum.wrapping_add(cost_normal)
    }
}

/// 0x005c70a0: gate result. `base_ok` is the `0x5c6f80` result, `ctx_blocked`
/// the (global != 0 && callee != 0 && byte d9 != 0) test, `busy` the `0x5a_d890`
/// result.
pub fn w16a07_gate(base_ok: bool, ctx_blocked: bool, busy: bool) -> u32 {
    if base_ok && !ctx_blocked && !busy {
        1
    } else {
        0
    }
}

/// 0x005d483e: true when low byte of `ebx` equals the call result byte.
pub fn w16a07_byte_matches(ebx: u32, result: u32) -> u32 {
    ((ebx & 0xff) == result) as u32
}

#[cfg(test)]
mod tests_wave16_agent07 {
    use super::*;

    #[test]
    fn flag_clear() {
        let mut f = 3u8;
        assert!(w16a07_clear_flag_if_ready(&mut f, 3, true));
        assert_eq!(f, 1);
        let mut g = 3u8;
        assert!(!w16a07_clear_flag_if_ready(&mut g, 2, true));
        assert_eq!(g, 3);
    }

    #[test]
    fn halves() {
        assert_eq!(w16a07_half_extents(true, 5, 5), None);
        let s = w16a07_half_extents(false, 5, 7).unwrap();
        assert_eq!((s.x_lo, s.x_hi, s.y_lo, s.y_hi), (2, 3, 3, 4));
    }

    #[test]
    fn unmark() {
        let mut v = [
            W16a07Entry {
                owner: 1,
                has_ctrl: true,
                ctrl_flags: 0x20,
            },
            W16a07Entry {
                owner: 2,
                has_ctrl: true,
                ctrl_flags: 0x20,
            },
            W16a07Entry {
                owner: 1,
                has_ctrl: false,
                ctrl_flags: 0x20,
            },
        ];
        let r = w16a07_unmark_entries(&mut v, 0, 1, |_| false);
        assert_eq!(r, vec![0]);
        assert_eq!(v[0].ctrl_flags, 0);
        assert_eq!(v[2].ctrl_flags, 0x20);
        assert!(w16a07_unmark_entries(&mut v, 1, 1, |_| true).is_empty());
    }

    #[test]
    fn cost_and_gate() {
        assert_eq!(
            w16a07_accumulate_cost(false, false, true, true, 0x3c, 10, 5, 7),
            15
        );
        assert_eq!(
            w16a07_accumulate_cost(false, false, true, true, 1, 10, 5, 7),
            17
        );
        assert_eq!(
            w16a07_accumulate_cost(true, false, true, true, 1, 10, 5, 7),
            10
        );
        assert_eq!(w16a07_gate(true, false, false), 1);
        assert_eq!(w16a07_gate(true, true, false), 0);
        assert_eq!(w16a07_byte_matches(0x1ff, 0xff), 1);
        assert_eq!(w16a07_set_flag(), 1);
    }
}
