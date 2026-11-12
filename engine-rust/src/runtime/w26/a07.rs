#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent07.rs
// ===========================================================================
/// Callbacks for the unresolved callees of the dispatcher.
pub trait W26a07Hooks {
    /// FUN_0087b640 predicate.
    fn mode_flag(&mut self) -> bool;
    /// FUN_0091d620 (param 0xc, flag clear).
    fn fallback_confirm(&mut self) -> u32;
    /// FUN_00914220 (generic command).
    fn fallback_command(&mut self, cmd: i32) -> u32;
    /// Virtual slot 0x100 on the object.
    fn query_value(&mut self) -> u32;
    /// FUN_00911980 test on the queried value.
    fn value_valid(&mut self, v: u32) -> bool;
    /// FUN_00606c80 / FUN_00606ca0 predicates.
    fn busy_a(&mut self) -> bool;
    fn busy_b(&mut self) -> bool;
    /// FUN_0091d0a0 with a context word, followed by FUN_0091d760.
    fn commit(&mut self, ctx_word: u32);
    /// FUN_009140a0.
    fn busy_result(&mut self) -> u32;
}

/// `0x00919a60`: command dispatcher.  `done_flag` is the byte at +0x1335,
/// `word_4ac` / `word_86` are dword indices 0x4ac and 0x86 of the object.
pub fn w26a07_dispatch<H: W26a07Hooks>(
    h: &mut H,
    cmd: i32,
    done_flag: bool,
    word_4ac: u32,
    word_86: u32,
) -> u32 {
    if !h.mode_flag() {
        if cmd == 0xc {
            return h.fallback_confirm();
        }
        return h.fallback_command(cmd);
    }
    if cmd == 0xc {
        if done_flag {
            return 1;
        }
        let v = h.query_value();
        if !h.value_valid(v) {
            return 0;
        }
        if !h.busy_a() && !h.busy_b() {
            h.commit(word_4ac);
            return 1;
        }
        return h.busy_result();
    }
    if cmd == 0xd {
        if !done_flag {
            h.commit(word_86);
        }
        return 1;
    }
    0
}

#[cfg(test)]
mod tests_wave26_agent07 {
    use super::*;
    #[derive(Default)]
    struct T {
        mode: bool,
        valid: bool,
        busy: bool,
        commits: Vec<u32>,
    }
    impl W26a07Hooks for T {
        fn mode_flag(&mut self) -> bool {
            self.mode
        }
        fn fallback_confirm(&mut self) -> u32 {
            70
        }
        fn fallback_command(&mut self, c: i32) -> u32 {
            c as u32 + 100
        }
        fn query_value(&mut self) -> u32 {
            5
        }
        fn value_valid(&mut self, _v: u32) -> bool {
            self.valid
        }
        fn busy_a(&mut self) -> bool {
            self.busy
        }
        fn busy_b(&mut self) -> bool {
            false
        }
        fn commit(&mut self, w: u32) {
            self.commits.push(w);
        }
        fn busy_result(&mut self) -> u32 {
            9
        }
    }
    #[test]
    fn dispatch_paths() {
        let mut t = T::default();
        assert_eq!(w26a07_dispatch(&mut t, 0xc, false, 1, 2), 70);
        assert_eq!(w26a07_dispatch(&mut t, 3, false, 1, 2), 103);
        t.mode = true;
        assert_eq!(w26a07_dispatch(&mut t, 0xc, true, 1, 2), 1);
        assert_eq!(w26a07_dispatch(&mut t, 0xc, false, 1, 2), 0);
        t.valid = true;
        assert_eq!(w26a07_dispatch(&mut t, 0xc, false, 1, 2), 1);
        t.busy = true;
        assert_eq!(w26a07_dispatch(&mut t, 0xc, false, 1, 2), 9);
        assert_eq!(w26a07_dispatch(&mut t, 0xd, false, 1, 2), 1);
        assert_eq!(w26a07_dispatch(&mut t, 0xd, true, 1, 2), 1);
        assert_eq!(w26a07_dispatch(&mut t, 7, false, 1, 2), 0);
        assert_eq!(t.commits, vec![1, 2]);
    }
}
