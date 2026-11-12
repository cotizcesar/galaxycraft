#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent65.rs
// ===========================================================================
// Wave 31 batch 65: Vorbis-style bit reader and codec state helpers.

/// Bit-reader state used by `0x009f02c0` and `0x009f03b0`
/// (`[endbyte, endbit, _, ptr, storage]`, ptr is an index into `buf`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W31a65BitReader {
    pub endbyte: i32,
    pub endbit: i32,
    pub field2: i32,
    pub ptr: Option<usize>,
    pub storage: i32,
}

/// `0x009f03b0`: initialises a buffer descriptor (five dwords): zero
/// `[0]`, `[1]`; `[2]` and `[3]` get `a`; `[4]` gets `b`.
pub fn w31a65_buffer_init(p: &mut [u32; 5], a: u32, b: u32) {
    p[0] = 0;
    p[1] = 0;
    p[3] = a;
    p[2] = a;
    p[4] = b;
}

/// `0x009f02c0`: reads `bits` (<= 32) bits LSB-first; returns `0xffffffff`
/// and exhausts the reader on overrun or `bits > 32`.  The mask table is
/// `2^n - 1` (assumed standard Ogg layout of the retail table).
pub fn w31a65_bit_read(r: &mut W31a65BitReader, buf: &[u8], bits: u32) -> u32 {
    let fail = |r: &mut W31a65BitReader| {
        r.endbyte = r.storage;
        r.ptr = None;
        r.endbit = 1;
        0xffff_ffffu32
    };
    if bits > 32 {
        return fail(r);
    }
    let mask: u32 = if bits == 32 {
        u32::MAX
    } else {
        (1u32 << bits) - 1
    };
    let total = bits as i32 + r.endbit;
    if r.storage - 4 <= r.endbyte {
        if r.storage - ((total + 7) >> 3) < r.endbyte {
            return fail(r);
        }
        if total == 0 {
            return 0;
        }
    }
    let base = r.ptr.unwrap_or(0);
    let byte = |k: usize| buf.get(base + k).copied().unwrap_or(0) as u32;
    let sh = (r.endbit & 31) as u32;
    let mut v = byte(0) >> sh;
    if total > 8 {
        v |= byte(1) << ((8 - sh) & 31);
        if total > 16 {
            v |= byte(2) << ((16 - sh) & 31);
            if total > 24 {
                v |= byte(3) << ((24u32.wrapping_sub(sh)) & 31);
                if total > 32 && r.endbit != 0 {
                    v |= byte(4) << ((32u32.wrapping_sub(sh)) & 31);
                }
            }
        }
    }
    r.endbit = total & 7;
    let adv = (total + ((total >> 31) & 7)) >> 3;
    r.ptr = Some(base + adv as usize);
    r.endbyte += adv;
    v & mask
}

/// `0x009f25c0`: frees `*[0]` through `free` when non-zero and clears the
/// seven-dword record; always returns 0.
pub fn w31a65_clear_record(rec: Option<&mut [i32; 7]>, mut free: impl FnMut(i32)) -> i32 {
    if let Some(r) = rec {
        if r[0] != 0 {
            free(r[0]);
        }
        *r = [0; 7];
    }
    0
}

/// `0x009f7200`: zeroes an eight-dword record and stores a freshly
/// allocated `0xe50` byte block handle in slot 7.
pub fn w31a65_state_new(rec: &mut [u32; 8], alloc: impl FnOnce(u32, u32) -> u32) {
    *rec = [0; 8];
    rec[7] = alloc(1, 0xe50);
}

/// `0x009f8460`: advances the cursor at `+0x18` by `delta`; fails with
/// `0xffffff7d` when `delta != 0` and `limit (+0x14) < cursor + delta`.
pub fn w31a65_advance(limit: i32, cursor: &mut i32, delta: i32) -> u32 {
    if delta != 0 && limit < cursor.wrapping_add(delta) {
        return 0xffff_ff7d;
    }
    *cursor = cursor.wrapping_add(delta);
    0
}

/// `0x009f8400`: when `0 <= cursor < limit`, fills `out[i] = base[i] +
/// cursor*4` for each channel and returns `limit - cursor`; else 0.
pub fn w31a65_channel_view(
    limit: i32,
    cursor: i32,
    base: &[i32],
    channels: usize,
    out: &mut [i32],
    want_out: bool,
) -> i32 {
    if cursor >= 0 && cursor < limit {
        if want_out {
            for i in 0..channels.min(base.len()).min(out.len()) {
                out[i] = base[i].wrapping_add(cursor.wrapping_mul(4));
            }
        }
        return limit - cursor;
    }
    0
}

/// Shared parameters for the two reset helpers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W31a65ResetInfo {
    pub block_size: i32,
    pub shift: u32,
}

/// `0x009f3270`: after init (`init_result`), non-zero runs `cleanup` and is
/// returned; otherwise resets sentinel fields in `s` (16 dwords) and the
/// two aux words, using `info` (block size >> (shift+1), >> shift).
pub fn w31a65_reset_a(
    init_result: i32,
    s: &mut [i32; 16],
    aux: Option<&mut [i32; 2]>,
    info: Option<W31a65ResetInfo>,
    mut cleanup: impl FnMut(),
) -> i32 {
    if init_result != 0 {
        cleanup();
        return init_result;
    }
    if let (Some(aux), Some(info)) = (aux, info) {
        let h = info.block_size >> ((info.shift + 1) & 31);
        s[5] = -1;
        s[0xc] = -1;
        s[0xd] = -1;
        s[0xb] = h;
        s[4] = h >> (info.shift & 31);
        s[0xe] = -1;
        s[0xf] = -1;
        s[7] = 0;
        aux[0] = -1;
        aux[1] = -1;
    }
    0
}

/// `0x009f8360`: same reset as the previous one on the other layout
/// (fields `+0x18,+0x38,+0x3c,+0x30,+0x14,+0x40,+0x44,+0x20` as dword
/// indices 6,14,15,12,5,16,17,8; aux `+0x80/+0x84`).  Returns 1 on init
/// failure (after `cleanup`), else 0.
pub fn w31a65_reset_b(
    init_result: i32,
    s: &mut [i32; 18],
    aux: Option<&mut [i32; 2]>,
    info: Option<W31a65ResetInfo>,
    mut cleanup: impl FnMut(),
) -> u32 {
    if init_result != 0 {
        cleanup();
        return 1;
    }
    if let (Some(aux), Some(info)) = (aux, info) {
        let h = info.block_size >> ((info.shift + 1) & 31);
        s[6] = -1;
        s[14] = -1;
        s[15] = -1;
        s[12] = h;
        s[5] = h >> (info.shift & 31);
        s[16] = -1;
        s[17] = -1;
        s[8] = 0;
        aux[0] = -1;
        aux[1] = -1;
    }
    0
}

/// `0x00a04da0`: tears down a handle: null -> 8; with a destructor
/// (`has_dtor`) and `[6] != 0` calls `dtor(handle)`, clears fields, returns
/// 0; otherwise sets `[2] = 1` and returns 1.
pub fn w31a65_destroy(h: Option<&mut [u32; 7]>, has_dtor: bool, dtor: impl FnOnce(u32)) -> u32 {
    let Some(h) = h else { return 8 };
    if has_dtor && h[1] != 0 && h[6] != 0 {
        dtor(h[6]);
        h[1] = 0;
        h[0] = 0;
        h[6] = 0;
        h[2] = 0;
        return 0;
    }
    h[2] = 1;
    1
}

/// `0x00a09140`: aligned allocation; `raw = alloc(size + 3 + align)`,
/// result `(align + 3 + raw) & -align` with `raw` stored at `result - 4`.
/// Returns `(aligned, raw)` or `None` when allocation fails.
pub fn w31a65_aligned_alloc(
    align: u32,
    size: u32,
    alloc: impl FnOnce(u32) -> u32,
) -> Option<(u32, u32)> {
    let raw = alloc(size.wrapping_add(3).wrapping_add(align));
    if raw == 0 {
        return None;
    }
    let aligned = align.wrapping_add(3).wrapping_add(raw) & align.wrapping_neg();
    Some((aligned, raw))
}

#[cfg(test)]
mod tests_wave31_agent65 {
    use super::*;

    #[test]
    fn small_records() {
        let mut p = [9u32; 5];
        w31a65_buffer_init(&mut p, 7, 3);
        assert_eq!(p, [0, 0, 7, 7, 3]);
        let mut freed = vec![];
        let mut r = [5, 1, 2, 3, 4, 5, 6];
        assert_eq!(w31a65_clear_record(Some(&mut r), |x| freed.push(x)), 0);
        assert_eq!((freed, r), (vec![5], [0; 7]));
        let mut s = [1u32; 8];
        w31a65_state_new(&mut s, |a, b| a + b);
        assert_eq!(s[7], 0xe51);
        assert_eq!(s[0], 0);
    }

    #[test]
    fn advance_and_view() {
        let mut c = 2;
        assert_eq!(w31a65_advance(5, &mut c, 4), 0xffff_ff7d);
        assert_eq!(c, 2);
        assert_eq!(w31a65_advance(5, &mut c, 3), 0);
        assert_eq!(c, 5);
        let mut out = [0; 2];
        assert_eq!(
            w31a65_channel_view(10, 3, &[100, 200], 2, &mut out, true),
            7
        );
        assert_eq!(out, [112, 212]);
        assert_eq!(w31a65_channel_view(10, 10, &[], 0, &mut out, false), 0);
    }

    #[test]
    fn bits() {
        let buf = [0b1010_0101u8, 0xff, 0, 0, 0, 0];
        let mut r = W31a65BitReader {
            endbyte: 0,
            endbit: 0,
            field2: 0,
            ptr: Some(0),
            storage: 6,
        };
        assert_eq!(w31a65_bit_read(&mut r, &buf, 4), 0b0101);
        assert_eq!(r.endbit, 4);
        assert_eq!(w31a65_bit_read(&mut r, &buf, 8), 0xfa);
        assert_eq!(w31a65_bit_read(&mut r, &buf, 33), 0xffff_ffff);
        assert_eq!(r.endbyte, 6);
    }

    #[test]
    fn resets() {
        let mut s = [0; 16];
        let mut a = [0; 2];
        let info = W31a65ResetInfo {
            block_size: 2048,
            shift: 8,
        };
        assert_eq!(
            w31a65_reset_a(0, &mut s, Some(&mut a), Some(info.clone()), || {}),
            0
        );
        assert_eq!((s[0xb], s[4], s[5], a), (4, 0, -1, [-1, -1]));
        let mut s2 = [0; 18];
        assert_eq!(
            w31a65_reset_b(0, &mut s2, Some(&mut a), Some(info), || {}),
            0
        );
        assert_eq!((s2[12], s2[6]), (4, -1));
        let mut called = false;
        assert_eq!(w31a65_reset_b(3, &mut s2, None, None, || called = true), 1);
        assert!(called);
    }

    #[test]
    fn destroy_and_alloc() {
        assert_eq!(w31a65_destroy(None, true, |_| {}), 8);
        let mut h = [1, 1, 0, 0, 0, 0, 5];
        assert_eq!(w31a65_destroy(Some(&mut h), true, |_| {}), 0);
        assert_eq!(h, [0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(w31a65_destroy(Some(&mut h), true, |_| {}), 1);
        assert_eq!(h[2], 1);
        assert_eq!(
            w31a65_aligned_alloc(16, 8, |_| 0x1001),
            Some((0x1010, 0x1001))
        );
        assert_eq!(w31a65_aligned_alloc(16, 8, |_| 0), None);
    }
}
