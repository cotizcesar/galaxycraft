#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent50.rs
// ===========================================================================
// Wave 24 batch 50: small vector helpers (capacity word growth, unique-insert of 16-byte ids).

/// Result of a unique-insert attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W24a50Insert {
    /// Id already present; nothing changed.
    Present,
    /// Appended in place.
    Appended,
    /// Storage full; caller must reallocate to this new capacity word.
    Grow(u32),
}

/// `0x007d8800`: reserve check on a vector whose word at +8 holds the capacity in
/// the low 31 bits.  If `cap < wanted`, the new capacity is `cap + cap/2` (at
/// least `wanted`) and the capacity bits are rewritten (top bit preserved).
/// Returns the new capacity word when a reallocation is required.
pub fn w24a50_reserve_word(cap_word: u32, wanted: u32) -> Option<u32> {
    let cap = cap_word & 0x7fff_ffff;
    if cap >= wanted {
        return None;
    }
    let mut new_cap = ((cap_word >> 1) & 0x3fff_ffff).wrapping_add(cap);
    if new_cap < wanted {
        new_cap = wanted;
    }
    Some(((cap_word ^ new_cap) & 0x7fff_ffff) ^ cap_word)
}

/// Capacity growth shared by the two unique-insert routines: `cap + max(cap/2, 1)`
/// packed into the low 31 bits; `None` when `cap + 1` overflows.
fn w24a50_grow_word(cap_word: u32) -> Option<u32> {
    let cap = cap_word & 0x7fff_ffff;
    if cap.wrapping_add(1) <= cap {
        return None;
    }
    let half = (cap_word >> 1) & 0x3fff_ffff;
    let add = if cap.wrapping_add(1) <= cap.wrapping_add(half) {
        half
    } else {
        1
    };
    Some(cap_word ^ ((cap.wrapping_add(add) ^ cap_word) & 0x7fff_ffff))
}

/// `0x007de3d0`: appends `id` (16 bytes) to `items` unless an equal entry
/// exists.  When `items.len()` equals the capacity in `cap_word`, the append
/// is deferred and the grown capacity word is returned.
pub fn w24a50_insert_unique(
    items: &mut Vec<[u8; 16]>,
    cap_word: u32,
    id: [u8; 16],
) -> W24a50Insert {
    if items.contains(&id) {
        return W24a50Insert::Present;
    }
    if items.len() as u32 == cap_word & 0x7fff_ffff
        && let Some(w) = w24a50_grow_word(cap_word)
    {
        return W24a50Insert::Grow(w);
    }
    items.push(id);
    W24a50Insert::Appended
}

/// `0x007de560`: `w24a50_insert_unique` over a range of ids.  Stops and
/// returns `Grow` at the first id needing reallocation (the count of ids
/// consumed so far is the second tuple field).
pub fn w24a50_insert_range(
    items: &mut Vec<[u8; 16]>,
    cap_word: u32,
    ids: &[[u8; 16]],
) -> (W24a50Insert, usize) {
    for (i, id) in ids.iter().enumerate() {
        if let r @ W24a50Insert::Grow(_) = w24a50_insert_unique(items, cap_word, *id) {
            return (r, i);
        }
    }
    (W24a50Insert::Appended, ids.len())
}

#[cfg(test)]
mod tests_wave24_agent50 {
    use super::*;

    #[test]
    fn reserve() {
        assert_eq!(w24a50_reserve_word(8, 8), None);
        assert_eq!(w24a50_reserve_word(8, 9), Some(12));
        assert_eq!(w24a50_reserve_word(0x8000_0008, 100), Some(0x8000_0064));
    }

    #[test]
    fn unique() {
        let mut v = vec![[1u8; 16]];
        assert_eq!(
            w24a50_insert_unique(&mut v, 4, [1; 16]),
            W24a50Insert::Present
        );
        assert_eq!(
            w24a50_insert_unique(&mut v, 4, [2; 16]),
            W24a50Insert::Appended
        );
        assert_eq!(v.len(), 2);
        assert_eq!(
            w24a50_insert_unique(&mut v, 2, [3; 16]),
            W24a50Insert::Grow(3)
        );
        assert_eq!(v.len(), 2);
        assert_eq!(
            w24a50_insert_unique(&mut vec![], 0, [3; 16]),
            W24a50Insert::Grow(1)
        );
    }

    #[test]
    fn range() {
        let mut v = vec![];
        let ids = [[1u8; 16], [1; 16], [2; 16]];
        assert_eq!(
            w24a50_insert_range(&mut v, 8, &ids),
            (W24a50Insert::Appended, 3)
        );
        assert_eq!(v.len(), 2);
        let mut v2 = vec![];
        assert_eq!(w24a50_insert_range(&mut v2, 0, &ids).1, 0);
    }
}
