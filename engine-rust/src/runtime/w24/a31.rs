#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent31.rs
// ===========================================================================
// Wave 24 agent 31 translations (checksum and tagged-link helpers).

/// `0x007968a0`: Fletcher-style running sum over `data`, walked from the last
/// byte to the first.  Returns `(sum2 * 0x8080808081 >> 39) & 0xff | s1 << 8`.
pub fn w24a31_fletcher_back(data: &[u8]) -> u32 {
    let mut s1: u32 = 0;
    let mut s2: u32 = 0;
    for &b in data.iter().rev() {
        let t = u32::from(b) + s1;
        s1 = if t < 0xff { t } else { t - 0xff };
        s2 = s2.wrapping_add(s1);
    }
    ((((s2 as u64).wrapping_mul(0x80_8080_8081) >> 0x27) as u32) & 0xff) | (s1 << 8)
}

/// `0x007968f0`: check-byte pair for a packet whose u16 length sits at offset 2.
/// The summed region is `buf[2 .. 2 + len - 2]`.  Returns `None` if the
/// buffer is too short.
pub fn w24a31_packet_check_bytes(buf: &[u8]) -> Option<u16> {
    let len = u16::from_le_bytes([*buf.get(2)?, *buf.get(3)?]) as usize;
    let n = len.wrapping_sub(2) & 0xffff;
    let region = buf.get(2..2 + n)?;
    let sum = w24a31_fletcher_back(region);
    let hi = (sum & 0xffff) >> 8;
    let c0 = (!(((sum & 0xff) + hi) % 0xff)) as u8;
    let c1 = (!((u32::from(c0) + hi) % 0xff)) as u8;
    Some((u16::from(c0) << 8) | u16::from(c1))
}

/// `0x00796870`: tagged link pointer; a value with bit 0 set or zero is the
/// list terminator (returns 0), otherwise the pointer itself.
pub fn w24a31_untag_link(v: u32) -> u32 {
    if v & 1 == 0 && v != 0 { v } else { 0 }
}

#[cfg(test)]
mod tests_wave24_agent31 {
    use super::*;

    #[test]
    fn fletcher() {
        assert_eq!(w24a31_fletcher_back(&[]), 0);
        assert_eq!(w24a31_fletcher_back(&[1]), (1 << 8) | 1);
        assert_eq!(w24a31_fletcher_back(&[0xff]) >> 8, 0);
        assert_eq!(w24a31_fletcher_back(&[1, 2]) >> 8, 3);
    }

    #[test]
    fn check_bytes() {
        assert_eq!(w24a31_packet_check_bytes(&[0, 0]), None);
        let buf = [0u8, 0, 4, 0, 1, 2];
        let sum = w24a31_fletcher_back(&buf[2..4]);
        let hi = sum >> 8;
        let c0 = (!(((sum & 0xff) + hi) % 0xff)) as u8;
        let r = w24a31_packet_check_bytes(&buf).unwrap();
        assert_eq!((r >> 8) as u8, c0);
    }

    #[test]
    fn untag() {
        assert_eq!(w24a31_untag_link(0), 0);
        assert_eq!(w24a31_untag_link(5), 0);
        assert_eq!(w24a31_untag_link(8), 8);
    }
}
