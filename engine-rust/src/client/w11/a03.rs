#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave11/agent03.rs
// ===========================================================================
// Wave 11 translation batch 03; temporary staging file.
// Merged into the natural module after the wave.
//
// Batch 03 covers the `decompiled_uncited` rows in
// `0x0064c000..=0x0064dfff`.  The window holds the client's packed
// string-blob view (the header accessors of the `StringTable` class that
// [`crate::tables::StringTable`] models), the pause/quit-menu mode machine
// that keys off the recovered screen byte at `+0xe61c`, one control-rect
// computation and a long run of C++ exception-frame helpers.  Only the rows
// with an observable computation are translated here; the rest are listed
// at the bottom with the reason they were skipped.

// ---------------------------------------------------------------------------
// Packed string-blob view (`StringTable` header)
// ---------------------------------------------------------------------------
//
// The view is three words: the mapped bytes, the recorded byte length and a
// `wide` flag.  Narrow blobs store a `u16` count and a table of `u16` byte
// offsets; wide blobs store a `u32` count and a table of `u32` offsets.  This
// is the layout [`crate::tables::StringTable::parse`] reads for
// `rez/stat_txt.tbl`, with the wide variant the recovered accessors also
// handle.

/// Narrow-table count-field width in bytes.  `0x0064dc10`'s recovered
/// `2 + 2 * index` uses the same two bytes as the start of the offset table.
pub const W11A03_NARROW_COUNT_WIDTH: u32 = 2;

/// Wide-table count-field width in bytes (a `u32` count and `u32` offsets).
pub const W11A03_WIDE_COUNT_WIDTH: u32 = 4;

/// The recovered `StringTable` blob view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W11a03StringBlob<'a> {
    /// `+0x00`: mapped table bytes; `None` mirrors the null pointer the
    /// recovered code tests before every read.
    pub data: Option<&'a [u8]>,
    /// `+0x04`: recorded byte length, used as the authoritative bound.
    pub size: u32,
    /// `+0x08`: byte flag; non-zero means 32-bit count and offsets.
    pub wide: bool,
}

/// `0x0064dca0` (`FUN_0064dca0`, 24 bytes): initialise the three-word blob
/// view from its pointer, length and width flag.
///
/// The recovered body stores `param_1` at `+0x00`, `param_2` at `+0x04` and
/// `param_3` at `+0x08` and nothing else; the Rust constructor takes the
/// mapped bytes in place of the raw pointer.
pub fn w11a03_0064dca0_bind_string_blob<'a>(
    data: Option<&'a [u8]>,
    size: u32,
    wide: bool,
) -> W11a03StringBlob<'a> {
    W11a03StringBlob { data, size, wide }
}

/// `0x0064d870` (`FUN_0064d870`, 18 bytes): reset the view to "unbound".
///
/// The body writes zero to `+0x00`, `+0x04` and `+0x08`, i.e. the null
/// pointer, a zero length and the narrow flag.  It is the three-field clear
/// the container uses before a blob is attached.
pub fn w11a03_0064d870_reset_string_blob(blob: &mut W11a03StringBlob<'_>) {
    blob.data = None;
    blob.size = 0;
    blob.wide = false;
}

/// `0x0064dc80` (`FUN_0064dc80`, 7 bytes): entry count of a packed blob.
///
/// The body returns `0` for the null pointer, the little-endian `u32` at
/// `+0x00` when the width flag is set and the little-endian `u16` at `+0x00`
/// otherwise.  The shipped routine does not check that the count field is
/// mapped; the port returns `0` when fewer than 2 (narrow) or 4 (wide) bytes
/// are recorded so the read stays inside the bound the view carries.
pub fn w11a03_0064dc80_string_count(blob: &W11a03StringBlob<'_>) -> u32 {
    let Some(data) = blob.data else {
        return 0;
    };
    let width = if blob.wide {
        W11A03_WIDE_COUNT_WIDTH
    } else {
        W11A03_NARROW_COUNT_WIDTH
    } as usize;
    if (blob.size as usize) < width || data.len() < width {
        return 0;
    }
    if blob.wide {
        u32::from_le_bytes([data[0], data[1], data[2], data[3]])
    } else {
        u16::from_le_bytes([data[0], data[1]]) as u32
    }
}

/// `0x0064dc10` (`FUN_0064dc10`, 16 bytes): byte position of the narrow
/// offset field for `index`, `2 + 2 * index`.
///
/// The recovered body is `(uint)param_1 * 2 + 2` on a byte argument, which is
/// both the position of the `u16` offset entry and the header size of a
/// narrow table holding `index` entries.
pub fn w11a03_0064dc10_narrow_offset_field(index: u32) -> u32 {
    index.wrapping_mul(2).wrapping_add(2)
}

/// `0x0064dc20` (`FUN_0064dc20`, 94 bytes): bytes of entry `index`, or
/// `None` when the entry is absent.
///
/// The recovered flow is:
///
/// * `(*view == 0) -> 0` (unbound view);
/// * `index >= count -> 0`;
/// * narrow: require `size >= 2 * index + 4`, then read the `u16` offset at
///   `2 * index + 2`; wide: require `size >= 4 * index + 8`, then read the
///   `u32` offset at `4 * (index + 1)`;
/// * `offset < size -> base + offset`, otherwise `0`.
///
/// The returned slice runs from the offset to the end of the recorded blob
/// (the callers treat the entry as a NUL-terminated string).  The bound test
/// is reproduced with the recorded `size`; the extra slice-length check only
/// keeps a view whose `size` exceeds the mapped bytes from reading out of
/// bounds.
pub fn w11a03_0064dc20_string_bytes<'a>(
    blob: &W11a03StringBlob<'a>,
    index: u32,
) -> Option<&'a [u8]> {
    let data = blob.data?;
    if index >= w11a03_0064dc80_string_count(blob) {
        return None;
    }

    let size = blob.size as usize;
    let (field, width) = if blob.wide {
        if size < (index as usize).checked_mul(4)?.checked_add(8)? {
            return None;
        }
        ((index as usize) * 4 + 4, 4)
    } else {
        if size < (index as usize).checked_mul(2)?.checked_add(4)? {
            return None;
        }
        ((index as usize) * 2 + 2, 2)
    };

    let raw = data.get(field..field + width)?;
    let offset = if width == 4 {
        u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize
    } else {
        u16::from_le_bytes([raw[0], raw[1]]) as usize
    };

    let end = size.min(data.len());
    if offset >= end {
        return None;
    }
    Some(&data[offset..end])
}

// ---------------------------------------------------------------------------
// Pause / quit-menu mode machine
// ---------------------------------------------------------------------------
//
// Three globals drive the recovered dispatcher: the screen byte at `+0xe61c`
// (0 in game, 0x28 for the modal branch, any other menu screen otherwise) and
// the two menu words `DAT_00fcd6b4` (0 closed, 1 list, 2 modal) and
// `DAT_00fcd6b8` (set by the close path that raises error 14).  The manager's
// widget registry is not part of this crate, so the two entry points below
// report the branch they take instead of calling into it.

/// Recovered menu words `DAT_00fcd6b4` / `DAT_00fcd6b8`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W11a03DialogMode {
    /// `DAT_00fcd6b4`: 0 closed, 1 list screen, 2 modal screen.
    pub mode: u8,
    /// `DAT_00fcd6b8`: latched by the close path that raises error 14.
    pub closed: bool,
}

/// Branch taken by [`w11a03_0064d5d0_dispatch`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W11a03MenuAction {
    /// State byte 0 with no open menu: nothing happens.
    Idle,
    /// State byte 0 with an open menu: the mode word is cleared and the
    /// release callback is registered.
    Closed,
    /// Any menu screen except the modal one: enter the list screen.
    OpenList,
    /// Screen byte `0x28`: enter the modal screen.
    OpenModal,
}

/// `0x0064d5d0` (`FUN_0064d5d0`, 86 bytes): pause/quit-menu dispatch on the
/// current screen byte.
///
/// * screen `0`: when the mode word is non-zero it is cleared and the
///   release callback is registered, then the routine returns;
/// * screen `0x28`: the modal opener runs;
/// * every other screen: the list opener runs (which sets mode 1).
///
/// The two openers own the widget registry and the mode-word writes, so the
/// dispatch returns which one the original tail-calls into.
pub fn w11a03_0064d5d0_dispatch(mode: &mut W11a03DialogMode, screen: u8) -> W11a03MenuAction {
    if screen == 0 {
        if mode.mode != 0 {
            mode.mode = 0;
            return W11a03MenuAction::Closed;
        }
        return W11a03MenuAction::Idle;
    }
    if screen == 0x28 {
        W11a03MenuAction::OpenModal
    } else {
        W11a03MenuAction::OpenList
    }
}

/// Effect of a list selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W11a03SelectionAction {
    /// The view is unbound, or the selection changed nothing.
    Ignored,
    /// Row 0 re-ran the state-0 dispatch, which closed the menu.
    Closed,
    /// The open mode word was cleared and the release callback registered.
    Released,
}

/// `0x0064cef0` (`FUN_0064cef0`, 66 bytes): list-selection change handler.
///
/// The recovered body returns immediately when the list view (`DAT_00fcd6b0`)
/// is null.  Otherwise:
///
/// * selection `0` re-runs the menu dispatch with screen byte `0` (the close
///   path of `0x0064d5d0`);
/// * any other selection with a non-zero mode word clears the mode word and
///   registers the release callback.
pub fn w11a03_0064cef0_selection_changed(
    mode: &mut W11a03DialogMode,
    list_present: bool,
    index: u32,
) -> W11a03SelectionAction {
    if !list_present {
        return W11a03SelectionAction::Ignored;
    }
    if index == 0 {
        return match w11a03_0064d5d0_dispatch(mode, 0) {
            W11a03MenuAction::Closed => W11a03SelectionAction::Closed,
            _ => W11a03SelectionAction::Ignored,
        };
    }
    if mode.mode != 0 {
        mode.mode = 0;
        W11a03SelectionAction::Released
    } else {
        W11a03SelectionAction::Ignored
    }
}

/// `0x0064cf60` (`FUN_0064cf60`, 144 bytes): close the menu and raise error
/// `0x0e`.
///
/// The body first runs the client's "can report" probe (five guard checks) and
/// either terminates or falls through; the recovered tail then clears the
/// mode word, latches the closed flag and reaches the non-returning throw
/// helper with `0x0e`.  `Err(0x0e)` is that throw; the two state writes are
/// kept because they happen before it.
pub fn w11a03_0064cf60_close_menu(mode: &mut W11a03DialogMode) -> Result<(), u32> {
    mode.mode = 0;
    mode.closed = true;
    Err(0x0e)
}

/// `0x0064ceb0` (`FUN_0064ceb0`, 24 bytes): release gate of the mode-0
/// callback.
///
/// The routine is the callback registered whenever the menu closes.  It tests
/// the control's `+0x3c` word and only calls the release helper when the word
/// is non-zero; returns whether the control is released.
pub fn w11a03_0064ceb0_release_gate(word_at_3c: u16) -> bool {
    word_at_3c != 0
}

// ---------------------------------------------------------------------------
// Control rectangle and slot release
// ---------------------------------------------------------------------------

/// Inclusive-left/exclusive-right rectangle produced by `0x0064c7a0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W11a03Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl W11a03Rect {
    /// Width in pixels; the right edge is exclusive.
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    /// Height in pixels; the bottom edge is exclusive.
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

/// `0x0064c7a0` (`FUN_0064c7a0`, 100 bytes): expand a control's cached edge
/// offsets by the draw origin.
///
/// The four signed words at `+0x04..=+0x0a` are the control's left, top, right
/// and bottom offsets.  The recovered body adds the origin's x to the left and
/// right offsets, the origin's y to the top and bottom offsets, and adds one
/// to the right and bottom results before handing the rectangle to the blit
/// helper (the sprite descriptor and `0x16` blit mode come from the caller).
pub fn w11a03_0064c7a0_control_rect(edges: [i16; 4], origin: (i16, i16)) -> W11a03Rect {
    let [left, top, right, bottom] = edges;
    let (x, y) = origin;
    W11a03Rect {
        left: left as i32 + x as i32,
        top: top as i32 + y as i32,
        right: right as i32 + x as i32 + 1,
        bottom: bottom as i32 + y as i32 + 1,
    }
}

/// `0x0064dff0` (`FUN_0064dff0`, 177 bytes): release every live slot of the
/// menu's control array.
///
/// When the array pointer at `+0x38` is null the body only clears the word at
/// `+0x3c`.  Otherwise it walks `0..count` (`count` at `+0x34`), releases the
/// object held by each non-null slot and stores zero in the slot, then clears
/// `+0x3c`.  The object release is the destructor call the decompiler cannot
/// express; the slot clear, the walk bound and the final `+0x3c` clear are
/// reproduced here.  The slice-length clamp is a Rust-side guard only.
pub fn w11a03_0064dff0_release_slots(slots: &mut [u32], count: u32, selected: &mut u32) {
    let end = (count as usize).min(slots.len());
    for slot in &mut slots[..end] {
        *slot = 0;
    }
    *selected = 0;
}

// ---------------------------------------------------------------------------
// Callback and error helpers
// ---------------------------------------------------------------------------

/// `0x0064c6e0` (`FUN_0064c6e0`, 27 bytes): invoke a stored `TCtrl`,
/// `unsigned short` callback.
///
/// The recovered object holds a function pointer at `+0x04`; the body loads
/// the control pointer from the first argument slot, the argument from the
/// second slot and forwards both values to it.  The Rust form takes the
/// callable directly.
pub fn w11a03_0064c6e0_invoke_control_callback<F>(callback: &mut F, ctrl: u32, arg: u16)
where
    F: FnMut(u32, u16),
{
    callback(ctrl, arg);
}

/// `0x0064d630` (`FUN_0064d630`, 113 bytes): throw the caller's message.
///
/// The body measures the NUL-terminated argument (the loop advances to the
/// terminator and subtracts the original pointer), builds a `std::string` of
/// that length in the caller-provided slot (`+0x04` = 0, `+0x00` = inline
/// buffer, `+0x08` = `0x8000_000f`) and reaches the non-returning throw
/// helper.  `panic!` carries the same message and never returns.
pub fn w11a03_0064d630_reject_with_message(message: &str) -> ! {
    panic!("{message}")
}

/// `0x0064d750` (`FUN_0064d750`, 26 bytes): forwarding wrapper around
/// [`w11a03_0064d630_reject_with_message`].
///
/// The body calls the message thrower with `*param_2` and would return the
/// out-slot `param_1`, but the thrower never returns, so neither does this
/// wrapper.
pub fn w11a03_0064d750_reject_and_forward(message: &str) -> ! {
    w11a03_0064d630_reject_with_message(message)
}

/// `0x0064d147` (`FUN_0064d147`, 2 bytes): empty body (`ret` only).
///
/// The row is one of the exception-frame placeholders in the window; calling
/// it has no effect.
pub fn w11a03_0064d147_noop() {}

// ---------------------------------------------------------------------------
// Rows not translated
// ---------------------------------------------------------------------------
//
// The remaining `decompiled_uncited` rows in `0x0064c000..=0x0064dfff` are
// compiler support for the client's C++ exception frames or depend on the
// widget registry, which this crate does not model:
//
// * `FUN_0064c010`, `FUN_0064c2d1`, `FUN_0064c588` - 7/10/8-byte trampolines into
//   the non-returning `FUN_00c89580`; no computation survives.
// * `FUN_0064c030`, `FUN_0064c0b0`, `FUN_0064c130`, `FUN_0064c190`, `FUN_0064c400`,
//   `FUN_0064c480`, `FUN_0064c2db`, `FUN_0064c500`, `FUN_0064c52e` - build and throw
//   a `std::string`, usually from a global message buffer.  The
//   buffer-filling helper's arguments were lost, so the only recoverable
//   behaviour is "throw the current global message"; `0x0064d630` covers the
//   same pattern where the message is a real argument.
// * `FUN_0064c210`, `FUN_0064c600`, `FUN_0064c640`, `FUN_0064c980`, `FUN_0064cbbb`,
//   `FUN_0064cde5`, `FUN_0064ce0f`, `FUN_0064ca64`, `FUN_0064cb83`, `FUN_0064d46a`,
//   `FUN_0064d494`, `FUN_0064df62`, `FUN_0064df7e`, `FUN_0064dfa6`, `FUN_0064dfa7`,
//   `FUN_0064dfce`, `FUN_0064ddc0` - SEH registration, funclet and frame-restore
//   bodies: they only move `ExceptionList` words or tail-call a rethrow.
// * `FUN_0064c9ce`, `FUN_0064cb30`, `FUN_0064ccb3`, `FUN_0064d090`, `FUN_0064d2bd`,
//   `FUN_0064dd6e` - partly misdecoded bodies (`in_EAX`/`in_ECX` garbage,
//   `halt_baddata`, `BADSPACEBASE`); the recovered words do not form a
//   reproducible computation.
// * `FUN_0064c700`, `FUN_0064d770` - copyable-function-object vtable writes; the
//   captures are already ordinary Rust closures (see
//   [`w11a03_0064c6e0_invoke_control_callback`]).
// * `FUN_0064c720`, `FUN_0064d780` - RTTI type descriptors.
// * `FUN_0064c730`, `FUN_0064cc20`, `FUN_0064d150`, `FUN_0064d360` - menu openers and
//   their "control not found" throws: control lookup by id (`-20`, `-28`,
//   `-10`, ...), release and event registration live in the client's widget
//   registry, which is not part of this crate.  The branch logic they smuggle
//   in is covered by [`w11a03_0064d5d0_dispatch`] and
//   [`w11a03_0064cef0_selection_changed`].
// * `FUN_0064c82d` - indirect jump through vtable slot `+0x1c`.
// * `FUN_0064c805`, `FUN_0064d225` - misdecoded prologues whose tail calls carry
//   arguments the decompiler dropped.
// * `FUN_0064ca19`, `FUN_0064d890` - text/list draw and container-building calls
//   whose stack slots were spilled into `unaff_EBP` words or routed through
//   opaque allocator/exception helpers; the element layout and argument order
//   cannot be recovered from the body.
// * `FUN_0064ce32`, `FUN_0064d4b4` - recursive list walks driven by the same
//   unrecovered `unaff_EBP` slots.
// * `FUN_0064dcc0`, `FUN_0064de00`, `FUN_0064ded0`, `FUN_0064df10` - `StringTable`
//   layout constructor, destructors, assignment and deleting destructor: pure
//   ownership plumbing over the two inline `std::string` slots, no computed
//   value.  The table layout itself is
//   [`crate::tables::StringTable::parse`].
// * `FUN_0064ddda` - two-byte infinite loop.

#[cfg(test)]
mod tests_wave11_agent03 {
    use super::*;

    /// A three-entry narrow table: `count = 3`, offsets 8/12/16, then
    /// `"one\0two\0three\0"`.
    const NARROW: [u8; 22] = *b"\
        \x03\x00\x08\x00\x0c\x00\x10\x00\
        one\x00two\x00three\x00";

    /// A two-entry wide table: `count = 2`, u32 offsets 12/15, then
    /// `"aa\0bb\0"`.
    const WIDE: [u8; 18] = *b"\
        \x02\x00\x00\x00\x0c\x00\x00\x00\x0f\x00\x00\x00\
        aa\x00bb\x00";

    #[test]
    fn bind_and_reset_cover_the_three_word_view() {
        let mut blob = w11a03_0064dca0_bind_string_blob(Some(&NARROW), 22, false);
        assert_eq!(blob.data, Some(&NARROW[..]));
        assert_eq!(blob.size, 22);
        assert!(!blob.wide);

        // The reset is the three-word clear; the count and lookup go away
        // with the pointer.
        w11a03_0064d870_reset_string_blob(&mut blob);
        assert_eq!(blob, W11a03StringBlob::default());
        assert_eq!(w11a03_0064dc80_string_count(&blob), 0);
        assert_eq!(w11a03_0064dc20_string_bytes(&blob, 0), None);

        // A null bind is the same unbound view.
        let null = w11a03_0064dca0_bind_string_blob(None, 0, false);
        assert_eq!(w11a03_0064dc80_string_count(&null), 0);
    }

    #[test]
    fn string_count_reads_the_narrow_and_wide_count_fields() {
        let narrow = w11a03_0064dca0_bind_string_blob(Some(&NARROW), 22, false);
        assert_eq!(w11a03_0064dc80_string_count(&narrow), 3);

        let wide = w11a03_0064dca0_bind_string_blob(Some(&WIDE), 18, true);
        assert_eq!(w11a03_0064dc80_string_count(&wide), 2);

        // A recorded length smaller than the count field reads as empty
        // rather than indexing the mapping.
        let stub = w11a03_0064dca0_bind_string_blob(Some(&NARROW[..1]), 1, false);
        assert_eq!(w11a03_0064dc80_string_count(&stub), 0);
    }

    #[test]
    fn narrow_offset_field_matches_the_header_size() {
        assert_eq!(W11A03_NARROW_COUNT_WIDTH, 2);
        assert_eq!(W11A03_WIDE_COUNT_WIDTH, 4);
        assert_eq!(w11a03_0064dc10_narrow_offset_field(0), 2);
        assert_eq!(w11a03_0064dc10_narrow_offset_field(3), 8);
        assert_eq!(w11a03_0064dc10_narrow_offset_field(255), 512);
        // The same expression is the byte position `string_bytes` reads for
        // a narrow entry.
        for index in 0..3u32 {
            let field = w11a03_0064dc10_narrow_offset_field(index) as usize;
            let value = u16::from_le_bytes([NARROW[field], NARROW[field + 1]]);
            assert_eq!(value as usize, 8 + 4 * index as usize);
        }
    }

    #[test]
    fn string_bytes_returns_the_narrow_entry_through_the_blob_end() {
        let blob = w11a03_0064dca0_bind_string_blob(Some(&NARROW), 22, false);
        assert_eq!(w11a03_0064dc20_string_bytes(&blob, 0), Some(&NARROW[8..22]));
        assert_eq!(
            w11a03_0064dc20_string_bytes(&blob, 2),
            Some(&NARROW[16..22])
        );
        assert_eq!(w11a03_0064dc20_string_bytes(&blob, 3), None);

        // `size` is the authoritative bound: a short record rejects the
        // offset whose entry does not fit.
        let short = w11a03_0064dca0_bind_string_blob(Some(&NARROW), 3, false);
        assert_eq!(w11a03_0064dc20_string_bytes(&short, 0), None);
    }

    #[test]
    fn string_bytes_uses_32_bit_offsets_for_a_wide_blob() {
        let blob = w11a03_0064dca0_bind_string_blob(Some(&WIDE), 18, true);
        assert_eq!(w11a03_0064dc20_string_bytes(&blob, 0), Some(&WIDE[12..18]));
        assert_eq!(w11a03_0064dc20_string_bytes(&blob, 1), Some(&WIDE[15..18]));
        assert_eq!(w11a03_0064dc20_string_bytes(&blob, 2), None);

        // Wide index 0 needs its full 8-byte header.
        let short = w11a03_0064dca0_bind_string_blob(Some(&WIDE), 7, true);
        assert_eq!(w11a03_0064dc20_string_bytes(&short, 0), None);
    }

    #[test]
    fn menu_dispatch_closes_only_an_open_menu() {
        let mut open = W11a03DialogMode {
            mode: 1,
            closed: false,
        };
        assert_eq!(
            w11a03_0064d5d0_dispatch(&mut open, 0),
            W11a03MenuAction::Closed
        );
        assert_eq!(open.mode, 0);

        let mut idle = W11a03DialogMode::default();
        assert_eq!(
            w11a03_0064d5d0_dispatch(&mut idle, 0),
            W11a03MenuAction::Idle
        );

        // A menu screen picks the list opener; 0x28 is the modal branch.
        assert_eq!(
            w11a03_0064d5d0_dispatch(&mut idle, 0x11),
            W11a03MenuAction::OpenList
        );
        assert_eq!(
            w11a03_0064d5d0_dispatch(&mut idle, 0x25),
            W11a03MenuAction::OpenList
        );
        assert_eq!(
            w11a03_0064d5d0_dispatch(&mut idle, 0x28),
            W11a03MenuAction::OpenModal
        );
    }

    #[test]
    fn selection_row_zero_runs_the_close_path() {
        let mut menu = W11a03DialogMode {
            mode: 2,
            closed: false,
        };
        assert_eq!(
            w11a03_0064cef0_selection_changed(&mut menu, true, 0),
            W11a03SelectionAction::Closed
        );
        assert_eq!(menu.mode, 0);

        // Nothing to close: the state-0 dispatch is idle.
        let mut closed = W11a03DialogMode::default();
        assert_eq!(
            w11a03_0064cef0_selection_changed(&mut closed, true, 0),
            W11a03SelectionAction::Ignored
        );

        // An unbound list view returns before either branch.
        let mut open = W11a03DialogMode {
            mode: 1,
            closed: false,
        };
        assert_eq!(
            w11a03_0064cef0_selection_changed(&mut open, false, 4),
            W11a03SelectionAction::Ignored
        );
        assert_eq!(open.mode, 1);
    }

    #[test]
    fn selection_past_row_zero_releases_an_open_menu() {
        let mut menu = W11a03DialogMode {
            mode: 1,
            closed: false,
        };
        assert_eq!(
            w11a03_0064cef0_selection_changed(&mut menu, true, 5),
            W11a03SelectionAction::Released
        );
        assert_eq!(menu.mode, 0);

        let mut closed = W11a03DialogMode::default();
        assert_eq!(
            w11a03_0064cef0_selection_changed(&mut closed, true, 5),
            W11a03SelectionAction::Ignored
        );
    }

    #[test]
    fn close_menu_latches_the_flag_and_raises_error_14() {
        let mut menu = W11a03DialogMode {
            mode: 1,
            closed: false,
        };
        assert_eq!(w11a03_0064cf60_close_menu(&mut menu), Err(0x0e));
        assert_eq!(menu.mode, 0);
        assert!(menu.closed);

        // A second close keeps the latch.
        assert_eq!(w11a03_0064cf60_close_menu(&mut menu), Err(0x0e));
        assert!(menu.closed);
    }

    #[test]
    fn release_gate_needs_a_live_control_word() {
        assert!(!w11a03_0064ceb0_release_gate(0));
        assert!(w11a03_0064ceb0_release_gate(1));
        assert!(w11a03_0064ceb0_release_gate(0xffff));
    }

    #[test]
    fn control_rect_expands_the_cached_edges_by_the_origin() {
        let rect = w11a03_0064c7a0_control_rect([10, 20, 110, 70], (5, 6));
        assert_eq!(
            rect,
            W11a03Rect {
                left: 15,
                top: 26,
                right: 116,
                bottom: 77
            }
        );
        assert_eq!(rect.width(), 101);
        assert_eq!(rect.height(), 51);

        // Negative origins and edges keep the signed arithmetic.
        let shifted = w11a03_0064c7a0_control_rect([-4, -2, 40, 12], (-10, 3));
        assert_eq!(
            shifted,
            W11a03Rect {
                left: -14,
                top: 1,
                right: 31,
                bottom: 16
            }
        );
    }

    #[test]
    fn release_slots_clears_the_live_prefix_and_the_selected_word() {
        let mut slots = [11, 22, 33, 44];
        let mut selected = 7;
        w11a03_0064dff0_release_slots(&mut slots, 2, &mut selected);
        assert_eq!(slots, [0, 0, 33, 44]);
        assert_eq!(selected, 0);

        // count 0 only clears the selected word.
        let mut slots = [11, 22];
        let mut selected = 3;
        w11a03_0064dff0_release_slots(&mut slots, 0, &mut selected);
        assert_eq!(slots, [11, 22]);
        assert_eq!(selected, 0);

        // A count past the slice clamps instead of indexing out of bounds.
        let mut slots = [5];
        let mut selected = 1;
        w11a03_0064dff0_release_slots(&mut slots, 9, &mut selected);
        assert_eq!(slots, [0]);
        assert_eq!(selected, 0);
    }

    #[test]
    fn control_callback_receives_the_control_and_the_argument() {
        let mut seen = Vec::new();
        {
            let mut callback = |control: u32, arg: u16| seen.push((control, arg));
            w11a03_0064c6e0_invoke_control_callback(&mut callback, 0x41, 9);
        }
        assert_eq!(seen, [(0x41, 9)]);
    }

    #[test]
    #[should_panic(expected = "list control missing")]
    fn reject_with_message_throws_the_message() {
        w11a03_0064d630_reject_with_message("list control missing");
    }

    #[test]
    #[should_panic(expected = "row out of range")]
    fn reject_and_forward_keeps_the_message() {
        w11a03_0064d750_reject_and_forward("row out of range");
    }

    #[test]
    fn empty_row_is_a_noop() {
        w11a03_0064d147_noop();
    }
}
