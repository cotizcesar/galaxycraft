#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent31.rs
// ===========================================================================
// Wave 32 translation batch 31.

fn w32a31_mul(a: u32, b: u32) -> (u32, u32) {
    let p = (a as u64) * (b as u64);
    (p as u32, (p >> 32) as u32)
}

fn w32a31_carry(a: u32, b: u32) -> u32 {
    a.checked_add(b).is_none() as u32
}

/// `0x00a88f40`: accumulates four 32x32 partial products into a two-word
/// result and stores `param_8 + 0x40 + param_4` as the third word.
pub fn w32a31_partial_products(
    param_2: u32,
    param_3: u32,
    param_4: i32,
    param_6: u32,
    param_7: u32,
    param_8: i32,
) -> [i32; 3] {
    let (lo6, hi1) = w32a31_mul(param_7, param_2);
    let (lo7, hi2) = w32a31_mul(param_6, param_3);
    let (_, edx) = w32a31_mul(param_6, param_2);
    let t = edx.wrapping_add(lo7);
    let u3 = w32a31_carry(edx, lo7)
        .wrapping_add(w32a31_carry(t, lo6))
        .wrapping_add((0x7fff_ffff < t.wrapping_add(lo6)) as u32);
    let (lo_d, hi_d) = w32a31_mul(param_7, param_3);
    let u4 = u3.wrapping_add(lo_d);
    let u5 = u4.wrapping_add(hi2);
    [
        u5.wrapping_add(hi1) as i32,
        hi_d.wrapping_add(w32a31_carry(u3, lo_d))
            .wrapping_add(w32a31_carry(u4, hi2))
            .wrapping_add(w32a31_carry(u5, hi1)) as i32,
        param_8.wrapping_add(0x40).wrapping_add(param_4),
    ]
}

/// `0x00a8a750`: builds an 8-word record from a source record
/// `[a, b, color, flag, value]`; zeroed when `src[3] == 0`.
pub fn w32a31_build_record(src: &[i32; 5]) -> [i32; 8] {
    let mut out = [0i32; 8];
    if src[3] != 0 {
        let c = src[2] as u32;
        let f = |v: u32| (v as f32 / 255.0f32).to_bits() as i32;
        out[4] = f(c & 0xff);
        out[5] = f((c >> 8) & 0xff);
        out[6] = f((c >> 16) & 0xff);
        out[7] = f(c >> 24);
        out[0] = src[0].wrapping_add((src[0] >> 31) & 3) >> 2;
        out[1] = src[1].wrapping_add((src[1] >> 31) & 3) >> 2;
        out[3] = src[3];
        // Table at image VA 0xd33180: [0.0, 4294967296.0].
        let tbl = [0.0f64, 4294967296.0f64];
        let idx = (src[4] < 0) as usize;
        out[2] = (((src[4] as f64 + tbl[idx]) as f32) / 100.0f32).to_bits() as i32;
    }
    out
}

#[cfg(test)]
mod tests_wave32_agent31 {
    use super::*;

    #[test]
    fn record() {
        assert_eq!(w32a31_build_record(&[1, 2, 3, 0, 4]), [0; 8]);
        let r = w32a31_build_record(&[-5, 9, 0xff, 7, 200]);
        assert_eq!(r[0], -1);
        assert_eq!(r[1], 2);
        assert_eq!(f32::from_bits(r[2] as u32), 2.0);
        assert_eq!(f32::from_bits(r[4] as u32), 1.0);
    }

    #[test]
    fn products() {
        assert_eq!(w32a31_partial_products(0, 0, 1, 0, 0, 2)[2], 0x43);
        assert_eq!(w32a31_partial_products(2, 3, 0, 5, 7, 0)[0], 21);
    }
}
