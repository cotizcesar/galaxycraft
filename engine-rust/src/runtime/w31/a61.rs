#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent61.rs
// ===========================================================================
/// `0x0098a060`: scans an allocation bitmap for a run of `count` clear bits.
/// Starts at bit `start`, stops at `limit` or `bit_len` (the bitmap's bit
/// capacity, field `+0x10`).  Aligned all-ones dwords are skipped 32 bits at
/// a time.  Returns the first bit of the run, or -1.
pub fn w31a61_find_clear_run(
    bitmap: &[u8],
    bit_len: i32,
    start: i32,
    limit: i32,
    count: i32,
) -> i32 {
    let mut pos = start as u32;
    let mut idx = ((start as i32) >> 3) as usize;
    let mut mask: i32 = 1 << (start & 7);
    let mut run: i32 = 0;
    if count > 0 {
        loop {
            if limit <= pos as i32 || bit_len <= pos as i32 {
                break;
            }
            let byte = bitmap.get(idx).copied().unwrap_or(0xff) as i32;
            let dword_full = pos & 0x1f == 0
                && bitmap
                    .get(idx..idx + 4)
                    .is_some_and(|d| d.iter().all(|&b| b == 0xff));
            if byte & mask == 0 && !dword_full {
                run += 1;
            } else {
                run = 0;
            }
            if dword_full {
                idx += 4;
                pos = pos.wrapping_add(0x20);
            } else {
                pos = pos.wrapping_add(1);
                mask = mask.wrapping_mul(2);
                if pos & 7 == 0 {
                    mask = 1;
                    idx += 1;
                }
            }
            if run >= count {
                break;
            }
        }
    }
    if run == count {
        (pos as i32).wrapping_sub(count)
    } else {
        -1
    }
}

#[cfg(test)]
mod tests_wave31_agent61 {
    use super::*;

    #[test]
    fn finds_runs() {
        let bm = [0xffu8, 0xff, 0xff, 0xff, 0x0f, 0x00, 0x00, 0x00];
        assert_eq!(w31a61_find_clear_run(&bm, 64, 0, 64, 4), 36);
        assert_eq!(w31a61_find_clear_run(&bm, 64, 0, 64, 28), 36);
        assert_eq!(w31a61_find_clear_run(&bm, 64, 0, 64, 29), -1);
        assert_eq!(w31a61_find_clear_run(&bm, 64, 0, 40, 5), -1);
        assert_eq!(w31a61_find_clear_run(&bm, 64, 0, 64, 0), 0);
    }
}
