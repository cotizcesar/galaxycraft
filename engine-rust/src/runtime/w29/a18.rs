#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent18.rs
// ===========================================================================
// Wave 29 agent 18: tact/CASC directory-handler helpers.

/// `0x00ae8ce0`: clamps a (offset, length) span to a limit.  Returns
/// `(offset, len)`; the length is zero when `limit <= offset`, otherwise
/// `min(len, limit - offset)`.
pub fn w29a18_clamp_span(offset: u64, len: u64, limit: u64) -> (u64, u64) {
    if limit <= offset {
        return (offset, 0);
    }
    (offset, len.min(limit - offset))
}

/// Query kind selected by `0x00ae7180`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W29a18Query {
    /// Bit 0 handler (callee `ae6520`).
    Bit0,
    /// Bit 1 handler (callee `ae7e60`).
    Bit1,
    /// Bit 2 handler, QA_READ_DATA (callee `ae73a0`).
    Bit2,
    /// Bit 3 handler, QA_READ_MAXSIZE (callee `ae7930`).
    Bit3,
    /// No known bit set: the original calls the error helper with code 3.
    Invalid,
}

/// `0x00ae7180`: picks the sub-handler from the query flags, testing bits
/// 0, 1, 2, 3 in that order.
pub fn w29a18_select_query(flags: u32) -> W29a18Query {
    if flags & 1 != 0 {
        W29a18Query::Bit0
    } else if flags & 2 != 0 {
        W29a18Query::Bit1
    } else if flags & 4 != 0 {
        W29a18Query::Bit2
    } else if flags & 8 != 0 {
        W29a18Query::Bit3
    } else {
        W29a18Query::Invalid
    }
}

/// Result of the path validation in `0x00ae6d30`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W29a18PathCheck {
    /// Null path: status code 2 and an empty result.
    Null,
    /// Longer than `0x104` bytes (excluding terminator): fatal.
    TooLong,
    /// Accepted; carries the length including the terminator.
    Ok(usize),
}

/// `0x00ae6d30`: validates a C path; limit is `0x104` bytes of text.
pub fn w29a18_check_path(path: Option<&[u8]>) -> W29a18PathCheck {
    let Some(p) = path else {
        return W29a18PathCheck::Null;
    };
    let n = p.iter().position(|&b| b == 0).unwrap_or(p.len());
    if n > 0x104 {
        W29a18PathCheck::TooLong
    } else {
        W29a18PathCheck::Ok(n + 1)
    }
}

/// `0x00ae7b30`: precondition of the segmented-storage write.  The span
/// `[start, end)` must contain `len` bytes from `off` (`end >= off`,
/// `end - off >= len`).
pub fn w29a18_write_in_bounds(off: u64, len: u64, end: u64) -> bool {
    end >= off && end - off >= len
}

#[cfg(test)]
mod tests_wave29_agent18 {
    use super::*;

    #[test]
    fn clamp() {
        assert_eq!(w29a18_clamp_span(10, 100, 10), (10, 0));
        assert_eq!(w29a18_clamp_span(10, 100, 50), (10, 40));
        assert_eq!(w29a18_clamp_span(10, 5, 50), (10, 5));
        assert_eq!(w29a18_clamp_span(1 << 33, 7, 1 << 34), (1 << 33, 7));
    }

    #[test]
    fn select() {
        assert_eq!(w29a18_select_query(1 | 8), W29a18Query::Bit0);
        assert_eq!(w29a18_select_query(2), W29a18Query::Bit1);
        assert_eq!(w29a18_select_query(4 | 8), W29a18Query::Bit2);
        assert_eq!(w29a18_select_query(8), W29a18Query::Bit3);
        assert_eq!(w29a18_select_query(0x10), W29a18Query::Invalid);
    }

    #[test]
    fn path() {
        assert_eq!(w29a18_check_path(None), W29a18PathCheck::Null);
        assert_eq!(w29a18_check_path(Some(b"ab\0")), W29a18PathCheck::Ok(3));
        let long = vec![b'a'; 0x105];
        assert_eq!(w29a18_check_path(Some(&long)), W29a18PathCheck::TooLong);
        assert_eq!(
            w29a18_check_path(Some(&long[..0x104])),
            W29a18PathCheck::Ok(0x105)
        );
    }

    #[test]
    fn write_bounds() {
        assert!(w29a18_write_in_bounds(4, 6, 10));
        assert!(!w29a18_write_in_bounds(4, 7, 10));
        assert!(!w29a18_write_in_bounds(11, 0, 10));
    }
}
