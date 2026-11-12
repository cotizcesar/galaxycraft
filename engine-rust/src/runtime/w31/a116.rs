#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent116.rs
// ===========================================================================
// Wave 31 translation batch 116: decimal output helper, 64-bit scale, offset out-param.

/// Output cursor used by the decimal writer: `buf` is the capacity-bounded
/// buffer, `len` counts every character produced (even past capacity).
pub struct W31a116Out<'a> {
    pub buf: &'a mut [u8],
    pub len: usize,
}

fn w31a116_put(out: &mut W31a116Out, c: u8) {
    if out.len < out.buf.len() {
        out.buf[out.len] = c;
    }
    out.len += 1;
}

/// `0x00c68d70`: appends the decimal digits of `value`, left-padded with
/// `'0'` to `width`.  Characters past the buffer capacity are dropped but
/// still counted in `len`.
pub fn w31a116_write_decimal_padded(out: &mut W31a116Out, value: i32, width: i32) {
    let mut digits: Vec<u8> = Vec::new();
    let mut v = value;
    loop {
        let q = v / 10;
        digits.push((v.wrapping_sub(q.wrapping_mul(10)) as u8).wrapping_add(b'0'));
        v = q;
        if v == 0 {
            break;
        }
    }
    digits.reverse();
    let n = digits.len() as i32;
    if n < width {
        for _ in 0..(width - n) {
            w31a116_put(out, b'0');
        }
    }
    for d in digits {
        if d == 0 {
            break;
        }
        w31a116_put(out, d);
    }
}

/// `0x00c6b350`: multiplies the 64-bit value `(hi << 32) | lo` by 100 and
/// adds the constant `0x5e6684f4_b3960000` (wrapping).  Returns (lo, hi).
pub fn w31a116_scale100_offset(lo: u32, hi: i32) -> (u32, u32) {
    let v = ((hi as u32 as u64) << 32) | lo as u64;
    let r = v.wrapping_mul(100).wrapping_add(0x5e66_84f4_b396_0000u64);
    (r as u32, (r >> 32) as u32)
}

/// `0x00c6bd40`: runs `parse` on the cursor `*cursor`; the parser returns
/// the new cursor, and the consumed byte count is stored into `consumed`
/// when supplied.
pub fn w31a116_parse_consumed(
    cursor: &mut i32,
    consumed: Option<&mut i32>,
    parse: impl FnOnce(i32) -> i32,
) {
    let start = *cursor;
    *cursor = parse(start);
    if let Some(c) = consumed {
        *c = cursor.wrapping_sub(start);
    }
}

// Skipped (FUN_00c68910 FUN_00c689d0 FUN_00c68b30 FUN_00c68c10 FUN_00c694f0
// FUN_00c6a6e0 FUN_00c6aab0 FUN_00c6ad20 FUN_00c6b0e0 FUN_00c6b1c0
// FUN_00c6bea0 FUN_00c6bf40 FUN_00c6bfe0 FUN_00c6c090): glue/non-returning.

#[cfg(test)]
mod tests_wave31_agent116 {
    use super::*;

    #[test]
    fn decimal_padded() {
        let mut b = [0u8; 8];
        let mut o = W31a116Out {
            buf: &mut b,
            len: 0,
        };
        w31a116_write_decimal_padded(&mut o, 42, 5);
        assert_eq!(o.len, 5);
        assert_eq!(&b[..5], b"00042");
        let mut b2 = [0u8; 2];
        let mut o2 = W31a116Out {
            buf: &mut b2,
            len: 0,
        };
        w31a116_write_decimal_padded(&mut o2, 12345, 0);
        assert_eq!(o2.len, 5);
        assert_eq!(&b2, b"12");
    }

    #[test]
    fn scale() {
        let (lo, hi) = w31a116_scale100_offset(0, 0);
        assert_eq!((lo, hi), (0xb396_0000, 0x5e66_84f4));
        let (lo, hi) = w31a116_scale100_offset(0x4c6a_0000, 0);
        assert_eq!(
            lo,
            0x4c6a_0000u32.wrapping_mul(100).wrapping_add(0xb396_0000)
        );
        assert_eq!(
            hi,
            ((0x4c6a_0000u64 * 100 + 0x5e66_84f4_b396_0000u64) >> 32) as u32
        );
    }

    #[test]
    fn consumed() {
        let mut c = 10;
        let mut n = 0;
        w31a116_parse_consumed(&mut c, Some(&mut n), |x| x + 7);
        assert_eq!((c, n), (17, 7));
        w31a116_parse_consumed(&mut c, None, |x| x + 1);
        assert_eq!(c, 18);
    }
}
