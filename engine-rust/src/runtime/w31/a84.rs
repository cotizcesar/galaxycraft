#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent84.rs
// ===========================================================================
/// `0x00b6faf0`: removes every 16-byte record whose first dword equals
/// `key` from the record vector, compacting survivors in order (stride
/// `0x10`).  Returns the new record count.
pub fn w31a84_remove_records_by_key(records: &mut Vec<[u32; 4]>, key: u32) -> usize {
    records.retain(|r| r[0] != key);
    records.len()
}

#[cfg(test)]
mod tests_wave31_agent84 {
    use super::*;

    #[test]
    fn removes_matching_keys_and_keeps_order() {
        let mut v = vec![
            [1, 10, 11, 12],
            [2, 20, 21, 22],
            [1, 30, 31, 32],
            [3, 40, 41, 42],
        ];
        assert_eq!(w31a84_remove_records_by_key(&mut v, 1), 2);
        assert_eq!(v, vec![[2, 20, 21, 22], [3, 40, 41, 42]]);
        assert_eq!(w31a84_remove_records_by_key(&mut v, 9), 2);
        let mut e: Vec<[u32; 4]> = vec![];
        assert_eq!(w31a84_remove_records_by_key(&mut e, 0), 0);
    }
}
