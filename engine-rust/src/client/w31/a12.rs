#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent12.rs
// ===========================================================================
// Wave 31 agent 12: small client-side comparison and lookup helpers.

/// `0x00848900`: inequality test of two records.  Returns 0 when both
/// leading words match and the caller-supplied tail comparison
/// (`FUN_00539640` in retail) reports equality, otherwise 1.
pub fn w31a12_record_not_equal(
    a: (u32, u32),
    b: (u32, u32),
    tail_equal: impl FnOnce() -> bool,
) -> u32 {
    if a.0 == b.0 && a.1 == b.1 && tail_equal() {
        0
    } else {
        1
    }
}

/// `0x00854940`: scans a list of entries in order and returns the first
/// whose name compares equal (comparator result 0, as `FUN_0053a3e0` in
/// retail) to `key`; `None` when no entry matches.
pub fn w31a12_find_by_name<'a, T>(
    entries: &'a [T],
    name_of: impl Fn(&T) -> &str,
    key: &str,
    compare: impl Fn(&str, &str) -> i32,
) -> Option<&'a T> {
    entries.iter().find(|e| compare(key, name_of(e)) == 0)
}

#[cfg(test)]
mod tests_wave31_agent12 {
    use super::*;

    #[test]
    fn not_equal_cases() {
        assert_eq!(w31a12_record_not_equal((1, 2), (1, 2), || true), 0);
        assert_eq!(w31a12_record_not_equal((1, 2), (1, 2), || false), 1);
        assert_eq!(w31a12_record_not_equal((1, 2), (1, 3), || true), 1);
        assert_eq!(w31a12_record_not_equal((0, 2), (1, 2), || true), 1);
    }

    #[test]
    fn find_by_name_first_match() {
        let v = ["a", "b", "b"];
        let cmp = |x: &str, y: &str| if x == y { 0 } else { 1 };
        let r = w31a12_find_by_name(&v, |s| s, "b", cmp).unwrap();
        assert!(std::ptr::eq(r, &v[1]));
        assert!(w31a12_find_by_name(&v, |s| s, "z", cmp).is_none());
    }
}
