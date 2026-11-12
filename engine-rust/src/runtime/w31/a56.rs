#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent56.rs
// ===========================================================================
// Wave 31 batch 56: small string, refcount and word-vector helpers.

/// Little word-vector header used by 0x0092d6a0 and neighbours: `len`
/// words stored in `words`, with `base` the first live word index.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W31a56WordVec {
    pub words: Vec<u32>,
    pub base: u32,
}

/// `0x00924180`: scans `hay` for the first position where `cmp(hay[i..],
/// needle)` (a strncmp over `needle.len()` bytes, `FUN_00c94358`) returns 0.
/// Stops at the terminating NUL of `hay`; returns the byte offset or `None`.
pub fn w31a56_find_substr(
    hay: &[u8],
    needle: &[u8],
    cmp: impl Fn(&[u8], &[u8], usize) -> i32,
) -> Option<usize> {
    let n = needle.iter().position(|&b| b == 0).unwrap_or(needle.len());
    let mut i = 0;
    while i < hay.len() && hay[i] != 0 {
        if cmp(&hay[i..], needle, n) == 0 {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Which call `0x00924240` makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W31a56Dispatch {
    /// flag 0x10 set: call FUN_0094bf00 with no arguments.
    Free,
    /// otherwise FUN_0094be90(p1, p2, mode, 1).
    Alloc { mode: u32 },
}

/// `0x00924240`: flag bit 0x10 selects the free path, otherwise the alloc
/// path with mode `(flags & 1) << 2`.
pub fn w31a56_dispatch(flags: u32) -> W31a56Dispatch {
    if flags & 0x10 != 0 {
        W31a56Dispatch::Free
    } else {
        W31a56Dispatch::Alloc {
            mode: (flags & 1) << 2,
        }
    }
}

/// `0x00924650`: drops one reference from the count at `obj+0x10`; returns
/// true when it reached zero and the destructor (FUN_00924e00) must run.
pub fn w31a56_release_ref(count: &mut i32) -> bool {
    *count = count.wrapping_sub(1);
    *count == 0
}

/// `0x00927be0`: walks a chain of nodes, decrementing each refcount and
/// invoking `destroy` on those hitting zero; the next link is read before
/// the destroy call.  `nodes[i] = (refcount, next)`.
pub fn w31a56_release_chain(
    nodes: &mut [(i32, Option<usize>)],
    start: usize,
    mut destroy: impl FnMut(usize),
) {
    let mut cur = Some(start);
    while let Some(i) = cur {
        nodes[i].0 = nodes[i].0.wrapping_sub(1);
        let next = nodes[i].1;
        if nodes[i].0 == 0 {
            destroy(i);
        }
        cur = next;
    }
}

/// `0x00925930`: returns the header word when it is non-zero and even,
/// otherwise 0.  The second value says whether the explicit-argument path
/// ran, in which case FUN_00924e00(1) is called afterwards.
pub fn w31a56_even_header(word: u32, explicit_arg: bool) -> (u32, bool) {
    let v = if word != 0 && word & 1 == 0 { word } else { 0 };
    (v, explicit_arg)
}

/// `0x0092d6a0`: assignment of one word vector to another; resizes to the
/// source length when different, copies the words, then copies `base`.
pub fn w31a56_wordvec_assign(dst: &mut W31a56WordVec, src: &W31a56WordVec) {
    if !std::ptr::eq(dst, src) {
        dst.words.clear();
        dst.words.extend_from_slice(&src.words);
    }
    dst.base = src.base;
}

/// `0x0092d780`: word at `base + index`, or 0 when out of range.
pub fn w31a56_wordvec_get(v: &W31a56WordVec, index: u32) -> u32 {
    let i = v.base.wrapping_add(index);
    if (i as usize) < v.words.len() {
        v.words[i as usize]
    } else {
        0
    }
}

/// `0x0092ed00`: appends the little-endian bytes of the live words to
/// `out`, skipping zero bytes that belong to the last
/// word.
pub fn w31a56_wordvec_to_bytes(out: &mut Vec<u8>, v: &W31a56WordVec) {
    let len = v.words.len() as u32;
    let base = v.base;
    let count = len.wrapping_sub(base) & 0x3fff_ffff;
    if count == 0 {
        return;
    }
    let mut k: u32 = 0;
    loop {
        let wi = base.wrapping_add(k >> 2);
        let w = if wi < len { v.words[wi as usize] } else { 0 };
        let b = w >> ((k & 3) * 8);
        if b != 0 || (k >> 2) + 1 < len.wrapping_sub(base) {
            out.push(b as u8);
        }
        k += 1;
        if k >= len.wrapping_sub(base).wrapping_mul(4) {
            break;
        }
    }
}

/// `0x0092eda0`: trims zero words from the top while `len != base`.
pub fn w31a56_wordvec_trim(v: &mut W31a56WordVec) {
    while v.words.len() as u32 != v.base {
        match v.words.last() {
            Some(&0) => {
                v.words.pop();
            }
            _ => return,
        }
    }
}

#[cfg(test)]
mod tests_wave31_agent56 {
    use super::*;

    fn cmp(a: &[u8], b: &[u8], n: usize) -> i32 {
        for i in 0..n {
            let x = a.get(i).copied().unwrap_or(0);
            if x != b[i] {
                return 1;
            }
        }
        0
    }

    #[test]
    fn substr() {
        assert_eq!(w31a56_find_substr(b"hello\0", b"ll\0", cmp), Some(2));
        assert_eq!(w31a56_find_substr(b"hello\0", b"zz\0", cmp), None);
        assert_eq!(w31a56_find_substr(b"\0", b"\0", cmp), None);
    }

    #[test]
    fn dispatch_flags() {
        assert_eq!(w31a56_dispatch(0x10), W31a56Dispatch::Free);
        assert_eq!(w31a56_dispatch(1), W31a56Dispatch::Alloc { mode: 4 });
        assert_eq!(w31a56_dispatch(0), W31a56Dispatch::Alloc { mode: 0 });
    }

    #[test]
    fn refs() {
        let mut c = 2;
        assert!(!w31a56_release_ref(&mut c));
        assert!(w31a56_release_ref(&mut c));
        let mut nodes = vec![(1, Some(1)), (2, None)];
        let mut d = vec![];
        w31a56_release_chain(&mut nodes, 0, |i| d.push(i));
        assert_eq!(d, vec![0]);
        assert_eq!(nodes[1].0, 1);
        assert_eq!(w31a56_even_header(4, true), (4, true));
        assert_eq!(w31a56_even_header(3, false), (0, false));
        assert_eq!(w31a56_even_header(0, false), (0, false));
    }

    #[test]
    fn wordvec() {
        let a = W31a56WordVec {
            words: vec![0x0102, 0x0304, 0],
            base: 0,
        };
        let mut b = W31a56WordVec {
            words: vec![9],
            base: 5,
        };
        w31a56_wordvec_assign(&mut b, &a);
        assert_eq!(b, a);
        assert_eq!(w31a56_wordvec_get(&a, 1), 0x0304);
        assert_eq!(w31a56_wordvec_get(&a, 3), 0);
        let mut out = vec![];
        w31a56_wordvec_to_bytes(&mut out, &a);
        assert_eq!(out.len(), 8);
        assert_eq!(&out[..2], &[2, 1]);
        let mut t = a.clone();
        w31a56_wordvec_trim(&mut t);
        assert_eq!(t.words.len(), 2);
    }
}
