#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent20.rs
// ===========================================================================
// Wave 26 agent 20: allocator / container / MD5-counter helpers (runtime.rs).

/// `0x0094aaa0`: lazy allocation of a 3-dimensional table.  `this` is
/// `[data, count, a, b, c]` (u32 words); when `data == 0` the element count
/// `c * b * a` is stored at word 1 and the allocator result at word 0.
pub fn w26a20_lazy_alloc(this: &mut [u32; 5], alloc: impl FnOnce(u32) -> u32) {
    if this[0] == 0 {
        let n = this[4].wrapping_mul(this[3]).wrapping_mul(this[2]);
        this[1] = n;
        this[0] = alloc(n);
    }
}

/// `0x0094ac70`: initialiser of a hash-table node (`TSHashTable<HotkeyData>`,
/// 0x2c bytes, 11 words).  `base` is the address of the node, used for the
/// self-referencing link at word 3.  Returns the words.
pub fn w26a20_hash_table_init(base: u32) -> [u32; 11] {
    let link = base.wrapping_add(12);
    let mut w = [0u32; 11];
    w[0] = 1;
    w[2] = 0xc;
    w[3] = link;
    w[4] = link | 1;
    w[9] = 0xffff_ffff;
    w[10] = 0x2000;
    w
}

/// `0x0094b850`: atomic reference-count release.  Returns `(previous,
/// destroy)`; `destroy` is true when the count reached zero.
pub fn w26a20_release(count: &std::sync::atomic::AtomicI32) -> (i32, bool) {
    let prev = count.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    (prev, prev - 1 == 0)
}

/// `0x0094bce0`: allocation request normalisation: a zero size becomes 1;
/// the allocator slot 1 is called with alignment 8.  Returns `(size, align)`.
pub fn w26a20_alloc_request(size: u32) -> (u32, u32) {
    (if size != 0 { size } else { 1 }, 8)
}

/// `0x0094bd20`: free guard: the allocator free slot is only invoked for a
/// non-null pointer.
pub fn w26a20_free_guard(ptr: u32, free: impl FnOnce(u32)) {
    if ptr != 0 {
        free(ptr);
    }
}

/// `0x0094bd60`: zeroes a 14-word (0x38 byte) record.
pub fn w26a20_zero_record(rec: &mut [u32; 14]) {
    *rec = [0; 14];
}

/// `0x0094bf40`: scans the four 18-byte records at offset 0x36 of `block`
/// for one whose bytes 0..=2 are zero, byte 3 is 0xfc and byte 4 is zero;
/// the text starts at record+5 and ends at `\n` or the record end.  With
/// `cap == 0` returns the text length + 1 as `Err`-free `Some(Length(n))`;
/// otherwise the bytes to copy (at most `cap - 1`).
pub fn w26a20_find_text(block: &[u8], cap: usize) -> Option<Result<usize, Vec<u8>>> {
    let mut rec = 0x36usize;
    while rec != 0x7e {
        if block.len() < rec + 18 {
            return None;
        }
        let r = &block[rec..rec + 18];
        if r[0] == 0 && r[1] == 0 && r[2] == 0 && r[3] == 0xfc && r[4] == 0 {
            let text = &r[5..];
            let end = text.iter().position(|&c| c == b'\n').unwrap_or(text.len());
            if cap == 0 {
                return Some(Ok(end + 1));
            }
            return Some(Err(text[..end.min(cap - 1)].to_vec()));
        }
        rec += 18;
    }
    None
}

/// `0x0094c0f0`: realloc through an allocator vtable.  `try_in_place(ptr,
/// size)` is slot +0xc, `alloc(size, align)` slot +4, `old_size(ptr)` slot
/// +0x14.  Returns the new pointer and the number of bytes to copy.
pub fn w26a20_realloc(
    ptr: u32,
    size: u32,
    align: u32,
    try_in_place: impl FnOnce(u32, u32) -> bool,
    alloc: impl FnOnce(u32, u32) -> u32,
    old_size: impl FnOnce(u32) -> u32,
) -> (u32, u32) {
    if ptr & align.wrapping_sub(1) == 0 && try_in_place(ptr, size) {
        return (ptr, 0);
    }
    let new = alloc(size, align);
    if new == 0 {
        return (0, 0);
    }
    (new, old_size(ptr).min(size))
}

/// `0x0094e5d0`: MD5 single-byte update bookkeeping.  `ctx` words: bit count
/// low at 0x14, high at 0x18, buffer at 0x1c (0x40 bytes).  Returns the
/// buffer index and the number of bytes copied from the source.
pub fn w26a20_md5_update_one(ctx: &mut [u8; 0x5c]) -> (usize, usize) {
    w26a20_md5_count(ctx, 1)
}

/// `0x0094f9d0`: MD5 update bookkeeping for `len` bytes.  Returns the buffer
/// index and the first copy length (`0x40 - idx` when the block fills,
/// otherwise `len`).
pub fn w26a20_md5_update(ctx: &mut [u8; 0x5c], len: u32) -> (usize, usize) {
    w26a20_md5_count(ctx, len)
}

fn w26a20_md5_count(ctx: &mut [u8; 0x5c], len: u32) -> (usize, usize) {
    let lo = u32::from_le_bytes(ctx[0x14..0x18].try_into().unwrap());
    let hi = u32::from_le_bytes(ctx[0x18..0x1c].try_into().unwrap());
    let idx = (lo >> 3) & 0x3f;
    let new_lo = lo.wrapping_add(len.wrapping_mul(8));
    let carry = u32::from(new_lo < len.wrapping_mul(8));
    let new_hi = (len >> 29).wrapping_add(hi.wrapping_add(carry));
    ctx[0x14..0x18].copy_from_slice(&new_lo.to_le_bytes());
    ctx[0x18..0x1c].copy_from_slice(&new_hi.to_le_bytes());
    let n = if idx + len > 0x3f { 0x40 - idx } else { len };
    (idx as usize, n as usize)
}

#[cfg(test)]
mod tests_wave26_agent20 {
    use super::*;

    #[test]
    fn alloc_helpers() {
        let mut t = [0, 0, 2, 3, 4];
        w26a20_lazy_alloc(&mut t, |n| n + 100);
        assert_eq!((t[0], t[1]), (124, 24));
        w26a20_lazy_alloc(&mut t, |_| 1);
        assert_eq!(t[0], 124);
        assert_eq!(w26a20_alloc_request(0), (1, 8));
        assert_eq!(w26a20_alloc_request(5), (5, 8));
        let mut called = false;
        w26a20_free_guard(0, |_| called = true);
        assert!(!called);
        w26a20_free_guard(3, |_| called = true);
        assert!(called);
        let mut r = [7u32; 14];
        w26a20_zero_record(&mut r);
        assert_eq!(r, [0; 14]);
    }

    #[test]
    fn table_and_refcount() {
        let w = w26a20_hash_table_init(0x1000);
        assert_eq!((w[3], w[4], w[2], w[10]), (0x100c, 0x100d, 0xc, 0x2000));
        let c = std::sync::atomic::AtomicI32::new(2);
        assert_eq!(w26a20_release(&c), (2, false));
        assert_eq!(w26a20_release(&c), (1, true));
    }

    #[test]
    fn find_text_and_realloc() {
        let mut b = vec![0u8; 0x90];
        let rec = 0x36 + 18;
        b[rec + 3] = 0xfc;
        b[rec + 5..rec + 8].copy_from_slice(b"ab\n");
        assert_eq!(w26a20_find_text(&b, 0), Some(Ok(3)));
        assert_eq!(w26a20_find_text(&b, 2), Some(Err(b"a".to_vec())));
        assert_eq!(w26a20_find_text(&[0u8; 0x90], 0), None);
        assert_eq!(
            w26a20_realloc(16, 8, 8, |_, _| true, |_, _| 0, |_| 0),
            (16, 0)
        );
        assert_eq!(
            w26a20_realloc(17, 8, 8, |_, _| true, |_, _| 64, |_| 4),
            (64, 4)
        );
        assert_eq!(
            w26a20_realloc(17, 8, 8, |_, _| true, |_, _| 0, |_| 4),
            (0, 0)
        );
    }

    #[test]
    fn md5_counts() {
        let mut c = [0u8; 0x5c];
        assert_eq!(w26a20_md5_update(&mut c, 10), (0, 10));
        assert_eq!(w26a20_md5_update(&mut c, 60), (10, 54));
        assert_eq!(u32::from_le_bytes(c[0x14..0x18].try_into().unwrap()), 560);
        c[0x14..0x18].copy_from_slice(&0xffff_fff8u32.to_le_bytes());
        w26a20_md5_update_one(&mut c);
        assert_eq!(u32::from_le_bytes(c[0x18..0x1c].try_into().unwrap()), 1);
    }
}
