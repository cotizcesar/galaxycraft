#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent18.rs
// ===========================================================================
// Wave 33 agent 18.

/// `0x009023e0`: dispatches on `kind`.  Kinds 5, 7 and 9 first call their
/// handler (`on5` = FUN_00901d20, `on7` = FUN_00903230, `on9` = FUN_00902430);
/// a non-zero result returns 1.  Otherwise (or for any other kind) the
/// fallback FUN_00918290 result is returned.
pub fn w33a18_dispatch_kind(
    kind: i32,
    on5: impl FnOnce() -> bool,
    on7: impl FnOnce() -> bool,
    on9: impl FnOnce() -> bool,
    fallback: impl FnOnce(i32) -> u32,
) -> u32 {
    let handled = match kind {
        5 => on5(),
        7 => on7(),
        9 => on9(),
        _ => false,
    };
    if handled { 1 } else { fallback(kind) }
}

#[cfg(test)]
mod tests_wave33_agent18 {
    use super::*;

    #[test]
    fn dispatch() {
        assert_eq!(
            w33a18_dispatch_kind(5, || true, || false, || false, |_| 9),
            1
        );
        assert_eq!(
            w33a18_dispatch_kind(7, || true, || false, || true, |k| k as u32),
            7
        );
        assert_eq!(
            w33a18_dispatch_kind(9, || false, || false, || true, |_| 9),
            1
        );
        assert_eq!(w33a18_dispatch_kind(6, || true, || true, || true, |_| 4), 4);
    }
}
