#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent07.rs
// ===========================================================================
// Wave 25 translation batch 07.

/// `0x008c6700`: rectangle built from a `(x, y, w, h)` i16 rect and an
/// `(a, b)` i16 pair (after the opaque callee adjusted them): returns
/// `[x - px + a, y - py + b, w + a + 1, h + b + 1]`.
pub fn w25a07_rect_offset(px: i32, py: i32, rect: [i16; 4], pair: [i16; 2]) -> [i32; 4] {
    [
        (rect[0] as i32 - px) + pair[0] as i32,
        (rect[1] as i32 - py) + pair[1] as i32,
        rect[2] as i32 + pair[0] as i32 + 1,
        rect[3] as i32 + pair[1] as i32 + 1,
    ]
}

/// Shared gate of `0x008c6a90` and `0x008c6b20`: when the flag at `+0x1e0`
/// is zero the local id must equal the current id; then the action is
/// allowed only if neither blocker is set and the enabling check holds.
pub fn w25a07_gate(
    flag_1e0: bool,
    local_id: i32,
    current_id: i32,
    blocked: bool,
    enabled: bool,
    extra_ok: bool,
) -> bool {
    if !flag_1e0 && local_id != current_id {
        return false;
    }
    !blocked && enabled && extra_ok
}

/// `0x008c6a90`: gated dispatch; returns the result of `act` when the gate
/// (checker `FUN_008c6bb0` as `extra_ok`) passes.
pub fn w25a07_gated_a<T>(
    flag_1e0: bool,
    local_id: i32,
    current_id: i32,
    blocked: bool,
    enabled: bool,
    extra_ok: bool,
    act: impl FnOnce() -> T,
) -> Option<T> {
    w25a07_gate(flag_1e0, local_id, current_id, blocked, enabled, extra_ok).then(act)
}

/// `0x008c6b20`: same gate as `0x008c6a90` with the other checker/action.
pub fn w25a07_gated_b<T>(
    flag_1e0: bool,
    local_id: i32,
    current_id: i32,
    blocked: bool,
    enabled: bool,
    extra_ok: bool,
    act: impl FnOnce() -> T,
) -> Option<T> {
    w25a07_gate(flag_1e0, local_id, current_id, blocked, enabled, extra_ok).then(act)
}

/// `0x009010f0`: builds the `(param_4, param_5, flags)` request block;
/// returns the packed flags dword.
pub fn w25a07_request_flags(param_4: u32, param_5: i32) -> u32 {
    let mut lo: u16 = if param_5 >= 0 { 0x196 } else { 0x116 };
    let mut packed = (lo as u32) << 16;
    if param_5 == 0 {
        lo |= 0x40;
        packed = (((lo & !0x40) as u32) << 16) | 0x400000;
    }
    if param_4 & 0x8000_0000 == 0 && param_5 == 0 {
        packed = ((lo as u32) << 16) | 0x200000;
    }
    packed
}

/// `0x0090cdd0`: returns true when `id` is below `floor`; otherwise, if the
/// record id matches and the lookup succeeds, runs `apply` and returns its
/// result.
pub fn w25a07_check_id(
    floor: u32,
    id: u32,
    rec_id: u32,
    lookup: impl FnOnce() -> bool,
    apply: impl FnOnce() -> bool,
) -> bool {
    if id < floor {
        return true;
    }
    if rec_id == id && lookup() {
        return apply();
    }
    false
}

#[cfg(test)]
mod tests_wave25_agent07 {
    use super::*;
    #[test]
    fn rect() {
        assert_eq!(
            w25a07_rect_offset(10, 20, [15, 30, 4, 5], [1, 2]),
            [6, 12, 6, 8]
        );
    }
    #[test]
    fn gates() {
        assert!(w25a07_gate(true, 1, 2, false, true, true));
        assert!(!w25a07_gate(false, 1, 2, false, true, true));
        assert!(!w25a07_gate(true, 1, 1, true, true, true));
        assert_eq!(
            w25a07_gated_a(false, 3, 3, false, true, true, || 7),
            Some(7)
        );
        assert_eq!(w25a07_gated_b(false, 3, 3, false, false, true, || 7), None);
    }
    #[test]
    fn flags() {
        assert_eq!(w25a07_request_flags(0, -1), 0x116 << 16);
        assert_eq!(w25a07_request_flags(0, 1), 0x196 << 16);
        assert_eq!(
            w25a07_request_flags(0, 0),
            ((0x196 | 0x40) << 16) | 0x200000
        );
        assert_eq!(
            w25a07_request_flags(0x8000_0000, 0),
            (0x196 << 16) | 0x400000
        );
    }
    #[test]
    fn check_id() {
        assert!(w25a07_check_id(5, 4, 0, || false, || false));
        assert!(w25a07_check_id(5, 5, 5, || true, || true));
        assert!(!w25a07_check_id(5, 5, 6, || true, || true));
        assert!(!w25a07_check_id(5, 5, 5, || false, || true));
    }
}
