#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent36.rs
// ===========================================================================
// Wave 33 agent 36: zlib `_tr_align` body and a bounded retry gate.

/// Deflate bit-writer state used by `w33a36_tr_align` (offsets 0x14, 0x16b8,
/// 0x16bc of the retail stream state; pending buffer at +8).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct W33a36Deflate {
    pub pending_buf: Vec<u8>,
    pub pending: usize,
    pub bi_buf: u16,
    pub bi_valid: i32,
}

/// `0x00c76860`: zlib `_tr_align` (send the empty static block header and
/// END_BLOCK code, then `bi_flush` via the caller supplied closure
/// `0xc7_6ed0`).
pub fn w33a36_tr_align(s: &mut W33a36Deflate, bi_flush: impl FnOnce(&mut W33a36Deflate)) {
    let valid = s.bi_valid;
    let mut buf: u16 = (2u32.wrapping_shl(valid as u32 & 31) as u16) | s.bi_buf;
    if valid < 14 {
        s.bi_valid = valid + 3;
    } else {
        s.bi_buf = buf;
        s.pending_buf[s.pending] = buf as u8;
        s.pending += 1;
        s.pending_buf[s.pending] = (s.bi_buf >> 8) as u8;
        s.pending += 1;
        let old = s.bi_valid as u16;
        s.bi_valid -= 13;
        buf = (2u32 >> (0x10u32.wrapping_sub(old as u32) & 31)) as u16;
    }
    s.bi_buf = buf;
    if s.bi_valid > 9 {
        s.pending_buf[s.pending] = s.bi_buf as u8;
        s.pending += 1;
        s.pending_buf[s.pending] = (s.bi_buf >> 8) as u8;
        s.pending += 1;
        s.bi_valid -= 9;
        s.bi_buf = 0;
    } else {
        s.bi_valid += 7;
    }
    bi_flush(s);
}

/// `0x00c810c0`: waits up to 10 polls (sleeping 100 ms each) for the busy
/// flag of slot `idx` to clear, returning -1 on timeout; if the ready flag
/// is zero then returns `start(idx)` (callee `0xc8_0ce0`), else 0.
pub fn w33a36_wait_and_start(
    busy: &[u8],
    ready: &[u8],
    idx: usize,
    mut sleep_ms: impl FnMut(u32),
    start: impl FnOnce() -> i32,
) -> i32 {
    if busy[idx] != 0 {
        // The flag is only re-read from shared memory in the retail code;
        // with a fixed slice it can never clear, so all ten waits expire.
        for _ in 0..10 {
            sleep_ms(100);
        }
        return -1;
    }
    if ready[idx] == 0 {
        return start();
    }
    0
}

#[cfg(test)]
mod tests_wave33_agent36 {
    use super::*;

    #[test]
    fn align_low_bits() {
        let mut s = W33a36Deflate {
            pending_buf: vec![0; 8],
            ..Default::default()
        };
        w33a36_tr_align(&mut s, |_| {});
        assert_eq!(s.bi_valid, 10);
    }

    #[test]
    fn align_overflow_writes() {
        let mut s = W33a36Deflate {
            pending_buf: vec![0; 8],
            bi_valid: 14,
            ..Default::default()
        };
        w33a36_tr_align(&mut s, |_| {});
        assert_eq!(s.pending, 2);
    }

    #[test]
    fn wait_times_out() {
        let mut n = 0;
        assert_eq!(w33a36_wait_and_start(&[1], &[0], 0, |_| n += 1, || 5), -1);
        assert_eq!(n, 10);
        assert_eq!(w33a36_wait_and_start(&[0], &[0], 0, |_| {}, || 5), 5);
        assert_eq!(w33a36_wait_and_start(&[0], &[1], 0, |_| {}, || 5), 0);
    }
}
