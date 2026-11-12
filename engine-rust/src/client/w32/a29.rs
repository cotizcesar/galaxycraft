#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent29.rs
// ===========================================================================
// Wave 32 translation batch 29.

/// `0x00a5fee0`: recursive tree blend.  `tree` holds signed node bytes at
/// `[node + base]` (<= 0 means leaf, `counts[-v]` is the leaf weight; > 0 is a
/// child node index).  The blended byte is written to `out[(node >> 1)]` and
/// the combined weight is returned.  `blend` is the 21-entry table at the
/// retail blend-weight address; `src` is the source byte plane.
pub fn w32a29_tree_blend(
    node: u32,
    tree: &[u8],
    src: &[u8],
    counts: &[u32],
    blend: &[u32; 21],
    out: &mut [u8],
) -> u32 {
    let side = |n: u32, out: &mut [u8]| -> u32 {
        let v = tree[n as usize] as i8 as i32;
        if v < 1 {
            counts[(-v) as usize]
        } else {
            w32a29_tree_blend(v as u32, tree, src, counts, blend, out)
        }
    };
    let a = side(node, out);
    let b = side(node + 1, out);
    let total = b.wrapping_add(a);
    let mut byte = src[(node >> 1) as usize];
    if total != 0 {
        let idx = total.min(0x14) as usize;
        let w = blend[idx];
        let q = ((a as u64) * 0x100 + (total >> 1) as u64) / total as u64;
        let q = q as u32;
        let v = if (q as i32) < 0x100 {
            if (q as i32) < 1 { 1 } else { q & 0xff }
        } else {
            0xff
        };
        byte = ((v * w + (0x100u32.wrapping_sub(w)) * byte as u32 + 0x80) >> 8) as u8;
    }
    out[(node >> 1) as usize] = byte;
    total
}

#[cfg(test)]
mod tests_wave32_agent29 {
    use super::*;
    #[test]
    fn blend_leaf() {
        let tree = [0u8, 0xff];
        let counts = [3u32, 1];
        let blend = [0x80u32; 21];
        let src = [100u8];
        let mut out = [0u8];
        let t = w32a29_tree_blend(0, &tree, &src, &counts, &blend, &mut out);
        assert_eq!(t, 4);
        // q = (3*256+2)/4 = 192; (192*128+128*100+128)>>8 = 146
        assert_eq!(out[0], 146);
    }
}
