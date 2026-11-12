#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent42.rs
// ===========================================================================
// Wave 31 agent 42: small runtime container helpers.

/// `0x00835770`: resets a 10-word record: word 0 = `param`, word 1 untouched,
/// words 2..=9 cleared.
pub fn w31a42_reset_record(rec: &mut [u32; 10], param: u32) {
    rec[0] = param;
    for w in rec.iter_mut().skip(2) {
        *w = 0;
    }
}

/// `0x00838f20`: returns the node pointer produced by the lookup plus `0x10`
/// (payload offset inside a tree node).
pub fn w31a42_node_payload(node: u32) -> u32 {
    node.wrapping_add(0x10)
}

/// Result of the tree insert-position hint.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum W31a42Hint {
    /// `(node, slot offset)` pair written to the out parameter.
    Slot(u32, u32),
    /// Fall back to the full search (the unresolved slow path).
    Fallback,
}

/// `0x00834b00`: hinted insert-position lookup of an ordered tree.
/// `head` is the sentinel, `size` its count (`+0x10`), `root` the word at
/// `+4`, `leftmost` the word at `+8`.  `key_of(n)` reads `+0x10`,
/// `right_of(n)` reads `+8`; `prev`/`next` are the unresolved neighbour
/// walkers.
#[allow(clippy::too_many_arguments)]
pub fn w31a42_hint_position(
    head: u32,
    size: u32,
    root: u32,
    leftmost: u32,
    hint: u32,
    key: u32,
    key_of: impl Fn(u32) -> u32,
    right_of: impl Fn(u32) -> u32,
    prev: impl Fn(u32) -> u32,
    next: impl Fn(u32) -> u32,
) -> W31a42Hint {
    if size == 0 {
        return W31a42Hint::Slot(head, head.wrapping_add(4));
    }
    if hint == root {
        if key < key_of(hint) {
            return W31a42Hint::Slot(root, root.wrapping_add(4));
        }
    } else if hint == head {
        if key_of(leftmost) < key {
            return W31a42Hint::Slot(leftmost, leftmost.wrapping_add(8));
        }
    } else if key < key_of(hint) {
        let p = prev(hint);
        if key_of(p) < key {
            if right_of(p) == 0 {
                return W31a42Hint::Slot(p, p.wrapping_add(8));
            }
            return W31a42Hint::Slot(hint, hint.wrapping_add(4));
        }
    } else if key_of(hint) < key {
        let n = next(hint);
        if n == head || key < key_of(n) {
            if right_of(hint) == 0 {
                return W31a42Hint::Slot(hint, hint.wrapping_add(8));
            }
            return W31a42Hint::Slot(n, n.wrapping_add(4));
        }
    }
    W31a42Hint::Fallback
}

/// Appends `src` to `buf` one byte at a time, as the retail loop does (the
/// source may alias `buf`, so reads are taken from the pre-growth contents
/// when the index is in range).
fn w31a42_append(buf: &mut Vec<u8>, src: &[u8]) {
    buf.reserve(src.len());
    for &b in src {
        buf.push(b);
    }
}

/// `0x00844570`: initialises a 4-word header and a byte vector holding a copy
/// of `bytes`.
pub fn w31a42_init_header_bytes(header: &mut [u32; 4], words: [u32; 4], bytes: &[u8]) -> Vec<u8> {
    *header = words;
    let mut v = Vec::new();
    w31a42_append(&mut v, bytes);
    v
}

/// `0x0083bc80`: appends `src` to `buf`, then runs the follow-up call
/// (`FUN_0083b650`, passed as `tail`).
pub fn w31a42_append_then(buf: &mut Vec<u8>, src: &[u8], tail: impl FnOnce()) {
    w31a42_append(buf, src);
    tail();
}

/// `0x0083e5d0`: when the flag byte (`+0xe8`) is non-zero, fetches the current
/// value and passes it to the handler.
pub fn w31a42_flag_dispatch(flag: u8, fetch: impl FnOnce() -> u32, handler: impl FnOnce(u32)) {
    if flag != 0 {
        handler(fetch());
    }
}

#[cfg(test)]
mod tests_wave31_agent42 {
    use super::*;

    #[test]
    fn reset_and_payload() {
        let mut r = [7u32; 10];
        w31a42_reset_record(&mut r, 9);
        assert_eq!(r, [9, 7, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(w31a42_node_payload(0x100), 0x110);
    }

    #[test]
    fn hint() {
        let key_of = |n: u32| n;
        let z = |_n: u32| 0;
        assert_eq!(
            w31a42_hint_position(1, 0, 2, 3, 4, 5, key_of, z, z, z),
            W31a42Hint::Slot(1, 5)
        );
        assert_eq!(
            w31a42_hint_position(1, 3, 50, 10, 50, 20, key_of, z, z, z),
            W31a42Hint::Slot(50, 54)
        );
        assert_eq!(
            w31a42_hint_position(1, 3, 50, 10, 1, 20, key_of, z, z, z),
            W31a42Hint::Slot(10, 18)
        );
        assert_eq!(
            w31a42_hint_position(1, 3, 50, 10, 30, 40, key_of, z, z, |_| 60),
            W31a42Hint::Slot(30, 38)
        );
        assert_eq!(
            w31a42_hint_position(1, 3, 50, 10, 30, 10, key_of, z, |_| 5, z),
            W31a42Hint::Slot(5, 13)
        );
        assert_eq!(
            w31a42_hint_position(1, 3, 50, 10, 30, 30, key_of, z, z, z),
            W31a42Hint::Fallback
        );
    }

    #[test]
    fn bytes() {
        let mut h = [0; 4];
        let v = w31a42_init_header_bytes(&mut h, [1, 2, 3, 4], b"abc");
        assert_eq!((h, v.as_slice()), ([1, 2, 3, 4], &b"abc"[..]));
        let mut b = vec![1u8];
        let mut called = false;
        w31a42_append_then(&mut b, &[2, 3], || called = true);
        assert!(called);
        assert_eq!(b, [1, 2, 3]);
        let mut got = None;
        w31a42_flag_dispatch(0, || 5, |v| got = Some(v));
        assert_eq!(got, None);
        w31a42_flag_dispatch(1, || 5, |v| got = Some(v));
        assert_eq!(got, Some(5));
    }
}
