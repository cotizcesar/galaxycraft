#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent08.rs
// ===========================================================================
// Wave 24 translation batch 08.
//
// Pure reference helpers for small command-handler routines.  Unresolved
// callees and opaque globals are caller-supplied closures or parameters.

/// Outcome of the shared map-bounds / unit-type gate used by the action
/// command handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W24a08Gate {
    /// Coordinates outside the map: nothing happens.
    OutOfBounds,
    /// In bounds and the type byte is `'t'` or `'v'`: the order check runs.
    OrderCheck(u8),
    /// In bounds with any other type byte: the target-resolution path runs.
    Resolve(u8),
}

fn w24a08_gate(x: u16, y: u16, map_w: u16, map_h: u16, kind: u8) -> W24a08Gate {
    if x < map_w && y < map_h {
        if kind == b't' || kind == b'v' {
            W24a08Gate::OrderCheck(kind)
        } else {
            W24a08Gate::Resolve(kind)
        }
    } else {
        W24a08Gate::OutOfBounds
    }
}

/// `0x00728270`: packet layout `[op, x:u16, y:u16, target:u16, _, kind]`;
/// x (offset 1) and y (offset 3) are compared against the map dimensions
/// (`DAT_00fe5b5c` / `DAT_00fe5b60`), the kind byte sits at offset 9.
pub fn w24a08_gate_9(pkt: &[u8], map_w: u16, map_h: u16) -> Option<W24a08Gate> {
    if pkt.len() < 10 {
        return None;
    }
    let x = u16::from_le_bytes([pkt[1], pkt[2]]);
    let y = u16::from_le_bytes([pkt[3], pkt[4]]);
    Some(w24a08_gate(x, y, map_w, map_h, pkt[9]))
}

/// `0x00728360`: same gate as the 9-offset variant but the kind byte sits
/// at offset 0xb (target field is 32 bits wide).
pub fn w24a08_gate_b(pkt: &[u8], map_w: u16, map_h: u16) -> Option<W24a08Gate> {
    if pkt.len() < 12 {
        return None;
    }
    let x = u16::from_le_bytes([pkt[1], pkt[2]]);
    let y = u16::from_le_bytes([pkt[3], pkt[4]]);
    Some(w24a08_gate(x, y, map_w, map_h, pkt[0xb]))
}

/// `0x00727ab0`: byte length of a command packet.  `base` is the static
/// length table (`DAT_00d4ea40`, 32-bit entries); ops 9/10/11 are
/// `2 + 2*n` and ops 99/100/101 are `2 + 4*n` with `n = pkt[1]`.
pub fn w24a08_packet_len(pkt: &[u8], base: &[i32]) -> Option<i32> {
    let op = *pkt.first()?;
    let n = || pkt.get(1).copied().map(i32::from);
    match op {
        9..=11 => Some(n()? * 2 + 2),
        99..=101 => Some(n()? * 4 + 2),
        _ => base.get(op as usize).copied(),
    }
}

/// `0x00727b60`: byte at offset 0x4c of the current record, or `0xff` when
/// no record is selected (`DAT_0119f710 == 0`).
pub fn w24a08_current_byte(rec: Option<&[u8]>) -> u32 {
    match rec {
        Some(r) if r.len() > 0x4c => r[0x4c] as u32,
        Some(_) => 0xff,
        None => 0xff,
    }
}

/// `0x00727a20`: scans `count` words; returns `Ok(())` (return value 0) when
/// every word is zero, otherwise `Err(index)` of the first non-zero word
/// (the original then tail-calls a non-returning routine).
pub fn w24a08_all_zero(words: &[i32], count: usize) -> Result<(), usize> {
    for (i, w) in words.iter().take(count).enumerate() {
        if *w != 0 {
            return Err(i);
        }
    }
    Ok(())
}

/// `0x00728d00`: toggle command, `[op, mode, slot]`.  Slots above 0x12 are
/// ignored.  Mode 0 calls `set(slot, 1)`, mode 2 calls `set(slot, 0)`, mode 1
/// calls `special(slot)`; other modes do nothing.  Returns whether the packet
/// was processed (slot in range).
pub fn w24a08_toggle(
    pkt: &[u8],
    mut set: impl FnMut(u8, u32),
    mut special: impl FnMut(u8),
) -> bool {
    if pkt.len() < 3 || pkt[2] > 0x12 {
        return false;
    }
    let slot = pkt[2];
    match pkt[1] {
        0 => set(slot, 1),
        1 => special(slot),
        2 => set(slot, 0),
        _ => {}
    }
    true
}

/// `0x00729bf0`: runs `f` only when the subsystem flag (`DAT_011a4580`) is
/// non-zero; returns whether it ran.
pub fn w24a08_if_enabled(flag: u32, f: impl FnOnce()) -> bool {
    if flag != 0 {
        f();
        true
    } else {
        false
    }
}

/// `0x00729c10`: when the subsystem flag is non-zero, passes the 32-bit
/// value at packet offset 1 to `f`; returns whether it ran.
pub fn w24a08_forward_u32(flag: u32, pkt: &[u8], f: impl FnOnce(u32)) -> bool {
    if flag == 0 || pkt.len() < 5 {
        return false;
    }
    f(u32::from_le_bytes([pkt[1], pkt[2], pkt[3], pkt[4]]));
    true
}

#[cfg(test)]
mod tests_wave24_agent08 {
    use super::*;

    #[test]
    fn gates() {
        let mut p = [0u8; 12];
        p[1] = 5;
        p[3] = 6;
        p[9] = b't';
        assert_eq!(
            w24a08_gate_9(&p, 10, 10),
            Some(W24a08Gate::OrderCheck(b't'))
        );
        p[9] = b'x';
        assert_eq!(w24a08_gate_9(&p, 10, 10), Some(W24a08Gate::Resolve(b'x')));
        assert_eq!(w24a08_gate_9(&p, 5, 10), Some(W24a08Gate::OutOfBounds));
        assert_eq!(w24a08_gate_9(&p[..5], 5, 10), None);
        p[0xb] = b'v';
        assert_eq!(w24a08_gate_b(&p, 10, 7), Some(W24a08Gate::OrderCheck(b'v')));
        assert_eq!(w24a08_gate_b(&p, 10, 6), Some(W24a08Gate::OutOfBounds));
    }

    #[test]
    fn packet_len() {
        let base = [7i32; 256];
        assert_eq!(w24a08_packet_len(&[9, 3], &base), Some(8));
        assert_eq!(w24a08_packet_len(&[0x65, 3], &base), Some(14));
        assert_eq!(w24a08_packet_len(&[12, 3], &base), Some(7));
        assert_eq!(w24a08_packet_len(&[], &base), None);
    }

    #[test]
    fn current_byte_and_zero() {
        let mut r = [0u8; 0x50];
        r[0x4c] = 9;
        assert_eq!(w24a08_current_byte(Some(&r)), 9);
        assert_eq!(w24a08_current_byte(None), 0xff);
        assert_eq!(w24a08_all_zero(&[0, 0, 0], 3), Ok(()));
        assert_eq!(w24a08_all_zero(&[0, 2, 0], 3), Err(1));
        assert_eq!(w24a08_all_zero(&[0, 2, 0], 1), Ok(()));
    }

    #[test]
    fn toggle_and_flags() {
        let mut log = vec![];
        let mut sp = vec![];
        assert!(w24a08_toggle(
            &[0, 0, 3],
            |s, v| log.push((s, v)),
            |s| sp.push(s)
        ));
        assert!(w24a08_toggle(
            &[0, 2, 4],
            |s, v| log.push((s, v)),
            |s| sp.push(s)
        ));
        assert!(w24a08_toggle(
            &[0, 1, 5],
            |s, v| log.push((s, v)),
            |s| sp.push(s)
        ));
        assert!(!w24a08_toggle(
            &[0, 0, 0x13],
            |s, v| log.push((s, v)),
            |s| sp.push(s)
        ));
        assert_eq!(log, vec![(3, 1), (4, 0)]);
        assert_eq!(sp, vec![5]);
        let mut hit = 0;
        assert!(!w24a08_if_enabled(0, || hit += 1));
        assert!(w24a08_if_enabled(1, || hit += 1));
        assert_eq!(hit, 1);
        let mut v = 0;
        assert!(w24a08_forward_u32(1, &[0, 1, 0, 0, 0], |x| v = x));
        assert_eq!(v, 1);
        assert!(!w24a08_forward_u32(0, &[0, 1, 0, 0, 0], |_| {}));
    }
}
