#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave30/agent15.rs
// ===========================================================================
// Wave 30 agent 15: argument-list integer parsing, hash-bucket gate,
// capacity growth and the RC4-style key schedule.

/// `0x00b29160`: parses argument `idx` as a signed 32-bit decimal.  An
/// optional leading `-` is skipped; `parse` models the unresolved
/// string-to-u64 helper and returns `(value, consumed_bytes)`.  The whole
/// string must be consumed; positive values must be below `0x8000_0000`,
/// negative magnitudes at most `0x8000_0000`.
pub fn w30a15_arg_to_i32(
    args: &[&str],
    idx: usize,
    parse: impl Fn(&str) -> (u64, usize),
) -> Option<i32> {
    let s = *args.get(idx)?;
    let (neg, body) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s),
    };
    let (v, n) = parse(body);
    if n != body.len() || (v >> 32) != 0 {
        return None;
    }
    let v = v as u32;
    if neg {
        (v <= 0x8000_0000).then(|| v.wrapping_neg() as i32)
    } else {
        (v < 0x8000_0000).then_some(v as i32)
    }
}

/// `0x00b29210`: parses argument `idx` as an unsigned 32-bit value; the
/// whole string must be consumed and the high half of the result be zero.
pub fn w30a15_arg_to_u32(
    args: &[&str],
    idx: usize,
    parse: impl Fn(&str) -> (u64, usize),
) -> Option<u32> {
    let s = *args.get(idx)?;
    let (v, n) = parse(s);
    (n == s.len() && (v >> 32) == 0).then_some(v as u32)
}

/// Table gate state used by `0x00b2b5e0`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct W30a15Gate {
    /// Bucket count (power of two); mask is `size - 1`.
    pub size: u32,
    /// Expected masked hash (field at +0x50).
    pub expect: u32,
    /// Visit counter (field at +0x54).
    pub visits: u32,
}

/// `0x00b2b5e0`: when the record length byte is above 4 and
/// `hash_byte5 & (size-1)` equals the expected slot, runs `insert`; if it
/// reports true the notify step is requested.  The visit counter is
/// incremented whenever the gate passes.  Returns `Some(notify)` on pass.
pub fn w30a15_gate_visit(
    g: &mut W30a15Gate,
    rec: &[u8],
    insert: impl FnOnce(&[u8]) -> bool,
) -> Option<bool> {
    if rec.len() < 6 || rec[0] <= 4 {
        return None;
    }
    if (rec[5] as u32 & g.size.wrapping_sub(1)) != g.expect {
        return None;
    }
    let notify = insert(rec);
    g.visits = g.visits.wrapping_add(1);
    Some(notify)
}

/// `0x00b2bc30`: grows a string capacity word (low 31 bits capacity, top
/// bit flag).  When `want` exceeds the capacity the new capacity is
/// `cap + cap/2` (at least `want`), the flag bit is preserved.  Returns
/// the new word when growth is needed.
pub fn w30a15_grow_capacity(word: u32, want: u32) -> Option<u32> {
    let cap = word & 0x7fff_ffff;
    if cap >= want {
        return None;
    }
    let mut new = ((word >> 1) & 0x3fff_ffff) + cap;
    if new < want {
        new = want;
    }
    Some((word & 0x8000_0000) | (new & 0x7fff_ffff))
}

/// RC4-style cipher state used by `0x00b2c6f0`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W30a15Rc4 {
    pub s: [u8; 256],
    pub i: u8,
    pub j: u8,
}

/// `0x00b2c6f0`: builds the 28-byte seed (`b` big-endian, `a`
/// big-endian, `extra` little-endian, 16 bytes of `ctx`), digests it to 16
/// bytes with `digest` (unresolved helper), runs the RC4 key schedule and
/// discards `0x300` keystream bytes.  `j` carries over from `state`.
pub fn w30a15_key_schedule(
    state: &mut W30a15Rc4,
    a: u32,
    b: u32,
    extra: u32,
    ctx: &[u8; 16],
    digest: impl Fn(&[u8; 28]) -> [u8; 16],
) {
    let mut seed = [0u8; 28];
    seed[0..4].copy_from_slice(&b.to_be_bytes());
    seed[4..8].copy_from_slice(&a.to_be_bytes());
    seed[8..12].copy_from_slice(&extra.to_le_bytes());
    seed[12..28].copy_from_slice(ctx);
    let key = digest(&seed);
    state.i = 0;
    for (k, v) in state.s.iter_mut().enumerate() {
        *v = k as u8;
    }
    let mut ks = 0u8;
    for k in 0..256usize {
        ks = ks.wrapping_add(state.s[k]).wrapping_add(key[k % 16]);
        state.s.swap(k, ks as usize);
    }
    let mut i = state.i;
    let mut j = state.j;
    for _ in 0..0x300 {
        i = i.wrapping_add(1);
        let t = state.s[i as usize];
        j = j.wrapping_add(t);
        state.s[i as usize] = state.s[j as usize];
        state.s[j as usize] = t;
    }
    state.j = j;
    state.i = i;
}

#[cfg(test)]
mod tests_wave30_agent15 {
    use super::*;

    fn dec(s: &str) -> (u64, usize) {
        let n = s.bytes().take_while(|c| c.is_ascii_digit()).count();
        (s[..n].parse().unwrap_or(0), n)
    }

    #[test]
    fn parse_signed() {
        let a = [
            "5",
            "-5",
            "2147483647",
            "2147483648",
            "-2147483648",
            "x",
            "4294967296",
        ];
        assert_eq!(w30a15_arg_to_i32(&a, 0, dec), Some(5));
        assert_eq!(w30a15_arg_to_i32(&a, 1, dec), Some(-5));
        assert_eq!(w30a15_arg_to_i32(&a, 2, dec), Some(i32::MAX));
        assert_eq!(w30a15_arg_to_i32(&a, 3, dec), None);
        assert_eq!(w30a15_arg_to_i32(&a, 4, dec), Some(i32::MIN));
        assert_eq!(w30a15_arg_to_i32(&a, 5, dec), None);
        assert_eq!(w30a15_arg_to_i32(&a, 6, dec), None);
        assert_eq!(w30a15_arg_to_i32(&a, 9, dec), None);
    }

    #[test]
    fn parse_unsigned() {
        let a = ["4294967295", "4294967296", "12z"];
        assert_eq!(w30a15_arg_to_u32(&a, 0, dec), Some(u32::MAX));
        assert_eq!(w30a15_arg_to_u32(&a, 1, dec), None);
        assert_eq!(w30a15_arg_to_u32(&a, 2, dec), None);
    }

    #[test]
    fn gate() {
        let mut g = W30a15Gate {
            size: 8,
            expect: 3,
            visits: 0,
        };
        let rec = [5u8, 0, 0, 0, 0, 0x0b];
        assert_eq!(w30a15_gate_visit(&mut g, &rec, |_| true), Some(true));
        assert_eq!(g.visits, 1);
        assert_eq!(
            w30a15_gate_visit(&mut g, &[4, 0, 0, 0, 0, 3], |_| true),
            None
        );
        assert_eq!(
            w30a15_gate_visit(&mut g, &[5, 0, 0, 0, 0, 4], |_| true),
            None
        );
        assert_eq!(g.visits, 1);
    }

    #[test]
    fn grow() {
        assert_eq!(w30a15_grow_capacity(15, 10), None);
        assert_eq!(w30a15_grow_capacity(15, 16), Some(22));
        assert_eq!(w30a15_grow_capacity(0x8000_000f, 100), Some(0x8000_0064));
    }

    #[test]
    fn rc4() {
        let mut st = W30a15Rc4 {
            s: [0; 256],
            i: 9,
            j: 0,
        };
        w30a15_key_schedule(&mut st, 1, 2, 3, &[7; 16], |s| {
            let mut o = [0u8; 16];
            o.copy_from_slice(&s[..16]);
            o
        });
        let mut v = st.s.to_vec();
        v.sort();
        assert!(v.iter().enumerate().all(|(k, b)| *b as usize == k));
        assert_eq!(st.i, 0); // 0x300 steps wrap the counter back to zero
    }
}
