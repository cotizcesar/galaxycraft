#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave14/agent10.rs
// ===========================================================================
// Wave 14 translation batch 10 (staging).
//
// Window: `0x00682000..0x00683fff`.
//
// This slice of the retail executable is mostly MSVC C++ runtime glue: the
// Battle.net chat-dialog wrappers, vftable copy constructors, RTTI getters,
// stack-cookie epilogues and `unaff_*` catch blocks.  The recoverable
// routines are the map-panorama scroll pair (`0x00682140` and `0x006822c0`),
// the Registry-ID copy and `Characters\` path append used by the profile
// loader (`0x00682540`, `0x00682960`, `0x00682e20`), the dword slot-array
// growth policy (`0x00682410`), and the Battle.net chat-dialog glue
// (`0x00683540`, `0x00683760`, `0x00683d80`, `0x00683e40`, `0x00683ff0`).
//
// ## Recovered layouts
//
// Panorama state (`0x00682140`, `0x006822c0`; corroborated by the recentre
// routine `FUN_00681ad0`):
//
// | global | type | role |
// |---|---|---|
// | `0x00fe5b88` | `i32` | horizontal scroll in map pixels |
// | `0x00fe5b8c` | `i32` | vertical scroll in map pixels |
// | `0x00fe5b4c` | `i32` | horizontal ceiling |
// | `0x00fe5b80` / `0x00fe5b84` | `i32` | last applied scroll per axis |
// | `0x00fe5b78` / `0x00fe5b7c` | `u16` | last reported tile column / row |
// | `0x00fe5af4` / `0x00fe5af8` | `i32` | wrap width / height in pixels |
// | `0x00fe5afc..=0x00fe5b0c` | five `i32` | x-layer offsets |
// | `0x00fe5b10..=0x00fe5b20` | five `i32` | y-layer offsets |
// | `0x00f064f0..=0x00f06500` | five `i32` | per-layer scroll speed |
//
// The shipped loops walk the five layer dwords from the *highest* address
// downwards, so index 0 is `0x00fe5b0c` / `0x00fe5b20` and index 4 is
// `0x00fe5afc` / `0x00fe5b10`.  The tile index is written into the view
// record at `+0xe0` (column) or `+0xe2` (row); that record is reached through
// the obfuscated pointer `0x00f063cc ^ 0x10fae560` and lies outside this
// batch, so the translations return the tile instead of storing it.
//
// Chat-dialog record initialised by `0x00683ff0`:
//
// | offset | field |
// |---|---|---|
// | `+0x00` | text pointer, set to the inline buffer at `+0x0c` |
// | `+0x04` | text length, cleared |
// | `+0x08` | capacity word `0x8000_000f` (flag | 15) |
// | `+0x0c` | 16-byte inline text, first byte cleared |
// | `+0x1c` | four colour dwords, copied from `0x00f3ca88..=0x00f3ca94` |
// | `+0x2c` | marker byte, cleared |
//
// The text widget state shared by `0x00683540` and `0x00683e40` is also
// visible: `0x011cb5e4` is the text mode (`0x11` in both label paths),
// `0x011cb5ec` holds the `(x, y)` pair as two little-endian `i16`s,
// `0x011cb5f0` the `(right, bottom)` pair and `0x011cb5fc` a cleared byte.
//
// ## Translation notes
//
// * `FUN_007490d0(0)` gates both scroll routines; the caller supplies its
//   result as `ui_blocked`, and `FUN_0060f8f0()` as `scale` (clamped up to
//   `1.0` exactly as the shipped code does).
// * The three candidates that only re-enter an address outside their own
//   body (`FUN_00682580`, `FUN_00683080`, `FUN_00683ff0`) or that wrap
//   `FUN_00683ff0` plus `FUN_00683810` add no recovered arithmetic and are
//   not translated.
// * The `Characters\` append (`0x00682960`, `0x00682e20`) is the complete
//   recovered string operation; the trailing `sprintf` of `0x00682960`
//   reads its format at `0x00d3ea7c` (outside the body) and the global
//   string assignment at the tail of `0x00682e20` is not modelled, so both
//   take the already-filled buffer from the caller.
// * Every other row in the window is an unwind/catch block (`unaff_EBP`,
//   `in_EAX`, `in_CF`/`in_ZF`), a stack-cookie tail, a vftable copy
//   constructor, a six-byte RTTI pointer getter, or a record scan whose
//   only tail action is a discarded allocator query.  Those are listed in
//   the batch report.
//
// Nothing here touches `World`, `SimUnit`, the renderer or the client.

// ---------------------------------------------------------------------------
// 0x00682140 / 0x006822c0 - map panorama scroll
// ---------------------------------------------------------------------------

/// Recovered panorama state shared by `0x00682140` (scroll right) and
/// `0x006822c0` (scroll up).  Fields are grouped per axis so one record can
/// serve both routines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14a10Panorama {
    /// `0x00fe5b88`: horizontal scroll in map pixels.
    pub x: i32,
    /// `0x00fe5b4c`: horizontal ceiling in map pixels.
    pub x_limit: i32,
    /// `0x00fe5b80`: last `x` the five x layers were moved for.
    pub x_applied: i32,
    /// `0x00fe5b78`: last reported tile column.
    pub x_reported: u16,
    /// `0x00fe5b8c`: vertical scroll in map pixels.
    pub y: i32,
    /// `0x00fe5b84`: last `y` the five y layers were moved for.
    pub y_applied: i32,
    /// `0x00fe5b7c`: last reported tile row.
    pub y_reported: u16,
    /// `0x00fe5af4`: map width in pixels (x-layer wrap).
    pub width: i32,
    /// `0x00fe5af8`: map height in pixels (y-layer wrap).
    pub height: i32,
    /// `0x00fe5afc..=0x00fe5b0c`: five x-layer offsets, index 0 highest.
    pub x_layers: [i32; 5],
    /// `0x00fe5b10..=0x00fe5b20`: five y-layer offsets, index 0 highest.
    pub y_layers: [i32; 5],
    /// `0x00f064f0..=0x00f06500`: per-layer speed, index 0 highest.
    pub speeds: [i32; 5],
}

/// Outcome of one panorama scroll step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W14a10ScrollStep {
    /// The tile index the shipped routine writes to `+0xe0` / `+0xe2`.
    pub tile: u16,
    /// The shipped routine's return value: `true` when the layers moved.
    pub moved: bool,
    /// Rows whose redraw `0x006822c0` requests, in call order (highest
    /// first).  Always empty for `0x00682140`.
    pub redraw_rows: Vec<u16>,
}

/// `0x00682140`: advances the horizontal panorama by `amount` scaled pixels,
/// clamps it to `x_limit`, moves the five x layers by `speed * delta` and
/// wraps them by `width`.  Returns `None` when `ui_blocked` mirrors the
/// non-zero result of `FUN_007490d0(0)`.
///
/// The shipped body ends with
/// `if (x_reported != tile && x_reported < tile) FUN_0060f870();`; that call
/// only reads the screen-width global and its result is discarded, so the
/// reported column is left untouched.
pub fn w14a10_panorama_scroll_right(
    state: &mut W14a10Panorama,
    amount: i32,
    ui_blocked: bool,
    scale: f32,
) -> Option<W14a10ScrollStep> {
    if ui_blocked {
        return None;
    }
    let mut delta = w14a10_scroll_delta(amount, scale);
    state.x = state.x.wrapping_add(delta);
    if state.x_limit < state.x {
        delta = delta.wrapping_add(state.x_limit.wrapping_sub(state.x));
        state.x = state.x_limit;
    }
    let tile = (state.x >> 5) as u16;
    let moved = state.x_applied != state.x;
    if moved {
        state.x_applied = state.x;
        for (layer, speed) in state.x_layers.iter_mut().zip(state.speeds.iter().copied()) {
            let next = layer.wrapping_sub(speed.wrapping_mul(delta));
            *layer = if next < 0 {
                next.wrapping_add(state.width)
            } else {
                next
            };
        }
    }
    Some(W14a10ScrollStep {
        tile,
        moved,
        redraw_rows: Vec::new(),
    })
}

/// `0x006822c0`: retreats the vertical panorama by `amount` scaled pixels,
/// clamps it at zero, moves the five y layers by `speed * delta`, wraps them
/// by `height`, and lowers the reported tile row to the new tile.  Returns
/// `None` when `ui_blocked` mirrors the non-zero result of `FUN_007490d0(0)`.
///
/// When the new tile is below the reported row, the shipped loop decrements
/// `y_reported` once per row and calls `FUN_006811c0(tile_base + (stride *
/// row + column) * 2, column, row)` after each decrement.  The rows are
/// returned in `redraw_rows` in that order; the caller owns `tile_base`,
/// `stride` and `column` (the view record is outside this batch).
pub fn w14a10_panorama_scroll_down(
    state: &mut W14a10Panorama,
    amount: i32,
    ui_blocked: bool,
    scale: f32,
) -> Option<W14a10ScrollStep> {
    if ui_blocked {
        return None;
    }
    let mut delta = w14a10_scroll_delta(amount, scale);
    state.y = state.y.wrapping_sub(delta);
    if state.y < 0 {
        delta = delta.wrapping_add(state.y);
        state.y = 0;
    }
    let tile = (state.y >> 5) as u16;
    let mut redraw_rows = Vec::new();
    let moved = state.y_applied != state.y;
    if moved {
        state.y_applied = state.y;
        for (layer, speed) in state.y_layers.iter_mut().zip(state.speeds.iter().copied()) {
            let next = layer.wrapping_add(speed.wrapping_mul(delta));
            *layer = if state.height < next {
                next.wrapping_sub(state.height)
            } else {
                next
            };
        }
        let mut row = state.y_reported;
        while tile < row {
            row -= 1;
            redraw_rows.push(row);
        }
        state.y_reported = row;
    }
    Some(W14a10ScrollStep {
        tile,
        moved,
        redraw_rows,
    })
}

/// The pixel delta common to both scroll routines: `FUN_0060f8f0()` value
/// (`scale`) is raised to `1.0` when it is not above it, then `amount` is
/// divided by it and truncated toward zero.
fn w14a10_scroll_delta(amount: i32, scale: f32) -> i32 {
    let scale = if scale <= 1.0 { 1.0 } else { scale };
    (amount as f32 / scale) as i32
}

// ---------------------------------------------------------------------------
// 0x00682410 - dword slot array growth
// ---------------------------------------------------------------------------

/// Result of [`w14a10_slot_array_grow`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W14a10Resize {
    /// The requested length already matches the used slots.
    Unchanged,
    /// New slots were appended and filled with `0xffff_ffff`.
    Grown,
    /// The shipped shrink branch calls an unresolved helper and does not
    /// even update the used-slot word; it is not modelled.
    ShrinkUnsupported,
}

/// Recovered dword array layout of `0x00682410`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W14a10SlotArray {
    /// `in_ECX[1]`: used slots.
    pub len: u32,
    /// `in_ECX[2]`: capacity in the low 31 bits; the top bit is the shipped
    /// flag and is preserved when the capacity is raised.
    pub capacity: u32,
    /// The slot storage (`in_ECX[0]` is its base pointer).
    pub slots: Vec<u32>,
}

/// `0x00682410`: grows the dword slot array to `wanted` entries.  When the
/// current capacity is too small it becomes
/// `max(capacity + capacity / 2, wanted)`; the high capacity bit is kept.
/// Each appended slot is set to `0xffff_ffff` and the used-slot word is
/// updated.  The shrink branch is not recoverable (the shipped call passes a
/// zero length to an unresolved helper and leaves `len` untouched).
pub fn w14a10_slot_array_grow(array: &mut W14a10SlotArray, wanted: u32) -> W14a10Resize {
    if wanted < array.len {
        return W14a10Resize::ShrinkUnsupported;
    }
    if wanted == array.len {
        return W14a10Resize::Unchanged;
    }
    let capacity = array.capacity & 0x7fff_ffff;
    if capacity < wanted {
        let grown = (capacity >> 1).wrapping_add(capacity);
        let target = grown.max(wanted);
        array.capacity = (array.capacity & 0x8000_0000) | (target & 0x7fff_ffff);
    }
    array.slots.resize(wanted as usize, u32::MAX);
    array.len = wanted;
    W14a10Resize::Grown
}

// ---------------------------------------------------------------------------
// 0x00682540 - Registry ID copy into a retail record
// ---------------------------------------------------------------------------

/// Copy budget of `0x00682540` (`0x5f` bytes).
pub const W14A10_NAME_COPY_BUDGET: usize = 0x5f;

/// `0x00682540`: copies at most `0x5f` bytes of a NUL-terminated Registry ID
/// into a destination buffer; the routine is called with the retail record's
/// second name field (`record + 0x70`) as destination.  No terminator is
/// appended when the source is at least that long (`memmove` with the
/// `min(strlen, 0x5f)` count).  Returns the copied byte count, or `None` when
/// `dest` is shorter than the copy.
pub fn w14a10_copy_record_name(dest: &mut [u8], name: &[u8]) -> Option<usize> {
    let count = name.len().min(W14A10_NAME_COPY_BUDGET);
    if dest.len() < count {
        return None;
    }
    dest[..count].copy_from_slice(&name[..count]);
    Some(count)
}

// ---------------------------------------------------------------------------
// 0x00682960 / 0x00682e20 - "Characters\" path append
// ---------------------------------------------------------------------------

/// The literal appended by both path routines (`0x0b` bytes).
const W14A10_CHARACTERS_DIR: &[u8] = b"Characters\\";
/// `0x104`-byte path buffers used by the shipped routines.
const W14A10_PATH_CAPACITY: usize = 0x104;
/// Last writable byte of a `0x104`-byte path buffer.
const W14A10_PATH_LAST: usize = 0x103;

/// Appends `Characters\` to a path of `len` bytes inside `buf`, clipping the
/// result to `0x103` bytes (the shipped `min(0x103, len + 0x0b)` count).
/// Returns the new length, or `None` when the source length is already
/// `0x104` (the shipped routine skips the append) or `buf` cannot hold the
/// clipped path.
fn w14a10_append_characters_into(buf: &mut [u8], len: usize) -> Option<usize> {
    if len >= W14A10_PATH_CAPACITY {
        return None;
    }
    let target = (len + W14A10_CHARACTERS_DIR.len()).min(W14A10_PATH_LAST);
    if buf.len() < target {
        return None;
    }
    let count = target - len;
    buf[len..target].copy_from_slice(&W14A10_CHARACTERS_DIR[..count]);
    Some(target)
}

/// `0x00682960`: appends `Characters\` to the current directory already
/// placed in a `0x104`-byte buffer by `FUN_0092a4f0`, clipping at `0x103`
/// bytes.  This is the recovered string operation only: the trailing
/// `sprintf` uses the format at `0x00d3ea7c` and a `.spc`/`.mpc` suffix
/// selected by `0x0119f7bc`, neither of which is visible in the body.
pub fn w14a10_append_characters_dir(buf: &mut [u8], len: usize) -> Option<usize> {
    w14a10_append_characters_into(buf, len)
}

/// `0x00682e20`: reads the NUL-terminated directory from the `0x104`-byte
/// buffer, appends `Characters\` with the same `0x103` clip, and returns the
/// resulting length (the shipped tail then assigns that length to the global
/// path string, which is not modelled).  Returns `None` when the buffer holds
/// no terminator within `0x104` bytes or is too short for the clipped path.
pub fn w14a10_characters_dir_length(buf: &mut [u8]) -> Option<usize> {
    let len = buf
        .iter()
        .take(W14A10_PATH_CAPACITY)
        .position(|&byte| byte == 0)?;
    w14a10_append_characters_into(buf, len)
}

// ---------------------------------------------------------------------------
// 0x00683540 / 0x00683e40 - text widget clip rectangle
// ---------------------------------------------------------------------------

/// Text mode written to `0x011cb5e4` by the label paths.
pub const W14A10_TEXT_MODE: u32 = 0x11;
/// Fifth `FUN_0097b5d0` argument for the normal label state (`0x4d`).
pub const W14A10_LABEL_ACCENT_NORMAL: u32 = 0x4d;
/// Fifth `FUN_0097b5d0` argument when bit 29 of the style word is set
/// (`0x35`).
pub const W14A10_LABEL_ACCENT_PRESSED: u32 = 0x35;
/// Label copy budget of `0x00683e40` into its 256-byte local (`0xff`).
pub const W14A10_LABEL_BUDGET: usize = 0xff;

/// The text clip rectangle stored at `0x011cb5ec` (`x`, `y`) and
/// `0x011cb5f0` (`right`, `bottom`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14a10TextClip {
    pub x: i16,
    pub y: i16,
    pub right: i16,
    pub bottom: i16,
}

/// `0x00683540`: installs the text clip from two packed dword pairs
/// (`param_3` into `0x011cb5ec`, `param_4` into `0x011cb5f0`), clears
/// `0x011cb5fc` and returns the visible text length: the terminator scan
/// result `text_len` clamped to the record byte capacity.  The temporary
/// clear of `0x00f38810`, the global string assignment and the terminal
/// stack-cookie check are not modelled.
pub fn w14a10_text_payload(
    packed_origin: u32,
    packed_corner: u32,
    text_len: u32,
    capacity: u32,
) -> (W14a10TextClip, u32) {
    (
        W14a10TextClip {
            x: packed_origin as u16 as i16,
            y: (packed_origin >> 16) as u16 as i16,
            right: packed_corner as u16 as i16,
            bottom: (packed_corner >> 16) as u16 as i16,
        },
        text_len.min(capacity),
    )
}

/// Recovered label layout of the list-item path in `0x00683e40`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14a10ListItemLabel {
    /// Text clip: the rectangle's top-left is offset by `dx`/`dy`, the
    /// right/bottom pair is unchanged.
    pub clip: W14a10TextClip,
    /// Inclusive frame width passed to `FUN_0097b5d0`:
    /// `(right - left) + 1`.
    pub frame_width: i32,
    /// The `0x4d`/`0x35` argument selected by bit 29 of the item style word.
    pub accent: u32,
    /// Label copy budget into the 256-byte local: `min(len, 0xff)`.
    pub budget: usize,
}

/// `0x00683e40`: recovered non-drawing core of the list-item label path.
/// The style word at `this + 0x30` supplies bit 29 (`pressed`); the label
/// rectangle is `[left, top, right, bottom]` with the text origin moved by
/// `dx`/`dy`, the frame width is inclusive, and the label is copied with a
/// `0xff` byte budget and text mode `0x11`.  The two palette globals, the
/// sprite call and the thrown string are outside the recovered body.
pub fn w14a10_list_item_label(
    rect: [i16; 4],
    dx: i16,
    dy: i16,
    label_len: usize,
    pressed: bool,
) -> W14a10ListItemLabel {
    W14a10ListItemLabel {
        clip: W14a10TextClip {
            x: rect[0].wrapping_add(dx),
            y: rect[1].wrapping_add(dy),
            right: rect[2],
            bottom: rect[3],
        },
        frame_width: rect[2] as i32 - rect[0] as i32 + 1,
        accent: if pressed {
            W14A10_LABEL_ACCENT_PRESSED
        } else {
            W14A10_LABEL_ACCENT_NORMAL
        },
        budget: label_len.min(W14A10_LABEL_BUDGET),
    }
}

// ---------------------------------------------------------------------------
// 0x00683760 - chat list insertion/selection sync
// ---------------------------------------------------------------------------

/// The six list-control operations used by `0x00683760`, with the meaning
/// recovered from the calls: lookup by message id (`FUN_009791b0` with
/// type `0x10`), list reset (`FUN_0097a1f0(this, 0)`), unread-counter store
/// (`FUN_00964f90`), selection (`FUN_0097a320`), row count
/// (`FUN_00979cc0`) and scroll-to-row (`FUN_0097a370`).
pub trait W14a10ChatList {
    /// Index of `message` in the list, or `None` when absent.
    fn find(&mut self, message: u32) -> Option<u8>;
    /// Clear the list before a retry.
    fn reset(&mut self);
    /// Store the new unread count.
    fn set_counter(&mut self, value: u8);
    /// Select `index` (`-1` when the retry also failed) at `stamp`.
    fn select(&mut self, index: i8, stamp: u32);
    /// Number of rows in the list.
    fn row_count(&mut self) -> u8;
    /// Scroll so `index` is visible.
    fn ensure_visible(&mut self, index: u8);
}

/// `0x00683760`: inserts a chat message into the dialog list.  The message
/// id is looked up in the list; when it is missing the list is reset, the
/// lookup is retried and the unread counter at `this + 0x67` is decremented
/// (with wrapping) and stored.  The found index is selected with the current
/// tick (`FUN_00a902b0`).  When the list's two scroll words (`+0x54` and
/// `+0x58`, passed as `flat`) were equal before the update, the last row is
/// scrolled into view.
pub fn w14a10_chat_message_sync<L: W14a10ChatList>(
    list: &mut L,
    message: u32,
    unread: &mut u8,
    stamp: u32,
    flat: bool,
) {
    let mut index = list.find(message);
    if index.is_none() {
        list.reset();
        index = list.find(message);
        if *unread != 0 {
            *unread = unread.wrapping_sub(1);
            list.set_counter(*unread);
        }
    }
    list.select(index.map_or(-1, |index| index as i8), stamp);
    if flat {
        let count = list.row_count();
        if count != 0 {
            list.ensure_visible(count - 1);
        }
    }
}

// ---------------------------------------------------------------------------
// 0x00683d80 - chat dialog callback dispatch
// ---------------------------------------------------------------------------

/// Event type handled by the chat callback (`0x0e`).
pub const W14A10_CHAT_EVENT: u16 = 0x0e;

/// What the caller of `0x00683d80` must do for the recovered actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W14a10ChatAction {
    /// Action 0: run the four open helpers, then increment the depth byte at
    /// `0x0117ee64`.  Returns 1.
    Opened { depth: u8 },
    /// Action 1: close the dialog, clear the modal handle `0x01165cfc`, and
    /// return 1.
    Close,
    /// Action 2 with `depth != 1`: subtract two from the depth byte and run
    /// `FUN_0074af70`.  Returns 1.
    Popped { depth: u8 },
    /// Action 2 with `depth == 1`: subtract one from the depth byte and run
    /// `FUN_00973630`.  Returns 1.
    ClosedLast { depth: u8 },
    /// Any other event/action: forward to `FUN_0071ef40` and return its
    /// result.
    Forward,
}

impl W14a10ChatAction {
    /// The shipped routine returns 1 for every translated arm; `Forward`
    /// returns the default handler's value instead.
    pub fn handled(self) -> bool {
        !matches!(self, W14a10ChatAction::Forward)
    }
}

/// `0x00683d80`: chat-dialog callback decision.  The record's low `i16` at
/// `+0x10` (`param_2[4]`) is the event type and `param_2[0]` the action
/// code.  Only event `0x0e` has recovered arms; everything else is forwarded
/// to `FUN_0071ef40`.  The depth byte at `0x0117ee64` is `u8` and its
/// arithmetic wraps (the shipped code uses a signed `char`).
pub fn w14a10_chat_dialog_event(event: u16, action: i32, depth: &mut u8) -> W14a10ChatAction {
    if event != W14A10_CHAT_EVENT {
        return W14a10ChatAction::Forward;
    }
    match action {
        0 => {
            *depth = depth.wrapping_add(1);
            W14a10ChatAction::Opened { depth: *depth }
        }
        1 => W14a10ChatAction::Close,
        2 => {
            if depth.wrapping_sub(1) != 0 {
                *depth = depth.wrapping_sub(2);
                W14a10ChatAction::Popped { depth: *depth }
            } else {
                *depth = depth.wrapping_sub(1);
                W14a10ChatAction::ClosedLast { depth: *depth }
            }
        }
        _ => W14a10ChatAction::Forward,
    }
}

// ---------------------------------------------------------------------------
// 0x00683ff0 - chat record initialisation
// ---------------------------------------------------------------------------

/// Capacity word installed in the record's inline text (`0x8000_000f`).
pub const W14A10_SSO_CAPACITY: u32 = 0x8000_000f;
/// Message kind that takes the append arm of `0x00683ff0`.
pub const W14A10_CHAT_MESSAGE_KIND: u32 = 5;

/// The inline text header installed at `+0x00..+0x0c` of the chat record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14a10SsoText {
    /// `+0x04`: current byte length.
    pub len: u32,
    /// `+0x08`: capacity word, with the `0x8000_0000` flag preserved.
    pub capacity: u32,
    /// `+0x0c`: inline buffer; the shipped code clears its first byte.
    pub inline: [u8; 16],
}

impl W14a10SsoText {
    /// The empty inline text as installed by the routine.
    pub const fn new() -> Self {
        W14a10SsoText {
            len: 0,
            capacity: W14A10_SSO_CAPACITY,
            inline: [0; 16],
        }
    }
}

impl Default for W14a10SsoText {
    fn default() -> Self {
        Self::new()
    }
}

/// The record built by `0x00683ff0` before the two-way branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14a10ChatInit {
    /// Inline text header at `+0x00`.
    pub text: W14a10SsoText,
    /// Four dwords copied from `0x00f3ca88..=0x00f3ca94` into `+0x1c`.
    pub colors: [u32; 4],
    /// Byte at `+0x2c`, cleared.
    pub marker: u8,
}

/// Follow-up selected by the message kind in `0x00683ff0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W14a10ChatInitNext {
    /// Kind 5 with subtype 0: `FUN_00713d60(handle, stamp, 0, record,
    /// record + 7)` followed by `FUN_00684570`.
    Append { handle: u32, stamp: u32 },
    /// Any other kind: `FUN_00538810(record[0xa9])` reserves that many bytes.
    Reserve { bytes: u32 },
}

/// `0x00683ff0`: initialises the inline text header (`len = 0`, capacity
/// `0x8000_000f`, pointer to the inline buffer), copies the four colour
/// dwords and clears the `+0x2c` marker.  Kind 5 with subtype 0 takes the
/// chat-append arm with the record's handle (`+0x2a0`) and the tick obtained
/// from `FUN_007eea60`; every other kind reserves the byte count stored at
/// `+0x2a4` (`param_2[0xa9]`).
pub fn w14a10_chat_init(
    kind: u32,
    subtype: u32,
    handle: u32,
    reserve_bytes: u32,
    stamp: u32,
    colors: [u32; 4],
) -> (W14a10ChatInit, W14a10ChatInitNext) {
    let init = W14a10ChatInit {
        text: W14a10SsoText::new(),
        colors,
        marker: 0,
    };
    let next = if kind == W14A10_CHAT_MESSAGE_KIND && subtype == 0 {
        W14a10ChatInitNext::Append { handle, stamp }
    } else {
        W14a10ChatInitNext::Reserve {
            bytes: reserve_bytes,
        }
    };
    (init, next)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_wave14_agent10 {
    use super::*;
    use std::collections::VecDeque;

    fn panorama() -> W14a10Panorama {
        W14a10Panorama {
            x: 0,
            x_limit: 0x1000,
            x_applied: 0,
            x_reported: 0,
            y: 0x800,
            y_applied: 0x800,
            y_reported: 64,
            width: 0x400,
            height: 0x800,
            x_layers: [0; 5],
            y_layers: [0; 5],
            speeds: [1, 2, 4, 8, 16],
        }
    }

    #[test]
    fn scroll_right_moves_layers_and_wraps_by_width() {
        let mut state = panorama();
        let step = w14a10_panorama_scroll_right(&mut state, 64, false, 1.0).expect("step");
        assert_eq!(step.tile, 2);
        assert!(step.moved);
        assert!(step.redraw_rows.is_empty());
        assert_eq!(state.x, 64);
        assert_eq!(state.x_applied, 64);
        assert_eq!(state.x_layers, [960, 896, 768, 512, 0]);

        let step = w14a10_panorama_scroll_right(&mut state, 0, false, 1.0).expect("step");
        assert_eq!(step.tile, 2);
        assert!(!step.moved);
        assert_eq!(state.x_layers, [960, 896, 768, 512, 0]);

        assert_eq!(
            w14a10_panorama_scroll_right(&mut state, 64, true, 1.0),
            None
        );
        assert_eq!(state.x, 64);
    }

    #[test]
    fn scroll_right_scales_and_clamps_delta() {
        let mut state = panorama();
        let step = w14a10_panorama_scroll_right(&mut state, 100, false, 2.0).expect("step");
        assert_eq!(state.x, 50);
        assert_eq!(step.tile, 1);

        let mut state = panorama();
        w14a10_panorama_scroll_right(&mut state, 100, false, 0.25).expect("step");
        assert_eq!(state.x, 100);

        let mut state = panorama();
        state.x = 0xff0;
        state.x_applied = 0xff0;
        let step = w14a10_panorama_scroll_right(&mut state, 0x40, false, 1.0).expect("step");
        assert_eq!(state.x, 0x1000);
        assert_eq!(step.tile, 0x80);
        // delta 0x40 is clipped by 0x30, so 0x10 pixels still move the layers.
        assert_eq!(state.x_layers, [1008, 992, 960, 896, 768]);
    }

    #[test]
    fn scroll_down_clamps_at_zero_and_redraws_rows() {
        let mut state = panorama();
        let step = w14a10_panorama_scroll_down(&mut state, 64, false, 1.0).expect("step");
        assert_eq!(state.y, 0x7c0);
        assert_eq!(step.tile, 62);
        assert!(step.moved);
        assert_eq!(step.redraw_rows, vec![63, 62]);
        assert_eq!(state.y_reported, 62);
        assert_eq!(state.y_layers, [64, 128, 256, 512, 1024]);

        let mut state = panorama();
        state.y = 0x20;
        state.y_applied = 0x20;
        let step = w14a10_panorama_scroll_down(&mut state, 0x40, false, 1.0).expect("step");
        assert_eq!(state.y, 0);
        assert_eq!(step.tile, 0);
        // delta 0x40 is clipped by 0x20, so 0x20 pixels still move the layers.
        assert_eq!(state.y_layers, [32, 64, 128, 256, 512]);

        let mut state = panorama();
        state.y_reported = 10;
        let step = w14a10_panorama_scroll_down(&mut state, 64, false, 1.0).expect("step");
        assert_eq!(step.tile, 62);
        assert_eq!(state.y_reported, 10);
        assert!(step.redraw_rows.is_empty());
    }

    #[test]
    fn scroll_down_wraps_layers_by_height() {
        let mut state = panorama();
        state.height = 0x300;
        state.speeds = [1, 2, 4, 8, 64];
        w14a10_panorama_scroll_down(&mut state, 64, false, 1.0).expect("step");
        // The shipped wrap subtracts the extent exactly once, even when the
        // moved offset is still above it (0x1000 - 0x300 = 0xd00).
        assert_eq!(state.y_layers, [64, 128, 256, 512, 0xd00]);
    }

    #[test]
    fn slot_array_grows_with_one_and_a_half_policy() {
        let mut array = W14a10SlotArray {
            len: 2,
            capacity: 0x8000_0004,
            slots: vec![10, 20],
        };
        assert_eq!(w14a10_slot_array_grow(&mut array, 5), W14a10Resize::Grown);
        assert_eq!(array.len, 5);
        assert_eq!(array.capacity, 0x8000_0006);
        assert_eq!(array.slots, vec![10, 20, u32::MAX, u32::MAX, u32::MAX]);

        assert_eq!(
            w14a10_slot_array_grow(&mut array, 5),
            W14a10Resize::Unchanged
        );
        assert_eq!(
            w14a10_slot_array_grow(&mut array, 4),
            W14a10Resize::ShrinkUnsupported
        );
        assert_eq!(array.len, 5);

        let mut array = W14a10SlotArray {
            len: 2,
            capacity: 0x8000_0004,
            slots: vec![1, 2],
        };
        assert_eq!(w14a10_slot_array_grow(&mut array, 10), W14a10Resize::Grown);
        assert_eq!(array.capacity, 0x8000_000a);
        assert_eq!(array.slots.len(), 10);

        let mut array = W14a10SlotArray {
            len: 2,
            capacity: 0x8000_0010,
            slots: vec![1, 2],
        };
        assert_eq!(w14a10_slot_array_grow(&mut array, 6), W14a10Resize::Grown);
        assert_eq!(array.capacity, 0x8000_0010);
    }

    #[test]
    fn record_name_copy_is_clipped_at_0x5f() {
        let name = [b'n'; W14A10_NAME_COPY_BUDGET + 5];
        let mut dest = [0u8; W14A10_NAME_COPY_BUDGET];
        assert_eq!(
            w14a10_copy_record_name(&mut dest, &name),
            Some(W14A10_NAME_COPY_BUDGET)
        );
        assert!(dest.iter().all(|&byte| byte == b'n'));

        let mut dest = [0u8; 4];
        assert_eq!(w14a10_copy_record_name(&mut dest, b"abc"), Some(3));
        assert_eq!(&dest[..3], b"abc");
        assert_eq!(dest[3], 0);

        assert_eq!(w14a10_copy_record_name(&mut [], b""), Some(0));
        assert_eq!(w14a10_copy_record_name(&mut [0u8; 2], b"abc"), None);
    }

    #[test]
    fn characters_append_clips_at_0x103() {
        let mut buf = [0u8; W14A10_PATH_CAPACITY];
        assert_eq!(w14a10_append_characters_dir(&mut buf, 0), Some(0x0b));
        assert_eq!(&buf[..0x0b], b"Characters\\");
        assert_eq!(buf[0x0b], 0);

        let mut buf = [b'x'; W14A10_PATH_CAPACITY];
        assert_eq!(w14a10_append_characters_dir(&mut buf, 0xf8), Some(0x103));
        assert_eq!(&buf[0xf8..0x103], b"Characters\\");

        let mut buf = [b'x'; W14A10_PATH_CAPACITY];
        assert_eq!(w14a10_append_characters_dir(&mut buf, 0x103), Some(0x103));
        assert!(buf.iter().all(|&byte| byte == b'x'));

        assert_eq!(w14a10_append_characters_dir(&mut buf, 0x104), None);
        assert_eq!(w14a10_append_characters_dir(&mut [0u8; 4], 0), None);
    }

    #[test]
    fn characters_dir_length_reads_the_c_string() {
        let mut buf = [0u8; W14A10_PATH_CAPACITY];
        buf[..4].copy_from_slice(b"dir\0");
        assert_eq!(w14a10_characters_dir_length(&mut buf), Some(0x0e));
        assert_eq!(&buf[3..0x0e], b"Characters\\");

        let mut full = [b'a'; W14A10_PATH_CAPACITY];
        assert_eq!(w14a10_characters_dir_length(&mut full), None);

        let mut short = [b'a'; 8];
        assert_eq!(w14a10_characters_dir_length(&mut short), None);
    }

    #[test]
    fn text_payload_unpacks_the_packed_pairs() {
        let (clip, len) = w14a10_text_payload(0x0064_00c8, 0x0190_012c, 300, 255);
        assert_eq!(
            clip,
            W14a10TextClip {
                x: 200,
                y: 100,
                right: 300,
                bottom: 400,
            }
        );
        assert_eq!(len, 255);

        let (clip, len) = w14a10_text_payload(0xffff_fffe, 0x0001_ffff, 10, 0);
        assert_eq!(
            clip,
            W14a10TextClip {
                x: -2,
                y: -1,
                right: -1,
                bottom: 1,
            }
        );
        assert_eq!(len, 0);
    }

    #[test]
    fn list_item_label_offsets_and_budget() {
        let label = w14a10_list_item_label([10, 20, 110, 40], 3, 4, 300, false);
        assert_eq!(
            label.clip,
            W14a10TextClip {
                x: 13,
                y: 24,
                right: 110,
                bottom: 40,
            }
        );
        assert_eq!(label.accent, W14A10_LABEL_ACCENT_NORMAL);
        assert_eq!(label.budget, W14A10_LABEL_BUDGET);
        assert_eq!(label.frame_width, 101);

        let label = w14a10_list_item_label([10, 20, 110, 40], 0, 0, 255, true);
        assert_eq!(label.accent, W14A10_LABEL_ACCENT_PRESSED);
        assert_eq!(label.budget, 255);
        assert_eq!(W14A10_TEXT_MODE, 0x11);

        let label = w14a10_list_item_label([0x7fff, 0, 0, 0], 1, -1, 0, false);
        assert_eq!(label.clip.x, -32768);
        assert_eq!(label.clip.y, -1);
    }

    struct MockList {
        lookups: VecDeque<Option<u8>>,
        calls: Vec<&'static str>,
        selected: Option<(i8, u32)>,
        counter: Option<u8>,
        visible: Option<u8>,
        rows: u8,
    }

    impl MockList {
        fn new(lookups: impl IntoIterator<Item = Option<u8>>, rows: u8) -> Self {
            MockList {
                lookups: lookups.into_iter().collect(),
                calls: Vec::new(),
                selected: None,
                counter: None,
                visible: None,
                rows,
            }
        }
    }

    impl W14a10ChatList for MockList {
        fn find(&mut self, _message: u32) -> Option<u8> {
            self.calls.push("find");
            self.lookups.pop_front().flatten()
        }

        fn reset(&mut self) {
            self.calls.push("reset");
        }

        fn set_counter(&mut self, value: u8) {
            self.calls.push("counter");
            self.counter = Some(value);
        }

        fn select(&mut self, index: i8, stamp: u32) {
            self.calls.push("select");
            self.selected = Some((index, stamp));
        }

        fn row_count(&mut self) -> u8 {
            self.calls.push("rows");
            self.rows
        }

        fn ensure_visible(&mut self, index: u8) {
            self.calls.push("visible");
            self.visible = Some(index);
        }
    }

    #[test]
    fn chat_sync_selects_an_existing_message() {
        let mut list = MockList::new([Some(3)], 7);
        let mut unread = 5u8;
        w14a10_chat_message_sync(&mut list, 0xabc, &mut unread, 100, true);
        assert_eq!(list.calls, ["find", "select", "rows", "visible"]);
        assert_eq!(list.selected, Some((3, 100)));
        assert_eq!(list.visible, Some(6));
        assert_eq!(unread, 5);
        assert_eq!(list.counter, None);
    }

    #[test]
    fn chat_sync_resets_and_retries_a_missing_message() {
        let mut list = MockList::new([None, Some(1)], 4);
        let mut unread = 2u8;
        w14a10_chat_message_sync(&mut list, 0xabc, &mut unread, 7, false);
        assert_eq!(list.calls, ["find", "reset", "find", "counter", "select"]);
        assert_eq!(unread, 1);
        assert_eq!(list.counter, Some(1));
        assert_eq!(list.selected, Some((1, 7)));
        assert_eq!(list.visible, None);
    }

    #[test]
    fn chat_sync_keeps_zero_unread_and_selects_minus_one() {
        let mut list = MockList::new([None, None], 0);
        let mut unread = 0u8;
        w14a10_chat_message_sync(&mut list, 0xabc, &mut unread, 9, true);
        assert_eq!(list.calls, ["find", "reset", "find", "select", "rows"]);
        assert_eq!(unread, 0);
        assert_eq!(list.counter, None);
        assert_eq!(list.selected, Some((-1, 9)));
        assert_eq!(list.visible, None);
    }

    #[test]
    fn chat_dialog_actions_wrap_the_depth_byte() {
        let mut depth = 0u8;
        assert_eq!(
            w14a10_chat_dialog_event(13, 0, &mut depth),
            W14a10ChatAction::Forward
        );
        assert_eq!(depth, 0);
        assert!(!W14a10ChatAction::Forward.handled());

        assert_eq!(
            w14a10_chat_dialog_event(W14A10_CHAT_EVENT, 0, &mut depth),
            W14a10ChatAction::Opened { depth: 1 }
        );
        assert!(W14a10ChatAction::Opened { depth: 1 }.handled());

        assert_eq!(
            w14a10_chat_dialog_event(W14A10_CHAT_EVENT, 1, &mut depth),
            W14a10ChatAction::Close
        );
        assert_eq!(depth, 1);

        assert_eq!(
            w14a10_chat_dialog_event(W14A10_CHAT_EVENT, 2, &mut depth),
            W14a10ChatAction::ClosedLast { depth: 0 }
        );

        let mut depth = 3u8;
        assert_eq!(
            w14a10_chat_dialog_event(W14A10_CHAT_EVENT, 2, &mut depth),
            W14a10ChatAction::Popped { depth: 1 }
        );

        let mut depth = 0u8;
        assert_eq!(
            w14a10_chat_dialog_event(W14A10_CHAT_EVENT, 2, &mut depth),
            W14a10ChatAction::Popped { depth: 254 }
        );

        assert_eq!(
            w14a10_chat_dialog_event(W14A10_CHAT_EVENT, 3, &mut depth),
            W14a10ChatAction::Forward
        );
    }

    #[test]
    fn chat_init_builds_the_inline_text_and_selects_the_follow_up() {
        let (init, next) = w14a10_chat_init(5, 0, 0x1234, 77, 900, [1, 2, 3, 4]);
        assert_eq!(init.text.len, 0);
        assert_eq!(init.text.capacity, W14A10_SSO_CAPACITY);
        assert_eq!(init.text.inline, [0u8; 16]);
        assert_eq!(init.colors, [1, 2, 3, 4]);
        assert_eq!(init.marker, 0);
        assert_eq!(
            next,
            W14a10ChatInitNext::Append {
                handle: 0x1234,
                stamp: 900,
            }
        );
        assert_eq!(W14a10SsoText::default(), W14a10SsoText::new());

        let (_, next) = w14a10_chat_init(5, 1, 0, 77, 900, [0; 4]);
        assert_eq!(next, W14a10ChatInitNext::Reserve { bytes: 77 });

        let (_, next) = w14a10_chat_init(4, 0, 0, 12, 900, [0; 4]);
        assert_eq!(next, W14a10ChatInitNext::Reserve { bytes: 12 });
    }
}
