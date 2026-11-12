#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent04.rs
// ===========================================================================
// Wave 29 agent 04: MSVC `std::vector` helpers over 8-byte elements.
// The capacity word keeps its high bit as a flag; only the low 31 bits count.

/// `0x00aa2420`: opens a gap of `n` slots at element index `idx` in a vector
/// of 8-byte elements (`insert(pos, n, ..)` front half).  Returns `None` when
/// `size + n` exceeds the capacity (retail reallocates, outside this body).
/// Otherwise the vector grows by `n` zeroed-or-moved slots and the result is
/// `(first, fill_begin, fill_end)` element indices the caller fills with the
/// new values.
pub fn w29a04_vector_open_gap(
    elems: &mut Vec<[u32; 2]>,
    cap_word: u32,
    idx: usize,
    n: usize,
) -> Option<(usize, usize, usize)> {
    let size = elems.len();
    if size + n > (cap_word & 0x7fff_ffff) as usize {
        return None;
    }
    let after = idx + n;
    elems.resize(size + n, [0, 0]);
    if size <= after {
        for i in idx..size {
            elems[after + (i - idx)] = elems[i];
        }
        Some((idx, size, after))
    } else {
        for i in 0..n {
            elems[size + i] = elems[size - n + i];
        }
        for i in (idx..size - n).rev() {
            elems[i + n] = elems[i];
        }
        Some((idx, after, after))
    }
}

/// `0x00aa25f0`: `push_back` of an 8-byte element.  `alias_idx` is the
/// element index when the source points inside the vector itself (it is read
/// before any growth); otherwise `value` is appended.  Returns `false` when
/// the vector is full (retail reallocates in code outside this body).
pub fn w29a04_vector_push_back(
    elems: &mut Vec<[u32; 2]>,
    cap_word: u32,
    value: [u32; 2],
    alias_idx: Option<usize>,
) -> bool {
    if elems.len() == (cap_word & 0x7fff_ffff) as usize {
        return false;
    }
    let v = match alias_idx {
        Some(i) if i < elems.len() => elems[i],
        _ => value,
    };
    elems.push(v);
    true
}

#[cfg(test)]
mod tests_wave29_agent04 {
    use super::*;

    fn v(n: u32) -> Vec<[u32; 2]> {
        (0..n).map(|i| [i, i + 100]).collect()
    }

    #[test]
    fn push_back_alias_and_full() {
        let mut e = v(2);
        assert!(w29a04_vector_push_back(
            &mut e,
            0x8000_0004,
            [9, 9],
            Some(1)
        ));
        assert_eq!(e[2], [1, 101]);
        assert!(w29a04_vector_push_back(&mut e, 4, [9, 9], Some(7)));
        assert_eq!(e[3], [9, 9]);
        assert!(!w29a04_vector_push_back(&mut e, 4, [0, 0], None));
        assert_eq!(e.len(), 4);
    }

    #[test]
    fn open_gap_cases() {
        let mut e = v(4);
        assert_eq!(w29a04_vector_open_gap(&mut e, 5, 0, 2), None);
        // size <= idx + n: tail shifts, gap spans [size, idx+n).
        let mut e = v(3);
        assert_eq!(w29a04_vector_open_gap(&mut e, 8, 2, 2), Some((2, 3, 4)));
        assert_eq!(e[4], [2, 102]);
        // middle insert.
        let mut e = v(4);
        assert_eq!(w29a04_vector_open_gap(&mut e, 8, 1, 2), Some((1, 3, 3)));
        let firsts: Vec<u32> = e.iter().map(|x| x[0]).collect();
        assert_eq!(&firsts[..1], &[0]);
        assert_eq!(&firsts[3..], &[1, 2, 3]);
    }
}
