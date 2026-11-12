#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave20/agent05.rs
// ===========================================================================
// Wave 20 translation batch 05: fmt v6 integer/spec writers and one SAD kernel.
//
// All helpers are pure; the `fmt` output buffer is modelled as a `Vec<u8>`.

/// Pair table the retail code indexes with `value % 100 * 2`.
#[allow(dead_code)]
const W20A05_DIGIT_PAIRS: &[u8; 200] = b"0001020304050607080910111213141516171819\
2021222324252627282930313233343536373839\
4041424344454647484950515253545556575859\
6061626364656667686970717273747576777879\
8081828384858687888990919293949596979899";

/// Fields the `fmt::v6::basic_format_specs` pointer exposes to the writers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub struct W20a05Specs {
    /// Field width (`+0x0`).
    pub width: usize,
    /// Fill byte (`+0x11`).
    pub fill: u8,
    /// Alignment: sign-extended low nibble of `+0xc` (2 right, 3 centre,
    /// anything else pads on the right).
    pub align: i8,
}

/// Format spec flags set by `w20a05_chrono_spec`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub struct W20a05Flags {
    /// Byte 0 is the caller's expected upper-case letter (`G`, `A`, `E`, `F`).
    pub expect: u8,
    pub b1: bool,
    pub b2: bool,
    pub b3: bool,
    pub b4: bool,
}

/// `0x00556d40`: marks the format-spec flag bytes for a presentation type.
/// `0`/`G`/`g` set b1 when `expect == 'G'`; `%` sets b2 and b3; `A`/`a`
/// and `E`/`e` set b1 when expect matches the upper-case letter; `F`/`f`
/// always set b2 and set b1 when expect is `F`; `n` sets b4.  Any other
/// type yields the "invalid type specifier" format error.
#[allow(dead_code)]
pub fn w20a05_chrono_spec(ty: u8, f: &mut W20a05Flags) -> Result<(), &'static str> {
    match ty {
        0 | b'G' | b'g' => {
            if f.expect == b'G' {
                f.b1 = true;
            }
        }
        b'%' => {
            f.b2 = true;
            f.b3 = true;
        }
        b'A' | b'a' => {
            if f.expect == b'A' {
                f.b1 = true;
            }
        }
        b'E' | b'e' => {
            if f.expect == b'E' {
                f.b1 = true;
            }
        }
        b'F' | b'f' => {
            f.b2 = true;
            if f.expect == b'F' {
                f.b1 = true;
            }
        }
        b'n' => f.b4 = true,
        _ => return Err("invalid type specifier"),
    }
    Ok(())
}

/// `0x00556510`: writes `value` in decimal two digits at a time (division
/// by 100, `DIGIT_PAIRS` lookup, a single leading digit when < 10).
#[allow(dead_code)]
pub fn w20a05_format_decimal(value: u64) -> Vec<u8> {
    let mut rev: Vec<u8> = Vec::new();
    let mut v = value;
    while v >= 100 {
        let r = (v % 100) as usize * 2;
        v /= 100;
        rev.push(W20A05_DIGIT_PAIRS[r + 1]);
        rev.push(W20A05_DIGIT_PAIRS[r]);
    }
    if v < 10 {
        rev.push(v as u8 + b'0');
    } else {
        let r = v as usize * 2;
        rev.push(W20A05_DIGIT_PAIRS[r + 1]);
        rev.push(W20A05_DIGIT_PAIRS[r]);
    }
    rev.reverse();
    rev
}

/// `0x00556380`: like the decimal writer but with digit grouping.  The
/// shared counter `*counter` is incremented after every digit written
/// except the most significant; whenever it reaches a multiple of 3 and
/// digits remain, `sep` is inserted before the next more-significant digit.
#[allow(dead_code)]
pub fn w20a05_format_decimal_grouped(value: u64, sep: u8, counter: &mut u32) -> Vec<u8> {
    let plain = w20a05_format_decimal(value);
    let n = plain.len();
    let mut rev: Vec<u8> = Vec::new();
    for (i, &d) in plain.iter().rev().enumerate() {
        rev.push(d);
        if i + 1 < n {
            *counter += 1;
            if (*counter).is_multiple_of(3) {
                rev.push(sep);
            }
        }
    }
    rev.reverse();
    rev
}

/// Shared padding step of the int writers (`write_padded`): right (2)
/// fills left; centre (3) puts `n/2` fill before and the remainder after;
/// any other alignment writes the body first.  A width not above the body
/// length writes the body alone.
#[allow(dead_code)]
fn w20a05_write_padded(out: &mut Vec<u8>, specs: &W20a05Specs, body: &[u8]) {
    if specs.width <= body.len() {
        out.extend_from_slice(body);
        return;
    }
    let n = specs.width - body.len();
    match specs.align {
        2 => {
            out.extend(std::iter::repeat_n(specs.fill, n));
            out.extend_from_slice(body);
        }
        3 => {
            out.extend(std::iter::repeat_n(specs.fill, n >> 1));
            out.extend_from_slice(body);
            out.extend(std::iter::repeat_n(specs.fill, n - (n >> 1)));
        }
        _ => {
            out.extend_from_slice(body);
            out.extend(std::iter::repeat_n(specs.fill, n));
        }
    }
}

/// Writes `value` into exactly `ndigits` bytes, least significant first
/// from the end, `shift` bits per digit, using `table`.
#[allow(dead_code)]
fn w20a05_radix_digits(mut value: u64, ndigits: usize, shift: u32, table: &[u8]) -> Vec<u8> {
    let mut buf = vec![b'0'; ndigits];
    let mask = (1u64 << shift) - 1;
    let mut i = ndigits;
    loop {
        if i == 0 {
            break;
        }
        i -= 1;
        buf[i] = table[(value & mask) as usize];
        value >>= shift;
        if value == 0 {
            break;
        }
    }
    buf
}

/// `0x00559c60`: writes a pointer: `0x` followed by `ndigits` lower-case
/// hex digits (total `ndigits + 2`), padded according to `specs`.
#[allow(dead_code)]
pub fn w20a05_write_pointer(out: &mut Vec<u8>, specs: &W20a05Specs, value: u32, ndigits: usize) {
    let mut body = b"0x".to_vec();
    body.extend(w20a05_radix_digits(
        value as u64,
        ndigits,
        4,
        b"0123456789abcdef",
    ));
    w20a05_write_padded(out, specs, &body);
}

/// Prefix, precision zeros and digit record behind the radix writers
/// (`param_2[1..]`): prefix bytes, zero count with its fill byte, value
/// and digit count.  `size` is the total body length (`param_2[0]`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub struct W20a05IntBody<'a> {
    pub size: usize,
    pub prefix: &'a [u8],
    pub zero_fill: u8,
    pub zeros: usize,
    pub value: u64,
    pub ndigits: usize,
}

#[allow(dead_code)]
fn w20a05_write_radix(out: &mut Vec<u8>, specs: &W20a05Specs, b: &W20a05IntBody<'_>, shift: u32) {
    let mut body = b.prefix.to_vec();
    body.extend(std::iter::repeat_n(b.zero_fill, b.zeros));
    body.extend(w20a05_radix_digits(b.value, b.ndigits, shift, b"01234567"));
    // `size` governs the padding decision in the retail code.
    let _ = b.size;
    w20a05_write_padded(out, specs, &body);
}

/// `0x00559db0`: binary (`0b`) integer body, 32-bit value, 1 bit/digit.
#[allow(dead_code)]
pub fn w20a05_write_bin32(out: &mut Vec<u8>, specs: &W20a05Specs, b: &W20a05IntBody<'_>) {
    let mut b = b.clone();
    b.value &= 0xffff_ffff;
    w20a05_write_radix(out, specs, &b, 1);
}

/// `0x00559f20`: binary integer body with a 64-bit value (low/high words
/// shifted right together), 1 bit/digit.
#[allow(dead_code)]
pub fn w20a05_write_bin64(out: &mut Vec<u8>, specs: &W20a05Specs, b: &W20a05IntBody<'_>) {
    w20a05_write_radix(out, specs, b, 1);
}

/// `0x0055a0a0`: octal integer body, 32-bit value, 3 bits/digit.
#[allow(dead_code)]
pub fn w20a05_write_oct32(out: &mut Vec<u8>, specs: &W20a05Specs, b: &W20a05IntBody<'_>) {
    let mut b = b.clone();
    b.value &= 0xffff_ffff;
    w20a05_write_radix(out, specs, &b, 3);
}

/// `0x004d7778`: 4x4 block SAD over eight horizontal offsets.  For each of
/// four rows (strides `stride_a` on `src`, `stride_b` on `refp`; rows 0/1
/// from `refp` at +0/+stride_b then +2*stride_b/+3*stride_b) it runs
/// `mpsadbw` imm 0: `out[j] += sum_k |ref[j+k] - src[k]|`, j in 0..8,
/// k in 0..4.  Result lanes are the eight sums zero-extended to u32.
#[allow(dead_code)]
pub fn w20a05_sad4x4_x8(src: &[u8], stride_a: usize, refp: &[u8], stride_b: usize) -> [u32; 8] {
    let mut out = [0u32; 8];
    for row in 0..4 {
        let s = &src[row * stride_a..row * stride_a + 4];
        let r = &refp[row * stride_b..row * stride_b + 16];
        for (j, o) in out.iter_mut().enumerate() {
            for k in 0..4 {
                *o += (r[j + k] as i32 - s[k] as i32).unsigned_abs();
            }
        }
    }
    out
}

#[cfg(test)]
mod tests_wave20_agent05 {
    use super::*;

    #[test]
    fn chrono_spec_flags() {
        let mut f = W20a05Flags {
            expect: b'F',
            ..Default::default()
        };
        w20a05_chrono_spec(b'f', &mut f).unwrap();
        assert!(f.b1 && f.b2);
        let mut g = W20a05Flags::default();
        w20a05_chrono_spec(b'%', &mut g).unwrap();
        assert!(g.b2 && g.b3 && !g.b1);
        w20a05_chrono_spec(b'n', &mut g).unwrap();
        assert!(g.b4);
        assert!(w20a05_chrono_spec(b'z', &mut g).is_err());
    }

    #[test]
    fn decimal() {
        for v in [0u64, 7, 10, 99, 100, 12345, u64::MAX] {
            assert_eq!(w20a05_format_decimal(v), v.to_string().into_bytes());
        }
    }

    #[test]
    fn grouped() {
        let mut c = 0;
        assert_eq!(
            w20a05_format_decimal_grouped(1234567, b',', &mut c),
            b"1,234,567"
        );
        assert_eq!(c, 6);
        let mut c = 0;
        assert_eq!(
            w20a05_format_decimal_grouped(123456, b',', &mut c),
            b"123,456"
        );
        let mut c = 0;
        assert_eq!(w20a05_format_decimal_grouped(12, b',', &mut c), b"12");
    }

    #[test]
    fn pointer_and_padding() {
        let mut out = Vec::new();
        let s = W20a05Specs {
            width: 0,
            fill: b' ',
            align: 0,
        };
        w20a05_write_pointer(&mut out, &s, 0xbeef, 4);
        assert_eq!(out, b"0xbeef");
        let mut out = Vec::new();
        let s = W20a05Specs {
            width: 10,
            fill: b'*',
            align: 2,
        };
        w20a05_write_pointer(&mut out, &s, 0xf, 1);
        assert_eq!(out, b"*******0xf");
        let mut out = Vec::new();
        let s = W20a05Specs {
            width: 9,
            fill: b'.',
            align: 3,
        };
        w20a05_write_pointer(&mut out, &s, 0xf, 1);
        assert_eq!(out, b"...0xf...");
    }

    #[test]
    fn radix_bodies() {
        let b = W20a05IntBody {
            size: 6,
            prefix: b"0b",
            zero_fill: b'0',
            zeros: 2,
            value: 5,
            ndigits: 3,
        };
        let mut out = Vec::new();
        let s = W20a05Specs {
            width: 0,
            fill: b' ',
            align: 2,
        };
        w20a05_write_bin32(&mut out, &s, &b);
        assert_eq!(out, b"0b00101");
        let b = W20a05IntBody {
            size: 4,
            prefix: b"0",
            zero_fill: b'0',
            zeros: 0,
            value: 0o755,
            ndigits: 3,
        };
        let mut out = Vec::new();
        w20a05_write_oct32(&mut out, &s, &b);
        assert_eq!(out, b"0755");
        let b = W20a05IntBody {
            size: 0,
            prefix: b"",
            zero_fill: b'0',
            zeros: 0,
            value: 1 << 40,
            ndigits: 41,
        };
        let mut out = Vec::new();
        w20a05_write_bin64(&mut out, &s, &b);
        assert_eq!(out.len(), 41);
        assert_eq!(out[0], b'1');
        let b = W20a05IntBody {
            size: 3,
            prefix: b"",
            zero_fill: b'0',
            zeros: 0,
            value: 5,
            ndigits: 3,
        };
        let s = W20a05Specs {
            width: 6,
            fill: b'_',
            align: 1,
        };
        let mut out = Vec::new();
        w20a05_write_bin32(&mut out, &s, &b);
        assert_eq!(out, b"101___");
    }

    #[test]
    fn sad() {
        let src = [1u8; 16];
        let mut r = [0u8; 64];
        for (i, v) in r.iter_mut().enumerate() {
            *v = (i % 16) as u8;
        }
        let out = w20a05_sad4x4_x8(&src, 4, &r, 16);
        // row sums for offset 0: |0..3 - 1| = 1+0+1+2 = 4 per row
        assert_eq!(out[0], 16);
        assert_eq!(out[1], 4 * 6);
    }
}
