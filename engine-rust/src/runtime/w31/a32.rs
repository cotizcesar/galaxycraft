#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent32.rs
// ===========================================================================
// Wave 31 agent 32: IDEA inverse, decimal formatters and a range-table test.

/// Two ASCII digits of `v % 100` (the retail 200-byte pair table).
fn w31a32_pair(v: u32) -> [u8; 2] {
    [b'0' + (v / 10 % 10) as u8, b'0' + (v % 10) as u8]
}

/// `0x00700610`: multiplicative inverse modulo 0x10001 (IDEA); values
/// below 2 map to themselves.
pub fn w31a32_mul_inverse(x: u32) -> u32 {
    if x < 2 {
        return x;
    }
    let mut x = x;
    let mut t1 = 0x10001 / x;
    let mut y = 0x10001 % x;
    if y == 1 {
        return 1u32.wrapping_sub(t1) & 0xffff;
    }
    let mut t0 = 1u32;
    loop {
        let q = x / y;
        x %= y;
        t0 = t0.wrapping_add(q.wrapping_mul(t1));
        if x == 1 {
            return t0;
        }
        let q2 = y / x;
        y %= x;
        t1 = t1.wrapping_add(q2.wrapping_mul(t0));
        if y == 1 {
            return 1u32.wrapping_sub(t1) & 0xffff;
        }
    }
}

/// `0x00706ce0`: writes a signed integer (|v| < 1000 in practice) in
/// decimal into `out`, returning the number of bytes written.
pub fn w31a32_write_i32_small(v: i32, out: &mut Vec<u8>) -> usize {
    let start = out.len();
    let mut v = v;
    if v < 0 {
        out.push(b'-');
        v = v.wrapping_neg();
    }
    if v > 99 {
        out.push((v / 100) as u8 + b'0');
        out.extend_from_slice(&w31a32_pair((v % 100) as u32));
    } else if v > 9 {
        out.extend_from_slice(&w31a32_pair(v as u32));
    } else {
        out.push(v as u8 + b'0');
    }
    out.len() - start
}

/// `0x00707570`: writes an unsigned 32-bit integer in decimal without
/// padding; returns bytes written.
pub fn w31a32_write_u32(v: u32, out: &mut Vec<u8>) -> usize {
    let start = out.len();
    out.extend_from_slice(v.to_string().as_bytes());
    out.len() - start
}

/// `0x00707770`: writes an unsigned 64-bit integer (`lo`, `hi` halves) in
/// decimal without padding; returns bytes written.
pub fn w31a32_write_u64(lo: u32, hi: u32, out: &mut Vec<u8>) -> usize {
    let v = ((hi as u64) << 32) | lo as u64;
    let start = out.len();
    out.extend_from_slice(v.to_string().as_bytes());
    out.len() - start
}

/// `0x00711a90`: true when `value` lies in any `[lo, hi]` pair of a table
/// whose count byte is at offset 8 and whose 8-byte entries start at
/// offset 9 (lo at +1 of the entry-relative base, hi at +2; the scan stops
/// when the next entry's marker byte at +6 is zero).
pub fn w31a32_in_range_table(table: &[u8], value: i32) -> bool {
    if table.get(8).copied().unwrap_or(0) == 0 {
        return false;
    }
    let mut p = 10usize;
    loop {
        let (Some(&lo), Some(&hi)) = (table.get(p - 1), table.get(p)) else {
            return false;
        };
        if lo as i32 <= value && value <= hi as i32 {
            return true;
        }
        let marker = table.get(p + 6).copied().unwrap_or(0);
        p += 8;
        if marker == 0 {
            return false;
        }
    }
}

#[cfg(test)]
mod tests_wave31_agent32 {
    use super::*;

    #[test]
    fn inverse() {
        assert_eq!(w31a32_mul_inverse(0), 0);
        assert_eq!(w31a32_mul_inverse(1), 1);
        for x in 2u32..2000 {
            let inv = w31a32_mul_inverse(x);
            assert_eq!((x as u64 * inv as u64) % 0x10001, 1, "x={x}");
        }
    }

    #[test]
    fn small_int() {
        let mut v = Vec::new();
        assert_eq!(w31a32_write_i32_small(-5, &mut v), 2);
        w31a32_write_i32_small(42, &mut v);
        w31a32_write_i32_small(907, &mut v);
        assert_eq!(v, b"-542907");
    }

    #[test]
    fn big_ints() {
        let mut v = Vec::new();
        w31a32_write_u32(4_000_000_000, &mut v);
        assert_eq!(v, b"4000000000");
        let mut v = Vec::new();
        assert_eq!(w31a32_write_u64(0xffff_ffff, 0xffff_ffff, &mut v), 20);
        assert_eq!(v, b"18446744073709551615");
    }

    #[test]
    fn range_table() {
        let mut t = vec![0u8; 8];
        t.push(2); // count at 8
        t.extend_from_slice(&[0; 40]);
        // entry 0: lo at 9, hi at 10, marker at 16
        t[9] = 3;
        t[10] = 5;
        t[16] = 1;
        t[17] = 10;
        t[18] = 12;
        assert!(w31a32_in_range_table(&t, 4));
        assert!(w31a32_in_range_table(&t, 11));
        assert!(!w31a32_in_range_table(&t, 8));
        assert!(!w31a32_in_range_table(&[0; 9], 4));
    }
}
