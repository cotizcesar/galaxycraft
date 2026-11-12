#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent131.rs
// ===========================================================================
// Wave 32 translation batch 131: CRT printf integer formatting helpers.

/// Formatter state touched by the integer-to-text helpers (offsets 0x28, 0x34, 0x38).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W32a131State {
    /// Remaining minimum digit count (+0x28).
    pub precision: i32,
    /// Index of the first digit written (+0x34).
    pub start: i32,
    /// Number of digits produced (+0x38).
    pub len: i32,
}

/// `0x00c983de`: dispatches on a global flag; when set calls the wide path
/// with 0, otherwise the narrow path with 1.  Returns which branch ran
/// (`wide_flag`) after invoking the supplied callbacks.
pub fn w32a131_dispatch(
    wide_flag: bool,
    param: u32,
    wide: &mut dyn FnMut(u32, u32),
    narrow: &mut dyn FnMut(u32, u32),
) {
    if wide_flag {
        wide(param, 0);
    } else {
        narrow(param, 1);
    }
}

fn w32a131_digit_char(rem: u32, lower_flag: u8) -> u8 {
    let mut c = (rem as u8).wrapping_add(b'0');
    if c > b'9' {
        c = c
            .wrapping_add((lower_flag ^ 1).wrapping_mul(b' '))
            .wrapping_add(7);
    }
    c
}

/// `0x00c98e52`: writes `value` in `radix` right-to-left into `buf` starting
/// at `end_index` (the narrow buffer end minus one, already computed from the
/// helper), padding with '0' while precision remains.  Updates state.
pub fn w32a131_utoa_narrow(
    st: &mut W32a131State,
    buf: &mut [u8],
    end_index: i32,
    mut value: u32,
    radix: u32,
    lower_flag: u8,
) {
    st.start = end_index;
    let mut cur = end_index;
    while st.precision > 0 || value != 0 {
        st.precision -= 1;
        let c = w32a131_digit_char(value % radix, lower_flag);
        if cur >= 0 && (cur as usize) < buf.len() {
            buf[cur as usize] = c;
        }
        st.start -= 1;
        cur = st.start;
        value /= radix;
    }
    st.len = end_index - cur;
    st.start += 1;
}

/// `0x00c98ec5`: wide-character variant; indices are in 16-bit units.
pub fn w32a131_utoa_wide(
    st: &mut W32a131State,
    buf: &mut [u16],
    end_index: i32,
    mut value: u32,
    radix: u32,
    lower_flag: u8,
) {
    st.start = end_index;
    let mut cur = end_index;
    while st.precision > 0 || value != 0 {
        st.precision -= 1;
        let c = w32a131_digit_char(value % radix, lower_flag) as u16;
        if cur >= 0 && (cur as usize) < buf.len() {
            buf[cur as usize] = c;
        }
        st.start -= 1;
        cur = st.start;
        value /= radix;
    }
    st.len = end_index - cur;
    st.start += 1;
}

#[cfg(test)]
mod tests_wave32_agent131 {
    use super::*;

    #[test]
    fn narrow_hex() {
        let mut st = W32a131State::default();
        let mut b = [b'?'; 8];
        w32a131_utoa_narrow(&mut st, &mut b, 7, 0xbeef, 16, 1);
        assert_eq!(st.len, 4);
        assert_eq!(&b[st.start as usize..], b"BEEF");
        let mut st = W32a131State {
            precision: 6,
            ..Default::default()
        };
        w32a131_utoa_narrow(&mut st, &mut b, 7, 0xbeef, 16, 0);
        assert_eq!(&b[st.start as usize..], b"00beef");
    }

    #[test]
    fn wide_dec_and_dispatch() {
        let mut st = W32a131State::default();
        let mut b = [0u16; 6];
        w32a131_utoa_wide(&mut st, &mut b, 5, 1234, 10, 0);
        assert_eq!(st.len, 4);
        assert_eq!(b[2..], [49, 50, 51, 52]);
        let r = std::cell::Cell::new(99);
        w32a131_dispatch(true, 5, &mut |_, f| r.set(f), &mut |_, f| r.set(10 + f));
        assert_eq!(r.get(), 0);
    }
}
