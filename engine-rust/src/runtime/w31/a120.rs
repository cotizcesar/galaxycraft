#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent120.rs
// ===========================================================================
// Wave 31 agent 120: zlib glue (memcpy tails are modelled as real copies).

/// `0x00c74920`: flush-pending style copy.  Returns how many bytes of
/// `pending` are copied to `out`: `min(avail_out, pending_len)`.
pub fn w31a120_flush_pending_len(avail_out: u32, pending_len: u32) -> u32 {
    avail_out.min(pending_len)
}

/// `0x00c74b20`: read_buf style copy.  Copies `min(avail_in, size)` bytes
/// from `input` to `dst` and returns the new `avail_in`; returns `None`
/// (the original's 0 return) when nothing is copied.
pub fn w31a120_read_buf(input: &[u8], avail_in: u32, dst: &mut [u8], size: u32) -> Option<u32> {
    let n = avail_in.min(size);
    if n == 0 {
        return None;
    }
    dst[..n as usize].copy_from_slice(&input[..n as usize]);
    Some(avail_in - n)
}

/// `0x00c76de0`: stored-block emit.  Pushes the 3-bit header (value
/// `header`) into the bit buffer `(bi_buf, bi_valid)`, flushes to a byte
/// boundary, then writes LEN, NLEN (little endian) and the payload.
pub fn w31a120_stored_block(
    out: &mut Vec<u8>,
    bi_buf: &mut u16,
    bi_valid: &mut i32,
    data: &[u8],
    header: u16,
) {
    let len = data.len() as u16;
    let valid = *bi_valid;
    if valid < 14 {
        *bi_buf |= header << valid;
        *bi_valid = valid + 3;
    } else {
        let v = (header << valid) | *bi_buf;
        out.push(v as u8);
        out.push((v >> 8) as u8);
        *bi_buf = header >> (16 - valid);
        *bi_valid = valid - 13;
    }
    if *bi_valid > 8 {
        out.push(*bi_buf as u8);
        out.push((*bi_buf >> 8) as u8);
    } else if *bi_valid > 0 {
        out.push(*bi_buf as u8);
    }
    *bi_buf = 0;
    *bi_valid = 0;
    out.push(len as u8);
    out.push((len >> 8) as u8);
    out.push(!(len as u8));
    out.push(!((len >> 8) as u8));
    out.extend_from_slice(data);
}

#[cfg(test)]
mod tests_wave31_agent120 {
    use super::*;

    #[test]
    fn flush_len() {
        assert_eq!(w31a120_flush_pending_len(5, 9), 5);
        assert_eq!(w31a120_flush_pending_len(9, 5), 5);
    }

    #[test]
    fn read_buf() {
        let mut d = [0u8; 4];
        assert_eq!(w31a120_read_buf(&[1, 2, 3], 3, &mut d, 2), Some(1));
        assert_eq!(&d[..2], &[1, 2]);
        assert_eq!(w31a120_read_buf(&[], 0, &mut d, 2), None);
    }

    #[test]
    fn stored() {
        let (mut o, mut b, mut v) = (Vec::new(), 0u16, 0i32);
        w31a120_stored_block(&mut o, &mut b, &mut v, &[9, 8], 0);
        assert_eq!(o, vec![0, 2, 0, 0xfd, 0xff, 9, 8]);
    }
}
