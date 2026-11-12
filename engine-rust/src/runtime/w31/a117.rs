#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent117.rs
// ===========================================================================
// Wave 31 agent 117: stdext stream helpers (memory stream read clamp, seek).

/// Shared clamp used by the memory-stream read thunks: number of bytes that
/// may be copied given the cursor `pos`, the end offset `end` and the
/// requested count.
#[allow(dead_code)]
fn w31a117_clamp(pos: u32, end: u32, want: u32) -> u32 {
    let avail = end.wrapping_sub(pos);
    if avail < want { avail } else { want }
}

/// `0x00c6c930`: memory-stream read; copies `min(want, end - pos)` bytes from
/// the cursor into `dst` and returns the count copied.
#[allow(dead_code)]
pub fn w31a117_mem_read(data: &[u8], pos: u32, end: u32, want: u32, dst: &mut [u8]) -> u32 {
    let n = w31a117_clamp(pos, end, want) as usize;
    let p = pos as usize;
    dst[..n].copy_from_slice(&data[p..p + n]);
    n as u32
}

/// `0x00c6c980`: second copy of the same read thunk (other base subobject
/// adjustment); identical clamped copy.
#[allow(dead_code)]
pub fn w31a117_mem_read_alt(data: &[u8], pos: u32, end: u32, want: u32, dst: &mut [u8]) -> u32 {
    w31a117_mem_read(data, pos, end, want, dst)
}

/// `0x00c6ca90`: memory-stream write; copies `min(len, end - pos)` bytes from
/// `src` into the buffer at the cursor and returns the count copied.
#[allow(dead_code)]
pub fn w31a117_mem_write(data: &mut [u8], pos: u32, end: u32, src: &[u8]) -> u32 {
    let n = w31a117_clamp(pos, end, src.len() as u32) as usize;
    let p = pos as usize;
    data[p..p + n].copy_from_slice(&src[..n]);
    n as u32
}

/// `0x00c6c9d0`: stream seek.  `whence` 0 is absolute, 1 is relative to
/// `tell`, anything else is relative to `size`.  Targets beyond `size` fail
/// ("Illegal attempt to seek outside of stream bounds"); otherwise the
/// target is applied through `set` and returned.
#[allow(dead_code)]
pub fn w31a117_seek(
    whence: i32,
    offset: u32,
    tell: u32,
    size: u32,
    set: &mut dyn FnMut(u32),
) -> Result<u32, &'static str> {
    let mut target = offset;
    if whence != 0 {
        target = if whence == 1 { tell } else { size }.wrapping_add(offset);
    }
    if size < target {
        return Err("Illegal attempt to seek outside of stream bounds");
    }
    set(target);
    Ok(target)
}

/// `0x00c6d0b0`: file-stream relative skip (move method 1, current); returns
/// `amount` on success, `Err` when the OS seek failed.
#[allow(dead_code)]
pub fn w31a117_file_skip(
    amount: u32,
    seek_cur: &mut dyn FnMut(u32) -> bool,
) -> Result<u32, &'static str> {
    if seek_cur(amount) {
        Ok(amount)
    } else {
        Err("seek failed")
    }
}

#[cfg(test)]
mod tests_wave31_agent117 {
    use super::*;

    #[test]
    fn read_write_clamp() {
        let data = [1u8, 2, 3, 4, 5];
        let mut d = [0u8; 8];
        assert_eq!(w31a117_mem_read(&data, 3, 5, 10, &mut d), 2);
        assert_eq!(&d[..2], &[4, 5]);
        assert_eq!(w31a117_mem_read_alt(&data, 0, 5, 3, &mut d), 3);
        let mut buf = [0u8; 4];
        assert_eq!(w31a117_mem_write(&mut buf, 2, 4, &[9, 8, 7]), 2);
        assert_eq!(buf, [0, 0, 9, 8]);
    }

    #[test]
    fn seek_modes() {
        let mut cur = 0;
        assert_eq!(w31a117_seek(0, 5, 2, 10, &mut |p| cur = p), Ok(5));
        assert_eq!(cur, 5);
        assert_eq!(w31a117_seek(1, 3, 2, 10, &mut |p| cur = p), Ok(5));
        assert_eq!(w31a117_seek(2, 0, 2, 10, &mut |p| cur = p), Ok(10));
        assert!(w31a117_seek(0, 11, 2, 10, &mut |_| {}).is_err());
    }

    #[test]
    fn skip() {
        assert_eq!(w31a117_file_skip(7, &mut |_| true), Ok(7));
        assert!(w31a117_file_skip(7, &mut |_| false).is_err());
    }
}
