#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent23.rs
// ===========================================================================
// Wave 31 agent 23: two-row SSSE3 vertical blend kernels and hex writers.

/// Weight pair used by the vertical blend kernels: signed-saturated words 3
/// and 4 of the 16-byte parameter vector (`packsswb` of `v >> 48`).
pub fn w31a23_weights(v: &[u8; 16]) -> [i8; 2] {
    let w = |i: usize| i16::from_le_bytes([v[i], v[i + 1]]).clamp(-128, 127) as i8;
    [w(6), w(8)]
}

/// One output byte: `pmaddubsw` of (row0, row1) with the weights, `pmulhrsw`
/// by 0x100, then clamp to 0..=255.
fn w31a23_blend(a: u8, b: u8, w: [i8; 2]) -> u8 {
    let s = (a as i32 * w[0] as i32 + b as i32 * w[1] as i32).clamp(-32768, 32767);
    let r = ((s >> 6) + 1) >> 1;
    r.clamp(0, 255) as u8
}

fn w31a23_rows<const N: usize>(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    rows: usize,
    v: &[u8; 16],
    average: bool,
) {
    let w = w31a23_weights(v);
    for r in 0..rows {
        for i in 0..N {
            let m = w31a23_blend(
                src[r * src_stride + i],
                src[r * src_stride + src_stride + i],
                w,
            );
            let d = &mut dst[r * dst_stride + i];
            *d = if average {
                ((m as u16 + *d as u16 + 1) >> 1) as u8
            } else {
                m
            };
        }
    }
}

/// `0x004db5ea`: 16-wide vertical blend of a row and the next row into `dst`.
/// The original is a do-while (rows == 0 would wrap); callers pass rows >= 1.
pub fn w31a23_blend16(src: &[u8], ss: usize, dst: &mut [u8], ds: usize, rows: usize, v: &[u8; 16]) {
    w31a23_rows::<16>(src, ss, dst, ds, rows, v, false);
}

/// `0x004db663`: 4-wide vertical blend averaged (`pavgb`) with `dst`.
pub fn w31a23_blend4_avg(
    src: &[u8],
    ss: usize,
    dst: &mut [u8],
    ds: usize,
    rows: usize,
    v: &[u8; 16],
) {
    w31a23_rows::<4>(src, ss, dst, ds, rows, v, true);
}

/// `0x004db6ce`: 8-wide vertical blend averaged with `dst`.
pub fn w31a23_blend8_avg(
    src: &[u8],
    ss: usize,
    dst: &mut [u8],
    ds: usize,
    rows: usize,
    v: &[u8; 16],
) {
    w31a23_rows::<8>(src, ss, dst, ds, rows, v, true);
}

/// `0x004db73d`: 16-wide vertical blend averaged with `dst`.
pub fn w31a23_blend16_avg(
    src: &[u8],
    ss: usize,
    dst: &mut [u8],
    ds: usize,
    rows: usize,
    v: &[u8; 16],
) {
    w31a23_rows::<16>(src, ss, dst, ds, rows, v, true);
}

/// `0x00555dd0`: writes `0x` then `value` as lowercase hex right-aligned in a
/// field of `width` bytes after the prefix; the cursor ends at the field end.
/// Digits are written backwards until the value is exhausted (do-while).
pub fn w31a23_write_hex32(buf: &mut [u8], pos: &mut usize, value: u32, width: usize) {
    buf[*pos] = b'0';
    buf[*pos + 1] = b'x';
    *pos += 2;
    let end = *pos + width;
    let mut p = end;
    let mut v = value;
    loop {
        p -= 1;
        buf[p] = b"0123456789abcdef"[(v & 0xf) as usize];
        v >>= 4;
        if v == 0 {
            break;
        }
    }
    *pos = end;
}

/// `0x00555ec0`: writes a 64-bit value as hex backwards from `*pos + width`;
/// uppercase unless `lower` (format flag byte == 'x'). No prefix.
pub fn w31a23_write_hex64(buf: &mut [u8], pos: &mut usize, value: u64, width: usize, lower: bool) {
    let digits: &[u8; 16] = if lower {
        b"0123456789abcdef"
    } else {
        b"0123456789ABCDEF"
    };
    let end = *pos + width;
    let mut p = end;
    let mut v = value;
    loop {
        p -= 1;
        buf[p] = digits[(v & 0xf) as usize];
        v >>= 4;
        if v == 0 {
            break;
        }
    }
    *pos = end;
}

/// `0x00555e80`: optional sign/prefix byte, then the callee formats the rest
/// and returns the new cursor.
pub fn w31a23_write_signed_field(
    buf: &mut [u8],
    pos: &mut usize,
    prefix: u8,
    format: impl FnOnce(&mut [u8], usize) -> usize,
) {
    if prefix != 0 {
        buf[*pos] = prefix;
        *pos += 1;
    }
    *pos = format(buf, *pos);
}

#[cfg(test)]
mod tests_wave31_agent23 {
    use super::*;

    fn vec_with(w3: i16, w4: i16) -> [u8; 16] {
        let mut v = [0u8; 16];
        v[6..8].copy_from_slice(&w3.to_le_bytes());
        v[8..10].copy_from_slice(&w4.to_le_bytes());
        v
    }

    #[test]
    fn blend_variants() {
        let v = vec_with(64, 64);
        let src: Vec<u8> = (0..32).map(|i| if i < 16 { 100 } else { 50 }).collect();
        let mut d = [0u8; 16];
        w31a23_blend16(&src, 16, &mut d, 16, 1, &v);
        assert!(d.iter().all(|&x| x == 75));
        let mut d = [100u8; 16];
        w31a23_blend16_avg(&src, 16, &mut d, 16, 1, &v);
        assert_eq!(d[0], 88);
        let mut d = [0u8; 8];
        w31a23_blend8_avg(&src, 16, &mut d, 8, 1, &vec_with(-5, 0));
        assert_eq!(d[0], 0);
        let mut d = [10u8; 4];
        w31a23_blend4_avg(&src, 16, &mut d, 4, 1, &vec_with(127, 127));
        assert_eq!(d[0], 80);
        assert_eq!(w31a23_weights(&vec_with(300, -300)), [127, -128]);
    }

    #[test]
    fn hex_writers() {
        let mut b = [b'.'; 16];
        let mut p = 0;
        w31a23_write_hex32(&mut b, &mut p, 0xbeef, 8);
        assert_eq!(&b[..10], b"0x....beef");
        assert_eq!(p, 10);
        let mut p = 0;
        w31a23_write_hex64(&mut b, &mut p, 0x1_0000_00af, 10, false);
        assert_eq!(&b[..10], b"01000000AF".as_slice().get(..10).unwrap());
        let mut p = 0;
        w31a23_write_hex64(&mut b, &mut p, 0xaf, 2, true);
        assert_eq!(&b[..2], b"af");
        let mut p = 0;
        w31a23_write_signed_field(&mut b, &mut p, b'-', |buf, at| {
            buf[at] = b'7';
            at + 1
        });
        assert_eq!(&b[..2], b"-7");
        assert_eq!(p, 2);
    }
}
