#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent53.rs
// ===========================================================================
// Wave 25 agent 53: small runtime/glue helpers.

/// `0x008c51f0`: releases a handle slot.  When the slot is not `-1` the
/// release callback runs on it and the slot is reset to `-1`.
pub fn w25a53_release_handle(slot: &mut i32, mut release: impl FnMut(i32)) {
    if *slot != -1 {
        release(*slot);
        *slot = -1;
    }
}

/// `0x008c6790`: event filter.  `kind` is the event code (`*param`), `ty`
/// the 16-bit event type.  For type 0xe, kinds 4/5 are consumed when
/// `a()` is true and `b()` is false (kind 5 also calls `act(true)`), kind 6
/// calls `act(false)` then falls through to the generic handlers
/// `first` then `second`.
pub fn w25a53_filter_event(
    kind: i32,
    ty: i16,
    mut a: impl FnMut() -> bool,
    mut b: impl FnMut() -> bool,
    mut act: impl FnMut(bool),
    mut first: impl FnMut() -> i32,
    mut second: impl FnMut() -> i32,
) -> i32 {
    if ty == 0xe {
        match kind {
            4 => return i32::from(a() && !b()),
            5 => {
                if a() && !b() {
                    act(true);
                    return 1;
                }
                return 0;
            }
            6 => act(false),
            _ => {}
        }
    }
    let r = first();
    if r == 0 { second() } else { r }
}

/// `0x008c6e70`: initialises an empty small-string header: size 0, data
/// pointer to inline buffer (index 3 words in), capacity word
/// `0x8000000f`, first inline byte 0.  Returns `[ptr_marker, len, cap, first]`.
pub fn w25a53_init_empty_string() -> [u32; 4] {
    [3, 0, 0x8000_000f, 0]
}

#[cfg(test)]
mod tests_wave25_agent53 {
    use super::*;

    #[test]
    fn release_handle() {
        let mut s = 5;
        let mut got = vec![];
        w25a53_release_handle(&mut s, |h| got.push(h));
        assert_eq!((s, got.clone()), (-1, vec![5]));
        w25a53_release_handle(&mut s, |h| got.push(h));
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn filter_event() {
        let r = w25a53_filter_event(4, 0xe, || true, || false, |_| {}, || 9, || 8);
        assert_eq!(r, 1);
        let mut acted = None;
        let r = w25a53_filter_event(5, 0xe, || true, || false, |v| acted = Some(v), || 9, || 8);
        assert_eq!((r, acted), (1, Some(true)));
        assert_eq!(
            w25a53_filter_event(5, 0xe, || true, || true, |_| {}, || 9, || 8),
            0
        );
        let mut acted = None;
        let r = w25a53_filter_event(6, 0xe, || true, || true, |v| acted = Some(v), || 0, || 8);
        assert_eq!((r, acted), (8, Some(false)));
        assert_eq!(
            w25a53_filter_event(4, 1, || false, || false, |_| {}, || 3, || 8),
            3
        );
    }

    #[test]
    fn empty_string() {
        assert_eq!(w25a53_init_empty_string()[2], 0x8000_000f);
    }
}
