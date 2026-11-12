#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent58.rs
// ===========================================================================
// Wave 31 agent 58: small runtime helpers (UI rect, bitset, lookups).

/// `0x009442a0`: true when the dword at offset `0x68` of the current
/// context record is non-zero.
pub fn w31a58_ctx_field68_set(ctx: &[u8]) -> bool {
    ctx.get(0x68..0x6c)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) != 0)
        .unwrap_or(false)
}

/// `0x00944e50`: bit 15 of a flags value returned by an indirect getter.
pub fn w31a58_flag_bit15(flags: u32) -> u32 {
    (flags >> 15) & 1
}

/// `0x00946b50`: stores a rectangle (x, y, x+w, y+h) at offsets
/// `0x30..0x40` of the context; returns true when the byte at `0x90` is
/// non-zero (the caller then runs the refresh routine with argument 1).
pub fn w31a58_set_rect(ctx: &mut [u8], x: i32, y: i32, w: i32, h: i32) -> bool {
    if ctx.len() < 0x91 {
        return false;
    }
    let vals = [x, y, w.wrapping_add(x), h.wrapping_add(y)];
    for (i, v) in vals.iter().enumerate() {
        ctx[0x30 + i * 4..0x34 + i * 4].copy_from_slice(&v.to_le_bytes());
    }
    ctx[0x90] != 0
}

/// `0x00954d90`: stores a byte into a global flag; returns the new value.
pub fn w31a58_set_byte_flag(slot: &mut u8, value: u8) {
    *slot = value;
}

/// `0x0095a8d0`: MSB-first bit test in a byte bitset.
pub fn w31a58_bitset_test(bits: &[u8], index: u32) -> bool {
    match bits.get((index >> 3) as usize) {
        Some(b) => b & (0x80u8 >> (index & 7)) != 0,
        None => false,
    }
}

/// `0x00962f30`: destroys `count` 8-byte elements via the callback and
/// resets the element count to zero.
pub fn w31a58_vec8_clear(count: &mut usize, mut destroy_range: impl FnMut(usize, usize)) {
    destroy_range(0, *count * 8);
    *count = 0;
}

/// `0x00963750`: returns 0 when bit 0 of flags at `0x34` is set or the
/// global lock is on; returns 0 when bit 25 of `0x30` is clear and another
/// active object exists that differs from this one; otherwise 1.
pub fn w31a58_can_accept(
    flags30: u32,
    flags34: u8,
    global_lock: bool,
    active_id: u32,
    this_id: u32,
) -> u32 {
    if flags34 & 1 != 0 || global_lock {
        return 0;
    }
    if (flags30 >> 0x19) & 1 == 0 && active_id != 0 && this_id != active_id {
        return 0;
    }
    1
}

/// `0x0095bab0`: lower-bound lookup in a key-sorted table; when the key
/// is absent, an optional fallback creator may supply a replacement key.
/// The resolved key (or the original) is passed to `emit` with `arg`.
pub fn w31a58_lookup_and_emit(
    table: &[(u32, u32)],
    key: u32,
    arg: u32,
    can_create: impl Fn(u32) -> bool,
    create: impl Fn(u32) -> Option<u32>,
    mut emit: impl FnMut(u32, u32),
) {
    let idx = table.partition_point(|e| e.0 < key);
    let mut found = table.get(idx).filter(|e| e.0 >= key).map(|e| e.1);
    if found.is_none() {
        if can_create(key) {
            found = create(key);
        }
        if found.is_none() {
            emit(key, arg);
            return;
        }
    }
    emit(found.unwrap(), arg);
}

#[cfg(test)]
mod tests_wave31_agent58 {
    use super::*;

    #[test]
    fn small_helpers() {
        let mut c = [0u8; 0x100];
        assert!(!w31a58_ctx_field68_set(&c));
        c[0x68] = 1;
        assert!(w31a58_ctx_field68_set(&c));
        assert_eq!(w31a58_flag_bit15(0x8000), 1);
        assert_eq!(w31a58_flag_bit15(0x7fff), 0);
        assert!(!w31a58_set_rect(&mut c, 1, 2, 3, 4));
        assert_eq!(c[0x38], 4);
        assert_eq!(c[0x3c], 6);
        c[0x90] = 1;
        assert!(w31a58_set_rect(&mut c, 0, 0, 0, 0));
        let mut f = 0;
        w31a58_set_byte_flag(&mut f, 5);
        assert_eq!(f, 5);
        assert!(w31a58_bitset_test(&[0x80], 0));
        assert!(!w31a58_bitset_test(&[0x80], 1));
        assert!(!w31a58_bitset_test(&[0x80], 99));
        let mut n = 3;
        let mut r = (0, 0);
        w31a58_vec8_clear(&mut n, |a, b| r = (a, b));
        assert_eq!((n, r), (0, (0, 24)));
    }

    #[test]
    fn accept() {
        assert_eq!(w31a58_can_accept(0, 1, false, 0, 0), 0);
        assert_eq!(w31a58_can_accept(0, 0, true, 0, 0), 0);
        assert_eq!(w31a58_can_accept(0, 0, false, 5, 6), 0);
        assert_eq!(w31a58_can_accept(0, 0, false, 5, 5), 1);
        assert_eq!(w31a58_can_accept(1 << 25, 0, false, 5, 6), 1);
    }

    #[test]
    fn lookup() {
        let t = [(1, 10), (5, 50)];
        let mut out = vec![];
        w31a58_lookup_and_emit(&t, 2, 9, |_| false, |_| None, |k, a| out.push((k, a)));
        w31a58_lookup_and_emit(&t, 5, 9, |_| false, |_| None, |k, a| out.push((k, a)));
        w31a58_lookup_and_emit(&t, 7, 9, |_| true, |_| Some(77), |k, a| out.push((k, a)));
        assert_eq!(out, vec![(50, 9), (50, 9), (77, 9)]);
    }
}
