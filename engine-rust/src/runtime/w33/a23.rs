#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent23.rs
// ===========================================================================
// Wave 33 agent 23: small stream-object helpers.

/// `0x009d3c90`: clears the dword at byte offset 0x30 of `this` (`obj`,
/// dword index 12) and, when the pointer dword at 0x10 (index 4) is
/// non-null, bumps the refcount (dword at +8) of the pointed object through
/// `bump_ref`.  Always returns 0.
pub fn w33a23_reset_and_addref(obj: &mut [u32], bump_ref: &mut dyn FnMut(u32)) -> u32 {
    obj[12] = 0;
    if obj[4] != 0 {
        bump_ref(obj[4]);
    }
    0
}

/// `0x009e4110`: write `count` items of `size` bytes.  `obj[0]` is the
/// underlying handle (0 means nothing to do, returns 0); byte 0x44 (dword 0x11,
/// low byte) selects the memory path (`mem_write(size*count)` == 0 returns
/// `count`, else 0).  Otherwise `file_write(size, count, handle)` is called,
/// then `obj[1]` is refreshed from `file_tell(handle)` and the write result
/// returned.
pub fn w33a23_write_items(
    obj: &mut [u32],
    size: i32,
    count: i32,
    mem_write: &mut dyn FnMut(i32) -> i32,
    file_write: &mut dyn FnMut(i32, i32, u32) -> i32,
    file_tell: &mut dyn FnMut(u32) -> u32,
) -> i32 {
    if obj[0] == 0 {
        return 0;
    }
    if (obj[0x11] & 0xff) != 0 {
        let r = mem_write(size.wrapping_mul(count));
        return if r == 0 { count } else { 0 };
    }
    let r = file_write(size, count, obj[0]);
    obj[1] = file_tell(obj[0]);
    r
}

#[cfg(test)]
mod tests_wave33_agent23 {
    use super::*;

    #[test]
    fn addref() {
        let mut o = [0u32; 16];
        o[12] = 9;
        o[4] = 7;
        let mut seen = vec![];
        assert_eq!(w33a23_reset_and_addref(&mut o, &mut |p| seen.push(p)), 0);
        assert_eq!(o[12], 0);
        assert_eq!(seen, vec![7]);
        o[4] = 0;
        assert_eq!(w33a23_reset_and_addref(&mut o, &mut |p| seen.push(p)), 0);
        assert_eq!(seen.len(), 1);
    }

    #[test]
    fn write() {
        let mut o = [0u32; 0x12];
        assert_eq!(
            w33a23_write_items(&mut o, 2, 3, &mut |_| 0, &mut |_, _, _| 5, &mut |_| 1),
            0
        );
        o[0] = 4;
        assert_eq!(
            w33a23_write_items(&mut o, 2, 3, &mut |_| 0, &mut |_, c, _| c, &mut |h| h + 1),
            3
        );
        assert_eq!(o[1], 5);
        o[0x11] = 1;
        assert_eq!(
            w33a23_write_items(&mut o, 2, 3, &mut |n| n - 6, &mut |_, _, _| 0, &mut |_| 0),
            3
        );
        assert_eq!(
            w33a23_write_items(&mut o, 2, 3, &mut |_| 1, &mut |_, _, _| 0, &mut |_| 0),
            0
        );
    }
}
