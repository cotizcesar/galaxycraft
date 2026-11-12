#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent16.rs
// ===========================================================================
// Wave 22 agent 16 (queue, module `runtime.rs`).

fn w22a16_cstr(buf: &[u8], at: usize) -> &[u8] {
    let end = buf[at..]
        .iter()
        .position(|&b| b == 0)
        .map_or(buf.len(), |n| at + n);
    &buf[at..end]
}

fn w22a16_eol_len(buf: &[u8], at: usize) -> usize {
    if buf[at] == b'\r' && buf.get(at + 1) == Some(&b'\n') {
        2
    } else {
        1
    }
}

/// `0x00678f70`: in-place line compactor over a NUL-terminated text
/// buffer.  `rd` is the read cursor (`*param_1`), `start` receives the
/// output start (`*param_2`, `None` when nothing was produced).  With no
/// `key`, comment lines (`;`/`#`) are kept and, when `strip_ws` is set,
/// blank lines before a comment collapse into `\n` padding; with a key a
/// line stops the scan when `less` says neither `line < key` nor
/// `key < line`.  `less` models the string comparer `FUN_00677890`.
/// Returns the retail bool result.
pub fn w22a16_compact_lines(
    buf: &mut [u8],
    rd: &mut usize,
    start: &mut Option<usize>,
    key: Option<&[u8]>,
    strip_ws: bool,
    less: &dyn Fn(&[u8], &[u8]) -> bool,
) -> bool {
    let mut wr = *rd;
    *start = Some(wr);
    let first_rd = *rd;
    let mut local5 = buf[*rd];
    let _ = first_rd;
    loop {
        if key.is_none() {
            let b = buf[*rd];
            if b != b';' && b != b'#' {
                let finish = |buf: &mut [u8],
                              rd: &mut usize,
                              start: &mut Option<usize>,
                              wr: usize,
                              local5: u8| {
                    if *start == Some(*rd) {
                        *start = None;
                        return false;
                    }
                    buf[wr - 1] = 0;
                    if key.is_some() && local5 != 0 {
                        buf[*rd] = local5;
                        *rd += w22a16_eol_len(buf, *rd);
                    }
                    true
                };
                if !strip_ws {
                    return finish(buf, rd, start, wr, local5);
                }
                let mut p = *rd;
                let mut nl = 0usize;
                loop {
                    let c = buf[p];
                    if !matches!(c, b' ' | b'\t' | b'\r' | b'\n') {
                        break;
                    }
                    let mut step = 1;
                    if c == b'\n' || c == b'\r' {
                        nl += 1;
                        if c == b'\r' && buf[p + 1] == b'\n' {
                            step = 2;
                        }
                    }
                    p += step;
                }
                let c = buf[p];
                if c != b';' && c != b'#' {
                    return finish(buf, rd, start, wr, local5);
                }
                for i in 0..nl {
                    buf[wr + i] = b'\n';
                }
                wr += nl;
                *rd = p;
                continue;
            }
        }
        let line_rd = *rd;
        let mut p = line_rd;
        while !matches!(buf[p], 0 | b'\n' | b'\r') {
            p += 1;
        }
        *rd = p;
        let len = p - line_rd;
        if wr < line_rd {
            buf.copy_within(line_rd..p, wr);
        }
        local5 = buf[p];
        buf[p] = 0;
        if let Some(k) = key {
            let line = w22a16_cstr(buf, wr).to_vec();
            if !less(&line, k) && !less(k, &line) {
                // retail jumps to the finish block
                if *start == Some(*rd) {
                    *start = None;
                    return false;
                }
                buf[wr - 1] = 0;
                if local5 != 0 {
                    buf[*rd] = local5;
                    *rd += w22a16_eol_len(buf, *rd);
                }
                return true;
            }
        }
        if local5 == 0 {
            return true;
        }
        buf[p] = local5;
        *rd = p + w22a16_eol_len(buf, p);
        buf[wr + len] = b'\n';
        wr += len + 1;
    }
}

#[cfg(test)]
mod tests_wave22_agent16 {
    use super::*;

    fn lt(a: &[u8], b: &[u8]) -> bool {
        a < b
    }

    #[test]
    fn compacts_comment_only_text() {
        let mut b = b"; a\r\n# b\0\0\0\0\0\0\0".to_vec();
        let (mut rd, mut st) = (0, None);
        assert!(w22a16_compact_lines(
            &mut b, &mut rd, &mut st, None, false, &lt
        ));
        assert_eq!(st, Some(0));
        assert_eq!(&b[..7], b"; a\n# b");
    }

    #[test]
    fn non_comment_first_line_yields_none() {
        let mut b = b"abc\0".to_vec();
        let (mut rd, mut st) = (0, None);
        assert!(!w22a16_compact_lines(
            &mut b, &mut rd, &mut st, None, false, &lt
        ));
        assert_eq!(st, None);
    }

    #[test]
    fn blank_lines_before_comment_collapse() {
        let mut b = b" \n\n;x\0\0\0\0\0\0".to_vec();
        let (mut rd, mut st) = (0, None);
        assert!(w22a16_compact_lines(
            &mut b, &mut rd, &mut st, None, true, &lt
        ));
        assert_eq!(&b[..2], b"\n\n");
    }

    #[test]
    fn key_equal_line_stops() {
        let mut b = b"a\nk\0\0".to_vec();
        let (mut rd, mut st) = (0, None);
        assert!(w22a16_compact_lines(
            &mut b,
            &mut rd,
            &mut st,
            Some(b"k"),
            false,
            &lt
        ));
        assert_eq!(st, Some(0));
        assert_eq!(b[1], 0);
    }
}
