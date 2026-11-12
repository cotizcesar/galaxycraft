#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave17/agent04.rs
// ===========================================================================
// Wave 17 translation batch 04.
//
// Window: queue: 20 addresses, module `runtime.rs`.

/// Ordered external call recorded by the 0x005696f0-family translations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W17a04Call {
    /// FUN_0056d080 (no arguments).
    Refresh,
    /// FUN_0057d5d0(byte).
    Apply(u8),
    /// FUN_0057d730(x, y).
    SetPos(u16, u16),
    /// FUN_0058b460(kind, flag).
    Notify(u32, u32),
}

/// Globals shared by the 0x00569xxx selection-state helpers.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct W17a04Globals {
    pub g98: u16,
    pub g9c: u16,
    pub ga0: u16,
    pub ga4: u16,
    pub ga8: u32,
    pub gac: u32,
    pub b10: u8,
    pub b11: u8,
    pub b13: u8,
}

fn w17a04_u16(t: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([t[o], t[o + 1]])
}
fn w17a04_put16(t: &mut [u8], o: usize, v: u16) {
    t[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w17a04_put32(t: &mut [u8], o: usize, v: u32) {
    t[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

fn w17a04_tail(g: &W17a04Globals, t: &[u8], out: &mut Vec<W17a04Call>) {
    out.push(W17a04Call::Refresh);
    out.push(W17a04Call::Apply(t[0x21]));
    if g.b11 != 0 {
        out.push(W17a04Call::Notify(0xc, 0));
    } else if g.b10 != 0 {
        out.push(W17a04Call::Notify(0xb, 1));
    }
}

/// `0x004c4640`: SSE2 row blend.  Per byte: `((d -sat a) +sat (a +sat b)) -sat b`
/// plus (wrapping) a source byte read at `rnd & 0xff` + column.  Runs
/// `max(rows, 1)` rows of `width` bytes rounded up to 16, advancing `dst` by
/// `stride`; `rnd` is the per-row random source (FUN_00c9334c).
#[allow(clippy::too_many_arguments)]
pub fn w17a04_blend_rows(
    dst: &mut [u8],
    src: &[u8],
    a: u8,
    b: u8,
    width: i32,
    rows: i32,
    stride: usize,
    mut rnd: impl FnMut() -> u32,
) {
    let ab = a.saturating_add(b);
    let mut base = 0usize;
    for _ in 0..rows.max(1) {
        let off = (rnd() & 0xff) as usize;
        let mut col = 0usize;
        loop {
            for k in 0..16 {
                let d = dst[base + col + k];
                let v = d.saturating_sub(a).saturating_add(ab).saturating_sub(b);
                dst[base + col + k] = v.wrapping_add(src[off + col + k]);
            }
            col += 16;
            if (col as i32) >= width {
                break;
            }
        }
        base += stride;
    }
}

/// `0x00566330`: initialises a 6-byte record: u32 = `global`, u16 = 0.
pub fn w17a04_init_record(rec: &mut [u8], global: u32) {
    w17a04_put32(rec, 0, global);
    w17a04_put16(rec, 4, 0);
}

/// `0x005663b0`: bumps per-class counters.  Record: u32 class at 0, byte
/// flag at 4, byte skip at 5.  Counters are indexed `class*4 + (flag != 0)`
/// and, for a flagged record, also the class total `class*4`.
pub fn w17a04_bump_counters(rec: &[u8], enabled: bool, counts: &mut [i32], sums: &mut [f32]) {
    if rec[5] != 0 {
        return;
    }
    let class = u32::from_le_bytes([rec[0], rec[1], rec[2], rec[3]]) as usize;
    let flag = rec[4] != 0;
    if class < 0xc && enabled {
        let i = class * 4 + usize::from(flag);
        counts[i] += 1;
        if flag {
            counts[class * 4] += 1;
        }
        sums[i] += 1.0;
        if flag {
            sums[class * 4] += 1.0;
        }
    }
}

/// `0x00567c80`: queue an (owner, target) pair in a 16-deep ring of 0x200
/// slots, then fire `act(param_2, 1)`.  `pairs` is the (a,b) slot table,
/// `head`/`count` the ring cursor.  Always returns 0.
#[allow(clippy::too_many_arguments)]
pub fn w17a04_try_queue_target(
    target: u32,
    cur_target: u32,
    cmp_a: u8,
    cmp_b: u8,
    cur_alt: u32,
    ok_a: bool,
    ok_b: bool,
    pairs: &mut [(u32, u32)],
    head: u32,
    count: &mut u32,
    mut act: impl FnMut(),
) -> u32 {
    if target != cur_target && (cmp_a != cmp_b || target == cur_alt) && ok_a && ok_b {
        if cur_target != 0 {
            if pairs
                .iter()
                .take(0x200)
                .any(|p| p.0 == cur_target && p.1 == target)
            {
                return 0;
            }
            if *count != 0x10 {
                let i = (head * 0x10 + *count) as usize;
                *count += 1;
                pairs[i] = (cur_target, target);
            }
        }
        act();
    }
    0
}

/// `0x00569620`: swaps the pending position (`this+0x28/0x2a`) with the
/// globals, returning 1 when it changed.  Layout: `+0x20` u8, `+0x21` u8,
/// `+0x23` u8 <- `+0x4a`, `+0x28/+0x2a` u16, `+0x2c/+0x30` u32, `+0x38`/`+0x3c` u32.
pub fn w17a04_commit_position(
    t: &mut [u8],
    g: &mut W17a04Globals,
    out: &mut Vec<W17a04Call>,
) -> u32 {
    g.ga0 = w17a04_u16(t, 0x28);
    let changed = u32::from(!(g.ga0 == g.g98 && w17a04_u16(t, 0x2a) == g.g9c));
    g.ga4 = w17a04_u16(t, 0x2a);
    t[0x23] = t[0x4a];
    t[0x20] = g.b13;
    let v = [t[0x38], t[0x39], t[0x3a], t[0x3b]];
    t[0x3c..0x40].copy_from_slice(&v);
    w17a04_put16(t, 0x28, g.g98);
    w17a04_put16(t, 0x2a, g.g9c);
    w17a04_put32(t, 0x2c, g.ga8);
    w17a04_put32(t, 0x30, g.gac);
    out.push(W17a04Call::SetPos(w17a04_u16(t, 0x28), w17a04_u16(t, 0x2a)));
    w17a04_tail(g, t, out);
    changed
}

/// `0x005696f0`: like the commit above without the position swap; clears `+0x3c`.
pub fn w17a04_reset_state(t: &mut [u8], g: &W17a04Globals, out: &mut Vec<W17a04Call>) {
    t[0x23] = t[0x4a];
    t[0x20] = g.b13;
    w17a04_put32(t, 0x3c, 0);
    w17a04_tail(g, t, out);
}

/// `0x00569750`: copies `+0x4a` to `+0x23`, refreshes, applies `+0x21`.
pub fn w17a04_sync_byte(t: &mut [u8], out: &mut Vec<W17a04Call>) {
    t[0x23] = t[0x4a];
    out.push(W17a04Call::Refresh);
    out.push(W17a04Call::Apply(t[0x21]));
}

/// `0x00a89790`: stores a u16 at `this+0xe`.
pub fn w17a04_set_word_e(t: &mut [u8], v: u16) {
    w17a04_put16(t, 0xe, v);
}

/// `0x006cd1d0`: forwards to the 6-arg helper with `(a, b, c, 0, table, 0)`.
pub fn w17a04_forward<R>(
    a: u32,
    b: u32,
    c: u32,
    table: u32,
    f: impl FnOnce(u32, u32, u32, u32, u32, u32) -> R,
) -> R {
    f(a, b, c, 0, table, 0)
}

/// `0x00a894d0`: resets header (`[0]=0,[1]=0`), stores the two u16 dims at
/// +4/+6, calls the factory and stores the handle at +8; true if non-zero.
pub fn w17a04_init_surface(h: &mut [u8], w: u16, hh: u16, make: impl FnOnce() -> u32) -> bool {
    h[0..8].fill(0);
    w17a04_put16(h, 4, w);
    w17a04_put16(h, 6, hh);
    let r = make();
    w17a04_put32(h, 8, r);
    r != 0
}

#[cfg(test)]
mod tests_wave17_agent04 {
    use super::*;

    #[test]
    fn blend() {
        let mut d = [10u8; 32];
        let s = [1u8; 300];
        w17a04_blend_rows(&mut d, &s, 5, 3, 16, 2, 16, || 0x1ff);
        // (10-5)+8-3 = 10, +1
        assert!(d.iter().all(|&x| x == 11));
        let mut d = [250u8; 16];
        w17a04_blend_rows(&mut d, &s, 0, 10, 1, 0, 16, || 0);
        assert_eq!(d[0], 246);
    }

    #[test]
    fn records_and_counters() {
        let mut r = [9u8; 6];
        w17a04_init_record(&mut r, 0x11223344);
        assert_eq!(r, [0x44, 0x33, 0x22, 0x11, 0, 0]);
        let mut c = [0i32; 48];
        let mut f = [0f32; 48];
        let mut rec = [2, 0, 0, 0, 1, 0];
        w17a04_bump_counters(&rec, true, &mut c, &mut f);
        assert_eq!((c[9], c[8], f[9]), (1, 1, 1.0));
        rec[5] = 1;
        w17a04_bump_counters(&rec, true, &mut c, &mut f);
        rec[5] = 0;
        rec[0] = 12;
        w17a04_bump_counters(&rec, true, &mut c, &mut f);
        w17a04_bump_counters(&[1, 0, 0, 0, 0, 0], false, &mut c, &mut f);
        assert_eq!(c.iter().sum::<i32>(), 2);
    }

    #[test]
    fn queue_target() {
        let mut pairs = vec![(0u32, 0u32); 0x200 + 0x10];
        let mut n = 0;
        let mut fired = 0;
        let go = |p: &mut Vec<(u32, u32)>, n: &mut u32, fired: &mut i32| {
            w17a04_try_queue_target(5, 7, 1, 2, 0, true, true, p, 0, n, || *fired += 1)
        };
        assert_eq!(go(&mut pairs, &mut n, &mut fired), 0);
        assert_eq!((n, fired, pairs[0]), (1, 1, (7, 5)));
        go(&mut pairs, &mut n, &mut fired);
        assert_eq!((n, fired), (1, 1));
        let mut f2 = 0;
        w17a04_try_queue_target(7, 7, 1, 2, 0, true, true, &mut pairs, 0, &mut n, || f2 += 1);
        assert_eq!(f2, 0);
    }

    #[test]
    fn position_family() {
        let mut t = [0u8; 0x50];
        t[0x4a] = 7;
        t[0x21] = 3;
        t[0x38] = 9;
        let mut g = W17a04Globals {
            g98: 4,
            g9c: 5,
            b13: 8,
            b10: 1,
            ..Default::default()
        };
        let mut out = vec![];
        assert_eq!(w17a04_commit_position(&mut t, &mut g, &mut out), 1);
        assert_eq!((t[0x23], t[0x20], t[0x3c], t[0x28]), (7, 8, 9, 4));
        assert_eq!(
            out,
            [
                W17a04Call::SetPos(4, 5),
                W17a04Call::Refresh,
                W17a04Call::Apply(3),
                W17a04Call::Notify(0xb, 1)
            ]
        );
        let mut out = vec![];
        assert_eq!(w17a04_commit_position(&mut t, &mut g, &mut out), 0);
        g.b11 = 1;
        let mut out = vec![];
        w17a04_reset_state(&mut t, &g, &mut out);
        assert_eq!(out.last(), Some(&W17a04Call::Notify(0xc, 0)));
        assert_eq!(&t[0x3c..0x40], &[0; 4]);
        let mut out = vec![];
        w17a04_sync_byte(&mut t, &mut out);
        assert_eq!(out, vec![W17a04Call::Refresh, W17a04Call::Apply(3)]);
    }

    #[test]
    fn small() {
        let mut t = [0u8; 16];
        w17a04_set_word_e(&mut t, 0xabcd);
        assert_eq!(&t[0xe..], &[0xcd, 0xab]);
        assert_eq!(
            w17a04_forward(1, 2, 3, 9, |a, b, c, d, e, f| a + b + c + d + e + f),
            15
        );
        let mut h = [0xffu8; 12];
        assert!(w17a04_init_surface(&mut h, 3, 4, || 77));
        assert_eq!(h, [0, 0, 0, 0, 3, 0, 4, 0, 77, 0, 0, 0]);
        assert!(!w17a04_init_surface(&mut h, 3, 4, || 0));
    }
}
