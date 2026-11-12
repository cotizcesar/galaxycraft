#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent40.rs
// ===========================================================================
// Wave 25 agent 40: MSVC `std::string` small-string / list-node initialisers.

/// MSVC small-string header: 32-bit words `[ptr, size, cap]` followed by the
/// inline 16-byte buffer.  Capacity `0x8000000f` marks "inline, empty".
pub const W25A40_SSO_CAP_FLAG: u32 = 0x8000_000f;

/// Inline small-string used by the initialisers below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W25a40Str {
    pub data_off: u32,
    pub size: u32,
    pub cap: u32,
    pub inline: [u8; 16],
}

/// `0x00876650`: builds an empty small string (size 0, cap `0x8000000f`,
/// inline byte 0, data pointer = inline buffer at word offset 3).
pub fn w25a40_str_init_876650() -> W25a40Str {
    W25a40Str {
        data_off: 12,
        size: 0,
        cap: W25A40_SSO_CAP_FLAG,
        inline: [0; 16],
    }
}

/// `0x00876860`: same empty-string initialiser as the sibling above
/// (unwind state differs only).
pub fn w25a40_str_init_876860() -> W25a40Str {
    w25a40_str_init_876650()
}

/// `0x008763c0`: zeroes the three leading words of a record.
pub fn w25a40_zero3_8763c0(words: &mut [u32]) {
    words[0] = 0;
    words[2] = 0;
    words[1] = 0;
}

/// `0x00876450`: initialises a list head (offsets 0/4 self-linked, count at 8,
/// zero at 0x10) followed by four empty small strings at 0x0c, 0x28, 0x44,
/// 0x60 whose size words are at 0x10, 0x2c, 0x48, 0x64 and cap words at
/// 0x14, 0x30, 0x4c, 0x68.  `base` is the node's own address.
pub fn w25a40_record_init_876450(buf: &mut [u8; 0x70], base: u32) {
    let put = |b: &mut [u8; 0x70], off: usize, v: u32| {
        b[off..off + 4].copy_from_slice(&v.to_le_bytes());
    };
    put(buf, 0, base);
    put(buf, 4, base);
    put(buf, 8, 0);
    for start in [0x0cusize, 0x28, 0x44, 0x60] {
        put(buf, start, base.wrapping_add(start as u32 + 0x0c));
        put(buf, start + 4, 0);
        put(buf, start + 8, W25A40_SSO_CAP_FLAG);
        buf[start + 12] = 0;
    }
}

/// `0x00876af0`: resets a list head: previous link at +4 and head at +0 point
/// to the head itself, count at +8 is cleared.  Returns the old +4 link; when
/// it differs from the head the caller must destroy the old nodes.
pub fn w25a40_list_reset_876af0(head: &mut [u32; 3], head_addr: u32) -> u32 {
    let old = head[1];
    head[1] = head_addr;
    head[0] = head_addr;
    head[2] = 0;
    old
}

#[cfg(test)]
mod tests_wave25_agent40 {
    use super::*;

    #[test]
    fn strings() {
        let s = w25a40_str_init_876650();
        assert_eq!((s.size, s.cap, s.inline[0]), (0, 0x8000_000f, 0));
        assert_eq!(w25a40_str_init_876860(), s);
    }

    #[test]
    fn zero3() {
        let mut w = [1u32, 2, 3];
        w25a40_zero3_8763c0(&mut w);
        assert_eq!(w, [0, 0, 0]);
    }

    #[test]
    fn record_init() {
        let mut b = [0xffu8; 0x70];
        w25a40_record_init_876450(&mut b, 0x1000);
        assert_eq!(u32::from_le_bytes(b[0..4].try_into().unwrap()), 0x1000);
        assert_eq!(
            u32::from_le_bytes(b[0x68..0x6c].try_into().unwrap()),
            0x8000_000f
        );
        assert_eq!(b[0x6c], 0);
        assert_eq!(
            u32::from_le_bytes(b[0x0c..0x10].try_into().unwrap()),
            0x1018
        );
    }

    #[test]
    fn list_reset() {
        let mut h = [7, 9, 4];
        assert_eq!(w25a40_list_reset_876af0(&mut h, 5), 9);
        assert_eq!(h, [5, 5, 0]);
    }
}
