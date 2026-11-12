#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent45.rs
// ===========================================================================
// Wave 33 agent 45: small iterator-advance helpers.

/// `0x00c98c9d`: bucket count from a hash-table header; `0x100` when the
/// field at `+0x404` is zero, else the field at `+0x400` shifted right by 2.
pub fn w33a45_bucket_count(field_400: u32, field_404: i32) -> u32 {
    if field_404 == 0 {
        0x100
    } else {
        field_400 >> 2
    }
}

/// `0x00c99080`: advances `count` times; each step calls `step`, which
/// returns false to set `*pos = -1` and stop, else increments `*pos`.
pub fn w33a45_advance_checked(count: i32, pos: &mut i32, mut step: impl FnMut() -> bool) {
    let mut i = 0;
    if 0 < count {
        loop {
            let v = if !step() {
                *pos = -1;
                -1
            } else {
                *pos = pos.wrapping_add(1);
                *pos
            };
            i += 1;
            if v == -1 || i >= count {
                break;
            }
        }
    }
}

/// `0x00c990bb`: calls `step` up to `count` times, stopping once it
/// leaves `*pos == -1`.
pub fn w33a45_advance_until_end(count: i32, pos: &mut i32, mut step: impl FnMut(&mut i32)) {
    let mut i = 0;
    if 0 < count {
        loop {
            step(pos);
            if *pos == -1 {
                return;
            }
            i += 1;
            if i >= count {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests_wave33_agent45 {
    use super::*;

    #[test]
    fn bucket() {
        assert_eq!(w33a45_bucket_count(64, 0), 0x100);
        assert_eq!(w33a45_bucket_count(64, 1), 16);
    }

    #[test]
    fn checked() {
        let mut p = 0;
        w33a45_advance_checked(3, &mut p, || true);
        assert_eq!(p, 3);
        let mut n = 0;
        w33a45_advance_checked(5, &mut p, || {
            n += 1;
            n < 2
        });
        assert_eq!(p, -1);
        assert_eq!(n, 2);
    }

    #[test]
    fn until_end() {
        let mut p = 0;
        let mut calls = 0;
        w33a45_advance_until_end(5, &mut p, |q| {
            calls += 1;
            if calls == 3 { *q = -1 } else { *q += 1 }
        });
        assert_eq!((p, calls), (-1, 3));
    }
}
