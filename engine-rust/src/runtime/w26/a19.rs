#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent19.rs
// ===========================================================================
// Wave 26 agent 19: small container / vector helpers.

/// `0x00947040`: writes an integer pair into a float pair (x, y).
pub fn w26a19_int_pair_to_float(out: &mut [f32; 2], x: i32, y: i32) {
    out[0] = x as f32 - 0.0;
    out[1] = y as f32 - 0.0;
}

/// `0x009470c0`: same conversion as the sibling at 0x00947040 (second copy).
pub fn w26a19_int_pair_to_float_b(out: &mut [f32; 2], x: i32, y: i32) {
    out[0] = x as f32 - 0.0;
    out[1] = y as f32 - 0.0;
}

/// `0x00947320`: capacity growth.  `field` is the dword at +8 (low 31 bits
/// capacity, top bit a flag).  Returns the updated field when growth to
/// `wanted` is required (new cap = max(wanted, cap + cap/2)), else `None`.
pub fn w26a19_grow_capacity_field(field: u32, wanted: u32) -> Option<u32> {
    if (field & 0x7fff_ffff) >= wanted {
        return None;
    }
    let mut new_cap = ((field >> 1) & 0x3fff_ffff).wrapping_add(field & 0x7fff_ffff);
    if new_cap < wanted {
        new_cap = wanted;
    }
    Some(((field ^ new_cap) & 0x7fff_ffff) ^ field)
}

/// `0x009473d0`: resizes a vector of 20-byte (five dword) records; shrinking
/// truncates, growing appends zeroed records.
pub fn w26a19_resize_records(v: &mut Vec<[u32; 5]>, new_len: usize) {
    if new_len < v.len() {
        v.truncate(new_len);
    } else if new_len > v.len() {
        v.resize(new_len, [0; 5]);
    }
}

/// Five-dword vector header: begin, end, then three allocator fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W26a19Header(pub [u32; 5]);

/// `0x0094a9b0`: move-assign; destination is reset, then takes all five
/// dwords of `src`, which is zeroed.
pub fn w26a19_move_header(dst: &mut W26a19Header, src: &mut W26a19Header) {
    dst.0 = src.0;
    src.0 = [0; 5];
}

/// `0x0094aa40`: initialises the header: fields 2..4 from args, 0 and 1 zero.
pub fn w26a19_init_header(h: &mut W26a19Header, a: u32, b: u32, c: u32) {
    h.0 = [0, 0, a, b, c];
}

/// `0x0094aa70`: clear; frees the buffer (caller-supplied) if non-null and
/// zeroes the first two fields.
pub fn w26a19_clear_header(h: &mut W26a19Header, free: impl FnOnce(u32)) {
    if h.0[0] != 0 {
        free(h.0[0]);
    }
    h.0[1] = 0;
    h.0[0] = 0;
}

#[cfg(test)]
mod tests_wave26_agent19 {
    use super::*;

    #[test]
    fn pair_convert() {
        let mut o = [0.0; 2];
        w26a19_int_pair_to_float(&mut o, -3, 7);
        assert_eq!(o, [-3.0, 7.0]);
        w26a19_int_pair_to_float_b(&mut o, 1, 2);
        assert_eq!(o, [1.0, 2.0]);
    }

    #[test]
    fn grow_field() {
        assert_eq!(w26a19_grow_capacity_field(10, 5), None);
        assert_eq!(w26a19_grow_capacity_field(10, 11), Some(15));
        assert_eq!(w26a19_grow_capacity_field(10, 100), Some(100));
        assert_eq!(
            w26a19_grow_capacity_field(0x8000_0010, 100),
            Some(0x8000_0064)
        );
    }

    #[test]
    fn resize() {
        let mut v = vec![[1; 5]];
        w26a19_resize_records(&mut v, 3);
        assert_eq!(v, vec![[1; 5], [0; 5], [0; 5]]);
        w26a19_resize_records(&mut v, 1);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn header_ops() {
        let mut a = W26a19Header([9, 8, 7, 6, 5]);
        let mut b = W26a19Header::default();
        w26a19_move_header(&mut b, &mut a);
        assert_eq!(b.0, [9, 8, 7, 6, 5]);
        assert_eq!(a.0, [0; 5]);
        w26a19_init_header(&mut a, 1, 2, 3);
        assert_eq!(a.0, [0, 0, 1, 2, 3]);
        let mut freed = 0;
        w26a19_clear_header(&mut b, |p| freed = p);
        assert_eq!(freed, 9);
        assert_eq!(b.0[..2], [0, 0]);
    }
}
