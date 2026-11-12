#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave17/agent09.rs
// ===========================================================================
// Wave 17 translation batch 09: sim-side UI/selection helpers.

/// `0x00780b30`: returns true when the queried helper result is zero.
pub fn w17a09_is_zero(helper_result: i32) -> bool {
    helper_result == 0
}

/// `0x00780db0`: scans the 12 slot words at the slot table (`bare 011b1fa8`..`bare 011b1fd7`);
/// returns false as soon as a non-zero slot satisfies `slot_matches`, otherwise
/// falls through to `fallback` (the final helper call) and returns its truth.
pub fn w17a09_slots_all_clear(
    slots: &[i32],
    slot_matches: impl Fn(i32) -> bool,
    fallback: impl FnOnce() -> bool,
) -> bool {
    for &s in slots.iter().take(12) {
        if s != 0 && slot_matches(s) {
            return false;
        }
    }
    fallback()
}

/// `0x0079f020`: true when `probe(a, flags 0x20800000)` hits; else, unless
/// `index + 1` (as i16) equals `limit`, the result of `probe(a + 1, 0x80_0000)`.
pub fn w17a09_probe_pair(index: i32, limit: i16, mut probe: impl FnMut(i32, u32) -> bool) -> bool {
    if probe(index, 0x2080_0000) {
        return true;
    }
    if (index.wrapping_add(1) as i16) == limit {
        return false;
    }
    probe(index.wrapping_add(1), 0x80_0000)
}

/// `0x00794960`: palette colour lookup.  `divisor_tbl` is the u16 table at
/// `bare 00f8e8e8` indexed by `kind`; `pick_tbl` (`bare 00f308f8`) is indexed by
/// `((value * 7) / divisor) * 2 + sub`; `remap` (`bare 011b20b8`) and `colors`
/// (`bare 00fa9068`) are indexed next.  `None` on zero divisor or out of range.
pub fn w17a09_palette_color(
    kind: u16,
    value: i32,
    sub: i32,
    divisor_tbl: &[u16],
    pick_tbl: &[u8],
    remap: &[u8],
    colors: &[u32],
) -> Option<u32> {
    let div = i32::from(*divisor_tbl.get(kind as usize)?);
    let q = (value.wrapping_mul(7)).checked_div(div)?;
    let i = usize::try_from(q.wrapping_mul(2).wrapping_add(sub)).ok()?;
    let p = *pick_tbl.get(i)?;
    let r = *remap.get(p as usize)?;
    colors.get(r as usize).copied()
}

/// Actions the dialog-focus routines request from the caller.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum W17a09FocusAction {
    /// Do nothing.
    None,
    /// Zero the widget's pointer (+0x40), set its id (+0x3e) to 0xffff and refresh it.
    ClearWidget,
    /// Focus `DAT_011b1fa4` with id `FUN_0078fdd0(FUN_005b4ce0())`, flag 1.
    Refocus,
}

/// `0x0078faa0`: with a target present, drops the pending handle (calls `release`
/// and clears it) then requests a refocus.
pub fn w17a09_refocus_release(
    target: i32,
    pending: &mut i32,
    release: impl FnOnce(),
) -> W17a09FocusAction {
    if target == 0 {
        return W17a09FocusAction::None;
    }
    if *pending != 0 {
        release();
        *pending = 0;
    }
    W17a09FocusAction::Refocus
}

/// `0x0078faf0`: requests a refocus only when both the widget (`bare 011b1e5c`)
/// and the target (`bare 011b1fa4`) exist.
pub fn w17a09_refocus_if_both(widget: i32, target: i32) -> W17a09FocusAction {
    if widget != 0 && target != 0 {
        W17a09FocusAction::Refocus
    } else {
        W17a09FocusAction::None
    }
}

/// `0x00791f30`: deferred-refocus flag (`bare 011b1df4`).  Always clears the flag
/// when set; the returned action tells what to do first.  `mode_is_two` is
/// `FUN_0078fc00()+4 == 2`; `widget_target` is the widget's +0x40 pointer;
/// `id_a`/`id_b` are two reads of `FUN_005b4ce0`.
pub fn w17a09_deferred_refocus(
    flag: &mut bool,
    e8c: i32,
    mode_is_two: bool,
    target: i32,
    widget_target: i32,
    id_a: i16,
    id_b: i16,
) -> W17a09FocusAction {
    if !*flag {
        return W17a09FocusAction::None;
    }
    *flag = false;
    if e8c != 0 && mode_is_two {
        return W17a09FocusAction::None;
    }
    if target == 0 {
        return W17a09FocusAction::ClearWidget;
    }
    if widget_target == target && (widget_target == 0 || id_a == id_b) {
        return W17a09FocusAction::None;
    }
    W17a09FocusAction::Refocus
}

#[cfg(test)]
mod tests_wave17_agent09 {
    use super::*;

    #[test]
    fn zero_and_slots() {
        assert!(w17a09_is_zero(0));
        assert!(!w17a09_is_zero(3));
        assert!(!w17a09_slots_all_clear(&[0, 5], |s| s == 5, || true));
        assert!(w17a09_slots_all_clear(&[0, 5], |_| false, || true));
        assert!(!w17a09_slots_all_clear(&[0], |_| false, || false));
    }

    #[test]
    fn probe_pair() {
        assert!(w17a09_probe_pair(3, 9, |_, f| f == 0x2080_0000));
        assert!(!w17a09_probe_pair(3, 4, |_, _| false));
        let mut seen = vec![];
        let r = w17a09_probe_pair(3, 9, |i, f| {
            seen.push((i, f));
            f == 0x80_0000
        });
        assert!(r);
        assert_eq!(seen, vec![(3, 0x2080_0000), (4, 0x80_0000)]);
    }

    #[test]
    fn palette() {
        let div = [0u16, 7];
        let pick = [0u8, 1, 1, 0];
        let remap = [2u8, 1];
        let colors = [10u32, 20, 30];
        assert_eq!(
            w17a09_palette_color(1, 1, 0, &div, &pick, &remap, &colors),
            Some(20)
        );
        assert_eq!(
            w17a09_palette_color(1, 1, 1, &div, &pick, &remap, &colors),
            Some(30)
        );
        assert_eq!(
            w17a09_palette_color(0, 1, 0, &div, &pick, &remap, &colors),
            None
        );
        assert_eq!(
            w17a09_palette_color(5, 1, 0, &div, &pick, &remap, &colors),
            None
        );
    }

    #[test]
    fn refocus() {
        let mut p = 1;
        let mut released = false;
        assert_eq!(
            w17a09_refocus_release(1, &mut p, || released = true),
            W17a09FocusAction::Refocus
        );
        assert!(released && p == 0);
        assert_eq!(
            w17a09_refocus_release(0, &mut p, || {}),
            W17a09FocusAction::None
        );
        assert_eq!(w17a09_refocus_if_both(1, 1), W17a09FocusAction::Refocus);
        assert_eq!(w17a09_refocus_if_both(1, 0), W17a09FocusAction::None);
    }

    #[test]
    fn deferred() {
        let mut f = false;
        assert_eq!(
            w17a09_deferred_refocus(&mut f, 0, false, 1, 1, 0, 1),
            W17a09FocusAction::None
        );
        f = true;
        assert_eq!(
            w17a09_deferred_refocus(&mut f, 1, true, 1, 0, 0, 0),
            W17a09FocusAction::None
        );
        assert!(!f);
        f = true;
        assert_eq!(
            w17a09_deferred_refocus(&mut f, 0, false, 0, 0, 0, 0),
            W17a09FocusAction::ClearWidget
        );
        f = true;
        assert_eq!(
            w17a09_deferred_refocus(&mut f, 0, false, 5, 5, 2, 2),
            W17a09FocusAction::None
        );
        f = true;
        assert_eq!(
            w17a09_deferred_refocus(&mut f, 0, false, 5, 5, 2, 3),
            W17a09FocusAction::Refocus
        );
        f = true;
        assert_eq!(
            w17a09_deferred_refocus(&mut f, 0, false, 5, 6, 2, 2),
            W17a09FocusAction::Refocus
        );
    }
}
