#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent24.rs
// ===========================================================================
// Wave 24 agent 24: small runtime helpers (state-table byte stores, capacity growth, EOF scan).

/// `0x00772f50`: stores `value` into the player/slot table of the decoded state
/// block.  Index `< 0x18` uses stride `0x18` at `0xdd34`, otherwise stride
/// `0x14` at `0xff48` (the index is added in both cases).
pub fn w24a24_store_slot_byte(state: &mut [u8], player: i32, index: u16, value: u8) {
    let off = if index < 0x18 {
        (index as i32 + player.wrapping_mul(0x18) + 0xdd34) as usize
    } else {
        (index as i32 + player.wrapping_mul(0x14) + 0xff48) as usize
    };
    if let Some(b) = state.get_mut(off) {
        *b = value;
    }
}

/// `0x00773030`: like the slot store but with stride `0x2e` at `0xe1c0` for
/// `index < 0x2e`; otherwise, only when the byte at `0x10350` is non-zero,
/// stride `0xf` at `0x1020e`.
pub fn w24a24_store_flag_byte(state: &mut [u8], player: i32, index: u16, value: u8) {
    let off = if index < 0x2e {
        (index as i32 + 0xe1c0 + player.wrapping_mul(0x2e)) as usize
    } else {
        if state.get(0x10350).copied().unwrap_or(0) == 0 {
            return;
        }
        (index as i32 + 0x1020e + player.wrapping_mul(0xf)) as usize
    };
    if let Some(b) = state.get_mut(off) {
        *b = value;
    }
}

/// `0x007742f0`: grows the capacity packed in the low 31 bits of `word`
/// (bit 31 is preserved).  New capacity is `max(wanted, cap + cap/2)`.
/// Returns `(new_word, reallocated)`.
pub fn w24a24_grow_capacity(word: u32, wanted: u32) -> (u32, bool) {
    if (word & 0x7fff_ffff) < wanted {
        let mut new_cap = ((word >> 1) & 0x3fff_ffff).wrapping_add(word & 0x7fff_ffff);
        if new_cap < wanted {
            new_cap = wanted;
        }
        (((word ^ new_cap) & 0x7fff_ffff) ^ word, true)
    } else {
        (word, false)
    }
}

/// `0x00772d30`: reads bytes one at a time until a `0x1a` (Ctrl-Z) marker.
/// Returns true when the marker was found, false on read failure/EOF.
/// Every byte consumed is passed to `sink`.
pub fn w24a24_scan_to_ctrl_z(
    mut read_byte: impl FnMut() -> Option<u8>,
    mut sink: impl FnMut(u8),
) -> bool {
    while let Some(b) = read_byte() {
        if b == 0x1a {
            return true;
        }
        sink(b);
    }
    false
}

/// `0x00776b20`: zeroes the `0x30`-byte block.
pub fn w24a24_clear_block(block: &mut [u8; 0x30]) {
    block.fill(0);
}

/// Field image written by `0x00777b00` (14 dwords, with word 3 split).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W24a24Header {
    pub words: [u32; 14],
}

/// `0x00777b00`: constructor; zeroes the 14-dword record, then stores
/// `param_1` at word 4, `param_3` at word 6 and `param_2` at word 0xb.
pub fn w24a24_init_header(param_1: u32, param_2: u32, param_3: u32) -> W24a24Header {
    let mut h = W24a24Header::default();
    h.words[4] = param_1;
    h.words[6] = param_3;
    h.words[0xb] = param_2;
    h
}

#[cfg(test)]
mod tests_wave24_agent24 {
    use super::*;

    #[test]
    fn slot_store() {
        let mut s = vec![0u8; 0x11000];
        w24a24_store_slot_byte(&mut s, 2, 3, 7);
        assert_eq!(s[0xdd34 + 48 + 3], 7);
        w24a24_store_slot_byte(&mut s, 1, 0x18, 9);
        assert_eq!(s[0xff48 + 0x14 + 0x18], 9);
    }

    #[test]
    fn flag_store() {
        let mut s = vec![0u8; 0x11000];
        w24a24_store_flag_byte(&mut s, 1, 0x2e, 5);
        assert!(s.iter().all(|&b| b == 0));
        w24a24_store_flag_byte(&mut s, 1, 1, 4);
        assert_eq!(s[0xe1c0 + 0x2e + 1], 4);
        s[0x10350] = 1;
        w24a24_store_flag_byte(&mut s, 1, 0x2e, 6);
        assert_eq!(s[0x1020e + 0xf + 0x2e], 6);
    }

    #[test]
    fn grow() {
        assert_eq!(w24a24_grow_capacity(0x8000_0010, 8), (0x8000_0010, false));
        assert_eq!(w24a24_grow_capacity(0x8000_0010, 0x11), (0x8000_0018, true));
        assert_eq!(w24a24_grow_capacity(0x10, 100), (100, true));
    }

    #[test]
    fn scan() {
        let data = [1u8, 2, 0x1a, 3];
        let mut i = 0;
        let mut got = vec![];
        let ok = w24a24_scan_to_ctrl_z(
            || {
                let b = data.get(i).copied();
                i += 1;
                b
            },
            |b| got.push(b),
        );
        assert!(ok);
        assert_eq!(got, [1, 2]);
        assert!(!w24a24_scan_to_ctrl_z(|| None, |_| {}));
    }

    #[test]
    fn clear_and_init() {
        let mut b = [1u8; 0x30];
        w24a24_clear_block(&mut b);
        assert!(b.iter().all(|&x| x == 0));
        let h = w24a24_init_header(1, 2, 3);
        assert_eq!((h.words[4], h.words[0xb], h.words[6]), (1, 2, 3));
    }
}
