#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave21/agent14.rs
// ===========================================================================
// Wave 21 agent 14: minimap mirror-table builder and small state helpers.

/// `0x005bb8f0`: copies the two dwords at offset `0x10` and `0x14` of the
/// record reached through `this+0xc -> +0x1c` into the globals pair
/// (`DAT_00fa371c`, `DAT_00fa3720`).  `rec` is that inner record.
pub fn w21a14_load_pair(rec: &[u8]) -> Option<(u32, u32)> {
    let a = rec.get(0x10..0x14)?;
    let b = rec.get(0x14..0x18)?;
    Some((
        u32::from_le_bytes([a[0], a[1], a[2], a[3]]),
        u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
    ))
}

/// `0x005c4ad0`: releases the table buffer when one is held and clears the
/// handle.  Returns true when a free was issued.
pub fn w21a14_release_table(handle: &mut u32, mut free: impl FnMut(u32)) -> bool {
    let had = *handle != 0;
    if had {
        free(*handle);
    }
    *handle = 0;
    had
}

/// Result of `w21a14_build_mirror_table`.
#[derive(Debug, PartialEq, Eq)]
pub struct W21a14Mirror {
    /// Table of `cols * rows * 4` bytes.
    pub table: Vec<u8>,
    /// Final `DAT_00fa372c` (`cols << 5`).
    pub cols_scaled: u32,
    /// Final `DAT_00fa3728` (`rows << 5`).
    pub rows_scaled: u32,
}

/// `0x005c4c40`: builds the four-way mirrored table.  `dim_b` is the byte
/// at `+3` of the map record (columns, `(b+0xf)>>5`), `dim_a` the byte at
/// `+2` (rows).  `sample(x, y)` models `FUN_00572860` (map tile query at
/// pixel offsets starting at 0x10 with step 0x20).  The scratch globals
/// `DAT_00fa3724`, `DAT_00fa3734`, `DAT_00fa3738` are zeroed by the caller.
pub fn w21a14_build_mirror_table(
    dim_a: u8,
    dim_b: u8,
    mut sample: impl FnMut(i32, i32) -> u8,
) -> W21a14Mirror {
    let cols = ((dim_b as i32) + 0xf) >> 5;
    let rows = ((dim_a as i32) + 0xf) >> 5;
    let mut table = vec![0u8; (cols * rows * 4) as usize];
    let mut put = |idx: i32, v: u8| {
        if idx >= 0
            && let Some(s) = table.get_mut(idx as usize)
        {
            *s = v;
        }
    };
    for i in 0..cols {
        let y = 0x10 + 0x20 * i;
        for j in 0..rows {
            let x = 0x10 + 0x20 * j;
            let v = sample(x, y);
            let a = (2 * i + 1) * rows;
            put(a + j, v);
            put(a - j - 1, v);
            let b = ((cols * 2 - i) * 2 - 1) * rows;
            put(b + j, v);
            put(b - j - 1, v);
        }
    }
    W21a14Mirror {
        table,
        cols_scaled: (cols << 5) as u32,
        rows_scaled: (rows << 5) as u32,
    }
}

#[cfg(test)]
mod tests_wave21_agent14 {
    use super::*;

    #[test]
    fn pair_load() {
        let mut r = [0u8; 0x18];
        r[0x10] = 1;
        r[0x14] = 2;
        assert_eq!(w21a14_load_pair(&r), Some((1, 2)));
        assert_eq!(w21a14_load_pair(&r[..0x17]), None);
    }

    #[test]
    fn release() {
        let mut h = 5;
        let mut freed = vec![];
        assert!(w21a14_release_table(&mut h, |x| freed.push(x)));
        assert_eq!((h, freed), (0, vec![5]));
        assert!(!w21a14_release_table(&mut h, |_| panic!()));
    }

    #[test]
    fn mirror() {
        let m = w21a14_build_mirror_table(49, 49, |x, y| (x / 0x20 + y / 0x20 * 4 + 1) as u8);
        // cols = rows = 2, table 16 bytes, scaled 64.
        assert_eq!(m.table.len(), 16);
        assert_eq!((m.cols_scaled, m.rows_scaled), (64, 64));
        // i=0,j=0 sample=1: a=2 -> [2],[1]; b=(4*1-... ) = (4-0)*2-1=7 *2=14 -> [14],[13]
        assert_eq!(m.table[2], 1);
        assert_eq!(m.table[1], 1);
        assert_eq!(m.table[14], 1);
        assert_eq!(m.table[13], 1);
        let z = w21a14_build_mirror_table(0, 0, |_, _| 9);
        assert!(z.table.is_empty());
    }
}
