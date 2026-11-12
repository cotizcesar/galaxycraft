#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent28.rs
// ===========================================================================
// Wave 31 translation batch 28.

/// Minimal model of the stream buffer used by the 0x005fd080 / 0x005fd260
/// output loops: a put area (`buf`, with `room` free bytes) and a virtual
/// overflow slot (vtable +0xc) that returns `0xffff_ffff` on failure.
#[allow(non_camel_case_types)]
pub struct w31a28_StreamBuf<'a> {
    pub buf: Vec<u8>,
    pub room: i32,
    pub has_put_area: bool,
    pub overflow: Box<dyn FnMut(u8) -> u32 + 'a>,
}

/// Single put step shared by both loops: returns the stored/overflow result.
fn w31a28_put(sb: &mut w31a28_StreamBuf, b: u8) -> u32 {
    if sb.has_put_area && sb.room >= 1 {
        sb.room -= 1;
        sb.buf.push(b);
        b as u32
    } else {
        (sb.overflow)(b)
    }
}

/// `0x005fd080`: writes `count` bytes from `src` through the stream buffer.
/// Returns the failure word (low byte forced to 1 on any EOF, upper bits
/// kept from `state`) and the buffer handle (`None` stays `None`; a missing
/// buffer also sets the failure byte).
pub fn w31a28_write_n<'a, 'b>(
    mut state: u32,
    mut sb: Option<&'b mut w31a28_StreamBuf<'a>>,
    src: &[u8],
    count: usize,
) -> (u32, Option<&'b mut w31a28_StreamBuf<'a>>) {
    for i in 0..count {
        let failed = match sb.as_deref_mut() {
            None => true,
            Some(s) => w31a28_put(s, src[i]) == 0xffff_ffff,
        };
        if failed {
            state = (state & 0xffff_ff00) | 1;
        }
    }
    (state, sb)
}

/// `0x005fd260`: same as the routine above but repeats one byte `count`
/// times.
pub fn w31a28_fill_n<'a, 'b>(
    mut state: u32,
    mut sb: Option<&'b mut w31a28_StreamBuf<'a>>,
    byte: u8,
    count: usize,
) -> (u32, Option<&'b mut w31a28_StreamBuf<'a>>) {
    for _ in 0..count {
        let failed = match sb.as_deref_mut() {
            None => true,
            Some(s) => w31a28_put(s, byte) == 0xffff_ffff,
        };
        if failed {
            state = (state & 0xffff_ff00) | 1;
        }
    }
    (state, sb)
}

/// `0x005fdd20`: copies the first dword of `this` into the output slot.
pub fn w31a28_copy_first(this: &[u32]) -> u32 {
    this[0]
}

/// `0x005ffb10`: copies the second dword (offset 4) of `this` into the
/// output slot.
pub fn w31a28_copy_second(this: &[u32]) -> u32 {
    this[1]
}

#[cfg(test)]
mod tests_wave31_agent28 {
    use super::*;

    #[test]
    fn write_and_fill() {
        let mut sb = w31a28_StreamBuf {
            buf: vec![],
            room: 2,
            has_put_area: true,
            overflow: Box::new(|_| 0xffff_ffff),
        };
        let (st, _) = w31a28_write_n(0x1200, Some(&mut sb), b"abc", 3);
        assert_eq!(st, 0x1201);
        assert_eq!(sb.buf, b"ab");
        let mut sb2 = w31a28_StreamBuf {
            buf: vec![],
            room: 0,
            has_put_area: true,
            overflow: Box::new(|b| b as u32),
        };
        let (st, _) = w31a28_fill_n(0x1200, Some(&mut sb2), 7, 3);
        assert_eq!(st, 0x1200);
        let (st, h) = w31a28_fill_n(0, None, 1, 1);
        assert_eq!(st, 1);
        assert!(h.is_none());
        assert_eq!(w31a28_fill_n(5, None, 1, 0).0, 5);
    }

    #[test]
    fn copies() {
        assert_eq!(w31a28_copy_first(&[9, 8]), 9);
        assert_eq!(w31a28_copy_second(&[9, 8]), 8);
    }
}
