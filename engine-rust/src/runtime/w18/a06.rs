#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave18/agent06.rs
// ===========================================================================
// Wave 18 translation batch 06: texture-blob loader, atlas rows, and small
// capacity-word vector helpers (high bit = inline storage, low 31 bits =
// capacity).

/// `0x00535de0`: texture blob decode dispatch.  `blob` starts with the
/// magic `"BMP "` (0x20504d42): the payload is `blob[4..len]`, the format is
/// `0` when `len / (w*h) != 1` else `4`, and the result is `len - 4` on
/// success.  Otherwise the header is 0x80 bytes, the format is `3` unless the
/// dword at offset 0x54 equals `"DXT1"` (0x31545844) in which case `2`, and
/// the result is `len - 0x80`.  `decode(payload, size, fmt)` models the
/// unresolved decoder call; failure yields 0.
pub fn w18a06_decode_texture_blob(
    blob: &[u8],
    len: u32,
    w: u32,
    h: u32,
    mut decode: impl FnMut(&[u8], u32, u32) -> bool,
) -> u32 {
    let dword = |o: usize| -> Option<u32> {
        blob.get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    if dword(0) == Some(0x2050_4d42) {
        let area = w.wrapping_mul(h);
        let fmt = if area != 0 && len / area != 1 { 0 } else { 4 };
        let size = len.wrapping_sub(4);
        if decode(blob.get(4..).unwrap_or(&[]), size, fmt) {
            return size;
        }
        0
    } else {
        let fmt = if dword(0x54) != Some(0x3154_5844) {
            3
        } else {
            2
        };
        let size = len.wrapping_sub(0x80);
        if decode(blob.get(0x80..).unwrap_or(&[]), size, fmt) {
            return size;
        }
        0
    }
}

/// Header words read by `0x00536550`.
#[allow(non_camel_case_types)]
pub struct w18a06_AtlasHeader<'a> {
    /// u16 at +0x0a (mode; 0 means 4).
    pub mode: u16,
    /// u16 at +0x0c.
    pub width: u16,
    /// u16 at +0x0e.
    pub height: u16,
    /// dwords at +0xb0, +0xc0 .. +0x110 (stride 0x10).
    pub slots: [u32; 7],
    /// 16-byte rows (eight i16) at the pointer stored at +0x118.
    pub rows: &'a [[i16; 8]],
}

/// `0x00536550`: builds the 15-word draw record for atlas row `index`.
/// Words 0..7 = header slots, 7..11 = rect from `rect_fn(x, y, w, h, fw, fh)`
/// (unresolved float callee), 11/12 = packed i16 pairs of row[2..6], 13 =
/// row[6..8] as u32 (| 2 when slot 2 is nonzero), 14 = mode (0 -> 4; mode 2
/// with `double` forces 4 and doubles the float size).  Out of range rows
/// return `None`.
pub fn w18a06_build_atlas_record(
    hdr: &w18a06_AtlasHeader,
    index: usize,
    double: bool,
    mut rect_fn: impl FnMut(f32, f32, f32, f32, f32, f32) -> [u32; 4],
) -> Option<[u32; 15]> {
    let r = hdr.rows.get(index)?;
    let mut out = [0u32; 15];
    out[11] = (r[2] as u16 as u32) | ((r[3] as u16 as u32) << 16);
    out[12] = (r[4] as u16 as u32) | ((r[5] as u16 as u32) << 16);
    out[13] = (r[6] as u16 as u32) | ((r[7] as u16 as u32) << 16);
    out[14] = if hdr.mode == 0 { 4 } else { hdr.mode as u32 };
    let mut fw = hdr.width as f32;
    let mut fh = hdr.height as f32;
    if hdr.mode == 2 && double {
        out[14] = 4;
        fw += fw;
        fh += fh;
    }
    let rect = rect_fn(r[0] as f32, r[1] as f32, r[4] as f32, r[5] as f32, fw, fh);
    out[7..11].copy_from_slice(&rect);
    out[..7].copy_from_slice(&hdr.slots);
    if out[2] != 0 {
        out[13] |= 2;
    }
    Some(out)
}

/// `0x005366d0`: fetches 32-byte entry `inner` of table `outer`.  Returns
/// `true` and fills `out` on success; if `inner` is out of range `out` is
/// zeroed and `false` is returned; if the table is absent or `outer` is out
/// of range `out` is untouched and `false` is returned.
pub fn w18a06_get_table_entry(
    tables: Option<&[Vec<[u32; 8]>]>,
    outer: usize,
    inner: usize,
    out: &mut [u32; 8],
) -> bool {
    let Some(t) = tables.and_then(|t| t.get(outer)) else {
        return false;
    };
    match t.get(inner) {
        Some(e) => {
            *out = *e;
            true
        }
        None => {
            *out = [0; 8];
            false
        }
    }
}

/// `0x00538490`: `(i32)(i16 at +0x32) as u32 / divisor`; `None` on zero.
pub fn w18a06_height_over_divisor(short_at_32: i16, divisor: u32) -> Option<u32> {
    (short_at_32 as i32 as u32).checked_div(divisor)
}

/// `0x005384a0`: `(i32)(i16 at +0x30) as u32 / divisor`; `None` on zero.
pub fn w18a06_width_over_divisor(short_at_30: i16, divisor: u32) -> Option<u32> {
    (short_at_30 as i32 as u32).checked_div(divisor)
}

/// 32-byte-element vector: `len`, capacity word (bit 31 = inline buffer).
#[derive(Default, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub struct w18a06_Vec32 {
    pub data: Vec<[u32; 8]>,
    pub cap_word: u32,
}

/// `0x005384b0`: move-assign from `src`.  Heap source (bit 31 clear): steal
/// buffer/len and capacity (dest keeps its own bit 31 overwritten by the
/// source's; source keeps only bit 31, emptied).  Inline source: copy the
/// elements if `src.len < dest capacity` and dest is inline; otherwise the
/// original never returns (`Err`).
pub fn w18a06_vec32_move_assign(dest: &mut w18a06_Vec32, src: &mut w18a06_Vec32) -> Result<(), ()> {
    if src.cap_word & 0x8000_0000 == 0 {
        dest.data = std::mem::take(&mut src.data);
        dest.cap_word = src.cap_word & 0x7fff_ffff;
        src.cap_word &= 0x8000_0000;
        return Ok(());
    }
    dest.data.clear();
    if (src.data.len() as u32) < (dest.cap_word & 0x7fff_ffff) && dest.cap_word & 0x8000_0000 != 0 {
        dest.data.extend_from_slice(&src.data);
        return Ok(());
    }
    Err(())
}

/// `0x005388a0`: reserve growth.  If capacity (low 31 bits) < `wanted`, the
/// new capacity is `max(cap + cap/2, wanted)` (cap/2 taken from bits 1..30);
/// returns the new capacity word keeping bit 31, else `None`.
pub fn w18a06_reserve_word(word: u32, wanted: u32) -> Option<u32> {
    let cap = word & 0x7fff_ffff;
    if cap >= wanted {
        return None;
    }
    let mut n = ((word >> 1) & 0x3fff_ffff).wrapping_add(cap);
    if n < wanted {
        n = wanted;
    }
    Some(word ^ ((word ^ n) & 0x7fff_ffff))
}

/// `0x00538710`: push_back of a dword.  `src_index` is the aliasing source
/// `(ptr - base) >> 2`; it is used only when `< len` (element lives inside
/// the vector), else `value` is used.  When `len == cap` growth is required
/// (original never returns): the new capacity word is returned in `Err`.
pub fn w18a06_push_back_u32(
    data: &mut Vec<u32>,
    word: &mut u32,
    src_index: u32,
    value: u32,
) -> Result<(), u32> {
    let cap = *word & 0x7fff_ffff;
    if data.len() as u32 == cap && cap < cap.wrapping_add(1) {
        let g = (*word >> 1) & 0x3fff_ffff;
        let n = if cap < cap.wrapping_add(g) { g } else { 1 };
        return Err(*word ^ (((cap + n) ^ *word) & 0x7fff_ffff));
    }
    let v = data.get(src_index as usize).copied().unwrap_or(value);
    data.push(v);
    Ok(())
}

/// Result of the shared resize helpers.
#[derive(Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum w18a06_Resize {
    /// Same length, nothing to do.
    Same,
    /// Shrink: the original erases the tail via a non-returning call.
    Shrink,
    /// Grown in place (zero-filled).
    Grown,
    /// Growth needs reallocation; carries the new capacity word.
    Realloc(u32),
}

fn w18a06_resize_zero<T: Clone>(
    data: &mut Vec<T>,
    word: &mut u32,
    new_len: u32,
    zero: T,
) -> w18a06_Resize {
    let len = data.len() as u32;
    if new_len < len {
        return w18a06_Resize::Shrink;
    }
    if len == new_len {
        return w18a06_Resize::Same;
    }
    if let Some(w) = w18a06_reserve_word(*word, new_len) {
        *word = w;
        return w18a06_Resize::Realloc(w);
    }
    data.resize(new_len as usize, zero);
    w18a06_Resize::Grown
}

/// `0x00538b90`: resize a vector of 16-byte elements, zero-filling growth.
pub fn w18a06_resize16(data: &mut Vec<[u32; 4]>, word: &mut u32, new_len: u32) -> w18a06_Resize {
    w18a06_resize_zero(data, word, new_len, [0; 4])
}

/// `0x00538cb0`: resize a vector of 12-byte elements, zero-filling growth.
pub fn w18a06_resize12(data: &mut Vec<[u32; 3]>, word: &mut u32, new_len: u32) -> w18a06_Resize {
    w18a06_resize_zero(data, word, new_len, [0; 3])
}

#[cfg(test)]
mod tests_wave18_agent06 {
    use super::*;

    #[test]
    fn blob_decode() {
        let mut b = vec![0u8; 0x100];
        b[..4].copy_from_slice(b"BMP ");
        let mut got = (0, 0);
        let r = w18a06_decode_texture_blob(&b, 100, 10, 5, |_, s, f| {
            got = (s, f);
            true
        });
        assert_eq!((r, got), (96, (96, 0)));
        let r = w18a06_decode_texture_blob(&b, 100, 10, 10, |_, _, f| {
            got.1 = f;
            true
        });
        assert_eq!((r, got.1), (96, 4));
        let mut d = vec![0u8; 0x100];
        d[0x54..0x58].copy_from_slice(b"DXT1");
        let r = w18a06_decode_texture_blob(&d, 0x100, 1, 1, |_, s, f| {
            got = (s, f);
            true
        });
        assert_eq!((r, got), (0x80, (0x80, 2)));
        assert_eq!(
            w18a06_decode_texture_blob(&b[1..], 0x100, 1, 1, |_, _, f| f == 3),
            0x80
        );
        assert_eq!(w18a06_decode_texture_blob(&b, 8, 1, 1, |_, _, _| false), 0);
    }

    #[test]
    fn atlas_record() {
        let rows = [[1i16, 2, 3, 4, 5, 6, 7, 0]];
        let mut hdr = w18a06_AtlasHeader {
            mode: 2,
            width: 8,
            height: 9,
            slots: [1, 2, 0, 4, 5, 6, 7],
            rows: &rows,
        };
        let mut seen = [0f32; 6];
        let r = w18a06_build_atlas_record(&hdr, 0, true, |a, b, c, d, e, f| {
            seen = [a, b, c, d, e, f];
            [9, 8, 7, 6]
        })
        .unwrap();
        assert_eq!(seen, [1.0, 2.0, 5.0, 6.0, 16.0, 18.0]);
        assert_eq!(&r[7..11], &[9, 8, 7, 6]);
        assert_eq!(r[11], 3 | (4 << 16));
        assert_eq!(r[13], 7);
        assert_eq!(r[14], 4);
        hdr.mode = 0;
        hdr.slots[2] = 1;
        let r = w18a06_build_atlas_record(&hdr, 0, true, |_, _, _, _, _, _| [0; 4]).unwrap();
        assert_eq!((r[14], r[13]), (4, 7 | 2));
        assert!(w18a06_build_atlas_record(&hdr, 1, false, |_, _, _, _, _, _| [0; 4]).is_none());
    }

    #[test]
    fn table_entry() {
        let t = vec![vec![[1u32; 8]]];
        let mut o = [5u32; 8];
        assert!(w18a06_get_table_entry(Some(&t), 0, 0, &mut o));
        assert_eq!(o, [1; 8]);
        assert!(!w18a06_get_table_entry(Some(&t), 0, 1, &mut o));
        assert_eq!(o, [0; 8]);
        o = [3; 8];
        assert!(!w18a06_get_table_entry(Some(&t), 1, 0, &mut o));
        assert!(!w18a06_get_table_entry(None, 0, 0, &mut o));
        assert_eq!(o, [3; 8]);
    }

    #[test]
    fn divisors() {
        assert_eq!(w18a06_height_over_divisor(100, 4), Some(25));
        assert_eq!(w18a06_width_over_divisor(-1, 2), Some(0x7fff_ffff));
        assert_eq!(w18a06_width_over_divisor(1, 0), None);
        assert_eq!(w18a06_height_over_divisor(1, 0), None);
    }

    #[test]
    fn move_assign() {
        let mut s = w18a06_Vec32 {
            data: vec![[1; 8]; 2],
            cap_word: 4,
        };
        let mut d = w18a06_Vec32 {
            data: vec![],
            cap_word: 0x8000_0002,
        };
        w18a06_vec32_move_assign(&mut d, &mut s).unwrap();
        assert_eq!(
            (d.data.len(), d.cap_word, s.cap_word, s.data.len()),
            (2, 4, 0, 0)
        );
        let mut s = w18a06_Vec32 {
            data: vec![[2; 8]],
            cap_word: 0x8000_0004,
        };
        let mut d = w18a06_Vec32 {
            data: vec![],
            cap_word: 0x8000_0004,
        };
        w18a06_vec32_move_assign(&mut d, &mut s).unwrap();
        assert_eq!(d.data, vec![[2; 8]]);
        let mut d2 = w18a06_Vec32 {
            data: vec![],
            cap_word: 0x8000_0001,
        };
        assert!(w18a06_vec32_move_assign(&mut d2, &mut s).is_err());
    }

    #[test]
    fn reserve_and_push() {
        assert_eq!(w18a06_reserve_word(10, 5), None);
        assert_eq!(w18a06_reserve_word(0x8000_0008, 9), Some(0x8000_000c));
        assert_eq!(w18a06_reserve_word(0x8000_0008, 100), Some(0x8000_0064));
        let mut v = vec![7u32, 8];
        let mut w = 5u32;
        w18a06_push_back_u32(&mut v, &mut w, 1, 99).unwrap();
        w18a06_push_back_u32(&mut v, &mut w, 100, 99).unwrap();
        assert_eq!(v, vec![7, 8, 8, 99]);
        assert_eq!(w18a06_push_back_u32(&mut v, &mut 4, 0, 1), Err(6));
        assert_eq!(
            w18a06_push_back_u32(&mut v, &mut 0x8000_0004, 0, 1),
            Err(0x8000_0006)
        );
    }

    #[test]
    fn resizes() {
        let mut v = vec![[1u32; 4]];
        let mut w = 4u32;
        assert_eq!(w18a06_resize16(&mut v, &mut w, 3), w18a06_Resize::Grown);
        assert_eq!(v[2], [0; 4]);
        assert_eq!(w18a06_resize16(&mut v, &mut w, 3), w18a06_Resize::Same);
        assert_eq!(w18a06_resize16(&mut v, &mut w, 1), w18a06_Resize::Shrink);
        assert_eq!(
            w18a06_resize16(&mut v, &mut w, 10),
            w18a06_Resize::Realloc(10)
        );
        let mut v = vec![];
        let mut w = 8u32;
        assert_eq!(w18a06_resize12(&mut v, &mut w, 2), w18a06_Resize::Grown);
        assert_eq!(v, vec![[0u32; 3]; 2]);
    }
}
