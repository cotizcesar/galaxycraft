#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent127.rs
// ===========================================================================
// Wave 32 translation batch 127.
//
// Skipped: FUN_00c8c5bf (ends in a non-returning call).

/// `0x00c8cddb`: compatibility check between three flag records.  `a_flags`,
/// `b_flags`, `c_flags` are the leading bytes of params 1..3; `a_name` and
/// `b_name` are the wide strings at `+8` of each record's pointee (`None` =
/// null pointer at `+4`).  `same_obj` is true when both `+4` pointers are equal.
pub fn w32a127_flags_compatible(
    a_flags: u8,
    a_name: Option<&[u16]>,
    b_flags: u8,
    b_name: Option<&[u16]>,
    same_obj: bool,
    c_flags: u8,
) -> bool {
    let an = match a_name {
        Some(n) if n.first().copied().unwrap_or(0) != 0 => n,
        _ => return true,
    };
    if a_flags & 0x80 != 0 && b_flags & 0x10 != 0 {
        return true;
    }
    if !same_obj {
        let bn = b_name.unwrap_or(&[]);
        let mut i = 0;
        loop {
            let x = an.get(i).copied().unwrap_or(0);
            let y = bn.get(i).copied().unwrap_or(0);
            if x != y {
                return false;
            }
            if x == 0 {
                break;
            }
            i += 1;
        }
    }
    (b_flags & 2 == 0 || a_flags & 8 != 0)
        && (c_flags & 1 == 0 || a_flags & 1 != 0)
        && (c_flags & 2 == 0 || a_flags & 2 != 0)
}

/// `0x00c93722`: chooses between two handlers; returns `true` when the
/// second (`FUN_00cad39f`) is selected, i.e. the pointer is non-null and the
/// pointee's field at `+4` is at least 2.
pub fn w32a127_use_second_handler(present: bool, field4: i32) -> bool {
    present && field4 >= 2
}

#[cfg(test)]
mod tests_wave32_agent127 {
    use super::*;
    #[test]
    fn compat() {
        let n: Vec<u16> = vec![65, 0];
        assert!(w32a127_flags_compatible(0, None, 0, None, false, 0));
        assert!(w32a127_flags_compatible(
            0x80,
            Some(&n),
            0x10,
            None,
            false,
            3
        ));
        assert!(!w32a127_flags_compatible(
            0,
            Some(&n),
            0,
            Some(&[66, 0]),
            false,
            0
        ));
        assert!(w32a127_flags_compatible(0, Some(&n), 0, None, true, 0));
        assert!(!w32a127_flags_compatible(0, Some(&n), 2, None, true, 0));
        assert!(!w32a127_flags_compatible(0, Some(&n), 0, None, true, 1));
    }
    #[test]
    fn handler() {
        assert!(!w32a127_use_second_handler(false, 5));
        assert!(!w32a127_use_second_handler(true, 1));
        assert!(w32a127_use_second_handler(true, 2));
    }
}
