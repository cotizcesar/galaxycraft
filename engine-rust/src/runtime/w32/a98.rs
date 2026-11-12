#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent98.rs
// ===========================================================================
// Wave 32 agent 98: lazy one-time initialisation guards.

/// `0x00ae8eb0`: lazy init of a list-head object (flag bytes at +0x10
/// "initialised", +0x11 "poisoned", +0x12 spin lock).  On first use sets
/// the head's self links (`links[0]`, `links[1]`) and size to the head
/// index/zero, marks it initialised, releases the lock and calls
/// `register_exit` (atexit registration).  Returns `true` when the
/// object pointer is returned (not poisoned), `false` for the null result.
/// The lock exchange is modelled as single-threaded.
pub fn w32a98_lazy_init_list_head(
    obj: &mut [u8; 0x13],
    this_addr: u32,
    register_exit: &mut dyn FnMut(),
) -> (bool, u32) {
    if obj[0x11] == 0 {
        obj[0x12] = 1; // lock taken
        if obj[0x10] == 0 {
            obj[0..4].copy_from_slice(&this_addr.to_le_bytes());
            obj[4..8].copy_from_slice(&this_addr.to_le_bytes());
            obj[8..12].copy_from_slice(&0u32.to_le_bytes());
            obj[0x10] = 1;
            obj[0x12] = 0;
            register_exit();
            return if obj[0x11] != 0 {
                (false, 0)
            } else {
                (true, this_addr)
            };
        }
        obj[0x12] = 0;
    }
    if obj[0x11] != 0 {
        (false, 0)
    } else {
        (true, this_addr)
    }
}

/// `0x00ae8f10`: lazy init of an object with flags at +8 "initialised",
/// +9 "poisoned", +10 spin lock.  First use runs `construct` (the callee
/// called with no arguments), marks initialised, releases the lock and
/// calls `register_exit`.  Returns `(not_poisoned, this_addr or 0)`.
pub fn w32a98_lazy_init_object(
    obj: &mut [u8; 0xb],
    this_addr: u32,
    construct: &mut dyn FnMut(),
    register_exit: &mut dyn FnMut(),
) -> (bool, u32) {
    if obj[9] == 0 {
        obj[10] = 1;
        if obj[8] == 0 {
            construct();
            obj[8] = 1;
            obj[10] = 0;
            register_exit();
            return if obj[9] != 0 {
                (false, 0)
            } else {
                (true, this_addr)
            };
        }
        obj[10] = 0;
    }
    if obj[9] != 0 {
        (false, 0)
    } else {
        (true, this_addr)
    }
}

#[cfg(test)]
mod tests_wave32_agent98 {
    use super::*;

    #[test]
    fn list_head_inits_once() {
        let mut o = [0u8; 0x13];
        let mut n = 0;
        let r = w32a98_lazy_init_list_head(&mut o, 0x1000, &mut || n += 1);
        assert_eq!(r, (true, 0x1000));
        assert_eq!(&o[0..4], &0x1000u32.to_le_bytes());
        assert_eq!(o[0x10], 1);
        let r = w32a98_lazy_init_list_head(&mut o, 0x1000, &mut || n += 1);
        assert_eq!(r, (true, 0x1000));
        assert_eq!(n, 1);
        o[0x11] = 1;
        assert_eq!(
            w32a98_lazy_init_list_head(&mut o, 5, &mut || n += 1),
            (false, 0)
        );
    }

    #[test]
    fn object_inits_once() {
        let mut o = [0u8; 0xb];
        let (mut c, mut e) = (0, 0);
        let mut ctor = || c += 1;
        let mut reg = || e += 1;
        assert_eq!(
            w32a98_lazy_init_object(&mut o, 7, &mut ctor, &mut reg),
            (true, 7)
        );
        assert_eq!(
            w32a98_lazy_init_object(&mut o, 7, &mut ctor, &mut reg),
            (true, 7)
        );
        o[9] = 1;
        assert_eq!(
            w32a98_lazy_init_object(&mut o, 7, &mut ctor, &mut reg),
            (false, 0)
        );
        assert_eq!((c, e), (1, 1));
    }
}
