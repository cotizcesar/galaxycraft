#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent54.rs
// ===========================================================================
// Wave 31 batch 54 (runtime.rs target).

/// `0x00911980`: ordered-map exact lookup (lower bound then `<=` check) over
/// the tree at `this + 0x714`; returns the value or `None`.
pub fn w31a54_map_find<V>(map: &std::collections::BTreeMap<u32, V>, key: u32) -> Option<&V> {
    map.range(key..)
        .next()
        .filter(|(k, _)| **k <= key)
        .map(|(_, v)| v)
}

/// `0x00913270`: returns the cached default at `this + 0x218` when the
/// lookup yields -1, otherwise the resolved value.
pub fn w31a54_resolve_or_default(
    lookup: i32,
    default: u32,
    resolve: impl FnOnce(i32) -> u32,
) -> u32 {
    if lookup == -1 {
        default
    } else {
        resolve(lookup)
    }
}

/// `0x009140a0`: logical negation of the flag returned by the helper.
pub fn w31a54_not_flag(flag: bool) -> bool {
    !flag
}

/// `0x00914220`: for id `0xc` with the flag clear returns 1, otherwise
/// defers to the fallback.
pub fn w31a54_id_check(id: i32, flag: bool, fallback: impl FnOnce(i32) -> u8) -> u8 {
    if id == 0xc && !flag {
        return 1;
    }
    fallback(id)
}

/// `0x00913ec0`: which handler to run.  `None` when no mode flag is set or
/// the object byte at `+0xf0` is non-zero; otherwise the first set mode in
/// order (a, b, c) selects handler 0, 1, 2 (c's branch is the final else).
pub fn w31a54_select_handler(a: bool, b: bool, c: bool, busy: u8) -> Option<u8> {
    if !(a || b || c) || busy != 0 {
        return None;
    }
    Some(if a {
        0
    } else if b {
        1
    } else {
        2
    })
}

/// `0x009185d0`: copies up to 20 point records (stride 8, u16 at +0 and +4)
/// into `dst` (stride 4 pairs) and returns the clamped count, or `None`
/// when `count` is zero.
pub fn w31a54_copy_points(src: &[u8], count: u16, dst: &mut [(u16, u16); 20]) -> Option<u16> {
    if count == 0 {
        return None;
    }
    let n = count.min(0x14) as usize;
    for i in 0..n {
        let r = |o: usize| u16::from_le_bytes([src[i * 8 + o], src[i * 8 + o + 1]]);
        dst[i] = (r(0), r(4));
    }
    Some(n as u16)
}

#[cfg(test)]
mod tests_wave31_agent54 {
    use super::*;

    #[test]
    fn map_find() {
        let mut m = std::collections::BTreeMap::new();
        m.insert(5u32, 'a');
        assert_eq!(w31a54_map_find(&m, 5), Some(&'a'));
        assert_eq!(w31a54_map_find(&m, 4), None);
        assert_eq!(w31a54_map_find(&m, 6), None);
    }

    #[test]
    fn small_predicates() {
        assert_eq!(w31a54_resolve_or_default(-1, 7, |x| x as u32), 7);
        assert_eq!(w31a54_resolve_or_default(3, 7, |x| x as u32 + 1), 4);
        assert!(w31a54_not_flag(false));
        assert_eq!(w31a54_id_check(0xc, false, |_| 9), 1);
        assert_eq!(w31a54_id_check(0xc, true, |_| 9), 9);
        assert_eq!(w31a54_id_check(1, false, |_| 9), 9);
        assert_eq!(w31a54_select_handler(false, false, false, 0), None);
        assert_eq!(w31a54_select_handler(true, false, false, 1), None);
        assert_eq!(w31a54_select_handler(false, true, false, 0), Some(1));
        assert_eq!(w31a54_select_handler(false, false, true, 0), Some(2));
    }

    #[test]
    fn copy_points() {
        let mut src = vec![0u8; 8 * 25];
        for i in 0..25 {
            src[i * 8] = i as u8;
            src[i * 8 + 4] = 100 + i as u8;
        }
        let mut dst = [(0, 0); 20];
        assert_eq!(w31a54_copy_points(&src, 0, &mut dst), None);
        assert_eq!(w31a54_copy_points(&src, 25, &mut dst), Some(20));
        assert_eq!(dst[19], (19, 119));
        assert_eq!(w31a54_copy_points(&src, 2, &mut dst), Some(2));
    }
}
