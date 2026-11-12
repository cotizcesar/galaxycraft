#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave19/agent08.rs
// ===========================================================================
// Wave 19 agent 08: small string/vector/vtable helpers.

/// Vftable address stored by `0x005504a0` (`*param_1 = &vftable`).
pub const W19A08_VFTABLE_00F02C0C: u32 = 0x00f0_2c0c;

/// `0x005504a0`: stores the fixed vftable pointer into the object's first dword.
pub fn w19a08_set_vftable(obj: &mut u32) {
    *obj = W19A08_VFTABLE_00F02C0C;
}

/// `0x00552560`: bounded string copy.  Returns the full source length; when
/// `dst_cap != 0` copies `min(len, dst_cap - 1)` bytes into `dst`.
pub fn w19a08_copy_bounded(dst: &mut [u8], src: &[u8], dst_cap: usize) -> usize {
    let len = src.iter().position(|&b| b == 0).unwrap_or(src.len());
    if dst_cap != 0 {
        let n = len.min(dst_cap - 1).min(dst.len());
        dst[..n].copy_from_slice(&src[..n]);
    }
    len
}

/// Shared growth rule of `0x00554110` and `0x00554210`: the capacity word's
/// low 31 bits are the capacity; returns `Some(new_word)` when a reallocation
/// to `max(cap + cap/2, wanted)` is needed (high bit preserved), else `None`.
pub fn w19a08_grow_word(word: u32, wanted: u32) -> Option<u32> {
    let cap = word & 0x7fff_ffff;
    if cap >= wanted {
        return None;
    }
    let mut new_cap = ((word >> 1) & 0x3fff_ffff).wrapping_add(cap);
    if new_cap < wanted {
        new_cap = wanted;
    }
    Some(((word ^ new_cap) & 0x7fff_ffff) ^ word)
}

/// `0x00554110`: reserve helper for the capacity word at offset 8.
pub fn w19a08_reserve_a(word: u32, wanted: u32) -> Option<u32> {
    w19a08_grow_word(word, wanted)
}

/// `0x00554210`: identical reserve helper (second instantiation).
pub fn w19a08_reserve_b(word: u32, wanted: u32) -> Option<u32> {
    w19a08_grow_word(word, wanted)
}

/// `0x005543c0`: truncates a byte string to `new_len`.  `Err(())` when
/// `new_len > len` (the original calls the out-of-range handler; note the
/// check is `len <= new_len`, so `new_len == len` is also handed to it only
/// for the equal case below returning early).
pub fn w19a08_truncate(buf: &mut [u8], len: &mut usize, new_len: usize) -> Result<(), ()> {
    if *len <= new_len {
        return Err(());
    }
    buf[new_len] = 0;
    *len = new_len;
    Ok(())
}

/// `0x00555140`: sets the thread-context vftable; returns true when the
/// object must be freed (`flags & 1`).
pub fn w19a08_ctx_dtor_a(obj: &mut u32, vftable: u32, flags: u8) -> bool {
    *obj = vftable;
    flags & 1 != 0
}

/// `0x00555170`: same as the other thread-context destructor.
pub fn w19a08_ctx_dtor_b(obj: &mut u32, vftable: u32, flags: u8) -> bool {
    *obj = vftable;
    flags & 1 != 0
}

/// Result of `0x005555b0`.
#[derive(Debug, PartialEq, Eq)]
pub enum W19a08Push {
    Pushed,
    /// Reallocation required; new capacity word.
    Grow(u32),
}

/// `0x005555b0`: pushes `value` (taken from `src_idx`, which may alias an
/// element of the vector itself) onto a vector of u32 whose capacity word
/// is `cap_word`.  The source slot is zeroed (move semantics).  When the
/// vector is full, returns the grown capacity word instead.
pub fn w19a08_push_move(
    items: &mut Vec<u32>,
    cap_word: u32,
    src_idx: Option<usize>,
    external: &mut u32,
) -> W19a08Push {
    let cap = cap_word & 0x7fff_ffff;
    if items.len() as u32 == cap && cap < cap.wrapping_add(1) {
        let extra = (cap_word >> 1) & 0x3fff_ffff;
        let add = if cap < cap + extra { extra } else { 1 };
        return W19a08Push::Grow(cap_word ^ (((cap + add) ^ cap_word) & 0x7fff_ffff));
    }
    let v = match src_idx {
        Some(i) if i < items.len() => std::mem::take(&mut items[i]),
        _ => std::mem::take(external),
    };
    items.push(v);
    W19a08Push::Pushed
}

#[cfg(test)]
mod tests_wave19_agent08 {
    use super::*;

    #[test]
    fn vtable_and_dtors() {
        let mut o = 0;
        w19a08_set_vftable(&mut o);
        assert_eq!(o, 0x00f0_2c0c);
        assert!(w19a08_ctx_dtor_a(&mut o, 7, 1));
        assert!(!w19a08_ctx_dtor_b(&mut o, 7, 2));
    }

    #[test]
    fn bounded_copy() {
        let mut d = [9u8; 8];
        assert_eq!(w19a08_copy_bounded(&mut d, b"hello\0", 4), 5);
        assert_eq!(&d[..3], b"hel");
        assert_eq!(w19a08_copy_bounded(&mut d, b"hi\0", 0), 2);
    }

    #[test]
    fn grow() {
        assert_eq!(w19a08_reserve_a(0x8000_0010, 8), None);
        assert_eq!(w19a08_reserve_b(0x10, 17), Some(0x18));
        assert_eq!(w19a08_reserve_a(0x8000_0010, 100), Some(0x8000_0064));
    }

    #[test]
    fn truncate() {
        let mut b = *b"abcdef";
        let mut l = 6;
        assert!(w19a08_truncate(&mut b, &mut l, 6).is_err());
        assert!(w19a08_truncate(&mut b, &mut l, 2).is_ok());
        assert_eq!((l, b[2]), (2, 0));
    }

    #[test]
    fn push_move() {
        let mut v = vec![5, 6];
        let mut e = 7;
        assert_eq!(
            w19a08_push_move(&mut v, 4, Some(0), &mut e),
            W19a08Push::Pushed
        );
        assert_eq!(v, vec![0, 6, 5]);
        assert_eq!(
            w19a08_push_move(&mut v, 4, None, &mut e),
            W19a08Push::Pushed
        );
        assert_eq!((v[3], e), (7, 0));
        assert_eq!(
            w19a08_push_move(&mut v, 4, None, &mut e),
            W19a08Push::Grow(6)
        );
    }
}
