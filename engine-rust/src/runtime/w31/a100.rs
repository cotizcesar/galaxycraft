#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent100.rs
// ===========================================================================
// Wave 31 agent 100: display-mode record fill (runtime glue).

/// Output record filled by `w31a100_fill_mode_info`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct W31a100ModeInfo {
    /// `[0]` product of the four extents, `[1]` ext0*ext2, `[2]` ext1*ext2*ext0.
    pub words: [i32; 12],
    /// Byte at offset `0x30`, selected from an 8-entry table.
    pub kind: u8,
    /// True when both name slots (`0x50`, `0x54`) hold the "Unknown" literal.
    pub names_unknown: bool,
}

/// `0x00c222e0`: resets the record names to "Unknown", optionally sets the
/// kind byte from `table[idx]` when `idx < 8`, then fills the extent words
/// from `src` = `[a0, a1..a5(4..8), b..]` as laid out by the query routine.
/// `kind_query` returns `Some(idx)` on success; `extent_query` returns the
/// 10 words `local_30..local_10` (index 0 = local_30 ... 9 = local_10).
pub fn w31a100_fill_mode_info(
    rec: &mut W31a100ModeInfo,
    table: &[u8; 8],
    kind_query: impl FnOnce() -> Option<u32>,
    extent_query: impl FnOnce() -> Option<[i32; 10]>,
) {
    rec.names_unknown = true;
    if let Some(idx) = kind_query() {
        if idx < 8 {
            rec.kind = table[idx as usize];
        }
    }
    if let Some(l) = extent_query() {
        // l[0]=local_30 l[1]=2c l[2]=28 l[3]=24 l[4]=20 l[5]=1c l[6]=18 l[7]=14 l[8]=10
        rec.words[3] = l[0];
        rec.words[9] = l[6];
        rec.words[10] = l[7];
        rec.words[11] = l[8];
        rec.words[0] = l[8]
            .wrapping_mul(l[7])
            .wrapping_mul(l[6])
            .wrapping_mul(l[0]);
        rec.words[1] = l[6].wrapping_mul(l[0]);
        rec.words[2] = l[7].wrapping_mul(l[6]).wrapping_mul(l[0]);
        rec.words[4] = l[1];
        rec.words[5] = l[2];
        rec.words[6] = l[3];
        rec.words[7] = l[4];
        rec.words[8] = l[5];
    }
}

#[cfg(test)]
mod tests_wave31_agent100 {
    use super::*;

    #[test]
    fn fill_products() {
        let mut r = W31a100ModeInfo::default();
        let t = [1, 2, 3, 4, 5, 6, 7, 8];
        w31a100_fill_mode_info(
            &mut r,
            &t,
            || Some(2),
            || Some([2, 10, 11, 12, 13, 14, 3, 5, 7, 0]),
        );
        assert_eq!(r.kind, 3);
        assert_eq!(r.words[0], 7 * 5 * 3 * 2);
        assert_eq!(r.words[1], 6);
        assert_eq!(r.words[2], 30);
        assert_eq!(r.words[8], 14);
        let mut r2 = W31a100ModeInfo::default();
        w31a100_fill_mode_info(&mut r2, &t, || Some(8), || None);
        assert_eq!(r2.kind, 0);
        assert!(r2.names_unknown);
    }
}
