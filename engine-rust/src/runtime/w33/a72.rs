#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent72.rs
// ===========================================================================
// Wave 33 agent 72.

/// `0x00cba6e8`: UTF-16 code unit to code point state machine (c16rtomb
/// style).  `state` is the two-word shift state (`param_3`, defaulting to
/// the static state by the caller).  `convert(ch, state)` models the callee
/// FUN_00cb7e22 and `error(state)` models FUN_00cb7e03 (the invalid-sequence
/// path).  A high surrogate is stashed as `((c & 0x27ff) + 0x40) * 0x400`
/// and returns 0; a low surrogate completes it.
pub fn w33a72_c16_step<C, E>(unit: u32, state: &mut [u32; 2], mut convert: C, mut error: E) -> u32
where
    C: FnMut(u32, &mut [u32; 2]) -> u32,
    E: FnMut(&mut [u32; 2]) -> u32,
{
    let low = (unit as u16).wrapping_add(0x2400);
    if state[0] == 0 {
        if low >= 0x400 {
            if (unit as u16).wrapping_add(0x2800) >= 0x400 {
                return convert(unit & 0xffff, state);
            }
            state[0] = ((unit & 0x27ff) + 0x40) * 0x400;
            return 0;
        }
    } else if low < 0x400 {
        let mut local = [0u32; 2];
        let r = convert((unit & 0x23ff).wrapping_add(state[0]), &mut local);
        state[0] = 0;
        state[1] = 0;
        return r;
    }
    error(state)
}

#[cfg(test)]
mod tests_wave33_agent72 {
    use super::*;

    #[test]
    fn surrogate_pair() {
        let mut st = [0u32; 2];
        let cv = |c: u32, _: &mut [u32; 2]| c;
        let er = |_: &mut [u32; 2]| 0xffff_ffff;
        assert_eq!(w33a72_c16_step(0xd83d, &mut st, cv, er), 0);
        assert_eq!(w33a72_c16_step(0xde00, &mut st, cv, er), 0x1f600);
        assert_eq!(st, [0, 0]);
        assert_eq!(w33a72_c16_step(0x41, &mut st, cv, er), 0x41);
        assert_eq!(w33a72_c16_step(0xde00, &mut st, cv, er), 0xffff_ffff);
    }
}
