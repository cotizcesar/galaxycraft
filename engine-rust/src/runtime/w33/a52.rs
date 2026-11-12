#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent52.rs
// ===========================================================================
// Wave 33 translation batch 52.
// Skipped: FUN_00caa96e (reads opaque CRT globals, calls unmodelled init helpers).

/// Object touched by `0x00ca6eb5` (fields at +0xc and +0x14).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct W33a52Slot {
    pub value: u32,
    pub flag: bool,
}

/// `0x00ca6eb5`: runs a pre-step, then calls the inner helper; on a zero
/// result stores `value` and sets the flag, otherwise clears both and
/// returns the helper's result.
pub fn w33a52_set_slot(
    slot: &mut W33a52Slot,
    value: u32,
    pre: impl FnOnce(),
    inner: impl FnOnce(u32) -> i32,
) -> i32 {
    pre();
    let r = inner(value);
    if r == 0 {
        slot.flag = true;
        slot.value = value;
        0
    } else {
        slot.value = 0;
        slot.flag = false;
        r
    }
}

/// `0x00caa9ed`: copies an environment block (NUL-separated strings ending
/// with an empty string), skipping entries that start with `=`.  Returns
/// the copied strings (the C pointer array minus its null terminator).
pub fn w33a52_copy_environment(block: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < block.len() && block[pos] != 0 {
        let end = block[pos..]
            .iter()
            .position(|&b| b == 0)
            .map_or(block.len(), |p| pos + p);
        if block[pos] != b'=' {
            out.push(block[pos..end].to_vec());
        }
        pos = end + 1;
    }
    out
}

#[cfg(test)]
mod tests_wave33_agent52 {
    use super::*;

    #[test]
    fn slot_success_and_failure() {
        let mut s = W33a52Slot::default();
        assert_eq!(w33a52_set_slot(&mut s, 7, || {}, |_| 0), 0);
        assert_eq!(
            s,
            W33a52Slot {
                value: 7,
                flag: true
            }
        );
        assert_eq!(w33a52_set_slot(&mut s, 9, || {}, |_| 22), 22);
        assert_eq!(
            s,
            W33a52Slot {
                value: 0,
                flag: false
            }
        );
    }

    #[test]
    fn env_copy_skips_equals() {
        let b = b"A=1\0=C:=x\0B=2\0\0";
        assert_eq!(
            w33a52_copy_environment(b),
            vec![b"A=1".to_vec(), b"B=2".to_vec()]
        );
        assert!(w33a52_copy_environment(b"\0").is_empty());
    }
}
