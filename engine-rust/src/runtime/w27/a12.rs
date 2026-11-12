#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent12.rs
// ===========================================================================
// Wave 27 agent 12: control/tile-map helpers.

/// `0x0097fd70`: counts bytes scanning backwards from `buf[len-1]` until the
/// predicate (callee FUN_007a7660) is true (that byte is counted) or a NUL
/// byte / the buffer start is reached.
pub fn w27a12_trailing_run(buf: &[u8], len: usize, mut pred: impl FnMut(u8) -> bool) -> usize {
    if len == 0 {
        return 0;
    }
    let mut n = 0;
    let mut i = len as isize - 1;
    while i >= 0 {
        let c = buf[i as usize];
        if c == 0 {
            break;
        }
        n += 1;
        if pred(c) {
            return n;
        }
        i -= 1;
    }
    n
}

/// `0x00980e60`: toggles the flag byte at +0x56.  `on_enable`/`on_disable`
/// are the callee sequences; returns the new flag.
pub fn w27a12_toggle_flag(
    flag: &mut u8,
    param: u8,
    mut on_enable: impl FnMut(),
    mut on_disable: impl FnMut(),
) {
    if param == 0 {
        if *flag != 0 {
            on_disable();
            *flag = 0;
        }
    } else if *flag == 0 {
        on_enable();
        *flag = param;
    }
}

/// `0x00982f20`: stores two bytes at offsets 0x60 and 0x63 of a record.
pub fn w27a12_set_bytes(rec: &mut [u8], a: u8, b: u8) {
    rec[0x60] = a;
    rec[0x63] = b;
}

/// Range control fields for `0x00982f90`: value at +0x54, min +0x56, max +0x58.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W27a12Range {
    pub value: i16,
    pub min: i16,
    pub max: i16,
}

/// `0x00982f90`: sets min/max and clamps the value, invoking `notify` when
/// the value changed.
pub fn w27a12_set_range(r: &mut W27a12Range, min: i16, max: i16, mut notify: impl FnMut()) {
    let same = r.min == min && r.max == max;
    r.min = min;
    r.max = max;
    if same {
        return;
    }
    let v = r.value;
    let target = if v < min {
        min
    } else if max < v {
        max
    } else {
        return;
    };
    if v != target {
        r.value = target;
        notify();
    }
}

fn w27a12_tile_flag(map: &[u16], w: u32, h: u32, x: u32, y: u32) -> bool {
    x < w && y < h && (map[(w * y + x) as usize] & 0x7ff0) == 0x10
}

/// `0x00983b30`: tile at (x, y) has class bits `0x10` (mask `0x7ff0`).
pub fn w27a12_tile_is_class10(map: &[u16], w: u32, x: u32, y: u32) -> bool {
    (map[(w * y + x) as usize] & 0x7ff0) == 0x10
}

/// `0x00983260`: number of the eight neighbours of (x, y) whose tile has
/// class `0x10`; out-of-range neighbours count as zero (unsigned wrap).
pub fn w27a12_neighbour_count(map: &[u16], w: u32, h: u32, x: u32, y: u32) -> u8 {
    let mut n = 0;
    for dy in [-1i32, 0, 1] {
        for dx in [-1i32, 0, 1] {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x.wrapping_add(dx as u32);
            let ny = y.wrapping_add(dy as u32);
            n += w27a12_tile_flag(map, w, h, nx, ny) as u8;
        }
    }
    n
}

/// `0x00983440`: pixel rectangle `[left, top, right, bottom]` around tile
/// (tx, ty), clamped to a `w`x`h` tile map.  Upper clamp applies only when
/// the lower bound did not need clamping (else-if in the original).
pub fn w27a12_visible_rect(tx: i32, ty: i32, w: i32, h: i32) -> [i32; 4] {
    let mut l = tx * 0x20 - 0x270;
    let mut r = tx * 0x20 + 0x290;
    let mut t = ty * 0x20 - 0x180;
    let mut b = ty * 0x20 + 0x1a0;
    if l < 0 {
        l = 0;
    } else if w * 0x20 <= r {
        r = w * 0x20 - 1;
    }
    if t < 0 {
        t = 0;
    } else if h * 0x20 <= b {
        b = h * 0x20 - 1;
    }
    [l, t, r, b]
}

/// `0x009834e0`: writes width*height to `out` when the tile buffer exists.
pub fn w27a12_tile_count(has_buffer: bool, w: i32, h: i32, out: Option<&mut i32>) {
    if !has_buffer {
        return;
    }
    if let Some(o) = out {
        *o = h * w;
    }
}

#[cfg(test)]
mod tests_wave27_agent12 {
    use super::*;

    #[test]
    fn trailing() {
        let b = b"ab cd";
        assert_eq!(w27a12_trailing_run(b, 0, |_| true), 0);
        assert_eq!(w27a12_trailing_run(b, 5, |c| c == b' '), 3);
        assert_eq!(w27a12_trailing_run(b, 5, |_| false), 5);
        assert_eq!(w27a12_trailing_run(b"a\0b", 3, |_| false), 1);
    }

    #[test]
    fn toggle() {
        let mut f = 0u8;
        let log = std::cell::RefCell::new(vec![]);
        w27a12_toggle_flag(
            &mut f,
            1,
            || log.borrow_mut().push("on"),
            || log.borrow_mut().push("off"),
        );
        assert_eq!(f, 1);
        w27a12_toggle_flag(
            &mut f,
            1,
            || log.borrow_mut().push("on"),
            || log.borrow_mut().push("off"),
        );
        w27a12_toggle_flag(
            &mut f,
            0,
            || log.borrow_mut().push("on"),
            || log.borrow_mut().push("off"),
        );
        assert_eq!((f, log.into_inner()), (0, vec!["on", "off"]));
    }

    #[test]
    fn bytes() {
        let mut r = [0u8; 0x70];
        w27a12_set_bytes(&mut r, 5, 6);
        assert_eq!((r[0x60], r[0x63]), (5, 6));
    }

    #[test]
    fn range() {
        let mut r = W27a12Range {
            value: 50,
            min: 0,
            max: 100,
        };
        let mut n = 0;
        w27a12_set_range(&mut r, 0, 100, || n += 1);
        assert_eq!(n, 0);
        w27a12_set_range(&mut r, 0, 40, || n += 1);
        assert_eq!((r.value, n), (40, 1));
        w27a12_set_range(&mut r, 60, 80, || n += 1);
        assert_eq!((r.value, n), (60, 2));
        w27a12_set_range(&mut r, 10, 90, || n += 1);
        assert_eq!((r.value, n), (60, 2));
    }

    #[test]
    fn tiles() {
        let mut m = vec![0u16; 9];
        m[4] = 0x10;
        m[0] = 0x8010;
        assert!(w27a12_tile_is_class10(&m, 3, 1, 1));
        assert!(w27a12_tile_is_class10(&m, 3, 0, 0));
        assert!(!w27a12_tile_is_class10(&m, 3, 1, 0));
        assert_eq!(w27a12_neighbour_count(&m, 3, 3, 0, 0), 1);
        assert_eq!(w27a12_neighbour_count(&m, 3, 3, 1, 1), 1);
        m[8] = 0x1f;
        assert_eq!(w27a12_neighbour_count(&m, 3, 3, 1, 1), 2);
        assert_eq!(w27a12_neighbour_count(&m, 3, 3, 2, 2), 1);
    }

    #[test]
    fn rect() {
        assert_eq!(w27a12_visible_rect(0, 0, 100, 100), [0, 0, 0x290, 0x1a0]);
        assert_eq!(
            w27a12_visible_rect(99, 99, 100, 100),
            [99 * 32 - 0x270, 99 * 32 - 0x180, 3199, 3199]
        );
        let mut n = 0;
        w27a12_tile_count(true, 4, 5, Some(&mut n));
        assert_eq!(n, 20);
        w27a12_tile_count(false, 4, 5, Some(&mut n));
        assert_eq!(n, 20);
    }
}
