#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent09.rs
// ===========================================================================
// Wave 28 translation batch 09: VP8-style in-loop deblocking filters
// (scalar versions) and a row smoother from the bundled video decoder.
//
// Taps are named `A..H` = indices 0..7; the edge lies between `D` (3) and
// `E` (4).  `blimit`, `limit` and `thresh` are the three bytes the
// pseudocode reads through its `param_3..param_5` pointers.

fn w28a09_clamp(v: i32) -> i32 {
    v.clamp(-0x80, 0x7f)
}

/// Shared mask + normal 4-tap filter; returns `(filter_mask_off, hev)`
/// inputs applied to `px` (indices 1..=6 may change).
fn w28a09_filter_taps(px: &mut [u8; 8], blimit: u8, limit: u8, thresh: u8, wide: bool) {
    let [a, b, c, d, e, f, g, h] = px.map(|v| v as i32);
    let lim = limit as i32;
    let ad_cf = (c - f).abs();
    let ad_cf = if ad_cf < 0 { ad_cf + 1 } else { ad_cf };
    let d_fe = (f - e).abs();
    let d_cd = (c - d).abs();
    let mask_off = ((ad_cf >> 1) + (d - e).abs() * 2 > blimit as i32)
        || (h - g).abs() > lim
        || (g - f).abs() > lim
        || d_fe > lim
        || d_cd > lim
        || (b - c).abs() > lim
        || (a - b).abs() > lim;
    let hev = d_cd > thresh as i32 || d_fe > thresh as i32;
    let flat_off = (h - e).abs() >= 2
        || (g - e).abs() >= 2
        || d_fe >= 2
        || d_cd >= 2
        || (b - d).abs() >= 2
        || (a - d).abs() >= 2;
    if wide && !flat_off && !mask_off {
        let o = [
            (3 * a + e + 2 * b + 4 + d + c) >> 3,
            (b + (a + 2 + c) * 2 + d + e + f) >> 3,
            (b + 2 * d + 4 + c + a + e + g + f) >> 3,
            (h + 2 * e + 4 + b + c + d + g + f) >> 3,
            (c + 4 + (h + f) * 2 + d + e + g) >> 3,
            (3 * h + d + 2 * g + e + 4 + f) >> 3,
        ];
        for (i, v) in o.iter().enumerate() {
            px[i + 1] = *v as u8;
        }
        return;
    }
    let sx = |v: i32| v - 0x80;
    let (ps1, ps0, qs0, qs1) = (sx(c), sx(d), sx(e), sx(f));
    let mut filt = w28a09_clamp(ps1 - qs1);
    if !hev {
        filt = 0;
    }
    filt = w28a09_clamp(filt + (qs0 - ps0) * 3);
    if mask_off {
        filt = 0;
    }
    let f1 = w28a09_clamp(filt + 4) >> 3;
    let f2 = w28a09_clamp(filt + 3) >> 3;
    let nq0 = w28a09_clamp(qs0 - f1);
    let np0 = w28a09_clamp(f2 + ps0);
    let mut f3 = (f1 + 1) >> 1;
    if hev {
        f3 = 0;
    }
    let nq1 = w28a09_clamp(qs1 - f3);
    let np1 = w28a09_clamp(ps1 + f3);
    px[2] = (np1 ^ 0x80) as u8;
    px[3] = (np0 ^ 0x80) as u8;
    px[4] = (nq0 ^ 0x80) as u8;
    px[5] = (nq1 ^ 0x80) as u8;
}

fn w28a09_edge(
    buf: &mut [u8],
    e_idx: usize,
    tap_step: usize,
    row_step: usize,
    t: [u8; 3],
    wide: bool,
) {
    for r in 0..8 {
        let base = e_idx + r * row_step;
        let mut px = [0u8; 8];
        for (k, p) in px.iter_mut().enumerate() {
            *p = buf[base + k * tap_step - 4 * tap_step];
        }
        w28a09_filter_taps(&mut px, t[0], t[1], t[2], wide);
        for (k, p) in px.iter().enumerate() {
            buf[base + k * tap_step - 4 * tap_step] = *p;
        }
    }
}

/// `0x00a4ae00`: loop filter across a horizontal edge, 8 columns; `e_idx` is
/// the first pixel below the edge, `stride` the row pitch.  Uses the wide
/// 7-tap smoother on flat areas, else the normal 4-tap filter.
pub fn w28a09_filter_horiz_edge(buf: &mut [u8], e_idx: usize, stride: usize, t: [u8; 3]) {
    w28a09_edge(buf, e_idx, stride, 1, t, true);
}

/// `0x00a4b5b0`: same filter across a vertical edge, 8 rows; `e_idx` is the
/// first pixel right of the edge.
pub fn w28a09_filter_vert_edge(buf: &mut [u8], e_idx: usize, stride: usize, t: [u8; 3]) {
    w28a09_edge(buf, e_idx, 1, stride, t, true);
}

/// `0x00a4b2e0`: vertical-edge filter, normal 4-tap only (no flat branch).
pub fn w28a09_filter_vert_edge_normal(buf: &mut [u8], e_idx: usize, stride: usize, t: [u8; 3]) {
    w28a09_edge(buf, e_idx, 1, stride, t, false);
}

/// `0x00a4b220`: filters two adjacent 8-wide horizontal edge segments
/// (`e_idx` and `e_idx + 8`) with separate threshold triples.
pub fn w28a09_filter_horiz_edge_16(
    buf: &mut [u8],
    e_idx: usize,
    stride: usize,
    t0: [u8; 3],
    t1: [u8; 3],
) {
    w28a09_filter_horiz_edge(buf, e_idx, stride, t0);
    w28a09_filter_horiz_edge(buf, e_idx + 8, stride, t1);
}

/// `0x00a4b280`: forwards its five arguments plus a trailing `1` to the
/// helper at FUN_00a49ae0 (supplied as a closure).
pub fn w28a09_fwd_a49ae0<T: Copy>(a: [T; 5], mut f: impl FnMut([T; 5], u32)) {
    f(a, 1);
}

/// `0x00a4b2a0`: forwards its five arguments plus `8` to FUN_00a4a370.
pub fn w28a09_fwd_a4a370_8<T: Copy>(a: [T; 5], mut f: impl FnMut([T; 5], u32)) {
    f(a, 8);
}

/// `0x00a4b2c0`: forwards its five arguments plus `0x10` to FUN_00a4a370.
pub fn w28a09_fwd_a4a370_16<T: Copy>(a: [T; 5], mut f: impl FnMut([T; 5], u32)) {
    f(a, 0x10);
}

/// `0x00a4fb30`: horizontal 16-tap smoother over `rows` rows of `cols`
/// pixels (row pitch `stride`, rows start at `start` which must be >= 8).
/// Left edge replicated 8 px, right edge 17 px; where local variance
/// (`15*sumsq - sum^2`) is below `flimit` the pixel becomes the rounded
/// window mean.
pub fn w28a09_smooth_rows(
    buf: &mut [u8],
    start: usize,
    stride: usize,
    rows: i32,
    cols: i32,
    flimit: i32,
) {
    for row in 0..rows.max(0) as usize {
        let p = start + row * stride;
        let first = buf[p];
        for k in 0..8 {
            buf[p - 8 + k] = first;
        }
        let last = buf[(p as i64 + cols as i64 - 1) as usize];
        for k in 0..0x11i64 {
            buf[(p as i64 + cols as i64 + k) as usize] = last;
        }
        let mut sum: i32 = 0;
        let mut sq: i32 = 0;
        let mut win = [0u8; 16];
        for i in 0..15 {
            let v = buf[p - 8 + i] as i32;
            sum += v;
            sq = sq.wrapping_add(v * v);
        }
        if cols > -8 {
            for u in 0..(cols + 8) as usize {
                let hi = buf[p + u + 7] as i32;
                let lo = buf[p + u - 8] as i32;
                let d = hi - lo;
                sum += d;
                sq = sq.wrapping_add((hi + lo) * d);
                let cur = buf[p + u];
                win[u & 15] = cur;
                if sq.wrapping_mul(15).wrapping_sub(sum.wrapping_mul(sum)) < flimit {
                    win[u & 15] = ((cur as i32 + 8 + sum) >> 4) as u8;
                }
                buf[p + u - 8] = win[(u + 8) & 15];
            }
        }
    }
}

#[cfg(test)]
mod tests_wave28_agent09 {
    use super::*;

    #[test]
    fn flat_region_unchanged() {
        let mut b = vec![100u8; 8 * 8];
        let o = b.clone();
        w28a09_filter_vert_edge(&mut b, 4, 8, [20, 10, 4]);
        assert_eq!(b, o);
        w28a09_filter_vert_edge_normal(&mut b, 4, 8, [20, 10, 4]);
        assert_eq!(b, o);
    }

    #[test]
    fn step_edge_is_smoothed() {
        let mut b = vec![0u8; 64];
        for r in 0..8 {
            for c in 0..8 {
                b[r * 8 + c] = if c < 4 { 100 } else { 104 };
            }
        }
        w28a09_filter_vert_edge(&mut b, 4, 8, [40, 20, 10]);
        // flat step: wide filter smooths monotonically
        assert!(b[3] > 100 && b[4] < 104);
        assert_eq!(b[0], 100);
        assert_eq!(b[7], 104);
    }

    #[test]
    fn horiz_matches_vert_transposed() {
        let src = [90u8, 92, 95, 99, 130, 128, 127, 125];
        let mut v = vec![0u8; 64];
        let mut h = vec![0u8; 64];
        for r in 0..8 {
            for c in 0..8 {
                v[r * 8 + c] = src[c];
                h[c * 8 + r] = src[c];
            }
        }
        let t = [30, 8, 3];
        w28a09_filter_vert_edge_normal(&mut v, 4, 8, t);
        w28a09_filter_horiz_edge(&mut h, 4 * 8, 8, t);
        let mut vt = vec![0u8; 64];
        for r in 0..8 {
            for c in 0..8 {
                vt[c * 8 + r] = v[r * 8 + c];
            }
        }
        // strong edge (31 > blimit/limit): filtered off, equal in both
        assert_eq!(vt, h);
    }

    #[test]
    fn two_segments_use_own_thresholds() {
        let mut b = vec![100u8; 16 * 8];
        for r in 4..8 {
            for c in 0..16 {
                b[r * 16 + c] = 108;
            }
        }
        w28a09_filter_horiz_edge_16(&mut b, 4 * 16, 16, [60, 20, 10], [0, 0, 0]);
        assert_ne!(b[3 * 16], 100);
        assert_eq!(b[3 * 16 + 8], 100);
    }

    #[test]
    fn forwarders() {
        let mut got = vec![];
        w28a09_fwd_a49ae0([1, 2, 3, 4, 5], |a, k| got.push((a, k)));
        w28a09_fwd_a4a370_8([1, 2, 3, 4, 5], |a, k| got.push((a, k)));
        w28a09_fwd_a4a370_16([1, 2, 3, 4, 5], |a, k| got.push((a, k)));
        assert_eq!(got.iter().map(|g| g.1).collect::<Vec<_>>(), [1, 8, 16]);
    }

    #[test]
    fn smooth_rows_flat_and_noisy() {
        let mut b = vec![50u8; 8 + 16 + 17 + 8];
        w28a09_smooth_rows(&mut b, 8, 0, 1, 16, 1 << 20);
        assert!(b[8..24].iter().all(|&v| v == 50));
        let mut n = vec![0u8; 8 + 16 + 17 + 8];
        for (i, v) in n.iter_mut().enumerate().skip(8) {
            *v = if i % 2 == 0 { 10 } else { 200 };
        }
        w28a09_smooth_rows(&mut n, 8, 0, 1, 16, 0);
        // limit 0: no smoothing; output lags 8 px behind (ring starts zeroed)
        assert!(n[..8].iter().all(|&v| v == 0));
        assert_eq!(n[8], 10);
        assert_eq!(n[9], 200);
    }
}
