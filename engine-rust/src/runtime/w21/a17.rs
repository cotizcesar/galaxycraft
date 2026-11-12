#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave21/agent17.rs
// ===========================================================================
// Wave 21 agent 17: runtime helpers.

/// `0x005e8790`: resets a tree-header (`this` = header "address" `head`):
/// field 1 (+4, old root) is replaced by `head`, field 0 becomes `head`,
/// field 2 (+8, size) becomes 0.  Returns the old root, or `None` when the
/// tree was already empty (old root == head, the early-return path).
pub fn w21a17_reset_tree_header(hdr: &mut [u32; 3], head: u32) -> Option<u32> {
    let old = hdr[1];
    hdr[1] = head;
    hdr[0] = head;
    hdr[2] = 0;
    if old == head { None } else { Some(old) }
}

/// `0x005e93e0`: guard that fails (error code `0x10`) when the global flag
/// is zero; `Ok(())` otherwise.
pub fn w21a17_require_flag(flag: u32) -> Result<(), u32> {
    if flag == 0 { Err(0x10) } else { Ok(()) }
}

#[cfg(test)]
mod tests_wave21_agent17 {
    use super::*;

    #[test]
    fn reset_header() {
        let mut h = [1, 2, 9];
        assert_eq!(w21a17_reset_tree_header(&mut h, 100), Some(2));
        assert_eq!(h, [100, 100, 0]);
        let mut e = [5, 100, 0];
        assert_eq!(w21a17_reset_tree_header(&mut e, 100), None);
        assert_eq!(e, [100, 100, 0]);
    }

    #[test]
    fn flag_guard() {
        assert_eq!(w21a17_require_flag(0), Err(0x10));
        assert_eq!(w21a17_require_flag(1), Ok(()));
    }
}
