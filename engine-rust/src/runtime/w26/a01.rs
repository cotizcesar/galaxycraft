#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent01.rs
// ===========================================================================
// Wave 26 translation batch 01: exception-unwind funclets reduced to their
// recoverable state updates.

/// `0x0091b4a0`: drain step of two counted ranges.  Each `(cursor, count)`
/// pair is advanced until `count` reaches zero (cursor += count).  The real
/// routine then tail-calls the noreturn unwinder; that part is not modelled.
pub fn w26a01_drain_ranges(a: (i32, i32), b: (i32, i32)) -> ((i32, i32), (i32, i32)) {
    fn drain(r: (i32, i32)) -> (i32, i32) {
        (r.0.wrapping_add(r.1), 0)
    }
    (drain(a), drain(b))
}

/// `0x00924e00`: releases a shared reference.  Decrements the strong count
/// and returns `(new_count, destroy_object, release_second_ref)` where the
/// object is destroyed when the count reaches zero and the secondary
/// pointer is released when non-zero.  The slot is then cleared by the caller.
pub fn w26a01_release_shared(count: i32, secondary: u32) -> (i32, bool, bool) {
    let c = count.wrapping_sub(1);
    (c, c == 0, secondary != 0)
}

/// `0x00900050`: resets an intrusive circular list head at `head_addr`:
/// `next = head`, `prev` unchanged, count = 0.  Returns `(next, count,
/// old_prev_was_foreign)`; foreign means old prev != head (nodes pending
/// free).
pub fn w26a01_reset_list_head(head_addr: u32, old_prev: u32) -> (u32, u32, bool) {
    (head_addr, 0, old_prev != head_addr)
}

#[cfg(test)]
mod tests_wave26_agent01 {
    use super::*;

    #[test]
    fn drain() {
        assert_eq!(w26a01_drain_ranges((10, 3), (0, 0)), ((13, 0), (0, 0)));
    }

    #[test]
    fn release() {
        assert_eq!(w26a01_release_shared(1, 5), (0, true, true));
        assert_eq!(w26a01_release_shared(2, 0), (1, false, false));
    }

    #[test]
    fn list_reset() {
        assert_eq!(w26a01_reset_list_head(100, 100), (100, 0, false));
        assert_eq!(w26a01_reset_list_head(100, 7), (100, 0, true));
    }
}
