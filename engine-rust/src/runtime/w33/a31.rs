#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent31.rs
// ===========================================================================
// Wave 33 translation batch 31: three validators over parsed request records.
// Callees are supplied by the caller as closures.

fn w33a31_rd32(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// `0x00c204e0`: validates a 0x21-byte request record against a permitted-bit
/// mask `mask` and the limits `lim_e0`, `lim_e4`, `lim_ec` (context fields at
/// `+0xe0`, `+0xe4`, `+0xec`).  `flags(kind)`, `pred_a(kind)` and `pred_b(kind)`
/// model the helpers FUN_00c12eb0, FUN_00c12950 and FUN_00c12990.
pub fn w33a31_validate_request(
    rec: &[u8],
    mask: u32,
    lim_e0: u32,
    lim_e4: u32,
    lim_ec: u32,
    flags: impl Fn(u8) -> u32,
    pred_a: impl Fn(u8) -> bool,
    pred_b: impl Fn(u8) -> bool,
) -> bool {
    let kind = rec[0];
    let sel = rec[8];
    let f = w33a31_rd32(rec, 4);
    let (p0c, p10, p14, p18, p1c) = (
        w33a31_rd32(rec, 0xc),
        w33a31_rd32(rec, 0x10),
        w33a31_rd32(rec, 0x14),
        w33a31_rd32(rec, 0x18),
        w33a31_rd32(rec, 0x1c),
    );
    let bit = |n: u32| (mask >> n) & 1 != 0;
    let mut ok = kind < 9 && rec[0x20] < 5 && (f & 0x1f) == f && sel < 0x7a && f != 0;
    if f & 1 != 0 {
        ok &= bit(6);
    }
    if f & 2 != 0 {
        ok &= bit(7);
    }
    if f & 4 != 0 {
        ok &= bit(if flags(sel) & 0x20 == 0 { 8 } else { 9 });
    }
    if f & 8 != 0 {
        ok &= bit(10);
    }
    if f & 0x10 != 0 {
        ok &= bit(11);
    }
    match kind {
        0 | 1 => ok &= p10 == 1 && p0c <= lim_e0 && p14 == 1 && bit(2),
        2..=5 => ok &= p14 == 1 && p10 <= lim_e4 && p0c <= lim_e4 && bit(3),
        6 | 7 => {
            let t = ok && p14 == 1;
            ok = p0c == p10 && p10 <= lim_e4 && p0c <= lim_e4 && t && bit(5);
        }
        8 => ok &= p0c <= lim_ec && p14 <= lim_ec && p10 <= lim_ec && bit(4),
        _ => {}
    }
    let ok2 = if !pred_a(kind) {
        ok && p18 == 1
    } else {
        ok && p18 != 0
    };
    let bits = if !pred_b(kind) {
        let m = if p10 < p0c { p0c } else { p10 };
        let v = p14.max(m);
        32 - (v | 1).leading_zeros()
    } else {
        1
    };
    let ok3 = ok2 && p1c != 0 && p1c <= bits;
    if !pred_b(kind) {
        ok3 && rec[0x20] == 0
    } else {
        ok3
    }
}

/// `0x00c20760`: checks a descriptor (`word0`, `lo`, `hi`, `opts`) against a
/// record (`rec[0]`, flags at `+4`, selector at `+8`) and `want` bits.
/// `check(a, b)` models FUN_00c12710 and `flags(sel)` models FUN_00c12eb0.
pub fn w33a31_check_descriptor(
    word0: u32,
    lo: u8,
    hi: u8,
    opts: u8,
    rec: &[u8],
    want: u32,
    check: impl Fn(u8, u8) -> u8,
    flags: impl Fn(u8) -> u32,
) -> bool {
    let mut ok = (w33a31_rd32(rec, 4) & want) != 0 && word0 != 0;
    if lo != 9 {
        ok = (check(lo, rec[0]) & 1 != 0) && lo < 9 && ok;
    }
    if hi != 0x7a {
        ok = ok && hi < 0x7a;
    }
    let low_one = !(flags(rec[8]) & 0x20 == 0 && opts & 1 != 0);
    if flags(rec[8]) & 0x40 == 0 && opts & 2 != 0 {
        return false;
    }
    ok && low_one
}

/// `0x00c20d70`: fit check of a size request `p` (ten u32 words) against the
/// remaining capacity word `cap` (`+0xc` of the second record) with context
/// alignments `align_114` and `align_118`.  Helpers FUN_00c12eb0, FUN_00c12dc0,
/// FUN_00c12d10, FUN_00c12fd0 and FUN_00c12ec0 are the closures.
pub fn w33a31_fit_request(
    p: &[u32],
    cap: u32,
    id: u32,
    align_114: u32,
    align_118: u32,
    flags: impl Fn(u32) -> u32,
    step_x: impl Fn(u32) -> i32,
    step_y: impl Fn(u32) -> i32,
    base: impl Fn(u32, u32) -> u32,
    count: impl Fn(u32, u32, u32) -> i32,
) -> bool {
    let (mut x, mut y, mul) = (p[7], p[8], p[9]);
    if flags(id) & 0x10 != 0 {
        let sx = step_x(id) as u32;
        let sy = step_y(id) as u32;
        x = x.wrapping_sub(1).wrapping_add(sx) & sx.wrapping_neg();
        y = y.wrapping_sub(1).wrapping_add(sy) & sy.wrapping_neg();
    }
    let v6 = base(id, x);
    let v3 = if p[1] != 0 { p[1] } else { v6 };
    let used = (count(id, v3, y) as u32).wrapping_mul(mul);
    let fits = used <= cap && (cap < used || p[0] <= cap.wrapping_sub(used));
    if p[8] != 1 && p[1] == 0 {
        return false;
    }
    v6 <= v3
        && p[1] & align_114.wrapping_sub(1) == 0
        && p[0] & align_118.wrapping_sub(1) == 0
        && fits
}

#[cfg(test)]
mod tests_wave33_agent31 {
    use super::*;

    #[test]
    fn descriptor_basic() {
        let mut rec = [0u8; 12];
        rec[4] = 1;
        assert!(w33a31_check_descriptor(
            1,
            9,
            0x7a,
            0,
            &rec,
            1,
            |_, _| 1,
            |_| 0x60
        ));
        assert!(!w33a31_check_descriptor(
            0,
            9,
            0x7a,
            0,
            &rec,
            1,
            |_, _| 1,
            |_| 0x60
        ));
        assert!(!w33a31_check_descriptor(
            1,
            9,
            0x7a,
            2,
            &rec,
            1,
            |_, _| 1,
            |_| 0
        ));
    }

    #[test]
    fn fit_basic() {
        let p = [0u32, 4, 0, 0, 0, 0, 0, 2, 1, 2];
        assert!(w33a31_fit_request(
            &p,
            10,
            0,
            4,
            4,
            |_| 0,
            |_| 1,
            |_| 1,
            |_, x| x,
            |_, _, _| 3
        ));
        assert!(!w33a31_fit_request(
            &p,
            5,
            0,
            4,
            4,
            |_| 0,
            |_| 1,
            |_| 1,
            |_, x| x,
            |_, _, _| 3
        ));
    }

    #[test]
    fn validate_basic() {
        let mut r = [0u8; 0x21];
        r[4] = 1;
        r[8] = 1;
        r[0xc] = 1;
        r[0x10] = 1;
        r[0x14] = 1;
        r[0x18] = 1;
        r[0x1c] = 1;
        let ok = |m| w33a31_validate_request(&r, m, 5, 5, 5, |_| 0, |_| false, |_| false);
        assert!(ok(!0));
        assert!(!ok(0));
    }
}
