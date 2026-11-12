#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave19/agent06.rs
// ===========================================================================
// Wave 19 translation batch 06: small container / refcount / stream helpers
// (queue, module `runtime.rs`).  Unresolved callees are closure parameters.

/// `0x00534f70`: releases a range of 8-byte shared-pointer slots.  Each slot
/// holds an optional control block `(use_count, weak_count)`; the use count is
/// decremented and, when the old value was 1, `dispose(i)` runs (vtable +8)
/// and the weak count is decremented; when that old value was 1, `destroy(i)`
/// runs (vtable +0xc).
pub fn w19a06_release_range(
    ctrls: &mut [Option<(i32, i32)>],
    mut dispose: impl FnMut(usize),
    mut destroy: impl FnMut(usize),
) {
    for (i, slot) in ctrls.iter_mut().enumerate() {
        if let Some((use_c, weak_c)) = slot {
            let old = *use_c;
            *use_c = use_c.wrapping_sub(1);
            if old == 1 {
                dispose(i);
                let old_w = *weak_c;
                *weak_c = weak_c.wrapping_sub(1);
                if old_w == 1 {
                    destroy(i);
                }
            }
        }
    }
}

/// `0x0053edb0`: mixes the words at +0x20/+0x24/+0x28 (murmur3 finalizer
/// constants) with `inner`, the result of the opaque hash call over words
/// 0 and 1 (`FUN_00c62800(w0, w1, 0)`).
pub fn w19a06_hash_key(w8: u32, w9: u32, w10: u32, inner: u32) -> u32 {
    const C1: u32 = 0x85eb_ca6b; // -0x7a143595
    const C2: u32 = 0xc2b2_ae35; // -0x3d4d51cb
    const C3: u32 = 0x8565_5c6a; // -0x7a9aa396
    let mut a = ((w10 >> 16) ^ w10).wrapping_mul(C1);
    a = ((a >> 13) ^ a).wrapping_mul(C2);
    let mut b = ((w9 >> 16) ^ w9).wrapping_mul(C1);
    b = (b >> 13) ^ b;
    let mut d = ((w8 >> 16) ^ w8).wrapping_mul(C1);
    d = ((d >> 13) ^ d).wrapping_mul(C2);
    let x = ((b.wrapping_mul(C2) & 0xffff_7fff) ^ (a >> 1)) >> 14;
    let y = (a & 0x3fff_ffff) ^ (d >> 16) ^ d;
    b.wrapping_mul(C3) ^ ((inner ^ x) >> 1) ^ (y & 0x7fff_ffff)
}

/// `0x0053f3a0`: walks a singly linked chain (first word of each node is the
/// next pointer) looking for a node whose key (ptr +4, len +8) equals `key`
/// byte for byte and whose sub-record at +0x20 satisfies `sub_eq`.  Returns
/// the link that points at the match (`None` = the starting link `param_1`,
/// `Some(i)` = the next-link inside node `i`), or the last link when the
/// chain ends.
pub fn w19a06_find_link<'a>(
    head: Option<usize>,
    key: &[u8],
    next: impl Fn(usize) -> Option<usize>,
    node_key: impl Fn(usize) -> &'a [u8],
    mut sub_eq: impl FnMut(usize) -> bool,
) -> Option<usize> {
    let mut link: Option<usize> = None;
    let mut cur = head;
    while let Some(n) = cur {
        // memcmp over the shorter length, then length difference
        if node_key(n) == key && sub_eq(n) {
            return link;
        }
        link = Some(n);
        cur = next(n);
    }
    link
}

/// `0x0053f470`: releases the buffer at +0xc through `free` (when non-null)
/// and zeroes the pointer (+0xc) and the length (+0x10).
pub fn w19a06_clear_buffer(ptr: &mut u32, len: &mut u32, free: impl FnOnce(u32)) {
    if *ptr != 0 {
        free(*ptr);
    }
    *len = 0;
    *ptr = 0;
}

/// `0x0053f540`: bucket-count check of a hash table `{buckets, table_ptr,
/// size, max_load: f32}`.  `count` is the result of the opaque size call
/// (`FUN_00c86e10`).  Returns the power-of-two (>= 8) bucket count wanted and
/// whether the table is already fine (`true` = early return; `false` = the
/// original calls the non-returning rehash routine).
pub fn w19a06_check_buckets(
    buckets: u32,
    table_ptr: u32,
    size: u32,
    max_load: f32,
    count: u32,
    requested: u32,
) -> (u32, bool) {
    // DAT_00d33180 is { 0.0, 4294967296.0 }, indexed by the sign bit
    let adj = |v: u32| {
        if (v as i32) < 0 {
            4_294_967_296.0_f64
        } else {
            0.0
        }
    };
    let lhs = ((size as i32 as f64) + adj(size)) as f32;
    let rhs = (((count as i32 as f64) + adj(count)) as f32) * max_load;
    let mut want = count.wrapping_add(1);
    if lhs <= rhs {
        want = count;
    }
    if want < requested {
        want = requested;
    }
    let mut n: u32 = 8;
    if want > 8 {
        loop {
            n = n.wrapping_mul(2);
            if n >= want {
                break;
            }
        }
    }
    (n, n == buckets && table_ptr != 0)
}

/// `0x0053f900`: initialises a six-word record: `[1]=a`, `[2]=b`, `[0]=0`,
/// `[3]=c`, `[4]=0`, `[5]=0x20000`.
pub fn w19a06_init_record(rec: &mut [u32; 6], a: u32, b: u32, c: u32) {
    rec[1] = a;
    rec[2] = b;
    rec[0] = 0;
    rec[3] = c;
    rec[4] = 0;
    rec[5] = 0x20000;
}

/// `0x0053fa50`: equality of two records `{str, w1, w2, sub(+0xc), w4}`.
/// `sub_eq` is the opaque comparison of the sub-records (`FUN_0061f4b0`);
/// `str_cmp` is the opaque string comparison tail call, used only when both
/// strings are present (its result is taken as the answer).  Two absent
/// strings are equal, exactly one absent is unequal.
pub fn w19a06_record_eq(
    a: (Option<&str>, u32, u32, u32),
    b: (Option<&str>, u32, u32, u32),
    sub_eq: impl FnOnce() -> bool,
    str_cmp: impl FnOnce(&str, &str) -> bool,
) -> bool {
    if a.1 != b.1 || a.2 != b.2 || !sub_eq() || a.3 != b.3 {
        return false;
    }
    match (a.0, b.0) {
        (None, None) => true,
        (Some(x), Some(y)) => str_cmp(x, y),
        _ => false,
    }
}

/// `0x00549640`: move-constructs the 6-word (0x18 byte) elements of `src`
/// into `dst`: words 0/1 and the low 31 bits of word 2 move over (source
/// words 0/1 cleared, source word 2 keeps only its top bit), words 3..5 are
/// copied.  An element whose word 2 has the top bit set aborts in the
/// original (`Err(index)`).  Returns the number of words written.
pub fn w19a06_move_range(src: &mut [u32], dst: &mut [u32]) -> Result<usize, usize> {
    let n = src.len() / 6;
    for i in 0..n {
        let s = i * 6;
        let d = i * 6;
        dst[d] = 0;
        dst[d + 1] = 0;
        dst[d + 2] = 0;
        if (src[s + 2] as i32) < 0 {
            return Err(i);
        }
        dst[d] = src[s];
        dst[d + 1] = src[s + 1];
        dst[d + 2] = src[s + 2] & 0x7fff_ffff;
        src[s + 2] &= 0x8000_0000;
        src[s] = 0;
        src[s + 1] = 0;
        dst[d + 3] = src[s + 3];
        dst[d + 4] = src[s + 4];
        dst[d + 5] = src[s + 5];
    }
    Ok(n * 6)
}

/// `0x00549950`: ensures 8 bytes are readable at `pos`.  `read(offset, len)`
/// is the stream vtable +4 call returning bytes read.  Returns `Ok(pos)` or
/// `Err("Premature end of stream")`.
pub fn w19a06_require_eight(
    pos: u32,
    mut read: impl FnMut(u32, u32) -> u32,
) -> Result<u32, &'static str> {
    let mut got = read(pos, 8);
    while got & 7 != 0 {
        let r = read(got.wrapping_add(pos), 8u32.wrapping_sub(got));
        if r == 0 {
            return Err("Premature end of stream");
        }
        got = got.wrapping_add(r);
    }
    if got > 7 {
        Ok(pos)
    } else {
        Err("Premature end of stream")
    }
}

/// `0x00549ac0`: move-constructs a 3-word `{ptr, aux, len|flag}` handle:
/// `dst` takes words 0/1 and the low 31 bits of word 2; the source keeps only
/// the top bit of word 2 and has words 0/1 cleared.  A source with the top
/// bit set aborts in the original (`Err`, `dst` left zeroed).
pub fn w19a06_move_handle3(dst: &mut [u32; 3], src: &mut [u32; 3]) -> Result<(), ()> {
    *dst = [0; 3];
    if (src[2] as i32) < 0 {
        return Err(());
    }
    dst[0] = src[0];
    dst[1] = src[1];
    dst[2] = src[2] & 0x7fff_ffff;
    src[2] &= 0x8000_0000;
    src[0] = 0;
    src[1] = 0;
    Ok(())
}

/// `0x00549cd0`: like the 3-word move above, then copies words 3..5 as well
/// (6-word record).  On abort only `dst[1]` is (re)zeroed.
pub fn w19a06_move_handle6(dst: &mut [u32; 6], src: &mut [u32; 6]) -> Result<(), ()> {
    *dst = [0; 6];
    if (src[2] as i32) < 0 {
        dst[1] = 0;
        return Err(());
    }
    dst[0] = src[0];
    dst[1] = src[1];
    dst[2] = src[2] & 0x7fff_ffff;
    src[2] &= 0x8000_0000;
    src[0] = 0;
    src[1] = 0;
    dst[3] = src[3];
    dst[4] = src[4];
    dst[5] = src[5];
    Ok(())
}

#[cfg(test)]
mod tests_wave19_agent06 {
    use super::*;

    #[test]
    fn release_counts() {
        let mut c = [Some((1, 1)), Some((2, 1)), None, Some((1, 2))];
        let (mut d, mut x) = (vec![], vec![]);
        w19a06_release_range(&mut c, |i| d.push(i), |i| x.push(i));
        assert_eq!(d, vec![0, 3]);
        assert_eq!(x, vec![0]);
        assert_eq!(c[1], Some((1, 1)));
        assert_eq!(c[3], Some((0, 1)));
    }

    #[test]
    fn hash_zero_and_dependence() {
        assert_eq!(w19a06_hash_key(0, 0, 0, 0), 0);
        assert_eq!(w19a06_hash_key(0, 0, 0, 4), 2);
        assert_ne!(w19a06_hash_key(1, 0, 0, 0), w19a06_hash_key(2, 0, 0, 0));
        assert_eq!(
            w19a06_hash_key(1, 2, 3, 4) >> 31,
            w19a06_hash_key(1, 2, 3, 4) >> 31
        );
    }

    #[test]
    fn find_link_chain() {
        let keys: [&[u8]; 3] = [b"aa", b"bb", b"cc"];
        let nx = |i: usize| if i < 2 { Some(i + 1) } else { None };
        let r = w19a06_find_link(Some(0), b"bb", nx, |i| keys[i], |_| true);
        assert_eq!(r, Some(0));
        let r = w19a06_find_link(Some(0), b"aa", nx, |i| keys[i], |_| true);
        assert_eq!(r, None);
        let r = w19a06_find_link(Some(0), b"zz", nx, |i| keys[i], |_| true);
        assert_eq!(r, Some(2));
        let r = w19a06_find_link(Some(0), b"bb", nx, |i| keys[i], |_| false);
        assert_eq!(r, Some(2));
        assert_eq!(
            w19a06_find_link(None, b"a", nx, |i| keys[i], |_| true),
            None
        );
    }

    #[test]
    fn clear_buffer() {
        let (mut p, mut l, mut freed) = (5u32, 9u32, 0);
        w19a06_clear_buffer(&mut p, &mut l, |v| freed = v);
        assert_eq!((p, l, freed), (0, 0, 5));
        let mut called = false;
        w19a06_clear_buffer(&mut p, &mut l, |_| called = true);
        assert!(!called);
    }

    #[test]
    fn buckets() {
        assert_eq!(w19a06_check_buckets(8, 1, 1, 1.0, 4, 0), (8, true));
        assert_eq!(w19a06_check_buckets(8, 0, 1, 1.0, 4, 0), (8, false));
        // size above load: count+1 = 9 -> 16
        assert_eq!(w19a06_check_buckets(8, 1, 10, 1.0, 8, 0), (16, false));
        assert_eq!(w19a06_check_buckets(8, 1, 1, 1.0, 4, 100), (128, false));
    }

    #[test]
    fn init_and_eq() {
        let mut r = [9; 6];
        w19a06_init_record(&mut r, 1, 2, 3);
        assert_eq!(r, [0, 1, 2, 3, 0, 0x20000]);
        let a = (None, 1, 2, 4);
        assert!(w19a06_record_eq(a, a, || true, |_, _| false));
        assert!(!w19a06_record_eq(a, (None, 1, 2, 5), || true, |_, _| true));
        assert!(!w19a06_record_eq(a, a, || false, |_, _| true));
        assert!(!w19a06_record_eq(
            a,
            (Some("x"), 1, 2, 4),
            || true,
            |_, _| true
        ));
        let s = (Some("x"), 1, 2, 4);
        assert!(w19a06_record_eq(s, s, || true, |p, q| p == q));
    }

    #[test]
    fn move_range_and_handles() {
        let mut s = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let mut d = [0; 12];
        assert_eq!(w19a06_move_range(&mut s, &mut d), Ok(12));
        assert_eq!(d, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        assert_eq!(&s[..3], &[0, 0, 0]);
        let mut s = [1, 2, 0x8000_0000, 0, 0, 0];
        let mut d = [0; 6];
        assert_eq!(w19a06_move_range(&mut s, &mut d), Err(0));

        let mut s3 = [1, 2, 7];
        let mut d3 = [9; 3];
        assert!(w19a06_move_handle3(&mut d3, &mut s3).is_ok());
        assert_eq!((d3, s3), ([1, 2, 7], [0, 0, 0]));
        let mut bad = [1, 2, 0x8000_0001];
        assert!(w19a06_move_handle3(&mut d3, &mut bad).is_err());
        assert_eq!(d3, [0; 3]);

        let mut s6 = [1, 2, 3, 4, 5, 6];
        let mut d6 = [0; 6];
        assert!(w19a06_move_handle6(&mut d6, &mut s6).is_ok());
        assert_eq!(d6, [1, 2, 3, 4, 5, 6]);
        assert_eq!(s6[..3], [0, 0, 0]);
        let mut bad6 = [1, 2, 0x8000_0000, 0, 0, 0];
        assert!(w19a06_move_handle6(&mut d6, &mut bad6).is_err());
    }

    #[test]
    fn require_eight() {
        assert_eq!(w19a06_require_eight(100, |_, _| 8), Ok(100));
        assert!(w19a06_require_eight(100, |_, _| 0).is_err());
        let mut calls = vec![];
        let mut seq = vec![4u32, 4].into_iter();
        let r = w19a06_require_eight(10, |o, l| {
            calls.push((o, l));
            seq.next().unwrap()
        });
        assert_eq!(r, Ok(10));
        assert_eq!(calls, vec![(10, 8), (14, 4)]);
    }
}
