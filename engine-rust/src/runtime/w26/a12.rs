#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent12.rs
// ===========================================================================
// Wave 26 agent 12: compression / image-buffer helpers.

/// `0x00931cf0`: number of bytes copied out of a cursor buffer; the
/// remaining span `end - pos` clamped to `wanted`.
pub fn w26a12_clamped_copy_len(pos: u32, end: u32, wanted: u32) -> u32 {
    let avail = end.wrapping_sub(pos);
    if wanted < avail { wanted } else { avail }
}

/// `0x00931f00`: zlib-style arena allocator.  Rounds `count * size` up to a
/// multiple of 4; returns the new cursor offset when it fits in the
/// remaining space, `None` when it does not.  `cursor`, `base`, `capacity`
/// mirror the stream fields 0, 1, 2 (remaining = capacity + base - cursor).
pub fn w26a12_arena_alloc(
    cursor: u32,
    base: u32,
    capacity: u32,
    count: u32,
    size: u32,
) -> Option<u32> {
    let mut n = count.wrapping_mul(size);
    if n & 3 != 0 {
        n += 4 - (n & 3);
    }
    let remaining = capacity.wrapping_add(base).wrapping_sub(cursor);
    if n < remaining {
        Some(cursor.wrapping_add(n))
    } else {
        None
    }
}

/// `0x00933340`: image-header row check.  `hdr` is the header bytes
/// (u16 at +3 = palette entries, u16 at +5 = width, byte +7 = bits per
/// pixel).  Returns `Some(palette_entries * 4)` zero-fill length when
/// `width * ((bpp + 7) / 8) == data_len`, else `None`.
pub fn w26a12_row_check_zero_len(hdr: &[u8], data_len: u32) -> Option<u32> {
    if hdr.len() < 8 {
        return None;
    }
    let pal = u32::from(u16::from_le_bytes([hdr[3], hdr[4]]));
    let width = u32::from(u16::from_le_bytes([hdr[5], hdr[6]]));
    let bytes_pp = (u32::from(hdr[7]) + 7) >> 3;
    if (width * bytes_pp).wrapping_sub(data_len) != 0 {
        return None;
    }
    Some(pal * 4)
}

/// `0x00933f50`: copy one row from the bottom of a bitmap.  Any zero
/// argument fails with error 0x57.  Returns `Ok((src_offset, len))` where
/// `src_offset = (row - 1) * stride` and `len = min(dst_len, src_len)`.
pub fn w26a12_bottom_row_copy(
    dst_len: u32,
    src_len: u32,
    stride: u32,
    row: u32,
) -> Result<(u32, u32), u32> {
    if dst_len == 0 || src_len == 0 || stride == 0 || row == 0 {
        return Err(0x57);
    }
    Ok((
        row.wrapping_sub(1).wrapping_mul(stride),
        dst_len.min(src_len),
    ))
}

#[cfg(test)]
mod tests_wave26_agent12 {
    use super::*;

    #[test]
    fn copy_len() {
        assert_eq!(w26a12_clamped_copy_len(4, 10, 100), 6);
        assert_eq!(w26a12_clamped_copy_len(4, 10, 3), 3);
    }

    #[test]
    fn alloc() {
        assert_eq!(w26a12_arena_alloc(0, 0, 100, 3, 3), Some(12));
        assert_eq!(w26a12_arena_alloc(0, 0, 8, 2, 4), None);
    }

    #[test]
    fn row_check() {
        let h = [0, 0, 0, 2, 0, 5, 0, 24];
        assert_eq!(w26a12_row_check_zero_len(&h, 15), Some(8));
        assert_eq!(w26a12_row_check_zero_len(&h, 14), None);
    }

    #[test]
    fn bottom_row() {
        assert_eq!(w26a12_bottom_row_copy(8, 4, 16, 3), Ok((32, 4)));
        assert_eq!(w26a12_bottom_row_copy(8, 4, 16, 0), Err(0x57));
    }
}
