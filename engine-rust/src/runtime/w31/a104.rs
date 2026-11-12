#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent104.rs
// ===========================================================================
// Wave 31 agent 104: small pure helpers recovered from the DX renderer
// middleware block (`0x00c3xxxx`).  Everything else in the assignment is
// COM-vtable glue, destructor chains or non-returning CRT calls and was
// skipped.

/// `0x00c32290` (formatting half): one line of the texture-view dump.
/// `packed` is the dword at `+8` of a 16-byte view record, `slot` the dword
/// at `+0xc`.  Fields: first mip (4 bits), last mip (4 bits), first array
/// slice (11 bits at 8), last array slice (11 bits at 19).
pub fn w31a104_view_dump_line(label: &str, packed: u32, slot: u32) -> String {
    format!(
        "{} firstMip={}, lastMip={}, firstArraySlice={}, lastArraySlice={}, slot={}\n",
        label,
        packed & 0xf,
        (packed >> 4) & 0xf,
        (packed >> 8) & 0x7ff,
        (packed >> 0x13) & 0x7ff,
        slot
    )
}

/// `0x00c336c0`: gathers up to four bit-fields out of a packed byte blob into
/// a 4-dword destination.  When `enabled` is false the four dwords of `src`
/// are copied verbatim.  `desc` is the 12-byte descriptor (four triples of
/// destination index, bit offset, bit width; width 0 skips a field).
/// Reads past the end of `src` are treated as zero bytes.
pub fn w31a104_gather_bitfields(enabled: bool, desc: &[u8; 12], src: &[u8], dst: &mut [u32; 4]) {
    let byte = |i: usize| src.get(i).copied().unwrap_or(0);
    if !enabled {
        for (i, d) in dst.iter_mut().enumerate() {
            *d = u32::from_le_bytes([
                byte(i * 4),
                byte(i * 4 + 1),
                byte(i * 4 + 2),
                byte(i * 4 + 3),
            ]);
        }
        return;
    }
    for g in 0..4 {
        let (idx, off, width) = (
            desc[g * 3] as usize,
            desc[g * 3 + 1] as usize,
            desc[g * 3 + 2],
        );
        if width == 0 {
            continue;
        }
        let at = off >> 3;
        let mut v =
            u32::from_le_bytes([byte(at), byte(at + 1), byte(at + 2), byte(at + 3)]) >> (off & 7);
        if width < 0x20 {
            v &= !(u32::MAX << width);
        }
        if let Some(d) = dst.get_mut(idx) {
            *d = v;
        }
    }
}

#[cfg(test)]
mod tests_wave31_agent104 {
    use super::*;

    #[test]
    fn view_line() {
        let packed = 3 | (5 << 4) | (7 << 8) | (9 << 0x13);
        assert_eq!(
            w31a104_view_dump_line("t", packed, 2),
            "t firstMip=3, lastMip=5, firstArraySlice=7, lastArraySlice=9, slot=2\n"
        );
    }

    #[test]
    fn gather() {
        let src = [0xAB, 0xCD, 0xEF, 0x01, 0, 0, 0, 0];
        let mut d = [0xffff_ffffu32; 4];
        w31a104_gather_bitfields(false, &[0; 12], &src, &mut d);
        assert_eq!(
            d,
            [0x01EF_CDAB, 0, 0xffff_ffff, 0xffff_ffff].map(|x| if x == 0xffff_ffff {
                0
            } else {
                x
            })
        );
        let desc = [1, 4, 8, 0, 0, 0, 3, 8, 32, 0, 0, 0];
        let mut d = [9u32; 4];
        w31a104_gather_bitfields(true, &desc, &src, &mut d);
        assert_eq!(d, [9, 0xDA, 9, 0x01EF_CDAB >> 8 | 0]);
    }
}
