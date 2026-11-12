#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent29.rs
// ===========================================================================
// Wave 24 agent 29: small runtime helpers.

/// RNG state used by `0x00791440` (three 32-bit words).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W24a29Rng {
    /// DAT_00f30890.
    pub s0: u32,
    /// DAT_00f30888.
    pub s1: u32,
    /// DAT_00f3088c.
    pub s2: u32,
}

/// `0x00791440`: ranged random.  Steps the mixer until the masked value
/// is below `hi - lo + 1`, then returns it plus `lo`.  `mask_p1` is
/// `param_3` (the mask is `param_3 - 1`).
pub fn w24a29_rand_range(st: &mut W24a29Rng, lo: u32, hi: u32, mask_p1: u32) -> u32 {
    loop {
        st.s0 = st.s0.wrapping_add(st.s1).wrapping_add(st.s2);
        st.s1 = st.s1.wrapping_add(st.s2) >> 1;
        st.s2 = st.s2.wrapping_add(0x7182_3402) ^ st.s0;
        let v = (st.s1 ^ st.s2) & mask_p1.wrapping_sub(1);
        if v < hi.wrapping_sub(lo).wrapping_add(1) {
            return v.wrapping_add(lo);
        }
    }
}

/// `0x00791ef0`: when the flag is set, clears it and runs the two
/// callbacks (`(handle, 4)` then `(handle, 3)`).  Returns whether it ran.
pub fn w24a29_clear_flag_and_notify(
    flag: &mut u32,
    handle: u32,
    mut call_009784c0: impl FnMut(u32, u32),
    mut call_007919d0: impl FnMut(u32, u32),
) -> bool {
    if *flag == 0 {
        return false;
    }
    *flag = 0;
    call_009784c0(handle, 4);
    call_007919d0(handle, 3);
    true
}

/// `0x00793090`: stores the vtable pointer into the object's first word.
pub fn w24a29_set_vtable(obj: &mut u32, vtable: u32) {
    *obj = vtable;
}

/// `0x00793010`: lazily-initialised static record of five words
/// `[8, 0, 0, 1.0f32 bits, 0x75]`.  `inited` is the init guard.
pub fn w24a29_static_record(inited: &mut bool, rec: &mut [u32; 5]) -> [u32; 5] {
    if !*inited {
        *rec = [8, 0, 0, 0x3f80_0000, 0x75];
        *inited = true;
    }
    *rec
}

/// `0x00793300`: copies the non-zero prefix of the 12-slot table, dropping
/// entries equal to `exclude`.  The result is what is passed to the
/// follow-up call.
pub fn w24a29_filter_table(table: &[i32; 12], exclude: i32) -> Vec<i32> {
    table
        .iter()
        .copied()
        .take_while(|&v| v != 0)
        .filter(|&v| v != exclude)
        .collect()
}

#[cfg(test)]
mod tests_wave24_agent29 {
    use super::*;

    #[test]
    fn rand_in_range() {
        let mut st = W24a29Rng {
            s0: 1,
            s1: 2,
            s2: 3,
        };
        for _ in 0..100 {
            let v = w24a29_rand_range(&mut st, 5, 12, 16);
            assert!((5..=12).contains(&v));
        }
        let mut a = W24a29Rng {
            s0: 1,
            s1: 2,
            s2: 3,
        };
        let v = w24a29_rand_range(&mut a, 0, 0xffff, 0x10000);
        assert_eq!(a.s0, 6);
        assert_eq!(a.s1, 2);
        assert_eq!(a.s2, 0x7182_3405 ^ 6);
        assert_eq!(v, (2 ^ (0x7182_3405 ^ 6)) & 0xffff);
    }

    #[test]
    fn notify_and_vtable() {
        let mut f = 1;
        let mut log = vec![];
        let mut l2 = vec![];
        assert!(w24a29_clear_flag_and_notify(
            &mut f,
            9,
            |a, b| log.push((a, b)),
            |a, b| l2.push((a, b))
        ));
        assert_eq!((f, log, l2), (0, vec![(9, 4)], vec![(9, 3)]));
        assert!(!w24a29_clear_flag_and_notify(
            &mut f,
            9,
            |_, _| {},
            |_, _| {}
        ));
        let mut o = 0;
        w24a29_set_vtable(&mut o, 0xf3089c);
        assert_eq!(o, 0xf3089c);
    }

    #[test]
    fn record_and_filter() {
        let mut i = false;
        let mut r = [0; 5];
        assert_eq!(
            w24a29_static_record(&mut i, &mut r),
            [8, 0, 0, 0x3f80_0000, 0x75]
        );
        assert!(i);
        let mut t = [0; 12];
        t[..4].copy_from_slice(&[1, 2, 3, 4]);
        assert_eq!(w24a29_filter_table(&t, 2), vec![1, 3, 4]);
        t[2] = 0;
        assert_eq!(w24a29_filter_table(&t, 9), vec![1, 2]);
    }
}
