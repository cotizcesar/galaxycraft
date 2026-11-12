#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent25.rs
// ===========================================================================
// Wave 33 agent 25 translations.

/// Model of the buffer-state record used by `0x009f34a0` (offsets 0x44..0x54).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W33a25State {
    /// +0x44 buffer handle.
    pub buf: u32,
    /// +0x48 cleared.
    pub f48: u32,
    /// +0x4c consumed size.
    pub f4c: u32,
    /// +0x50 pending size.
    pub f50: u32,
    /// +0x54 chain of (node handle, payload handle), in link order.
    pub chain: Vec<(u32, u32)>,
}

/// `0x009f34a0`: frees every chain node (payload first, then the node),
/// then folds the pending size into the consumed size via `realloc`
/// (returns 0xffffff75 when realloc yields 0), and clears +0x48/+0x54.
pub fn w33a25_release_chain(
    st: &mut W33a25State,
    mut free: impl FnMut(u32),
    realloc: impl FnOnce(u32, u32) -> u32,
) -> u32 {
    for (node, payload) in std::mem::take(&mut st.chain) {
        free(payload);
        free(node);
    }
    let mut ret = 0;
    if st.f50 != 0 {
        let total = st.f4c.wrapping_add(st.f50);
        let h = realloc(st.buf, total);
        st.f4c = total;
        ret = 0xffff_ff75;
        st.buf = h;
        st.f50 = 0;
        if h != 0 {
            ret = 0;
        }
    }
    st.f48 = 0;
    ret
}

/// `0x00ae4340`: after `locate` (callee result 1 with start,len) positions a
/// cursor at start+len, scans to the next `\n` (consumed) or the end of
/// `buf[..size]`.  Returns the cursor and the remaining byte count.
pub fn w33a25_next_line(
    buf: &[u8],
    size: usize,
    locate: impl FnOnce() -> Option<(usize, usize)>,
) -> Option<(usize, i64)> {
    let (a, b) = locate()?;
    let end = size.min(buf.len());
    let mut p = a + b;
    while p < end {
        if buf[p] == b'\n' {
            p += 1;
            break;
        }
        p += 1;
    }
    Some((p, end as i64 - p as i64))
}

#[cfg(test)]
mod tests_wave33_agent25 {
    use super::*;

    #[test]
    fn release_chain_ok_and_fail() {
        let mut st = W33a25State {
            buf: 1,
            f4c: 4,
            f50: 6,
            f48: 9,
            chain: vec![(10, 20), (11, 21)],
        };
        let mut freed = vec![];
        let r = w33a25_release_chain(
            &mut st,
            |x| freed.push(x),
            |h, n| {
                assert_eq!((h, n), (1, 10));
                7
            },
        );
        assert_eq!(r, 0);
        assert_eq!(freed, vec![20, 10, 21, 11]);
        assert_eq!((st.buf, st.f4c, st.f50, st.f48), (7, 10, 0, 0));
        st.f50 = 1;
        assert_eq!(w33a25_release_chain(&mut st, |_| {}, |_, _| 0), 0xffff_ff75);
        st.f50 = 0;
        assert_eq!(w33a25_release_chain(&mut st, |_| {}, |_, _| 5), 0);
    }

    #[test]
    fn next_line_cases() {
        let b = b"ab\ncd";
        assert_eq!(w33a25_next_line(b, 5, || Some((0, 1))), Some((3, 2)));
        assert_eq!(w33a25_next_line(b, 5, || Some((3, 2))), Some((5, 0)));
        assert_eq!(w33a25_next_line(b, 5, || None), None);
        assert_eq!(w33a25_next_line(b, 5, || Some((6, 1))), Some((7, -2)));
    }
}
