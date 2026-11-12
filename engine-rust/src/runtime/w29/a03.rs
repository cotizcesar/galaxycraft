#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent03.rs
// ===========================================================================
// Wave 29 translation batch 03: Direct3D 9 renderer glue helpers.

/// `0x00a9dff0`: HRESULT check.  A negative value is a failure; the original
/// formats `0x%08X|%s|%s` and throws, otherwise returns.  Returns the
/// formatted prefix on failure (name/description strings are supplied by the
/// caller because the lookup tables are opaque).
pub fn w29a03_check_hresult(hr: i32, name: &str, desc: &str) -> Result<(), String> {
    if hr < 0 {
        Err(format!("0x{:08X}|{}|{}", hr as u32, name, desc))
    } else {
        Ok(())
    }
}

/// One render-target surface description built by `0x00a9fe20`
/// (record stride `0x14`: format at +4, width at +8, height at +0xc,
/// usage at +0x10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W29a03SurfaceDesc {
    pub format: u32,
    pub width: u32,
    pub height: u32,
    pub usage: u32,
}

/// `0x00a9fe20`: builds `count` surface descriptors.  Every surface gets
/// format `0x15` except index `special`, which gets `0x71`; both extents are
/// `(size & 0xff) + 1` and usage is `3`.
pub fn w29a03_build_surfaces(count: u32, size: u32, special: u32) -> Vec<W29a03SurfaceDesc> {
    let extent = (size & 0xff) + 1;
    (0..count)
        .map(|i| W29a03SurfaceDesc {
            format: if i == special { 0x71 } else { 0x15 },
            width: extent,
            height: extent,
            usage: 3,
        })
        .collect()
}

/// GPU vendor classification used by `0x00aa0800`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W29a03Vendor {
    /// PCI id `0x1002` (3-character name).
    Amd,
    /// PCI id `0x10de` (6-character name).
    Nvidia,
    /// PCI id `0x8086` ("intel").
    Intel,
    Other,
}

/// `0x00aa0800`: maps the adapter PCI vendor id to a vendor.
pub fn w29a03_vendor_from_id(id: i32) -> W29a03Vendor {
    match id {
        0x1002 => W29a03Vendor::Amd,
        0x10de => W29a03Vendor::Nvidia,
        0x8086 => W29a03Vendor::Intel,
        _ => W29a03Vendor::Other,
    }
}

/// Viewport rectangle (left, top, right, bottom) computed in `0x00a9ec90`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W29a03Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// `0x00a9ec90` (viewport block): converts a normalised rect
/// `[x, y, w, h]` into pixels for a `width` x `height` target.  `y` and `h`
/// are clamped to `[0, 1]` for the height term; the top uses the unclamped
/// `h`; bottom carries a `+2.0` pixel margin.
pub fn w29a03_viewport_rect(norm: [f32; 4], width: i32, height: i32) -> W29a03Rect {
    let [x, y, w, h] = norm;
    let (wf, hf) = (width as f32, height as f32);
    let yc = y.clamp(0.0, 1.0);
    let hc = h.clamp(0.0, 1.0);
    let left = wf * x;
    let top = ((1.0 - yc) - h) * hf;
    W29a03Rect {
        left: left as i32,
        top: top as i32,
        right: (wf * w + left) as i32,
        bottom: (hf * hc + top + 2.0) as i32,
    }
}

#[cfg(test)]
mod tests_wave29_agent03 {
    use super::*;

    #[test]
    fn hresult() {
        assert!(w29a03_check_hresult(0, "a", "b").is_ok());
        assert!(w29a03_check_hresult(1, "a", "b").is_ok());
        assert_eq!(
            w29a03_check_hresult(-1, "N", "D").unwrap_err(),
            "0xFFFFFFFF|N|D"
        );
    }

    #[test]
    fn surfaces() {
        let s = w29a03_build_surfaces(3, 0x1ff, 1);
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].format, 0x15);
        assert_eq!(s[1].format, 0x71);
        assert_eq!(s[2].width, 0x100);
        assert_eq!(s[0].usage, 3);
        assert!(w29a03_build_surfaces(0, 0, 0).is_empty());
    }

    #[test]
    fn vendor() {
        assert_eq!(w29a03_vendor_from_id(0x1002), W29a03Vendor::Amd);
        assert_eq!(w29a03_vendor_from_id(0x10de), W29a03Vendor::Nvidia);
        assert_eq!(w29a03_vendor_from_id(0x8086), W29a03Vendor::Intel);
        assert_eq!(w29a03_vendor_from_id(0), W29a03Vendor::Other);
    }

    #[test]
    fn viewport() {
        let r = w29a03_viewport_rect([0.0, 0.0, 1.0, 1.0], 100, 50);
        assert_eq!(
            r,
            W29a03Rect {
                left: 0,
                top: 0,
                right: 100,
                bottom: 52
            }
        );
        let r = w29a03_viewport_rect([0.5, 0.25, 0.5, 0.5], 200, 100);
        assert_eq!(r.left, 100);
        assert_eq!(r.top, 25);
        assert_eq!(r.right, 200);
        assert_eq!(r.bottom, 77);
    }
}
