#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent47.rs
// ===========================================================================
// Wave 33 agent 47: bounded UTF-16 output sink helper.

/// Output sink state read through the context pointer at `0x00c9b3d1`.
pub struct W33a47Sink<'a> {
    /// Destination buffer; the write cursor is `pos` (offset 0 of the record).
    pub buf: &'a mut [u16],
    pub pos: usize,
    /// Offset +4: capacity limit.
    pub limit: i32,
    /// Offset +8: current count.
    pub count: i32,
    /// Offset +0xc: flag returned/used when the sink is full.
    pub full_flag: u8,
}

/// `0x00c9b3d1`: append one UTF-16 unit to a bounded sink.  When the
/// count equals the limit the unit is dropped: `total` becomes -1 if the
/// flag is zero, otherwise it is incremented, and the flag is returned.
/// Otherwise `total` and `count` are incremented, the unit is stored, the
/// cursor advances, and 1 is returned.
pub fn w33a47_put_u16(sink: &mut W33a47Sink, unit: u16, total: &mut i32) -> u8 {
    if sink.count == sink.limit {
        if sink.full_flag == 0 {
            *total = -1;
        } else {
            *total = total.wrapping_add(1);
        }
        sink.full_flag
    } else {
        *total = total.wrapping_add(1);
        sink.count += 1;
        sink.buf[sink.pos] = unit;
        sink.pos += 1;
        1
    }
}

#[cfg(test)]
mod tests_wave33_agent47 {
    use super::*;

    #[test]
    fn put_and_full() {
        let mut b = [0u16; 2];
        let mut s = W33a47Sink {
            buf: &mut b,
            pos: 0,
            limit: 1,
            count: 0,
            full_flag: 0,
        };
        let mut t = 0;
        assert_eq!(w33a47_put_u16(&mut s, 7, &mut t), 1);
        assert_eq!((t, s.pos), (1, 1));
        assert_eq!(w33a47_put_u16(&mut s, 8, &mut t), 0);
        assert_eq!(t, -1);
        s.full_flag = 1;
        t = 5;
        assert_eq!(w33a47_put_u16(&mut s, 8, &mut t), 1);
        assert_eq!(t, 6);
        assert_eq!(b[0], 7);
    }
}
