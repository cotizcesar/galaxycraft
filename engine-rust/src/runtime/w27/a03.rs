#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent03.rs
// ===========================================================================
// Wave 27 agent 03: small runtime/state helpers (0x0095xxxx block).

/// Two saved 16-byte quads plus their trailing flag words (globals
/// `011cb6c8..011cb6f0`).  `cur` is the live pair, `prev` the history slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W27a03Pair {
    pub cur: [u32; 4],
    pub cur_flag: u32,
    pub prev: [u32; 4],
    pub prev_flag: u32,
}

/// `0x009583a0`: push a new value; the old current becomes `prev`.  The
/// flag word keeps only `flag` in its low byte (upper bytes were stack
/// garbage in retail; modelled as zero).
pub fn w27a03_push_pair(s: &mut W27a03Pair, quad: [u32; 4], flag: u8) {
    s.prev_flag = s.cur_flag;
    s.prev = s.cur;
    s.cur_flag = u32::from(flag);
    s.cur = quad;
}

/// `0x00958560`: swap the current and previous slots.
pub fn w27a03_swap_pair(s: &mut W27a03Pair) {
    std::mem::swap(&mut s.cur, &mut s.prev);
    std::mem::swap(&mut s.cur_flag, &mut s.prev_flag);
}

/// `0x00958b60`: if the global flag is set, defer to `other(0x20, 0)`,
/// otherwise return `byte10 >> 1` of the record.
pub fn w27a03_half_byte10(flag_set: bool, rec: &[u8], other: impl FnOnce(u32, u32) -> f64) -> f64 {
    if flag_set {
        other(0x20, 0)
    } else {
        f64::from(rec[10] >> 1)
    }
}

/// Outcome of [`w27a03_resize_pairs`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W27a03Resize {
    Unchanged,
    Shrunk,
    Grown,
    /// Capacity word must be replaced by this value and storage reallocated.
    Realloc(u32),
}

/// `0x00958bb0`: resize a vector of 8-byte elements.  Shrinking truncates
/// (the destructor range call is modelled as truncate).  Growing fills with
/// `(0xffffffff, 0)`; if `cap_word & 0x7fffffff < new_len` the new capacity
/// is `max(new_len, cap + (cap >> 1 & 0x3fffffff))` merged under the high bit.
pub fn w27a03_resize_pairs(v: &mut Vec<[u32; 2]>, cap_word: u32, new_len: u32) -> W27a03Resize {
    let len = v.len() as u32;
    if new_len < len {
        v.truncate(new_len as usize);
        W27a03Resize::Shrunk
    } else if len < new_len {
        if (cap_word & 0x7fff_ffff) < new_len {
            let mut want = ((cap_word >> 1) & 0x3fff_ffff).wrapping_add(cap_word & 0x7fff_ffff);
            if want < new_len {
                want = new_len;
            }
            return W27a03Resize::Realloc(((cap_word ^ want) & 0x7fff_ffff) ^ cap_word);
        }
        v.resize(new_len as usize, [0xffff_ffff, 0]);
        W27a03Resize::Grown
    } else {
        W27a03Resize::Unchanged
    }
}

/// `0x00958cd0`: allocate `w * h` (two i16 at offsets 0 and 2) and store the
/// result at offset 4.
pub fn w27a03_alloc_grid(hdr: &mut [u8; 8], alloc: impl FnOnce(i32) -> u32) {
    let w = i16::from_le_bytes([hdr[0], hdr[1]]);
    let h = i16::from_le_bytes([hdr[2], hdr[3]]);
    let p = alloc(i32::from(h).wrapping_mul(i32::from(w)));
    hdr[4..8].copy_from_slice(&p.to_le_bytes());
}

/// `0x00958d00`: release the pointer at offset 4 when non-null.
pub fn w27a03_free_grid(hdr: &[u8; 8], free: impl FnOnce(u32)) {
    let p = u32::from_le_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]);
    if p != 0 {
        free(p);
    }
}

/// `0x00958f80`: if the mode flag is set, clear it and zero the 0x400-byte
/// table A.
pub fn w27a03_leave_mode(flag: &mut bool, table_a: &mut [u8]) {
    if *flag {
        *flag = false;
        table_a[..0x400].fill(0);
    }
}

/// `0x00958fe0`: if the mode flag is clear, set it and zero the 0x400-byte
/// table B.
pub fn w27a03_enter_mode(flag: &mut bool, table_b: &mut [u8]) {
    if !*flag {
        *flag = true;
        table_b[..0x400].fill(0);
    }
}

/// `0x009590a0`: zero the 0x80-byte table.
pub fn w27a03_clear_small(table: &mut [u8]) {
    table[..0x80].fill(0);
}

/// `0x00959230`: after `prepare(0)`, copy `count * 4` bytes from `src` into
/// table B at `index * 4`.
pub fn w27a03_copy_entries(
    table_b: &mut [u8],
    index: u8,
    count: u16,
    src: &[u8],
    prepare: impl FnOnce(u32),
) {
    prepare(0);
    let off = usize::from(index) * 4;
    let n = usize::from(count) << 2;
    table_b[off..off + n].copy_from_slice(&src[..n]);
}

#[cfg(test)]
mod tests_wave27_agent03 {
    use super::*;

    #[test]
    fn push_swap() {
        let mut s = W27a03Pair::default();
        w27a03_push_pair(&mut s, [1, 2, 3, 4], 7);
        w27a03_push_pair(&mut s, [5, 6, 7, 8], 9);
        assert_eq!(
            (s.cur, s.cur_flag, s.prev, s.prev_flag),
            ([5, 6, 7, 8], 9, [1, 2, 3, 4], 7)
        );
        w27a03_swap_pair(&mut s);
        assert_eq!((s.cur, s.cur_flag, s.prev_flag), ([1, 2, 3, 4], 7, 9));
    }

    #[test]
    fn half_byte() {
        let mut r = [0u8; 16];
        r[10] = 9;
        assert_eq!(w27a03_half_byte10(false, &r, |_, _| 0.0), 4.0);
        assert_eq!(w27a03_half_byte10(true, &r, |a, b| f64::from(a + b)), 32.0);
    }

    #[test]
    fn resize() {
        let mut v = vec![[1, 1]; 2];
        assert_eq!(w27a03_resize_pairs(&mut v, 4, 2), W27a03Resize::Unchanged);
        assert_eq!(w27a03_resize_pairs(&mut v, 4, 4), W27a03Resize::Grown);
        assert_eq!(v[3], [0xffff_ffff, 0]);
        assert_eq!(w27a03_resize_pairs(&mut v, 4, 1), W27a03Resize::Shrunk);
        assert_eq!(v.len(), 1);
        assert_eq!(
            w27a03_resize_pairs(&mut v, 4, 10),
            W27a03Resize::Realloc(10)
        );
        assert_eq!(w27a03_resize_pairs(&mut v, 4, 5), W27a03Resize::Realloc(6));
    }

    #[test]
    fn grid() {
        let mut h = [0u8; 8];
        h[0] = 3;
        h[2] = 0xfe;
        h[3] = 0xff;
        w27a03_alloc_grid(&mut h, |n| (n * 10) as u32);
        assert_eq!(
            u32::from_le_bytes([h[4], h[5], h[6], h[7]]),
            (-60i32) as u32
        );
        let mut hit = 0;
        w27a03_free_grid(&h, |_| hit += 1);
        w27a03_free_grid(&[0; 8], |_| hit += 10);
        assert_eq!(hit, 1);
    }

    #[test]
    fn tables() {
        let mut a = vec![1u8; 0x400];
        let mut f = true;
        w27a03_leave_mode(&mut f, &mut a);
        assert!(!f && a.iter().all(|&x| x == 0));
        let mut b = vec![1u8; 0x400];
        w27a03_enter_mode(&mut f, &mut b);
        assert!(f && b[0] == 0);
        b.fill(1);
        w27a03_enter_mode(&mut f, &mut b);
        assert_eq!(b[0], 1);
        let mut s = vec![1u8; 0x80];
        w27a03_clear_small(&mut s);
        assert!(s.iter().all(|&x| x == 0));
        let mut seen = 9;
        w27a03_copy_entries(&mut b, 2, 2, &[7; 8], |v| seen = v);
        assert_eq!((seen, b[8], b[15], b[16]), (0, 7, 7, 1));
    }
}
