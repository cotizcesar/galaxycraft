#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent109.rs
// ===========================================================================
// Wave 31 agent 109 (runtime.rs window).

/// `0x00c4e2d0`: tail-calls memset(dst, 0, count * 8); zeroes `count`
/// 8-byte elements at the start of `dst` (clamped to the slice length).
pub fn w31a109_zero_qwords(dst: &mut [u8], count: usize) {
    let n = count.saturating_mul(8).min(dst.len());
    for b in &mut dst[..n] {
        *b = 0;
    }
}

#[cfg(test)]
mod tests_wave31_agent109 {
    use super::*;

    #[test]
    fn zero_qwords_clears_prefix() {
        let mut v = [0xffu8; 24];
        w31a109_zero_qwords(&mut v, 2);
        assert!(v[..16].iter().all(|&b| b == 0));
        assert!(v[16..].iter().all(|&b| b == 0xff));
        w31a109_zero_qwords(&mut v, 100);
        assert!(v.iter().all(|&b| b == 0));
    }
}
