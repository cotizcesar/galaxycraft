#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent130.rs
// ===========================================================================
// Wave 31 agent 130: CRT string/number helpers (runtime.rs target).

/// `0x00c9cb00`: swaps `count` bytes between two buffers (`_swap`-style
/// helper); a no-op when the buffers are the same object.
pub fn w31a130_swap_bytes(a: &mut [u8], b: &mut [u8], count: usize) {
    for i in 0..count {
        std::mem::swap(&mut a[i], &mut b[i]);
    }
}

/// `0x00c9ca61`: C-locale `strupr` path: ASCII `a..z` are mapped to
/// `A..Z` in place up to the NUL terminator.  `None` models the null
/// pointer (errno 0x16, returns null).
pub fn w31a130_strupr_c(s: Option<&mut [u8]>) -> Option<&mut [u8]> {
    let s = s?;
    for c in s.iter_mut() {
        if *c == 0 {
            break;
        }
        if (c.wrapping_add(0x9f)) < 0x1a {
            *c = c.wrapping_sub(0x20);
        }
    }
    Some(s)
}

/// Digit value of a character (0-9, a-z, A-Z) or `u32::MAX`.
pub fn w31a130_digit_value(c: u8) -> u32 {
    if c.wrapping_sub(0x30) < 10 {
        (c - 0x30) as u32
    } else if c.wrapping_add(0x9f) < 0x1a {
        (c - 0x57) as u32
    } else if c.wrapping_add(0xbf) < 0x1a {
        (c - 0x37) as u32
    } else {
        u32::MAX
    }
}

/// `0xc9_d774`: overflow predicate used by the strtol core (flags: 1 signed, 2 negative,
/// 4 overflow already seen).
pub fn w31a130_strtol_overflow(flags: u8, value: u32) -> bool {
    if flags & 4 != 0 {
        return true;
    }
    if flags & 1 == 0 {
        return false;
    }
    if flags & 2 == 0 {
        value >= 0x8000_0000
    } else {
        value > 0x8000_0000
    }
}

/// `0x00c9d7e6`: C-locale `strtol`/`strtoul` core.  Returns the value, the
/// consumed length (0 when nothing parsed) and the errno (0, 0x16 invalid
/// base, 0x22 range).  `signed` is the low bit of the retail `param_5`.
pub fn w31a130_strtol_core(s: &[u8], base: u32, signed: bool) -> (u32, usize, u32) {
    if !(base == 0 || (2..=36).contains(&base)) {
        return (0, 0, 0x16);
    }
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    let mut i = 0usize;
    let mut c;
    loop {
        c = at(i);
        i += 1;
        if !matches!(c, 9..=13 | 0x20) {
            break;
        }
    }
    let mut flags: u8 = signed as u8;
    if c == b'-' || c == b'+' {
        if c == b'-' {
            flags |= 2;
        }
        c = at(i);
        i += 1;
    }
    let mut base = base;
    if base == 0 || base == 16 {
        if w31a130_digit_value(c) == 0 {
            let n = at(i);
            i += 1;
            if n == b'x' || n == b'X' {
                if base == 0 {
                    base = 16;
                }
                c = at(i);
                i += 1;
            } else {
                if base == 0 {
                    base = 8;
                }
                i -= 1;
            }
        } else if base == 0 {
            base = 10;
        }
    }
    let max_div = u32::MAX / base;
    let max_rem = u32::MAX % base;
    let mut acc: u32 = 0;
    loop {
        let d = w31a130_digit_value(c);
        if d == u32::MAX || d >= base {
            break;
        }
        if acc < max_div || (acc == max_div && d <= max_rem) {
            flags |= 8;
            acc = acc.wrapping_mul(base).wrapping_add(d);
        } else {
            flags |= 12;
        }
        c = at(i);
        i += 1;
    }
    // the retail loop leaves the cursor one past the stopping character
    let end = i - 1;
    if flags & 8 == 0 {
        return (0, 0, 0);
    }
    if w31a130_strtol_overflow(flags, acc) {
        let v = if flags & 1 == 0 {
            u32::MAX
        } else if flags & 2 == 0 {
            0x7fff_ffff
        } else {
            0x8000_0000
        };
        (v, end, 0x22)
    } else if flags & 2 != 0 {
        (acc.wrapping_neg(), end, 0)
    } else {
        (acc, end, 0)
    }
}

#[cfg(test)]
mod tests_wave31_agent130 {
    use super::*;

    #[test]
    fn swap_and_upper() {
        let mut a = *b"abc";
        let mut b = *b"xyz";
        w31a130_swap_bytes(&mut a, &mut b, 2);
        assert_eq!((&a, &b), (b"xyc", b"abz"));
        let mut s = *b"aZ{`\0q";
        w31a130_strupr_c(Some(&mut s));
        assert_eq!(&s, b"AZ{`\0q");
        assert!(w31a130_strupr_c(None).is_none());
    }

    #[test]
    fn strtol_cases() {
        assert_eq!(w31a130_strtol_core(b"  42x", 10, true), (42, 4, 0));
        assert_eq!(w31a130_strtol_core(b"-5", 0, true).0, (-5i32) as u32);
        assert_eq!(w31a130_strtol_core(b"0x1f", 0, true), (31, 4, 0));
        assert_eq!(w31a130_strtol_core(b"017", 0, true), (15, 3, 0));
        assert_eq!(w31a130_strtol_core(b"zz", 10, true), (0, 0, 0));
        assert_eq!(w31a130_strtol_core(b"1", 1, true).2, 0x16);
        assert_eq!(w31a130_strtol_core(b"99999999999", 10, true).0, 0x7fff_ffff);
        assert_eq!(
            w31a130_strtol_core(b"-99999999999", 10, true).0,
            0x8000_0000
        );
        assert_eq!(w31a130_strtol_core(b"99999999999", 10, false).0, u32::MAX);
        assert!(w31a130_strtol_overflow(1, 0x8000_0000));
        assert!(!w31a130_strtol_overflow(3, 0x8000_0000));
    }
}
