#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave21/agent20.rs
// ===========================================================================
// Wave 21 agent 20: MSVC STL deallocation / copy helpers.
// Translated: 0x005fdfe0, 0x005fde60, 0x005fd530.

/// Big-allocation threshold of the MSVC debug-free allocator (`0x1000`).
pub const W21A20_BIG_ALLOC: usize = 0x1000;

/// Result of the aligned-free adjustment: `(block_address, size)` handed to
/// the raw deallocator, or `None` when the original-pointer check fails
/// (the retail code then aborts).
pub type W21a20FreePlan = Option<(usize, usize)>;

/// Shared adjustment: blocks of `0x1000` bytes or more keep the original
/// allocation pointer in the word before the user pointer.
fn w21a20_big_free(
    ptr: usize,
    size: usize,
    extra: usize,
    read_word: impl Fn(usize) -> usize,
) -> W21a20FreePlan {
    if size >= W21A20_BIG_ALLOC {
        let orig = read_word(ptr.wrapping_sub(4));
        if ptr.wrapping_sub(orig).wrapping_sub(4) > 0x1f {
            return None;
        }
        Some((orig, size + extra))
    } else {
        Some((ptr, size))
    }
}

/// `0x005fdfe0`: deallocation plan for an array of `count` 0x1c-byte
/// elements at `ptr`; `read_word` reads the stored original pointer.
pub fn w21a20_free_array_0x1c(
    ptr: usize,
    count: usize,
    read_word: impl Fn(usize) -> usize,
) -> W21a20FreePlan {
    w21a20_big_free(ptr, count.wrapping_mul(0x1c), 0x23, read_word)
}

/// What the node-teardown at `0x005fde60` does with the first list node.
#[derive(Debug, PartialEq, Eq)]
pub enum W21a20NodeFree {
    /// List was empty after sentinel self-link.
    Empty,
    /// Short string (capacity < 0x10): string reset, node (0x20 bytes) freed.
    ShortNode { node: usize },
    /// Heap string buffer free plan `(addr, size)`, or `None` on bad header.
    Buffer(W21a20FreePlan),
}

/// `0x005fde60`: first-node release decision of the list destructor.
/// `node_cap` is the string capacity at node+0x1c, `node_buf` the buffer
/// pointer at node+8.
pub fn w21a20_list_free_first(
    has_node: bool,
    node: usize,
    node_cap: usize,
    node_buf: usize,
    read_word: impl Fn(usize) -> usize,
) -> W21a20NodeFree {
    if !has_node {
        return W21a20NodeFree::Empty;
    }
    if node_cap < 0x10 {
        return W21a20NodeFree::ShortNode { node };
    }
    W21a20NodeFree::Buffer(w21a20_big_free(node_buf, node_cap + 1, 0x23, read_word))
}

/// `0x005fd530`: copy-construct a range of 0x1c-byte elements: the first
/// two bytes are copied verbatim, the string at +4 via `copy_string`.
/// Returns the number of elements written.
pub fn w21a20_copy_range_0x1c(
    src: &[[u8; 0x1c]],
    dst: &mut [[u8; 0x1c]],
    mut copy_string: impl FnMut(&[u8], &mut [u8]),
) -> usize {
    for (s, d) in src.iter().zip(dst.iter_mut()) {
        d[0] = s[0];
        d[1] = s[1];
        copy_string(&s[4..], &mut d[4..]);
    }
    src.len().min(dst.len())
}

#[cfg(test)]
mod tests_wave21_agent20 {
    use super::*;

    #[test]
    fn free_array() {
        assert_eq!(w21a20_free_array_0x1c(0x100, 2, |_| 0), Some((0x100, 0x38)));
        let n = 0x1000 / 0x1c + 1;
        assert_eq!(
            w21a20_free_array_0x1c(0x2000, n, |a| {
                assert_eq!(a, 0x1ffc);
                0x1fe0
            }),
            Some((0x1fe0, n * 0x1c + 0x23))
        );
        assert_eq!(w21a20_free_array_0x1c(0x2000, n, |_| 0x1000), None);
    }

    #[test]
    fn list_first() {
        assert_eq!(
            w21a20_list_free_first(false, 1, 0, 0, |_| 0),
            W21a20NodeFree::Empty
        );
        assert_eq!(
            w21a20_list_free_first(true, 5, 0xf, 0, |_| 0),
            W21a20NodeFree::ShortNode { node: 5 }
        );
        assert_eq!(
            w21a20_list_free_first(true, 5, 0x20, 0x300, |_| 0),
            W21a20NodeFree::Buffer(Some((0x300, 0x21)))
        );
        assert_eq!(
            w21a20_list_free_first(true, 5, 0xfff, 0x2000, |_| 0x1ff0),
            W21a20NodeFree::Buffer(Some((0x1ff0, 0x1000 + 0x23)))
        );
    }

    #[test]
    fn copy_range() {
        let mut s = [[0u8; 0x1c]; 2];
        s[1][0] = 7;
        s[1][1] = 9;
        s[1][4] = 3;
        let mut d = [[0u8; 0x1c]; 2];
        let n = w21a20_copy_range_0x1c(&s, &mut d, |a, b| b[0] = a[0]);
        assert_eq!(n, 2);
        assert_eq!(d[1][..5], [7, 9, 0, 0, 3]);
    }
}
