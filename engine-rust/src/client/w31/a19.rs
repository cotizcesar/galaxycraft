#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent19.rs
// ===========================================================================
// Wave 31 translation batch 19: draw buffer sizing, string-key lookups.

/// `0x00a986c0`: byte-count helper: `(count / elems(mode)) * k` where `k` is
/// 6 for mode 1 and -1 otherwise.  `elems` models `FUN_00a98750`.
pub fn w31a19_index_budget(mode: i32, count: u32, elems: impl Fn(i32) -> u32) -> i32 {
    let k: i32 = if mode == 1 { 6 } else { -1 };
    let e = elems(mode);
    ((count / e) as i32).wrapping_mul(k)
}

/// `0x00a98610`: byte address of element `index` of a vertex buffer.
/// `buf_base` is `*(this+8)`, `offset` is `*(param_1+0x2c)` (-1 gives 0),
/// `mode` is `*(param_1+0x24)`; `flags` are the vertex flag bits.
pub fn w31a19_vertex_address(
    buf_base: i32,
    offset: i32,
    mode: i32,
    index: u32,
    flags: u32,
    elems: impl Fn(i32) -> u32,
) -> i32 {
    if offset == -1 {
        return 0;
    }
    let count = match mode {
        4 => {
            if index > 1 {
                index.wrapping_add(2)
            } else {
                0
            }
        }
        5 => index.wrapping_mul(3),
        m => elems(m).wrapping_mul(index),
    };
    let mut s = (flags & 1) * 8;
    if flags & 2 != 0 {
        s += 8;
    }
    if flags & 4 != 0 {
        s += 8;
    }
    if flags & 8 != 0 {
        s += 8;
    }
    if flags & 0x10 != 0 {
        s += 4;
    }
    if flags & 0x20 != 0 {
        s += 8;
    }
    buf_base
        .wrapping_add(offset)
        .wrapping_add(((s >> 2).wrapping_mul(count).wrapping_mul(4)) as i32)
}

/// Fields touched by `0x00a97a30` and `0x00a98c20`.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct W31a19Buffers {
    pub width: u32,
    pub width_double: bool,
    pub width_dirty: bool,
    pub height: u32,
    pub height_double: bool,
    pub height_dirty: bool,
}

/// `0x00a97a30`: doubles width/height when their double flags are set,
/// marks both dirty, and returns the surface size `(width*4, height*2)`.
pub fn w31a19_resize_buffers(b: &mut W31a19Buffers) -> (u32, u32) {
    if b.width_double {
        b.width = b.width.wrapping_mul(2);
    }
    b.width_dirty = true;
    if b.height_double {
        b.height = b.height.wrapping_mul(2);
    }
    b.height_dirty = true;
    (b.width.wrapping_mul(4), b.height.wrapping_mul(2))
}

/// `0x00a98c20`: doubles `size` when `double` is set, flags dirty, and
/// returns the allocation size `(size & 0xfffffff) << 2`.
pub fn w31a19_grow_buffer(size: &mut u32, double: bool, dirty: &mut bool) -> u32 {
    if double {
        *size = size.wrapping_mul(2);
    }
    *dirty = true;
    (*size & 0x0fff_ffff) << 2
}

/// `0x00a96720`: xorshift32 random float between `a` and `b`, scaled by the
/// global range `[lo, hi]`.  `state` must already be non-zero (the seed
/// fallback of 1 is applied when the lazily seeded value is 0).
pub fn w31a19_random_between(a: f32, b: f32, state: &mut u32, lo: f32, hi: f32) -> f32 {
    let (mn, mx) = if b < a { (b, a) } else { (a, b) };
    if *state == 0 {
        *state = 1;
    }
    let old = *state;
    let mut x = old << 13 ^ old;
    x ^= x >> 17;
    *state = x ^ (x << 5);
    let r = ((state.wrapping_add(old) & 0x7f_ffff) as f32 + 0.0) * 1.192_092_9e-7;
    (r * (hi - lo) + lo) * (mx - mn) + mn
}

/// `0x00ac67b0`: element-wise equality of two 28-byte-record vectors whose
/// leading (ptr,len) string is compared with `FUN_0053a3e0` against the
/// string at the same offset in the other vector.
pub fn w31a19_records_equal(a: &[(Vec<u8>, Vec<u8>)], b: &[(Vec<u8>, Vec<u8>)]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).all(|(x, y)| {
        crate::client::w20a01_bounded_compare(
            &x.0,
            x.0.len() as u32,
            0,
            x.0.len() as u32,
            &y.0,
            y.0.len() as u32,
        ) == 0
    })
}

/// `0x00ac7ac0`: true when the string equals the stored name.
pub fn w31a19_name_matches(name: &[u8], other: &[u8]) -> bool {
    crate::client::w20a01_bounded_compare(
        name,
        name.len() as u32,
        0,
        name.len() as u32,
        other,
        other.len() as u32,
    ) == 0
}

/// One node of the intrusive list walked by `0x00ac7030`.
#[derive(Debug, Clone)]
pub struct W31a19Entry {
    pub name: Vec<u8>,
    pub flag: bool,
}

/// `0x00ac7030`: first entry whose name equals `wanted` and whose flag is set; with
/// `None` the first entry with its flag set (offset 0x40).  Returns the index.
pub fn w31a19_find_entry(entries: &[W31a19Entry], wanted: Option<&[u8]>) -> Option<usize> {
    entries.iter().position(|e| match wanted {
        None => e.flag,
        Some(w) => w31a19_name_matches(&e.name, w) && e.flag,
    })
}

/// Tree node used by `0x00b64c40` (fields +4 left, +8 right, +0x10 key).
#[derive(Debug, Clone)]
pub struct W31a19Node {
    pub ge: Option<usize>,
    pub lt: Option<usize>,
    pub key: Vec<u8>,
}

/// `0x00b64c40`: lower-bound search in a string-keyed tree.  Compares by
/// bytes over the shorter length, then by length; returns the last node
/// not less than `key` and only when `key` compares >= its key.
pub fn w31a19_tree_find(nodes: &[W31a19Node], root: Option<usize>, key: &[u8]) -> Option<usize> {
    let mut cur = root;
    let mut cand: Option<usize> = None;
    while let Some(i) = cur {
        let n = &nodes[i];
        let m = n.key.len().min(key.len());
        let mut c: i64 = match n.key[..m].cmp(&key[..m]) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Equal => 0,
        };
        if c == 0 {
            c = n.key.len() as i64 - key.len() as i64;
        }
        if c < 0 {
            cur = n.lt;
        } else {
            cand = Some(i);
            cur = n.ge;
        }
    }
    let i = cand?;
    let r = crate::client::w20a01_bounded_compare(
        key,
        key.len() as u32,
        0,
        key.len() as u32,
        &nodes[i].key,
        nodes[i].key.len() as u32,
    );
    if r >= 0 { Some(i) } else { None }
}

#[cfg(test)]
mod tests_wave31_agent19 {
    use super::*;

    fn el(m: i32) -> u32 {
        if m == 2 { 10 } else { 4 }
    }

    #[test]
    fn budget_and_address() {
        assert_eq!(w31a19_index_budget(1, 8, el), 12);
        assert_eq!(w31a19_index_budget(0, 8, el), -2);
        assert_eq!(w31a19_vertex_address(100, -1, 0, 1, 0, el), 0);
        assert_eq!(
            w31a19_vertex_address(100, 4, 5, 2, 1, el),
            100 + 4 + 2 * 3 * 4 * 2
        );
        assert_eq!(w31a19_vertex_address(0, 0, 4, 1, 1, el), 0);
    }

    #[test]
    fn buffers() {
        let mut b = W31a19Buffers {
            width: 8,
            width_double: true,
            height: 4,
            ..Default::default()
        };
        assert_eq!(w31a19_resize_buffers(&mut b), (64, 8));
        let (mut s, mut d) = (0x1000_0001u32, false);
        assert_eq!(w31a19_grow_buffer(&mut s, true, &mut d), 8);
        assert!(d);
    }

    #[test]
    fn random_and_names() {
        let mut st = 1;
        let v = w31a19_random_between(5.0, 1.0, &mut st, 0.0, 1.0);
        assert!((1.0..=5.0).contains(&v));
        assert_ne!(st, 1);
        assert!(w31a19_name_matches(b"ab", b"ab"));
        assert!(!w31a19_name_matches(b"ab", b"abc"));
        let a = vec![(b"x".to_vec(), vec![])];
        assert!(w31a19_records_equal(&a, &a.clone()));
        assert!(!w31a19_records_equal(&a, &[]));
    }

    #[test]
    fn find_and_tree() {
        let es = vec![
            W31a19Entry {
                name: b"a".to_vec(),
                flag: false,
            },
            W31a19Entry {
                name: b"b".to_vec(),
                flag: false,
            },
        ];
        assert_eq!(w31a19_find_entry(&es, Some(b"b")), None);
        assert_eq!(w31a19_find_entry(&es, None), None);
        let nodes = vec![
            W31a19Node {
                ge: Some(1),
                lt: None,
                key: b"m".to_vec(),
            },
            W31a19Node {
                ge: None,
                lt: None,
                key: b"c".to_vec(),
            },
        ];
        assert_eq!(w31a19_tree_find(&nodes, Some(0), b"m"), Some(0));
        assert_eq!(w31a19_tree_find(&nodes, Some(0), b"a"), None);
    }
}
