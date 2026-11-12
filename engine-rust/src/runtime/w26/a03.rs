#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent03.rs
// ===========================================================================
// Wave 26 translation batch 03: runtime function-object slots and error mapping.

/// Outcome of `w26a03_classify_error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W26a03ErrorAction {
    /// The `FUN_0070a410(1)` call is made (code 7).
    pub call_reset: bool,
    /// The held resource (`iStack_5b8`) is released when non-null.
    pub release_resource: bool,
    /// Message id passed to `FUN_006f9720`, if any.
    pub message: Option<u32>,
}

/// `0x00907820`: maps the result code of the dialog/loop call to the action
/// taken: 7 resets, -1 and 0xffe0 do nothing, -3/3/6/0x3ea release the
/// resource and show message 2, 0x3e9 shows 0xe, anything else shows 0xc.
pub fn w26a03_classify_error(code: i32) -> W26a03ErrorAction {
    let (call_reset, release_resource, message) = match code {
        7 => (true, false, None),
        -1 | 0xffe0 => (false, false, None),
        -3 | 3 | 6 | 0x3ea => (false, true, Some(2)),
        0x3e9 => (false, false, Some(0xe)),
        _ => (false, false, Some(0xc)),
    };
    W26a03ErrorAction {
        call_reset,
        release_resource,
        message,
    }
}

/// `0x0090ca70`: when `mode == 2` and the callee succeeded, stores the
/// decoded u16 at offset `0x70` of the target record and returns 1.
pub fn w26a03_store_u16_at_70(record: &mut [u8], mode: i32, decoded: Option<u16>) -> u32 {
    if mode == 2
        && let Some(v) = decoded
        && record.len() >= 0x72
    {
        record[0x70..0x72].copy_from_slice(&v.to_le_bytes());
        return 1;
    }
    0
}

/// Shared body of the function-object slot reset: the slot word has bit 0 =
/// "heap-owned"; the target pointer is the word with bit 0 cleared (inline
/// otherwise).  Returns `(new_word, needs_free)`.
fn w26a03_reset_slot(word: u32, mut call_target: impl FnMut(u32)) -> (u32, bool) {
    let owned = word & 1 != 0;
    let target = if owned { word & !1 } else { word };
    if target != 0 {
        call_target(target);
        if owned {
            return (word, true);
        }
        return (1, false);
    }
    (word, false)
}

/// `0x0090b600`: resets the function-object slot at object offset `0x318`
/// (call the target's first vtable entry with 0, then slot = 1 when inline).
pub fn w26a03_reset_slot_318(word: u32, call_target: impl FnMut(u32)) -> (u32, bool) {
    w26a03_reset_slot(word, call_target)
}

/// `0x0090bbf0`: same reset for the slot at object offset `0x310`.
pub fn w26a03_reset_slot_310(word: u32, call_target: impl FnMut(u32)) -> (u32, bool) {
    w26a03_reset_slot(word, call_target)
}

#[cfg(test)]
mod tests_wave26_agent03 {
    use super::*;

    #[test]
    fn error_map() {
        assert!(w26a03_classify_error(7).call_reset);
        assert_eq!(w26a03_classify_error(-1).message, None);
        assert_eq!(w26a03_classify_error(0xffe0).message, None);
        for c in [-3, 3, 6, 0x3ea] {
            let a = w26a03_classify_error(c);
            assert!(a.release_resource);
            assert_eq!(a.message, Some(2));
        }
        assert_eq!(w26a03_classify_error(0x3e9).message, Some(0xe));
        assert_eq!(w26a03_classify_error(8).message, Some(0xc));
        assert_eq!(w26a03_classify_error(0).message, Some(0xc));
    }

    #[test]
    fn store_u16() {
        let mut r = [0u8; 0x80];
        assert_eq!(w26a03_store_u16_at_70(&mut r, 2, Some(0x1234)), 1);
        assert_eq!(&r[0x70..0x72], &[0x34, 0x12]);
        assert_eq!(w26a03_store_u16_at_70(&mut r, 1, Some(1)), 0);
        assert_eq!(w26a03_store_u16_at_70(&mut r, 2, None), 0);
    }

    #[test]
    fn slots() {
        let mut calls = vec![];
        assert_eq!(w26a03_reset_slot_318(0x1000, |t| calls.push(t)), (1, false));
        assert_eq!(calls, vec![0x1000]);
        assert_eq!(
            w26a03_reset_slot_310(0x2001, |t| calls.push(t)),
            (0x2001, true)
        );
        assert_eq!(calls, vec![0x1000, 0x2000]);
        assert_eq!(w26a03_reset_slot_310(0, |_| panic!()), (0, false));
        assert_eq!(w26a03_reset_slot_318(1, |_| panic!()), (1, false));
    }
}
