#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent49.rs
// ===========================================================================
// Wave 34 agent 49 translations.

/// `0x00c6bdd0`: runs a cursor-advancing routine over `*cursor` and, when an
/// out-slot is supplied, stores the number of bytes consumed.
pub fn w34a49_consumed_len(
    start: i32,
    param3: u32,
    advance: impl FnOnce(i32, u32) -> i32,
    out: Option<&mut i32>,
) {
    let end = advance(start, param3);
    if let Some(o) = out {
        *o = end.wrapping_sub(start);
    }
}

/// `0x00c713f0`: directory-exists check on a NUL-terminated UTF-16 path.
/// `exists` is the probe with flag 0, `attrs` returns file attributes
/// (`0xffffffff` on failure; bit 4 is the directory flag).  When attributes
/// fail, the path is cut at the last `\` or `/` and the parent is checked
/// recursively; on success the separator is restored as `\` and re-probed.
pub fn w34a49_dir_exists(
    path: &mut [u16],
    exists: &dyn Fn(&[u16]) -> bool,
    attrs: &dyn Fn(&[u16]) -> u32,
) -> u32 {
    if exists(path) {
        return 1;
    }
    let a = attrs(path);
    if a != 0xffff_ffff {
        return (a >> 4) & 1;
    }
    let find = |p: &[u16], c: u16| {
        let n = p.iter().position(|&x| x == 0).unwrap_or(p.len());
        p[..n].iter().rposition(|&x| x == c)
    };
    let s1 = find(path, 0x5c);
    let s2 = find(path, 0x2f);
    let cut = match (s1, s2) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    if let Some(i) = cut {
        path[i] = 0;
        if w34a49_dir_exists(path, exists, attrs) & 0xff != 0 {
            path[i] = 0x5c;
            return u32::from(exists(path));
        }
    }
    0
}

/// State tables for `0x00c80ce0`.
#[derive(Default, Clone, Debug)]
pub struct W34a49NvState {
    pub handle: [i32; 2],
    pub loaded: [i32; 2],
    pub flag_a: [i32; 2],
    pub flag_b: [i32; 2],
}

/// `0x00c80ce0`: lazily loads `nvapi.dll` (index 0) or `nvpowerapi.dll`
/// (otherwise); `mode` must be 0 or 1 (else -3).  Load failure -> -2; init
/// error is returned after unloading.
pub fn w34a49_nv_load(
    st: &mut W34a49NvState,
    index: usize,
    mode: i32,
    load: impl FnOnce(&str) -> i32,
    init: impl FnOnce(i32, usize, i32) -> i32,
    unload: impl FnOnce(i32),
) -> i32 {
    let m = match mode {
        0 => 0,
        1 => 1,
        _ => return -3,
    };
    if st.handle[index] == 0 {
        let name = if index != 0 {
            "nvpowerapi.dll"
        } else {
            "nvapi.dll"
        };
        let h = load(name);
        if h == 0 {
            return -2;
        }
        let r = init(h, index, mode);
        if r != 0 {
            unload(h);
            return r;
        }
        st.handle[index] = h;
        st.loaded[index] = 1;
        st.flag_a[m] = 1;
        st.flag_b[m] = 1;
    }
    0
}

#[cfg(test)]
mod tests_wave34_agent49 {
    use super::*;

    #[test]
    fn consumed() {
        let mut o = 0;
        w34a49_consumed_len(10, 0, |s, _| s + 7, Some(&mut o));
        assert_eq!(o, 7);
        w34a49_consumed_len(10, 0, |s, _| s, None);
    }

    #[test]
    fn dir() {
        let mut p: Vec<u16> = "a\\b".encode_utf16().chain([0]).collect();
        let ex = |q: &[u16]| {
            q.first() == Some(&0x61) && q.get(1) == Some(&0x5c) && q.get(2) == Some(&0x62)
        };
        let at = |q: &[u16]| {
            if q.get(1) == Some(&0) {
                0x10
            } else {
                0xffff_ffff
            }
        };
        assert_eq!(w34a49_dir_exists(&mut p, &ex, &at), 1);
        let mut p2: Vec<u16> = "a\\c".encode_utf16().chain([0]).collect();
        assert_eq!(w34a49_dir_exists(&mut p2, &ex, &at), 0);
    }

    #[test]
    fn nv() {
        let mut st = W34a49NvState::default();
        assert_eq!(
            w34a49_nv_load(&mut st, 0, 2, |_| 1, |_, _, _| 0, |_| {}),
            -3
        );
        assert_eq!(
            w34a49_nv_load(&mut st, 0, 0, |_| 0, |_, _, _| 0, |_| {}),
            -2
        );
        assert_eq!(
            w34a49_nv_load(
                &mut st,
                1,
                1,
                |n| {
                    assert_eq!(n, "nvpowerapi.dll");
                    5
                },
                |_, _, _| 0,
                |_| {}
            ),
            0
        );
        assert_eq!(st.handle[1], 5);
        assert_eq!(st.flag_a[1], 1);
        assert_eq!(w34a49_nv_load(&mut st, 0, 0, |_| 6, |_, _, _| 9, |_| {}), 9);
        assert_eq!(st.handle[0], 0);
    }
}
