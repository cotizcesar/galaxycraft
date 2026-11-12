#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent35.rs
// ===========================================================================
// Wave 31 translation batch 35: small record/predicate helpers.

/// Result of `0x0075c260`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W31a35LimitResult {
    /// Value stored into the current-limit global.
    pub stored: u32,
    /// True when the reset path (flag set, counter cleared, callback 6) runs.
    pub reset: bool,
}

/// `0x0075c260`: clamps `wanted` to `cap`; when the clamped value is below
/// `current` the reset path is requested.  The clamped value is stored.
pub fn w31a35_clamp_limit(wanted: u32, cap: u32, current: u32) -> W31a35LimitResult {
    let v = wanted.min(cap);
    W31a35LimitResult {
        stored: v,
        reset: v < current,
    }
}

/// `0x00774280`: push-back of a 6-byte element.  `src` indexes an element of
/// the vector itself when it is inside the current length (aliasing safe),
/// otherwise `fallback` is the value copied.
pub fn w31a35_push_back6(v: &mut Vec<[u8; 6]>, src: Option<usize>, fallback: [u8; 6]) {
    let elem = match src {
        Some(i) if i < v.len() => v[i],
        _ => fallback,
    };
    v.push(elem);
}

/// Action chosen by `0x0078dfa0`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum W31a35Dispatch {
    /// Return 0 without calling the base handler.
    Zero,
    /// Call the base handler and return its result.
    Base,
    /// Run the pre-step (two helper calls and an indirect call), then base.
    PreThenBase,
}

/// `0x0078dfa0`: message filter on `(kind as i16, code)`.
pub fn w31a35_filter_message(kind: i16, code: i32) -> W31a35Dispatch {
    match kind {
        0 | 1 | 3 | 4 | 6 | 7 | 9 | 0xf => W31a35Dispatch::Zero,
        0xe if code == 0 => W31a35Dispatch::PreThenBase,
        0xe if code == 4 => W31a35Dispatch::Zero,
        _ => W31a35Dispatch::Base,
    }
}

/// Pre-call chosen by `0x007a0770`: `(helper, arg1, arg2)`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum W31a35Pre {
    /// `FUN_00715ef0(obj, arg, flags)`.
    Notify(u8, u32),
    /// `FUN_007a01e0(field_4c, delta)`.
    Adjust(i32),
}

/// `0x007a0770`: for kind 0xe, code 0 notifies with `4 + (type != 3)` and
/// flag `0x20000000`; code 2 adjusts by `+1`/`-1`.  The base handler always runs after.
pub fn w31a35_pre_dispatch(kind: i16, code: i32, obj_type: i16) -> Option<W31a35Pre> {
    if kind != 0xe {
        return None;
    }
    match code {
        0 => Some(W31a35Pre::Notify((obj_type != 3) as u8 + 4, 0x2000_0000)),
        2 => Some(W31a35Pre::Adjust(if obj_type != 3 { 1 } else { -1 })),
        _ => None,
    }
}

/// `0x007a7660`: true unless the byte is >= 0x80 and outside 0xC2..=0xF4
/// (valid UTF-8 lead byte range check).
pub fn w31a35_is_utf8_lead_or_ascii(b: u8) -> bool {
    !(b > 0x7f && !(0xc2..=0xf4).contains(&b))
}

/// `0x00797200`: stores a pending position record into a player record.
/// `rec` must be at least `0x21c` bytes; returns false when the slot is
/// unused (`rec[0x21a] == 0xff`).  Sets bit 3 of the flags at `0x118`, the
/// u16 at `0x218` and the u32 at `0x214`.
pub fn w31a35_store_pending(rec: &mut [u8], v16: u16, v32: u32) -> bool {
    if rec.len() < 0x21c || rec[0x21a] == 0xff {
        return false;
    }
    let f = u32::from_le_bytes([rec[0x118], rec[0x119], rec[0x11a], rec[0x11b]]) | 8;
    rec[0x118..0x11c].copy_from_slice(&f.to_le_bytes());
    rec[0x218..0x21a].copy_from_slice(&v16.to_le_bytes());
    rec[0x214..0x218].copy_from_slice(&v32.to_le_bytes());
    true
}

/// `0x00797270`: clears the `+0x208` flag of every entry of both lists, then
/// sets it on `target` (an index into `list_b`, when found).
pub fn w31a35_reset_flags(list_a: &mut [u32], list_b: &mut [u32], target: Option<usize>) {
    list_a.iter_mut().for_each(|f| *f = 0);
    list_b.iter_mut().for_each(|f| *f = 0);
    if let Some(i) = target {
        if let Some(f) = list_b.get_mut(i) {
            *f = 1;
        }
    }
}

#[cfg(test)]
mod tests_wave31_agent35 {
    use super::*;

    #[test]
    fn clamp() {
        assert_eq!(
            w31a35_clamp_limit(9, 5, 6),
            W31a35LimitResult {
                stored: 5,
                reset: true
            }
        );
        assert!(!w31a35_clamp_limit(9, 5, 5).reset);
    }
    #[test]
    fn push() {
        let mut v = vec![[1; 6], [2; 6]];
        w31a35_push_back6(&mut v, Some(1), [9; 6]);
        w31a35_push_back6(&mut v, Some(7), [9; 6]);
        assert_eq!(v[2], [2; 6]);
        assert_eq!(v[3], [9; 6]);
    }
    #[test]
    fn filter() {
        assert_eq!(w31a35_filter_message(3, 0), W31a35Dispatch::Zero);
        assert_eq!(w31a35_filter_message(0xe, 0), W31a35Dispatch::PreThenBase);
        assert_eq!(w31a35_filter_message(0xe, 4), W31a35Dispatch::Zero);
        assert_eq!(w31a35_filter_message(0xe, 1), W31a35Dispatch::Base);
        assert_eq!(w31a35_filter_message(2, 0), W31a35Dispatch::Base);
    }
    #[test]
    fn pre() {
        assert_eq!(
            w31a35_pre_dispatch(0xe, 0, 3),
            Some(W31a35Pre::Notify(4, 0x2000_0000))
        );
        assert_eq!(w31a35_pre_dispatch(0xe, 2, 1), Some(W31a35Pre::Adjust(1)));
        assert_eq!(w31a35_pre_dispatch(0xe, 2, 3), Some(W31a35Pre::Adjust(-1)));
        assert_eq!(w31a35_pre_dispatch(1, 0, 3), None);
    }
    #[test]
    fn utf8() {
        assert!(w31a35_is_utf8_lead_or_ascii(0x7f));
        assert!(!w31a35_is_utf8_lead_or_ascii(0x80));
        assert!(w31a35_is_utf8_lead_or_ascii(0xc2));
        assert!(w31a35_is_utf8_lead_or_ascii(0xf4));
        assert!(!w31a35_is_utf8_lead_or_ascii(0xf5));
    }
    #[test]
    fn store() {
        let mut r = vec![0u8; 0x21c];
        r[0x21a] = 0xff;
        assert!(!w31a35_store_pending(&mut r, 1, 2));
        r[0x21a] = 0;
        assert!(w31a35_store_pending(&mut r, 0x1234, 0xdead));
        assert_eq!(r[0x118], 8);
        assert_eq!(r[0x218], 0x34);
        assert_eq!(r[0x214], 0xad);
    }
    #[test]
    fn reset() {
        let mut a = [1, 1];
        let mut b = [1, 1];
        w31a35_reset_flags(&mut a, &mut b, Some(1));
        assert_eq!(a, [0, 0]);
        assert_eq!(b, [0, 1]);
    }
}
