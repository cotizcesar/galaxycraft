#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent02.rs
// ===========================================================================
// Wave 23 translation batch 02: unit-table id resolution, block rebasing,
// slot-row scans and small release helpers.

/// `0x006fc4d0`: resolves each non-zero id in `ids` (count = `ids.len()`,
/// callee 00572310 supplies it) to the value of the nearest earlier slot with
/// the same id, written in place into `vals` (stride 4).  Zero ids map to 0.
/// A non-zero id with no earlier match is the fatal "unit\\%s" error path;
/// returns `Err(id & 0xffff)`.
pub fn w23a02_resolve_ids(vals: &mut [u32], ids: &[u32]) -> Result<(), u32> {
    for (i, &id) in ids.iter().enumerate() {
        let v = if id == 0 {
            0
        } else {
            let mut j = i as isize - 1;
            loop {
                if j < 0 {
                    return Err(id & 0xffff);
                }
                if ids[j as usize] == id {
                    break vals[j as usize];
                }
                j -= 1;
            }
        };
        if let Some(slot) = vals.get_mut(i) {
            *slot = v;
        }
    }
    Ok(())
}

/// `0x006fc280` (second half): rebases one block.  Header u16 low 15 bits is
/// the entry count `n`; entry `m` (1..=n) has a u32 at byte `8*m + 2` whose
/// low 31 bits are an offset that becomes `(v & 0x7fffffff) + addr`.
/// Entries are processed from the last to the first.
pub fn w23a02_rebase_block(block: &mut [u8], addr: u32) {
    if block.len() < 2 {
        return;
    }
    let n = (u16::from_le_bytes([block[0], block[1]]) & 0x7fff) as usize;
    for m in (1..=n).rev() {
        let o = 8 * m + 2;
        if o + 4 > block.len() {
            continue;
        }
        let v = u32::from_le_bytes([block[o], block[o + 1], block[o + 2], block[o + 3]]);
        let r = (v & 0x7fff_ffff).wrapping_add(addr);
        block[o..o + 4].copy_from_slice(&r.to_le_bytes());
    }
}

/// `0x006fc280`: after resolving ids (see 006fc4d0 logic) walks the pointer
/// list from last to first and rebases each block once; a block whose address
/// already appears among the earlier slots is skipped.  `rebase(i)` is called
/// for each slot index to process, in the original order.
pub fn w23a02_rebase_unique(addrs: &[u32], mut rebase: impl FnMut(usize)) {
    for i in (0..addrs.len()).rev() {
        if addrs[..i].contains(&addrs[i]) {
            continue;
        }
        rebase(i);
    }
}

/// `0x006fc650`: slot-row base byte offset: `0xd70 + (idx + game * 0x12) * 0x30`.
pub fn w23a02_slot_row_offset(game: u32, idx: u8) -> usize {
    0xd70 + (idx as usize + game as usize * 0x12) * 0x30
}

/// `0x006fc650`: when the net gate (`FUN_00748d60`) is 0 and `idx < 10`,
/// returns the row's first entry when non-zero (passed to FUN_005b4d20).
/// A row holds 12 u32 entries.
pub fn w23a02_slot_head(gate: i32, idx: u8, row: &[u32; 12]) -> Option<u32> {
    if gate != 0 || idx >= 10 {
        return None;
    }
    if row[0] != 0 { Some(row[0]) } else { None }
}

/// `0x006fc7e0`: same as 006fc650 but takes a 32-bit parameter whose low
/// byte is the index; the caller then always runs the 006fc940 clear.
pub fn w23a02_slot_head_wide(gate: i32, param: u32, row: &[u32; 12]) -> Option<u32> {
    w23a02_slot_head(gate, (param & 0xff) as u8, row)
}

/// `0x00708000`: releases the handle if non-zero via `release`, then clears it.
pub fn w23a02_release_handle(h: &mut u32, mut release: impl FnMut(u32)) {
    if *h != 0 {
        release(*h);
    }
    *h = 0;
}

/// `0x00708030`: seven handles at stride 5 u32; each non-zero handle is
/// released and cleared.
pub fn w23a02_release_handles(table: &mut [u32], mut release: impl FnMut(u32)) {
    for k in 0..7 {
        if let Some(h) = table.get_mut(k * 5) {
            w23a02_release_handle(h, &mut release);
        }
    }
}

/// `0x006f0870`: bounded append.  `len` = bounded strlen of `dst` (cap
/// `cap`); when `len != cap`, copies `min(strlen(src), cap-1-len)` bytes of
/// `src` at `dst[len..]` and returns that count (None when full).
pub fn w23a02_bounded_append(dst: &mut [u8], src: &[u8], cap: usize) -> Option<usize> {
    let len = dst
        .iter()
        .take(cap)
        .position(|&b| b == 0)
        .unwrap_or(cap.min(dst.len()));
    if len == cap {
        return None;
    }
    let slen = src.iter().position(|&b| b == 0).unwrap_or(src.len());
    let n = slen.min((cap - 1).wrapping_sub(len));
    let n = n.min(dst.len().saturating_sub(len));
    dst[len..len + n].copy_from_slice(&src[..n]);
    Some(n)
}

#[cfg(test)]
mod tests_wave23_agent02 {
    use super::*;

    #[test]
    fn resolve_ids_backward_search() {
        let ids = [5, 0, 5, 7];
        let mut v = [10, 11, 12, 13];
        // slot 0 (id 5) has no earlier slot to match.
        assert_eq!(w23a02_resolve_ids(&mut v, &ids), Err(5));
        let ids = [5, 0, 5];
        let mut v = [10, 11, 12];
        assert_eq!(w23a02_resolve_ids(&mut v, &ids), Err(5));
    }

    #[test]
    fn resolve_ids_ok() {
        let ids = [0, 0, 0];
        let mut v = [1, 2, 3];
        assert!(w23a02_resolve_ids(&mut v, &ids).is_ok());
        assert_eq!(v, [0, 0, 0]);
    }

    #[test]
    fn rebase_block_entries() {
        let mut b = vec![0u8; 26];
        b[0] = 2;
        b[10..14].copy_from_slice(&0x8000_0010u32.to_le_bytes());
        b[18..22].copy_from_slice(&0x20u32.to_le_bytes());
        w23a02_rebase_block(&mut b, 0x1000);
        assert_eq!(u32::from_le_bytes(b[10..14].try_into().unwrap()), 0x1010);
        assert_eq!(u32::from_le_bytes(b[18..22].try_into().unwrap()), 0x1020);
    }

    #[test]
    fn rebase_unique_skips_dups() {
        let mut seen = vec![];
        w23a02_rebase_unique(&[1, 2, 1, 3], |i| seen.push(i));
        assert_eq!(seen, vec![3, 1, 0]);
    }

    #[test]
    fn slot_rows() {
        assert_eq!(w23a02_slot_row_offset(0, 0), 0xd70);
        assert_eq!(w23a02_slot_row_offset(1, 2), 0xd70 + 20 * 0x30);
        let mut row = [0u32; 12];
        assert_eq!(w23a02_slot_head(0, 3, &row), None);
        row[0] = 9;
        assert_eq!(w23a02_slot_head(0, 9, &row), Some(9));
        assert_eq!(w23a02_slot_head(0, 10, &row), None);
        assert_eq!(w23a02_slot_head(1, 0, &row), None);
        assert_eq!(w23a02_slot_head_wide(0, 0x103, &row), Some(9));
    }

    #[test]
    fn release_handles() {
        let mut t = [0u32; 40];
        t[0] = 1;
        t[10] = 2;
        t[30] = 3;
        t[35] = 4; // index 7 is outside the seven slots
        let mut got = vec![];
        w23a02_release_handles(&mut t, |h| got.push(h));
        assert_eq!(got, vec![1, 2, 3]);
        assert_eq!((t[0], t[10], t[30], t[35]), (0, 0, 0, 4));
        let mut h = 5;
        w23a02_release_handle(&mut h, |_| {});
        assert_eq!(h, 0);
    }

    #[test]
    fn bounded_append_clamps() {
        let mut d = *b"ab\0\0\0\0\0\0";
        assert_eq!(w23a02_bounded_append(&mut d, b"xyz\0", 8), Some(3));
        assert_eq!(&d[..5], b"abxyz");
        let mut d = *b"ab\0\0";
        assert_eq!(w23a02_bounded_append(&mut d, b"xyz\0", 4), Some(1));
        let mut d = *b"abcd";
        assert_eq!(w23a02_bounded_append(&mut d, b"x\0", 4), None);
    }
}
