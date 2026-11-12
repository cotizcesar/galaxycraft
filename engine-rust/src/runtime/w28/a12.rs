#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent12.rs
// ===========================================================================
// Wave 28 agent 12: libwebm `mkvparser` cluster helpers (retail 1.23.10 image).
// Most assigned rows are destructors that end in a non-returning free; only
// the two with real logic are translated.

/// Outcome of [`w28a12_insert_entry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W28a12Insert {
    /// Entry stored at its index, count incremented (returns true).
    Stored,
    /// Rejected (returns false).
    Rejected,
    /// Array is full; retail allocates a new array of `new_cap` slots.
    NeedsGrow { new_cap: i32 },
}

/// `0x00a6cde0`: insert an entry into the cluster entry-pointer array
/// (`this+0x84` base, `+0x88` count, `+0x8c` preloaded, `+0x90` capacity).
/// `elements` holds each slot's entry index (`entry+0x10`, `None` = null
/// slot).  `entry_idx` is the new entry's index (negative or null = reject).
pub fn w28a12_insert_entry(
    elements: &mut [Option<i32>],
    count: &mut i32,
    preload: i32,
    cap: i32,
    entry_idx: Option<i32>,
) -> W28a12Insert {
    let Some(idx) = entry_idx else {
        return W28a12Insert::Rejected;
    };
    if idx < 0 {
        return W28a12Insert::Rejected;
    }
    let total = count.wrapping_add(preload);
    if total > cap || idx != *count {
        return W28a12Insert::Rejected;
    }
    if total < cap {
        let store = |elements: &mut [Option<i32>], count: &mut i32| {
            elements[idx as usize] = Some(idx);
            *count += 1;
            W28a12Insert::Stored
        };
        if preload < 1 {
            return store(elements, count);
        }
        let first = elements[*count as usize];
        if matches!(first, Some(p) if p < 0) {
            let mut i = (*count + preload) as usize;
            if (i as i32) < cap {
                while let Some(p) = elements[i - 1] {
                    if p >= 0 {
                        break;
                    }
                    elements[i] = elements[i - 1];
                    if i - 1 == *count as usize {
                        return store(elements, count);
                    }
                    i -= 1;
                }
            }
            return W28a12Insert::Rejected;
        }
        // Slot empty or already a real entry: falls through to false.
        return W28a12Insert::Rejected;
    }
    W28a12Insert::NeedsGrow {
        new_cap: if cap < 1 { 0x800 } else { cap.wrapping_mul(2) },
    }
}

/// Decision taken by [`w28a12_add_child`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W28a12Child {
    /// Id not 0xa0/0xa3 (high dword 0): returns -1.
    Reject,
    /// First use: capacity set to 0x400, 0x1000 bytes allocated, count 0.
    InitArray,
    /// Array full: capacity doubled (allocation of `cap*2*4` bytes).
    GrowArray { new_cap: i32 },
    /// Id 0xa0 (BlockGroup): delegate to the group parser.
    ParseBlockGroup,
    /// Id 0xa3 (SimpleBlock): create entry at index `count`.
    CreateSimpleBlock { index: i32 },
}

/// `0x00a6d200`: `Cluster::ParseBlockEntry`-style dispatcher on element id
/// (`id_lo`, `id_hi`), with `this+0x30` array, `+0x34` capacity, `+0x38`
/// count (negative = not yet allocated).
pub fn w28a12_add_child(id_lo: i32, id_hi: i32, cap: i32, count: i32) -> W28a12Child {
    if !((id_lo == 0xa0 || id_lo == 0xa3) && id_hi == 0) {
        return W28a12Child::Reject;
    }
    if count < 0 {
        return W28a12Child::InitArray;
    }
    if cap <= count {
        return W28a12Child::GrowArray {
            new_cap: cap.wrapping_mul(2),
        };
    }
    if id_lo == 0xa0 {
        W28a12Child::ParseBlockGroup
    } else {
        W28a12Child::CreateSimpleBlock { index: count }
    }
}

#[cfg(test)]
mod tests_wave28_agent12 {
    use super::*;

    #[test]
    fn insert_basic_and_reject() {
        let mut el = vec![None; 4];
        let mut c = 0;
        assert_eq!(
            w28a12_insert_entry(&mut el, &mut c, 0, 4, Some(0)),
            W28a12Insert::Stored
        );
        assert_eq!(c, 1);
        assert_eq!(
            w28a12_insert_entry(&mut el, &mut c, 0, 4, Some(5)),
            W28a12Insert::Rejected
        );
        assert_eq!(
            w28a12_insert_entry(&mut el, &mut c, 0, 4, None),
            W28a12Insert::Rejected
        );
        assert_eq!(
            w28a12_insert_entry(&mut el, &mut c, 0, 4, Some(-1)),
            W28a12Insert::Rejected
        );
    }

    #[test]
    fn insert_grow_and_shift() {
        let mut el = vec![None; 2];
        let mut c = 0;
        assert_eq!(
            w28a12_insert_entry(&mut el, &mut c, 0, 0, Some(0)),
            W28a12Insert::NeedsGrow { new_cap: 0x800 }
        );
        let mut el = vec![None; 2];
        assert_eq!(
            w28a12_insert_entry(&mut el, &mut c, 2, 2, Some(0)),
            W28a12Insert::NeedsGrow { new_cap: 4 }
        );
        // one preloaded (negative) entry at slot 0, capacity 3
        let mut el = vec![Some(-1), None, None];
        let mut c = 0;
        assert_eq!(
            w28a12_insert_entry(&mut el, &mut c, 1, 3, Some(0)),
            W28a12Insert::Stored
        );
        assert_eq!(el, vec![Some(0), Some(-1), None]);
        assert_eq!(c, 1);
    }

    #[test]
    fn child_dispatch() {
        assert_eq!(w28a12_add_child(0x81, 0, 4, 0), W28a12Child::Reject);
        assert_eq!(w28a12_add_child(0xa0, 1, 4, 0), W28a12Child::Reject);
        assert_eq!(w28a12_add_child(0xa0, 0, 0, -1), W28a12Child::InitArray);
        assert_eq!(
            w28a12_add_child(0xa3, 0, 4, 4),
            W28a12Child::GrowArray { new_cap: 8 }
        );
        assert_eq!(
            w28a12_add_child(0xa0, 0, 4, 1),
            W28a12Child::ParseBlockGroup
        );
        assert_eq!(
            w28a12_add_child(0xa3, 0, 4, 2),
            W28a12Child::CreateSimpleBlock { index: 2 }
        );
    }
}
