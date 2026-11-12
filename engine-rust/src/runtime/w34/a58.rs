#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent58.rs
// ===========================================================================
// Wave 34 translation batch 58.
//
// Skipped: FUN_00caafd9 (calls a non-modelled CRT initialiser),
// FUN_00cad363 (SEH/CRT stream glue with uninitialised locals).

/// `0xca_db44` helper (not cited as implemented): replicate low 6 bits.
fn w34a58_replicate(v: u32) -> u32 {
    (((v & 0x3f) << 8 | (v & 0x3f)) << 16) | v
}

/// Reorders the six exception-flag bits of an x87/SSE status value
/// (helper body of the FPU status reader used by `0x00cad705`).
fn w34a58_swizzle(s: u32) -> u32 {
    let s = s & 0x3f;
    (((s >> 1) & 1) << 5)
        | (((s >> 2) & 1) << 3)
        | (((s >> 3) & 1) << 2)
        | (((s >> 4) & 1) * 2)
        | ((s & 1) << 4)
        | (s >> 5)
}

/// `0x00cad705`: CRT `_clearfp`-style status mask. `fpu_status` and
/// `mxcsr` stand for the hardware registers, `has_sse` for the global
/// feature flag. When `mask & 0x1f == 0` the output is 0, else the swizzled
/// status word ANDed with the mask replicated across bytes. Returns 0.
pub fn w34a58_fp_status_mask(
    out: &mut u32,
    mask: u32,
    fpu_status: u16,
    mxcsr: u32,
    has_sse: bool,
) -> u32 {
    if mask & 0x1f == 0 {
        *out = 0;
    } else {
        let x = w34a58_swizzle(fpu_status as u32);
        let m = if has_sse {
            w34a58_swizzle(mxcsr)
        } else {
            w34a58_swizzle(0)
        };
        let status = (((m << 8) | x) << 16) | m | x;
        *out = w34a58_replicate(mask & 0x1f) & status;
    }
    0
}

#[cfg(test)]
mod tests_wave34_agent58 {
    use super::*;

    #[test]
    fn zero_mask() {
        let mut o = 5;
        assert_eq!(w34a58_fp_status_mask(&mut o, 0x20, 0x3f, 0x3f, true), 0);
        assert_eq!(o, 0);
    }

    #[test]
    fn masked() {
        let mut o = 0;
        w34a58_fp_status_mask(&mut o, 0x1f, 0x01, 0, false);
        assert_eq!(o, 0x0010_0010 & 0x1f1f_1f1f | 0);
    }
}
