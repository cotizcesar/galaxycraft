#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent137.rs
// ===========================================================================
// Wave 31 agent 137: CRT error-code and flag helpers.

/// `0x00cab7a9`: maps a mode value to the errno to store: 1 gives 0x21,
/// 2 or 3 give 0x22, anything else leaves errno untouched (`None`).
pub fn w31a137_errno_for_mode(mode: i32) -> Option<u32> {
    if mode == 1 {
        Some(0x21)
    } else if (mode as u32).wrapping_sub(2) < 2 {
        Some(0x22)
    } else {
        None
    }
}

/// `0x00cab9c5`: reads the global value into `out`; a missing output gives
/// EINVAL (0x16).
pub fn w31a137_get_global(out: Option<&mut u32>, global: u32) -> u32 {
    match out {
        None => 0x16,
        Some(o) => {
            *o = global;
            0
        }
    }
}

/// `0x00cab9f2`: sets the global to `value` when it is 0x4000, 0x8000 or
/// 0x10000; otherwise EINVAL (0x16).
pub fn w31a137_set_global(global: &mut i32, value: i32) -> u32 {
    if value != 0x4000 && value != 0x8000 && value != 0x10000 {
        return 0x16;
    }
    *global = value;
    0
}

/// `0x00cabfae`: query/set bit 1 of the per-thread flags at +0x350.
/// `param` -1 sets `*sentinel` to all ones, 1 sets the bit, 2 clears it,
/// 0 only queries, anything else is an error (-1).  Returns 1 when the
/// bit was set, else 2 (before the change).
pub fn w31a137_flag_mode(flags: &mut u32, sentinel: &mut u32, param: i32) -> i32 {
    let old = *flags;
    if param == -1 {
        *sentinel = 0xffff_ffff;
    } else if param != 0 {
        match param {
            1 => *flags = old | 2,
            2 => *flags = old & !2,
            _ => return -1,
        }
    }
    ((old & 2) == 0) as i32 + 1
}

/// `0x00cacd3c`: atomically swaps the global with 0 or 1 and returns the
/// previous value; other inputs return -1 (EINVAL).
pub fn w31a137_swap_bool(global: &mut i32, value: i32) -> i32 {
    if value != 0 && value != 1 {
        return -1;
    }
    std::mem::replace(global, value)
}

/// `0x00cacd71`: masks `mask` with `!0x80000` and rejects (EINVAL 0x16)
/// when `value & mask & 0xfcf0fce0` is non-zero.  On success returns the
/// accepted `(value, mask)` pair for the caller's control-word call.
pub fn w31a137_check_fp_mask(value: u32, mask: u32) -> Result<(u32, u32), u32> {
    let mask = mask & 0xfff7_ffff;
    if value & mask & 0xfcf0_fce0 == 0 {
        Ok((value, mask))
    } else {
        Err(0x16)
    }
}

/// `0x00cacdd0`: reads the dword at +0x10 of a record, or -1 when absent.
pub fn w31a137_field_0x10(rec: Option<&[u8]>) -> i32 {
    match rec.and_then(|r| r.get(0x10..0x14)) {
        Some(b) => i32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        None => -1,
    }
}

#[cfg(test)]
mod tests_wave31_agent137 {
    use super::*;

    #[test]
    fn errno_and_globals() {
        assert_eq!(w31a137_errno_for_mode(1), Some(0x21));
        assert_eq!(w31a137_errno_for_mode(2), Some(0x22));
        assert_eq!(w31a137_errno_for_mode(3), Some(0x22));
        assert_eq!(w31a137_errno_for_mode(4), None);
        assert_eq!(w31a137_errno_for_mode(0), None);
        let mut o = 0;
        assert_eq!(w31a137_get_global(Some(&mut o), 7), 0);
        assert_eq!(o, 7);
        assert_eq!(w31a137_get_global(None, 7), 0x16);
        let mut g = 0;
        assert_eq!(w31a137_set_global(&mut g, 0x8000), 0);
        assert_eq!(g, 0x8000);
        assert_eq!(w31a137_set_global(&mut g, 5), 0x16);
        assert_eq!(g, 0x8000);
    }

    #[test]
    fn flags_and_swap() {
        let (mut f, mut s) = (0u32, 0u32);
        assert_eq!(w31a137_flag_mode(&mut f, &mut s, 1), 2);
        assert_eq!(f, 2);
        assert_eq!(w31a137_flag_mode(&mut f, &mut s, 2), 1);
        assert_eq!(f, 0);
        assert_eq!(w31a137_flag_mode(&mut f, &mut s, -1), 2);
        assert_eq!(s, u32::MAX);
        assert_eq!(w31a137_flag_mode(&mut f, &mut s, 9), -1);
        let mut g = 1;
        assert_eq!(w31a137_swap_bool(&mut g, 0), 1);
        assert_eq!(g, 0);
        assert_eq!(w31a137_swap_bool(&mut g, 2), -1);
    }

    #[test]
    fn mask_and_field() {
        assert_eq!(w31a137_check_fp_mask(0x1, 0x1), Ok((1, 1)));
        assert_eq!(w31a137_check_fp_mask(0x20, 0x20), Err(0x16));
        assert_eq!(w31a137_check_fp_mask(0x80000, 0x80000), Ok((0x80000, 0)));
        let mut r = [0u8; 0x14];
        r[0x10] = 9;
        assert_eq!(w31a137_field_0x10(Some(&r)), 9);
        assert_eq!(w31a137_field_0x10(None), -1);
    }
}
