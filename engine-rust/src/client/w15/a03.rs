#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave15/agent03.rs
// ===========================================================================
// Wave 15 translation batch 03.
//
// Small table accessors and a `strnlen` kernel.  Skipped rows (callee-heavy
// `this`-based methods, SEH funclets, CRT stubs):
// FUN_005a3330, FUN_005afc80, FUN_005c51f0, FUN_005c70f0, FUN_005cbc10,
// FUN_005d9640, FUN_005dd37c (SEH funclet), FUN_006506a0, FUN_00650ee0,
// FUN_00654670, FUN_00657390 (all depend on many untranslated callees),
// FUN_00952b40 (int3 trap stub), FUN_00c66860 (indirect call thunk).

/// `0x00635c40`: player-table accessor.  Rows have stride `0x4e8`; `rows`
/// starts at the byte read as the result (retail `0xfaa211`) and the active
/// flag lives at row offset `0x1f7`.  Returns 0 when the flag is clear.
pub fn w15a03_player_value_if_active(rows: &[u8], player: usize) -> u8 {
    let base = player * 0x4e8;
    if rows[base + 0x1f7] != 0 {
        rows[base]
    } else {
        0
    }
}

/// `0x00695c10`: reads the dword at index `param_2 + param_1 * 12` of the
/// decoded table (retail table at decoded-base + `0x5cf4`).
pub fn w15a03_table_dword(table: &[u32], param_1: u16, param_2: u8) -> u32 {
    table[param_2 as usize + param_1 as usize * 12]
}

/// `0x0075d630`: converts the 24.8 fixed-point pair of record `id`
/// (stride `0x40`, fields at `0x44a0c` and `0x44a10` from the base) to
/// integers with an arithmetic shift right by 8.  `base` is the buffer
/// starting at the table base.
pub fn w15a03_record_xy(base: &[u8], id: u16) -> (i32, i32) {
    let at = id as usize * 0x40;
    let rd = |off: usize| {
        let o = at + off;
        i32::from_le_bytes([base[o], base[o + 1], base[o + 2], base[o + 3]])
    };
    (rd(0x44a0c) >> 8, rd(0x44a10) >> 8)
}

/// `0x0076a500`: returns the i16 at offset 0 of the `0x30`-byte row for
/// `index` (rows start at retail `0x11ac91a`), or 0 when `index` is 0 or
/// `>= 0xfa`.
pub fn w15a03_row_short(rows: &[u8], index: u32) -> i32 {
    if index != 0 && index < 0xfa {
        let o = index as usize * 0x30;
        i16::from_le_bytes([rows[o], rows[o + 1]]) as i32
    } else {
        0
    }
}

/// `0x0076a7b0`: tries to claim row `index` (flag byte at row offset 5, so
/// retail `0x11ac91f`): fails (false) when index is in `1..0xfa` and bits
/// 0|1 are already set; otherwise sets bit 1 and returns true.  Unlike
/// retail an out-of-range index still indexes the table (as the original).
pub fn w15a03_claim_row(rows: &mut [u8], index: u32) -> bool {
    let o = index as usize * 0x30 + 5;
    if index != 0 && index < 0xfa && rows[o] & 3 != 0 {
        return false;
    }
    rows[o] |= 2;
    true
}

/// `0x0076a7f0`: clears bit 1 of the flag byte (row offset 5) of row `index`.
pub fn w15a03_release_row(rows: &mut [u8], index: u32) {
    rows[index as usize * 0x30 + 5] &= 0xfd;
}

/// `0x00c9307a`: vectorised `strnlen`.  All SSE2/AVX2/scalar branches compute
/// the index of the first NUL within the first `max` bytes (or `max`).
pub fn w15a03_strnlen(buf: &[u8], max: usize) -> usize {
    let lim = max.min(buf.len());
    buf[..lim].iter().position(|&b| b == 0).unwrap_or(max)
}

#[cfg(test)]
mod tests_wave15_agent03 {
    use super::*;

    #[test]
    fn player_value() {
        let mut t = vec![0u8; 0x4e8 * 2];
        t[0x4e8] = 7;
        assert_eq!(w15a03_player_value_if_active(&t, 1), 0);
        t[0x4e8 + 0x1f7] = 1;
        assert_eq!(w15a03_player_value_if_active(&t, 1), 7);
    }

    #[test]
    fn dword_table() {
        let mut t = vec![0u32; 30];
        t[12 + 3] = 9;
        assert_eq!(w15a03_table_dword(&t, 1, 3), 9);
    }

    #[test]
    fn record_xy() {
        let mut b = vec![0u8; 0x44a14 + 0x40];
        b[0x40 + 0x44a0c..0x40 + 0x44a10].copy_from_slice(&(-512i32).to_le_bytes());
        b[0x40 + 0x44a10..0x40 + 0x44a14].copy_from_slice(&(0x300i32).to_le_bytes());
        assert_eq!(w15a03_record_xy(&b, 1), (-2, 3));
    }

    #[test]
    fn rows() {
        let mut r = vec![0u8; 0x30 * 0x100];
        r[0x30 * 5..0x30 * 5 + 2].copy_from_slice(&(-4i16).to_le_bytes());
        assert_eq!(w15a03_row_short(&r, 5), -4);
        assert_eq!(w15a03_row_short(&r, 0), 0);
        assert_eq!(w15a03_row_short(&r, 0xfa), 0);
        assert!(w15a03_claim_row(&mut r, 5));
        assert!(!w15a03_claim_row(&mut r, 5));
        w15a03_release_row(&mut r, 5);
        assert!(w15a03_claim_row(&mut r, 5));
    }

    #[test]
    fn strnlen() {
        assert_eq!(w15a03_strnlen(b"abc\0def", 7), 3);
        assert_eq!(w15a03_strnlen(b"abcdef", 4), 4);
        assert_eq!(w15a03_strnlen(b"abcdef", 6), 6);
        assert_eq!(w15a03_strnlen(b"", 0), 0);
    }
}
