#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent02.rs
// ===========================================================================
// Wave 29 batch 02: render-target binding and GL init helpers (pure models).

/// One call recorded by the render-target binders: `(slot, target)` for
/// `SetRenderTarget` (vtable +0x94) or the viewport (vtable +0xbc).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W29a02Op {
    /// `SetRenderTarget(slot, target)`; `None` clears the slot.
    SetTarget(u32, Option<u32>),
    /// `SetViewport` with `[x, y, w, h, 0.0, 1.0]` (z range as f32 bits).
    SetViewport([u32; 6]),
}

const W29A02_ONE_BITS: u32 = 0x3f80_0000;

fn w29a02_viewport(rect: &[u32; 4]) -> W29a02Op {
    W29a02Op::SetViewport([rect[0], rect[1], rect[2], rect[3], 0, W29A02_ONE_BITS])
}

/// `0x00a9d950`: binds every device slot below `device_slots`; slots with
/// an entry in `targets` get that surface, the others are cleared.  Ends
/// by applying the viewport `rect` (this+4..+0x14) with z range 0..1.
pub fn w29a02_bind_all_targets(
    device_slots: u32,
    targets: &[u32],
    rect: &[u32; 4],
) -> Vec<W29a02Op> {
    let mut ops = Vec::new();
    for slot in 0..device_slots {
        ops.push(W29a02Op::SetTarget(
            slot,
            targets.get(slot as usize).copied(),
        ));
    }
    ops.push(w29a02_viewport(rect));
    ops
}

/// `0x00a9da40`: binds target `index` to slot 0 and clears slots
/// `1..device_slots`; nothing happens when `index > targets.len()`
/// (retail compares with `<=`, so `index == len` reads past the end; here
/// that case yields `None`).
pub fn w29a02_bind_single_target(
    index: u32,
    device_slots: u32,
    targets: &[u32],
    rect: &[u32; 4],
) -> Vec<W29a02Op> {
    let mut ops = Vec::new();
    if index as usize > targets.len() {
        return ops;
    }
    for slot in 1..device_slots {
        ops.push(W29a02Op::SetTarget(slot, None));
    }
    ops.push(W29a02Op::SetTarget(0, targets.get(index as usize).copied()));
    ops.push(w29a02_viewport(rect));
    ops
}

/// `0x00a9db40`: clears slots `count..device_slots`, then binds slot `i`
/// (for `i < count`) to `targets[order[i]]`, then sets the viewport.
pub fn w29a02_bind_target_list(
    order: &[u32],
    count: u32,
    device_slots: u32,
    targets: &[u32],
    rect: &[u32; 4],
) -> Vec<W29a02Op> {
    let mut ops = Vec::new();
    for slot in count..device_slots {
        ops.push(W29a02Op::SetTarget(slot, None));
    }
    for i in 0..count {
        let t = order
            .get(i as usize)
            .and_then(|&o| targets.get(o as usize))
            .copied();
        ops.push(W29a02Op::SetTarget(i, t));
    }
    ops.push(w29a02_viewport(rect));
    ops
}

/// `0x00a9bd80`: reads back the framebuffer as RGBA8 (`0x1908`/`0x1401`).
/// Allocates `w*h*4` bytes through `alloc`, fills it via `read_pixels`
/// (x=0, y=0, w, h) and reports the size through the optional outputs.
pub fn w29a02_read_framebuffer(
    width: i32,
    height: i32,
    out_w: Option<&mut i32>,
    out_h: Option<&mut i32>,
    mut alloc: impl FnMut(usize) -> Vec<u8>,
    mut read_pixels: impl FnMut(i32, i32, i32, i32, &mut [u8]),
) -> Vec<u8> {
    let size = (height.wrapping_mul(width).wrapping_mul(4)) as usize;
    let mut buf = alloc(size);
    read_pixels(0, 0, width, height, &mut buf);
    if let Some(w) = out_w {
        *w = width;
    }
    if let Some(h) = out_h {
        *h = height;
    }
    buf
}

/// Outcome of the capability check in `0x00a9c060`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W29a02GlInit {
    /// Intel driver string found (renderer or vendor): takes the string
    /// path with no render-target setup.
    IntelPath,
    /// `("GL Error", "Only 1 render target")` is reported.
    SingleTargetError,
    /// Blend (0xbe2) is enabled with (0x302, 0x303) and setup proceeds.
    Ready,
}

/// `0x00a9c060`: decides GL setup from the two driver strings containing
/// "intel" and the limits `GL_MAX_COLOR_ATTACHMENTS` (0x8cdf) and
/// `GL_MAX_DRAW_BUFFERS` (0x8824).  Fewer than 6 of either is an error.
pub fn w29a02_gl_init_decision(
    renderer_has_intel: bool,
    vendor_has_intel: bool,
    max_color_attachments: i32,
    max_draw_buffers: i32,
) -> W29a02GlInit {
    if renderer_has_intel || vendor_has_intel {
        W29a02GlInit::IntelPath
    } else if max_color_attachments < 6 || max_draw_buffers < 6 {
        W29a02GlInit::SingleTargetError
    } else {
        W29a02GlInit::Ready
    }
}

#[cfg(test)]
mod tests_wave29_agent02 {
    use super::*;

    const R: [u32; 4] = [1, 2, 3, 4];
    const VP: W29a02Op = W29a02Op::SetViewport([1, 2, 3, 4, 0, 0x3f80_0000]);

    #[test]
    fn bind_all() {
        let ops = w29a02_bind_all_targets(3, &[10, 11], &R);
        assert_eq!(
            ops,
            vec![
                W29a02Op::SetTarget(0, Some(10)),
                W29a02Op::SetTarget(1, Some(11)),
                W29a02Op::SetTarget(2, None),
                VP
            ]
        );
        assert_eq!(w29a02_bind_all_targets(0, &[], &R), vec![VP]);
    }

    #[test]
    fn bind_single() {
        let ops = w29a02_bind_single_target(1, 3, &[10, 11], &R);
        assert_eq!(
            ops,
            vec![
                W29a02Op::SetTarget(1, None),
                W29a02Op::SetTarget(2, None),
                W29a02Op::SetTarget(0, Some(11)),
                VP
            ]
        );
        assert!(w29a02_bind_single_target(3, 3, &[10, 11], &R).is_empty());
    }

    #[test]
    fn bind_list() {
        let ops = w29a02_bind_target_list(&[1, 0], 2, 3, &[10, 11], &R);
        assert_eq!(
            ops,
            vec![
                W29a02Op::SetTarget(2, None),
                W29a02Op::SetTarget(0, Some(11)),
                W29a02Op::SetTarget(1, Some(10)),
                VP
            ]
        );
    }

    #[test]
    fn readback() {
        let (mut w, mut h) = (0, 0);
        let buf = w29a02_read_framebuffer(
            2,
            3,
            Some(&mut w),
            Some(&mut h),
            |n| vec![0; n],
            |x, y, ww, hh, b| {
                assert_eq!((x, y, ww, hh), (0, 0, 2, 3));
                b[0] = 7;
            },
        );
        assert_eq!((buf.len(), buf[0], w, h), (24, 7, 2, 3));
    }

    #[test]
    fn gl_decision() {
        assert_eq!(
            w29a02_gl_init_decision(true, false, 8, 8),
            W29a02GlInit::IntelPath
        );
        assert_eq!(
            w29a02_gl_init_decision(false, false, 5, 8),
            W29a02GlInit::SingleTargetError
        );
        assert_eq!(
            w29a02_gl_init_decision(false, false, 8, 5),
            W29a02GlInit::SingleTargetError
        );
        assert_eq!(
            w29a02_gl_init_decision(false, false, 6, 6),
            W29a02GlInit::Ready
        );
    }
}
