#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent17.rs
// ===========================================================================
// Wave 23 agent 17: minimap/terrain-cache helpers (runtime.rs target).

/// `0x0071b490`: returns the pixel-buffer base for plane `plane`: the shared
/// buffer when present, otherwise the per-plane base stored at `+8 + plane*0xc`.
pub fn w23a17_plane_base(shared: Option<u32>, plane_bases: &[u32], plane: usize) -> u32 {
    match shared {
        Some(v) => v,
        None => plane_bases[plane],
    }
}

/// `0x0071b4d0`: reads the u16 at `0x14 + ((tile >> 4 & 0x7ff) * 0x1a +
/// (tile & 0xf)) * 2` from `table` (megatile-group lookup).
pub fn w23a17_tile_lookup(table: &[u8], tile: u16) -> u16 {
    let idx = ((tile >> 4 & 0x7ff) as usize * 0x1a + (tile & 0xf) as usize) * 2 + 0x14;
    u16::from_le_bytes([table[idx], table[idx + 1]])
}

/// `0x0071b500`: address of 4-byte cell `(x, y)` in a row-major grid:
/// `base + (y * stride + x) * 4` (wrapping 32-bit).
pub fn w23a17_cell_addr(base: u32, x: i32, y: i32, stride: i32) -> u32 {
    base.wrapping_add((y.wrapping_mul(stride).wrapping_add(x) as u32).wrapping_mul(4))
}

/// `0x0071b9c0`: converts `w*h` u16 tile words to bytes: for each word
/// `v`, `out = tab3[ remap[(grid[(v & 0x7fff) * 0x10] & !1) * 0x20 + 0x37] ]`
/// where the callers' tables are given as slices (`grid` indexed in u32 words,
/// `remap` indexed as bytes, `tab3` as bytes).
pub fn w23a17_convert_tiles(
    out: &mut [u8],
    words: &[u16],
    w: u16,
    h: u16,
    grid: &[u32],
    remap: &[u8],
    tab3: &[u8],
) {
    let n = w as usize * h as usize;
    for i in 0..n {
        let v = words[i] as usize;
        let g = (grid[(v & 0x7fff) * 0x10] & 0xffff_fffe) as usize;
        out[i] = tab3[remap[g * 0x20 + 0x37] as usize];
    }
}

/// Handler chosen by `0x00719150`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W23a17Handler {
    /// kind 2 (`0x71_8ee0`)
    Kind2,
    /// kind 3 (`0x00719730`)
    Kind3,
    /// kind 4 (`0x71_7670`)
    Kind4,
    /// any other kind: default callback
    Default,
}

/// `0x00719150`: selects the callback by the short at `+0x3a`.
pub fn w23a17_select_handler(kind: i16) -> W23a17Handler {
    match kind {
        2 => W23a17Handler::Kind2,
        3 => W23a17Handler::Kind3,
        4 => W23a17Handler::Kind4,
        _ => W23a17Handler::Default,
    }
}

/// `0x0071a590`: accumulates weighted colour channels for samples
/// `start..end`.  `weights[i]` is the weight, `sample(i)` the index byte,
/// `remap` the 256-byte table at `+0x418`, `palette` the u32 table at word 6.
/// `acc = [r, g, b, weight_sum]`.
pub fn w23a17_accumulate(
    acc: &mut [i32; 4],
    weights: &[i32],
    sample: impl Fn(usize) -> u8,
    remap: &[u8],
    palette: &[u32],
    start: usize,
    end: usize,
) {
    for i in start..end {
        let wt = weights[i];
        let c = palette[remap[sample(i) as usize] as usize];
        acc[0] = acc[0].wrapping_add(((c & 0xff) as i32).wrapping_mul(wt));
        acc[1] = acc[1].wrapping_add(((c >> 8 & 0xff) as i32).wrapping_mul(wt));
        acc[2] = acc[2].wrapping_add(((c >> 16 & 0xff) as i32).wrapping_mul(wt));
        acc[3] = acc[3].wrapping_add(wt);
    }
}

/// `0x0071a910`: sets the first `w0*h0` words of the plane-2 buffer to opaque
/// black (`0xff000000`) and zeroes the `w1*h1` words of the main buffer.
pub fn w23a17_clear_buffers(plane2: &mut [u32], count: usize, main: &mut [u32], main_count: usize) {
    for v in plane2.iter_mut().take(count) {
        *v = 0xff00_0000;
    }
    for v in main.iter_mut().take(main_count) {
        *v = 0;
    }
}

/// Result of `0x007177c0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W23a17Scale {
    pub width: u16,
    pub height: u16,
    pub field_114: u32,
    pub field_11c: u32,
    pub field_120: u32,
    /// Which blitter pair: 0 = none/guard, 1 = 0x40 case, 2 = 0x60/0x80, 3 = 0xc0/0x100.
    pub blitters: u8,
    /// Rect adjustments `(d4, d8, d6, d10)` as `(+x0, -x1, +y0, -y1)` offsets.
    pub rect_delta: (i16, i16, i16, i16),
}

/// `0x007177c0`: derives the minimap scale configuration from the source
/// size `(w, h)` (u16 at `+0xe4`/`+0xe6`).
pub fn w23a17_scale_config(w: u16, h: u16) -> W23a17Scale {
    let mut cw = w;
    let ch: u16;
    let (f114, f11c, f120, blit);
    match w.max(h) {
        0x40 => {
            cw = w.wrapping_mul(2);
            ch = h.wrapping_mul(2);
            (f114, f11c, f120, blit) = (1, 1, 0x10, 1);
        }
        0x60 | 0x80 => {
            ch = h;
            (f114, f11c, f120, blit) = (1, 0, 0x20, 2);
        }
        0xc0 | 0x100 => {
            cw = w >> 1;
            ch = h >> 1;
            (f114, f11c, f120, blit) = (2, 0, 0x40, 3);
        }
        _ => {
            if cw <= h {
                cw = h;
            }
            if cw < 0x40 {
                (f114, f11c, f120) = (1, 1, 0x10);
                cw = 0x40;
            } else {
                if cw < 0x80 {
                    (f114, f120) = (1, 0x20);
                } else {
                    (f114, f120) = (2, 0x40);
                }
                f11c = 0;
                cw = 0x80;
            }
            ch = cw;
            blit = 0;
        }
    }
    let dx = ((0x80 - cw as i32) / 2) as i16;
    let dy = ((0x80 - ch as i32) / 2) as i16;
    W23a17Scale {
        width: cw,
        height: ch,
        field_114: f114,
        field_11c: f11c,
        field_120: f120,
        blitters: blit,
        rect_delta: (dx, dx.wrapping_neg(), dy, dy.wrapping_neg()),
    }
}

#[cfg(test)]
mod tests_wave23_agent17 {
    use super::*;

    #[test]
    fn small_helpers() {
        assert_eq!(w23a17_plane_base(Some(7), &[1, 2], 1), 7);
        assert_eq!(w23a17_plane_base(None, &[1, 2], 1), 2);
        assert_eq!(w23a17_cell_addr(100, 3, 2, 10), 100 + 23 * 4);
        let mut t = vec![0u8; 0x14 + 0x1a * 2 * 4];
        let idx = 0x14 + (0x1a + 2) * 2;
        t[idx] = 0x34;
        t[idx + 1] = 0x12;
        assert_eq!(w23a17_tile_lookup(&t, 0x0012), 0x1234);
    }

    #[test]
    fn handler() {
        assert_eq!(w23a17_select_handler(3), W23a17Handler::Kind3);
        assert_eq!(w23a17_select_handler(2), W23a17Handler::Kind2);
        assert_eq!(w23a17_select_handler(4), W23a17Handler::Kind4);
        assert_eq!(w23a17_select_handler(9), W23a17Handler::Default);
    }

    #[test]
    fn convert() {
        let grid = vec![0u32; 0x10 * 2];
        let mut g = grid.clone();
        g[0x10] = 3; // & !1 = 2
        let mut remap = vec![0u8; 2 * 0x20 + 0x38];
        remap[2 * 0x20 + 0x37] = 5;
        let mut tab = vec![0u8; 8];
        tab[5] = 99;
        let mut out = [0u8; 2];
        w23a17_convert_tiles(&mut out, &[1, 0], 2, 1, &g, &remap, &tab);
        assert_eq!(out, [99, 0]);
    }

    #[test]
    fn accumulate() {
        let mut acc = [0; 4];
        let remap: Vec<u8> = (0..=255).collect();
        let mut pal = vec![0u32; 256];
        pal[1] = 0x0030_2010;
        w23a17_accumulate(&mut acc, &[2, 3], |_| 1, &remap, &pal, 0, 2);
        assert_eq!(acc, [0x10 * 5, 0x20 * 5, 0x30 * 5, 5]);
        w23a17_accumulate(&mut acc, &[2, 3], |_| 1, &remap, &pal, 1, 1);
        assert_eq!(acc[3], 5);
    }

    #[test]
    fn clear() {
        let mut a = [1u32; 4];
        let mut b = [1u32; 4];
        w23a17_clear_buffers(&mut a, 3, &mut b, 2);
        assert_eq!(a, [0xff00_0000, 0xff00_0000, 0xff00_0000, 1]);
        assert_eq!(b, [0, 0, 1, 1]);
    }

    #[test]
    fn scale() {
        let s = w23a17_scale_config(0x40, 0x20);
        assert_eq!(
            (s.width, s.height, s.field_120, s.blitters),
            (0x80, 0x40, 0x10, 1)
        );
        assert_eq!(s.rect_delta, (0, 0, 0x20, -0x20));
        let s = w23a17_scale_config(0x100, 0x80);
        assert_eq!((s.width, s.height, s.field_114), (0x80, 0x40, 2));
        let s = w23a17_scale_config(0x80, 0x60);
        assert_eq!((s.width, s.height, s.field_120), (0x80, 0x60, 0x20));
        let s = w23a17_scale_config(0x30, 0x20);
        assert_eq!(
            (s.width, s.height, s.field_11c, s.blitters),
            (0x40, 0x40, 1, 0)
        );
        let s = w23a17_scale_config(0x50, 0x20);
        assert_eq!((s.width, s.field_114, s.field_120), (0x80, 1, 0x20));
        let s = w23a17_scale_config(0x90, 0x20);
        assert_eq!((s.width, s.field_114, s.field_120), (0x80, 2, 0x40));
    }
}
