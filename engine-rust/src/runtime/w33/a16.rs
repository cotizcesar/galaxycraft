#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent16.rs
// ===========================================================================
// Wave 33 agent 16 translations.

/// `0x008993a0`: key-code dispatch.  Codes 4 and 7 first ask their own
/// handler (`on_four` / `on_seven`); if it reports handled the result is
/// `true`, otherwise (and for every other code) the fallback handler runs.
pub fn w33a16_dispatch_key(
    code: i32,
    on_four: impl FnOnce() -> bool,
    on_seven: impl FnOnce() -> bool,
    fallback: impl FnOnce(i32) -> bool,
) -> bool {
    let handled = match code {
        4 => on_four(),
        7 => on_seven(),
        _ => false,
    };
    if handled {
        return true;
    }
    fallback(code)
}

/// `0x008a7760`: sets the 16-bit field at dword index 0x1be; when the
/// value changes it invokes the virtual slot at +0x144 and then the
/// refresh helper.  Returns whether the value changed.
pub fn w33a16_set_short_field(
    field: &mut i16,
    value: i16,
    virtual_slot: impl FnOnce(),
    refresh: impl FnOnce(),
) -> bool {
    if *field == value {
        return false;
    }
    *field = value;
    virtual_slot();
    refresh();
    true
}

#[cfg(test)]
mod tests_wave33_agent16 {
    use super::*;

    #[test]
    fn dispatch() {
        assert!(w33a16_dispatch_key(4, || true, || false, |_| false));
        assert!(!w33a16_dispatch_key(7, || true, || false, |_| false));
        assert!(w33a16_dispatch_key(1, || true, || true, |c| c == 1));
        assert!(w33a16_dispatch_key(4, || false, || false, |c| c == 4));
    }

    #[test]
    fn setter() {
        let mut f = 3i16;
        let mut calls = 0;
        assert!(!w33a16_set_short_field(&mut f, 3, || calls += 1, || {}));
        assert_eq!(calls, 0);
        assert!(w33a16_set_short_field(&mut f, 5, || calls += 1, || {}));
        assert_eq!((f, calls), (5, 1));
    }
}
