#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent70.rs
// ===========================================================================
// Wave 33 translation batch 70.

/// `0x00cb645f`: releases a locale-like object unless it is null, the
/// static default instance, or flagged (slot 0x2c non-zero).  Calls
/// `release_a` then `release_b`; returns whether they ran.
pub fn w33a70_free_if_dynamic<T>(
    obj: Option<&mut T>,
    is_default: bool,
    slot_2c: u32,
    mut release_a: impl FnMut(&mut T),
    mut release_b: impl FnMut(&mut T),
) -> bool {
    match obj {
        Some(o) if !is_default && slot_2c == 0 => {
            release_a(o);
            release_b(o);
            true
        }
        _ => false,
    }
}

/// `0x00cb6c9c`: binary search over a table of 12-byte rows `(name, value, _)`.
/// `cmp(key, name)` is the three-way string compare.  On a match `*key` is
/// replaced with the row's value (offset +4).  `hi` is the last index
/// (negative means empty).  Returns whether a match was found.
pub fn w33a70_bsearch_table(
    rows: &[(u32, u32)],
    hi: i32,
    key: &mut u32,
    mut cmp: impl FnMut(u32, u32) -> i32,
) -> bool {
    let mut hi = hi;
    let mut lo = 0i32;
    let mut r = 1i32;
    if hi >= 0 {
        loop {
            let first = r == 0;
            r = 0;
            if first {
                break;
            }
            let mid = (lo + hi) / 2;
            let row = rows[mid as usize];
            r = cmp(*key, row.0);
            if r == 0 {
                *key = row.1;
            } else if r < 0 {
                hi = mid - 1;
            } else {
                lo = mid + 1;
            }
            if lo > hi {
                break;
            }
        }
    }
    r == 0
}

#[cfg(test)]
mod tests_wave33_agent70 {
    use super::*;

    #[test]
    fn free_runs() {
        let v = std::cell::RefCell::new(vec![]);
        let mut o = 1;
        assert!(w33a70_free_if_dynamic(
            Some(&mut o),
            false,
            0,
            |_| v.borrow_mut().push(1),
            |_| v.borrow_mut().push(2)
        ));
        assert_eq!(*v.borrow(), [1, 2]);
        assert!(!w33a70_free_if_dynamic(
            Some(&mut o),
            true,
            0,
            |_| {},
            |_| {}
        ));
        assert!(!w33a70_free_if_dynamic::<i32>(
            None,
            false,
            0,
            |_| {},
            |_| {}
        ));
    }

    #[test]
    fn search() {
        let rows = [(1, 10), (3, 30), (5, 50)];
        let mut k = 3;
        assert!(w33a70_bsearch_table(&rows, 2, &mut k, |a, b| a as i32 - b as i32));
        assert_eq!(k, 30);
        let mut k = 4;
        assert!(!w33a70_bsearch_table(&rows, 2, &mut k, |a, b| a as i32 - b as i32));
    }
}
