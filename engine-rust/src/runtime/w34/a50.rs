#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent50.rs
// ===========================================================================
// Wave 34 batch 50: API-id trampolines (counter, gate, lazy resolve, hooks).

/// Hook state shared by the trampolines (globals `0x0140e064`, resolver,
/// pre-call and post-call hooks).
pub struct W34a50Hooks<'a> {
    pub counter: i32,
    pub gate: &'a mut dyn FnMut() -> i32,
    pub resolve: Option<&'a mut dyn FnMut(u32) -> Option<usize>>,
    #[allow(clippy::type_complexity)]
    pub pre: Option<&'a mut dyn FnMut(u32, &mut u64)>,
    pub post: Option<&'a mut dyn FnMut(u32, u32, u32, i32)>,
}

fn w34a50_trampoline<F: FnMut(usize) -> i32>(
    h: &mut W34a50Hooks,
    cached: &mut Option<usize>,
    id: u32,
    mut call: F,
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
    let rc = call(target);
    if let Some(p) = h.post.as_mut() {
        p(id, cookie as u32, (cookie >> 32) as u32, rc);
    }
    h.counter -= 1;
    rc
}

/// `0x00c811e0`: trampoline for API id `0x59e890f9` (5 args, slot `0x0140e910`).
pub fn w34a50_call_59e890f9(
    h: &mut W34a50Hooks,
    cached: &mut Option<usize>,
    args: [u32; 5],
    call: &mut dyn FnMut(usize, [u32; 5]) -> i32,
) -> i32 {
    w34a50_trampoline(h, cached, 0x59e890f9, |t| call(t, args))
}

/// `0x00c812b0`: trampoline for API id `0xd4e26bbf` (5 args, slot `0x0140e90c`).
pub fn w34a50_call_d4e26bbf(
    h: &mut W34a50Hooks,
    cached: &mut Option<usize>,
    args: [u32; 5],
    call: &mut dyn FnMut(usize, [u32; 5]) -> i32,
) -> i32 {
    w34a50_trampoline(h, cached, 0xd4e26bbf, |t| call(t, args))
}

/// `0x00c81380`: trampoline for API id `0x7aaf7a04` (4 args, slot `0x0140e794`).
pub fn w34a50_call_7aaf7a04(
    h: &mut W34a50Hooks,
    cached: &mut Option<usize>,
    args: [u32; 4],
    call: &mut dyn FnMut(usize, [u32; 4]) -> i32,
) -> i32 {
    w34a50_trampoline(h, cached, 0x7aaf7a04, |t| call(t, args))
}

#[cfg(test)]
mod tests_wave34_agent50 {
    use super::*;

    #[test]
    fn resolves_and_calls() {
        let mut gate = || 0;
        let mut res = |id: u32| if id == 0x7aaf7a04 { Some(9) } else { None };
        let mut h = W34a50Hooks {
            counter: 0,
            gate: &mut gate,
            resolve: Some(&mut res),
            pre: None,
            post: None,
        };
        let mut c = None;
        let mut call = |t: usize, a: [u32; 4]| (t as i32) + a[3] as i32;
        assert_eq!(
            w34a50_call_7aaf7a04(&mut h, &mut c, [1, 2, 3, 4], &mut call),
            13
        );
        assert_eq!(h.counter, 0);
        let mut c2 = None;
        let mut call5 = |_t: usize, _a: [u32; 5]| 0;
        assert_eq!(
            w34a50_call_59e890f9(&mut h, &mut c2, [0; 5], &mut call5),
            -3
        );
        assert_eq!(
            w34a50_call_d4e26bbf(&mut h, &mut c2, [0; 5], &mut call5),
            -3
        );
        assert_eq!(h.counter, 0);
    }
}
