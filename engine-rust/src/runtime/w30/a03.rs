#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave30/agent03.rs
// ===========================================================================
// Wave 30 translation batch 03: tact async memory file and ref vector helpers.

/// Inline buffer capacity of the async memory file (`0x100` bytes).
pub const W30A03_INLINE_CAP: usize = 0x100;

/// Model of the async memory file layout (len at `+0x10c`, pos at `+0x110`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W30a03MemFile {
    pub data: Vec<u8>,
    pub len: u32,
    pub pos: u32,
}

/// `0x00af9760`: constructor; stores the length (`+0x10c`), zeroes the
/// position, and copies `len` bytes from `src` (inline when `<= 0x100`,
/// heap otherwise; both modelled as one owned buffer).
pub fn w30a03_memfile_new(src: &[u8], len: u32) -> W30a03MemFile {
    let n = (len as usize).min(src.len());
    let mut data = src[..n].to_vec();
    data.resize(len as usize, 0);
    W30a03MemFile { data, len, pos: 0 }
}

/// `0x00af9960`: read; clamps `*req` to `len - pos`, copies that many
/// bytes starting at `pos` into `dst`.  Position is not advanced.
pub fn w30a03_memfile_read(f: &W30a03MemFile, dst: &mut [u8], req: &mut u32) {
    let avail = f.len.wrapping_sub(f.pos);
    let n = (*req).min(avail);
    *req = n;
    let s = f.pos as usize;
    let n = n as usize;
    dst[..n].copy_from_slice(&f.data[s..s + n]);
}

/// `0x00af9ab0`: grow capacity word (low 31 bits capacity, bit 31 flag
/// preserved): adds `max(1, cap / 2)`; `None` on overflow.
pub fn w30a03_grow_cap(word: u32) -> Option<u32> {
    let cap = word & 0x7fff_ffff;
    cap.checked_add(1)?;
    let half = (word >> 1) & 0x3fff_ffff;
    let add = if half >= 1 { half } else { 1 };
    let new = cap.wrapping_add(add);
    Some((new ^ word) & 0x7fff_ffff ^ word)
}

/// `0x00af9ab0`: push a shared handle; grows when full and bumps the
/// handle's refcount (`+4`) if non-zero.
pub fn w30a03_push_ref(
    items: &mut Vec<u32>,
    cap_word: &mut u32,
    handle: u32,
    refs: &mut std::collections::HashMap<u32, u32>,
) {
    if items.len() as u32 == (*cap_word & 0x7fff_ffff) {
        if let Some(w) = w30a03_grow_cap(*cap_word) {
            *cap_word = w;
        }
    }
    items.push(handle);
    if handle != 0 {
        *refs.entry(handle).or_insert(0) += 1;
    }
}

#[cfg(test)]
mod tests_wave30_agent03 {
    use super::*;

    #[test]
    fn ctor_and_read_clamp() {
        let f = w30a03_memfile_new(&[1, 2, 3, 4], 4);
        let mut f2 = f.clone();
        f2.pos = 1;
        let mut d = [0u8; 8];
        let mut r = 10;
        w30a03_memfile_read(&f2, &mut d, &mut r);
        assert_eq!(r, 3);
        assert_eq!(&d[..3], &[2, 3, 4]);
        assert_eq!(f.pos, 0);
        let big = w30a03_memfile_new(&[], 0x101);
        assert_eq!(big.data.len(), 0x101);
    }

    #[test]
    fn grow_and_push() {
        assert_eq!(w30a03_grow_cap(0), Some(1));
        assert_eq!(w30a03_grow_cap(10), Some(15));
        assert_eq!(w30a03_grow_cap(0x8000_0000 | 4), Some(0x8000_0006));
        let mut v = vec![];
        let mut c = 0;
        let mut m = std::collections::HashMap::new();
        w30a03_push_ref(&mut v, &mut c, 7, &mut m);
        w30a03_push_ref(&mut v, &mut c, 0, &mut m);
        assert_eq!(v, vec![7, 0]);
        assert_eq!(m[&7], 1);
        assert_eq!(c, 2);
    }
}
