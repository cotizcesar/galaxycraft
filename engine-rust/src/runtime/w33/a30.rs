#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent30.rs
// ===========================================================================
// Wave 33 agent 30.

/// `0x00b89520`: forwards the object's first two words plus two caller
/// arguments to the callee (modelled as a closure).
pub fn w33a30_forward_pair<R>(
    obj: &[u32; 2],
    a: u32,
    b: u32,
    callee: impl FnOnce(u32, u32, u32, u32) -> R,
) -> R {
    callee(obj[0], obj[1], a, b)
}

/// `0x00b8fb50`: MD5-style finalisation.  `state[0..2]` is the 64-bit bit
/// count, `state[2..6]` the digest words.  Feeds `((-(count>>3) - 9) & 0x3f) + 1`
/// padding bytes from `pad` and then the 8 count bytes to `update`, and
/// returns the four digest words.
pub fn w33a30_finalize(state: &[u32; 6], pad: &[u8], mut update: impl FnMut(&[u8])) -> [u32; 4] {
    let c = state[0];
    let n = ((0u32.wrapping_sub(c >> 3).wrapping_sub(9)) & 0x3f) + 1;
    let n = (n as usize).min(pad.len());
    update(&pad[..n]);
    let mut len = [0u8; 8];
    len[..4].copy_from_slice(&c.to_le_bytes());
    len[4..].copy_from_slice(&state[1].to_le_bytes());
    update(&len);
    [state[2], state[3], state[4], state[5]]
}

/// `0x00c12a00`: maps a PCI GPU vendor id to its name.
pub fn w33a30_gpu_vendor_name(id: u16) -> Option<&'static str> {
    match id {
        0x1414 => Some("Microsoft"),
        0x1002 => Some("AMD"),
        0x106b => Some("Apple"),
        0x10de => Some("NVidia"),
        0x4351 => Some("Qualcomm"),
        0x8086 => Some("Intel"),
        _ => None,
    }
}

#[cfg(test)]
mod tests_wave33_agent30 {
    use super::*;
    #[test]
    fn vendor() {
        assert_eq!(w33a30_gpu_vendor_name(0x8086), Some("Intel"));
        assert_eq!(w33a30_gpu_vendor_name(1), None);
    }
    #[test]
    fn fin() {
        let mut got = 0;
        let d = w33a30_finalize(&[0, 0, 1, 2, 3, 4], &[0x80; 64], |s| got += s.len());
        assert_eq!(d, [1, 2, 3, 4]);
        assert_eq!(got, 56 + 8);
    }
    #[test]
    fn fwd() {
        assert_eq!(
            w33a30_forward_pair(&[1, 2], 3, 4, |a, b, c, d| a + b + c + d),
            10
        );
    }
}
