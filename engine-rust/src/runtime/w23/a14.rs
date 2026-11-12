#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent14.rs
// ===========================================================================
// Wave 23 agent 14: JSON value getters, JSON writer number output,
// dtoa prettify and small tileset/table helpers.

/// JSON value flag bits at record offset `0xe` (u16) shared by the getters.
pub const W23A14_FLAG_DOUBLE: u16 = 0x200;
pub const W23A14_FLAG_BOOL: u16 = 0x008;

/// Error text raised by the retail getters for a mismatching JSON type.
pub const W23A14_BAD_TYPE: &str = "Unexpected JSON type.";

/// Shared tail of the 32-bit getters: flags 0x100/0x80/0x40/0x20 return the
/// raw word, 0x200 converts the double through `ftol`, 0x08 is a bool where
/// `flags == 10` means true, anything else is a type error.
fn w23a14_get32(low: u32, flags: u16, ftol: impl FnOnce() -> u32) -> Result<u32, &'static str> {
    if flags & 0x100 != 0 || flags & 0x80 != 0 || flags & 0x40 != 0 || flags & 0x20 != 0 {
        return Ok(low);
    }
    if flags & W23A14_FLAG_DOUBLE != 0 {
        return Ok(ftol());
    }
    if flags & W23A14_FLAG_BOOL != 0 {
        return Ok((flags == 10) as u32);
    }
    Err(W23A14_BAD_TYPE)
}

/// `0x00705930`: JSON member as u32 (member lookup already resolved by the
/// caller; `ftol` stands for the double-to-int conversion).
pub fn w23a14_member_as_u32(
    low: u32,
    flags: u16,
    ftol: impl FnOnce() -> u32,
) -> Result<u32, &'static str> {
    w23a14_get32(low, flags, ftol)
}

/// `0x00705bf0`: JSON value as u32 (same rules as the member getter).
pub fn w23a14_value_as_u32(
    low: u32,
    flags: u16,
    ftol: impl FnOnce() -> u32,
) -> Result<u32, &'static str> {
    w23a14_get32(low, flags, ftol)
}

/// `0x00705b40`: JSON value as u64.  0x100/0x80 return all 64 bits, 0x40
/// zero-extends the low word, 0x20 sign-extends it.
pub fn w23a14_value_as_u64(
    raw: u64,
    flags: u16,
    ftol64: impl FnOnce() -> u64,
) -> Result<u64, &'static str> {
    if flags & 0x100 != 0 || flags & 0x80 != 0 {
        return Ok(raw);
    }
    if flags & 0x40 != 0 {
        return Ok(raw as u32 as u64);
    }
    if flags & 0x20 != 0 {
        return Ok(raw as u32 as i32 as i64 as u64);
    }
    if flags & W23A14_FLAG_DOUBLE != 0 {
        return Ok(ftol64());
    }
    if flags & W23A14_FLAG_BOOL != 0 {
        return Ok((flags == 10) as u64);
    }
    Err(W23A14_BAD_TYPE)
}

/// One writer nesting level: value count (at `-8`) and array flag (at `-4`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W23a14Level {
    pub count: u32,
    pub is_array: bool,
}

/// Writer prefix shared by the four number writers (the inlined separator
/// logic of the retail writer): with an empty level stack only the
/// `has_root` flag is set; otherwise a non-first value emits `,` (arrays)
/// or `:` for odd counts / `,` for even counts (objects), then the top
/// level count is incremented.
pub fn w23a14_write_prefix(levels: &mut [W23a14Level], has_root: &mut bool, out: &mut Vec<u8>) {
    match levels.last_mut() {
        None => *has_root = true,
        Some(top) => {
            if top.count != 0 {
                if top.is_array {
                    out.push(b',');
                } else {
                    out.push(if top.count & 1 != 0 { b':' } else { b',' });
                }
            }
            top.count += 1;
        }
    }
}

/// `0x007060f0`: writer Int32: prefix, optional `-`, decimal digits
/// (negation is `!x + 1`, so `i32::MIN` prints as 2147483648 with sign).
pub fn w23a14_write_i32(
    levels: &mut [W23a14Level],
    has_root: &mut bool,
    out: &mut Vec<u8>,
    v: i32,
) {
    w23a14_write_prefix(levels, has_root, out);
    if v < 0 {
        out.push(b'-');
    }
    let mag = if v < 0 {
        (!(v as u32)).wrapping_add(1)
    } else {
        v as u32
    };
    out.extend_from_slice(mag.to_string().as_bytes());
}

/// `0x00706970`: writer Uint32.
pub fn w23a14_write_u32(
    levels: &mut [W23a14Level],
    has_root: &mut bool,
    out: &mut Vec<u8>,
    v: u32,
) {
    w23a14_write_prefix(levels, has_root, out);
    out.extend_from_slice(v.to_string().as_bytes());
}

/// `0x00705f90`: writer Int64 (sign handled via 64-bit two's complement).
pub fn w23a14_write_i64(
    levels: &mut [W23a14Level],
    has_root: &mut bool,
    out: &mut Vec<u8>,
    v: i64,
) {
    w23a14_write_prefix(levels, has_root, out);
    if v < 0 {
        out.push(b'-');
    }
    let mag = if v < 0 {
        (!(v as u64)).wrapping_add(1)
    } else {
        v as u64
    };
    out.extend_from_slice(mag.to_string().as_bytes());
}

/// `0x00706830`: writer Uint64.
pub fn w23a14_write_u64(
    levels: &mut [W23a14Level],
    has_root: &mut bool,
    out: &mut Vec<u8>,
    v: u64,
) {
    w23a14_write_prefix(levels, has_root, out);
    out.extend_from_slice(v.to_string().as_bytes());
}

/// `0x00706230`: double formatting Prettify over `buf` holding `length`
/// digits with decimal exponent `k`.  Returns the new end index, or `None`
/// when the layout would exceed the buffer.  `write_exp(exp, dst)` is the
/// exponent writer (callee at 706ce0) returning the end index.
pub fn w23a14_prettify(
    buf: &mut [u8],
    length: i32,
    k: i32,
    max_decimal_places: i32,
    mut write_exp: impl FnMut(i32, &mut [u8]) -> usize,
) -> Option<usize> {
    let kk = length + k;
    let l = length as usize;
    if (0..=0x7fff_ffff).contains(&k) && kk < 0x16 {
        let kk_u = kk as usize;
        if l < kk_u {
            for b in &mut buf[l..kk_u] {
                *b = b'0';
            }
        }
        buf[kk_u] = b'.';
        buf[kk_u + 1] = b'0';
        return Some(kk_u + 2);
    }
    if (1..0x16).contains(&kk) {
        let kk_u = kk as usize;
        buf.copy_within(kk_u..l, kk_u + 1);
        buf[kk_u] = b'.';
        return Some(l + 1);
    }
    if (-5..=0).contains(&kk) {
        let off = (2 - kk) as usize;
        buf.copy_within(0..l, off);
        buf[0] = b'0';
        buf[1] = b'.';
        for b in &mut buf[2..off] {
            *b = b'0';
        }
        return Some(l + off);
    }
    if kk < -max_decimal_places {
        buf[0] = b'0';
        buf[1] = b'.';
        buf[2] = b'0';
        return Some(3);
    }
    if length == 1 {
        buf[1] = b'e';
        return Some(write_exp(kk - 1, &mut buf[2..]) + 2);
    }
    buf.copy_within(1..l, 2);
    buf[1] = b'.';
    buf[l + 1] = b'e';
    Some(write_exp(kk - 1, &mut buf[l + 2..]) + l + 2)
}

/// `0x00707500`: string-buffer snapshot.  `used` is end-begin of the write
/// cursor; when it exceeds the cached length the NUL terminator is stored
/// at `buf[used]` and the cache updated.  Returns the cached-length bytes.
pub fn w23a14_stringbuffer_snapshot(
    buf: &mut [u8],
    cached_len: &mut usize,
    used: usize,
) -> Vec<u8> {
    if *cached_len < used {
        buf[used] = 0;
        *cached_len = used;
    }
    buf[..*cached_len].to_vec()
}

/// `0x00708070`: builds a 256-entry byte table, filled from index 255 down
/// to 0 by `f` (callee at 711a90).
pub fn w23a14_build_byte_table(mut f: impl FnMut(u8) -> u8) -> Vec<u8> {
    let mut t = vec![0u8; 256];
    for i in (0..256usize).rev() {
        t[i] = f(i as u8);
    }
    t
}

/// `0x007081c0`: resets the 256-byte remap table to identity and returns the
/// tileset dark palette path.
pub fn w23a14_reset_remap_and_dark_path(table: &mut [u8; 256], tileset: &str) -> String {
    for (i, b) in table.iter_mut().enumerate() {
        *b = i as u8;
    }
    format!("Tileset\\{tileset}\\dark.pcx")
}

/// `0x00708270`: zeroes `w * h * 6` bytes (6-byte map cells).
pub fn w23a14_clear_cells(dims: (i32, i32), buf: &mut [u8]) {
    let n = (dims.0.wrapping_mul(dims.1).wrapping_mul(6)) as usize;
    for b in &mut buf[..n] {
        *b = 0;
    }
}

/// `0x00708480`: walks `count` 28-byte (7 dword) records and calls `free`
/// with the pointer stored at dword 6 of each when non-zero.
pub fn w23a14_free_records(ptrs: &[u32], count: usize, mut free: impl FnMut(u32)) {
    for i in 0..count {
        let p = ptrs[i * 7 + 6];
        if p != 0 {
            free(p);
        }
    }
}

/// `0x00708850`: zeroes a three-dword vector header (begin, end, cap).
pub fn w23a14_vec_header_clear(h: &mut [u32; 3]) {
    *h = [0; 3];
}

#[cfg(test)]
mod tests_wave23_agent14 {
    use super::*;

    #[test]
    fn getters() {
        assert_eq!(w23a14_member_as_u32(7, 0x100, || 1), Ok(7));
        assert_eq!(w23a14_value_as_u32(0, 0x200, || 9), Ok(9));
        assert_eq!(w23a14_value_as_u32(0, 10, || 0), Ok(1));
        assert_eq!(w23a14_value_as_u32(0, 9, || 0), Ok(0));
        assert_eq!(w23a14_value_as_u32(0, 0, || 0), Err(W23A14_BAD_TYPE));
        assert_eq!(w23a14_value_as_u64(0xffff_ffff, 0x20, || 0), Ok(u64::MAX));
        assert_eq!(
            w23a14_value_as_u64(0xffff_ffff, 0x40, || 0),
            Ok(0xffff_ffff)
        );
        assert_eq!(w23a14_value_as_u64(5 << 40, 0x80, || 0), Ok(5 << 40));
        assert_eq!(w23a14_value_as_u64(0, 10, || 0), Ok(1));
    }

    #[test]
    fn writers() {
        let mut lv = vec![W23a14Level {
            count: 0,
            is_array: true,
        }];
        let mut root = false;
        let mut out = Vec::new();
        w23a14_write_i32(&mut lv, &mut root, &mut out, -5);
        w23a14_write_u32(&mut lv, &mut root, &mut out, 7);
        w23a14_write_i64(&mut lv, &mut root, &mut out, i64::MIN);
        w23a14_write_u64(&mut lv, &mut root, &mut out, u64::MAX);
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "-5,7,-9223372036854775808,18446744073709551615"
        );
        let mut lv = vec![W23a14Level {
            count: 0,
            is_array: false,
        }];
        let mut out = Vec::new();
        for _ in 0..3 {
            w23a14_write_u32(&mut lv, &mut root, &mut out, 1);
        }
        assert_eq!(out, b"1:1,1");
        let mut none: Vec<W23a14Level> = vec![];
        let mut r = false;
        w23a14_write_i32(&mut none, &mut r, &mut Vec::new(), i32::MIN);
        assert!(r);
    }

    #[test]
    fn prettify() {
        let mut b = [0u8; 32];
        b[..3].copy_from_slice(b"123");
        assert_eq!(w23a14_prettify(&mut b, 3, 2, 324, |_, _| 0), Some(7));
        assert_eq!(&b[..7], b"12300.0");
        let mut b = [0u8; 32];
        b[..3].copy_from_slice(b"123");
        assert_eq!(w23a14_prettify(&mut b, 3, -1, 324, |_, _| 0), Some(4));
        assert_eq!(&b[..4], b"12.3");
        let mut b = [0u8; 32];
        b[..2].copy_from_slice(b"12");
        assert_eq!(w23a14_prettify(&mut b, 2, -4, 324, |_, _| 0), Some(6));
        assert_eq!(&b[..6], b"0.0012");
        let mut b = [0u8; 32];
        b[..1].copy_from_slice(b"5");
        let r = w23a14_prettify(&mut b, 1, 30, 324, |e, d| {
            assert_eq!(e, 30);
            d[0] = b'3';
            d[1] = b'0';
            2
        });
        assert_eq!(r, Some(4));
        assert_eq!(&b[..4], b"5e30");
        let mut b = [0u8; 32];
        b[0] = b'5';
        assert_eq!(w23a14_prettify(&mut b, 1, -400, 324, |_, _| 0), Some(3));
        assert_eq!(&b[..3], b"0.0");
    }

    #[test]
    fn small_helpers() {
        let mut buf = *b"abcd\0xx";
        let mut c = 2;
        assert_eq!(w23a14_stringbuffer_snapshot(&mut buf, &mut c, 4), b"abcd");
        assert_eq!(c, 4);
        let t = w23a14_build_byte_table(|i| i ^ 1);
        assert_eq!((t[0], t[255]), (1, 254));
        let mut tb = [9u8; 256];
        assert_eq!(
            w23a14_reset_remap_and_dark_path(&mut tb, "jungle"),
            "Tileset\\jungle\\dark.pcx"
        );
        assert_eq!(tb[255], 255);
        let mut cells = [1u8; 30];
        w23a14_clear_cells((2, 2), &mut cells);
        assert!(cells[..24].iter().all(|&b| b == 0) && cells[24] == 1);
        let mut freed = vec![];
        w23a14_free_records(&[0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0], 2, |p| {
            freed.push(p)
        });
        assert_eq!(freed, vec![5]);
        let mut h = [1, 2, 3];
        w23a14_vec_header_clear(&mut h);
        assert_eq!(h, [0; 3]);
    }
}
