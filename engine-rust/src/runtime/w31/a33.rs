#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent33.rs
// ===========================================================================
// Wave 31 agent 33 reference helpers.

/// `0x0071a480`: zero-initialises a three-dword record.
pub fn w31a33_zero_triple(rec: &mut [u32; 3]) {
    *rec = [0; 3];
}

/// `0x0073b830`: true when the queried value is strictly above `0x78`.
pub fn w31a33_above_threshold(value: u32) -> bool {
    0x78 < value
}

/// `0x0073bb80`: fills the eight-entry table with 4 and clears the slot at
/// `own_index` (global at the original address `DAT_00f2acfc`).  Returns
/// false-less `true` like the original (always 1); `None` index is
/// rejected when out of range.
pub fn w31a33_reset_slot_table(table: &mut [u32], own_index: usize) -> Option<bool> {
    if table.len() < 8 || own_index >= table.len() {
        return None;
    }
    for v in table.iter_mut().take(8) {
        *v = 4;
    }
    table[own_index] = 0;
    Some(true)
}

/// `0x0072c840`: builds the five-byte message `[0x5d, value LE]`.
pub fn w31a33_build_5d_message(value: u32) -> [u8; 5] {
    let b = value.to_le_bytes();
    [0x5d, b[0], b[1], b[2], b[3]]
}

/// `0x007364f0`: state machine for the one-byte `0x54` message.  `state`
/// is the global byte (`DAT_011a12f0`).  `flush(3)` is invoked when
/// `param != 0`; `send` transmits one byte.
pub fn w31a33_state_54(
    param: u32,
    state: &mut u8,
    mut flush: impl FnMut(u32),
    mut send: impl FnMut(&[u8]),
) {
    if param != 0 {
        flush(3);
    }
    if *state == 1 {
        if param != 0 {
            *state = 0;
            return;
        }
        send(&[0x54]);
        *state = 2;
    }
}

type W31Handler<'a, T> = dyn FnMut(T, &[u8]) -> usize + 'a;

/// `0x00739e60`: netmgr message pump.  `next_chunk` yields the next
/// `(token, buffer)`; each message's first byte (< 3) selects a handler
/// returning the bytes consumed (0 stops the chunk).
pub fn w31a33_process_messages<T: Copy>(
    mut next_chunk: impl FnMut() -> Option<(T, Vec<u8>)>,
    handlers: &mut [&mut W31Handler<'_, T>; 3],
) {
    while let Some((tok, buf)) = next_chunk() {
        let mut off = 0usize;
        while off < buf.len() && buf[off] < 3 {
            let used = (handlers[buf[off] as usize])(tok, &buf[off..]);
            if used == 0 {
                break;
            }
            off += used;
        }
    }
}

/// Outcome of `0x00715e80`.
#[derive(Debug, PartialEq, Eq)]
pub enum W31a33Outcome {
    Handled,
    Default,
}

/// `0x00715e80`: key-event hook on a widget.  `flags` is the dword at
/// offset 0x30, `kind` the event's first dword, `code` its field at 0x10.
/// Code 0xe/kind 0 installs the handler (`flags |= 0x20000000`, stores
/// `cached`); code 0xe/kind 2 records the widget id and forwards the event.
pub fn w31a33_widget_event(
    flags: &mut u32,
    installed: &mut Option<u16>,
    widget_id: i16,
    cached: u16,
    kind: i32,
    code: i16,
    last_id: &mut i32,
    mut forward: impl FnMut(),
) -> W31a33Outcome {
    if code == 0xe && kind == 0 {
        *flags |= 0x2000_0000;
        *installed = Some(cached);
    }
    if code == 0xe && kind == 2 {
        *last_id = widget_id as i32;
        forward();
        return W31a33Outcome::Handled;
    }
    W31a33Outcome::Default
}

#[cfg(test)]
mod tests_wave31_agent33 {
    use super::*;

    #[test]
    fn small_helpers() {
        let mut r = [1, 2, 3];
        w31a33_zero_triple(&mut r);
        assert_eq!(r, [0; 3]);
        assert!(!w31a33_above_threshold(0x78));
        assert!(w31a33_above_threshold(0x79));
        assert_eq!(w31a33_build_5d_message(0x01020304), [0x5d, 4, 3, 2, 1]);
    }

    #[test]
    fn slot_table() {
        let mut t = [9u32; 8];
        assert_eq!(w31a33_reset_slot_table(&mut t, 3), Some(true));
        assert_eq!(t, [4, 4, 4, 0, 4, 4, 4, 4]);
        assert_eq!(w31a33_reset_slot_table(&mut t, 8), None);
    }

    #[test]
    fn state_machine() {
        let mut st = 1u8;
        let mut sent = vec![];
        let mut fl = vec![];
        w31a33_state_54(0, &mut st, |v| fl.push(v), |b| sent.extend_from_slice(b));
        assert_eq!((st, sent.clone(), fl.len()), (2, vec![0x54], 0));
        let mut st = 1u8;
        w31a33_state_54(1, &mut st, |v| fl.push(v), |_| panic!());
        assert_eq!((st, fl), (0, vec![3]));
    }

    #[test]
    fn pump() {
        let mut chunks = vec![(7u8, vec![0u8, 9, 1, 5, 9])];
        let mut log = vec![];
        let mut h0 = |t: u8, b: &[u8]| {
            log.push((0, t, b.len()));
            2usize
        };
        let mut h1 = |_t: u8, _b: &[u8]| 0usize;
        let mut h2 = |_t: u8, _b: &[u8]| 1usize;
        w31a33_process_messages(|| chunks.pop(), &mut [&mut h0, &mut h1, &mut h2]);
        assert_eq!(log, vec![(0, 7, 5)]);
    }

    #[test]
    fn widget() {
        let (mut f, mut inst, mut last) = (0u32, None, 0i32);
        let mut n = 0;
        let r = w31a33_widget_event(&mut f, &mut inst, 5, 3, 0, 0xe, &mut last, || n += 1);
        assert_eq!(
            (r, f, inst, n),
            (W31a33Outcome::Default, 0x2000_0000, Some(3), 0)
        );
        let r = w31a33_widget_event(&mut f, &mut inst, 5, 3, 2, 0xe, &mut last, || n += 1);
        assert_eq!((r, last, n), (W31a33Outcome::Handled, 5, 1));
    }
}
