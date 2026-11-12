#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave15/agent09.rs
// ===========================================================================
// Wave 15 translation batch 09.
//
// Most rows of this window are truncated by `noreturn` callees or call
// helpers whose arguments Ghidra dropped, so only three are recoverable.
//
// Skipped (reason): FUN_006e4a10 noreturn-truncated, args lost;
// FUN_006fa550 bare two-call wrapper; FUN_00717000, FUN_007197e0,
// FUN_00719a50 noreturn-truncated loops; FUN_007286d0 guard icall targets
// lost; FUN_00751140 noreturn-truncated; FUN_007517e0 thunk;
// FUN_00751f10 noreturn-truncated; FUN_00753c00, FUN_00754190,
// FUN_00754570, FUN_007654a0, FUN_007676d0 helper args lost;
// FUN_00769920, FUN_00769fc0, FUN_0076a330 noreturn-truncated.

/// `0x007141b0`: gate for the per-player callback.  Fires only when the
/// player pointer is non-null, `player_id` equals the local player id and the
/// two cached global ids (`DAT_00f2acec`, `DAT_00f2acf8`) match.  Returns
/// whether the callee (`FUN_007141e0`) is invoked.
pub fn w15a09_player_gate(
    has_player: bool,
    player_id: i32,
    local_player: i32,
    cached_a: i32,
    cached_b: i32,
) -> bool {
    has_player && player_id == local_player && cached_a == cached_b
}

/// `0x00751c10`: if the lock flag is zero, clears the pending flag (running
/// `on_clear` once when it was set) and runs `then`; otherwise does nothing.
/// Returns whether `then` ran.
pub fn w15a09_clear_pending_then(
    lock: u32,
    pending: &mut bool,
    on_clear: impl FnOnce(),
    then: impl FnOnce(),
) -> bool {
    if lock != 0 {
        return false;
    }
    if *pending {
        on_clear();
        *pending = false;
    }
    then();
    true
}

/// `0x00769880`: walks a linked list and returns the first node for which
/// `skip(node)` is false, `accept(node)` is true and the byte at offset
/// `0xd8` equals `wanted`; `None` when the list ends.
pub fn w15a09_find_node<T>(
    nodes: &[T],
    wanted: u32,
    skip: impl Fn(&T) -> bool,
    accept: impl Fn(&T) -> bool,
    byte_d8: impl Fn(&T) -> u8,
) -> Option<usize> {
    nodes
        .iter()
        .position(|n| !skip(n) && accept(n) && u32::from(byte_d8(n)) == wanted)
}

#[cfg(test)]
mod tests_wave15_agent09 {
    use super::*;

    #[test]
    fn gate() {
        assert!(w15a09_player_gate(true, 1, 1, 5, 5));
        assert!(!w15a09_player_gate(false, 1, 1, 5, 5));
        assert!(!w15a09_player_gate(true, 2, 1, 5, 5));
        assert!(!w15a09_player_gate(true, 1, 1, 5, 6));
    }

    #[test]
    fn pending() {
        let mut p = true;
        let (mut a, mut b) = (0, 0);
        assert!(w15a09_clear_pending_then(0, &mut p, || a += 1, || b += 1));
        assert!(!p);
        assert_eq!((a, b), (1, 1));
        let mut p = true;
        assert!(!w15a09_clear_pending_then(1, &mut p, || {}, || {}));
        assert!(p);
    }

    #[test]
    fn find() {
        let v = [(0u8, true, 3u8), (1, true, 4), (0, true, 4), (0, false, 4)];
        let r = w15a09_find_node(&v, 4, |n| n.0 != 0, |n| n.1, |n| n.2);
        assert_eq!(r, Some(2));
        assert_eq!(w15a09_find_node(&v, 9, |_| false, |_| true, |n| n.2), None);
    }
}
