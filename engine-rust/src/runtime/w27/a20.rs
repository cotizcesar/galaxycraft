#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent20.rs
// ===========================================================================
// Wave 27 translation batch 20: audio-library helpers (stream length math,
// Vorbis identification header).

/// `0x009e17b0` (tail computation): divisor selected from the codec mode
/// field at `+0x560` of the stream record.  Modes 1..=3 give `8 * mode`,
/// 4 and 5 give 32, everything else 0.
pub fn w27a20_bits_per_unit(mode: u32) -> u32 {
    match mode {
        1 => 8,
        2 => 0x10,
        3 => 0x18,
        4 | 5 => 0x20,
        _ => 0,
    }
}

/// `0x009e17b0`: computes `(bytes * 8) / bits_per_unit / channels`.  `kind`
/// is the field at `+0x5790` (values 2, 7..=9 and 1000 force a divisor of
/// 2); otherwise `field_570` is used.  Returns `None` on division by zero.
pub fn w27a20_stream_length(bytes: u32, mode: u32, kind: i32, field_570: u32) -> Option<u32> {
    let per = w27a20_bits_per_unit(mode) as u64;
    let channels: u64 = if kind < 10 {
        if kind < 7 && kind != 2 {
            field_570 as u64
        } else {
            2
        }
    } else if kind == 1000 {
        2
    } else {
        field_570 as u64
    };
    let bits = (bytes << 3) as u64;
    if per == 0 || channels == 0 {
        return None;
    }
    Some(((bits / per) / channels) as u32)
}

/// Parsed Vorbis identification fields (`0x009e4540`).
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct W27a20VorbisInfo {
    pub version: i32,
    pub channels: i32,
    pub rate: i32,
    pub bitrate_upper: i32,
    pub bitrate_nominal: i32,
    pub bitrate_lower: i32,
    pub blocksize0: i32,
    pub blocksize1: i32,
}

/// `0x009e4540` (packet type 1 branch): after the 8-bit type and the
/// six-byte `"vorbis"` magic, reads version(32, must be 0), channels(8),
/// rate, upper, nominal, lower (32 each), two 4-bit block sizes and the
/// framing bit.  Returns `None` when the magic, version, sizes or framing
/// are invalid.  `read(bits)` is the bit reader (`FUN_009f02c0`).
pub fn w27a20_parse_vorbis_ident(mut read: impl FnMut(u32) -> i32) -> Option<W27a20VorbisInfo> {
    if read(8) != 1 {
        return None;
    }
    let magic: Vec<u8> = (0..6).map(|_| read(8) as u8).collect();
    if magic != b"vorbis" {
        return None;
    }
    let mut info = W27a20VorbisInfo {
        version: read(32),
        ..Default::default()
    };
    if info.version != 0 {
        return None;
    }
    info.channels = read(8);
    info.rate = read(32);
    info.bitrate_upper = read(32);
    info.bitrate_nominal = read(32);
    info.bitrate_lower = read(32);
    info.blocksize0 = 1i32 << (read(4) & 0x1f);
    info.blocksize1 = 1i32 << (read(4) & 0x1f);
    if info.rate > 0
        && info.channels > 0
        && info.blocksize0 > 0x3f
        && info.blocksize0 <= info.blocksize1
        && info.blocksize1 < 0x2001
        && read(1) != 1
    {
        Some(info)
    } else {
        None
    }
}

#[cfg(test)]
mod tests_wave27_agent20 {
    use super::*;

    #[test]
    fn length_math() {
        assert_eq!(w27a20_bits_per_unit(3), 0x18);
        assert_eq!(w27a20_bits_per_unit(9), 0);
        assert_eq!(w27a20_stream_length(1000, 2, 2, 5), Some(8000 / 16 / 2));
        assert_eq!(w27a20_stream_length(1000, 4, 3, 5), Some(8000 / 32 / 5));
        assert_eq!(w27a20_stream_length(1000, 4, 1000, 5), Some(125));
        assert_eq!(w27a20_stream_length(1000, 0, 2, 5), None);
        assert_eq!(w27a20_stream_length(1000, 4, 3, 0), None);
    }

    #[test]
    fn vorbis_ident() {
        let mut v: Vec<i32> = vec![1];
        v.extend(b"vorbis".iter().map(|&b| b as i32));
        v.extend([0, 2, 44100, 0, 128000, 0, 8, 11, 0]);
        let mut it = v.clone().into_iter();
        let i = w27a20_parse_vorbis_ident(|_| it.next().unwrap()).unwrap();
        assert_eq!(
            (i.channels, i.rate, i.blocksize0, i.blocksize1),
            (2, 44100, 256, 2048)
        );
        let mut bad = v;
        bad[0] = 3;
        let mut it = bad.into_iter();
        assert!(w27a20_parse_vorbis_ident(|_| it.next().unwrap()).is_none());
    }
}
