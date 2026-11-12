#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent04.rs
// ===========================================================================
// Wave 28 translation batch 04 (libvpx-style decoder helpers).

/// `0x00a08a00`: once-init gate.  State 0 -> 1 claims and runs `init`, then
/// bumps to 2; other callers spin (calling `yield_now`) until state is 2.
pub fn w28a04_once_gate(
    state: &std::sync::atomic::AtomicI32,
    init: &mut dyn FnMut(),
    yield_now: &mut dyn FnMut(),
) {
    use std::sync::atomic::Ordering::SeqCst;
    if state.compare_exchange(0, 1, SeqCst, SeqCst).is_ok() {
        init();
        state.fetch_add(1, SeqCst);
        return;
    }
    while state.load(SeqCst) != 2 {
        yield_now();
    }
}

/// `0x00a090b0`: aligned calloc geometry.  Returns `(raw_size, aligned_ptr)`
/// for a request of `count * size` bytes: raw block is `count*size + 0xb`,
/// the user pointer is `(raw + 0xb) & !7` with the raw pointer stored at
/// `aligned - 4`.
pub fn w28a04_aligned_calloc_layout(count: u32, size: u32, raw_ptr: u32) -> (u32, u32) {
    (
        count.wrapping_mul(size).wrapping_add(0xb),
        raw_ptr.wrapping_add(0xb) & 0xffff_fff8,
    )
}

/// `0x00a0ab30`: decoder thread-count setup.  Inputs are the configured
/// thread count (`+0x3188`) and the available count (`+0x3000`).  Returns
/// `(multithread_enabled, worker_count)`; workers are `min(min(cfg,8),avail)-1`
/// and multithreading is only enabled when that minimum exceeds 1.
pub fn w28a04_thread_setup(configured: i32, available: i32) -> (bool, i32) {
    let n = configured.min(8);
    let t = if n <= available { n } else { available };
    if t > 1 { (true, t - 1) } else { (false, 0) }
}

/// `0x00a11250`: parses a packed trailer at the end of `data`.  Last byte
/// `b` must satisfy `b & 0xe0 == 0xc0`; count `n = (b&7)+1`, width
/// `w = ((b>>3)&3)+1`; the block is `w*n+2` bytes long, starts with the same
/// byte `b`, and holds `n` little-endian `w`-byte values.  Returns the values
/// or an empty vector when invalid.
pub fn w28a04_parse_trailer(data: &[u8]) -> Vec<u32> {
    let Some(&b) = data.last() else {
        return Vec::new();
    };
    if b & 0xe0 != 0xc0 {
        return Vec::new();
    }
    let n = (b & 7) as usize + 1;
    let w = ((b >> 3) & 3) as usize + 1;
    let total = w * n + 2;
    if total > data.len() {
        return Vec::new();
    }
    let start = data.len() - total;
    if data[start] != b {
        return Vec::new();
    }
    data[start + 1..start + 1 + w * n]
        .chunks(w)
        .map(|c| {
            c.iter()
                .enumerate()
                .fold(0u32, |a, (i, &v)| a | (v as u32) << (8 * i))
        })
        .collect()
}

/// `0x00a11990`: two-tap bilinear 2D filter.  First pass produces `h + 1`
/// rows of `w` u16 values `(s[i]*hf[0] + s[i+1]*hf[1] + 0x40) >> 7`; second
/// pass writes `h` rows `(t[i]*vf[0] + t[i+w]*vf[1] + 0x40) >> 7` as bytes.
#[allow(clippy::too_many_arguments)]
pub fn w28a04_bilinear_2d(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    hf: [i16; 2],
    vf: [i16; 2],
    w: usize,
    h: usize,
) {
    let mut tmp = vec![0u16; w * (h + 1)];
    for r in 0..=h {
        for i in 0..w {
            let a = src[r * src_stride + i] as i32;
            let b = src[r * src_stride + i + 1] as i32;
            tmp[r * w + i] = ((b * hf[1] as i32 + 0x40 + a * hf[0] as i32) >> 7) as u16;
        }
    }
    for r in 0..h {
        for i in 0..w {
            let a = tmp[r * w + i] as i32;
            let b = tmp[(r + 1) * w + i] as i32;
            dst[r * dst_stride + i] = ((vf[0] as i32 * a + 0x40 + b * vf[1] as i32) >> 7) as u8;
        }
    }
}

#[cfg(test)]
mod tests_wave28_agent04 {
    use super::*;
    use std::sync::atomic::{AtomicI32, Ordering};

    #[test]
    fn once_gate() {
        let s = AtomicI32::new(0);
        let mut n = 0;
        w28a04_once_gate(&s, &mut || n += 1, &mut || {});
        assert_eq!((n, s.load(Ordering::SeqCst)), (1, 2));
        w28a04_once_gate(&s, &mut || n += 1, &mut || panic!());
        assert_eq!(n, 1);
    }

    #[test]
    fn layout_and_threads() {
        assert_eq!(w28a04_aligned_calloc_layout(4, 3, 0x1001), (0x17, 0x1008));
        assert_eq!(w28a04_thread_setup(16, 12), (true, 7));
        assert_eq!(w28a04_thread_setup(4, 1), (false, 0));
        assert_eq!(w28a04_thread_setup(2, 8), (true, 1));
    }

    #[test]
    fn trailer() {
        // n=2, w=2 -> b = 0xc0 | (1<<3) | 1 = 0xc9
        let d = [9, 0xc9, 1, 2, 3, 4, 0xc9];
        assert_eq!(w28a04_parse_trailer(&d), vec![0x0201, 0x0403]);
        assert!(w28a04_parse_trailer(&[0xc9]).is_empty());
        assert!(w28a04_parse_trailer(&[0xc8, 1, 0xc9]).is_empty());
        assert!(w28a04_parse_trailer(&[0x10]).is_empty());
    }

    #[test]
    fn bilinear() {
        let src = [10u8; 12];
        let mut dst = [0u8; 4];
        w28a04_bilinear_2d(&src, 4, &mut dst, 2, [128, 0], [128, 0], 2, 2);
        assert_eq!(dst, [10; 4]);
    }
}
