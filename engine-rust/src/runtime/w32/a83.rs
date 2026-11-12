#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent83.rs
// ===========================================================================
// Wave 32 translation batch 83.

/// Context record (7 dwords) initialised by `w32a83_init_context`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct W32a83Ctx {
    pub first: u32,
    pub desc_present: bool,
    pub status: u32,
    pub field3: u32,
    pub flags: u32,
    pub param: u32,
    pub state: Option<u32>,
}

/// Descriptor (4 dwords): word0, kind (must be 5), flags, init callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W32a83Desc {
    pub word0: u32,
    pub kind: u32,
    pub flags: u32,
}

/// `0x00a04e50`: validates version 10 and the descriptor, then runs the
/// descriptor's init callback.  `init` returns (status, optional state word
/// stored at ctx[6]); `cleanup` stands for the teardown helper called on
/// failure.  Returns the status, also stored in `ctx.status` when present.
pub fn w32a83_init_context(
    ctx: Option<&mut W32a83Ctx>,
    desc: Option<&W32a83Desc>,
    param3: u32,
    req_flags: u32,
    version: i32,
    init: impl FnOnce(&mut W32a83Ctx) -> u32,
    cleanup: impl FnOnce(&mut W32a83Ctx),
) -> u32 {
    let mut ctx = ctx;
    let status = (|| {
        if version != 10 {
            return 3;
        }
        let (Some(c), Some(d)) = (ctx.as_deref_mut(), desc) else {
            return 8;
        };
        if d.kind != 5 {
            return 3;
        }
        if (req_flags & 0x10000 != 0 && d.flags & 0x40000 == 0)
            || (req_flags & 0x20000 != 0 && d.flags & 0x80000 == 0)
            || (req_flags & 0x40000 != 0 && d.flags & 0x100000 == 0)
            || d.flags & 1 == 0
        {
            return 4;
        }
        *c = W32a83Ctx {
            first: d.word0,
            desc_present: true,
            status: 0,
            field3: 0,
            flags: req_flags,
            param: param3,
            state: None,
        };
        let r = init(c);
        if r != 0 {
            c.field3 = c.state.unwrap_or(0);
            cleanup(c);
        }
        r
    })();
    if let Some(c) = ctx {
        c.status = status;
    }
    status
}

#[cfg(test)]
mod tests_wave32_agent83 {
    use super::*;

    const D: W32a83Desc = W32a83Desc {
        word0: 7,
        kind: 5,
        flags: 1,
    };

    #[test]
    fn init_paths() {
        let mut c = W32a83Ctx::default();
        assert_eq!(
            w32a83_init_context(Some(&mut c), Some(&D), 0, 0, 9, |_| 0, |_| {}),
            3
        );
        assert_eq!(
            w32a83_init_context(None, Some(&D), 0, 0, 10, |_| 0, |_| {}),
            8
        );
        let bad = W32a83Desc { kind: 4, ..D };
        assert_eq!(
            w32a83_init_context(Some(&mut c), Some(&bad), 0, 0, 10, |_| 0, |_| {}),
            3
        );
        assert_eq!(
            w32a83_init_context(Some(&mut c), Some(&D), 0, 0x10000, 10, |_| 0, |_| {}),
            4
        );
        let nf = W32a83Desc { flags: 0, ..D };
        assert_eq!(
            w32a83_init_context(Some(&mut c), Some(&nf), 0, 0, 10, |_| 0, |_| {}),
            4
        );
        assert_eq!(
            w32a83_init_context(Some(&mut c), Some(&D), 5, 0, 10, |_| 0, |_| {}),
            0
        );
        assert_eq!(c.first, 7);
        assert_eq!(c.param, 5);
        let mut called = false;
        let r = w32a83_init_context(Some(&mut c), Some(&D), 5, 0, 10, |_| 2, |_| called = true);
        assert_eq!((r, called, c.status), (2, true, 2));
    }
}
