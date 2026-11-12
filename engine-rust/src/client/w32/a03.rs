#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent03.rs
// ===========================================================================
// Wave 32 translation batch 03.
//
// Skipped: FUN_00674440 (thunk to another function), FUN_0068f332
// (`in` port instruction fragment).

/// One chained entry of the table walked by `0x00693350`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub struct w32a03_Node {
    /// Field `[1]`: the entry is visited only when non-zero.
    pub active: u32,
}

/// `0x00693350`: walks every bucket chain of a hash table; for each entry
/// with a non-zero flag it calls the virtual method at vtable offset 0x14
/// (`call`) until one returns true.  Once true, later calls are skipped but
/// the walk finishes.  Returns whether any call returned true.
pub fn w32a03_any_entry_call(
    buckets: &[Vec<w32a03_Node>],
    mut call: impl FnMut(usize, usize) -> bool,
) -> bool {
    let mut hit = false;
    for (b, chain) in buckets.iter().enumerate() {
        for (i, n) in chain.iter().enumerate() {
            if n.active != 0 && !hit && call(b, i) {
                hit = true;
            }
        }
    }
    hit
}

#[cfg(test)]
mod tests_wave32_agent03 {
    use super::*;

    #[test]
    fn short_circuits_calls() {
        let n = |a| w32a03_Node { active: a };
        let t = vec![vec![n(1), n(0), n(1)], vec![], vec![n(1)]];
        let mut calls = vec![];
        let r = w32a03_any_entry_call(&t, |b, i| {
            calls.push((b, i));
            b == 0 && i == 2
        });
        assert!(r);
        assert_eq!(calls, vec![(0, 0), (0, 2)]);
        assert!(!w32a03_any_entry_call(&[], |_, _| true));
    }
}
