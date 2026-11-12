#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent117.rs
// ===========================================================================
// Wave 32 agent 117: GL render-state cache helpers (state diffing).

/// One GL call emitted by the state-cache helpers.
#[derive(Debug, Clone, PartialEq)]
pub enum W32a117Call {
    /// Indexed call made when a bit turns off (`DAT_013d3684`).
    Disable(u32),
    /// No-argument call made when a bit turns on (`DAT_013d3694`).
    Enable,
    /// `DAT_013d3654(table value)`.
    Face(u32),
    /// `DAT_013d36a8(arg)`.
    Mode(u32),
    /// `DAT_013d3754(param_3, param_2)`.
    Pair(f32, f32),
    /// `DAT_013d377c(value)`.
    Byte(u32),
    /// `DAT_013d3778(target, a, b, c)`.
    Func(u32, u32, u32, u32),
    /// `DAT_013d3788(target, a, b, c)`.
    Op(u32, u32, u32, u32),
}

/// Cached fields at the object offsets used by the three routines.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct W32a117State {
    pub flags_168: u32,
    pub f_16c: f32,
    pub f_170: f32,
    pub w_178: u32,
    pub w_17c: u32,
    pub w_180: u32,
    pub mask_184: u32,
}

/// `0x00c5d120`: diffs the enable mask at +0x184 against `param_1`; for
/// each changed bit (lowest first) emits Enable when set, Disable(bit) when clear.
pub fn w32a117_update_mask(st: &mut W32a117State, param_1: u32, out: &mut Vec<W32a117Call>) {
    let mut diff = st.mask_184 ^ param_1;
    if diff == 0 {
        return;
    }
    st.mask_184 = param_1;
    while diff != 0 {
        let bit = diff.trailing_zeros();
        let m = 1u32 << bit;
        diff &= !m;
        if param_1 & m == 0 {
            out.push(W32a117Call::Disable(bit));
        } else {
            out.push(W32a117Call::Enable);
        }
    }
}

/// `0x00c5d170`: caches the low 3 flag bits at +0x168 and two floats at
/// +0x16c/+0x170; `face_table` is the dword table at 0x00e3_efac.
pub fn w32a117_update_flags(
    st: &mut W32a117State,
    param_1: u32,
    p2: f32,
    p3: f32,
    face_table: &[u32],
    out: &mut Vec<W32a117Call>,
) {
    let low = param_1 & 3;
    if st.flags_168 & 3 != low {
        st.flags_168 = (st.flags_168 & !3) | low;
        if low != 0 {
            out.push(W32a117Call::Face(face_table[low as usize]));
        }
    }
    if (st.flags_168 ^ param_1) & 4 != 0 {
        st.flags_168 = (st.flags_168 & !4) | (param_1 & 4);
        out.push(W32a117Call::Mode(((param_1 & 4) | 0x2400) >> 2));
    }
    if st.f_16c != p2 || st.f_170 != p3 {
        st.f_16c = p2;
        st.f_170 = p3;
        out.push(W32a117Call::Pair(p3, p2));
    }
}

/// `0x00c5d230`: caches the packed stencil words at +0x178/+0x17c and emits
/// the func/op calls; `func_table` is at 0x00e3_eef0, `op_table` at 0x00e3_efd0.
pub fn w32a117_update_stencil(
    st: &mut W32a117State,
    p1: u32,
    p2: u32,
    func_table: &[u32],
    op_table: &[u32],
    out: &mut Vec<W32a117Call>,
) {
    if st.w_178 == p1 && st.w_17c == p2 {
        return;
    }
    st.w_17c = p2;
    st.w_178 = p1;
    let r = st.w_180;
    out.push(W32a117Call::Byte((p1 >> 8) & 0xff));
    out.push(W32a117Call::Func(
        0x404,
        func_table[((p1 >> 16) & 0xf) as usize],
        r,
        p1 & 0xff,
    ));
    out.push(W32a117Call::Func(
        0x405,
        func_table[(p2 & 0xf) as usize],
        r,
        p1 & 0xff,
    ));
    let o = |v: u32| op_table[(v & 7) as usize];
    out.push(W32a117Call::Op(
        0x404,
        o(p1 >> 20),
        o(p1 >> 23),
        o(p1 >> 26),
    ));
    out.push(W32a117Call::Op(0x405, o(p2 >> 4), o(p2 >> 7), o(p2 >> 10)));
}

#[cfg(test)]
mod tests_wave32_agent117 {
    use super::*;

    #[test]
    fn mask_diff() {
        let mut s = W32a117State {
            mask_184: 0b01,
            ..Default::default()
        };
        let mut out = vec![];
        w32a117_update_mask(&mut s, 0b10, &mut out);
        assert_eq!(out, vec![W32a117Call::Disable(0), W32a117Call::Enable]);
        out.clear();
        w32a117_update_mask(&mut s, 0b10, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn flags() {
        let mut s = W32a117State::default();
        let mut out = vec![];
        w32a117_update_flags(&mut s, 5, 1.0, 2.0, &[0, 10, 20, 30], &mut out);
        assert_eq!(
            out,
            vec![
                W32a117Call::Face(10),
                W32a117Call::Mode(0x901),
                W32a117Call::Pair(2.0, 1.0)
            ]
        );
    }

    #[test]
    fn stencil() {
        let mut s = W32a117State::default();
        let mut out = vec![];
        let t: Vec<u32> = (0..16).collect();
        w32a117_update_stencil(&mut s, 0x0012_3456, 0x21, &t, &t, &mut out);
        assert_eq!(out.len(), 5);
        assert_eq!(out[0], W32a117Call::Byte(0x34));
        w32a117_update_stencil(&mut s, 0x0012_3456, 0x21, &t, &t, &mut out);
        assert_eq!(out.len(), 5);
    }
}
