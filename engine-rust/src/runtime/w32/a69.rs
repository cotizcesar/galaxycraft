#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent69.rs
// ===========================================================================
// Wave 32 agent 69.

/// Node fields touched by `0x009814d0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W32a69Node {
    pub a: i16,
    pub b: i16,
    pub last_adjust: i16,
}

/// `0x009814d0`: walks the node list; removes the previous adjustment from
/// both fields, then re-applies a new one: 0 when `is_flagged(i)`, else
/// `delta(i)` halved (truncating toward zero) unless `is_full(i)`.
pub fn w32a69_update_nodes(
    nodes: &mut [W32a69Node],
    is_flagged: impl Fn(usize) -> bool,
    is_full: impl Fn(usize) -> bool,
    delta: impl Fn(usize) -> i32,
) {
    for (i, n) in nodes.iter_mut().enumerate() {
        n.a = n.a.wrapping_sub(n.last_adjust);
        n.b = n.b.wrapping_sub(n.last_adjust);
        let v = if is_flagged(i) {
            0
        } else {
            let full = is_full(i);
            let d = delta(i);
            if full { d } else { d / 2 }
        };
        let s = v as i16;
        n.a = n.a.wrapping_add(s);
        n.b = n.b.wrapping_add(s);
        n.last_adjust = s;
    }
}

#[cfg(test)]
mod tests_wave32_agent69 {
    use super::*;
    #[test]
    fn adjusts() {
        let mut n = [W32a69Node {
            a: 10,
            b: 20,
            last_adjust: 2,
        }];
        w32a69_update_nodes(&mut n, |_| false, |_| false, |_| 7);
        assert_eq!(
            n[0],
            W32a69Node {
                a: 11,
                b: 21,
                last_adjust: 3
            }
        );
        w32a69_update_nodes(&mut n, |_| true, |_| false, |_| 7);
        assert_eq!(
            n[0],
            W32a69Node {
                a: 8,
                b: 18,
                last_adjust: 0
            }
        );
    }
}
