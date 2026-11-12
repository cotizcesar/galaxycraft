#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent19.rs
// ===========================================================================
// Wave 28 agent 19: small UI/render helper reconstructions.

/// `0x00a8ecb0`: seconds elapsed between two millisecond stamps, clamped to 0.5.
pub fn w28a19_elapsed_clamped(now_ms: i32, start_ms: i32) -> f32 {
    let v = ((now_ms - start_ms) as f64 / 1000.0) as f32;
    if v <= 0.5 { v } else { 0.5 }
}

/// `0x00a8eca0`: returns the two globals at consecutive addresses as one 64-bit value (low, high).
pub fn w28a19_pair_u64(lo: u32, hi: u32) -> u64 {
    ((hi as u64) << 32) | lo as u64
}

/// `0x00a8fc70`: alpha byte, `(int)(color[3] * 255.0)` (truncating).
pub fn w28a19_color_alpha_byte(c: &[f32; 4]) -> i32 {
    (c[3] as f64 * 255.0) as i32
}

/// `0x00a8fca0`: channel at index 2 scaled to a byte.
pub fn w28a19_color_ch2_byte(c: &[f32; 4]) -> i32 {
    (c[2] as f64 * 255.0) as i32
}

/// `0x00a8fcd0`: channel at index 1 scaled to a byte.
pub fn w28a19_color_ch1_byte(c: &[f32; 4]) -> i32 {
    (c[1] as f64 * 255.0) as i32
}

/// `0x00a8fd00`: channel at index 0 scaled to a byte.
pub fn w28a19_color_ch0_byte(c: &[f32; 4]) -> i32 {
    (c[0] as f64 * 255.0) as i32
}

/// `0x00a8fd30`: component-wise linear interpolation of two 4-float colors, `a + (b - a) * t`.
pub fn w28a19_color_lerp(a: &[f32; 4], b: &[f32; 4], t: f32) -> [f32; 4] {
    [
        (b[0] - a[0]) * t + a[0],
        (b[1] - a[1]) * t + a[1],
        (b[2] - a[2]) * t + a[2],
        (b[3] - a[3]) * t + a[3],
    ]
}

/// `0x00a90e90`: address of entry `index` in the 0x38-byte record array at `base`.
pub fn w28a19_record_addr(base: u32, index: i32) -> u32 {
    base.wrapping_add((index as u32).wrapping_mul(0x38))
}

/// `0x00a8eb40`: vector push_back of a 4-byte value where the argument may alias an
/// element of the vector (it is re-read from its index when it does).  Here the
/// value is passed by index-or-value: `alias` is the element index if aliased.
pub fn w28a19_push_u32(v: &mut Vec<u32>, value: u32, alias: Option<usize>) {
    let value = match alias {
        Some(i) if i < v.len() => v[i],
        _ => value,
    };
    v.push(value);
}

/// `0x00a917b0`: same push_back for 2-byte elements.
pub fn w28a19_push_u16(v: &mut Vec<u16>, value: u16, alias: Option<usize>) {
    let value = match alias {
        Some(i) if i < v.len() => v[i],
        _ => value,
    };
    v.push(value);
}

/// `0x00a91360`: kind-to-style selector (first part). `kind` is param_3, `id` the
/// low 16 bits of param_1, `flag` the result of the unresolved `FUN_00a8c430`,
/// `class_of_id` the result of `FUN_00a98340(id)`, `override_val` the optional param_9.
pub fn w28a19_style_for_kind(
    id: u16,
    kind: u32,
    flag: bool,
    class_of_id: i32,
    override_val: Option<i32>,
) -> i32 {
    let base = match kind {
        0 => {
            if id < 4 {
                if flag { 0x1a } else { 0xf }
            } else if class_of_id == 2 {
                if flag { 0x19 } else { 0xe }
            } else {
                0
            }
        }
        2 => 5,
        3 => 6,
        4 => 9,
        5 => 0xb,
        6 => 0x1e + flag as i32,
        7 => 0x28,
        _ => 4,
    };
    override_val.unwrap_or(base)
}

/// `0x00a8de50`: width/height defaults. If an explicit size is absent the size is
/// `(int)((dim as unsigned-corrected double) * scale)`; `dim` is the record's
/// int at +0xc / +0x10.  The `0xd33180` table is modelled as `[0, 2^32]`.
pub fn w28a19_default_extent(dim: i32, scale: f32, explicit: Option<i32>) -> i32 {
    match explicit {
        Some(v) => v,
        None => {
            let bias = if dim < 0 { 4294967296.0f64 } else { 0.0 };
            ((dim as f64 + bias) as f32 * scale) as i32
        }
    }
}

#[cfg(test)]
mod tests_wave28_agent19 {
    use super::*;

    #[test]
    fn elapsed() {
        assert_eq!(w28a19_elapsed_clamped(1200, 1000), 0.2);
        assert_eq!(w28a19_elapsed_clamped(5000, 1000), 0.5);
    }
    #[test]
    fn pair() {
        assert_eq!(w28a19_pair_u64(1, 2), 0x2_0000_0001);
    }
    #[test]
    fn bytes() {
        let c = [1.0, 0.5, 0.25, 0.0];
        assert_eq!(w28a19_color_ch0_byte(&c), 255);
        assert_eq!(w28a19_color_ch1_byte(&c), 127);
        assert_eq!(w28a19_color_ch2_byte(&c), 63);
        assert_eq!(w28a19_color_alpha_byte(&c), 0);
    }
    #[test]
    fn lerp() {
        let r = w28a19_color_lerp(&[0.0; 4], &[1.0, 2.0, 3.0, 4.0], 0.5);
        assert_eq!(r, [0.5, 1.0, 1.5, 2.0]);
    }
    #[test]
    fn addr() {
        assert_eq!(w28a19_record_addr(0x1000, 2), 0x1070);
    }
    #[test]
    fn push() {
        let mut v = vec![7u32];
        w28a19_push_u32(&mut v, 0, Some(0));
        w28a19_push_u32(&mut v, 9, None);
        assert_eq!(v, vec![7, 7, 9]);
        let mut w = vec![3u16];
        w28a19_push_u16(&mut w, 0, Some(0));
        assert_eq!(w, vec![3, 3]);
    }
    #[test]
    fn style() {
        assert_eq!(w28a19_style_for_kind(1, 0, true, 0, None), 0x1a);
        assert_eq!(w28a19_style_for_kind(9, 0, false, 2, None), 0xe);
        assert_eq!(w28a19_style_for_kind(9, 0, false, 1, None), 0);
        assert_eq!(w28a19_style_for_kind(0, 6, true, 0, None), 0x1f);
        assert_eq!(w28a19_style_for_kind(0, 7, false, 0, None), 0x28);
        assert_eq!(w28a19_style_for_kind(0, 1, false, 0, None), 4);
        assert_eq!(w28a19_style_for_kind(0, 3, false, 0, Some(77)), 77);
    }
    #[test]
    fn extent() {
        assert_eq!(w28a19_default_extent(100, 0.5, None), 50);
        assert_eq!(w28a19_default_extent(100, 0.5, Some(3)), 3);
    }
}
