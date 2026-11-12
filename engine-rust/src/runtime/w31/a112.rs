#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent112.rs
// ===========================================================================
// Wave 31 translation batch 112: pm_gles state reset (graphics driver glue).

/// Recovered fields of the pm_gles device touched by the reset routine.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct W31a112GlesState {
    /// +0x008
    pub flag8: u8,
    /// +0x134
    pub dirty: u32,
    /// +0x168 enable mask
    pub enable_mask: u32,
    /// +0x174
    pub f174: u32,
    /// +0x180..+0x18c
    pub rect180: [u32; 4],
    /// +0x198..+0x1a4 (float bit patterns)
    pub f198: [u32; 4],
    /// +0x1a8 as 64-bit value
    pub f1a8: u64,
    /// +0x1b8, 0x80 bytes cleared
    pub block: Vec<u8>,
}

/// `0x00c5a370`: resets cached GL state.  The unresolved callees are
/// reported through `calls` in order; the 0x80-byte block at +0x1b8 is zeroed.
pub fn w31a112_reset_state(st: &mut W31a112GlesState, calls: &mut Vec<(&'static str, Vec<u32>)>) {
    st.flag8 = 0;
    st.dirty = 0;
    calls.push(("c5d0c0", vec![0]));
    calls.push(("c5ccf0", vec![0x7800_0000]));
    calls.push(("c5d170", vec![0, 0]));
    let old = st.enable_mask;
    if old != 0 {
        st.enable_mask = 0;
        if old & 1 != 0 {
            calls.push(("gl3674", vec![0]));
        }
        if old & 0x1e != 0 {
            calls.push(("gl3670", vec![0x200]));
        }
    }
    calls.push(("c5d230", vec![]));
    st.f174 = 0;
    calls.push(("c5d120", vec![0]));
    calls.push(("c5d590", vec![0]));
    st.rect180 = [0; 4];
    calls.push(("gl381c", vec![0, 0, 1, 1]));
    calls.push(("gl3678", vec![0, 0x3f80_0000]));
    st.f198 = [0, 0, 0x3f80_0000, 0x3f80_0000];
    st.f1a8 = 0x3f80_0000_0000_0000;
    if st.block.len() < 0x80 {
        st.block.resize(0x80, 0);
    }
    st.block[..0x80].fill(0);
}

#[cfg(test)]
mod tests_wave31_agent112 {
    use super::*;

    #[test]
    fn reset_clears() {
        let mut st = W31a112GlesState {
            enable_mask: 0x3,
            dirty: 5,
            block: vec![7; 0x80],
            ..Default::default()
        };
        let mut calls = Vec::new();
        w31a112_reset_state(&mut st, &mut calls);
        assert_eq!(st.enable_mask, 0);
        assert_eq!(st.dirty, 0);
        assert!(st.block.iter().all(|&b| b == 0));
        assert!(calls.iter().any(|c| c.0 == "gl3674"));
        assert!(calls.iter().any(|c| c.0 == "gl3670" && c.1 == vec![0x200]));
        assert_eq!(st.f198[2], 0x3f80_0000);
    }

    #[test]
    fn reset_no_mask() {
        let mut st = W31a112GlesState::default();
        let mut calls = Vec::new();
        w31a112_reset_state(&mut st, &mut calls);
        assert!(
            !calls
                .iter()
                .any(|c| c.0.starts_with("gl367") && c.0 != "gl3678")
        );
    }
}
