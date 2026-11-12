#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent106.rs
// ===========================================================================
// Wave 32 translation batch 106.

/// `0x00b7e010`: formats a nanosecond duration into a wide buffer using the
/// largest non-zero unit cascade (d/h/m, h/m/s, m.s, s.ms, ms.us, us.ns, ns).
/// `fmt` models the callee FUN_00937d00 (buffer, capacity, format, args) and
/// returns the number of converted fields.  The ns-only format string lives in
/// an unread data item; `%uns` is assumed.  A zero duration writes a single
/// `'0'` and returns true when `cap > 1`.
pub fn w32a106_format_duration(
    buf: &mut [u16],
    cap: u32,
    ns: u64,
    mut fmt: impl FnMut(&mut [u16], u32, &str, &[u32]) -> i32,
) -> bool {
    let us = ns / 1000;
    let ms = us / 1000;
    let s = ms / 1000;
    let min = s / 60;
    let hr = min / 60;
    let day = hr / 24;
    let h = (hr - day * 24) as u32;
    let m = (min - hr * 60) as u32;
    let sec = (s - min * 60) as u32;
    let msr = (ms - s * 1000) as u32;
    let usr = (us - ms * 1000) as u32;
    let nsr = (ns - us * 1000) as u32;
    if day != 0 {
        return fmt(buf, cap, "%ud%uh%um", &[day as u32, h, m]) == 3;
    }
    if hr != 0 {
        return fmt(buf, cap, "%uh%um%us", &[h, m, sec]) == 3;
    }
    if min != 0 {
        return fmt(buf, cap, "%um%u.%03us", &[m, sec, msr]) == 3;
    }
    if s != 0 {
        return fmt(buf, cap, "%u.%03us", &[sec, msr]) == 2;
    }
    if ms != 0 {
        return fmt(buf, cap, "%u.%03ums", &[msr, usr]) == 2;
    }
    if us != 0 {
        return fmt(buf, cap, "%u.%03uus", &[usr, nsr]) == 2;
    }
    if ns != 0 {
        return fmt(buf, cap, "%uns", &[nsr]) == 1;
    }
    if cap > 1 {
        buf[0] = 0x30;
        return true;
    }
    false
}

/// `0x00b79cd0`: constructor of a growable buffer header: `[1]=param_5`,
/// `[0]=0`, `[2]=inline storage index (6)`, `[3]=0x1000` capacity,
/// `[4]=0`, `[5]=param_1`; then the callee copies params 2..4.
pub fn w32a106_init_buffer_header(hdr: &mut [u32; 6], param_1: u32, param_5: u32) {
    hdr[1] = param_5;
    hdr[0] = 0;
    hdr[2] = 6;
    hdr[3] = 0x1000;
    hdr[4] = 0;
    hdr[5] = param_1;
}

#[cfg(test)]
mod tests_wave32_agent106 {
    use super::*;

    fn run(ns: u64) -> (String, bool) {
        let mut out = String::new();
        let mut b = [0u16; 4];
        let r = w32a106_format_duration(&mut b, 32, ns, |_, _, f, a| {
            let mut s = String::new();
            let mut it = a.iter();
            let mut rest = f;
            while let Some(k) = rest.find('%') {
                s.push_str(&rest[..k]);
                if rest[k..].starts_with("%03u") {
                    s.push_str(&format!("{:03}", it.next().unwrap()));
                    rest = &rest[k + 4..];
                } else {
                    s.push_str(&it.next().unwrap().to_string());
                    rest = &rest[k + 2..];
                }
            }
            s.push_str(rest);
            out = s;
            a.len() as i32
        });
        (out, r)
    }

    #[test]
    fn durations() {
        assert_eq!(run(1500), ("1.500us".into(), true));
        assert_eq!(run(2_000_000_005).0, "2.000s");
        assert_eq!(run(90_000_000_000).0, "1m30.000s");
        assert_eq!(run(3_661_000_000_000).0, "1h1m1s");
        assert_eq!(run(90_061_000_000_000).0, "1d1h1m");
        assert_eq!(run(7).0, "7ns");
        let mut b = [1u16; 2];
        assert!(w32a106_format_duration(&mut b, 2, 0, |_, _, _, _| 0));
        assert_eq!(b[0], 0x30);
        assert!(!w32a106_format_duration(&mut b, 1, 0, |_, _, _, _| 0));
    }

    #[test]
    fn header() {
        let mut h = [9u32; 6];
        w32a106_init_buffer_header(&mut h, 5, 7);
        assert_eq!(h, [0, 7, 6, 0x1000, 0, 5]);
    }
}
