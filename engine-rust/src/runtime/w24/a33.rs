#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent33.rs
// ===========================================================================
/// `0x0079cc50`: selects one of three cached flag bytes.  When `flag` is
/// non-zero the first byte (`DAT_011b24b9`) is returned; otherwise the
/// second byte (`DAT_011b24ba`) if `id == 0xe4`, else the third
/// (`DAT_011b24bb`).  The retail routine first calls a refresh helper
/// (not modelled; the caller supplies the refreshed bytes).
pub fn w24a33_select_cached_byte(flag: i32, id: i16, cached: [u8; 3]) -> u8 {
    if flag != 0 {
        return cached[0];
    }
    if id as u16 != 0xe4 {
        cached[2]
    } else {
        cached[1]
    }
}

#[cfg(test)]
mod tests_wave24_agent33 {
    use super::*;

    #[test]
    fn select_cached_byte() {
        let c = [1, 2, 3];
        assert_eq!(w24a33_select_cached_byte(1, 0xe4, c), 1);
        assert_eq!(w24a33_select_cached_byte(0, 0xe4, c), 2);
        assert_eq!(w24a33_select_cached_byte(0, 0xe5, c), 3);
        assert_eq!(w24a33_select_cached_byte(0, 0, c), 3);
    }
}
