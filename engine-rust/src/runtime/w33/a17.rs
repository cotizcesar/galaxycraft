#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent17.rs
// ===========================================================================
// Wave 33 agent 17 translations (runtime glue).

/// `0x008ba7d0`: Scrollbar constructor tail.  After the base constructor and
/// the vtable store, dwords `0x14..=0x18` of the object are zeroed.
pub fn w33a17_scrollbar_init_fields(obj: &mut [u32]) {
    for v in obj.iter_mut().skip(0x14).take(5) {
        *v = 0;
    }
}

/// `0x008bd100`: key/event dispatch.  `9` calls `h9`; `0x13`/`0x15` call
/// `h_alt` and, when it returns false, run `h_fallback` and return 1;
/// `0x13`/`0x15`/`0x12` then call `h12`.  A true handler result returns 1,
/// otherwise `default` handles the code.
pub fn w33a17_dispatch(
    code: u32,
    h9: impl FnOnce() -> bool,
    h_alt: impl FnOnce() -> bool,
    h_fallback: impl FnOnce(),
    h12: impl FnOnce() -> bool,
    default: impl FnOnce(u32) -> u32,
) -> u32 {
    let handled = match code {
        9 => h9(),
        0x13 | 0x15 => {
            if !h_alt() {
                h_fallback();
                return 1;
            }
            h12()
        }
        0x12 => h12(),
        _ => false,
    };
    if handled { 1 } else { default(code) }
}

#[cfg(test)]
mod tests_wave33_agent17 {
    use super::*;

    #[test]
    fn scrollbar_zeroes() {
        let mut o = [7u32; 0x20];
        w33a17_scrollbar_init_fields(&mut o);
        assert_eq!(o[0x13], 7);
        assert!(o[0x14..0x19].iter().all(|&v| v == 0));
        assert_eq!(o[0x19], 7);
    }

    #[test]
    fn dispatch_paths() {
        assert_eq!(
            w33a17_dispatch(9, || true, || true, || {}, || false, |_| 5),
            1
        );
        assert_eq!(
            w33a17_dispatch(0x13, || false, || false, || {}, || false, |_| 5),
            1
        );
        assert_eq!(
            w33a17_dispatch(0x13, || false, || true, || {}, || false, |_| 5),
            5
        );
        assert_eq!(
            w33a17_dispatch(0x12, || false, || true, || {}, || true, |_| 5),
            1
        );
        assert_eq!(
            w33a17_dispatch(3, || true, || true, || {}, || true, |c| c + 1),
            4
        );
    }
}
