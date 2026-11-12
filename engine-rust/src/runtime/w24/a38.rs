#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent38.rs
// ===========================================================================
// Wave 24 agent 38: audio decoder-state helpers (queue, `runtime.rs`).

/// `0x007af9d0`: resets the decoder bookkeeping fields at `+0x110..+0x11e`
/// of the state block (`-1, -1, 0, 0u16`) and zeroes the `0xe0` bytes at
/// `+0x30`.  `buf` is the state block (at least `0x120` bytes).  The two
/// callees (`FUN_009f7200`, `FUN_009e4350`) are caller-supplied hooks.
pub fn w24a38_reset_decoder_state(
    buf: &mut [u8],
    mut hook_a: impl FnMut(),
    mut hook_b: impl FnMut(&mut [u8]),
) {
    buf[0x110..0x114].copy_from_slice(&(-1i32).to_le_bytes());
    buf[0x114..0x118].copy_from_slice(&(-1i32).to_le_bytes());
    buf[0x118..0x11c].copy_from_slice(&0u32.to_le_bytes());
    buf[0x11c..0x11e].copy_from_slice(&0u16.to_le_bytes());
    hook_a();
    hook_b(&mut buf[0x20..]);
    buf[0x30..0x30 + 0xe0].fill(0);
}

/// `0x007afb50`: pushes `arg` into the sub-decoder (`push`, rc at `+0xa0`);
/// a non-zero rc yields `rc == -0x87`; otherwise `flush` (at `+0x30`) must
/// return zero.
pub fn w24a38_feed_ok(
    arg: u32,
    mut push: impl FnMut(u32) -> i32,
    mut flush: impl FnMut() -> i32,
) -> bool {
    let rc = push(arg);
    if rc != 0 {
        return rc == -0x87;
    }
    flush() == 0
}

/// `0x007afdc0`: zeroes the `0x40030` bytes at `+0x3c` of the state block.
pub fn w24a38_clear_sample_buffer(buf: &mut [u8]) {
    buf[0x3c..0x3c + 0x40030].fill(0);
}

/// `0x007afd60`: clears the sample buffer, then when flag `+0x40089` is set
/// calls `close_a` (block `+4`) and, if flag `+0x4008d` is also set,
/// `close_b` (block `+0x20`); both flags are cleared and `+4..+0x3c` zeroed.
pub fn w24a38_close_stream(
    buf: &mut [u8],
    mut close_a: impl FnMut(&mut [u8]),
    mut close_b: impl FnMut(&mut [u8]),
) {
    w24a38_clear_sample_buffer(buf);
    if buf[0x40089] != 0 {
        close_a(&mut buf[4..]);
        if buf[0x4008d] != 0 {
            close_b(&mut buf[0x20..]);
        }
    }
    buf[0x40089] = 0;
    buf[0x4008d] = 0;
    buf[4..4 + 0x38].fill(0);
}

/// Output cursor used by `0x007afba0` (`param_1`: base, frames done, frames total).
pub struct W24a38Pcm<'a> {
    pub samples: &'a mut [i16],
    pub done: u32,
    pub total: u32,
}

/// `0x007afba0`: converts planar float channels to interleaved clamped
/// `i16` (`v * 32767.0 + 0.5`, truncated, clamped to `-0x8000..=0x7fff`).
/// `avail` returns the planar channel slices for the currently readable
/// frames (empty = none); `consume(n)` returns non-zero on failure.
/// Returns `false` only when `consume` fails.
#[allow(clippy::needless_range_loop)]
pub fn w24a38_pcm_float_to_i16(
    channels: usize,
    out: &mut W24a38Pcm<'_>,
    mut avail: impl FnMut() -> Vec<Vec<f32>>,
    mut consume: impl FnMut(u32) -> i32,
) -> bool {
    let mut planes = avail();
    let mut frames = planes.first().map_or(0, |p| p.len()) as i32;
    if frames <= 0 {
        return true;
    }
    loop {
        let room = out.total.wrapping_sub(out.done);
        let n = (frames as u32).min(room);
        for f in 0..n as usize {
            for c in 0..channels {
                let v = (planes[c][f] * 32767.0 + 0.5) as i32;
                let v = v.clamp(-0x8000, 0x7fff);
                out.samples[(out.done as usize + f) * channels + c] = v as i16;
            }
        }
        out.done += n;
        if n == 0 {
            return true;
        }
        if consume(n) != 0 {
            return false;
        }
        if out.done == out.total {
            return true;
        }
        planes = avail();
        frames = planes.first().map_or(0, |p| p.len()) as i32;
        if frames < 1 {
            return true;
        }
    }
}

/// `0x007b18b0`: walks the tagged-pointer list from `head` (low bit set or
/// zero ends the walk; next link is `next(node)`) looking for `target`;
/// returns true when found (the caller then runs the destructor
/// `FUN_007b1270`).  A zero `target` returns false.
pub fn w24a38_list_contains(head: u32, target: u32, mut next: impl FnMut(u32) -> u32) -> bool {
    if target == 0 {
        return false;
    }
    let mut cur = if head & 1 == 0 && head != 0 { head } else { 0 };
    loop {
        if cur & 1 != 0 || cur == 0 {
            return false;
        }
        let hit = cur == target;
        cur = next(cur);
        if hit {
            return true;
        }
    }
}

/// `0x007b2030`: true when the global handle (`DAT_011b9128`) is non-zero.
pub fn w24a38_handle_valid(handle: u32) -> bool {
    handle != 0
}

#[cfg(test)]
mod tests_wave24_agent38 {
    use super::*;

    #[test]
    fn reset_and_clear() {
        let mut b = vec![0xffu8; 0x120];
        let mut seen = 0;
        w24a38_reset_decoder_state(&mut b, || seen += 1, |_| {});
        assert_eq!(seen, 1);
        assert_eq!(&b[0x110..0x118], &[0xff; 8]);
        assert_eq!(&b[0x118..0x11e], &[0; 6]);
        assert!(b[0x30..0x110].iter().all(|&x| x == 0));
        assert_eq!(b[0x2f], 0xff);
    }

    #[test]
    fn feed() {
        assert!(w24a38_feed_ok(1, |_| -0x87, || 5));
        assert!(!w24a38_feed_ok(1, |_| 3, || 0));
        assert!(w24a38_feed_ok(1, |_| 0, || 0));
        assert!(!w24a38_feed_ok(1, |_| 0, || 1));
    }

    #[test]
    fn close() {
        let mut b = vec![1u8; 0x40100];
        let calls = std::cell::RefCell::new(Vec::new());
        w24a38_close_stream(
            &mut b,
            |_| calls.borrow_mut().push('a'),
            |_| calls.borrow_mut().push('b'),
        );
        assert_eq!(*calls.borrow(), vec!['a', 'b']);
        assert_eq!(b[0x40089], 0);
        assert_eq!(b[0x4008d], 0);
        assert!(b[4..0x3c].iter().all(|&x| x == 0));
        assert_eq!(b[0x3b], 0);
        assert_eq!(b[0x3c], 0);
        let mut b = vec![0u8; 0x40100];
        b[0x4008d] = 1;
        let n = std::cell::Cell::new(0);
        w24a38_close_stream(&mut b, |_| n.set(1), |_| n.set(1));
        assert_eq!(n.get(), 0);
    }

    #[test]
    fn pcm() {
        let mut buf = [0i16; 6];
        let mut out = W24a38Pcm {
            samples: &mut buf,
            done: 0,
            total: 3,
        };
        let mut left = 1;
        let ok = w24a38_pcm_float_to_i16(
            2,
            &mut out,
            || {
                if left == 0 {
                    return vec![];
                }
                left -= 1;
                vec![vec![1.0, -1.0, 0.0], vec![2.0, 0.0, 0.5]]
            },
            |_| 0,
        );
        assert!(ok);
        assert_eq!(buf, [32767, 32767, -32766, 0, 0, 16384]);
        let mut buf = [0i16; 2];
        let mut out = W24a38Pcm {
            samples: &mut buf,
            done: 0,
            total: 2,
        };
        assert!(!w24a38_pcm_float_to_i16(
            1,
            &mut out,
            || vec![vec![0.0; 2]],
            |_| 1
        ));
    }

    #[test]
    fn list_and_handle() {
        let next = |n: u32| if n == 8 { 16 } else { 1 };
        assert!(w24a38_list_contains(8, 16, next));
        assert!(!w24a38_list_contains(8, 24, next));
        assert!(!w24a38_list_contains(8, 0, next));
        assert!(!w24a38_list_contains(1, 8, next));
        assert!(w24a38_handle_valid(1) && !w24a38_handle_valid(0));
    }
}
