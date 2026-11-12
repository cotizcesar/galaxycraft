#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent04.rs
// ===========================================================================
// Wave 33 translation batch 04.

/// `0x0093ecd0`: scans a NUL-terminated path and returns the offset of the
/// last `.` that follows the last `/` or `\`, or the end offset (terminator)
/// when there is none.  `param_2` is the extra offset added to the dot
/// position (the original returns `ptr + param_2`).
pub fn w33a04_find_extension(path: &[u8], extra: usize) -> usize {
    let mut found: Option<usize> = None;
    let mut i = 0usize;
    loop {
        match path.get(i).copied().unwrap_or(0) {
            0 => return found.unwrap_or(i),
            b'.' => {
                found = Some(i + extra);
                i += 1;
            }
            b'/' | b'\\' => {
                found = None;
                i += 1;
            }
            _ => i += 1,
        }
    }
}

/// `0x0095d1e0`: looks up entry `index` of a big-endian offset table.
/// `table` is the bytes at `+0x10 + +4`; `count` is the field at `+0xc`;
/// `mode` the field at `+0x2c` (0 = 16-bit entries scaled by 2, 1 = 24-bit
/// entries followed by a byte, scaled 0x100); `base` is the field at
/// `+0x18`.  Returns `base + offset`, or -1 when out of range, when mode
/// is >= 2, or when the entry equals the next one (empty).
pub fn w33a04_table_entry(table: &[u8], count: i32, mode: i32, base: i32, index: i32) -> i32 {
    if !(index < count && mode < 2) || index < 0 {
        return -1;
    }
    let i = index as usize;
    let b = |o: usize| table.get(o).copied().unwrap_or(0) as i32;
    if mode == 0 {
        let a = base.wrapping_add(((b(i * 2) << 8) | b(i * 2 + 1)).wrapping_mul(2));
        let n = base.wrapping_add(((b(i * 2 + 2) << 8) | b(i * 2 + 3)).wrapping_mul(2));
        return if a == n { -1 } else { a };
    }
    let ent = |o: usize| {
        let hi = (b(o) << 16) | (b(o + 1) << 8) | b(o + 2);
        b(o + 3)
            .wrapping_add(hi.wrapping_mul(0x100))
            .wrapping_add(base)
    };
    let a = ent(i * 4);
    if a == ent(i * 4 + 4) { -1 } else { a }
}

#[cfg(test)]
mod tests_wave33_agent04 {
    use super::*;

    #[test]
    fn ext() {
        assert_eq!(w33a04_find_extension(b"a/b.c/d\0", 0), 7);
        assert_eq!(w33a04_find_extension(b"a/b.c\0", 0), 3);
        assert_eq!(w33a04_find_extension(b"a.b.c\0", 10), 13);
    }

    #[test]
    fn entry() {
        let t = [0u8, 1, 0, 3, 0, 3];
        assert_eq!(w33a04_table_entry(&t, 3, 0, 100, 0), 102);
        assert_eq!(w33a04_table_entry(&t, 3, 0, 100, 1), -1);
        assert_eq!(w33a04_table_entry(&t, 3, 0, 100, 3), -1);
        assert_eq!(w33a04_table_entry(&t, 3, 2, 100, 0), -1);
        let t4 = [0u8, 0, 1, 5, 0, 0, 2, 0, 0, 0, 2, 0];
        assert_eq!(w33a04_table_entry(&t4, 3, 1, 0, 0), 0x105);
        assert_eq!(w33a04_table_entry(&t4, 3, 1, 0, 1), -1);
    }
}
