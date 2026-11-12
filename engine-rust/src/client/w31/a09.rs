#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent09.rs
// ===========================================================================
// Wave 31 agent 09 translations.

/// Record fields touched by `0x00797340` (offsets 0x124..0x134).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W31a09PingRecord {
    /// +0x124 send tick.
    pub sent_tick: u32,
    /// +0x128 expected token.
    pub expected: u32,
    /// +0x12c acknowledged token.
    pub acked: u32,
    /// +0x130 last round trip.
    pub rtt: u32,
    /// +0x134 max round trip.
    pub max_rtt: u32,
}

/// `0x00797340`: record a reply. When a 4-byte payload is present its token
/// must equal `expected` (else the reply is ignored), then it is stored.
/// rtt = now - sent_tick; max_rtt keeps the maximum.
pub fn w31a09_record_reply(
    rec: Option<&mut W31a09PingRecord>,
    payload: Option<(u32, u32)>,
    now: u32,
) {
    let Some(rec) = rec else { return };
    if let Some((token, len)) = payload {
        if len == 4 {
            if token != rec.expected {
                return;
            }
            rec.acked = token;
        }
    }
    let rtt = now.wrapping_sub(rec.sent_tick);
    rec.rtt = rtt;
    if rec.max_rtt <= rtt {
        rec.max_rtt = rtt;
    }
}

/// Blink state used by `0x007a0b20` (dword indices 0x172..0x177).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W31a09Blink {
    /// [0x172] toggled phase.
    pub phase: u32,
    /// [0x173] accumulated time.
    pub elapsed: u32,
    /// [0x174] low byte: expiry enabled.
    pub expire_enabled: u32,
    /// [0x177] step.
    pub step: u32,
}

/// `0x007a0b20`: toggle phase, add step; returns true when elapsed exceeds
/// 1999 and the low byte of the enable flag is set (caller then runs the
/// close callbacks).
pub fn w31a09_blink_tick(b: &mut W31a09Blink) -> bool {
    b.phase = u32::from(b.phase == 0);
    b.elapsed = b.elapsed.wrapping_add(b.step);
    (b.elapsed as i32) > 1999 && (b.expire_enabled & 0xff) != 0
}

/// `0x007a73c0`: walk a NUL-terminated byte string validating each lead
/// byte. Control bytes 1-8, 0xe-0x11, 0x15-0x19, 0x1b-0x1f and 0,9,10,12,13,
/// 0x12,0x13 advance one; others go to `validate(pos)` (the callee at
/// 76b0), then ASCII advances 1 and UTF-8 leads advance 2/3/4. An invalid
/// lead returns false (the original would spin on it).
pub fn w31a09_utf8_walk(s: &[u8], validate: &mut dyn FnMut(usize) -> bool) -> bool {
    let mut pos = 0usize;
    loop {
        let b = s.get(pos).copied().unwrap_or(0);
        if b == 0 {
            return true;
        }
        let step = match b {
            1..=8
            | 0x0e..=0x11
            | 0x15..=0x19
            | 0x1b..=0x1f
            | 9
            | 10
            | 0x0c
            | 0x0d
            | 0x12
            | 0x13 => 1,
            _ => {
                if !validate(pos) {
                    return false;
                }
                if b < 0x80 {
                    1
                } else if (b & 0xe0) == 0xc0 && b > 0xc1 {
                    2
                } else if (b & 0xf0) == 0xe0 {
                    3
                } else if (b & 0xf8) == 0xf0 && b < 0xf5 {
                    4
                } else {
                    return false;
                }
            }
        };
        pos += step;
    }
}

/// Drag-select rectangle state for `0x007a7e00`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W31a09DragRect {
    /// DAT 011b8678 active flag.
    pub active: bool,
    /// Anchor point (011b867c, 011b867e).
    pub anchor: (i16, i16),
    /// left, top, right, bottom (011b8680..011b8686).
    pub rect: (i16, i16, i16, i16),
}

/// `0x007a7e00`: update the drag rectangle from the point at (x, y). When
/// the anchor differs from the point, `fix_anchor` runs (cursor/clip check)
/// and returns the anchor to use afterwards.
pub fn w31a09_update_drag(
    d: &mut W31a09DragRect,
    x: i16,
    y: i16,
    fix_anchor: &mut dyn FnMut() -> (i16, i16),
) {
    if d.anchor != (x, y) {
        d.anchor = fix_anchor();
    }
    let (ax, ay) = d.anchor;
    d.active = true;
    let (l, r) = if ax <= x { (ax, x) } else { (x, ax) };
    let (t, b) = if y < ay { (y, ay) } else { (ay, y) };
    d.rect = (l, t, r, b);
}

/// `0x007a89e0`: clears the active flag; returns true when it was set
/// (caller then runs the follow-up routine).
pub fn w31a09_end_drag(d: &mut W31a09DragRect) -> bool {
    if d.active {
        d.active = false;
        true
    } else {
        false
    }
}

/// `0x007b1900`: scale a point by two factors (truncating toward zero).
pub fn w31a09_scale_point(fx: f64, fy: f64, x: i32, y: i32) -> (i32, i32) {
    ((fx * x as f64) as i32, (fy * y as f64) as i32)
}

/// `0x007b2d30`: divide a point by two factors (truncating).
pub fn w31a09_unscale_point(fx: f64, fy: f64, x: i32, y: i32) -> (i32, i32) {
    ((x as f64 / fx) as i32, (y as f64 / fy) as i32)
}

/// `0x007b2d80`: like the unscale above, then add the origin `(ox, oy)`.
pub fn w31a09_unscale_point_offset(
    fx: f64,
    fy: f64,
    x: i32,
    y: i32,
    origin: (i32, i32),
) -> (i32, i32) {
    (
        origin.0.wrapping_add((x as f64 / fx) as i32),
        origin.1.wrapping_add((y as f64 / fy) as i32),
    )
}

#[cfg(test)]
mod tests_wave31_agent09 {
    use super::*;

    #[test]
    fn reply() {
        let mut r = W31a09PingRecord {
            sent_tick: 10,
            expected: 7,
            max_rtt: 50,
            ..Default::default()
        };
        w31a09_record_reply(Some(&mut r), Some((8, 4)), 100);
        assert_eq!(r.rtt, 0);
        w31a09_record_reply(Some(&mut r), Some((7, 4)), 30);
        assert_eq!((r.acked, r.rtt, r.max_rtt), (7, 20, 50));
        w31a09_record_reply(Some(&mut r), None, 90);
        assert_eq!((r.rtt, r.max_rtt), (80, 80));
        w31a09_record_reply(None, None, 1);
    }

    #[test]
    fn blink() {
        let mut b = W31a09Blink {
            elapsed: 1990,
            expire_enabled: 1,
            step: 10,
            ..Default::default()
        };
        assert!(w31a09_blink_tick(&mut b));
        assert_eq!(b.phase, 1);
        b.expire_enabled = 0x100;
        assert!(!w31a09_blink_tick(&mut b));
        assert_eq!(b.phase, 0);
    }

    #[test]
    fn walk() {
        let mut ok = |_p: usize| true;
        assert!(w31a09_utf8_walk(b"", &mut ok));
        assert!(w31a09_utf8_walk(
            "ab\u{e9}\u{20ac}\u{1f600}\n".as_bytes(),
            &mut ok
        ));
        assert!(!w31a09_utf8_walk(&[0xc0, 0x80], &mut ok));
        assert!(!w31a09_utf8_walk(&[0xf5], &mut ok));
        let mut no = |_p: usize| false;
        assert!(!w31a09_utf8_walk(b"a", &mut no));
        assert!(w31a09_utf8_walk(&[1, 9, 0x13], &mut no));
    }

    #[test]
    fn drag() {
        let mut d = W31a09DragRect {
            anchor: (10, 20),
            ..Default::default()
        };
        w31a09_update_drag(&mut d, 5, 30, &mut || (10, 20));
        assert!(d.active);
        assert_eq!(d.rect, (5, 20, 10, 30));
        w31a09_update_drag(&mut d, 15, 2, &mut || (10, 20));
        assert_eq!(d.rect, (10, 2, 15, 20));
        assert!(w31a09_end_drag(&mut d));
        assert!(!w31a09_end_drag(&mut d));
    }

    #[test]
    fn scale() {
        assert_eq!(w31a09_scale_point(1.5, 2.0, 3, -3), (4, -6));
        assert_eq!(w31a09_unscale_point(2.0, 4.0, 5, -9), (2, -2));
        assert_eq!(
            w31a09_unscale_point_offset(2.0, 2.0, 4, 6, (100, 200)),
            (102, 203)
        );
    }
}
