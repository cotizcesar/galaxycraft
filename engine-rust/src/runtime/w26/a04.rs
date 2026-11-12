#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent04.rs
// ===========================================================================
// Wave 26 translation batch 04.
//
// Small container helpers (growth policy of an 8-byte-element vector and
// of a `u16` vector) plus two tiny record helpers.  Re-allocation paths
// are non-returning in the pseudocode, so they are reported as `Err`.

/// `0x0090cb90`: when `mode == 2` and `read_value` yields a value, stores
/// `value & 7` into the u16 at offset `0x30` of `record`; returns whether
/// the store happened.
pub fn w26a04_store_masked_u16(
    record: &mut [u8],
    mode: i32,
    read_value: impl FnOnce() -> Option<u32>,
) -> bool {
    if mode != 2 {
        return false;
    }
    match read_value() {
        Some(v) => {
            record[0x30..0x32].copy_from_slice(&((v & 7) as u16).to_le_bytes());
            true
        }
        None => false,
    }
}

/// Vector of 8-byte elements with the retail capacity word (low 31 bits).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct W26a04Vec8 {
    pub items: Vec<[u32; 2]>,
    pub cap: u32,
}

/// Source record layout read by `0x0090cf90`: dword at `+4`, u16 at `+8`,
/// u8 at `+0x10`.
pub fn w26a04_vec8_element(src: &[u8]) -> [u32; 2] {
    let a = u32::from_le_bytes(src[4..8].try_into().unwrap());
    let w = u16::from_le_bytes(src[8..10].try_into().unwrap()) as u32;
    [a, ((src[0x10] as u32) << 16) | w]
}

/// `0x0090cf90`: push-back of the element built from `src`.  When
/// `len == cap` the retail code grows the capacity by `max(1, cap / 2)`
/// and reallocates (non-returning here): `Err(new_cap)`.
pub fn w26a04_vec8_push(v: &mut W26a04Vec8, src: &[u8]) -> Result<(), u32> {
    let cap = v.cap & 0x7fff_ffff;
    if v.items.len() as u32 == cap && cap < cap.wrapping_add(1) {
        let half = (v.cap >> 1) & 0x3fff_ffff;
        let add = if cap < cap + half { half } else { 1 };
        return Err(cap + add);
    }
    v.items.push(w26a04_vec8_element(src));
    Ok(())
}

/// `0x0090d940`: resize of a `u16` vector to `n` elements.  Shrinking
/// truncates; growing within capacity zero-fills; otherwise returns
/// `Err(new_cap)` with `new_cap = max(n, cap + (cap >> 1))`.
pub fn w26a04_u16vec_resize(items: &mut Vec<u16>, cap_word: u32, n: u32) -> Result<(), u32> {
    let len = items.len() as u32;
    if n < len {
        items.truncate(n as usize);
    } else if len < n {
        let cap = cap_word & 0x7fff_ffff;
        if cap < n {
            let grown = ((cap_word >> 1) & 0x3fff_ffff) + cap;
            return Err(grown.max(n));
        }
        items.resize(n as usize, 0);
    }
    Ok(())
}

/// `0x0090e2e0`: stores `value` into `*dst` when `a == b`; otherwise the
/// retail code calls a non-returning error routine (`Err`).
pub fn w26a04_store_if_equal(dst: &mut u32, value: u32, a: i32, b: i32) -> Result<(), ()> {
    if a != b {
        return Err(());
    }
    *dst = value;
    Ok(())
}

/// Header copied by `0x0090ec10` (dwords 0, 2, 3, 4..=7 of the source).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct W26a04Header {
    pub d0: u32,
    pub d2: u32,
    pub d3: u32,
    pub d4: u32,
    pub d5: u32,
    pub d6: u32,
    pub d7: u32,
}

/// `0x0090ec10`: copies the header dwords from `src` (dword indices 0, 2,
/// 3, 4, 5, 6, 7) and returns them with the length of the trailing string
/// (dword 9) that the retail code then copies into an SSO string.
pub fn w26a04_copy_header(src: &[u32]) -> (W26a04Header, u32) {
    (
        W26a04Header {
            d0: src[0],
            d2: src[2],
            d3: src[3],
            d4: src[4],
            d5: src[5],
            d6: src[6],
            d7: src[7],
        },
        src[9],
    )
}

#[cfg(test)]
mod tests_wave26_agent04 {
    use super::*;

    #[test]
    fn masked_store() {
        let mut r = [0u8; 0x40];
        assert!(!w26a04_store_masked_u16(&mut r, 1, || Some(5)));
        assert!(!w26a04_store_masked_u16(&mut r, 2, || None));
        assert!(w26a04_store_masked_u16(&mut r, 2, || Some(0x1d)));
        assert_eq!(r[0x30], 5);
    }

    #[test]
    fn vec8_push_and_grow() {
        let mut src = [0u8; 0x11];
        src[4..8].copy_from_slice(&7u32.to_le_bytes());
        src[8..10].copy_from_slice(&0x1234u16.to_le_bytes());
        src[0x10] = 3;
        assert_eq!(w26a04_vec8_element(&src), [7, 0x31234]);
        let mut v = W26a04Vec8 {
            items: vec![],
            cap: 0,
        };
        assert_eq!(w26a04_vec8_push(&mut v, &src), Err(1));
        v.cap = 2;
        assert_eq!(w26a04_vec8_push(&mut v, &src), Ok(()));
        assert_eq!(w26a04_vec8_push(&mut v, &src), Ok(()));
        assert_eq!(w26a04_vec8_push(&mut v, &src), Err(3));
    }

    #[test]
    fn u16_resize() {
        let mut v = vec![1u16, 2, 3];
        assert_eq!(w26a04_u16vec_resize(&mut v, 4, 2), Ok(()));
        assert_eq!(v, [1, 2]);
        assert_eq!(w26a04_u16vec_resize(&mut v, 4, 4), Ok(()));
        assert_eq!(v, [1, 2, 0, 0]);
        assert_eq!(w26a04_u16vec_resize(&mut v, 4, 9), Err(9));
        assert_eq!(w26a04_u16vec_resize(&mut v, 4, 5), Err(6));
    }

    #[test]
    fn store_and_header() {
        let mut d = 0;
        assert_eq!(w26a04_store_if_equal(&mut d, 5, 1, 2), Err(()));
        assert_eq!(w26a04_store_if_equal(&mut d, 5, 2, 2), Ok(()));
        assert_eq!(d, 5);
        let s: Vec<u32> = (0..10).collect();
        let (h, n) = w26a04_copy_header(&s);
        assert_eq!((h.d0, h.d2, h.d7, n), (0, 2, 7, 9));
    }
}
