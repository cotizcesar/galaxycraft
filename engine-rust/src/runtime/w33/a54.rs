#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent54.rs
// ===========================================================================
// Wave 33 agent 54 (runtime.rs target).

/// `0x00cab042`: atomic-style release of a shared refcounted block.  The
/// count is decremented; when it reaches zero and the block is not the
/// static default block, the block is freed (`free_block`) and the handle
/// is repointed at the default block (`default_id`).  Returns true when the
/// block was released.
pub fn w33a54_release_refcounted<T: Copy + PartialEq>(
    count: &mut i32,
    handle: &mut T,
    default_id: T,
    free_block: impl FnOnce(T),
) -> bool {
    *count = count.wrapping_sub(1);
    if *count == 0 && *handle != default_id {
        free_block(*handle);
        *handle = default_id;
        return true;
    }
    false
}

#[cfg(test)]
mod tests_wave33_agent54 {
    use super::*;

    #[test]
    fn releases_at_zero() {
        let mut c = 1;
        let mut h = 5u32;
        let mut freed = None;
        assert!(w33a54_release_refcounted(&mut c, &mut h, 0, |x| freed = Some(x)));
        assert_eq!((c, h, freed), (0, 0, Some(5)));
    }

    #[test]
    fn keeps_default_and_shared() {
        let mut c = 2;
        let mut h = 5u32;
        assert!(!w33a54_release_refcounted(&mut c, &mut h, 0, |_| panic!()));
        let mut c = 1;
        let mut h = 0u32;
        assert!(!w33a54_release_refcounted(&mut c, &mut h, 0, |_| panic!()));
    }
}
