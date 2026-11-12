#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent06.rs
// ===========================================================================
// Wave 29 translation batch 06: small container/record helpers.

/// `0x00aa9590`: address of the `index`-th record of the table at
/// `DAT_0136c2c0` (stride `0x78`); returned as a byte offset from the base.
pub fn w29a06_record_offset(index: i32) -> i32 {
    index.wrapping_mul(0x78)
}

/// `0x00aab280`: capacity growth for a string/vector header word `flags`
/// (low 31 bits = capacity, top bit = flag).  When `wanted` exceeds the
/// capacity, the new capacity is `cap + cap/2` (at least `wanted`); returns
/// `(new_flags, new_cap)` with the top bit preserved, else `None`.
pub fn w29a06_grow_capacity(flags: u32, wanted: u32) -> Option<(u32, u32)> {
    if (flags & 0x7fff_ffff) >= wanted {
        return None;
    }
    let mut new_cap = ((flags >> 1) & 0x3fff_ffff).wrapping_add(flags & 0x7fff_ffff);
    if new_cap < wanted {
        new_cap = wanted;
    }
    Some((((flags ^ new_cap) & 0x7fff_ffff) ^ flags, new_cap))
}

/// `0x00aac9b0`: resize a vector of 32-bit slots to `new_len`; shrinking
/// truncates, growing zero-fills the new slots and bumps the capacity word
/// (`cap_flags`) with the same rule as the grow helper at 0x00aab280.
pub fn w29a06_resize_slots(slots: &mut Vec<u32>, cap_flags: &mut u32, new_len: usize) {
    let len = slots.len();
    if new_len < len {
        slots.truncate(new_len);
    } else if len < new_len {
        if let Some((flags, _)) = w29a06_grow_capacity(*cap_flags, new_len as u32) {
            *cap_flags = flags;
        }
        slots.resize(new_len, 0);
    }
}

/// `0x00ac04b0`: formats through the callee `format` (which yields a
/// `(buf, len)` pair) and passes it to `emit`; negative results map to -1.
pub fn w29a06_format_result(
    format: impl FnOnce() -> (u32, u32),
    emit: impl FnOnce(u32, u32) -> i32,
) -> i32 {
    let (a, b) = format();
    let r = emit(a, b);
    if r < 0 { -1 } else { r }
}

/// `0x00ac08a0`: initialises a `0x18` byte record: byte 0 = low byte of the
/// length, bytes 1..0x18 zeroed, then `len` bytes of `src` copied to byte 1
/// (clamped to the 0x17 payload bytes).
pub fn w29a06_init_record_with_len(src: &[u8]) -> [u8; 0x18] {
    let mut rec = [0u8; 0x18];
    rec[0] = src.len() as u8;
    let n = src.len().min(0x17);
    rec[1..1 + n].copy_from_slice(&src[..n]);
    rec
}

/// `0x00ac08e0`: like the initialiser at 0x00ac08a0 but leaves byte 0
/// untouched (`first`); bytes 1..0x18 are zeroed then `src` copied in.
pub fn w29a06_reset_record(first: u8, src: &[u8]) -> [u8; 0x18] {
    let mut rec = [0u8; 0x18];
    rec[0] = first;
    let n = src.len().min(0x17);
    rec[1..1 + n].copy_from_slice(&src[..n]);
    rec
}

#[cfg(test)]
mod tests_wave29_agent06 {
    use super::*;

    #[test]
    fn record_offset() {
        assert_eq!(w29a06_record_offset(0), 0);
        assert_eq!(w29a06_record_offset(2), 0xf0);
    }

    #[test]
    fn grow() {
        assert_eq!(w29a06_grow_capacity(10, 5), None);
        assert_eq!(w29a06_grow_capacity(10, 12), Some((15, 15)));
        assert_eq!(
            w29a06_grow_capacity(0x8000_0010, 100),
            Some((0x8000_0064, 100))
        );
    }

    #[test]
    fn resize() {
        let mut v = vec![1, 2, 3];
        let mut cap = 3;
        w29a06_resize_slots(&mut v, &mut cap, 5);
        assert_eq!(v, [1, 2, 3, 0, 0]);
        assert_eq!(cap, 5);
        w29a06_resize_slots(&mut v, &mut cap, 1);
        assert_eq!(v, [1]);
    }

    #[test]
    fn format_result() {
        assert_eq!(w29a06_format_result(|| (1, 2), |_, _| -5), -1);
        assert_eq!(w29a06_format_result(|| (1, 2), |a, b| (a + b) as i32), 3);
    }

    #[test]
    fn records() {
        let r = w29a06_init_record_with_len(&[7, 8]);
        assert_eq!(&r[..4], &[2, 7, 8, 0]);
        let r = w29a06_reset_record(9, &[1]);
        assert_eq!(&r[..3], &[9, 1, 0]);
    }
}
