#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent20.rs
// ===========================================================================
// Wave 22 agent 20: small runtime helpers.

/// `0x00696370`: ostream-style write of `(ptr,len)`.  `len == 0` is a no-op.
/// If the stream state is clear, `put` is called; a short write sets bit 1
/// (badbit); a non-clear state just ORs bit 1.  Returns the new state.
pub fn w22a20_stream_write(state: u32, len: u32, put: impl FnOnce(u32) -> u64) -> u32 {
    if len == 0 {
        return state;
    }
    if state == 0 {
        if put(len) != len as u64 {
            return state | 1;
        }
        state
    } else {
        state | 1
    }
}

/// `0x00696470`: intrusive list head init: next and prev point to self and the
/// size is zero.  Returns `(next, prev, size)`.
pub fn w22a20_list_head_init(this: u32) -> (u32, u32, u32) {
    (this, this, 0)
}

/// `0x006969c0`: owned-pointer reset.  Low bit of `word` tags an external
/// buffer.  When the pointer is non-null, `destroy` runs; the word becomes 1.
/// Returns `None` when a tagged (heap) block would need freeing (non-returning
/// free path in the original), else the new word.
pub fn w22a20_owned_reset(word: u32, mut destroy: impl FnMut(u32)) -> Option<u32> {
    let ptr = if word & 1 != 0 { word & !1 } else { word };
    if ptr == 0 {
        return Some(word);
    }
    destroy(ptr);
    if word & 1 != 0 {
        return None;
    }
    Some(1)
}

/// `0x006984d0`: scan a 64-entry `(name, key)` table for `key`, only when
/// `blocked` is false, the flag mask lacks `0x2000_0000` and `key != 0`.
pub fn w22a20_lookup_key64(
    blocked: bool,
    flags: u32,
    key: i16,
    table: &[(u32, i16)],
) -> Option<u32> {
    w22a20_lookup_key(blocked, flags, key, table, 0x40)
}

/// `0x00698530`: same lookup as `0x006984d0` with a one-entry table.
pub fn w22a20_lookup_key1(
    blocked: bool,
    flags: u32,
    key: i16,
    table: &[(u32, i16)],
) -> Option<u32> {
    w22a20_lookup_key(blocked, flags, key, table, 1)
}

fn w22a20_lookup_key(
    blocked: bool,
    flags: u32,
    key: i16,
    table: &[(u32, i16)],
    count: usize,
) -> Option<u32> {
    if blocked || flags & 0x2000_0000 != 0 || key == 0 {
        return None;
    }
    table.iter().take(count).find(|e| e.1 == key).map(|e| e.0)
}

/// `0x006993c0`: length of the text before the first `'>'` in `s`.
pub fn w22a20_len_until_gt(s: &[u8]) -> Option<usize> {
    s.iter().position(|&b| b == b'>')
}

/// `0x00699450`: string capacity growth.  Low 31 bits of `flags` hold the
/// capacity, the top bit is preserved.  Returns the new flags word when a
/// grow is needed (1.5x, at least `want`), else `None`.
pub fn w22a20_grow_capacity(flags: u32, want: u32) -> Option<u32> {
    let cap = flags & 0x7fff_ffff;
    if cap >= want {
        return None;
    }
    let mut n = ((flags >> 1) & 0x3fff_ffff).wrapping_add(cap);
    if n < want {
        n = want;
    }
    Some((flags & 0x8000_0000) | (n & 0x7fff_ffff))
}

#[cfg(test)]
mod tests_wave22_agent20 {
    use super::*;

    #[test]
    fn stream_write() {
        assert_eq!(w22a20_stream_write(0, 0, |_| panic!()), 0);
        assert_eq!(w22a20_stream_write(0, 4, |n| n as u64), 0);
        assert_eq!(w22a20_stream_write(0, 4, |_| 2), 1);
        assert_eq!(w22a20_stream_write(4, 4, |_| panic!()), 5);
    }

    #[test]
    fn list_head() {
        assert_eq!(w22a20_list_head_init(7), (7, 7, 0));
    }

    #[test]
    fn owned_reset() {
        let mut n = 0;
        assert_eq!(w22a20_owned_reset(0, |_| n += 1), Some(0));
        assert_eq!(w22a20_owned_reset(0x100, |_| n += 1), Some(1));
        assert_eq!(w22a20_owned_reset(0x101, |_| n += 1), None);
        assert_eq!(n, 2);
    }

    #[test]
    fn lookup() {
        let t = [(10, 5), (11, 6)];
        assert_eq!(w22a20_lookup_key64(false, 0, 6, &t), Some(11));
        assert_eq!(w22a20_lookup_key64(true, 0, 6, &t), None);
        assert_eq!(w22a20_lookup_key64(false, 0x2000_0000, 6, &t), None);
        assert_eq!(w22a20_lookup_key64(false, 0, 0, &t), None);
        assert_eq!(w22a20_lookup_key1(false, 0, 6, &t), None);
        assert_eq!(w22a20_lookup_key1(false, 0, 5, &t), Some(10));
    }

    #[test]
    fn len_and_grow() {
        assert_eq!(w22a20_len_until_gt(b"ab>c"), Some(2));
        assert_eq!(w22a20_len_until_gt(b"abc"), None);
        assert_eq!(w22a20_grow_capacity(15, 10), None);
        assert_eq!(w22a20_grow_capacity(0x8000_000f, 16), Some(0x8000_0016));
        assert_eq!(w22a20_grow_capacity(15, 100), Some(100));
    }
}
