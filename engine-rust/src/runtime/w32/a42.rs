#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent42.rs
// ===========================================================================
// Wave 32 translation batch 42: exception copy constructors.

/// Class tag written as the final vftable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W32a42Kind {
    ValidationError,
    SystemError,
    BadCast,
}

/// Copy-constructed exception object: name holder plus optional error words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W32a42Exception {
    pub kind: W32a42Kind,
    pub name: crate::runtime::W31a125Name,
    /// `this[3]`, `this[4]` (system error code and category), if present.
    pub code: Option<(u32, u32)>,
}

fn w32a42_copy(
    kind: W32a42Kind,
    src: &crate::runtime::W31a125Name,
    code: Option<(u32, u32)>,
) -> W32a42Exception {
    let mut name = crate::runtime::W31a125Name::default();
    crate::runtime::w31a125_name_copy(src, &mut name);
    W32a42Exception { kind, name, code }
}

/// `0x005f2930`: copy constructor of `args::ValidationError`; copies the name
/// holder via `0xc8_94fe` and sets the final vftable.
pub fn w32a42_validation_error_copy(src: &crate::runtime::W31a125Name) -> W32a42Exception {
    w32a42_copy(W32a42Kind::ValidationError, src, None)
}

/// `0x005f2c10`: copy constructor of `std::_System_error`; as above plus the
/// two words at source offsets `0xc` and `0x10`.
pub fn w32a42_system_error_copy(
    src: &crate::runtime::W31a125Name,
    code: u32,
    category: u32,
) -> W32a42Exception {
    w32a42_copy(W32a42Kind::SystemError, src, Some((code, category)))
}

/// `0x005f2c60`: copy constructor of `std::bad_cast`.
pub fn w32a42_bad_cast_copy(src: &crate::runtime::W31a125Name) -> W32a42Exception {
    w32a42_copy(W32a42Kind::BadCast, src, None)
}

#[cfg(test)]
mod tests_wave32_agent42 {
    use super::*;
    use crate::runtime::W31a125Name;

    #[test]
    fn copies() {
        let owned = W31a125Name {
            data: Some("x".into()),
            owned: true,
        };
        let e = w32a42_validation_error_copy(&owned);
        assert_eq!(e.kind, W32a42Kind::ValidationError);
        assert!(e.name.owned);
        let shared = W31a125Name {
            data: Some("y".into()),
            owned: false,
        };
        let e = w32a42_system_error_copy(&shared, 5, 7);
        assert!(!e.name.owned);
        assert_eq!(e.code, Some((5, 7)));
        assert_eq!(w32a42_bad_cast_copy(&owned).kind, W32a42Kind::BadCast);
    }
}
