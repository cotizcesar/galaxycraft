#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent11.rs
// ===========================================================================
// Wave 24 translation batch 11: lockstep turn-state resets and the
// fixed-size command-record walker.
//
// Skipped (FUN_ names, no 0x): 00733080 00733aea 00734980 00734c10 00734cc0
// 007354d0 00735540 007355c0 007356b0 00735e50 00735f00 00735fe0 00736496
// 00739ab0 00739d20 007330f0 (noreturn/SEH/STL glue or opaque globals).

/// Turn-state globals cleared by the routines below.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W24a11TurnState {
    pub flag_f2af48: u32,
    pub word_11a1020: u32,
    pub byte_11a1028: u8,
}

/// `0x007331c0`: zeroes the three turn-state globals.
pub fn w24a11_reset_turn_state(s: &mut W24a11TurnState) {
    s.flag_f2af48 = 0;
    s.word_11a1020 = 0;
    s.byte_11a1028 = 0;
}

/// `0x0073a8b0`: sets the "last turn" cursor to -1 and the pending count to 0.
pub fn w24a11_reset_turn_cursor(cursor: &mut i32, pending: &mut i32) {
    *cursor = -1;
    *pending = 0;
}

/// `0x007388d5`: marks `slots[idx]` as 0x10000, bumps the counter and then
/// runs the follow-up call (`FUN_00739470`) supplied by the caller.
pub fn w24a11_mark_slot_full(
    slots: &mut [u32],
    idx: usize,
    counter: &mut u32,
    follow_up: impl FnOnce(),
) -> bool {
    let Some(slot) = slots.get_mut(idx) else {
        return false;
    };
    *slot = 0x10000;
    *counter = counter.wrapping_add(1);
    follow_up();
    true
}

/// Record size for an opcode of the walker at `0x00736570`, `None` if the
/// opcode terminates the walk.  Opcode 5 is skipped (1 byte, no handler).
pub fn w24a11_record_size(op: u8) -> Option<usize> {
    Some(match op {
        0x05 | 0x39 | 0x3c | 0x5b => 1,
        0x3d | 0x42 | 0x43 => 2,
        0x41 | 0x44 | 0x45 | 0x67 | 0x68 => 3,
        0x38 => 4,
        0x3e => 6,
        0x3f => 8,
        0x48 => 0xd,
        0x69 => 0x9a,
        0x40 => 0x16c,
        _ => return None,
    })
}

/// `0x00736570`: walks a buffer of fixed-size command records, calling
/// `handler(opcode, record)` for each (opcode 5 has no handler).  Stops at an
/// unknown opcode, a truncated record, or opcode 0x54 (which never returns in
/// retail).  Returns the number of bytes consumed.
pub fn w24a11_walk_records(buf: &[u8], mut handler: impl FnMut(u8, &[u8])) -> usize {
    let mut pos = 0usize;
    while pos < buf.len() {
        let op = buf[pos];
        let Some(sz) = w24a11_record_size(op) else {
            break;
        };
        if pos + sz > buf.len() {
            break;
        }
        if op != 5 {
            handler(op, &buf[pos..pos + sz]);
        }
        pos += sz;
    }
    pos
}

#[cfg(test)]
mod tests_wave24_agent11 {
    use super::*;

    #[test]
    fn resets() {
        let mut s = W24a11TurnState {
            flag_f2af48: 1,
            word_11a1020: 5,
            byte_11a1028: 9,
        };
        w24a11_reset_turn_state(&mut s);
        assert_eq!(s, W24a11TurnState::default());
        let (mut c, mut p) = (7, 3);
        w24a11_reset_turn_cursor(&mut c, &mut p);
        assert_eq!((c, p), (-1, 0));
    }

    #[test]
    fn mark_slot() {
        let mut slots = [0u32; 3];
        let mut n = 0;
        let mut called = false;
        assert!(w24a11_mark_slot_full(&mut slots, 1, &mut n, || called = true));
        assert_eq!((slots[1], n, called), (0x10000, 1, true));
        assert!(!w24a11_mark_slot_full(&mut slots, 3, &mut n, || {}));
    }

    #[test]
    fn walker() {
        let mut seen = vec![];
        let buf = [5u8, 0x3d, 1, 0x38, 1, 2, 3, 0x99, 0];
        let used = w24a11_walk_records(&buf, |op, r| seen.push((op, r.len())));
        assert_eq!(seen, vec![(0x3d, 2), (0x38, 4)]);
        assert_eq!(used, 7);
        // truncated record
        assert_eq!(w24a11_walk_records(&[0x40, 0, 0], |_, _| {}), 0);
        assert_eq!(w24a11_record_size(0x54), None);
        assert_eq!(w24a11_record_size(0x69), Some(0x9a));
    }
}
