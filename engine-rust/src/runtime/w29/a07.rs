#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent07.rs
// ===========================================================================
// Wave 29 agent 07: length-prefixed compare and shared-pointer release.

/// `0x00ac1cc0`: equality of two length-prefixed byte records (first byte is
/// the length, followed by that many bytes).  Returns false on a length
/// mismatch, otherwise true when all payload bytes match.
pub fn w29a07_prefixed_eq(a: &[u8], b: &[u8]) -> bool {
    let (Some(&la), Some(&lb)) = (a.first(), b.first()) else {
        return false;
    };
    if la != lb {
        return false;
    }
    let n = la as usize;
    match (a.get(1..1 + n), b.get(1..1 + n)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

/// Control block of an intrusive shared pointer: use count at +4, weak count
/// at +8 (the vtable pointer sits at +0).
pub struct W29a07Ctrl {
    pub uses: std::sync::atomic::AtomicI32,
    pub weaks: std::sync::atomic::AtomicI32,
}

/// Release step used by `0x00ac1950` for each of its two shared members:
/// decrement uses; when it was 1 call `dispose` (vtable +8), then decrement
/// weaks and when that was 1 call `destroy` (vtable +0xc).
pub fn w29a07_release(
    c: Option<&W29a07Ctrl>,
    dispose: &mut dyn FnMut(),
    destroy: &mut dyn FnMut(),
) {
    use std::sync::atomic::Ordering::SeqCst;
    let Some(c) = c else { return };
    if c.uses.fetch_sub(1, SeqCst) == 1 {
        dispose();
        if c.weaks.fetch_sub(1, SeqCst) == 1 {
            destroy();
        }
    }
}

/// `0x00ac1950`: destructor body releasing the shared pointer at +0x10 then
/// the one at +8 (after the `+0x48` string-free check, a caller concern).
pub fn w29a07_drop_two_shared(
    first: Option<&W29a07Ctrl>,
    second: Option<&W29a07Ctrl>,
    dispose: &mut dyn FnMut(u8),
    destroy: &mut dyn FnMut(u8),
) {
    w29a07_release(first, &mut || dispose(0), &mut || destroy(0));
    w29a07_release(second, &mut || dispose(1), &mut || destroy(1));
}

// Skipped as "bnet" (tact::ClientHandler/KeyRing, CDN client) or
// non-returning glue: FUN_00ac0920 FUN_00ac0960 FUN_00ac0ad0 FUN_00ac0de0
// FUN_00ac1130 FUN_00ac1500 FUN_00ac1620 FUN_00ac1690 FUN_00ac16e0
// FUN_00ac18d0 FUN_00ac1a40 FUN_00ac1ab0 FUN_00ac1b30 FUN_00ac1b50
// FUN_00ac1c50 FUN_00ac1d30 FUN_00ac1d60 FUN_00ac1d90.

#[cfg(test)]
mod tests_wave29_agent07 {
    use super::*;
    use std::sync::atomic::AtomicI32;

    #[test]
    fn prefixed_eq_cases() {
        assert!(w29a07_prefixed_eq(&[3, 1, 2, 3], &[3, 1, 2, 3, 9]));
        assert!(!w29a07_prefixed_eq(&[3, 1, 2, 3], &[3, 1, 2, 4]));
        assert!(!w29a07_prefixed_eq(&[2, 1, 2], &[3, 1, 2, 3]));
        assert!(w29a07_prefixed_eq(&[0], &[0]));
        assert!(!w29a07_prefixed_eq(&[], &[0]));
        assert!(!w29a07_prefixed_eq(&[5, 1], &[5, 1]));
    }

    #[test]
    fn release_and_drop() {
        let a = W29a07Ctrl {
            uses: AtomicI32::new(1),
            weaks: AtomicI32::new(1),
        };
        let b = W29a07Ctrl {
            uses: AtomicI32::new(2),
            weaks: AtomicI32::new(1),
        };
        let mut log = Vec::new();
        let log_cell = std::cell::RefCell::new(&mut log);
        w29a07_drop_two_shared(
            Some(&a),
            Some(&b),
            &mut |i| log_cell.borrow_mut().push(('d', i)),
            &mut |i| log_cell.borrow_mut().push(('x', i)),
        );
        assert_eq!(log, vec![('d', 0), ('x', 0)]);
        assert_eq!(b.uses.load(std::sync::atomic::Ordering::SeqCst), 1);
        w29a07_release(None, &mut || panic!(), &mut || panic!());
    }
}
