#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent24.rs
// ===========================================================================
// Wave 33 translation batch 24: bit-length byte size, slot lookup, bump allocator.

/// `0x009f01e0`: byte offset after a bit cursor: `base + ceil(bits / 8)`
/// with the signed-division rounding of the original (`bits` signed).
pub fn w33a24_bit_cursor_end(base: i32, bits: i32) -> i32 {
    let t = bits.wrapping_add(7);
    (t.wrapping_add(((t >> 31) as u32 & 7) as i32) >> 3).wrapping_add(base)
}

/// `0x009f11a0`: when `count > 0`, looks up the index with `find` and
/// returns `values[index]`; otherwise (or on a negative index) `-1`.
pub fn w33a24_lookup_value(count: i32, values: &[i32], key: u32, find: impl Fn(u32) -> i32) -> i32 {
    if count > 0 {
        let i = find(key);
        if i >= 0 {
            return values.get(i as usize).copied().unwrap_or(-1);
        }
    }
    -1
}

/// Arena state used by `0x009f3420` (offsets 0x44..0x54 of the owner).
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct W33a24Arena {
    /// +0x44 current block (0 = none)
    pub block: i32,
    /// +0x48 used bytes
    pub used: i32,
    /// +0x4c block capacity
    pub cap: i32,
    /// +0x50 total bytes in retired blocks
    pub retired_total: i32,
    /// +0x54 head of retired block list (node address)
    pub retired_head: i32,
}

/// `0x009f3420`: bump-allocates `size` (rounded up to 8) from the current
/// block, otherwise retires it onto a list (node alloc of 8 bytes via
/// `alloc`, storing `[block, next]` through `write_node`) and allocates a
/// fresh block of the rounded size.  Returns the address or 0 on failure.
pub fn w33a24_arena_alloc(
    arena: &mut W33a24Arena,
    size: i32,
    mut alloc: impl FnMut(i32) -> i32,
    mut write_node: impl FnMut(i32, i32, i32),
) -> i32 {
    let mut off = arena.used;
    let rounded = (size.wrapping_add(7) as u32 & 0xffff_fff8) as i32;
    if arena.cap < off.wrapping_add(rounded) {
        if arena.block != 0 {
            let node = alloc(8);
            if node == 0 {
                return 0;
            }
            arena.retired_total = arena.retired_total.wrapping_add(arena.used);
            write_node(node, arena.block, arena.retired_head);
            arena.retired_head = node;
        }
        arena.cap = rounded;
        arena.block = alloc(rounded);
        if arena.block == 0 {
            return 0;
        }
        off = 0;
    }
    arena.used = off.wrapping_add(rounded);
    arena.block.wrapping_add(off)
}

#[cfg(test)]
mod tests_wave33_agent24 {
    use super::*;

    #[test]
    fn cursor() {
        assert_eq!(w33a24_bit_cursor_end(10, 9), 12);
        assert_eq!(w33a24_bit_cursor_end(10, 0), 10);
    }

    #[test]
    fn lookup() {
        assert_eq!(w33a24_lookup_value(0, &[5], 1, |_| 0), -1);
        assert_eq!(w33a24_lookup_value(1, &[5], 1, |_| 0), 5);
        assert_eq!(w33a24_lookup_value(1, &[5], 1, |_| -1), -1);
    }

    #[test]
    fn arena() {
        let mut a = W33a24Arena::default();
        let mut next = 1000;
        let mut nodes = vec![];
        let r = w33a24_arena_alloc(
            &mut a,
            10,
            |n| {
                next += 100;
                let _ = n;
                next
            },
            |n, b, h| nodes.push((n, b, h)),
        );
        assert_eq!(r, 1100);
        assert_eq!(a.used, 16);
        let r2 = w33a24_arena_alloc(&mut a, 20, |_| 5000, |n, b, h| nodes.push((n, b, h)));
        assert_eq!(r2, 5000);
        assert_eq!(nodes, vec![(5000, 1100, 0)]);
        assert_eq!(a.retired_total, 16);
    }
}
