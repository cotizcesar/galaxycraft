#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent113.rs
// ===========================================================================
// Wave 31 translation batch 113: small vector-growth helpers.

/// Packed capacity word growth shared by the vector helpers: low 31 bits are
/// the capacity, bit 31 is a flag that is preserved.  Returns the new packed
/// word for a request that needs at least `needed` elements.
pub fn w31a113_grow_word(word: u32, needed: u32) -> u32 {
    let cap = word & 0x7fff_ffff;
    let mut new_cap = (word >> 1 & 0x3fff_ffff).wrapping_add(cap);
    if new_cap < needed {
        new_cap = needed;
    }
    ((word ^ new_cap) & 0x7fff_ffff) ^ word
}

/// `0x00c5dc30`: reserve check.  When the capacity (low 31 bits of `word`)
/// is below `wanted`, returns `Some(new_word)` (the retail code then calls
/// the reallocation routine); otherwise `None`.
pub fn w31a113_reserve_check(word: u32, wanted: u32) -> Option<u32> {
    if (word & 0x7fff_ffff) < wanted {
        Some(w31a113_grow_word(word, wanted))
    } else {
        None
    }
}

/// `0x00c5bc50`: push a 0x20-byte record `[0,0,0,p1,p4,p2,p3,p5]` (u32
/// words).  When full (`len == cap`) the retail code updates the packed word
/// and tail-calls the reallocator; here that is `Err(new_word)`.
pub fn w31a113_push_rec32(
    rows: &mut Vec<[u32; 8]>,
    word: &mut u32,
    p: [u32; 5],
) -> Result<(), u32> {
    let cap = *word & 0x7fff_ffff;
    if rows.len() as u32 == cap && cap < cap.wrapping_add(1) {
        let half = *word >> 1 & 0x3fff_ffff;
        let delta = if cap < cap + half { half } else { 1 };
        *word = ((cap.wrapping_add(delta) ^ *word) & 0x7fff_ffff) ^ *word;
        return Err(*word);
    }
    rows.push([0, 0, 0, p[0], p[3], p[1], p[2], p[4]]);
    Ok(())
}

/// `0x00c5bda0`: conditional push of a 0x14-byte record `[p1,p4,p2,p3,p5]`,
/// only when bit 3 of the flags byte is set.
pub fn w31a113_push_rec20(
    flags: u8,
    rows: &mut Vec<[u32; 5]>,
    word: &mut u32,
    p: [u32; 5],
) -> Result<(), u32> {
    if flags & 8 == 0 {
        return Ok(());
    }
    let cap = *word & 0x7fff_ffff;
    if rows.len() as u32 == cap && cap < cap.wrapping_add(1) {
        let half = *word >> 1 & 0x3fff_ffff;
        let delta = if cap < cap + half { half } else { 1 };
        *word = ((cap.wrapping_add(delta) ^ *word) & 0x7fff_ffff) ^ *word;
        return Err(*word);
    }
    rows.push([p[0], p[3], p[1], p[2], p[4]]);
    Ok(())
}

#[cfg(test)]
mod tests_wave31_agent113 {
    use super::*;

    #[test]
    fn reserve() {
        assert_eq!(w31a113_reserve_check(4, 4), None);
        assert_eq!(w31a113_reserve_check(4, 10), Some(10));
        assert_eq!(w31a113_reserve_check(0x8000_0008, 9), Some(0x8000_000c));
    }

    #[test]
    fn push32() {
        let mut v = Vec::new();
        let mut w = 2u32;
        assert!(w31a113_push_rec32(&mut v, &mut w, [1, 2, 3, 4, 5]).is_ok());
        assert_eq!(v[0], [0, 0, 0, 1, 4, 2, 3, 5]);
        assert!(w31a113_push_rec32(&mut v, &mut w, [0; 5]).is_ok());
        assert_eq!(w31a113_push_rec32(&mut v, &mut w, [0; 5]), Err(3));
    }

    #[test]
    fn push20() {
        let mut v = Vec::new();
        let mut w = 1u32;
        assert!(w31a113_push_rec20(0, &mut v, &mut w, [1; 5]).is_ok());
        assert!(v.is_empty());
        assert!(w31a113_push_rec20(8, &mut v, &mut w, [1, 2, 3, 4, 5]).is_ok());
        assert_eq!(v[0], [1, 4, 2, 3, 5]);
        assert_eq!(w31a113_push_rec20(8, &mut v, &mut w, [0; 5]), Err(2));
    }
}
