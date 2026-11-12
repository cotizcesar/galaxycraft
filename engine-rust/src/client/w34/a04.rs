#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent04.rs
// ===========================================================================
// Wave 34 translation batch 04.

/// `0x00c67c80`: incremental UTF-8 decoder step.  `state` packs the partial
/// code point (low 24 bits), the bytes still expected (bits 28..29) and the
/// sequence kind (bits 26..27).  Returns 0 when a complete code point was
/// written to `out`, 1 when more bytes are needed, 3 on invalid input.
pub fn w34a04_utf8_step(byte: u8, out: &mut u32, state: &mut u32) -> u32 {
    let mut s = *state;
    let b = byte as u32;
    let kind = (s >> 0x1a) & 3;
    // 0 = fall to continuation handling, 1 = reset state then return 3
    let mut reset = false;
    if kind == 0 {
        if byte >= 0x80 {
            if byte & 0xe0 == 0xc0 {
                if byte < 0xc2 {
                    return 3;
                }
                s = (s & 0xdf00_0000) | (b & 0x1f) | 0x1000_0000;
            } else if byte & 0xf0 == 0xe0 {
                s = (s & 0xef00_0000) | (b & 0xf) | 0x2000_0000;
            } else {
                if byte & 0xf8 != 0xf0 || byte > 0xf4 {
                    return 3;
                }
                s = (s & 0xff00_0000) | (b & 7) | 0x3000_0000;
            }
        } else {
            s = (s & 0xcf00_0000) | b;
        }
        *state = s;
        if s & 0x3000_0000 != 0 {
            *state = (s & 0xf7ff_ffff) | 0x0400_0000;
            return 1;
        }
        *out = s & 0x00ff_ffff;
        return 0;
    }
    let mut cont = true;
    if kind == 1 {
        let n = (s >> 0x1c) & 3;
        let low = s & 0x00ff_ffff;
        if n != 1 {
            if n == 2 {
                if low == 0 {
                    if byte < 0xa0 {
                        reset = true;
                    }
                } else if low == 0xd && byte > 0x9f {
                    *state = s & 0xc3ff_ffff;
                    return 3;
                }
            } else if n == 3 {
                if low == 0 {
                    if byte < 0x90 {
                        reset = true;
                    }
                } else if low == 4 && byte > 0x8f {
                    *state = s & 0xc3ff_ffff;
                    return 3;
                }
            }
        } else if low <= 1 {
            reset = true;
        }
        if reset {
            *state = s & 0xc3ff_ffff;
            cont = false;
        }
    }
    if cont {
        if s & 0x3000_0000 != 0 {
            if byte & 0xc0 != 0x80 {
                *state = s & 0xc3ff_ffff;
                return 3;
            }
            let mut u = ((s & 0x3_ffff) << 6) | (b & 0x3f) | (s & 0xff00_0000);
            u = ((u.wrapping_add(0x0400_0000) ^ u) & 0x0c00_0000) ^ u;
            s = ((((s >> 0x1c).wrapping_sub(1)).wrapping_mul(0x1000_0000) ^ u) & 0x3000_0000) ^ u;
            *state = s;
            if s & 0x3000_0000 != 0 {
                return 1;
            }
        }
        *state = s & 0xf3ff_ffff;
        let cp = s & 0x00ff_ffff;
        if cp > 0x10ffff || ((s & 0xfffe) != 0xfffe && cp.wrapping_sub(0xfdd0) > 0x1f) {
            *out = cp;
            return 0;
        }
    }
    3
}

/// `0x00c6bd70`: runs the scanner `scan(start, arg, 0)` (callee
/// `FUN_00cbcc80`, which returns the end position) from `*cursor`, and
/// optionally stores the consumed length `end - start` into `consumed`.
pub fn w34a04_scan_consumed(
    cursor: i32,
    consumed: Option<&mut i32>,
    arg: u32,
    scan: impl FnOnce(i32, u32, u32) -> i32,
) {
    let end = scan(cursor, arg, 0);
    if let Some(c) = consumed {
        *c = end.wrapping_sub(cursor);
    }
}

#[cfg(test)]
mod tests_wave34_agent04 {
    use super::*;

    fn decode(bytes: &[u8]) -> Vec<u32> {
        let (mut st, mut out, mut r) = (0u32, 0u32, vec![]);
        for &b in bytes {
            if w34a04_utf8_step(b, &mut out, &mut st) == 0 {
                r.push(out);
            }
        }
        r
    }

    #[test]
    fn utf8() {
        assert_eq!(
            decode("aé€😀".as_bytes()),
            vec![0x61, 0xe9, 0x20ac, 0x1f600]
        );
        let (mut st, mut out) = (0u32, 0u32);
        assert_eq!(w34a04_utf8_step(0xc1, &mut out, &mut st), 3);
        assert_eq!(w34a04_utf8_step(0xf5, &mut out, &mut st), 3);
        assert_eq!(w34a04_utf8_step(0xe2, &mut out, &mut st), 1);
        assert_eq!(w34a04_utf8_step(0x41, &mut out, &mut st), 3);
        // surrogate rejected
        let (mut st, mut out) = (0u32, 0u32);
        w34a04_utf8_step(0xed, &mut out, &mut st);
        assert_eq!(w34a04_utf8_step(0xa0, &mut out, &mut st), 3);
    }

    #[test]
    fn consumed() {
        let mut c = 0;
        w34a04_scan_consumed(100, Some(&mut c), 7, |s, a, z| {
            assert_eq!((a, z), (7, 0));
            s + 5
        });
        assert_eq!(c, 5);
        w34a04_scan_consumed(1, None, 0, |s, _, _| s);
    }
}
