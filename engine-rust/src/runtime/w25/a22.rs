#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent22.rs
// ===========================================================================
// Wave 25 agent 22: small MSVC vector helpers (8-byte and 0x24-byte records).

/// Result of a vector growth check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W25a22Grow {
    /// Enough capacity; write in place.
    InPlace,
    /// Reallocation needed with this new capacity.
    Realloc(u32),
}

/// `0x00804220`: push_back of an 8-byte record.  Capacity lives in the low 31
/// bits of `cap_word`; when `len == cap` the vector grows by
/// max(1, cap/2) (cap/2 taken from `cap_word >> 1 & 0x3fffffff`).
/// Otherwise the element is appended; if the argument aliased an element
/// inside the vector (`alias_index < len`) that element is copied instead.
pub fn w25a22_push_back8(
    data: &mut Vec<[u32; 2]>,
    cap_word: u32,
    value: [u32; 2],
    alias_index: Option<usize>,
) -> W25a22Grow {
    let cap = cap_word & 0x7fff_ffff;
    let len = data.len() as u32;
    if len == cap && cap < cap.wrapping_add(1) {
        let half = (cap_word >> 1) & 0x3fff_ffff;
        let step = if half > 0 { half } else { 1 };
        return W25a22Grow::Realloc(cap + step);
    }
    let v = match alias_index {
        Some(i) if i < data.len() => data[i],
        _ => value,
    };
    data.push(v);
    W25a22Grow::InPlace
}

/// Outcome of the resize routine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W25a22Resize {
    /// Elements `[new_len, old_len)` destroyed; length becomes `new_len`.
    Shrunk(u32),
    /// Nothing to do.
    Unchanged,
    /// Reallocation needed with this capacity.
    Realloc(u32),
    /// New elements zero-initialised (empty small string, cap word
    /// `0x8000000f`); length becomes `new_len`.
    Grown(u32),
}

/// `0x00807a10`: resize of a vector of `0x24`-byte records (each holding an
/// SSO string whose capacity word is `0x8000000f`).
pub fn w25a22_resize36(len: u32, cap_word: u32, new_len: u32) -> W25a22Resize {
    if new_len < len {
        return W25a22Resize::Shrunk(new_len);
    }
    if len < new_len {
        let cap = cap_word & 0x7fff_ffff;
        if cap < new_len {
            let grown = (cap_word >> 1 & 0x3fff_ffff) + cap;
            return W25a22Resize::Realloc(grown.max(new_len));
        }
        return W25a22Resize::Grown(new_len);
    }
    W25a22Resize::Unchanged
}

/// Initial SSO capacity word written into freshly built strings.
pub const W25A22_SSO_CAP_WORD: u32 = 0x8000_000f;

#[cfg(test)]
mod tests_wave25_agent22 {
    use super::*;

    #[test]
    fn push_back_paths() {
        let mut v = vec![[1, 2]];
        assert_eq!(
            w25a22_push_back8(&mut v, 4, [3, 4], None),
            W25a22Grow::InPlace
        );
        assert_eq!(v[1], [3, 4]);
        assert_eq!(
            w25a22_push_back8(&mut v, 4, [9, 9], Some(0)),
            W25a22Grow::InPlace
        );
        assert_eq!(v[2], [1, 2]);
        assert_eq!(
            w25a22_push_back8(&mut v, 3, [0, 0], None),
            W25a22Grow::Realloc(4)
        );
        let mut e: Vec<[u32; 2]> = vec![];
        assert_eq!(
            w25a22_push_back8(&mut e, 0, [0, 0], None),
            W25a22Grow::Realloc(1)
        );
    }

    #[test]
    fn resize_paths() {
        assert_eq!(w25a22_resize36(5, 8, 2), W25a22Resize::Shrunk(2));
        assert_eq!(w25a22_resize36(5, 8, 5), W25a22Resize::Unchanged);
        assert_eq!(w25a22_resize36(5, 8, 8), W25a22Resize::Grown(8));
        assert_eq!(w25a22_resize36(5, 8, 9), W25a22Resize::Realloc(12));
        assert_eq!(w25a22_resize36(0, 2, 20), W25a22Resize::Realloc(20));
        assert_eq!(W25A22_SSO_CAP_WORD, 0x8000_000f);
    }
}
