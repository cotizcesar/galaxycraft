#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent66.rs
// ===========================================================================
// Wave 31 agent 66 translations (small struct-init / loop helpers).

/// Record initialised by `0x00a6c220` (22 dwords).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[allow(non_camel_case_types)]
pub struct w31a66_Record {
    pub d: [u32; 22],
}

/// `0x00a6c220`: initialises the record from four leading and two trailing
/// parameters; the remaining fields get fixed sentinel values (0 or -1).
pub fn w31a66_init_record(
    rec: &mut w31a66_Record,
    p1: u32,
    p2: u32,
    p3: u32,
    p4: u32,
    p5: u32,
    p6: u32,
) {
    let d = &mut rec.d;
    d[0] = p1;
    d[1] = p2;
    d[2] = p3;
    d[3] = p4;
    // word at +0x18 = 0xffff, byte at +0x1a = 0 (upper half of dword 6)
    d[6] = 0x0000_ffff;
    d[8] = 0xffff_ffff;
    d[4] = 0;
    d[5] = 0;
    d[7] = 0;
    d[10] = 0xffff_ffff;
    d[11] = 0xffff_ffff;
    d[0x10] = 0xffff_ffff;
    d[0x11] = 0xffff_ffff;
    d[0xc] = 0;
    d[0xd] = 0;
    d[0xe] = 0;
    d[0xf] = 0;
    d[0x12] = 0;
    d[0x13] = 0;
    d[0x14] = p5;
    d[0x15] = p6;
}

/// `0x00a367f0`: walks a `width` x `height` area in `step` sized blocks,
/// calling `f(a_off, a_stride, b_off, b_stride)` for each block and summing
/// the two returned values.  Returns `(sum0, sum1)`.
#[allow(clippy::too_many_arguments)]
pub fn w31a66_block_sum(
    a: i32,
    a_stride: i32,
    b: i32,
    b_stride: i32,
    width: i32,
    height: i32,
    step: i32,
    mut f: impl FnMut(i32, i32, i32, i32) -> (i32, i32),
) -> (i32, i32) {
    let (mut s0, mut s1) = (0i32, 0i32);
    if step <= 0 {
        return (0, 0);
    }
    let (mut a, mut b) = (a, b);
    let mut y = 0;
    while y < height {
        let mut x = 0;
        while x < width {
            let (r0, r1) = f(a + x, a_stride, b + x, b_stride);
            s0 = s0.wrapping_add(r0);
            s1 = s1.wrapping_add(r1);
            x += step;
        }
        b = b.wrapping_add(b_stride.wrapping_mul(step));
        y += step;
        a = a.wrapping_add(a_stride.wrapping_mul(step));
    }
    (s0, s1)
}

/// `0x00a56fc0`: invokes `f(index, 0x18, value)` for index 0..4 where value
/// is `0x70`, or `0x80` when `field_1d8 != 0 && flag_1e8 == 0 && field_1d4 == 0`.
pub fn w31a66_fill_four(
    field_1d8: i32,
    flag_1e8: u8,
    field_1d4: i32,
    mut f: impl FnMut(u8, u32, u32),
) {
    let v = if field_1d8 == 0 || flag_1e8 != 0 {
        0x70
    } else if field_1d4 == 0 {
        0x80
    } else {
        0x70
    };
    for i in 0..4u8 {
        f(i, 0x18, v);
    }
}

#[cfg(test)]
mod tests_wave31_agent66 {
    use super::*;

    #[test]
    fn init_record() {
        let mut r = w31a66_Record::default();
        w31a66_init_record(&mut r, 1, 2, 3, 4, 5, 6);
        assert_eq!(&r.d[0..4], &[1, 2, 3, 4]);
        assert_eq!(r.d[6], 0xffff);
        assert_eq!(r.d[8], u32::MAX);
        assert_eq!((r.d[0x14], r.d[0x15]), (5, 6));
    }

    #[test]
    fn block_sum() {
        let mut n = 0;
        let r = w31a66_block_sum(0, 10, 0, 10, 8, 8, 4, |_, _, _, _| {
            n += 1;
            (1, 2)
        });
        assert_eq!(n, 4);
        assert_eq!(r, (4, 8));
        assert_eq!(
            w31a66_block_sum(0, 1, 0, 1, 8, 8, 0, |_, _, _, _| (1, 1)),
            (0, 0)
        );
    }

    #[test]
    fn fill_four() {
        let mut got = vec![];
        w31a66_fill_four(1, 0, 0, |i, a, v| got.push((i, a, v)));
        assert_eq!(got.len(), 4);
        assert!(got.iter().all(|g| g.2 == 0x80 && g.1 == 0x18));
        got.clear();
        w31a66_fill_four(1, 0, 1, |_, _, v| got.push((0, 0, v)));
        assert_eq!(got[0].2, 0x70);
        got.clear();
        w31a66_fill_four(0, 0, 0, |_, _, v| got.push((0, 0, v)));
        assert_eq!(got[0].2, 0x70);
    }
}
