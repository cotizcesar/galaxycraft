#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent19.rs
// ===========================================================================
// Wave 27 translation batch 19 (audio/mixer glue helpers).

/// `0x009dd2a0`: validates a channel count (`1` or `2`) against the
/// capacity; on success the destination (`count * stride * 4` bytes) is
/// zeroed and `0` is returned, otherwise the error code `0x25` is returned.
pub fn w27a19_clear_channels(dst: &mut [u8], capacity: i32, channels: i32, stride: i32) -> u32 {
    if (channels.wrapping_sub(1) as u32) < 2 && channels <= capacity {
        let n = (capacity as i64 * stride as i64 * 4).max(0) as usize;
        let n = n.min(dst.len());
        dst[..n].fill(0);
        return 0;
    }
    0x25
}

/// Parsed GUID record produced by `0x009df520`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W27a19Guid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

fn w27a19_wide(src: &[u8], off: usize, chars: usize) -> Vec<u16> {
    (0..chars)
        .map(|i| {
            let p = off + i * 2;
            u16::from_le_bytes([
                src.get(p).copied().unwrap_or(0),
                src.get(p + 1).copied().unwrap_or(0),
            ])
        })
        .collect()
}

/// `0x009df520`: parses a wide-character GUID string (`src` bytes) into a
/// GUID.  Fields are 8, 4, 4, 2, 2 and six 2-char hex groups taken at byte
/// offsets `0x24`, `0x36`, `0x40`, `0x4a`, `0x4e` and `0x54 + 4*i`.  The
/// unresolved radix parser is supplied as `parse(text, radix)`.
pub fn w27a19_parse_guid(src: &[u8], parse: &dyn Fn(&[u16], u32) -> u32) -> W27a19Guid {
    let mut g = W27a19Guid {
        data1: parse(&w27a19_wide(src, 0x24, 8), 16),
        data2: parse(&w27a19_wide(src, 0x36, 4), 16) as u16,
        data3: parse(&w27a19_wide(src, 0x40, 4), 16) as u16,
        ..Default::default()
    };
    g.data4[0] = parse(&w27a19_wide(src, 0x4a, 2), 16) as u8;
    g.data4[1] = parse(&w27a19_wide(src, 0x4e, 2), 16) as u8;
    for i in 0..6 {
        g.data4[2 + i] = parse(&w27a19_wide(src, 0x54 + 4 * i, 2), 16) as u8;
    }
    g
}

#[cfg(test)]
mod tests_wave27_agent19 {
    use super::*;

    #[test]
    fn clear_channels_bounds() {
        let mut b = [1u8; 64];
        assert_eq!(w27a19_clear_channels(&mut b, 2, 2, 2), 0);
        assert_eq!(b[..16], [0; 16]);
        assert_eq!(b[16], 1);
        assert_eq!(w27a19_clear_channels(&mut b, 2, 3, 2), 0x25);
        assert_eq!(w27a19_clear_channels(&mut b, 1, 2, 2), 0x25);
        assert_eq!(w27a19_clear_channels(&mut b, 2, 0, 2), 0x25);
    }

    #[test]
    fn parse_guid_fields() {
        let s = "{00112233-4455-6677-8899-AABBCCDDEEFF}";
        let mut raw = vec![0u8; 0x24 - 2];
        for c in s.encode_utf16() {
            raw.extend_from_slice(&c.to_le_bytes());
        }
        raw.extend_from_slice(&[0; 8]);
        let p =
            |t: &[u16], r: u32| u32::from_str_radix(&String::from_utf16_lossy(t), r).unwrap_or(0);
        let g = w27a19_parse_guid(&raw, &p);
        assert_eq!(g.data1, 0x0011_2233);
        assert_eq!(g.data2, 0x4455);
        assert_eq!(g.data3, 0x6677);
        assert_eq!(g.data4[0], 0x88);
        assert_eq!(g.data4[1], 0x99);
        assert_eq!(g.data4[7], 0xFF);
    }
}
