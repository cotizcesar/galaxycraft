#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent41.rs
// ===========================================================================
// Wave 24 agent 41: container helpers recovered from `runtime.rs` glue.

/// `0x007c2dc0`: appends the first dword of each 28-byte (7 dword) source
/// record in `src` to `dst`, as the push-back loop of the retail vector copy.
pub fn w24a41_copy_first_dwords(src: &[u32], dst: &mut Vec<u32>) {
    for rec in src.chunks_exact(7) {
        dst.push(rec[0]);
    }
}

/// `0x007c3600`: walks a block deque (16 entries per block, `map_size` a
/// power of two) from `offset` for `count` slots and calls `f` for each
/// non-null element; returns the number of calls.
pub fn w24a41_deque_destroy_all(
    blocks: &[Vec<u32>],
    map_size: usize,
    offset: usize,
    count: usize,
    mut f: impl FnMut(u32),
) -> usize {
    let mut n = 0;
    for i in offset..offset + count {
        let block = (i >> 4) & map_size.wrapping_sub(1);
        let v = blocks[block][i & 0xf];
        if v != 0 {
            f(v);
            n += 1;
        }
    }
    n
}

/// `0x007c3a20`: releases the shared-pointer slots (stride 8, control block
/// at +4): atomically decrements each use count; returns how many reached
/// zero (the retail code then calls the control block's dispose slot).
/// `None` slots are skipped.
pub fn w24a41_release_shared(counts: &mut [Option<i32>]) -> usize {
    let mut disposed = 0;
    for c in counts.iter_mut().flatten() {
        let old = *c;
        *c -= 1;
        if old == 1 {
            disposed += 1;
        }
    }
    disposed
}

#[cfg(test)]
mod tests_wave24_agent41 {
    use super::*;

    #[test]
    fn copy_first() {
        let mut v = vec![];
        let src: Vec<u32> = (0..14).collect();
        w24a41_copy_first_dwords(&src, &mut v);
        assert_eq!(v, vec![0, 7]);
    }

    #[test]
    fn deque() {
        let blocks = vec![(0..16).collect::<Vec<u32>>(), vec![5; 16]];
        let mut seen = vec![];
        let n = w24a41_deque_destroy_all(&blocks, 2, 14, 4, |v| seen.push(v));
        assert_eq!(n, 4);
        assert_eq!(seen, vec![14, 15, 5, 5]);
    }

    #[test]
    fn shared() {
        let mut c = [Some(1), None, Some(2)];
        assert_eq!(w24a41_release_shared(&mut c), 1);
        assert_eq!(c, [Some(0), None, Some(1)]);
    }
}
