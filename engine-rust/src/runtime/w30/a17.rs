#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave30/agent17.rs
// ===========================================================================
// Wave 30 agent 17: tact file-tree helpers (pure reference models).

/// `0x00b33410`: initialises a stream-slot record from a six-word source
/// descriptor; words 1..7 are copied and word 7 is `flags | 0x80`.
pub fn w30a17_init_slot_flags(src: &[u32; 6], flags: u32) -> [u32; 8] {
    let mut out = [0u32; 8];
    out[1..7].copy_from_slice(src);
    out[7] = flags | 0x80;
    out
}

/// `0x00b334c0`: initialises an 0x12-word record from a six-word source;
/// words 1..7 are copied and words 8..0x12 are zeroed.
pub fn w30a17_init_slot_zeroed(src: &[u32; 6]) -> [u32; 0x12] {
    let mut out = [0u32; 0x12];
    out[1..7].copy_from_slice(src);
    out
}

/// `0x00b335c0`: identical to the `0x00b334c0` initialiser (separate
/// template instance).
pub fn w30a17_init_slot_zeroed_b(src: &[u32; 6]) -> [u32; 0x12] {
    w30a17_init_slot_zeroed(src)
}

/// `0x00b34a50`: path-join length check.  Returns `false` when the root
/// length plus separator plus the relative length exceeds `0x104`
/// (MAX_PATH), the case where the original raises an error.
pub fn w30a17_path_fits(root_len: u32, rel_len: u32) -> bool {
    root_len.wrapping_add(1).wrapping_add(rel_len) <= 0x104
}

/// Outcome of the open/create flag decision in [`w30a17_create_dir_plan`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum W30a17DirPlan {
    /// Neither read nor write bit set: nothing happens.
    Nothing,
    /// Proceed to resolve the path without creating it.
    Resolve,
    /// Create the directory, then resolve.
    CreateThenResolve,
    /// Read-only tree: "failed attempt to create a subdirectory" error.
    ErrReadOnly,
}

/// `0x00b35a40`: decides what to do from the mode `flags` (bits 0..2 access,
/// bit 2 = create dir) and the read-only state of the tree.
pub fn w30a17_create_dir_plan(flags: u32, read_only: bool) -> W30a17DirPlan {
    if flags & 3 == 0 {
        return W30a17DirPlan::Nothing;
    }
    if flags & 4 != 0 {
        if read_only {
            return W30a17DirPlan::ErrReadOnly;
        }
        return W30a17DirPlan::CreateThenResolve;
    }
    W30a17DirPlan::Resolve
}

/// `0x00b35a40` (second half): mkdir failure handling.  `errno` 7 is the
/// tolerated "exists" code; with the exclusive bit (8) it is an error.
/// Returns `Ok(())` if execution continues, `Err(())` on a logged error.
pub fn w30a17_mkdir_failure(errno: i32, flags: u32) -> Result<(), ()> {
    if errno != 7 || flags & 8 != 0 {
        Err(())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests_wave30_agent17 {
    use super::*;

    #[test]
    fn slot_init() {
        let s = [1, 2, 3, 4, 5, 6];
        let a = w30a17_init_slot_flags(&s, 1);
        assert_eq!(a, [0, 1, 2, 3, 4, 5, 6, 0x81]);
        let b = w30a17_init_slot_zeroed(&s);
        assert_eq!(&b[1..7], &s);
        assert!(b[8..].iter().all(|&x| x == 0));
        assert_eq!(w30a17_init_slot_zeroed_b(&s), b);
    }

    #[test]
    fn path_len() {
        assert!(w30a17_path_fits(0x100, 3));
        assert!(!w30a17_path_fits(0x100, 4));
    }

    #[test]
    fn dir_plan() {
        assert_eq!(w30a17_create_dir_plan(0, false), W30a17DirPlan::Nothing);
        assert_eq!(w30a17_create_dir_plan(1, false), W30a17DirPlan::Resolve);
        assert_eq!(
            w30a17_create_dir_plan(5, false),
            W30a17DirPlan::CreateThenResolve
        );
        assert_eq!(w30a17_create_dir_plan(5, true), W30a17DirPlan::ErrReadOnly);
        assert!(w30a17_mkdir_failure(7, 0).is_ok());
        assert!(w30a17_mkdir_failure(7, 8).is_err());
        assert!(w30a17_mkdir_failure(2, 0).is_err());
    }
}
