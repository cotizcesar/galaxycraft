#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent06.rs
// ===========================================================================
// Wave 33 translation batch 06.
//
// Skipped: FUN_0098b9f0 (both branches tail into an unmodelled callee),
// FUN_0098fa20 (pure thunk into an unmodelled callee).

/// `0x00990310`: reads two optional out-values from a object: `*out_a` gets
/// the field at `+0x584` (`a`), `*out_b` gets `b / a` where `b` is the field
/// at `+0x588`.  Returns 0 on success.  `None` models the original divide
/// fault when `a == 0` and the quotient is requested.
pub fn w33a06_get_count_and_ratio(
    a: u32,
    b: u32,
    out_a: Option<&mut u32>,
    out_b: Option<&mut u32>,
) -> Option<u32> {
    if let Some(o) = out_a {
        *o = a;
    }
    if let Some(o) = out_b {
        *o = b.checked_div(a)?;
    }
    Some(0)
}

#[cfg(test)]
mod tests_wave33_agent06 {
    use super::*;

    #[test]
    fn ratio() {
        let (mut x, mut y) = (0, 0);
        assert_eq!(
            w33a06_get_count_and_ratio(4, 20, Some(&mut x), Some(&mut y)),
            Some(0)
        );
        assert_eq!((x, y), (4, 5));
        assert_eq!(w33a06_get_count_and_ratio(0, 20, None, None), Some(0));
        assert_eq!(w33a06_get_count_and_ratio(0, 20, None, Some(&mut y)), None);
    }
}
