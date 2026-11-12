#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent10.rs
// ===========================================================================
// Wave 25 agent 10: small UI-list helpers.

/// Node of the singly linked widget list walked by `0x00964a60`.
#[derive(Clone, Copy, Debug, Default)]
pub struct W25a10Node {
    /// Link at dword 0 (used when `flag != 0`).
    pub next_primary: Option<usize>,
    /// Link at dword 0x17 (used when `flag == 0`).
    pub next_secondary: Option<usize>,
    /// Short at dword 0xf (offset 0x3c).
    pub flag: u16,
}

/// `0x00964a60`: stores `param3 >> 2` in the global short (returned) and calls
/// `visit(index)` on every node, choosing the next link by `flag`.
pub fn w25a10_walk_list(
    nodes: &[W25a10Node],
    start: Option<usize>,
    param3: u32,
    mut visit: impl FnMut(usize),
) -> u16 {
    let mut cur = start;
    while let Some(i) = cur {
        let n = &nodes[i];
        cur = if n.flag != 0 {
            n.next_primary
        } else {
            n.next_secondary
        };
        visit(i);
    }
    (param3 >> 2) as u16
}

/// `0x00979cc0`: returns the byte at offset 0x60 (item count).
pub fn w25a10_item_count(widget: &[u8]) -> u8 {
    widget[0x60]
}

/// `0x00983510`: `*out = height * width * 2` when the surface exists.
pub fn w25a10_surface_bytes(present: bool, width: i32, height: i32) -> Option<i32> {
    if !present {
        return None;
    }
    Some(height.wrapping_mul(width).wrapping_mul(2))
}

/// `0x00983b60`: true when `(flags & 0x7ff0) == 0x10`.
pub fn w25a10_flags_match(flags: u16) -> bool {
    flags & 0x7ff0 == 0x10
}

/// Subset of the dialog list state used by `0x009791b0`.
#[derive(Clone, Debug, Default)]
pub struct W25a10List {
    pub count: u8,
    pub capacity: u8,
    pub item_flags: Vec<u8>,
    pub item_text: Vec<String>,
}

/// `0x009791b0`: appends an item; returns the new index or `-1` when full.
/// Flag bit 0x10 means the text is duplicated (always owned here).
pub fn w25a10_add_item(list: &mut W25a10List, text: &str, flags: u8) -> i8 {
    if list.capacity <= list.count {
        return -1;
    }
    let idx = list.count as usize;
    if list.item_flags.len() <= idx {
        list.item_flags.resize(idx + 1, 0);
        list.item_text.resize(idx + 1, String::new());
    }
    list.item_flags[idx] = flags;
    list.item_text[idx] = text.to_string();
    list.count += 1;
    idx as i8
}

#[cfg(test)]
mod tests_wave25_agent10 {
    use super::*;

    #[test]
    fn walk() {
        let nodes = [
            W25a10Node {
                next_primary: Some(1),
                next_secondary: None,
                flag: 1,
            },
            W25a10Node {
                next_primary: None,
                next_secondary: Some(2),
                flag: 0,
            },
            W25a10Node::default(),
        ];
        let mut seen = vec![];
        let v = w25a10_walk_list(&nodes, Some(0), 0x41, |i| seen.push(i));
        assert_eq!(seen, vec![0, 1, 2]);
        assert_eq!(v, 0x10);
    }

    #[test]
    fn small() {
        let mut w = [0u8; 0x70];
        w[0x60] = 7;
        assert_eq!(w25a10_item_count(&w), 7);
        assert_eq!(w25a10_surface_bytes(true, 3, 4), Some(24));
        assert_eq!(w25a10_surface_bytes(false, 3, 4), None);
        assert!(w25a10_flags_match(0x8010));
        assert!(!w25a10_flags_match(0x0020));
    }

    #[test]
    fn add() {
        let mut l = W25a10List {
            capacity: 1,
            ..Default::default()
        };
        assert_eq!(w25a10_add_item(&mut l, "a", 0x10), 0);
        assert_eq!(w25a10_add_item(&mut l, "b", 0), -1);
    }
}
