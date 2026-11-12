#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent08.rs
// ===========================================================================
// Wave 23 translation batch 08.
//
// Window: queue: 20 addresses, module `runtime.rs`.

/// `0x006ebb40`: when the handle is non-zero, stores state 3 and invokes
/// the callback with the handle.  Returns the new state if it was written.
pub fn w23a08_signal_handle(
    handle: u32,
    state: &mut u32,
    mut notify: impl FnMut(u32),
) -> Option<u32> {
    if handle != 0 {
        *state = 3;
        notify(handle);
        Some(3)
    } else {
        None
    }
}

/// `0x006ecf10`: guarded append.  Accepted only when `count < 0xfa` and the
/// record's flag byte (offset 4) matches `flag_global != 0`.  The 0x34-dword
/// (0xd0 byte) payload is copied to offset 8 of a 0xd8-byte slot; returns the
/// slot and the incremented count.
pub fn w23a08_append_record(
    count: u32,
    flag_global: u8,
    record: &[u8],
) -> Option<([u8; 0xd8], u32)> {
    if count >= 0xfa || record.len() < 0xd0 {
        return None;
    }
    // byte 4 of the source record is the flag
    if record.len() < 8 || (record[4] != 0) != (flag_global != 0) {
        return None;
    }
    let mut slot = [0u8; 0xd8];
    slot[8..8 + 0xd0].copy_from_slice(&record[..0xd0]);
    Some((slot, count + 1))
}

/// Ghidra's "node pointer" test: odd or zero values are treated as null.
fn w23a08_node(raw: u32) -> Option<u32> {
    if raw & 1 == 1 || raw == 0 {
        None
    } else {
        Some(raw)
    }
}

/// Outcome of `0x006ecfd0`.
#[derive(Debug, PartialEq, Eq)]
pub enum W23a08Walk {
    /// Index byte is 0xff or the list head is null: the fallback branch.
    Fallback,
    /// Head valid: node reached after `index` `next` steps (None if the
    /// chain ended early).
    Active(Option<u32>),
}

/// `0x006ecfd0`: selects the branch from the index byte at offset 0x62 and
/// the list head, walking `index` links (`next(node)` reads offset 4).
pub fn w23a08_select_walk(index: u8, head: u32, next: impl Fn(u32) -> u32) -> W23a08Walk {
    let Some(mut node) = w23a08_node(head) else {
        return W23a08Walk::Fallback;
    };
    if index == 0xff {
        return W23a08Walk::Fallback;
    }
    for _ in 0..index {
        match w23a08_node(next(node)) {
            Some(n) => node = n,
            None => return W23a08Walk::Active(None),
        }
    }
    W23a08Walk::Active(Some(node))
}

/// `0x006ed640`: true when the index byte (offset 0x62) is not 0xff, the
/// head is valid, and walking `index` links ends on a valid node (the
/// retail code then raises its error path).
pub fn w23a08_walk_hits(index: u8, head: u32, next: impl Fn(u32) -> u32) -> bool {
    if index == 0xff {
        return false;
    }
    let Some(mut node) = w23a08_node(head) else {
        return false;
    };
    let mut left = index;
    loop {
        if left == 0 {
            break;
        }
        match w23a08_node(next(node)) {
            Some(n) => node = n,
            None => return false,
        }
        left -= 1;
    }
    true
}

/// `0x006ede80`: decimal text of `(scale * value) / 1000` (unsigned,
/// 32-bit wrapping product), as produced by the 9-char itoa buffer.
pub fn w23a08_scaled_ms_text(scale: i32, value: i32) -> String {
    ((scale.wrapping_mul(value) as u32) / 1000).to_string()
}

/// `0x006f08d0`: true when `id` is found in the zero-terminated byte list
/// or in the signed-char vector.
pub fn w23a08_id_in_lists(id: u16, terminated: &[u8], vector: &[i8]) -> bool {
    terminated
        .iter()
        .take_while(|&&b| b != 0)
        .any(|&b| u16::from(b) == id)
        || vector.iter().any(|&c| i32::from(c) == i32::from(id))
}

/// `0x006f0090`: bounded concatenation.  `dst` holds a C string in a buffer
/// of `cap` bytes; `src` is appended truncated to `cap - 1` total.
/// Returns the resulting length.
pub fn w23a08_bounded_cat(dst: &mut Vec<u8>, src: &[u8], cap: usize) -> usize {
    let cur = dst.iter().position(|&b| b == 0).unwrap_or(dst.len());
    dst.truncate(cur);
    if cur != cap {
        let srclen = src.iter().position(|&b| b == 0).unwrap_or(src.len());
        let end = (cur + srclen).min(cap.saturating_sub(1));
        if end > cur {
            dst.extend_from_slice(&src[..end - cur]);
        }
    }
    dst.len()
}

#[cfg(test)]
mod tests_wave23_agent08 {
    use super::*;

    #[test]
    fn signal() {
        let mut st = 0;
        let mut got = 0;
        assert_eq!(w23a08_signal_handle(0, &mut st, |h| got = h), None);
        assert_eq!((st, got), (0, 0));
        assert_eq!(w23a08_signal_handle(9, &mut st, |h| got = h), Some(3));
        assert_eq!((st, got), (3, 9));
    }

    #[test]
    fn append() {
        let mut rec = [7u8; 0xd0];
        rec[4] = 1;
        let (slot, n) = w23a08_append_record(5, 1, &rec).unwrap();
        assert_eq!(n, 6);
        assert_eq!(slot[8], 7);
        assert_eq!(slot[0], 0);
        assert!(w23a08_append_record(0xfa, 1, &rec).is_none());
        assert!(w23a08_append_record(0xf9, 0, &rec).is_none());
    }

    #[test]
    fn walks() {
        let next = |n: u32| n + 2;
        assert_eq!(w23a08_select_walk(0xff, 4, next), W23a08Walk::Fallback);
        assert_eq!(w23a08_select_walk(1, 3, next), W23a08Walk::Fallback);
        assert_eq!(w23a08_select_walk(2, 4, next), W23a08Walk::Active(Some(8)));
        assert_eq!(
            w23a08_select_walk(2, 4, |n| n + 1),
            W23a08Walk::Active(None)
        );
        assert_eq!(w23a08_select_walk(0, 4, next), W23a08Walk::Active(Some(4)));
        let chain = |n: u32| if n < 10 { n + 2 } else { 0 };
        assert_eq!(
            w23a08_select_walk(3, 4, chain),
            W23a08Walk::Active(Some(10))
        );
        assert!(w23a08_walk_hits(0, 4, chain));
        assert!(w23a08_walk_hits(3, 4, chain));
        assert!(!w23a08_walk_hits(4, 4, chain));
        assert!(!w23a08_walk_hits(0xff, 4, chain));
        assert!(!w23a08_walk_hits(0, 0, chain));
    }

    #[test]
    fn text() {
        assert_eq!(w23a08_scaled_ms_text(24, 1000), "24");
        assert_eq!(w23a08_scaled_ms_text(999, 1), "0");
    }

    #[test]
    fn lists() {
        assert!(w23a08_id_in_lists(5, &[3, 5, 0, 9], &[]));
        assert!(!w23a08_id_in_lists(9, &[3, 5, 0, 9], &[]));
        assert!(w23a08_id_in_lists(7, &[], &[1, 7]));
        assert!(!w23a08_id_in_lists(0x107, &[], &[7]));
    }

    #[test]
    fn cat() {
        let mut d = b"ab\0xx".to_vec();
        assert_eq!(w23a08_bounded_cat(&mut d, b"cdef\0", 5), 4);
        assert_eq!(d, b"abcd");
        let mut d = b"abc".to_vec();
        assert_eq!(w23a08_bounded_cat(&mut d, b"zz", 10), 5);
    }
}
