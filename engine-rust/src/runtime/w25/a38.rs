#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent38.rs
// ===========================================================================
// Wave 25 agent 38: tiny shared-ptr-manager destructors and a string-key helper.

/// Outcome of a scalar-deleting destructor: whether the object memory is freed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W25a38DtorResult {
    /// The vtable pointer was reset to `blz::_shared_ptr_manager::vftable`.
    pub vtable_reset: bool,
    /// Bit 0 of the flag was set, so the operator-delete helper is called.
    pub free_memory: bool,
}

/// Shared body of the three destructors.
pub fn w25a38_shared_ptr_manager_dtor(flags: u8) -> W25a38DtorResult {
    W25a38DtorResult {
        vtable_reset: true,
        free_memory: flags & 1 != 0,
    }
}

/// `0x0086f4d0`: shared-ptr manager scalar deleting destructor.
pub fn w25a38_dtor_86f4d0(flags: u8) -> W25a38DtorResult {
    w25a38_shared_ptr_manager_dtor(flags)
}

/// `0x0086f500`: shared-ptr manager scalar deleting destructor.
pub fn w25a38_dtor_86f500(flags: u8) -> W25a38DtorResult {
    w25a38_shared_ptr_manager_dtor(flags)
}

/// `0x0086f530`: shared-ptr manager scalar deleting destructor.
pub fn w25a38_dtor_86f530(flags: u8) -> W25a38DtorResult {
    w25a38_shared_ptr_manager_dtor(flags)
}

/// `0x00871800`: forwards a (ptr, len) pair plus the literal key
/// `"harassment"` (length 10) to the string-compare helper.
pub fn w25a38_harassment_key<R>(pair: (u32, u32), f: impl FnOnce(u32, u32, &str, u32) -> R) -> R {
    f(pair.0, pair.1, "harassment", 10)
}

#[cfg(test)]
mod tests_wave25_agent38 {
    use super::*;

    #[test]
    fn dtors_free_only_on_bit0() {
        for f in [w25a38_dtor_86f4d0, w25a38_dtor_86f500, w25a38_dtor_86f530] {
            assert!(f(1).free_memory && f(3).free_memory);
            assert!(!f(0).free_memory && !f(2).free_memory);
            assert!(f(0).vtable_reset);
        }
    }

    #[test]
    fn harassment_key_args() {
        let r = w25a38_harassment_key((5, 6), |a, b, s, n| (a, b, s.len() as u32, n));
        assert_eq!(r, (5, 6, 10, 10));
    }
}
