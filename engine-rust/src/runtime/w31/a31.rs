#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent31.rs
// ===========================================================================
// Wave 31 agent 31: small helpers recovered for `runtime.rs`.

/// `0x006c54f0`: copies a player record into two tables.  For
/// `1 <= slot < 5` the first 0x1e bytes of `src` go to `names[slot * 0x1e..]`
/// and the first 0x74 bytes go to `rows[slot * 0x74..]`.  Returns false when
/// the slot is out of range or a buffer is too small.
pub fn w31a31_copy_player_slot(names: &mut [u8], rows: &mut [u8], slot: i32, src: &[u8]) -> bool {
    if slot <= 0 || slot >= 5 {
        return false;
    }
    let s = slot as usize;
    if src.len() < 0x74 || names.len() < s * 0x1e + 0x1e || rows.len() < s * 0x74 + 0x74 {
        return false;
    }
    names[s * 0x1e..s * 0x1e + 0x1e].copy_from_slice(&src[..0x1e]);
    rows[s * 0x74..s * 0x74 + 0x74].copy_from_slice(&src[..0x74]);
    true
}

/// `0x006dfa00`: returns `fallback()` when the override flag is set,
/// otherwise the table entry at `index`.
pub fn w31a31_table_or_fallback(
    override_flag: bool,
    table: &[u32],
    index: usize,
    fallback: impl FnOnce() -> u32,
) -> Option<u32> {
    if override_flag {
        Some(fallback())
    } else {
        table.get(index).copied()
    }
}

/// `0x006e90f0`: forwards the byte at offset 0x10 of the record to `sink`
/// and always reports success (1).
pub fn w31a31_forward_byte_field(record: &[u8], sink: impl FnOnce(u8)) -> u32 {
    sink(record.get(0x10).copied().unwrap_or(0));
    1
}

/// `0x006eceb0`: walks an ordered list (tagged next pointers: low bit set or
/// zero ends the list) counting nodes whose key is `<= key`.  Returns the
/// count and the first node with a greater key (the insertion successor).
/// Nodes are `(key, next)` pairs; `next` is an index, `None` at the end.
pub fn w31a31_count_sorted_prefix(
    nodes: &[(u32, Option<usize>)],
    head: Option<usize>,
    key: u32,
) -> (i32, Option<usize>) {
    let mut count = 0;
    let mut cur = head;
    while let Some(i) = cur {
        let Some(&(k, next)) = nodes.get(i) else {
            return (count, None);
        };
        if k > key {
            return (count, Some(i));
        }
        count += 1;
        cur = next;
    }
    (count, None)
}

/// Outcome of `0x006f8da0`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum W31a31Dispatch {
    /// Returned 1 without chaining.
    Consumed,
    /// Chained to the default handler.
    Chain,
}

/// `0x006f8da0`: message filter.  For message kind 0xe with code 0, when the
/// UI is idle and neither the 731110-style check nor `quiet` allows it, the
/// pre-handler is skipped; otherwise it runs.  Code 2 is consumed when the
/// check returns false.
pub fn w31a31_filter_message(
    quiet: bool,
    kind: u16,
    code: i32,
    idle: bool,
    idle_check: impl FnOnce() -> bool,
    mut pre_handler: impl FnMut(),
    code2_check: impl FnOnce() -> bool,
) -> W31a31Dispatch {
    if kind == 0xe {
        if code == 0 {
            if idle && !idle_check() && quiet {
                return W31a31Dispatch::Chain;
            }
            pre_handler();
        } else if code == 2 && !code2_check() {
            return W31a31Dispatch::Consumed;
        }
    }
    W31a31Dispatch::Chain
}

#[cfg(test)]
mod tests_wave31_agent31 {
    use super::*;

    #[test]
    fn copy_slot() {
        let mut n = [0u8; 0x1e * 5];
        let mut r = [0u8; 0x74 * 5];
        let src = [7u8; 0x74];
        assert!(!w31a31_copy_player_slot(&mut n, &mut r, 0, &src));
        assert!(!w31a31_copy_player_slot(&mut n, &mut r, 5, &src));
        assert!(w31a31_copy_player_slot(&mut n, &mut r, 4, &src));
        assert_eq!(n[4 * 0x1e], 7);
        assert_eq!(r[4 * 0x74 + 0x73], 7);
        assert_eq!(r[0], 0);
    }

    #[test]
    fn table_and_forward() {
        assert_eq!(w31a31_table_or_fallback(true, &[1], 0, || 9), Some(9));
        assert_eq!(w31a31_table_or_fallback(false, &[1, 2], 1, || 9), Some(2));
        assert_eq!(w31a31_table_or_fallback(false, &[1], 3, || 9), None);
        let mut got = 0;
        let mut rec = [0u8; 0x20];
        rec[0x10] = 5;
        assert_eq!(w31a31_forward_byte_field(&rec, |b| got = b), 1);
        assert_eq!(got, 5);
    }

    #[test]
    fn sorted_prefix() {
        let nodes = [(1, Some(1)), (3, Some(2)), (8, None)];
        assert_eq!(w31a31_count_sorted_prefix(&nodes, Some(0), 3), (2, Some(2)));
        assert_eq!(w31a31_count_sorted_prefix(&nodes, Some(0), 9), (3, None));
        assert_eq!(w31a31_count_sorted_prefix(&nodes, Some(0), 0), (0, Some(0)));
        assert_eq!(w31a31_count_sorted_prefix(&nodes, None, 0), (0, None));
    }

    #[test]
    fn filter() {
        let mut ran = 0;
        let r = w31a31_filter_message(true, 0xe, 0, true, || false, || ran += 1, || true);
        assert_eq!((r, ran), (W31a31Dispatch::Chain, 0));
        let r = w31a31_filter_message(false, 0xe, 0, true, || false, || ran += 1, || true);
        assert_eq!((r, ran), (W31a31Dispatch::Chain, 1));
        let r = w31a31_filter_message(false, 0xe, 2, false, || true, || {}, || false);
        assert_eq!(r, W31a31Dispatch::Consumed);
        let r = w31a31_filter_message(false, 3, 2, false, || true, || {}, || false);
        assert_eq!(r, W31a31Dispatch::Chain);
    }
}
