#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent26.rs
// ===========================================================================
// Wave 33 agent 26: token-buffer scanning helpers.

fn w33a26_is_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | 0x0b | b'\r' | 0x0c | b'\n')
}

/// `0x00ae43e0`: validates whitespace-separated tokens of a fetched buffer.
/// `fetch` is the result of the opaque fetch helper (status, buffer); any
/// status other than 1 is returned unchanged.  At most `max_tokens` tokens are
/// checked; each must have even length and satisfy `token_ok`.  Returns 1 on
/// success, -1 on a bad token or too few tokens.
pub fn w33a26_validate_tokens(
    fetch: Result<&[u8], i32>,
    max_tokens: u32,
    token_ok: &dyn Fn(&[u8]) -> bool,
) -> i32 {
    let buf = match fetch {
        Ok(b) => b,
        Err(s) => return s,
    };
    let end = buf.len();
    let mut p = 0usize;
    let mut count = 0u32;
    while p < end {
        if count >= max_tokens {
            return 1;
        }
        while p < end && w33a26_is_ws(buf[p]) {
            p += 1;
        }
        if p == end {
            break;
        }
        let mut q = p;
        while q < end && !w33a26_is_ws(buf[q]) {
            q += 1;
        }
        if q == p || (q - p) & 1 != 0 || !token_ok(&buf[p..q]) {
            return -1;
        }
        count += 1;
        p = q;
    }
    if count < max_tokens { -1 } else { 1 }
}

/// `0x00ae4600`: counts token groups in a fetched buffer (a group is a run of
/// non-space bytes followed by a run of whitespace; a leading whitespace run
/// counts as one group).  Returns 0 when the fetch status is not 1.
pub fn w33a26_count_tokens(fetch: Result<&[u8], i32>) -> i32 {
    let buf = match fetch {
        Ok(b) => b,
        Err(_) => return 0,
    };
    let end = buf.len();
    let mut p = 0usize;
    let mut n = 0;
    while p < end {
        while p < end {
            if w33a26_is_ws(buf[p]) {
                while p < end && w33a26_is_ws(buf[p]) {
                    p += 1;
                }
                break;
            }
            p += 1;
        }
        n += 1;
    }
    n
}

#[cfg(test)]
mod tests_wave33_agent26 {
    use super::*;

    #[test]
    fn validate() {
        let ok = |_: &[u8]| true;
        assert_eq!(w33a26_validate_tokens(Ok(b"ab  cdef\n"), 2, &ok), 1);
        assert_eq!(w33a26_validate_tokens(Ok(b"abc"), 1, &ok), -1);
        assert_eq!(w33a26_validate_tokens(Ok(b"ab"), 2, &ok), -1);
        assert_eq!(w33a26_validate_tokens(Ok(b"ab cd"), 1, &ok), 1);
        assert_eq!(w33a26_validate_tokens(Ok(b"ab"), 1, &|_| false), -1);
        assert_eq!(w33a26_validate_tokens(Err(7), 1, &ok), 7);
    }

    #[test]
    fn count() {
        assert_eq!(w33a26_count_tokens(Ok(b"a b  c")), 3);
        assert_eq!(w33a26_count_tokens(Ok(b"a b ")), 2);
        assert_eq!(w33a26_count_tokens(Ok(b"")), 0);
        assert_eq!(w33a26_count_tokens(Err(0)), 0);
    }
}
