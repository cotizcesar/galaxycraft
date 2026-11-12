#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent27.rs
// ===========================================================================
// Wave 25 agent 27: deque slot allocation and small field helpers.

/// Result of the deque push-back slot lookup.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum W25a27Slot {
    /// The block map must grow first (tail call into the grow routine).
    NeedGrow,
    /// The block pointed to by the map entry is null and must be allocated.
    NeedBlock { block: usize },
    /// Address of the slot: block pointer plus `(idx & 3) * 0x2c0`.
    Slot { block_ptr: u32, offset: u32 },
}

/// `0x00834d80`: deque push-back slot lookup (4 elements per block,
/// element stride `0x2c0`).  `map_size`, `offset`, `count` are the fields
/// at +4, +8, +0xc; `count` is incremented when a slot is returned.
pub fn w25a27_deque_next_slot(
    map: &[u32],
    map_size: u32,
    offset: u32,
    count: &mut u32,
) -> W25a27Slot {
    if (offset.wrapping_add(*count) & 3) == 0 && map_size <= (count.wrapping_add(4) >> 2) {
        return W25a27Slot::NeedGrow;
    }
    let idx = count.wrapping_add(offset);
    *count = count.wrapping_add(1);
    let block = (map_size.wrapping_sub(1) & (idx >> 2)) as usize;
    match map.get(block).copied().unwrap_or(0) {
        0 => W25a27Slot::NeedBlock { block },
        p => W25a27Slot::Slot {
            block_ptr: p,
            offset: (idx & 3) * 0x2c0,
        },
    }
}

/// `0x00833cf0`: stores the byte argument at object offset `0x1b4`.
pub fn w25a27_set_flag_1b4(obj: &mut [u8], value: u8) {
    obj[0x1b4] = value;
}

/// `0x00835830`: scalar deleting destructor of the shared-ptr manager;
/// resets the vtable and returns true when bit 0 of the flag asks for delete.
pub fn w25a27_manager_dtor(flags: u8) -> bool {
    flags & 1 != 0
}

#[cfg(test)]
mod tests_wave25_agent27 {
    use super::*;

    #[test]
    fn deque_slot() {
        let map = [0x1000u32, 0, 0, 0, 0, 0, 0, 0];
        let mut c = 1;
        assert_eq!(
            w25a27_deque_next_slot(&map, 8, 0, &mut c),
            W25a27Slot::Slot {
                block_ptr: 0x1000,
                offset: 0x2c0
            }
        );
        assert_eq!(c, 2);
        let mut c = 0;
        assert_eq!(
            w25a27_deque_next_slot(&map, 0, 0, &mut c),
            W25a27Slot::NeedGrow
        );
        let mut c = 4;
        assert_eq!(
            w25a27_deque_next_slot(&map, 8, 0, &mut c),
            W25a27Slot::NeedBlock { block: 1 }
        );
    }

    #[test]
    fn small() {
        let mut o = vec![0u8; 0x200];
        w25a27_set_flag_1b4(&mut o, 5);
        assert_eq!(o[0x1b4], 5);
        assert!(w25a27_manager_dtor(1));
        assert!(!w25a27_manager_dtor(0));
    }
}
