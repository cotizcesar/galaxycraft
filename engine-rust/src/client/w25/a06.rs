#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent06.rs
// ===========================================================================
// Wave 25 agent 06: selection-list control helpers (menu widget).

/// Control fields touched by the list helpers: `cur` is the dword at `+0xb4`
/// (pending selection, `0xffff_ffff` none, `0xffff_fffe` suspended), `tail`
/// the dword at `+0xc0`.  `list_ready` is `*(list + 0x30) >= 0`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct W25a06Ctl {
    pub cur: u32,
    pub tail: u32,
    pub list_ready: bool,
}

fn w25a06_sel_byte(v: u32) -> u8 {
    if v == 0xffff_ffff {
        0xff
    } else {
        (v & 0xff) as u8
    }
}

/// `0x00897530`: runs the list command (`first`, `-1` aborts with -1); if the
/// list then holds exactly one entry it selects entry 0 (or stores 0 in `cur`
/// when the list is not ready).  Returns `count - 1`.
pub fn w25a06_select_single(
    ctl: &mut W25a06Ctl,
    first: i8,
    mut count: impl FnMut() -> u8,
    mut set_sel: impl FnMut(u8),
) -> i32 {
    if first == -1 {
        return -1;
    }
    if count() == 1 {
        if ctl.list_ready {
            set_sel(0);
            return i32::from(count()) - 1;
        }
        ctl.cur = 0;
    }
    i32::from(count()) - 1
}

/// `0x008975f0`: resets the list, clears the bucket table (`table` is the
/// `+0xbc` array; a non-zero entry reaches a non-returning call, reported as
/// `Err(index)`), zeroes `tail`, then clears the selection.
pub fn w25a06_reset_list(
    ctl: &mut W25a06Ctl,
    table: &mut [u32],
    mut reset: impl FnMut(),
    mut set_sel: impl FnMut(u8),
) -> Result<(), usize> {
    reset();
    for (i, e) in table.iter_mut().enumerate() {
        if *e != 0 {
            return Err(i);
        }
        *e = 0;
    }
    ctl.tail = 0;
    if !ctl.list_ready {
        ctl.cur = 0xffff_ffff;
    } else {
        set_sel(0xff);
    }
    Ok(())
}

/// `0x008976a0`: clears the selection (`0xff`) or marks `cur` as none.
pub fn w25a06_clear_selection(ctl: &mut W25a06Ctl, mut set_sel: impl FnMut(u8)) {
    if ctl.list_ready {
        set_sel(0xff);
    } else {
        ctl.cur = 0xffff_ffff;
    }
}

/// `0x008976f0`: sets the list flag to 1, then (unless already suspended)
/// applies the pending selection and marks `cur` suspended (`0xffff_fffe`).
pub fn w25a06_suspend_selection(
    ctl: &mut W25a06Ctl,
    mut set_flag: impl FnMut(u8),
    mut set_sel: impl FnMut(u8),
) {
    set_flag(1);
    if ctl.cur != 0xffff_fffe {
        if ctl.list_ready {
            set_sel(w25a06_sel_byte(ctl.cur));
        }
        ctl.cur = 0xffff_fffe;
    }
}

/// `0x00897c90`: selects `index` now, or stores it in `cur` if not ready.
pub fn w25a06_select_index(ctl: &mut W25a06Ctl, index: u32, mut set_sel: impl FnMut(u8)) {
    if ctl.list_ready {
        set_sel(w25a06_sel_byte(index));
    } else {
        ctl.cur = index;
    }
}

/// Hash used by the bucket lookup in `0x008979f0` (murmur3 finaliser).
pub fn w25a06_hash(key: u32) -> u32 {
    let mut h = (key >> 16 ^ key).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    h >> 16 ^ h
}

/// Bucket index for a table of `buckets` (power of two) entries.
pub fn w25a06_bucket(key: u32, buckets: u32) -> u32 {
    w25a06_hash(key) & buckets.wrapping_sub(1)
}

/// `0x008979f0`: finds the first entry whose looked-up record (`+0x10`,
/// `+0x14`) equals `(a, b)` and selects it.  `lookup(key)` yields that pair
/// (retail dereferences a null record on a miss; here a miss is a non-match).
/// Returns true when selected (index `0xffff_ffff` never occurs in the loop).
pub fn w25a06_select_by_pair(
    ctl: &mut W25a06Ctl,
    a: i32,
    b: i32,
    mut count: impl FnMut() -> u8,
    mut key_at: impl FnMut(u8) -> u32,
    mut lookup: impl FnMut(u32) -> Option<(i32, i32)>,
    mut set_sel: impl FnMut(u8),
) -> bool {
    let mut i: u32 = 0;
    if count() == 0 {
        return false;
    }
    loop {
        let key = key_at((i & 0xff) as u8);
        if lookup(key) == Some((a, b)) {
            if i == 0xffff_ffff {
                return false;
            }
            if ctl.list_ready {
                set_sel(w25a06_sel_byte(i));
            } else {
                ctl.cur = i;
            }
            return true;
        }
        i += 1;
        if (i as i32) >= i32::from(count()) {
            return false;
        }
    }
}

/// Shared body of the four wrappers: saves the dword at `+0xcc` of the
/// global context, runs `inner`, calls the apply routine with
/// `kind == 10` (the word at `+0xe`), and restores the dword.
pub fn w25a06_with_saved_cc(
    cc: &mut u32,
    kind: u16,
    inner: impl FnOnce(),
    apply: impl FnOnce(bool),
) {
    let saved = *cc;
    inner();
    apply(kind == 10);
    *cc = saved;
}

/// `0x008ae590`: wrapper over callee A.
pub fn w25a06_wrap_ae590(cc: &mut u32, kind: u16, inner: impl FnOnce(), apply: impl FnOnce(bool)) {
    w25a06_with_saved_cc(cc, kind, inner, apply)
}

/// `0x008ae5d0`: wrapper over callee B.
pub fn w25a06_wrap_ae5d0(cc: &mut u32, kind: u16, inner: impl FnOnce(), apply: impl FnOnce(bool)) {
    w25a06_with_saved_cc(cc, kind, inner, apply)
}

/// `0x008ae610`: wrapper over callee C.
pub fn w25a06_wrap_ae610(cc: &mut u32, kind: u16, inner: impl FnOnce(), apply: impl FnOnce(bool)) {
    w25a06_with_saved_cc(cc, kind, inner, apply)
}

/// `0x008b0540`: wrapper over callee D.
pub fn w25a06_wrap_b0540(cc: &mut u32, kind: u16, inner: impl FnOnce(), apply: impl FnOnce(bool)) {
    w25a06_with_saved_cc(cc, kind, inner, apply)
}

#[cfg(test)]
mod tests_wave25_agent06 {
    use super::*;

    #[test]
    fn select_single() {
        let mut c = W25a06Ctl {
            list_ready: true,
            ..Default::default()
        };
        assert_eq!(w25a06_select_single(&mut c, -1, || 3, |_| {}), -1);
        let mut s = vec![];
        assert_eq!(w25a06_select_single(&mut c, 0, || 1, |v| s.push(v)), 0);
        assert_eq!(s, vec![0]);
        c.list_ready = false;
        c.cur = 9;
        assert_eq!(w25a06_select_single(&mut c, 0, || 1, |_| {}), 0);
        assert_eq!(c.cur, 0);
        assert_eq!(w25a06_select_single(&mut c, 0, || 4, |_| {}), 3);
    }

    #[test]
    fn reset_and_clear() {
        let mut c = W25a06Ctl {
            cur: 5,
            tail: 7,
            list_ready: false,
        };
        let mut t = [0u32; 3];
        assert_eq!(w25a06_reset_list(&mut c, &mut t, || {}, |_| {}), Ok(()));
        assert_eq!((c.tail, c.cur), (0, 0xffff_ffff));
        let mut t = [0, 4];
        assert_eq!(w25a06_reset_list(&mut c, &mut t, || {}, |_| {}), Err(1));
        let mut s = vec![];
        c.list_ready = true;
        w25a06_clear_selection(&mut c, |v| s.push(v));
        assert_eq!(s, vec![0xff]);
        c.list_ready = false;
        w25a06_clear_selection(&mut c, |_| {});
        assert_eq!(c.cur, 0xffff_ffff);
    }

    #[test]
    fn suspend_and_index() {
        let mut c = W25a06Ctl {
            cur: 0x1_05,
            tail: 0,
            list_ready: true,
        };
        let (mut f, mut s) = (vec![], vec![]);
        w25a06_suspend_selection(&mut c, |v| f.push(v), |v| s.push(v));
        assert_eq!((f.clone(), s.clone()), (vec![1], vec![5]));
        assert_eq!(c.cur, 0xffff_fffe);
        w25a06_suspend_selection(&mut c, |v| f.push(v), |v| s.push(v));
        assert_eq!(s.len(), 1);
        w25a06_select_index(&mut c, 0xffff_ffff, |v| s.push(v));
        assert_eq!(s[1], 0xff);
        c.list_ready = false;
        w25a06_select_index(&mut c, 3, |_| {});
        assert_eq!(c.cur, 3);
    }

    #[test]
    fn hash_and_pair() {
        assert_eq!(w25a06_hash(0), 0);
        assert!(w25a06_bucket(12345, 16) < 16);
        let mut c = W25a06Ctl {
            cur: 0,
            tail: 0,
            list_ready: false,
        };
        let r = w25a06_select_by_pair(
            &mut c,
            1,
            2,
            || 3,
            |i| u32::from(i),
            |k| Some((k as i32, 2)),
            |_| {},
        );
        assert!(r);
        assert_eq!(c.cur, 1);
        let r = w25a06_select_by_pair(&mut c, 9, 9, || 3, |i| u32::from(i), |_| None, |_| {});
        assert!(!r);
    }

    #[test]
    fn wrappers() {
        let mut cc = 5u32;
        let mut got = None;
        w25a06_wrap_ae590(&mut cc, 10, || {}, |b| got = Some(b));
        assert_eq!(got, Some(true));
        w25a06_wrap_ae5d0(&mut cc, 3, || {}, |b| got = Some(b));
        assert_eq!(got, Some(false));
        w25a06_wrap_ae610(&mut cc, 3, || {}, |_| {});
        w25a06_wrap_b0540(&mut cc, 3, || {}, |_| {});
        assert_eq!(cc, 5);
    }
}
