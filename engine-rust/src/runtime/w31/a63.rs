#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent63.rs
// ===========================================================================
// Wave 31 translation batch 63: small audio-emitter parameter helpers.

/// Cone/range snapshot copied out by `0x009a8470`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct W31a63Range {
    pub position: [f32; 3],
    pub min_distance: f32,
    pub max_distance: f32,
}

/// `0x009a8470`: copies the emitter position (+0x208..+0x214), min
/// distance (+0x214) and max distance (+0x218) into the optional outputs.
/// Always returns 0.
pub fn w31a63_get_range(
    src: &W31a63Range,
    pos: Option<&mut [f32; 3]>,
    min: Option<&mut f32>,
    max: Option<&mut f32>,
) -> u32 {
    if let Some(p) = pos {
        *p = src.position;
    }
    if let Some(m) = min {
        *m = src.min_distance;
    }
    if let Some(m) = max {
        *m = src.max_distance;
    }
    0
}

/// Emitter distance-attenuation fields used by `0x009a7ed0`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct W31a63Atten {
    pub enabled: bool,
    pub origin: [f32; 3],
    pub min: f32,
    pub max: f32,
    pub falloff: f32,
}

/// `0x009a7ed0`: computes `(scaled, linear)` attenuation for a point.
/// `sqrt` models FUN_00c94230; `pow10` models the 10.0-based helper
/// FUN_00c9b940 (assumed 10^x).  Disabled emitters give `(1, 1)`.
pub fn w31a63_attenuation(
    a: &W31a63Atten,
    p: [f32; 3],
    sqrt: impl Fn(f32) -> f32,
    pow10: impl Fn(f32) -> f32,
) -> (f32, f32) {
    if !a.enabled {
        return (1.0, 1.0);
    }
    let dy = p[1] - a.origin[1];
    let dx = p[0] - a.origin[0];
    let dz = p[2] - a.origin[2];
    let d = sqrt(dy * dy + dx * dx + dz * dz);
    if a.min < d {
        if d < a.max {
            if 0.0 < a.falloff {
                let lin = 1.0 - (d - a.min) / a.falloff;
                (pow10(lin), lin)
            } else {
                (1.0, 1.0)
            }
        } else {
            (0.0, 0.0)
        }
    } else {
        (1.0, 1.0)
    }
}

/// Parameter block written by `0x009b4f10`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct W31a63Params {
    pub p14c: f32,
    pub p150: f32,
    pub p154: f32,
    pub p158: f32,
    pub flag15c: bool,
}

/// `0x009b4f10`: sets parameter `id` (0..=4); returns 0 or 0x25 for an
/// unknown id.  Ids 0/1 are divided by 100, id 4 is `value >= 0.5`.  The
/// caller performs the locked queue notification when `notify && ret == 0`.
pub fn w31a63_set_param(p: &mut W31a63Params, id: u32, value: f32) -> u32 {
    match id {
        0 => p.p14c = value / 100.0,
        1 => p.p150 = value / 100.0,
        2 => p.p154 = value,
        3 => p.p158 = value,
        4 => p.flag15c = 0.5 <= value,
        _ => return 0x25,
    }
    0
}

/// Round-half-away-from-zero used by FUN_009b77e0 for integer fields.
pub fn w31a63_round_half_away(v: f32) -> i32 {
    if 0.0 <= v {
        (v + 0.5) as i32
    } else {
        (v - 0.5) as i32
    }
}

#[cfg(test)]
mod tests_wave31_agent63 {
    use super::*;

    #[test]
    fn range_copy() {
        let s = W31a63Range {
            position: [1.0, 2.0, 3.0],
            min_distance: 4.0,
            max_distance: 5.0,
        };
        let (mut p, mut a, mut b) = ([0.0; 3], 0.0, 0.0);
        assert_eq!(
            w31a63_get_range(&s, Some(&mut p), Some(&mut a), Some(&mut b)),
            0
        );
        assert_eq!((p, a, b), ([1.0, 2.0, 3.0], 4.0, 5.0));
        assert_eq!(w31a63_get_range(&s, None, None, None), 0);
    }

    #[test]
    fn atten() {
        let mut a = W31a63Atten {
            enabled: false,
            origin: [0.0; 3],
            min: 1.0,
            max: 5.0,
            falloff: 2.0,
        };
        assert_eq!(
            w31a63_attenuation(&a, [9.0, 0.0, 0.0], f32::sqrt, |x| x),
            (1.0, 1.0)
        );
        a.enabled = true;
        assert_eq!(
            w31a63_attenuation(&a, [0.5, 0.0, 0.0], f32::sqrt, |x| x),
            (1.0, 1.0)
        );
        assert_eq!(
            w31a63_attenuation(&a, [9.0, 0.0, 0.0], f32::sqrt, |x| x),
            (0.0, 0.0)
        );
        assert_eq!(
            w31a63_attenuation(&a, [2.0, 0.0, 0.0], f32::sqrt, |x| x * 2.0),
            (1.0, 0.5)
        );
        a.falloff = 0.0;
        assert_eq!(
            w31a63_attenuation(&a, [2.0, 0.0, 0.0], f32::sqrt, |x| x),
            (1.0, 1.0)
        );
    }

    #[test]
    fn params() {
        let mut p = W31a63Params::default();
        assert_eq!(w31a63_set_param(&mut p, 0, 50.0), 0);
        assert_eq!(p.p14c, 0.5);
        w31a63_set_param(&mut p, 1, 25.0);
        assert_eq!(p.p150, 0.25);
        w31a63_set_param(&mut p, 2, 3.0);
        w31a63_set_param(&mut p, 3, 4.0);
        assert_eq!((p.p154, p.p158), (3.0, 4.0));
        w31a63_set_param(&mut p, 4, 0.5);
        assert!(p.flag15c);
        assert_eq!(w31a63_set_param(&mut p, 5, 0.0), 0x25);
    }

    #[test]
    fn rounding() {
        assert_eq!(w31a63_round_half_away(1.5), 2);
        assert_eq!(w31a63_round_half_away(-1.5), -2);
        assert_eq!(w31a63_round_half_away(0.0), 0);
    }
}
