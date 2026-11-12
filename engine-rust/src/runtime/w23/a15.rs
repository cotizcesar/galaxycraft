#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent15.rs
// ===========================================================================
// Wave 23 agent 15: small runtime helpers (size gates, path clamp, list node).

/// Maximum path copy length used by the path-copy helpers (`0x103`).
pub const W23A15_PATH_COPY_MAX: usize = 0x103;

/// Length clamp shared by the bounded path copies: `min(strlen, 0x103)`.
fn w23a15_clamped_len(s: &[u8]) -> usize {
    let n = s.iter().position(|&b| b == 0).unwrap_or(s.len());
    n.min(W23A15_PATH_COPY_MAX)
}

/// `0x0070e100`: bounded path copy; returns the clamped byte count
/// (`min(strlen, 0x103)`) copied from `src` into `dst`.
pub fn w23a15_copy_path_e100(dst: &mut [u8], src: &[u8]) -> usize {
    let n = w23a15_clamped_len(src).min(dst.len());
    dst[..n].copy_from_slice(&src[..n]);
    n
}

/// `0x0070f8f0`: same clamped copy into the global path buffer
/// (offset `+0xc4c` of the state block); returns the copied count.
pub fn w23a15_copy_path_f8f0(dst: &mut [u8], src: &[u8]) -> usize {
    w23a15_copy_path_e100(dst, src)
}

/// `0x0070f710`: validates the 19 fixed tables in order, stopping at the first
/// failure.  `check(table_index, size)` models the per-table validator; the
/// sizes are `0x390, 0x1c8, 0xe4, 0x1c8 x4, 0x104 x2, 0x7a x6, 0x58 x4`.
pub fn w23a15_validate_tables(mut check: impl FnMut(usize, u32) -> bool) -> bool {
    const SIZES: [u32; 19] = [
        0x390, 0x1c8, 0xe4, 0x1c8, 0x1c8, 0x1c8, 0x1c8, 0x104, 0x104, 0x7a, 0x7a, 0x7a, 0x7a, 0x7a,
        0x7a, 0x58, 0x58, 0x58, 0x58,
    ];
    SIZES.iter().enumerate().all(|(i, &s)| check(i, s))
}

/// `0x0070fb50`: size gate; succeeds only when `size == 8` and the reader
/// returns true.
pub fn w23a15_read_exact8(size: u32, read: impl FnOnce() -> bool) -> bool {
    size == 8 && read()
}

/// `0x0070fda0`: size gate for `0x500`-byte records.
pub fn w23a15_read_0x500(size: u32, read: impl FnOnce() -> bool) -> bool {
    size == 0x500 && read()
}

/// `0x0070fdd0`: size gate for `0x13ec`-byte records.
pub fn w23a15_read_0x13ec(size: u32, read: impl FnOnce() -> bool) -> bool {
    size == 0x13ec && read()
}

/// `0x00710900`: size gate for 4-byte values.
pub fn w23a15_read_exact4(size: u32, read: impl FnOnce() -> bool) -> bool {
    size == 4 && read()
}

/// `0x0070fe30`: returns the record count when `size` is a multiple of
/// `0x960`, else `None`.
pub fn w23a15_record_count_0x960(size: u32) -> Option<u32> {
    (size.is_multiple_of(0x960)).then_some(size / 0x960)
}

/// `0x0070eaf0`: runs `release` only when `param == 0`.
pub fn w23a15_release_if_zero(param: i32, release: impl FnOnce()) {
    if param == 0 {
        release();
    }
}

/// Doubly linked node built by `0x007116c0` (stride `0x14`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W23a15Node {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub prev_index: Option<usize>,
    pub next_index: Option<usize>,
}

/// `0x007116c0`: appends a node `{p3, p4, p2, prev, 0}` to `nodes` and links
/// the previous tail's `next`; returns the new tail (or `head` when
/// `p4 == 0`).
pub fn w23a15_push_node(
    nodes: &mut Vec<W23a15Node>,
    head: Option<usize>,
    p2: u32,
    p3: u32,
    p4: u32,
) -> Option<usize> {
    if p4 == 0 {
        return head;
    }
    let idx = nodes.len();
    nodes.push(W23a15Node {
        a: p3,
        b: p4,
        c: p2,
        prev_index: head,
        next_index: None,
    });
    if let Some(h) = head {
        nodes[h].next_index = Some(idx);
    }
    Some(idx)
}

/// `0x00711710`: releases the cached handle; returns 1 when none is held,
/// otherwise the release result, clearing the handle.
pub fn w23a15_release_handle(handle: &mut u32, release: impl FnOnce(u32) -> u32) -> u32 {
    if *handle == 0 {
        return 1;
    }
    let r = release(*handle);
    *handle = 0;
    r
}

/// `0x0070e9a0`: resets a state block (`u32` words).  With `flag` set it
/// writes `[100] = -1`, `[99] = 0xb` and returns code `0xb`; otherwise
/// `[1] = -1`, `[0] = 0` and returns code 0.
pub fn w23a15_reset_state(words: &mut [u32], flag: bool) -> u32 {
    if flag {
        words[100] = 0xffff_ffff;
        words[99] = 0xb;
        0xb
    } else {
        words[1] = 0xffff_ffff;
        words[0] = 0;
        0
    }
}

#[cfg(test)]
mod tests_wave23_agent15 {
    use super::*;

    #[test]
    fn path_copy_clamps() {
        let mut d = [0u8; 0x200];
        let src = vec![b'a'; 0x180];
        assert_eq!(w23a15_copy_path_e100(&mut d, &src), 0x103);
        assert_eq!(w23a15_copy_path_f8f0(&mut d, b"abc\0zz"), 3);
    }

    #[test]
    fn tables_stop_at_first_failure() {
        let mut seen = vec![];
        assert!(!w23a15_validate_tables(|i, s| {
            seen.push(s);
            i < 2
        }));
        assert_eq!(seen, vec![0x390, 0x1c8, 0xe4]);
        let mut n = 0;
        assert!(w23a15_validate_tables(|_, _| {
            n += 1;
            true
        }));
        assert_eq!(n, 19);
    }

    #[test]
    fn size_gates() {
        assert!(w23a15_read_exact8(8, || true));
        assert!(!w23a15_read_exact8(9, || true));
        assert!(w23a15_read_0x500(0x500, || true));
        assert!(!w23a15_read_0x500(0x500, || false));
        assert!(w23a15_read_0x13ec(0x13ec, || true));
        assert!(!w23a15_read_0x13ec(0, || true));
        assert!(w23a15_read_exact4(4, || true));
        assert!(!w23a15_read_exact4(5, || true));
    }

    #[test]
    fn record_count() {
        assert_eq!(w23a15_record_count_0x960(0x960 * 3), Some(3));
        assert_eq!(w23a15_record_count_0x960(0x961), None);
    }

    #[test]
    fn release_and_reset() {
        let mut hit = false;
        w23a15_release_if_zero(1, || hit = true);
        assert!(!hit);
        w23a15_release_if_zero(0, || hit = true);
        assert!(hit);
        let mut h = 0;
        assert_eq!(w23a15_release_handle(&mut h, |_| 7), 1);
        h = 5;
        assert_eq!(w23a15_release_handle(&mut h, |x| x + 1), 6);
        assert_eq!(h, 0);
        let mut w = [0u32; 101];
        assert_eq!(w23a15_reset_state(&mut w, true), 0xb);
        assert_eq!((w[100], w[99]), (u32::MAX, 0xb));
        assert_eq!(w23a15_reset_state(&mut w, false), 0);
        assert_eq!((w[1], w[0]), (u32::MAX, 0));
    }

    #[test]
    fn node_push() {
        let mut v = vec![];
        assert_eq!(w23a15_push_node(&mut v, None, 1, 2, 0), None);
        let a = w23a15_push_node(&mut v, None, 1, 2, 3);
        assert_eq!(a, Some(0));
        let b = w23a15_push_node(&mut v, a, 4, 5, 6);
        assert_eq!(b, Some(1));
        assert_eq!(v[0].next_index, Some(1));
        assert_eq!(v[1].prev_index, Some(0));
        assert_eq!((v[1].a, v[1].b, v[1].c), (5, 6, 4));
    }
}
