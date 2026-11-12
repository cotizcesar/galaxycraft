#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent143.rs
// ===========================================================================
/// Buffer descriptor used by the two reset helpers (stride 0x18 bytes of u32 words).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W32a143Buf {
    pub init_ptr: u32,
    pub init_len: u32,
    pub cur_ptr: u32,
    pub cur_len: u32,
    pub used: u32,
    pub is_static: u8,
}

fn w32a143_reset(
    b: &mut W32a143Buf,
    scale: u32,
    mut alloc: impl FnMut(u32) -> u32,
    mut copy: impl FnMut(u32, u32, u32, u32),
) -> u32 {
    let mut ret = b.cur_ptr;
    if ret == 0 || b.used == 0 {
        return 0;
    }
    if b.is_static == 0 {
        ret = alloc(b.used.wrapping_mul(scale));
        copy(ret, b.used, b.cur_ptr, b.cur_len);
    }
    b.used = 0;
    b.cur_ptr = b.init_ptr;
    b.cur_len = b.init_len;
    ret
}

/// `0x00ca3657`: resets the buffer to its initial pointer/length, optionally
/// copying the used bytes into a fresh allocation first; returns 0 if empty.
pub fn w32a143_buf_reset(
    b: &mut W32a143Buf,
    alloc: impl FnMut(u32) -> u32,
    copy: impl FnMut(u32, u32, u32, u32),
) -> u32 {
    w32a143_reset(b, 1, alloc, copy)
}

/// `0x00ca36a7`: wide-char variant of the reset helper (allocation size doubled).
pub fn w32a143_buf_reset_wide(
    b: &mut W32a143Buf,
    alloc: impl FnMut(u32) -> u32,
    copy: impl FnMut(u32, u32, u32, u32),
) -> u32 {
    w32a143_reset(b, 2, alloc, copy)
}

#[cfg(test)]
mod tests_wave32_agent143 {
    use super::*;

    fn sample(st: u8) -> W32a143Buf {
        W32a143Buf {
            init_ptr: 10,
            init_len: 20,
            cur_ptr: 30,
            cur_len: 40,
            used: 5,
            is_static: st,
        }
    }

    #[test]
    fn reset_narrow_and_wide() {
        let mut b = sample(0);
        let mut sz = 0;
        let r = w32a143_buf_reset(
            &mut b,
            |n| {
                sz = n;
                99
            },
            |d, _, _, _| assert_eq!(d, 99),
        );
        assert_eq!((r, sz, b.cur_ptr, b.cur_len, b.used), (99, 5, 10, 20, 0));
        let mut b = sample(0);
        w32a143_buf_reset_wide(
            &mut b,
            |n| {
                sz = n;
                99
            },
            |_, _, _, _| {},
        );
        assert_eq!(sz, 10);
        let mut b = sample(1);
        assert_eq!(
            w32a143_buf_reset(&mut b, |_| panic!(), |_, _, _, _| panic!()),
            30
        );
        let mut b = sample(0);
        b.used = 0;
        assert_eq!(w32a143_buf_reset(&mut b, |_| 1, |_, _, _, _| {}), 0);
    }
}
