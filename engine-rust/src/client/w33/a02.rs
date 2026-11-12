#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent02.rs
// ===========================================================================
// Wave 33 translation batch 02.

/// `0x00805e30`: copies the 8-byte pair `src` into the object's fields at
/// offsets `0x98` and `0x9c` and returns `true`.
pub fn w33a02_store_pair(obj: &mut [u8], src: [u32; 2]) -> bool {
    if obj.len() < 0xa0 {
        return false;
    }
    obj[0x98..0x9c].copy_from_slice(&src[0].to_le_bytes());
    obj[0x9c..0xa0].copy_from_slice(&src[1].to_le_bytes());
    true
}

/// `0x0087b3f0`: when the record's field `0x48` is nonzero, fetches a point
/// from `get_point` (callee FUN_0087bc70) and calls `apply` (callee
/// FUN_0087c580) with that point offset by the record's `(0x40, 0x44)`.
pub fn w33a02_apply_offset_point(
    rec: &[u8],
    get_point: impl FnOnce() -> (i32, i32),
    mut apply: impl FnMut(i32, i32),
) -> bool {
    let rd = |o: usize| i32::from_le_bytes([rec[o], rec[o + 1], rec[o + 2], rec[o + 3]]);
    if rec.len() < 0x4c || rd(0x48) == 0 {
        return false;
    }
    let (x, y) = get_point();
    apply(x.wrapping_add(rd(0x40)), y.wrapping_add(rd(0x44)));
    true
}

#[cfg(test)]
mod tests_wave33_agent02 {
    use super::*;

    #[test]
    fn store_pair() {
        let mut o = vec![0u8; 0xa0];
        assert!(w33a02_store_pair(&mut o, [1, 0x0102_0304]));
        assert_eq!(&o[0x98..0xa0], &[1, 0, 0, 0, 4, 3, 2, 1]);
        assert!(!w33a02_store_pair(&mut [0u8; 4], [0, 0]));
    }

    #[test]
    fn offset_point() {
        let mut r = vec![0u8; 0x4c];
        r[0x40] = 5;
        r[0x44] = 0xfe;
        r[0x45..0x48].fill(0xff);
        let mut got = None;
        assert!(!w33a02_apply_offset_point(
            &r,
            || (1, 1),
            |x, y| got = Some((x, y))
        ));
        assert!(got.is_none());
        r[0x48] = 1;
        assert!(w33a02_apply_offset_point(
            &r,
            || (10, 20),
            |x, y| got = Some((x, y))
        ));
        assert_eq!(got, Some((15, 18)));
    }
}
