#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent07.rs
// ===========================================================================
// Wave 27 agent 07: vector growth-size computations (MSVC std::vector
// reserve-style helpers over `[owner, _, begin, end, cap_end, hint]`).

/// Shared growth rule of the four helpers below.  `begin == 0` means empty
/// storage; with a zero `owner` that path aborts (`None`), otherwise the
/// old capacity is the `hint` field.  Otherwise old capacity grows by 1.5x.
/// Returns the new capacity in bytes (0 means free the buffer).
fn w27a07_grow_bytes(
    owner: u32,
    begin: u32,
    end: u32,
    cap_end: u32,
    hint: u32,
    count: u32,
    stride: u32,
) -> Option<u32> {
    let old = if begin == 0 {
        if owner == 0 {
            return None;
        }
        hint
    } else {
        let cap = cap_end.wrapping_sub(begin);
        cap.wrapping_add(cap.wrapping_add(1) >> 1)
    };
    let need = count
        .wrapping_mul(stride)
        .wrapping_sub(begin)
        .wrapping_add(end);
    Some(need.max(old))
}

/// `0x00966650`: new byte capacity for a 12-byte-element vector grown by
/// `count` elements; `None` when the abort path (`owner == 0`, empty) hits.
pub fn w27a07_grow_bytes_stride_0c(
    owner: u32,
    begin: u32,
    end: u32,
    cap_end: u32,
    hint: u32,
    count: u32,
) -> Option<u32> {
    w27a07_grow_bytes(owner, begin, end, cap_end, hint, count, 0xc)
}

/// `0x009666d0`: same as the 12-byte variant with element stride `0x24`.
pub fn w27a07_grow_bytes_stride_24(
    owner: u32,
    begin: u32,
    end: u32,
    cap_end: u32,
    hint: u32,
    count: u32,
) -> Option<u32> {
    w27a07_grow_bytes(owner, begin, end, cap_end, hint, count, 0x24)
}

/// `0x00966750`: same growth rule with element stride `0x3c`.
pub fn w27a07_grow_bytes_stride_3c(
    owner: u32,
    begin: u32,
    end: u32,
    cap_end: u32,
    hint: u32,
    count: u32,
) -> Option<u32> {
    w27a07_grow_bytes(owner, begin, end, cap_end, hint, count, 0x3c)
}

/// `0x009667e0`: same growth rule with element stride `0x10`.
pub fn w27a07_grow_bytes_stride_10(
    owner: u32,
    begin: u32,
    end: u32,
    cap_end: u32,
    hint: u32,
    count: u32,
) -> Option<u32> {
    w27a07_grow_bytes(owner, begin, end, cap_end, hint, count, 0x10)
}

#[cfg(test)]
mod tests_wave27_agent07 {
    use super::*;

    #[test]
    fn grow_rules() {
        // empty, no owner: abort
        assert_eq!(w27a07_grow_bytes_stride_0c(0, 0, 0, 0, 5, 1), None);
        // empty with owner: max(need, hint); need = 4*12 - 0 + 0
        assert_eq!(w27a07_grow_bytes_stride_0c(1, 0, 0, 0, 5, 4), Some(48));
        assert_eq!(w27a07_grow_bytes_stride_0c(1, 0, 0, 0, 100, 4), Some(100));
        // non-empty: cap=100 -> 150 beats need
        assert_eq!(
            w27a07_grow_bytes_stride_24(1, 1000, 1010, 1100, 0, 1),
            Some(150)
        );
        // need dominates: 10*0x3c - 1000 + 1010 = 610
        assert_eq!(
            w27a07_grow_bytes_stride_3c(1, 1000, 1010, 1100, 0, 10),
            Some(610)
        );
        assert_eq!(w27a07_grow_bytes_stride_10(1, 0, 0, 0, 0, 3), Some(48));
    }
}
