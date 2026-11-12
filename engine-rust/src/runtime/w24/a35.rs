#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent35.rs
// ===========================================================================
// Wave 24 agent 35: UTF-8 helpers and small record initialisers.

/// `0x007a7590`: byte length of the UTF-8 sequence announced by lead byte
/// `b` (1 for ASCII and for invalid leads, 2..4 otherwise).
pub fn w24a35_utf8_lead_len(b: u8) -> u32 {
    if b < 0x80 {
        return 1;
    }
    if (b & 0xe0) == 0xc0 && b > 0xc1 {
        return 2;
    }
    if (b & 0xf0) == 0xe0 {
        return 3;
    }
    if (b & 0xf8) == 0xf0 && b < 0xf5 {
        return 4;
    }
    1
}

/// `0x007a7730`: with `limit == 0`, truncates the NUL-terminated string in
/// `buf` at a trailing incomplete UTF-8 sequence and returns 0.  Otherwise
/// returns the number of bytes of `src` (up to its NUL) that are copied,
/// `min(strlen(src), limit - 1)`, copying them into `buf`.
pub fn w24a35_utf8_copy_trunc(buf: &mut [u8], src: &[u8], limit: usize) -> usize {
    let src_len = src.iter().position(|&c| c == 0).unwrap_or(src.len());
    if limit != 0 {
        let n = src_len.min(limit - 1).min(buf.len());
        buf[..n].copy_from_slice(&src[..n]);
        return n;
    }
    let nul = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    if nul == 0 {
        return 0;
    }
    let mut p = nul - 1;
    loop {
        let b = buf[p];
        if b > 0x7f && !(0xc2..=0xf4).contains(&b) {
            if p == 0 {
                return 0;
            }
            p -= 1;
            continue;
        }
        let len = if b >= 0x80 {
            if (b & 0xe0) == 0xc0 && b > 0xc1 {
                2
            } else if (b & 0xf0) == 0xe0 {
                3
            } else if (b & 0xf8) == 0xf0 && b < 0xf5 {
                4
            } else {
                0
            }
        } else {
            1
        };
        if nul < p + len {
            buf[p] = 0;
        }
        return 0;
    }
}

/// `0x007a77f0`: scans `slots[..count]`; a slot is skipped when it equals
/// `empty`, or when the clock `now()` has not fallen below the value read at
/// entry and `value != empty`.  The first other slot is set to `value`.
pub fn w24a35_claim_slot(
    slots: &mut [i32],
    count: usize,
    value: i32,
    empty: i32,
    mut now: impl FnMut() -> u32,
) {
    let start = now();
    if count == 0 {
        return;
    }
    let mut remaining = count;
    let mut i = 0;
    loop {
        remaining -= 1;
        if slots[i] == empty || (start <= now() && value != empty) {
            i += 1;
            if remaining == 0 {
                return;
            }
        } else {
            slots[i] = value;
            return;
        }
    }
}

/// `0x007a8210`: true for the eight special ids 0x0e, 0x55, 0x69, 0xca,
/// 0xcd..=0xd0.
pub fn w24a35_is_special_id(id: u16) -> bool {
    matches!(id, 0x0e | 0x55 | 0x69 | 0xca | 0xcd | 0xce | 0xcf | 0xd0)
}

/// Record built by `0x007a8c30` (seven dwords).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct W24a35Record {
    pub f0: u32,
    pub index: u32,
    pub has_owner: bool,
    pub f3: u32,
    pub f4: u32,
    pub f5: u32,
    pub f6: u32,
}

/// `0x007a8c30`: initialises the record; `table_value` is the dword at
/// `0x011b869c + index * 12`; `owner` is `Some((ptr, ptr_field_0x24))`
/// when both globals 0x01165f54/0x01165f58 are non-null.
pub fn w24a35_init_record(
    f0: u32,
    index: u32,
    table_value: u32,
    owner: Option<(u32, u32)>,
) -> W24a35Record {
    let mut r = W24a35Record {
        f0,
        index,
        f4: table_value,
        ..Default::default()
    };
    if let Some((ptr, field24)) = owner {
        r.has_owner = true;
        r.f5 = field24;
        r.f6 = ptr;
    }
    r
}

#[cfg(test)]
mod tests_wave24_agent35 {
    use super::*;

    #[test]
    fn lead_len() {
        assert_eq!(w24a35_utf8_lead_len(b'a'), 1);
        assert_eq!(w24a35_utf8_lead_len(0xc1), 1);
        assert_eq!(w24a35_utf8_lead_len(0xc2), 2);
        assert_eq!(w24a35_utf8_lead_len(0xe2), 3);
        assert_eq!(w24a35_utf8_lead_len(0xf4), 4);
        assert_eq!(w24a35_utf8_lead_len(0xf5), 1);
    }

    #[test]
    fn trunc_and_copy() {
        let mut b = *b"ab\xe2\x82\0";
        assert_eq!(w24a35_utf8_copy_trunc(&mut b, b"", 0), 0);
        assert_eq!(&b, b"ab\0\x82\0");
        let mut ok = *b"ab\xc3\xa9\0";
        w24a35_utf8_copy_trunc(&mut ok, b"", 0);
        assert_eq!(&ok, b"ab\xc3\xa9\0");
        let mut d = [0u8; 8];
        assert_eq!(w24a35_utf8_copy_trunc(&mut d, b"hello\0", 4), 3);
        assert_eq!(&d[..3], b"hel");
    }

    #[test]
    fn claim() {
        let mut s = [5, 0, 0];
        let mut t = 0;
        w24a35_claim_slot(&mut s, 3, 9, 5, || {
            t += 1;
            if t == 1 { 10 } else { 3 }
        });
        assert_eq!(s, [5, 9, 0]);
        let mut e = [1, 1];
        w24a35_claim_slot(&mut e, 2, 9, 1, || 0);
        assert_eq!(e, [1, 1]);
    }

    #[test]
    fn special_and_record() {
        assert!(w24a35_is_special_id(0xcd));
        assert!(!w24a35_is_special_id(0xcc));
        let r = w24a35_init_record(1, 2, 3, Some((4, 5)));
        assert!(r.has_owner && r.f5 == 5 && r.f6 == 4 && r.f4 == 3);
        assert!(!w24a35_init_record(1, 2, 3, None).has_owner);
    }
}
