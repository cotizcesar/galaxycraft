#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent15.rs
// ===========================================================================
// Wave 25 agent 15: MSVC STL helpers (string init, growth, tree walk).

/// SSO capacity word written by empty-string constructors.
pub const W25A15_EMPTY_STR_CAP: u32 = 0x8000_000f;

/// Empty small-string header (size, capacity word, first inline byte).
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W25a15EmptyStr {
    pub size: u32,
    pub cap: u32,
    pub first_byte: u8,
}

/// `0x007ec790`, `0x007ec910`, `0x007ec9b0`: the shared inline-empty-string
/// initialisation (size 0, cap `0x8000000f`, byte 0 cleared) that precedes the
/// copy-from-source call.  Returns the initialised header.
pub fn w25a15_init_empty_string() -> W25a15EmptyStr {
    W25a15EmptyStr {
        size: 0,
        cap: W25A15_EMPTY_STR_CAP,
        first_byte: 0,
    }
}

/// `0x007ec610`: capacity-word update for a `reserve`.  `cap_word` holds the
/// capacity in the low 31 bits; when `wanted` exceeds it, the new capacity is
/// `cap + cap/2` (or `wanted` if larger), keeping the top flag bit.  Returns
/// the new word, or `None` when no growth is needed.
pub fn w25a15_grow_capacity_word(cap_word: u32, wanted: u32) -> Option<u32> {
    if (cap_word & 0x7fff_ffff) >= wanted {
        return None;
    }
    let mut new_cap = ((cap_word >> 1) & 0x3fff_ffff).wrapping_add(cap_word & 0x7fff_ffff);
    if new_cap < wanted {
        new_cap = wanted;
    }
    Some(((cap_word ^ new_cap) & 0x7fff_ffff) ^ cap_word)
}

/// `0x007edb70`: if the mode word equals 1, store `value` into the slot.
pub fn w25a15_store_if_mode_one(mode: u32, slot: &mut u32, value: u32) {
    if mode == 1 {
        *slot = value;
    }
}

/// `0x007ed860`: debug-fill marker written into the object at offset `0xd4`.
pub const W25A15_DEBUG_FILL: u32 = 0xbaad_f00d;

/// `0x007ed860`: stamps the `0xbaadf00d` marker over the field.
pub fn w25a15_stamp_debug_fill(field: &mut u32) {
    *field = W25A15_DEBUG_FILL;
}

/// Red-black tree node (MSVC layout: parent, left, right, ..., key at +0x10).
#[derive(Debug, Clone, Copy)]
pub struct W25a15Node {
    pub parent: usize,
    pub left: Option<usize>,
    pub right: Option<usize>,
    pub key: u32,
}

/// In-order successor used inside `0x007ed290` (header node is the one whose
/// right link points back at the root chain; modelled as in the pseudocode).
pub fn w25a15_tree_next(nodes: &[W25a15Node], cur: usize) -> usize {
    if let Some(mut r) = nodes[cur].right {
        while let Some(l) = nodes[r].left {
            r = l;
        }
        return r;
    }
    let mut cur = cur;
    let mut p = nodes[cur].parent;
    if Some(cur) == nodes[p].right {
        loop {
            cur = p;
            p = nodes[cur].parent;
            if Some(cur) != nodes[p].right {
                break;
            }
        }
    }
    if nodes[cur].right != Some(p) {
        cur = p;
    }
    cur
}

/// `0x007ed290`: walks two sorted tree ranges `[a, a_end)` and `[b, b_end)`
/// in lockstep, advancing the smaller key.  A shared key aborts (the
/// original calls a non-returning handler): `Err(key)`.  On exhaustion returns
/// `Ok(())` (the caller's out value is stored).
pub fn w25a15_ranges_disjoint(
    nodes: &[W25a15Node],
    mut a: usize,
    a_end: usize,
    mut b: usize,
    b_end: usize,
) -> Result<(), u32> {
    if a == a_end {
        return Ok(());
    }
    while b != b_end {
        let (ka, kb) = (nodes[a].key, nodes[b].key);
        if ka < kb {
            a = w25a15_tree_next(nodes, a);
        } else if ka == kb {
            return Err(ka);
        } else {
            b = w25a15_tree_next(nodes, b);
        }
        if a == a_end {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests_wave25_agent15 {
    use super::*;

    #[test]
    fn empty_string_and_growth() {
        let s = w25a15_init_empty_string();
        assert_eq!((s.size, s.cap, s.first_byte), (0, 0x8000_000f, 0));
        assert_eq!(w25a15_grow_capacity_word(15, 10), None);
        assert_eq!(
            w25a15_grow_capacity_word(0x8000_000f, 16),
            Some(0x8000_0016)
        );
        assert_eq!(w25a15_grow_capacity_word(15, 100), Some(100));
    }

    #[test]
    fn small_helpers() {
        let mut v = 0;
        w25a15_store_if_mode_one(2, &mut v, 5);
        assert_eq!(v, 0);
        w25a15_store_if_mode_one(1, &mut v, 5);
        assert_eq!(v, 5);
        w25a15_stamp_debug_fill(&mut v);
        assert_eq!(v, 0xbaad_f00d);
    }

    // Header node 0 (root=1); chain tree: 1(key 1) -> right 2(key 3).
    fn tree() -> Vec<W25a15Node> {
        vec![
            W25a15Node {
                parent: 1,
                left: None,
                right: Some(1),
                key: 0,
            },
            W25a15Node {
                parent: 0,
                left: None,
                right: Some(2),
                key: 1,
            },
            W25a15Node {
                parent: 1,
                left: None,
                right: None,
                key: 3,
            },
        ]
    }

    #[test]
    fn tree_walk() {
        let t = tree();
        assert_eq!(w25a15_tree_next(&t, 1), 2);
        assert_eq!(w25a15_ranges_disjoint(&t, 1, 2, 2, 0), Ok(()));
        assert_eq!(w25a15_ranges_disjoint(&t, 1, 0, 2, 0), Err(3));
    }
}
