#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent04.rs
// ===========================================================================
// Wave 31 translation batch 04.
//
// Skipped (no `0x` prefix): FUN_0064eeb0 TLS guard glue, FUN_0064ef10 thunk,
// FUN_0066bb20 non-returning, FUN_0066d0c0 STL tree erase, FUN_00679c90 SEH,
// FUN_0067c5a0 STL tree bound, FUN_00681360 call chain, FUN_00688270 thunk,
// FUN_00688730 thunk, FUN_006ad69d SEH funclet, FUN_006bb560 opaque calls,
// FUN_006bb600 call chain.

/// `0x0064ef30`: walks a chained hash table (bucket heads in `heads`,
/// `count` buckets) calling `visit` on every node of every non-empty
/// bucket.  Retail only scans when bucket 0 is non-empty; otherwise it
/// takes the error path, reported here as `false`.  Node 0 means null.
pub fn w31a04_hash_visit_all(
    heads: &[usize],
    count: usize,
    next: impl Fn(usize) -> usize,
    mut visit: impl FnMut(usize),
) -> bool {
    let n = count.min(heads.len());
    if n == 0 || heads[0] == 0 {
        return false;
    }
    for &head in &heads[..n] {
        let mut node = head;
        while node != 0 {
            visit(node);
            node = next(node);
        }
    }
    true
}

/// `0x006a9830`: when the guard flag is set, releases the held handle.
pub fn w31a04_guard_release(handle: u32, armed: bool, mut release: impl FnMut(u32)) {
    if armed {
        release(handle);
    }
}

/// `0x006a9840`: when armed and the pointed-to object is non-null, invokes
/// `call` with the object and a zero flag.  Returns whether it called.
pub fn w31a04_guard_call(obj: u32, armed: bool, mut call: impl FnMut(u32, u32)) -> bool {
    if armed && obj != 0 {
        call(obj, 0);
        true
    } else {
        false
    }
}

/// `0x0067b060`: with `flag == 0`, queries `(status, count)`; if status is 0
/// and count is positive, resets.  Always returns 0.
pub fn w31a04_conditional_reset(
    flag: i32,
    query: impl FnOnce() -> (i32, i32),
    reset: impl FnOnce(),
) -> u32 {
    if flag == 0 {
        let (status, count) = query();
        if status == 0 && count > 0 {
            reset();
        }
    }
    0
}

/// `0x0068ac30`: scans `entries` (each `(a, b)`) and returns 1 as soon as
/// `cmp(a, b, key0, key1)` yields 0, otherwise 0.
pub fn w31a04_list_contains(
    entries: &[(u32, u32)],
    key0: u32,
    key1: u32,
    mut cmp: impl FnMut(u32, u32, u32, u32) -> i32,
) -> u32 {
    for &(a, b) in entries {
        if cmp(a, b, key0, key1) == 0 {
            return 1;
        }
    }
    0
}

#[cfg(test)]
mod tests_wave31_agent04 {
    use super::*;

    #[test]
    fn hash_visit() {
        let heads = [1usize, 0, 3];
        let nxt = |n: usize| if n == 1 { 2 } else { 0 };
        let mut seen = vec![];
        assert!(w31a04_hash_visit_all(&heads, 3, nxt, |n| seen.push(n)));
        assert_eq!(seen, vec![1, 2, 3]);
        assert!(!w31a04_hash_visit_all(&[0, 1], 2, nxt, |_| {}));
    }

    #[test]
    fn guards() {
        let mut v = vec![];
        w31a04_guard_release(7, true, |h| v.push(h));
        w31a04_guard_release(8, false, |h| v.push(h));
        assert_eq!(v, vec![7]);
        assert!(w31a04_guard_call(1, true, |_, f| assert_eq!(f, 0)));
        assert!(!w31a04_guard_call(0, true, |_, _| {}));
        assert!(!w31a04_guard_call(1, false, |_, _| {}));
    }

    #[test]
    fn reset_and_contains() {
        let mut r = 0;
        assert_eq!(w31a04_conditional_reset(0, || (0, 1), || r += 1), 0);
        assert_eq!(w31a04_conditional_reset(0, || (0, 0), || r += 1), 0);
        assert_eq!(w31a04_conditional_reset(1, || (0, 5), || r += 1), 0);
        assert_eq!(r, 1);
        let e = [(1, 2), (3, 4)];
        assert_eq!(
            w31a04_list_contains(&e, 3, 4, |a, b, k, l| (a != k || b != l) as i32),
            1
        );
        assert_eq!(
            w31a04_list_contains(&e, 9, 9, |a, b, k, l| (a != k || b != l) as i32),
            0
        );
    }
}
