#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent04.rs
// ===========================================================================
// Wave 22 agent 04 (queue, merge target `client.rs`).

/// `0x00657250`: clears the global word at 0x00fd7b98 (returns the new value).
pub fn w22a04_clear_flag(flag: &mut u32) {
    *flag = 0;
}

/// Outcome of `0x00658030`.
#[derive(Debug, PartialEq, Eq)]
pub enum W22a04Dispatch {
    /// Entry `+0x134` was non-zero: the retail path calls a non-returning helper.
    NoReturn,
    /// Player slot >= 8: nothing happens.
    Ignored,
    /// Called the sink with `(table[player] + index * 0x34, 1)`.
    Called,
}

/// `0x00658030`: if `entry[0x134] != 0` the retail code diverges; else when the
/// byte at `+0x4c` is < 8 it picks `tables[player]`, computes a 16-bit index
/// via `index_fn(a, b)` and calls `sink(base + (idx & 0xffff) * 0x34, 1)`.
pub fn w22a04_dispatch_slot(
    entry: &[u8],
    tables: &[u32; 8],
    a: u32,
    b: u32,
    index_fn: impl Fn(u32, u32) -> u32,
    mut sink: impl FnMut(u32, u32),
) -> W22a04Dispatch {
    let word = |o: usize| u32::from_le_bytes([entry[o], entry[o + 1], entry[o + 2], entry[o + 3]]);
    if word(0x134) != 0 {
        return W22a04Dispatch::NoReturn;
    }
    let player = entry[0x4c] as usize;
    if player >= 8 {
        return W22a04Dispatch::Ignored;
    }
    let idx = index_fn(a, b) & 0xffff;
    sink(tables[player].wrapping_add(idx.wrapping_mul(0x34)), 1);
    W22a04Dispatch::Called
}

/// Base virtual address of the modelled window for `0x00659590`.
pub const W22A04_POOL_BASE: u32 = 0x00fd_7b80;
/// Size in bytes of the window (through the last write at 0x00fdf8e4+4).
pub const W22A04_POOL_LEN: usize = 0x00fd_f8e8 - 0x00fd_7b80;

fn w22a04_put32(mem: &mut [u8], addr: u32, v: u32) {
    let o = (addr - W22A04_POOL_BASE) as usize;
    mem[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

fn w22a04_put16(mem: &mut [u8], addr: u32, v: u16) {
    let o = (addr - W22A04_POOL_BASE) as usize;
    mem[o..o + 2].copy_from_slice(&v.to_le_bytes());
}

/// `0x00659590`: initialises the 0x20-byte-stride pool at 0x00fd7ba0 (entries
/// 0x20..0x7ce0) and the list-head block at 0x00fdf880..0x00fdf8e8.  `mem` is
/// a window of `W22A04_POOL_LEN` bytes starting at `W22A04_POOL_BASE`.
/// Sizes of the 0x00fdf8ac.. zero stores are assumed to be 4 bytes.
pub fn w22a04_init_pool(mem: &mut [u8]) {
    let b = W22A04_POOL_BASE;
    for (a, v) in [
        (0xfd7ba4u32, 0u32),
        (0xfd7bb0, 0),
        (0xfd7bb4, 0),
        (0xfd7bb8, 0),
        (0xfd7bbc, 0),
        (0xfd7ba0, 0xfd7bc0),
        (0xfd7bac, 0),
    ] {
        w22a04_put32(mem, a, v);
    }
    mem[(0xfd7ba8 - b) as usize] = 0;
    let mut off = 0x20u32;
    while off < 0x7ce0 {
        w22a04_put32(mem, 0xfd7ba4 + off, 0xfd7b80 + off);
        w22a04_put32(mem, 0xfd7ba0 + off, 0xfd7bc0 + off);
        mem[(0xfd7ba8 + off - b) as usize] = 0;
        w22a04_put32(mem, 0xfd7bac + off, 0);
        w22a04_put32(mem, 0xfd7bb0 + off, 0);
        w22a04_put32(mem, 0xfd7bb4 + off, 0);
        w22a04_put16(mem, 0xfd7bb8 + off, 0);
        w22a04_put32(mem, 0xfd7bbc + off, 0);
        off += 0x20;
    }
    w22a04_put32(mem, 0xfdf884, 0xfdf860);
    w22a04_put32(mem, 0xfdf880, 0);
    mem[(0xfdf888 - b) as usize] = 0;
    for a in [0xfdf88cu32, 0xfdf890, 0xfdf894, 0xfdf898, 0xfdf89c] {
        w22a04_put32(mem, a, 0);
    }
    w22a04_put32(mem, 0xfdf8a0, 0xfd7ba0);
    w22a04_put32(mem, 0xfdf8a8, 0xfd7ba0);
    for a in (0xfdf8acu32..=0xfdf8e4).step_by(8) {
        w22a04_put32(mem, a, 0);
    }
    for a in [
        0xfdf8b0u32,
        0xfdf8b8,
        0xfdf8c0,
        0xfdf8c8,
        0xfdf8d0,
        0xfdf8d8,
        0xfdf8e0,
    ] {
        w22a04_put32(mem, a, 0xfd7ba0);
    }
}

#[cfg(test)]
mod tests_wave22_agent04 {
    use super::*;

    fn rd(mem: &[u8], a: u32) -> u32 {
        let o = (a - W22A04_POOL_BASE) as usize;
        u32::from_le_bytes([mem[o], mem[o + 1], mem[o + 2], mem[o + 3]])
    }

    #[test]
    fn clear_flag() {
        let mut f = 7;
        w22a04_clear_flag(&mut f);
        assert_eq!(f, 0);
    }

    #[test]
    fn dispatch() {
        let tables = [0x1000u32; 8];
        let mut e = vec![0u8; 0x140];
        e[0x4c] = 3;
        let mut got = None;
        let r = w22a04_dispatch_slot(
            &e,
            &tables,
            1,
            2,
            |_, _| 0x1_0002,
            |p, f| got = Some((p, f)),
        );
        assert_eq!(r, W22a04Dispatch::Called);
        assert_eq!(got, Some((0x1000 + 2 * 0x34, 1)));
        e[0x4c] = 8;
        assert_eq!(
            w22a04_dispatch_slot(&e, &tables, 0, 0, |_, _| 0, |_, _| {}),
            W22a04Dispatch::Ignored
        );
        e[0x134] = 1;
        assert_eq!(
            w22a04_dispatch_slot(&e, &tables, 0, 0, |_, _| 0, |_, _| {}),
            W22a04Dispatch::NoReturn
        );
    }

    #[test]
    fn pool_init() {
        let mut m = vec![0xffu8; W22A04_POOL_LEN];
        w22a04_init_pool(&mut m);
        assert_eq!(rd(&m, 0xfd7ba0), 0xfd7bc0);
        assert_eq!(rd(&m, 0xfd7ba4 + 0x20), 0xfd7b80 + 0x20);
        assert_eq!(rd(&m, 0xfd7ba0 + 0x20), 0xfd7bc0 + 0x20);
        assert_eq!(rd(&m, 0xfd7ba4 + 0x7cc0), 0xfd7b80 + 0x7cc0);
        assert_eq!(rd(&m, 0xfdf884), 0xfdf860);
        assert_eq!(rd(&m, 0xfdf8a8), 0xfd7ba0);
        assert_eq!(rd(&m, 0xfdf8e0), 0xfd7ba0);
        assert_eq!(rd(&m, 0xfdf8e4), 0);
    }
}
