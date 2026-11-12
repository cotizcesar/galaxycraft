#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent46.rs
// ===========================================================================
// Wave 24 agent 46: small object field setters and hash-bucket clearing.

/// `0x007cf450`: stores `value` into the dword field at byte offset `100`
/// (0x64) of the object (`this` in ECX).
pub fn w24a46_set_field_0x64(obj: &mut [u8], value: u32) -> bool {
    match obj.get_mut(0x64..0x68) {
        Some(d) => {
            d.copy_from_slice(&value.to_le_bytes());
            true
        }
        None => false,
    }
}

/// `0x007cf460`: copies the dword `*src` into the field at byte offset
/// `0x60` of the object.
pub fn w24a46_copy_field_0x60(obj: &mut [u8], src: u32) -> bool {
    match obj.get_mut(0x60..0x64) {
        Some(d) => {
            d.copy_from_slice(&src.to_le_bytes());
            true
        }
        None => false,
    }
}

/// Clears one bucket array; returns false if a non-empty bucket was found
/// (the retail code diverges into its node-release path there).
fn w24a46_clear_buckets(buckets: &mut [u32], count: u32) -> bool {
    let mut ok = true;
    for b in buckets.iter_mut().take(count as usize) {
        if *b != 0 {
            ok = false;
            break;
        }
        *b = 0;
    }
    ok
}

/// Hash table triple used by `0x007d0ec0`: (bucket count, buckets, element
/// count) at object offsets 0x10/0x14/0x18, 0x20/0x24/0x28, 0x30/0x34/0x38.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct W24a46Table {
    pub buckets: Vec<u32>,
    pub bucket_count: u32,
    pub size: u32,
}

/// `0x007d0ec0`: after the base reset, walks each of the three tables'
/// bucket arrays zeroing every slot and then zeroes the size.  A non-empty
/// bucket aborts (returns false) before further clearing.
pub fn w24a46_clear_tables(tables: &mut [W24a46Table; 3]) -> bool {
    for t in tables.iter_mut() {
        if !t.buckets.is_empty() && !w24a46_clear_buckets(&mut t.buckets, t.bucket_count) {
            return false;
        }
        t.size = 0;
    }
    true
}

#[cfg(test)]
mod tests_wave24_agent46 {
    use super::*;

    #[test]
    fn setters() {
        let mut o = [0u8; 0x70];
        assert!(w24a46_set_field_0x64(&mut o, 0x01020304));
        assert_eq!(&o[0x64..0x68], &[4, 3, 2, 1]);
        assert!(w24a46_copy_field_0x60(&mut o, 7));
        assert_eq!(o[0x60], 7);
        assert!(!w24a46_set_field_0x64(&mut [0u8; 8], 1));
        assert!(!w24a46_copy_field_0x60(&mut [0u8; 8], 1));
    }

    #[test]
    fn clear() {
        let mut t = [
            W24a46Table {
                buckets: vec![0; 4],
                bucket_count: 4,
                size: 5,
            },
            W24a46Table::default(),
            W24a46Table {
                buckets: vec![0; 2],
                bucket_count: 2,
                size: 9,
            },
        ];
        assert!(w24a46_clear_tables(&mut t));
        assert_eq!(t[0].size, 0);
        assert_eq!(t[2].size, 0);
        t[0].buckets[1] = 3;
        t[0].size = 1;
        assert!(!w24a46_clear_tables(&mut t));
        assert_eq!(t[0].size, 1);
    }
}
