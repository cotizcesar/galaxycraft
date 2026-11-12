#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent03.rs
// ===========================================================================
// Wave 31 translation batch 03: JSON value parser dispatch/array/object
// (client-side config parser) and two small helpers.

/// Parser state shared by the JSON routines (`param_1` pair plus the error
/// slots at `+0x18`/`+0x1c` of the reader object).
#[derive(Debug, Clone, Default)]
pub struct W31a03Parser {
    pub buf: Vec<u8>,
    pub pos: usize,
    pub begin: usize,
    pub err: u32,
    pub err_off: usize,
}

impl W31a03Parser {
    fn peek(&self) -> u8 {
        self.buf.get(self.pos).copied().unwrap_or(0)
    }
    fn fail(&mut self, code: u32) {
        self.err = code;
        self.err_off = self.pos.wrapping_sub(self.begin);
    }
}

/// Callbacks for the unrecovered callees of the value routines.
pub trait W31a03Hooks {
    /// string parse (FUN_0061d100); `is_key` is the third argument.
    fn string(&mut self, p: &mut W31a03Parser, st: &mut Vec<[u32; 4]>, is_key: bool);
    /// number parse (FUN_0061c5b0, the default branch).
    fn number(&mut self, p: &mut W31a03Parser, st: &mut Vec<[u32; 4]>);
    /// separator skip (FUN_0061d600).
    fn skip(&mut self, p: &mut W31a03Parser);
    /// bool commit (func_0x61_d970); true means handled.
    fn commit_bool(&mut self, v: bool) -> bool;
    /// array commit (FUN_0061d9b0).
    fn commit_array(&mut self, n: u32) -> bool;
    /// object commit (FUN_0061da10).
    fn commit_object(&mut self, n: u32) -> bool;
}

fn w31a03_skip_ws(p: &mut W31a03Parser) {
    while matches!(p.peek(), b' ' | b'\n' | b'\r' | b'\t') && p.pos < p.buf.len() {
        p.pos += 1;
    }
}

fn w31a03_push_container(st: &mut Vec<[u32; 4]>, kind: u32) {
    st.push([0, 0, 0, kind << 16]);
}

fn w31a03_reset_top(st: &mut [[u32; 4]], kind: u32) {
    if let Some(t) = st.last_mut() {
        t[0] = 0;
        t[1] = 0;
        t[2] = 0;
        t[3] = (t[3] & 0xffff) | (kind << 16);
    }
}

/// `0x0061c3f0`: parses a JSON array body after `[`; pushes a 16-byte
/// value tagged 4, error codes 7 (missing `]`) and 0x10 (commit refused).
pub fn w31a03_parse_array<H: W31a03Hooks>(p: &mut W31a03Parser, st: &mut Vec<[u32; 4]>, h: &mut H) {
    p.pos += 1;
    w31a03_push_container(st, 4);
    w31a03_skip_ws(p);
    if p.err != 0 {
        return;
    }
    if p.peek() == b']' {
        p.pos += 1;
        w31a03_reset_top(st, 4);
        return;
    }
    let mut count = 0u32;
    w31a03_parse_value(p, st, h);
    if p.err != 0 {
        return;
    }
    loop {
        count += 1;
        w31a03_skip_ws(p);
        if p.err != 0 {
            return;
        }
        if p.peek() != b',' {
            break;
        }
        p.pos += 1;
        h.skip(p);
        if p.err != 0 {
            return;
        }
        w31a03_parse_value(p, st, h);
        if p.err != 0 {
            return;
        }
    }
    if p.peek() == b']' {
        p.pos += 1;
        if !h.commit_array(count) {
            p.fail(0x10);
        }
    } else {
        p.fail(7);
    }
}

/// `0x0061cdb0`: parses a JSON object body after `{`; pushes a value tagged
/// 3; error codes 4 (key not a string), 5 (missing `:`), 6 (missing `}`),
/// 0x10 (commit refused).
pub fn w31a03_parse_object<H: W31a03Hooks>(
    p: &mut W31a03Parser,
    st: &mut Vec<[u32; 4]>,
    h: &mut H,
) {
    p.pos += 1;
    w31a03_push_container(st, 3);
    w31a03_skip_ws(p);
    if p.err != 0 {
        return;
    }
    if p.peek() == b'}' {
        p.pos += 1;
        w31a03_reset_top(st, 3);
        return;
    }
    let mut count = 0u32;
    loop {
        if p.peek() != b'"' {
            p.fail(4);
            return;
        }
        h.string(p, st, true);
        if p.err != 0 {
            return;
        }
        w31a03_skip_ws(p);
        if p.err != 0 {
            return;
        }
        if p.peek() != b':' {
            p.fail(5);
            return;
        }
        p.pos += 1;
        w31a03_skip_ws(p);
        if p.err != 0 {
            return;
        }
        w31a03_parse_value(p, st, h);
        if p.err != 0 {
            return;
        }
        h.skip(p);
        if p.err != 0 {
            return;
        }
        count += 1;
        if p.peek() != b',' {
            break;
        }
        p.pos += 1;
        h.skip(p);
        if p.err != 0 {
            return;
        }
    }
    if p.peek() != b'}' {
        p.fail(6);
        return;
    }
    p.pos += 1;
    if !h.commit_object(count) {
        p.fail(0x10);
    }
}

/// `0x0061d450`: JSON value dispatch on the first byte (`"`, `[`, `{`,
/// `null`, `true`, `false`, otherwise number).  Bad literals set error 3.
pub fn w31a03_parse_value<H: W31a03Hooks>(p: &mut W31a03Parser, st: &mut Vec<[u32; 4]>, h: &mut H) {
    let start = p.pos;
    match p.peek() {
        b'"' => h.string(p, st, false),
        b'[' => w31a03_parse_array(p, st, h),
        b'{' => w31a03_parse_object(p, st, h),
        c @ (b'f' | b'n' | b't') => {
            let word: &[u8] = match c {
                b'f' => b"false",
                b'n' => b"null",
                _ => b"true",
            };
            // The decompiled code advances one byte at a time and stops at
            // the first mismatch, leaving pos on the offending byte.
            for i in 1..word.len() {
                p.pos = start + i;
                if p.peek() != word[i] {
                    p.fail(3);
                    return;
                }
            }
            p.pos = start + word.len();
            match c {
                b'n' => {
                    if st.capacity() == st.len() {
                        st.reserve(1);
                    }
                    st.push([0; 4]);
                }
                _ => {
                    if !h.commit_bool(c == b't') {
                        p.fail(0x10);
                    }
                }
            }
        }
        _ => h.number(p, st),
    }
}

/// `0x0060aa90`: intern lookup.  When the bucket table exists and `find`
/// returns a node, its id (`+0x20`) is returned; otherwise the counter is
/// incremented, `grow` is called (FUN_0060aaf0) and the new count returned.
pub fn w31a03_intern(
    count: &mut u32,
    has_buckets: bool,
    find: impl FnOnce() -> Option<u32>,
    grow: impl FnOnce(),
) -> u32 {
    if has_buckets {
        if let Some(id) = find() {
            return id;
        }
    }
    *count += 1;
    grow();
    *count
}

/// `0x00635990`: true when byte 0 of the `0x4e8`-byte slot `index` is zero.
pub fn w31a03_slot_free(slots: &[u8], index: usize) -> bool {
    slots.get(index * 0x4e8).is_some_and(|b| *b == 0)
}

#[cfg(test)]
mod tests_wave31_agent03 {
    use super::*;

    #[derive(Default)]
    struct H {
        arr: Vec<u32>,
        obj: Vec<u32>,
        bools: Vec<bool>,
        refuse: bool,
    }
    impl W31a03Hooks for H {
        fn string(&mut self, p: &mut W31a03Parser, st: &mut Vec<[u32; 4]>, _k: bool) {
            p.pos += 1;
            while p.peek() != b'"' {
                p.pos += 1;
            }
            p.pos += 1;
            st.push([0, 0, 0, 5 << 16]);
        }
        fn number(&mut self, p: &mut W31a03Parser, st: &mut Vec<[u32; 4]>) {
            if !p.peek().is_ascii_digit() {
                p.fail(3);
                return;
            }
            while p.peek().is_ascii_digit() {
                p.pos += 1;
            }
            st.push([1, 0, 0, 6 << 16]);
        }
        fn skip(&mut self, p: &mut W31a03Parser) {
            w31a03_skip_ws(p);
        }
        fn commit_bool(&mut self, v: bool) -> bool {
            self.bools.push(v);
            !self.refuse
        }
        fn commit_array(&mut self, n: u32) -> bool {
            self.arr.push(n);
            !self.refuse
        }
        fn commit_object(&mut self, n: u32) -> bool {
            self.obj.push(n);
            !self.refuse
        }
    }

    fn run(s: &str, h: &mut H) -> (W31a03Parser, Vec<[u32; 4]>) {
        let mut buf = s.as_bytes().to_vec();
        buf.push(0);
        let mut p = W31a03Parser {
            buf,
            ..Default::default()
        };
        let mut st = Vec::new();
        w31a03_parse_value(&mut p, &mut st, h);
        (p, st)
    }

    #[test]
    fn array_counts() {
        let mut h = H::default();
        let (p, st) = run("[1, 22 ,3]", &mut h);
        assert_eq!(p.err, 0);
        assert_eq!(h.arr, vec![3]);
        assert_eq!(st[0][3] >> 16, 4);
        let (p, _) = run("[ ]", &mut h);
        assert_eq!(p.err, 0);
        assert_eq!(h.arr, vec![3]);
        let (p, _) = run("[1 2]", &mut h);
        assert_eq!((p.err, p.err_off), (7, 3));
    }

    #[test]
    fn object_errors_and_count() {
        let mut h = H::default();
        let (p, _) = run("{\"a\": 1, \"b\": [2]}", &mut h);
        assert_eq!(p.err, 0);
        assert_eq!(h.obj, vec![2]);
        assert_eq!(run("{1:2}", &mut h).0.err, 4);
        assert_eq!(run("{\"a\" 2}", &mut h).0.err, 5);
        assert_eq!(run("{\"a\":2 3}", &mut h).0.err, 6);
        assert_eq!(run("{}", &mut h).0.err, 0);
    }

    #[test]
    fn literals() {
        let mut h = H::default();
        let (p, st) = run("null", &mut h);
        assert_eq!((p.err, p.pos, st.len()), (0, 4, 1));
        run("true", &mut h);
        run("false", &mut h);
        assert_eq!(h.bools, vec![true, false]);
        let (p, _) = run("trux", &mut h);
        assert_eq!((p.err, p.err_off), (3, 3));
        h.refuse = true;
        let (p, _) = run("true", &mut h);
        assert_eq!((p.err, p.err_off), (0x10, 4));
    }

    #[test]
    fn intern_and_slot() {
        let mut n = 4;
        assert_eq!(w31a03_intern(&mut n, true, || Some(9), || panic!()), 9);
        assert_eq!(n, 4);
        let mut grown = false;
        assert_eq!(w31a03_intern(&mut n, true, || None, || grown = true), 5);
        assert!(grown);
        assert_eq!(w31a03_intern(&mut n, false, || Some(1), || {}), 6);
        let mut s = vec![1u8; 0x4e8 * 2];
        s[0x4e8] = 0;
        assert!(!w31a03_slot_free(&s, 0));
        assert!(w31a03_slot_free(&s, 1));
        assert!(!w31a03_slot_free(&s, 2));
    }
}
