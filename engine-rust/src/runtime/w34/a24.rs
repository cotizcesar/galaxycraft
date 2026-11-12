#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent24.rs
// ===========================================================================
// Wave 34 translation batch 24.
//
// Implemented: one 16-lane Hadamard-style butterfly (scalar SSE2 body).
// Skipped: FUN_004c1220 and FUN_004c13b8 (SIMD DCT with unresolved constant tables).

/// `0x004c1760`: two-stage add/sub butterfly over 16 i16 lanes with a final
/// `(x + 3) >> 3` round; results scatter to `out[k * 0x10]` (even lane
/// groups at multiples of 0x20, odd at 0x10 + multiples of 0x20).
/// `out` must hold at least 0xf1 elements.
pub fn w34a24_butterfly16(inp: &[i16; 16], out: &mut [i16]) {
    let a = |i: usize| inp[i];
    let mut s = [0i16; 8];
    let mut d = [0i16; 8];
    // lanes 0..3 pair with 12..15, lanes 4..7 pair with 8..11
    for i in 0..4 {
        s[i] = a(i).wrapping_add(a(i + 12));
        d[i] = a(i).wrapping_sub(a(i + 12));
        s[i + 4] = a(i + 4).wrapping_add(a(i + 8));
        d[i + 4] = a(i + 4).wrapping_sub(a(i + 8));
    }
    // second stage
    let mut p = [0i16; 4]; // sum of s[i+4]+s[i]  (sVar17,19,20,21)
    let mut m = [0i16; 4]; // s[i]-s[i+4]
    let mut q = [0i16; 4]; // d[i+4]+d[i]  (sVar22..25)
    let mut r = [0i16; 4]; // d[i]-d[i+4]
    for i in 0..4 {
        p[i] = s[i + 4].wrapping_add(s[i]);
        m[i] = s[i].wrapping_sub(s[i + 4]);
        q[i] = d[i + 4].wrapping_add(d[i]);
        r[i] = d[i].wrapping_sub(d[i + 4]);
    }
    // third stage: pairs (0,3) and (1,2)
    let t5 = p[0].wrapping_add(p[3]);
    let t6 = q[0].wrapping_add(q[3]);
    let t7 = m[0].wrapping_add(m[3]);
    let t8 = r[0].wrapping_add(r[3]);
    let t13 = p[1].wrapping_add(p[2]);
    let t14 = q[1].wrapping_add(q[2]);
    let t15 = m[1].wrapping_add(m[2]);
    let t16 = r[1].wrapping_add(r[2]);
    let t17 = p[0].wrapping_sub(p[3]);
    let t22 = q[0].wrapping_sub(q[3]);
    let t1 = m[0].wrapping_sub(m[3]);
    let t9 = r[0].wrapping_sub(r[3]);
    let t19 = p[1].wrapping_sub(p[2]);
    let t23 = q[1].wrapping_sub(q[2]);
    let t2 = m[1].wrapping_sub(m[2]);
    let t10 = r[1].wrapping_sub(r[2]);
    let lo_a = [
        t13.wrapping_add(t5),
        t14.wrapping_add(t6),
        t15.wrapping_add(t7),
        t16.wrapping_add(t8),
        t19.wrapping_add(t17),
        t23.wrapping_add(t22),
        t2.wrapping_add(t1),
        t10.wrapping_add(t9),
    ];
    let lo_b = [
        t5.wrapping_sub(t13),
        t6.wrapping_sub(t14),
        t7.wrapping_sub(t15),
        t8.wrapping_sub(t16),
        t17.wrapping_sub(t19),
        t22.wrapping_sub(t23),
        t1.wrapping_sub(t2),
        t9.wrapping_sub(t10),
    ];
    for k in 0..8 {
        let base = (k % 4) * 0x40 + (k / 4) * 0x10;
        out[base] = lo_a[k].wrapping_add(3) >> 3;
        out[base + 0x20] = lo_b[k].wrapping_add(3) >> 3;
    }
}

#[cfg(test)]
mod tests_wave34_agent24 {
    use super::*;

    #[test]
    fn zero_and_dc() {
        let mut out = vec![0i16; 0x100];
        w34a24_butterfly16(&[0; 16], &mut out);
        assert!(out.iter().all(|&v| v == 0));
        w34a24_butterfly16(&[1; 16], &mut out);
        // DC sum = 16, (16+3)>>3 = 2 at index 0; all others (0+3)>>3 = 0
        assert_eq!(out[0], 2);
        assert_eq!(out[0x20], 0);
        assert_eq!(out[0xf0], 0);
    }
}
