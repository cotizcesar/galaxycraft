#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent08.rs
// ===========================================================================
// Wave 29 translation batch 08 (tact client-handler glue, reference helpers).

/// `0x00ac2510`: lazy once-init of a service slot.  `disabled` is the byte at
/// +9, `init_done` the byte at +8, `value` the dword at +0.  Returns whether
/// the slot is usable (the original returns the object or null when the
/// disabled flag is set); on first use it zeroes the value and marks init.
pub fn w29a08_lazy_slot_get(disabled: bool, init_done: &mut bool, value: &mut u32) -> bool {
    if !disabled && !*init_done {
        *value = 0;
        *init_done = true;
    }
    !disabled
}

/// `0x00ac2960`: dispatches to the handler when present.  Returns `Err` when
/// all four 32-bit ids are zero (original raises a ClientHandler error),
/// otherwise `Ok(true)` if the handler ran and `Ok(false)` if absent.
pub fn w29a08_dispatch_ids(
    ids: [u32; 4],
    handler: Option<&mut dyn FnMut([u32; 4])>,
) -> Result<bool, ()> {
    if ids == [0; 4] {
        return Err(());
    }
    match handler {
        Some(h) => {
            h(ids);
            Ok(true)
        }
        None => Ok(false),
    }
}

/// `0x00ac47c0`: precondition check.  Fails when no streaming module is
/// configured and the flag byte at +0xa7 is clear.
pub fn w29a08_check_streaming_ready(module_present: bool, flag_a7: bool) -> Result<(), ()> {
    if !module_present && !flag_a7 {
        Err(())
    } else {
        Ok(())
    }
}

/// `0x00ac4ba0` (reuse branch): for each of the three slots (+0x424/+0x428/
/// +0x42c, sourced from +0x30/+0x34/+0x38) an empty destination adopts the
/// non-null source and add-refs it.  Returns the number of adopted slots.
pub fn w29a08_adopt_components(
    dst: &mut [u32; 3],
    src: [u32; 3],
    mut add_ref: impl FnMut(u32),
) -> usize {
    let mut n = 0;
    for i in 0..3 {
        if dst[i] == 0 && src[i] != 0 {
            dst[i] = src[i];
            add_ref(src[i]);
            n += 1;
        }
    }
    n
}

/// `0x00ac5390`: keyring key intake.  `len` is the byte at +0x99; zero means
/// no key (`Ok(None)`), any value except 16 is an error, otherwise the 16
/// key bytes are returned.
pub fn w29a08_keyring_key(len: u8, key: &[u8]) -> Result<Option<[u8; 16]>, ()> {
    match len {
        0 => Ok(None),
        16 if key.len() >= 16 => {
            let mut out = [0u8; 16];
            out.copy_from_slice(&key[..16]);
            Ok(Some(out))
        }
        _ => Err(()),
    }
}

/// `0x00ac3f60`: zero-initialises `count` 24-byte (6 dword) records.
pub fn w29a08_zero_records(recs: &mut [u32], count: usize) {
    for r in recs.chunks_mut(6).take(count) {
        for w in r.iter_mut() {
            *w = 0;
        }
    }
}

#[cfg(test)]
mod tests_wave29_agent08 {
    use super::*;

    #[test]
    fn lazy_slot() {
        let (mut d, mut v) = (false, 7);
        assert!(w29a08_lazy_slot_get(false, &mut d, &mut v));
        assert!(d && v == 0);
        let (mut d2, mut v2) = (false, 7);
        assert!(!w29a08_lazy_slot_get(true, &mut d2, &mut v2));
        assert!(!d2 && v2 == 7);
    }

    #[test]
    fn dispatch() {
        assert!(w29a08_dispatch_ids([0; 4], None).is_err());
        assert_eq!(w29a08_dispatch_ids([0, 0, 0, 1], None), Ok(false));
        let mut seen = 0;
        let mut f = |i: [u32; 4]| seen = i[3];
        assert_eq!(w29a08_dispatch_ids([0, 0, 0, 5], Some(&mut f)), Ok(true));
        assert_eq!(seen, 5);
    }

    #[test]
    fn streaming_ready() {
        assert!(w29a08_check_streaming_ready(false, false).is_err());
        assert!(w29a08_check_streaming_ready(true, false).is_ok());
        assert!(w29a08_check_streaming_ready(false, true).is_ok());
    }

    #[test]
    fn adopt() {
        let mut d = [0, 9, 0];
        let mut refs = vec![];
        let n = w29a08_adopt_components(&mut d, [1, 2, 0], |p| refs.push(p));
        assert_eq!(n, 1);
        assert_eq!(d, [1, 9, 0]);
        assert_eq!(refs, vec![1]);
    }

    #[test]
    fn keyring() {
        assert_eq!(w29a08_keyring_key(0, &[]), Ok(None));
        assert!(w29a08_keyring_key(15, &[0; 16]).is_err());
        assert_eq!(w29a08_keyring_key(16, &[3; 16]), Ok(Some([3; 16])));
    }

    #[test]
    fn zero() {
        let mut r = [1u32; 18];
        w29a08_zero_records(&mut r, 2);
        assert!(r[..12].iter().all(|&x| x == 0));
        assert!(r[12..].iter().all(|&x| x == 1));
    }
}
