#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent22.rs
// ===========================================================================
// Wave 22 agent 22: eud array/memory-view helpers.

/// `0x006a3940`: applies `value` to `*out` by mode.  Mode 0 stores
/// directly; otherwise `fetch` (virtual slot +8) must succeed first, then
/// mode 1 adds and mode -1 saturating-subtracts (clamps to 0 unless
/// `value < *out`).  Other modes leave the fetched value.
pub fn w22a22_apply_mode(
    out: &mut u32,
    value: u32,
    mode: i32,
    fetch: impl FnOnce(&mut u32) -> bool,
) -> bool {
    if mode == 0 {
        *out = value;
        return true;
    }
    if !fetch(out) {
        return false;
    }
    if mode == 1 {
        *out = out.wrapping_add(value);
    } else if mode == -1 {
        *out = if value < *out { *out - value } else { 0 };
    }
    true
}

/// `0x006a7a80`: reads a u32 at `offset` of `buf` (the view's base pointer,
/// `None` when unset -> false).  `|base - offset|` < 4 bytes gives a
/// partial copy into a zeroed word.
pub fn w22a22_read_word(buf: Option<&[u8]>, base: u32, offset: u32) -> Option<u32> {
    let buf = buf?;
    let dist = base.abs_diff(offset) as usize;
    let off = offset as usize;
    let mut w = [0u8; 4];
    let n = dist.min(4);
    w[..n].copy_from_slice(buf.get(off..off + n)?);
    Some(u32::from_le_bytes(w))
}

/// `0x006a7cb0`: read-modify-write of a u32 at `offset` (modes 0 set,
/// 1 add, -1 saturating subtract to 0).  Distances under 4 bytes only
/// round-trip that many bytes.  Returns false when no buffer.
pub fn w22a22_write_word(
    buf: Option<&mut [u8]>,
    base: u32,
    offset: u32,
    value: u32,
    mode: i32,
) -> bool {
    let Some(buf) = buf else { return false };
    let dist = (base.abs_diff(offset) as usize).min(4);
    let off = offset as usize;
    let Some(slot) = buf.get_mut(off..off + 4) else {
        return false;
    };
    let mut cur = u32::from_le_bytes([slot[0], slot[1], slot[2], slot[3]]);
    if dist < 4 {
        cur &= if dist == 0 {
            0
        } else {
            u32::MAX >> (32 - 8 * dist)
        };
    }
    let new = match mode {
        0 => value,
        1 => cur.wrapping_add(value),
        -1 => {
            if value < cur {
                cur - value
            } else {
                0
            }
        }
        _ => cur,
    };
    slot[..4].copy_from_slice(&new.to_le_bytes());
    true
}

/// Tree node for `w22a22_find_range`: child links, key and the range
/// record `(start, len)` the node points at.
#[derive(Clone, Copy, Debug)]
pub struct W22a22Node {
    pub left: Option<usize>,
    pub right: Option<usize>,
    pub key: u32,
    pub start: u32,
    pub len: u32,
}

/// `0x006a7d50`: lower-bound search by `key` (`root`); an exact key hit
/// returns that node; else the predecessor (`prev`, the tree decrement
/// helper; `None` = header) is returned if `start <= key < start+len`.
pub fn w22a22_find_range(
    nodes: &[W22a22Node],
    root: Option<usize>,
    first: Option<usize>,
    key: u32,
    prev: impl Fn(Option<usize>) -> Option<usize>,
) -> Option<usize> {
    let mut best: Option<usize> = None;
    let mut cur = root;
    while let Some(i) = cur {
        if nodes[i].key < key {
            cur = nodes[i].right;
        } else {
            best = Some(i);
            cur = nodes[i].left;
        }
    }
    if let Some(b) = best
        && nodes[b].start == key
    {
        return Some(b);
    }
    if best != first {
        let p = prev(best)?;
        let n = &nodes[p];
        if n.start <= key && key < n.start.wrapping_add(n.len) {
            return Some(p);
        }
    }
    None
}

#[cfg(test)]
mod tests_wave22_agent22 {
    use super::*;

    #[test]
    fn apply_mode() {
        let mut v = 5;
        assert!(w22a22_apply_mode(&mut v, 9, 0, |_| false));
        assert_eq!(v, 9);
        assert!(!w22a22_apply_mode(&mut v, 1, 1, |_| false));
        assert!(w22a22_apply_mode(&mut v, 3, 1, |_| true));
        assert_eq!(v, 12);
        assert!(w22a22_apply_mode(&mut v, 20, -1, |_| true));
        assert_eq!(v, 0);
        let mut v = 10;
        w22a22_apply_mode(&mut v, 4, -1, |_| true);
        assert_eq!(v, 6);
    }

    #[test]
    fn read_word() {
        let b = [1u8, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(w22a22_read_word(None, 0, 0), None);
        assert_eq!(w22a22_read_word(Some(&b), 100, 0), Some(0x04030201));
        assert_eq!(w22a22_read_word(Some(&b), 2, 0), Some(0x0201));
        assert_eq!(w22a22_read_word(Some(&b), 1, 1), Some(0));
    }

    #[test]
    fn write_word() {
        let mut b = [0u8; 8];
        assert!(!w22a22_write_word(None, 0, 0, 1, 0));
        assert!(w22a22_write_word(Some(&mut b), 100, 4, 7, 0));
        assert!(w22a22_write_word(Some(&mut b), 100, 4, 3, 1));
        assert_eq!(&b[4..], &[10, 0, 0, 0]);
        assert!(w22a22_write_word(Some(&mut b), 100, 4, 99, -1));
        assert_eq!(&b[4..], &[0, 0, 0, 0]);
    }

    #[test]
    fn find_range() {
        let n = |key, left, right, start, len| W22a22Node {
            left,
            right,
            key,
            start,
            len,
        };
        let nodes = [
            n(10, None, None, 10, 5),
            n(20, Some(0), Some(2), 20, 10),
            n(40, None, None, 40, 4),
        ];
        let prev = |i: Option<usize>| match i {
            None => Some(2),
            Some(1) => Some(0),
            Some(2) => Some(1),
            _ => None,
        };
        assert_eq!(
            w22a22_find_range(&nodes, Some(1), Some(0), 20, prev),
            Some(1)
        );
        assert_eq!(
            w22a22_find_range(&nodes, Some(1), Some(0), 25, prev),
            Some(1)
        );
        assert_eq!(w22a22_find_range(&nodes, Some(1), Some(0), 35, prev), None);
        assert_eq!(w22a22_find_range(&nodes, Some(1), Some(0), 5, prev), None);
    }
}
