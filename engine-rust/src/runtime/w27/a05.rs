#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent05.rs
// ===========================================================================
// Wave 27 agent 05: glyph-atlas row allocator and small UI-record helpers.

/// Decoded 8-byte atlas row/slot descriptor returned by `0x00961d10`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W27a05Slot {
    pub lo: u16,
    pub hi: u16,
    pub offset: u32,
}

/// `0x00961d10`: reads slot `idx` of a table with a `u16` count header and
/// 8-byte entries starting at byte 8; an index above the count is replaced by
/// 0.  `base_addr` is the table address added to the 31-bit offset.
pub fn w27a05_read_slot(table: &[u8], idx: u8, base_addr: u32) -> Option<W27a05Slot> {
    let count = u16::from_le_bytes([*table.first()?, *table.get(1)?]);
    let i = if u16::from(idx) < count {
        idx as usize
    } else {
        0
    };
    let o = 8 + i * 8;
    let e = table.get(o..o + 8)?;
    Some(W27a05Slot {
        lo: u16::from(e[0]),
        hi: u16::from(e[1]),
        offset: (u32::from_le_bytes([e[2], e[3], e[4], e[5]]) & 0x7fff_ffff)
            .wrapping_add(base_addr),
    })
}

/// `0x00964660`: bit 5 of the flags word at record+0x34.
pub fn w27a05_flag34_bit5(flags: u32) -> bool {
    (flags >> 5) & 1 != 0
}

/// `0x00964690`: bit 2 of the flags word at record+0x30.
pub fn w27a05_flag30_bit2(flags: u32) -> bool {
    (flags >> 2) & 1 != 0
}

/// Atlas shelf allocator state (fields at +8 width, +0x0a height, +0x20 cursor).
#[derive(Clone, Debug, Default)]
pub struct W27a05Atlas {
    pub width: u16,
    pub height: i16,
    pub cursor_y: u16,
    /// 8-byte rows `[y, width, height, 0]`.
    pub rows: Vec<[u16; 4]>,
}

/// `0x00962b90`: allocates a new row of height `round(h)+2` (rounding helper
/// supplied by the caller) at the cursor; returns its index, or `None` when it
/// would overflow the atlas height.
pub fn w27a05_alloc_row(a: &mut W27a05Atlas, h: u16, round: impl Fn(f64) -> f32) -> Option<usize> {
    let rh = (round(f64::from(h)) + 2.0) as u32 as u16;
    if i32::from(a.height) < i32::from(rh) + i32::from(a.cursor_y) {
        return None;
    }
    a.rows.push([a.cursor_y, a.width, rh, 0]);
    a.cursor_y = a.cursor_y.wrapping_add(rh);
    Some(a.rows.len() - 1)
}

#[cfg(test)]
mod tests_wave27_agent05 {
    use super::*;

    #[test]
    fn slot_read() {
        let mut t = vec![0u8; 8 + 16];
        t[0] = 2;
        t[8 + 8] = 7;
        t[8 + 9] = 9;
        t[8 + 10..8 + 14].copy_from_slice(&0x8000_0010u32.to_le_bytes());
        let s = w27a05_read_slot(&t, 1, 0x100).unwrap();
        assert_eq!((s.lo, s.hi, s.offset), (7, 9, 0x110));
        assert_eq!(w27a05_read_slot(&t, 5, 0).unwrap().lo, 0);
        assert!(w27a05_read_slot(&[], 0, 0).is_none());
    }

    #[test]
    fn flags() {
        assert!(w27a05_flag34_bit5(0x20));
        assert!(!w27a05_flag34_bit5(0xdf));
        assert!(w27a05_flag30_bit2(4));
        assert!(!w27a05_flag30_bit2(0xfb));
    }

    #[test]
    fn rows() {
        let mut a = W27a05Atlas {
            width: 64,
            height: 20,
            ..Default::default()
        };
        let r = |v: f64| v as f32;
        assert_eq!(w27a05_alloc_row(&mut a, 8, r), Some(0));
        assert_eq!(a.rows[0], [0, 64, 10, 0]);
        assert_eq!(a.cursor_y, 10);
        assert_eq!(w27a05_alloc_row(&mut a, 8, r), Some(1));
        assert_eq!(w27a05_alloc_row(&mut a, 1, r), None);
    }
}
