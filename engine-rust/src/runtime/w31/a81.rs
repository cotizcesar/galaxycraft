#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent81.rs
// ===========================================================================
// Wave 31 agent 81 translations (runtime helpers).

/// Deadline tracker used by `0x00b65bd0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W31a81Deadline {
    /// Current 64-bit value (words 0 and 1).
    pub current: i64,
    /// Armed flag (word 2).
    pub armed: u32,
    /// Last reset stamp (words 4 and 5).
    pub last: i64,
}

/// `0x00b65bd0`: compares `now` against the tracker.  When armed and
/// `now - last >= interval` the tracker is disarmed and re-based on `now`
/// and the result is false; otherwise returns `now < current` (signed 64).
pub fn w31a81_deadline_check(t: &mut W31a81Deadline, interval: i64, now: i64) -> bool {
    if t.armed != 0 && now.wrapping_sub(t.last) >= interval {
        t.armed = 0;
        t.last = now;
        t.current = now;
        return false;
    }
    now < t.current
}

/// `0x00b64d40`: new capacity word for a vector growth request.  The low 31
/// bits hold the capacity, the top bit is preserved.  Returns `None` when
/// no growth is needed.
pub fn w31a81_grow_capacity_word(word: u32, wanted: u32) -> Option<u32> {
    let cap = word & 0x7fff_ffff;
    if cap >= wanted {
        return None;
    }
    let mut new_cap = ((word >> 1) & 0x3fff_ffff).wrapping_add(cap);
    if new_cap < wanted {
        new_cap = wanted;
    }
    Some(((word ^ new_cap) & 0x7fff_ffff) ^ word)
}

#[cfg(test)]
mod tests_wave31_agent81 {
    use super::*;

    #[test]
    fn deadline() {
        let mut t = W31a81Deadline {
            current: 100,
            armed: 1,
            last: 0,
        };
        assert!(w31a81_deadline_check(&mut t, 50, 40) && t.armed == 1);
        assert!(!w31a81_deadline_check(&mut t, 50, 60));
        assert_eq!((t.armed, t.last, t.current), (0, 60, 60));
        assert!(w31a81_deadline_check(&mut t, 50, 10));
    }

    #[test]
    fn grow() {
        assert_eq!(w31a81_grow_capacity_word(10, 5), None);
        assert_eq!(w31a81_grow_capacity_word(10, 11), Some(15));
        assert_eq!(w31a81_grow_capacity_word(10, 40), Some(40));
        assert_eq!(w31a81_grow_capacity_word(0x8000_0004, 9), Some(0x8000_0009));
    }
}
