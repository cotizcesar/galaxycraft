#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent86.rs
// ===========================================================================
// Wave 32 agent 86 translations.

/// `0x00a76070`: reads an EBML-style variable-length integer at the 64-bit
/// offset `pos`.  `read(pos, buf)` models the stream callback (negative =
/// error, positive = end of data, zero = one byte stored).  The length is
/// the position of the first set bit among the top four bits of the first
/// byte; the marker bit is kept in the value.  Returns `Ok((value, length))`
/// or `Err(-2)` for malformed data / `Err(-3)` for end of data.
pub fn w32a86_read_vint(
    pos: i64,
    read: &mut dyn FnMut(i64, &mut u8) -> i32,
) -> Result<(u64, i32), i32> {
    if pos < 0 {
        return Err(-2);
    }
    let mut b = 0u8;
    let r = read(pos, &mut b);
    if r < 0 {
        return Err(-2);
    }
    if r > 0 {
        return Err(-3);
    }
    if b == 0 {
        return Err(-2);
    }
    let first = b as u64;
    for i in 0..4 {
        let len = i + 1;
        if (first & (0x80u64 >> i)) != 0 {
            let mut value = first;
            for j in 1..len {
                let r = read(pos + j as i64, &mut b);
                if r < 0 {
                    return Err(-2);
                }
                if r > 0 {
                    return Err(-3);
                }
                value = (value << 8) | b as u64;
            }
            return Ok((value, len));
        }
    }
    Err(-2)
}

#[cfg(test)]
mod tests_wave32_agent86 {
    use super::*;

    fn run(data: &[u8], pos: i64) -> Result<(u64, i32), i32> {
        w32a86_read_vint(pos, &mut |p, b| {
            if p as usize >= data.len() {
                return 1;
            }
            *b = data[p as usize];
            0
        })
    }

    #[test]
    fn vint() {
        assert_eq!(run(&[0x81], 0), Ok((0x81, 1)));
        assert_eq!(run(&[0x40, 0x02], 0), Ok((0x4002, 2)));
        assert_eq!(run(&[0x10, 1, 2, 3], 0), Ok((0x10010203, 4)));
        assert_eq!(run(&[0x08], 0), Err(-2));
        assert_eq!(run(&[0x00], 0), Err(-2));
        assert_eq!(run(&[0x40], 0), Err(-3));
        assert_eq!(run(&[0x40], -1), Err(-2));
    }
}
