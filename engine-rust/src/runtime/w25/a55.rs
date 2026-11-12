#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent55.rs
// ===========================================================================
// Wave 25 agent 55: scalar-deleting-destructor wrappers and string release checks.

/// `0x008cd5c0`: scalar deleting destructor wrapper.  Runs the destructor
/// body, then frees the object only when bit 0 of `flags` is set.
/// Returns whether the storage was freed.
pub fn w25a55_scalar_deleting_dtor_a(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) -> bool {
    dtor();
    if flags & 1 != 0 {
        free();
        true
    } else {
        false
    }
}

/// `0x008dca80`: same scalar deleting destructor shape as the wrapper above
/// with a different destructor body.
pub fn w25a55_scalar_deleting_dtor_b(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) -> bool {
    dtor();
    if flags & 1 != 0 {
        free();
        true
    } else {
        false
    }
}

/// `0x008cc3c0`: string-buffer release check.  The capacity/flag dword at
/// offset 0x18 is released when it is non-negative (`-1 < value`).
pub fn w25a55_buffer_needs_release(field_0x18: i32) -> bool {
    field_0x18 >= 0
}

#[cfg(test)]
mod tests_wave25_agent55 {
    use super::*;

    #[test]
    fn deleting_dtors() {
        let log = std::cell::RefCell::new(Vec::new());
        assert!(w25a55_scalar_deleting_dtor_a(
            1,
            || log.borrow_mut().push('d'),
            || log.borrow_mut().push('f')
        ));
        assert_eq!(*log.borrow(), ['d', 'f']);
        log.borrow_mut().clear();
        assert!(!w25a55_scalar_deleting_dtor_a(
            0,
            || log.borrow_mut().push('d'),
            || log.borrow_mut().push('f')
        ));
        assert_eq!(*log.borrow(), ['d']);
        assert!(w25a55_scalar_deleting_dtor_b(3, || {}, || {}));
        assert!(!w25a55_scalar_deleting_dtor_b(2, || {}, || {}));
    }

    #[test]
    fn release_check() {
        assert!(w25a55_buffer_needs_release(0));
        assert!(!w25a55_buffer_needs_release(-1));
    }
}
