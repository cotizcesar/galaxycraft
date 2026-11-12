#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent80.rs
// ===========================================================================
// Wave 32 translation batch 80: Vorbis-style codec helpers.

/// `0x009f6bc0`: fills `out` with one byte per call of `read_bits(8)`
/// (the 8-bit LSB-first reader at 0xaa_d220), in order.
pub fn w32a80_read_bytes(out: &mut [u8], mut read_bits: impl FnMut(u32) -> u32) {
    for b in out.iter_mut() {
        *b = read_bits(8) as u8;
    }
}

/// Result state of `0x009f3af0` (`param_2` dword fields that it writes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W32a80BlockState {
    pub f0: i32,
    pub f6: i32,
    pub f7: i32,
    pub f8: i32,
    pub f9: i32,
    pub f10: i32,
    pub f11: i32,
    pub f12_15: [i32; 4],
}

/// `0x009f3af0`: Vorbis block header parse.  `pre` models the first call
/// (0x9f_34a0, non-zero result is returned as is), `init_buf` the buffer
/// init (0x9f_03b0) from `src[0]`, `src[1]`, `read` the bit reader.
/// `mode_bits` is the field at +0x10 of the mode info, `mode_blockflag(m)`
/// the first dword of mode entry `m`.  `src` has 8 dwords.  Returns the
/// status (0, -0x87, -0x88, or `pre`'s value).
pub fn w32a80_block_header(
    st: &mut W32a80BlockState,
    src: &[i32; 8],
    mode_bits: u32,
    pre: impl FnOnce() -> i32,
    init_buf: impl FnOnce(i32, i32),
    mut read: impl FnMut(u32) -> i32,
    mode_blockflag: impl Fn(i32) -> i32,
) -> i32 {
    let r = pre();
    if r != 0 {
        return r;
    }
    init_buf(src[0], src[1]);
    if read(1) != 0 {
        return -0x87;
    }
    let mode = read(mode_bits);
    if mode != -1 {
        st.f10 = mode;
        let flag = mode_blockflag(mode);
        st.f7 = flag;
        let mut ok = true;
        if flag == 0 {
            st.f6 = 0;
            st.f8 = 0;
        } else {
            st.f6 = read(1);
            let v = read(1);
            st.f8 = v;
            ok = v != -1;
        }
        if ok {
            st.f12_15 = [src[4], src[5], src[6], src[7]];
            st.f11 = src[3];
            st.f9 = 0;
            st.f0 = 0;
            return 0;
        }
    }
    -0x88
}

/// `0x009f4610`: Vorbis floor1 packet decode.  `info` is the dword view of
/// `param_3` (`[0x100]` posts, `[0x102]` range, `[0x82..]` neighbour
/// table), `fl` the floor setup (`piVar1`).  `read_bits(n)` reads n bits,
/// `decode(book)` is 0x9f_11a0 on codebook `book` (-1 as `u32::MAX`).
/// Returns the post values or `None` (unused packet / error).
pub fn w32a80_floor1_decode(
    info: &[i32],
    fl: &[i32],
    mut read_bits: impl FnMut(u32) -> i32,
    mut decode: impl FnMut(i32) -> u32,
) -> Option<Vec<u32>> {
    if read_bits(1) != 1 {
        return None;
    }
    let n = *info.get(0x100)?;
    let q = *info.get(0x102)?;
    if n < 2 {
        return None;
    }
    let mut out = vec![0u32; n as usize];
    let mut bits = 0u32;
    let mut t = (q - 1) as u32;
    while t != 0 {
        bits += 1;
        t >>= 1;
    }
    out[0] = read_bits(bits) as u32;
    out[1] = read_bits(bits) as u32;
    let mut pos = 2usize;
    for i in 0..*fl.first()? {
        let cls = *fl.get(1 + i as usize)? as usize;
        let cdim = *fl.get(cls + 0x20)?;
        let csub = *fl.get(cls + 0x30)?;
        let mut cval = 0u32;
        if csub != 0 {
            cval = decode(*fl.get(cls + 0x40)?);
            if cval == u32::MAX {
                return None;
            }
        }
        for _ in 0..cdim {
            let m = (1u32 << (csub as u32 & 31)) - 1;
            let sel = cval & m;
            cval = ((cval as i32) >> (csub as u32 & 31)) as u32;
            let book = *fl.get(sel as usize + cls * 8 + 0x50)?;
            let slot = out.get_mut(pos)?;
            if book < 0 {
                *slot = 0;
            } else {
                let v = decode(book);
                *slot = v;
                if v == u32::MAX {
                    return None;
                }
            }
            pos += 1;
        }
    }
    for k in 2..n as usize {
        let lo = *info.get(0x82 + k - 2)? as usize;
        let hi = *info.get(0x82 + k - 2 + 0x3f)? as usize;
        let xk = *fl.get(0xd3 + k - 2)?;
        let xhi = *fl.get(hi + 0xd1)?;
        let xlo = *fl.get(lo + 0xd1)?;
        let _ = (xhi, xlo);
        // piVar11 = info+0x82+(k-2): *piVar11 = lo, piVar11[0x3f] = hi.
        let d = (out[lo] & 0x7fff).wrapping_sub(out[hi] & 0x7fff) as i32;
        let den = xlo.wrapping_sub(xhi);
        if den == 0 {
            return None;
        }
        let mag = (d.wrapping_abs()).wrapping_mul(xk.wrapping_sub(xhi)) / den;
        let adj = if d >= 0 { mag } else { -mag };
        let pred = (out[hi] & 0x7fff).wrapping_add(adj as u32);
        let cur = out[k];
        if cur == 0 {
            out[k] = pred | 0x8000;
        } else {
            let room = (q as u32).wrapping_sub(pred);
            let lim = if (room as i32) < (pred as i32) {
                room
            } else {
                pred
            };
            let step: i32 = if (cur as i32) < (lim as i32).wrapping_mul(2) {
                if cur & 1 == 0 {
                    (cur as i32) >> 1
                } else {
                    -(((cur + 1) as i32) >> 1)
                }
            } else if (pred as i32) < (room as i32) {
                cur.wrapping_sub(pred) as i32
            } else {
                (room.wrapping_sub(cur) as i32) - 1
            };
            out[k] = (step as u32).wrapping_add(pred);
            out[hi] &= 0x7fff;
            out[lo] &= 0x7fff;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests_wave32_agent80 {
    use super::*;

    #[test]
    fn bytes() {
        let mut o = [0u8; 3];
        let mut n = 0;
        w32a80_read_bytes(&mut o, |b| {
            assert_eq!(b, 8);
            n += 1;
            n
        });
        assert_eq!(o, [1, 2, 3]);
    }

    #[test]
    fn header() {
        let mut st = W32a80BlockState::default();
        let src = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut seq = vec![0, 1, 0, 1].into_iter();
        let r = w32a80_block_header(
            &mut st,
            &src,
            3,
            || 0,
            |_, _| {},
            |_| seq.next().unwrap(),
            |_| 1,
        );
        assert_eq!(r, 0);
        assert_eq!((st.f10, st.f7, st.f6, st.f8, st.f11), (1, 1, 0, 1, 4));
        assert_eq!(st.f12_15, [5, 6, 7, 8]);
        let r = w32a80_block_header(&mut st, &src, 3, || 0, |_, _| {}, |_| 1, |_| 0);
        assert_eq!(r, -0x87);
        let r = w32a80_block_header(&mut st, &src, 3, || 5, |_, _| {}, |_| 0, |_| 0);
        assert_eq!(r, 5);
        let r = w32a80_block_header(
            &mut st,
            &src,
            3,
            || 0,
            |_, _| {},
            |b| if b == 1 { 0 } else { -1 },
            |_| 0,
        );
        assert_eq!(r, -0x88);
    }

    #[test]
    fn floor_unused() {
        assert!(w32a80_floor1_decode(&[0; 0x200], &[0; 0x200], |_| 0, |_| 0).is_none());
    }

    #[test]
    fn floor_two_posts() {
        let mut info = vec![0i32; 0x200];
        info[0x100] = 2;
        info[0x102] = 128;
        let fl = vec![0i32; 0x200];
        let mut c = 0;
        let r = w32a80_floor1_decode(
            &info,
            &fl,
            |b| {
                c += 1;
                if b == 1 { 1 } else { 10 + c }
            },
            |_| 0,
        )
        .unwrap();
        assert_eq!(r, vec![12, 13]);
    }
}
