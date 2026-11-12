#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent38.rs
// ===========================================================================
// Wave 33 agent 38: MSVC exception copy-constructors (vtable retag).

/// Vtable tags the three constructors store at word 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W33a38Vtable {
    Base,
    LogicError,
    OutOfRange,
    RegexError,
}

/// Model of the exception object words (`this[0]` vtable, `this[3]` regex code).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W33a38Exc {
    pub vtable: W33a38Vtable,
    pub words: [u32; 4],
}

/// `0x00c83fb3`: copy-construct base from `src`, then set vtable to
/// `std::logic_error`.
pub fn w33a38_logic_error_copy(this: &mut W33a38Exc, src: &W33a38Exc) {
    this.words = src.words;
    this.vtable = W33a38Vtable::LogicError;
}

/// `0x00c83fce`: copy-construct base from `src`, then set vtable to
/// `std::out_of_range`.
pub fn w33a38_out_of_range_copy(this: &mut W33a38Exc, src: &W33a38Exc) {
    this.words = src.words;
    this.vtable = W33a38Vtable::OutOfRange;
}

/// `0x00c8400b`: copy-construct base, set vtable to `std::regex_error`,
/// then copy word 3 (the error code) from `src`.
pub fn w33a38_regex_error_copy(this: &mut W33a38Exc, src: &W33a38Exc) {
    this.words = src.words;
    this.vtable = W33a38Vtable::RegexError;
    this.words[3] = src.words[3];
}

#[cfg(test)]
mod tests_wave33_agent38 {
    use super::*;

    #[test]
    fn copies_and_retags() {
        let src = W33a38Exc {
            vtable: W33a38Vtable::Base,
            words: [1, 2, 3, 4],
        };
        let mut d = W33a38Exc {
            vtable: W33a38Vtable::Base,
            words: [0; 4],
        };
        w33a38_logic_error_copy(&mut d, &src);
        assert_eq!(d.vtable, W33a38Vtable::LogicError);
        w33a38_out_of_range_copy(&mut d, &src);
        assert_eq!(d.vtable, W33a38Vtable::OutOfRange);
        w33a38_regex_error_copy(&mut d, &src);
        assert_eq!((d.vtable, d.words[3]), (W33a38Vtable::RegexError, 4));
    }
}
