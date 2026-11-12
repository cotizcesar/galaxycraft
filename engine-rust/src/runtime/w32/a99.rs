#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent99.rs
// ===========================================================================
// Wave 32 agent 99: lazy-init guards and a checked field read.

/// `0x00ae8f70`: one-shot lazy initialisation of a circular list head.
/// `obj` layout: +0 next, +4 prev (self links), +8 count, +0x10 inited,
/// +0x11 skip flag, +0x12 spin lock.  Returns true when the registration
/// callback (`FUN_00b8fa60` with the exit routine) must run.
pub fn w32a99_lazy_list_init(obj: &mut [u8; 0x14], self_addr: u32) -> bool {
    if obj[0x11] != 0 {
        return false;
    }
    // Spin lock acquire (exchange with 1 until the old value was 0 is
    // modelled as a single-threaded acquire).
    obj[0x12] = 1;
    if obj[0x10] == 0 {
        obj[0..4].copy_from_slice(&self_addr.to_le_bytes());
        obj[4..8].copy_from_slice(&self_addr.to_le_bytes());
        obj[8..12].copy_from_slice(&0u32.to_le_bytes());
        obj[0x10] = 1;
        obj[0x12] = 0;
        return true;
    }
    obj[0x12] = 0;
    false
}

/// `0x00ae8fc0`: one-shot lazy initialisation guard.  `obj` layout: +8
/// inited, +9 skip flag, +10 spin lock.  Runs `ctor` (`FUN_00c627c0`)
/// once and returns true when the exit registration must follow.
pub fn w32a99_lazy_init(obj: &mut [u8; 0xb], ctor: impl FnOnce()) -> bool {
    if obj[9] != 0 {
        return false;
    }
    obj[10] = 1;
    if obj[8] == 0 {
        ctor();
        obj[8] = 1;
        obj[10] = 0;
        return true;
    }
    obj[10] = 0;
    false
}

/// `0x00ae9f10`: when the pointer at +0x34 is non-null and the state at
/// +0x30 equals 1 and `check` (`FUN_00b0ab10`) is true, returns the dword
/// at offset 4 of the pointed record (`read_u32_at`), else 0.
pub fn w32a99_checked_field(
    state: u32,
    ptr: u32,
    check: impl FnOnce() -> bool,
    read_u32_at: impl FnOnce(u32) -> u32,
) -> u32 {
    if ptr != 0 && state == 1 && check() {
        return read_u32_at(ptr.wrapping_add(4));
    }
    0
}

#[cfg(test)]
mod tests_wave32_agent99 {
    use super::*;

    #[test]
    fn list_init_once() {
        let mut o = [0u8; 0x14];
        assert!(w32a99_lazy_list_init(&mut o, 0x1000));
        assert_eq!(&o[0..4], &0x1000u32.to_le_bytes());
        assert!(!w32a99_lazy_list_init(&mut o, 0x1000));
        o[0x10] = 0;
        o[0x11] = 1;
        assert!(!w32a99_lazy_list_init(&mut o, 5));
    }

    #[test]
    fn lazy_init_once() {
        let mut o = [0u8; 0xb];
        let mut n = 0;
        assert!(w32a99_lazy_init(&mut o, || n += 1));
        assert!(!w32a99_lazy_init(&mut o, || n += 1));
        assert_eq!(n, 1);
    }

    #[test]
    fn checked() {
        assert_eq!(w32a99_checked_field(1, 8, || true, |a| a), 12);
        assert_eq!(w32a99_checked_field(2, 8, || true, |a| a), 0);
        assert_eq!(w32a99_checked_field(1, 0, || true, |a| a), 0);
        assert_eq!(w32a99_checked_field(1, 8, || false, |a| a), 0);
    }
}
