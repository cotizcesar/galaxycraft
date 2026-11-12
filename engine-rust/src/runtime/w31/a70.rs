#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent70.rs
// ===========================================================================
// Wave 31 batch 70: small runtime helpers.

/// `0x00aa2150`: maps a kind id to the palette index passed to both draw
/// calls (default 0xf).
pub fn w31a70_kind_color(kind: u32) -> u32 {
    match kind {
        0x13 | 0x15 => 0xc,
        0x12 => 10,
        0x14 => 9,
        0x28 => 7,
        0x29 => 8,
        _ => 0xf,
    }
}

/// `0x00aa4fd0`: packs a 32-bit key.  `kind` is `[+0x28]`, `mode` is
/// `[+0x24]`, `flags` is `[+0x3e]`, `low6` is the field at `+0x18` of the
/// record returned by the kind lookup.  `entries` holds, per entry of the
/// optional list, whether its tag byte at `+0x10` is `'2'`.  The pseudocode
/// then hands the result to a sink, so the key is returned.
pub fn w31a70_pack_key(
    kind: u32,
    mode: u32,
    flags: u16,
    low6: u32,
    entries: Option<&[bool]>,
) -> u32 {
    let hi = match kind {
        0x13 | 0x15 => 0x30000,
        0x12 => 0x28000,
        0x14 => 0x24000,
        0x28 => 0x18000,
        0x29 => 0x20000,
        _ => 0x3c000,
    };
    let sel: u32 = match mode {
        1 | 5 => 3,
        3 => 1,
        _ => 4,
    };
    let mut key =
        sel << 0x12 | ((((flags as u32) & 3) << 6 | (kind & 0x3f)) << 6) | (low6 & 0x3f) | hi;
    match entries {
        None => key |= 0x400000,
        Some(list) => {
            let mut bit: u32 = 1;
            for (i, &is_two) in list.iter().enumerate() {
                key ^= ((bit << 0x16 | key) ^ key) & 0xfc00000;
                if is_two {
                    key = (key & 0xfffffff) | ((i as u32) << 0x1d) | 0x10000000;
                }
                bit = bit.rotate_left(1);
            }
        }
    }
    key
}

/// `0x00aa95b0`: builds the 8-dword record: `[0..6]` slots, `[6]` count,
/// byte flag at dword 7.
pub fn w31a70_slot_record(kind: u32) -> [u32; 8] {
    let mut r = [0u32; 8];
    fn push(r: &mut [u32; 8], v: u32) {
        r[6] += 1;
        r[r[6] as usize] = v;
    }
    match kind {
        4 | 0x20 | 0x22 | 0x24 | 0x2a => r[6] += 1,
        0x10 => {
            r[6] += 1;
            push(&mut r, 1);
            r[6] += 1;
        }
        0x12..=0x14 => {
            r[0] = 1;
            r[6] += 1;
        }
        0x15 => {
            r[0] = 4;
            r[6] += 1;
        }
        0x21 | 0x23 => {
            if kind == 0x23 {
                r[6] += 1;
                r[r[6] as usize] = 2;
            }
            push(&mut r, 3);
            push(&mut r, 4);
            r[6] += 1;
        }
        0x25 => {
            r[0] = 2;
            r[6] += 1;
        }
        0x27 => {
            r[6] += 1;
            r[r[6] as usize] = 5;
            r[6] += 1;
        }
        _ => r[7] = 1,
    }
    r
}

/// Bit reader state for `0x00aad220` (`[0]` pos, `[1]` bit offset, `[3]`
/// cursor, `[4]` length).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W31a70BitReader {
    pub pos: i32,
    pub bit: i32,
    pub cursor: usize,
    pub len: i32,
}

/// `0x00aad220`: reads `count` (<= 32) bits LSB-first from `data`, using
/// the retail mask table (`masks[count]`).  Returns `0xffffffff` and
/// exhausts the reader on overflow.
pub fn w31a70_read_bits(
    rd: &mut W31a70BitReader,
    data: &[u8],
    masks: &[u32; 33],
    count: u32,
) -> u32 {
    let fail = |rd: &mut W31a70BitReader| {
        rd.pos = rd.len;
        rd.cursor = 0;
        rd.bit = 1;
        0xffff_ffffu32
    };
    if count >= 0x21 {
        return fail(rd);
    }
    let mask = masks[count as usize];
    let total = count as i32 + rd.bit;
    if rd.len - 4 <= rd.pos {
        if rd.len - ((total + 7) >> 3) < rd.pos {
            return fail(rd);
        }
        if total == 0 {
            return 0;
        }
    }
    let b = |i: usize| -> u32 { data.get(rd.cursor + i).copied().unwrap_or(0) as u32 };
    let sh = rd.bit as u32;
    let mut v = b(0) >> sh;
    if total > 8 {
        v |= b(1) << (8 - sh);
        if total > 0x10 {
            v |= b(2) << (0x10 - sh);
            if total > 0x18 {
                v |= b(3) << (0x18 - sh);
                if total > 0x20 && rd.bit != 0 {
                    v |= b(4) << (0x20 - sh);
                }
            }
        }
    }
    rd.bit = total & 7;
    let adv = (total + ((total >> 31) & 7)) >> 3;
    rd.cursor += adv as usize;
    rd.pos += adv;
    v & mask
}

/// `0x00ac0630`: formats the 6-byte "xx/xx/" prefix.  `src[0]` is the
/// unit count; `convert(src_tail, n, dst)` is the unresolved converter
/// that writes into `dst` and returns the length.  Returns `src[0]*2+6`.
pub fn w31a70_format_pair(
    dst: &mut [u8],
    cap: u32,
    src: &[u8],
    convert: impl FnOnce(&[u8], u32, &mut [u8]) -> usize,
) -> u32 {
    let n = src[0] as u32;
    let ret = n * 2 + 6;
    if cap < 6 {
        if cap != 0 {
            dst[0] = 0;
        }
        return ret;
    }
    let mut take = n * 2;
    if cap.wrapping_sub(7) < take {
        take = cap - 7;
    }
    let len = convert(&src[1..], take >> 1, &mut dst[6..]);
    dst[6 + len] = 0;
    let (a, b) = if n == 0 {
        (0x30, 0x30)
    } else {
        (dst[6], dst[7])
    };
    dst[0] = a;
    dst[1] = b;
    dst[2] = 0x2f;
    let (c, d) = if n < 2 {
        (0x30, 0x30)
    } else {
        (dst[8], dst[9])
    };
    dst[3] = c;
    dst[4] = d;
    dst[5] = 0x2f;
    ret
}

#[cfg(test)]
mod tests_wave31_agent70 {
    use super::*;

    #[test]
    fn color() {
        assert_eq!(w31a70_kind_color(0x15), 0xc);
        assert_eq!(w31a70_kind_color(0x12), 10);
        assert_eq!(w31a70_kind_color(0), 0xf);
        assert_eq!(w31a70_kind_color(0x29), 8);
    }

    #[test]
    fn pack() {
        let k = w31a70_pack_key(0x12, 3, 0, 5, None);
        assert_eq!(k, (1 << 0x12) | (0x12 << 6) | 5 | 0x28000 | 0x400000);
        let k = w31a70_pack_key(0x12, 0, 0, 0, Some(&[false, true]));
        assert_eq!(k >> 28, 0x1 | (1 << 1));
        assert_eq!(k & 0x10000000, 0x10000000);
    }

    #[test]
    fn record() {
        assert_eq!(w31a70_slot_record(0x23), [0, 2, 3, 4, 0, 0, 4, 0]);
        assert_eq!(w31a70_slot_record(0x21)[..4], [0, 3, 4, 0]);
        assert_eq!(w31a70_slot_record(0x99)[7], 1);
        assert_eq!(w31a70_slot_record(0x15)[0], 4);
        assert_eq!(w31a70_slot_record(0x10)[2], 1);
    }

    #[test]
    fn bits() {
        let mut masks = [0u32; 33];
        for (i, m) in masks.iter_mut().enumerate() {
            *m = if i == 32 { u32::MAX } else { (1u32 << i) - 1 };
        }
        let data = [0xABu8, 0xCD, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut rd = W31a70BitReader {
            pos: 0,
            bit: 0,
            cursor: 0,
            len: 10,
        };
        assert_eq!(w31a70_read_bits(&mut rd, &data, &masks, 4), 0xB);
        assert_eq!(rd.bit, 4);
        assert_eq!(w31a70_read_bits(&mut rd, &data, &masks, 8), 0xDA);
        assert_eq!(w31a70_read_bits(&mut rd, &data, &masks, 33), 0xffff_ffff);
        assert_eq!(rd.pos, 10);
    }

    #[test]
    fn pair() {
        let mut dst = [0u8; 16];
        let r = w31a70_format_pair(&mut dst, 16, &[2, b'1', b'2', b'3', b'4'], |s, n, d| {
            d[..n as usize].copy_from_slice(&s[..n as usize]);
            n as usize
        });
        assert_eq!(r, 10);
        assert_eq!(&dst[..6], &[b'1', b'2', 0x2f, 0, 0, 0x2f]);
        let mut d2 = [9u8; 4];
        assert_eq!(w31a70_format_pair(&mut d2, 3, &[1, 0], |_, _, _| 0), 8);
        assert_eq!(d2[0], 0);
    }
}
