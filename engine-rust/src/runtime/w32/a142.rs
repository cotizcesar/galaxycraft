#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent142.rs
// ===========================================================================
// Wave 32 translation batch 142: tiny wrappers over caller-supplied callees.

/// `0x00ca0b5d`: forwards to the strtol-style parser with endptr = null,
/// base 10 and flag byte 1.  The parser is supplied by the caller as
/// `parse(locale, text, endptr, base, flags)`.
pub fn w32a142_parse_base10<R>(
    text: &str,
    parse: impl FnOnce(u32, &str, Option<usize>, u32, u8) -> R,
) -> R {
    parse(0, text, None, 10, 1)
}

/// `0x00ca13a4`: forwards directly to its single callee with no arguments.
pub fn w32a142_forward<R>(callee: impl FnOnce() -> R) -> R {
    callee()
}

/// Handle record of `0x00ca2946`: two optional resource handles at +8 and +0xc.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct W32a142Record {
    pub res_a: u32,
    pub res_b: u32,
}

/// `0x00ca2946`: releases the record held in `slot` (if any): frees `res_a`
/// when non-zero, then `res_b` when non-zero, then the record itself, and
/// finally clears the slot.
pub fn w32a142_release(
    slot: &mut Option<W32a142Record>,
    mut free_a: impl FnMut(u32),
    mut free_b: impl FnMut(u32),
) {
    if let Some(rec) = slot.take() {
        if rec.res_a != 0 {
            free_a(rec.res_a);
        }
        if rec.res_b != 0 {
            free_b(rec.res_b);
        }
    }
}

#[cfg(test)]
mod tests_wave32_agent142 {
    use super::*;

    #[test]
    fn parse_args() {
        let r = w32a142_parse_base10("42", |l, t, e, b, f| (l, t.len(), e, b, f));
        assert_eq!(r, (0, 2, None, 10, 1));
    }

    #[test]
    fn forward_calls() {
        assert_eq!(w32a142_forward(|| 7), 7);
    }

    #[test]
    fn release_order() {
        let mut slot = Some(W32a142Record { res_a: 1, res_b: 0 });
        let (mut a, mut b) = (vec![], vec![]);
        w32a142_release(&mut slot, |x| a.push(x), |x| b.push(x));
        assert!(slot.is_none());
        assert_eq!(a, vec![1]);
        assert!(b.is_empty());
    }
}
