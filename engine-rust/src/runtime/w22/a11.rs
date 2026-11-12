#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent11.rs
// ===========================================================================
// Wave 22 agent 11: per-slot record accessors (stride 0x4e8, base 0x00faa400).

/// Record stride of the slot table.
pub const W22A11_STRIDE: u32 = 0x4e8;
/// Table base address.
pub const W22A11_BASE: u32 = 0x00faa400;

fn w22a11_addr(base: u32, idx: i32) -> u32 {
    base.wrapping_add((idx as u32).wrapping_mul(W22A11_STRIDE))
}

/// `0x00634a60`: address of field at table+0xf8 for slot `idx`.
pub fn w22a11_field_f8_addr(idx: i32) -> u32 {
    w22a11_addr(0x00faa4f8, idx)
}
/// `0x00634a80`: address of field at table+0xd0 for slot `idx`.
pub fn w22a11_field_d0_addr(idx: i32) -> u32 {
    w22a11_addr(0x00faa4d0, idx)
}
/// `0x00634aa0`: address of field at table+0xa8 for slot `idx`.
pub fn w22a11_field_a8_addr(idx: i32) -> u32 {
    w22a11_addr(0x00faa4a8, idx)
}
/// `0x006352c0`: address of field at table+0x1c0 for slot `idx`.
pub fn w22a11_field_1c0_addr(idx: i32) -> u32 {
    w22a11_addr(0x00faa5c0, idx)
}
/// `0x006352e0`: address of field at table+0x198 for slot `idx`.
pub fn w22a11_field_198_addr(idx: i32) -> u32 {
    w22a11_addr(0x00faa598, idx)
}
/// `0x00635300`: address of field at table+0x170 for slot `idx`.
pub fn w22a11_field_170_addr(idx: i32) -> u32 {
    w22a11_addr(0x00faa570, idx)
}
/// `0x00635320`: address of field at table+0x148 for slot `idx`.
pub fn w22a11_field_148_addr(idx: i32) -> u32 {
    w22a11_addr(0x00faa548, idx)
}

/// `0x00635450`: sets the byte at record+0x25 to 1.
pub fn w22a11_set_flag25(rec: &mut [u8]) {
    rec[0x25] = 1;
}

/// `0x00635890`: always returns 0 (all branches removed as unreachable).
pub fn w22a11_always_zero() -> u32 {
    0
}

/// `0x00635960`: 1 when byte +0x0b is non-zero and bit 0x800 of the u16 at +0x10 is set.
pub fn w22a11_has_flag800(rec: &[u8]) -> u32 {
    let w = u16::from_le_bytes([rec[0x10], rec[0x11]]);
    u32::from(rec[0x0b] != 0 && w & 0x800 != 0)
}

/// `0x006359b0`: true when the byte at record+0x26 is zero.
pub fn w22a11_byte26_is_zero(rec: &[u8]) -> bool {
    rec[0x26] == 0
}

/// `0x00635b50`: sets the byte at record+0x24 to 0x18.
pub fn w22a11_set_byte24(rec: &mut [u8]) {
    rec[0x24] = 0x18;
}

/// `0x00635ca0`: stores the u32 `src` (read from decoded global + 0xe608) at record+0x1c.
pub fn w22a11_copy_u32_1c(rec: &mut [u8], src: u32) {
    rec[0x1c..0x20].copy_from_slice(&src.to_le_bytes());
}

#[cfg(test)]
mod tests_wave22_agent11 {
    use super::*;

    #[test]
    fn addrs() {
        assert_eq!(w22a11_field_f8_addr(0), 0x00faa4f8);
        assert_eq!(w22a11_field_d0_addr(1), 0x00faa4d0 + 0x4e8);
        assert_eq!(w22a11_field_a8_addr(2), 0x00faa4a8 + 0x9d0);
        assert_eq!(w22a11_field_1c0_addr(0), 0x00faa5c0);
        assert_eq!(w22a11_field_198_addr(0), 0x00faa598);
        assert_eq!(w22a11_field_170_addr(0), 0x00faa570);
        assert_eq!(w22a11_field_148_addr(1), 0x00faa548 + 0x4e8);
        assert_eq!(w22a11_always_zero(), 0);
    }

    #[test]
    fn bytes() {
        let mut r = vec![0u8; 0x4e8];
        assert!(w22a11_byte26_is_zero(&r));
        w22a11_set_flag25(&mut r);
        w22a11_set_byte24(&mut r);
        assert_eq!((r[0x25], r[0x24]), (1, 0x18));
        r[0x26] = 1;
        assert!(!w22a11_byte26_is_zero(&r));
        w22a11_copy_u32_1c(&mut r, 0x01020304);
        assert_eq!(r[0x1c], 4);
        assert_eq!(r[0x1f], 1);
    }

    #[test]
    fn flag800() {
        let mut r = vec![0u8; 0x4e8];
        r[0x11] = 0x08;
        assert_eq!(w22a11_has_flag800(&r), 0);
        r[0x0b] = 1;
        assert_eq!(w22a11_has_flag800(&r), 1);
        r[0x11] = 0;
        assert_eq!(w22a11_has_flag800(&r), 0);
    }
}
