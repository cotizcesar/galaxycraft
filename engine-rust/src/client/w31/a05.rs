#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent05.rs
// ===========================================================================
// Wave 31 translation batch 05: small client-side predicates and helpers.

/// Cell record of the 0x18-byte grid at `+0xe8` (`0x006c2b90`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct W31a05Cell {
    pub flags: u16,
    pub cost: f32,
    pub counter_a: u16,
    pub counter_b: u16,
}

/// `0x006c2b90`: refreshes bit 0 of every cell: set only when bit 1 is set,
/// bit 2 is clear and `cost >= threshold`; clears both counters.
pub fn w31a05_refresh_cells(cells: &mut [W31a05Cell], threshold: f32) {
    for c in cells.iter_mut() {
        let b = c.flags as u8;
        if (b >> 2) & 1 != 0 || (b >> 1) & 1 == 0 || c.cost < threshold {
            c.flags &= 0xfffe;
        } else {
            c.flags |= 1;
        }
        c.counter_a = 0;
        c.counter_b = 0;
    }
}

/// Action selected by the key handlers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W31a05Action {
    None,
    Cancel,
    A,
    B,
}

/// `0x006cc570`: dispatch on the event code (`0x2d` -> A, `0x2e` -> B);
/// the global flag forces Cancel.
pub fn w31a05_key_dispatch(flag: bool, code: i16) -> W31a05Action {
    if flag {
        return W31a05Action::Cancel;
    }
    match code {
        0x2d => W31a05Action::A,
        0x2e => W31a05Action::B,
        _ => W31a05Action::None,
    }
}

/// `0x006cc5b0`: true (Cancel is invoked) when the code is `0x2d` or `0x2e`.
pub fn w31a05_key_cancel(code: i16) -> bool {
    code == 0x2e || code == 0x2d
}

/// Scroll request of `0x006cc710`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum W31a05Scroll {
    None,
    Cancel,
    Immediate(f32),
    Step(f32, i32),
}

/// `0x006cc710`: code `0x11` / `0x12` scroll forward / backward; `held`
/// is the result of the `0x11` key query.
pub fn w31a05_scroll_dispatch(flag: bool, code: i16, held: bool) -> W31a05Scroll {
    if flag {
        return W31a05Scroll::Cancel;
    }
    match code {
        0x11 if held => W31a05Scroll::Immediate(1.0),
        0x11 => W31a05Scroll::Step(f32::from_bits(0x3dcc_cccd), 2),
        0x12 if held => W31a05Scroll::Immediate(-1.0),
        0x12 => W31a05Scroll::Step(f32::from_bits(0xbdcc_cccd), 2),
        _ => W31a05Scroll::None,
    }
}

/// `0x006cdc70`: stores `v` at `obj[0x60]`; `pre`/`post` model the calls
/// before/after, `on_zero` the call made when `v == 0`.
pub fn w31a05_set_flag_wrapped(
    obj: &mut [u8],
    v: u8,
    mut pre: impl FnMut(),
    mut on_zero: impl FnMut(),
    mut post: impl FnMut(),
) {
    pre();
    if v == 0 {
        on_zero();
    }
    obj[0x60] = v;
    post();
}

/// `0x006cddf0`: as above without the surrounding calls.
pub fn w31a05_set_flag(obj: &mut [u8], v: u8, mut on_zero: impl FnMut()) {
    if v == 0 {
        on_zero();
    }
    obj[0x60] = v;
}

/// `0x006e1a00`: returns 0 when the query holds and `rec[4] == 0`, else 1.
pub fn w31a05_gate(query: bool, rec: &[u8]) -> u32 {
    if query && rec[4] == 0 { 0 } else { 1 }
}

/// `0x006e5460`: true when mode byte is 1, byte2 is 0, counter < 1 and
/// the external check is false.
pub fn w31a05_idle_ready(mode: u8, flag: u8, counter: i32, ext: impl FnOnce() -> bool) -> bool {
    mode == 1 && flag == 0 && counter < 1 && !ext()
}

/// `0x006ec3c0`: scans the ten (key, value) pairs from the last down;
/// returns the mapped value (if any) and whether the fallback call with the
/// original value is made (when the `2` query is false).
pub fn w31a05_table_lookup(table: &[(u8, u8); 10], key: u32, query2: bool) -> (Option<u8>, bool) {
    let mapped = table.iter().rev().find(|e| e.0 as u32 == key).map(|e| e.1);
    (mapped, !query2)
}

#[cfg(test)]
mod tests_wave31_agent05 {
    use super::*;

    #[test]
    fn cells() {
        let mut v = vec![
            W31a05Cell {
                flags: 2,
                cost: 5.0,
                counter_a: 3,
                counter_b: 4,
            },
            W31a05Cell {
                flags: 6,
                cost: 5.0,
                counter_a: 1,
                counter_b: 1,
            },
            W31a05Cell {
                flags: 3,
                cost: 1.0,
                counter_a: 1,
                counter_b: 1,
            },
        ];
        w31a05_refresh_cells(&mut v, 2.0);
        assert_eq!(v[0].flags, 3);
        assert_eq!(v[1].flags, 6);
        assert_eq!(v[2].flags, 2);
        assert!(v.iter().all(|c| c.counter_a == 0 && c.counter_b == 0));
    }

    #[test]
    fn keys() {
        assert_eq!(w31a05_key_dispatch(true, 0), W31a05Action::Cancel);
        assert_eq!(w31a05_key_dispatch(false, 0x2d), W31a05Action::A);
        assert_eq!(w31a05_key_dispatch(false, 0x2e), W31a05Action::B);
        assert_eq!(w31a05_key_dispatch(false, 1), W31a05Action::None);
        assert!(w31a05_key_cancel(0x2d) && !w31a05_key_cancel(0));
        assert_eq!(
            w31a05_scroll_dispatch(false, 0x12, true),
            W31a05Scroll::Immediate(-1.0)
        );
        assert_eq!(
            w31a05_scroll_dispatch(false, 0x11, false),
            W31a05Scroll::Step(f32::from_bits(0x3dcc_cccd), 2)
        );
        assert_eq!(
            w31a05_scroll_dispatch(true, 0x11, false),
            W31a05Scroll::Cancel
        );
        assert_eq!(w31a05_scroll_dispatch(false, 3, false), W31a05Scroll::None);
    }

    #[test]
    fn flags_and_gates() {
        let mut o = [9u8; 0x80];
        let mut n = 0;
        w31a05_set_flag(&mut o, 0, || n += 1);
        assert_eq!((o[0x60], n), (0, 1));
        w31a05_set_flag_wrapped(&mut o, 1, || {}, || n += 10, || {});
        assert_eq!((o[0x60], n), (1, 1));
        assert_eq!(w31a05_gate(true, &[0; 5]), 0);
        assert_eq!(w31a05_gate(false, &[0; 5]), 1);
        assert!(w31a05_idle_ready(1, 0, 0, || false));
        assert!(!w31a05_idle_ready(1, 0, 1, || false));
        let t = [
            (1, 11),
            (2, 12),
            (3, 13),
            (4, 14),
            (5, 15),
            (6, 16),
            (7, 17),
            (8, 18),
            (9, 19),
            (3, 99),
        ];
        assert_eq!(w31a05_table_lookup(&t, 3, false), (Some(99), true));
        assert_eq!(w31a05_table_lookup(&t, 50, true), (None, false));
    }
}
