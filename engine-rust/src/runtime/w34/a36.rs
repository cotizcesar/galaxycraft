#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent36.rs
// ===========================================================================
/// `0x00535ef0`: readiness predicate over a record.  Returns false when bit 3
/// of the flag byte at offset `0x34` is set; otherwise true only if the
/// dwords at indices 1, 4, 5 and 6 (indices 0, 2 and 3 are ignored) are all
/// non-zero.  `rec` must cover at least 0x35 bytes.
pub fn w34a36_record_ready(rec: &[u8]) -> bool {
    if rec.len() < 0x35 || rec[0x34] & 8 != 0 {
        return false;
    }
    [1usize, 4, 5, 6].iter().all(|&i| {
        let o = i * 4;
        u32::from_le_bytes([rec[o], rec[o + 1], rec[o + 2], rec[o + 3]]) != 0
    })
}

#[cfg(test)]
mod tests_wave34_agent36 {
    use super::*;

    fn full() -> Vec<u8> {
        let mut v = vec![0u8; 0x38];
        for i in [1usize, 4, 5, 6] {
            v[i * 4] = 1;
        }
        v
    }

    #[test]
    fn ready_cases() {
        let mut v = full();
        assert!(w34a36_record_ready(&v));
        v[0x34] = 8;
        assert!(!w34a36_record_ready(&v));
        v[0x34] = 7;
        assert!(w34a36_record_ready(&v));
        v[5 * 4] = 0;
        assert!(!w34a36_record_ready(&v));
        assert!(!w34a36_record_ready(&[0u8; 4]));
    }
}
