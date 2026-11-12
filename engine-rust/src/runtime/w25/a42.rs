#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent42.rs
// ===========================================================================
// Wave 25 agent 42: small registry/vector helpers.

/// `0x0087bcb0`: copies the two words at offsets 4 and 8 of the record
/// referenced by the object's pointer at offset 4 into `out`.
pub fn w25a42_copy_pair(record: &[u32; 3]) -> [u32; 2] {
    [record[1], record[2]]
}

/// `0x00880310`: scans the global object registry for the first entry
/// whose field at offset 4 equals `wanted`; returns its index.
pub fn w25a42_find_by_field4(
    registry: &[u32],
    wanted: u32,
    field4: impl Fn(u32) -> u32,
) -> Option<usize> {
    registry.iter().position(|&o| field4(o) == wanted)
}

/// `0x00880280`: scans the registry; for each object walks its u32 member
/// list and resolves each member through `resolve`; returns the first
/// object (index) holding a member that resolves to `wanted`.
pub fn w25a42_find_owner(
    registry: &[u32],
    members: impl Fn(u32) -> Vec<u32>,
    resolve: impl Fn(u32) -> u32,
    wanted: u32,
) -> Option<usize> {
    registry.iter().position(|&o| {
        members(o)
            .into_iter()
            .any(|m| resolve(m) as i32 == wanted as i32)
    })
}

/// `0x0087eb60`: vector push_back of a u32; when the value is an element
/// of the vector itself (aliasing) the element is copied before growth.
pub fn w25a42_push_back(v: &mut Vec<u32>, value: u32) {
    v.push(value);
}

#[cfg(test)]
mod tests_wave25_agent42 {
    use super::*;

    #[test]
    fn pair() {
        assert_eq!(w25a42_copy_pair(&[9, 1, 2]), [1, 2]);
    }

    #[test]
    fn find_field() {
        let reg = [10, 20, 30];
        assert_eq!(w25a42_find_by_field4(&reg, 21, |o| o + 1), Some(1));
        assert_eq!(w25a42_find_by_field4(&reg, 5, |o| o), None);
        assert_eq!(w25a42_find_by_field4(&[], 5, |o| o), None);
    }

    #[test]
    fn owner() {
        let reg = [1, 2];
        let m = |o: u32| vec![o * 10, o * 10 + 1];
        assert_eq!(w25a42_find_owner(&reg, m, |x| x, 21), Some(1));
        assert_eq!(w25a42_find_owner(&reg, m, |x| x, 99), None);
    }

    #[test]
    fn push() {
        let mut v = vec![1];
        w25a42_push_back(&mut v, 7);
        assert_eq!(v, vec![1, 7]);
    }
}
