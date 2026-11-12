#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave30/agent08.rs
// ===========================================================================
// Wave 30 agent 08: tact encoder/header helpers (pure reference helpers).

/// Result of the record probe in `0x00b0cf80`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W30a08Probe {
    /// More bytes are required (the `param_5` output); value is the total length needed.
    Need(u32),
    /// Record accepted; value is the consumed length (the `param_4` output).
    Accepted(u32),
    /// Record rejected with no output written.
    Rejected,
}

/// `0x00b0cf80`: probes an `'E'` record: `E, n, key[n], m, tag[m], 'S'|'A'`.
/// `n != 0`, `m <= 0x10`; `verify` receives the first `n` key bytes (after
/// the two header bytes) and returns the produced length, accepted when it
/// is 0 or <= 0x10.  Fewer than 3 bytes requests 3.
#[allow(clippy::type_complexity)]
pub fn w30a08_probe_record(data: &[u8], verify: Option<&dyn Fn(&[u8]) -> u32>) -> W30a08Probe {
    let len = data.len() as u32;
    if len < 3 {
        return W30a08Probe::Need(3);
    }
    if data[0] != b'E' || data[1] == 0 {
        return W30a08Probe::Rejected;
    }
    let n = data[1] as u32;
    let mut need = n + 3;
    if need <= len {
        let m = data[(n + 2) as usize] as u32;
        if m > 0x10 {
            return W30a08Probe::Rejected;
        }
        need = m + 4 + n;
        if need <= len {
            let tag = data[(n + m + 3) as usize];
            if tag == b'S' || tag == b'A' {
                let ok = match verify {
                    None => true,
                    Some(f) => {
                        let r = f(&data[2..2 + n as usize]);
                        r == 0 || r < 0x11
                    }
                };
                if ok {
                    return W30a08Probe::Accepted(need);
                }
            }
            return W30a08Probe::Rejected;
        }
    }
    W30a08Probe::Need(need)
}

/// `0x00b0f000`: serialises a BLTE header.  Layout: `"BLTE"` (LE u32
/// `0x45544c42`), big-endian header size `n * 0x18 + 0xc`, flag byte `0x0f`,
/// 24-bit big-endian chunk count, then per chunk two big-endian u32 and a
/// raw 16-byte hash.  Returns the header bytes.
pub fn w30a08_write_blte_header(chunks: &[(u32, u32, [u8; 16])]) -> Vec<u8> {
    let n = chunks.len() as u32;
    let size = n * 0x18 + 0xc;
    let mut out = Vec::with_capacity(size as usize);
    out.extend_from_slice(&0x4554_4c42u32.to_le_bytes());
    out.extend_from_slice(&size.to_be_bytes());
    out.push(0x0f);
    out.extend_from_slice(&n.to_be_bytes()[1..]);
    for (a, b, h) in chunks {
        out.extend_from_slice(&a.to_be_bytes());
        out.extend_from_slice(&b.to_be_bytes());
        out.extend_from_slice(h);
    }
    out
}

/// `0x00b0f7f0`: LZ4 encoder block-size validity: power of two within
/// `0x400..=0x10000`.
pub fn w30a08_lz4_block_size_valid(size: u32) -> bool {
    (size & size.wrapping_sub(1)) == 0 && size.wrapping_sub(0x400) < 0xfc01
}

/// `0x00b0f7f0`: bytes of the input the encoder may take this round:
/// `min(wanted_remaining, block - filled)` once `filled < block`, else 0.
pub fn w30a08_lz4_take(block: u32, filled: u32, wanted_remaining: u32) -> u32 {
    if filled < block {
        wanted_remaining.min(block - filled)
    } else {
        0
    }
}

/// `0x00b0f7f0`: the "final chunk" flag and the usable input length.
/// Returns `(final, limit)` where `final` iff `hi_diff == (lo < avail_lo)`
/// and `total - consumed <= request`.
pub fn w30a08_lz4_final(
    total: u32,
    total_hi: u32,
    consumed: u32,
    consumed_hi: u32,
    request: u32,
) -> (bool, u32) {
    let fin = total_hi.wrapping_sub(consumed_hi) == u32::from(total < consumed)
        && total.wrapping_sub(consumed) <= request;
    let limit = if fin {
        total.wrapping_sub(consumed)
    } else {
        request
    };
    (fin, limit)
}

#[cfg(test)]
mod tests_wave30_agent08 {
    use super::*;

    #[test]
    fn probe() {
        assert_eq!(w30a08_probe_record(&[1, 2], None), W30a08Probe::Need(3));
        assert_eq!(w30a08_probe_record(b"Xab", None), W30a08Probe::Rejected);
        assert_eq!(w30a08_probe_record(b"E\x02ab", None), W30a08Probe::Need(5));
        assert_eq!(
            w30a08_probe_record(b"E\x02ab\x11zz", None),
            W30a08Probe::Rejected
        );
        assert_eq!(
            w30a08_probe_record(b"E\x02ab\x01x", None),
            W30a08Probe::Need(7)
        );
        assert_eq!(
            w30a08_probe_record(b"E\x02ab\x01xS", None),
            W30a08Probe::Accepted(7)
        );
        assert_eq!(
            w30a08_probe_record(b"E\x02ab\x00A", None),
            W30a08Probe::Accepted(6)
        );
        assert_eq!(
            w30a08_probe_record(b"E\x02ab\x00Q", None),
            W30a08Probe::Rejected
        );
        let bad = |_: &[u8]| 0x11;
        assert_eq!(
            w30a08_probe_record(b"E\x02ab\x00A", Some(&bad)),
            W30a08Probe::Rejected
        );
        let good = |k: &[u8]| k.len() as u32;
        assert_eq!(
            w30a08_probe_record(b"E\x02ab\x00A", Some(&good)),
            W30a08Probe::Accepted(6)
        );
    }

    #[test]
    fn blte() {
        let h = w30a08_write_blte_header(&[(0x01020304, 0x05060708, [9; 16])]);
        assert_eq!(h.len(), 0x24);
        assert_eq!(&h[..4], b"BLTE");
        assert_eq!(&h[4..8], &[0, 0, 0, 0x24]);
        assert_eq!(h[8], 0x0f);
        assert_eq!(&h[9..12], &[0, 0, 1]);
        assert_eq!(&h[12..20], &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(&h[20..], &[9; 16]);
        assert_eq!(w30a08_write_blte_header(&[]).len(), 12);
    }

    #[test]
    fn lz4() {
        assert!(w30a08_lz4_block_size_valid(0x400));
        assert!(w30a08_lz4_block_size_valid(0x10000));
        assert!(!w30a08_lz4_block_size_valid(0x200));
        assert!(!w30a08_lz4_block_size_valid(0x20000));
        assert!(!w30a08_lz4_block_size_valid(0x600));
        assert!(!w30a08_lz4_block_size_valid(0));
        assert_eq!(w30a08_lz4_take(0x400, 0x300, 0x500), 0x100);
        assert_eq!(w30a08_lz4_take(0x400, 0x400, 5), 0);
        assert_eq!(w30a08_lz4_take(0x400, 0, 5), 5);
        assert_eq!(w30a08_lz4_final(10, 0, 4, 0, 6), (true, 6));
        assert_eq!(w30a08_lz4_final(10, 0, 4, 0, 5), (false, 5));
    }
}
