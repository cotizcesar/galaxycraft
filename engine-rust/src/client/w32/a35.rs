#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent35.rs
// ===========================================================================
// Wave 32 translation batch 35.

/// `0x00c69220`: splits a 64-bit nanosecond timestamp plus a UTC offset in
/// minutes into a 10-field broken-down time: `[nanos, sec, min, hour, mday,
/// mon (0-based), year-1900, wday, yday, offset_minutes]`.  The seconds
/// quotient is truncated to 32 bits like the retail code.
pub fn w32a35_break_down_time(ns: i64, offset_min: i32) -> [i32; 10] {
    let secs = (ns / 1_000_000_000) as i32;
    let t = secs.wrapping_add(offset_min.wrapping_mul(60));
    let mut days = t / 86400;
    let mut rem = t % 86400;
    if rem < 0 {
        days -= 1;
        rem += 86400;
    }
    let d = days.wrapping_add(0xafa6c);
    let q = if d < 0 { days.wrapping_add(0x8bfbc) } else { d } / 0x23ab1;
    let d = d - q * 0x23ab1;
    let y = (d - d / 0x5b4 + d / 0x8eac - d / 0x23ab0) / 0x16d;
    let doy = d - y * 0x16d + y / 100 - ((y + ((y >> 31) & 3)) >> 2);
    let m = (doy * 5 + 2) / 0x99;
    let madj = if m < 10 { 2 } else { -10 };
    let year = q * 400 + i32::from(madj + m < 2) + y;
    let leap_adj = if doy < 0x132 {
        let leap = year % 4 == 0 && (year != (year / 100) * 100 || year == (year / 400) * 400);
        if leap { 0x3c } else { 0x3b }
    } else {
        -0x132
    };
    [
        (ns - (ns / 1_000_000_000) * 1_000_000_000) as i32,
        rem % 60,
        (rem % 3600) / 60,
        rem / 3600,
        doy - (m * 0x99 + 2) / 5 + 1,
        madj + m,
        year - 1900,
        (days + 4) % 7,
        doy + leap_adj,
        offset_min,
    ]
}

#[cfg(test)]
mod tests_wave32_agent35 {
    use super::*;

    #[test]
    fn epoch() {
        assert_eq!(
            w32a35_break_down_time(0, 0),
            [0, 0, 0, 0, 1, 0, 70, 4, 0, 0]
        );
    }

    #[test]
    fn leap_day_2000() {
        // 2000-02-29 12:34:56.5 UTC = 951827696 s
        let r = w32a35_break_down_time(951_827_696_500_000_000, 0);
        assert_eq!(r[..9], [500_000_000, 56, 34, 12, 29, 1, 100, 2, 59]);
    }

    #[test]
    fn offset_and_negative() {
        let r = w32a35_break_down_time(0, -60);
        assert_eq!(r[..9], [0, 0, 0, 23, 31, 11, 69, 3, 364]);
    }
}
