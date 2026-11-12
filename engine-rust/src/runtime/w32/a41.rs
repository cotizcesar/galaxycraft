#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent41.rs
// ===========================================================================
// Wave 32 batch 41: `args::` exception copy constructors (MSVC `std::exception` layout).

/// Vtable tag of a copied exception object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W32a41Vtable {
    /// `std::exception::vftable` (intermediate state during construction).
    StdException,
    /// `args::ParseError::vftable`.
    ParseError,
    /// `args::RequiredError::vftable`.
    RequiredError,
    /// `args::UsageError::vftable`.
    UsageError,
}

/// Exception object: vtable slot plus the 8-byte message record at +4.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W32a41Exception {
    pub vtable: W32a41Vtable,
    pub msg: Option<String>,
}

fn w32a41_copy_ctor<F: FnOnce(&Option<String>, &mut Option<String>)>(
    src: &W32a41Exception,
    final_vtable: W32a41Vtable,
    copy_msg: F,
) -> W32a41Exception {
    // std::exception base: vtable set, message record zeroed, then copied.
    let mut out = W32a41Exception {
        vtable: W32a41Vtable::StdException,
        msg: None,
    };
    copy_msg(&src.msg, &mut out.msg);
    out.vtable = final_vtable;
    out
}

/// `0x005f25b0`: copy constructor of `args::ParseError`.
pub fn w32a41_parse_error_copy(src: &W32a41Exception) -> W32a41Exception {
    w32a41_copy_ctor(src, W32a41Vtable::ParseError, |s, d| d.clone_from(s))
}

/// `0x005f2810`: copy constructor of `args::RequiredError`.
pub fn w32a41_required_error_copy(src: &W32a41Exception) -> W32a41Exception {
    w32a41_copy_ctor(src, W32a41Vtable::RequiredError, |s, d| d.clone_from(s))
}

/// `0x005f28a0`: copy constructor of `args::UsageError`.
pub fn w32a41_usage_error_copy(src: &W32a41Exception) -> W32a41Exception {
    w32a41_copy_ctor(src, W32a41Vtable::UsageError, |s, d| d.clone_from(s))
}

#[cfg(test)]
mod tests_wave32_agent41 {
    use super::*;

    #[test]
    fn copies_keep_message_and_set_vtable() {
        let s = W32a41Exception {
            vtable: W32a41Vtable::ParseError,
            msg: Some("x".into()),
        };
        let a = w32a41_parse_error_copy(&s);
        let b = w32a41_required_error_copy(&s);
        let c = w32a41_usage_error_copy(&s);
        assert_eq!(
            (a.vtable, a.msg.as_deref()),
            (W32a41Vtable::ParseError, Some("x"))
        );
        assert_eq!(b.vtable, W32a41Vtable::RequiredError);
        assert_eq!(c.vtable, W32a41Vtable::UsageError);
    }
}
