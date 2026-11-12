#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent57.rs
// ===========================================================================
// Wave 31 agent 57: small runtime helpers (MD5 update, path root length, wrappers).

/// `0x00937ce0`: forwards to the 937760 worker with the constant flag
/// word `0x100000` inserted as the second argument.
pub fn w31a57_forward_flag_100000<R>(
    a: u32,
    b: u32,
    c: u32,
    worker: impl FnOnce(u32, u32, u32, u32) -> R,
) -> R {
    worker(a, 0x100000, b, c)
}

/// `0x0093ebe0`: forwards its argument to the 94bd20 worker with a zero
/// second argument.
pub fn w31a57_forward_zero<R>(a: u32, worker: impl FnOnce(u32, u32) -> R) -> R {
    worker(a, 0)
}

/// `0x00937d00`: takes the 64-bit pair produced by the formatter, ORs 1 into
/// the low word, calls the converter and maps negative results to -1.
pub fn w31a57_or1_pair_result(lo: u32, hi: u32, convert: impl FnOnce(u32, u32) -> i32) -> i32 {
    let r = convert(lo | 1, hi);
    if r < 0 { -1 } else { r }
}

/// `0x0093abb0`: passes the formatter's 64-bit pair to the converter and maps
/// negative results to -1.
pub fn w31a57_pair_result(lo: u32, hi: u32, convert: impl FnOnce(u32, u32) -> i32) -> i32 {
    let r = convert(lo, hi);
    if r < 0 { -1 } else { r }
}

/// MD5 context as used by `0x0093da90`: 64-bit bit count, 4-word state and a
/// 64-byte pending buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W31a57Md5Ctx {
    pub count: [u32; 2],
    pub state: [u32; 4],
    pub buffer: [u8; 64],
}

impl Default for W31a57Md5Ctx {
    fn default() -> Self {
        Self {
            count: [0; 2],
            state: [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476],
            buffer: [0; 64],
        }
    }
}

/// `0x0093da90`: MD5 update.  Adds `data.len() * 8` to the bit count, fills
/// and flushes the pending buffer, transforms whole 64-byte blocks directly
/// and stashes the tail.  `transform` stands in for the block function.
pub fn w31a57_md5_update(
    ctx: &mut W31a57Md5Ctx,
    data: &[u8],
    mut transform: impl FnMut(&mut [u32; 4], &[u8; 64]),
) {
    let len = data.len() as u32;
    let mut index = ((ctx.count[0] >> 3) & 0x3f) as usize;
    let bits = (data.len() as u64).wrapping_mul(8);
    let (old, add) = (ctx.count[0], bits as u32);
    ctx.count[0] = old.wrapping_add(add);
    ctx.count[1] = ctx.count[1]
        .wrapping_add((bits >> 32) as u32)
        .wrapping_add(u32::from(ctx.count[0] < old));
    let mut pos = 0usize;
    let mut remaining = len as usize;
    if index != 0 {
        if index + remaining < 0x40 {
            ctx.buffer[index..index + remaining].copy_from_slice(&data[..remaining]);
            return;
        }
        let n = 0x40 - index;
        ctx.buffer[index..].copy_from_slice(&data[..n]);
        let buf = ctx.buffer;
        transform(&mut ctx.state, &buf);
        pos = n;
        remaining -= n;
        index = 0;
    }
    while remaining >= 0x40 {
        let mut blk = [0u8; 64];
        blk.copy_from_slice(&data[pos..pos + 0x40]);
        transform(&mut ctx.state, &blk);
        pos += 0x40;
        remaining -= 0x40;
    }
    ctx.buffer[index..index + remaining].copy_from_slice(&data[pos..pos + remaining]);
}

/// `0x0093f2c0`: length of the root prefix of a path: 3 for `X:\` / `X:/`,
/// 2 for a bare `X:x`, for a UNC path (`\\host\share`) the offset just past
/// the share separator (or the whole length), otherwise 0.  Strings of
/// length < 2 yield 0.
pub fn w31a57_path_root_len(path: &[u8]) -> usize {
    let len = path.iter().position(|&b| b == 0).unwrap_or(path.len());
    let p = &path[..len];
    if len <= 1 {
        return 0;
    }
    if p[1] == b':' {
        return if len > 2 && (p[2] == b'\\' || p[2] == b'/') {
            3
        } else {
            2
        };
    }
    let c = p[0];
    if (c == b'\\' || c == b'/') && c == p[1] {
        let mut cur: Option<usize> = Some(1);
        for _ in 0..2 {
            if let Some(i) = cur {
                cur = p[i + 1..].iter().position(|&b| b == c).map(|k| i + 1 + k);
            }
        }
        return match cur {
            Some(i) => i + 1,
            None => len,
        };
    }
    0
}

/// `0x00940860`: when the new flag differs from the cached one, calls the
/// setter with the handle and either 0 (flag clear) or the stored value, then
/// caches the flag.  Returns whether the setter ran.
pub fn w31a57_set_cached_flag(
    cached: &mut u8,
    flag: u8,
    stored_value: u32,
    setter: impl FnOnce(u32),
) -> bool {
    if flag == *cached {
        return false;
    }
    setter(if flag != 0 { stored_value } else { 0 });
    *cached = flag;
    true
}

#[cfg(test)]
mod tests_wave31_agent57 {
    use super::*;

    #[test]
    fn forwarders() {
        assert_eq!(
            w31a57_forward_flag_100000(1, 2, 3, |a, f, b, c| [a, f, b, c]),
            [1, 0x100000, 2, 3]
        );
        assert_eq!(w31a57_forward_zero(9, |a, z| (a, z)), (9, 0));
        assert_eq!(w31a57_or1_pair_result(2, 5, |l, h| (l + h) as i32), 8);
        assert_eq!(w31a57_or1_pair_result(2, 5, |_, _| -4), -1);
        assert_eq!(w31a57_pair_result(2, 5, |l, h| (l + h) as i32), 7);
        assert_eq!(w31a57_pair_result(0, 0, |_, _| -9), -1);
    }

    #[test]
    fn md5_update_blocks() {
        let mut c = W31a57Md5Ctx::default();
        let mut n = 0;
        w31a57_md5_update(&mut c, &[1u8; 10], |_, _| n += 1);
        assert_eq!((n, c.count[0], c.buffer[9]), (0, 80, 1));
        w31a57_md5_update(&mut c, &[2u8; 200], |_, _| n += 1);
        assert_eq!(n, 3);
        assert_eq!(c.count[0], 80 + 1600);
        assert_eq!(
            c.buffer[..10 + 200 - 192]
                .iter()
                .filter(|&&b| b == 2)
                .count(),
            18
        );
    }

    #[test]
    fn path_root() {
        assert_eq!(w31a57_path_root_len(b"C:\\x\0"), 3);
        assert_eq!(w31a57_path_root_len(b"C:x\0"), 2);
        assert_eq!(w31a57_path_root_len(b"\\\\h\\s\\f\0"), 6);
        assert_eq!(w31a57_path_root_len(b"\\\\h\0"), 3);
        assert_eq!(w31a57_path_root_len(b"abc\0"), 0);
        assert_eq!(w31a57_path_root_len(b"a\0"), 0);
    }

    #[test]
    fn cached_flag() {
        let mut cached = 0u8;
        let mut got = None;
        assert!(w31a57_set_cached_flag(&mut cached, 1, 7, |v| got = Some(v)));
        assert_eq!((cached, got), (1, Some(7)));
        assert!(!w31a57_set_cached_flag(&mut cached, 1, 7, |_| panic!()));
        assert!(w31a57_set_cached_flag(&mut cached, 0, 7, |v| got = Some(v)));
        assert_eq!(got, Some(0));
    }
}
