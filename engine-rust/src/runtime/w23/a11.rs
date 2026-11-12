#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent11.rs
// ===========================================================================
// Wave 23 translation batch 11 (runtime glue helpers).

/// `0x006fbd60`: eases `*cur` toward `target`: the step is a quarter of the
/// difference (truncating toward zero), or the whole difference when it is
/// smaller than 4 in magnitude.
pub fn w23a11_ease_toward(cur: &mut i16, target: i16) {
    let d = target.wrapping_sub(*cur);
    let step = if d > 0 {
        if d < 4 { d } else { d >> 2 }
    } else if d < 0 {
        if d > -4 { d } else { (d + 3) >> 2 }
    } else {
        return;
    };
    *cur = cur.wrapping_add(step);
}

/// `0x006fc480`: reads a table record.  Header: `u16` (low 15 bits = count),
/// two `u16` copied to `out_pair`.  Entry index is `idx` when `idx < count`,
/// else 0; entries are 8 bytes at offset 6 (two `u32`).  Returns
/// `(first, second + base)` where `base` is the record's own address.
pub fn w23a11_read_record(rec: &[u8], idx: u16, base: u32) -> Option<((u16, u16), (u32, u32))> {
    let u16_at = |o: usize| rec.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let u32_at = |o: usize| {
        rec.get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let count = u16_at(0)? & 0x7fff;
    let pair = (u16_at(2)?, u16_at(4)?);
    let e = if idx < count { idx as usize } else { 0 };
    let a = u32_at(6 + e * 8)?;
    let b = u32_at(10 + e * 8)?.wrapping_add(base);
    Some((pair, (a, b)))
}

/// `0x006fa4f0`: number of bytes copied out of the NUL-terminated `src`
/// into a buffer of `cap` bytes: `min(strlen, cap - 1)`; nothing when
/// `cap == 0`.
pub fn w23a11_bounded_copy_len(src: &[u8], cap: usize) -> usize {
    if cap == 0 {
        return 0;
    }
    let len = src.iter().position(|&c| c == 0).unwrap_or(src.len());
    len.min(cap - 1)
}

/// `0x006fc200`: releases each distinct non-zero handle in `handles` once:
/// later duplicates are zeroed before `free` is called for the first.
pub fn w23a11_free_unique(handles: &mut [usize], mut free: impl FnMut(usize)) {
    for i in 0..handles.len() {
        let h = handles[i];
        if h != 0 {
            for x in handles[i + 1..].iter_mut() {
                if *x == h {
                    *x = 0;
                }
            }
            free(h);
        }
    }
}

/// `0x006fb530`: leave-glue guard.  Only when `state == 0x3bff7ba4` does it
/// act; returns the code passed on (0x21 if `current == requested`, else
/// 0xc), after which the caller resets the globals.
pub fn w23a11_leave_code(state: u32, current: i32, requested: i32) -> Option<u32> {
    if state != 0x3bff7ba4 {
        return None;
    }
    Some(if current == requested { 0x21 } else { 0xc })
}

#[cfg(test)]
mod tests_wave23_agent11 {
    use super::*;

    #[test]
    fn ease() {
        let mut v = 0i16;
        w23a11_ease_toward(&mut v, 100);
        assert_eq!(v, 25);
        w23a11_ease_toward(&mut v, 27);
        assert_eq!(v, 27);
        let mut v = 100i16;
        w23a11_ease_toward(&mut v, 0);
        assert_eq!(v, 75);
        let mut v = 2i16;
        w23a11_ease_toward(&mut v, 0);
        assert_eq!(v, 0);
        let mut v = 5i16;
        w23a11_ease_toward(&mut v, 5);
        assert_eq!(v, 5);
    }

    #[test]
    fn record() {
        let mut r = vec![0u8; 22];
        r[0..2].copy_from_slice(&0x8002u16.to_le_bytes());
        r[2..4].copy_from_slice(&7u16.to_le_bytes());
        r[4..6].copy_from_slice(&9u16.to_le_bytes());
        r[6..10].copy_from_slice(&1u32.to_le_bytes());
        r[10..14].copy_from_slice(&2u32.to_le_bytes());
        r[14..18].copy_from_slice(&3u32.to_le_bytes());
        r[18..22].copy_from_slice(&4u32.to_le_bytes());
        assert_eq!(w23a11_read_record(&r, 1, 100), Some(((7, 9), (3, 104))));
        assert_eq!(w23a11_read_record(&r, 5, 100), Some(((7, 9), (1, 102))));
        assert_eq!(w23a11_read_record(&r[..8], 0, 0), None);
    }

    #[test]
    fn copy_len() {
        assert_eq!(w23a11_bounded_copy_len(b"abc\0", 0), 0);
        assert_eq!(w23a11_bounded_copy_len(b"abc\0", 2), 1);
        assert_eq!(w23a11_bounded_copy_len(b"abc\0", 10), 3);
    }

    #[test]
    fn free_once() {
        let mut h = [5, 0, 5, 6, 6];
        let mut out = vec![];
        w23a11_free_unique(&mut h, |x| out.push(x));
        assert_eq!(out, vec![5, 6]);
        assert_eq!(h, [5, 0, 0, 6, 0]);
    }

    #[test]
    fn leave() {
        assert_eq!(w23a11_leave_code(1, 2, 2), None);
        assert_eq!(w23a11_leave_code(0x3bff7ba4, 2, 2), Some(0x21));
        assert_eq!(w23a11_leave_code(0x3bff7ba4, 2, 3), Some(0xc));
    }
}
