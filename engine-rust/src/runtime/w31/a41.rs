#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent41.rs
// ===========================================================================
// Wave 31 agent 41: small predicates and a bit-vector insert from the
// runtime (STL) region.

/// `0x0080a7c0`: true when `is_ready(obj)` (callee 814160) holds, else true
/// only if the secondary check (callee 606c70) holds and the flag byte at
/// object offset `0x111` is zero.
pub fn w31a41_ready_or_unflagged(
    flag_0x111: u8,
    is_ready: bool,
    secondary: impl FnOnce() -> bool,
) -> bool {
    if is_ready {
        return true;
    }
    secondary() && flag_0x111 == 0
}

/// `0x0080a820`: true when the primary check (callee 814160) holds, else
/// the result of the secondary check (callee 606c70).
pub fn w31a41_ready_or_secondary(is_ready: bool, secondary: impl FnOnce() -> bool) -> bool {
    is_ready || secondary()
}

/// Bit vector as laid out by the 32-bit MSVC `vector<bool>`: 32-bit words
/// plus a bit count.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct W31a41BitVec {
    pub words: Vec<u32>,
    pub len: usize,
}

impl W31a41BitVec {
    fn get(&self, i: usize) -> bool {
        self.words[i >> 5] & (1 << (i & 31)) != 0
    }
    fn set(&mut self, i: usize, v: bool) {
        if v {
            self.words[i >> 5] |= 1 << (i & 31);
        } else {
            self.words[i >> 5] &= !(1 << (i & 31));
        }
    }
}

/// `0x00802070`: `vector<bool>::insert(pos, count, value)`.  Grows the word
/// storage, shifts the tail up by `count` bits (copying high to low), then
/// fills `[pos, pos+count)` with `value`.  Returns the start bit index.
pub fn w31a41_bitvec_insert(v: &mut W31a41BitVec, pos: usize, count: usize, value: bool) -> usize {
    if count != 0 {
        assert!(count <= 0x7fff_ffff - v.len, "vector<bool> too long");
        let new_len = v.len + count;
        v.words.resize(new_len.div_ceil(32).max(v.words.len()), 0);
        let old_len = v.len;
        v.len = new_len;
        let mut i = old_len;
        while i > pos {
            i -= 1;
            let b = v.get(i);
            v.set(i + count, b);
        }
        for i in pos..pos + count {
            v.set(i, value);
        }
    }
    pos
}

#[cfg(test)]
mod tests_wave31_agent41 {
    use super::*;

    #[test]
    fn predicates() {
        assert!(w31a41_ready_or_unflagged(5, true, || false));
        assert!(w31a41_ready_or_unflagged(0, false, || true));
        assert!(!w31a41_ready_or_unflagged(1, false, || true));
        assert!(!w31a41_ready_or_unflagged(0, false, || false));
        assert!(w31a41_ready_or_secondary(false, || true));
        assert!(!w31a41_ready_or_secondary(false, || false));
        assert!(w31a41_ready_or_secondary(true, || false));
    }

    #[test]
    fn insert_bits() {
        let mut v = W31a41BitVec {
            words: vec![0b1011],
            len: 4,
        };
        assert_eq!(w31a41_bitvec_insert(&mut v, 1, 2, false), 1);
        assert_eq!(v.len, 6);
        assert_eq!(v.words[0], 0b101001);
        let mut w = W31a41BitVec::default();
        w31a41_bitvec_insert(&mut w, 0, 40, true);
        assert_eq!(w.words, vec![u32::MAX, 0xff]);
    }
}
