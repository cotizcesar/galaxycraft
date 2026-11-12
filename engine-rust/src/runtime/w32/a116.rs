#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent116.rs
// ===========================================================================
// Wave 32 agent 116: GLES command-list render-state diffing.

/// GL call emitted by the state-diff helpers (function pointers in the
/// retail GL dispatch table, modelled as data).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W32a116Gl {
    /// Disable call with a capability argument (dispatch slot `3680`).
    Disable(u32),
    /// Enable call (dispatch slot `3690`).
    Enable,
    /// Two-argument call (dispatch slot `360c`).
    Pair(u32, u32),
    /// Four-argument call (dispatch slot `3614`).
    Quad(u32, u32, u32, u32),
    /// Four-flag call (dispatch slot `3634`).
    Flags(u32, u32, u32, u32),
}

/// `0x00c5ccf0`: diffs the packed render-state word `new` against the first
/// of eight cached words (offset `0x148`), updates all eight and returns the
/// GL calls.  `table_a` is the 8-entry table at `0xe3_ef54`, `table_b` the
/// 32-entry table at `0xe3_ef10`, `table_c` the 32-entry table at `0xe3_ef68`.
pub fn w32a116_apply_state(
    cache: &mut [u32; 8],
    new: u32,
    table_a: &[u32; 8],
    table_b: &[u32; 32],
    table_c: &[u32; 32],
) -> Vec<W32a116Gl> {
    let mut out = Vec::new();
    let diff = cache[0] ^ new;
    if diff == 0 {
        return out;
    }
    if diff & 1 != 0 {
        out.push(if new & 1 == 0 {
            W32a116Gl::Disable(0xbe2)
        } else {
            W32a116Gl::Enable
        });
    }
    if new & 1 == 0 {
        let hi = new & 0x7800_0000;
        for c in cache.iter_mut() {
            *c = (*c & 0x87ff_fffe) | hi;
        }
    } else {
        *cache = [new; 8];
        if ((diff >> 3) | diff) & 0xe != 0 {
            out.push(W32a116Gl::Pair(
                table_a[(new >> 1 & 7) as usize],
                table_a[(new >> 4 & 7) as usize],
            ));
        }
        if ((((diff >> 5) | diff) >> 5 | diff) >> 5 | diff) & 0xf80 > 0x7f {
            out.push(W32a116Gl::Quad(
                table_b[(new >> 7 & 0x1f) as usize],
                table_b[(new >> 12 & 0x1f) as usize],
                table_c[(new >> 17 & 0x1f) as usize],
                table_c[(new >> 22 & 0x1f) as usize],
            ));
        }
    }
    if diff & 0x7800_0000 != 0 {
        let b = (new >> 24) as u8 as u32;
        out.push(W32a116Gl::Flags(
            b >> 3 & 1,
            b >> 4 & 1,
            b >> 5 & 1,
            b >> 6 & 1,
        ));
    }
    out
}

/// `0x00c5d0c0`: stores the new enable-bit mask and, for each changed bit
/// (lowest first), emits disable(`caps[bit]`) when cleared or enable when
/// set.  `caps` is the table at `0xe3_f018`.
pub fn w32a116_apply_enable_bits(cache: &mut u32, new: u32, caps: &[u32; 32]) -> Vec<W32a116Gl> {
    let mut diff = *cache ^ new;
    *cache = new;
    let mut out = Vec::new();
    while diff != 0 {
        let bit = diff.trailing_zeros();
        diff &= !(1u32 << bit);
        out.push(if new >> bit & 1 == 0 {
            W32a116Gl::Disable(caps[bit as usize])
        } else {
            W32a116Gl::Enable
        });
    }
    out
}

#[cfg(test)]
mod tests_wave32_agent116 {
    use super::*;

    #[test]
    fn enable_bits() {
        let mut caps = [0u32; 32];
        caps[2] = 77;
        let mut c = 0b100;
        let o = w32a116_apply_enable_bits(&mut c, 0b001, &caps);
        assert_eq!(o, vec![W32a116Gl::Enable, W32a116Gl::Disable(77)]);
        assert_eq!(c, 1);
    }

    #[test]
    fn state() {
        let mut cache = [0u32; 8];
        let o = w32a116_apply_state(&mut cache, 1, &[5; 8], &[6; 32], &[7; 32]);
        assert_eq!(o[0], W32a116Gl::Enable);
        assert_eq!(cache, [1; 8]);
        let o = w32a116_apply_state(&mut cache, 0x0800_0000, &[5; 8], &[6; 32], &[7; 32]);
        assert_eq!(o[0], W32a116Gl::Disable(0xbe2));
        assert_eq!(cache[3], 0x0800_0000);
        assert!(matches!(o[1], W32a116Gl::Flags(1, 0, 0, 0)));
    }
}
