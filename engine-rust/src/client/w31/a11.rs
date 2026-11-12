#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent11.rs
// ===========================================================================
// Wave 31 agent 11 translations.

/// `0x007fa6c0`: returns 1 when both objects have a non-zero field at
/// offset 4 and the callee `FUN_00646d60` (supplied as `check`) accepts
/// the second object; otherwise 0.
pub fn w31a11_both_ready_and_check(
    a_field4: u32,
    b_field4: u32,
    check: impl FnOnce() -> u32,
) -> u32 {
    if a_field4 != 0 && b_field4 != 0 && check() != 0 {
        1
    } else {
        0
    }
}

/// Recovered view of the record compared by `0x00807c00`.
#[derive(Clone, Debug, Default)]
pub struct W31a11Record {
    pub w0: u32,
    pub w1: u32,
    /// Sub-object at word 2 (compared through `FUN_00539640`).
    pub sub_a: u32,
    /// Sub-object at word 9 (compared through `FUN_00539640`).
    pub sub_b: u32,
    /// Bytes at offsets 0x40..=0x49.
    pub flags: [u8; 10],
    pub w15: u32,
    /// Values of the intrusive list (sentinel at word 0x13).
    pub list: Vec<u32>,
}

/// `0x00807c00`: returns 0 when the two records are equal, 1 otherwise.
/// `sub_eq` models `FUN_00539640`.  If the other list is shorter than
/// this one the records are reported different.
pub fn w31a11_record_differs(
    this: &W31a11Record,
    other: &W31a11Record,
    sub_eq: impl Fn(u32, u32) -> bool,
) -> u32 {
    let same = this.w0 == other.w0
        && this.w1 == other.w1
        && sub_eq(this.sub_a, other.sub_a)
        && sub_eq(this.sub_b, other.sub_b)
        && this.flags == other.flags
        && this.w15 == other.w15
        && this.list.len() <= other.list.len()
        && this.list.iter().zip(other.list.iter()).all(|(a, b)| a == b);
    if same { 0 } else { 1 }
}

#[cfg(test)]
mod tests_wave31_agent11 {
    use super::*;

    #[test]
    fn ready_check() {
        assert_eq!(w31a11_both_ready_and_check(1, 1, || 5), 1);
        assert_eq!(w31a11_both_ready_and_check(0, 1, || 5), 0);
        assert_eq!(w31a11_both_ready_and_check(1, 0, || 5), 0);
        assert_eq!(w31a11_both_ready_and_check(1, 1, || 0), 0);
    }

    #[test]
    fn record_compare() {
        let a = W31a11Record {
            list: vec![1, 2],
            ..Default::default()
        };
        let mut b = a.clone();
        assert_eq!(w31a11_record_differs(&a, &b, |x, y| x == y), 0);
        b.flags[3] = 1;
        assert_eq!(w31a11_record_differs(&a, &b, |x, y| x == y), 1);
        b = a.clone();
        b.list[1] = 9;
        assert_eq!(w31a11_record_differs(&a, &b, |x, y| x == y), 1);
        assert_eq!(w31a11_record_differs(&a, &a, |_, _| false), 1);
    }
}
