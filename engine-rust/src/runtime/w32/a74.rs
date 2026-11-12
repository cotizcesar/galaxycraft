#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent74.rs
// ===========================================================================
// Wave 32 translation batch 74: FMOD glue (shutdown callback, name copy, no-sound driver name).

/// `0x009c2f50`: shutdown/close hook.  Calls the object-level callback
/// (`+0x1d0`) if set, else the fallback from the parent (`+0x5734`), with the
/// two user words (`+0x1e4`, `+0x1e8`); then releases the handle at `+0x1c8`
/// through `release` and clears it.  Always returns 0.
pub fn w32a74_close_hook(
    own_cb: Option<&dyn Fn(u32, u32)>,
    parent_cb: Option<&dyn Fn(u32, u32)>,
    user: (u32, u32),
    handle: &mut u32,
    release: &mut dyn FnMut(u32),
) -> u32 {
    if let Some(cb) = own_cb.or(parent_cb) {
        cb(user.0, user.1);
    }
    if *handle != 0 {
        release(*handle);
        *handle = 0;
    }
    0
}

/// `0x009c57f0`: copies a NUL-terminated name into `dst[1..]` (up to 0x100
/// bytes, zero padded).  Returns 0x25 (invalid param) when the length exceeds
/// 0xff.
pub fn w32a74_set_name(dst: &mut [u8], name: &[u8]) -> u32 {
    let len = name.iter().position(|&b| b == 0).unwrap_or(name.len());
    if len > 0xff {
        return 0x25;
    }
    for i in 0..0x100usize {
        if let Some(d) = dst.get_mut(1 + i) {
            *d = if i < len { name[i] } else { 0 };
        }
    }
    0
}

/// `0x009cfb40`: reports the driver name "NoSound Driver" into `out`
/// (copy of at most `max` bytes, NUL padded).  Returns 0.
pub fn w32a74_nosound_driver_name(out: &mut [u8], max: usize) -> u32 {
    let src = b"NoSound Driver\0";
    for i in 0..max.min(out.len()) {
        out[i] = src.get(i).copied().unwrap_or(0);
    }
    0
}

#[cfg(test)]
mod tests_wave32_agent74 {
    use super::*;

    #[test]
    fn close_hook_prefers_own() {
        let c = std::cell::Cell::new(0);
        let own = |a: u32, b: u32| c.set(a + b);
        let par = |_: u32, _: u32| c.set(99);
        let mut h = 5;
        let mut rel = 0;
        let r = w32a74_close_hook(Some(&own), Some(&par), (1, 2), &mut h, &mut |x| rel = x);
        assert_eq!((r, c.get(), h, rel), (0, 3, 0, 5));
    }

    #[test]
    fn set_name_limits() {
        let mut d = vec![0xffu8; 0x110];
        assert_eq!(w32a74_set_name(&mut d, b"ab\0"), 0);
        assert_eq!(&d[1..4], b"ab\0");
        let long = vec![b'x'; 0x100];
        assert_eq!(w32a74_set_name(&mut d, &long), 0x25);
    }

    #[test]
    fn nosound() {
        let mut o = [1u8; 20];
        assert_eq!(w32a74_nosound_driver_name(&mut o, 20), 0);
        assert_eq!(&o[..14], b"NoSound Driver");
        assert_eq!(o[15], 0);
    }
}
