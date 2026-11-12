#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent52.rs
// ===========================================================================
// Wave 25 agent 52: glues FileEnumerator-derived constructors.

/// Size in bytes of the enumerator header initialised by the constructors.
pub const W25A52_ENUM_SIZE: usize = 0x68;

fn w25a52_put(buf: &mut [u8], word: usize, v: u32) {
    buf[word * 4..word * 4 + 4].copy_from_slice(&v.to_le_bytes());
}

/// Shared base-class (FileEnumerator) body: three SSO strings at words
/// 1/8/0xf (ptr to inline buffer at +3/+0xa/+0x11 words... i.e. word+3),
/// capacity 0x8000000f, plus a fourth at words 0x16.. and word 0x17 = 0.
/// `base` is the guest address of the object (used for self pointers).
fn w25a52_init_fields(buf: &mut [u8], base: u32, vtable: u32) {
    w25a52_put(buf, 0, vtable);
    let strs = [
        (1usize, 2usize, 3usize, 4usize),
        (8, 9, 10, 0xb),
        (0xf, 0x10, 0x11, 0x12),
    ];
    for (ptr, len, cap, inl) in strs {
        w25a52_put(buf, ptr, base + (inl as u32) * 4);
        w25a52_put(buf, len, 0);
        w25a52_put(buf, cap, 0x8000_000f);
        buf[inl * 4] = 0;
    }
    w25a52_put(buf, 0x16, base + 0x19 * 4);
    w25a52_put(buf, 0x17, 0);
    w25a52_put(buf, 0x18, 0x8000_000f);
    buf[0x19 * 4] = 0;
}

/// `0x008c2dd0`: SinglePlayerMapReplayEnumerator constructor field init.
pub fn w25a52_map_replay_enumerator_init(buf: &mut [u8; W25A52_ENUM_SIZE], base: u32, vtable: u32) {
    w25a52_init_fields(buf, base, vtable);
}

/// `0x008c31f0`: SinglePlayerReplayEnumerator constructor field init.
pub fn w25a52_replay_enumerator_init(buf: &mut [u8; W25A52_ENUM_SIZE], base: u32, vtable: u32) {
    w25a52_init_fields(buf, base, vtable);
}

#[cfg(test)]
mod tests_wave25_agent52 {
    use super::*;

    fn rd(b: &[u8], w: usize) -> u32 {
        u32::from_le_bytes(b[w * 4..w * 4 + 4].try_into().unwrap())
    }

    #[test]
    fn init_layout() {
        for f in [
            w25a52_map_replay_enumerator_init,
            w25a52_replay_enumerator_init,
        ] {
            let mut b = [0xAAu8; W25A52_ENUM_SIZE];
            f(&mut b, 0x1000, 0x42);
            assert_eq!(rd(&b, 0), 0x42);
            assert_eq!(rd(&b, 1), 0x1010);
            assert_eq!(rd(&b, 3), 0x8000_000f);
            assert_eq!(rd(&b, 8), 0x102c);
            assert_eq!(rd(&b, 0xf), 0x1048);
            assert_eq!(rd(&b, 0x16), 0x1064);
            assert_eq!(rd(&b, 0x17), 0);
            assert_eq!(b[0x19 * 4], 0);
        }
    }
}
