#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent36.rs
// ===========================================================================
// Wave 31 translation batch 36.

/// `0x007a9080`: de-obfuscates a stored global value: `(v ^ 0x10fae560) + 0xeb70`
/// with 32-bit wrapping arithmetic.
pub fn w31a36_deobfuscate_global(stored: u32) -> u32 {
    (stored ^ 0x10fa_e560).wrapping_add(0xeb70)
}

/// Layout of the object reset by `0x007afcf0` (dword indices into `this`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W31a36Context {
    pub words: Vec<u32>,
}

impl W31a36Context {
    pub fn new() -> Self {
        Self {
            words: vec![0xdead_beef; 0x10024],
        }
    }
}

impl Default for W31a36Context {
    fn default() -> Self {
        Self::new()
    }
}

/// `0x007afcf0`: resets the context header fields (dword 0, and dwords
/// 0x1001b..=0x10022, with 0x1001e set to 0x102 and the u16 at 0x10023 cleared),
/// then calls the follow-up reset `after` (callee `FUN_007afdc0`).
pub fn w31a36_reset_context(ctx: &mut W31a36Context, after: impl FnOnce(&mut W31a36Context)) {
    let w = &mut ctx.words;
    w[0] = 0;
    w[0x1001b] = 0;
    w[0x1001c] = 0;
    w[0x1001d] = 0;
    w[0x1001e] = 0x102;
    w[0x1001f] = 0;
    w[0x10020] = 0;
    w[0x10021] = 0;
    w[0x10022] = 0;
    w[0x10023] &= 0xffff_0000;
    after(ctx);
}

#[cfg(test)]
mod tests_wave31_agent36 {
    use super::*;

    #[test]
    fn deobfuscate() {
        assert_eq!(w31a36_deobfuscate_global(0x10fa_e560), 0xeb70);
        assert_eq!(
            w31a36_deobfuscate_global(0x10fa_e560 ^ 0xffff_ffff),
            0xffff_ffffu32.wrapping_add(0xeb70)
        );
    }

    #[test]
    fn reset() {
        let mut c = W31a36Context::new();
        let mut called = false;
        w31a36_reset_context(&mut c, |_| called = true);
        assert!(called);
        assert_eq!(c.words[0x1001e], 0x102);
        assert_eq!(c.words[0], 0);
        assert_eq!(c.words[0x10023], 0xdead_0000);
        assert_eq!(c.words[0x10022], 0);
        assert_eq!(c.words[1], 0xdead_beef);
    }
}
