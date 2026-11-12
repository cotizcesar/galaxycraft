#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent39.rs
// ===========================================================================
/// `0x007de530`: calls `action` only when `index` is below the container
/// size stored at offset 8 of the receiver.  Returns whether it ran.
pub fn w31a39_call_if_in_range(index: u32, size: u32, action: impl FnOnce()) -> bool {
    if index < size {
        action();
        true
    } else {
        false
    }
}

/// `0x007e3a90`: walks a list and invokes `on_match` for every node whose
/// key (virtual slot 1 of the node object) equals `wanted`.  Returns the
/// number of matches.
pub fn w31a39_for_each_matching<T>(
    nodes: &[T],
    wanted: i32,
    key: impl Fn(&T) -> i32,
    mut on_match: impl FnMut(&T),
) -> usize {
    let mut n = 0;
    for node in nodes {
        if key(node) == wanted {
            on_match(node);
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests_wave31_agent39 {
    use super::*;

    #[test]
    fn range_call() {
        let mut hit = 0;
        assert!(w31a39_call_if_in_range(2, 3, || hit += 1));
        assert!(!w31a39_call_if_in_range(3, 3, || hit += 1));
        assert_eq!(hit, 1);
    }

    #[test]
    fn matching() {
        let mut seen = vec![];
        let n = w31a39_for_each_matching(&[1, 2, 1], 1, |x| *x, |x| seen.push(*x));
        assert_eq!(n, 2);
        assert_eq!(seen, vec![1, 1]);
    }
}
