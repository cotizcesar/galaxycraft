#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent88.rs
// ===========================================================================
// Wave 32 translation batch 88: RTTI type-name matching helpers.
//
// Virtual calls and the fallback name builder are caller-supplied closures.

fn w32a88_names_match(a: &[u8], b: &[u8]) -> bool {
    let n = a.len().max(b.len()) + 1;
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return false;
        }
        if x == 0 {
            return true;
        }
    }
    true
}

/// `0x00a7be10`: walks a linked list (next at +0x28); for each node whose
/// getter yields an object, returns it when `wanted` is `None` or when the
/// object's type name equals / compares equal (strncmp) to `wanted`.
pub fn w32a88_find_matching<T, N: Copy>(
    head: Option<N>,
    wanted: Option<&[u8]>,
    next: impl Fn(N) -> Option<N>,
    get: impl Fn(N) -> Option<T>,
    type_name: impl Fn(&T) -> Vec<u8>,
) -> Option<T> {
    let mut cur = head;
    while let Some(n) = cur {
        if let Some(obj) = get(n) {
            match wanted {
                None => return Some(obj),
                Some(w) => {
                    if w32a88_names_match(&type_name(&obj), w) {
                        return Some(obj);
                    }
                }
            }
        }
        cur = next(n);
    }
    None
}

/// `0x00a7d720`: fetches the other object's (vtable slot 0x10) type; returns
/// true when it is absent-false, else when its name matches `own_name`.
pub fn w32a88_type_matches_slot10<T>(
    other: Option<&T>,
    own_name: &[u8],
    type_name: impl Fn(&T) -> Vec<u8>,
) -> bool {
    match other {
        None => false,
        Some(o) => w32a88_names_match(&type_name(o), own_name),
    }
}

/// `0x00a7d790`: same as the slot 0x10 variant but for vtable slot 0x20.
pub fn w32a88_type_matches_slot20<T>(
    other: Option<&T>,
    own_name: &[u8],
    type_name: impl Fn(&T) -> Vec<u8>,
) -> bool {
    match other {
        None => false,
        Some(o) => w32a88_names_match(&type_name(o), own_name),
    }
}

#[cfg(test)]
mod tests_wave32_agent88 {
    use super::*;

    #[test]
    fn find() {
        let names = [b"A".to_vec(), b"B".to_vec()];
        let r = w32a88_find_matching(
            Some(0usize),
            Some(b"B"),
            |n| if n + 1 < 2 { Some(n + 1) } else { None },
            Some,
            |i| names[*i].clone(),
        );
        assert_eq!(r, Some(1));
        let r = w32a88_find_matching(Some(0usize), None, |_| None, Some, |i| names[*i].clone());
        assert_eq!(r, Some(0));
        let r = w32a88_find_matching(
            Some(0usize),
            Some(b"Z"),
            |_| None,
            Some,
            |i| names[*i].clone(),
        );
        assert_eq!(r, None);
    }

    #[test]
    fn slots() {
        assert!(!w32a88_type_matches_slot10::<u8>(None, b"A", |_| vec![]));
        assert!(w32a88_type_matches_slot10(Some(&1u8), b"A", |_| b"A".to_vec()));
        assert!(!w32a88_type_matches_slot20(Some(&1u8), b"A", |_| b"B".to_vec()));
    }
}
