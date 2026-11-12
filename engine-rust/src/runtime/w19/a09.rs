#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave19/agent09.rs
// ===========================================================================
// Wave 19 agent 09: MSVC `num_put` digit generators and small list walkers.

/// Writes `value` in `radix_bits`-wide digits backwards from `end`
/// (do-while: at least one digit); returns the new start index.
fn w19a09_fill_backwards(buf: &mut [u8], end: usize, mut value: u64, shift: u32) -> usize {
    let mask = (1u64 << shift) - 1;
    let mut p = end;
    loop {
        p -= 1;
        buf[p] = (value & mask) as u8 + b'0';
        value >>= shift;
        if value == 0 {
            return p;
        }
    }
}

/// `0x00555950`: 64-bit value (lo at +0x18, hi at +0x1c) as binary digits
/// written backwards from `end` (= +0x20 field); returns the start index.
pub fn w19a09_put_binary64(buf: &mut [u8], end: usize, value: u64) -> usize {
    w19a09_fill_backwards(buf, end, value, 1)
}

/// `0x005559d0`: 32-bit value (+0x14) as octal digits ending at `end`
/// (+0x18 field); returns the start index.
pub fn w19a09_put_octal32(buf: &mut [u8], end: usize, value: u32) -> usize {
    w19a09_fill_backwards(buf, end, u64::from(value), 3)
}

/// `0x00555a40`: 64-bit value as octal digits ending at `end`; returns the
/// start index.
pub fn w19a09_put_octal64(buf: &mut [u8], end: usize, value: u64) -> usize {
    w19a09_fill_backwards(buf, end, value, 3)
}

/// `0x00555ba0`: 32-bit value as hex digits ending at `end`; the digit set
/// is lowercase when the format flag char is `'x'`, else uppercase.
pub fn w19a09_put_hex32(buf: &mut [u8], end: usize, value: u32, spec: u8) -> usize {
    let table: &[u8; 16] = if spec == b'x' {
        b"0123456789abcdef"
    } else {
        b"0123456789ABCDEF"
    };
    let mut v = value;
    let mut p = end;
    loop {
        p -= 1;
        buf[p] = table[(v & 0xf) as usize];
        v >>= 4;
        if v == 0 {
            return p;
        }
    }
}

/// `0x005562d0`: renders `value` as decimal right-aligned in a `width`-byte
/// scratch buffer (two digits per step via the "00..99" pair table) and
/// returns those `width` bytes (the original copies them to the output).
/// Returns `None` when the digits do not fit.
pub fn w19a09_put_decimal_u32(mut value: u32, width: usize) -> Option<Vec<u8>> {
    const PAIRS: &[u8; 200] = b"0001020304050607080910111213141516171819\
2021222324252627282930313233343536373839\
4041424344454647484950515253545556575859\
6061626364656667686970717273747576777879\
8081828384858687888990919293949596979899";
    let mut buf = vec![0u8; width];
    let mut p = width;
    while value > 99 {
        let i = (value % 100) as usize * 2;
        if p < 2 {
            return None;
        }
        p -= 2;
        buf[p] = PAIRS[i];
        buf[p + 1] = PAIRS[i + 1];
        value /= 100;
    }
    if value < 10 {
        if p < 1 {
            return None;
        }
        p -= 1;
        buf[p] = value as u8 + b'0';
    } else {
        if p < 2 {
            return None;
        }
        p -= 2;
        buf[p] = PAIRS[value as usize * 2];
        buf[p + 1] = PAIRS[value as usize * 2 + 1];
    }
    Some(buf)
}

/// `0x00556090`: clears the byte at `*dst`.
pub fn w19a09_clear_byte(dst: &mut u8) {
    *dst = 0;
}

/// `0x00579110`: walks the singly linked list whose head is the pointer at
/// `owner + 0x20` (next link at node `+4`), invoking `visit` on each node.
pub fn w19a09_visit_list(head: u32, mut next: impl FnMut(u32) -> u32, mut visit: impl FnMut(u32)) {
    let mut node = head;
    while node != 0 {
        visit(node);
        node = next(node);
    }
}

/// `0x005790c0`: for each bucket `0..count` (count is the u16 at +0xe6 of
/// the decoded global), walks the bucket list (next at `+4`) and, per
/// bucket node, its child list (head at `+0x20`, next at `+4`), calling
/// `visit` on every child.
pub fn w19a09_visit_all(
    count: u16,
    bucket_head: impl Fn(usize) -> u32,
    next: impl Fn(u32) -> u32,
    child_head: impl Fn(u32) -> u32,
    mut visit: impl FnMut(u32),
) {
    for i in 0..usize::from(count) {
        let mut b = bucket_head(i);
        while b != 0 {
            let mut c = child_head(b);
            while c != 0 {
                visit(c);
                c = next(c);
            }
            b = next(b);
        }
    }
}

#[cfg(test)]
mod tests_wave19_agent09 {
    use super::*;

    #[test]
    fn digits_bases() {
        let mut b = [b'?'; 70];
        let s = w19a09_put_binary64(&mut b, 70, 0);
        assert_eq!(&b[s..70], b"0");
        let s = w19a09_put_binary64(&mut b, 70, 0x1_0000_0005);
        assert_eq!(&b[s..70], b"100000000000000000000000000000101");
        let s = w19a09_put_binary64(&mut b, 70, u64::MAX);
        assert_eq!(70 - s, 64);
        let s = w19a09_put_octal32(&mut b, 70, 0o755);
        assert_eq!(&b[s..70], b"755");
        let s = w19a09_put_octal64(&mut b, 70, 1 << 33);
        assert_eq!(&b[s..70], b"100000000000");
    }

    #[test]
    fn hex_case() {
        let mut b = [0u8; 16];
        let s = w19a09_put_hex32(&mut b, 16, 0xBEEF, b'x');
        assert_eq!(&b[s..16], b"beef");
        let s = w19a09_put_hex32(&mut b, 16, 0xBEEF, b'X');
        assert_eq!(&b[s..16], b"BEEF");
        let s = w19a09_put_hex32(&mut b, 16, 0, b'x');
        assert_eq!(&b[s..16], b"0");
    }

    #[test]
    fn decimal() {
        assert_eq!(w19a09_put_decimal_u32(0, 1).unwrap(), b"0");
        assert_eq!(w19a09_put_decimal_u32(7, 3).unwrap(), b"\0\x007");
        assert_eq!(w19a09_put_decimal_u32(42, 2).unwrap(), b"42");
        assert_eq!(w19a09_put_decimal_u32(100, 3).unwrap(), b"100");
        assert_eq!(w19a09_put_decimal_u32(u32::MAX, 10).unwrap(), b"4294967295");
        assert!(w19a09_put_decimal_u32(1000, 3).is_none());
        let mut c = 5u8;
        w19a09_clear_byte(&mut c);
        assert_eq!(c, 0);
    }

    #[test]
    fn lists() {
        // nodes 1->2->3, child lists: 1:{10->11}, 2:{}, 3:{12}
        let next = |n: u32| match n {
            1 => 2,
            2 => 3,
            10 => 11,
            _ => 0,
        };
        let child = |n: u32| match n {
            1 => 10,
            3 => 12,
            _ => 0,
        };
        let mut seen = vec![];
        w19a09_visit_list(10, next, |n| seen.push(n));
        assert_eq!(seen, [10, 11]);
        let mut seen = vec![];
        w19a09_visit_all(2, |i| [1, 3][i], next, child, |n| seen.push(n));
        assert_eq!(seen, [10, 11, 12, 12]);
    }
}
