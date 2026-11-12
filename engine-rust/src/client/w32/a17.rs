#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent17.rs
// ===========================================================================
// Wave 32 translation batch 17.
//
// Skipped: FUN_00946500 (SEH frame, ends in a non-returning call).

/// `0x0093f1c0`: resolves a handle through `lookup(param_1, 1)` and returns
/// true when `compare(handle, param_2)` yields zero.
pub fn w32a17_handle_compare_is_zero(
    param_1: u32,
    param_2: u32,
    lookup: impl Fn(u32, u32) -> u32,
    compare: impl Fn(u32, u32) -> i32,
) -> bool {
    compare(lookup(param_1, 1), param_2) == 0
}

fn w32a17_be16(d: &[u8], at: usize) -> u16 {
    ((d[at] as u16) << 8) | d[at + 1] as u16
}

/// `0x0095b880`: TrueType advance (hmtx) scaled by `scale / divisor`, plus an
/// optional format-0 kern pair adjustment.  `base` is the table offset
/// (`ctx+4`), `hhea` holds the metric count at `base+0x22`, `hmtx` the 4-byte
/// records, `kern` the optional kern table.  `glyph_a`/`glyph_b` are the two
/// glyph-index lookups (the original calls at `0x96_0490`).  When
/// `divisor == 1` the result is the ceiling of the scaled advance.
#[allow(clippy::too_many_arguments)]
pub fn w32a17_scaled_advance(
    base: usize,
    hhea: &[u8],
    hmtx: &[u8],
    kern: Option<&[u8]>,
    scale: f32,
    divisor: i32,
    want_kern: bool,
    glyph_a: i32,
    glyph_b: i32,
) -> f32 {
    let count = w32a17_be16(hhea, 0x22 + base) as i32;
    let idx = if glyph_a < count {
        base + (glyph_a as usize) * 4
    } else {
        base + (count as usize) * 4 - 4
    };
    let adv = w32a17_be16(hmtx, idx) as i16 as i32;
    let div = divisor as f32;
    let scaled = ((adv as f32) * scale) / div;
    if divisor == 1 {
        return (scaled as f64).ceil() as f32;
    }
    if !want_kern {
        return scaled;
    }
    let mut value = 0i32;
    if let Some(k) = kern {
        let t = &k[base..];
        if w32a17_be16(t, 2) != 0 && w32a17_be16(t, 8) == 1 {
            let pair = ((glyph_b as u32) << 16) | (glyph_a as u32);
            let mut hi = (t[0xb] as i32 - 1) + (t[10] as i32) * 256;
            let mut lo = 0i32;
            while lo <= hi {
                let mid = (hi + lo) >> 1;
                let o = 0x12 + mid as usize * 6;
                let key = u32::from_be_bytes([t[o], t[o + 1], t[o + 2], t[o + 3]]);
                if pair < key {
                    hi = mid - 1;
                } else if pair > key {
                    lo = mid + 1;
                } else {
                    value = w32a17_be16(t, o + 4) as i16 as i32;
                    break;
                }
            }
        }
    }
    ((value as f32) * scale) / div + scaled
}

#[cfg(test)]
mod tests_wave32_agent17 {
    use super::*;

    #[test]
    fn compare_zero() {
        assert!(w32a17_handle_compare_is_zero(
            5,
            7,
            |a, _| a,
            |h, p| h as i32 - 5 + p as i32 - 7
        ));
        assert!(!w32a17_handle_compare_is_zero(5, 7, |a, _| a, |_, _| 1));
    }

    #[test]
    fn advance_and_kern() {
        let mut hhea = vec![0u8; 0x30];
        hhea[0x23] = 2;
        let hmtx = [0, 10, 0, 0, 0, 20, 0, 0];
        assert_eq!(
            w32a17_scaled_advance(0, &hhea, &hmtx, None, 1.0, 1, false, 1, 0),
            20.0
        );
        assert_eq!(
            w32a17_scaled_advance(0, &hhea, &hmtx, None, 1.0, 2, false, 5, 0),
            10.0
        );
        let mut k = vec![0u8; 0x20];
        k[3] = 1;
        k[9] = 1;
        k[11] = 1; // one pair
        k[0x12..0x16].copy_from_slice(&((3u32 << 16) | 1).to_be_bytes());
        k[0x16..0x18].copy_from_slice(&(-4i16).to_be_bytes());
        let r = w32a17_scaled_advance(0, &hhea, &hmtx, Some(&k), 1.0, 2, true, 1, 3);
        assert_eq!(r, 10.0 - 2.0);
    }
}
