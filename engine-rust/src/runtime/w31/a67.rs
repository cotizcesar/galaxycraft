#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent67.rs
// ===========================================================================
// Wave 31 agent 67: EBML element header reader helper.

/// `0x00a73f30`: reads an element id and an element size at `*pos`, advancing
/// `*pos` past both.  `limit < 0` means unbounded.  `read_id` and `read_size`
/// take the current position and return `(value, byte_length)`; a negative
/// value is an error.  The id read stores its value in `*id_out`, the size
/// read in `*size_out`.  Returns `0` on success and `-2` (0xfffffffe) when the
/// stream ends before or inside a field, or the length is outside `1..=8`.
pub fn w31a67_read_element_header(
    pos: &mut i64,
    limit: i64,
    id_out: &mut i64,
    size_out: &mut i64,
    mut read_id: impl FnMut(i64) -> (i64, i32),
    mut read_size: impl FnMut(i64) -> (i64, i32),
) -> i32 {
    const FAIL: i32 = -2;
    if !(limit < 0 || *pos < limit) {
        return FAIL;
    }
    let (id, len) = read_id(*pos);
    *id_out = id;
    if id < 0 {
        return FAIL;
    }
    *pos = pos.wrapping_add(len as i64);
    if !(limit < 0 || *pos < limit) {
        return FAIL;
    }
    let (size, len) = read_size(*pos);
    *size_out = size;
    if size < 0 || !(1..=8).contains(&len) {
        return FAIL;
    }
    let next = pos.wrapping_add(len as i64);
    if next < 0 {
        return FAIL;
    }
    *pos = next;
    if limit < 0 || next <= limit { 0 } else { FAIL }
}

#[cfg(test)]
mod tests_wave31_agent67 {
    use super::*;

    #[test]
    fn header_ok_and_bounds() {
        let (mut id, mut sz) = (0, 0);
        let mut pos = 10;
        let r =
            w31a67_read_element_header(&mut pos, -1, &mut id, &mut sz, |_| (0x1a, 4), |_| (7, 1));
        assert_eq!((r, pos, id, sz), (0, 15, 0x1a, 7));
        let mut pos = 10;
        let r = w31a67_read_element_header(&mut pos, 15, &mut id, &mut sz, |_| (1, 4), |_| (7, 1));
        assert_eq!(r, 0);
        let mut pos = 10;
        let r = w31a67_read_element_header(&mut pos, 14, &mut id, &mut sz, |_| (1, 4), |_| (7, 1));
        assert_eq!(r, -2);
        let mut pos = 20;
        assert_eq!(
            w31a67_read_element_header(&mut pos, 20, &mut id, &mut sz, |_| (1, 1), |_| (1, 1)),
            -2
        );
        let mut pos = 0;
        assert_eq!(
            w31a67_read_element_header(&mut pos, -1, &mut id, &mut sz, |_| (1, 1), |_| (1, 9)),
            -2
        );
        assert_eq!(
            w31a67_read_element_header(&mut pos, -1, &mut id, &mut sz, |_| (-1, 1), |_| (1, 1)),
            -2
        );
    }
}
