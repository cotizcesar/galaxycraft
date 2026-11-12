#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent13.rs
// ===========================================================================
// Wave 26 agent 13: bit packer, PCX RLE decoder, image-option setter and
// registry-wrapper argument validation.

/// Win32 `ERROR_INVALID_PARAMETER`, set by the validators below.
pub const W26A13_ERROR_INVALID_PARAMETER: u32 = 0x57;

/// Bit-packer state for `0x00933fd0` (GIF-style variable-width codes).
/// Field order mirrors the int array of the retail struct.
#[derive(Debug, Clone)]
pub struct W26a13BitPacker {
    /// `[0]` current code width in bits.
    pub code_bits: i32,
    /// `[1]` current maximum code.
    pub max_code: i32,
    /// `[3]` bit accumulator.
    pub accum: i32,
    /// `[4]` bits held in the accumulator.
    pub accum_bits: i32,
    /// `[9]` next-free-code limit.
    pub limit: i32,
    /// `[0xc]` bytes queued in the block.
    pub block_len: i32,
    /// Block buffer: byte 0 is the length byte, data follows.
    pub block: [u8; 0x100],
}

/// `0x00933fd0`: appends `code` at the current bit width, spills whole bytes
/// into the block (a full block of 0xff bytes is passed to `flush` with its
/// length byte and the queue is restarted), then widens the code (width 12
/// sets max code 0x1000).
pub fn w26a13_pack_code(st: &mut W26a13BitPacker, code: i32, mut flush: impl FnMut(&[u8])) {
    let total = st.code_bits + st.accum_bits;
    let mut cur = ((code as u32) << ((st.accum_bits as u32) & 0x1f)) as i32 | st.accum;
    st.accum = cur;
    st.accum_bits = total;
    while st.accum_bits > 7 {
        st.block_len += 1;
        st.block[st.block_len as usize] = cur as u8;
        if st.block_len > 0xfe {
            st.block_len += 1;
            st.block[0] = (st.block_len - 1) as u8;
            flush(&st.block[..st.block_len as usize]);
            st.block_len = 0;
        }
        st.accum >>= 8;
        st.accum_bits -= 8;
        cur = st.accum & 0xff;
    }
    if st.max_code < st.limit {
        st.code_bits += 1;
        if st.code_bits == 0xc {
            st.max_code = 0x1000;
            return;
        }
        st.max_code = (1 << st.code_bits) - 1;
    }
}

/// `0x00934080`: PCX run-length decode into an interleaved image.
/// `width` is bytes per scanline plane to keep, `stride` the encoded bytes
/// per plane line, `planes` the plane count.  Bytes below 0xc0 are literals;
/// otherwise the low six bits are a run length of the next byte.  Planes are
/// written to `dst[row + x*planes + plane]` (plane counted downward, so the
/// first encoded plane lands in the highest slot).  Decoding stops when
/// either buffer is exhausted.  The loop-top end test is applied to the
/// destination position.
pub fn w26a13_pcx_rle_decode(dst: &mut [u8], src: &[u8], width: u32, stride: u32, planes: u32) {
    if planes == 0 || stride == 0 {
        return;
    }
    let dst_end = dst.len() as i64;
    let row_adv = width as i64 * planes as i64;
    let planes_i = planes as i64;
    let mut base: i64 = 0;
    let mut x: u32 = 0;
    let mut plane: i64 = planes_i - 1;
    let mut sp = 0usize;
    let put = |dst: &mut [u8], idx: i64, v: u8| {
        if idx >= 0 && idx < dst_end {
            dst[idx as usize] = v;
        }
    };
    loop {
        if base + x as i64 >= dst_end || sp >= src.len() {
            return;
        }
        let b = src[sp];
        sp += 1;
        if b < 0xc0 {
            if x < width {
                let idx = base + x as i64 * planes_i + plane;
                if idx < dst_end {
                    put(dst, idx, b);
                }
            }
            x += 1;
            if x >= stride {
                x -= stride;
                plane -= 1;
                if plane < 0 {
                    base += row_adv;
                    plane = planes_i - 1;
                }
            }
        } else {
            let val = src.get(sp).copied().unwrap_or(0);
            sp += 1;
            let mut left = ((b & 0x3f) as u32).min((dst_end - base).max(0) as u32);
            loop {
                if width <= x {
                    let n = left.min(stride - x);
                    x += n;
                    if x >= stride {
                        x -= stride;
                        plane -= 1;
                        if plane < 0 {
                            base += row_adv;
                            plane = planes_i - 1;
                        }
                    }
                    left -= n;
                    if left == 0 {
                        break;
                    }
                }
                let n = left.min(width - x);
                let mut i = plane + x as i64 * planes_i;
                let lim = (n as i64 * planes_i).min(dst_end - base);
                let end = i + lim;
                while i < end {
                    put(dst, base + i, val);
                    i += planes_i;
                }
                x += n;
                if x >= stride {
                    x -= stride;
                    plane -= 1;
                    if plane < 0 {
                        base += row_adv;
                        plane = planes_i - 1;
                    }
                }
                left -= n;
                if left == 0 {
                    break;
                }
            }
        }
    }
}

/// Image-option globals written by `0x00934f20`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct W26a13ImageOptions {
    pub opt1: u32,
    /// Option 2 (must be a power of two).
    pub opt2: u32,
    pub opt3: u32,
    pub opt4: u32,
    pub opt5: u32,
    pub opt6: [u32; 2],
    pub opt7: u32,
    /// Boolean options 9, 10, 0xc, 0xd, 0xe, 0xf.
    pub flag9: u32,
    pub flag10: u32,
    pub flag12: u32,
    pub flag13: u32,
    pub flag14: u32,
    pub flag15: u32,
}

/// `0x00934f20`: sets image option `id` from `data` (`size` bytes).  Sets
/// `last_error` to 0x57 first and to 0 on success; returns true on success.
pub fn w26a13_set_image_option(
    o: &mut W26a13ImageOptions,
    id: u32,
    data: Option<&[u32]>,
    size: u32,
    last_error: &mut u32,
) -> bool {
    *last_error = W26A13_ERROR_INVALID_PARAMETER;
    let Some(d) = data else { return false };
    let ok = match id {
        1 if size == 4 => {
            o.opt1 = d[0];
            true
        }
        2 if size == 4
            && d[0] != 0
            && (d[0].wrapping_sub(1) ^ d[0]) == d[0].wrapping_mul(2).wrapping_sub(1) =>
        {
            o.opt2 = d[0];
            true
        }
        3 if size == 4 => {
            o.opt3 = d[0];
            true
        }
        4 if size == 4 => {
            o.opt4 = d[0];
            true
        }
        5 if size == 4 => {
            o.opt5 = d[0];
            true
        }
        6 if size == 8 && d.len() >= 2 => {
            o.opt6 = [d[0], d[1]];
            true
        }
        7 if size == 4 => {
            o.opt7 = d[0];
            true
        }
        9 if size == 4 => {
            o.flag9 = (d[0] != 0) as u32;
            true
        }
        10 if size == 4 => {
            o.flag10 = (d[0] != 0) as u32;
            true
        }
        0xc if size == 4 => {
            o.flag12 = (d[0] != 0) as u32;
            true
        }
        0xd if size == 4 => {
            o.flag13 = (d[0] != 0) as u32;
            true
        }
        0xe if size == 4 => {
            o.flag14 = (d[0] != 0) as u32;
            true
        }
        0xf if size == 4 => {
            o.flag15 = (d[0] != 0) as u32;
            true
        }
        _ => false,
    };
    if ok {
        *last_error = 0;
    }
    ok
}

/// Shared validation of the registry wrappers: key name non-empty and the
/// value name present.
fn w26a13_valid_names(key: &str, value_present: bool, last_error: &mut u32) -> bool {
    if key.is_empty() || !value_present {
        *last_error = W26A13_ERROR_INVALID_PARAMETER;
        return false;
    }
    true
}

/// `0x00935ec0`: validates the names, then forwards to the registry reader
/// with `out_len` (or a local when absent).  Failure sets error 0x57.
pub fn w26a13_reg_read_wrapper(
    key: &str,
    value_present: bool,
    out_len: Option<&mut u32>,
    last_error: &mut u32,
    read: impl FnOnce(&mut u32) -> bool,
) -> bool {
    if !w26a13_valid_names(key, value_present, last_error) {
        return false;
    }
    let mut local = 0u32;
    read(out_len.unwrap_or(&mut local))
}

/// `0x009362c0`: validated registry write of value type 3 (binary).
pub fn w26a13_reg_write_type3(
    key: &str,
    value_present: bool,
    last_error: &mut u32,
    write: impl FnOnce(u32) -> bool,
) -> bool {
    w26a13_valid_names(key, value_present, last_error) && write(3)
}

/// `0x00936590`: validated registry write of value type 0xb (QWORD), 8 bytes.
pub fn w26a13_reg_write_qword(
    key: &str,
    value_present: bool,
    last_error: &mut u32,
    write: impl FnOnce(u32, u32) -> bool,
) -> bool {
    w26a13_valid_names(key, value_present, last_error) && write(0xb, 8)
}

/// `0x009365d0`: validated registry write of value type 4 (DWORD), 4 bytes.
pub fn w26a13_reg_write_dword(
    key: &str,
    value_present: bool,
    last_error: &mut u32,
    write: impl FnOnce(u32, u32) -> bool,
) -> bool {
    w26a13_valid_names(key, value_present, last_error) && write(4, 4)
}

#[cfg(test)]
mod tests_wave26_agent13 {
    use super::*;

    fn packer(bits: i32) -> W26a13BitPacker {
        W26a13BitPacker {
            code_bits: bits,
            max_code: (1 << bits) - 1,
            accum: 0,
            accum_bits: 0,
            limit: 0,
            block_len: 0,
            block: [0; 0x100],
        }
    }

    #[test]
    fn pack_code_bytes_and_widen() {
        let mut p = packer(9);
        p.limit = 1000;
        p.max_code = 5;
        w26a13_pack_code(&mut p, 0x1ff, |_| panic!());
        assert_eq!(p.block_len, 1);
        assert_eq!(p.block[1], 0xff);
        assert_eq!(p.accum_bits, 1);
        assert_eq!(p.code_bits, 10);
        assert_eq!(p.max_code, 1023);
        let mut q = packer(11);
        q.limit = 5000;
        q.max_code = 0;
        w26a13_pack_code(&mut q, 0, |_| {});
        assert_eq!(q.code_bits, 12);
        assert_eq!(q.max_code, 0x1000);
    }

    #[test]
    fn pack_code_flushes_full_block() {
        let mut p = packer(8);
        p.block_len = 0xfe;
        let mut got = 0;
        w26a13_pack_code(&mut p, 0x41, |b| {
            got = b.len();
            assert_eq!(b[0], 0xff);
        });
        assert_eq!(got, 0x100);
        assert_eq!(p.block_len, 0);
    }

    #[test]
    fn pcx_literal_and_run_single_plane() {
        let mut d = [0u8; 4];
        w26a13_pcx_rle_decode(&mut d, &[0x01, 0xc2, 0x07, 0x09], 4, 4, 1);
        assert_eq!(d, [1, 7, 7, 9]);
    }

    #[test]
    fn pcx_three_planes() {
        // 2 pixels, 3 planes, stride 2: planes arrive in order 2,1,0.
        let mut d = [0u8; 6];
        let src = [1, 2, 3, 4, 5, 6];
        w26a13_pcx_rle_decode(&mut d, &src, 2, 2, 3);
        assert_eq!(d, [5, 3, 1, 6, 4, 2]);
    }

    #[test]
    fn pcx_stops_on_short_input() {
        let mut d = [9u8; 4];
        w26a13_pcx_rle_decode(&mut d, &[], 4, 4, 1);
        assert_eq!(d, [9; 4]);
    }

    #[test]
    fn image_options() {
        let mut o = W26a13ImageOptions::default();
        let mut e = 0;
        assert!(w26a13_set_image_option(&mut o, 1, Some(&[7]), 4, &mut e));
        assert_eq!((o.opt1, e), (7, 0));
        assert!(!w26a13_set_image_option(&mut o, 2, Some(&[6]), 4, &mut e));
        assert_eq!(e, 0x57);
        assert!(w26a13_set_image_option(&mut o, 2, Some(&[8]), 4, &mut e));
        assert_eq!(o.opt2, 8);
        assert!(w26a13_set_image_option(&mut o, 6, Some(&[1, 2]), 8, &mut e));
        assert_eq!(o.opt6, [1, 2]);
        assert!(w26a13_set_image_option(&mut o, 0xf, Some(&[99]), 4, &mut e));
        assert_eq!(o.flag15, 1);
        assert!(!w26a13_set_image_option(&mut o, 3, Some(&[1]), 8, &mut e));
        assert!(!w26a13_set_image_option(&mut o, 8, Some(&[1]), 4, &mut e));
        assert!(!w26a13_set_image_option(&mut o, 1, None, 4, &mut e));
    }

    #[test]
    fn registry_wrappers() {
        let mut e = 0;
        assert!(!w26a13_reg_read_wrapper("", true, None, &mut e, |_| true));
        assert_eq!(e, 0x57);
        let mut n = 5u32;
        assert!(w26a13_reg_read_wrapper(
            "k",
            true,
            Some(&mut n),
            &mut e,
            |l| {
                *l = 9;
                true
            }
        ));
        assert_eq!(n, 9);
        assert!(w26a13_reg_write_type3("k", true, &mut e, |t| t == 3));
        assert!(w26a13_reg_write_qword("k", true, &mut e, |t, s| (t, s) == (0xb, 8)));
        assert!(w26a13_reg_write_dword("k", true, &mut e, |t, s| (t, s) == (4, 4)));
        assert!(!w26a13_reg_write_dword("k", false, &mut e, |_, _| true));
    }
}
