#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent20.rs
// ===========================================================================
// Wave 23 agent 20: lobby command-packet builders and small handlers.

/// Lazily allocated packet slot: the retail code allocates `len` bytes on first use.
fn w23a20_slot(slot: &mut Option<Vec<u8>>, len: usize) -> &mut Vec<u8> {
    slot.get_or_insert_with(|| vec![0u8; len])
}

/// `0x00722750`: stores the lobby state; state 2 also clears the companion word.
pub fn w23a20_set_state(state: &mut u8, companion: &mut u32, value: u8) {
    *state = value;
    if value == 2 {
        *companion = 0;
    }
}

/// `0x00722950`: validity check, true when `value <= 7` (otherwise retail aborts).
pub fn w23a20_valid_slot(value: u32) -> bool {
    value <= 7
}

/// `0x00722b00`: when not suppressed and not host, returns the byte at `+1` and
/// the 8 bytes at `+2` of the packet.
pub fn w23a20_parse_info(packet: &[u8], flag: i32, is_host: bool) -> Option<(u8, [u8; 8])> {
    if flag != 0 || is_host || packet.len() < 10 {
        return None;
    }
    let mut b = [0u8; 8];
    b.copy_from_slice(&packet[2..10]);
    Some((packet[1], b))
}

/// `0x00722d30`: gate predicate; true when retail proceeds to the follow-up call.
pub fn w23a20_gate(
    is_host: bool,
    state: u8,
    local_ok: bool,
    packet: &[u8],
    blocked: impl Fn(u8) -> bool,
) -> bool {
    if !is_host || state <= 3 || !local_ok {
        return false;
    }
    match packet.get(1) {
        Some(&b) if b != 0 && b < 5 => !blocked(b),
        _ => false,
    }
}

/// Result of `0x00723df0`: the arguments for the follow-up call.
#[derive(Debug, PartialEq, Eq)]
pub struct W23a20Update {
    pub slot: u8,
    pub flags: u16,
    pub a: u16,
    pub b: u16,
}

/// `0x00723df0`: slot update. `lookup` returns the current flag word for a slot.
/// `Err(())` models the retail abort when the slot byte exceeds 7.
pub fn w23a20_slot_update(
    state: u8,
    suppressed: i32,
    is_host: bool,
    packet: &[u8],
    lookup: impl Fn(u8) -> Option<u32>,
) -> Result<Option<W23a20Update>, ()> {
    if state <= 3 || suppressed != 0 || packet.len() < 8 {
        return Ok(None);
    }
    let slot = packet[1];
    if slot > 7 {
        return Err(());
    }
    if is_host {
        return Ok(None);
    }
    let mut flags = u16::from_le_bytes([packet[2], packet[3]]) & 0xff03;
    if let Some(cur) = lookup(slot) {
        flags |= cur as u16;
    }
    Ok(Some(W23a20Update {
        slot,
        flags,
        a: u16::from_le_bytes([packet[4], packet[5]]),
        b: u16::from_le_bytes([packet[6], packet[7]]),
    }))
}

/// `0x00724fd0`: builds the 3-byte packet `[0x67, a, b]`.
pub fn w23a20_pkt_67(slot: &mut Option<Vec<u8>>, a: u8, b: u8) {
    let p = w23a20_slot(slot, 3);
    p[0] = 0x67;
    p[1] = a;
    p[2] = b;
}

/// `0x00725010`: builds `[0x68, a, b]`; returns the offset `a` plus 66442 to store
/// `b` at when `is_host` (the caller applies it and refreshes).
pub fn w23a20_pkt_68(
    slot: &mut Option<Vec<u8>>,
    a: u8,
    b: u8,
    is_host: bool,
) -> Option<(usize, u8)> {
    let p = w23a20_slot(slot, 3);
    p[0] = 0x68;
    p[1] = a;
    p[2] = b;
    is_host.then(|| (a as usize + 66442, b))
}

/// `0x00725080`: builds the 2-byte packet `[0x43, a]`.
pub fn w23a20_pkt_43(slot: &mut Option<Vec<u8>>, a: u8) {
    let p = w23a20_slot(slot, 2);
    p[0] = 0x43;
    p[1] = a;
}

/// `0x00725d30`: builds the 1-byte packet `[0x39]`.
pub fn w23a20_pkt_39(slot: &mut Option<Vec<u8>>) {
    let p = w23a20_slot(slot, 1);
    p[0] = 0x39;
}

/// `0x00726090`: builds `[0x41, a, b]`; `a` equal to `local_id` becomes 8.
pub fn w23a20_pkt_41(slot: &mut Option<Vec<u8>>, a: u8, b: u8, local_id: u8) {
    let a = if a == local_id { 8 } else { a };
    let p = w23a20_slot(slot, 3);
    p[0] = 0x41;
    p[1] = a;
    p[2] = b;
}

/// `0x007260f0`: builds the 4-byte packet `[0x38, c, a, b]`.
pub fn w23a20_pkt_38(slot: &mut Option<Vec<u8>>, a: u8, b: u8, c: u8) {
    let p = w23a20_slot(slot, 4);
    p[0] = 0x38;
    p[1] = c;
    p[2] = a;
    p[3] = b;
}

/// `0x00726140`: builds `[0x44, a, b]`.
pub fn w23a20_pkt_44(slot: &mut Option<Vec<u8>>, a: u8, b: u8) {
    let p = w23a20_slot(slot, 3);
    p[0] = 0x44;
    p[1] = a;
    p[2] = b;
}

/// `0x00726180`: builds `[0x45, a, b]` (retail then calls a non-returning tail).
pub fn w23a20_pkt_45(slot: &mut Option<Vec<u8>>, a: u8, b: u8) {
    let p = w23a20_slot(slot, 3);
    p[0] = 0x45;
    p[1] = a;
    p[2] = b;
}

#[cfg(test)]
mod tests_wave23_agent20 {
    use super::*;

    #[test]
    fn packets() {
        let mut s = None;
        w23a20_pkt_67(&mut s, 1, 2);
        assert_eq!(s.as_deref(), Some(&[0x67, 1, 2][..]));
        let mut s = None;
        assert_eq!(w23a20_pkt_68(&mut s, 2, 9, true), Some((66444, 9)));
        assert_eq!(w23a20_pkt_68(&mut s, 2, 9, false), None);
        let mut s = None;
        w23a20_pkt_43(&mut s, 5);
        assert_eq!(s.unwrap(), vec![0x43, 5]);
        let mut s = None;
        w23a20_pkt_39(&mut s);
        assert_eq!(s.unwrap(), vec![0x39]);
        let mut s = None;
        w23a20_pkt_41(&mut s, 3, 4, 3);
        assert_eq!(s.unwrap(), vec![0x41, 8, 4]);
        let mut s = None;
        w23a20_pkt_38(&mut s, 1, 2, 3);
        assert_eq!(s.unwrap(), vec![0x38, 3, 1, 2]);
        let mut s = None;
        w23a20_pkt_44(&mut s, 1, 2);
        assert_eq!(s.unwrap(), vec![0x44, 1, 2]);
        let mut s = None;
        w23a20_pkt_45(&mut s, 1, 2);
        assert_eq!(s.unwrap(), vec![0x45, 1, 2]);
    }

    #[test]
    fn state_and_checks() {
        let (mut st, mut c) = (0u8, 7u32);
        w23a20_set_state(&mut st, &mut c, 1);
        assert_eq!(c, 7);
        w23a20_set_state(&mut st, &mut c, 2);
        assert_eq!((st, c), (2, 0));
        assert!(w23a20_valid_slot(7) && !w23a20_valid_slot(8));
        let p = [0, 5, 1, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(w23a20_parse_info(&p, 0, false).unwrap().0, 5);
        assert!(w23a20_parse_info(&p, 1, false).is_none());
        assert!(w23a20_gate(true, 4, true, &[0, 4], |_| false));
        assert!(!w23a20_gate(true, 3, true, &[0, 4], |_| false));
        assert!(!w23a20_gate(true, 4, true, &[0, 5], |_| false));
        assert!(!w23a20_gate(true, 4, true, &[0, 2], |_| true));
    }

    #[test]
    fn slot_update() {
        let p = [0, 2, 0xff, 0xff, 1, 0, 2, 0];
        let r = w23a20_slot_update(4, 0, false, &p, |_| Some(0x0c))
            .unwrap()
            .unwrap();
        assert_eq!(r.flags, 0xff03 | 0x0c);
        assert_eq!((r.a, r.b), (1, 2));
        assert!(w23a20_slot_update(4, 0, false, &[0, 9, 0, 0, 0, 0, 0, 0], |_| None).is_err());
        assert_eq!(w23a20_slot_update(3, 0, false, &p, |_| None), Ok(None));
    }
}
