#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent68.rs
// ===========================================================================
// Wave 33 agent 68: CRT locale-struct free helpers.

/// `0x00cb5b85`: frees the owned string fields of an lconv-like record.
/// Fields 0, 1, 2, 12 and 13 are released through `free` unless they still
/// point at the static default (`is_default(field_index, value)`).
pub fn w33a68_free_lconv_strings(
    fields: Option<&[usize]>,
    is_default: impl Fn(usize, usize) -> bool,
    mut free: impl FnMut(usize),
) {
    let Some(f) = fields else { return };
    for idx in [0usize, 1, 2, 0xc, 0xd] {
        if !is_default(idx, f[idx]) {
            free(f[idx]);
        }
    }
}

/// `0x00cb5dff`: frees each pointer of an array of `count` entries; the
/// count is clamped to 30 bits and forced to zero on pointer overflow
/// (`base + count*4` wrapping).
pub fn w33a68_free_ptr_array(
    base: u32,
    count: u32,
    entries: &[usize],
    mut free: impl FnMut(usize),
) {
    let wraps = base.wrapping_add(count.wrapping_mul(4)) < base;
    let n = if wraps { 0 } else { count & 0x3fff_ffff } as usize;
    for e in entries.iter().take(n) {
        free(*e);
    }
}

#[cfg(test)]
mod tests_wave33_agent68 {
    use super::*;

    #[test]
    fn lconv_frees_non_default() {
        let f = [10usize, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 8];
        let mut freed = vec![];
        w33a68_free_lconv_strings(Some(&f), |i, v| i == 1 && v == 1, |p| freed.push(p));
        assert_eq!(freed, vec![10, 2, 7, 8]);
        w33a68_free_lconv_strings(None, |_, _| false, |_| panic!());
    }

    #[test]
    fn ptr_array() {
        let mut freed = vec![];
        w33a68_free_ptr_array(0x1000, 2, &[5, 6, 7], |p| freed.push(p));
        assert_eq!(freed, vec![5, 6]);
        freed.clear();
        w33a68_free_ptr_array(0xffff_fff0, 0x10, &[5], |p| freed.push(p));
        assert!(freed.is_empty());
    }
}
