#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent18.rs
// ===========================================================================
// Wave 27 translation batch 18: audio output helpers (WAV header writer and
// multichannel float downmix).

/// `0x009d4cd0`: folds interleaved float frames into one sample per frame.
/// `out_count` is the output length (param_1), `channels` the frame width
/// (param_2).  One channel is a plain copy of `out_count` floats; six and
/// eight channels only process `out_count >> 2` blocks of four frames
/// (the remainder is untouched); every other width sums each frame, with a
/// zero seed (zero when `channels <= 0`).
pub fn w27a18_downmix(out: &mut [f32], input: &[f32], out_count: usize, channels: i32) {
    if channels == 1 {
        out[..out_count].copy_from_slice(&input[..out_count]);
        return;
    }
    if channels == 6 || channels == 8 {
        let ch = channels as usize;
        for i in 0..(out_count >> 2) * 4 {
            let f = &input[i * ch..i * ch + ch];
            let mut s = f[0] + f[1];
            for v in &f[2..] {
                s += *v;
            }
            out[i] = s;
        }
        return;
    }
    for i in 0..out_count {
        out[i] = 0.0;
        if channels > 0 {
            let ch = channels as usize;
            let mut s = 0.0f32;
            for v in &input[i * ch..i * ch + ch] {
                s += *v;
            }
            out[i] = s;
        }
    }
}

/// `0x009d0680`: builds the 68-byte RIFF/WAVE header written before the PCM
/// data: "RIFF", size `data_bytes + 0x38`, "WAVE", "fmt " (0x28), a 40-byte
/// WAVEFORMATEXTENSIBLE, "data", `data_bytes`.  `is_float` is format kind 5.
/// The tag is 0xfffe (extensible, GUID data1 3 for float / 1) only when
/// float with more than two channels, otherwise 3 (float) or 1 (PCM) with
/// zeroed extension fields.  The "WAVE" tag literal is read from a global
/// (assumed ASCII "WAVE").
pub fn w27a18_wav_header(
    channels: i32,
    sample_rate: i32,
    bits: i32,
    data_bytes: i32,
    is_float: bool,
) -> Vec<u8> {
    let tag: u16 = if is_float && channels > 2 {
        0xfffe
    } else if is_float {
        3
    } else {
        1
    };
    let avg = {
        let x = bits.wrapping_mul(sample_rate).wrapping_mul(channels);
        x.wrapping_add((x >> 31) & 7) >> 3
    };
    let block = {
        let x = bits.wrapping_mul(channels);
        x.wrapping_add((x >> 31) & 7) >> 3
    };
    let mut fmt = [0u8; 40];
    fmt[0..2].copy_from_slice(&tag.to_le_bytes());
    fmt[2..4].copy_from_slice(&(channels as u16).to_le_bytes());
    fmt[4..8].copy_from_slice(&sample_rate.to_le_bytes());
    fmt[8..12].copy_from_slice(&avg.to_le_bytes());
    fmt[12..14].copy_from_slice(&(block as u16).to_le_bytes());
    fmt[14..16].copy_from_slice(&(bits as u16).to_le_bytes());
    if tag == 0xfffe {
        fmt[16..18].copy_from_slice(&0x16u16.to_le_bytes());
        fmt[18..20].copy_from_slice(&(bits as u16).to_le_bytes());
        let d1: u32 = if is_float { 3 } else { 1 };
        fmt[24..28].copy_from_slice(&d1.to_le_bytes());
        fmt[28..32].copy_from_slice(&0x0010_0000u32.to_le_bytes());
        fmt[32..36].copy_from_slice(&0xaa00_0080u32.to_le_bytes());
        fmt[36..40].copy_from_slice(&0x719b_3800u32.to_le_bytes());
    }
    let mut out = Vec::with_capacity(68);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&data_bytes.wrapping_add(0x38).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&0x28u32.to_le_bytes());
    out.extend_from_slice(&fmt);
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_bytes.to_le_bytes());
    out
}

#[cfg(test)]
mod tests_wave27_agent18 {
    use super::*;

    #[test]
    fn downmix_modes() {
        let inp: Vec<f32> = (0..48).map(|x| x as f32).collect();
        let mut o = [9.0f32; 8];
        w27a18_downmix(&mut o, &inp, 3, 1);
        assert_eq!(&o[..4], &[0.0, 1.0, 2.0, 9.0]);
        w27a18_downmix(&mut o, &inp, 5, 6);
        assert_eq!(o[0], 15.0);
        assert_eq!(o[3], 18.0 * 6.0 + 15.0);
        assert_eq!(o[4], 9.0);
        w27a18_downmix(&mut o, &inp, 2, 2);
        assert_eq!(&o[..2], &[1.0, 5.0]);
        w27a18_downmix(&mut o, &inp, 2, 0);
        assert_eq!(&o[..2], &[0.0, 0.0]);
        let mut o8 = [0.0f32; 4];
        w27a18_downmix(&mut o8, &inp, 4, 8);
        assert_eq!(o8[1], (8..16).sum::<i32>() as f32);
    }

    #[test]
    fn wav_header() {
        let h = w27a18_wav_header(2, 44100, 16, 1000, false);
        assert_eq!(h.len(), 68);
        assert_eq!(&h[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(h[4..8].try_into().unwrap()), 1000 + 0x38);
        assert_eq!(u16::from_le_bytes(h[20..22].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(h[28..32].try_into().unwrap()), 176400);
        assert_eq!(u16::from_le_bytes(h[32..34].try_into().unwrap()), 4);
        let e = w27a18_wav_header(6, 48000, 32, 8, true);
        assert_eq!(u16::from_le_bytes(e[20..22].try_into().unwrap()), 0xfffe);
        assert_eq!(u32::from_le_bytes(e[44..48].try_into().unwrap()), 3);
        let f = w27a18_wav_header(2, 48000, 32, 8, true);
        assert_eq!(u16::from_le_bytes(f[20..22].try_into().unwrap()), 3);
    }
}
