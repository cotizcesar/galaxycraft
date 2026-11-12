#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent102.rs
// ===========================================================================
// Wave 31 agent 102: pm_dx11 command-queue teardown and descriptor helpers.

/// Unpacked descriptor built by `0x00c2a3c0` before invoking the callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W31a102Desc {
    pub low2: u8,
    pub mid4: u8,
    pub w1: u32,
    pub w2: u32,
    pub w3: u32,
}

/// `0x00c2a3c0`: unpacks the record at +0x18 (bits 0..1, bits 2..5) and the
/// words at +0x1c/+0x20/+0x24 into the descriptor passed to the callback.
pub fn w31a102_unpack_desc(flags: u32, w1: u32, w2: u32, w3: u32) -> W31a102Desc {
    W31a102Desc {
        low2: (flags & 3) as u8,
        mid4: ((flags >> 2) & 0xf) as u8,
        w1,
        w2,
        w3,
    }
}

/// `0x00c2b660`: CmdQueue destructor body. Releases the object at word 10
/// (if any) through `release`, then zeroes words 3..=7. Words are 32-bit
/// slots of the C++ object.
pub fn w31a102_cmdqueue_teardown(words: &mut [u32], mut release: impl FnMut(u32)) {
    if words[10] != 0 {
        release(words[10]);
        words[10] = 0;
    }
    for w in &mut words[3..=7] {
        *w = 0;
    }
}

/// `0x00c2b710`: ComputeCmdQueue destructor body. Releases words 10, 5 and 7
/// (in the original order 10, 5, 7), clearing each, then zeroes words 4 and 6.
pub fn w31a102_compute_queue_teardown(words: &mut [u32], mut release: impl FnMut(u32)) {
    for idx in [10usize, 5, 7] {
        if words[idx] != 0 {
            release(words[idx]);
            words[idx] = 0;
        }
    }
    words[4] = 0;
    words[6] = 0;
}

/// `0x00c2b7d0`: CopyCmdQueue destructor body. Releases words 10, 6 and 7
/// in order, clearing each.
pub fn w31a102_copy_queue_teardown(words: &mut [u32], mut release: impl FnMut(u32)) {
    for idx in [10usize, 6, 7] {
        if words[idx] != 0 {
            release(words[idx]);
            words[idx] = 0;
        }
    }
}

#[cfg(test)]
mod tests_wave31_agent102 {
    use super::*;

    #[test]
    fn unpack() {
        let d = w31a102_unpack_desc(0x3f, 1, 2, 3);
        assert_eq!((d.low2, d.mid4, d.w1, d.w2, d.w3), (3, 0xf, 1, 2, 3));
        assert_eq!(w31a102_unpack_desc(0x40, 0, 0, 0).mid4, 0);
    }

    #[test]
    fn cmdqueue() {
        let mut w = [9u32; 12];
        let mut got = vec![];
        w31a102_cmdqueue_teardown(&mut w, |v| got.push(v));
        assert_eq!(got, vec![9]);
        assert_eq!(w[10], 0);
        assert!(w[3..=7].iter().all(|&x| x == 0));
        assert_eq!(w[8], 9);
    }

    #[test]
    fn compute_and_copy() {
        let mut w = [1u32; 12];
        w[5] = 0;
        let mut got = vec![];
        w31a102_compute_queue_teardown(&mut w, |v| got.push(v));
        assert_eq!(got.len(), 2);
        assert_eq!((w[4], w[6], w[7], w[10]), (0, 0, 0, 0));
        let mut w = [2u32; 12];
        let mut n = 0;
        w31a102_copy_queue_teardown(&mut w, |_| n += 1);
        assert_eq!(n, 3);
        assert_eq!((w[6], w[7], w[10], w[5]), (0, 0, 0, 2));
    }
}
