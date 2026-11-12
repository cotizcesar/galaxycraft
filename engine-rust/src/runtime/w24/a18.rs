#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent18.rs
// ===========================================================================
// Wave 24 agent 18: small pure helpers (selectors, predicates, rounding).

/// `0x00750950`: maps a mode byte to a message id (0 -> 0x29, 1 -> 7,
/// 2 -> 0x40, anything else -> 0xe4).
pub fn w24a18_message_id_for_mode(mode: i8) -> u32 {
    match mode {
        0 => 0x29,
        1 => 7,
        2 => 0x40,
        _ => 0xe4,
    }
}

/// `0x00751471`: for command byte `'G'` returns `Some((0x47, lo, hi))` with the
/// two signed 16-bit halves of `packed`; otherwise `None` (fallback path).
pub fn w24a18_split_g_command(cmd: u8, packed: u32) -> Option<(u32, i32, i32)> {
    if cmd == b'G' {
        Some((
            0x47,
            (packed as u16 as i16) as i32,
            ((packed >> 16) as u16 as i16) as i32,
        ))
    } else {
        None
    }
}

/// `0x00751680`: clamps both dimensions to at least the per-slot minimums
/// then rounds up to a multiple of 32: `(v + 0x1f) & 0xffe0`.
pub fn w24a18_round_dims(rec_w: u16, rec_h: u16, min_w: i16, min_h: i16) -> (u16, u16) {
    let w = if (rec_w as i32) <= min_w as i32 {
        min_w as u16
    } else {
        rec_w
    };
    let h = if (rec_h as i32) <= min_h as i32 {
        min_h as u16
    } else {
        rec_h
    };
    (w.wrapping_add(0x1f) & 0xffe0, h.wrapping_add(0x1f) & 0xffe0)
}

/// `0x00752a10`: true when the mode byte is 2.
pub fn w24a18_mode_is_two(mode: i8) -> bool {
    mode == 2
}

/// `0x00752a20`: true for modes 1, 2, 3, 4 and 7.
pub fn w24a18_mode_in_set_a(mode: i8) -> bool {
    matches!(mode, 1 | 2 | 3 | 4 | 7)
}

/// `0x00752a50`: true for modes 3, 4, 5 and 7.
pub fn w24a18_mode_in_set_b(mode: i8) -> bool {
    matches!(mode, 3 | 4 | 5 | 7)
}

/// `0x00752df0`: returns `None` when `kind == 0x18` (handled by a separate
/// call), else `Some(dirty)` where dirty is set when `kind != 0xf` or the key
/// is 0x1b, 0xd or 0x20.
pub fn w24a18_input_sets_flag(kind: i16, key: i16) -> Option<bool> {
    if kind == 0x18 {
        return None;
    }
    Some(kind != 0xf || matches!(key, 0x1b | 0xd | 0x20))
}

/// `0x00754000`: true when the queried state byte is 1, 2, 4, 5 or 6.
pub fn w24a18_state_is_active(state: u8) -> bool {
    matches!(state, 1 | 2 | 4 | 5 | 6)
}

/// `0x00754e00`: given the lookup success flag and out value, returns the
/// two result bytes `(found, found && value != 0)`.
pub fn w24a18_lookup_flags(found: bool, value: i32) -> (u8, u8) {
    (found as u8, (found && value != 0) as u8)
}

#[cfg(test)]
mod tests_wave24_agent18 {
    use super::*;

    #[test]
    fn selectors() {
        assert_eq!(w24a18_message_id_for_mode(0), 0x29);
        assert_eq!(w24a18_message_id_for_mode(1), 7);
        assert_eq!(w24a18_message_id_for_mode(2), 0x40);
        assert_eq!(w24a18_message_id_for_mode(9), 0xe4);
        assert_eq!(
            w24a18_split_g_command(b'G', 0xfffe_0003),
            Some((0x47, 3, -2))
        );
        assert_eq!(w24a18_split_g_command(b'H', 1), None);
    }

    #[test]
    fn rounding() {
        assert_eq!(w24a18_round_dims(33, 1, 0, 40), (64, 64));
        assert_eq!(w24a18_round_dims(32, 32, 0, 0), (32, 32));
    }

    #[test]
    fn predicates() {
        assert!(w24a18_mode_is_two(2) && !w24a18_mode_is_two(1));
        for m in 0..10i8 {
            assert_eq!(w24a18_mode_in_set_a(m), [1, 2, 3, 4, 7].contains(&m));
            assert_eq!(w24a18_mode_in_set_b(m), [3, 4, 5, 7].contains(&m));
        }
        assert_eq!(w24a18_input_sets_flag(0x18, 0), None);
        assert_eq!(w24a18_input_sets_flag(0xf, 0x1b), Some(true));
        assert_eq!(w24a18_input_sets_flag(0xf, 0x41), Some(false));
        assert_eq!(w24a18_input_sets_flag(1, 0x41), Some(true));
        assert!(w24a18_state_is_active(6) && !w24a18_state_is_active(3));
        assert_eq!(w24a18_lookup_flags(true, 5), (1, 1));
        assert_eq!(w24a18_lookup_flags(true, 0), (1, 0));
        assert_eq!(w24a18_lookup_flags(false, 5), (0, 0));
    }
}
