#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent38.rs
// ===========================================================================
// Wave 32 translation batch 38: MSVC exception copy constructors.

/// Which vftable an exception object points at after construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W32a38Vtable {
    StdException,
    RuntimeError,
    StreamError,
}

/// Exception object: vtable tag plus the two-word name holder at `+4`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W32a38Exception {
    pub vtable: W32a38Vtable,
    pub name: Option<String>,
    pub owned: bool,
}

fn w32a38_base_copy(src: &W32a38Exception) -> W32a38Exception {
    // Zero the holder, then copy via the name-copy helper (owning sources
    // are duplicated, non-owning ones shared).
    let mut dst = W32a38Exception {
        vtable: W32a38Vtable::StdException,
        name: None,
        owned: false,
    };
    match (&src.name, src.owned) {
        (Some(s), true) => {
            dst.name = Some(s.clone());
            dst.owned = true;
        }
        _ => dst.name = src.name.clone(),
    }
    dst
}

/// `0x00535570`: `std::exception` copy constructor.
pub fn w32a38_exception_copy(src: &W32a38Exception) -> W32a38Exception {
    w32a38_base_copy(src)
}

/// `0x005355a0`: `std::runtime_error` copy constructor (base copy, then
/// the runtime_error vftable is installed).
pub fn w32a38_runtime_error_copy(src: &W32a38Exception) -> W32a38Exception {
    let mut d = w32a38_base_copy(src);
    d.vtable = W32a38Vtable::RuntimeError;
    d
}

/// `0x005355e0`: `stdext::stream_error` copy constructor.
pub fn w32a38_stream_error_copy(src: &W32a38Exception) -> W32a38Exception {
    let mut d = w32a38_base_copy(src);
    d.vtable = W32a38Vtable::StreamError;
    d
}

#[cfg(test)]
mod tests_wave32_agent38 {
    use super::*;

    fn src(owned: bool) -> W32a38Exception {
        W32a38Exception {
            vtable: W32a38Vtable::RuntimeError,
            name: Some("x".into()),
            owned,
        }
    }

    #[test]
    fn copies() {
        let d = w32a38_exception_copy(&src(true));
        assert_eq!(d.vtable, W32a38Vtable::StdException);
        assert!(d.owned);
        assert_eq!(
            w32a38_runtime_error_copy(&src(false)).vtable,
            W32a38Vtable::RuntimeError
        );
        let s = w32a38_stream_error_copy(&src(false));
        assert_eq!(s.vtable, W32a38Vtable::StreamError);
        assert!(!s.owned);
    }
}
