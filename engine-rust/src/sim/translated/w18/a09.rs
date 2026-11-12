#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave18/agent09.rs
// ===========================================================================
// Wave 18 translation batch 09.  Pure helpers recovered from small
// `sim.rs`-adjacent routines; unresolved callees are closures.

/// `0x00570e20`: resets a list-header record.  When the optional limit
/// (`*(u16*)(this+0x2c)`) exists and `this.u16@0x1a` is below it the retail
/// code calls a non-returning handler: returned as `false` (record untouched).
/// Otherwise the dwords at `0x1c`, `0x20`, `0x24`, `0x28` are cleared.
pub fn w18a09_reset_list_header(this: &mut [u8], limit: Option<u16>) -> bool {
    let cur = u16::from_le_bytes([this[0x1a], this[0x1b]]);
    if let Some(l) = limit
        && cur < l
    {
        return false;
    }
    for off in [0x1cusize, 0x20, 0x24, 0x28] {
        this[off..off + 4].fill(0);
    }
    true
}

/// `0x005714b0`: looks up a script id in a table of `(id, value)` u16 pairs
/// terminated by id `0xffff`.  Returns the value stored at `this+0x10`;
/// `None` means retail's "script %d does not exist" fatal path.
pub fn w18a09_script_lookup(table: &[u16], id: u32) -> Option<u16> {
    let mut i = 0;
    loop {
        let cur = *table.get(i)?;
        if cur as u32 == id {
            return table.get(i + 1).copied();
        }
        if cur == 0xffff {
            return None;
        }
        i += 2;
    }
}

/// `0x0057b6f0`: walks the linked list headed at `this+0x20` (next pointer
/// at node+4) and invokes `f(node)` for every node.  Nodes are 1-based ids;
/// `next(node)` reads the link, `0` ends the walk.
pub fn w18a09_for_each_node_a(head: u32, next: impl Fn(u32) -> u32, mut f: impl FnMut(u32)) {
    let mut n = head;
    while n != 0 {
        f(n);
        n = next(n);
    }
}

/// `0x0057b720`: identical walk to the sibling at 57b6f0 with a different
/// per-node callee.
pub fn w18a09_for_each_node_b(head: u32, next: impl Fn(u32) -> u32, f: impl FnMut(u32)) {
    w18a09_for_each_node_a(head, next, f)
}

/// `0x005882f0`: clears the guard flag, runs the two callees with
/// `(a+1, b+1)` and `(2, 1)`, then sets the flag again.
pub fn w18a09_with_guard(
    a: i32,
    b: i32,
    flag: &mut bool,
    mut c1: impl FnMut(i32, i32),
    mut c2: impl FnMut(i32, i32),
) {
    *flag = false;
    c1(a.wrapping_add(1), b.wrapping_add(1));
    c2(2, 1);
    *flag = true;
}

/// `0x00588d30`: pointer to 1-based index (stride 16): `0` stays `0`,
/// otherwise `((p - base) >> 4) + 1` with an arithmetic shift.
pub fn w18a09_ptr_to_index(p: i32, base: i32) -> i32 {
    if p == 0 {
        0
    } else {
        (p.wrapping_sub(base) >> 4).wrapping_add(1)
    }
}

/// `0x005b4c40`: returns `(hi << 8) | lo` where `hi = (v >> 8) as u8` and
/// `lo = table[v] + byte@0x8e` (wrapping u8) with `v = u16@0x64`.
pub fn w18a09_pack_table_byte(v: u16, table: &[u8], byte8e: u8) -> u16 {
    let lo = table[v as usize].wrapping_add(byte8e);
    (((v >> 8) as u8 as u16) << 8) | lo as u16
}

/// `0x006876b0`: cell test on a map-flags array indexed by
/// `width * y + x`; true when the cell has any of `want` and none of
/// `forbid`.
pub fn w18a09_cell_flags_match(
    flags: &[u32],
    width: u16,
    x: i16,
    y: i16,
    want: u32,
    forbid: u32,
) -> bool {
    let idx = (width as i32 * y as i32 + x as i32) as usize;
    let v = flags[idx];
    (want & v) != 0 && (forbid & v) == 0
}

/// `0x00687240`: footprint-fit helper.  `size` is the (x, y) i16 pair of the
/// type (divided by 32 rounding toward zero), `flag_word` its flags.  With
/// map dims `(w, h)`: when `x,y >= 0` and the footprint fits, returns
/// `max(f490(~(flags>>1)&1), f860)`; otherwise `None` (retail fatal path).
#[allow(clippy::too_many_arguments)]
pub fn w18a09_footprint_check(
    x: i16,
    y: i16,
    size: (i16, i16),
    flag_word: u32,
    map_w: u16,
    map_h: u16,
    f860: impl FnOnce(i16, i16) -> i32,
    f490: impl FnOnce(i16, i16, u32) -> i32,
) -> Option<i32> {
    let sx = ((size.0 as i32 + ((size.0 as i32 >> 31) & 0x1f)) >> 5) as i16;
    let sy = ((size.1 as i32 + ((size.1 as i32 >> 31) & 0x1f)) >> 5) as i16;
    if x >= 0
        && y >= 0
        && sx as i32 + x as i32 <= map_w as i32
        && sy as i32 + y as i32 <= map_h as i32
    {
        let a = f860(sx, sy);
        let b = f490(sx, sy, !(flag_word >> 1) & 1);
        Some(b.max(a))
    } else {
        None
    }
}

/// Inputs/outputs of the mode-selector at `0x006f4cc0`.
pub struct W18a09Mode {
    pub pending_flag: bool,
    pub prev_mode: u32,
    pub net_active: bool,
    pub flag_10350: bool,
}

/// `0x006f4cc0`: picks a menu/game mode.  Pending flag: returns the saved
/// mode and arms `0x21`.  Else with `net_active` (or `net_count > 0`) returns
/// 6 after `reset6`.  Otherwise `alt` closure result, param `0x3f` gives
/// 0x12/0x20, else `probe` zero gives 0xc, else `default()`.
pub fn w18a09_select_mode(
    s: &mut W18a09Mode,
    net_count: i32,
    param: i32,
    alt: impl FnOnce() -> Option<u32>,
    probe_zero: impl FnOnce() -> bool,
    default: impl FnOnce() -> u32,
    reset6: impl FnOnce(u32),
) -> u32 {
    if s.pending_flag {
        let r = s.prev_mode;
        s.pending_flag = false;
        s.prev_mode = 0x21;
        return r;
    }
    if net_count < 1 && !s.net_active {
        if let Some(v) = alt() {
            return v;
        }
        if param == 0x3f {
            return if s.flag_10350 { 0x20 } else { 0x12 };
        }
        if probe_zero() {
            return 0xc;
        }
        return default();
    }
    reset6(6);
    6
}

#[cfg(test)]
mod tests_wave18_agent09 {
    use super::*;

    #[test]
    fn reset_header() {
        let mut b = [0xffu8; 0x30];
        b[0x1a] = 3;
        b[0x1b] = 0;
        assert!(!w18a09_reset_list_header(&mut b, Some(4)));
        assert_eq!(b[0x1c], 0xff);
        assert!(w18a09_reset_list_header(&mut b, Some(3)));
        assert_eq!(&b[0x1c..0x2c], &[0u8; 16]);
        assert_eq!(b[0x2c], 0xff);
    }

    #[test]
    fn script() {
        let t = [5u16, 100, 9, 200, 0xffff, 0];
        assert_eq!(w18a09_script_lookup(&t, 9), Some(200));
        assert_eq!(w18a09_script_lookup(&t, 7), None);
    }

    #[test]
    fn walk_and_guard() {
        let mut seen = vec![];
        w18a09_for_each_node_a(1, |n| if n < 3 { n + 1 } else { 0 }, |n| seen.push(n));
        assert_eq!(seen, [1, 2, 3]);
        let mut c = 0;
        w18a09_for_each_node_b(0, |_| 0, |_| c += 1);
        assert_eq!(c, 0);
        let mut f = true;
        let mut calls = vec![];
        let mut calls2 = vec![];
        w18a09_with_guard(
            1,
            2,
            &mut f,
            |a, b| calls.push((a, b)),
            |a, b| calls2.push((a, b)),
        );
        assert_eq!(calls, [(2, 3)]);
        assert_eq!(calls2, [(2, 1)]);
        assert!(f);
    }

    #[test]
    fn small() {
        assert_eq!(w18a09_ptr_to_index(0, 100), 0);
        assert_eq!(w18a09_ptr_to_index(132, 100), 3);
        assert_eq!(w18a09_ptr_to_index(68, 100), -1);
        assert_eq!(
            w18a09_pack_table_byte(
                0x0102,
                &{
                    let mut t = [0u8; 0x103];
                    t[0x102] = 0xff;
                    t
                },
                2
            ),
            0x0101
        );
        let fl = [0, 0b01, 0b11, 0];
        assert!(!w18a09_cell_flags_match(&fl, 2, 0, 1, 1, 2));
        assert!(w18a09_cell_flags_match(&fl, 2, 1, 0, 1, 2));
    }

    #[test]
    fn footprint() {
        let r = w18a09_footprint_check(
            1,
            1,
            (64, -33),
            0,
            10,
            10,
            |_, _| 5,
            |a, b, f| {
                assert_eq!((a, b, f), (2, -1, 1));
                7
            },
        );
        assert_eq!(r, Some(7));
        assert_eq!(
            w18a09_footprint_check(9, 0, (64, 0), 2, 10, 10, |_, _| 0, |_, _, _| 0),
            None
        );
        assert_eq!(
            w18a09_footprint_check(-1, 0, (0, 0), 2, 10, 10, |_, _| 0, |_, _, _| 0),
            None
        );
    }

    #[test]
    fn mode() {
        let mut s = W18a09Mode {
            pending_flag: true,
            prev_mode: 9,
            net_active: false,

            flag_10350: true,
        };
        assert_eq!(
            w18a09_select_mode(&mut s, 0, 0, || None, || false, || 1, |_| {}),
            9
        );
        assert_eq!(s.prev_mode, 0x21);
        assert_eq!(
            w18a09_select_mode(&mut s, 0, 0x3f, || None, || false, || 1, |_| {}),
            0x20
        );
        assert_eq!(
            w18a09_select_mode(&mut s, 0, 1, || None, || true, || 1, |_| {}),
            0xc
        );
        assert_eq!(
            w18a09_select_mode(&mut s, 0, 1, || None, || false, || 1, |_| {}),
            1
        );
        assert_eq!(
            w18a09_select_mode(&mut s, 1, 1, || None, || false, || 1, |_| {}),
            6
        );
    }
}
