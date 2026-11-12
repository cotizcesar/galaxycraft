#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent13.rs
// ===========================================================================
// Wave 31 agent 13 translations (client glue helpers).

/// `0x00878300`: resolves `handle`; when the lookup is non-zero the
/// result is passed to the follow-up call.  Returns whether it fired.
pub fn w31a13_resolve_and_notify(
    handle: u32,
    lookup: impl Fn(u32) -> i32,
    mut notify: impl FnMut(i32),
) -> bool {
    let r = lookup(handle);
    if r != 0 {
        notify(r);
        true
    } else {
        false
    }
}

/// `0x00878350`: forwards the global at `DAT_00fc9d8c` to its handler.
pub fn w31a13_forward_global(global: u32, mut f: impl FnMut(u32)) {
    f(global);
}

/// `0x00878480`: calls the first handler with the global, then the second.
pub fn w31a13_forward_then_finish(
    global: u32,
    mut first: impl FnMut(u32),
    mut second: impl FnMut(),
) {
    first(global);
    second();
}

/// `0x0087b450`: releases the handle (low 16 bits of field `+0x4c`) and
/// resets `+0x4c` to -1 and `+0x48` to 0.
pub fn w31a13_release_handle_slot(
    field_48: &mut u32,
    field_4c: &mut u32,
    mut release: impl FnMut(u16),
) {
    release(*field_4c as u16);
    *field_4c = 0xffff_ffff;
    *field_48 = 0;
}

/// `0x0087c1f0`: invokes the optional callback (object `+0x18`) with the
/// object's context (`+4`) and four arguments.
pub fn w31a13_invoke_callback(
    callback: Option<&dyn Fn(u32, u32, u32, u32, u32)>,
    context: u32,
    args: [u32; 4],
) -> bool {
    match callback {
        Some(cb) => {
            cb(context, args[0], args[1], args[2], args[3]);
            true
        }
        None => false,
    }
}

/// `0x00880ca0`: collects the entries of the handle list whose looked-up
/// record has type `0xf` at offset `+0x3c`, in order.
pub fn w31a13_collect_type_f(handles: &[u32], record_type: impl Fn(u32) -> u16) -> Vec<u32> {
    handles
        .iter()
        .copied()
        .filter(|&h| record_type(h) == 0xf)
        .collect()
}

/// `0x008964f0`: after a pre-call, scans entries `0..count` for one equal to
/// `(wanted, 0)`; when found at an index other than `current`, removes it.
/// Returns the removed index.
pub fn w31a13_remove_matching(
    wanted: u32,
    entries: &[(u32, u32)],
    current: i32,
    mut remove: impl FnMut(usize),
) -> Option<usize> {
    let idx = entries
        .iter()
        .position(|&(lo, hi)| lo == wanted && hi == 0)?;
    if current != idx as i32 {
        remove(idx);
        Some(idx)
    } else {
        None
    }
}

/// `0x00897770`: forwards the container's inner field to the count query.
pub fn w31a13_count_of(inner: u32, count: impl Fn(u32) -> u8) -> u8 {
    count(inner)
}

/// `0x008a01c0`: walks a singly linked list; returns the node preceding the
/// first one satisfying `pred`, the last node when none does, and `None`
/// (the list head) for an empty list or a match on the first node.
pub fn w31a13_find_prev(len: usize, pred: impl Fn(usize) -> bool) -> Option<usize> {
    for i in 0..len {
        if pred(i) {
            return i.checked_sub(1);
        }
    }
    len.checked_sub(1)
}

#[cfg(test)]
mod tests_wave31_agent13 {
    use super::*;

    #[test]
    fn small_helpers() {
        let mut got = vec![];
        assert!(w31a13_resolve_and_notify(3, |h| h as i32, |r| got.push(r)));
        assert!(!w31a13_resolve_and_notify(3, |_| 0, |r| got.push(r)));
        assert_eq!(got, vec![3]);
        let mut v = 0;
        w31a13_forward_global(9, |g| v = g);
        assert_eq!(v, 9);
        let mut log = vec![];
        let mut l2 = vec![];
        w31a13_forward_then_finish(5, |g| log.push(g), || l2.push(1));
        assert_eq!((log, l2), (vec![5], vec![1]));
        assert_eq!(w31a13_count_of(4, |x| x as u8 + 1), 5);
    }

    #[test]
    fn slot_release() {
        let (mut a, mut b) = (7, 0x1_0005);
        let mut r = 0;
        w31a13_release_handle_slot(&mut a, &mut b, |h| r = h);
        assert_eq!((a, b, r), (0, 0xffff_ffff, 5));
    }

    #[test]
    fn callback() {
        assert!(!w31a13_invoke_callback(None, 1, [0; 4]));
        let f =
            |c: u32, a: u32, b: u32, d: u32, e: u32| assert_eq!((c, a, b, d, e), (1, 2, 3, 4, 5));
        assert!(w31a13_invoke_callback(Some(&f), 1, [2, 3, 4, 5]));
    }

    #[test]
    fn collect_and_remove() {
        let t = |h: u32| if h % 2 == 0 { 0xf } else { 1 };
        assert_eq!(w31a13_collect_type_f(&[1, 2, 4, 5], t), vec![2, 4]);
        let mut removed = vec![];
        let e = [(1, 0), (2, 1), (2, 0)];
        assert_eq!(
            w31a13_remove_matching(2, &e, 0, |i| removed.push(i)),
            Some(2)
        );
        assert_eq!(w31a13_remove_matching(2, &e, 2, |i| removed.push(i)), None);
        assert_eq!(w31a13_remove_matching(9, &e, 0, |i| removed.push(i)), None);
        assert_eq!(removed, vec![2]);
    }

    #[test]
    fn find_prev() {
        assert_eq!(w31a13_find_prev(0, |_| true), None);
        assert_eq!(w31a13_find_prev(3, |i| i == 0), None);
        assert_eq!(w31a13_find_prev(3, |i| i == 2), Some(1));
        assert_eq!(w31a13_find_prev(3, |_| false), Some(2));
    }
}
