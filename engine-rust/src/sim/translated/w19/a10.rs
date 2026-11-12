#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave19/agent10.rs
// ===========================================================================
// Wave 19 translation batch 10: small map/tile and dispatch helpers.

fn w19a10_u16(b: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([b[off], b[off + 1]])
}

/// `0x006f3490`: resolves a table entry for `param_2 + 0xf` and hands it,
/// with the 16-bit value derived from `param_1`, to the sink.  The three
/// callees are caller-supplied closures.
pub fn w19a10_f3490<T>(
    param_1: u32,
    param_2: u8,
    lookup: impl FnOnce(u32) -> T,
    derive: impl FnOnce(u32) -> u16,
    sink: impl FnOnce(T, u16),
) {
    let entry = lookup(u32::from(param_2) + 0xf);
    let v = derive(param_1);
    sink(entry, v);
}

/// `0x0075d9f0`: megatile id lookup.  `map` is the map block: u16 grid at
/// `+0xc` with row stride 0x100 entries, indexed by `(y / 32) * 0x100 +
/// x / 32` (division truncating toward zero like the compiler's bias).  If
/// the value exceeds 0x1fff the result is
/// `(v*3 >> 16) << 16 | u16 at 0x1400e + v*6`.
pub fn w19a10_75d9f0(map: &[u8], x: i32, y: i32) -> u32 {
    let gx = (x.wrapping_add((x >> 31) & 0x1f)) >> 5;
    let gy = (y.wrapping_add((y >> 31) & 0x1f)) >> 5;
    let idx = gy.wrapping_mul(0x100).wrapping_add(gx) as usize;
    let v = u32::from(w19a10_u16(map, 0xc + idx * 2));
    if v > 0x1fff {
        let hi = (v * 3) >> 16;
        let lo = u32::from(w19a10_u16(map, 0x1400e + v as usize * 6));
        (hi << 16) | lo
    } else {
        v
    }
}

/// `0x00754830`: probes `(a, b)`; if the probe is non-zero, fires cue 0x1f
/// and returns it.  Else if `flag` is set, fires cue 0x1f and returns 0;
/// otherwise performs the placement action and returns 0.
pub fn w19a10_754830(
    flag: i32,
    probe: impl FnOnce() -> i32,
    mut cue: impl FnMut(u32),
    place: impl FnOnce(),
) -> i32 {
    let r = probe();
    if r != 0 {
        cue(0x1f);
        return r;
    }
    if flag != 0 {
        cue(0x1f);
        return 0;
    }
    place();
    0
}

/// `0x00983b80`: true when both per-type flag tables have bit 0 set for
/// `kind & 0xffff`, the cell `(cx, cy)` lies inside the bounds `[x0,y0,x1,y1]`
/// produced by the extent routine, and the 32-pixel-cell centre
/// `(cx*32 - px + 16, cy*32 - py + 16)` is not rejected by `reject`.
#[allow(clippy::too_many_arguments)]
pub fn w19a10_983b80(
    cx: i32,
    cy: i32,
    kind: u32,
    px: i32,
    py: i32,
    flags_a: &[u32],
    flags_b: &[u32],
    bounds: impl FnOnce() -> [i32; 4],
    reject: impl FnOnce(i32, i32) -> bool,
) -> bool {
    let k = (kind & 0xffff) as usize;
    if flags_a[k * 4] & 1 == 0 || flags_b[k] & 1 == 0 {
        return false;
    }
    let [x0, y0, x1, y1] = bounds();
    if x0 <= cx && cx <= x1 && y0 <= cy && cy <= y1 {
        return !reject(cx * 32 - px + 16, cy * 32 - py + 16);
    }
    false
}

/// `0x00983f10`: runs the cell walk with the default callback and returns 1.
pub fn w19a10_983f10(walk: impl FnOnce()) -> u32 {
    walk();
    1
}

/// `0x00983f60`: runs the secondary walk, then if a hook is installed and
/// returns non-zero returns 1; otherwise runs the default walk and returns 1.
pub fn w19a10_983f60(
    walk2: impl FnOnce(),
    hook: Option<impl FnOnce() -> i32>,
    walk: impl FnOnce(),
) -> u32 {
    walk2();
    if let Some(h) = hook
        && h() != 0
    {
        return 1;
    }
    walk();
    1
}

/// Inputs for [`w19a10_79eaf0`] (retail globals made explicit).
pub struct W19a10TileCtx<'a> {
    /// map width in cells (`+0xe4` of the map header)
    pub width: u32,
    /// u16 cell array (stride 2)
    pub cells16: &'a mut [u16],
    /// u32 cell flag array
    pub flags32: &'a mut [u32],
    /// tile group table: group stride 0x34 (u16 at +2 = flags), u16 at
    /// `0x14 + (group*0x1a + sub)*2` = tile index
    pub groups: &'a [u8],
    /// per-tile u32 flag table
    pub tile_flags: &'a [u32],
    /// retail mode predicate result
    pub alt_mode: bool,
    /// global mask used when `alt_mode` is false
    pub mask_a: u32,
    /// global byte used when `alt_mode` is true
    pub mask_b: u8,
    /// companion global tested when `alt_mode` is true
    pub mask_c: u32,
}

/// `0x0079eaf0`: updates one map cell from a packed tile value `tile`
/// (`group = tile >> 4 & 0x7ff`, `sub = tile & 0xf`).  `mode == 2` only sets
/// bit 0x8000 of the cell and calls `after_set`; otherwise the cell takes
/// the group's tile index (when the visibility gate passes), toggles bit
/// 30 of the flags, then merges group/tile flags under mask 0x45_0000,
/// clears 0x40000 handling and calls `refresh`.
pub fn w19a10_79eaf0(
    c: &mut W19a10TileCtx,
    x: usize,
    y: usize,
    mode: i32,
    tile: u16,
    mut after_set: impl FnMut(),
    mut refresh: impl FnMut(usize, usize),
) {
    let i = c.width as usize * y + x;
    if mode == 2 {
        c.cells16[i] |= 0x8000;
        after_set();
        return;
    }
    let group = usize::from(tile >> 4 & 0x7ff);
    let sub = usize::from(tile & 0xf);
    let f = c.flags32[i];
    let skip = if !c.alt_mode {
        f & c.mask_a != 0
    } else {
        (c.mask_b & !(f as u8)) == 0 && c.mask_c == 0
    };
    if !skip {
        c.cells16[i] = w19a10_u16(c.groups, 0x14 + (group * 0x1a + sub) * 2);
        c.flags32[i] = if mode != 0 {
            c.flags32[i] & 0xbfff_ffff
        } else {
            c.flags32[i] | 0x4000_0000
        };
        after_set();
    }
    let tidx = usize::from(w19a10_u16(c.groups, 0x14 + (group * 0x1a + sub) * 2) & 0x7fff);
    let gf = u32::from(w19a10_u16(c.groups, group * 0x34 + 2)) & 0xffff_fffa;
    let merged = (gf << 16) | c.tile_flags[tidx];
    let mut v = c.flags32[i];
    v ^= (merged ^ v) & 0x45_0000;
    if v & 0x1_0000 != 0 {
        v &= 0xfffb_ffff;
    }
    if v & 0x45_0000 == 0 {
        v |= 0x4_0000;
    }
    c.flags32[i] = v;
    refresh(x, y);
}

#[cfg(test)]
mod tests_wave19_agent10 {
    use super::*;

    #[test]
    fn lookup_f3490() {
        let mut out = None;
        w19a10_f3490(5, 1, |k| k * 2, |p| p as u16 + 1, |a, b| out = Some((a, b)));
        assert_eq!(out, Some((32, 6)));
    }

    #[test]
    fn megatile() {
        let mut m = vec![0u8; 0x1400e + 0x2001 * 6 + 2];
        let put = |m: &mut Vec<u8>, o: usize, v: u16| m[o..o + 2].copy_from_slice(&v.to_le_bytes());
        put(&mut m, 0xc + (0x100 + 2) * 2, 0x123);
        assert_eq!(w19a10_75d9f0(&m, 64, 32), 0x123);
        put(&mut m, 0xc, 0x2001);
        put(&mut m, 0x1400e + 0x2001 * 6, 0xbeef);
        assert_eq!(w19a10_75d9f0(&m, 0, 0), 0xbeef);
        // negative coordinate truncates toward zero -> cell 0
        assert_eq!(w19a10_75d9f0(&m, -1, -1), 0xbeef);
    }

    #[test]
    fn place_flow() {
        let mut cues = vec![];
        let mut placed = false;
        assert_eq!(
            w19a10_754830(0, || 4, |c| cues.push(c), || placed = true),
            4
        );
        assert_eq!(
            w19a10_754830(1, || 0, |c| cues.push(c), || placed = true),
            0
        );
        assert!(!placed);
        assert_eq!(cues, [0x1f, 0x1f]);
        w19a10_754830(0, || 0, |c| cues.push(c), || placed = true);
        assert!(placed);
    }

    #[test]
    fn cell_in_bounds() {
        let a = vec![1u32; 8];
        let b = vec![1u32; 2];
        let r = w19a10_983b80(
            2,
            2,
            1,
            0,
            0,
            &a,
            &b,
            || [1, 1, 3, 3],
            |x, y| (x, y) == (80, 80),
        );
        assert!(!r);
        let r = w19a10_983b80(2, 2, 1, 0, 0, &a, &b, || [3, 1, 3, 3], |_, _| false);
        assert!(!r);
        let r = w19a10_983b80(2, 2, 1, 0, 0, &a, &b, || [1, 1, 3, 3], |_, _| false);
        assert!(r);
        let z = vec![0u32; 8];
        assert!(!w19a10_983b80(
            2,
            2,
            1,
            0,
            0,
            &z,
            &b,
            || [1, 1, 3, 3],
            |_, _| false
        ));
    }

    #[test]
    fn walks() {
        let mut n = 0;
        assert_eq!(w19a10_983f10(|| n += 1), 1);
        let o = std::cell::RefCell::new(Vec::new());
        w19a10_983f60(
            || o.borrow_mut().push(1),
            Some(|| 1),
            || o.borrow_mut().push(3),
        );
        assert_eq!(*o.borrow(), vec![1]);
        w19a10_983f60(
            || o.borrow_mut().push(1),
            None::<fn() -> i32>,
            || o.borrow_mut().push(3),
        );
        assert_eq!(*o.borrow(), vec![1, 1, 3]);
    }

    #[test]
    fn tile_update() {
        let mut groups = vec![0u8; 0x34 * 2 + 0x14 + 0x1a * 2];
        // group 1, sub 2 -> tile index 5
        let off = 0x14 + (0x1a + 2) * 2;
        groups[off] = 5;
        let mut c16 = vec![0u16; 4];
        let mut f32v = vec![0u32; 4];
        let tf = vec![0, 0, 0, 0, 0, 0x10000];
        let mut ctx = W19a10TileCtx {
            width: 2,
            cells16: &mut c16,
            flags32: &mut f32v,
            groups: &groups,
            tile_flags: &tf,
            alt_mode: false,
            mask_a: 0,
            mask_b: 0,
            mask_c: 0,
        };
        let mut calls = 0;
        let mut refreshed = None;
        w19a10_79eaf0(
            &mut ctx,
            1,
            1,
            0,
            (1 << 4) | 2,
            || calls += 1,
            |x, y| refreshed = Some((x, y)),
        );
        assert_eq!(ctx.cells16[3], 5);
        assert_eq!(calls, 1);
        assert_eq!(refreshed, Some((1, 1)));
        // 0x10000 bit merged then 0x40000 mask logic: 0x10000 set -> cleared 0x40000 bit only
        assert_eq!(ctx.flags32[3] & 0x4000_0000, 0x4000_0000);
        assert_eq!(ctx.flags32[3] & 0x45_0000, 0x1_0000);
        w19a10_79eaf0(&mut ctx, 0, 0, 2, 0, || calls += 1, |_, _| {});
        assert_eq!(ctx.cells16[0], 0x8000);
        assert_eq!(calls, 2);
    }
}

#[cfg(test)]
mod refinery_placement_tests {
    use super::*;

    #[test]
    fn a_refinery_goes_on_the_geyser_nearest_to_the_cursor() {
        let data = Arc::new(GameData::bundled().expect("bundled tables"));
        let mut w = World::new(data, Terrain::open(64, 64), 2);
        let first = w.spawn_px(0, uid::VESPENE_GEYSER, 200, 200).unwrap();
        let second = w.spawn_px(0, uid::VESPENE_GEYSER, 1500, 1200).unwrap();
        let (sx, sy) = (w.units[second].x / fp(1), w.units[second].y / fp(1));
        let spot = w.placement_spot(uid::REFINERY, (sx, sy));
        assert_eq!(
            spot,
            Some((sx, sy)),
            "must not fall back to the first geyser"
        );
        // An under-construction refinery already occupies its geyser.
        let r = w.spawn_px(0, uid::REFINERY, sx, sy).unwrap();
        w.units[r].completed = false;
        assert_eq!(w.placement_spot(uid::REFINERY, (sx, sy)), None);
        let _ = first;
    }
}
