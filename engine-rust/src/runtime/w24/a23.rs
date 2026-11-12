#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent23.rs
// ===========================================================================
// Wave 24 agent 23: runtime glue helpers (reference only).

/// `0x0076d920`: message handler.  `kind` is `param_2[4]` (low 16 bits),
/// `arg0` is `param_2[0]`.  Kind 0xe with `arg0 == 0` sets bit 31 of the
/// flags dword at `+0x30` and stores 3 at byte `+0x52`; kind 0xf calls the
/// `0x0076cfe0` handler and returns 1; everything else falls through to
/// the default dispatcher (returned by `default_handler`).
pub fn w24a23_message_handler(
    this: &mut [u8],
    arg0: i32,
    kind: i32,
    mut on_kind_f: impl FnMut(),
    mut default_handler: impl FnMut() -> u32,
) -> u32 {
    match kind as i16 {
        0xe => {
            if arg0 == 0 {
                let f = u32::from_le_bytes(this[0x30..0x34].try_into().unwrap()) | 0x8000_0000;
                this[0x30..0x34].copy_from_slice(&f.to_le_bytes());
                this[0x52] = 3;
            }
        }
        0xf => {
            on_kind_f();
            return 1;
        }
        _ => {}
    }
    default_handler()
}

/// `0x0076e150`: if the guard byte at `this[4]` is non-zero, invokes the
/// release callback with `base + 0x24`.  Returns whether it was invoked.
pub fn w24a23_guarded_release(guard: u8, base: u32, mut release: impl FnMut(u32)) -> bool {
    if guard != 0 {
        release(base.wrapping_add(0x24));
        true
    } else {
        false
    }
}

/// `0x0076e170`: if the guard byte is non-zero, invokes the (non-returning
/// in retail) callback with the owner handle; returns whether it ran.
pub fn w24a23_guarded_call(guard: u8, owner: u32, mut call: impl FnMut(u32)) -> bool {
    if guard != 0 {
        call(owner);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests_wave24_agent23 {
    use super::*;

    #[test]
    fn handler_kinds() {
        let mut b = vec![0u8; 0x60];
        let r = w24a23_message_handler(&mut b, 0, 0xe, || {}, || 7);
        assert_eq!(r, 7);
        assert_eq!(b[0x33], 0x80);
        assert_eq!(b[0x52], 3);
        let mut b2 = vec![0u8; 0x60];
        w24a23_message_handler(&mut b2, 1, 0xe, || {}, || 0);
        assert_eq!(b2[0x52], 0);
        let mut hit = false;
        assert_eq!(
            w24a23_message_handler(&mut b2, 0, 0xf, || hit = true, || 9),
            1
        );
        assert!(hit);
        assert_eq!(
            w24a23_message_handler(&mut b2, 0, 0x10000 | 0xe, || {}, || 5),
            5
        );
    }

    #[test]
    fn guards() {
        let mut v = 0;
        assert!(w24a23_guarded_release(1, 0x100, |x| v = x));
        assert_eq!(v, 0x124);
        assert!(!w24a23_guarded_release(0, 0x100, |_| panic!()));
        assert!(w24a23_guarded_call(2, 5, |x| v = x));
        assert_eq!(v, 5);
        assert!(!w24a23_guarded_call(0, 5, |_| panic!()));
    }
}
