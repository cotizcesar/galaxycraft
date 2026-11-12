#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent81.rs
// ===========================================================================
// Wave 32 agent 81: Vorbis header helpers (comment unpack, quant values).

/// Bit source consumed by the comment unpacker (models the reader object).
pub trait W32a81Reader {
    /// Reads `bits` bits as a signed value (the bit-read helper).
    fn read(&mut self, bits: u32) -> i32;
    /// Total bit budget field of the reader.
    fn total(&self) -> i32;
    /// Current position helper value.
    fn pos(&self) -> i32;
}

/// Unpacked comment block: vendor bytes plus comment strings.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct W32a81Comment {
    pub vendor: Vec<u8>,
    pub comments: Vec<Vec<u8>>,
}

/// `0x009f6e90`: unpacks a Vorbis comment header; `None` on any bounds
/// failure or a missing framing bit (the original frees and returns -133).
pub fn w32a81_comment_unpack<R: W32a81Reader>(r: &mut R) -> Option<W32a81Comment> {
    let vlen = r.read(0x20);
    if vlen < 0 || vlen > r.total() - 8 {
        return None;
    }
    let mut out = W32a81Comment::default();
    for _ in 0..vlen {
        out.vendor.push(r.read(8) as u8);
    }
    let n = r.read(0x20);
    if n < 0 || n > (r.total() - r.pos()) >> 2 {
        return None;
    }
    for _ in 0..n {
        let len = r.read(0x20);
        if len < 0 || r.total() - r.pos() < len {
            return None;
        }
        let mut s = Vec::with_capacity(len as usize);
        for _ in 0..len {
            s.push(r.read(8) as u8);
        }
        out.comments.push(s);
    }
    if r.read(1) == 1 { Some(out) } else { None }
}

/// `0x009f8640`: largest integer `v` with `v^dim <= entries < (v+1)^dim`
/// (Vorbis lookup-table quantization values).  `None` for `dim <= 0`.
pub fn w32a81_quantvals(dim: i32, entries: i32) -> Option<i32> {
    if dim <= 0 || entries < 1 {
        return None;
    }
    let mut v = (entries as f64).powf(1.0 / dim as f64) as i32;
    loop {
        let (mut lo, mut hi) = (1i64, 1i64);
        for _ in 0..dim {
            lo = lo.wrapping_mul(v as i64);
            hi = hi.wrapping_mul(v as i64 + 1);
        }
        let e = entries as i64;
        if lo <= e && e < hi {
            return Some(v);
        }
        v += if lo <= e { 1 } else { -1 };
        if v < 0 {
            return None;
        }
    }
}

#[cfg(test)]
mod tests_wave32_agent81 {
    use super::*;

    struct R(Vec<u8>, usize);
    impl W32a81Reader for R {
        fn read(&mut self, bits: u32) -> i32 {
            let mut v: u64 = 0;
            for i in 0..bits as usize / 8 + (bits % 8 != 0) as usize {
                v |= (*self.0.get(self.1 + i).unwrap_or(&0) as u64) << (8 * i);
            }
            self.1 += bits as usize / 8;
            if bits == 1 {
                self.1 += 1;
            }
            (v & ((1u64 << bits) - 1)) as i32
        }
        fn total(&self) -> i32 {
            self.0.len() as i32
        }
        fn pos(&self) -> i32 {
            self.1 as i32
        }
    }

    #[test]
    fn quantvals() {
        assert_eq!(w32a81_quantvals(2, 16), Some(4));
        assert_eq!(w32a81_quantvals(3, 26), Some(2));
        assert_eq!(w32a81_quantvals(0, 5), None);
    }

    #[test]
    fn comment() {
        let mut b = vec![2, 0, 0, 0, b'a', b'b', 1, 0, 0, 0, 1, 0, 0, 0, b'x', 1];
        let c = w32a81_comment_unpack(&mut R(b.clone(), 0)).unwrap();
        assert_eq!(c.vendor, b"ab");
        assert_eq!(c.comments, vec![b"x".to_vec()]);
        b.pop();
        b.push(0);
        assert!(w32a81_comment_unpack(&mut R(b, 0)).is_none());
    }
}
