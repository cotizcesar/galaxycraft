#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent38.rs
// ===========================================================================
// Wave 34 agent 38: small dispatch helpers.

/// Fields of the record read through `this` by `0x00538450`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W34a38Record {
    pub f08: u32,
    pub f0c: u32,
    pub f10: u32,
    pub f14: u32,
    pub f18: u32,
    pub flag_1c: u8,
}

/// Which callee `0x00538450` dispatched to, with its arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W34a38Call {
    /// FUN_00536d90(f14, f18, f08, f0c, f10, param)
    Wide([u32; 6]),
    /// FUN_005367b0(f08, f0c, f10)
    Narrow([u32; 3]),
}

/// `0x00538450`: when the flag byte at +0x1c is non-zero, calls the six
/// argument routine with (+0x14, +0x18, +0x8, +0xc, +0x10, param); otherwise
/// calls the three argument routine with (+0x8, +0xc, +0x10).
pub fn w34a38_dispatch(rec: &W34a38Record, param: u32) -> W34a38Call {
    if rec.flag_1c != 0 {
        W34a38Call::Wide([rec.f14, rec.f18, rec.f08, rec.f0c, rec.f10, param])
    } else {
        W34a38Call::Narrow([rec.f08, rec.f0c, rec.f10])
    }
}

/// `0x005386a0`: if the object exists, invokes its first vtable slot
/// (scalar deleting destructor) with flag 1.  Returns whether it was called.
pub fn w34a38_delete_virtual<F: FnOnce(u32)>(obj_present: bool, slot0: F) -> bool {
    if obj_present {
        slot0(1);
    }
    obj_present
}

/// `0x005386b0`: tail-calls vtable slot 1 (offset +4) of the object held at
/// +0xc of `this`.
pub fn w34a38_forward_slot1<R, F: FnOnce() -> R>(slot1: F) -> R {
    slot1()
}

#[cfg(test)]
mod tests_wave34_agent38 {
    use super::*;

    #[test]
    fn dispatch_picks_callee() {
        let mut r = W34a38Record {
            f08: 1,
            f0c: 2,
            f10: 3,
            f14: 4,
            f18: 5,
            flag_1c: 0,
        };
        assert_eq!(w34a38_dispatch(&r, 9), W34a38Call::Narrow([1, 2, 3]));
        r.flag_1c = 1;
        assert_eq!(w34a38_dispatch(&r, 9), W34a38Call::Wide([4, 5, 1, 2, 3, 9]));
    }

    #[test]
    fn delete_and_forward() {
        let mut got = 0;
        assert!(w34a38_delete_virtual(true, |f| got = f));
        assert_eq!(got, 1);
        assert!(!w34a38_delete_virtual(false, |_| panic!()));
        assert_eq!(w34a38_forward_slot1(|| 7), 7);
    }
}
