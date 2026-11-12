#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent75.rs
// ===========================================================================
// Wave 32 agent 75: small D3D wrapper glue modelled with caller closures.

/// `0x009d0bc0`: adjusts the interface pointer by `-0x1c` (null stays null),
/// asks `lookup(base + 0xe4)` for a handle and stores it at `base + 0x204`.
/// A zero handle yields error `0x17`; otherwise the result of `follow_up`.
pub fn w32a75_resolve_interface(
    param: u32,
    lookup: &mut dyn FnMut(u32) -> u32,
    store: &mut dyn FnMut(u32, u32),
    follow_up: &mut dyn FnMut() -> u32,
) -> u32 {
    let base = if param != 0 {
        param.wrapping_sub(0x1c)
    } else {
        0
    };
    let handle = lookup(base.wrapping_add(0xe4));
    store(base.wrapping_add(0x204), handle);
    if handle == 0 {
        return 0x17;
    }
    follow_up()
}

/// `0x009d7640`: runs the base step; if it succeeds and the object's field at
/// `+0x18c` is non-null, runs the release step then the bind step on it.
/// The first non-zero code is returned, else 0.
pub fn w32a75_release_and_bind(
    field_18c: u32,
    base_step: &mut dyn FnMut() -> i32,
    release_step: &mut dyn FnMut() -> i32,
    bind_step: &mut dyn FnMut(u32) -> i32,
) -> i32 {
    let r = base_step();
    if r != 0 {
        return r;
    }
    if field_18c != 0 {
        let r = release_step();
        if r != 0 {
            return r;
        }
        let r = bind_step(field_18c);
        if r != 0 {
            return r;
        }
    }
    0
}

#[cfg(test)]
mod tests_wave32_agent75 {
    use super::*;

    #[test]
    fn resolve_interface_paths() {
        let mut stored = Vec::new();
        let r = w32a75_resolve_interface(
            0x100,
            &mut |a| {
                assert_eq!(a, 0x100 - 0x1c + 0xe4);
                0
            },
            &mut |a, v| stored.push((a, v)),
            &mut || 7,
        );
        assert_eq!(r, 0x17);
        assert_eq!(stored, vec![(0x100 - 0x1c + 0x204, 0)]);
        let r = w32a75_resolve_interface(
            0,
            &mut |a| {
                assert_eq!(a, 0xe4);
                5
            },
            &mut |_, _| {},
            &mut || 7,
        );
        assert_eq!(r, 7);
    }

    #[test]
    fn release_and_bind_paths() {
        assert_eq!(
            w32a75_release_and_bind(1, &mut || 3, &mut || 0, &mut |_| 0),
            3
        );
        assert_eq!(
            w32a75_release_and_bind(0, &mut || 0, &mut || 9, &mut |_| 9),
            0
        );
        assert_eq!(
            w32a75_release_and_bind(1, &mut || 0, &mut || 4, &mut |_| 9),
            4
        );
        assert_eq!(
            w32a75_release_and_bind(2, &mut || 0, &mut || 0, &mut |p| p as i32 + 1),
            3
        );
    }
}
