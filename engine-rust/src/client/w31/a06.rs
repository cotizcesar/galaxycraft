#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent06.rs
// ===========================================================================
// Wave 31 agent 06: client-side UI/redraw helpers.

/// Handler kind chosen by `0x006fda20`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W31a06Handler {
    /// Handler at data address 0x006c_bef0.
    Primary,
    /// Handler at function address 0x006c_cbb0.
    Secondary,
}

/// `0x006fda20`: maps mode 1/2/3 to (index into the three-word callback
/// table at 0x0117_a2d8, handler).  Other modes select nothing.
pub fn w31a06_select_handler(mode: i32) -> Option<(usize, W31a06Handler)> {
    match mode {
        1 => Some((0, W31a06Handler::Primary)),
        2 => Some((1, W31a06Handler::Primary)),
        3 => Some((2, W31a06Handler::Secondary)),
        _ => None,
    }
}

/// `0x007012c0`: after the common prelude, dispatches on the mode word:
/// returns 1 or 2 for the two sub-handlers, `None` otherwise.
pub fn w31a06_dispatch_mode(mode: i32) -> Option<u8> {
    match mode {
        1 => Some(1),
        2 => Some(2),
        _ => None,
    }
}

/// `0x00715280`: clears the low bits of every framebuffer word with a mask
/// chosen from `flags`.  Returns true when a mask was applied (caller then
/// runs the redraw call).  `force_clear` drops bits 0 and 14 first.
pub fn w31a06_mask_clear(
    flags: &mut u32,
    force_clear: bool,
    buf: &mut [u32],
    count: usize,
) -> bool {
    if force_clear {
        *flags &= 0xffff_bffe;
    }
    let mask = if *flags & 1 != 0 {
        0xffff_0000u32
    } else if *flags & 0x4000 != 0 {
        0xffff_ff00u32
    } else {
        return false;
    };
    for w in buf.iter_mut().take(count) {
        *w &= mask;
    }
    true
}

/// `0x007155e0`: when neither bit 0 nor bit 14 of `flags` is set, ORs 0xff
/// into every word.  `state` gets bit 1 set when bit 0 is clear.  Returns
/// true when the fill ran.
pub fn w31a06_fill_low(
    flags: &mut u32,
    force_clear: bool,
    state: &mut u8,
    buf: &mut [u32],
    count: usize,
) -> bool {
    if force_clear {
        *flags &= 0xffff_bffe;
    }
    if *state & 1 == 0 {
        *state |= 2;
    }
    if *flags & 0x4001 != 0 {
        return false;
    }
    for w in buf.iter_mut().take(count) {
        *w |= 0xff;
    }
    true
}

/// One 0x14-byte slot of the marker table at 0x0117_efc8.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct W31a06Slot {
    pub active: u8,
    pub x: u32,
    pub y: u32,
    pub zero: u16,
    pub kind: u8,
    pub extra: u32,
}

/// `0x00719ea0`: claims the first free slot of 16 and fills it with
/// map-pixel coordinates `((v >> 5) << shift) / div - 0x40`.  Returns the
/// slot index, or `None` when full (or `div` is zero).
pub fn w31a06_add_slot(
    slots: &mut [W31a06Slot; 16],
    x: u32,
    y: u32,
    kind: u8,
    extra: u32,
    shift: u8,
    div: u32,
) -> Option<usize> {
    let i = slots.iter().position(|s| s.active == 0)?;
    if div == 0 {
        return None;
    }
    let conv = |v: u32| {
        ((v >> 5) << (shift & 0x1f))
            .wrapping_div(div)
            .wrapping_sub(0x40)
    };
    slots[i] = W31a06Slot {
        active: 1,
        x: conv(x),
        y: conv(y),
        zero: 0,
        kind,
        extra,
    };
    Some(i)
}

/// `0x0071ae50`: writes palette entry `(a * 2 | b)` into the pixel at
/// `(x, y)` of a surface with `stride` words.
pub fn w31a06_put_pixel(
    dst: &mut [u32],
    palette: &[u32],
    stride: usize,
    x: usize,
    y: usize,
    a: usize,
    b: usize,
) {
    let v = palette[(a * 2) | b];
    dst[stride * y + x] = v;
}

/// `0x00711b80`: looks a row up with `find`, then calls `emit` with the row
/// words chosen by `mode` (1: words 0,1; 2: words 4,5 or 6,7 depending on
/// `alt`).  Rows have ten words.  Returns 0 when not found or mode unknown.
pub fn w31a06_emit_row(
    rows: &[[u32; 10]],
    find: impl FnOnce() -> Option<usize>,
    mode: i32,
    alt: bool,
    mut emit: impl FnMut(u32, u32, bool) -> u32,
) -> u32 {
    let Some(i) = find() else { return 0 };
    let r = &rows[i];
    match mode {
        1 => emit(r[0], r[1], true),
        2 => {
            let (a, b) = if alt { (r[6], r[7]) } else { (r[4], r[5]) };
            emit(a, b, false)
        }
        _ => 0,
    }
}

/// `0x007129b0`: when `guard` is -1, runs the redraw variant picked by
/// `variant_a` and then the common tail.  Returns which variant ran.
pub fn w31a06_redraw_pick(guard: i32, variant_a: bool) -> Option<char> {
    if guard != -1 {
        return None;
    }
    Some(if variant_a { 'a' } else { 'b' })
}

#[cfg(test)]
mod tests_wave31_agent06 {
    use super::*;

    #[test]
    fn handler_and_dispatch() {
        assert_eq!(w31a06_select_handler(1), Some((0, W31a06Handler::Primary)));
        assert_eq!(
            w31a06_select_handler(3),
            Some((2, W31a06Handler::Secondary))
        );
        assert_eq!(w31a06_select_handler(0), None);
        assert_eq!(w31a06_dispatch_mode(2), Some(2));
        assert_eq!(w31a06_dispatch_mode(3), None);
        assert_eq!(w31a06_redraw_pick(-1, true), Some('a'));
        assert_eq!(w31a06_redraw_pick(-1, false), Some('b'));
        assert_eq!(w31a06_redraw_pick(0, true), None);
    }

    #[test]
    fn masks() {
        let mut f = 1u32;
        let mut b = [0xaabb_ccddu32; 3];
        assert!(w31a06_mask_clear(&mut f, false, &mut b, 2));
        assert_eq!(b, [0xaabb_0000, 0xaabb_0000, 0xaabb_ccdd]);
        let mut f = 0x4000u32;
        let mut b = [0x1234_5678u32];
        assert!(w31a06_mask_clear(&mut f, false, &mut b, 1));
        assert_eq!(b[0], 0x1234_5600);
        let mut f = 0x4001u32;
        assert!(!w31a06_mask_clear(&mut f, true, &mut b, 1));
        assert_eq!(f, 0);

        let mut f = 0u32;
        let mut st = 0u8;
        let mut b = [0x100u32; 2];
        assert!(w31a06_fill_low(&mut f, false, &mut st, &mut b, 2));
        assert_eq!(st, 2);
        assert_eq!(b, [0x1ff, 0x1ff]);
        let mut f = 1u32;
        assert!(!w31a06_fill_low(&mut f, false, &mut st, &mut b, 2));
    }

    #[test]
    fn slots() {
        let mut s = [W31a06Slot::default(); 16];
        assert_eq!(w31a06_add_slot(&mut s, 0x100, 0x200, 7, 9, 0, 1), Some(0));
        assert_eq!(s[0].x, 8u32.wrapping_sub(0x40));
        assert_eq!(s[0].y, 16u32.wrapping_sub(0x40));
        assert_eq!(s[0].kind, 7);
        assert_eq!(w31a06_add_slot(&mut s, 0x1000, 0, 1, 2, 1, 2), Some(1));
        assert_eq!(s[1].x, 0x80 - 0x40);
        for x in s.iter_mut() {
            x.active = 1;
        }
        assert_eq!(w31a06_add_slot(&mut s, 0, 0, 0, 0, 0, 1), None);
    }

    #[test]
    fn pixel_and_rows() {
        let mut d = [0u32; 6];
        let pal = [10, 11, 12, 13];
        w31a06_put_pixel(&mut d, &pal, 3, 2, 1, 1, 1);
        assert_eq!(d[5], 13);
        let rows = [[0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9]];
        let e = |a, b, f: bool| a * 100 + b + f as u32;
        assert_eq!(w31a06_emit_row(&rows, || Some(0), 1, false, e), 2);
        assert_eq!(w31a06_emit_row(&rows, || Some(0), 2, false, e), 405);
        assert_eq!(w31a06_emit_row(&rows, || Some(0), 2, true, e), 607);
        assert_eq!(w31a06_emit_row(&rows, || None, 1, false, e), 0);
        assert_eq!(w31a06_emit_row(&rows, || Some(0), 3, false, e), 0);
    }
}
