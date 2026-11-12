#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent05.rs
// ===========================================================================
// Wave 24 translation batch 05: client helpers (bit test, selection
// cache, string append, mode selector, flag broadcast precheck).

/// `0x00781580`: tests bit `bit` of the per-player byte mask located at
/// `table[(bit >> 3) + player * 6]`; returns the masked byte (nonzero = set).
/// The table is the decoded base at offset `0x10140`, stride 6 per player.
pub fn w24a05_player_bit_test(table: &[u8], bit: u8, player: usize) -> u32 {
    let idx = (bit >> 3) as usize + player * 6;
    let byte = table.get(idx).copied().unwrap_or(0);
    (byte as u32) & (1u32 << (bit & 7))
}

/// New selection-cache state produced by [`w24a05_select_cache_update`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W24a05SelectCache {
    /// Cached object handle (0 = none).
    pub current: u32,
    /// Cached auxiliary id (`0xffffffff` = none).
    pub aux: u32,
    /// True when the follow-up refresh call (callee at 786e80) must run.
    pub refresh: bool,
    /// True when the clear path ran (callee at 603ee0 must run).
    pub cleared: bool,
}

/// `0x00785b20`: updates the cached selected object.  `busy` is the result
/// of the gate check, `lookup_matches` says the object found for the owner
/// id equals `obj`, `obj_aux` is the object's field at `+0x40`.
pub fn w24a05_select_cache_update(
    busy: bool,
    lookup_matches: bool,
    obj: u32,
    obj_aux: u32,
    cached: u32,
) -> W24a05SelectCache {
    if !busy && lookup_matches {
        if obj != cached {
            return W24a05SelectCache {
                current: obj,
                aux: obj_aux,
                refresh: true,
                cleared: false,
            };
        }
        return W24a05SelectCache {
            current: cached,
            aux: obj_aux,
            refresh: false,
            cleared: false,
        };
    }
    W24a05SelectCache {
        current: 0,
        aux: 0xffff_ffff,
        refresh: false,
        cleared: true,
    }
}

/// `0x007a1dd0`: bounded append of the C string `src` onto the C string in
/// `dst` (capacity `0x318`, at most `0x317` characters kept).  Does nothing
/// when `dst` is already full.  Returns the number of bytes appended.
pub fn w24a05_bounded_append(dst: &mut [u8], src: &[u8]) -> usize {
    const CAP: usize = 0x318;
    let lim = dst.len().min(CAP);
    let len = dst[..lim].iter().position(|&b| b == 0).unwrap_or(lim);
    if len == CAP {
        return 0;
    }
    let slen = src.iter().position(|&b| b == 0).unwrap_or(src.len());
    let n = (slen + len).min(0x317) - len;
    dst[len..len + n].copy_from_slice(&src[..n]);
    if len + n < dst.len() {
        dst[len + n] = 0;
    }
    n
}

/// `0x0079dde0`: first-pass state selection (the `DAT_011b24c0` byte when it
/// is zero).  `initialised` is the global gate, `flags` the word at
/// `+10` of the event record, `a`/`b` the two probe results (callees
/// 748d60 and 6e5460) and `mode_byte` the byte at offset `0x2c70`.
pub fn w24a05_ui_state_select(
    initialised: bool,
    flags: u16,
    a: bool,
    b: bool,
    mode_byte: u8,
) -> u8 {
    if !initialised {
        return 1;
    }
    if a {
        return 5;
    }
    if flags & 3 != 0 {
        return 2;
    }
    if b && (flags >> 2) & 1 != 0 {
        return 3;
    }
    if mode_byte == 8 {
        return 2;
    }
    if b && mode_byte == 9 {
        return 3;
    }
    4
}

/// `0x0079c060`: pre-check and new flag word for the flag broadcast.
/// Errors carry the Win32 code passed to the error setter: `0x57` (no
/// context), `0x85100070` (invalid handle), `0x120` (no active object).
/// `clear_low_bit` is `param_2 != 0`; the result is the value stored in the
/// broadcast word.
pub fn w24a05_flag_broadcast_value(
    have_ctx: bool,
    handle_invalid: bool,
    active_object: bool,
    flags: u32,
    clear_low_bit: bool,
) -> Result<u32, u32> {
    if !have_ctx {
        return Err(0x57);
    }
    if handle_invalid {
        return Err(0x8510_0070);
    }
    if !active_object {
        return Err(0x120);
    }
    Ok(if clear_low_bit { flags & !1 } else { flags })
}

#[cfg(test)]
mod tests_wave24_agent05 {
    use super::*;

    #[test]
    fn bit_test() {
        let t = [0u8, 0, 0, 0, 0, 0, 0b0000_0100, 0, 0, 0, 0, 0];
        assert_eq!(w24a05_player_bit_test(&t, 2, 1), 4);
        assert_eq!(w24a05_player_bit_test(&t, 3, 1), 0);
        assert_eq!(w24a05_player_bit_test(&t, 2, 0), 0);
    }

    #[test]
    fn select_cache() {
        let s = w24a05_select_cache_update(false, true, 5, 9, 0);
        assert!(s.refresh && s.current == 5 && s.aux == 9);
        let s = w24a05_select_cache_update(false, true, 5, 9, 5);
        assert!(!s.refresh && !s.cleared);
        let s = w24a05_select_cache_update(true, true, 5, 9, 5);
        assert!(s.cleared && s.current == 0 && s.aux == 0xffff_ffff);
    }

    #[test]
    fn append() {
        let mut b = [0u8; 0x318];
        b[..2].copy_from_slice(b"ab");
        assert_eq!(w24a05_bounded_append(&mut b, b"cd\0"), 2);
        assert_eq!(&b[..5], b"abcd\0");
        let mut full = [b'x'; 0x318];
        assert_eq!(w24a05_bounded_append(&mut full, b"y\0"), 0);
        let mut near = [b'x'; 0x318];
        near[0x316] = 0;
        assert_eq!(w24a05_bounded_append(&mut near, b"yz\0"), 1);
        assert_eq!(near[0x316], b'y');
    }

    #[test]
    fn ui_state() {
        assert_eq!(w24a05_ui_state_select(false, 0, false, false, 0), 1);
        assert_eq!(w24a05_ui_state_select(true, 0, true, false, 0), 5);
        assert_eq!(w24a05_ui_state_select(true, 1, false, false, 0), 2);
        assert_eq!(w24a05_ui_state_select(true, 4, false, true, 0), 3);
        assert_eq!(w24a05_ui_state_select(true, 0, false, false, 8), 2);
        assert_eq!(w24a05_ui_state_select(true, 0, false, true, 9), 3);
        assert_eq!(w24a05_ui_state_select(true, 0, false, false, 9), 4);
    }

    #[test]
    fn flag_broadcast() {
        assert_eq!(
            w24a05_flag_broadcast_value(false, false, true, 3, true),
            Err(0x57)
        );
        assert_eq!(
            w24a05_flag_broadcast_value(true, true, true, 3, true),
            Err(0x8510_0070)
        );
        assert_eq!(
            w24a05_flag_broadcast_value(true, false, false, 3, true),
            Err(0x120)
        );
        assert_eq!(
            w24a05_flag_broadcast_value(true, false, true, 3, true),
            Ok(2)
        );
        assert_eq!(
            w24a05_flag_broadcast_value(true, false, true, 3, false),
            Ok(3)
        );
    }
}
