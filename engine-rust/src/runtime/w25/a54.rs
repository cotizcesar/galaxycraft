#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent54.rs
// ===========================================================================
// Wave 25 agent 54: tiny MSVC container helpers (list sentinel / SSO string).

/// Result of a checked store (`0x008c93f0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W25a54Checked {
    Stored(u32),
    /// The two guard values differed; retail calls the non-returning handler.
    Invalid,
}

/// `0x008c93f0`: if `a == b` store `value` through the out pointer, else the
/// non-returning invalid-parameter handler runs.
pub fn w25a54_checked_store(out: &mut u32, value: u32, a: i32, b: i32) -> W25a54Checked {
    if a != b {
        return W25a54Checked::Invalid;
    }
    *out = value;
    W25a54Checked::Stored(value)
}

/// Doubly linked list head (`prev` at +0, `next` at +4, size at +8) with node
/// ids; `head` is the sentinel id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W25a54ListHead {
    pub prev: u32,
    pub next: u32,
    pub size: u32,
}

/// `0x008c9710`: reset a list head to empty (`next = prev = head`, `size = 0`).
/// Returns the old `next`; if it was not the sentinel, retail then walks and
/// frees the old nodes (caller responsibility).
pub fn w25a54_list_reset(head: &mut W25a54ListHead, head_id: u32) -> Option<u32> {
    let old = head.next;
    head.next = head_id;
    head.prev = head_id;
    head.size = 0;
    if old != head_id { Some(old) } else { None }
}

/// MSVC small-string header: ptr(+0), size(+4), capacity(+8), inline buf (+0xc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W25a54SsoString {
    pub ptr_is_inline: bool,
    pub size: u32,
    pub capacity: u32,
    pub inline_buf: [u8; 16],
}

/// `0x008cab10`: initialise an empty SSO string (size 0, capacity
/// `0x8000000f`, buf[0] = 0, ptr -> inline buffer) before the assign call.
pub fn w25a54_sso_init() -> W25a54SsoString {
    W25a54SsoString {
        ptr_is_inline: true,
        size: 0,
        capacity: 0x8000_000f,
        inline_buf: [0; 16],
    }
}

#[cfg(test)]
mod tests_wave25_agent54 {
    use super::*;

    #[test]
    fn checked_store() {
        let mut o = 0;
        assert_eq!(
            w25a54_checked_store(&mut o, 7, 1, 1),
            W25a54Checked::Stored(7)
        );
        assert_eq!(o, 7);
        assert_eq!(
            w25a54_checked_store(&mut o, 9, 1, 2),
            W25a54Checked::Invalid
        );
        assert_eq!(o, 7);
    }

    #[test]
    fn list_reset() {
        let mut h = W25a54ListHead {
            prev: 5,
            next: 5,
            size: 3,
        };
        assert_eq!(w25a54_list_reset(&mut h, 5), None);
        assert_eq!(h.size, 0);
        let mut h = W25a54ListHead {
            prev: 1,
            next: 2,
            size: 3,
        };
        assert_eq!(w25a54_list_reset(&mut h, 9), Some(2));
        assert_eq!((h.prev, h.next), (9, 9));
    }

    #[test]
    fn sso() {
        let s = w25a54_sso_init();
        assert_eq!(s.capacity, 0x8000_000f);
        assert_eq!(s.size, 0);
        assert!(s.ptr_is_inline);
    }
}
