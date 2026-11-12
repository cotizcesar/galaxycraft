#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent54.rs
// ===========================================================================
// Wave 32 agent 54: glues::Control construction and widget move.

/// `0x0087b0a0`: glues::Control constructor body.  `base_ctor` stands for
/// the base constructor call; `obj` is the object as 32-bit words.
pub fn w32a54_control_ctor(
    obj: &mut [u32],
    param_1: u32,
    param_2: u32,
    vftable: u32,
    base_ctor: &mut dyn FnMut(&mut [u32]),
) {
    base_ctor(obj);
    obj[0xd] = param_1;
    obj[0xe] = param_2;
    obj[0] = vftable;
    // Byte at word 0xf is cleared; the rest of that word is untouched.
    obj[0xf] &= !0xff;
    obj[0x10] = 0;
    obj[0x11] = 0;
    obj[0x12] = 0;
    obj[0x13] = 0xffff_ffff;
}

fn w32a54_rd(b: &[u8], off: usize) -> i16 {
    i16::from_le_bytes([b[off], b[off + 1]])
}

fn w32a54_add(b: &mut [u8], off: usize, d: i16) {
    let v = w32a54_rd(b, off).wrapping_add(d);
    b[off..off + 2].copy_from_slice(&v.to_le_bytes());
}

/// `0x0087c580`: moves a widget record (`w`, the object at `this+4`) so its
/// origin becomes (`x`, `y`).  Type 0xe widgets also shift the extra
/// coordinate fields.  `apply` receives `[left, top, right, bottom]` offset
/// per axis (the callee `FUN_0087c750`), once per axis.
pub fn w32a54_move_widget(w: &mut [u8], x: i16, y: i16, apply: &mut dyn FnMut([i16; 4])) {
    let dx = x.wrapping_sub(w32a54_rd(w, 4));
    if w32a54_rd(w, 0x3c) == 0xe {
        for off in [0x60, 0x64, 0x50, 0x54] {
            w32a54_add(w, off, dx);
        }
    }
    apply([
        w32a54_rd(w, 4).wrapping_add(dx),
        w32a54_rd(w, 6),
        w32a54_rd(w, 8).wrapping_add(dx),
        w32a54_rd(w, 10),
    ]);
    let dy = y.wrapping_sub(w32a54_rd(w, 6));
    if w32a54_rd(w, 0x3c) == 0xe {
        for off in [0x62, 0x66, 0x52, 0x56] {
            w32a54_add(w, off, dy);
        }
    }
    apply([
        w32a54_rd(w, 4),
        w32a54_rd(w, 6).wrapping_add(dy),
        w32a54_rd(w, 8),
        w32a54_rd(w, 10).wrapping_add(dy),
    ]);
}

#[cfg(test)]
mod tests_wave32_agent54 {
    use super::*;

    #[test]
    fn ctor_sets_fields() {
        let mut o = [7u32; 0x14];
        w32a54_control_ctor(&mut o, 1, 2, 0x99, &mut |_| {});
        assert_eq!((o[0xd], o[0xe], o[0]), (1, 2, 0x99));
        assert_eq!(o[0xf], 0);
        assert_eq!(o[0x13], u32::MAX);
    }

    #[test]
    fn move_type_e() {
        let mut w = [0u8; 0x70];
        w[0x3c] = 0xe;
        w[4..6].copy_from_slice(&10i16.to_le_bytes());
        w[8..10].copy_from_slice(&20i16.to_le_bytes());
        let mut calls = vec![];
        w32a54_move_widget(&mut w, 15, 0, &mut |r| calls.push(r));
        assert_eq!(calls[0], [15, 0, 25, 0]);
        assert_eq!(w32a54_rd(&w, 0x60), 5);
        assert_eq!(calls.len(), 2);
    }
}
