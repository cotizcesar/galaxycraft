#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent19.rs
// ===========================================================================
// Wave 23 agent 19 translations (runtime glue).

/// `0x00721a00`: selects the argument passed to the dispatcher: `0xb` when
/// `flag` is non-zero, otherwise `0`.
pub fn w23a19_dispatch_arg(flag: u8) -> u32 {
    if flag != 0 { 0xb } else { 0 }
}

/// `0x00721d70`: scans slots 12..=15 (stride `0x24`, byte at +8) for the
/// value `0x0c`.  Returns the slot index passed on (index + 0x74 when below
/// 12, which never happens here) or `8` when none matches.
pub fn w23a19_find_slot_arg(slot_bytes: &[u8; 4]) -> u32 {
    let mut i: i32 = 0xc;
    loop {
        let arg = if i < 0xc { i } else { i + 0x74 };
        if slot_bytes[(i - 0xc) as usize] == 0x0c {
            return arg as u32;
        }
        i += 1;
        if i > 0xf {
            return 8;
        }
    }
}

/// Shared tail of `0x00720dc0` / `0x007210b0`: effective flag handed to the
/// presenter. `provider_flag` is byte +4 of the object, `probe` the result
/// of the probe call (only consulted when `provider_flag == 0`),
/// `override_active` is the byte at 0119f5ef, `stored` the byte at 0119f5ee.
pub fn w23a19_effective_flag(
    provider_flag: u8,
    probe: bool,
    override_active: bool,
    stored: u8,
) -> u8 {
    if provider_flag == 0 && !probe && !override_active {
        0
    } else {
        stored
    }
}

/// `0x00720dc0`: stores `new_value` into the global byte when different
/// and returns the effective flag.
pub fn w23a19_set_flag(
    stored: &mut u8,
    new_value: u8,
    provider_flag: u8,
    probe: bool,
    override_active: bool,
) -> u8 {
    if new_value != *stored {
        *stored = new_value;
    }
    w23a19_effective_flag(provider_flag, probe, override_active, *stored)
}

/// Result of `0x007210b0`: rectangle (x, y, w, h), new override flag and
/// effective flag.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W23a19Rect {
    pub rect: [i32; 4],
    pub override_active: bool,
}

/// `0x007210b0`: computes the viewport rectangle.  `explicit` is the
/// caller rect (l, t, r, b) -> (l, t, r-l, b-t) with override set.  With
/// `None`: if `fullscreen_like` is false the base rect is inset by
/// (`ix`, `iy`) giving (x+ix, y+iy, w-2*ix, h-2*iy); otherwise the rect from
/// the provider is used unchanged.  Override is cleared in both cases.
pub fn w23a19_viewport_rect(
    explicit: Option<[i32; 4]>,
    fullscreen_like: bool,
    base: [i32; 4],
    provider_rect: [i32; 4],
    ix: i32,
    iy: i32,
) -> W23a19Rect {
    match explicit {
        Some(r) => W23a19Rect {
            rect: [r[0], r[1], r[2].wrapping_sub(r[0]), r[3].wrapping_sub(r[1])],
            override_active: true,
        },
        None if !fullscreen_like => W23a19Rect {
            rect: [
                base[0].wrapping_add(ix),
                base[1].wrapping_add(iy),
                base[2].wrapping_sub(ix.wrapping_mul(2)),
                base[3].wrapping_sub(iy.wrapping_mul(2)),
            ],
            override_active: false,
        },
        None => W23a19Rect {
            rect: provider_rect,
            override_active: false,
        },
    }
}

#[cfg(test)]
mod tests_wave23_agent19 {
    use super::*;

    #[test]
    fn dispatch_arg() {
        assert_eq!(w23a19_dispatch_arg(0), 0);
        assert_eq!(w23a19_dispatch_arg(1), 0xb);
    }

    #[test]
    fn find_slot() {
        assert_eq!(w23a19_find_slot_arg(&[0, 0, 0x0c, 0]), 0xe + 0x74);
        assert_eq!(w23a19_find_slot_arg(&[0x0c, 0, 0, 0]), 0xc + 0x74);
        assert_eq!(w23a19_find_slot_arg(&[0; 4]), 8);
    }

    #[test]
    fn flag() {
        let mut s = 0;
        assert_eq!(w23a19_set_flag(&mut s, 1, 0, false, false), 0);
        assert_eq!(s, 1);
        assert_eq!(w23a19_set_flag(&mut s, 1, 0, true, false), 1);
        assert_eq!(w23a19_set_flag(&mut s, 1, 1, false, false), 1);
        assert_eq!(w23a19_set_flag(&mut s, 1, 0, false, true), 1);
    }

    #[test]
    fn rect() {
        let e = w23a19_viewport_rect(Some([1, 2, 11, 22]), false, [0; 4], [0; 4], 0, 0);
        assert_eq!(
            e,
            W23a19Rect {
                rect: [1, 2, 10, 20],
                override_active: true
            }
        );
        let i = w23a19_viewport_rect(None, false, [0, 0, 100, 80], [9; 4], 5, 3);
        assert_eq!(i.rect, [5, 3, 90, 74]);
        let p = w23a19_viewport_rect(None, true, [0; 4], [1, 2, 3, 4], 5, 3);
        assert_eq!(p.rect, [1, 2, 3, 4]);
    }
}
