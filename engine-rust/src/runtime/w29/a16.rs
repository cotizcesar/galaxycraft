#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent16.rs
// ===========================================================================
// Wave 29 agent 16: TACT encoding-table header and VFS tree helpers.

/// Parsed and validated encoding-table header layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W29a16EncodingLayout {
    pub ckey_size: u32,
    pub ekey_size: u32,
    pub ckey_page_size: u32,
    pub ekey_page_size: u32,
    pub ckey_page_count: u32,
    pub ekey_page_count: u32,
    pub espec_size: u32,
    pub espec_off: u32,
    pub ckey_index_off: u32,
    pub ckey_pages_size: u32,
    pub ckey_pages_off: u32,
    pub ckey_pages_end: u32,
    pub ekey_index_size: u32,
    pub ekey_index_off: u32,
    pub ekey_pages_size: u32,
    pub ekey_pages_off: u32,
    pub ekey_pages_end: u32,
    pub trailer: i32,
}

/// `0x00ade020`: validates the 22-byte `EN` encoding-table header (read
/// status must be 0 by the caller) and derives the section layout.
/// Returns `None` when magic/version/zero byte or any size is invalid.
pub fn w29a16_parse_encoding_header(hdr: &[u8], total_size: i32) -> Option<W29a16EncodingLayout> {
    if hdr.len() < 22 || hdr[0] != b'E' || hdr[1] != b'N' || hdr[2] != 1 || hdr[17] != 0 {
        return None;
    }
    let be16 = |i: usize| u16::from_be_bytes([hdr[i], hdr[i + 1]]) as u32;
    let be32 = |i: usize| u32::from_be_bytes([hdr[i], hdr[i + 1], hdr[i + 2], hdr[i + 3]]);
    let ck = hdr[3] as u32;
    let ek = hdr[4] as u32;
    let (cp, ep) = (be16(5), be16(7));
    let (cn, en, espec) = (be32(9), be32(13), be32(18));
    if ck == 0 || ck > 0x20 || ek == 0 || ek > 0x20 || cp == 0 || ep == 0 {
        return None;
    }
    if cn == 0 || en == 0 || espec == 0 {
        return None;
    }
    let (cpb, epb) = (cp.wrapping_mul(0x400), ep.wrapping_mul(0x400));
    let espec_off = espec.wrapping_add(0x16);
    let ckey_index_size = (ck + 0x10).wrapping_mul(cn);
    let ckey_pages_off = ckey_index_size.wrapping_add(espec_off);
    let ckey_pages_size = cpb.wrapping_mul(cn);
    let ckey_pages_end = ckey_pages_size.wrapping_add(ckey_pages_off);
    let ekey_index_size = (ek + 0x10).wrapping_mul(en);
    let ekey_pages_off = ekey_index_size.wrapping_add(ckey_pages_end);
    let ekey_pages_size = epb.wrapping_mul(en);
    let ekey_pages_end = ekey_pages_size.wrapping_add(ekey_pages_off);
    Some(W29a16EncodingLayout {
        ckey_size: ck,
        ekey_size: ek,
        ckey_page_size: cpb,
        ekey_page_size: epb,
        ckey_page_count: cn,
        ekey_page_count: en,
        espec_size: espec,
        espec_off,
        ckey_index_off: ckey_index_size,
        ckey_pages_size,
        ckey_pages_off,
        ckey_pages_end,
        ekey_index_size,
        ekey_index_off: ckey_pages_end,
        ekey_pages_size,
        ekey_pages_off,
        ekey_pages_end,
        trailer: total_size.wrapping_sub(ekey_pages_end as i32),
    })
}

/// Zeroed result/status record initialised by `0x00adf3c0` (nine dwords
/// plus a 0xff-byte message buffer at +0x21; status field +0x10 = 0x16).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W29a16Status {
    pub words: [u32; 8],
    pub flag: u8,
    pub text: [u8; 0xff],
}

/// `0x00adf3c0`: constructs the default status record.
pub fn w29a16_status_init() -> W29a16Status {
    let mut words = [0u32; 8];
    words[4] = 0x17;
    W29a16Status {
        words,
        flag: 0,
        text: [0; 0xff],
    }
}

/// `0x00ae1240`: walks child trees calling `op`; stops at the first result
/// that is neither 0 nor 0x16. Returns that result (or 0 if exhausted);
/// `None` when a child slot is missing (error path of the original).
pub fn w29a16_first_failing_child<T>(
    children: &[Option<T>],
    mut op: impl FnMut(&T) -> i32,
) -> Option<i32> {
    for c in children {
        let r = op(c.as_ref()?);
        if r != 0x16 && r != 0 {
            return Some(r);
        }
    }
    Some(0)
}

#[cfg(test)]
mod tests_wave29_agent16 {
    use super::*;

    fn hdr() -> [u8; 22] {
        let mut h = [0u8; 22];
        h[0] = b'E';
        h[1] = b'N';
        h[2] = 1;
        h[3] = 16;
        h[4] = 9;
        h[6] = 4; // ckey page = 4 KiB
        h[8] = 4;
        h[12] = 2;
        h[16] = 3;
        h[21] = 5;
        h
    }

    #[test]
    fn header_layout() {
        let l = w29a16_parse_encoding_header(&hdr(), 100_000).unwrap();
        assert_eq!(l.espec_off, 5 + 0x16);
        assert_eq!(l.ckey_pages_off, 27 + 32 * 2);
        assert_eq!(l.ckey_page_size, 4096);
        assert_eq!(l.ekey_pages_end, l.ekey_pages_off + 4096 * 3);
        assert_eq!(l.trailer, 100_000 - l.ekey_pages_end as i32);
    }

    #[test]
    fn header_rejects() {
        let mut h = hdr();
        h[2] = 2;
        assert!(w29a16_parse_encoding_header(&h, 0).is_none());
        let mut h = hdr();
        h[3] = 0x21;
        assert!(w29a16_parse_encoding_header(&h, 0).is_none());
        let mut h = hdr();
        h[17] = 1;
        assert!(w29a16_parse_encoding_header(&h, 0).is_none());
        assert!(w29a16_parse_encoding_header(&hdr()[..10], 0).is_none());
    }

    #[test]
    fn status_default() {
        let s = w29a16_status_init();
        assert_eq!(s.words[4], 0x17);
        assert!(s.text.iter().all(|&b| b == 0));
    }

    #[test]
    fn child_walk() {
        let v = [Some(0), Some(0x16), Some(7), Some(9)];
        assert_eq!(w29a16_first_failing_child(&v, |x| *x), Some(7));
        assert_eq!(
            w29a16_first_failing_child(&[Some(0), Some(0x16)], |x| *x),
            Some(0)
        );
        assert_eq!(w29a16_first_failing_child(&[Some(0), None], |x| *x), None);
    }
}
