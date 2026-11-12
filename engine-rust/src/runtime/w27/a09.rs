#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent09.rs
// ===========================================================================
// Wave 27 translation batch 09: rapidjson-style arena, JSON-pointer parser,
// string value constructor and small registry/geometry helpers.

/// One arena chunk: capacity and bytes used (header is 12 bytes in retail).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W27a09Chunk {
    pub capacity: u32,
    pub used: u32,
}

/// Chunked bump allocator (retail layout: head chunk, default size, flag).
#[derive(Debug, Clone, Default)]
pub struct W27a09Arena {
    /// Chunks in allocation order; the last one is the retail list head.
    pub chunks: Vec<W27a09Chunk>,
    pub default_size: u32,
    /// Retail `+0xc` field: zero means allocation is a fatal error.
    pub allocator_ok: bool,
}

/// `0x00972100`: bump-allocates `size` bytes (rounded up to 4) from the head
/// chunk, else opens a new chunk of `max(default, size)`.  Returns
/// `Ok(None)` for size 0 / failed alloc, `Ok(Some((chunk, offset)))` where
/// offset is relative to the chunk payload (retail adds the 0xc header), or
/// `Err(())` for the fatal path.
pub fn w27a09_arena_alloc(
    a: &mut W27a09Arena,
    size: u32,
    alloc_ok: bool,
) -> Result<Option<(usize, u32)>, ()> {
    if size == 0 {
        return Ok(None);
    }
    let rounded = size.wrapping_add(3) & 0xffff_fffc;
    if let Some(h) = a.chunks.last_mut()
        && h.used.wrapping_add(rounded) <= h.capacity
    {
        let off = h.used;
        h.used = off.wrapping_add(rounded);
        return Ok(Some((a.chunks.len() - 1, off)));
    }
    let cap = a.default_size.max(rounded);
    if !a.allocator_ok {
        return Err(());
    }
    if cap.wrapping_add(0xc) == 0 || !alloc_ok {
        return Ok(None);
    }
    a.chunks.push(W27a09Chunk {
        capacity: cap,
        used: rounded,
    });
    Ok(Some((a.chunks.len() - 1, 0)))
}

/// `0x00972190`: guarded allocation; fatal when the allocator flag (`+0x10`)
/// is clear, `None` for a zero request.
pub fn w27a09_guarded_alloc<T>(
    flag: u32,
    size: u32,
    alloc: impl FnOnce(u32) -> T,
) -> Result<Option<T>, ()> {
    if flag == 0 {
        return Err(());
    }
    Ok(if size != 0 { Some(alloc(size)) } else { None })
}

/// One parsed JSON-pointer token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W27a09Token {
    pub name: Vec<u8>,
    /// Array index, or `0xffff_ffff` when the token is not a canonical index.
    pub index: u32,
}

/// Error `(code, offset)`: 1 missing leading '/', 2 bad `~` escape,
/// 3 bad percent encoding, 4 invalid URI-fragment char.
pub type W27a09PtrError = (u32, usize);

/// `0x009724d0`: parses a JSON pointer (optionally `#` URI-fragment form)
/// into tokens.  `decode_percent(input, pos)` models the percent decoder
/// and returns `(decoded bytes, position after the sequence)`.
pub fn w27a09_parse_json_pointer(
    s: &[u8],
    mut decode_percent: impl FnMut(&[u8], usize) -> Option<(Vec<u8>, usize)>,
) -> Result<Vec<W27a09Token>, W27a09PtrError> {
    let len = s.len();
    let fragment = s.first() == Some(&b'#');
    let mut pos = usize::from(fragment);
    let mut out = Vec::new();
    if pos == len {
        return Ok(out);
    }
    if s[pos] != b'/' {
        return Err((1, pos));
    }
    while pos < len {
        pos += 1; // skip '/'
        let mut name: Vec<u8> = Vec::new();
        let mut numeric = true;
        while pos < len && s[pos] != b'/' {
            let mut c = s[pos];
            let mut next = pos + 1;
            if fragment {
                if c == b'%' {
                    match decode_percent(s, pos) {
                        Some((bytes, end)) if !bytes.is_empty() => {
                            if bytes.len() != 1 {
                                numeric = false;
                                name.extend_from_slice(&bytes);
                                pos = end;
                                continue;
                            }
                            c = bytes[0];
                            next = end;
                        }
                        _ => return Err((3, pos)),
                    }
                } else if !(c.is_ascii_alphanumeric() || matches!(c, b'-' | b'.' | b'_' | b'~')) {
                    return Err((4, pos));
                }
            }
            if c == b'~' {
                if next >= len {
                    return Err((2, next));
                }
                c = match s[next] {
                    b'0' => b'~',
                    b'1' => b'/',
                    _ => return Err((2, next)),
                };
                next += 1;
            }
            if !c.is_ascii_digit() {
                numeric = false;
            }
            name.push(c);
            pos = next;
        }
        let mut is_index = numeric && !name.is_empty();
        if is_index && name.len() > 1 && name[0] == b'0' {
            is_index = false;
        }
        let mut val = 0u32;
        if is_index {
            for &d in &name {
                let n = (d as u32)
                    .wrapping_add(val.wrapping_mul(10))
                    .wrapping_sub(0x30);
                if n < val {
                    is_index = false;
                    break;
                }
                val = n;
            }
        }
        out.push(W27a09Token {
            name,
            index: if is_index { val } else { 0xffff_ffff },
        });
    }
    Ok(out)
}

/// `0x00972bd0`: builds the 16-byte JSON string value.  Without `copy` the
/// value references `ptr` (flags 0x405, len in word 0).  With `copy`,
/// strings shorter than 14 bytes are stored inline (flags 0x1c05, byte 13 =
/// 13 - len) otherwise `len` goes in word 0, `long_ptr` in word 2, flags
/// 0xc05.
pub fn w27a09_make_string_value(
    data: &[u8],
    ptr: u32,
    len: u32,
    copy: bool,
    long_ptr: u32,
) -> [u8; 16] {
    let mut v = [0u8; 16];
    if !copy {
        v[0..4].copy_from_slice(&len.to_le_bytes());
        v[8..12].copy_from_slice(&ptr.to_le_bytes());
        v[14..16].copy_from_slice(&0x405u16.to_le_bytes());
    } else if len < 0xe {
        v[14..16].copy_from_slice(&0x1c05u16.to_le_bytes());
        v[13] = (13 - len) as u8;
        let n = (len as usize).min(data.len());
        v[..n].copy_from_slice(&data[..n]);
    } else {
        v[0..4].copy_from_slice(&len.to_le_bytes());
        v[14..16].copy_from_slice(&0xc05u16.to_le_bytes());
        v[8..12].copy_from_slice(&long_ptr.to_le_bytes());
    }
    v
}

/// `0x009734b0`: initialises a widget record: flags |= 0x44000000, clears
/// `+0x64`, optionally stores `param_3` at `+0x44`, clears `+0x3a`, then
/// shifts x by `dx` (0 when `is_a`, else `base` or `base/2` unless `is_b`).
pub fn w27a09_widget_init(rec: &mut [u8], param_3: u32, is_a: bool, is_b: bool, base: i32) {
    let rd32 = |r: &[u8], o: usize| u32::from_le_bytes(r[o..o + 4].try_into().unwrap());
    let rd16 = |r: &[u8], o: usize| i16::from_le_bytes(r[o..o + 2].try_into().unwrap());
    let f = rd32(rec, 0x30) | 0x4400_0000;
    rec[0x30..0x34].copy_from_slice(&f.to_le_bytes());
    rec[0x64..0x68].copy_from_slice(&0u32.to_le_bytes());
    if param_3 != 0 {
        rec[0x44..0x48].copy_from_slice(&param_3.to_le_bytes());
    }
    rec[0x3a..0x3c].copy_from_slice(&0u16.to_le_bytes());
    let mut d = 0i32;
    if !is_a {
        d = base;
        if !is_b {
            d /= 2;
        }
    }
    let d16 = d as i16;
    let x1 = rd16(rec, 4).wrapping_add(d16);
    let x2 = x1.wrapping_add(rd16(rec, 8));
    let y2 = rd16(rec, 6).wrapping_add(rd16(rec, 10));
    rec[4..6].copy_from_slice(&x1.to_le_bytes());
    rec[8..10].copy_from_slice(&x2.to_le_bytes());
    rec[0x68..0x6a].copy_from_slice(&d16.to_le_bytes());
    rec[10..12].copy_from_slice(&y2.to_le_bytes());
}

/// Number of slots in the registry at 0x011c_de70..0x011c_ded4.
pub const W27A09_SLOT_COUNT: usize = 25;

/// `0x009737d0`: index of the first occupied slot, if any.
pub fn w27a09_first_occupied(slots: &[u32; W27A09_SLOT_COUNT]) -> Option<usize> {
    slots.iter().position(|&s| s != 0)
}

/// `0x00974340`: stores `value` in an empty slot; `Err` when occupied
/// (retail takes a non-returning error path).
pub fn w27a09_slot_set(
    slots: &mut [u32; W27A09_SLOT_COUNT],
    index: usize,
    value: u32,
) -> Result<(), ()> {
    if slots[index] != 0 {
        return Err(());
    }
    slots[index] = value;
    Ok(())
}

/// Destination addresses (low 32 bits) of the 30 snapshot words, in source
/// order starting at record offset 0x148 with stride 2.
pub const W27A09_SNAPSHOT_DEST: [u32; 30] = [
    0x011c_dd68,
    0x011c_dd6c,
    0x011c_dd78,
    0x011c_dd7c,
    0x011c_dd70,
    0x011c_dd74,
    0x011c_dd80,
    0x011c_dd84,
    0x011c_dd88,
    0x011c_dd8c,
    0x011c_dd90,
    0x011c_dd94,
    0x011c_dd98,
    0x011c_dd9c,
    0x011c_dda4,
    0x011c_dda8,
    0x011c_dda0,
    0x011c_ddac,
    0x011c_ddb0,
    0x011c_ddb4,
    0x011c_ddb8,
    0x011c_ddc0,
    0x011c_ddc4,
    0x011c_ddbc,
    0x011c_ddc8,
    0x011c_ddcc,
    0x011c_ddd0,
    0x011c_ddd4,
    0x011c_ddd8,
    0x011c_dddc,
];

/// `0x00974120`: copies the 30 16-bit fields at `+0x148..0x184` of the
/// record into the snapshot globals; returns `(dest address, value)` pairs.
/// (Skipped when the record is already the cached one; the three
/// slot-pointer refreshes at the start are not modelled.)
pub fn w27a09_snapshot_fields(rec: &[u8]) -> [(u32, u16); 30] {
    let mut out = [(0u32, 0u16); 30];
    for (i, o) in out.iter_mut().enumerate() {
        let off = 0x148 + i * 2;
        *o = (
            W27A09_SNAPSHOT_DEST[i],
            u16::from_le_bytes([rec[off], rec[off + 1]]),
        );
    }
    out
}

#[cfg(test)]
mod tests_wave27_agent09 {
    use super::*;

    #[test]
    fn arena() {
        let mut a = W27a09Arena {
            default_size: 16,
            allocator_ok: true,
            ..Default::default()
        };
        assert_eq!(w27a09_arena_alloc(&mut a, 0, true), Ok(None));
        assert_eq!(w27a09_arena_alloc(&mut a, 5, true), Ok(Some((0, 0))));
        assert_eq!(w27a09_arena_alloc(&mut a, 8, true), Ok(Some((0, 8))));
        assert_eq!(w27a09_arena_alloc(&mut a, 20, true), Ok(Some((1, 0))));
        assert_eq!(a.chunks[1].capacity, 20);
        a.allocator_ok = false;
        assert_eq!(w27a09_arena_alloc(&mut a, 100, true), Err(()));
        assert_eq!(w27a09_guarded_alloc(0, 4, |n| n), Err(()));
        assert_eq!(w27a09_guarded_alloc(1, 0, |n| n), Ok(None));
        assert_eq!(w27a09_guarded_alloc(1, 4, |n| n), Ok(Some(4)));
    }

    #[test]
    fn pointer() {
        let no = |_: &[u8], _| None;
        let t = w27a09_parse_json_pointer(b"/a/10/0/~0~1", no).unwrap();
        assert_eq!(t.len(), 4);
        assert_eq!(t[0].index, 0xffff_ffff);
        assert_eq!(t[1].index, 10);
        assert_eq!(t[2].index, 0);
        assert_eq!(t[3].name, b"~/");
        assert_eq!(w27a09_parse_json_pointer(b"", no).unwrap().len(), 0);
        assert_eq!(w27a09_parse_json_pointer(b"a", no), Err((1, 0)));
        assert_eq!(w27a09_parse_json_pointer(b"/~2", no), Err((2, 2)));
        assert_eq!(w27a09_parse_json_pointer(b"#/a b", no), Err((4, 3)));
        assert_eq!(w27a09_parse_json_pointer(b"#/%", no), Err((3, 2)));
        let t = w27a09_parse_json_pointer(b"/01/", no).unwrap();
        assert_eq!(t[0].index, 0xffff_ffff);
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn string_value() {
        let v = w27a09_make_string_value(b"hi", 0, 2, true, 0);
        assert_eq!(&v[..2], b"hi");
        assert_eq!(v[13], 11);
        assert_eq!(u16::from_le_bytes([v[14], v[15]]), 0x1c05);
        let v = w27a09_make_string_value(b"", 0x1234, 5, false, 0);
        assert_eq!(u16::from_le_bytes([v[14], v[15]]), 0x405);
        let v = w27a09_make_string_value(b"", 0, 14, true, 9);
        assert_eq!(u16::from_le_bytes([v[14], v[15]]), 0xc05);
    }

    #[test]
    fn widget_and_slots() {
        let mut r = vec![0u8; 0x70];
        r[4] = 10;
        r[8] = 20;
        r[6] = 3;
        r[10] = 4;
        w27a09_widget_init(&mut r, 7, false, false, 8);
        assert_eq!(r[4], 14);
        assert_eq!(r[8], 34);
        assert_eq!(r[10], 7);
        assert_eq!(r[0x68], 4);
        assert_eq!(r[0x33], 0x44);
        let mut s = [0u32; W27A09_SLOT_COUNT];
        assert_eq!(w27a09_first_occupied(&s), None);
        assert_eq!(w27a09_slot_set(&mut s, 3, 9), Ok(()));
        assert_eq!(w27a09_slot_set(&mut s, 3, 9), Err(()));
        assert_eq!(w27a09_first_occupied(&s), Some(3));
        let mut rec = vec![0u8; 0x184];
        rec[0x148] = 1;
        rec[0x182] = 2;
        let o = w27a09_snapshot_fields(&rec);
        assert_eq!(o[0], (0x011c_dd68, 1));
        assert_eq!(o[29], (0x011c_dddc, 2));
    }
}
