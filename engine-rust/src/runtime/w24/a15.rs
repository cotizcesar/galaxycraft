#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent15.rs
// ===========================================================================
// Wave 24 translation batch 15 (runtime helpers).

/// `0x00745ff0`: stores the two dwords of `src` into the pair at 011a30a8 and
/// reports whether the follow-up call (075a690) must run (flag 011a4580 == 0).
pub fn w24a15_store_pair(dst: &mut [u32; 2], src: [u32; 2], flag_011a4580: u32) -> bool {
    dst[0] = src[0];
    dst[1] = src[1];
    flag_011a4580 == 0
}

/// `0x00746ee0`: setter for the dword at 011a41bc.
pub fn w24a15_set_base_time(slot: &mut u32, value: u32) {
    *slot = value;
}

/// `0x00747120`: elapsed milliseconds / 1000 plus the base.  When the pause
/// flag (011a41b8) is set the original calls a non-returning helper, so
/// `None` is returned.
pub fn w24a15_elapsed_seconds(paused: u32, now: u32, start: u32, base: u32) -> Option<u32> {
    if paused != 0 {
        return None;
    }
    Some(now.wrapping_sub(start) / 1000u32).map(|v| v.wrapping_add(base))
}

/// `0x00747190`: returns true when the pause flag is clear; otherwise the
/// original calls a non-returning helper.
pub fn w24a15_check_not_paused(paused: u32) -> bool {
    paused == 0
}

/// Ring state used by `0x00746b20` (bytes 011a30b0..b4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W24a15Ring {
    pub b0: u8,
    pub b1: u8,
    pub head: u8,
    pub cursor: u8,
    pub count: u8,
}

/// One 0x10c-byte record header written by `0x00746b20`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W24a15Record {
    pub word: u16,
    pub id: u8,
    pub b0: u8,
    pub b1: u8,
}

/// `0x00746b20`: advances the 16-slot head (wraps at 0x10), advances the
/// cursor (wraps at `count`), and fills the record at the new head with the
/// id from `ids[cursor]`, `lookup(id)` (007468c0) and the two state bytes.
/// Returns the head index.  The trailing 0057e2f0 call is left to the caller.
pub fn w24a15_ring_advance(
    ring: &mut W24a15Ring,
    ids: &[u8],
    records: &mut [W24a15Record; 16],
    lookup: impl Fn(u8) -> u16,
) -> usize {
    let n = ring.head.wrapping_add(1);
    ring.head = if n < 0x10 { n } else { 0 };
    ring.cursor = ring.cursor.wrapping_add(1);
    if ring.count <= ring.cursor {
        ring.cursor = 0;
    }
    let id = ids[ring.cursor as usize];
    let rec = &mut records[ring.head as usize];
    rec.b0 = ring.b0;
    rec.b1 = ring.b1;
    rec.word = lookup(id);
    rec.id = id;
    ring.head as usize
}

/// Result of `0x007482d0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W24a15Fields {
    pub hi: u32,
    pub lo: u32,
    pub value: u32,
}

/// `0x007482d0`: after `decode` (00748100) succeeds, reads a tag with
/// `read(&mut v, pos, bits)` (007486f0, returns the new position); when it
/// equals 0xfeedda7a, reads a 4-bit-prefixed byte (hi = >>4 & 0xf, lo = & 0xf)
/// and a final value.
pub fn w24a15_decode_fields(
    decode: impl FnOnce(&mut u32) -> bool,
    mut read: impl FnMut(&mut u32, u32, u32) -> u32,
) -> Option<W24a15Fields> {
    let mut v = 0u32;
    if !decode(&mut v) {
        return None;
    }
    let p0 = v.wrapping_add(1);
    let pos = read(&mut v, p0, 0);
    if v != 0xfeed_da7a {
        return None;
    }
    v = 0;
    let pos = read(&mut v, pos, 4);
    let b = v;
    v = 0;
    read(&mut v, pos, 0);
    Some(W24a15Fields {
        hi: (b >> 4) & 0xf,
        lo: b & 0xf,
        value: v,
    })
}

#[cfg(test)]
mod tests_wave24_agent15 {
    use super::*;

    #[test]
    fn pair_and_time() {
        let mut d = [0; 2];
        assert!(w24a15_store_pair(&mut d, [1, 2], 0));
        assert_eq!(d, [1, 2]);
        assert!(!w24a15_store_pair(&mut d, [3, 4], 1));
        let mut s = 0;
        w24a15_set_base_time(&mut s, 7);
        assert_eq!(s, 7);
        assert_eq!(w24a15_elapsed_seconds(0, 5000, 1000, 7), Some(11));
        assert_eq!(w24a15_elapsed_seconds(1, 5000, 1000, 7), None);
        assert!(w24a15_check_not_paused(0));
        assert!(!w24a15_check_not_paused(1));
    }

    #[test]
    fn ring() {
        let mut r = W24a15Ring {
            b0: 5,
            b1: 6,
            head: 15,
            cursor: 2,
            count: 3,
        };
        let mut recs = [W24a15Record::default(); 16];
        let i = w24a15_ring_advance(&mut r, &[9, 8, 7], &mut recs, |id| id as u16 * 2);
        assert_eq!(i, 0);
        assert_eq!(r.cursor, 0);
        assert_eq!(
            recs[0],
            W24a15Record {
                word: 18,
                id: 9,
                b0: 5,
                b1: 6
            }
        );
    }

    #[test]
    fn fields() {
        let mut step = 0;
        let r = w24a15_decode_fields(
            |_| true,
            |v, pos, _| {
                step += 1;
                *v = match step {
                    1 => 0xfeed_da7a,
                    2 => 0xab,
                    _ => 99,
                };
                pos
            },
        );
        assert_eq!(
            r,
            Some(W24a15Fields {
                hi: 0xa,
                lo: 0xb,
                value: 99
            })
        );
        assert_eq!(w24a15_decode_fields(|_| false, |_, p, _| p), None);
    }
}
