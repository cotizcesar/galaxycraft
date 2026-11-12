#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent15.rs
// ===========================================================================
// Wave 27 agent 15: small library-support helpers (sound mixer timing/buffers).

/// `0x009aaa00`: clamped read from a memory-backed stream.  Returns the
/// bytes copied; `len` is limited to `size - pos`.
pub fn w27a15_mem_read(buf: &[u8], size: u32, pos: u32, dst: &mut [u8], len: u32) -> usize {
    let mut n = len as i64;
    if (size as u64) < pos as u64 + len as u64 {
        n = size as i64 - pos as i64;
    }
    let n = n.max(0) as usize;
    let n = n.min(dst.len());
    let start = pos as usize;
    let n = n.min(buf.len().saturating_sub(start));
    dst[..n].copy_from_slice(&buf[start..start + n]);
    n
}

/// `0x009a81d0`: zero-fills a `0x70` byte record.
pub fn w27a15_zero_0x70(rec: &mut [u8]) {
    let n = rec.len().min(0x70);
    rec[..n].fill(0);
}

/// `0x009ad480`: zero-fills a `0x84` byte global record.
pub fn w27a15_zero_0x84(rec: &mut [u8]) {
    let n = rec.len().min(0x84);
    rec[..n].fill(0);
}

/// Timer record used by `0x009aa810` (offsets 0x08, 0x0c, 0x28, 0x2c, 0x30, 0x34).
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct W27a15Timer {
    pub start: u32,
    pub stop: u32,
    pub active: u8,
    pub accum: u32,
    pub depth: u32,
    pub enabled: bool,
}

/// `0x009aa810`: nested start/stop counter.  `flag != 0` pushes (sampling
/// the clock on first entry), `flag == 0` pops and on reaching zero adds the
/// elapsed time to `accum`.  `now` models the clock reader.  Returns 0.
pub fn w27a15_timer_toggle(t: &mut W27a15Timer, flag: u8, now: &mut dyn FnMut() -> u32) -> u32 {
    if !t.enabled {
        return 0;
    }
    let depth = t.depth;
    if flag != 0 {
        let mut d = depth;
        if d == 0 {
            t.start = now();
            d = t.depth;
        }
        t.active = flag;
        t.depth = d + 1;
        return 0;
    }
    t.depth = depth.wrapping_sub(1);
    if t.depth == 0 {
        t.stop = now();
        if t.start < t.stop {
            t.accum = t.accum.wrapping_add(t.stop - t.start);
        }
    }
    t.active = 0;
    0
}

/// `0x009aa8a0`: CPU-usage smoothing update.  `s` is the 14-word record
/// (floats stored as bit patterns), `pct` the percent weight and `now` the
/// sampled clock.  Returns 0.
pub fn w27a15_usage_update(s: &mut [u32; 14], pct: i32, now: u32) -> u32 {
    let w = pct as f32 / 100.0;
    s[1] = now;
    s[5] = now;
    let mut f3 = if now < s[4] {
        0.0f32
    } else {
        (now - s[4]) as i32 as f64 as f32
    };
    let mut f4 = f32::from_bits(s[6]) * w;
    f3 += f32::from_bits(s[7]) * w;
    s[6] = f4.to_bits();
    s[7] = f3.to_bits();
    if s[0] < now {
        let d = now.wrapping_sub(s[11]).wrapping_sub(s[0]) as i32;
        f4 += (d as f64 as f32 * 100.0) / (f3 * (1.0 - w));
        s[6] = f4.to_bits();
    }
    s[4] = now;
    s[11] = 0;
    s[12] = 0;
    s[9] = ((1.0 - w) * f4) as u32;
    s[13] &= !0xff;
    0
}

#[cfg(test)]
mod tests_wave27_agent15 {
    use super::*;

    #[test]
    fn mem_read_clamps() {
        let b = [1u8, 2, 3, 4, 5];
        let mut d = [0u8; 8];
        assert_eq!(w27a15_mem_read(&b, 5, 3, &mut d, 8), 2);
        assert_eq!(&d[..2], &[4, 5]);
        assert_eq!(w27a15_mem_read(&b, 5, 0, &mut d, 3), 3);
    }

    #[test]
    fn zero_fill() {
        let mut a = [1u8; 0x80];
        w27a15_zero_0x70(&mut a);
        assert!(a[..0x70].iter().all(|&x| x == 0) && a[0x70] == 1);
        let mut b = [1u8; 0x90];
        w27a15_zero_0x84(&mut b);
        assert!(b[..0x84].iter().all(|&x| x == 0) && b[0x84] == 1);
    }

    #[test]
    fn timer_nesting() {
        let mut t = W27a15Timer {
            enabled: true,
            ..Default::default()
        };
        let mut c = 100u32;
        let mut clk = || {
            c += 50;
            c
        };
        w27a15_timer_toggle(&mut t, 1, &mut clk);
        w27a15_timer_toggle(&mut t, 1, &mut clk);
        assert_eq!(t.depth, 2);
        w27a15_timer_toggle(&mut t, 0, &mut clk);
        assert_eq!(t.accum, 0);
        w27a15_timer_toggle(&mut t, 0, &mut clk);
        assert_eq!(t.accum, 50);
        assert_eq!(t.active, 0);
        let mut off = W27a15Timer::default();
        assert_eq!(w27a15_timer_toggle(&mut off, 1, &mut clk), 0);
        assert_eq!(off.depth, 0);
    }

    #[test]
    fn usage_update_basic() {
        let mut s = [0u32; 14];
        s[6] = 10.0f32.to_bits();
        s[7] = 20.0f32.to_bits();
        s[13] = 0x1ff;
        w27a15_usage_update(&mut s, 50, 100);
        assert_eq!(s[1], 100);
        assert_eq!(s[4], 100);
        assert_eq!(f32::from_bits(s[7]), 110.0);
        assert_eq!(s[13], 0x100);
        assert_eq!(s[11], 0);
    }
}
