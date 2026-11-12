#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent26.rs
// ===========================================================================
// Wave 31 agent 26: MSVC std::string range move and args-library init glue.

/// Layout of an MSVC (32-bit) `std::string` as seen by the range mover:
/// 16-byte SSO buffer, size at +0x10, capacity at +0x14 (0xf when empty).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W31a26Str {
    pub buf: [u8; 16],
    pub size: u32,
    pub cap: u32,
}

impl W31a26Str {
    pub fn empty() -> Self {
        Self {
            buf: [0; 16],
            size: 0,
            cap: 0xf,
        }
    }
}

/// `0x005f0450`: moves each 24-byte string of `src` into `dst` (copying
/// the 16-byte buffer, size and capacity), then leaves the source empty
/// (size 0, capacity 0xf, first byte 0).  Returns the destination end
/// index; the trailing fix-up call is the allocator-side no-op.
pub fn w31a26_move_string_range(src: &mut [W31a26Str], dst: &mut [W31a26Str]) -> usize {
    let mut n = 0;
    for (s, d) in src.iter_mut().zip(dst.iter_mut()) {
        d.size = 0;
        d.cap = 0;
        d.buf = s.buf;
        d.size = s.size;
        d.cap = s.cap;
        s.size = 0;
        s.cap = 0xf;
        s.buf[0] = 0;
        n += 1;
    }
    n
}

/// Field values written by `0x005f2500` (args::NamedBase constructor),
/// as dword-index/value pairs after the two base-class vtable stores.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W31a26NamedBase {
    pub flags_param: u32,
    pub byte_08: u8,
    pub byte_3c: u8,
    pub dw_50: u32,
    pub dw_54: u32,
    pub byte_40: u8,
    pub byte_58: u8,
    pub dw_5c: u32,
    pub dw_60: u32,
    pub dw_64: u32,
    pub byte_68: u8,
}

/// `0x005f2500`: initialises the args::NamedBase fields: stores `param_3`
/// at dword 1, clears the byte at +8, and sets the string at dword 0x10
/// to empty (cap dword 0x15 = 0xf) and all trailing members to zero.
pub fn w31a26_namedbase_init(param_3: u32) -> W31a26NamedBase {
    W31a26NamedBase {
        flags_param: param_3,
        byte_08: 0,
        byte_3c: 0,
        dw_50: 0,
        dw_54: 0xf,
        byte_40: 0,
        byte_58: 0,
        dw_5c: 0,
        dw_60: 0,
        dw_64: 0,
        byte_68: 0,
    }
}

/// Result of the unordered-container header init in `0x005f0e10`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W31a26HashInit {
    pub max_load_factor_bits: u32,
    pub dw_c: u32,
    pub dw_10: u32,
    pub dw_14: u32,
    pub mask: u32,
    pub bucket_count: u32,
}

/// `0x005f0e10`: sets max load factor 1.0f (0x3f800000), zeroes the size
/// fields, and initialises mask = 7, bucket count = 8.
pub fn w31a26_hash_header_init() -> W31a26HashInit {
    W31a26HashInit {
        max_load_factor_bits: 0x3f800000,
        dw_c: 0,
        dw_10: 0,
        dw_14: 0,
        mask: 7,
        bucket_count: 8,
    }
}

#[cfg(test)]
mod tests_wave31_agent26 {
    use super::*;

    #[test]
    fn move_range() {
        let mut a = vec![
            W31a26Str {
                buf: [7; 16],
                size: 3,
                cap: 0xf
            };
            2
        ];
        let mut b = vec![W31a26Str::empty(); 2];
        assert_eq!(w31a26_move_string_range(&mut a, &mut b), 2);
        assert_eq!(b[1].size, 3);
        assert_eq!(
            a[0],
            W31a26Str {
                buf: {
                    let mut x = [7; 16];
                    x[0] = 0;
                    x
                },
                size: 0,
                cap: 0xf
            }
        );
    }

    #[test]
    fn namedbase_and_hash() {
        let n = w31a26_namedbase_init(9);
        assert_eq!((n.flags_param, n.dw_54), (9, 0xf));
        let h = w31a26_hash_header_init();
        assert_eq!(
            (h.mask, h.bucket_count, h.max_load_factor_bits),
            (7, 8, 0x3f800000)
        );
    }
}
