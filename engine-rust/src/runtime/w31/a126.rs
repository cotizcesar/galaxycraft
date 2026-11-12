#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent126.rs
// ===========================================================================
// Wave 31 agent 126: CRT floating-point status / floor / validation helpers.

/// `0x00c9383d`: maps the x87/SSE exception-flag word `flags` (bits
/// 0x1 0x4 0x8 0x10 0x20) to the portable bit order, applies the
/// caller-supplied `native` conversion (the opaque callee), ORs it into
/// `mask`, and reports the errno-style code the routine stores:
/// `0x21` when bit 1 is set, else `0x22` when any of bits 0x4/0x8/0x10 is
/// set, else `None`.  Returns `(merged_mask, errno)`.
pub fn w31a126_map_fp_flags(
    flags: u32,
    mask: u32,
    native: impl Fn(u32) -> u32,
) -> (u32, Option<u32>) {
    let mut p = flags;
    if p & 0x18 != 0 {
        p |= 0x20;
    }
    let mut v = (p >> 5) & 1;
    if p & 0x10 != 0 {
        v |= 2;
    }
    if p & 8 != 0 {
        v |= 4;
    }
    if p & 4 != 0 {
        v |= 8;
    }
    if p & 1 != 0 {
        v |= 0x10;
    }
    let merged = mask | native(v);
    let errno = if p & 1 != 0 {
        Some(0x21)
    } else if p & 0x1c != 0 {
        Some(0x22)
    } else {
        None
    };
    (merged, errno)
}

/// `0x00c94358`: argument validation of the CRT compare routine in its
/// non-locale path.  Returns `Err(0x7fffffff)` (errno EINVAL 0x16) when
/// either pointer is null or the count is >= 0x80000000, else `Ok(())`
/// meaning the caller proceeds to the underlying compare.
pub fn w31a126_validate_compare_args(a_null: bool, b_null: bool, count: u32) -> Result<(), i32> {
    if a_null || b_null || count >= 0x8000_0000 {
        Err(0x7fff_ffff)
    } else {
        Ok(())
    }
}

/// `0x00c94490`: the SSE fast path of the CRT `floor` for finite inputs
/// (exponent-mask arithmetic on the IEEE-754 bits).  Non-finite values and
/// out-of-range magnitudes are returned unchanged.
pub fn w31a126_floor_fast(x: f64) -> f64 {
    let bits = x.to_bits();
    let hi = (bits >> 32) as u32;
    let e = hi >> 20; // sign + 11-bit exponent
    let shift = 0x433u32.wrapping_sub(e & 0x7ff) as u64;
    if e & 0x800 == 0 {
        if e < 0x3ff {
            return 0.0;
        }
        if e < 0x433 {
            return f64::from_bits((bits >> shift) << shift);
        }
    } else if !x.is_nan() {
        if e < 0xbff {
            return if x < -0.0 { -1.0 } else { 0.0 };
        }
        if e < 0xc33 {
            let t = f64::from_bits((bits >> shift) << shift);
            return if x < t { t - 1.0 } else { t };
        }
    }
    x
}

#[cfg(test)]
mod tests_wave31_agent126 {
    use super::*;

    #[test]
    fn flags_map() {
        let (m, e) = w31a126_map_fp_flags(1, 0x100, |v| v);
        assert_eq!((m, e), (0x110, Some(0x21)));
        let (m, e) = w31a126_map_fp_flags(8, 0, |v| v);
        assert_eq!((m, e), (0x5, Some(0x22)));
        assert_eq!(w31a126_map_fp_flags(0, 3, |v| v), (3, None));
    }

    #[test]
    fn validate() {
        assert_eq!(
            w31a126_validate_compare_args(true, false, 1),
            Err(0x7fff_ffff)
        );
        assert_eq!(
            w31a126_validate_compare_args(false, false, 0x8000_0000),
            Err(0x7fff_ffff)
        );
        assert_eq!(
            w31a126_validate_compare_args(false, false, 0x7fff_ffff),
            Ok(())
        );
    }

    #[test]
    fn floor() {
        for x in [
            0.5, 1.5, 2.0, -0.5, -1.5, -2.0, 123.75, -123.75, 1e20, -1e20,
        ] {
            assert_eq!(w31a126_floor_fast(x), x.floor(), "{x}");
        }
        assert!(w31a126_floor_fast(f64::NAN).is_nan());
    }
}
