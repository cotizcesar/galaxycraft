#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent01.rs
// ===========================================================================
// Wave 33 translation batch 01.

/// `0x007a5320`: decodes one code point from `buf[*pos..*pos + *remaining]`
/// using the step callback `step(byte, &mut out, &mut state)` (the routine at
/// FUN_00c67c80).  Step result 0 consumes one byte and returns `out`; 1
/// consumes one byte and marks the sequence as extended; 3 ends the
/// sequence, rewinding to the start byte if it was extended; any other
/// value retries the same byte.  Running out of input or result 3 yields
/// `0xfffd`.  `pos` and `remaining` are updated.
pub fn w33a01_decode_step(
    buf: &[u8],
    pos: &mut usize,
    remaining: &mut usize,
    step: &mut dyn FnMut(u8, &mut u32, &mut u32) -> i32,
) -> u32 {
    let start = *pos;
    let end = start + *remaining;
    let mut p = start;
    let mut simple = true;
    let mut out: u32 = 0;
    let mut state: u32 = 0;
    let mut cur;
    loop {
        if p == end {
            out = 0xfffd;
            break;
        }
        cur = p;
        let b = buf.get(p).copied().unwrap_or(0);
        match step(b, &mut out, &mut state) {
            0 => {
                p += 1;
                *remaining = (*remaining as isize + (start as isize - p as isize)) as usize;
                *pos = p;
                return out;
            }
            1 => {
                p += 1;
                simple = false;
            }
            3 => {
                p += 1;
                if !simple {
                    p = cur;
                }
                out = 0xfffd;
                break;
            }
            _ => {}
        }
    }
    *remaining = (*remaining as isize + (start as isize - p as isize)) as usize;
    *pos = p;
    out
}

/// `0x007a72a0`: validates a NUL-terminated byte string; returns 1 when the
/// count of counted bytes differs from the count of 0x92 markers or the
/// markers differ from those followed by an ASCII-positive byte, 0 otherwise
/// or on an invalid double-byte pair.  `marker_ok(rest)` models FUN_007a76b0
/// and `pair_ok(code)` models FUN_00937520 (must return 1).
pub fn w33a01_validate_text(
    s: &[u8],
    marker_ok: &dyn Fn(&[u8]) -> bool,
    pair_ok: &dyn Fn(u16) -> i32,
) -> u32 {
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    let mut p = 0usize;
    let (mut markers, mut positive, mut counted) = (0i32, 0i32, 0i32);
    let mut c = at(0);
    if c == 0 {
        return 0;
    }
    loop {
        let skip = matches!(c, 1..=8 | 0x0e..=0x11 | 0x15..=0x19 | 0x1b..=0x1f)
            || matches!(c, 0 | 9 | 10 | 12 | 13 | 0x12 | 0x13);
        if !skip {
            counted += 1;
            if c == 0x92 {
                markers += 1;
                let rest = s.get(p + 1..).unwrap_or(&[]);
                if marker_ok(rest) && (at(p + 1) as i8) >= 0 {
                    positive += 1;
                }
            } else if (0x81..=0xfe).contains(&c) {
                p += 1;
                let t = at(p);
                if !(0x41..=0xfe).contains(&t) {
                    return 0;
                }
                if pair_ok(((c as u16) << 8) | t as u16) != 1 {
                    return 0;
                }
            }
        }
        c = at(p + 1);
        p += 1;
        if c == 0 {
            break;
        }
    }
    u32::from(counted != markers || markers != positive)
}

#[cfg(test)]
mod tests_wave33_agent01 {
    use super::*;

    #[test]
    fn decode_single() {
        let buf = [0x41u8, 0x42];
        let (mut p, mut r) = (0, 2);
        let v = w33a01_decode_step(&buf, &mut p, &mut r, &mut |b, o, _| {
            *o = b as u32;
            0
        });
        assert_eq!((v, p, r), (0x41, 1, 1));
    }

    #[test]
    fn decode_empty_and_reject() {
        let buf = [1u8];
        let (mut p, mut r) = (0, 0);
        assert_eq!(
            w33a01_decode_step(&buf, &mut p, &mut r, &mut |_, _, _| 0),
            0xfffd
        );
        let (mut p, mut r) = (0, 1);
        assert_eq!(
            w33a01_decode_step(&buf, &mut p, &mut r, &mut |_, _, _| 3),
            0xfffd
        );
        assert_eq!((p, r), (1, 0));
    }

    #[test]
    fn validate_ascii() {
        let ok = |_: &[u8]| true;
        let pair = |_: u16| 1;
        assert_eq!(w33a01_validate_text(b"\0", &ok, &pair), 0);
        assert_eq!(w33a01_validate_text(b"abc\0", &ok, &pair), 1);
        assert_eq!(w33a01_validate_text(b"\x01\x02\0", &ok, &pair), 0);
        assert_eq!(w33a01_validate_text(b"\x81\x20\0", &ok, &pair), 0);
    }
}
