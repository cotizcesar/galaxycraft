#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent13.rs
// ===========================================================================
// Wave 23 agent 13: JSON value coercions and buffer growth (runtime.rs).

/// JSON value flag bits at offset `+0xe` (patch 1.23.10 layout).
pub const W23A13_F_BOOL: u16 = 0x0008;
/// Signed 32-bit flag.
pub const W23A13_F_I32: u16 = 0x0020;
/// Unsigned 32-bit flag.
pub const W23A13_F_U32: u16 = 0x0040;
/// 64-bit flag (bit 7).
pub const W23A13_F_I64: u16 = 0x0080;
/// 64-bit flag (bit 8).
pub const W23A13_F_U64: u16 = 0x0100;
/// Double flag.
pub const W23A13_F_F64: u16 = 0x0200;

/// Error for "Unexpected JSON type.".
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W23a13BadType;

fn w23a13_is64(flags: u16) -> bool {
    flags & (W23A13_F_U64 | W23A13_F_I64) != 0
}

/// `0x00705600`: JSON value to bool.  Bool-typed values are true only when
/// the flag word is exactly 10; numbers compare against zero.
pub fn w23a13_as_bool(flags: u16, payload: u64, as_f64: f64) -> Result<bool, W23a13BadType> {
    if flags & W23A13_F_BOOL != 0 {
        return Ok(flags == 10);
    }
    if w23a13_is64(flags) {
        return Ok(payload != 0);
    }
    if flags & (W23A13_F_U32 | W23A13_F_I32) != 0 {
        return Ok(payload as u32 != 0);
    }
    if flags & W23A13_F_F64 == 0 {
        return Err(W23a13BadType);
    }
    Ok(as_f64 != 0.0)
}

/// `0x007056c0`: JSON value to i64 (as raw u64); doubles truncate.
pub fn w23a13_as_i64(flags: u16, payload: u64, as_f64: f64) -> Result<u64, W23a13BadType> {
    if w23a13_is64(flags) {
        return Ok(payload);
    }
    if flags & W23A13_F_U32 != 0 {
        return Ok(payload as u32 as u64);
    }
    if flags & W23A13_F_I32 != 0 {
        return Ok(payload as u32 as i32 as i64 as u64);
    }
    if flags & W23A13_F_F64 != 0 {
        return Ok(as_f64 as i64 as u64);
    }
    if flags & W23A13_F_BOOL != 0 {
        return Ok((flags == 10) as u64);
    }
    Err(W23a13BadType)
}

/// `0x00705770`: JSON value to u32; doubles truncate through i32.
pub fn w23a13_as_u32(flags: u16, payload: u64, as_f64: f64) -> Result<u32, W23a13BadType> {
    if w23a13_is64(flags) || flags & (W23A13_F_U32 | W23A13_F_I32) != 0 {
        return Ok(payload as u32);
    }
    if flags & W23A13_F_F64 != 0 {
        return Ok(as_f64 as i32 as u32);
    }
    if flags & W23A13_F_BOOL != 0 {
        return Ok((flags == 10) as u32);
    }
    Err(W23a13BadType)
}

/// `0x00705830`: JSON value to u64; doubles in `[2^63, 2^64)` are
/// converted via the bias-and-flip trick.
pub fn w23a13_as_u64(flags: u16, payload: u64, as_f64: f64) -> Result<u64, W23a13BadType> {
    if w23a13_is64(flags) {
        return Ok(payload);
    }
    if flags & W23A13_F_U32 != 0 {
        return Ok(payload as u32 as u64);
    }
    if flags & W23A13_F_I32 != 0 {
        return Ok(payload as u32 as i32 as i64 as u64);
    }
    if flags & W23A13_F_F64 != 0 {
        let bias = 9.223372036854776e18_f64;
        if as_f64 >= bias && as_f64 - bias < bias {
            return Ok(((as_f64 - bias) as i64 as u64) ^ (1u64 << 63));
        }
        return Ok(as_f64 as i64 as u64);
    }
    if flags & W23A13_F_BOOL != 0 {
        return Ok((flags == 10) as u64);
    }
    Err(W23a13BadType)
}

/// `0x00704980`: new byte capacity for an 8-byte-element buffer growth.
/// `None` when begin is null and the inline flag is zero (fatal path).
pub fn w23a13_grow_bytes(
    inline_flag: u32,
    begin: u32,
    cur: u32,
    cap_end: u32,
    inline_cap: u32,
    n_elems: u32,
) -> Option<u32> {
    let a = if begin == 0 {
        if inline_flag == 0 {
            return None;
        }
        inline_cap
    } else {
        let used = cap_end.wrapping_sub(begin);
        used.wrapping_add(used.wrapping_add(1) >> 1)
    };
    let b = n_elems
        .wrapping_mul(8)
        .wrapping_sub(begin)
        .wrapping_add(cur);
    Some(if b <= a { a } else { b })
}

#[cfg(test)]
mod tests_wave23_agent13 {
    use super::*;

    #[test]
    fn bool_coercion() {
        assert_eq!(w23a13_as_bool(10, 0, 0.0), Ok(true));
        assert_eq!(w23a13_as_bool(9, 1, 0.0), Ok(false));
        assert_eq!(w23a13_as_bool(0x40, 0x1_0000_0000, 0.0), Ok(false));
        assert_eq!(w23a13_as_bool(0x100, 0x1_0000_0000, 0.0), Ok(true));
        assert_eq!(w23a13_as_bool(0x200, 0, 0.5), Ok(true));
        assert_eq!(w23a13_as_bool(0x04, 0, 0.0), Err(W23a13BadType));
    }

    #[test]
    fn i64_coercion() {
        assert_eq!(w23a13_as_i64(0x20, 0xffff_ffff, 0.0), Ok(u64::MAX));
        assert_eq!(w23a13_as_i64(0x40, 0xffff_ffff, 0.0), Ok(0xffff_ffff));
        assert_eq!(w23a13_as_i64(0x200, 0, -2.7), Ok((-2i64) as u64));
        assert_eq!(w23a13_as_i64(10, 0, 0.0), Ok(1));
        assert_eq!(w23a13_as_i64(0, 0, 0.0), Err(W23a13BadType));
    }

    #[test]
    fn u32_coercion() {
        assert_eq!(w23a13_as_u32(0x100, 0x1_0000_0007, 0.0), Ok(7));
        assert_eq!(w23a13_as_u32(0x200, 0, -1.0), Ok(u32::MAX));
        assert_eq!(w23a13_as_u32(9, 0, 0.0), Ok(0));
        assert_eq!(w23a13_as_u32(0, 0, 0.0), Err(W23a13BadType));
    }

    #[test]
    fn u64_coercion() {
        assert_eq!(w23a13_as_u64(0x20, 0xffff_fffe, 0.0), Ok(u64::MAX - 1));
        let big = 1.5 * 9.223372036854776e18_f64;
        assert_eq!(w23a13_as_u64(0x200, 0, big), Ok(0xC000_0000_0000_0000));
        assert_eq!(w23a13_as_u64(0x200, 0, 12.0), Ok(12));
        assert_eq!(w23a13_as_u64(10, 0, 0.0), Ok(1));
        assert_eq!(w23a13_as_u64(1, 0, 0.0), Err(W23a13BadType));
    }

    #[test]
    fn grow() {
        assert_eq!(w23a13_grow_bytes(0, 0, 0, 0, 0, 1), None);
        assert_eq!(w23a13_grow_bytes(1, 0, 0, 0, 100, 1), Some(100));
        // used=16 -> 24; need 8 + (16-... ) = 8*1 - 1000 + 1016 = 24
        assert_eq!(w23a13_grow_bytes(0, 1000, 1016, 1016, 0, 1), Some(24));
        assert_eq!(w23a13_grow_bytes(0, 1000, 1016, 1016, 0, 4), Some(48));
    }
}
