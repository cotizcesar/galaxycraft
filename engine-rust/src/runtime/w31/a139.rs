#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent139.rs
// ===========================================================================
// Wave 31 translation batch 139: CRT calendar helpers.

/// `0x00cb111a`: validates a day-of-month.  `cum` is the cumulative days
/// table (index `month + 1` minus index `month` is the month length);
/// `leap_feb_ok` is the result of the leap-year callee.  Returns false for
/// `day < 1`; February (month 1) in a leap year allows day 29.
pub fn w31a139_day_valid(cum: &[i32], month: usize, day: i32, leap: bool) -> bool {
    if day < 1 {
        return false;
    }
    let len = cum[month + 1] - cum[month];
    if len < day && (!leap || month != 1 || day > 0x1d) {
        return false;
    }
    true
}

/// Two-digit store used by the asctime formatter (callee at 0xcb_10e1):
/// pads with `'0'` when `pad` is set, otherwise with a space.
pub fn w31a139_store_two(out: &mut Vec<u8>, val: i32, pad: bool) {
    if val < 10 {
        out.push(if pad { b'0' } else { b' ' });
    }
    out.extend_from_slice(val.to_string().as_bytes());
}

/// `0x00cb0f70`: asctime formatter.  Fields are `[sec, min, hour, mday,
/// mon, year-1900, wday]`.  Returns the 26-byte text "Www Mmm dd hh:mm:ss
/// yyyy\n", or the errno `0x16` when the buffer (`cap`) is under 26 or a
/// field is out of range.
pub fn w31a139_asctime(tm: &[i32; 7], cap: usize, cum: &[i32], leap: bool) -> Result<Vec<u8>, u32> {
    let [sec, min, hour, mday, mon, year, wday] = *tm;
    let ok = cap > 0x19
        && year >= 0
        && (0..12).contains(&mon)
        && (0..24).contains(&hour)
        && (0..60).contains(&min)
        && (0..61).contains(&sec)
        && (0..7).contains(&wday)
        && w31a139_day_valid(cum, mon as usize, mday, leap);
    if !ok {
        return Err(0x16);
    }
    let mut out = Vec::with_capacity(26);
    let d = wday as usize * 3;
    out.extend_from_slice(&b"SunMonTueWedThuFriSat"[d..d + 3]);
    out.push(b' ');
    let m = mon as usize * 3;
    out.extend_from_slice(&b"JanFebMarAprMayJunJulAugSepOctNovDec"[m..m + 3]);
    out.push(b' ');
    w31a139_store_two(&mut out, mday, false);
    out.push(b' ');
    w31a139_store_two(&mut out, hour, true);
    out.push(b':');
    w31a139_store_two(&mut out, min, true);
    out.push(b':');
    w31a139_store_two(&mut out, sec, true);
    out.push(b' ');
    w31a139_store_two(&mut out, (year + 0x76c) / 100, true);
    w31a139_store_two(&mut out, (year + 0x76c) % 100, true);
    out.push(b'\n');
    Ok(out)
}

/// `0x00cafb4f` (range test only): given `[sec, min, hour, ?, ?, ?, ?,
/// ordinal]` style fields `(sec, min, hour, ord)` and a window
/// `(start_ord, start_ms, end_ord, end_ms)`, returns whether the time falls
/// inside the (possibly wrapping) window.
pub fn w31a139_in_window(
    sec: i32,
    min: i32,
    hour: i32,
    ord: i32,
    win: (i32, i32, i32, i32),
) -> bool {
    let (s_ord, s_ms, e_ord, e_ms) = win;
    let ms = ((hour * 0x3c + min) * 0x3c + sec) * 1000;
    let edge = |ms: i32| {
        if ord == s_ord { s_ms <= ms } else { ms < e_ms }
    };
    if s_ord < e_ord {
        if ord < s_ord || e_ord < ord {
            return false;
        }
        if ord <= s_ord || e_ord <= ord {
            return edge(ms);
        }
    } else if e_ord <= ord && ord <= s_ord {
        if ord <= e_ord || s_ord <= ord {
            return edge(ms);
        }
        return false;
    }
    true
}

#[cfg(test)]
mod tests_wave31_agent139 {
    use super::*;

    fn cum() -> Vec<i32> {
        vec![0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334, 365]
    }

    #[test]
    fn day_valid() {
        let c = cum();
        assert!(!w31a139_day_valid(&c, 0, 0, false));
        assert!(w31a139_day_valid(&c, 0, 31, false));
        assert!(!w31a139_day_valid(&c, 1, 29, false));
        assert!(w31a139_day_valid(&c, 1, 29, true));
        assert!(!w31a139_day_valid(&c, 1, 30, true));
    }

    #[test]
    fn asctime() {
        let c = cum();
        let s = w31a139_asctime(&[5, 4, 3, 2, 0, 99, 1], 26, &c, false).unwrap();
        assert_eq!(s, b"Mon Jan  2 03:04:05 1999\n");
        assert_eq!(w31a139_asctime(&[0; 7], 25, &c, false), Err(0x16));
        assert_eq!(
            w31a139_asctime(&[61, 0, 0, 1, 0, 0, 0], 26, &c, false),
            Err(0x16)
        );
    }

    #[test]
    fn window() {
        assert!(w31a139_in_window(0, 0, 0, 5, (3, 0, 8, 0)));
        assert!(!w31a139_in_window(0, 0, 0, 9, (3, 0, 8, 0)));
        assert!(w31a139_in_window(1, 0, 0, 3, (3, 0, 8, 0)));
        assert!(!w31a139_in_window(0, 0, 0, 3, (3, 5000, 8, 0)));
    }
}
