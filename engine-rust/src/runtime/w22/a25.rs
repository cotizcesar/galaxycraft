#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent25.rs
// ===========================================================================
// Wave 22 agent 25: resource-record copy-on-write and lookup helpers.

/// Flag bit 11 (`0x800`) of the record flags word at record offset 8.
pub const W22A25_FLAG_OWNED: u32 = 0x800;

/// A `(start, len)` span (record offset 4 and 0x14 of a block descriptor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W22a25Span {
    pub start: u32,
    pub len: u32,
}

/// `0x006ad270`: if the owned flag (0x800) is clear, copies 600 dwords
/// (0x960 bytes) from `src` into a fresh buffer and sets the flag.
pub fn w22a25_clone_960(flags: &mut u32, src: &[u32]) -> Option<Vec<u32>> {
    w22a25_clone_dwords(flags, src, 600)
}

/// `0x006ad2b0`: same as the 600-dword variant but copies 0x4f dwords (0x13c bytes).
pub fn w22a25_clone_13c(flags: &mut u32, src: &[u32]) -> Option<Vec<u32>> {
    w22a25_clone_dwords(flags, src, 0x4f)
}

fn w22a25_clone_dwords(flags: &mut u32, src: &[u32], n: usize) -> Option<Vec<u32>> {
    if (*flags >> 0xb) & 1 != 0 {
        return None;
    }
    let mut out = vec![0u32; n];
    let k = n.min(src.len());
    out[..k].copy_from_slice(&src[..k]);
    *flags |= W22A25_FLAG_OWNED;
    Some(out)
}

fn w22a25_in_span(s: Option<W22a25Span>, p: u32) -> bool {
    match s {
        Some(s) => p >= s.start && (s.len as u64 + s.start as u64) > p as u64,
        None => false,
    }
}

/// `0x006ae590`: tries the two cached spans (slots 0x50 and 0x54); when
/// `p` lies in neither, runs the slow lookup `fallback` (0x6a_7d50).
/// Returns the fallback result, or `None` when a cached span matched.
pub fn w22a25_lookup_cached<T>(
    slot50: Option<W22a25Span>,
    slot54: Option<W22a25Span>,
    p: u32,
    fallback: impl FnOnce(u32) -> T,
) -> Option<T> {
    if w22a25_in_span(slot50, p) || w22a25_in_span(slot54, p) {
        None
    } else {
        Some(fallback(p))
    }
}

/// Record seen by the typed lookups: `key` (offset 4), `flags` (offset 8), `value` (offset 0xc).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W22a25Rec {
    pub key: u32,
    pub flags: u32,
    pub value: u32,
}

fn w22a25_typed(rec: Option<W22a25Rec>, p: u32, tag: u32) -> u32 {
    match rec {
        Some(r) if r.flags & 0xff00_0000 == tag && r.key == p => r.value,
        _ => 0,
    }
}

/// `0x006ae5e0`: record lookup requiring type byte 0x3e and exact key; else 0.
pub fn w22a25_typed_3e(rec: Option<W22a25Rec>, p: u32) -> u32 {
    w22a25_typed(rec, p, 0x3e00_0000)
}

/// `0x006ae660`: as above with type byte 0x34.
pub fn w22a25_typed_34(rec: Option<W22a25Rec>, p: u32) -> u32 {
    w22a25_typed(rec, p, 0x3400_0000)
}

fn w22a25_index_map(p: u32, base: u32, count: u32, stride: u32, mul: i32, add: i32) -> i32 {
    if p == 0 {
        return 0;
    }
    let end = base.wrapping_add(count.wrapping_mul(stride));
    if base <= p && p < end {
        ((p - base) as i32 / stride as i32)
            .wrapping_mul(mul)
            .wrapping_add(add)
    } else {
        0
    }
}

/// `0x006ae6e0`: pointer into a 0x14-stride table mapped to `idx * mul + add`, else 0.
pub fn w22a25_map_stride_14(p: u32, base: u32, count: u32, mul: i32, add: i32) -> i32 {
    w22a25_index_map(p, base, count, 0x14, mul, add)
}

/// `0x006ae740`: same with stride 0x28.
pub fn w22a25_map_stride_28(p: u32, base: u32, count: u32, mul: i32, add: i32) -> i32 {
    w22a25_index_map(p, base, count, 0x28, mul, add)
}

/// `0x006ae7a0`: same with stride 0x150.
pub fn w22a25_map_stride_150(p: u32, base: u32, count: u32, mul: i32, add: i32) -> i32 {
    w22a25_index_map(p, base, count, 0x150, mul, add)
}

/// `0x006ae920`: finds the map entry (sorted by key, `tree`) with key exactly
/// `key`, takes its span, requires
/// `span.len <= offset`, resolves
/// `span.start + offset` through `resolve` (0x006ae590) and returns the
/// result plus the address.  Nonzero result required.
pub fn w22a25_resolve_in_block(
    tree: &[(u32, Option<W22a25Span>)],
    key: u32,
    offset: u32,
    resolve: impl FnOnce(u32) -> i32,
) -> Option<(i32, u32)> {
    let idx = tree.partition_point(|(k, _)| *k < key);
    let (k, span) = *tree.get(idx)?;
    if k > key {
        return None;
    }
    let span = span?;
    if span.len > offset {
        return None;
    }
    let addr = span.start.wrapping_add(offset);
    let r = resolve(addr);
    if r != 0 { Some((r, addr)) } else { None }
}

#[cfg(test)]
mod tests_wave22_agent25 {
    use super::*;

    #[test]
    fn clone_flags() {
        let mut f = 0;
        let src = vec![7u32; 600];
        let v = w22a25_clone_960(&mut f, &src).unwrap();
        assert_eq!(v.len(), 600);
        assert_eq!(f, 0x800);
        assert!(w22a25_clone_960(&mut f, &src).is_none());
        let mut g = 1;
        assert_eq!(w22a25_clone_13c(&mut g, &src).unwrap().len(), 0x4f);
        assert_eq!(g, 0x801);
    }

    #[test]
    fn cached_span() {
        let s = Some(W22a25Span { start: 10, len: 5 });
        assert_eq!(w22a25_lookup_cached(s, None, 14, |p| p), None);
        assert_eq!(w22a25_lookup_cached(s, None, 15, |p| p), Some(15));
        assert_eq!(w22a25_lookup_cached(None, s, 9, |p| p), Some(9));
    }

    #[test]
    fn typed() {
        let r = W22a25Rec {
            key: 5,
            flags: 0x3e00_0001,
            value: 99,
        };
        assert_eq!(w22a25_typed_3e(Some(r), 5), 99);
        assert_eq!(w22a25_typed_3e(Some(r), 6), 0);
        assert_eq!(w22a25_typed_34(Some(r), 5), 0);
        let r2 = W22a25Rec {
            flags: 0x3400_0000,
            ..r
        };
        assert_eq!(w22a25_typed_34(Some(r2), 5), 99);
        assert_eq!(w22a25_typed_34(None, 5), 0);
    }

    #[test]
    fn maps() {
        assert_eq!(w22a25_map_stride_14(0, 100, 4, 3, 1), 0);
        assert_eq!(w22a25_map_stride_14(100 + 0x28, 100, 4, 3, 1), 7);
        assert_eq!(w22a25_map_stride_14(100 + 0x50, 100, 4, 3, 1), 0);
        assert_eq!(w22a25_map_stride_28(100 + 0x28, 100, 2, 2, 0), 2);
        assert_eq!(w22a25_map_stride_150(100 + 0x150, 100, 2, 5, 1), 6);
        assert_eq!(w22a25_map_stride_150(99, 100, 2, 5, 1), 0);
    }

    #[test]
    fn resolve() {
        let t = [
            (1, Some(W22a25Span { start: 100, len: 0 })),
            (3, Some(W22a25Span { start: 200, len: 4 })),
        ];
        assert_eq!(
            w22a25_resolve_in_block(&t, 1, 8, |a| a as i32),
            Some((108, 108))
        );
        assert_eq!(w22a25_resolve_in_block(&t, 2, 8, |a| a as i32), None);
        assert_eq!(w22a25_resolve_in_block(&t, 3, 2, |a| a as i32), None);
        assert_eq!(w22a25_resolve_in_block(&t, 1, 8, |_| 0), None);
    }
}
