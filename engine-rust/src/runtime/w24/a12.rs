#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent12.rs
// ===========================================================================
// Wave 24 agent 12: small helpers destined for `runtime.rs`.

/// Control bytes accepted/rejected by the string scan at `0x0073b060`.
fn w24a12_is_special_ctrl(b: u8) -> bool {
    matches!(b, 1..=8 | 0xe..=0x11 | 0x15..=0x19 | 0x1b..=0x1f)
}

/// `0x0073b060`: scans at most `min(len, limit)` bytes for a NUL.  Returns
/// the count consumed including the NUL, or 0 when a rejected control byte
/// is met (or the budget runs out).  `flag_a` is param_4, `flag_b` is param_5.
pub fn w24a12_scan_text(s: &[u8], len: u32, limit: u32, flag_a: bool, flag_b: bool) -> i32 {
    let mut n = len.min(limit);
    let mut count = 0i32;
    let mut i = 0usize;
    while n != 0 {
        let b = s.get(i).copied().unwrap_or(0);
        n -= 1;
        count += 1;
        if b == 0 {
            return count;
        }
        if !flag_b && w24a12_is_special_ctrl(b) {
            return 0;
        }
        if !flag_a && b < 0x20 && !w24a12_is_special_ctrl(b) {
            return 0;
        }
        i += 1;
    }
    0
}

/// `0x0073c6c0`: on the state block, clears byte `0x2c71` and the 8 bytes
/// at `0x2c72`.
pub fn w24a12_clear_state_2c71(block: &mut [u8]) {
    block[0x2c71] = 0;
    block[0x2c72..0x2c7a].fill(0);
}

/// `0x0073c6f0`: zeroes the `0x90` bytes at offset `0xe544` of the block.
pub fn w24a12_clear_block_e544(block: &mut [u8]) {
    block[0xe544..0xe544 + 0x90].fill(0);
}

/// `0x0073cf20`: zeroes the twelve dwords of the global table at offset
/// `0x011a1f14..0x011a1f44` (given as `[u32; 12]` in address order).
pub fn w24a12_clear_table_1f14(t: &mut [u32; 12]) {
    *t = [0; 12];
}

/// `0x0073e620`: stores the global 16-bit value into `*out`.
pub fn w24a12_store_u16(out: &mut u16, global: u16) {
    *out = global;
}

/// `0x0073d820`: runs the pre-step, then forwards `param`, replaced by 3
/// when `override_flag` is non-zero.  Returns the value to forward.
pub fn w24a12_pick_param(param: u32, override_flag: u32, pre: impl FnOnce()) -> u32 {
    pre();
    if override_flag != 0 { 3 } else { param }
}

/// `0x0073e470`: re-initialises six `0x15e`-byte records inside `buf`
/// (offsets 0, 0x15e.., 0x1068, 0x11c6, 0x1324, 0x1482).  The first record
/// is zeroed and then replicated forward over `0xf0a` bytes.  Needs 0x15e0 bytes.
pub fn w24a12_reset_records(buf: &mut [u8]) {
    const REC: usize = 0x15e;
    buf[..REC].fill(0);
    for i in 0..(0x3c2 * 4 + 2) {
        buf[REC + i] = buf[i];
    }
    for off in [0x1068usize, 0x11c6, 0x1324, 0x1482] {
        buf[off..off + REC].fill(0);
    }
}

#[cfg(test)]
mod tests_wave24_agent12 {
    use super::*;

    #[test]
    fn scan_text() {
        assert_eq!(w24a12_scan_text(b"ab\0", 10, 10, false, false), 3);
        assert_eq!(w24a12_scan_text(b"a\x01\0", 10, 10, false, false), 0);
        assert_eq!(w24a12_scan_text(b"a\x01\0", 10, 10, true, true), 3);
        assert_eq!(w24a12_scan_text(b"a\x0a\0", 10, 10, false, true), 0);
        assert_eq!(w24a12_scan_text(b"a\x01\0", 10, 10, false, true), 3);
        assert_eq!(w24a12_scan_text(b"abc\0", 2, 10, true, true), 0);
    }

    #[test]
    fn clears() {
        let mut b = vec![0xffu8; 0xf000];
        w24a12_clear_state_2c71(&mut b);
        assert_eq!(b[0x2c70], 0xff);
        assert_eq!(b[0x2c71], 0);
        assert_eq!(b[0x2c79], 0);
        assert_eq!(b[0x2c7a], 0xff);
        w24a12_clear_block_e544(&mut b);
        assert_eq!(b[0xe543], 0xff);
        assert_eq!(b[0xe544 + 0x8f], 0);
        assert_eq!(b[0xe544 + 0x90], 0xff);
        let mut t = [7u32; 12];
        w24a12_clear_table_1f14(&mut t);
        assert_eq!(t, [0; 12]);
    }

    #[test]
    fn misc() {
        let mut v = 0;
        w24a12_store_u16(&mut v, 0x1234);
        assert_eq!(v, 0x1234);
        let mut called = false;
        assert_eq!(w24a12_pick_param(9, 0, || called = true), 9);
        assert!(called);
        assert_eq!(w24a12_pick_param(9, 1, || {}), 3);
    }

    #[test]
    fn reset_records() {
        let mut b = vec![0xaau8; 0x1700];
        w24a12_reset_records(&mut b);
        assert!(b[..0x1068].iter().all(|&x| x == 0));
        assert!(b[0x1068..0x15e0].iter().all(|&x| x == 0));
        assert_eq!(b[0x15e0], 0xaa);
    }
}
