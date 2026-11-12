#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave15/agent10.rs
// ===========================================================================
// Wave 15 translation batch 10 (staging).
//
// Window: queue: 20 addresses, module `sim.rs`.
//
// Skipped (reason in each line, written without the hex prefix):
// - FUN_00677fb0: INI line tokenizer built on three unidentified character-class helpers.
// - FUN_0076bce0, FUN_0076c0b0: intrusive free-list pool over unmodelled globals and unnamed callees.
// - FUN_0078bc40, FUN_0078cb90, FUN_007a8310: UI message / sound dispatch through many unnamed callees.
// - FUN_0078d6f0, FUN_0078df20: compare/store of a packed global via unnamed predicates.
// - FUN_00794400, FUN_007945f0: GPU quad drawing with unnamed renderer calls.
// - FUN_0079f4e0, FUN_0092d5e0: noreturn tail calls / TLS thunk.
// - FUN_007a8030, FUN_007a8a00, FUN_007a9450: player-mask scans over unnamed predicates and noreturn calls.

/// Port of 0x0065b0e0: advance a 4/8-slot iterator.  Returns the new
/// counter and, when the original would reach the (noreturn) dispatcher
/// FUN_00748d40, the slot value passed to it.  `flagged` is
/// `DAT_0119f7bc != 0 && DAT_011a0ed5 != 0`; `slots` is the byte table at
/// offset 0xdc of the decoded base.
pub fn w15a10_slot_iter_next(counter: u32, flagged: bool, slots: &[u8; 4]) -> (u32, Option<u32>) {
    let mut idx = counter.wrapping_add(1);
    if flagged {
        while idx < 4 {
            let b = slots[idx as usize];
            if b < 8 {
                return (idx, Some(b as u32));
            }
            idx = idx.wrapping_add(1);
        }
        (idx, None)
    } else if idx < 8 {
        (idx, Some(idx))
    } else {
        (idx, None)
    }
}

/// Port of 0x0065b0c0: reset the iterator to -1 then advance once.
pub fn w15a10_slot_iter_begin(flagged: bool, slots: &[u8; 4]) -> (u32, Option<u32>) {
    w15a10_slot_iter_next(0xffff_ffff, flagged, slots)
}

/// Port of 0x00776850 (table part): in each of the 8 zero-terminated rows
/// of 12 ids, remove the last occurrence of `id` by shifting the tail down.
pub fn w15a10_remove_id_from_rows(rows: &mut [[u32; 12]; 8], id: u32) {
    for row in rows.iter_mut() {
        let mut count = 12usize;
        let mut last = 12usize;
        for (i, &v) in row.iter().enumerate() {
            if v == 0 {
                count = i;
                break;
            }
            if v == id {
                last = i;
            }
        }
        if id != 0 && count != 0 && last != 12 {
            let n = count - 1;
            if last < n {
                row.copy_within(last + 1..=n, last);
            }
            row[n] = 0;
        }
    }
}

/// Port of 0x0078aff0 (colour selection): control character preceding the
/// "cur/max" readout; 6 below 34 percent, otherwise 3 or 7.
pub fn w15a10_ratio_color_code(cur: i32, max: i32) -> u8 {
    let pct = cur.wrapping_mul(100) / max;
    if (pct as u32) < 0x22 {
        6
    } else {
        ((0x20 < (pct as u32).wrapping_sub(0x22)) as u8) * 4 + 3
    }
}

/// Port of 0x0078aff0 (text part): format `"%c%d/%d"` for the readout.
pub fn w15a10_ratio_text(cur: i32, max: i32) -> String {
    format!(
        "{}{}/{}",
        w15a10_ratio_color_code(cur, max) as char,
        cur,
        max
    )
}

/// Port of 0x00983ae0: classify an opcode (0x83/0x8f -> `arg`, 0x84/0x85/0x90/0x92 -> 1).
pub fn w15a10_opcode_class(op: u16, arg: u32) -> u32 {
    match op {
        0x83 | 0x8f => arg,
        0x84 | 0x85 | 0x90 | 0x92 => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests_wave15_agent10 {
    use super::*;

    #[test]
    fn opcode() {
        assert_eq!(w15a10_opcode_class(0x83, 9), 9);
        assert_eq!(w15a10_opcode_class(0x92, 9), 1);
        assert_eq!(w15a10_opcode_class(0x10, 9), 0);
    }

    #[test]
    fn color() {
        assert_eq!(w15a10_ratio_color_code(10, 100), 6);
        assert_eq!(w15a10_ratio_color_code(50, 100), 3);
        assert_eq!(w15a10_ratio_color_code(90, 100), 7);
        assert_eq!(w15a10_ratio_text(50, 100), "\u{3}50/100");
    }

    #[test]
    fn rows() {
        let mut rows = [[0u32; 12]; 8];
        rows[0][..4].copy_from_slice(&[1, 5, 2, 5]);
        rows[1][..3].copy_from_slice(&[7, 8, 9]);
        rows[2] = [5; 12];
        w15a10_remove_id_from_rows(&mut rows, 5);
        assert_eq!(&rows[0][..4], &[1, 5, 2, 0]);
        assert_eq!(&rows[1][..3], &[7, 8, 9]);
        assert_eq!(rows[2][10], 5);
        assert_eq!(rows[2][11], 0);
        w15a10_remove_id_from_rows(&mut rows, 0);
        assert_eq!(&rows[1][..3], &[7, 8, 9]);
    }

    #[test]
    fn iter() {
        assert_eq!(w15a10_slot_iter_begin(false, &[0; 4]), (0, Some(0)));
        assert_eq!(w15a10_slot_iter_next(7, false, &[0; 4]), (8, None));
        assert_eq!(w15a10_slot_iter_next(0, true, &[9, 9, 3, 1]), (2, Some(3)));
        assert_eq!(w15a10_slot_iter_next(3, true, &[0; 4]), (4, None));
    }
}
