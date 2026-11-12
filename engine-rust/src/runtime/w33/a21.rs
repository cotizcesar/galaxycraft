#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent21.rs
// ===========================================================================
// Wave 33 agent 21: runtime helpers.

/// `0x00963680`: dispatch on a record's kind (`kind`) and sub-state
/// (`state`, low 16 bits of field 4).  When `state == 0xe` and `kind == 1`
/// the `pre` callback runs and the state is re-read via `reload_state`.
/// If the state is not 0xe the result is 0; otherwise kinds 0, 6, 8, 10,
/// 0xd and 0xe call `handler`, everything else returns 0.
pub fn w33a21_dispatch<P, R, H>(
    kind: i32,
    state: i16,
    mut pre: P,
    reload_state: R,
    handler: H,
) -> u32
where
    P: FnMut(),
    R: FnOnce() -> i16,
    H: FnOnce() -> u32,
{
    let mut st = state;
    if st == 0xe {
        if kind == 1 {
            pre();
            st = reload_state();
            if st != 0xe {
                return 0;
            }
        }
    } else {
        return 0;
    }
    match kind {
        0 | 6 | 8 | 10 | 0xd | 0xe => handler(),
        _ => 0,
    }
}

/// `0x0099eba0`: strcpy; copies `src` through its NUL into `dst` and returns
/// the number of bytes written (dst pointer is returned by the original).
pub fn w33a21_strcpy(dst: &mut [u8], src: &[u8]) -> usize {
    let mut i = 0;
    loop {
        let c = src.get(i).copied().unwrap_or(0);
        dst[i] = c;
        i += 1;
        if c == 0 {
            return i;
        }
    }
}

/// `0x0099ec90`: strlen; count of bytes before the first NUL (or slice end).
pub fn w33a21_strlen(s: &[u8]) -> usize {
    s.iter().position(|&b| b == 0).unwrap_or(s.len())
}

#[cfg(test)]
mod tests_wave33_agent21 {
    use super::*;

    #[test]
    fn strings() {
        let mut d = [9u8; 8];
        assert_eq!(w33a21_strcpy(&mut d, b"ab\0"), 3);
        assert_eq!(&d[..4], b"ab\0\x09");
        assert_eq!(w33a21_strlen(b"abc\0d"), 3);
        assert_eq!(w33a21_strlen(b"ab"), 2);
    }

    #[test]
    fn dispatch() {
        assert_eq!(w33a21_dispatch(0, 3, || {}, || 0, || 7), 0);
        assert_eq!(w33a21_dispatch(0, 0xe, || {}, || 0xe, || 7), 7);
        assert_eq!(w33a21_dispatch(3, 0xe, || {}, || 0xe, || 7), 0);
        assert_eq!(w33a21_dispatch(1, 0xe, || {}, || 0xe, || 7), 0);
        assert_eq!(w33a21_dispatch(1, 0xe, || {}, || 5, || 7), 0);
        assert_eq!(w33a21_dispatch(0xd, 0xe, || {}, || 0xe, || 7), 7);
    }
}
