#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent125.rs
// ===========================================================================
// Wave 31 agent 125: MSVC CRT exception-handling and memchr helpers.

/// Result of the try-block range search (`0x00c88fdb`): iterator bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W31a125Range {
    pub low: u32,
    pub high: u32,
}

/// `0x00c88fdb`: finds the try-block range for a try level.  `entries` are
/// 5-dword (0x14 byte) records; fields 1 and 2 (byte offsets 4 and 8) are
/// the inclusive bounds compared with `level`.  `count` is the table length,
/// `depth` is the nesting depth requested.  `None` means the retail code
/// terminates (invalid range).
pub fn w31a125_range_of_trys(
    entries: &[[i32; 5]],
    count: u32,
    level: i32,
    mut depth: i32,
) -> Option<W31a125Range> {
    let mut u3 = count;
    let mut u4 = count;
    let mut local8 = count;
    if depth >= 0 {
        loop {
            if u3 == u32::MAX {
                return None;
            }
            u3 = u3.wrapping_sub(1);
            let hit = entries
                .get(u3 as usize)
                .is_some_and(|e| e[1] < level && level <= e[2]);
            if hit || u3 == u32::MAX {
                depth -= 1;
                u4 = local8;
                local8 = u3;
            }
            if depth < 0 {
                break;
            }
        }
    }
    if u4 <= count && u3.wrapping_add(1) <= u4 {
        Some(W31a125Range {
            low: u3.wrapping_add(1),
            high: u4,
        })
    } else {
        None
    }
}

/// `0x00c8926e`: unlinks `node` from a singly linked list given as a head
/// index and a `next` array.  Returns `false` when the node is not in the
/// list (retail terminates).
pub fn w31a125_unlink(head: &mut Option<usize>, next: &mut [Option<usize>], node: usize) -> bool {
    if *head == Some(node) {
        *head = next[node];
        return true;
    }
    let mut cur = match *head {
        Some(c) => c,
        None => return false,
    };
    loop {
        match next[cur] {
            None => return false,
            Some(n) if n == node => {
                next[cur] = next[node];
                return true;
            }
            Some(n) => cur = n,
        }
    }
}

/// Outcome of `0x00c894a2`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W31a125Unwind {
    Continue,
    Terminate,
}

/// `0x00c894a2`: exception-code filter that maintains the per-thread
/// uncaught counter.  Two codes decrement a positive counter; the C++
/// exception code clears it and terminates.
pub fn w31a125_filter(code: i32, counter: &mut i32) -> W31a125Unwind {
    if code == -0x1fbcbcae || code == -0x1fbcb0b3 {
        if *counter > 0 {
            *counter -= 1;
        }
    } else if code == -0x1f928c9d {
        *counter = 0;
        return W31a125Unwind::Terminate;
    }
    W31a125Unwind::Continue
}

/// Exception-name holder: data plus an "owns the buffer" flag.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct W31a125Name {
    pub data: Option<String>,
    pub owned: bool,
}

/// `0x00c894fe`: copies `src` into `dst`.  A non-owning (or null) source is
/// shared by reference; an owning source is duplicated and `dst` owns it.
pub fn w31a125_name_copy(src: &W31a125Name, dst: &mut W31a125Name) {
    match (&src.data, src.owned) {
        (Some(s), true) => {
            dst.data = Some(s.clone());
            dst.owned = true;
        }
        _ => {
            dst.data = src.data.clone();
            dst.owned = false;
        }
    }
}

/// `0x00c89561`: releases the holder (drops owned data) and clears it.
pub fn w31a125_name_free(n: &mut W31a125Name) {
    n.data = None;
    n.owned = false;
}

/// `0x00c8a380`: memchr; index of the first byte equal to `needle` in
/// `buf[..len]`, or `None`.
pub fn w31a125_memchr(buf: &[u8], needle: u8, len: usize) -> Option<usize> {
    let n = len.min(buf.len());
    buf[..n].iter().position(|&b| b == needle)
}

/// `0x00c8d2d2`: true when any of `rows` thrown types matches any of the
/// `catches` handler types, via `matches(row, catch)`.
pub fn w31a125_any_match(
    rows: usize,
    catches: usize,
    mut matches: impl FnMut(usize, usize) -> bool,
) -> bool {
    (0..rows).any(|r| (0..catches).any(|c| matches(r, c)))
}

#[cfg(test)]
mod tests_wave31_agent125 {
    use super::*;

    #[test]
    fn range() {
        let e = [[0, 0, 5, 0, 0], [0, 6, 9, 0, 0]];
        assert_eq!(
            w31a125_range_of_trys(&e, 2, 7, 0),
            Some(W31a125Range { low: 2, high: 2 })
        );
        assert_eq!(w31a125_range_of_trys(&e, 2, 100, 5), None);
    }

    #[test]
    fn unlink() {
        let mut next = vec![Some(1), Some(2), None];
        let mut head = Some(0);
        assert!(w31a125_unlink(&mut head, &mut next, 1));
        assert_eq!(next[0], Some(2));
        assert!(w31a125_unlink(&mut head, &mut next, 0));
        assert_eq!(head, Some(2));
        assert!(!w31a125_unlink(&mut head, &mut next, 1));
    }

    #[test]
    fn filter() {
        let mut c = 2;
        assert_eq!(w31a125_filter(-0x1fbcbcae, &mut c), W31a125Unwind::Continue);
        assert_eq!(c, 1);
        assert_eq!(
            w31a125_filter(-0x1f928c9d, &mut c),
            W31a125Unwind::Terminate
        );
        assert_eq!(c, 0);
    }

    #[test]
    fn names() {
        let src = W31a125Name {
            data: Some("x".into()),
            owned: true,
        };
        let mut d = W31a125Name::default();
        w31a125_name_copy(&src, &mut d);
        assert!(d.owned);
        w31a125_name_free(&mut d);
        assert_eq!(d, W31a125Name::default());
    }

    #[test]
    fn memchr_and_match() {
        assert_eq!(w31a125_memchr(b"abcd", b'c', 4), Some(2));
        assert_eq!(w31a125_memchr(b"abcd", b'd', 3), None);
        assert!(w31a125_any_match(2, 2, |r, c| r == 1 && c == 0));
        assert!(!w31a125_any_match(0, 2, |_, _| true));
    }
}
