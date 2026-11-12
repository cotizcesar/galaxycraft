#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent71.rs
// ===========================================================================
// Wave 32 agent 71: small object-record getters.

/// Bounded string copy (callee 0099ed50 shape): copies up to `n` bytes of
/// `src`, stopping after the NUL terminator.
pub fn w32a71_copy_bounded(dst: &mut [u8], src: &[u8], n: usize) {
    for i in 0..n.min(dst.len()) {
        let c = src.get(i).copied().unwrap_or(0);
        dst[i] = c;
        if c == 0 {
            break;
        }
    }
}

/// Unbounded NUL-terminated copy (callee 0099eba0 shape), including the NUL.
pub fn w32a71_copy_cstr(dst: &mut [u8], src: &[u8]) {
    for i in 0..dst.len() {
        let c = src.get(i).copied().unwrap_or(0);
        dst[i] = c;
        if c == 0 {
            break;
        }
    }
}

/// Fields read by `0x0099d6a0` (object offsets 0x6c name, 0x8c, 0x90, 0xbc, 0xc0).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W32a71Info {
    pub name: [u8; 0x20],
    pub f8c: u32,
    pub f90: u32,
    pub fbc: u32,
    pub fc0: u32,
}

/// `0x0099d6a0`: optional-output getter; copies up to 0x20 name bytes and
/// the four scalar fields into whichever outputs are supplied.  Returns 0.
pub fn w32a71_get_info(
    info: &W32a71Info,
    name: Option<&mut [u8]>,
    a: Option<&mut u32>,
    b: Option<&mut u32>,
    c: Option<&mut u32>,
    d: Option<&mut u32>,
) -> u32 {
    if let Some(n) = name {
        w32a71_copy_bounded(n, &info.name, 0x20);
    }
    if let Some(x) = a {
        *x = info.f8c;
    }
    if let Some(x) = b {
        *x = info.f90;
    }
    if let Some(x) = c {
        *x = info.fbc;
    }
    if let Some(x) = d {
        *x = info.fc0;
    }
    0
}

/// One 0x30-byte record: +0 u32, +4 u32, +0xc name, +0x1c name, +0x2c text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W32a71Entry {
    pub v0: u32,
    pub v4: u32,
    pub name_c: Vec<u8>,
    pub name_1c: Vec<u8>,
    /// `None` models a null pointer at +0x2c.
    pub text: Option<Vec<u8>>,
}

/// `0x0099db60`: fetch record `index` into optional outputs.  Returns
/// 0x25 when `index` is out of range, else 0.
#[allow(clippy::too_many_arguments)]
pub fn w32a71_get_entry(
    entries: &[W32a71Entry],
    index: i32,
    out_c: Option<&mut [u8]>,
    out_1c: Option<&mut [u8]>,
    text: Option<&mut [u8]>,
    out0: Option<&mut u32>,
    out4: Option<&mut u32>,
) -> u32 {
    if index < 0 || index as usize >= entries.len() {
        return 0x25;
    }
    let e = &entries[index as usize];
    if let Some(o) = out_c {
        w32a71_copy_cstr(o, &e.name_c);
    }
    if let Some(t) = text {
        if !t.is_empty() {
            match &e.text {
                None => t[0] = 0,
                Some(s) => {
                    let n = t.len();
                    w32a71_copy_bounded(t, s, n);
                }
            }
        }
    }
    if let Some(o) = out_1c {
        w32a71_copy_cstr(o, &e.name_1c);
    }
    if let Some(o) = out0 {
        *o = e.v0;
    }
    if let Some(o) = out4 {
        *o = e.v4;
    }
    0
}

#[cfg(test)]
mod tests_wave32_agent71 {
    use super::*;

    #[test]
    fn info_getter() {
        let mut info = W32a71Info {
            f8c: 1,
            f90: 2,
            fbc: 3,
            fc0: 4,
            ..Default::default()
        };
        info.name[..3].copy_from_slice(b"ab\0");
        let mut n = [9u8; 0x20];
        let (mut a, mut b, mut c, mut d) = (0, 0, 0, 0);
        let r = w32a71_get_info(
            &info,
            Some(&mut n),
            Some(&mut a),
            Some(&mut b),
            Some(&mut c),
            Some(&mut d),
        );
        assert_eq!((r, a, b, c, d), (0, 1, 2, 3, 4));
        assert_eq!(&n[..4], b"ab\0\x09");
        assert_eq!(w32a71_get_info(&info, None, None, None, None, None), 0);
    }

    #[test]
    fn entry_getter() {
        let es = vec![W32a71Entry {
            v0: 7,
            v4: 8,
            name_c: b"x\0".to_vec(),
            name_1c: b"y\0".to_vec(),
            text: None,
        }];
        let (mut o0, mut o4) = (0, 0);
        let mut c = [5u8; 4];
        let mut t = [5u8; 4];
        let r = w32a71_get_entry(
            &es,
            0,
            Some(&mut c),
            None,
            Some(&mut t),
            Some(&mut o0),
            Some(&mut o4),
        );
        assert_eq!((r, o0, o4, c[0], t[0]), (0, 7, 8, b'x', 0));
        assert_eq!(w32a71_get_entry(&es, 1, None, None, None, None, None), 0x25);
        assert_eq!(
            w32a71_get_entry(&es, -1, None, None, None, None, None),
            0x25
        );
    }
}
