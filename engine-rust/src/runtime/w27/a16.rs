#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent16.rs
// ===========================================================================
// Wave 27 translation batch 16: FMOD-style DSP unit helpers (state resets,
// soft clipper, biquad coefficients).  Unit state is modelled as a slice of
// 32-bit words indexed by `byte_offset / 4`.

/// Zeroing request produced by a reset: `(base_word_value, byte_len)` of the
/// buffer that the original clears with the CRT fill helper.
pub type W27a16Fill = Option<(u32, u32)>;

#[inline]
fn w27a16_w(s: &[u32], off: usize) -> u32 {
    s[off / 4]
}

#[inline]
fn w27a16_set(s: &mut [u32], off: usize, v: u32) {
    s[off / 4] = v;
}

/// `0x009ae2b0`: DSP reset.  Clears the fill counters at `0x188/0x18c`,
/// copies the "initial" words (`0x144`, `0x154`, ...) into the live words and
/// returns the buffer (`0x178`, length `0x184`) to zero when present.
pub fn w27a16_reset_delay_unit(s: &mut [u32]) -> W27a16Fill {
    w27a16_set(s, 0x18c, 0);
    w27a16_set(s, 0x188, 0);
    let fill = (w27a16_w(s, 0x178) != 0).then(|| (w27a16_w(s, 0x178), w27a16_w(s, 0x184)));
    for (dst, src) in [
        (0x140, 0x144),
        (0x148, 0x154),
        (0x14c, 0x158),
        (0x150, 0x15c),
        (0x168, 0x16c),
        (0x170, 0x174),
        (0x138, 0x13c),
        (0x160, 0x164),
        (0x19c, 0x198),
    ] {
        let v = w27a16_w(s, src);
        w27a16_set(s, dst, v);
    }
    fill
}

/// `0x009b26f0`: DSP reset for the smaller unit variant (counters
/// `0x168/0x16c`, buffer `0x15c` of length `0x164`).
pub fn w27a16_reset_small_unit(s: &mut [u32]) -> W27a16Fill {
    w27a16_set(s, 0x16c, 0);
    w27a16_set(s, 0x168, 0);
    let fill = (w27a16_w(s, 0x15c) != 0).then(|| (w27a16_w(s, 0x15c), w27a16_w(s, 0x164)));
    for (dst, src) in [
        (0x140, 0x144),
        (0x148, 0x14c),
        (0x138, 0x13c),
        (0x150, 0x154),
        (0x178, 0x174),
    ] {
        let v = w27a16_w(s, src);
        w27a16_set(s, dst, v);
    }
    fill
}

/// `0x009b11e0`: reset of the 16-tap unit.  Clears `0x210`, rebuilds the 16
/// words at `0x214..0x254` as `ref - tap` (0 when the tap at `0x1c0..` is
/// zero) with `ref` at `0x20c`, and returns the buffer (`0x200`, length
/// `0x208`) to zero when present.
pub fn w27a16_reset_tap_unit(s: &mut [u32]) -> W27a16Fill {
    w27a16_set(s, 0x210, 0);
    let reference = w27a16_w(s, 0x20c);
    for i in 0..16 {
        let tap = w27a16_w(s, 0x214 + i * 4 - 0x15 * 4);
        let v = if tap == 0 {
            0
        } else {
            (reference as i32).wrapping_sub(tap as i32) as u32
        };
        w27a16_set(s, 0x214 + i * 4, v);
    }
    (w27a16_w(s, 0x200) != 0).then(|| (w27a16_w(s, 0x200), w27a16_w(s, 0x208)))
}

/// `0x009aff40`: clears the fill counter at `0x14c` and returns the buffer
/// (`0x140`, length `0x148`) to zero when its pointer is non-null.
pub fn w27a16_clear_single_buffer(s: &mut [u32]) -> W27a16Fill {
    w27a16_set(s, 0x14c, 0);
    (w27a16_w(s, 0x140) != 0).then(|| (w27a16_w(s, 0x140), w27a16_w(s, 0x148)))
}

/// `0x009b4e40`: clears the counters `0x15c/0x160` and returns up to two
/// buffers to zero: (`0x144`, len `0x154`) and (`0x148`, len `0x158`).  The
/// original returns after the first non-null buffer; the second is reported
/// separately so the caller can apply both.
pub fn w27a16_clear_dual_buffer(s: &mut [u32]) -> [W27a16Fill; 2] {
    w27a16_set(s, 0x15c, 0);
    w27a16_set(s, 0x160, 0);
    [
        (w27a16_w(s, 0x144) != 0).then(|| (w27a16_w(s, 0x144), w27a16_w(s, 0x154))),
        (w27a16_w(s, 0x148) != 0).then(|| (w27a16_w(s, 0x148), w27a16_w(s, 0x158))),
    ]
}

/// `0x009ae9b0`: soft clipper.  `amount >= 1` uses the constant 19994.682,
/// otherwise `2a / (1 - a)`.  Interleaved `channels`; channel `c` is shaped
/// when bit `c` of `mask` is set (`y = x(k+1)/(|x|k+1)`), else copied.  When no
/// active channel bit is set the whole block is copied.
pub fn w27a16_soft_clip(
    input: &[f32],
    output: &mut [f32],
    frames: usize,
    channels: usize,
    mask: u16,
    amount: f32,
) {
    let total = frames * channels;
    if channels == 0 {
        return;
    }
    let active = (1u32 << (channels as u32 & 31)).wrapping_sub(1) & mask as u32;
    if active == 0 {
        output[..total].copy_from_slice(&input[..total]);
        return;
    }
    let k = if amount >= 1.0 {
        19994.682
    } else {
        (amount + amount) / (1.0 - amount)
    };
    for f in 0..frames {
        for c in 0..channels {
            let i = f * channels + c;
            let x = input[i];
            output[i] = if (mask >> (c as u32 & 15)) & 1 != 0 {
                x * (k + 1.0) / (x.abs() * k + 1.0)
            } else {
                x
            };
        }
    }
}

/// `0x009b6630`: shelving biquad coefficients.  `gain` (param[0]) is clamped
/// to `-10000..=0`; `freq` (param[1]) is prewarped by `tan(freq*pi/rate)`.
/// `exp_like` models the x87 helper applied to `gain/100 * 0.057564627`.
/// Returns `(stored_gain, [c164,c168,c16c,c170,c174])` where `stored_gain`
/// is `gain/100` written to `0x178`.
pub fn w27a16_shelf_coeffs(
    gain: &mut i32,
    freq: i32,
    rate: i32,
    tan_fn: impl Fn(f32) -> f32,
    exp_fn: impl Fn(f32) -> f32,
) -> (f32, [f32; 5]) {
    *gain = (*gain).clamp(-10000, 0);
    let g = *gain as f32 / 100.0;
    let w = tan_fn((freq as f32 * std::f32::consts::PI) / rate as f32);
    let k = exp_fn(g * 0.057564627);
    let wk = w / k;
    let a = 1.0 / ((wk + std::f32::consts::SQRT_2) * wk + 1.0);
    let b0 = ((w + std::f32::consts::SQRT_2) * w + 1.0) * a;
    let b1 = (w * w - 1.0) * a;
    let b2 = ((w - std::f32::consts::SQRT_2) * w + 1.0) * a;
    let a1 = (wk * wk - 1.0) * -2.0 * a;
    let a2 = -(((wk - std::f32::consts::SQRT_2) * wk + 1.0) * a);
    (g, [b0, b1 + b1, b2, a1, a2])
}

#[cfg(test)]
mod tests_wave27_agent16 {
    use super::*;

    #[test]
    fn resets() {
        let mut s = vec![0u32; 0x80];
        s[0x178 / 4] = 5;
        s[0x184 / 4] = 9;
        s[0x144 / 4] = 7;
        s[0x188 / 4] = 3;
        s[0x198 / 4] = 11;
        assert_eq!(w27a16_reset_delay_unit(&mut s), Some((5, 9)));
        assert_eq!(s[0x140 / 4], 7);
        assert_eq!(s[0x188 / 4], 0);
        assert_eq!(s[0x19c / 4], 11);

        let mut s = vec![0u32; 0x80];
        s[0x174 / 4] = 4;
        assert_eq!(w27a16_reset_small_unit(&mut s), None);
        assert_eq!(s[0x178 / 4], 4);

        let mut s = vec![0u32; 0x80];
        s[0x140 / 4] = 1;
        s[0x148 / 4] = 2;
        assert_eq!(w27a16_clear_single_buffer(&mut s), Some((1, 2)));
        s[0x144 / 4] = 3;
        s[0x154 / 4] = 4;
        s[0x15c / 4] = 8;
        let r = w27a16_clear_dual_buffer(&mut s);
        assert_eq!(r, [Some((3, 4)), Some((2, 0))]);
        assert_eq!(s[0x15c / 4], 0);
    }

    #[test]
    fn tap_unit() {
        let mut s = vec![0u32; 0x100];
        s[0x20c / 4] = 100;
        s[0x1c0 / 4] = 30;
        s[0x200 / 4] = 1;
        s[0x208 / 4] = 16;
        s[0x210 / 4] = 9;
        assert_eq!(w27a16_reset_tap_unit(&mut s), Some((1, 16)));
        assert_eq!(s[0x214 / 4], 70);
        assert_eq!(s[0x218 / 4], 0);
        assert_eq!(s[0x210 / 4], 0);
    }

    #[test]
    fn clip() {
        let inp = [0.5f32, 0.5, -1.0, -1.0];
        let mut out = [0.0f32; 4];
        w27a16_soft_clip(&inp, &mut out, 2, 2, 0b01, 0.5);
        // k = 2: 0.5*3/(1+1)=0.75
        assert!((out[0] - 0.75).abs() < 1e-6);
        assert_eq!(out[1], 0.5);
        assert!((out[2] + 1.0).abs() < 1e-6);
        w27a16_soft_clip(&inp, &mut out, 2, 2, 0, 0.5);
        assert_eq!(out, inp);
    }

    #[test]
    fn shelf() {
        let mut g = 5;
        let (sg, c) = w27a16_shelf_coeffs(&mut g, 1000, 44100, |x| x.tan(), |x| x.exp());
        assert_eq!(g, 0);
        assert_eq!(sg, 0.0);
        assert!(c.iter().all(|v| v.is_finite()));
        let mut g = -20000;
        w27a16_shelf_coeffs(&mut g, 1000, 44100, |x| x, |x| x.exp());
        assert_eq!(g, -10000);
    }
}
