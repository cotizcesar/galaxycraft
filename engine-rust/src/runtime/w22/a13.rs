#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent13.rs
// ===========================================================================
// Wave 22 translation batch 13.
//
// Window: queue: 20 addresses, module `runtime.rs`.
// Implemented: 0x006515b0, 0x006517b0.

/// 0x006515b0: map two coordinate pairs through `pack` (FUN_0075d720) and
/// forward `(param_1, pack(p2,p3), pack(p4,p5))` to `sink` (FUN_006515e0).
/// Note the retail order: the second pair is packed first.
pub fn w22a13_pack_pairs<R>(
    param_1: u32,
    param_2: i32,
    param_3: i32,
    param_4: i32,
    param_5: i32,
    mut pack: impl FnMut(i32, i32) -> u16,
    sink: impl FnOnce(u32, u16, u16) -> R,
) -> R {
    let b = pack(param_4, param_5);
    let a = pack(param_2, param_3);
    sink(param_1, a, b)
}

/// 0x006517b0: pack both objects via `pack` (FUN_0075d7b0) and forward with
/// the byte at offset 0x4c of the first object (`kind`) to `sink`.
pub fn w22a13_pack_objects<R>(
    first: i32,
    first_kind: u8,
    second: i32,
    mut pack: impl FnMut(i32) -> u16,
    sink: impl FnOnce(u8, u16, u16) -> R,
) -> R {
    let b = pack(second);
    let a = pack(first);
    sink(first_kind, a, b)
}

#[cfg(test)]
mod tests_wave22_agent13 {
    use super::*;

    #[test]
    fn pairs_order_and_forward() {
        let mut calls = vec![];
        let r = w22a13_pack_pairs(
            7,
            1,
            2,
            3,
            4,
            |x, y| {
                calls.push((x, y));
                (x * 10 + y) as u16
            },
            |p, a, b| (p, a, b),
        );
        assert_eq!(r, (7, 12, 34));
        assert_eq!(calls, vec![(3, 4), (1, 2)]);
    }

    #[test]
    fn objects_order_and_forward() {
        let mut calls = vec![];
        let r = w22a13_pack_objects(
            5,
            9,
            6,
            |x| {
                calls.push(x);
                x as u16
            },
            |k, a, b| (k, a, b),
        );
        assert_eq!(r, (9, 5, 6));
        assert_eq!(calls, vec![6, 5]);
    }
}
