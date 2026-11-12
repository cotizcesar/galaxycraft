#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave30/agent16.rs
// ===========================================================================
// Wave 30 agent 16: small field-layout helpers from the encoded-file module.

/// Range record filled by `0x00b31830` (field offsets in bytes).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct W30a16Range {
    /// +0x00
    pub tag: u32,
    /// +0x08 low / +0x0c high (u64 start)
    pub start: u64,
    /// +0x10 low / +0x14 high (u64 end)
    pub end: u64,
    /// +0x18 low / +0x1c high (u64 end - start)
    pub length: u64,
}

/// `0x00b31830`: stores the tag and the two 64-bit bounds (low word first,
/// as `param_2`/`param_3` and `param_4`/`param_5`) and computes the 64-bit
/// difference `end - start` with borrow, as the original does.
pub fn w30a16_set_range(
    tag: u32,
    end_lo: u32,
    end_hi: u32,
    start_lo: u32,
    start_hi: u32,
) -> W30a16Range {
    let end = (u64::from(end_hi) << 32) | u64::from(end_lo);
    let start = (u64::from(start_hi) << 32) | u64::from(start_lo);
    W30a16Range {
        tag,
        start,
        end,
        length: end.wrapping_sub(start),
    }
}

/// Buffer state reset by `0x00b2e3c0` (offsets +0x170/+0x180/+0x184/+0x188).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct W30a16Buffer {
    /// +0x170 handle, released through a callback.
    pub handle: Option<u32>,
    /// +0x180 data buffer.
    pub data: Option<Vec<u8>>,
    /// +0x184 buffer length.
    pub len: u32,
    /// +0x188 read position.
    pub pos: u32,
}

/// `0x00b2e3c0`: clears length and position, drops the data buffer and
/// takes the handle (returned so the caller can release it); result is 0.
pub fn w30a16_reset_buffer(b: &mut W30a16Buffer) -> (u32, Option<u32>) {
    b.len = 0;
    b.pos = 0;
    b.data = None;
    (0, b.handle.take())
}

#[cfg(test)]
mod tests_wave30_agent16 {
    use super::*;

    #[test]
    fn range_borrow() {
        let r = w30a16_set_range(7, 5, 1, 10, 0);
        assert_eq!(r.length, (1u64 << 32 | 5) - 10);
        assert_eq!(r.tag, 7);
        assert_eq!(r.start, 10);
    }

    #[test]
    fn reset_buffer() {
        let mut b = W30a16Buffer {
            handle: Some(3),
            data: Some(vec![1]),
            len: 4,
            pos: 2,
        };
        assert_eq!(w30a16_reset_buffer(&mut b), (0, Some(3)));
        assert_eq!(b, W30a16Buffer::default());
    }
}
