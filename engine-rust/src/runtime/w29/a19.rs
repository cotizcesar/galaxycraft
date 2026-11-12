#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent19.rs
// ===========================================================================
// Wave 29 agent 19: Crypto++ style decoder block-span helpers.

/// `0x00aea220`: builds the 0x18-byte header record: byte 0 is the low byte of
/// `len`, bytes 1.. are zeroed and then receive the first `len` key bytes.
pub fn w29a19_build_header(len: u32, key: &[u8]) -> [u8; 0x18] {
    let mut out = [0u8; 0x18];
    out[0] = len as u8;
    let n = (len as usize).min(key.len()).min(0x17);
    out[1..1 + n].copy_from_slice(&key[..n]);
    out
}

/// `0x00aede00`: size getter by decoder kind (1 and 3 carry a size, else 0).
pub fn w29a19_kind_size(kind: u32, size_k1: u32, size_k3: u32) -> u32 {
    match kind {
        1 => size_k1,
        3 => size_k3,
        _ => 0,
    }
}

/// `0x00aedea0` (clamp part): clamps `req` to `end - pos`; the flag is true
/// when the clamped request reaches `end` exactly.
pub fn w29a19_clamp_request(end: u64, pos: u64, req: u32) -> (u32, bool) {
    let rem = end.wrapping_sub(pos);
    let n = if rem >> 32 != 0 || (req as u64) <= rem {
        req
    } else {
        rem as u32
    };
    (n, pos.wrapping_add(n as u64) == end)
}

/// `0x00aedea0` (accounting part): when the inner call succeeded (0), the
/// input position grows by `consumed` and the output counter by `produced`.
pub fn w29a19_advance(
    in_pos: &mut u64,
    out_pos: &mut u64,
    result: i32,
    consumed: u32,
    produced: u32,
) {
    if result == 0 {
        *in_pos = in_pos.wrapping_add(consumed as u64);
        *out_pos = out_pos.wrapping_add(produced as u64);
    }
}

/// `0x00aea260` (non-frame branch of GetOutputBlockSpan): `off` is the field
/// at +0x18, `c_end` at +0x50, `e_size` at +0x58, `e_set` at +0x61.
/// Returns `(start, len)` or an error message.
pub fn w29a19_output_block_span(
    off: u32,
    c_end: u64,
    e_size: u64,
    e_set: bool,
    a: u64,
    b: u64,
) -> Result<(u64, u64), &'static str> {
    if !e_set {
        return Err("missing E-size");
    }
    let sum = a.wrapping_add(b);
    if sum > c_end {
        return Err("input span out of bounds in GetOutputBlockSpan()");
    }
    if a <= off as u64 && sum >= c_end {
        Ok((0, e_size))
    } else {
        Ok((0, 0))
    }
}

/// `0x00aea4f0` (non-frame branch of GetInputBlockSpan): `mode` is the field
/// at +0x30.  Returns `(start, len)` or an error message.
pub fn w29a19_input_block_span(
    off: u32,
    mode: u32,
    c_end: u64,
    e_size: u64,
    e_set: bool,
    a: u64,
    b: u64,
) -> Result<(u64, u64), &'static str> {
    if !e_set {
        return Err("missing E-size in GetInputBlockSpan()");
    }
    if a.wrapping_add(b) > e_size {
        return Err("output span out of bounds in GetInputBlockSpan()");
    }
    if mode == 0 {
        Ok((off as u64 + a, b))
    } else {
        Ok((off as u64, c_end.wrapping_sub(off as u64)))
    }
}

#[cfg(test)]
mod tests_wave29_agent19 {
    use super::*;

    #[test]
    fn header_and_size() {
        let h = w29a19_build_header(3, &[1, 2, 3, 4]);
        assert_eq!(&h[..5], &[3, 1, 2, 3, 0]);
        assert_eq!(w29a19_kind_size(1, 7, 9), 7);
        assert_eq!(w29a19_kind_size(3, 7, 9), 9);
        assert_eq!(w29a19_kind_size(2, 7, 9), 0);
    }

    #[test]
    fn clamp_and_advance() {
        assert_eq!(w29a19_clamp_request(100, 90, 50), (10, true));
        assert_eq!(w29a19_clamp_request(100, 90, 5), (5, false));
        let (mut i, mut o) = (1u64, 2u64);
        w29a19_advance(&mut i, &mut o, 1, 5, 5);
        assert_eq!((i, o), (1, 2));
        w29a19_advance(&mut i, &mut o, 0, 5, 6);
        assert_eq!((i, o), (6, 8));
    }

    #[test]
    fn spans() {
        assert!(w29a19_output_block_span(8, 100, 50, false, 0, 1).is_err());
        assert!(w29a19_output_block_span(8, 100, 50, true, 0, 101).is_err());
        assert_eq!(
            w29a19_output_block_span(8, 100, 50, true, 8, 92),
            Ok((0, 50))
        );
        assert_eq!(
            w29a19_output_block_span(8, 100, 50, true, 9, 10),
            Ok((0, 0))
        );
        assert_eq!(
            w29a19_input_block_span(8, 0, 100, 50, true, 4, 6),
            Ok((12, 6))
        );
        assert_eq!(
            w29a19_input_block_span(8, 1, 100, 50, true, 4, 6),
            Ok((8, 92))
        );
        assert!(w29a19_input_block_span(8, 0, 100, 50, true, 4, 60).is_err());
        assert!(w29a19_input_block_span(8, 0, 100, 50, false, 4, 6).is_err());
    }
}
