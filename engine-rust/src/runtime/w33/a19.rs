#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent19.rs
// ===========================================================================
// Wave 33 agent 19: small dialog-command dispatch helpers.

/// `0x0090b5d0`: when `command == 3` invokes virtual slot 0x78 with 0x3eb
/// and returns 1; otherwise defers to the base handler (`fallback`).
pub fn w33a19_dispatch_cmd3(
    command: i32,
    mut virt78: impl FnMut(u32),
    fallback: impl FnOnce() -> u32,
) -> u32 {
    if command == 3 {
        virt78(0x3eb);
        return 1;
    }
    fallback()
}

/// `0x0090bbc0`: when `command == 2` invokes virtual slot 0x78 with 0x3ea
/// and returns 1; otherwise defers to the base handler (`fallback`).
pub fn w33a19_dispatch_cmd2(
    command: i32,
    mut virt78: impl FnMut(u32),
    fallback: impl FnOnce() -> u32,
) -> u32 {
    if command != 2 {
        return fallback();
    }
    virt78(0x3ea);
    1
}

/// `0x0093ec50`: forwards both arguments to the 4-argument worker with the
/// last two arguments fixed at zero.
pub fn w33a19_forward_zero_tail<R>(
    a: u32,
    b: u32,
    worker: impl FnOnce(u32, u32, u32, u32) -> R,
) -> R {
    worker(a, b, 0, 0)
}

#[cfg(test)]
mod tests_wave33_agent19 {
    use super::*;

    #[test]
    fn cmd3() {
        let mut got = vec![];
        assert_eq!(w33a19_dispatch_cmd3(3, |v| got.push(v), || 9), 1);
        assert_eq!(got, [0x3eb]);
        assert_eq!(w33a19_dispatch_cmd3(2, |v| got.push(v), || 9), 9);
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn cmd2() {
        let mut got = vec![];
        assert_eq!(w33a19_dispatch_cmd2(2, |v| got.push(v), || 9), 1);
        assert_eq!(got, [0x3ea]);
        assert_eq!(w33a19_dispatch_cmd2(3, |v| got.push(v), || 9), 9);
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn fwd() {
        assert_eq!(
            w33a19_forward_zero_tail(1, 2, |a, b, c, d| [a, b, c, d]),
            [1, 2, 0, 0]
        );
    }
}
