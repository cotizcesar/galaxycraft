#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent07.rs
// ===========================================================================
// Wave 31 translation batch 07: client-side slot/hotkey/menu-flow helpers.

/// `0x00746f10`: tallies the 12 slot flag words.  Returns
/// `(occupied, without_0x40000, result_flag, notify)` where `result_flag` is
/// `Some(1)` when the halves tie, `Some(2)` when the minority holds (game
/// aborts, no notifications) and `None` otherwise.  `notify` lists the player
/// ids (slot, or slot + 0x78 for slots >= 8) that get the 0x40000006 message.
pub fn w31a07_slot_tally(slots: &[u32; 12]) -> (i32, i32, Option<u8>, Vec<u32>) {
    let mut occ = 0i32;
    let mut cnt = 0i32;
    // The first word of the first block sets both counters at once.
    for (i, &w) in slots.iter().take(8).enumerate() {
        if w & 0x10000 != 0 {
            occ += 1;
            if w & 0x40000 == 0 {
                cnt += 1;
            }
        }
        let _ = i;
    }
    let flag = if occ - cnt == cnt {
        Some(1)
    } else if occ - cnt < cnt {
        return (occ, cnt, Some(2), Vec::new());
    } else {
        None
    };
    let mut notify = Vec::new();
    for (i, &w) in slots.iter().enumerate() {
        if w & 0x10000 != 0 && w & 0x40000 == 0 {
            let id = if i < 8 { i as u32 } else { i as u32 + 0x78 };
            notify.push(id);
        }
    }
    (occ, cnt, flag, notify)
}

/// `0x00748e00`: maps the key byte to a table row (0x80..0x83 fold to
/// 0x0c..0x0f) and, when the lock is clear and the row kind is 2, returns the
/// byte to latch.
pub fn w31a07_hotkey_latch(key: u8, lock: u32, row_kind: impl Fn(usize) -> u8) -> Option<u8> {
    if lock != 0 {
        return None;
    }
    let idx = if (key as u32).wrapping_sub(0x80) < 4 {
        ((key as u32 & 0xffff_ff7f) + 0xc) as usize
    } else {
        key as usize
    };
    (row_kind(idx) == 2).then_some(key)
}

/// `0x007490d0`: reads the latched hotkey byte; 0 when the latch is off or
/// (lock clear and the current mode is outside 0x80..0x83).
pub fn w31a07_latched_key(latch_on: bool, lock: u32, mode: u32, latched: u32) -> u8 {
    let active = latch_on && !(lock == 0 && mode.wrapping_sub(0x80) >= 4);
    if active { latched as u8 } else { 0 }
}

/// `0x0072e710`: builds the two-byte packet `[0x5f, value]` when networking
/// is on and either 5000 ms passed since `last` or `force` is set.
pub fn w31a07_throttled_packet(
    online: bool,
    now: u32,
    last: u32,
    force: bool,
    value: u8,
) -> Option<[u8; 2]> {
    if online && (last <= now.wrapping_sub(5000) || force) {
        Some([0x5f, value])
    } else {
        None
    }
}

/// `0x0072c4d0`: five-byte packet, opcode `0x12` followed by `param` (LE).
pub fn w31a07_packet_12(param: u32) -> [u8; 5] {
    let b = param.to_le_bytes();
    [0x12, b[0], b[1], b[2], b[3]]
}

/// `0x0074cbf0`: run-length table lookup for the selected cell.  `blob` is
/// the record whose header holds x/y offsets (bytes 0x26/0x27), width/height
/// (0x28/0x29) and the data offset (u32 at 0x2a, top bit ignored).  Returns
/// the cell byte (0 when out of range or inside a skip run).
pub fn w31a07_rle_cell(blob: &[u8]) -> u8 {
    let get = |i: usize| blob.get(i).copied();
    let (Some(h26), Some(h27), Some(h28), Some(h29)) = (get(0x26), get(0x27), get(0x28), get(0x29))
    else {
        return 0;
    };
    let mut col = -(h26 as i32);
    let row = -(h27 as i32);
    if !(col > -1 && row > -1 && col < h28 as i32 && row < h29 as i32) {
        return 0;
    }
    let Some(off) = blob
        .get(0x2a..0x2e)
        .map(|b| (u32::from_le_bytes([b[0], b[1], b[2], b[3]]) & 0x7fff_ffff) as usize)
    else {
        return 0;
    };
    let Some(rp) = off.checked_sub(h27 as usize * 2) else {
        return 0;
    };
    let Some(rel) = blob
        .get(rp..rp + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]) as usize)
    else {
        return 0;
    };
    let mut p = off + rel;
    loop {
        let Some(h) = get(p) else { return 0 };
        let run = (h & 0x3f) as i32;
        if col < run {
            if h & 0x80 != 0 {
                return 0;
            }
            return if h & 0x40 == 0 {
                get(p + 1 + col as usize).unwrap_or(0)
            } else {
                get(p + 1).unwrap_or(0)
            };
        }
        col -= run;
        let mut next = p + 1;
        if h & 0x80 == 0 {
            next = if h & 0x40 == 0 {
                next + run as usize
            } else {
                p + 2
            };
        }
        p = next;
        if col < 0 {
            return 0;
        }
    }
}

/// Menu-flow step emitted by the back/escape handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W31a07Step {
    /// Enter the alternate screen routine.
    EnterAlt,
    /// Run the pre-step routine.
    Pre,
    /// Run the screen switch with the given code.
    Switch(u8),
    /// Run the extra online-less routine.
    Extra,
    /// Run the common finish routine.
    Finish,
    /// Run the secondary-screen routine.
    Secondary,
}

/// `0x0074cf00`: back handler.  Returns the steps and the new counter.
pub fn w31a07_back_flow(sel: i32, counter: i8) -> (Vec<W31a07Step>, i8) {
    if sel == -99 {
        return (vec![W31a07Step::EnterAlt], counter);
    }
    if sel == -2 {
        return (
            vec![W31a07Step::Pre, W31a07Step::Switch(2), W31a07Step::Finish],
            counter,
        );
    }
    let c = counter.wrapping_sub(1);
    if c != 0 {
        (vec![W31a07Step::Secondary], counter.wrapping_sub(2))
    } else {
        (vec![W31a07Step::Finish], c)
    }
}

/// `0x0074cf80`: back handler variant.  `online` is the connection flag,
/// `slot_state_is_3` the retail slot-state test.
pub fn w31a07_back_flow2(
    sel: i32,
    counter: i8,
    online: bool,
    slot_state_is_3: bool,
) -> (Vec<W31a07Step>, i8) {
    if sel == -2 {
        if !online {
            return (
                vec![
                    W31a07Step::Pre,
                    W31a07Step::Switch(4),
                    W31a07Step::Extra,
                    W31a07Step::Finish,
                ],
                counter,
            );
        }
        if !slot_state_is_3 {
            return (
                vec![W31a07Step::Pre, W31a07Step::Switch(8), W31a07Step::Finish],
                counter,
            );
        }
        return (
            vec![W31a07Step::Pre, W31a07Step::Switch(7), W31a07Step::Finish],
            counter,
        );
    }
    if sel == 1 {
        return (vec![W31a07Step::Finish], 0);
    }
    let c = counter.wrapping_sub(1);
    if c != 0 {
        (vec![W31a07Step::Secondary], counter.wrapping_sub(2))
    } else {
        (vec![W31a07Step::Finish], c)
    }
}

#[cfg(test)]
mod tests_wave31_agent07 {
    use super::*;

    #[test]
    fn tally() {
        let mut s = [0u32; 12];
        s[0] = 0x10000;
        s[1] = 0x10000 | 0x40000;
        let (o, c, f, n) = w31a07_slot_tally(&s);
        assert_eq!((o, c, f), (2, 1, Some(1)));
        assert_eq!(n, vec![0]);
        s[2] = 0x10000;
        s[3] = 0x10000;
        assert_eq!(w31a07_slot_tally(&s).2, Some(2));
        s = [0u32; 12];
        s[0] = 0x10000;
        s[1] = 0x50000;
        s[8] = 0x10000;
        assert!(w31a07_slot_tally(&s).3.contains(&(8 + 0x78)));
    }

    #[test]
    fn hotkey() {
        assert_eq!(
            w31a07_hotkey_latch(0x81, 0, |i| if i == 0xd { 2 } else { 0 }),
            Some(0x81)
        );
        assert_eq!(w31a07_hotkey_latch(0x81, 1, |_| 2), None);
        assert_eq!(w31a07_latched_key(true, 0, 0x90, 5), 0);
        assert_eq!(w31a07_latched_key(true, 0, 0x81, 5), 5);
        assert_eq!(w31a07_latched_key(false, 1, 0x81, 5), 0);
    }

    #[test]
    fn packets() {
        assert_eq!(w31a07_packet_12(0x01020304), [0x12, 4, 3, 2, 1]);
        assert_eq!(
            w31a07_throttled_packet(true, 6000, 1000, false, 9),
            Some([0x5f, 9])
        );
        assert_eq!(w31a07_throttled_packet(true, 5999, 1000, false, 9), None);
        assert_eq!(
            w31a07_throttled_packet(true, 10, 1000, true, 9),
            Some([0x5f, 9])
        );
        assert_eq!(w31a07_throttled_packet(false, 6000, 0, true, 9), None);
    }

    #[test]
    fn rle() {
        let mut b = vec![0u8; 0x40];
        b[0x28] = 4;
        b[0x29] = 1;
        b[0x2a] = 0x30;
        b[0x30] = 0;
        b[0x31] = 0;
        b[0x32] = 0x03; // literal run of 3
        b[0x33] = 7;
        b[0x34] = 8;
        b[0x35] = 9;
        assert_eq!(w31a07_rle_cell(&b), 7);
        b[0x32] = 0x43; // repeat run
        assert_eq!(w31a07_rle_cell(&b), 7);
        b[0x32] = 0x83; // skip run
        assert_eq!(w31a07_rle_cell(&b), 0);
        b[0x26] = 1;
        assert_eq!(w31a07_rle_cell(&b), 0);
    }

    #[test]
    fn flows() {
        assert_eq!(w31a07_back_flow(-99, 3).0, vec![W31a07Step::EnterAlt]);
        assert_eq!(w31a07_back_flow(0, 3), (vec![W31a07Step::Secondary], 1));
        assert_eq!(w31a07_back_flow(0, 1), (vec![W31a07Step::Finish], 0));
        assert_eq!(
            w31a07_back_flow2(1, 5, true, true),
            (vec![W31a07Step::Finish], 0)
        );
        assert_eq!(
            w31a07_back_flow2(-2, 5, false, true).0[1],
            W31a07Step::Switch(4)
        );
        assert_eq!(
            w31a07_back_flow2(-2, 5, true, true).0[1],
            W31a07Step::Switch(7)
        );
        assert_eq!(
            w31a07_back_flow2(-2, 5, true, false).0[1],
            W31a07Step::Switch(8)
        );
    }
}
