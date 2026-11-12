#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent124.rs
// ===========================================================================
// Wave 31 agent 124: MSVC C++ exception record helpers.

/// MSVC C++ exception code (`0xE06D7363`).
pub const W31A124_CXX_EXCEPTION_CODE: i32 = -0x1f92_8c9d;

/// `0x00c8801a` (predicate part): true when the record is a C++ exception
/// (code `0xE06D7363`, 3 parameters, magic number one of the known
/// `0x19930520..22` / `0x01994000` values); the retail body then calls a
/// non-returning terminate, otherwise it returns 0.
pub fn w31a124_is_cxx_exception(code: i32, num_params: u32, magic: i32) -> bool {
    code == W31A124_CXX_EXCEPTION_CODE
        && num_params == 3
        && matches!(magic, 0x1993_0520 | 0x1993_0521 | 0x1993_0522 | 0x0199_4000)
}

/// Copied exception record (first part of `0x00c853f1`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct W31a124Record {
    pub code: i32,
    pub flags: i32,
    pub zero_a: i32,
    pub zero_b: i32,
    pub num_params: u32,
    pub params: [i32; 15],
    pub is_alloc_flag: u8,
}

/// `0x00c853f1` (copy prefix): copies code/flags, clears the two link
/// fields, copies up to 15 parameters and zero-fills the rest.
pub fn w31a124_copy_record(
    code: i32,
    flags: i32,
    num_params: u32,
    src_params: &[i32],
    flag: u8,
) -> W31a124Record {
    let mut r = W31a124Record {
        code,
        flags,
        num_params,
        is_alloc_flag: flag,
        ..Default::default()
    };
    for i in 0..(num_params.min(15) as usize) {
        r.params[i] = src_params.get(i).copied().unwrap_or(0);
    }
    r
}

#[cfg(test)]
mod tests_wave31_agent124 {
    use super::*;

    #[test]
    fn predicate() {
        assert!(w31a124_is_cxx_exception(-0x1f92_8c9d, 3, 0x1993_0522));
        assert!(!w31a124_is_cxx_exception(-0x1f92_8c9d, 2, 0x1993_0520));
        assert!(!w31a124_is_cxx_exception(1, 3, 0x1993_0520));
        assert!(!w31a124_is_cxx_exception(-0x1f92_8c9d, 3, 5));
    }

    #[test]
    fn copy() {
        let r = w31a124_copy_record(1, 2, 20, &[7; 20], 1);
        assert_eq!(r.params, [7; 15]);
        let r = w31a124_copy_record(1, 2, 2, &[7, 8], 0);
        assert_eq!(&r.params[..3], &[7, 8, 0]);
    }
}
