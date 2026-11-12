#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent31.rs
// ===========================================================================
// Wave 25 agent 31: small container helpers (runtime.rs target).

/// Small-string header used by the init helpers: size, capacity, inline buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W25a31SmallString {
    pub len: u32,
    pub cap: u32,
    pub inline: Vec<u8>,
}

/// Shared body of the two string-init helpers: len 0, capacity `0x8000000f`
/// (inline flag + 15), first inline byte cleared.
fn w25a31_init_small(inline_len: usize) -> W25a31SmallString {
    W25a31SmallString {
        len: 0,
        cap: 0x8000_000f,
        inline: vec![0; inline_len.max(1)],
    }
}

/// `0x00846d20`: initialises an empty inline string; the original then calls
/// a helper with argument `0x13` (returned so the caller can dispatch it).
pub fn w25a31_init_small_string_13() -> (W25a31SmallString, u32) {
    (w25a31_init_small(16), 0x13)
}

/// `0x00846d80`: same as above with helper argument `0x19`.
pub fn w25a31_init_small_string_19() -> (W25a31SmallString, u32) {
    (w25a31_init_small(16), 0x19)
}

/// `0x00845d80`: walks a pointer array and invokes the virtual deleting
/// destructor (vtable slot 3, flag 1) on each non-null entry.  `drop_one`
/// receives the element index.  Returns true when the buffer capacity word
/// is negative (inline storage, nothing to free), false when the original
/// would tail into the free path.
pub fn w25a31_destroy_ptr_array<T>(
    items: &mut [Option<T>],
    cap_word: i32,
    mut drop_one: impl FnMut(usize, T),
) -> bool {
    for (i, slot) in items.iter_mut().enumerate() {
        if let Some(v) = slot.take() {
            drop_one(i, v);
        }
    }
    cap_word < 0
}

#[cfg(test)]
mod tests_wave25_agent31 {
    use super::*;

    #[test]
    fn small_string_init() {
        let (s, a) = w25a31_init_small_string_13();
        assert_eq!((s.len, s.cap, s.inline[0], a), (0, 0x8000_000f, 0, 0x13));
        let (_, b) = w25a31_init_small_string_19();
        assert_eq!(b, 0x19);
    }

    #[test]
    fn destroy_array() {
        let mut v = vec![Some(1), None, Some(3)];
        let mut seen = vec![];
        assert!(w25a31_destroy_ptr_array(&mut v, -1, |i, x| seen.push((i, x))));
        assert_eq!(seen, vec![(0, 1), (2, 3)]);
        assert!(!w25a31_destroy_ptr_array::<u8>(&mut [], 5, |_, _| {}));
    }
}
