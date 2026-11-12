#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent43.rs
// ===========================================================================
// Wave 34 translation batch 43.

/// `0x009e0140`: indexed entry lookup.  When `initialised` is false the
/// `init` callback (FUN_009dfc40) runs and a nonzero result is returned
/// as is.  Index outside `0..names.len()` yields 0x25.  Name and path
/// outputs are copied truncated to `len - 1` units and terminated with 0;
/// the 16-byte record is copied to `guid_out` when present.
pub fn w34a43_get_entry(
    initialised: bool,
    init: impl FnOnce() -> i32,
    index: i32,
    names: &[Vec<u16>],
    paths: &[Vec<u16>],
    guids: &[[u32; 4]],
    name_out: Option<&mut [u16]>,
    path_out: Option<&mut [u16]>,
    guid_out: Option<&mut [u32; 4]>,
) -> i32 {
    if !initialised {
        let r = init();
        if r != 0 {
            return r;
        }
    }
    if index < 0 || index as usize >= names.len() {
        return 0x25;
    }
    let i = index as usize;
    let copy = |dst: Option<&mut [u16]>, src: &[u16]| {
        if let Some(d) = dst {
            if d.is_empty() {
                return;
            }
            let n = d.len() - 1;
            for (k, slot) in d.iter_mut().take(n).enumerate() {
                *slot = src.get(k).copied().unwrap_or(0);
            }
            d[n] = 0;
        }
    };
    copy(name_out, &names[i]);
    copy(path_out, paths.get(i).map(|v| v.as_slice()).unwrap_or(&[]));
    if let (Some(g), Some(s)) = (guid_out, guids.get(i)) {
        *g = *s;
    }
    0
}

#[cfg(test)]
mod tests_wave34_agent43 {
    use super::*;

    #[test]
    fn entry_lookup() {
        let names = vec![vec![65u16, 66, 67]];
        let paths = vec![vec![1u16]];
        let guids = vec![[1, 2, 3, 4]];
        let mut n = [9u16; 3];
        let mut g = [0u32; 4];
        let r = w34a43_get_entry(
            true,
            || 7,
            0,
            &names,
            &paths,
            &guids,
            Some(&mut n),
            None,
            Some(&mut g),
        );
        assert_eq!((r, n, g), (0, [65, 66, 0], [1, 2, 3, 4]));
        assert_eq!(
            w34a43_get_entry(true, || 0, 1, &names, &paths, &guids, None, None, None),
            0x25
        );
        assert_eq!(
            w34a43_get_entry(false, || 5, 0, &names, &paths, &guids, None, None, None),
            5
        );
    }
}
