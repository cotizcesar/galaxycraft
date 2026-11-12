#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent42.rs
// ===========================================================================
// Wave 34 agent 42: FMOD DSP parameter / dump helpers.

/// `0x009bc230`: FMOD oscillator DSP parameter query.  `param == 0` yields
/// the waveform type (value as float plus its name; sine, square, saw up,
/// saw down, triangle, noise), `param == 1` the rate formatted `%.02f`.
/// Returns `(value, description)`; other indexes return `None`.
pub fn w34a42_osc_get_param(ty: i32, rate: f32, param: i32) -> Option<(f32, String)> {
    match param {
        0 => {
            let name = match ty {
                0 => "sine",
                1 => "square",
                2 => "saw up",
                3 => "saw down",
                4 => "triangle",
                5 => "noise",
                _ => "",
            };
            Some((ty as f32, name.to_string()))
        }
        1 => Some((rate, format!("{rate:.2}"))),
        _ => None,
    }
}

/// `0x009cc840`: reads the 4-byte version field via `read(4, 1)`; returns
/// 5 on short read, otherwise sets `*flag` when the field equals 1 and
/// returns 0.
pub fn w34a42_read_flag(read_ok: bool, field: i32, flag: &mut u8) -> u32 {
    if !read_ok {
        return 5;
    }
    if field == 1 {
        *flag = 1;
    }
    0
}

/// `0x009ccae0`: like the above but when the field is 1 also reads two
/// 8-byte values; both must succeed to set `*flag`, else returns 5.
pub fn w34a42_read_flag_pair(
    read_ok: bool,
    field: i32,
    read2_ok: bool,
    read3_ok: bool,
    flag: &mut u8,
) -> u32 {
    if read_ok {
        if field != 1 {
            return 0;
        }
        if read2_ok && read3_ok {
            *flag = 1;
            return 0;
        }
    }
    5
}

#[cfg(test)]
mod tests_wave34_agent42 {
    use super::*;

    #[test]
    fn osc() {
        assert_eq!(w34a42_osc_get_param(1, 0.0, 0).unwrap().1, "square");
        assert_eq!(w34a42_osc_get_param(0, 1.5, 1).unwrap().1, "1.50");
        assert!(w34a42_osc_get_param(0, 0.0, 2).is_none());
    }

    #[test]
    fn flags() {
        let mut f = 0;
        assert_eq!(w34a42_read_flag(false, 1, &mut f), 5);
        assert_eq!(w34a42_read_flag(true, 1, &mut f), 0);
        assert_eq!(f, 1);
        let mut g = 0;
        assert_eq!(w34a42_read_flag_pair(true, 2, true, true, &mut g), 0);
        assert_eq!(g, 0);
        assert_eq!(w34a42_read_flag_pair(true, 1, true, false, &mut g), 5);
        assert_eq!(w34a42_read_flag_pair(true, 1, true, true, &mut g), 0);
        assert_eq!(g, 1);
    }
}
