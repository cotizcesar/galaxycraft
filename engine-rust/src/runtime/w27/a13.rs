#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent13.rs
// ===========================================================================
// Wave 27 agent 13: allocator bitmap range helper and a thin wrapper.

/// `0x0098a4b0`: sets (`set != 0`) or clears `count` bits starting at bit
/// `start` of the byte bitmap `bits`; `hint` is the cached first-free-bit
/// field (`this[9]`), `limit` is `this[4]` and `find_free(from, limit, 1)`
/// models the neighbour search helper.  Bits are LSB-first inside a byte.
/// The decompiler lost the code after the whole-byte memset path; this
/// version falls through to the trailing-bit loop (uncertain).
pub fn w27a13_bitmap_set_range(
    bits: &mut [u8],
    hint: &mut i32,
    limit: i32,
    start: i32,
    set: bool,
    count: i32,
    mut find_free: impl FnMut(i32, i32, i32) -> i32,
) {
    let mut bit = (start & 7) as u32;
    let mut byte = (start / 8) as usize;
    let mut rem = count;
    let put = |bits: &mut [u8], byte: usize, bit: u32| {
        if set {
            bits[byte] |= 1 << bit;
        } else {
            bits[byte] &= !(1u8 << bit);
        }
    };
    let step = |byte: &mut usize, bit: &mut u32| {
        *bit += 1;
        if *bit >= 8 {
            *bit = 0;
            *byte += 1;
        }
    };
    if start & 0x1f != 0 {
        let mut n = 0x20 - (start & 0x1f);
        if count < n {
            n = count;
        }
        if n != 0 {
            rem = count - n;
            for _ in 0..n {
                put(bits, byte, bit);
                step(&mut byte, &mut bit);
            }
        }
    }
    let whole = rem / 8;
    if whole != 0 {
        let fill = if set { 0xff } else { 0 };
        for b in &mut bits[byte..byte + whole as usize] {
            *b = fill;
        }
        byte += whole as usize;
        rem -= whole * 8;
    }
    for _ in 0..(rem & 0x1f).max(0) {
        put(bits, byte, bit);
        step(&mut byte, &mut bit);
    }
    if whole != 0 {
        return;
    }
    if !set {
        if start < *hint {
            *hint = start;
        }
    } else {
        if start == *hint {
            *hint = count + start;
        }
        let h = *hint;
        if bits[(h / 8) as usize] & (1 << (h & 7)) != 0 {
            *hint = find_free(h, limit, 1);
        }
    }
}

/// `0x00989fc0`: forwards its four arguments to the worker with a trailing
/// flag of `1`.
pub fn w27a13_forward_with_flag<R>(
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    mut worker: impl FnMut(u32, u32, u32, u32, u32) -> R,
) -> R {
    worker(a, b, c, d, 1)
}

#[cfg(test)]
mod tests_wave27_agent13 {
    use super::*;

    #[test]
    fn set_and_clear_small_range() {
        let mut bits = [0u8; 4];
        let mut hint = 0;
        w27a13_bitmap_set_range(&mut bits, &mut hint, 32, 2, true, 3, |f, _, _| f + 100);
        assert_eq!(bits[0], 0b0001_1100);
        assert_eq!(hint, 0);
    }

    #[test]
    fn clear_lowers_hint() {
        let mut bits = [0xffu8; 4];
        let mut hint = 20;
        w27a13_bitmap_set_range(&mut bits, &mut hint, 32, 4, false, 2, |_, _, _| 0);
        assert_eq!(bits[0], 0b1100_1111);
        assert_eq!(hint, 4);
    }

    #[test]
    fn forward_flag() {
        assert_eq!(
            w27a13_forward_with_flag(1, 2, 3, 4, |a, b, c, d, e| a + b + c + d + e),
            11
        );
    }
}
