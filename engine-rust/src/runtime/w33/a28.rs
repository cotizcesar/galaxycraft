#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent28.rs
// ===========================================================================
// Wave 33 agent 28 translations (runtime.rs target).

/// `0x00b0ab30`: matches a lookup result against a key.  `lookup(p1, p2)`
/// models the callee and returns `(index, key_lo, key_hi)`.  Returns true when
/// the found key equals `(p1, p2)` and either the index equals `end_index`
/// (`ecx[1]`) or `p4 == 0` and the i32 at byte offset `4` of the `0x1c`-byte
/// row `index` in `rows` equals `p3`.
pub fn w33a28_key_matches(
    rows: &[u8],
    end_index: i32,
    p1: i32,
    p2: i32,
    p3: i32,
    p4: i32,
    lookup: impl FnOnce(i32, i32) -> (i32, i32, i32),
) -> bool {
    let (idx, lo, hi) = lookup(p1, p2);
    if p1 != lo || p2 != hi {
        return false;
    }
    if idx == end_index {
        return true;
    }
    if p4 != 0 || idx < 0 {
        return false;
    }
    let off = idx as usize * 0x1c + 4;
    match rows.get(off..off + 4) {
        Some(b) => i32::from_le_bytes([b[0], b[1], b[2], b[3]]) == p3,
        None => false,
    }
}

/// `0x00b4f0c0`: if `dirty`, sorts the 16-byte `[a, b, delta, pad]` entries
/// with `sort`, merges runs sharing `(a, b)` and keeps only entries where the
/// running sum crosses zero (emitting `+1` when it becomes positive, `-1` when
/// it drops to zero or below), then truncates and clears `dirty`.
pub fn w33a28_normalize_edges(
    entries: &mut Vec<[i32; 4]>,
    dirty: &mut bool,
    sort: impl FnOnce(&mut [[i32; 4]]),
) {
    if !*dirty {
        return;
    }
    sort(entries);
    let n = entries.len();
    let mut w = 0usize;
    let mut i = 0usize;
    let mut carry = 0i32;
    while i < n {
        let key = (entries[i][0], entries[i][1]);
        let mut sum = entries[i][2].wrapping_add(carry);
        let mut j = i + 1;
        while j < n && (entries[j][0], entries[j][1]) == key {
            sum = sum.wrapping_add(entries[j][2]);
            j += 1;
        }
        if carry < 1 {
            if sum > 0 {
                entries[w][0] = key.0;
                entries[w][1] = key.1;
                entries[w][2] = 1;
                w += 1;
            }
        } else if sum < 1 {
            entries[w][0] = key.0;
            entries[w][1] = key.1;
            entries[w][2] = -1;
            w += 1;
        }
        carry = sum;
        i = j;
    }
    entries.truncate(w);
    *dirty = false;
}

#[cfg(test)]
mod tests_wave33_agent28 {
    use super::*;

    #[test]
    fn edges() {
        let mut v = vec![[1, 1, 1, 0], [1, 1, 1, 0], [2, 2, -2, 0], [3, 3, 1, 0]];
        let mut d = true;
        w33a28_normalize_edges(&mut v, &mut d, |_| {});
        assert!(!d);
        assert_eq!(v, vec![[1, 1, 1, 0], [2, 2, -1, 0], [3, 3, 1, 0]]);
    }

    #[test]
    fn matches() {
        let mut rows = vec![0u8; 0x38];
        rows[0x1c + 4..0x1c + 8].copy_from_slice(&7i32.to_le_bytes());
        assert!(w33a28_key_matches(&rows, 9, 1, 2, 7, 0, |_, _| (1, 1, 2)));
        assert!(!w33a28_key_matches(&rows, 9, 1, 2, 8, 0, |_, _| (1, 1, 2)));
        assert!(w33a28_key_matches(&rows, 1, 1, 2, 8, 5, |_, _| (1, 1, 2)));
    }
}
