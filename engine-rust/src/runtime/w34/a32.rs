#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent32.rs
// ===========================================================================
// Wave 34 agent 32 translations.

/// Seven-dword default table written by `0x00513ea0` (globals at
/// 00fa904c..00fa9068, in address order).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W34a32Defaults {
    pub d_904c: u32,
    pub d_9050: u32,
    pub d_9054: u32,
    pub d_9058: u32,
    pub d_905c: u32,
    pub d_9060: u32,
    pub d_9064: u32,
}

/// `0x00513ea0`: stores the constant defaults 5000, 2500, 500, 1700, 100,
/// 2000 (the 64-bit store at the 905c slot) and 500.
pub fn w34a32_init_defaults() -> W34a32Defaults {
    W34a32Defaults {
        d_904c: 5000,
        d_9050: 0x9c4,
        d_9054: 500,
        d_9058: 0x6a4,
        d_905c: 100,
        d_9060: 0x7d0,
        d_9064: 500,
    }
}

#[cfg(test)]
mod tests_wave34_agent32 {
    use super::*;

    #[test]
    fn defaults() {
        let d = w34a32_init_defaults();
        assert_eq!(d.d_904c, 5000);
        assert_eq!(d.d_9050, 2500);
        assert_eq!(d.d_9058, 1700);
        assert_eq!((d.d_905c, d.d_9060), (100, 2000));
    }
}
