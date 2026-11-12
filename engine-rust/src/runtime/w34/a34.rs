#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent34.rs
// ===========================================================================
// Wave 34 translation batch 34.

/// `0x005340c0`: invokes the callback `count` times (loop over a counter).
pub fn w34a34_repeat_call(count: i32, mut f: impl FnMut()) {
    let mut n = count;
    while n != 0 {
        f();
        n = n.wrapping_sub(1);
    }
}

/// Small-buffer string header (`0x005340f0`): size 0, data pointer at the
/// inline buffer, capacity word `0x8000000f`, first inline byte 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W34a34SsoHeader {
    pub ptr_is_inline: bool,
    pub size: u32,
    pub cap: u32,
    pub inline_first: u8,
}

/// `0x005340f0`: initialises an empty inline-buffer string header.
pub fn w34a34_init_empty_sso() -> W34a34SsoHeader {
    W34a34SsoHeader {
        ptr_is_inline: true,
        size: 0,
        cap: 0x8000_000f,
        inline_first: 0,
    }
}

/// AnimationLoader object fields (dword layout; vtable at slot 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W34a34AnimationLoader {
    pub f1: u32,
    pub f2: u32,
    pub f3: u32,
    pub flags: u32,
    pub f5: u32,
    pub f6: u32,
    pub flag_byte: u8,
}

/// `0x00535470`: AnimationLoader constructor; `flags = param_6 | 0x1000`.
pub fn w34a34_animation_loader_init(
    p1: u32,
    p2: u32,
    p3: u32,
    p4: u32,
    p5: u32,
    p6: u32,
    p7: u8,
) -> W34a34AnimationLoader {
    W34a34AnimationLoader {
        f1: p1,
        f2: p2,
        f3: p3,
        flags: p6 | 0x1000,
        f5: p4,
        f6: p5,
        flag_byte: p7,
    }
}

#[cfg(test)]
mod tests_wave34_agent34 {
    use super::*;

    #[test]
    fn repeat() {
        let mut c = 0;
        w34a34_repeat_call(3, || c += 1);
        assert_eq!(c, 3);
    }

    #[test]
    fn sso() {
        assert_eq!(w34a34_init_empty_sso().cap, 0x8000_000f);
    }

    #[test]
    fn loader() {
        let l = w34a34_animation_loader_init(1, 2, 3, 4, 5, 6, 7);
        assert_eq!((l.flags, l.f5, l.f6, l.flag_byte), (0x1006, 4, 5, 7));
    }
}
