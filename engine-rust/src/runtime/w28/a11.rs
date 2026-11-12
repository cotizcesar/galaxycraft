#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent11.rs
// ===========================================================================
// Wave 28 translation batch 11: image-codec border/probability helpers.

/// `0x00a56bb0`: fills `width + 5` bytes with `0x7f`, starting one byte
/// before `plane_origin - stride` (the top border row of a plane).
pub fn w28a11_fill_top_border(buf: &mut [u8], plane_origin: usize, stride: usize, width: usize) {
    let start = plane_origin.wrapping_sub(stride).wrapping_sub(1);
    let end = start.saturating_add(width + 5).min(buf.len());
    if start < end {
        buf[start..end].fill(0x7f);
    }
}

/// `0x00a58a70`: zeroes the 0x60 bytes starting at offset 0x10 of a state block.
pub fn w28a11_clear_state_block(block: &mut [u8]) {
    let end = 0x70.min(block.len());
    if block.len() > 0x10 {
        block[0x10..end].fill(0);
    }
}

/// `0x00a5aaa0`: when `enabled`, zeroes `((bits + 7) >> 3) * rows * 0xa0`
/// bytes of the mask buffer (clamped to the buffer length).
pub fn w28a11_clear_mask_buffer(mask: &mut [u8], enabled: bool, bits: i32, rows: i32) {
    if !enabled {
        return;
    }
    let n = (((bits + 7) >> 3) as i64 * rows as i64 * 0xa0).max(0) as usize;
    let n = n.min(mask.len());
    mask[..n].fill(0);
}

/// `0x00a56de0`: per-probability blend step.  `c0`/`c1` are the two counts,
/// `old` the previous probability, `limit` (nonzero) the saturation count and
/// `scale` the blend numerator.  Returns the new 8-bit probability.
pub fn w28a11_blend_prob(c0: u32, c1: u32, old: u8, limit: u32, scale: u32) -> u8 {
    let total = c0.wrapping_add(c1);
    let prob: u32 = if total == 0 {
        0x80
    } else {
        let q = ((c0 as u64 * 0x100 + (total >> 1) as u64) / total as u64) as u32 as i32;
        if q < 0x100 {
            (q.max(1) as u32) & 0xff
        } else {
            0xff
        }
    };
    let w = (total.min(limit) as u64 * scale as u64 / limit as u64) as u32;
    ((prob * w + 0x80 + old as u32 * 0x100u32.wrapping_sub(w)) >> 8) as u8
}

#[cfg(test)]
mod tests_wave28_agent11 {
    use super::*;

    #[test]
    fn border_fill() {
        let mut b = vec![0u8; 32];
        w28a11_fill_top_border(&mut b, 20, 10, 3);
        assert_eq!(&b[9..17], &[0x7f; 8]);
        assert_eq!(b[8], 0);
        assert_eq!(b[17], 0);
    }

    #[test]
    fn clear_state() {
        let mut b = vec![1u8; 0x80];
        w28a11_clear_state_block(&mut b);
        assert_eq!(b[0xf], 1);
        assert!(b[0x10..0x70].iter().all(|&x| x == 0));
        assert_eq!(b[0x70], 1);
    }

    #[test]
    fn clear_mask() {
        let mut m = vec![1u8; 0x400];
        w28a11_clear_mask_buffer(&mut m, false, 8, 1);
        assert_eq!(m[0], 1);
        w28a11_clear_mask_buffer(&mut m, true, 8, 2);
        assert!(m[..0x140].iter().all(|&x| x == 0));
        assert_eq!(m[0x140], 1);
    }

    #[test]
    fn blend() {
        assert_eq!(
            w28a11_blend_prob(0, 0, 0x40, 20, 10),
            ((0x80u32 * 0 + 0x80 + 0x40 * 0x100) >> 8) as u8
        );
        // all counts on c0: prob 0xff, full weight 0x100 -> 0xff
        assert_eq!(w28a11_blend_prob(30, 0, 0x10, 20, 0x100), 0xff);
        // all on c1: prob clamps to 1
        assert_eq!(w28a11_blend_prob(0, 30, 0x10, 20, 0x100), 1);
    }
}
