#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent110.rs
// ===========================================================================
// Wave 31 translation batch 110: D3D device wrapper argument builders.
//
// Skipped (no pure logic or non-returning tails): FUN_00c4f7e0, FUN_00c4f970,
// FUN_00c4fb00, FUN_00c4fe50, FUN_00c50280, FUN_00c50440, FUN_00c506a0,
// FUN_00c50e10, FUN_00c51d90, FUN_00c52ce0.

/// `0x00c53470` and `0x00c534d0`: converts `(x, y, w, h)` into the
/// `(left, top, right, bottom)` rectangle passed to the device vtable slot
/// `+0x58`.  The two routines differ only in which device pointer is used
/// (`+0xc` for `0x00c53470`, `+0x18` for `0x00c534d0`).
pub fn w31a110_rect_from_xywh(src: [i32; 4]) -> [i32; 4] {
    [
        src[0],
        src[1],
        src[2].wrapping_add(src[0]),
        src[3].wrapping_add(src[1]),
    ]
}

/// Draw-command record built by `0x00c52be0` and `0x00c52c60`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W31a110Cmd {
    pub offset: u64,
    pub arg: u32,
    pub kind: u32,
}

/// `0x00c52be0` / `0x00c52c60`: record for vtable slot `+0xac`; the offset is
/// `param_2` plus the 64-bit base at `+8`, and the kind is `0x2a` when the
/// flag is set, else `0x39`.
pub fn w31a110_build_cmd(base: u64, param2: u32, arg: u32, flag: bool) -> W31a110Cmd {
    W31a110Cmd {
        offset: base.wrapping_add(param2 as u64),
        arg,
        kind: if flag { 0x2a } else { 0x39 },
    }
}

/// 24-byte record emitted by `0x00c53570` and `0x00c53650`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W31a110Rec24 {
    pub offset: u64,
    pub zero: u32,
    pub stride: u32,
    pub dims: [u32; 2],
}

/// `0x00c53570` / `0x00c53650`: for each slot builds a record.  A null buffer
/// (`None`) yields offset 0; otherwise offset is `offsets[i]` plus the
/// buffer's 64-bit base.  `dims` is the pair read at `+0x598/+0x59c` of the
/// device object (`+4` in the first routine, `+0x10` in the second).
pub fn w31a110_build_recs24(
    bases: &[Option<u64>],
    offsets: &[u32],
    strides: &[u32],
    dims: [u32; 2],
) -> Vec<W31a110Rec24> {
    bases
        .iter()
        .zip(offsets)
        .zip(strides)
        .map(|((b, &o), &s)| W31a110Rec24 {
            offset: b.map_or(0, |b| b.wrapping_add(o as u64)),
            zero: 0,
            stride: s,
            dims,
        })
        .collect()
}

/// `0x00c53730`: 16-byte records `(offset_lo, offset_hi, stride, first_word)`
/// where `first_word` is the first dword of the object `refs[i]` points to.
pub fn w31a110_build_recs16(
    bases: &[u64],
    offsets: &[u32],
    strides: &[u32],
    first_words: &[u32],
) -> Vec<[u32; 4]> {
    let mut out = Vec::new();
    for i in 0..bases
        .len()
        .min(offsets.len())
        .min(strides.len())
        .min(first_words.len())
    {
        let v = bases[i].wrapping_add(offsets[i] as u64);
        out.push([v as u32, (v >> 32) as u32, strides[i], first_words[i]]);
    }
    out
}

#[cfg(test)]
mod tests_wave31_agent110 {
    use super::*;

    #[test]
    fn rect() {
        assert_eq!(w31a110_rect_from_xywh([1, 2, 3, 4]), [1, 2, 4, 6]);
    }

    #[test]
    fn cmd() {
        let c = w31a110_build_cmd(0xffff_fff0, 0x20, 7, true);
        assert_eq!(c.offset, 0x1_0000_0010);
        assert_eq!(c.kind, 0x2a);
        assert_eq!(w31a110_build_cmd(0, 0, 0, false).kind, 0x39);
    }

    #[test]
    fn recs() {
        let r = w31a110_build_recs24(&[None, Some(10)], &[5, 5], &[8, 16], [640, 480]);
        assert_eq!(r[0].offset, 0);
        assert_eq!(r[1].offset, 15);
        assert_eq!(r[1].dims, [640, 480]);
        let q = w31a110_build_recs16(&[0xffff_ffff], &[1], &[4], &[9]);
        assert_eq!(q[0], [0, 1, 4, 9]);
    }
}
