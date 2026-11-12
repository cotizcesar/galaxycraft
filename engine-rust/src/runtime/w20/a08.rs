#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave20/agent08.rs
// ===========================================================================
// Wave 20 agent 08: small container helpers (hash capacity, SSO string, heap-flag scans).

/// Unsigned 32-bit value to `f32` the way the x87 code does it (signed
/// convert plus the 2^32 fixup from the constant table when negative).
fn w20a08_u32_f32(v: u32) -> f32 {
    ((v as i32) as f64 + if (v as i32) < 0 { 4294967296.0 } else { 0.0 }) as f32
}

/// `0x005611d0`: bucket capacity chosen for a hash container.  `size` is
/// `[2]`, `buckets` the current bucket value (callee result), `load` the
/// `f32` at `[3]`, `want` the argument.  Starts at `buckets`, or
/// `buckets + 1` when `size > buckets * load`; raises to `want`; rounds up
/// to a power of two with a floor of 8.  Returns `(capacity, unchanged)`
/// where `unchanged` is true when it equals `cur_cap` and `cur_ptr_nonzero`
/// (the early-return path; otherwise the original reallocates).
pub fn w20a08_hash_capacity(
    size: u32,
    buckets: u32,
    load: f32,
    want: u32,
    cur_cap: u32,
    cur_ptr_nonzero: bool,
) -> (u32, bool) {
    let mut n = buckets.wrapping_add(1);
    if w20a08_u32_f32(size) <= w20a08_u32_f32(buckets) * load {
        n = buckets;
    }
    if n < want {
        n = want;
    }
    let mut cap: u32 = 8;
    if n > 8 {
        while cap < n {
            cap = cap.wrapping_mul(2);
        }
    }
    (cap, cap == cur_cap && cur_ptr_nonzero)
}

/// `0x005618b0`: size to request when appending the NUL-terminated `s`
/// to a string whose length field (`[1]`) is `len`: `len + strlen(s)`.
pub fn w20a08_append_request(s: &[u8], len: u32) -> u32 {
    let n = s.iter().position(|&b| b == 0).unwrap_or(s.len());
    len.wrapping_add(n as u32)
}

/// Fields of a small-string-optimised header (`ptr`, `len`, `cap`, inline byte).
#[derive(Debug, PartialEq, Eq)]
pub struct W20a08SsoInit {
    pub ptr_is_inline: bool,
    pub len: u32,
    pub cap: u32,
    pub inline0: u8,
    /// Size handed to the allocator call (`other.len`).
    pub request: u32,
}

/// `0x00561b10`: initialise an empty SSO string (`len = 0`, `cap = 0x8000000f`,
/// pointer to the inline buffer at `+0xc`, first byte 0) and request `other_len`.
pub fn w20a08_sso_init(other_len: u32) -> W20a08SsoInit {
    W20a08SsoInit {
        ptr_is_inline: true,
        len: 0,
        cap: 0x8000_000f,
        inline0: 0,
        request: other_len,
    }
}

fn w20a08_rd_i32(b: &[u8], off: usize) -> Option<i32> {
    Some(i32::from_le_bytes(b.get(off..off + 4)?.try_into().ok()?))
}

/// `0x00561b70`: scan `0x24`-byte records of `rows` (index range `from..to`)
/// and return the first whose i32 at `+8` is non-negative (heap-owned
/// capacity flag clear; the original then frees), else `None`.
pub fn w20a08_first_heap_record_24(rows: &[u8], from: usize, to: usize) -> Option<usize> {
    (from..to).find(|&i| w20a08_rd_i32(rows, i * 0x24 + 8).is_some_and(|v| v >= 0))
}

/// `0x00561ff0`: destructor scan over a vector (`count` records of `0x24`
/// bytes) with capacity word `cap`; returns true when something must be
/// freed (a record per `0x00561b70`, or `cap >= 0`).
pub fn w20a08_vec24_needs_free(rows: &[u8], count: usize, cap: i32) -> bool {
    w20a08_first_heap_record_24(rows, 0, count).is_some() || cap >= 0
}

/// `0x00562060`: destructor scan over `count` records of `0x68` bytes; each
/// is checked at `+0x64`, `+0x58`, `+0x0c` (i32 >= 0 means owned heap).
/// Returns the first `(index, offset)` needing a free, else checks `cap`
/// and returns `Some((count, usize::MAX))` when the vector buffer itself is owned.
pub fn w20a08_vec68_first_free(rows: &[u8], count: usize, cap: i32) -> Option<(usize, usize)> {
    for i in 0..count {
        for off in [0x64usize, 0x58, 0x0c] {
            if w20a08_rd_i32(rows, i * 0x68 + off).is_some_and(|v| v >= 0) {
                return Some((i, off));
            }
        }
    }
    (cap >= 0).then_some((count, usize::MAX))
}

#[cfg(test)]
mod tests_wave20_agent08 {
    use super::*;

    #[test]
    fn capacity_rounding() {
        assert_eq!(w20a08_hash_capacity(0, 0, 1.0, 0, 0, false), (8, false));
        assert_eq!(w20a08_hash_capacity(5, 8, 1.0, 0, 8, true), (8, true));
        // size > buckets*load -> buckets+1 = 9 -> 16
        assert_eq!(w20a08_hash_capacity(9, 8, 1.0, 0, 8, true).0, 16);
        assert_eq!(w20a08_hash_capacity(0, 4, 1.0, 100, 0, false).0, 128);
    }

    #[test]
    fn append_and_sso() {
        assert_eq!(w20a08_append_request(b"abc\0xyz", 10), 13);
        assert_eq!(w20a08_append_request(b"", 4), 4);
        let s = w20a08_sso_init(7);
        assert_eq!((s.len, s.cap, s.request), (0, 0x8000_000f, 7));
    }

    #[test]
    fn scans() {
        let mut v = vec![0u8; 0x24 * 3];
        for i in 0..3 {
            v[i * 0x24 + 8..i * 0x24 + 12].copy_from_slice(&(-1i32).to_le_bytes());
        }
        assert_eq!(w20a08_first_heap_record_24(&v, 0, 3), None);
        assert!(!w20a08_vec24_needs_free(&v, 3, -1));
        v[2 * 0x24 + 8..2 * 0x24 + 12].copy_from_slice(&5i32.to_le_bytes());
        assert_eq!(w20a08_first_heap_record_24(&v, 0, 3), Some(2));
        assert!(w20a08_vec24_needs_free(&v, 3, -1));
        assert!(w20a08_vec24_needs_free(&[], 0, 0));
    }

    #[test]
    fn scan68() {
        let mut v = vec![0u8; 0x68 * 2];
        for i in 0..2 {
            for off in [0x64, 0x58, 0x0c] {
                v[i * 0x68 + off..i * 0x68 + off + 4].copy_from_slice(&(-1i32).to_le_bytes());
            }
        }
        assert_eq!(w20a08_vec68_first_free(&v, 2, -1), None);
        assert_eq!(w20a08_vec68_first_free(&v, 2, 0), Some((2, usize::MAX)));
        v[0x68 + 0x58..0x68 + 0x5c].copy_from_slice(&0i32.to_le_bytes());
        assert_eq!(w20a08_vec68_first_free(&v, 2, -1), Some((1, 0x58)));
    }
}
