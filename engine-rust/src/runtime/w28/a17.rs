#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent17.rs
// ===========================================================================
// Wave 28 agent 17: small runtime helpers (XML-style printer, buffers).

/// Output sink state of the stream printer used by the three writers below.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W28a17Printer {
    /// Bytes written so far (stands in for the FILE / string target).
    pub out: Vec<u8>,
    /// Offset 0x4: a start tag is still open and needs a closing `>`.
    pub tag_open: bool,
    /// Offset 0x3c: text-written flag, cleared before every write.
    pub text_flag: bool,
    /// Offset 0x4d: suppresses the pending newline.
    pub no_newline: bool,
    /// Offset 0x48: depth counter; negative means a newline is due.
    pub depth: i32,
    /// Offset 0x4c: last copied flag byte.
    pub flag_4c: u8,
}

fn w28a17_close_open_tag(p: &mut W28a17Printer, newline: &mut dyn FnMut(&mut W28a17Printer)) {
    if p.tag_open {
        p.tag_open = false;
        p.out.push(b'>');
    }
    if p.depth < 0 && !p.text_flag && !p.no_newline {
        newline(p);
    }
    p.text_flag = false;
}

fn w28a17_cstr(s: &[u8]) -> &[u8] {
    let n = s.iter().position(|&b| b == 0).unwrap_or(s.len());
    &s[..n]
}

/// `0x00a7dab0`: writes an XML comment `<!--text-->`; text is empty when
/// the node's virtual slot 6 (`is_empty_text`) reports non-zero.
pub fn w28a17_write_comment(
    p: &mut W28a17Printer,
    text: &[u8],
    is_empty_text: bool,
    newline: &mut dyn FnMut(&mut W28a17Printer),
) -> u32 {
    let body: &[u8] = if is_empty_text {
        &[]
    } else {
        w28a17_cstr(text)
    };
    w28a17_close_open_tag(p, newline);
    p.out.extend_from_slice(b"<!--");
    p.out.extend_from_slice(body);
    p.out.extend_from_slice(b"-->");
    1
}

/// `0x00a7dc90`: writes a declaration-style node `<!text>`.
pub fn w28a17_write_declaration(
    p: &mut W28a17Printer,
    text: &[u8],
    is_empty_text: bool,
    newline: &mut dyn FnMut(&mut W28a17Printer),
) -> u32 {
    let body: &[u8] = if is_empty_text {
        &[]
    } else {
        w28a17_cstr(text)
    };
    w28a17_close_open_tag(p, newline);
    p.out.extend_from_slice(b"<!");
    p.out.extend_from_slice(body);
    p.out.push(b'>');
    1
}

/// `0x00a7ddd0`: copies byte 0x35 of the node to the printer flag and, if
/// byte 0x34 is set, writes the constant string `prefix` (data at the
/// opaque global).  Always returns 1.
pub fn w28a17_write_flagged(p: &mut W28a17Printer, node: &[u8; 0x36], prefix: &[u8]) -> u32 {
    p.flag_4c = node[0x35];
    if node[0x34] != 0 {
        p.out.extend_from_slice(w28a17_cstr(prefix));
    }
    1
}

/// `0x00a7def0`: appends the NUL-terminated string to the output.
pub fn w28a17_write_cstr(p: &mut W28a17Printer, s: &[u8]) {
    p.out.extend_from_slice(w28a17_cstr(s));
}

/// `0x00a88fd0`: allocation size for `count` wide (2-byte) elements.
/// `None` for zero; sizes under 0x1000 are exact, larger ones get 0x23
/// bytes of header padding, saturating to `u32::MAX` on overflow.
pub fn w28a17_alloc_size(count: u32) -> Option<u32> {
    let mut bytes = count.wrapping_mul(2);
    if count < 0x8000_0000 {
        if bytes < 0x1000 {
            return if bytes != 0 { Some(bytes) } else { None };
        }
    } else {
        bytes = 0xffff_ffff;
    }
    let padded = bytes.wrapping_add(0x23);
    Some(if padded <= bytes { 0xffff_ffff } else { padded })
}

/// `0x00a892d0`: zero-initialises a four-word record.
pub fn w28a17_zero4(rec: &mut [u32; 4]) {
    *rec = [0; 4];
}

/// `0x00a89770`: signed 16-bit field at +6 divided by the unsigned 16-bit
/// field at +0xe (`None` when the divisor is zero).
pub fn w28a17_div_fields(rec: &[u8; 0x10]) -> Option<i32> {
    let a = i16::from_le_bytes([rec[6], rec[7]]) as i32;
    let b = u16::from_le_bytes([rec[0xe], rec[0xf]]) as i32;
    if b == 0 { None } else { Some(a / b) }
}

#[cfg(test)]
mod tests_wave28_agent17 {
    use super::*;

    #[test]
    fn comment_and_decl() {
        let mut p = W28a17Printer {
            tag_open: true,
            ..Default::default()
        };
        let mut nl = |q: &mut W28a17Printer| q.out.push(b'\n');
        assert_eq!(w28a17_write_comment(&mut p, b"hi\0zz", false, &mut nl), 1);
        assert_eq!(p.out, b">\x3c!--hi-->".to_vec());
        let mut q = W28a17Printer {
            depth: -1,
            ..Default::default()
        };
        w28a17_write_declaration(&mut q, b"DOC\0", false, &mut nl);
        assert_eq!(q.out, b"\n<!DOC>".to_vec());
        let mut r = W28a17Printer::default();
        w28a17_write_declaration(&mut r, b"x", true, &mut nl);
        assert_eq!(r.out, b"<!>".to_vec());
    }

    #[test]
    fn flagged_and_cstr() {
        let mut p = W28a17Printer::default();
        let mut node = [0u8; 0x36];
        node[0x35] = 7;
        assert_eq!(w28a17_write_flagged(&mut p, &node, b"ab\0"), 1);
        assert_eq!(p.flag_4c, 7);
        assert!(p.out.is_empty());
        node[0x34] = 1;
        w28a17_write_flagged(&mut p, &node, b"ab\0");
        w28a17_write_cstr(&mut p, b"cd\0e");
        assert_eq!(p.out, b"abcd".to_vec());
    }

    #[test]
    fn sizes_and_fields() {
        assert_eq!(w28a17_alloc_size(0), None);
        assert_eq!(w28a17_alloc_size(0x7ff), Some(0xffe));
        assert_eq!(w28a17_alloc_size(0x800), Some(0x1023));
        assert_eq!(w28a17_alloc_size(0x8000_0000), Some(0xffff_ffff));
        let mut z = [1, 2, 3, 4];
        w28a17_zero4(&mut z);
        assert_eq!(z, [0; 4]);
        let mut r = [0u8; 0x10];
        r[6..8].copy_from_slice(&(-9i16).to_le_bytes());
        assert_eq!(w28a17_div_fields(&r), None);
        r[0xe] = 2;
        assert_eq!(w28a17_div_fields(&r), Some(-4));
    }
}
