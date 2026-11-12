#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave21/agent04.rs
// ===========================================================================
// Wave 21 translation batch 04: flingy/sprite position helpers and CRT
// timezone rule arithmetic.  Records are borrowed byte slices.

fn w21a04_rd_i16(b: &[u8], o: usize) -> i16 {
    i16::from_le_bytes([b[o], b[o + 1]])
}
fn w21a04_wr_i16(b: &mut [u8], o: usize, v: i16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w21a04_rd_i32(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn w21a04_wr_i32(b: &mut [u8], o: usize, v: i32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

/// `0x00569350`: clamps the 16-bit position (+0x28, +0x2a) into
/// `0..width` / `0..height`, rewriting the 24.8 mirrors (+0x2c, +0x30) and
/// calling `notify(x, y)` (callee 57d730) for every clamp.
pub fn w21a04_clamp_position(
    rec: &mut [u8],
    width: u16,
    height: u16,
    mut notify: impl FnMut(i16, i16),
) {
    let x = w21a04_rd_i16(rec, 0x28);
    if x < 0 {
        w21a04_wr_i16(rec, 0x28, 0);
        w21a04_wr_i32(rec, 0x2c, 0);
        let y = w21a04_rd_i16(rec, 0x2a);
        w21a04_wr_i32(rec, 0x30, (y as i32) << 8);
        notify(0, y);
    } else if width as i32 <= x as i32 {
        let nx = (width as i16).wrapping_sub(1);
        w21a04_wr_i32(rec, 0x2c, (nx as i32) << 8);
        w21a04_wr_i16(rec, 0x28, nx);
        let y = w21a04_rd_i16(rec, 0x2a);
        w21a04_wr_i32(rec, 0x30, (y as i32) << 8);
        notify(nx, y);
    }
    let y = w21a04_rd_i16(rec, 0x2a);
    if y < 0 {
        w21a04_wr_i16(rec, 0x2a, 0);
        let x = w21a04_rd_i16(rec, 0x28);
        w21a04_wr_i32(rec, 0x2c, (x as i32) << 8);
        w21a04_wr_i32(rec, 0x30, 0);
        notify(x, 0);
    } else if height as i32 <= y as i32 {
        let ny = (height as i16).wrapping_sub(1);
        let x = w21a04_rd_i16(rec, 0x28);
        w21a04_wr_i32(rec, 0x2c, (x as i32) << 8);
        w21a04_wr_i16(rec, 0x2a, ny);
        w21a04_wr_i32(rec, 0x30, (ny as i32) << 8);
        notify(x, ny);
    }
}

/// `0x00569a20`: sets +0x3c to `speed`; when +0x38 differs, stores it and
/// recomputes the velocity pair (+0x40, +0x44) from the direction table
/// (8-byte rows of two i32, indexed by the byte at +0x4a), `>> 8`.
pub fn w21a04_set_speed(rec: &mut [u8], speed: i32, dir_table: &[[i32; 2]]) {
    w21a04_wr_i32(rec, 0x3c, speed);
    if w21a04_rd_i32(rec, 0x38) != speed {
        w21a04_wr_i32(rec, 0x38, speed);
        w21a04_update_velocity_inner(rec, dir_table);
    }
}

fn w21a04_update_velocity_inner(rec: &mut [u8], dir_table: &[[i32; 2]]) {
    let row = dir_table[rec[0x4a] as usize];
    let s = w21a04_rd_i32(rec, 0x38);
    w21a04_wr_i32(rec, 0x40, row[0].wrapping_mul(s) >> 8);
    w21a04_wr_i32(rec, 0x44, row[1].wrapping_mul(s) >> 8);
}

/// `0x0056c7b0`: recomputes velocity (+0x40, +0x44) from the current speed
/// at +0x38 and the direction row selected by the byte at +0x4a.
pub fn w21a04_update_velocity(rec: &mut [u8], dir_table: &[[i32; 2]]) {
    w21a04_update_velocity_inner(rec, dir_table);
}

/// `0x00569ae0`: flingy.dat row pointer to 1-based id (0 for null); rows
/// are 0x4c bytes starting at `0x00f9_5ab8`.
pub fn w21a04_flingy_ptr_to_id(ptr: u32) -> u32 {
    if ptr == 0 {
        0
    } else {
        ptr.wrapping_sub(0x00f9_5ab8) / 0x4c + 1
    }
}

/// `0x0056c850`: stores `value` at +0x34.
pub fn w21a04_set_field_34(rec: &mut [u8], value: i32) {
    w21a04_wr_i32(rec, 0x34, value);
}

/// `0x0056c860`: sets the 24.8 position (+0x2c, +0x30) and its integer
/// part (+0x28, +0x2a = value >> 8), then calls `notify(x, y)`.
pub fn w21a04_set_position_fixed(rec: &mut [u8], x: i32, y: i32, mut notify: impl FnMut(u16, u16)) {
    w21a04_wr_i16(rec, 0x28, ((x as u32) >> 8) as i16);
    w21a04_wr_i16(rec, 0x2a, ((y as u32) >> 8) as i16);
    w21a04_wr_i32(rec, 0x2c, x);
    w21a04_wr_i32(rec, 0x30, y);
    notify(((x >> 8) & 0xffff) as u16, ((y >> 8) & 0xffff) as u16);
}

/// `0x0056c8c0`: updates the (x, y) pair at +0x10/+0x12 (flags at +0x20
/// become `(f & 0xfb) | 1`, +0x14 cleared, +0x18 = dword at +0x10) when it
/// changed, and the pair at +0x1c/+0x1e when it changed.
pub fn w21a04_set_target_pair(rec: &mut [u8], x: i32, y: i32) {
    if x != w21a04_rd_i16(rec, 0x10) as i32 || y != w21a04_rd_i16(rec, 0x12) as i32 {
        w21a04_wr_i16(rec, 0x10, x as i16);
        w21a04_wr_i16(rec, 0x12, y as i16);
        rec[0x20] = (rec[0x20] & 0xfb) | 1;
        w21a04_wr_i32(rec, 0x14, 0);
        let v = w21a04_rd_i32(rec, 0x10);
        w21a04_wr_i32(rec, 0x18, v);
    }
    if x != w21a04_rd_i16(rec, 0x1c) as i32 || y != w21a04_rd_i16(rec, 0x1e) as i32 {
        w21a04_wr_i16(rec, 0x1c, x as i16);
        w21a04_wr_i16(rec, 0x1e, y as i16);
    }
}

/// Output of the timezone rule conversion (globals 00f4e9d8..00f4e9ec).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct W21a04TzRule {
    pub year: i32,
    pub day: i32,
    pub ms: i32,
}

/// `0x00cafd6b`: CRT timezone transition-rule computation.  `cum_normal`
/// / `cum_leap` are the cumulative-day tables (index = month);
/// `is_leap(year)` (c9c63a), `leap_days(year)` (c9c600) and
/// `bias_secs()` (cafaf7, `None` = fatal error) are callees.  `end_rule`
/// is the `param_1 != 0` mode, which applies the bias to the time of day
/// and normalises across day boundaries.  Returns `None` on bias failure.
#[allow(clippy::too_many_arguments)]
pub fn w21a04_tz_rule(
    end_rule: bool,
    mode: i32,
    year: i32,
    month: usize,
    week: i32,
    wday: i32,
    mday: i32,
    hour: i32,
    min: i32,
    sec: i32,
    ms: i32,
    cum_normal: &[i32],
    cum_leap: &[i32],
    is_leap: impl Fn(i32) -> bool,
    leap_days: impl Fn(i32) -> i32,
    bias_secs: impl Fn() -> Option<i32>,
) -> Option<W21a04TzRule> {
    let leap = is_leap(year);
    let table = if leap { cum_leap } else { cum_normal };
    let mut day;
    if mode == 1 {
        let base = table[month];
        let d = (leap_days(year) + base + 1 + year * 0x16d - 0x63ca) % 7;
        day = base + 1 + (week * 7 - d) + wday;
        if d <= wday {
            day -= 7;
        }
        let month_end = table[month + 1];
        if week == 5 && month_end < day {
            day -= 7;
        }
    } else {
        day = table[month] + mday;
    }
    let mut t = ms + ((hour * 0x3c + min) * 0x3c + sec) * 1000;
    if !end_rule {
        return Some(W21a04TzRule { year, day, ms: t });
    }
    let bias = bias_secs()?;
    t += bias * 1000;
    if t < 0 {
        t += 86_400_000;
        day -= 1;
    } else if t > 86_399_999 {
        t -= 86_400_000;
        day += 1;
    }
    Some(W21a04TzRule { year, day, ms: t })
}

/// Skipped: FUN_00568fc0 vector resize (noreturn memmove/alloc glue),
/// FUN_005698b0/005698d0/005698f0/00569a60/00569c00/00569dc0/0056c450/
/// 0056c69a/00596f60/005b1710 (call chains, SEH, file loaders),
/// FUN_00cb2813 (mktime glue with opaque callees).
#[cfg(test)]
mod tests_wave21_agent04 {
    use super::*;

    fn dir() -> Vec<[i32; 2]> {
        (0..4).map(|i| [256 * (i + 1), -128 * (i + 1)]).collect()
    }

    #[test]
    fn clamp_low_and_high() {
        let mut r = [0u8; 0x50];
        w21a04_wr_i16(&mut r, 0x28, -5);
        w21a04_wr_i16(&mut r, 0x2a, 7);
        let mut calls = vec![];
        w21a04_clamp_position(&mut r, 10, 10, |x, y| calls.push((x, y)));
        assert_eq!(calls, vec![(0, 7)]);
        assert_eq!(w21a04_rd_i32(&r, 0x30), 7 << 8);
        w21a04_wr_i16(&mut r, 0x2a, 12);
        calls.clear();
        w21a04_clamp_position(&mut r, 10, 10, |x, y| calls.push((x, y)));
        assert_eq!(calls, vec![(0, 9)]);
        assert_eq!(w21a04_rd_i32(&r, 0x30), 9 << 8);
        assert_eq!(w21a04_rd_i32(&r, 0x2c), 0);
        calls.clear();
        w21a04_wr_i16(&mut r, 0x28, 10);
        w21a04_clamp_position(&mut r, 10, 10, |x, y| calls.push((x, y)));
        assert_eq!(calls, vec![(9, 9)]);
    }

    #[test]
    fn speed_velocity() {
        let mut r = [0u8; 0x50];
        r[0x4a] = 1;
        w21a04_set_speed(&mut r, 512, &dir());
        assert_eq!(w21a04_rd_i32(&r, 0x3c), 512);
        assert_eq!(w21a04_rd_i32(&r, 0x40), 1024);
        assert_eq!(w21a04_rd_i32(&r, 0x44), -512);
        r[0x4a] = 2;
        w21a04_set_speed(&mut r, 512, &dir());
        assert_eq!(w21a04_rd_i32(&r, 0x40), 1024); // unchanged speed
        w21a04_update_velocity(&mut r, &dir());
        assert_eq!(w21a04_rd_i32(&r, 0x40), 1536);
    }

    #[test]
    fn ptr_id_and_field() {
        assert_eq!(w21a04_flingy_ptr_to_id(0), 0);
        assert_eq!(w21a04_flingy_ptr_to_id(0x00f9_5ab8), 1);
        assert_eq!(w21a04_flingy_ptr_to_id(0x00f9_5ab8 + 0x4c * 3), 4);
        let mut r = [0u8; 0x50];
        w21a04_set_field_34(&mut r, -2);
        assert_eq!(w21a04_rd_i32(&r, 0x34), -2);
    }

    #[test]
    fn fixed_position() {
        let mut r = [0u8; 0x50];
        let mut c = None;
        w21a04_set_position_fixed(&mut r, 0x1234, -256, |x, y| c = Some((x, y)));
        assert_eq!(w21a04_rd_i16(&r, 0x28), 0x12);
        assert_eq!(w21a04_rd_i16(&r, 0x2a), -1);
        assert_eq!(c, Some((0x12, 0xffff)));
    }

    #[test]
    fn target_pair() {
        let mut r = [0u8; 0x50];
        r[0x20] = 0x04;
        w21a04_set_target_pair(&mut r, 3, 4);
        assert_eq!(r[0x20], 1);
        assert_eq!(w21a04_rd_i32(&r, 0x18), 3 | (4 << 16));
        assert_eq!(w21a04_rd_i16(&r, 0x1e), 4);
        r[0x20] = 0x04;
        w21a04_set_target_pair(&mut r, 3, 4);
        assert_eq!(r[0x20], 4);
    }

    #[test]
    fn tz_rule() {
        let n = [-1, 30, 58, 89, 119, 150, 180, 211, 242, 272, 303, 333, 364];
        let l = [-1, 30, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334, 365];
        let leap = |y: i32| y % 4 == 0;
        let ld = |_y: i32| 0;
        let r = w21a04_tz_rule(
            false,
            0,
            100,
            2,
            0,
            0,
            10,
            2,
            0,
            0,
            5,
            &n,
            &l,
            leap,
            ld,
            || Some(0),
        )
        .unwrap();
        assert_eq!(
            r,
            W21a04TzRule {
                year: 100,
                day: 69,
                ms: 7_200_005
            }
        );
        let r = w21a04_tz_rule(
            true,
            0,
            101,
            2,
            0,
            0,
            10,
            0,
            0,
            0,
            0,
            &n,
            &l,
            leap,
            ld,
            || Some(-1),
        )
        .unwrap();
        assert_eq!(r.ms, 86_399_000);
        assert_eq!(r.day, 67);
        assert!(
            w21a04_tz_rule(
                true,
                0,
                101,
                2,
                0,
                0,
                10,
                0,
                0,
                0,
                0,
                &n,
                &l,
                leap,
                ld,
                || None
            )
            .is_none()
        );
        let r = w21a04_tz_rule(
            false,
            1,
            101,
            2,
            2,
            0,
            0,
            0,
            0,
            0,
            0,
            &n,
            &l,
            leap,
            ld,
            || Some(0),
        )
        .unwrap();
        assert!(r.day > 58 && r.day < 89);
    }
}
