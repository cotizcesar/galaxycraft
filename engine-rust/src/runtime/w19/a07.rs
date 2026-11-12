#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave19/agent07.rs
// ===========================================================================
// Wave 19 translation batch 07: scalar-deleting-destructor glue and tiny helpers.
//
// Unresolved callees are closures.  The `flag & 1` pattern is the MSVC
// "scalar deleting destructor" delete bit.

fn w19a07_delete_bit(flags: u8) -> bool {
    flags & 1 != 0
}

/// `0x0054b320`: deleting-destructor wrapper with no member teardown; calls
/// `free` only when bit 0 of `flags` is set.  Returns whether it freed.
pub fn w19a07_dtor_free_only(flags: u8, mut free: impl FnMut()) -> bool {
    let d = w19a07_delete_bit(flags);
    if d {
        free();
    }
    d
}

/// `0x0054b340`: runs one base teardown, then frees when bit 0 is set.
pub fn w19a07_dtor_one_base_a(
    flags: u8,
    mut base_dtor: impl FnMut(),
    mut free: impl FnMut(),
) -> bool {
    base_dtor();
    w19a07_dtor_free_only(flags, &mut free)
}

/// `0x0054b370`: runs one base teardown, then frees when bit 0 is set.
pub fn w19a07_dtor_one_base_b(
    flags: u8,
    mut base_dtor: impl FnMut(),
    mut free: impl FnMut(),
) -> bool {
    base_dtor();
    w19a07_dtor_free_only(flags, &mut free)
}

/// `0x0054b3a3`: runs one base teardown, then frees when bit 0 is set (the
/// flag byte sits at `[ebp+8]` of the parent frame).
pub fn w19a07_dtor_one_base_c(
    flags: u8,
    mut base_dtor: impl FnMut(),
    mut free: impl FnMut(),
) -> bool {
    base_dtor();
    w19a07_dtor_free_only(flags, &mut free)
}

/// `0x0054b420`: runs one base teardown, then frees when bit 0 is set.
pub fn w19a07_dtor_one_base_d(
    flags: u8,
    mut base_dtor: impl FnMut(),
    mut free: impl FnMut(),
) -> bool {
    base_dtor();
    w19a07_dtor_free_only(flags, &mut free)
}

/// `0x0054a030`: member teardown.  Frees the buffer at `this+0x34` when
/// non-null (`buffer`), then runs four member destructors in order.
pub fn w19a07_member_teardown(
    buffer: u32,
    mut free_buffer: impl FnMut(u32),
    steps: &mut [&mut dyn FnMut()],
) {
    if buffer != 0 {
        free_buffer(buffer);
    }
    for s in steps.iter_mut() {
        s();
    }
}

/// `0x0054b3d0`: same member teardown as the routine at 0x0054a030, then the
/// delete bit of `flags` frees the object.  Returns whether it freed.
pub fn w19a07_member_teardown_deleting(
    flags: u8,
    buffer: u32,
    free_buffer: impl FnMut(u32),
    steps: &mut [&mut dyn FnMut()],
    mut free: impl FnMut(),
) -> bool {
    w19a07_member_teardown(buffer, free_buffer, steps);
    w19a07_dtor_free_only(flags, &mut free)
}

/// `0x0054a070`: length of a NUL-terminated string (bytes before the first
/// NUL, or the whole slice when none); the original forwards it onward.
pub fn w19a07_cstr_len(s: &[u8]) -> usize {
    s.iter().position(|&b| b == 0).unwrap_or(s.len())
}

/// `0x0054b530`: forwarder that calls the shared worker with two leading
/// zero arguments.
pub fn w19a07_forward_zero_zero<R>(
    a: u32,
    b: u32,
    worker: impl FnOnce(u32, u32, u32, u32) -> R,
) -> R {
    worker(0, 0, a, b)
}

#[cfg(test)]
mod tests_wave19_agent07 {
    use super::*;

    #[test]
    fn delete_bit_only() {
        let mut n = 0;
        assert!(w19a07_dtor_free_only(1, || n += 1));
        assert!(!w19a07_dtor_free_only(2, || n += 1));
        assert!(w19a07_dtor_free_only(3, || n += 1));
        assert_eq!(n, 2);
    }

    #[test]
    fn one_base_variants() {
        let mut log = Vec::new();
        let r = std::cell::RefCell::new(&mut log);
        assert!(w19a07_dtor_one_base_a(
            1,
            || r.borrow_mut().push('b'),
            || r.borrow_mut().push('f')
        ));
        assert!(!w19a07_dtor_one_base_b(
            0,
            || r.borrow_mut().push('b'),
            || r.borrow_mut().push('f')
        ));
        assert!(w19a07_dtor_one_base_c(
            1,
            || r.borrow_mut().push('b'),
            || r.borrow_mut().push('f')
        ));
        assert!(!w19a07_dtor_one_base_d(
            0,
            || r.borrow_mut().push('b'),
            || r.borrow_mut().push('f')
        ));
        let _ = r.into_inner();
        assert_eq!(log, vec!['b', 'f', 'b', 'b', 'f', 'b']);
    }

    #[test]
    fn member_teardown_order() {
        let log = std::cell::RefCell::new(Vec::new());
        let mut a = || log.borrow_mut().push(1);
        let mut b = || log.borrow_mut().push(2);
        w19a07_member_teardown(0, |_| log.borrow_mut().push(99), &mut [&mut a, &mut b]);
        assert_eq!(*log.borrow(), vec![1, 2]);
        log.borrow_mut().clear();
        let mut a = || log.borrow_mut().push(1);
        let freed = w19a07_member_teardown_deleting(
            1,
            0x10,
            |p| log.borrow_mut().push(p as i32),
            &mut [&mut a],
            || log.borrow_mut().push(-1),
        );
        assert!(freed);
        assert_eq!(*log.borrow(), vec![0x10, 1, -1]);
    }

    #[test]
    fn strlen_and_forward() {
        assert_eq!(w19a07_cstr_len(b"abc\0def"), 3);
        assert_eq!(w19a07_cstr_len(b"\0"), 0);
        assert_eq!(w19a07_cstr_len(b"xy"), 2);
        assert_eq!(
            w19a07_forward_zero_zero(5, 6, |a, b, c, d| [a, b, c, d]),
            [0, 0, 5, 6]
        );
    }
}
