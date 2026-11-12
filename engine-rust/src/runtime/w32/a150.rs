#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent150.rs
// ===========================================================================
// Wave 32 agent 150: CRT helpers (runtime.rs).

/// `0x00caa21a`: looks up an optional callback; when present, calls it and
/// returns 1 if it reported nonzero, else 0.
pub fn w32a150_call_optional(callback: Option<&mut dyn FnMut() -> i32>) -> u32 {
    match callback {
        Some(cb) => u32::from(cb() != 0),
        None => 0,
    }
}

/// `0x00caa914`: overflow-checked allocation of `count * 4 + a * b` bytes.
/// Returns 0 when the arguments overflow; otherwise the allocator result.
/// `alloc(size)` models the allocator, `release()` the trailing cleanup call.
pub fn w32a150_checked_alloc(
    count: u32,
    a: u32,
    b: u32,
    alloc: &mut dyn FnMut(u32) -> u32,
    release: &mut dyn FnMut(),
) -> u32 {
    if count < 0x3fff_ffff && b != 0 && (a as u64) < 0xffff_ffffu64 / b as u64 {
        let prod = a.wrapping_mul(b);
        if prod < !count.wrapping_mul(4) {
            let r = alloc(count.wrapping_mul(4).wrapping_add(prod));
            release();
            return r;
        }
    }
    0
}

/// `0x00caa99a`: one-time initialisation of the global handle.  Returns 0 if
/// already set, `0xffffffff` on failure, 0 on success (both slots set).
pub fn w32a150_init_once(
    slot_a: &mut u32,
    slot_b: &mut u32,
    pre: &mut dyn FnMut(),
    acquire: &mut dyn FnMut() -> u32,
    build: &mut dyn FnMut(u32) -> u32,
    release: &mut dyn FnMut(u32),
) -> u32 {
    if *slot_a != 0 {
        return 0;
    }
    pre();
    let h = acquire();
    let ret = if h == 0 {
        0xffff_ffff
    } else {
        let v = build(h);
        let r = if v == 0 {
            0xffff_ffff
        } else {
            *slot_a = v;
            *slot_b = v;
            0
        };
        release(0);
        r
    };
    release(h);
    ret
}

#[cfg(test)]
mod tests_wave32_agent150 {
    use super::*;

    #[test]
    fn optional() {
        assert_eq!(w32a150_call_optional(None), 0);
        assert_eq!(w32a150_call_optional(Some(&mut || 5)), 1);
        assert_eq!(w32a150_call_optional(Some(&mut || 0)), 0);
    }

    #[test]
    fn alloc() {
        let mut n = 0;
        assert_eq!(
            w32a150_checked_alloc(2, 3, 4, &mut |s| s, &mut || n += 1),
            20
        );
        assert_eq!(n, 1);
        assert_eq!(
            w32a150_checked_alloc(0x3fff_ffff, 1, 1, &mut |s| s, &mut || {}),
            0
        );
        assert_eq!(
            w32a150_checked_alloc(1, 0xffff_fff0, 16, &mut |s| s, &mut || {}),
            0
        );
        assert_eq!(w32a150_checked_alloc(1, 1, 0, &mut |s| s, &mut || {}), 0);
    }

    #[test]
    fn init() {
        let (mut a, mut b) = (0, 0);
        assert_eq!(
            w32a150_init_once(
                &mut a,
                &mut b,
                &mut || {},
                &mut || 7,
                &mut |h| h + 1,
                &mut |_| {}
            ),
            0
        );
        assert_eq!((a, b), (8, 8));
        assert_eq!(
            w32a150_init_once(
                &mut a,
                &mut b,
                &mut || {},
                &mut || 0,
                &mut |h| h,
                &mut |_| {}
            ),
            0
        );
        let (mut c, mut d) = (0, 0);
        assert_eq!(
            w32a150_init_once(
                &mut c,
                &mut d,
                &mut || {},
                &mut || 0,
                &mut |h| h,
                &mut |_| {}
            ),
            0xffff_ffff
        );
        assert_eq!(
            w32a150_init_once(
                &mut c,
                &mut d,
                &mut || {},
                &mut || 3,
                &mut |_| 0,
                &mut |_| {}
            ),
            0xffff_ffff
        );
    }
}
