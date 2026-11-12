#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent15.rs
// ===========================================================================
// Wave 31 agent 15: small client-side helpers.

/// `0x008c5390`: if the handle is not `-1`, releases it through `release`
/// and resets it to `-1`.
pub fn w31a15_release_handle(handle: &mut i32, mut release: impl FnMut(i32)) {
    if *handle != -1 {
        release(*handle);
        *handle = -1;
    }
}

/// `0x008c1c10`: calls `probe`; when it returns false, releases the 16-bit
/// handle in the slot (offset `0x804`) and resets the slot to `0xffffffff`.
/// Returns the probe result.
pub fn w31a15_probe_or_release(
    slot: &mut u32,
    probe: impl FnOnce() -> bool,
    mut release: impl FnMut(u16),
) -> bool {
    let ok = probe();
    if !ok {
        release(*slot as u16);
        *slot = 0xffff_ffff;
    }
    ok
}

/// `0x008c5ef0`: forwards its argument to a callee and returns it unchanged.
pub fn w31a15_forward_arg(value: u32, mut callee: impl FnMut(u32)) -> u32 {
    callee(value);
    value
}

/// `0x008c0f60`: walks the list entries (`(ptr, len, handle)` triples) and
/// returns the index of the first whose compare against `name` yields 0.
/// `cmp(ptr, len, name_bytes)` models the callee `compare`.
pub fn w31a15_find_entry<T>(
    entries: &[T],
    name: &[u8],
    mut cmp: impl FnMut(&T, &[u8]) -> i32,
) -> Option<usize> {
    let len = name.iter().position(|&b| b == 0).unwrap_or(name.len());
    entries.iter().position(|e| cmp(e, &name[..len]) == 0)
}

/// `0x008b0920`: flag passed to the callee is whether the u16 at +0xe equals 10.
pub fn w31a15_is_kind_10_a(kind: u16) -> bool {
    kind == 10
}

#[cfg(test)]
mod tests_wave31_agent15 {
    use super::*;

    #[test]
    fn release_and_probe() {
        let mut h = 5;
        let mut got = vec![];
        w31a15_release_handle(&mut h, |x| got.push(x));
        w31a15_release_handle(&mut h, |x| got.push(x));
        assert_eq!((h, got), (-1, vec![5]));
        let mut s = 0x1_0007u32;
        let mut r = 0;
        assert!(w31a15_probe_or_release(&mut s, || true, |x| r = x));
        assert!(!w31a15_probe_or_release(&mut s, || false, |x| r = x));
        assert_eq!((s, r), (0xffff_ffff, 7));
    }

    #[test]
    fn forward_find_kind() {
        let mut seen = 0;
        assert_eq!(w31a15_forward_arg(9, |v| seen = v), 9);
        assert_eq!(seen, 9);
        let e = ["a", "bb", "cc"];
        let f = w31a15_find_entry(&e, b"cc\0zz", |x, n| (x.as_bytes() != n) as i32);
        assert_eq!(f, Some(2));
        assert!(w31a15_is_kind_10_a(10) && !w31a15_is_kind_10_a(9));
    }
}
