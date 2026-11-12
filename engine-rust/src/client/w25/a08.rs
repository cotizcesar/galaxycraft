#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent08.rs
// ===========================================================================
// Wave 25 translation batch 08: C-runtime style helpers (string copy,
// CRC32, FILETIME / performance-counter time conversion).

/// Bounded C-string copy shared by the two copies below: copies at most
/// `cap - 1` bytes of the NUL-terminated `src`; `cap == 0` yields no copy.
fn w25a08_bounded_copy(src: &[u8], cap: usize) -> Option<Vec<u8>> {
    if cap == 0 {
        return None;
    }
    let len = src.iter().position(|&b| b == 0).unwrap_or(src.len());
    Some(src[..len.min(cap - 1)].to_vec())
}

/// `0x0092a130`: copies the name string stored at offset `0x14` of a looked
/// up record into a caller buffer of `cap` bytes (truncating to `cap - 1`).
/// `record_name` is `None` when the lookup fails (returns `None`).  The
/// caller's buffer is cleared first; the refcount release is not modelled.
pub fn w25a08_copy_record_name(record_name: Option<&[u8]>, cap: usize) -> Option<Vec<u8>> {
    let name = record_name?;
    Some(w25a08_bounded_copy(name, cap).unwrap_or_default())
}

/// `0x0092a380`: copies the string at offset `0x10` of a record into a
/// buffer of `cap` bytes, truncated to `cap - 1`.  Returns `Some(bytes)` on
/// success (success even when `cap == 0`, where nothing is copied).
pub fn w25a08_copy_record_string(record_string: &[u8], cap: usize) -> Option<Vec<u8>> {
    Some(w25a08_bounded_copy(record_string, cap).unwrap_or_default())
}

/// `0x0092feb0`: table-driven CRC32 step.  `flags & 1` seeds with
/// `0xffffffff`, `flags & 2` processes `data`, `flags & 4` inverts the
/// result.  `state` is the optional in/out running value.  The table is the
/// retail CRC table (`DAT_00f37a88`) supplied by the caller.
pub fn w25a08_crc32(data: &[u8], state: Option<&mut u32>, flags: u32, table: &[u32; 256]) -> u32 {
    let mut crc = state.as_ref().map_or(0, |s| **s);
    if flags & 1 != 0 {
        crc = 0xffff_ffff;
    }
    if flags & 2 != 0 {
        for &b in data {
            crc = (crc >> 8) ^ table[((b as u32 ^ crc) & 0xff) as usize];
        }
    }
    let out = if flags & 4 != 0 { !crc } else { crc };
    if let Some(s) = state {
        *s = out;
    }
    out
}

/// `0x0093f610`: converts a Windows FILETIME (100 ns ticks since 1601, split
/// lo/hi) to Unix seconds: subtract `0x019db1ded53e8000`, divide by
/// 10,000,000 (unsigned, wrapping).  `None` input yields `None` (the retail
/// code reports ERROR_INVALID_PARAMETER 0x57).
pub fn w25a08_filetime_to_unix(ft: Option<(u32, u32)>) -> Option<u64> {
    let (lo, hi) = ft?;
    let ticks = (((hi as u64) << 32) | lo as u64).wrapping_sub(0x019d_b1de_d53e_8000);
    Some(ticks / 10_000_000)
}

/// `0x0093f6d0`: milliseconds from a performance counter: `counter * 1000`
/// (wrapping 64-bit) divided by the frequency.  The frequency is cached in
/// `cached_freq`; when it is zero it is fetched through `query_freq` first.
/// Returns `None` if the frequency stays zero.
pub fn w25a08_counter_to_ms(
    counter: u64,
    cached_freq: &mut u64,
    query_freq: impl FnOnce() -> u64,
) -> Option<u64> {
    if *cached_freq == 0 {
        *cached_freq = query_freq();
    }
    if *cached_freq == 0 {
        return None;
    }
    Some(counter.wrapping_mul(1000) / *cached_freq)
}

/// `0x0093fb70`: closes a handle unless it is the invalid handle `-1`;
/// returns whether `close` was invoked.
pub fn w25a08_close_handle(handle: i32, close: impl FnOnce(i32)) -> bool {
    if handle == -1 {
        return false;
    }
    close(handle);
    true
}

#[cfg(test)]
mod tests_wave25_agent08 {
    use super::*;

    #[test]
    fn copies_truncate() {
        assert_eq!(w25a08_copy_record_name(None, 8), None);
        assert_eq!(
            w25a08_copy_record_name(Some(b"hello\0x"), 4),
            Some(b"hel".to_vec())
        );
        assert_eq!(
            w25a08_copy_record_name(Some(b"hi\0"), 8),
            Some(b"hi".to_vec())
        );
        assert_eq!(w25a08_copy_record_name(Some(b"hi\0"), 0), Some(vec![]));
        assert_eq!(w25a08_copy_record_string(b"abc\0", 3), Some(b"ab".to_vec()));
    }

    fn table() -> [u32; 256] {
        let mut t = [0u32; 256];
        for (i, e) in t.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xedb8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
            *e = c;
        }
        t
    }

    #[test]
    fn crc_check_value() {
        let t = table();
        assert_eq!(w25a08_crc32(b"123456789", None, 7, &t), 0xcbf4_3926);
        let mut st = 0;
        w25a08_crc32(b"1234", Some(&mut st), 3, &t);
        let r = w25a08_crc32(b"56789", Some(&mut st), 6, &t);
        assert_eq!(r, 0xcbf4_3926);
        assert_eq!(st, r);
    }

    #[test]
    fn time_conversions() {
        assert_eq!(w25a08_filetime_to_unix(None), None);
        let ft = 0x019d_b1de_d53e_8000u64 + 5 * 10_000_000;
        assert_eq!(
            w25a08_filetime_to_unix(Some((ft as u32, (ft >> 32) as u32))),
            Some(5)
        );
        let mut f = 0;
        assert_eq!(
            w25a08_counter_to_ms(3_000_000, &mut f, || 1_000_000),
            Some(3000)
        );
        assert_eq!(f, 1_000_000);
        let mut z = 0;
        assert_eq!(w25a08_counter_to_ms(1, &mut z, || 0), None);
    }

    #[test]
    fn close_handle_skips_invalid() {
        let mut got = None;
        assert!(!w25a08_close_handle(-1, |h| got = Some(h)));
        assert!(w25a08_close_handle(7, |h| got = Some(h)));
        assert_eq!(got, Some(7));
    }
}
