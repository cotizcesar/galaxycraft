#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent18.rs
// ===========================================================================
// Wave 31 agent 18: small client/audio-object helpers.

/// `0x00964a30`: copies the entry `table[idx - 1]` into `*slot` when the
/// object has a positive 16-bit index (`+0x3a`), a non-zero count (`+0x3c`)
/// and the entry is non-zero.
pub fn w31a18_select_entry(idx: i16, count: i16, table: &[u32], slot: &mut u32) {
    if idx > 0 && count != 0 {
        if let Some(&v) = table.get(idx as usize - 1) {
            if v != 0 {
                *slot = v;
            }
        }
    }
}

/// `0x0097a320`: swaps `value` into `slots[index]` and returns the old one.
pub fn w31a18_swap_slot(slots: &mut [u32], index: u8, value: u32) -> u32 {
    std::mem::replace(&mut slots[index as usize], value)
}

/// `0x00989c70`: handle out-param validator. `0x25` for a missing output,
/// `0x24` for a null handle, else stores the handle and returns 0.
pub fn w31a18_store_handle(handle: u32, out: Option<&mut u32>) -> u32 {
    match out {
        None => 0x25,
        Some(_) if handle == 0 => 0x24,
        Some(o) => {
            *o = handle;
            0
        }
    }
}

/// `0x009945c0`: validates that `handle` is a member of the intrusive list
/// (`members` holds the handles currently linked).  Null handle gives
/// `0x24`, missing output `0x25`, a non-member `0x24`.
pub fn w31a18_validate_member(handle: u32, members: &[u32], out: Option<&mut u32>) -> u32 {
    if handle != 0 {
        let Some(o) = out else { return 0x25 };
        if members.contains(&handle) {
            *o = handle;
            return 0;
        }
    }
    0x24
}

/// `0x00a8fa40`: unpacks a `0xAABBGGRR` colour into four floats in `[0,1]`.
pub fn w31a18_unpack_color(c: u32) -> [f32; 4] {
    [
        (c & 0xff) as f32 / 255.0,
        (c >> 8 & 0xff) as f32 / 255.0,
        (c >> 16 & 0xff) as f32 / 255.0,
        (c >> 24) as f32 / 255.0,
    ]
}

/// Model of the voice-group object used by the volume/pan setters.
#[derive(Clone, Debug, Default)]
pub struct W31a18Group {
    pub has_primary: bool,
    pub voice_count: usize,
    pub primary_flags: u8,
    pub flags90: u8,
    pub volume: f32,
    pub pan: f32,
    pub pan_dirty: u32,
}

/// `0x00997ac0`: clamps `pan` to `[-1,1]`, stores it (`+0xf4`), clears the
/// dirty word (`+0x94`) and, unless the primary flag `0x10` is set, applies
/// it to each voice via `apply(i, pan)` returning the first non-zero error.
/// Returns `0x24` when there is no primary voice.
pub fn w31a18_set_pan(
    g: &mut W31a18Group,
    pan: f32,
    propagate: bool,
    mut apply: impl FnMut(usize, f32) -> i32,
) -> i32 {
    if !g.has_primary {
        return 0x24;
    }
    let p = pan.clamp(-1.0, 1.0);
    g.pan_dirty = 0;
    g.pan = p;
    if g.primary_flags & 0x10 != 0 {
        return 0;
    }
    let mut err = 0;
    if propagate {
        for i in 0..g.voice_count {
            let r = apply(i, p);
            if err == 0 {
                err = r;
            }
        }
    }
    err
}

/// `0x00998790`: clamps `vol` to `[0,1]`, stores it (`+0xec`), forces 0 when
/// flag `0x08` of `+0x90` is set, applies to every voice (last result kept)
/// and calls `refresh` when the value changed or `force` is set.
pub fn w31a18_set_volume(
    g: &mut W31a18Group,
    vol: f32,
    force: bool,
    mut apply: impl FnMut(usize, f32) -> u32,
    mut refresh: impl FnMut() -> u32,
) -> u32 {
    if !g.has_primary {
        return 0x24;
    }
    let v = vol.clamp(0.0, 1.0);
    let changed = g.volume != v;
    g.volume = v;
    let eff = if g.flags90 & 8 != 0 { 0.0 } else { v };
    let mut r = 0;
    for i in 0..g.voice_count {
        r = apply(i, eff);
    }
    if changed || force {
        r = refresh();
    }
    r
}

#[cfg(test)]
mod tests_wave31_agent18 {
    use super::*;

    #[test]
    fn entries_and_slots() {
        let mut s = 7;
        w31a18_select_entry(2, 1, &[5, 9], &mut s);
        assert_eq!(s, 9);
        w31a18_select_entry(0, 1, &[5, 9], &mut s);
        w31a18_select_entry(1, 0, &[5, 9], &mut s);
        w31a18_select_entry(1, 1, &[0], &mut s);
        assert_eq!(s, 9);
        let mut v = [1, 2];
        assert_eq!(w31a18_swap_slot(&mut v, 1, 8), 2);
        assert_eq!(v, [1, 8]);
    }

    #[test]
    fn handles() {
        let mut o = 0;
        assert_eq!(w31a18_store_handle(1, None), 0x25);
        assert_eq!(w31a18_store_handle(0, Some(&mut o)), 0x24);
        assert_eq!(w31a18_store_handle(3, Some(&mut o)), 0);
        assert_eq!(o, 3);
        let mut o2 = 0;
        assert_eq!(w31a18_validate_member(3, &[3], Some(&mut o2)), 0);
        assert_eq!(w31a18_validate_member(4, &[3], Some(&mut o2)), 0x24);
        assert_eq!(w31a18_validate_member(3, &[3], None), 0x25);
        assert_eq!(w31a18_validate_member(0, &[3], None), 0x24);
    }

    #[test]
    fn color() {
        let c = w31a18_unpack_color(0xff00_0080);
        assert_eq!(c[3], 1.0);
        assert_eq!(c[0], 128.0 / 255.0);
        assert_eq!(c[1], 0.0);
    }

    #[test]
    fn pan_volume() {
        let mut g = W31a18Group {
            has_primary: true,
            voice_count: 2,
            ..Default::default()
        };
        let mut calls = 0;
        assert_eq!(
            w31a18_set_pan(&mut g, 5.0, true, |_, p| {
                calls += 1;
                assert_eq!(p, 1.0);
                if calls == 2 { 9 } else { 0 }
            }),
            9
        );
        assert_eq!(g.pan, 1.0);
        g.primary_flags = 0x10;
        assert_eq!(w31a18_set_pan(&mut g, -3.0, true, |_, _| 1), 0);
        assert_eq!(g.pan, -1.0);
        g.has_primary = false;
        assert_eq!(w31a18_set_pan(&mut g, 0.0, false, |_, _| 0), 0x24);

        g.has_primary = true;
        g.flags90 = 8;
        let mut seen = vec![];
        let r = w31a18_set_volume(
            &mut g,
            2.0,
            false,
            |_, v| {
                seen.push(v);
                0
            },
            || 4,
        );
        assert_eq!(r, 4);
        assert_eq!(g.volume, 1.0);
        assert_eq!(seen, vec![0.0, 0.0]);
        assert_eq!(w31a18_set_volume(&mut g, 1.0, false, |_, _| 6, || 4), 6);
    }
}
