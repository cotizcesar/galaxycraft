#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent27.rs
// ===========================================================================
// Wave 32 translation batch 27.

/// `0x009ae780`: seek helper.  Unless `mode == 8` the offset is rescaled as
/// `offset * num / den` (64-bit unsigned) and rounded down to a multiple of
/// `align`; the absolute position `base + offset` is passed to `seek`.
/// Returns `None` when `den` or `align` is zero (the original would fault).
pub fn w32a27_seek_scaled(
    base: u32,
    offset: u32,
    mode: i32,
    num: u32,
    den: u32,
    align: u32,
    seek: &mut dyn FnMut(u32),
) -> Option<u32> {
    let mut off = offset;
    if mode != 8 {
        if den == 0 || align == 0 {
            return None;
        }
        off = ((offset as u64 * num as u64) / den as u64) as u32;
        off -= off % align;
    }
    let pos = base.wrapping_add(off);
    seek(pos);
    Some(pos)
}

#[cfg(test)]
mod tests_wave32_agent27 {
    use super::*;

    #[test]
    fn scaled() {
        let mut got = 0;
        assert_eq!(
            w32a27_seek_scaled(100, 10, 0, 3, 2, 4, &mut |p| got = p),
            Some(112)
        );
        assert_eq!(got, 112);
        assert_eq!(
            w32a27_seek_scaled(100, 10, 8, 3, 2, 4, &mut |_| {}),
            Some(110)
        );
        assert_eq!(w32a27_seek_scaled(0, 1, 0, 1, 0, 1, &mut |_| {}), None);
    }
}
