#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent03.rs
// ===========================================================================
// Wave 24 translation batch 03.
//
// Skipped (no recoverable logic or opaque globals): FUN_0075b1e0,
// FUN_0075b350, FUN_0075c860, FUN_0075cb40, FUN_0075d1d0, FUN_00762e40,
// FUN_00767d30, FUN_00769dc0, FUN_0076b120, FUN_0076dab0, FUN_0076eb70,
// FUN_00771b00, FUN_00771d90, FUN_00771f10, FUN_00771fd0, FUN_007723b0.

/// `0x0075d970`: true when the region record `idx` (record stride `0x40`,
/// field at `+0x449fc` of the map block) holds the sentinel `0x1ffd`.
/// `records` is the byte image starting at `+0x449fc`; `idx` is the already
/// resolved (masked to 16 bits) region id.
pub fn w24a03_region_is_sentinel(records: &[u8], idx: u16) -> bool {
    let off = idx as usize * 0x40;
    match records.get(off..off + 2) {
        Some(b) => u16::from_le_bytes([b[0], b[1]]) == 0x1ffd,
        None => false,
    }
}

/// `0x00767ff0`: depth-first walk over region neighbours.  A region is
/// visited when it is non-zero, not yet flagged in `visited`, and
/// `depth <= max_depth`; it is flagged, `on_visit` is called, and unless the
/// region id appears in `stop_list` (the global u16 list of `count` entries)
/// its neighbours (via `neighbours`) are recursed into with `depth + 1`.
pub fn w24a03_region_walk(
    region: u16,
    depth: u32,
    visited: &mut [u32],
    max_depth: u32,
    stop_list: &[u16],
    neighbours: &dyn Fn(u16) -> Vec<u16>,
    on_visit: &mut dyn FnMut(u16),
) {
    let r = region as usize;
    if region == 0 || r >= visited.len() || visited[r] != 0 || depth > max_depth {
        return;
    }
    on_visit(region);
    visited[r] = 1;
    if stop_list.contains(&region) {
        return;
    }
    for n in neighbours(region) {
        w24a03_region_walk(
            n,
            depth + 1,
            visited,
            max_depth,
            stop_list,
            neighbours,
            on_visit,
        );
    }
}

/// `0x007661b0`: marks neighbouring regions.  The region used is `explicit`
/// when it is `< 0x9c4`, else `own_region` (the unit position region).  Every
/// neighbour whose id differs from `own_group` (unit field `+0x160`) gets its
/// dword at record offset `+0x44a04` (stride `0x40`) set to 1.  Returns the
/// number of records marked; `flags` is indexed by region id.
pub fn w24a03_mark_neighbours(
    flags: &mut [u32],
    explicit: u16,
    own_region: u16,
    own_group: u16,
    neighbours: &dyn Fn(u16) -> Vec<u16>,
) -> usize {
    let region = if explicit < 0x9c4 {
        explicit
    } else {
        own_region
    };
    let mut n = 0;
    for id in neighbours(region) {
        if id != own_group {
            if let Some(f) = flags.get_mut(id as usize) {
                *f = 1;
                n += 1;
            }
        }
    }
    n
}

/// `0x0076de60`: bounded append into a 32-byte name buffer.  Finds the
/// current length (`strnlen(dst, 0x20)`); if the buffer is full nothing
/// happens, else copies `min(len + strlen(src), 0x1f) - len` bytes of `src`
/// after it.  Returns the number of bytes copied.
pub fn w24a03_append_name(dst: &mut [u8; 0x20], src: &[u8]) -> usize {
    let len = dst.iter().position(|&b| b == 0).unwrap_or(0x20);
    if len == 0x20 {
        return 0;
    }
    let slen = src.iter().position(|&b| b == 0).unwrap_or(src.len());
    let n = (len + slen).min(0x1f) - len;
    dst[len..len + n].copy_from_slice(&src[..n]);
    n
}

#[cfg(test)]
mod tests_wave24_agent03 {
    use super::*;

    #[test]
    fn sentinel() {
        let mut r = vec![0u8; 0x80];
        r[0x40] = 0xfd;
        r[0x41] = 0x1f;
        assert!(w24a03_region_is_sentinel(&r, 1));
        assert!(!w24a03_region_is_sentinel(&r, 0));
        assert!(!w24a03_region_is_sentinel(&r, 9));
    }

    #[test]
    fn walk() {
        let mut vis = vec![0u32; 8];
        let mut seen = vec![];
        let nb = |r: u16| if r < 4 { vec![r + 1] } else { vec![] };
        w24a03_region_walk(1, 0, &mut vis, 1, &[], &nb, &mut |r| seen.push(r));
        assert_eq!(seen, vec![1, 2]);
        let mut vis = vec![0u32; 8];
        let mut seen = vec![];
        w24a03_region_walk(1, 0, &mut vis, 9, &[3], &nb, &mut |r| seen.push(r));
        assert_eq!(seen, vec![1, 2, 3]);
        w24a03_region_walk(0, 0, &mut vis, 9, &[], &nb, &mut |r| seen.push(r));
        assert_eq!(seen.len(), 3);
    }

    #[test]
    fn mark() {
        let mut f = vec![0u32; 8];
        let nb = |r: u16| if r == 5 { vec![1, 2, 3] } else { vec![7] };
        assert_eq!(w24a03_mark_neighbours(&mut f, 0x9c4, 5, 2, &nb), 2);
        assert_eq!(f, vec![0, 1, 0, 1, 0, 0, 0, 0]);
        assert_eq!(w24a03_mark_neighbours(&mut f, 4, 5, 2, &nb), 1);
        assert_eq!(f[7], 1);
    }

    #[test]
    fn append() {
        let mut d = [0u8; 0x20];
        d[..3].copy_from_slice(b"abc");
        assert_eq!(w24a03_append_name(&mut d, b"de\0"), 2);
        assert_eq!(&d[..6], b"abcde\0");
        let mut d = [b'x'; 0x20];
        assert_eq!(w24a03_append_name(&mut d, b"y"), 0);
        let mut d = [0u8; 0x20];
        assert_eq!(w24a03_append_name(&mut d, &[b'z'; 40]), 0x1f);
    }
}
