#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent30.rs
// ===========================================================================
// Wave 24 agent 30 reference helpers (peer-slot bookkeeping glue).

/// `0x007938f0`: returns the callee's integer result (FUN_007b30e0) divided
/// by two, using C signed division (truncation toward zero).
pub fn w24a30_half_of(value: i32) -> i32 {
    value / 2
}

/// `0x00795740`: finds the peer slot whose id byte (offset `0x21a`) equals
/// `id`.  `id == 0xff` means "none" and returns `None` at once.  The
/// pseudocode scans the list at DAT_00f30b18 first, then the one at
/// DAT_00f30b0c; a list whose head has bit 0 set is treated as empty.
/// Each list is modelled as a slice of id bytes; the result is
/// `(list_index, position)` with list 0 = first scanned (DAT_00f30b18).
pub fn w24a30_find_peer_by_id(
    primary: Option<&[u8]>,
    secondary: Option<&[u8]>,
    id: i32,
) -> Option<(usize, usize)> {
    if id == 0xff {
        return None;
    }
    for (li, list) in [primary, secondary].into_iter().enumerate() {
        if let Some(ids) = list
            && let Some(pos) = ids.iter().position(|&b| b == id as u8)
        {
            return Some((li, pos));
        }
    }
    None
}

/// `0x00795eb0`: records an acknowledged sequence number.  When a network
/// provider is installed (DAT_011b2100 != 0) the original takes a
/// non-returning path and this returns `false` without touching anything.
/// Otherwise it stores the u16 at message offset 6 into the peer's u16 table
/// at `0x1ee + kind * 2` and clears the u32 at `0x1f8 + kind * 4`
/// (`kind` is the message byte at offset 8).  `peer` must be at least
/// `0x1f8 + 4 * (kind + 1)` bytes.
pub fn w24a30_record_ack(peer: &mut [u8], msg: &[u8], provider_installed: bool) -> bool {
    if provider_installed {
        return false;
    }
    let kind = msg[8] as usize;
    let seq = [msg[6], msg[7]];
    peer[0x1ee + kind * 2..0x1ee + kind * 2 + 2].copy_from_slice(&seq);
    peer[0x1f8 + kind * 4..0x1f8 + kind * 4 + 4].fill(0);
    true
}

#[cfg(test)]
mod tests_wave24_agent30 {
    use super::*;

    #[test]
    fn half_truncates_toward_zero() {
        assert_eq!(w24a30_half_of(7), 3);
        assert_eq!(w24a30_half_of(-7), -3);
        assert_eq!(w24a30_half_of(0), 0);
    }

    #[test]
    fn find_peer_order_and_none() {
        let a = [1u8, 2, 3];
        let b = [9u8, 2];
        assert_eq!(w24a30_find_peer_by_id(Some(&a), Some(&b), 2), Some((0, 1)));
        assert_eq!(w24a30_find_peer_by_id(Some(&a), Some(&b), 9), Some((1, 0)));
        assert_eq!(w24a30_find_peer_by_id(None, Some(&b), 2), Some((1, 1)));
        assert_eq!(w24a30_find_peer_by_id(Some(&a), Some(&b), 0xff), None);
        assert_eq!(w24a30_find_peer_by_id(Some(&a), None, 7), None);
    }

    #[test]
    fn record_ack_writes_tables() {
        let mut peer = vec![0xaau8; 0x220];
        let mut msg = [0u8; 9];
        msg[6] = 0x34;
        msg[7] = 0x12;
        msg[8] = 2;
        assert!(w24a30_record_ack(&mut peer, &msg, false));
        assert_eq!(&peer[0x1f2..0x1f4], &[0x34, 0x12]);
        assert_eq!(&peer[0x200..0x204], &[0, 0, 0, 0]);
        let mut p2 = vec![1u8; 0x220];
        assert!(!w24a30_record_ack(&mut p2, &msg, true));
        assert!(p2.iter().all(|&b| b == 1));
    }
}
