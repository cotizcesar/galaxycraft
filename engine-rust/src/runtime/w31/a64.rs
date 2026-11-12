#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent64.rs
// ===========================================================================
// Wave 31 agent 64: small field-update helpers (Vorbis/mixer-style glue).

/// `0x009e4350`: clears a four-dword record.
pub fn w31a64_zero_quad(p: &mut [u32; 4]) {
    *p = [0; 4];
}

/// `0x009e2a30`: updates the pending-slot word at `+0x24` of the sub-record
/// and the flags word (`+0x120`).  `lock` stands for the critical-section
/// pair around the clear case; `linked_id` is the `+0x11c` field of the
/// linked object (`None` when the pointer at `+0x180` is null).
pub fn w31a64_set_slot_a(
    enable: bool,
    locked: bool,
    flags: &mut u32,
    slot: &mut u32,
    linked_id: Option<u32>,
    mut lock: impl FnMut(bool),
) {
    if !enable {
        *slot = 0xffff_ffff;
        *flags &= 0xffff_fff7;
        return;
    }
    if locked {
        lock(true);
        *slot = 0;
        lock(false);
        return;
    }
    if let Some(id) = linked_id {
        *slot = id.wrapping_add(1);
    }
}

/// `0x009e3e50`: like the previous helper but the slot lives at `+0x168`
/// and a locked call still falls through to the linked-id update.
pub fn w31a64_set_slot_b(
    enable: bool,
    locked: bool,
    flags: &mut u32,
    slot: &mut u32,
    linked_id: Option<u32>,
    mut lock: impl FnMut(bool),
) {
    if !enable {
        *flags &= 0xffff_fff7;
        *slot = 0xffff_ffff;
        return;
    }
    if locked {
        lock(true);
        *slot = 0;
        lock(false);
    }
    if let Some(id) = linked_id {
        *slot = id.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests_wave31_agent64 {
    use super::*;

    #[test]
    fn zero_quad() {
        let mut q = [1, 2, 3, 4];
        w31a64_zero_quad(&mut q);
        assert_eq!(q, [0; 4]);
    }

    #[test]
    fn slot_a() {
        let (mut f, mut s) = (0xf, 5);
        w31a64_set_slot_a(false, false, &mut f, &mut s, None, |_| {});
        assert_eq!((f, s), (0x7, 0xffff_ffff));
        let mut calls = vec![];
        w31a64_set_slot_a(true, true, &mut f, &mut s, Some(9), |b| calls.push(b));
        assert_eq!((s, calls), (0, vec![true, false]));
        w31a64_set_slot_a(true, false, &mut f, &mut s, Some(9), |_| {});
        assert_eq!(s, 10);
    }

    #[test]
    fn slot_b() {
        let (mut f, mut s) = (0xf, 5);
        w31a64_set_slot_b(false, false, &mut f, &mut s, None, |_| {});
        assert_eq!((f, s), (0x7, 0xffff_ffff));
        w31a64_set_slot_b(true, true, &mut f, &mut s, Some(3), |_| {});
        assert_eq!(s, 4);
        w31a64_set_slot_b(true, true, &mut f, &mut s, None, |_| {});
        assert_eq!(s, 0);
    }
}
