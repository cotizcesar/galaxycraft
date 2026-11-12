#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent64.rs
// ===========================================================================
// Wave 32 agent 64: path trimming helpers.

fn w32a64_cstr_len(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

/// `0x0093f380`: truncates a NUL-terminated path in place after its last
/// `/` or `\` separator (keeping the separator), i.e. removes the file name.
/// When the cut point lies inside the root prefix, the string is cut at the
/// root length instead.
pub fn w32a64_strip_filename(buf: &mut [u8]) {
    let len = w32a64_cstr_len(buf);
    let mut cut = 0usize;
    for (i, &c) in buf[..len].iter().enumerate() {
        if c == b'/' || c == b'\\' {
            cut = i + 1;
        }
    }
    if len != 0 && cut != 0 {
        let root = crate::runtime::w31a57_path_root_len(buf);
        if cut < root {
            buf[root] = 0;
            return;
        }
    }
    buf[cut] = 0;
}

/// `0x0093f3d0`: when the path is longer than its root prefix, drops one
/// trailing separator and then strips the last path component with
/// the routine at `0x0093f380`.
pub fn w32a64_strip_last_component(buf: &mut [u8]) {
    let len = w32a64_cstr_len(buf);
    let root = crate::runtime::w31a57_path_root_len(buf);
    if root < len {
        if buf[len - 1] == b'\\' || buf[len - 1] == b'/' {
            buf[len - 1] = 0;
        }
        w32a64_strip_filename(buf);
    }
}

#[cfg(test)]
mod tests_wave32_agent64 {
    use super::*;

    fn s(b: &[u8]) -> String {
        let n = w32a64_cstr_len(b);
        String::from_utf8_lossy(&b[..n]).into_owned()
    }

    #[test]
    fn strip_filename_cases() {
        let mut a = b"C:\\dir\\file.txt\0".to_vec();
        w32a64_strip_filename(&mut a);
        assert_eq!(s(&a), "C:\\dir\\");
        let mut b = b"file\0".to_vec();
        w32a64_strip_filename(&mut b);
        assert_eq!(s(&b), "");
    }

    #[test]
    fn strip_last_component_cases() {
        let mut a = b"C:\\dir\\sub\\\0".to_vec();
        w32a64_strip_last_component(&mut a);
        assert_eq!(s(&a), "C:\\dir\\");
        let mut b = b"C:\\\0".to_vec();
        w32a64_strip_last_component(&mut b);
        assert_eq!(s(&b), "C:\\");
    }
}
