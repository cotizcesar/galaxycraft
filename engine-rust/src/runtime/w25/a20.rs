#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent20.rs
// ===========================================================================
/// `0x007ff810`: regex char-class helper.  For every byte value 0..256 the
/// class test (`mask == 0xffff` means word class: `_` or ctype bits `0x107`;
/// otherwise `ctype[c] & mask != 0`) is compared with `negate`; when they
/// differ the bit `c` is set in the 32-byte `bits` table.
pub fn w25a20_class_bits(ctype: &[u16; 256], bits: &mut [u8; 32], mask: u16, negate: bool) {
    for c in 0..256usize {
        let hit = if mask == 0xffff {
            c == b'_' as usize || (ctype[c] & 0x107) != 0
        } else {
            (ctype[c] & mask) != 0
        };
        if hit != negate {
            bits[c >> 3] |= 1 << (c & 7);
        }
    }
}

/// A node of the sorted (descending key) list used by `0x00800590`.
pub struct W25a20Node {
    pub key: u32,
    pub data: Vec<u8>,
}

/// `0x00800590`: finds the first node with `key <= wanted`; if its key equals
/// `wanted`, appends `bytes` to its buffer, otherwise returns `Err(())`
/// (the original raises error 0x14).
pub fn w25a20_append_to_node(
    nodes: &mut [W25a20Node],
    wanted: u32,
    bytes: &[u8],
) -> Result<(), ()> {
    match nodes.iter_mut().find(|n| n.key <= wanted) {
        Some(n) if n.key == wanted => {
            n.data.extend_from_slice(bytes);
            Ok(())
        }
        _ => Err(()),
    }
}

#[cfg(test)]
mod tests_wave25_agent20 {
    use super::*;

    #[test]
    fn class_bits_word_and_mask() {
        let mut ct = [0u16; 256];
        ct[b'a' as usize] = 0x103;
        let mut bits = [0u8; 32];
        w25a20_class_bits(&ct, &mut bits, 0xffff, true);
        assert_eq!(bits[(b'a' >> 3) as usize] & (1 << (b'a' & 7)), 0);
        assert_eq!(bits[(b'_' >> 3) as usize] & (1 << (b'_' & 7)), 0);
        assert_ne!(bits[0] & 1, 0);
        let mut bits = [0u8; 32];
        w25a20_class_bits(&ct, &mut bits, 0x2, false);
        assert_ne!(bits[(b'a' >> 3) as usize] & (1 << (b'a' & 7)), 0);
        assert_eq!(bits[0], 0);
    }

    #[test]
    fn append_requires_exact_key() {
        let mut v = vec![
            W25a20Node {
                key: 5,
                data: vec![],
            },
            W25a20Node {
                key: 2,
                data: vec![],
            },
        ];
        assert!(w25a20_append_to_node(&mut v, 2, b"ab").is_ok());
        assert_eq!(v[1].data, b"ab");
        assert!(w25a20_append_to_node(&mut v, 3, b"x").is_err());
        assert!(w25a20_append_to_node(&mut v, 1, b"x").is_err());
    }
}
