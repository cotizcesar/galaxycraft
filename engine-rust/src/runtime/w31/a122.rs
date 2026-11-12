#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent122.rs
// ===========================================================================
// Wave 31 agent 122: CRT locale-aware case mapping and lock selection.

/// Which lock a CRT lock/unlock thunk acts on.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum W31a122LockTarget {
    /// Index 0: the global critical-section helper.
    Global,
    /// Indices 1..8: slot in the 0x18-byte lock table.
    Table(usize),
    /// Index >= 8: nothing is done.
    None,
}

/// `0x00c83e9c`: lock-index selector; 0 is the global lock, 1..7 index
/// the 0x18-byte lock table, anything else does nothing.
pub fn w31a122_lock_target(index: i32) -> W31a122LockTarget {
    if index == 0 {
        W31a122LockTarget::Global
    } else if index < 8 {
        W31a122LockTarget::Table(index as usize)
    } else {
        W31a122LockTarget::None
    }
}

/// `0x00c83ef4`: the unlock twin of the selector above; same index
/// classification (stride `0x18` into the lock table).
pub fn w31a122_unlock_target(index: i32) -> W31a122LockTarget {
    w31a122_lock_target(index)
}

/// `0x00c8427f`: `tolower` for the C locale path (no locale table):
/// only `A..=Z` (0x41..=0x5a) is shifted by 0x20.
pub fn w31a122_tolower_c(c: i32) -> i32 {
    if (0x41..=0x5a).contains(&c) {
        c + 0x20
    } else {
        c
    }
}

/// `0x00c843f0`: `toupper` for the C locale path: only `a..=z`
/// (0x61..=0x7a) is shifted by -0x20.
pub fn w31a122_toupper_c(c: i32) -> i32 {
    if (0x61..=0x7a).contains(&c) {
        c - 0x20
    } else {
        c
    }
}

/// `0x00c8427f` with a locale ctype table (`ctype[c*2] & 1` = upper bit):
/// returns whether the byte needs a locale lookup (`c < 0x100`).
pub fn w31a122_tolower_needs_map(ctype: &[u8], c: u32) -> bool {
    c >= 0x100 || ctype.get(c as usize * 2).is_some_and(|b| b & 1 != 0)
}

/// `0x00c843f0` locale variant: lower bit mask is `2` (lower-case flag).
pub fn w31a122_toupper_needs_map(ctype: &[u8], c: u32) -> bool {
    c >= 0x100 || ctype.get(c as usize * 2).is_some_and(|b| b & 2 != 0)
}

#[cfg(test)]
mod tests_wave31_agent122 {
    use super::*;

    #[test]
    fn lock_target() {
        assert_eq!(w31a122_lock_target(0), W31a122LockTarget::Global);
        assert_eq!(w31a122_lock_target(7), W31a122LockTarget::Table(7));
        assert_eq!(w31a122_unlock_target(8), W31a122LockTarget::None);
    }

    #[test]
    fn case_c() {
        assert_eq!(w31a122_tolower_c(0x40), 0x40);
        assert_eq!(w31a122_tolower_c(0x41), 0x61);
        assert_eq!(w31a122_tolower_c(0x5a), 0x7a);
        assert_eq!(w31a122_tolower_c(0x5b), 0x5b);
        assert_eq!(w31a122_toupper_c(0x60), 0x60);
        assert_eq!(w31a122_toupper_c(0x61), 0x41);
        assert_eq!(w31a122_toupper_c(0x7a), 0x5a);
        assert_eq!(w31a122_toupper_c(0x7b), 0x7b);
    }

    #[test]
    fn needs_map() {
        let t = [1u8, 0, 2, 0];
        assert!(w31a122_tolower_needs_map(&t, 0));
        assert!(!w31a122_tolower_needs_map(&t, 1));
        assert!(w31a122_toupper_needs_map(&t, 1));
        assert!(w31a122_toupper_needs_map(&t, 0x100));
    }
}
