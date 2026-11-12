#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent141.rs
// ===========================================================================
// Wave 31 agent 141: CRT locale / float-digit helpers.

/// `0x00cb75a7`: validates a locale id.  `queried` is the result of the
/// locale-info query (`param_1 & 0x3ff | 0x400`, type `0x20000001`, 2
/// bytes); `None` means the query failed.  When the ids differ and
/// `check_len` is set, the id is rejected if the sub-language length
/// equals the length of the cached name.
pub fn w31a141_locale_id_valid(
    id: u32,
    queried: Option<u32>,
    check_len: bool,
    name_len: usize,
    sublang_len: usize,
) -> bool {
    match queried {
        None => false,
        Some(q) => !(id != q && check_len && sublang_len == name_len),
    }
}

/// `0x00cb7e03`: clears the two-word output and reports errno `0x2a`;
/// returns `(-1, errno)`.
pub fn w31a141_clear_pair_einval(out: &mut [u32; 2]) -> (i32, u32) {
    out[0] = 0;
    out[1] = 0;
    (-1, 0x2a)
}

/// `0x00cb6fb1` / `0x00cb7097` / `0x00cb6ffc` shared test: a locale name
/// is a "three letter" code when its wide-string length is exactly 3.
pub fn w31a141_is_three_letter(name: &[u16]) -> bool {
    name.iter().position(|&c| c == 0).unwrap_or(name.len()) == 3
}

/// `0x00cb6fb1`: sets the country-flag (`+0x64`) from the name length and
/// clears the four result bytes unless bit 2 of the first byte is set.
pub fn w31a141_country_flag_update(country: &[u16], flag: &mut u32, out: &mut [u8; 4]) {
    *flag = w31a141_is_three_letter(country) as u32;
    if out[0] & 4 == 0 {
        *out = [0; 4];
    }
}

/// `0x00cb7097`: sets the language-flag (`+0x60`) and the code (`+0x5c`,
/// `2` for a three letter name, else `other`), then clears the result
/// bytes unless bit 2 of the first byte is set.
pub fn w31a141_language_flag_update(
    language: &[u16],
    other: u32,
    flag: &mut u32,
    code: &mut u32,
    out: &mut [u8; 4],
) {
    let three = w31a141_is_three_letter(language);
    *flag = three as u32;
    *code = if three { 2 } else { other };
    if out[0] & 4 == 0 {
        *out = [0; 4];
    }
}

/// `0x00cb6ffc`: updates both flags and the language code, then zeroes the
/// state word unless bit 8 and bit 9 are set and the low three bits are
/// non-zero.  Also clears `slot` (`param_1[1]`).
pub fn w31a141_locale_flags_update(
    language: &[u16],
    country: &[u16],
    other: u32,
    lang_flag: &mut u32,
    country_flag: &mut u32,
    code: &mut u32,
    state: &mut [u32; 2],
) {
    *lang_flag = w31a141_is_three_letter(language) as u32;
    *country_flag = w31a141_is_three_letter(country) as u32;
    state[1] = 0;
    *code = if *lang_flag != 0 { 2 } else { other };
    let s = state[0];
    if (s >> 8) & 1 == 0 || (s & 7) == 0 || (s >> 9) & 1 == 0 {
        state[0] = 0;
    }
}

/// `0x00cb7e2d`: rounds the digit string `digits` (NUL terminated or
/// ended by the slice) to `ndigits` places into `buf`, with a leading
/// `'0'` slot.  Returns the errno (`0` ok, `0x16` bad args, `0x22` buffer
/// too small).  `decpt` is incremented when rounding carries into the
/// leading slot; otherwise the buffer is shifted left by one.
pub fn w31a141_fptostr(buf: &mut [u8], ndigits: i32, digits: Option<(&[u8], &mut i32)>) -> u32 {
    if buf.is_empty() {
        return 0x16;
    }
    buf[0] = 0;
    let n = ndigits.max(0) as usize;
    if buf.len() <= n + 1 {
        return 0x22;
    }
    let Some((src, decpt)) = digits else {
        return 0x16;
    };
    let at = |i: usize| src.get(i).copied().unwrap_or(0);
    buf[0] = b'0';
    let mut si = 0usize;
    let mut pos = 1usize;
    let mut rem = ndigits;
    while rem > 0 {
        let c = at(si);
        buf[pos] = if c == 0 {
            b'0'
        } else {
            si += 1;
            c
        };
        pos += 1;
        rem -= 1;
    }
    buf[pos] = 0;
    if rem > -1 && at(si) > b'4' {
        loop {
            pos -= 1;
            if buf[pos] != b'9' {
                break;
            }
            buf[pos] = b'0';
        }
        buf[pos] += 1;
    }
    if buf[0] == b'1' {
        *decpt += 1;
    } else {
        let len = buf[1..]
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(buf.len() - 1);
        buf.copy_within(1..=1 + len, 0);
    }
    0
}

#[cfg(test)]
mod tests_wave31_agent141 {
    use super::*;

    fn w(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    #[test]
    fn locale_valid() {
        assert!(!w31a141_locale_id_valid(1, None, true, 2, 3));
        assert!(w31a141_locale_id_valid(1, Some(1), true, 3, 3));
        assert!(!w31a141_locale_id_valid(1, Some(2), true, 3, 3));
        assert!(w31a141_locale_id_valid(1, Some(2), false, 3, 3));
    }

    #[test]
    fn einval() {
        let mut o = [5, 6];
        assert_eq!(w31a141_clear_pair_einval(&mut o), (-1, 0x2a));
        assert_eq!(o, [0, 0]);
    }

    #[test]
    fn flags() {
        assert!(w31a141_is_three_letter(&w("ENU")));
        assert!(!w31a141_is_three_letter(&w("EN")));
        let mut f = 9;
        let mut o = [0u8, 1, 2, 3];
        w31a141_country_flag_update(&w("USA"), &mut f, &mut o);
        assert_eq!((f, o), (1, [0; 4]));
        let mut o = [4u8, 1, 2, 3];
        w31a141_country_flag_update(&w("US"), &mut f, &mut o);
        assert_eq!((f, o), (0, [4, 1, 2, 3]));
        let (mut lf, mut c) = (0, 0);
        w31a141_language_flag_update(&w("ENU"), 7, &mut lf, &mut c, &mut o);
        assert_eq!((lf, c), (1, 2));
        w31a141_language_flag_update(&w("en"), 7, &mut lf, &mut c, &mut o);
        assert_eq!((lf, c), (0, 7));
        let (mut cf, mut st) = (0, [0x307u32, 5]);
        w31a141_locale_flags_update(&w("ENU"), &w("US"), 7, &mut lf, &mut cf, &mut c, &mut st);
        assert_eq!((lf, cf, c, st), (1, 0, 2, [0x307, 0]));
        st[0] = 0x107;
        w31a141_locale_flags_update(&w("ENU"), &w("US"), 7, &mut lf, &mut cf, &mut c, &mut st);
        assert_eq!(st[0], 0);
    }

    #[test]
    fn fptostr() {
        let mut b = [0u8; 8];
        let mut dp = 1;
        assert_eq!(w31a141_fptostr(&mut b, 2, Some((b"125\0", &mut dp))), 0);
        assert_eq!(&b[..3], b"13\0");
        let mut b = [0u8; 8];
        assert_eq!(w31a141_fptostr(&mut b, 2, Some((b"996\0", &mut dp))), 0);
        assert_eq!(&b[..4], b"100\0");
        assert_eq!(dp, 2);
        let mut b = [0u8; 2];
        assert_eq!(w31a141_fptostr(&mut b, 2, Some((b"1", &mut dp))), 0x22);
        assert_eq!(w31a141_fptostr(&mut [0u8; 4], 1, None), 0x16);
    }
}
