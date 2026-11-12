#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent123.rs
// ===========================================================================
// Wave 32 agent 123: lazily-resolved, hook-instrumented API trampolines.

/// Shared state of the trampoline family: the in-flight call counter and the
/// optional resolver / pre-hook / post-hook callbacks.
pub struct W32a123Hooks<'a> {
    /// In-flight call counter.
    pub counter: i32,
    /// Gate check (`FUN_00c810c0(0,0)`); non-zero aborts the call.
    pub gate: &'a mut dyn FnMut() -> i32,
    /// Resolver by hash id; may be absent.
    pub resolve: Option<&'a mut dyn FnMut(u32) -> Option<usize>>,
    /// Pre-call hook, fills the 64-bit timestamp/cookie.
    #[allow(clippy::type_complexity)]
    pub pre: Option<&'a mut dyn FnMut(u32, &mut u64)>,
    /// Post-call hook, receives the cookie halves and the result.
    pub post: Option<&'a mut dyn FnMut(u32, u32, u32, i32)>,
}

/// Common body of the two trampolines below.  `cached` is the cached target
/// slot; `call` invokes a resolved target with the three arguments.
pub fn w32a123_trampoline(
    h: &mut W32a123Hooks,
    cached: &mut Option<usize>,
    id: u32,
    args: (u32, u32, u32),
    call: &mut dyn FnMut(usize, u32, u32, u32) -> i32,
) -> i32 {
    h.counter += 1;
    let gate = (h.gate)();
    if gate != 0 {
        h.counter -= 1;
        return gate;
    }
    if cached.is_none() {
        if let Some(r) = h.resolve.as_mut() {
            *cached = r(id);
        }
        if cached.is_none() {
            h.counter -= 1;
            return -3;
        }
    }
    let target = cached.unwrap();
    let mut cookie: u64 = 0;
    if let Some(p) = h.pre.as_mut() {
        p(id, &mut cookie);
    }
    let rc = call(target, args.0, args.1, args.2);
    if let Some(p) = h.post.as_mut() {
        p(id, cookie as u32, (cookie >> 32) as u32, rc);
    }
    h.counter -= 1;
    rc
}

/// `0x00c81120`: trampoline for API id `0x5f68da40` (cache slot `0x0140e798`).
pub fn w32a123_call_5f68da40(
    h: &mut W32a123Hooks,
    cached: &mut Option<usize>,
    args: (u32, u32, u32),
    call: &mut dyn FnMut(usize, u32, u32, u32) -> i32,
) -> i32 {
    w32a123_trampoline(h, cached, 0x5f68da40, args, call)
}

/// `0x00c81460`: trampoline for API id `0x3dfacec8` (cache slot `0x0140e8ac`).
pub fn w32a123_call_3dfacec8(
    h: &mut W32a123Hooks,
    cached: &mut Option<usize>,
    args: (u32, u32, u32),
    call: &mut dyn FnMut(usize, u32, u32, u32) -> i32,
) -> i32 {
    w32a123_trampoline(h, cached, 0x3dfacec8, args, call)
}

#[cfg(test)]
mod tests_wave32_agent123 {
    use super::*;

    #[test]
    fn resolves_and_calls() {
        let mut gate = || 0;
        let mut res = |id: u32| if id == 0x5f68da40 { Some(7) } else { None };
        let mut seen = Vec::new();
        let mut post = |id: u32, _a: u32, _b: u32, rc: i32| seen.push((id, rc));
        let mut h = W32a123Hooks {
            counter: 0,
            gate: &mut gate,
            resolve: Some(&mut res),
            pre: None,
            post: Some(&mut post),
        };
        let mut cached = None;
        let mut call = |t: usize, a: u32, b: u32, c: u32| (t as u32 + a + b + c) as i32;
        assert_eq!(
            w32a123_call_5f68da40(&mut h, &mut cached, (1, 2, 3), &mut call),
            13
        );
        assert_eq!(h.counter, 0);
        assert_eq!(cached, Some(7));
        assert_eq!(
            w32a123_call_3dfacec8(&mut h, &mut None, (1, 2, 3), &mut call),
            -3
        );
        assert_eq!(h.counter, 0);
    }

    #[test]
    fn gate_fails() {
        let mut gate = || 5;
        let mut h = W32a123Hooks {
            counter: 0,
            gate: &mut gate,
            resolve: None,
            pre: None,
            post: None,
        };
        let mut call = |_: usize, _: u32, _: u32, _: u32| 0;
        assert_eq!(
            w32a123_call_3dfacec8(&mut h, &mut Some(1), (0, 0, 0), &mut call),
            5
        );
        assert_eq!(h.counter, 0);
    }
}
