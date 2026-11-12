#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent76.rs
// ===========================================================================
// Wave 31 translation batch 76.
//
// Most of this queue is STL/SEH glue or ends in non-returning calls and
// was skipped.  Skipped: FUN_00b4e420, FUN_00b4e9c0, FUN_00b4ee70,
// FUN_00b4fb30, FUN_00b50300, FUN_00b50550, FUN_00b508d0, FUN_00b51400,
// FUN_00b51780, FUN_00b51aa0, FUN_00b51cf0, FUN_00b52510, FUN_00b525e0,
// FUN_00b529b0, FUN_00b52ac0.

/// `0x00b4f010`: pushes two 16-byte records into a fixed-capacity vector
/// (`begin`, `end`, `cap` semantic): `(a,b,weight)` then `(c,d,-weight)`, and
/// sets the dirty flag.  Returns `false` (growth path in the retail code,
/// which is a non-returning allocation call) when the vector is full at the
/// first push.
pub fn w31a76_push_edge_pair(
    rows: &mut Vec<[u32; 4]>,
    capacity: usize,
    dirty: &mut bool,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    weight: i32,
) -> bool {
    if rows.len() == capacity {
        return false;
    }
    rows.push([a, b, weight as u32, 0]);
    rows.push([c, d, weight.wrapping_neg() as u32, 0]);
    *dirty = true;
    true
}

/// `0x00b529f0`: validates a name string.  A string not starting with `#`
/// is accepted only when `has_table` is false (returns 0 otherwise); a
/// string starting `##` is accepted only when `escaped_ok` says so.
pub fn w31a76_name_ok(name: &[u8], has_table: bool, escaped_ok: impl Fn(&[u8]) -> bool) -> bool {
    if name.first() != Some(&b'#') {
        return !has_table;
    }
    !(name.get(1) == Some(&b'#') && !escaped_ok(name))
}

#[cfg(test)]
mod tests_wave31_agent76 {
    use super::*;

    #[test]
    fn push_pair() {
        let mut v = Vec::new();
        let mut d = false;
        assert!(w31a76_push_edge_pair(&mut v, 4, &mut d, 1, 2, 3, 4, 5));
        assert!(d);
        assert_eq!(v[0][2], 5);
        assert_eq!(v[1][2] as i32, -5);
        let mut full = vec![[0; 4]; 4];
        assert!(!w31a76_push_edge_pair(&mut full, 4, &mut d, 1, 2, 3, 4, 5));
    }

    #[test]
    fn name_ok() {
        assert!(w31a76_name_ok(b"abc", false, |_| true));
        assert!(!w31a76_name_ok(b"abc", true, |_| true));
        assert!(w31a76_name_ok(b"#x", true, |_| false));
        assert!(!w31a76_name_ok(b"##x", true, |_| false));
        assert!(w31a76_name_ok(b"##x", true, |_| true));
    }
}
