#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent68.rs
// ===========================================================================
// Wave 31 agent 68: XML-printer and effect-record helpers.

/// Shared shape of `0x00a7c3d0`, `0x00a7c410` and `0x00a7c930`: when the literal
/// match returned 0 the printer reports `code` (10, 11 and 12 respectively).
/// Returns the match result and the error code to report, if any.
pub fn w31a68_match_or_report(matched: i32, code: u32) -> (i32, Option<u32>) {
    (matched, if matched == 0 { Some(code) } else { None })
}

/// `0x00a7c3d0`: literal match, reports error 10 on failure.
pub fn w31a68_match_report_10(matched: i32) -> (i32, Option<u32>) {
    w31a68_match_or_report(matched, 10)
}

/// `0x00a7c410`: literal match, reports error 11 on failure.
pub fn w31a68_match_report_11(matched: i32) -> (i32, Option<u32>) {
    w31a68_match_or_report(matched, 11)
}

/// `0x00a7c930`: literal match, reports error 12 on failure.
pub fn w31a68_match_report_12(matched: i32) -> (i32, Option<u32>) {
    w31a68_match_or_report(matched, 12)
}

/// `0x00a7c450` (prefix step): a leading `/` marks a closing tag; the
/// state field at `+0x34` becomes 2 and the slash is skipped.
pub fn w31a68_strip_close_slash(name: &str, state: &mut u32) -> String {
    match name.strip_prefix('/') {
        Some(rest) => {
            *state = 2;
            rest.to_string()
        }
        None => name.to_string(),
    }
}

/// `0x00a7c890` (length selection): literal length for the declaration
/// match: `2 + (flag != 0)`, or `6 + (flag != 0)` when the mode at `+0x3c` is 1.
pub fn w31a68_decl_match_len(flag: u8, mode: u32) -> u8 {
    let base = if mode == 1 { 6 } else { 2 };
    base + u8::from(flag != 0)
}

/// `0x00a7cae0`: appends `count` copies of four spaces to `out`.
pub fn w31a68_write_indent(out: &mut Vec<u8>, count: i32) {
    for _ in 0..count.max(0) {
        out.extend_from_slice(b"    ");
    }
}

/// `0x00a7d140`: appends one character to the output.
pub fn w31a68_put_char(out: &mut Vec<u8>, c: u8) {
    out.push(c);
}

/// `0x00a7d050`: closes an open tag (writing `>`), then emits either the CDATA
/// wrapper around `body` or the plain-text path.  Returns the new depth.
pub fn w31a68_write_text(
    out: &mut Vec<u8>,
    tag_open: &mut bool,
    depth: i32,
    cdata: bool,
    body: &[u8],
) -> i32 {
    let new_depth = depth - 1;
    if *tag_open {
        *tag_open = false;
        out.push(b'>');
    }
    out.extend_from_slice(if cdata { b"<![CDATA[" } else { b"" });
    out.extend_from_slice(body);
    if cdata {
        out.extend_from_slice(b"]]>");
    }
    new_depth
}

/// `0x00a7da20`: clamps a negative formatted-length result to -1.
pub fn w31a68_clamp_neg(r: i32) -> i32 {
    if r < 0 { -1 } else { r }
}

/// `0x00a892e0`: initialises the first two words of a record to zero.
pub fn w31a68_init_pair(rec: &mut [u32; 2]) {
    rec[0] = 0;
    rec[1] = 0;
}

/// `0x00a8adc0` (record tail): stores three words at `+0x90..+0x98` and
/// optionally sets the u16 at `+0x3e` to 2.
pub fn w31a68_set_effect_tail(rec: &mut [u8], a: u32, b: u32, c: u32, flag: bool) {
    rec[0x90..0x94].copy_from_slice(&a.to_le_bytes());
    rec[0x94..0x98].copy_from_slice(&b.to_le_bytes());
    rec[0x98..0x9c].copy_from_slice(&c.to_le_bytes());
    if flag {
        rec[0x3e..0x40].copy_from_slice(&2u16.to_le_bytes());
    }
}

/// `0x00a8aaa0` (record tail): flag sets u16 at `+0x3e` to 2; kind 2 copies
/// the word at `+8` of the source into `+0xc`.
pub fn w31a68_effect_post(rec: &mut [u8], src: &[u8], kind: i32, flag: bool) {
    if flag {
        rec[0x3e..0x40].copy_from_slice(&2u16.to_le_bytes());
    }
    if kind == 2 {
        rec[0xc..0x10].copy_from_slice(&src[8..12]);
    }
}

#[cfg(test)]
mod tests_wave31_agent68 {
    use super::*;

    #[test]
    fn report() {
        assert_eq!(w31a68_match_report_10(0), (0, Some(10)));
        assert_eq!(w31a68_match_report_11(3), (3, None));
        assert_eq!(w31a68_match_report_12(0).1, Some(12));
    }

    #[test]
    fn slash_and_len() {
        let mut s = 0;
        assert_eq!(w31a68_strip_close_slash("/a", &mut s), "a");
        assert_eq!(s, 2);
        assert_eq!(w31a68_decl_match_len(1, 1), 7);
        assert_eq!(w31a68_decl_match_len(0, 0), 2);
    }

    #[test]
    fn writers() {
        let mut o = vec![];
        w31a68_write_indent(&mut o, 2);
        w31a68_put_char(&mut o, b'x');
        assert_eq!(o, b"        x");
        let mut open = true;
        let mut o = vec![];
        assert_eq!(w31a68_write_text(&mut o, &mut open, 3, true, b"q"), 2);
        assert_eq!(o, b">\x3c![CDATA[q]]>");
        assert_eq!(w31a68_clamp_neg(-5), -1);
        assert_eq!(w31a68_clamp_neg(4), 4);
    }

    #[test]
    fn records() {
        let mut r = [9, 9];
        w31a68_init_pair(&mut r);
        assert_eq!(r, [0, 0]);
        let mut b = vec![0u8; 0xa0];
        w31a68_set_effect_tail(&mut b, 1, 2, 3, true);
        assert_eq!(b[0x90], 1);
        assert_eq!(b[0x3e], 2);
        let src = [0u8, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0];
        w31a68_effect_post(&mut b, &src, 2, false);
        assert_eq!(b[0xc], 7);
    }
}
