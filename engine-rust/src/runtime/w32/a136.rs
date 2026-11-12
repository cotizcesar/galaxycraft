#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent136.rs
// ===========================================================================
// Wave 32 translation batch 136: printf-engine helpers (callees injected).

/// `0x00c9b182`: stores the pair (8, 10) at object offsets 0x28/0x2c, then
/// calls the wide-argument prepare helper with (0x10, 1).  Returns the pair.
pub fn w32a136_set_radix_fields(call_prepare: &mut dyn FnMut(u32, u32)) -> (u32, u32) {
    call_prepare(0x10, 1);
    (8, 10)
}

/// `0x00c9b416`: true when the stream flags (bit 12 of word at +0xc) are clear
/// or the field at +4 is nonzero and the char lookup returns anything but -1;
/// otherwise (bit set and +4 zero) returns true directly.
pub fn w32a136_char_ok(
    flags: u32,
    field4: i32,
    ch: i8,
    lookup: &mut dyn FnMut(i32) -> i32,
) -> bool {
    if (flags >> 12) & 1 == 0 || field4 != 0 {
        lookup(ch as i32) != -1
    } else {
        true
    }
}

/// `0x00c9b59e`: when the flag byte is zero and count > 0, decodes `count`
/// items from `pos` using `decode` (returns (value, consumed)); a consumed
/// value below 1 sets `*state = -1` and stops; else `push(value)`.  Otherwise
/// calls `bulk(pos, count)`.  Always returns 1.
pub fn w32a136_decode_run(
    flag: u8,
    count: i32,
    pos: i32,
    state: &mut i32,
    decode: &mut dyn FnMut(i32) -> (u16, i32),
    push: &mut dyn FnMut(u16),
    bulk: &mut dyn FnMut(i32, i32),
) -> u32 {
    if flag == 0 && count > 0 {
        let mut p = pos;
        for _ in 0..count {
            let (v, n) = decode(p);
            if n < 1 {
                *state = -1;
                return 1;
            }
            push(v);
            p += n;
        }
    } else {
        bulk(pos, count);
    }
    1
}

#[cfg(test)]
mod tests_wave32_agent136 {
    use super::*;

    #[test]
    fn radix() {
        let mut c = vec![];
        assert_eq!(
            w32a136_set_radix_fields(&mut |a, b| c.push((a, b))),
            (8, 10)
        );
        assert_eq!(c, vec![(0x10, 1)]);
    }

    #[test]
    fn char_ok() {
        assert!(w32a136_char_ok(0x1000, 0, 1, &mut |_| -1));
        assert!(!w32a136_char_ok(0, 0, 1, &mut |_| -1));
        assert!(w32a136_char_ok(0, 0, 1, &mut |_| 3));
    }

    #[test]
    fn run() {
        let mut st = 0;
        let mut out = vec![];
        let r = w32a136_decode_run(
            0,
            2,
            0,
            &mut st,
            &mut |p| (p as u16 + 1, 1),
            &mut |v| out.push(v),
            &mut |_, _| {},
        );
        assert_eq!((r, out), (1, vec![1, 2]));
        w32a136_decode_run(
            0,
            2,
            0,
            &mut st,
            &mut |_| (0, 0),
            &mut |_| {},
            &mut |_, _| {},
        );
        assert_eq!(st, -1);
    }
}
