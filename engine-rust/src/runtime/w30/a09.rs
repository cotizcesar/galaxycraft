#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave30/agent09.rs
// ===========================================================================
// Wave 30 translation batch 09.  Almost the whole queue is TACT (Blizzard
// CDN / content-delivery) glue: skipped as "bnet" or non-returning chains.

/// `0x00b15b30`: lazily creates a shared object in an atomic 64-bit slot.
/// Returns the slot value if non-zero; otherwise, when `create` is true,
/// builds a value with `make`, compare-exchanges it into the slot from 0 and,
/// if another thread won, destroys its own value with `destroy` and returns
/// the winner's value.  With `create == false` an empty slot yields 0.
pub fn w30a09_lazy_init_slot(
    slot: &std::sync::atomic::AtomicU64,
    create: bool,
    make: impl FnOnce() -> u64,
    destroy: impl FnOnce(u64),
) -> u64 {
    use std::sync::atomic::Ordering::SeqCst;
    let cur = slot.load(SeqCst);
    if cur != 0 || !create {
        return cur;
    }
    let mine = make();
    match slot.compare_exchange(0, mine, SeqCst, SeqCst) {
        Ok(_) => mine,
        Err(winner) => {
            destroy(mine);
            winner
        }
    }
}

#[cfg(test)]
mod tests_wave30_agent09 {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn lazy_init() {
        let s = AtomicU64::new(0);
        assert_eq!(w30a09_lazy_init_slot(&s, false, || 5, |_| {}), 0);
        assert_eq!(w30a09_lazy_init_slot(&s, true, || 5, |_| {}), 5);
        assert_eq!(w30a09_lazy_init_slot(&s, true, || 9, |_| panic!()), 5);
    }

    #[test]
    fn lazy_init_race_destroys_loser() {
        let s = AtomicU64::new(0);
        let mut d = 0;
        let r = w30a09_lazy_init_slot(
            &s,
            true,
            || {
                s.store(7, Ordering::SeqCst);
                8
            },
            |v| d = v,
        );
        assert_eq!((r, d), (7, 8));
    }
}
