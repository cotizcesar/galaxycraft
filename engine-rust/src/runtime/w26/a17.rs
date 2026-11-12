#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent17.rs
// ===========================================================================
// Wave 26 batch 17: Win32 glue helpers (pure reference forms).

/// `0x0093f750`: converts a raw millisecond-tick counter into seconds.  The
/// counter is a signed 32-bit value; when negative, `bias[1]` is added (the
/// unsigned wrap correction), otherwise `bias[0]`.  The sum is rounded to
/// `f32` and scaled by `0.001`.  `bias` is the two-double table at the
/// opaque global (layout not visible; caller supplies it).
pub fn w26a17_ticks_to_seconds(ticks: i32, bias: [f64; 2]) -> f32 {
    let idx = (-(ticks >> 31)) as usize;
    (((ticks as f64) + bias[idx]) as f32 as f64 * 0.001) as f32
}

/// `0x0093f870`: validates a pair of arguments; both must be non-zero.
/// Returns `Err(0x57)` (ERROR_INVALID_PARAMETER) otherwise, `Ok(())` when
/// the wrapped call should be forwarded.
pub fn w26a17_check_two_args(a: usize, b: usize) -> Result<(), u32> {
    if a != 0 && b != 0 { Ok(()) } else { Err(0x57) }
}

/// `0x00940720`: forwards `(p1, p2, p3, p4, 0)` to the wrapped call when
/// both `p2` and `p4` are non-zero; otherwise sets error 0x57 and yields
/// 0.  `forward` models the imported call.
pub fn w26a17_forward_checked(
    p1: u32,
    p2: u32,
    p3: u32,
    p4: u32,
    mut forward: impl FnMut(u32, u32, u32, u32, u32) -> u32,
    mut set_error: impl FnMut(u32),
) -> u32 {
    if p2 != 0 && p4 != 0 {
        forward(p1, p2, p3, p4, 0)
    } else {
        set_error(0x57);
        0
    }
}

/// `0x00942320`: converts a `RECT {left, top, right, bottom}` into
/// `[x, y, width, height]`, i.e. `[left, top, right-left, bottom-top]`.
pub fn w26a17_rect_to_xywh(rect: [i32; 4]) -> [i32; 4] {
    [
        rect[0],
        rect[1],
        rect[2].wrapping_sub(rect[0]),
        rect[3].wrapping_sub(rect[1]),
    ]
}

#[cfg(test)]
mod tests_wave26_agent17 {
    use super::*;

    #[test]
    fn ticks() {
        let b = [0.0, 4294967296.0];
        assert!((w26a17_ticks_to_seconds(1500, b) - 1.5).abs() < 1e-6);
        assert!((w26a17_ticks_to_seconds(-1, b) - 4_294_967.3).abs() < 1.0);
    }

    #[test]
    fn checks() {
        assert_eq!(w26a17_check_two_args(1, 0), Err(0x57));
        assert_eq!(w26a17_check_two_args(1, 2), Ok(()));
    }

    #[test]
    fn forward() {
        let mut err = 0;
        assert_eq!(
            w26a17_forward_checked(1, 0, 3, 4, |_, _, _, _, _| 9, |e| err = e),
            0
        );
        assert_eq!(err, 0x57);
        assert_eq!(
            w26a17_forward_checked(1, 2, 3, 4, |a, _, _, _, e| a + e + 7, |_| {}),
            8
        );
    }

    #[test]
    fn rect() {
        assert_eq!(w26a17_rect_to_xywh([1, 2, 11, 7]), [1, 2, 10, 5]);
    }
}
