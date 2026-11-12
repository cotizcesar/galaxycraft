#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave30/agent11.rs
// ===========================================================================
// Wave 30 agent 11: weak-pointer lock, async token bind check and lazy-init
// guard helpers.

/// `0x00b1c090`: weak-pointer lock.  `strong` is the control block's strong
/// count (offset 4).  Returns `true` and increments it only if it is
/// non-zero (CAS loop); a zero count yields an empty pointer.
pub fn w30a11_weak_lock(strong: &std::sync::atomic::AtomicI32) -> bool {
    use std::sync::atomic::Ordering;
    let mut cur = strong.load(Ordering::SeqCst);
    while cur != 0 {
        match strong.compare_exchange(cur, cur + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => return true,
            Err(actual) => cur = actual,
        }
    }
    false
}

/// One 0x18-byte async token slot (fields at +8, +0xc, +0x10, +0x18, +0x1c).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W30a11TokenSlot {
    pub id: u32,
    pub generation: i32,
    pub flags: u8,
    pub refs: i32,
    pub pending: i32,
}

/// `0x00b1cfd0`: token bind.  The slot is `token & 0xff`; binding is valid
/// when id and generation match and either the slot is open (`flags & 3 == 0`
/// and `pending == 0`) or `force` is set.  On success `refs += delta` and
/// `true` is returned; otherwise the retail code logs "failed attempt to bind
/// to closed or non-existing token ID" (AsyncTokenTable) and returns `false`.
pub fn w30a11_token_bind(
    slots: &mut [W30a11TokenSlot],
    token: u32,
    generation: i32,
    delta: i32,
    force: bool,
) -> bool {
    let Some(s) = slots.get_mut((token & 0xff) as usize) else {
        return false;
    };
    if s.id != token || s.generation != generation {
        return false;
    }
    if ((s.flags & 3) != 0 || s.pending != 0) && !force {
        return false;
    }
    s.refs = s.refs.wrapping_add(delta);
    true
}

/// `0x00b1d340`: lazy-init guard.  If not yet `done`, takes the spin flag,
/// runs `init` when not `constructed`, releases the flag.  Returns `true`
/// when the object is usable (retail returns `this`, or null once `done`
/// is set).
pub fn w30a11_lazy_get(done: bool, constructed: &mut bool, init: impl FnOnce()) -> bool {
    if !done {
        if !*constructed {
            init();
            *constructed = true;
        }
    }
    !done
}

/// `0x00b1d470`: same lazy-init guard as `0x00b1d340` without a return
/// value; runs `init` once when not done and not constructed.
pub fn w30a11_lazy_init(done: bool, constructed: &mut bool, init: impl FnOnce()) {
    let _ = w30a11_lazy_get(done, constructed, init);
}

#[cfg(test)]
mod tests_wave30_agent11 {
    use super::*;
    use std::sync::atomic::{AtomicI32, Ordering};

    #[test]
    fn weak_lock() {
        let a = AtomicI32::new(2);
        assert!(w30a11_weak_lock(&a));
        assert_eq!(a.load(Ordering::SeqCst), 3);
        let z = AtomicI32::new(0);
        assert!(!w30a11_weak_lock(&z));
        assert_eq!(z.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn bind() {
        let mut v = vec![W30a11TokenSlot::default(); 256];
        v[5] = W30a11TokenSlot {
            id: 0x105,
            generation: 7,
            ..Default::default()
        };
        assert!(w30a11_token_bind(&mut v, 0x105, 7, 3, false));
        assert_eq!(v[5].refs, 3);
        assert!(!w30a11_token_bind(&mut v, 0x105, 8, 1, false));
        assert!(!w30a11_token_bind(&mut v, 0x205, 7, 1, false));
        v[5].flags = 1;
        assert!(!w30a11_token_bind(&mut v, 0x105, 7, 1, false));
        assert!(w30a11_token_bind(&mut v, 0x105, 7, 1, true));
        assert_eq!(v[5].refs, 4);
    }

    #[test]
    fn lazy() {
        let mut c = false;
        let mut n = 0;
        assert!(w30a11_lazy_get(false, &mut c, || n += 1));
        assert!(w30a11_lazy_get(false, &mut c, || n += 1));
        assert_eq!(n, 1);
        assert!(!w30a11_lazy_get(true, &mut c, || n += 1));
        let mut c2 = false;
        w30a11_lazy_init(false, &mut c2, || n += 1);
        assert_eq!(n, 2);
    }
}
