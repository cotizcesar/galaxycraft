#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave17/agent07.rs
// ===========================================================================
// Wave 17 translation batch 07.
//
// Window: queue: 20 addresses, module `sim.rs`.
// Skipped (noreturn / bnet / opaque): FUN_006e2c60, FUN_006e2d40,
// FUN_006e2df0, FUN_006fcb80, FUN_006fdf80, FUN_00728ae0, FUN_0072aeb0,
// FUN_0072b360, FUN_0072b9f0, FUN_0072c560, FUN_0072c940.

/// `0x006c64f0`: runs `action` only when all four guards hold: `a` and `b`
/// are non-zero, `c` (the 005a3150 probe) is zero and `d(player_slot)` is
/// zero.  Guards evaluate lazily in that order.
pub fn w17a07_conditional_action(
    mut a: impl FnMut() -> i32,
    mut b: impl FnMut() -> i32,
    mut c: impl FnMut() -> i32,
    mut d: impl FnMut() -> i32,
    mut action: impl FnMut(),
) -> bool {
    if a() != 0 && b() != 0 && c() == 0 && d() == 0 {
        action();
        return true;
    }
    false
}

/// `0x006cdb80`: walks an intrusive list (node link at offset 4) starting at
/// `head` (0 = end) and calls `visit(node)` on each node.
pub fn w17a07_walk_list(head: u32, mut next: impl FnMut(u32) -> u32, mut visit: impl FnMut(u32)) {
    let mut node = head;
    while node != 0 {
        visit(node);
        node = next(node);
    }
}

/// `0x006e2c10`: true when `(a * 100) / b` (signed 32-bit, wrapping) read as
/// unsigned is below 0x22 (34 percent).  Negative results are huge unsigned
/// values, hence false.  `None` on division by zero (hardware trap).
pub fn w17a07_below_34_percent(a: i32, b: i32) -> Option<bool> {
    if b == 0 {
        return None;
    }
    let q = a.wrapping_mul(100).wrapping_div(b);
    Some((q as u32) < 0x22)
}

/// `0x006e5250`: calls the 005b6800 helper with 100 and returns 0.
pub fn w17a07_call_with_100(mut f: impl FnMut(i32)) -> u32 {
    f(100);
    0
}

/// `0x006f8ec0`: decodes the obfuscated global `DAT_00f259ac` (xor key
/// 0x3bff7b85).
pub fn w17a07_decode_global(raw: u32) -> u32 {
    raw ^ 0x3bff_7b85
}

/// `0x006f91d0`: compares the decoded global to `param`.  The decompiler
/// returns CONCAT31(decoded >> 8 low 24 bits, equal); the low byte is the
/// result and upper 24 bits are the decoded value's bits 8..32.
pub fn w17a07_decoded_equals(raw: u32, param: u32) -> u32 {
    let d = raw ^ 0x3bff_7b85;
    ((d >> 8) << 8) | u32::from(d == param)
}

/// `0x00719e60`: sets both flags (`DAT_0117f116`, `DAT_0119f140`) to 1.
pub fn w17a07_set_flags(flag_a: &mut u8, flag_b: &mut u32) {
    *flag_a = 1;
    *flag_b = 1;
}

/// `0x007141e0`: copies `min(strlen(src), 0xd9)` bytes (no terminator) from
/// the C string `src` into `dst` (`DAT_0117dac8`).  `None` source is a
/// no-op.  Returns the number of bytes copied.
pub fn w17a07_copy_name(dst: &mut [u8], src: Option<&[u8]>) -> usize {
    let Some(src) = src else { return 0 };
    let len = src.iter().position(|&b| b == 0).unwrap_or(src.len());
    let n = len.min(0xd9).min(dst.len());
    dst[..n].copy_from_slice(&src[..n]);
    n
}

/// `0x00727cc0`: removes `wanted` from the 12-entry list at `DAT_0119f710`
/// (zero terminated or full): the count is the first zero index (else 12);
/// the matching entry is replaced by the last used entry which is then
/// cleared.  Returns true when something was removed.
pub fn w17a07_remove_entry(list: &mut [u32; 12], wanted: u32) -> bool {
    let count = list.iter().position(|&v| v == 0).unwrap_or(12);
    for i in 0..count {
        if list[i] == wanted {
            list[i] = list[count - 1];
            list[count - 1] = 0;
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests_wave17_agent07 {
    use super::*;

    #[test]
    fn guards_and_walk() {
        let mut n = 0;
        assert!(w17a07_conditional_action(|| 1, || 1, || 0, || 0, || n += 1));
        assert!(!w17a07_conditional_action(
            || 1,
            || 0,
            || 0,
            || 0,
            || n += 1
        ));
        assert!(!w17a07_conditional_action(
            || 1,
            || 1,
            || 1,
            || 0,
            || n += 1
        ));
        assert_eq!(n, 1);
        let mut seen = vec![];
        w17a07_walk_list(1, |x| if x < 3 { x + 1 } else { 0 }, |x| seen.push(x));
        assert_eq!(seen, [1, 2, 3]);
    }

    #[test]
    fn percent_and_decode() {
        assert_eq!(w17a07_below_34_percent(33, 100), Some(true));
        assert_eq!(w17a07_below_34_percent(34, 100), Some(false));
        assert_eq!(w17a07_below_34_percent(-1, 100), Some(false));
        assert_eq!(w17a07_below_34_percent(1, 0), None);
        let mut v = 0;
        assert_eq!(w17a07_call_with_100(|x| v = x), 0);
        assert_eq!(v, 100);
        assert_eq!(w17a07_decode_global(0x3bff_7b85), 0);
        assert_eq!(w17a07_decoded_equals(0x3bff_7b85, 0), 1);
        assert_eq!(w17a07_decoded_equals(0x3bff_7b85 ^ 0x100, 0), 0x0000_0100);
    }

    #[test]
    fn flags_copy_remove() {
        let (mut a, mut b) = (0u8, 0u32);
        w17a07_set_flags(&mut a, &mut b);
        assert_eq!((a, b), (1, 1));
        let mut dst = [0u8; 0xe0];
        assert_eq!(w17a07_copy_name(&mut dst, Some(b"abc\0def")), 3);
        assert_eq!(&dst[..4], b"abc\0");
        assert_eq!(w17a07_copy_name(&mut dst, Some(&[b'x'; 300])), 0xd9);
        assert_eq!(w17a07_copy_name(&mut dst, None), 0);
        let mut l = [5, 6, 7, 0, 9, 0, 0, 0, 0, 0, 0, 0];
        assert!(w17a07_remove_entry(&mut l, 5));
        assert_eq!(&l[..4], &[7, 6, 0, 0]);
        assert!(!w17a07_remove_entry(&mut l, 9));
        let mut full = [1u32; 12];
        full[3] = 2;
        assert!(w17a07_remove_entry(&mut full, 2));
        assert_eq!(full[3], 1);
        assert_eq!(full[11], 0);
    }
}
