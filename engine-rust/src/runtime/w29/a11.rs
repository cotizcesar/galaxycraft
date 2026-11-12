#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent11.rs
// ===========================================================================
// Wave 29 agent 11: small container helpers next to the CDN-config code.

/// `0x00acbbe0`: small-string/vector reserve policy.  `flags` is the packed
/// capacity word (low 31 bits capacity, top bit a heap flag).  When the
/// capacity is below `wanted`, the new capacity is `max(wanted, cap + cap/2)`
/// (`(flags >> 1) & 0x3fffffff` is the growth step) and is merged into the low
/// 31 bits keeping the top bit.  Returns `Some(new_flags)` when a
/// reallocation is required, `None` otherwise.
pub fn w29a11_reserve_policy(flags: u32, wanted: u32) -> Option<u32> {
    let cap = flags & 0x7fff_ffff;
    if cap >= wanted {
        return None;
    }
    let mut new_cap = ((flags >> 1) & 0x3fff_ffff).wrapping_add(cap);
    if new_cap < wanted {
        new_cap = wanted;
    }
    Some((((flags ^ new_cap) & 0x7fff_ffff) ^ flags) as u32)
}

/// `0x00acc7e0`: stores `new` into the slot and returns the previous value
/// (the original frees it when non-zero; the caller decides).
pub fn w29a11_replace_ptr(slot: &mut u32, new: u32) -> Option<u32> {
    let old = std::mem::replace(slot, new);
    if old != 0 { Some(old) } else { None }
}

#[cfg(test)]
mod tests_wave29_agent11 {
    use super::*;

    #[test]
    fn reserve_policy() {
        assert_eq!(w29a11_reserve_policy(0x8000_0010, 8), None);
        // cap 16, step 0x4000_0008 & 0x3fff_ffff = 8 -> 24 >= 20
        assert_eq!(w29a11_reserve_policy(0x8000_0010, 20), Some(0x8000_0018));
        assert_eq!(w29a11_reserve_policy(0x10, 100), Some(100));
    }

    #[test]
    fn replace_ptr() {
        let mut s = 0;
        assert_eq!(w29a11_replace_ptr(&mut s, 5), None);
        assert_eq!(w29a11_replace_ptr(&mut s, 7), Some(5));
        assert_eq!(s, 7);
    }
}
