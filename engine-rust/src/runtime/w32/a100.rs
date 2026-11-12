#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent100.rs
// ===========================================================================
// Wave 32 agent 100 translations.

/// Fields read from the object passed in ECX by the two dispatchers below.
#[derive(Debug, Clone, Copy, Default)]
pub struct W32a100Obj {
    /// Offset 0x30.
    pub mode: i32,
    /// Offset 0x34.
    pub handle: i32,
    /// Offset 0x58.
    pub size_a: i32,
    /// Offset 0x5c.
    pub size_b: i32,
    /// Offset 0x61.
    pub flag: bool,
}

/// `0x00ae9f40`: returns 2 unless the handle is set, mode is 1 and the
/// predicate holds; otherwise forwards to the delegate.
pub fn w32a100_dispatch(
    obj: &W32a100Obj,
    pred: impl FnOnce() -> bool,
    delegate: impl FnOnce() -> u32,
) -> u32 {
    if obj.handle != 0 && obj.mode == 1 && pred() {
        return delegate();
    }
    2
}

/// `0x00aeaa50`: delegated query, else matches (0, 0, size_a, size_b).
pub fn w32a100_query(
    obj: &W32a100Obj,
    p: [i32; 4],
    pred: impl FnOnce() -> bool,
    delegate: impl FnOnce() -> u32,
) -> u32 {
    if obj.handle != 0 {
        if obj.mode == 1 && pred() {
            return delegate();
        }
        if obj.flag && p[0] == 0 && p[1] == 0 && p[2] == obj.size_a && p[3] == obj.size_b {
            return 1;
        }
    }
    0
}

#[cfg(test)]
mod tests_wave32_agent100 {
    use super::*;

    #[test]
    fn dispatch() {
        let mut o = W32a100Obj {
            handle: 1,
            mode: 1,
            ..Default::default()
        };
        assert_eq!(w32a100_dispatch(&o, || true, || 7), 7);
        assert_eq!(w32a100_dispatch(&o, || false, || 7), 2);
        o.mode = 0;
        assert_eq!(w32a100_dispatch(&o, || true, || 7), 2);
    }

    #[test]
    fn query() {
        let o = W32a100Obj {
            handle: 1,
            mode: 0,
            size_a: 3,
            size_b: 4,
            flag: true,
        };
        assert_eq!(w32a100_query(&o, [0, 0, 3, 4], || true, || 9), 1);
        assert_eq!(w32a100_query(&o, [0, 1, 3, 4], || true, || 9), 0);
        let o = W32a100Obj { mode: 1, ..o };
        assert_eq!(w32a100_query(&o, [0, 1, 3, 4], || true, || 9), 9);
    }
}
