#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent09.rs
// ===========================================================================
// Wave 24 agent 09: net/lobby packet helpers (pure reference translations).

/// `0x0072a7b0`: scans bits 7..=0 of the 16-bit mask `word`; if a set bit's
/// slot check (`check(nation, bit)`) is false it returns `None`.  If no bit
/// fails, returns the new per-nation cell: `(old & 0xff00) | (word & 0xff) |
/// (1 << (nation & 31))`.
pub fn w24a09_merge_nation_mask(
    old: u32,
    word: u16,
    nation: u32,
    mut check: impl FnMut(u32, u32) -> bool,
) -> Option<u32> {
    for bit in (0..8u32).rev() {
        if (word & (1u16 << bit)) != 0 && !check(nation, bit) {
            return None;
        }
    }
    Some((old & 0xff00) | (word as u8 as u32) | (1u32 << (nation & 31)))
}

/// `0x0072a820`: when the gate flag is non-zero, forwards the packet fields
/// `(u32 at +2, u32 at +6, byte at +1)`; returns them, else `None`.
pub fn w24a09_forward_fields(gate: u32, pkt: &[u8]) -> Option<(u32, u32, u8)> {
    if gate == 0 || pkt.len() < 10 {
        return None;
    }
    let a = u32::from_le_bytes([pkt[2], pkt[3], pkt[4], pkt[5]]);
    let b = u32::from_le_bytes([pkt[6], pkt[7], pkt[8], pkt[9]]);
    Some((a, b, pkt[1]))
}

/// `0x0072b970`: builds the 10-byte packet (`0x7802`, four u16 params) and
/// calls `send(slot, &packet)` for each slot 0x80..0x84 where `active(slot)`.
pub fn w24a09_broadcast_rect(
    p: [u16; 4],
    mut active: impl FnMut(u32) -> bool,
    mut send: impl FnMut(u32, &[u8; 10]),
) {
    let mut pkt = [0u8; 10];
    pkt[0..2].copy_from_slice(&0x7802u16.to_le_bytes());
    for (i, v) in p.iter().enumerate() {
        pkt[2 + i * 2..4 + i * 2].copy_from_slice(&v.to_le_bytes());
    }
    for slot in 0x80..0x84u32 {
        if active(slot) {
            send(slot, &pkt);
        }
    }
}

/// `0x0072c590`: builds the 3-byte packet `[0x13, param_1, param_2]`.
/// The caller then clears the 0x30-byte table when `param_1 == 1`, else
/// stores 0xff in the trailing byte; the second value reports that choice.
pub fn w24a09_build_select_packet(param_1: u8, param_2: u8) -> ([u8; 3], bool) {
    ([0x13, param_1, param_2], param_1 == 1)
}

/// `0x0072cc70`: builds the 5-byte packet `[0x0e, u32 le]`.
pub fn w24a09_build_u32_packet(v: u32) -> [u8; 5] {
    let mut p = [0u8; 5];
    p[0] = 0x0e;
    p[1..5].copy_from_slice(&v.to_le_bytes());
    p
}

/// `0x0072ea50`: validity test for a player/nation id; returns true when
/// `id <= 7` (larger ids take an error path in the original).
pub fn w24a09_nation_id_ok(id: u32) -> bool {
    id <= 7
}

/// `0x0072ea70`: emits the "Net Data:" debug dump through `emit`; the six
/// ids are (human, storm, nation) local then active.
pub fn w24a09_net_data_dump(
    local: (u32, u32, u32),
    active: (u32, u32, u32),
    mut emit: impl FnMut(String),
) {
    emit("Net Data:".to_string());
    emit(format!("    g_LocalHumanID      : {}", local.0));
    emit(format!("    g_LocalStormID      : {}", local.1));
    emit(format!("    g_LocalNationID     : {}", local.2));
    emit(format!("    g_ActiveHumanID     : {}", active.0));
    emit(format!("    g_ActiveStormID     : {}", active.1));
    emit(format!("    g_ActiveNationID    : {}", active.2));
}

/// `0x0072ecb0`: resets the six net id globals to 8 (invalid).
pub fn w24a09_reset_net_ids() -> [u32; 6] {
    [8; 6]
}

#[cfg(test)]
mod tests_wave24_agent09 {
    use super::*;

    #[test]
    fn merge_mask() {
        assert_eq!(
            w24a09_merge_nation_mask(0x1234, 0x0105, 3, |_, _| true),
            Some(0x1200 | 0x05 | 8)
        );
        assert_eq!(w24a09_merge_nation_mask(0, 0x80, 0, |_, b| b != 7), None);
        assert_eq!(w24a09_merge_nation_mask(0, 0x100, 0, |_, _| false), Some(1));
    }

    #[test]
    fn forward() {
        let mut p = [0u8; 10];
        p[1] = 9;
        p[2] = 1;
        p[6] = 2;
        assert_eq!(w24a09_forward_fields(1, &p), Some((1, 2, 9)));
        assert_eq!(w24a09_forward_fields(0, &p), None);
    }

    #[test]
    fn broadcast() {
        let mut slots = vec![];
        w24a09_broadcast_rect(
            [1, 2, 3, 4],
            |s| s != 0x81,
            |s, p| {
                assert_eq!(p[0..2], [2, 0x78]);
                slots.push(s);
            },
        );
        assert_eq!(slots, vec![0x80, 0x82, 0x83]);
    }

    #[test]
    fn packets() {
        assert_eq!(w24a09_build_select_packet(1, 7), ([0x13, 1, 7], true));
        assert_eq!(w24a09_build_u32_packet(0x01020304), [0x0e, 4, 3, 2, 1]);
    }

    #[test]
    fn ids() {
        assert!(w24a09_nation_id_ok(7));
        assert!(!w24a09_nation_id_ok(8));
        assert_eq!(w24a09_reset_net_ids(), [8; 6]);
        let mut n = 0;
        w24a09_net_data_dump((1, 2, 3), (4, 5, 6), |_| n += 1);
        assert_eq!(n, 7);
    }
}
