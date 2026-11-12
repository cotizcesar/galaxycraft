#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave13/agent09.rs
// ===========================================================================
// Wave 13 translation batch 09: settings-group bookkeeping and the
// mission-end dispatcher in `0066c000..0066dfff`.
//
// The ledger lists 27 `decompiled_uncited` rows in this window. Eighteen of
// them carry a recoverable contract and are translated below; the other nine
// are listed under [Skipped rows](#skipped-rows) with the reason each one is
// not modelled.
//
// The block is client/configuration plumbing: a small string-keyed settings
// table, the two save/flush paths that commit it, a tagged-value destructor,
// and the mission-end state machine (start, dispatch, dialog flags). Nothing
// below touches [`crate::sim::World`] or `SimUnit`; every translation
// is a free function over values, slices and caller-supplied closures. Calls
// that leave the window enter as arguments or closures, and no helper outside
// this file is assumed to exist.
//
// ## Recovered layouts
//
// | location | layout |
// |----------|--------|
// | settings group record | `0x20` bytes: `+0x00` sixteen-byte key descriptor (inline short string, compared by length then bytes), `+0x10` sixteen-byte value payload |
// | settings owner | `+0x00` vtable, `+0x04` payload pointer (null = absent), `+0x08` pending-upload byte, `+0x0c` mode dword, `+0x10` group vector `{count, capacity, data}`, `+0x98` player group, `+0xa0`/`+0xac` dirty bytes, `+0xa4` mission group |
// | tagged value | slot at `+0x14`: bit 0 set = heap pointer in `word & !1`; `word == 1` = empty; after release the tag is `1` |
// | tree node | `0x18` bytes: `+0x00` parent, `+0x04` left, `+0x08` right, `+0x0c` colour byte, `+0x10` key, `+0x14` tagged value |
// | small string | `+0x00` buffer pointer, `+0x04` size, `+0x08` capacity word; low 31 bits are the capacity and `0x8000_000f` means inline capacity `15` |
// | mission state | `fe5944` dismissed, `fe5948` audio armed, `fe594c` end sequence active, `11cdd50` UI state |
// | end dialog flags | enable bytes sent to dialog items `-0x14`, `-0x15`, `-0x17`, `-0x18` |
//
// The `fe59xx`/`11cdd50`/`119f7bc` names in this file abbreviate image globals;
// the low three bytes of each name are the exact global offset.
//
// ## Skipped rows
//
// * `FUN_0066c0b0` - two-instruction forwarder that adds `0x70` to `this` and
//   jumps to the out-of-window destructor `669ce0`; no own semantics.
// * `FUN_0066c0c0` - the same forwarder with a `0x4c` adjustment.
// * `FUN_0066c160` - obfuscated body: overlapping instructions, two `rdtsc`
//   probes, removed unreachable blocks and non-returning tails; the surviving
//   control flow is not a recoverable contract.
// * `FUN_0066c960` - settings upload path (void): formats the settings
//   document and posts it through opaque UI/network calls, with the same kind
//   of unreachable junk block in the middle; no value survives.
// * `FUN_0066cef9` - eight bytes that end in `halt_baddata` on a truncated
//   instruction; not a callable routine.
// * `FUN_0066cf26` - SEH unwind stub: restores `ExceptionList` and resumes at
//   a label outside the body; no arguments and no return value.
// * `FUN_0066cf37` - seven-byte tail jump to the security-cookie check; a
//   thunk, not a routine.
// * `FUN_0066d0c0` - red-black-tree erase range: 463 bytes of inlined rotations
//   and colour fixup over a node whose value field is only partially visible;
//   it has no isolated value contract and the inlined tail call has no
//   recovered arguments.
// * `FUN_0066d92d` - `int3` (`swi(3)`) crash stub.

// ---------------------------------------------------------------------------
// Settings-group table (`+0x10` vector of 0x20-byte records)
// ---------------------------------------------------------------------------

/// Byte stride of one settings-group record.
pub const W13A09_GROUP_SLOT_BYTES: usize = 0x20;

/// Offset of the sixteen-byte value payload inside a record.
pub const W13A09_GROUP_VALUE_OFFSET: usize = 0x10;

/// Number of value bytes copied by the set/erase routines.
pub const W13A09_GROUP_VALUE_BYTES: usize = 0x10;

/// Value word cleared in the caller's payload after an overwrite.
pub const W13A09_GROUP_VALUE_TAG_OFFSET: usize = 0x0e;

/// One `0x20`-byte settings record.
///
/// The retail record stores the key as an inline short-string descriptor in
/// its first sixteen bytes.  The lookup helper compares the descriptor length
/// and then the bytes, so this model keeps the key as a byte vector and the
/// value as the exact sixteen-byte payload at `+0x10`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W13a09GroupEntry {
    /// Group name, compared by exact length and bytes.
    pub key: Vec<u8>,
    /// Sixteen-byte value payload at `+0x10`.
    pub value: [u8; W13A09_GROUP_VALUE_BYTES],
}

/// The two settings collections touched by the routines below.
///
/// `entries` is the vector at owner `+0x10` (count/`capacity`/data); `dirty`
/// holds the owner bytes at `+0xa0` and `+0xac`, which every set raises to `1`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W13a09GroupState {
    /// Records of the `+0x10` vector, in storage order.
    pub entries: Vec<W13a09GroupEntry>,
    /// Owner dirty bytes `+0xa0` and `+0xac`.
    pub dirty: [u8; 2],
}

/// `0x0066c0d0`: remove the record selected by `key` from the `0x20`-stride
/// table.
///
/// The routine builds the sixteen-byte string descriptor for `key`, asks the
/// out-of-window lookup for the first matching record, and - when the returned
/// pointer is not the end pointer - shifts the following records left by one
/// `0x20`-byte slot and decrements the count.  A miss leaves the table
/// untouched.  The returned flag reports whether a slot was removed; the
/// retail body is `void`, so the flag is the recovered branch result.
pub fn w13a09_remove_group(state: &mut W13a09GroupState, key: &[u8]) -> bool {
    let Some(index) = w13a09_find_group(&state.entries, key) else {
        return false;
    };
    state.entries.remove(index);
    true
}

/// `0x0066c580`: set the sixteen-byte value of the group named `key`.
///
/// The body first emits the `SetGroup {0}` log record, then searches the
/// `0x20`-stride table.  On a hit it copies the caller's sixteen bytes into
/// record `+0x10` and clears the two-byte tag word at caller offset `0x0e`;
/// on a miss it inserts a new record with the same key and value.  Both paths
/// finish by raising owner bytes `+0xa0` and `+0xac` to `1`.
///
/// Returns `true` when a record was inserted and `false` when an existing one
/// was overwritten; the retail body is `void`.
pub fn w13a09_set_group(
    state: &mut W13a09GroupState,
    key: &[u8],
    value: &mut [u8; W13A09_GROUP_VALUE_BYTES],
) -> bool {
    let inserted = match w13a09_find_group(&state.entries, key) {
        Some(index) => {
            state.entries[index].value = *value;
            value[W13A09_GROUP_VALUE_TAG_OFFSET..].fill(0);
            false
        }
        None => {
            state.entries.push(W13a09GroupEntry {
                key: key.to_vec(),
                value: *value,
            });
            true
        }
    };
    state.dirty = [1, 1];
    inserted
}

/// First record whose key equals `key`, or `None`.
///
/// This mirrors the linear descriptor comparison of the out-of-window lookup:
/// exact length first, then the bytes, first match wins.
fn w13a09_find_group(entries: &[W13a09GroupEntry], key: &[u8]) -> Option<usize> {
    entries.iter().position(|entry| entry.key == key)
}

// ---------------------------------------------------------------------------
// Save/commit path
// ---------------------------------------------------------------------------

/// `0x0066c450`: commit the player group and then the mission group.
///
/// The body calls the flush of owner `+0x98` without looking at a result, then
/// calls the flush of owner `+0xa4`.  Only when the second flush returns true
/// does it format the settings document (the retail body also builds a
/// thirteen-byte `SaveAndUpload` tag string here) and run the upload path on
/// the mission group.  The returned flag is the second flush's result.
pub fn w13a09_commit_groups<FP, FM, FF, UP>(
    flush_player: FP,
    flush_mission: FM,
    format_document: FF,
    upload: UP,
) -> bool
where
    FP: FnOnce(),
    FM: FnOnce() -> bool,
    FF: FnOnce(),
    UP: FnOnce(),
{
    flush_player();
    if !flush_mission() {
        return false;
    }
    format_document();
    upload();
    true
}

/// `0x0066cb80`: flush one settings group and clear its pending byte.
///
/// The retail body runs the virtual prepare predicate in vtable slot at `+0x8`
/// and returns false immediately when it fails.  It then performs the guarded
/// lazy construction of the settings singleton (thread-safe static
/// initialisation: construct only while `static_ready` is clear), reads the
/// group value through vtable slot `0`, writes it, and on success stores `0` in
/// owner byte `+0x8`.  Returns the writer's result.
pub fn w13a09_flush_group<PR, ST, RD, WR, T>(
    prepare: PR,
    static_ready: &mut bool,
    construct_static: ST,
    read_value: RD,
    write: WR,
    pending: &mut u8,
) -> bool
where
    PR: FnOnce() -> bool,
    ST: FnOnce(),
    RD: FnOnce() -> T,
    WR: FnOnce(T) -> bool,
{
    if !prepare() {
        return false;
    }
    if !*static_ready {
        construct_static();
        *static_ready = true;
    }
    let ok = write(read_value());
    if ok {
        *pending = 0;
    }
    ok
}

/// Number of leading owner bytes before the group vector.
pub const W13A09_GROUP_VECTOR_OFFSET: usize = 0x10;

/// Owner byte that records a pending upload.
pub const W13A09_PENDING_OFFSET: usize = 0x08;

/// Owner dword that gates the pending probe.
pub const W13A09_MODE_OFFSET: usize = 0x0c;

/// `0x0066c830`: the pending byte of a settings view, or `0` when the view has
/// no payload.
///
/// `record` is the twelve-byte view the routine reads: a dword at `+0x04`
/// (null means absent) and the byte at `+0x08`.
pub fn w13a09_slot_flag(record: &[u8; 12]) -> u8 {
    let present = u32::from_le_bytes([record[4], record[5], record[6], record[7]]) != 0;
    if present { record[8] } else { 0 }
}

/// `0x0066c840`: decide whether the pending-upload probe may run.
///
/// The recovered control flow is four gates in sequence:
///
/// 1. the payload dword at `+0x04` is non-null;
/// 2. the mode dword at `+0x0c` is neither `0` nor `2`;
/// 3. the caller-supplied probe (`probe`, the boolean result of the
///    out-of-window predicate) is true;
/// 4. the pending byte at `+0x08` is non-zero.
///
/// Any failing gate yields false; only the all-true path returns true.
pub fn w13a09_should_upload(has_payload: bool, mode: u32, probe: bool, pending: u8) -> bool {
    has_payload && mode != 0 && mode != 2 && probe && pending != 0
}

// ---------------------------------------------------------------------------
// Writers and the settings error
// ---------------------------------------------------------------------------

/// The message installed by the settings catch handler.
pub const W13A09_SETTINGS_WRITE_FAILURE: &str = "Unknown user settings write failure";

/// The error raised when a settings write fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a09SettingsError {
    /// Fixed diagnostic text.
    pub message: &'static str,
}

/// `0x0066ce10`: the catch handler of the settings writer.
///
/// The handler receives the frame's message buffer and constructs the
/// exception that is thrown out of the writer.  Its only recovered payload is
/// the fixed diagnostic text, returned here as a value.
pub fn w13a09_settings_write_failure() -> W13a09SettingsError {
    W13a09SettingsError {
        message: W13A09_SETTINGS_WRITE_FAILURE,
    }
}

/// `0x0066cd90`: apply one value to the settings singleton.
///
/// The body copies the group name at owner `+0x10` into a temporary, then
/// passes the temporary and the caller's value to the writer.  The call is
/// wrapped by the catch handler at `0x0066ce10`, so a failing write surfaces
/// as [`W13a09SettingsError`].  On success the routine returns `1`, modelled
/// as `Ok(true)`.
pub fn w13a09_apply_setting<AP>(apply: AP) -> Result<bool, W13a09SettingsError>
where
    AP: FnOnce() -> Result<(), W13a09SettingsError>,
{
    apply()?;
    Ok(true)
}

// ---------------------------------------------------------------------------
// Tagged values and tree teardown
// ---------------------------------------------------------------------------

/// Tag word of an empty tagged slot.
pub const W13A09_TAGGED_EMPTY: u32 = 1;

/// Where a tagged slot keeps its object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a09TaggedPayload {
    /// Inline object stored in the slot itself.
    Inline,
    /// Heap object at this address (`word & !1`).
    Heap(u32),
}

/// `0x0066cf60`: destroy the object in one tagged slot.
///
/// The slot word is read from `+0x14`.  An odd word holds a heap pointer in
/// `word & !1`; the empty value `1` has a null pointer and is skipped.  An
/// even word means the object lives inline in the slot.  The routine invokes
/// the object's destructor, frees the heap block when there is one, and writes
/// the empty tag `1` back into the slot.  It returns that new tag.
///
/// `destroy` receives [`W13a09TaggedPayload::Inline`] or
/// [`W13a09TaggedPayload::Heap`]; `free` runs only on the heap path.
pub fn w13a09_release_tagged<D, F>(word: u32, destroy: D, free: F) -> u32
where
    D: FnOnce(W13a09TaggedPayload),
    F: FnOnce(u32),
{
    if word & 1 != 0 {
        let address = word & !1;
        if address == 0 {
            return W13A09_TAGGED_EMPTY;
        }
        destroy(W13a09TaggedPayload::Heap(address));
        free(address);
    } else {
        destroy(W13a09TaggedPayload::Inline);
    }
    W13A09_TAGGED_EMPTY
}

/// One `0x18`-byte node of the settings tree.
///
/// The teardown routine only reads the left child at `+0x04`, the right child
/// at `+0x08` and the tagged value at `+0x14`; the parent and colour fields are
/// included so the recovered record size is explicit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a09TreeNode {
    /// Parent link at `+0x00`.
    pub parent: Option<usize>,
    /// Left child at `+0x04`.
    pub left: Option<usize>,
    /// Right child at `+0x08`.
    pub right: Option<usize>,
    /// Colour byte at `+0x0c`.
    pub colour: u8,
    /// Key word at `+0x10`.
    pub key: u32,
    /// Tagged value word at `+0x14`.
    pub payload: u32,
}

/// `0x0066d000`: destroy a whole tree of `0x18`-byte nodes.
///
/// The recursive retail body destroys the left child (`+0x04`), releases the
/// node's tagged value (`+0x14`), frees the node and then continues with the
/// right child (`+0x08`).  This is the in-order sequence left, node, right.
/// The translation walks the same order with an explicit stack and returns the
/// number of nodes visited.
///
/// `destroy` and `free` are forwarded to [`w13a09_release_tagged`]; the empty
/// tag `1` performs neither action.  An out-of-range child link ends that
/// branch instead of panicking.
pub fn w13a09_destroy_tree<D, F>(
    nodes: &[W13a09TreeNode],
    root: Option<usize>,
    mut destroy: D,
    mut free: F,
) -> usize
where
    D: FnMut(W13a09TaggedPayload),
    F: FnMut(u32),
{
    let mut stack: Vec<usize> = Vec::new();
    let mut cursor = root;
    let mut visited = 0usize;

    while cursor.is_some() || !stack.is_empty() {
        while let Some(index) = cursor {
            let Some(node) = nodes.get(index) else {
                break;
            };
            stack.push(index);
            cursor = node.left;
        }

        let Some(index) = stack.pop() else {
            break;
        };
        let node = nodes[index];
        w13a09_release_tagged(node.payload, &mut destroy, &mut free);
        visited += 1;
        cursor = node.right;
    }

    visited
}

// ---------------------------------------------------------------------------
// Small-string copy
// ---------------------------------------------------------------------------

/// Capacity word written into a freshly constructed small string.
pub const W13A09_SSO_CAPACITY_WORD: u32 = 0x8000_000f;

/// Inline capacity encoded by [`W13A09_SSO_CAPACITY_WORD`].
pub const W13A09_SSO_CAPACITY: usize = 15;

/// A copied small string.
///
/// The retail object stores the logical length separately and keeps a
/// terminating zero byte after the payload, so the model exposes both.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W13a09SmallString {
    /// Payload bytes followed by one zero byte.
    pub buffer: Vec<u8>,
    /// Logical length, excluding the terminator.
    pub len: u32,
}

/// `0x0066d290`: copy a `(pointer, length)` pair into a small string.
///
/// The body reserves `length` bytes through the string reserve helper, copies
/// the source bytes, writes the terminating zero at `buffer[length]` and
/// stores `length` as the size.  Before the copy the object is initialised to
/// size `0`, inline buffer, capacity word [`W13A09_SSO_CAPACITY_WORD`].  The
/// returned value carries the same NUL-terminated buffer and logical size.
pub fn w13a09_copy_small_string(src: &[u8]) -> W13a09SmallString {
    let mut buffer = Vec::with_capacity(src.len() + 1);
    buffer.extend_from_slice(src);
    buffer.push(0);
    W13a09SmallString {
        buffer,
        len: src.len() as u32,
    }
}

/// Whether a payload of `len` bytes leaves the inline buffer.
///
/// Derived from the capacity word written by `0x0066d290`: the reserve helper
/// grows the buffer when the low 31 bits of the word (`15` here) are smaller
/// than the requested size.
pub fn w13a09_string_uses_heap(len: usize) -> bool {
    len > W13A09_SSO_CAPACITY
}

// ---------------------------------------------------------------------------
// Mission-end state machine
// ---------------------------------------------------------------------------

/// UI state that asks for the replay/continue warning branch.
pub const W13A09_UI_STATE_CONTINUE: i32 = 1;

/// UI state that asks for the replay notification branch.
pub const W13A09_UI_STATE_REPLAY: i32 = -2;

/// Notification code sent in the replay branch.
pub const W13A09_REPLAY_NOTIFY_CODE: i32 = 7;

/// Event code handled by the mission-end dispatcher.
pub const W13A09_MISSION_EVENT_CODE: i16 = 0x0e;

/// The recovered mission-end globals.
///
/// `active` is `fe594c` (the end sequence is running), `dismissed` is `fe5944`
/// (the end screen has been acknowledged), `armed` is `fe5948` (the one-shot
/// audio hook has fired) and `ui_state` is the `11cdd50` selector.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a09MissionFlags {
    /// `fe594c`: end sequence active.
    pub active: bool,
    /// `fe5944`: end sequence acknowledged.
    pub dismissed: bool,
    /// `fe5948`: one-shot audio hook armed.
    pub armed: bool,
    /// `11cdd50`: UI selector.
    pub ui_state: i32,
}

/// `0x0066d2e0`: clear the acknowledged and audio-armed flags.
///
/// Only `fe5944` and `fe5948` are written; the active flag and the UI selector
/// are deliberately left alone by the retail body.
pub fn w13a09_reset_mission_flags(state: &mut W13a09MissionFlags) {
    state.dismissed = false;
    state.armed = false;
}

/// `0x0066d300`: arm the one-shot mission-end audio hook.
///
/// The body calls the out-of-window audio query, and only when it reports
/// availability and `fe5948` is still zero does it set `fe5948` and run the
/// audio start helper.  Returns whether the hook was armed by this call.
pub fn w13a09_arm_music_once(available: bool, state: &mut W13a09MissionFlags) -> bool {
    if !available || state.armed {
        return false;
    }
    state.armed = true;
    true
}

/// Which mission-end document the begin routine installs.
///
/// The two twins in this window differ only in the document key pushed into
/// the UI registry (`wmission` for the win path and `lmission` for the lose
/// path) and in the continuation callback they register.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a09MissionUi {
    /// Win document (`wmission`).
    Win,
    /// Lose document (`lmission`).
    Lose,
}

/// The effects that precede the begin routine's non-returning tail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a09MissionEndBegin {
    /// Whether the one-shot audio hook was armed by this call.
    pub play_sound: bool,
    /// Document key to install.
    pub ui: W13a09MissionUi,
    /// Table index read at `+0xf2` of the UI table.
    pub ui_index: u8,
}

/// Shared body of the two mission-end begin routines.
fn w13a09_begin_mission_end_impl(
    state: &mut W13a09MissionFlags,
    audio_ready: bool,
    ui_index: u8,
    ui: W13a09MissionUi,
) -> Option<W13a09MissionEndBegin> {
    if state.active || state.dismissed {
        return None;
    }
    let play_sound = audio_ready && !state.armed;
    if play_sound {
        state.armed = true;
    }
    state.active = true;
    state.ui_state = 0;
    Some(W13a09MissionEndBegin {
        play_sound,
        ui,
        ui_index,
    })
}

/// `0x0066d330`: begin the lose-mission end sequence.
///
/// The body returns immediately when `fe594c` or `fe5944` is set; otherwise it
/// may arm the one-shot audio hook, raises `fe594c`, clears the UI selector,
/// looks up the table byte at `+0xf2`, and installs the `lmission` document
/// before taking a non-returning tail.  The returned description covers the
/// observable state changes before that tail; `None` is the guard path.
pub fn w13a09_begin_mission_end_lose(
    state: &mut W13a09MissionFlags,
    audio_ready: bool,
    ui_index: u8,
) -> Option<W13a09MissionEndBegin> {
    w13a09_begin_mission_end_impl(state, audio_ready, ui_index, W13a09MissionUi::Lose)
}

/// `0x0066d520`: begin the win-mission end sequence.
///
/// Same recovered control flow as the lose twin, with the `wmission` document
/// and the win continuation callback.  See
/// [`w13a09_begin_mission_end_lose`] for the guard and flag semantics.
pub fn w13a09_begin_mission_end_win(
    state: &mut W13a09MissionFlags,
    audio_ready: bool,
    ui_index: u8,
) -> Option<W13a09MissionEndBegin> {
    w13a09_begin_mission_end_impl(state, audio_ready, ui_index, W13a09MissionUi::Win)
}

/// Side effects requested by the mission-end event dispatcher.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a09MissionEventAction {
    /// Show the victory/result dialog (`66d950`).
    pub show_victory: bool,
    /// Stop the victory audio (`71edb0`).
    pub stop_victory: bool,
    /// Notification code to send, when the replay branch is taken.
    pub notify_code: Option<i32>,
    /// Show the continue warning (`strCONTINUEWARNING`).
    pub show_continue_warning: bool,
    /// Mark the end sequence acknowledged (`fe5944 = 1`).
    pub dismiss: bool,
}

/// `0x0066de30`: dispatch one mission-end UI event.
///
/// Events whose code word at `+0x10` is not `0x0e` fall straight through to
/// the event-finish call.  For the handled code the kind dword at `+0x00`
/// selects:
///
/// * `0` - show the victory/result dialog;
/// * `1` - stop the victory audio and clear `fe594c`;
/// * `2` - if the UI selector is `-2`, send notification `7`; if it is `1`,
///   show the continue warning; then set `fe5944`;
/// * any other kind - no branch.
///
/// Every path ends with the same event-finish call, which is left to the
/// caller.
pub fn w13a09_dispatch_mission_event(
    state: &mut W13a09MissionFlags,
    kind: u32,
    code: i16,
) -> W13a09MissionEventAction {
    let mut action = W13a09MissionEventAction::default();
    if code != W13A09_MISSION_EVENT_CODE {
        return action;
    }
    match kind {
        0 => action.show_victory = true,
        1 => {
            action.stop_victory = true;
            state.active = false;
        }
        2 => {
            if state.ui_state == W13A09_UI_STATE_REPLAY {
                action.notify_code = Some(W13A09_REPLAY_NOTIFY_CODE);
            } else if state.ui_state == W13A09_UI_STATE_CONTINUE {
                action.show_continue_warning = true;
            }
            state.dismissed = true;
            action.dismiss = true;
        }
        _ => {}
    }
    action
}

/// Enable bytes written to the four end-screen dialog items.
///
/// The item ids are the values pushed by the retail body: `0xe8`, `0xe9`,
/// `0xeb` and `0xec` (the negative dialog ids `-0x18`, `-0x17`, `-0x15` and
/// `-0x14`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a09DialogFlags {
    /// Byte sent to item `-0x18` (`0xe8`).
    pub item_e8: u8,
    /// Byte sent to item `-0x17` (`0xe9`).
    pub item_e9: u8,
    /// Byte sent to item `-0x15` (`0xeb`).
    pub item_eb: u8,
    /// Byte sent to item `-0x14` (`0xec`).
    pub item_ec: u8,
}

/// `0x0066d950`: compute the four dialog enable bytes of the result screen.
///
/// Inside the larger result-screen routine, the block at `66da97`..`66db45`
/// derives four enable bytes from the selected result, the alternate-UI global
/// (`11a4580`), the debug-overlay gate and the list-availability test
/// (`11b9948 > 0` or its helper).  The translation models exactly that truth
/// table; the surrounding sound/widget calls are presentation only.
///
/// * `debug_overlay` - the debug byte is set and its helper returns true;
/// * `result` - the selected result value;
/// * `alternate_ui` - `11a4580` is non-zero;
/// * `list_available` - the `11b9948`/helper gate is true.
pub fn w13a09_end_dialog_flags(
    debug_overlay: bool,
    result: u32,
    alternate_ui: bool,
    list_available: bool,
) -> W13a09DialogFlags {
    // `a` is the byte for item `-0x15`, `b` for `-0x14`, `d` for `-0x17` and
    // `c` for `-0x18`.
    let mut a = 0u8;
    let mut b = 0u8;
    let mut d = 0u8;

    if debug_overlay {
        a = 1;
    } else if result == 3 && !alternate_ui {
        d = 1;
    } else {
        b = 1;
    }

    let mut c = 0u8;
    if list_available {
        let keep_silent = d == 0 && result != 3 && result != 0;
        if !keep_silent {
            c = 1;
            b = 0;
            a = 0;
        }
        d = 0;
    }

    W13a09DialogFlags {
        item_e8: c,
        item_e9: d,
        item_eb: a,
        item_ec: b,
    }
}

/// Shape descriptor returned by the table-shape accessor.
///
/// The three fields are the recovered constants; the names are descriptive
/// only, because the body does not reveal the consumer's field names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a09TableShape {
    /// First word written at `+0x00` (`0x18`).
    pub record_bytes: u32,
    /// Second word written at `+0x04` (`8`).
    pub count: u32,
    /// Byte written at `+0x08` (`1`).
    pub enabled: u8,
}

/// `0x0066c940`: fill the common three-field table descriptor.
///
/// The routine writes `0x18` at `+0x00`, `8` at `+0x04` and `1` at `+0x08` and
/// leaves the rest of the caller's record untouched.
pub fn w13a09_table_shape() -> W13a09TableShape {
    W13a09TableShape {
        record_bytes: 0x18,
        count: 8,
        enabled: 1,
    }
}

#[cfg(test)]
mod tests_wave13_agent09 {
    use super::*;
    use std::cell::RefCell;

    // -- 0x0066c0d0 --------------------------------------------------------

    fn entry(key: &[u8], seed: u8) -> W13a09GroupEntry {
        W13a09GroupEntry {
            key: key.to_vec(),
            value: [seed; W13A09_GROUP_VALUE_BYTES],
        }
    }

    #[test]
    fn remove_group_shifts_and_reports_the_hit() {
        let mut state = W13a09GroupState {
            entries: vec![entry(b"alpha", 1), entry(b"beta", 2), entry(b"gamma", 3)],
            dirty: [0, 0],
        };

        assert!(w13a09_remove_group(&mut state, b"beta"));
        assert_eq!(state.entries.len(), 2);
        assert_eq!(state.entries[0].key, b"alpha");
        assert_eq!(state.entries[1].key, b"gamma");
        // Removal does not touch the dirty bytes.
        assert_eq!(state.dirty, [0, 0]);

        assert!(!w13a09_remove_group(&mut state, b"beta"));
        assert_eq!(state.entries.len(), 2);
    }

    #[test]
    fn remove_group_compares_the_whole_key() {
        let mut state = W13a09GroupState {
            entries: vec![entry(b"a", 1), entry(b"ab", 2)],
            dirty: [0, 0],
        };

        assert!(!w13a09_remove_group(&mut state, b"abc"));
        assert!(w13a09_remove_group(&mut state, b"a"));
        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.entries[0].key, b"ab");
    }

    // -- 0x0066c580 --------------------------------------------------------

    #[test]
    fn set_group_updates_an_existing_slot_and_clears_the_tag_word() {
        let mut state = W13a09GroupState::default();
        let mut first = [0x11u8; W13A09_GROUP_VALUE_BYTES];
        assert!(w13a09_set_group(&mut state, b"group", &mut first));
        assert_eq!(state.dirty, [1, 1]);
        assert_eq!(state.entries.len(), 1);

        // Reset the dirty bytes so the update path is observable on its own.
        state.dirty = [0, 0];
        let mut second = [0x22u8; W13A09_GROUP_VALUE_BYTES];
        second[W13A09_GROUP_VALUE_TAG_OFFSET] = 0xaa;
        second[W13A09_GROUP_VALUE_TAG_OFFSET + 1] = 0xbb;

        assert!(!w13a09_set_group(&mut state, b"group", &mut second));
        assert_eq!(state.entries.len(), 1);
        let mut expected = [0x22u8; W13A09_GROUP_VALUE_BYTES];
        expected[W13A09_GROUP_VALUE_TAG_OFFSET] = 0xaa;
        expected[W13A09_GROUP_VALUE_TAG_OFFSET + 1] = 0xbb;
        // The copy reads the tag bytes before the caller's word is cleared.
        assert_eq!(state.entries[0].value, expected);
        assert_eq!(second[W13A09_GROUP_VALUE_TAG_OFFSET], 0);
        assert_eq!(second[W13A09_GROUP_VALUE_TAG_OFFSET + 1], 0);
        assert_eq!(state.dirty, [1, 1]);
    }

    #[test]
    fn set_group_appends_on_a_miss_and_keeps_record_order() {
        let mut state = W13a09GroupState::default();
        let mut value = [0u8; W13A09_GROUP_VALUE_BYTES];
        value[0] = 1;
        assert!(w13a09_set_group(&mut state, b"one", &mut value));
        value[0] = 2;
        assert!(w13a09_set_group(&mut state, b"two", &mut value));
        // A miss does not clear the caller's tag word.
        assert_eq!(value[W13A09_GROUP_VALUE_TAG_OFFSET], 0);

        assert_eq!(state.entries[0].key, b"one");
        assert_eq!(state.entries[0].value[0], 1);
        assert_eq!(state.entries[1].key, b"two");
        assert_eq!(state.entries[1].value[0], 2);
    }

    // -- 0x0066c450 --------------------------------------------------------

    #[test]
    fn commit_stops_before_formatting_when_the_second_flush_fails() {
        let log = RefCell::new(Vec::<&str>::new());
        let committed = w13a09_commit_groups(
            || log.borrow_mut().push("player"),
            || {
                log.borrow_mut().push("mission");
                false
            },
            || log.borrow_mut().push("format"),
            || log.borrow_mut().push("upload"),
        );

        assert!(!committed);
        assert_eq!(*log.borrow(), ["player", "mission"]);
    }

    #[test]
    fn commit_formats_and_uploads_after_a_successful_second_flush() {
        let log = RefCell::new(Vec::<&str>::new());
        let committed = w13a09_commit_groups(
            || log.borrow_mut().push("player"),
            || {
                log.borrow_mut().push("mission");
                true
            },
            || log.borrow_mut().push("format"),
            || log.borrow_mut().push("upload"),
        );

        assert!(committed);
        assert_eq!(*log.borrow(), ["player", "mission", "format", "upload"]);
    }

    // -- 0x0066c830 / 0x0066c840 ------------------------------------------

    #[test]
    fn recovered_offsets_match_the_disassembly() {
        assert_eq!(W13A09_GROUP_SLOT_BYTES, 0x20);
        assert_eq!(W13A09_GROUP_VALUE_OFFSET, 0x10);
        assert_eq!(W13A09_GROUP_VALUE_BYTES, 0x10);
        assert_eq!(W13A09_GROUP_VALUE_TAG_OFFSET, 0x0e);
        assert_eq!(W13A09_GROUP_VECTOR_OFFSET, 0x10);
        assert_eq!(W13A09_PENDING_OFFSET, 0x08);
        assert_eq!(W13A09_MODE_OFFSET, 0x0c);
        assert_eq!(W13A09_TAGGED_EMPTY, 1);
    }

    #[test]
    fn slot_flag_returns_zero_without_a_payload() {
        let absent = [0u8; 12];
        assert_eq!(w13a09_slot_flag(&absent), 0);

        let mut present = [0u8; 12];
        present[4] = 1;
        present[8] = 0x5a;
        assert_eq!(w13a09_slot_flag(&present), 0x5a);

        present[8] = 0;
        present[7] = 0x80;
        assert_eq!(w13a09_slot_flag(&present), 0);
    }

    #[test]
    fn upload_probe_needs_all_four_gates() {
        assert!(w13a09_should_upload(true, 1, true, 1));
        assert!(!w13a09_should_upload(false, 1, true, 1));
        assert!(!w13a09_should_upload(true, 0, true, 1));
        assert!(!w13a09_should_upload(true, 2, true, 1));
        assert!(!w13a09_should_upload(true, 1, false, 1));
        assert!(!w13a09_should_upload(true, 1, true, 0));
        assert!(w13a09_should_upload(true, 0xffff_ffff, true, 0xff));
    }

    // -- 0x0066cb80 --------------------------------------------------------

    #[test]
    fn flush_group_skips_the_write_when_prepare_fails() {
        let trace = RefCell::new(Vec::<&str>::new());
        let mut static_ready = false;
        let mut pending = 9u8;
        let ok = w13a09_flush_group(
            || {
                trace.borrow_mut().push("prepare");
                false
            },
            &mut static_ready,
            || trace.borrow_mut().push("static"),
            || {
                trace.borrow_mut().push("read");
                1u32
            },
            |_| {
                trace.borrow_mut().push("write");
                true
            },
            &mut pending,
        );

        assert!(!ok);
        assert!(!static_ready);
        assert_eq!(pending, 9);
        assert_eq!(*trace.borrow(), ["prepare"]);
    }

    #[test]
    fn flush_group_constructs_the_static_once_and_clears_pending_on_success() {
        let trace = RefCell::new(Vec::<&str>::new());
        let mut static_ready = false;
        let mut pending = 9u8;
        let ok = w13a09_flush_group(
            || {
                trace.borrow_mut().push("prepare");
                true
            },
            &mut static_ready,
            || trace.borrow_mut().push("static"),
            || {
                trace.borrow_mut().push("read");
                42u32
            },
            |value| {
                trace.borrow_mut().push("write");
                assert_eq!(value, 42);
                false
            },
            &mut pending,
        );

        assert!(!ok);
        assert!(static_ready);
        assert_eq!(pending, 9);
        assert_eq!(*trace.borrow(), ["prepare", "static", "read", "write"]);

        // The second call skips the construction and succeeds.
        let trace = RefCell::new(Vec::<&str>::new());
        let ok = w13a09_flush_group(
            || {
                trace.borrow_mut().push("prepare");
                true
            },
            &mut static_ready,
            || trace.borrow_mut().push("static"),
            || {
                trace.borrow_mut().push("read");
                7u32
            },
            |_| {
                trace.borrow_mut().push("write");
                true
            },
            &mut pending,
        );

        assert!(ok);
        assert!(static_ready);
        assert_eq!(pending, 0);
        assert_eq!(*trace.borrow(), ["prepare", "read", "write"]);
    }

    // -- 0x0066cd90 / 0x0066ce10 ------------------------------------------

    #[test]
    fn apply_setting_maps_success_to_true_and_propagates_the_error() {
        assert_eq!(w13a09_apply_setting(|| Ok(())), Ok(true));

        let error = w13a09_apply_setting(|| Err(w13a09_settings_write_failure()));
        assert_eq!(error, Err(w13a09_settings_write_failure()));
    }

    #[test]
    fn settings_write_failure_carries_the_recovered_message() {
        let error = w13a09_settings_write_failure();
        assert_eq!(error.message, W13A09_SETTINGS_WRITE_FAILURE);
        assert!(error.message.contains("user settings"));
    }

    // -- 0x0066cf60 --------------------------------------------------------

    #[test]
    fn release_tagged_skips_the_empty_word() {
        let calls = RefCell::new(Vec::<String>::new());
        let tag = w13a09_release_tagged(
            W13A09_TAGGED_EMPTY,
            |payload| calls.borrow_mut().push(format!("destroy:{payload:?}")),
            |address| calls.borrow_mut().push(format!("free:{address:x}")),
        );

        assert_eq!(tag, W13A09_TAGGED_EMPTY);
        assert!(calls.borrow().is_empty());
    }

    #[test]
    fn release_tagged_destroys_inline_and_heap_payloads() {
        let calls = RefCell::new(Vec::<String>::new());
        let tag = w13a09_release_tagged(
            0,
            |payload| {
                assert_eq!(payload, W13a09TaggedPayload::Inline);
                calls.borrow_mut().push("destroy:inline".to_owned());
            },
            |_| calls.borrow_mut().push("free:inline".to_owned()),
        );
        assert_eq!(tag, W13A09_TAGGED_EMPTY);
        assert_eq!(*calls.borrow(), ["destroy:inline"]);

        let calls = RefCell::new(Vec::<String>::new());
        let tag = w13a09_release_tagged(
            0x1235,
            |payload| {
                assert_eq!(payload, W13a09TaggedPayload::Heap(0x1234));
                calls.borrow_mut().push("destroy:heap".to_owned());
            },
            |address| calls.borrow_mut().push(format!("free:{address:x}")),
        );
        assert_eq!(tag, W13A09_TAGGED_EMPTY);
        assert_eq!(*calls.borrow(), ["destroy:heap", "free:1234"]);
    }

    // -- 0x0066d000 --------------------------------------------------------

    #[test]
    fn destroy_tree_visits_left_node_right_and_releases_payloads() {
        let nodes = [
            W13a09TreeNode {
                left: Some(1),
                right: Some(2),
                payload: 0,
                ..W13a09TreeNode::default()
            },
            W13a09TreeNode {
                payload: 0x1235,
                ..W13a09TreeNode::default()
            },
            W13a09TreeNode {
                left: Some(3),
                payload: W13A09_TAGGED_EMPTY,
                ..W13a09TreeNode::default()
            },
            W13a09TreeNode {
                payload: 0x21,
                ..W13a09TreeNode::default()
            },
        ];

        let destroyed = RefCell::new(Vec::<String>::new());
        let freed = RefCell::new(Vec::<String>::new());
        let visited = w13a09_destroy_tree(
            &nodes,
            Some(0),
            |payload| {
                destroyed.borrow_mut().push(match payload {
                    W13a09TaggedPayload::Inline => "inline".to_owned(),
                    W13a09TaggedPayload::Heap(address) => format!("heap:{address:x}"),
                });
            },
            |address| freed.borrow_mut().push(format!("{address:x}")),
        );

        assert_eq!(visited, 4);
        assert_eq!(*destroyed.borrow(), ["heap:1234", "inline", "heap:20"]);
        assert_eq!(*freed.borrow(), ["1234", "20"]);
    }

    #[test]
    fn destroy_tree_handles_an_empty_root_and_out_of_range_links() {
        let destroyed = RefCell::new(Vec::<W13a09TaggedPayload>::new());
        let visited = w13a09_destroy_tree(
            &[],
            None,
            |payload| destroyed.borrow_mut().push(payload),
            |_| {},
        );
        assert_eq!(visited, 0);
        assert!(destroyed.borrow().is_empty());

        let nodes = [W13a09TreeNode {
            left: Some(5),
            payload: 0,
            ..W13a09TreeNode::default()
        }];
        let visited = w13a09_destroy_tree(&nodes, Some(0), |_| {}, |_| {});
        assert_eq!(visited, 1);
    }

    // -- 0x0066d290 --------------------------------------------------------

    #[test]
    fn copy_small_string_terminates_and_tracks_the_length() {
        let empty = w13a09_copy_small_string(b"");
        assert_eq!(empty.buffer, [0]);
        assert_eq!(empty.len, 0);

        let short = w13a09_copy_small_string(b"abc");
        assert_eq!(short.buffer, [b'a', b'b', b'c', 0]);
        assert_eq!(short.len, 3);
        assert!(!w13a09_string_uses_heap(short.len as usize));

        assert!(!w13a09_string_uses_heap(W13A09_SSO_CAPACITY));
        assert!(w13a09_string_uses_heap(W13A09_SSO_CAPACITY + 1));
        assert_eq!(W13A09_SSO_CAPACITY_WORD & 0x7fff_ffff, 15);
    }

    // -- 0x0066d2e0 / 0x0066d300 ------------------------------------------

    #[test]
    fn reset_mission_flags_leaves_active_and_ui_state_alone() {
        let mut state = W13a09MissionFlags {
            active: true,
            dismissed: true,
            armed: true,
            ui_state: -2,
        };
        w13a09_reset_mission_flags(&mut state);
        assert_eq!(
            state,
            W13a09MissionFlags {
                active: true,
                dismissed: false,
                armed: false,
                ui_state: -2,
            }
        );
    }

    #[test]
    fn music_hook_arms_only_once() {
        let mut state = W13a09MissionFlags::default();
        assert!(!w13a09_arm_music_once(false, &mut state));
        assert!(!state.armed);
        assert!(w13a09_arm_music_once(true, &mut state));
        assert!(state.armed);
        assert!(!w13a09_arm_music_once(true, &mut state));
        assert!(state.armed);
    }

    // -- 0x0066d330 / 0x0066d520 ------------------------------------------

    #[test]
    fn lose_begin_honours_the_guard_and_audio_gate() {
        let mut state = W13a09MissionFlags::default();
        let begin = w13a09_begin_mission_end_lose(&mut state, true, 7).unwrap();
        assert_eq!(
            begin,
            W13a09MissionEndBegin {
                play_sound: true,
                ui: W13a09MissionUi::Lose,
                ui_index: 7,
            }
        );
        assert!(state.active);
        assert!(state.armed);
        assert_eq!(state.ui_state, 0);
        assert!(!state.dismissed);

        let mut blocked = W13a09MissionFlags {
            active: true,
            ..W13a09MissionFlags::default()
        };
        assert_eq!(w13a09_begin_mission_end_lose(&mut blocked, true, 1), None);
        assert!(!blocked.armed);

        let mut blocked = W13a09MissionFlags {
            dismissed: true,
            ..W13a09MissionFlags::default()
        };
        assert_eq!(w13a09_begin_mission_end_lose(&mut blocked, true, 1), None);

        let mut silent = W13a09MissionFlags {
            armed: true,
            ..W13a09MissionFlags::default()
        };
        let begin = w13a09_begin_mission_end_lose(&mut silent, true, 3).unwrap();
        assert!(!begin.play_sound);
        assert!(silent.active);

        let mut no_audio = W13a09MissionFlags::default();
        let begin = w13a09_begin_mission_end_lose(&mut no_audio, false, 4).unwrap();
        assert!(!begin.play_sound);
        assert!(!no_audio.armed);
    }

    #[test]
    fn win_begin_uses_the_win_document_and_the_same_flags() {
        let mut state = W13a09MissionFlags::default();
        let begin = w13a09_begin_mission_end_win(&mut state, true, 0x11).unwrap();
        assert_eq!(begin.ui, W13a09MissionUi::Win);
        assert_eq!(begin.ui_index, 0x11);
        assert!(begin.play_sound);
        assert!(state.active);
        assert_eq!(state.ui_state, 0);
    }

    // -- 0x0066de30 --------------------------------------------------------

    #[test]
    fn dispatcher_ignores_other_event_codes() {
        let mut state = W13a09MissionFlags {
            active: true,
            ui_state: W13A09_UI_STATE_CONTINUE,
            ..W13a09MissionFlags::default()
        };
        let action = w13a09_dispatch_mission_event(&mut state, 2, 0x0d);
        assert_eq!(action, W13a09MissionEventAction::default());
        assert!(!state.dismissed);
        assert!(state.active);
    }

    #[test]
    fn dispatcher_show_victory_keeps_the_flags() {
        let mut state = W13a09MissionFlags {
            active: true,
            ..W13a09MissionFlags::default()
        };
        let action = w13a09_dispatch_mission_event(&mut state, 0, W13A09_MISSION_EVENT_CODE);
        assert!(action.show_victory);
        assert!(!action.stop_victory);
        assert!(state.active);
        assert!(!state.dismissed);
    }

    #[test]
    fn dispatcher_stop_clears_the_active_flag() {
        let mut state = W13a09MissionFlags {
            active: true,
            ..W13a09MissionFlags::default()
        };
        let action = w13a09_dispatch_mission_event(&mut state, 1, W13A09_MISSION_EVENT_CODE);
        assert!(action.stop_victory);
        assert!(!action.show_victory);
        assert!(!state.active);
        assert!(!state.dismissed);
    }

    #[test]
    fn dispatcher_dismisses_and_selects_the_ui_branch() {
        let mut replay = W13a09MissionFlags {
            ui_state: W13A09_UI_STATE_REPLAY,
            ..W13a09MissionFlags::default()
        };
        let action = w13a09_dispatch_mission_event(&mut replay, 2, W13A09_MISSION_EVENT_CODE);
        assert_eq!(action.notify_code, Some(W13A09_REPLAY_NOTIFY_CODE));
        assert!(!action.show_continue_warning);
        assert!(action.dismiss);
        assert!(replay.dismissed);

        let mut warning = W13a09MissionFlags {
            ui_state: W13A09_UI_STATE_CONTINUE,
            ..W13a09MissionFlags::default()
        };
        let action = w13a09_dispatch_mission_event(&mut warning, 2, W13A09_MISSION_EVENT_CODE);
        assert_eq!(action.notify_code, None);
        assert!(action.show_continue_warning);
        assert!(warning.dismissed);

        let mut plain = W13a09MissionFlags {
            ui_state: 4,
            ..W13a09MissionFlags::default()
        };
        let action = w13a09_dispatch_mission_event(&mut plain, 2, W13A09_MISSION_EVENT_CODE);
        assert_eq!(action.notify_code, None);
        assert!(!action.show_continue_warning);
        assert!(plain.dismissed);

        // An unhandled kind does not dismiss anything.
        let mut other = W13a09MissionFlags::default();
        let action = w13a09_dispatch_mission_event(&mut other, 7, W13A09_MISSION_EVENT_CODE);
        assert_eq!(action, W13a09MissionEventAction::default());
        assert!(!other.dismissed);
    }

    // -- 0x0066c940 / 0x0066d950 ------------------------------------------

    #[test]
    fn table_shape_returns_the_recovered_constants() {
        assert_eq!(
            w13a09_table_shape(),
            W13a09TableShape {
                record_bytes: 0x18,
                count: 8,
                enabled: 1,
            }
        );
    }

    #[test]
    fn end_dialog_flags_follow_the_recovered_truth_table() {
        // No debug overlay and no list gate: the plain result enables item ec.
        assert_eq!(
            w13a09_end_dialog_flags(false, 5, false, false),
            W13a09DialogFlags {
                item_e8: 0,
                item_e9: 0,
                item_eb: 0,
                item_ec: 1,
            }
        );

        // Result 3 with the alternate UI absent enables item e9 instead.
        assert_eq!(
            w13a09_end_dialog_flags(false, 3, false, false),
            W13a09DialogFlags {
                item_e8: 0,
                item_e9: 1,
                item_eb: 0,
                item_ec: 0,
            }
        );

        // The alternate UI removes the result-3 exception.
        assert_eq!(
            w13a09_end_dialog_flags(false, 3, true, false),
            W13a09DialogFlags {
                item_e8: 0,
                item_e9: 0,
                item_eb: 0,
                item_ec: 1,
            }
        );

        // The debug overlay starts with item eb enabled.
        assert_eq!(
            w13a09_end_dialog_flags(true, 7, false, false),
            W13a09DialogFlags {
                item_e8: 0,
                item_e9: 0,
                item_eb: 1,
                item_ec: 0,
            }
        );

        // With the list gate open, a result outside {0, 3} keeps item eb and
        // leaves item e8 clear.
        assert_eq!(
            w13a09_end_dialog_flags(true, 7, false, true),
            W13a09DialogFlags {
                item_e8: 0,
                item_e9: 0,
                item_eb: 1,
                item_ec: 0,
            }
        );

        // Result 3 with the list gate open enables item e8 and clears the rest.
        assert_eq!(
            w13a09_end_dialog_flags(true, 3, false, true),
            W13a09DialogFlags {
                item_e8: 1,
                item_e9: 0,
                item_eb: 0,
                item_ec: 0,
            }
        );

        // Result 0 takes the same reset branch.
        assert_eq!(
            w13a09_end_dialog_flags(false, 0, false, true),
            W13a09DialogFlags {
                item_e8: 1,
                item_e9: 0,
                item_eb: 0,
                item_ec: 0,
            }
        );

        // The result-3 branch sets item e9 before the gate, then the gate opens
        // and resets it while enabling item e8.
        assert_eq!(
            w13a09_end_dialog_flags(false, 3, false, true),
            W13a09DialogFlags {
                item_e8: 1,
                item_e9: 0,
                item_eb: 0,
                item_ec: 0,
            }
        );
    }
}
