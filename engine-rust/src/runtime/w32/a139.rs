#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent139.rs
// ===========================================================================
// Wave 32 agent 139: small CRT-style helpers.
// Ports `0x00c9e4f0`, `0x00c9e549`, `0x00c9e568`.

/// `0x00c9e4f0`: if the "owned" flag (byte at +0x14) is set, calls `release`
/// with the handle (dword at +8) and clears the flag.
pub fn w32a139_release_if_owned(handle: u32, owned: &mut bool, release: &mut dyn FnMut(u32)) {
    if *owned {
        release(handle);
        *owned = false;
    }
}

/// `0x00c9e549`: stores the allocation result in `out` and returns 0 on
/// success or 12 (ENOMEM) when the result is null.
pub fn w32a139_alloc_store(out: &mut u32, alloc: &mut dyn FnMut() -> u32) -> i32 {
    let v = alloc();
    *out = v;
    if v != 0 { 0 } else { 12 }
}

/// `0x00c9e568`: with a null argument returns `finish(0)`; otherwise runs
/// `parse` on a zeroed 6-field state, returning `u32::MAX` when it fails
/// (non-zero) or `finish(state[2])` on success; the state's owned buffer
/// (field 2) is released via `release` when its flag is set.
pub fn w32a139_parse_finish(
    arg: u32,
    parse: &mut dyn FnMut(u32, &mut [u32; 5], &mut bool) -> i32,
    finish: &mut dyn FnMut(u32) -> u32,
    release: &mut dyn FnMut(u32),
) -> u32 {
    if arg == 0 {
        return finish(0);
    }
    let mut st = [0u32; 5];
    let mut flag = false;
    let r = if parse(arg, &mut st, &mut flag) == 0 {
        finish(st[2])
    } else {
        u32::MAX
    };
    if flag {
        release(st[2]);
    }
    r
}

#[cfg(test)]
mod tests_wave32_agent139 {
    use super::*;

    #[test]
    fn release() {
        let mut n = vec![];
        let mut o = true;
        w32a139_release_if_owned(7, &mut o, &mut |h| n.push(h));
        w32a139_release_if_owned(7, &mut o, &mut |h| n.push(h));
        assert_eq!(n, vec![7]);
        assert!(!o);
    }

    #[test]
    fn alloc() {
        let mut o = 1;
        assert_eq!(w32a139_alloc_store(&mut o, &mut || 0), 12);
        assert_eq!(o, 0);
        assert_eq!(w32a139_alloc_store(&mut o, &mut || 5), 0);
        assert_eq!(o, 5);
    }

    #[test]
    fn parse() {
        let mut rel = vec![];
        let r = w32a139_parse_finish(
            1,
            &mut |_, s, f| {
                s[2] = 9;
                *f = true;
                0
            },
            &mut |x| x + 1,
            &mut |x| rel.push(x),
        );
        assert_eq!(r, 10);
        assert_eq!(rel, vec![9]);
        assert_eq!(
            w32a139_parse_finish(0, &mut |_, _, _| 1, &mut |x| x + 3, &mut |_| {}),
            3
        );
        assert_eq!(
            w32a139_parse_finish(1, &mut |_, _, _| 1, &mut |x| x, &mut |_| {}),
            u32::MAX
        );
    }
}
