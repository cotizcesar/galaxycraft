#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent02.rs
// ===========================================================================
// Wave 27 agent 02: text/font layout globals and small record helpers.

/// `0x00954880`: when the slot's first byte (stride `0x14` table) is
/// non-zero, sets bit 0 of its second byte.
pub fn w27a02_set_slot_flag(table: &mut [u8], index: u8) {
    let base = index as usize * 0x14;
    if table.get(base).copied().unwrap_or(0) != 0 {
        if let Some(b) = table.get_mut(base + 1) {
            *b |= 1;
        }
    }
}

/// `0x00955ad0`: bounds-checked row lookup.  The container has a count at
/// offset 4 and 8-byte rows starting at offset 0xc; returns the byte offset
/// of the row or `None` (retail returns 0).
pub fn w27a02_row_offset(count: u32, index: u32) -> Option<usize> {
    if count <= index {
        None
    } else {
        Some(index as usize * 8 + 0xc)
    }
}

/// `0x009576d0`: builds `[x, y, w, h]` from the packed text rectangle
/// (`left_top`: low i16 = left, high i16 = top; `right_bottom` likewise).
/// Retail overwrites the height with the font's byte at offset 0xb.
pub fn w27a02_text_rect(left_top: u32, right_bottom: u32, font_height: u8) -> [i32; 4] {
    let left = (left_top as u16 as i16) as i32;
    let top = ((left_top >> 16) as u16 as i16) as i32;
    let right = (right_bottom as u16 as i16) as i32;
    let bottom = ((right_bottom >> 16) as u16 as i16) as i32;
    let _ = bottom - top; // computed then overwritten in retail
    [left, top, right - left, font_height as i32]
}

/// `0x00957a80`: true when `buf` (NUL-terminated) is a prefix of `other`
/// (compares `strlen(buf)` bytes).
pub fn w27a02_buffer_is_prefix(buf: &[u8], other: &[u8]) -> bool {
    let n = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    other.len() >= n && buf[..n] == other[..n]
}

/// `0x00957ac0`: true when the font-mode flag equals 1.
pub fn w27a02_font_mode_is_one(mode: u32) -> bool {
    mode == 1
}

/// `0x00957b10`: font file path, format `font\%s.fnt`.
pub fn w27a02_font_path(name: &str) -> String {
    format!("font\\{name}.fnt")
}

#[cfg(test)]
mod tests_wave27_agent02 {
    use super::*;

    #[test]
    fn slot_flag() {
        let mut t = vec![0u8; 0x28];
        w27a02_set_slot_flag(&mut t, 1);
        assert_eq!(t[0x15], 0);
        t[0x14] = 1;
        w27a02_set_slot_flag(&mut t, 1);
        assert_eq!(t[0x15], 1);
    }

    #[test]
    fn row_offset() {
        assert_eq!(w27a02_row_offset(3, 2), Some(0x1c));
        assert_eq!(w27a02_row_offset(3, 3), None);
    }

    #[test]
    fn rect() {
        let r = w27a02_text_rect(0x0005_fff0, 0x0020_0010, 12);
        assert_eq!(r, [-16, 5, 32, 12]);
    }

    #[test]
    fn prefix_and_mode() {
        assert!(w27a02_buffer_is_prefix(b"ab\0", b"abc"));
        assert!(!w27a02_buffer_is_prefix(b"abd\0", b"abc"));
        assert!(w27a02_font_mode_is_one(1));
        assert!(!w27a02_font_mode_is_one(0));
        assert_eq!(w27a02_font_path("font16"), "font\\font16.fnt");
    }
}
