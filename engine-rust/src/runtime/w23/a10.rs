#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent10.rs
// ===========================================================================
// Wave 23 translation batch 10: dialog init-handler decisions and small
// list helpers from the runtime module window.

/// Action chosen by a dialog event handler for the common "bitmap backed
/// dialog" pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W23a10DialogAction {
    /// Release the cached bitmap (if loaded), clear the dialog image pointer.
    ReleaseBitmap { free_loaded: bool },
    /// Load the bitmap if not yet loaded, set image pointer, then init.
    InitBitmap { load_needed: bool },
    /// Fall through to the default dialog handler.
    Default,
    /// Handler consumed the event; no default handler.
    Consumed,
}

/// `0x006f5d60`: pInset dialog handler decision.  `event_type` is the
/// 16-bit value at event+0x10, `code` is the first event dword.
/// `child_ptr` / `child_field_14` model `*(dlg+0x40)` and `*(*(dlg+0x40)+0x14)`.
pub fn w23a10_inset_dialog_action(
    event_type: i16,
    code: i32,
    bitmap_loaded: bool,
    child_ptr: u32,
    child_field_14: u32,
) -> W23a10DialogAction {
    if event_type == 0xe {
        match code {
            1 => {
                return W23a10DialogAction::ReleaseBitmap {
                    free_loaded: bitmap_loaded,
                };
            }
            10 => {
                return W23a10DialogAction::InitBitmap {
                    load_needed: !bitmap_loaded,
                };
            }
            0xd if child_ptr == 0 || child_field_14 == 0 => {
                return W23a10DialogAction::Consumed;
            }
            _ => {}
        }
    }
    W23a10DialogAction::Default
}

/// `0x006f8350`: scorebox dialog handler decision.  Same shape as the inset
/// handler but code 0xd is not special and the default handler always runs
/// after the init/release branches (`follow_default` is always true).
pub fn w23a10_scorebox_dialog_action(
    event_type: i16,
    code: i32,
    bitmap_loaded: bool,
) -> (W23a10DialogAction, bool) {
    if event_type == 0xe {
        match code {
            1 => {
                return (
                    W23a10DialogAction::ReleaseBitmap {
                        free_loaded: bitmap_loaded,
                    },
                    true,
                );
            }
            10 => {
                return (
                    W23a10DialogAction::InitBitmap {
                        load_needed: !bitmap_loaded,
                    },
                    true,
                );
            }
            _ => {}
        }
    }
    (W23a10DialogAction::Default, true)
}

/// Outcome of the untab dialog handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W23a10UntabAction {
    Release {
        free_loaded: bool,
    },
    /// `handler_is_alt` selects `UNK_006f5f50` (dialog+0x3a == 6) over
    /// `UNK_006f8a90` for the dialog+0x48 callback.
    Init {
        load_needed: bool,
        handler_is_alt: bool,
    },
    /// Calls `func_0x6f_6c60(dialog)` and skips the default handler.
    CloseOnly,
    /// Calls `func_0x6f_6c60(dialog)` and then the default handler.
    CloseThenDefault,
    Default,
}

/// `0x006f8b60`: untab dialog handler decision.  `dialog_kind` is the u16 at
/// dialog+0x3a; `is_dialog_slot3` tells whether the dialog equals child 3 of
/// itself (case 0xd); `param1` is the second event dword (case 0xb).
pub fn w23a10_untab_dialog_action(
    event_type: i16,
    code: i32,
    param1: i32,
    bitmap_loaded: bool,
    dialog_kind: u16,
    is_dialog_slot3: bool,
) -> W23a10UntabAction {
    if event_type != 0xe {
        return W23a10UntabAction::Default;
    }
    match code {
        1 => W23a10UntabAction::Release {
            free_loaded: bitmap_loaded,
        },
        2 => W23a10UntabAction::CloseOnly,
        10 => W23a10UntabAction::Init {
            load_needed: !bitmap_loaded,
            handler_is_alt: dialog_kind == 6,
        },
        0xb => {
            if param1 == 0 {
                W23a10UntabAction::Default
            } else {
                W23a10UntabAction::CloseOnly
            }
        }
        0xd if is_dialog_slot3 => W23a10UntabAction::CloseThenDefault,
        _ => W23a10UntabAction::Default,
    }
}

/// `0x006f6b90`: stores `value` in the global at 011_79c60; returns the new
/// global value.
pub fn w23a10_set_global(slot: &mut u32, value: u32) -> u32 {
    *slot = value;
    *slot
}

/// `0x006f7100`: when the player-slot count is not positive, the routine
/// walks 8 rows of 6 control ids starting at 0xd (ids base+0..base+5, base =
/// 0xd + row*6) and hides each.  Returns the ids visited in order.
pub fn w23a10_row_control_ids(slot_count: i32) -> Vec<i32> {
    if slot_count > 0 {
        return Vec::new();
    }
    let mut ids = Vec::new();
    for row in 0..8 {
        let base = row * 6;
        ids.push(base + 0xd);
        ids.push(base + 0xe);
        for k in 0..3 {
            ids.push(k + 0xf + base);
        }
        ids.push(base + 0x12);
    }
    ids
}

/// `0x006f8e50`: target slot computation when unlinking a 12-byte list
/// node.  `node_addr` is the node address, `link` its second dword and
/// `owner_prev_off` the dword at `*(first) + 4`.  Returns the address of the
/// slot that receives `first` (even non-zero link means "relative to the
/// node", otherwise the link with bit0 cleared is the address).
pub fn w23a10_unlink_slot(node_addr: u32, link: u32, owner_prev_off: u32) -> u32 {
    if link & 1 == 0 && link != 0 {
        node_addr.wrapping_add(link.wrapping_sub(owner_prev_off))
    } else {
        link & 0xffff_fffe
    }
}

/// `0x006f8520`: cursor advance for the ring in 011_78340..011_78344.
/// Returns `(new_cursor, quotient)` where quotient is `new_cursor / stride`
/// (None if stride is zero).  If the span is zero the cursor is unchanged.
pub fn w23a10_ring_advance(cursor: i32, end: i32, stride: i32) -> (i32, Option<i32>) {
    let mut new_cursor = cursor;
    let span = end.wrapping_sub(cursor);
    if span != 0 {
        let mut step = span.wrapping_add((span >> 31) & 7) >> 3;
        if step < 4 {
            step = 4;
        }
        let target = step.wrapping_add(cursor);
        new_cursor = end;
        if target <= end {
            new_cursor = target;
        }
    }
    let q = if stride == 0 {
        None
    } else {
        Some(new_cursor.wrapping_div(stride))
    };
    (new_cursor, q)
}

#[cfg(test)]
mod tests_wave23_agent10 {
    use super::*;

    #[test]
    fn inset_and_scorebox() {
        use W23a10DialogAction as A;
        assert_eq!(
            w23a10_inset_dialog_action(0xe, 1, true, 1, 1),
            A::ReleaseBitmap { free_loaded: true }
        );
        assert_eq!(
            w23a10_inset_dialog_action(0xe, 10, false, 1, 1),
            A::InitBitmap { load_needed: true }
        );
        assert_eq!(
            w23a10_inset_dialog_action(0xe, 0xd, false, 0, 0),
            A::Consumed
        );
        assert_eq!(
            w23a10_inset_dialog_action(0xe, 0xd, false, 1, 5),
            A::Default
        );
        assert_eq!(w23a10_inset_dialog_action(3, 1, true, 0, 0), A::Default);
        assert_eq!(
            w23a10_scorebox_dialog_action(0xe, 10, true),
            (A::InitBitmap { load_needed: false }, true)
        );
        assert_eq!(
            w23a10_scorebox_dialog_action(0xe, 0xd, true),
            (A::Default, true)
        );
    }

    #[test]
    fn untab() {
        use W23a10UntabAction as U;
        assert_eq!(
            w23a10_untab_dialog_action(1, 1, 0, true, 0, false),
            U::Default
        );
        assert_eq!(
            w23a10_untab_dialog_action(0xe, 2, 0, true, 0, false),
            U::CloseOnly
        );
        assert_eq!(
            w23a10_untab_dialog_action(0xe, 0xb, 0, true, 0, false),
            U::Default
        );
        assert_eq!(
            w23a10_untab_dialog_action(0xe, 0xb, 1, true, 0, false),
            U::CloseOnly
        );
        assert_eq!(
            w23a10_untab_dialog_action(0xe, 10, 0, false, 6, false),
            U::Init {
                load_needed: true,
                handler_is_alt: true
            }
        );
        assert_eq!(
            w23a10_untab_dialog_action(0xe, 0xd, 0, true, 0, true),
            U::CloseThenDefault
        );
        assert_eq!(
            w23a10_untab_dialog_action(0xe, 0xd, 0, true, 0, false),
            U::Default
        );
    }

    #[test]
    fn small_helpers() {
        let mut g = 0;
        assert_eq!(w23a10_set_global(&mut g, 7), 7);
        assert!(w23a10_row_control_ids(1).is_empty());
        let ids = w23a10_row_control_ids(0);
        assert_eq!(ids.len(), 48);
        assert_eq!(ids[0], 0xd);
        assert_eq!(ids[5], 0x12);
        assert_eq!(ids[6], 0x13);
        assert_eq!(*ids.last().unwrap(), 0x12 + 42);
        assert_eq!(w23a10_unlink_slot(100, 0x1001, 4), 0x1000);
        assert_eq!(w23a10_unlink_slot(100, 0, 4), 0);
        assert_eq!(w23a10_unlink_slot(100, 20, 4), 116);
    }

    #[test]
    fn ring() {
        assert_eq!(w23a10_ring_advance(5, 5, 2), (5, Some(2)));
        assert_eq!(w23a10_ring_advance(0, 80, 4), (10, Some(2)));
        assert_eq!(w23a10_ring_advance(0, 8, 0), (4, None));
        assert_eq!(w23a10_ring_advance(0, 2, 1), (2, Some(2)));
    }
}
