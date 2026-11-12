#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent72.rs
// ===========================================================================
// Wave 31 agent 72: small runtime helpers.

/// `0x00af44f0`: width code of a value: 12 above 0xffffff, 11 above 0xffff,
/// otherwise 9 plus one when above 0xff.
pub fn w31a72_width_code(v: u32) -> u8 {
    if v > 0xff_ffff {
        12
    } else if v > 0xffff {
        11
    } else {
        9 + (v > 0xff) as u8
    }
}

/// `0x00af4520`: reads a big-endian u32 at `offset`; `None` when out of range.
pub fn w31a72_read_be32(buf: &[u8], offset: usize) -> Option<u32> {
    let b = buf.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// `0x00b06490`: zeroes every 16-byte record in `recs`.
pub fn w31a72_zero_records(recs: &mut [[u32; 4]]) {
    for r in recs.iter_mut() {
        *r = [0; 4];
    }
}

/// `0x00b0ab10`: returns 0 only when `count < 2` and the first word is zero.
pub fn w31a72_is_nonzero_small(first_word: i32, count: u32) -> u32 {
    if count < 2 && first_word == 0 { 0 } else { 1 }
}

/// `0x00b166f0`: picks among three candidates; `pred_a`/`pred_b` are the
/// (unresolved) predicate evaluated on the a and b slots.  Returns 0 = a,
/// 1 = b, 2 = default.
pub fn w31a72_select3(pred_a: bool, pred_b_first: bool, pred_b_second: bool) -> u8 {
    if !pred_a {
        if !pred_b_first {
            if pred_b_second { 1 } else { 0 }
        } else {
            2
        }
    } else if pred_b_first {
        0
    } else if pred_b_second {
        1
    } else {
        2
    }
}

/// `0x00b1d200`: updates the two flag bytes at +0x5918/+0x5919 and reports
/// whether the two follow-up calls must run (first set, second clear).
/// Afterwards both flags are 1.
pub fn w31a72_set_flags(flags: &mut [u8; 2]) -> bool {
    let run = flags[0] != 0 && flags[1] == 0;
    *flags = [1, 1];
    run
}

/// `0x00af8460`: error code from a state block: the stored error when
/// nonzero, 0xb when the done flag is set, otherwise 0 (after `reset`).
pub fn w31a72_state_error(error: i32, done: bool, reset: impl FnOnce()) -> i32 {
    if error != 0 {
        return error;
    }
    if done {
        return 0xb;
    }
    reset();
    0
}

/// `0x00af85b0`: true when no error/done and the kind `kind_of_top` (resolved
/// by caller, negative = invalid) equals 2.
pub fn w31a72_top_kind_is_2(
    error: i32,
    done: bool,
    kind_of_top: impl FnOnce() -> Option<i32>,
) -> bool {
    error == 0 && !done && kind_of_top() == Some(2)
}

/// `0x00af8650`: as above but compares the kind with 4.
pub fn w31a72_top_kind_is_4(
    error: i32,
    done: bool,
    kind_of_top: impl FnOnce() -> Option<i32>,
) -> bool {
    error == 0 && !done && kind_of_top() == Some(4)
}

/// `0x00af3f80`: when the header kind is 1, sums the second word of each of
/// `count` entries; otherwise 0.
pub fn w31a72_sum_entries(kind: i32, count: u32, entry_len: impl Fn(u32) -> i32) -> i32 {
    if kind != 1 {
        return 0;
    }
    (0..count).fold(0i32, |a, i| a.wrapping_add(entry_len(i)))
}

#[cfg(test)]
mod tests_wave31_agent72 {
    use super::*;

    #[test]
    fn width_and_be() {
        assert_eq!(w31a72_width_code(0xff), 9);
        assert_eq!(w31a72_width_code(0x100), 10);
        assert_eq!(w31a72_width_code(0x1_0000), 11);
        assert_eq!(w31a72_width_code(0x100_0000), 12);
        assert_eq!(w31a72_read_be32(&[0, 1, 2, 3, 4], 1), Some(0x0102_0304));
        assert_eq!(w31a72_read_be32(&[0, 1, 2, 3], 1), None);
    }

    #[test]
    fn records_and_flags() {
        let mut r = [[1, 2, 3, 4]; 2];
        w31a72_zero_records(&mut r);
        assert_eq!(r, [[0; 4]; 2]);
        assert_eq!(w31a72_is_nonzero_small(0, 1), 0);
        assert_eq!(w31a72_is_nonzero_small(0, 2), 1);
        let mut f = [1, 0];
        assert!(w31a72_set_flags(&mut f));
        assert_eq!(f, [1, 1]);
        assert!(!w31a72_set_flags(&mut f));
    }

    #[test]
    fn select_and_state() {
        assert_eq!(w31a72_select3(false, false, true), 1);
        assert_eq!(w31a72_select3(false, false, false), 0);
        assert_eq!(w31a72_select3(false, true, false), 2);
        assert_eq!(w31a72_select3(true, true, false), 0);
        assert_eq!(w31a72_select3(true, false, true), 1);
        assert_eq!(w31a72_select3(true, false, false), 2);
        assert_eq!(w31a72_state_error(5, true, || {}), 5);
        assert_eq!(w31a72_state_error(0, true, || {}), 0xb);
        let mut hit = false;
        assert_eq!(w31a72_state_error(0, false, || hit = true), 0);
        assert!(hit);
        assert!(w31a72_top_kind_is_2(0, false, || Some(2)));
        assert!(!w31a72_top_kind_is_2(0, false, || None));
        assert!(w31a72_top_kind_is_4(0, false, || Some(4)));
        assert_eq!(w31a72_sum_entries(1, 3, |i| i as i32 + 1), 6);
        assert_eq!(w31a72_sum_entries(2, 3, |_| 9), 0);
    }
}
