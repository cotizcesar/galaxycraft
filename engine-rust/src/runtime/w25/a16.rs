#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent16.rs
// ===========================================================================
// Wave 25 agent 16: runtime helpers (most of the queue is SEH/unwind glue).

/// `0x007f12c0`: returns 1 when the object is "in use": word 0, word 2 or
/// word 3 is non-zero, else 0.
pub fn w25a16_is_in_use(words: &[u32]) -> u32 {
    if words[0] == 0 && (words[2] == 0 && words[3] == 0) {
        0
    } else {
        1
    }
}

/// `0x007f0ed0`: if word 0 is set, clears it, stores the running generation
/// counter in word 0x92 and increments the counter.  Returns true when
/// the reset happened.
pub fn w25a16_release_generation(words: &mut [u32], counter: &mut u32) -> bool {
    if words[0] != 0 {
        words[0] = 0;
        words[0x92] = *counter;
        *counter = counter.wrapping_add(1);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests_wave25_agent16 {
    use super::*;

    #[test]
    fn in_use_predicate() {
        assert_eq!(w25a16_is_in_use(&[0, 9, 0, 0]), 0);
        assert_eq!(w25a16_is_in_use(&[1, 0, 0, 0]), 1);
        assert_eq!(w25a16_is_in_use(&[0, 0, 1, 0]), 1);
        assert_eq!(w25a16_is_in_use(&[0, 0, 0, 1]), 1);
    }

    #[test]
    fn release_generation() {
        let mut w = vec![0u32; 0x93];
        let mut c = 7;
        assert!(!w25a16_release_generation(&mut w, &mut c));
        assert_eq!(c, 7);
        w[0] = 1;
        assert!(w25a16_release_generation(&mut w, &mut c));
        assert_eq!((w[0], w[0x92], c), (0, 7, 8));
    }
}
