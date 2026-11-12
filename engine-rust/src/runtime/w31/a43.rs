#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent43.rs
// ===========================================================================
// Wave 31 agent 43: small control/event helpers.

/// `0x0087b4a0`: forwards `flag ^ 1` (low byte) to the callee together with
/// the field at offset 4 of the object.
pub fn w31a43_toggle_arg(field4: u32, flag: u8, call: &mut dyn FnMut(u32, u8)) {
    call(field4, flag ^ 1);
}

/// `0x0084d540`: runs two helpers in order (setup then follow-up).
pub fn w31a43_run_two(first: &mut dyn FnMut(), second: &mut dyn FnMut()) {
    first();
    second();
}

/// `0x0084f1a0`: if the predicate accepts the pair, dispatch through the
/// vtable slot 0x24 of the inner object; otherwise run the fallback.
pub fn w31a43_dispatch_or_fallback(
    a: u32,
    b: u32,
    accepts: &dyn Fn(u32, u32) -> bool,
    slot_0x24: &mut dyn FnMut(u32, u32),
    fallback: &mut dyn FnMut(u32),
) {
    if accepts(a, b) {
        slot_0x24(a, b);
    } else {
        fallback(a);
    }
}

/// `0x0087b860`: when the object (field at 0x48) is active, computes the
/// translated position `(pos + origin)` from the helper result and the
/// origin at 0x40/0x44, returning it for the caller to forward.
pub fn w31a43_offset_position(
    active_0x48: i32,
    origin_0x40: i32,
    origin_0x44: i32,
    pos: (i32, i32),
) -> Option<(i32, i32)> {
    if active_0x48 == 0 {
        return None;
    }
    Some((
        pos.0.wrapping_add(origin_0x40),
        pos.1.wrapping_add(origin_0x44),
    ))
}

/// `0x0087b970`: ControlBase constructor field initialisation (dword
/// indices of the object).  Returns the first 11 dwords; the vtable
/// pointer at index 0 is left as 0 (caller-supplied), and index 0xb/0xc
/// are written from helper results.
pub fn w31a43_control_base_init(handle: u32) -> [u32; 13] {
    let mut d = [0u32; 13];
    d[2] = 8;
    d[5] = 0x3f80_0000;
    d[7] = 8;
    d[10] = 0x3f80_0000;
    d[11] = handle;
    d
}

/// `0x0087bc20`: looks up an object by key, creating it when missing,
/// then calls vtable slot 0x74 with four arguments.
pub fn w31a43_lookup_or_create_call<T>(
    key: u32,
    lookup: &dyn Fn(u32) -> Option<T>,
    create: &mut dyn FnMut(u32) -> T,
    call: &mut dyn FnMut(&T, [u32; 4]),
    args: [u32; 4],
) {
    let obj = match lookup(key) {
        Some(o) => o,
        None => create(key),
    };
    call(&obj, args);
}

/// Result of an event dispatch: vtable slot offset and arguments.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W31a43Dispatch {
    pub slot: u32,
    pub args: [i32; 3],
    pub argc: u8,
}

/// `0x0087c220`: maps an event record (type at byte 0x10) to a vtable
/// slot and arguments.  Types 0xe, 0x10 and unknown ones yield `None`
/// (the function returns 0).
pub fn w31a43_event_dispatch(ev: &[u8; 0x18]) -> Option<W31a43Dispatch> {
    let u16at = |o: usize| u16::from_le_bytes([ev[o], ev[o + 1]]) as i32;
    let i16at = |o: usize| i16::from_le_bytes([ev[o], ev[o + 1]]) as i32;
    let u32at = |o: usize| u32::from_le_bytes([ev[o], ev[o + 1], ev[o + 2], ev[o + 3]]) as i32;
    let ty = u16at(0x10);
    let d = |slot, args: [i32; 3], argc| Some(W31a43Dispatch { slot, args, argc });
    let (x, y) = (i16at(0x12), i16at(0x14));
    match ty {
        0 => d(0x34, [u16at(8), u16at(10), 0], 2),
        1 => d(0x38, [u16at(8), u16at(10), 0], 2),
        2 => d(0x3c, [u16at(8), u16at(10), 0], 2),
        3 => d(0x40, [x, y, 0], 2),
        4 => d(0x44, [x, y, 1], 3),
        5 => d(0x48, [x, y, 1], 3),
        6 => d(0x4c, [x, y, 1], 3),
        7 => d(0x44, [x, y, 2], 3),
        8 => d(0x48, [x, y, 2], 3),
        9 => d(0x4c, [x, y, 2], 3),
        10 => d(0x44, [x, y, 3], 3),
        11 => d(0x48, [x, y, 3], 3),
        12 => d(0x4c, [x, y, 3], 3),
        13 => d(0x28, [0; 3], 0),
        15 => d(0x2c, [u16at(8), u16at(10), 0], 2),
        17 => d(0x50, [x, y, 1], 3),
        18 => d(0x50, [x, y, -1], 3),
        19 => d(0x54, [u32at(0), 0, 0], 1),
        20 => d(0x58, [0; 3], 0),
        21 => d(0x5c, [0; 3], 0),
        22 => d(0x60, [0; 3], 0),
        23 => d(0x64, [0; 3], 0),
        24 => d(0x30, [u32at(8), u16at(12), 0], 2),
        _ => None,
    }
}

#[cfg(test)]
mod tests_wave31_agent43 {
    use super::*;

    #[test]
    fn toggle_and_two() {
        let mut got = (0, 0);
        w31a43_toggle_arg(7, 1, &mut |a, b| got = (a, b));
        assert_eq!(got, (7, 0));
        w31a43_toggle_arg(7, 0, &mut |a, b| got = (a, b));
        assert_eq!(got.1, 1);
        let mut log = vec![];
        let l = std::cell::RefCell::new(&mut log);
        w31a43_run_two(&mut || l.borrow_mut().push(1), &mut || {
            l.borrow_mut().push(2)
        });
        assert_eq!(log, vec![1, 2]);
    }

    #[test]
    fn dispatch_fallback() {
        let r = std::cell::Cell::new(0);
        w31a43_dispatch_or_fallback(1, 2, &|_, _| true, &mut |a, b| r.set(a + b), &mut |_| {
            r.set(99)
        });
        assert_eq!(r.get(), 3);
        w31a43_dispatch_or_fallback(1, 2, &|_, _| false, &mut |_, _| r.set(0), &mut |a| {
            r.set(a + 100)
        });
        assert_eq!(r.get(), 101);
    }

    #[test]
    fn offset_and_init() {
        assert_eq!(w31a43_offset_position(0, 1, 2, (3, 4)), None);
        assert_eq!(w31a43_offset_position(1, 1, 2, (3, 4)), Some((4, 6)));
        let d = w31a43_control_base_init(5);
        assert_eq!(
            (d[2], d[5], d[7], d[10], d[11]),
            (8, 0x3f80_0000, 8, 0x3f80_0000, 5)
        );
    }

    #[test]
    fn lookup_create() {
        let mut seen = 0;
        w31a43_lookup_or_create_call(
            3,
            &|_| None,
            &mut |k| k * 2,
            &mut |o, a| seen = *o + a[0],
            [1, 0, 0, 0],
        );
        assert_eq!(seen, 7);
        w31a43_lookup_or_create_call(
            3,
            &|_| Some(10u32),
            &mut |_| 0,
            &mut |o, a| seen = *o + a[0],
            [1, 0, 0, 0],
        );
        assert_eq!(seen, 11);
    }

    #[test]
    fn event_dispatch() {
        let mut ev = [0u8; 0x18];
        ev[0x10] = 18;
        ev[0x12..0x14].copy_from_slice(&(-3i16).to_le_bytes());
        ev[0x14..0x16].copy_from_slice(&5i16.to_le_bytes());
        let d = w31a43_event_dispatch(&ev).unwrap();
        assert_eq!((d.slot, d.args, d.argc), (0x50, [-3, 5, -1], 3));
        ev[0x10] = 14;
        assert_eq!(w31a43_event_dispatch(&ev), None);
        ev[0x10] = 24;
        ev[8..12].copy_from_slice(&9u32.to_le_bytes());
        ev[12] = 4;
        assert_eq!(w31a43_event_dispatch(&ev).unwrap().args, [9, 4, 0]);
    }
}
