#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent18.rs
// ===========================================================================
// Wave 22 translation batch 18 (runtime helpers).

/// `0x00687580`: placement check.  `table` holds `(dx, dy)` i16 pairs
/// indexed by `kind`; each is divided by 32 rounding toward zero.  Returns 3
/// when `x`/`y` (low 16 bits, signed) are negative or the footprint exceeds
/// the map size (`map_w`, `map_h`, fields at +0xe4/+0xe6); otherwise the max
/// of the two caller-supplied checks (`check_a` = FUN_00686bf0 result,
/// `check_b` = FUN_00686490 result).
pub fn w22a18_placement_check(
    table: &[(i16, i16)],
    kind: u32,
    x: u32,
    y: u32,
    map_w: u16,
    map_h: u16,
    check_a: impl FnOnce() -> i32,
    check_b: impl FnOnce() -> i32,
) -> i32 {
    let (dx, dy) = table
        .get((kind & 0xffff) as usize)
        .copied()
        .unwrap_or((0, 0));
    let sx = (dx as i32 + ((dx as i32 >> 31) & 0x1f)) >> 5;
    let sy = (dy as i32 + ((dy as i32 >> 31) & 0x1f)) >> 5;
    let (sx, sy) = (sx as i16 as i32, sy as i16 as i32);
    let (x, y) = (x as u16 as i16 as i32, y as u16 as i16 as i32);
    if x >= 0 && y >= 0 && sx + x <= map_w as i32 && sy + y <= map_h as i32 {
        let a = check_a();
        let b = check_b();
        return b.max(a);
    }
    3
}

/// `0x00687b60`: reads a little-endian u32 from a stream through `read`
/// (which fills a slice and returns the byte count).  Loops while the count
/// is not a multiple of 4 (a zero read fails); fewer than 4 bytes total is
/// "Premature end of stream".
pub fn w22a18_stream_read_u32(
    mut read: impl FnMut(&mut [u8]) -> usize,
) -> Result<u32, &'static str> {
    let mut buf = [0u8; 4];
    let mut n = read(&mut buf);
    while n & 3 != 0 {
        if n > 4 {
            return Err("Premature end of stream");
        }
        let m = read(&mut buf[n..]);
        if m == 0 {
            return Err("Premature end of stream");
        }
        n += m;
    }
    if n > 3 {
        Ok(u32::from_le_bytes(buf))
    } else {
        Err("Premature end of stream")
    }
}

/// `0x00687bd0`: writes 4 bytes through `write` (returns bytes accepted),
/// retrying on partial writes; zero progress or nothing written fails with
/// "Stream rejected write operation".
pub fn w22a18_stream_write_u32(
    data: [u8; 4],
    mut write: impl FnMut(&[u8]) -> usize,
) -> Result<(), &'static str> {
    let mut n = write(&data);
    while n & 3 != 0 {
        if n > 4 {
            return Err("Stream rejected write operation");
        }
        let m = write(&data[n..]);
        if m == 0 {
            return Err("Stream rejected write operation");
        }
        n += m;
    }
    if n > 3 {
        Ok(())
    } else {
        Err("Stream rejected write operation")
    }
}

/// `0x006881a0`: runs `codec(capacity)` which returns `(status, out_len)`.
/// When status is 0 and `out_len < capacity`, returns `(out_len, true)`
/// (the codec's own buffer is selected); else `(capacity, false)` (the
/// input buffer is kept).
pub fn w22a18_pick_compressed(capacity: u32, codec: impl FnOnce(u32) -> (i32, u32)) -> (u32, bool) {
    let (status, out) = codec(capacity);
    if status == 0 && out < capacity {
        (out, true)
    } else {
        (capacity, false)
    }
}

#[cfg(test)]
mod tests_wave22_agent18 {
    use super::*;

    #[test]
    fn placement() {
        let t = [(64i16, -33i16), (0, 0)];
        assert_eq!(w22a18_placement_check(&t, 0, 0, 0, 10, 10, || 1, || 5), 5);
        assert_eq!(w22a18_placement_check(&t, 0, 0, 0, 10, 10, || 7, || 5), 7);
        assert_eq!(
            w22a18_placement_check(&t, 0, 0xffff, 0, 10, 10, || 1, || 1),
            3
        );
        assert_eq!(w22a18_placement_check(&t, 0, 9, 0, 10, 10, || 1, || 1), 3);
        assert_eq!(w22a18_placement_check(&t, 0, 8, 1, 10, 10, || 1, || 2), 2);
    }

    #[test]
    fn read_u32() {
        let src = [1u8, 2, 3, 4];
        let mut pos = 0;
        let v = w22a18_stream_read_u32(|b| {
            let n = b.len().min(1).min(src.len() - pos);
            b[..n].copy_from_slice(&src[pos..pos + n]);
            pos += n;
            n
        });
        assert_eq!(v, Ok(0x04030201));
        assert!(w22a18_stream_read_u32(|_| 0).is_err());
    }

    #[test]
    fn write_u32() {
        let mut out = Vec::new();
        let r = w22a18_stream_write_u32([9, 8, 7, 6], |d| {
            out.push(d[0]);
            1
        });
        assert!(r.is_ok());
        assert_eq!(out.len(), 4);
        assert!(w22a18_stream_write_u32([0; 4], |_| 0).is_err());
    }

    #[test]
    fn pick() {
        assert_eq!(w22a18_pick_compressed(100, |_| (0, 50)), (50, true));
        assert_eq!(w22a18_pick_compressed(100, |_| (0, 100)), (100, false));
        assert_eq!(w22a18_pick_compressed(100, |_| (1, 10)), (100, false));
    }
}
