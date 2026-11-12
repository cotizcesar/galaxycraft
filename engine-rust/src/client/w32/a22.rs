#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent22.rs
// ===========================================================================
// Wave 32 agent 22: three small handle-lookup wrappers (graphics device
// state accessors).  The handle resolver (FUN_009945c0) is modelled as a
// caller-supplied closure returning `Some(record)` on success.

/// `0x00984d20`: resolves `handle`; on success forwards `(a, b)` to the
/// callee (FUN_00990310).  Returns whether the callee ran.
pub fn w32a22_forward2<R, F: FnOnce(&R, u32, u32)>(
    resolve: impl FnOnce(u32) -> Option<R>,
    handle: u32,
    a: u32,
    b: u32,
    call: F,
) -> bool {
    match resolve(handle) {
        Some(r) => {
            call(&r, a, b);
            true
        }
        None => false,
    }
}

/// `0x00984d50`: resolves `handle`; on success forwards `a` to the callee
/// (FUN_00990690).  Returns whether the callee ran.
pub fn w32a22_forward1<R, F: FnOnce(&R, u32)>(
    resolve: impl FnOnce(u32) -> Option<R>,
    handle: u32,
    a: u32,
    call: F,
) -> bool {
    match resolve(handle) {
        Some(r) => {
            call(&r, a);
            true
        }
        None => false,
    }
}

/// Fields returned by `0x00984d80`; each is `None` when not requested.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct W32a22Info {
    pub f564: Option<u32>,
    pub f560: Option<u32>,
    pub f570: Option<u32>,
    pub f56c: Option<u32>,
    pub f5718: Option<u32>,
    pub extra: Option<u32>,
}

/// `0x00984d80`: reads record fields at +0x564, +0x560, +0x570, +0x56c and
/// +0x5718 (`rd` returns the dword at an offset) and derives the extra
/// value from the kind at +0x560 (1->8, 2->0x10, 3->0x18, 4|5->0x20, the
/// listed others and unknown kinds -> 0).  `want` flags the six outputs.
pub fn w32a22_query(rd: impl Fn(u32) -> u32, want: [bool; 6]) -> W32a22Info {
    let g = |i: usize, off: u32| if want[i] { Some(rd(off)) } else { None };
    let extra = if want[5] {
        Some(match rd(0x560) {
            1 => 8,
            2 => 0x10,
            3 => 0x18,
            4 | 5 => 0x20,
            _ => 0,
        })
    } else {
        None
    };
    W32a22Info {
        f564: g(0, 0x564),
        f560: g(1, 0x560),
        f570: g(2, 0x570),
        f56c: g(3, 0x56c),
        f5718: g(4, 0x5718),
        extra,
    }
}

#[cfg(test)]
mod tests_wave32_agent22 {
    use super::*;

    #[test]
    fn forwards() {
        let mut got = 0;
        assert!(w32a22_forward2(|h| Some(h), 1, 2, 3, |_, a, b| got = a + b));
        assert_eq!(got, 5);
        assert!(!w32a22_forward1(|_| None::<u32>, 1, 2, |_, _| panic!()));
    }

    #[test]
    fn query() {
        let i = w32a22_query(|o| if o == 0x560 { 4 } else { o }, [true; 6]);
        assert_eq!(i.extra, Some(0x20));
        assert_eq!(i.f564, Some(0x564));
        let j = w32a22_query(|_| 9, [false; 6]);
        assert_eq!(j, W32a22Info::default());
    }
}
