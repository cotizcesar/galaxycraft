#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent43.rs
// ===========================================================================
// Wave 32 agent 43: MSVC `std::system_error` family copy constructors.

/// Model of the 0x14-byte exception object: vftable tag, 8-byte message
/// slot (words 1..2), error value (word 3) and category pointer (word 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W32a43SystemError {
    pub vftable: u32,
    pub msg: u64,
    pub code_value: u32,
    pub category: u32,
}

pub const W32A43_VT_EXCEPTION: u32 = 1;
pub const W32A43_VT_SYSTEM_ERROR_BASE: u32 = 2;
pub const W32A43_VT_SYSTEM_ERROR: u32 = 3;
pub const W32A43_VT_IOS_FAILURE: u32 = 4;

fn w32a43_copy_ctor(
    src: &W32a43SystemError,
    copy_msg: impl Fn(u64) -> u64,
    final_vt: u32,
) -> W32a43SystemError {
    // vftable is set to std::exception, message cleared then copied by the
    // exception-copy helper, then the _System_error vftable, code and
    // category copied, finally the most-derived vftable.
    let _ = (W32A43_VT_EXCEPTION, W32A43_VT_SYSTEM_ERROR_BASE);
    W32a43SystemError {
        vftable: final_vt,
        msg: copy_msg(src.msg),
        code_value: src.code_value,
        category: src.category,
    }
}

/// `0x005f2ca0`: `std::ios_base::failure` copy constructor.
pub fn w32a43_ios_failure_copy(
    src: &W32a43SystemError,
    copy_msg: impl Fn(u64) -> u64,
) -> W32a43SystemError {
    w32a43_copy_ctor(src, copy_msg, W32A43_VT_IOS_FAILURE)
}

/// `0x005f2eb0`: `std::system_error` copy constructor.
pub fn w32a43_system_error_copy(
    src: &W32a43SystemError,
    copy_msg: impl Fn(u64) -> u64,
) -> W32a43SystemError {
    w32a43_copy_ctor(src, copy_msg, W32A43_VT_SYSTEM_ERROR)
}

#[cfg(test)]
mod tests_wave32_agent43 {
    use super::*;

    #[test]
    fn copies_fields() {
        let s = W32a43SystemError {
            vftable: 9,
            msg: 5,
            code_value: 7,
            category: 11,
        };
        let a = w32a43_ios_failure_copy(&s, |m| m);
        assert_eq!(a.vftable, W32A43_VT_IOS_FAILURE);
        assert_eq!((a.msg, a.code_value, a.category), (5, 7, 11));
        let b = w32a43_system_error_copy(&s, |m| m + 1);
        assert_eq!(b.vftable, W32A43_VT_SYSTEM_ERROR);
        assert_eq!(b.msg, 6);
    }
}
