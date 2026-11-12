#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent26.rs
// ===========================================================================
// Wave 22 agent 26: runtime.rs helpers (stream word readers, small flag helpers).

/// Shared tail of the word readers: reads a little-endian dword at `off`
/// from `buf`.  When fewer than 4 bytes remain (distance `|len - off|`) the
/// result is zero-padded with the available bytes.  `None` when `buf` is absent.
pub fn w22a26_read_tail_word(buf: Option<&[u8]>, off: u32) -> Option<u32> {
    let buf = buf?;
    let len = buf.len() as u32;
    let remain = len.abs_diff(off);
    let mut out = [0u8; 4];
    let n = remain.min(4) as usize;
    for (i, b) in out.iter_mut().enumerate().take(n) {
        *b = buf.get(off as usize + i).copied().unwrap_or(0);
    }
    Some(u32::from_le_bytes(out))
}

fn w22a26_swap_pair(this: &mut [u32; 2], src: (u32, u32), changed: impl FnOnce() -> u32) -> u32 {
    let old = (this[0], this[1]);
    this[0] = src.0;
    this[1] = src.1;
    if old == src { 1 } else { changed() }
}

/// `0x006b0100`: stores the pair (fields 0x10/0x14) from `src` (0x48/0x4c);
/// returns 1 when unchanged, otherwise the result of the change callback.
pub fn w22a26_set_pair_a(
    this: &mut [u32; 2],
    src: (u32, u32),
    changed: impl FnOnce() -> u32,
) -> u32 {
    w22a26_swap_pair(this, src, changed)
}

/// `0x006b0140`: same as the pair setter at 6b0100 with a different change callback.
pub fn w22a26_set_pair_b(
    this: &mut [u32; 2],
    src: (u32, u32),
    changed: impl FnOnce() -> u32,
) -> u32 {
    w22a26_swap_pair(this, src, changed)
}

/// `0x006b0180`: same as the pair setter at 6b0100 with a third change callback.
pub fn w22a26_set_pair_c(
    this: &mut [u32; 2],
    src: (u32, u32),
    changed: impl FnOnce() -> u32,
) -> u32 {
    w22a26_swap_pair(this, src, changed)
}

/// `0x006b0250`: reads dword at `off`; if the cached entry `(off >> 2)` of
/// the 8-byte-stride table has a non-zero flag its value wins, else falls
/// back to the raw buffer read.
pub fn w22a26_read_cached_word(table: &[(u32, u8)], buf: Option<&[u8]>, off: u32) -> Option<u32> {
    if let Some(&(v, flag)) = table.get((off >> 2) as usize)
        && flag != 0
    {
        return Some(v);
    }
    w22a26_read_tail_word(buf, off)
}

/// `0x006b02d0`: offset 0 yields `reg_a` (field 0x18), offset 4 yields `reg_b`
/// (field 0x1c); anything else reads the buffer at `off - 8`.
pub fn w22a26_read_reg_or_word(
    reg_a: u32,
    reg_b: u32,
    buf: Option<&[u8]>,
    off: u32,
) -> Option<u32> {
    match off {
        0 => Some(reg_a),
        4 => Some(reg_b),
        _ => w22a26_read_tail_word(buf, off.wrapping_sub(8)),
    }
}

/// `0x006b03d0`: reads `byte_count / 4` dwords starting at `start` through an
/// aligned word reader, stitching unaligned dwords from two aligned reads.
/// `None` when any read fails.
pub fn w22a26_read_words(
    start: u32,
    byte_count: u32,
    mut read_aligned: impl FnMut(u32) -> Option<u32>,
) -> Option<Vec<u32>> {
    let mut out = Vec::new();
    let mut addr = start;
    for _ in 0..byte_count >> 2 {
        let base = addr & !3;
        if addr == base {
            out.push(read_aligned(addr)?);
        } else {
            let lo = read_aligned(base)?;
            let hi = read_aligned(addr.wrapping_add(4) & !3)?;
            let mut bytes = [0u8; 8];
            bytes[..4].copy_from_slice(&lo.to_le_bytes());
            bytes[4..].copy_from_slice(&hi.to_le_bytes());
            let k = (addr - base) as usize;
            out.push(u32::from_le_bytes([
                bytes[k],
                bytes[k + 1],
                bytes[k + 2],
                bytes[k + 3],
            ]));
        }
        addr = addr.wrapping_add(4);
    }
    Some(out)
}

/// `0x006b0470`: offsets 0x30..=0x38 are assembled from four single-byte
/// reads (little-endian); other offsets read the buffer.
pub fn w22a26_read_word_or_bytes(
    buf: Option<&[u8]>,
    off: u32,
    mut read_byte: impl FnMut(u32) -> u8,
) -> Option<u32> {
    if off.wrapping_sub(0x30) < 9 {
        let b = [
            read_byte(off),
            read_byte(off + 1),
            read_byte(off + 2),
            read_byte(off + 3),
        ];
        return Some(u32::from_le_bytes(b));
    }
    w22a26_read_tail_word(buf, off)
}

/// `0x006b0560`: offsets at or past `limit` (field 0x18) read as zero,
/// otherwise the buffer is read.
pub fn w22a26_read_word_limited(limit: u32, buf: Option<&[u8]>, off: u32) -> Option<u32> {
    if limit <= off {
        return Some(0);
    }
    w22a26_read_tail_word(buf, off)
}

/// `0x006b1860`: resizes a vector of 8-byte `(u32, u8)` records; shrinking
/// erases the tail, growing zero-fills the new records.
pub fn w22a26_resize_records(v: &mut Vec<(u32, u8)>, new_len: usize) {
    if new_len < v.len() {
        v.truncate(new_len);
    } else {
        v.resize(new_len, (0, 0));
    }
}

fn w22a26_flush_copy(flags: &mut u32, src: Option<&[u32]>, dst: &mut [u32], n: usize) -> bool {
    if (*flags >> 11) & 1 == 0 {
        return false;
    }
    match src {
        Some(s) => {
            dst[..n].copy_from_slice(&s[..n]);
            true
        }
        None => {
            *flags &= 0xffff_f7ff;
            false
        }
    }
}

/// `0x006b1980`: when flag bit 11 is set and a source exists, copies 600
/// dwords into `dst` (then the source is released); with no source clears
/// the flag.  Returns whether a copy occurred.
pub fn w22a26_flush_600(flags: &mut u32, src: Option<&[u32]>, dst: &mut [u32]) -> bool {
    w22a26_flush_copy(flags, src, dst, 600)
}

/// `0x006b19d0`: same as 6b1980 but copies 0x4f dwords.
pub fn w22a26_flush_4f(flags: &mut u32, src: Option<&[u32]>, dst: &mut [u32]) -> bool {
    w22a26_flush_copy(flags, src, dst, 0x4f)
}

/// `0x006b1ad0`: if flag bit 14 is already set returns `(flags >> 22) << 8`;
/// otherwise sets it and returns `((flags >> 22) << 8) | 1`, calling `on_zero`
/// first when `byte40` is zero (the callback's own work follows).
pub fn w22a26_mark_flag14(flags: &mut u32, byte40: u8, on_zero: impl FnOnce()) -> u32 {
    let hi = (*flags >> 0x16) << 8;
    if (*flags >> 14) & 1 != 0 {
        return hi;
    }
    *flags |= 0x4000;
    if byte40 == 0 {
        on_zero();
    }
    hi | 1
}

#[cfg(test)]
mod tests_wave22_agent26 {
    use super::*;

    #[test]
    fn pair_setters() {
        let mut p = [1, 2];
        assert_eq!(w22a26_set_pair_a(&mut p, (1, 2), || 9), 1);
        assert_eq!(w22a26_set_pair_b(&mut p, (3, 4), || 9), 9);
        assert_eq!(p, [3, 4]);
        assert_eq!(w22a26_set_pair_c(&mut p, (3, 5), || 7), 7);
    }

    #[test]
    fn readers() {
        let buf = [1u8, 0, 0, 0, 2, 0];
        assert_eq!(w22a26_read_tail_word(Some(&buf), 0), Some(1));
        assert_eq!(w22a26_read_tail_word(Some(&buf), 4), Some(2));
        assert_eq!(w22a26_read_tail_word(None, 0), None);
        assert_eq!(w22a26_read_cached_word(&[(5, 1)], None, 0), Some(5));
        assert_eq!(w22a26_read_cached_word(&[(5, 0)], None, 0), None);
        assert_eq!(w22a26_read_reg_or_word(7, 8, None, 4), Some(8));
        assert_eq!(w22a26_read_reg_or_word(7, 8, Some(&buf), 8), Some(1));
        assert_eq!(w22a26_read_word_limited(4, None, 4), Some(0));
        assert_eq!(w22a26_read_word_limited(8, Some(&buf), 0), Some(1));
        assert_eq!(
            w22a26_read_word_or_bytes(None, 0x30, |a| a as u8),
            Some(0x33323130)
        );
        assert_eq!(w22a26_read_word_or_bytes(None, 0x39, |a| a as u8), None);
    }

    #[test]
    fn unaligned_words() {
        let mem = [0x0403_0201u32, 0x0807_0605];
        let r = |a: u32| mem.get((a / 4) as usize).copied();
        assert_eq!(
            w22a26_read_words(0, 8, r),
            Some(vec![0x04030201, 0x08070605])
        );
        assert_eq!(w22a26_read_words(1, 4, r), Some(vec![0x05040302]));
        assert_eq!(w22a26_read_words(4, 8, r), None);
    }

    #[test]
    fn records_and_flags() {
        let mut v = vec![(1, 1)];
        w22a26_resize_records(&mut v, 3);
        assert_eq!(v, [(1, 1), (0, 0), (0, 0)]);
        w22a26_resize_records(&mut v, 1);
        assert_eq!(v.len(), 1);

        let mut f = 0x800u32;
        let mut dst = vec![0u32; 600];
        assert!(!w22a26_flush_600(&mut f, None, &mut dst));
        assert_eq!(f, 0);
        f = 0x800;
        let src = vec![3u32; 600];
        assert!(w22a26_flush_600(&mut f, Some(&src), &mut dst));
        assert_eq!(dst[599], 3);
        assert!(w22a26_flush_4f(&mut f, Some(&src), &mut dst));
        assert!(!w22a26_flush_4f(&mut 0, Some(&src), &mut dst));

        let mut fl = 0u32;
        let mut called = false;
        assert_eq!(w22a26_mark_flag14(&mut fl, 0, || called = true), 1);
        assert!(called && fl == 0x4000);
        assert_eq!(w22a26_mark_flag14(&mut fl, 0, || {}), 0);
    }
}
