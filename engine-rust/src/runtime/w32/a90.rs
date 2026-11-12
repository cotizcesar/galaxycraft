#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent90.rs
// ===========================================================================
// Wave 32 translation batch 90.

/// `0x00a7d9b0`: advances from `start` while bytes are whitespace per
/// `is_space`, counting `\n` into `lines` when supplied; stops at the first
/// byte with bit 7 set or the first non-space.  Returns the stop index.
pub fn w32a90_skip_space(
    s: &[u8],
    start: usize,
    mut lines: Option<&mut i32>,
    is_space: impl Fn(u8) -> bool,
) -> usize {
    let mut i = start;
    loop {
        let c = s.get(i).copied().unwrap_or(0);
        if c & 0x80 != 0 || !is_space(c) {
            return i;
        }
        if c == b'\n' {
            if let Some(l) = lines.as_deref_mut() {
                *l += 1;
            }
        }
        i += 1;
    }
}

/// `0x00a7d9f0`: true when both are the same object or the bounded string
/// compare (`strncmp` result) is zero.
pub fn w32a90_str_equal(same_ptr: bool, a: &[u8], b: &[u8], n: usize) -> bool {
    if same_ptr {
        return true;
    }
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return false;
        }
        if x == 0 {
            break;
        }
    }
    true
}

/// `0x00a7dc40`: calls virtual slot `+0x18`; when nonzero the text writer
/// gets `None`, otherwise the buffered text from `buffered()`.  The writer is
/// called with the cdata flag (`param_1[0xd]` low byte).  Always returns 1.
pub fn w32a90_flush_text<T>(
    cdata_flag: u8,
    vslot: impl FnOnce() -> i32,
    buffered: impl FnOnce() -> T,
    write: impl FnOnce(Option<T>, u8),
) -> u32 {
    if vslot() != 0 {
        write(None, cdata_flag);
    } else {
        let v = buffered();
        write(Some(v), cdata_flag);
    }
    1
}

#[cfg(test)]
mod tests_wave32_agent90 {
    use super::*;

    #[test]
    fn skip() {
        let mut n = 0;
        let i = w32a90_skip_space(b" \n \nx", 0, Some(&mut n), |c| c == b' ' || c == b'\n');
        assert_eq!((i, n), (4, 2));
        assert_eq!(w32a90_skip_space(&[b' ', 0x80], 0, None, |c| c == b' '), 1);
    }

    #[test]
    fn eq() {
        assert!(w32a90_str_equal(true, b"a", b"b", 1));
        assert!(w32a90_str_equal(false, b"ab", b"ac", 1));
        assert!(!w32a90_str_equal(false, b"ab", b"ac", 2));
    }

    #[test]
    fn flush() {
        let mut got = None;
        assert_eq!(
            w32a90_flush_text(1, || 0, || 5, |v, f| got = Some((v, f))),
            1
        );
        assert_eq!(got, Some((Some(5), 1)));
        assert_eq!(
            w32a90_flush_text(0, || 1, || 5, |v, f| got = Some((v, f))),
            1
        );
        assert_eq!(got, Some((None, 0)));
    }
}
