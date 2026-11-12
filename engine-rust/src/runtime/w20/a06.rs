#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave20/agent06.rs
// ===========================================================================
// Wave 20 translation batch 06: fmt v6 padded-writer helpers.
//
// The buffer is modelled as `Vec<u8>` (retail: `{vtable, ptr, size, cap}`,
// grow slot called when `cap < new_size`).  The format spec is modelled as
// [`W20a06Spec`] (`width` at +0, fill byte at +0x11, signed low nibble of
// the dword at +0xC is the alignment: 2 right, 3 center, otherwise left).

/// Format spec fields read by every padded writer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W20a06Spec {
    /// Field width (dword at +0).
    pub width: u32,
    /// Fill byte (byte at +0x11).
    pub fill: u8,
    /// Raw dword at +0xC; only the low nibble (sign-extended) matters.
    pub align_word: u32,
}

/// Sign-extended low nibble of the alignment dword, as `(x << 28) >> 28`.
pub fn w20a06_align(spec: &W20a06Spec) -> i32 {
    ((spec.align_word << 28) as i32) >> 28
}

/// Shared padding skeleton of the writers below: when `width <= size` the
/// body is emitted as is, otherwise `width - size` fill bytes go before
/// (align 2), split around (align 3, left half is `pad >> 1`) or after
/// (any other align) the body.
pub fn w20a06_write_padded(
    out: &mut Vec<u8>,
    spec: &W20a06Spec,
    size: u32,
    body: impl FnOnce(&mut Vec<u8>),
) {
    if spec.width <= size {
        body(out);
        return;
    }
    let pad = (spec.width - size) as usize;
    match w20a06_align(spec) {
        2 => {
            out.resize(out.len() + pad, spec.fill);
            body(out);
        }
        3 => {
            let left = pad >> 1;
            out.resize(out.len() + left, spec.fill);
            body(out);
            out.resize(out.len() + (pad - left), spec.fill);
        }
        _ => {
            body(out);
            out.resize(out.len() + pad, spec.fill);
        }
    }
}

/// Integer writer record shared by the numeric writers: `size` (+0), prefix
/// bytes (+4/+8), zero padding `(fill, count)` (+0xC/+0x10).
#[derive(Clone, Debug, Default)]
pub struct W20a06IntParts<'a> {
    /// Total emitted size (+0).
    pub size: u32,
    /// Prefix bytes (sign / base prefix).
    pub prefix: &'a [u8],
    /// Zero-padding fill byte.
    pub zero_fill: u8,
    /// Zero-padding count.
    pub zero_count: u32,
}

fn w20a06_head(out: &mut Vec<u8>, p: &W20a06IntParts) {
    out.extend_from_slice(p.prefix);
    out.resize(out.len() + p.zero_count as usize, p.zero_fill);
}

/// Writes `ndigits` slots filled backwards from the end with `radix`
/// digits of `value` (do/while: at least one digit).
fn w20a06_digits_back(out: &mut Vec<u8>, mut value: u64, ndigits: u32, shift: u32, table: &[u8]) {
    let start = out.len();
    out.resize(start + ndigits as usize, 0);
    let mask = (1u64 << shift) - 1;
    let mut pos = ndigits as usize;
    loop {
        if pos == 0 {
            break;
        }
        pos -= 1;
        out[start + pos] = table[(value & mask) as usize];
        value >>= shift;
        if value == 0 {
            break;
        }
    }
}

/// `0x0055a220`: padded octal integer.  Value is a 64-bit pair (+0x18/+0x1C),
/// digit count at +0x20; digits are `'0' + (v & 7)`, written backwards.
pub fn w20a06_0055a220(
    out: &mut Vec<u8>,
    spec: &W20a06Spec,
    p: &W20a06IntParts,
    value: u64,
    ndigits: u32,
) {
    w20a06_write_padded(out, spec, p.size, |o| {
        w20a06_head(o, p);
        w20a06_digits_back(o, value, ndigits, 3, b"01234567");
    });
}

/// `0x0055a3a0`: padded integer whose digits come from the inner writer
/// at FUN_005562d0 (supplied as `inner`).
pub fn w20a06_0055a3a0(
    out: &mut Vec<u8>,
    spec: &W20a06Spec,
    p: &W20a06IntParts,
    inner: impl FnOnce(&mut Vec<u8>),
) {
    w20a06_write_padded(out, spec, p.size, |o| {
        w20a06_head(o, p);
        inner(o);
    });
}

/// `0x0055a510`: same skeleton as `0x0055a3a0`, digits from the inner
/// writer at FUN_00556510.
pub fn w20a06_0055a510(
    out: &mut Vec<u8>,
    spec: &W20a06Spec,
    p: &W20a06IntParts,
    inner: impl FnOnce(&mut Vec<u8>),
) {
    w20a06_write_padded(out, spec, p.size, |o| {
        w20a06_head(o, p);
        inner(o);
    });
}

/// `0x0055a680`: padded hexadecimal integer; lowercase table when the type
/// character is `'x'`, uppercase otherwise.  `ndigits` is at +0x18.
pub fn w20a06_0055a680(
    out: &mut Vec<u8>,
    spec: &W20a06Spec,
    p: &W20a06IntParts,
    value: u32,
    lowercase: bool,
    ndigits: u32,
) {
    let table: &[u8] = if lowercase {
        b"0123456789abcdef"
    } else {
        b"0123456789ABCDEF"
    };
    w20a06_write_padded(out, spec, p.size, |o| {
        w20a06_head(o, p);
        w20a06_digits_back(o, value as u64, ndigits, 4, table);
    });
}

/// `0x0055a820`: padded integer body written by FUN_00555ec0 after the
/// prefix and zero padding.
pub fn w20a06_0055a820(
    out: &mut Vec<u8>,
    spec: &W20a06Spec,
    p: &W20a06IntParts,
    inner: impl FnOnce(&mut Vec<u8>),
) {
    w20a06_write_padded(out, spec, p.size, |o| {
        w20a06_head(o, p);
        inner(o);
    });
}

/// `0x0055a990`: padded float body; after prefix and zero padding the
/// inner writer FUN_00556160 is invoked (supplied as `inner`).
pub fn w20a06_0055a990(
    out: &mut Vec<u8>,
    spec: &W20a06Spec,
    p: &W20a06IntParts,
    inner: impl FnOnce(&mut Vec<u8>),
) {
    w20a06_write_padded(out, spec, p.size, |o| {
        w20a06_head(o, p);
        inner(o);
    });
}

/// `0x0055ab30`: as `0x0055a990` with the FUN_00556380 float writer.
pub fn w20a06_0055ab30(
    out: &mut Vec<u8>,
    spec: &W20a06Spec,
    p: &W20a06IntParts,
    inner: impl FnOnce(&mut Vec<u8>),
) {
    w20a06_write_padded(out, spec, p.size, |o| {
        w20a06_head(o, p);
        inner(o);
    });
}

/// `0x0055acd0`: padded string; size is the string length, the body is the
/// string bytes.
pub fn w20a06_0055acd0(out: &mut Vec<u8>, spec: &W20a06Spec, s: &[u8]) {
    w20a06_write_padded(out, spec, s.len() as u32, |o| o.extend_from_slice(s));
}

/// `0x0055ae20`: padded `[sign] + bytes` field (size is `sign != 0` plus the
/// byte count).
pub fn w20a06_0055ae20(out: &mut Vec<u8>, spec: &W20a06Spec, sign: u8, body: &[u8]) {
    let size = (sign != 0) as u32 + body.len() as u32;
    w20a06_write_padded(out, spec, size, |o| {
        if sign != 0 {
            o.push(sign);
        }
        o.extend_from_slice(body);
    });
}

/// `0x0055afa0`: padded `[sign] + 3 bytes` (inf / nan text); size is
/// `(byte1 != 0) + (sign != 0) + 3` as decoded.
pub fn w20a06_0055afa0(out: &mut Vec<u8>, spec: &W20a06Spec, sign: u8, second: u8, text: &[u8; 3]) {
    let size = (second != 0) as u32 + (sign != 0) as u32 + 3;
    w20a06_write_padded(out, spec, size, |o| {
        if sign != 0 {
            o.push(sign);
        }
        o.extend_from_slice(text);
    });
}

/// `0x0055b120`: padded `[sign] + inner` field with size `len + (sign != 0)`;
/// when the width does not exceed the size no fill is written.
pub fn w20a06_0055b120(
    out: &mut Vec<u8>,
    spec: &W20a06Spec,
    sign: u8,
    len: u32,
    inner: impl FnOnce(&mut Vec<u8>),
) {
    let size = (sign != 0) as u32 + len;
    w20a06_write_padded(out, spec, size, |o| {
        if sign != 0 {
            o.push(sign);
        }
        inner(o);
    });
}

/// `0x0055b5d0`: copies literal format-string text `[begin, end)`.  With no
/// `'}'` the whole range is appended; a `"}}"` pair appends text up to and
/// including the first brace; a lone brace is the error
/// `unmatched '}' in format string`.  Returns the offset after the text
/// consumed (`end` or past the escaped pair).
pub fn w20a06_0055b5d0(out: &mut Vec<u8>, text: &[u8]) -> Result<usize, &'static str> {
    if text.is_empty() {
        return Ok(0);
    }
    match text.iter().position(|&b| b == b'}') {
        None => {
            out.extend_from_slice(text);
            Ok(text.len())
        }
        Some(i) => {
            let p = i + 1;
            if p != text.len() && text[p] == b'}' {
                out.extend_from_slice(&text[..p]);
                Ok(p + 1)
            } else {
                Err("unmatched '}' in format string")
            }
        }
    }
}

/// `0x0055bad0`: periodic trigger; true (the original calls the flush at
/// FUN_0055bb70) when the enabled flag is set and `count % interval == 0`.
/// A zero interval is a hardware divide fault in retail; here it is false.
pub fn w20a06_0055bad0(enabled: bool, interval: u32, count: u32) -> bool {
    enabled && interval != 0 && count % interval == 0
}

#[cfg(test)]
mod tests_wave20_agent06 {
    use super::*;

    fn spec(width: u32, align: u32) -> W20a06Spec {
        W20a06Spec {
            width,
            fill: b'*',
            align_word: align,
        }
    }
    fn parts(size: u32) -> W20a06IntParts<'static> {
        W20a06IntParts {
            size,
            prefix: b"0",
            zero_fill: b'0',
            zero_count: 1,
        }
    }

    #[test]
    fn padding_alignments() {
        let mut o = vec![];
        w20a06_0055acd0(&mut o, &spec(7, 2), b"ab");
        assert_eq!(o, b"*****ab");
        o.clear();
        w20a06_0055acd0(&mut o, &spec(7, 3), b"ab");
        assert_eq!(o, b"**ab***");
        o.clear();
        w20a06_0055acd0(&mut o, &spec(7, 1), b"ab");
        assert_eq!(o, b"ab*****");
        o.clear();
        w20a06_0055acd0(&mut o, &spec(2, 2), b"ab");
        assert_eq!(o, b"ab");
        assert_eq!(w20a06_align(&spec(0, 0xfffffff2)), 2);
        assert_eq!(w20a06_align(&spec(0, 0x1a)), -6);
    }

    #[test]
    fn octal_and_hex() {
        let mut o = vec![];
        w20a06_0055a220(&mut o, &spec(8, 2), &parts(5), 0o755, 3);
        assert_eq!(o, b"***00755");
        o.clear();
        w20a06_0055a220(&mut o, &spec(0, 0), &parts(3), 0, 1);
        assert_eq!(o, b"000");
        o.clear();
        w20a06_0055a680(&mut o, &spec(0, 0), &parts(4), 0xbeef, true, 4);
        assert_eq!(o, b"00beef");
        o.clear();
        w20a06_0055a680(&mut o, &spec(9, 1), &parts(6), 0xBEEF, false, 4);
        assert_eq!(o, b"00BEEF***");
    }

    #[test]
    fn inner_writers() {
        for f in [
            w20a06_0055a3a0 as fn(&mut Vec<u8>, &W20a06Spec, &W20a06IntParts, _),
            w20a06_0055a510,
            w20a06_0055a820,
            w20a06_0055a990,
            w20a06_0055ab30,
        ] {
            let mut o = vec![];
            f(&mut o, &spec(6, 3), &parts(4), |b: &mut Vec<u8>| {
                b.extend_from_slice(b"12")
            });
            assert_eq!(o, b"*0012*");
        }
    }

    #[test]
    fn sign_and_text_fields() {
        let mut o = vec![];
        w20a06_0055ae20(&mut o, &spec(6, 2), b'-', b"12");
        assert_eq!(o, b"***-12");
        o.clear();
        w20a06_0055afa0(&mut o, &spec(4, 1), 0, 0, b"inf");
        assert_eq!(o, b"inf*");
        o.clear();
        w20a06_0055b120(&mut o, &spec(5, 2), b'+', 2, |b| b.extend_from_slice(b"99"));
        assert_eq!(o, b"**+99");
        o.clear();
        w20a06_0055b120(&mut o, &spec(2, 2), 0, 2, |b| b.extend_from_slice(b"99"));
        assert_eq!(o, b"99");
    }

    #[test]
    fn literal_text_and_trigger() {
        let mut o = vec![];
        assert_eq!(w20a06_0055b5d0(&mut o, b"abc"), Ok(3));
        assert_eq!(o, b"abc");
        o.clear();
        assert_eq!(w20a06_0055b5d0(&mut o, b"a}}b"), Ok(3));
        assert_eq!(o, b"a}");
        assert!(w20a06_0055b5d0(&mut o, b"a}b").is_err());
        assert!(w20a06_0055b5d0(&mut o, b"a}").is_err());
        assert_eq!(w20a06_0055b5d0(&mut o, b""), Ok(0));
        assert!(w20a06_0055bad0(true, 4, 8));
        assert!(!w20a06_0055bad0(true, 4, 9));
        assert!(!w20a06_0055bad0(false, 4, 8));
        assert!(!w20a06_0055bad0(true, 0, 8));
    }
}
