#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent05.rs
// ===========================================================================
// Wave 26 agent 05: runtime.rs queue (mostly SEH/noreturn glue).

/// `0x00916170`: linear search over a circular list of nodes, returning the
/// index of the first node whose field at offset `0x24` equals `key`, or
/// `None` when the sentinel (end) is reached.
pub fn w26a05_find_node_by_key(keys: &[u32], key: u32) -> Option<usize> {
    keys.iter().position(|&k| k == key)
}

/// One row of the six-entry table built by `0x00914520`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W26a05Pair {
    pub id: u16,
    pub value: u32,
}

/// `0x00914520`: builds the constant six-row `(u16, u32)` table that is
/// passed (count 6, mode 2, flag 0) to the table consumer.
pub fn w26a05_build_table() -> [W26a05Pair; 6] {
    let p = |id, value| W26a05Pair { id, value };
    [p(1, 0), p(2, 2), p(3, 3), p(4, 2), p(0x13, 0), p(0x2a, 0)]
}

#[cfg(test)]
mod tests_wave26_agent05 {
    use super::*;

    #[test]
    fn find_node() {
        assert_eq!(w26a05_find_node_by_key(&[5, 7, 7], 7), Some(1));
        assert_eq!(w26a05_find_node_by_key(&[5], 9), None);
        assert_eq!(w26a05_find_node_by_key(&[], 0), None);
    }

    #[test]
    fn table() {
        let t = w26a05_build_table();
        assert_eq!(t[0], W26a05Pair { id: 1, value: 0 });
        assert_eq!(t[3].value, 2);
        assert_eq!(t[5].id, 0x2a);
    }
}
