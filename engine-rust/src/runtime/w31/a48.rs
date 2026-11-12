#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent48.rs
// ===========================================================================
// Wave 31 agent 48: thin option/setter wrappers (runtime.rs target).

/// `0x008af7d0`: sets the "MouseConfine" option from a single byte flag by
/// forwarding the name and the flag to the option-store setter.
pub fn w31a48_set_mouse_confine<F: FnMut(&str, u8)>(flag: u8, mut set_option: F) {
    set_option("MouseConfine", flag);
}

/// `0x008af840`: resolves `handle` with the flag `1` through the resolver
/// and passes the result to the consumer.
pub fn w31a48_resolve_flagged_then_apply<R, T>(
    handle: u32,
    resolve: impl FnOnce(u32, u32) -> R,
    apply: impl FnOnce(R) -> T,
) -> T {
    apply(resolve(handle, 1))
}

/// `0x008af8a0`: resolves `handle` through the resolver and passes the
/// result to the first consumer.
pub fn w31a48_resolve_then_apply_a<R, T>(
    handle: u32,
    resolve: impl FnOnce(u32) -> R,
    apply: impl FnOnce(R) -> T,
) -> T {
    apply(resolve(handle))
}

/// `0x008b04f0`: same shape as the previous wrapper but feeding the second
/// consumer.
pub fn w31a48_resolve_then_apply_b<R, T>(
    handle: u32,
    resolve: impl FnOnce(u32) -> R,
    apply: impl FnOnce(R) -> T,
) -> T {
    apply(resolve(handle))
}

#[cfg(test)]
mod tests_wave31_agent48 {
    use super::*;

    #[test]
    fn wrappers() {
        let mut got = None;
        w31a48_set_mouse_confine(1, |n, f| got = Some((n.to_string(), f)));
        assert_eq!(got, Some(("MouseConfine".to_string(), 1)));
        assert_eq!(
            w31a48_resolve_flagged_then_apply(5, |h, f| h + f, |r| r * 2),
            12
        );
        assert_eq!(w31a48_resolve_then_apply_a(5, |h| h + 1, |r| r), 6);
        assert_eq!(w31a48_resolve_then_apply_b(5, |h| h + 2, |r| r), 7);
    }
}
