#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent93.rs
// ===========================================================================
// Wave 32 agent 93: hex-encoded length-prefixed string and a once-flag helper.

/// `0x00ac05d0`: hex-encodes a length-prefixed (first byte = count) byte
/// string into `out` as lowercase hex and appends a NUL terminator.
/// Returns the number of hex digits written (the callee's return value).
pub fn w32a93_pascal_to_hex(src: &[u8], out: &mut Vec<u8>) -> usize {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let n = src.first().copied().unwrap_or(0) as usize;
    let body = src.get(1..).unwrap_or(&[]);
    let body = &body[..n.min(body.len())];
    let start = out.len();
    for &b in body {
        out.push(HEX[(b >> 4) as usize]);
        out.push(HEX[(b & 15) as usize]);
    }
    let written = out.len() - start;
    out.push(0);
    written
}

/// State block used by `0x00ac2c20` (offsets 0 value, 8 done, 9 skip, 10 lock).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct W32a93Once {
    pub value: u32,
    pub done: bool,
    pub skip: bool,
    pub lock: bool,
}

/// `0x00ac2c20`: if not skipped and not yet done, zeroes the value and marks
/// done under the spin flag; returns true when the exit-handler registration
/// (the non-modelled callee) must be performed by the caller.
pub fn w32a93_once_init(s: &mut W32a93Once) -> bool {
    if s.skip {
        return false;
    }
    s.lock = true;
    let fire = !s.done;
    if fire {
        s.value = 0;
        s.done = true;
    }
    s.lock = false;
    fire
}

#[cfg(test)]
mod tests_wave32_agent93 {
    use super::*;

    #[test]
    fn hex() {
        let mut o = Vec::new();
        assert_eq!(w32a93_pascal_to_hex(&[2, 0xab, 0x01], &mut o), 4);
        assert_eq!(o, b"ab01\0");
    }

    #[test]
    fn once() {
        let mut s = W32a93Once {
            value: 5,
            ..Default::default()
        };
        assert!(w32a93_once_init(&mut s));
        assert_eq!(s.value, 0);
        assert!(!w32a93_once_init(&mut s));
        s.skip = true;
        s.done = false;
        assert!(!w32a93_once_init(&mut s));
    }
}
