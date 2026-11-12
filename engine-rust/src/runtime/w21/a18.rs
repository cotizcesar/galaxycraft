#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave21/agent18.rs
// ===========================================================================
// Wave 21 translation batch 18: MSVC STL glue around the args parser.

/// MSVC `std::string` (x86, 24 bytes) modelled as its observable fields.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct W21a18String {
    /// Length in bytes.
    pub size: u32,
    /// Capacity; `0xf` for an empty small-buffer string.
    pub cap: u32,
    /// Contents.
    pub data: Vec<u8>,
}

impl W21a18String {
    /// Empty small-buffer string (`size == 0`, `cap == 0xf`).
    pub fn new_empty() -> Self {
        Self {
            size: 0,
            cap: 0xf,
            data: Vec::new(),
        }
    }
}

/// Fields of the 0x34-byte record moved by `0x005f1550`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W21a18Pair {
    /// Word 0 (vftable/tag).
    pub head: u32,
    /// Words 1..=4 of the first block (word 1 low byte is the SSO byte).
    pub block_a: [u32; 4],
    /// First string (words 5..=6 are size/cap, moved out).
    pub str_a: W21a18String,
    /// Words 7..=10 of the second block.
    pub block_b: [u32; 4],
    /// Second string (words 0xb..=0xc).
    pub str_b: W21a18String,
}

/// `0x005f1550`: move constructor.  Copies the head and both plain blocks,
/// steals both strings (leaving the source empty: size 0, cap 0xf) and
/// clears the source's first byte of each block at words 1 and 7.
pub fn w21a18_move_pair(src: &mut W21a18Pair) -> W21a18Pair {
    let dst = W21a18Pair {
        head: src.head,
        block_a: src.block_a,
        str_a: std::mem::take(&mut src.str_a),
        block_b: src.block_b,
        str_b: std::mem::take(&mut src.str_b),
    };
    src.str_a = W21a18String::new_empty();
    src.str_b = W21a18String::new_empty();
    src.block_a[0] &= !0xff;
    src.block_b[0] &= !0xff;
    dst
}

/// `0x005eeb50` (hash part): FNV-1a style bucket index for a one-byte key,
/// `mask & ((key ^ 0x811c9dc5) * 0x1000193)`, scaled by 8 (bucket pair size).
pub fn w21a18_bucket_offset(key: u8, mask: u32) -> u32 {
    (mask & ((key as u32 ^ 0x811c_9dc5).wrapping_mul(0x0100_0193))).wrapping_mul(8)
}

/// `0x005eeb50` (insert): inserts `key` into a byte-keyed unique list set.
/// Returns `(index, inserted)`; a duplicate yields the existing index and
/// `false` (the pseudocode frees the new node and returns).
pub fn w21a18_insert_unique(keys: &mut Vec<u8>, key: u8) -> (usize, bool) {
    if let Some(i) = keys.iter().position(|&k| k == key) {
        return (i, false);
    }
    keys.push(key);
    (keys.len() - 1, true)
}

/// `0x005f29c0`: zeroes the 0x34-byte locale-facet cache block after the
/// first word; returns the fields as `(offset, value)` writes of byte width.
pub fn w21a18_locale_reset(block: &mut [u8; 0x34]) {
    // (offset, width): dwords at 4/c/14/1c/24/2c, words at 18/20, bytes at 8/10/28/30.
    for (off, w) in [
        (0x4usize, 4usize),
        (0x8, 1),
        (0xc, 4),
        (0x10, 1),
        (0x14, 4),
        (0x18, 2),
        (0x1c, 4),
        (0x20, 2),
        (0x24, 4),
        (0x28, 1),
        (0x2c, 4),
        (0x30, 1),
    ] {
        block[off..off + w].fill(0);
    }
}

/// `0x005f37e0`: `args::Base` destructor tail; resets the string at words
/// 3..=8 to empty when it uses the small buffer (`cap < 0x10`).  Returns
/// `true` if heap storage must be freed instead.
pub fn w21a18_base_dtor(size_word7: &mut u32, cap_word8: &mut u32, byte_word3: &mut u8) -> bool {
    if *cap_word8 < 0x10 {
        *size_word7 = 0;
        *cap_word8 = 0xf;
        *byte_word3 = 0;
        return false;
    }
    true
}

#[cfg(test)]
mod tests_wave21_agent18 {
    use super::*;

    #[test]
    fn move_pair_steals() {
        let mut p = W21a18Pair {
            head: 7,
            block_a: [0x1234, 2, 3, 4],
            str_a: W21a18String {
                size: 3,
                cap: 0x1f,
                data: b"abc".to_vec(),
            },
            block_b: [0x55, 6, 7, 8],
            str_b: W21a18String::new_empty(),
        };
        let d = w21a18_move_pair(&mut p);
        assert_eq!(d.str_a.data, b"abc");
        assert_eq!(d.block_a[0], 0x1234);
        assert_eq!(p.str_a, W21a18String::new_empty());
        assert_eq!(p.block_a[0], 0x1200);
        assert_eq!(p.block_b[0], 0);
    }

    #[test]
    fn bucket() {
        assert_eq!(
            w21a18_bucket_offset(0, 0xff),
            (0x811c_9dc5u32.wrapping_mul(0x0100_0193) & 0xff) * 8
        );
        assert_eq!(w21a18_bucket_offset(5, 0), 0);
    }

    #[test]
    fn insert() {
        let mut v = vec![];
        assert_eq!(w21a18_insert_unique(&mut v, 4), (0, true));
        assert_eq!(w21a18_insert_unique(&mut v, 9), (1, true));
        assert_eq!(w21a18_insert_unique(&mut v, 4), (0, false));
    }

    #[test]
    fn locale() {
        let mut b = [0xffu8; 0x34];
        w21a18_locale_reset(&mut b);
        assert_eq!(b[0], 0xff);
        assert!(b[4..0x31].iter().enumerate().all(|(i, &x)| {
            x == 0
                || [0x9, 0xa, 0xb, 0xd - 4 + 4].contains(&(i + 4)) && false
                || [
                    0x9, 0xa, 0xb, 0x11, 0x12, 0x13, 0x1a, 0x1b, 0x22, 0x23, 0x29, 0x2a, 0x2b, 0x31,
                ]
                .contains(&(i + 4))
        }));
    }

    #[test]
    fn base_dtor() {
        let (mut s, mut c, mut b) = (5, 0xf, 9);
        assert!(!w21a18_base_dtor(&mut s, &mut c, &mut b));
        assert_eq!((s, c, b), (0, 0xf, 0));
        let (mut s, mut c, mut b) = (50, 0x3f, 1);
        assert!(w21a18_base_dtor(&mut s, &mut c, &mut b));
    }
}
