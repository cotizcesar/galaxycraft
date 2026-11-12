#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave21/agent05.rs
// ===========================================================================
// Wave 21 agent 05: movement-state helpers (object at `this`) and a hash lookup.

fn w21a05_r16(b: &[u8], o: usize) -> i16 {
    i16::from_le_bytes([b[o], b[o + 1]])
}
fn w21a05_r32(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn w21a05_w16(b: &mut [u8], o: usize, v: i16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w21a05_w32(b: &mut [u8], o: usize, v: i32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

/// `0x0056ca30`: stores the (x, y) pair at `+0x28/+0x2a` and its 8.8
/// fixed-point copies at `+0x2c/+0x30`, then calls FUN_0057d730 (`notify`).
pub fn w21a05_set_position(this: &mut [u8], x: i16, y: i16, mut notify: impl FnMut(i16, i16)) {
    w21a05_w32(this, 0x2c, (x as i32) << 8);
    w21a05_w16(this, 0x28, x);
    w21a05_w16(this, 0x2a, y);
    w21a05_w32(this, 0x30, (y as i32) << 8);
    notify(x, y);
}

/// Values written to the globals at 0xf95a98/9c/a8/ac/b0 by `0x0056cf20`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct W21a05NextPos {
    pub x: i16,
    pub y: i16,
    pub fx: i32,
    pub fy: i32,
    pub remaining: i32,
    /// True when the tail call FUN_0056c7e0(0) is made.
    pub stop: bool,
}

/// `0x0056cf20`: computes the next position.  `dist` models FUN_00604060.
pub fn w21a05_next_position(
    this: &[u8],
    mut dist: impl FnMut(i32, i32, i32, i32) -> i32,
) -> W21a05NextPos {
    let mut o = W21a05NextPos {
        remaining: w21a05_r32(this, 0x38),
        ..Default::default()
    };
    let rem = w21a05_r32(this, 0x38);
    if this[0x20] & 2 == 0 || rem == 0 {
        o.x = w21a05_r16(this, 0x28);
        o.y = w21a05_r16(this, 0x2a);
        o.fx = w21a05_r32(this, 0x2c);
        o.fy = w21a05_r32(this, 0x30);
        return o;
    }
    let d = dist(
        w21a05_r32(this, 0x2c),
        (w21a05_r16(this, 0x18) as i32) << 8,
        w21a05_r32(this, 0x30),
        (w21a05_r16(this, 0x1a) as i32) << 8,
    );
    if rem < d {
        o.fx = w21a05_r32(this, 0x40).wrapping_add(w21a05_r32(this, 0x2c));
        o.fy = w21a05_r32(this, 0x44).wrapping_add(w21a05_r32(this, 0x30));
        o.x = ((o.fx as u32) >> 8) as i16;
        o.y = ((o.fy as u32) >> 8) as i16;
    } else {
        o.x = w21a05_r16(this, 0x18);
        o.y = w21a05_r16(this, 0x1a);
        o.fx = (o.x as i32) << 8;
        o.fy = (o.y as i32) << 8;
        o.remaining = d;
    }
    o.stop = this[0x27] == 2;
    o
}

/// `0x0056d000`: updates flag bit 1 at `+0x20`; returns (DAT_00f95b10, DAT_00f95b11).
pub fn w21a05_update_moving_flag(this: &mut [u8]) -> (bool, bool) {
    let b = this[0x20];
    let (mut f10, mut f11) = (false, false);
    let same = w21a05_r16(this, 0x10) == w21a05_r16(this, 0x28)
        && w21a05_r16(this, 0x12) == w21a05_r16(this, 0x2a);
    if b & 2 != 0 && same {
        this[0x20] = b & 0xfd;
        if b & 8 == 0 {
            f11 = true;
        }
    } else if b & 2 == 0 && !same && (this[0x27] != 2 || b & 8 == 0) {
        this[0x20] = b | 2;
        f10 = b & 8 == 0;
    }
    (f10, f11)
}

/// `0x0056d080`: steps the signed turn state (`+0x21` heading, `+0x22` turn
/// rate, `+0x4b` desired heading); returns false when bits 1 set and 0 clear.
pub fn w21a05_step_heading(this: &mut [u8]) -> bool {
    let b = this[0x20];
    if b & 2 != 0 && b & 1 == 0 {
        return false;
    }
    let rate = this[0x22] as i8;
    let cur = this[0x21] as i8;
    let want = this[0x4b] as i8;
    let mut d = want.wrapping_sub(cur);
    if d < -0x7f {
        d = -0x80;
    }
    let new = if rate < d {
        rate.wrapping_add(cur)
    } else if (d as i32) < -(rate as i32) {
        cur.wrapping_sub(rate)
    } else {
        cur.wrapping_add(d)
    };
    this[0x21] = new as u8;
    let t = u16::from_le_bytes([this[0x24], this[0x25]]);
    if (t > 0x8c && t < 0xac) || (t > 200 && t < 0xcf) {
        this[0x22] = (rate.wrapping_add(1)) as u8;
    }
    if this[0x23] == this[0x4b] && new as u8 == this[0x4b] {
        this[0x20] = b & 0xfe;
    }
    true
}

/// Callbacks for `0x0056d130`.
pub struct W21a05SpeedCallbacks<'a> {
    /// FUN_0056c7e0(speed)
    pub set_speed: &'a mut dyn FnMut(u32),
    /// FUN_00604060(x0, y0, x1, y1)
    pub dist: &'a mut dyn FnMut(i32, i32, i32, i32) -> u32,
    /// FUN_00569770()
    pub stop_dist: &'a mut dyn FnMut() -> u32,
    /// FUN_00569580(accel)
    pub accelerate: &'a mut dyn FnMut(i32),
}

fn w21a05_turn_delta(a: u8, b: u8) -> u32 {
    let u = (b as u32).wrapping_sub(a as u32) & 0xff;
    if u < 0x81 { u } else { 0x100 - u }
}

/// `0x0056d130`: chooses acceleration or braking.  `dir_table` is the
/// 8-byte-stride (dx, dy) table at 0xf031b8 indexed by `+0x4a`.
pub fn w21a05_update_speed(
    this: &mut [u8],
    dir_table: &[(i32, i32)],
    cb: &mut W21a05SpeedCallbacks,
) {
    let same = w21a05_r16(this, 0x10) == w21a05_r16(this, 0x28)
        && w21a05_r16(this, 0x12) == w21a05_r16(this, 0x2a);
    let kind = this[0x27];
    match kind {
        0 => {
            if same && w21a05_r32(this, 0x3c) < 0xc0 && this[0x20] & 0x30 == 0 {
                this[0x20] &= 0xfb;
                (cb.set_speed)(0);
                return;
            }
        }
        1 => {
            if same {
                this[0x20] &= 0xfb;
                (cb.set_speed)(0);
                return;
            }
        }
        2 => {
            if !same {
                let b = this[0x20];
                if b & 1 != 0 && w21a05_turn_delta(this[0x4b], this[0x21]) >= 0x20 {
                    // note: pseudocode computes (+0x21) - (+0x4b)
                    if b & 2 == 0 {
                        return;
                    }
                    this[0x20] = b & 0xf9;
                    (cb.set_speed)(0);
                    return;
                }
                (cb.set_speed)(w21a05_r32(this, 0x3c) as u32);
                return;
            }
            this[0x20] &= 0xfb;
            (cb.set_speed)(0);
            return;
        }
        _ => {}
    }
    let i3c = w21a05_r32(this, 0x3c);
    if w21a05_r32(this, 0x38) != i3c {
        w21a05_w32(this, 0x38, i3c);
        let e = dir_table[this[0x4a] as usize];
        w21a05_w32(this, 0x40, e.0.wrapping_mul(i3c) >> 8);
        w21a05_w32(this, 0x44, e.1.wrapping_mul(i3c) >> 8);
    }
    let u5 = (cb.dist)(
        w21a05_r16(this, 0x28) as i32,
        w21a05_r16(this, 0x18) as i32,
        w21a05_r16(this, 0x2a) as i32,
        w21a05_r16(this, 0x1a) as i32,
    );
    let mut brake = false;
    if this[0x4a] != this[0x4b] && u5 < 0x20 {
        let u4 = w21a05_turn_delta(this[0x4a], this[0x4b]);
        let rate = this[0x22] as i32;
        let num = (rate - 1).wrapping_add(u4.wrapping_mul(2) as i32);
        let q = num.checked_div(rate).unwrap_or(0);
        let thr = ((q.wrapping_mul(i3c) as u32) >> 8).wrapping_mul(3) >> 1;
        if u5 < thr {
            brake = true;
        }
    }
    if !brake {
        let b = this[0x20];
        if this[0x27] == 0
            && (b & 0x10 == 0 || b & 0x40 != 0)
            && (b & 0x20 == 0 || w21a05_r32(this, 0x18) != w21a05_r32(this, 0x10))
        {
            let u5 = (cb.stop_dist)();
            let u4 = (cb.dist)(
                w21a05_r32(this, 0x2c),
                (w21a05_r16(this, 0x18) as i32) << 8,
                w21a05_r32(this, 0x30),
                (w21a05_r16(this, 0x1a) as i32) << 8,
            );
            if u4 <= u5 {
                brake = true;
            }
        }
    }
    let acc = u16::from_le_bytes([this[0x48], this[0x49]]) as u32;
    if brake {
        (cb.accelerate)(0u32.wrapping_sub(acc) as i32);
    } else {
        (cb.accelerate)(acc as i32);
    }
}

/// `0x0056dd90` (hash part): bucket index for a u16 key in a table of
/// `size` (power of two) buckets.
pub fn w21a05_hash_u16(key: u16, size: u32) -> u32 {
    let mut h = (key as u32).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    (h >> 16 ^ h) & size.wrapping_sub(1)
}

/// `0x0056dd90`: chained-hash lookup.  `buckets[i]` is the head node index,
/// `nodes[n] = (next, key)`.  Returns the node index, or `None` where the
/// retail code falls into its error path.
pub fn w21a05_hash_find(
    buckets: &[Option<usize>],
    nodes: &[(Option<usize>, u16)],
    key: u16,
) -> Option<usize> {
    let idx = w21a05_hash_u16(key, buckets.len() as u32) as usize;
    let mut cur = buckets[idx];
    while let Some(n) = cur {
        if nodes[n].1 == key {
            return Some(n);
        }
        cur = nodes[n].0;
    }
    None
}

#[cfg(test)]
mod tests_wave21_agent05 {
    use super::*;

    #[test]
    fn set_position() {
        let mut b = [0u8; 0x50];
        let mut got = None;
        w21a05_set_position(&mut b, -2, 3, |x, y| got = Some((x, y)));
        assert_eq!(w21a05_r32(&b, 0x2c), -512);
        assert_eq!(w21a05_r32(&b, 0x30), 768);
        assert_eq!(w21a05_r16(&b, 0x28), -2);
        assert_eq!(got, Some((-2, 3)));
    }

    #[test]
    fn next_pos() {
        let mut b = [0u8; 0x50];
        w21a05_w16(&mut b, 0x28, 5);
        let o = w21a05_next_position(&b, |_, _, _, _| 0);
        assert_eq!(o.x, 5);
        b[0x20] = 2;
        w21a05_w32(&mut b, 0x38, 10);
        w21a05_w32(&mut b, 0x40, 256);
        w21a05_w32(&mut b, 0x2c, 256);
        let o = w21a05_next_position(&b, |_, _, _, _| 100);
        assert_eq!((o.x, o.fx, o.remaining, o.stop), (2, 512, 10, false));
        let o = w21a05_next_position(&b, |_, _, _, _| 4);
        assert_eq!((o.x, o.remaining), (0, 4));
        b[0x27] = 2;
        assert!(w21a05_next_position(&b, |_, _, _, _| 4).stop);
    }

    #[test]
    fn moving_flag() {
        let mut b = [0u8; 0x50];
        w21a05_w16(&mut b, 0x10, 1);
        assert_eq!(w21a05_update_moving_flag(&mut b), (true, false));
        assert_eq!(b[0x20], 2);
        w21a05_w16(&mut b, 0x28, 1);
        assert_eq!(w21a05_update_moving_flag(&mut b), (false, true));
        assert_eq!(b[0x20], 0);
    }

    #[test]
    fn heading() {
        let mut b = [0u8; 0x50];
        b[0x20] = 2;
        assert!(!w21a05_step_heading(&mut b));
        b[0x20] = 1;
        b[0x22] = 8;
        b[0x4b] = 100;
        b[0x23] = 100;
        assert!(w21a05_step_heading(&mut b));
        assert_eq!(b[0x21], 8);
        b[0x21] = 96;
        b[0x24] = 0x90;
        assert!(w21a05_step_heading(&mut b));
        assert_eq!((b[0x21], b[0x22], b[0x20]), (100, 9, 0));
    }

    #[test]
    fn speed_type1_stops() {
        let mut b = [0u8; 0x50];
        b[0x27] = 1;
        b[0x20] = 4;
        let mut calls = vec![];
        let mut s = |v: u32| calls.push(v);
        let mut d = |_, _, _, _| 0u32;
        let mut sd = || 0u32;
        let mut a = |_| {};
        let mut cb = W21a05SpeedCallbacks {
            set_speed: &mut s,
            dist: &mut d,
            stop_dist: &mut sd,
            accelerate: &mut a,
        };
        w21a05_update_speed(&mut b, &[(0, 0); 256], &mut cb);
        assert_eq!(b[0x20], 0);
        assert_eq!(calls, vec![0]);
    }

    #[test]
    fn speed_accel_and_brake() {
        let mut b = [0u8; 0x50];
        b[0x27] = 3;
        b[0x48] = 7;
        w21a05_w32(&mut b, 0x3c, 256);
        let mut acc = vec![];
        let mut s = |_: u32| {};
        let mut d = |_, _, _, _| 100u32;
        let mut sd = || 0u32;
        let mut a = |v: i32| acc.push(v);
        let mut cb = W21a05SpeedCallbacks {
            set_speed: &mut s,
            dist: &mut d,
            stop_dist: &mut sd,
            accelerate: &mut a,
        };
        let mut tbl = [(0, 0); 256];
        tbl[0] = (512, 256);
        w21a05_update_speed(&mut b, &tbl, &mut cb);
        assert_eq!(w21a05_r32(&b, 0x40), 512);
        assert_eq!(w21a05_r32(&b, 0x44), 256);
        assert_eq!(w21a05_r32(&b, 0x38), 256);
        assert_eq!(acc, vec![7]);
        // brake: type 0 path, stop distance larger than remaining
        b[0x27] = 0;
        let mut acc2 = vec![];
        let mut s = |_: u32| {};
        let mut d = |_, _, _, _| 5u32;
        let mut sd = || 9u32;
        let mut a = |v: i32| acc2.push(v);
        let mut cb = W21a05SpeedCallbacks {
            set_speed: &mut s,
            dist: &mut d,
            stop_dist: &mut sd,
            accelerate: &mut a,
        };
        w21a05_w32(&mut b, 0x3c, 0x100);
        w21a05_update_speed(&mut b, &tbl, &mut cb);
        assert_eq!(acc2, vec![-7]);
    }

    #[test]
    fn hash() {
        assert_eq!(w21a05_hash_u16(0, 16), 0);
        let buckets_n = 8u32;
        let i = w21a05_hash_u16(0x1234, buckets_n) as usize;
        assert!(i < 8);
        let mut buckets = vec![None; 8];
        buckets[i] = Some(0);
        let nodes = vec![(Some(1), 5u16), (None, 0x1234)];
        assert_eq!(w21a05_hash_find(&buckets, &nodes, 0x1234), Some(1));
        assert_eq!(w21a05_hash_find(&buckets, &nodes, 0x4321), None);
    }
}
