#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent17.rs
// ===========================================================================
// Wave 24 agent 17: string-capacity, refcount and small geometry helpers.

/// `0x0074eac0`: MSVC string grow policy.  `cap_field` is the packed
/// capacity word at +8 (low 31 bits capacity, top bit flag).  Returns the
/// new packed word when growth to `wanted` is needed, else `None`.
pub fn w24a17_grow_capacity(cap_field: u32, wanted: u32) -> Option<u32> {
    if (cap_field & 0x7fff_ffff) >= wanted {
        return None;
    }
    let mut new_cap = ((cap_field >> 1) & 0x3fff_ffff).wrapping_add(cap_field & 0x7fff_ffff);
    if new_cap < wanted {
        new_cap = wanted;
    }
    Some(((cap_field ^ new_cap) & 0x7fff_ffff) ^ cap_field)
}

/// `0x0074fbc0`: identical capacity growth policy for the narrow-string
/// variant.
pub fn w24a17_grow_capacity_narrow(cap_field: u32, wanted: u32) -> Option<u32> {
    w24a17_grow_capacity(cap_field, wanted)
}

/// `0x0074ea60`: drops one reference of a shared block; returns
/// `(new_count, freed)`.  The block is released when the previous count was 1.
pub fn w24a17_release_ref(count: &mut i32) -> bool {
    let prev = *count;
    *count = prev.wrapping_sub(1);
    prev == 1
}

/// `0x0074f610`: true when the path is non-empty and does not already end
/// with `\` or `/`, i.e. a separator must be appended.
pub fn w24a17_needs_separator(path: &[u8]) -> bool {
    matches!(path.last(), Some(&c) if c != b'\\' && c != b'/')
}

/// `0x0074eda0`: same trailing-separator test on the wide-copy path variant.
pub fn w24a17_needs_separator_copy(path: &[u8]) -> bool {
    matches!(path.last(), Some(&c) if c != b'\\' && c != b'/')
}

/// `0x0074faa0`: substring length clamp: `min(len, found_end - start)` where
/// `found_end` is the scan result over the buffer.
pub fn w24a17_clamped_len(start: usize, found_end: usize, len: usize) -> usize {
    len.min(found_end.wrapping_sub(start))
}

/// `0x00750640`: offsets a point `(x, y)` by `(w1 + w2 + 1, h1 + h2 + 1)`
/// taken from two packed i16 pairs, mirrored to negative when the point is in
/// the far half of the screen (`half_w`, `half_h` are width/height >> 1).
pub fn w24a17_mirrored_offset(
    pair_a: u32,
    pair_b: u32,
    pos: (i16, i16),
    half_w: u32,
    half_h: u32,
) -> (i32, i32) {
    let mut dx = (pair_a as u16 as i16)
        .wrapping_add(pair_b as u16 as i16)
        .wrapping_add(1) as i32;
    let mut dy = ((pair_b >> 16) as u16 as i16)
        .wrapping_add(1)
        .wrapping_add((pair_a >> 16) as u16 as i16) as i32;
    let (x, y) = (pos.0 as i32, pos.1 as i32);
    if half_h as i32 <= y {
        dy = -dy;
    }
    if half_w as i32 <= x {
        dx = -dx;
    }
    (dx + x, dy + y)
}

#[cfg(test)]
mod tests_wave24_agent17 {
    use super::*;

    #[test]
    fn grow() {
        assert_eq!(w24a17_grow_capacity(0x8000_000f, 10), None);
        assert_eq!(w24a17_grow_capacity(0x8000_000f, 16), Some(0x8000_0016));
        assert_eq!(w24a17_grow_capacity_narrow(15, 100), Some(100));
    }

    #[test]
    fn refs_and_paths() {
        let mut c = 2;
        assert!(!w24a17_release_ref(&mut c));
        assert!(w24a17_release_ref(&mut c));
        assert!(w24a17_needs_separator(b"a"));
        assert!(!w24a17_needs_separator(b"a/"));
        assert!(!w24a17_needs_separator_copy(b""));
        assert_eq!(w24a17_clamped_len(2, 10, 5), 5);
        assert_eq!(w24a17_clamped_len(2, 4, 5), 2);
    }

    #[test]
    fn mirrored() {
        assert_eq!(
            w24a17_mirrored_offset(0x0002_0001, 0x0004_0003, (10, 10), 320, 240),
            (15, 17)
        );
        assert_eq!(
            w24a17_mirrored_offset(0x0002_0001, 0x0004_0003, (400, 300), 320, 240),
            (395, 293)
        );
    }
}
