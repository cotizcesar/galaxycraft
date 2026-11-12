#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent26.rs
// ===========================================================================
/// `0x0077e0a0`: clamps a volume percentage to at most 100; the clamped
/// value is what gets pushed to the sound-state setter.
pub fn w24a26_clamp_volume(v: u32) -> u32 {
    v.min(100)
}

/// `0x0077e7b0`: number of 16-bit entries the drain loop visits, computed from
/// a base `base` and entry count `count` with 32-bit wrapping arithmetic;
/// the count is reset to 0 afterwards (second tuple field).
pub fn w24a26_drain_count(base: u32, count: u32) -> (u32, u32) {
    let end = base.wrapping_add(count.wrapping_mul(2));
    let n = if end < base {
        0
    } else {
        end.wrapping_sub(base).wrapping_add(1) >> 1
    };
    (n, 0)
}

/// `0x0077f0d0`: runs `action` only when all three gates hold (two flag bytes
/// non-zero and a word non-zero); otherwise returns false.
pub fn w24a26_gated_action(
    gate_a: u8,
    gate_b: u8,
    gate_c: u32,
    action: impl FnOnce() -> bool,
) -> bool {
    if gate_a != 0 && gate_b != 0 && gate_c != 0 {
        action()
    } else {
        false
    }
}

/// `0x0077fc30`: converts an (hours, minutes, seconds, millis) record
/// (bytes 0..2 then u16 at +4) to milliseconds.
pub fn w24a26_time_to_ms(rec: &[u8; 6]) -> u32 {
    let ms = u16::from_le_bytes([rec[4], rec[5]]) as u32;
    ((rec[2] as u32 + (rec[1] as u32 + rec[0] as u32 * 60) * 60) * 1000).wrapping_add(ms)
}

/// `0x00780e50`: true when the probe result is zero.
pub fn w24a26_probe_is_zero(probe: i32) -> bool {
    probe == 0
}

/// `0x00781135`: clears the byte flag and returns true if any of the
/// commands 0xa6, 0xa2, 0xa4, 0xac is active.
pub fn w24a26_any_of_four(flag: &mut u8, active: impl Fn(u32) -> bool) -> bool {
    *flag = 0;
    [0xa6, 0xa2, 0xa4, 0xac].iter().any(|&c| active(c))
}

/// `0x007811a0`: when `enabled == 1`, true if any of eight commands is active.
pub fn w24a26_any_of_eight(enabled: u8, active: impl Fn(u32) -> bool) -> bool {
    enabled == 1
        && [0x9b, 0xa7, 0xa3, 0xab, 0xa9, 0xa5, 0x9f, 0xaa]
            .iter()
            .any(|&c| active(c))
}

/// `0x007814c0`: when `enabled == 1`, forwards the query result.
pub fn w24a26_enabled_query(enabled: u8, query: impl FnOnce() -> bool) -> bool {
    enabled == 1 && query()
}

/// `0x00781540`: clears bit `bit & 7` in the byte at `(bit >> 3) + row * 8`.
/// Returns false when out of range.
pub fn w24a26_clear_bit(bits: &mut [u8], bit: u8, row: usize) -> bool {
    match bits.get_mut((bit >> 3) as usize + row * 8) {
        Some(b) => {
            *b &= !(1 << (bit & 7));
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests_wave24_agent26 {
    use super::*;

    #[test]
    fn volume_and_drain() {
        assert_eq!(w24a26_clamp_volume(0), 0);
        assert_eq!(w24a26_clamp_volume(100), 100);
        assert_eq!(w24a26_clamp_volume(101), 100);
        assert_eq!(w24a26_drain_count(10, 4), (4, 0));
        assert_eq!(w24a26_drain_count(0xffff_fff0, 0x10), (0, 0));
    }

    #[test]
    fn gates_and_time() {
        assert!(w24a26_gated_action(1, 1, 1, || true));
        assert!(!w24a26_gated_action(1, 0, 1, || true));
        assert!(!w24a26_gated_action(1, 1, 0, || true));
        assert_eq!(w24a26_time_to_ms(&[1, 2, 3, 0, 5, 0]), 3_723_005);
        assert!(w24a26_probe_is_zero(0));
        assert!(!w24a26_probe_is_zero(3));
    }

    #[test]
    fn command_sets() {
        let mut f = 9;
        assert!(w24a26_any_of_four(&mut f, |c| c == 0xac));
        assert_eq!(f, 0);
        assert!(!w24a26_any_of_four(&mut f, |c| c == 0x9b));
        assert!(w24a26_any_of_eight(1, |c| c == 0xaa));
        assert!(!w24a26_any_of_eight(0, |_| true));
        assert!(!w24a26_any_of_eight(1, |c| c == 0xac));
        assert!(w24a26_enabled_query(1, || true));
        assert!(!w24a26_enabled_query(2, || true));
    }

    #[test]
    fn clear_bit() {
        let mut b = [0xffu8; 16];
        assert!(w24a26_clear_bit(&mut b, 9, 1));
        assert_eq!(b[9], 0xfd);
        assert!(!w24a26_clear_bit(&mut b, 9, 5));
    }
}
