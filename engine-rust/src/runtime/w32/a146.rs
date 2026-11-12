#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent146.rs
// ===========================================================================
// Wave 32 translation batch 146: CRT stat-mode helpers.

/// `0x00ca4e6a`: CRT stream status check.  `None` models a null stream and
/// returns `closure_null()`; otherwise returns -1 when `has_error(stream)` is
/// nonzero, or when the stream's flag bit 11 is set and `handle_error` of the
/// stream's handle is nonzero; else 0.
pub fn w32a146_stream_error<S: Copy>(
    stream: Option<S>,
    flags: impl Fn(S) -> u32,
    closure_null: impl Fn() -> i32,
    has_error: impl Fn(S) -> bool,
    handle: impl Fn(S) -> u32,
    handle_error: impl Fn(u32) -> bool,
) -> i32 {
    let Some(s) = stream else {
        return closure_null();
    };
    if has_error(s) {
        return -1;
    }
    if (flags(s) >> 11) & 1 != 0 && handle_error(handle(s)) {
        return -1;
    }
    0
}

/// `0x00ca56a4`: builds a POSIX-style st_mode from DOS attributes.  `attr`
/// bit 4 is the directory flag; `is_dir_hint` stands in for the path probe.
/// The extension (text after the last `.`, ASCII case-insensitive) being
/// exe/cmd/bat/com adds the execute bit; the user bits are then replicated
/// into group and other.
pub fn w32a146_attr_to_mode(attr: u32, path: Option<&str>, is_dir_hint: bool) -> u32 {
    let mut m: u32 = if (attr >> 4) & 1 == 0 && !is_dir_hint {
        0x8100
    } else {
        0x4140
    };
    m |= !(attr << 7) & 0x80;
    if let Some(p) = path {
        if let Some(i) = p.rfind('.') {
            let ext = &p[i..];
            if [".exe", ".cmd", ".bat", ".com"]
                .iter()
                .any(|e| ext.eq_ignore_ascii_case(e))
            {
                m |= 0x40;
            }
        }
    }
    ((m & 0x1c0) >> 6) | m | ((m >> 3) & 0x38)
}

#[cfg(test)]
mod tests_wave32_agent146 {
    use super::*;

    #[test]
    fn mode_basic() {
        assert_eq!(w32a146_attr_to_mode(0, None, false), 0x81b6);
        let m = w32a146_attr_to_mode(1, Some("a.EXE"), false);
        assert_eq!(m & 0x40, 0x40);
        let d = w32a146_attr_to_mode(0x10, Some("a.txt"), false);
        assert_eq!(d & 0x4000, 0x4000);
    }

    #[test]
    fn stream() {
        assert_eq!(
            w32a146_stream_error(None::<u32>, |_| 0, || 7, |_| false, |s| s, |_| false),
            7
        );
        assert_eq!(
            w32a146_stream_error(Some(1u32), |_| 0x800, || 7, |_| false, |s| s, |_| true),
            -1
        );
        assert_eq!(
            w32a146_stream_error(Some(1u32), |_| 0, || 7, |_| false, |s| s, |_| true),
            0
        );
    }
}
