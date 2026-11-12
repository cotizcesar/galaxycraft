#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent87.rs
// ===========================================================================
// Translation batch 87.  Only this file is edited by the batch.
//
// Batch 87 owns `0x00640000..=0x00641fff`.  The ledger lists 24 rows in that
// window: 23 `decompiled_uncited` and one (`FUN_00640b20`) already `translated`
// with `src/aiscript.rs` as its evidence (that module's header cites the
// interpreter at `0x7a1e0b20`).  20 of the 23 uncited rows are reconstructed
// below; the three rows that are only re-decompiled interior entries of
// `FUN_00640b20` are listed at the end of this file with their reason.
//
// The window is the lifetime plumbing of the AI-script interpreter at
// `FUN_00640b20`: the static 100-record script pool, the two loaded
// `aiscript.bin` / `bwscript.bin` images, the active/free doubly linked
// lists, the per-tick delay sweep, the owner sweep, the script-id dispatch
// table, and the operand/cost queries the interpreter calls from its switch.
//
// Recovered globals (all absolute addresses; `pool` is `0x00fc8710`):
//
// | address      | role in the recovered bodies                                  |
// |--------------|---------------------------------------------------------------|
// | `0x00fc8700` | mapped `aiscript.bin` base                                     |
// | `0x00fc8704` | mapped `aiscript.bin` length (rounded up, see `0x00640620`)    |
// | `0x00fc8708` | mapped `bwscript.bin` base                                     |
// | `0x00fc870c` | mapped `bwscript.bin` length                                   |
// | `0x00fc8710` | first of 100 pool records, `0x34` bytes each (`0x1450` bytes)  |
// | `0x00fc86dc` | index origin: `pool - 0x34`, i.e. saved index 1 -> record 0     |
// | `0x00fc9b60` | free-list head (pool `+0x1450`)                                |
// | `0x00fc9b64` | active-list owner word (equal to the pool base)                |
// | `0x00fc9b68` | active-list head                                              |
// | `0x00f056b8` | 5-entry script-id word table `"EnBkEnTrExTrDWHePB3F"`          |
// | `0x00f4e578` | security cookie used by the `0x1454`-byte frame prologue       |
//
// Recovered record layout (13 dwords, byte offsets from the record base):
//
// | offset | role                                                        |
// |--------|-------------------------------------------------------------|
// | `+0x00`| next link (absolute address, 0 = end)                       |
// | `+0x04`| previous link (absolute address, 0 = end)                   |
// | `+0x08`| interpreter read cursor                                     |
// | `+0x0c`| delay counter decremented by `0x006408b0`                   |
// | `+0x10`| player word handed to `FUN_00640b20`                          |
// | `+0x14`| script operand block (two dwords)                           |
// | `+0x1c`| interpreter scratch                                         |
// | `+0x20`| interpreter scratch                                         |
// | `+0x24`| interpreter scratch                                         |
// | `+0x28`| interpreter scratch                                         |
// | `+0x2c`| owner word compared by `0x00640830`                         |
// | `+0x30`| flags; bit 2 (`0x4`) marks a record that is unlinked by     |
// |         | `0x006408b0` without running it                             |
//
// Two conventions used by the rows below:
//
// * a *saved* pool stores the two link words as **1-based indices**: a zero
//   word means "end of list", and `n` means the record at
//   `0x00fc86dc + n * 0x34` (so `n = 1` is the first record).  `0x006401a6`
//   turns indices into addresses and `0x006403f0` turns them back;
// * helper results that the shipped code receives from outside this window
//   (allocator handles, script-table file loads, the interpreter's
//   `FUN_00695be0`/`FUN_00695c10` cost tables) are explicit parameters or
//   model scaffolding here, never cited as implemented Rust functions.
//
// Address to function:
//
// * `0x00640010` [`a87_fill_index_table`]
// * `0x00640020` [`A87ScriptRuntime::a87_release_record`]
// * `0x00640080` [`a87_check_no_pending_object`]
// * `0x006400a7` [`a87_selector_matches`]
// * `0x006401a6` [`a87_restore_script_pool`]
// * `0x00640320` [`a87_push_script_exception_frame`]
// * `0x00640348` [`a87_script_frame_bytes`]
// * `0x00640390` [`A87ScriptRuntime::a87_clear_chain`]
// * `0x006403f0` [`a87_save_script_pool`]
// * `0x006405c0` [`a87_unmap_script_image`]
// * `0x00640620` [`a87_script_image_size`] / [`A87ScriptRuntime::a87_init`] /
//   [`A87MappedImage::a87_load`]
// * `0x006406b8` [`a87_rotate_and_resize`]
// * `0x00640830` [`A87ScriptRuntime::a87_kill_owned`]
// * `0x006408b0` [`A87ScriptRuntime::a87_tick`]
// * `0x00640940` [`a87_dispatch_script`] / [`a87_script_id_accepted`]
// * `0x00640999` [`a87_panel_slot`] / [`a87_panel_state`]
// * `0x006409d3` [`a87_special_script_matches`]
// * `0x00640a90` [`a87_script_cost`]
// * `0x00640be8` [`a87_interpreter_unwind`] (interior epilogue entry)
// * `0x00640e25` [`a87_operand_word_advance`] (interior operand-guard entry)

/// `0x00fc8710`: first record of the static script pool.
pub const A87_SCRIPT_POOL_BASE: u32 = 0x00fc_8710;

/// `0x00fc8710 + 0x1450`: the free-list head word.
pub const A87_SCRIPT_FREE_LIST: u32 = 0x00fc_9b60;

/// `0x00fc9b64`: the active-list owner word (holds the pool base).
pub const A87_SCRIPT_LIST_OWNER: u32 = 0x00fc_9b64;

/// `0x00fc9b68`: the active-list head word.
pub const A87_SCRIPT_ACTIVE_HEAD: u32 = 0x00fc_9b68;

/// `0x00fc8700`: mapped `aiscript.bin` base word.
pub const A87_AISCRIPT_HANDLE: u32 = 0x00fc_8700;

/// `0x00fc8704`: mapped `aiscript.bin` length word.
pub const A87_AISCRIPT_SIZE: u32 = 0x00fc_8704;

/// `0x00fc8708`: mapped `bwscript.bin` base word.
pub const A87_BWSCRIPT_HANDLE: u32 = 0x00fc_8708;

/// `0x00fc870c`: mapped `bwscript.bin` length word.
pub const A87_BWSCRIPT_SIZE: u32 = 0x00fc_870c;

/// Size of one pool record: 13 dwords.
pub const A87_SCRIPT_RECORD_DWORDS: usize = 13;

/// Byte size of one pool record (`0x34`), used by both index conversions.
pub const A87_SCRIPT_RECORD_BYTES: u32 = 0x34;

/// Number of records in the pool: `0x1450 / 0x34`.
pub const A87_SCRIPT_POOL_RECORDS: usize = 100;

/// Total pool byte size (`0x1450`), which is also the offset of the free-list
/// head inside the pool block.
pub const A87_SCRIPT_POOL_BYTES: u32 = 0x1450;

/// `pool - 0x34`: the origin of the 1-based saved-index formula
/// `0x00fc86dc + index * 0x34`.
pub const A87_SCRIPT_INDEX_ORIGIN: u32 = A87_SCRIPT_POOL_BASE - A87_SCRIPT_RECORD_BYTES;

/// `0x00640010` writes this many identity bytes (`0..=0x1b`).
pub const A87_INDEX_TABLE_BYTES: usize = 0x1c;

/// `0x00640620` adds this much headroom before aligning a script image.
pub const A87_SCRIPT_IMAGE_HEADROOM: u32 = 0x6000;

/// Alignment mask applied to the script image length (`and 0xfffffffc`).
pub const A87_SCRIPT_IMAGE_ALIGN_MASK: u32 = 0xffff_fffc;

/// Dword index of the `next` link inside a record (byte `+0x00`).
pub const A87_REC_NEXT: usize = 0;

/// Dword index of the `previous` link inside a record (byte `+0x04`).
pub const A87_REC_PREV: usize = 1;

/// Dword index of the interpreter read cursor (byte `+0x08`).
pub const A87_REC_CURSOR: usize = 2;

/// Dword index of the delay counter (byte `+0x0c`).
pub const A87_REC_DELAY: usize = 3;

/// Dword index of the player word (byte `+0x10`).
pub const A87_REC_PLAYER: usize = 4;

/// Dword index of the owner word compared by `0x00640830` (byte `+0x2c`).
pub const A87_REC_OWNER: usize = 11;

/// Dword index of the flag dword (byte `+0x30`); only the low byte is read.
pub const A87_REC_FLAGS: usize = 12;

/// Flag bit 2: `0x006408b0` unlinks such a record instead of running it.
pub const A87_REC_FLAG_UNLINK: u32 = 4;

/// The delay value the interpreter reloads on its loop/exit path.
pub const A87_SCRIPT_DELAY_RELOAD: u32 = 0x1e;

/// `0x75637352` = `"Rscu"`, the single id `0x006409d3` dispatches.
pub const A87_SPECIAL_SCRIPT_ID: u32 = u32::from_le_bytes(*b"Rscu");

/// Panel record stride used by `0x00640999` and `0x006409d3`.
pub const A87_PANEL_RECORD_BYTES: u32 = 0x24;

/// Panel slot bias: `(index + 0xc) * 0x24`.
pub const A87_PANEL_SLOT_BIAS: u32 = 0x0c;

/// Panel state byte tested by `0x00640999` is at `slot + 8`.
pub const A87_PANEL_STATE_OFFSET: u32 = 8;

/// Panel state value that `0x00640999` accepts.
pub const A87_PANEL_STATE_RUNNING: u8 = 1;

/// Panel kind that `0x006409d3` requires (a byte at `+8` equal to 7).
pub const A87_PANEL_KIND_SPECIAL: u8 = 7;

/// The 5-entry script-id table at `0x00f056b8`, recovered from the string
/// `"EnBkEnTrExTrDWHePB3F"` that the symbol name spells out.
pub const A87_SCRIPT_ID_TABLE: [u32; 5] = [
    u32::from_le_bytes(*b"EnBk"),
    u32::from_le_bytes(*b"EnTr"),
    u32::from_le_bytes(*b"ExTr"),
    u32::from_le_bytes(*b"DWHe"),
    u32::from_le_bytes(*b"PB3F"),
];

/// `0x00640a90` special-cases this cost-table entry (`0x1e`).
pub const A87_COST_TYPE_DELAY: u32 = 0x1e;

/// `0x00640a90` special-cases this cost-table entry (`5`) too.
pub const A87_COST_TYPE_BUILD: u32 = 5;

/// `0x00640320`/`0x00640348`: the interpreter frame size.
pub const A87_SCRIPT_FRAME_BYTES: u32 = 0x1454;

/// `0x00640320`: the exception handler installed for the frame
/// (`&DAT_00cc7258`).
pub const A87_SCRIPT_SEH_HANDLER: u32 = 0x00cc_7258;

/// `0x00f4e578`: the security-cookie word xor-ed with the frame pointer.
pub const A87_SECURITY_COOKIE_ADDRESS: u32 = 0x00f4_e578;

// ---------------------------------------------------------------------------
// The 100-record script pool
// ---------------------------------------------------------------------------

/// One 100-record script pool.  Link words hold absolute addresses, exactly
/// like the shipped initialiser at `0x00640620` (which writes
/// `0x00fc8744 + n * 0x34` into `next` and `0x00fc86dc + n * 0x34` into
/// `previous`); a zero word terminates the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A87ScriptPool {
    pub records: [[u32; A87_SCRIPT_RECORD_DWORDS]; A87_SCRIPT_POOL_RECORDS],
}

impl Default for A87ScriptPool {
    fn default() -> Self {
        Self {
            records: [[0; A87_SCRIPT_RECORD_DWORDS]; A87_SCRIPT_POOL_RECORDS],
        }
    }
}

/// Address of pool record `index` (`0x00fc8710 + index * 0x34`).
pub fn a87_record_address(index: usize) -> u32 {
    A87_SCRIPT_POOL_BASE.wrapping_add(index as u32 * A87_SCRIPT_RECORD_BYTES)
}

/// Pool index of an absolute record address, or `None` for a null/foreign
/// word.
pub fn a87_record_index(address: u32) -> Option<usize> {
    if address < A87_SCRIPT_POOL_BASE {
        return None;
    }
    let offset = address - A87_SCRIPT_POOL_BASE;
    if offset % A87_SCRIPT_RECORD_BYTES != 0 {
        return None;
    }
    let index = (offset / A87_SCRIPT_RECORD_BYTES) as usize;
    (index < A87_SCRIPT_POOL_RECORDS).then_some(index)
}

/// Saved-pool word for an absolute link address.
///
/// `0x006403f0` computes `(address - 0x00fc8710) / 0x34 + 1` on the unsigned
/// dwords and stores `0` for a null link, so a link to record 0 is saved as
/// `1` and a link to record 99 as `100`.
pub fn a87_saved_index(address: u32) -> u32 {
    if address == 0 {
        0
    } else {
        address
            .wrapping_sub(A87_SCRIPT_POOL_BASE)
            .wrapping_div(A87_SCRIPT_RECORD_BYTES)
            .wrapping_add(1)
    }
}

/// Absolute link address for a saved-pool word.
///
/// `0x006401a6` reloads `0x00fc86dc + word * 0x34` for every nonzero word,
/// i.e. the exact inverse of [`a87_saved_index`] for in-range indices.
pub fn a87_restored_address(saved_index: u32) -> u32 {
    if saved_index == 0 {
        0
    } else {
        A87_SCRIPT_INDEX_ORIGIN.wrapping_add(saved_index.wrapping_mul(A87_SCRIPT_RECORD_BYTES))
    }
}

/// A pool image as `0x006403f0` writes it: the 13-dword records with the two
/// link words replaced by 1-based indices, plus the free-list head index at
/// pool offset `0x1450`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A87SavedScriptPool {
    pub records: [[u32; A87_SCRIPT_RECORD_DWORDS]; A87_SCRIPT_POOL_RECORDS],
    pub free_head: u32,
}

impl Default for A87SavedScriptPool {
    fn default() -> Self {
        Self {
            records: [[0; A87_SCRIPT_RECORD_DWORDS]; A87_SCRIPT_POOL_RECORDS],
            free_head: 0,
        }
    }
}

/// The script pool plus the two list heads and the active-list tail.
///
/// The shipped code keeps the free-list head at `0x00fc9b60`, the active head
/// at `0x00fc9b68` and the active tail in the second dword of the list header
/// at `0x00fc9b64`.  All three words hold absolute addresses here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A87ScriptRuntime {
    pub pool: A87ScriptPool,
    pub free_head: u32,
    pub active_head: u32,
    pub active_tail: u32,
}

impl A87ScriptRuntime {
    /// `0x00640620`: build the shipped pool.
    ///
    /// The initialiser zeroes the header words, then walks `uVar2` from
    /// `0x34` while `uVar2 < 0x141c` in `0x34` steps writing
    /// `next = 0x00fc8744 + u`, `previous = 0x00fc86dc + u` and zeroing the
    /// remaining eleven dwords.  Record 0 is written separately
    /// (`next = 0x00fc8744`, `previous = 0`), and record 99 is written by the
    /// second header block at `0x00fc9b2c` (`next = 0`,
    /// `previous = 0x00fc9af8`).  The result is one 100-record chain with the
    /// free-list head at record 0 and no active records.
    pub fn a87_init() -> Self {
        let mut pool = A87ScriptPool::default();
        for index in 0..A87_SCRIPT_POOL_RECORDS {
            pool.records[index][A87_REC_NEXT] = if index + 1 < A87_SCRIPT_POOL_RECORDS {
                a87_record_address(index + 1)
            } else {
                0
            };
            pool.records[index][A87_REC_PREV] = if index == 0 {
                0
            } else {
                a87_record_address(index - 1)
            };
        }
        Self {
            pool,
            free_head: a87_record_address(0),
            active_head: 0,
            active_tail: 0,
        }
    }

    /// `FUN_00640b20` exit paths and `0x00640be1`: the delay is reloaded with
    /// `0x1e` and the following scratch word is cleared.
    pub fn a87_reload_delay(&mut self, index: usize) -> bool {
        if index >= A87_SCRIPT_POOL_RECORDS {
            return false;
        }
        self.pool.records[index][A87_REC_DELAY] = A87_SCRIPT_DELAY_RELOAD;
        self.pool.records[index][A87_REC_DELAY + 1] = 0;
        true
    }

    /// `0x00640020`: unlink a record from the active list and push it onto the
    /// free list.
    ///
    /// The shipped body updates the tail when the record is the tail, patches
    /// `prev->next` and `next->prev`, clears the record's `previous` word,
    /// links its `next` word to the old free-list head and stores the record
    /// as the new head.  The old head's `previous` word is patched back to the
    /// record, so the free list keeps both link words.
    pub fn a87_release_record(&mut self, index: usize) -> bool {
        if index >= A87_SCRIPT_POOL_RECORDS {
            return false;
        }
        let address = a87_record_address(index);
        let next = self.pool.records[index][A87_REC_NEXT];
        let prev = self.pool.records[index][A87_REC_PREV];

        if self.active_tail == address {
            self.active_tail = next;
        }
        if self.active_head == address {
            self.active_head = next;
        }
        if let Some(prev_index) = a87_record_index(prev) {
            self.pool.records[prev_index][A87_REC_NEXT] = next;
        }
        if let Some(next_index) = a87_record_index(next) {
            self.pool.records[next_index][A87_REC_PREV] = prev;
        }

        let old_free_head = self.free_head;
        self.pool.records[index][A87_REC_PREV] = 0;
        self.pool.records[index][A87_REC_NEXT] = old_free_head;
        if let Some(head_index) = a87_record_index(old_free_head) {
            self.pool.records[head_index][A87_REC_PREV] = address;
        }
        self.free_head = address;
        true
    }

    /// Model scaffolding for the insert side of the list.  No row in this
    /// window inserts; `FUN_00642a80` (out of batch) does.  This helper exists
    /// so the sweep rows below can be exercised on a live chain: it removes
    /// the record from the free list and appends it to the active list.
    pub fn a87_push_active(&mut self, index: usize) -> bool {
        if index >= A87_SCRIPT_POOL_RECORDS {
            return false;
        }
        let address = a87_record_address(index);
        if !self.a87_unlink_free(address) {
            return false;
        }
        self.pool.records[index][A87_REC_NEXT] = 0;
        self.pool.records[index][A87_REC_PREV] = self.active_tail;
        if self.active_tail == 0 {
            self.active_head = address;
        } else if let Some(tail) = a87_record_index(self.active_tail) {
            self.pool.records[tail][A87_REC_NEXT] = address;
        }
        self.active_tail = address;
        true
    }

    fn a87_unlink_free(&mut self, address: u32) -> bool {
        let mut cursor = self.free_head;
        let mut prev = 0;
        while cursor != 0 {
            let Some(index) = a87_record_index(cursor) else {
                return false;
            };
            let next = self.pool.records[index][A87_REC_NEXT];
            if cursor == address {
                if prev == 0 {
                    self.free_head = next;
                } else if let Some(prev_index) = a87_record_index(prev) {
                    self.pool.records[prev_index][A87_REC_NEXT] = next;
                }
                if let Some(next_index) = a87_record_index(next) {
                    self.pool.records[next_index][A87_REC_PREV] = prev;
                }
                return true;
            }
            prev = cursor;
            cursor = next;
        }
        false
    }

    /// `0x00640390`: zero dwords 2..=12 of every record in the chain that
    /// starts at `head`, following `next` until the null terminator.
    ///
    /// The shipped loop leaves the two link words alone and then falls into
    /// the non-returning routine at `FUN_00c8a080`; the model returns the number
    /// of visited records.  The 100-record cap only guards against a corrupt
    /// cycle, which the shipped loop would not survive either.
    pub fn a87_clear_chain(&mut self, head: usize) -> usize {
        let mut cursor = Some(head);
        let mut visited = 0;
        while let Some(index) = cursor {
            if index >= A87_SCRIPT_POOL_RECORDS || visited >= A87_SCRIPT_POOL_RECORDS {
                break;
            }
            for field in 2..A87_SCRIPT_RECORD_DWORDS {
                self.pool.records[index][field] = 0;
            }
            visited += 1;
            cursor = a87_record_index(self.pool.records[index][A87_REC_NEXT]);
        }
        visited
    }

    /// `0x00640830`: remove every active record whose `+0x2c` owner word
    /// equals `owner`.
    ///
    /// The shipped walker saves `next` before touching the record and then
    /// relinks the record onto the free list, so a record removed earlier in
    /// the walk cannot break the traversal.  Returns the removed indices in
    /// visit order.
    pub fn a87_kill_owned(&mut self, owner: u32) -> Vec<usize> {
        let mut removed = Vec::new();
        let mut cursor = self.active_head;
        while cursor != 0 {
            let Some(index) = a87_record_index(cursor) else {
                break;
            };
            let next = self.pool.records[index][A87_REC_NEXT];
            if self.pool.records[index][A87_REC_OWNER] == owner {
                self.a87_release_record(index);
                removed.push(index);
            }
            cursor = next;
        }
        removed
    }

    /// `0x006408b0`: one delay-sweep pass over the active list.
    ///
    /// For every visited record the shipped code saves `next`, reads the delay
    /// word, stores `delay - 1` and only then tests the value it read: a
    /// record whose delay was already zero runs (flag bit 2 clear) or is
    /// unlinked (flag bit 2 set).  The decrement wraps like the original
    /// 32-bit store.
    pub fn a87_tick(&mut self) -> Vec<A87ScriptStep> {
        let mut steps = Vec::new();
        let mut cursor = self.active_head;
        while cursor != 0 {
            let Some(index) = a87_record_index(cursor) else {
                break;
            };
            let next = self.pool.records[index][A87_REC_NEXT];
            let delay = self.pool.records[index][A87_REC_DELAY];
            self.pool.records[index][A87_REC_DELAY] = delay.wrapping_sub(1);
            if delay == 0 {
                if self.pool.records[index][A87_REC_FLAGS] & A87_REC_FLAG_UNLINK == 0 {
                    steps.push(A87ScriptStep::Executed(index));
                } else {
                    self.a87_release_record(index);
                    steps.push(A87ScriptStep::Unlinked(index));
                }
            } else {
                steps.push(A87ScriptStep::Decremented(index, delay - 1));
            }
            cursor = next;
        }
        steps
    }

    /// Walk the active list and return the indices in visit order (test and
    /// inspection helper; never longer than the pool).
    pub fn a87_active_list(&self) -> Vec<usize> {
        let mut out = Vec::new();
        let mut cursor = self.active_head;
        while let Some(index) = a87_record_index(cursor) {
            if out.len() >= A87_SCRIPT_POOL_RECORDS {
                break;
            }
            out.push(index);
            cursor = self.pool.records[index][A87_REC_NEXT];
        }
        out
    }

    /// Walk the free list and return the indices in visit order.
    pub fn a87_free_list(&self) -> Vec<usize> {
        let mut out = Vec::new();
        let mut cursor = self.free_head;
        while let Some(index) = a87_record_index(cursor) {
            if out.len() >= A87_SCRIPT_POOL_RECORDS {
                break;
            }
            out.push(index);
            cursor = self.pool.records[index][A87_REC_NEXT];
        }
        out
    }
}

/// Result of one record visited by [`A87ScriptRuntime::a87_tick`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A87ScriptStep {
    /// The delay word was nonzero; it was decremented to the given value.
    Decremented(usize, u32),
    /// The delay word was zero and the record's flag byte had bit 2 clear.
    Executed(usize),
    /// The delay word was zero and the record was relinked onto the free list.
    Unlinked(usize),
}

// ---------------------------------------------------------------------------
// Row 0x00640010: identity byte table
// ---------------------------------------------------------------------------

/// `0x00640010`: fill `0x1c` bytes with their own indices.
///
/// The shipped loop is `do { *(char *)(base + i) = (char)i; i++; } while (i <
/// 0x1c);` with `uVar1` starting at zero, so the table is the byte sequence
/// `00 01 .. 1b`.
pub fn a87_fill_index_table(out: &mut [u8; A87_INDEX_TABLE_BYTES]) {
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = index as u8;
    }
}

// ---------------------------------------------------------------------------
// Row 0x00640080: "no object may be pending" guard
// ---------------------------------------------------------------------------

/// `0x00640080`: succeed only when the head-of-list probe returns null.
///
/// The row calls `func_0x005b0fb0(param_1)` (a list probe outside this
/// window) and either returns `0` or enters the non-returning failure routine
/// `FUN_005b4cd0`.  The probe result is therefore the input here: `None` and
/// a zero word both mean "empty" and yield `Ok`; any live pointer yields
/// `Err(pointer)` for the caller to turn into the shipped failure.
pub fn a87_check_no_pending_object(first: Option<u32>) -> Result<(), u32> {
    match first {
        None | Some(0) => Ok(()),
        Some(pointer) => Err(pointer),
    }
}

// ---------------------------------------------------------------------------
// Row 0x006400a7: selector predicate
// ---------------------------------------------------------------------------

/// `0x006400a7`: the exact match predicate of the register-based selector.
///
/// The recovered body returns the `ESI` object when all three conditions
/// hold: `AX == DI`, `EBX <= (FUN_005b4af0() & 0xff)` (an unsigned compare)
/// and the class byte at `ESI+0x4d` is one of `-100`, `-0x60` or `2`.
/// Otherwise it consults the global iterator (`FUN_005b4bb0`) and fails when
/// that returns null, so the predicate below covers the direct-match half.
pub const A87_SELECTOR_CLASSES: [i8; 3] = [-100, -0x60, 2];

/// The class-byte test of `0x006400a7`.
pub fn a87_selector_class_matches(class_byte: i8) -> bool {
    A87_SELECTOR_CLASSES.contains(&class_byte)
}

/// The full direct-match predicate of `0x006400a7`.
pub fn a87_selector_matches(ax: i16, di: i16, ebx: u32, query: u8, class_byte: i8) -> bool {
    ax == di && ebx <= u32::from(query) && a87_selector_class_matches(class_byte)
}

// ---------------------------------------------------------------------------
// Rows 0x006401a6 / 0x006403f0: pool restore and save
// ---------------------------------------------------------------------------

/// `0x006401a6`: rebuild the pool from a saved image.
///
/// The row copies the 100 saved records into the static block, replacing each
/// nonzero link-index word with `0x00fc86dc + index * 0x34`, and reloads the
/// free-list head from the saved word at offset `0x1450` with the same
/// conversion.  The interpreter then returns through the non-returning
/// routine `FUN_00923e90`; the model returns the rebuilt runtime with an empty
/// active list.
pub fn a87_restore_script_pool(saved: &A87SavedScriptPool) -> A87ScriptRuntime {
    let mut pool = A87ScriptPool::default();
    for index in 0..A87_SCRIPT_POOL_RECORDS {
        pool.records[index] = saved.records[index];
        pool.records[index][A87_REC_NEXT] =
            a87_restored_address(saved.records[index][A87_REC_NEXT]);
        pool.records[index][A87_REC_PREV] =
            a87_restored_address(saved.records[index][A87_REC_PREV]);
    }
    A87ScriptRuntime {
        pool,
        free_head: a87_restored_address(saved.free_head),
        active_head: 0,
        active_tail: 0,
    }
}

/// `0x006403f0`: save the pool.
///
/// The mirror of `0x006401a6`: every record is copied and the two link words
/// are replaced by their 1-based indices through
/// `(address - 0x00fc8710) / 0x34 + 1`; the free-list head index is written
/// at saved offset `0x1450`.  The row then calls `FUN_00687d90` and exits
/// through `FUN_00923e90`; those calls are outside this window.
pub fn a87_save_script_pool(runtime: &A87ScriptRuntime) -> A87SavedScriptPool {
    let mut saved = A87SavedScriptPool::default();
    for index in 0..A87_SCRIPT_POOL_RECORDS {
        saved.records[index] = runtime.pool.records[index];
        saved.records[index][A87_REC_NEXT] =
            a87_saved_index(runtime.pool.records[index][A87_REC_NEXT]);
        saved.records[index][A87_REC_PREV] =
            a87_saved_index(runtime.pool.records[index][A87_REC_PREV]);
    }
    saved.free_head = a87_saved_index(runtime.free_head);
    saved
}

// ---------------------------------------------------------------------------
// Rows 0x00640320 / 0x00640348: interpreter frame setup
// ---------------------------------------------------------------------------

/// One SEH frame as `0x00640320` links it: the previous list head and the
/// handler word `&DAT_00cc7258`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A87ExceptionFrame {
    pub previous: Option<u32>,
    pub handler: u32,
}

/// `0x00640320`: link a new exception frame for the `0x1454`-byte
/// interpreter frame and compute the cookie.
///
/// The row stores `0xffffffff` in the frame's state slot, sets
/// `handler = &DAT_00cc7258`, pushes the current `ExceptionList` and calls
/// `FUN_00923df0(0x1454, DAT_00f4e578 ^ &stack0xfffffffc)`.  The first
/// argument is the frame size and the second is the security cookie xor-ed
/// with the frame pointer; both are reproduced here, while the stack probe
/// itself is outside the window.
pub fn a87_push_script_exception_frame(
    previous: Option<u32>,
    security_cookie: u32,
    stack_pointer: u32,
) -> (A87ExceptionFrame, u32) {
    (
        A87ExceptionFrame {
            previous,
            handler: A87_SCRIPT_SEH_HANDLER,
        },
        security_cookie ^ stack_pointer,
    )
}

/// `0x00640348`: the bare frame probe.  The row is a single
/// `FUN_00923df0(0x1454)` call, so the recovered constant is the frame size.
pub fn a87_script_frame_bytes() -> u32 {
    A87_SCRIPT_FRAME_BYTES
}

// ---------------------------------------------------------------------------
// Row 0x006405c0: release one mapped script image
// ---------------------------------------------------------------------------

/// One of the two loaded script images (`0x00fc8700/0x04` or
/// `0x00fc8708/0x0c`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A87MappedImage {
    /// Nonzero ownership token standing in for the mapping address.
    pub handle: u32,
    /// Length word, already rounded by [`a87_script_image_size`].
    pub size: u32,
}

impl A87MappedImage {
    /// `0x00640620`: round the file length, then allocate and copy it.
    ///
    /// The shipped code sets the length word first, allocates
    /// `FUN_009241e0(length, &DAT_00d32a20, 0, 8)` and copies `local_8`
    /// bytes with `FUN_00c89580`.  Both calls are outside this window; the
    /// model records the rounded length and a nonzero handle for a present
    /// file, and an all-zero image for an absent one.
    pub fn a87_load(file_size: Option<u32>) -> Self {
        match file_size {
            Some(size) => Self {
                handle: 1,
                size: a87_script_image_size(size),
            },
            None => Self::default(),
        }
    }
}

/// `0x00640620`: the length stored for a mapped script file.
///
/// The row computes `local_8 + 0x6000U & 0xfffffffc`, i.e. `0x6000` bytes of
/// headroom followed by a four-byte alignment.  The addition wraps like the
/// shipped unsigned add.
pub fn a87_script_image_size(file_size: u32) -> u32 {
    file_size.wrapping_add(A87_SCRIPT_IMAGE_HEADROOM) & A87_SCRIPT_IMAGE_ALIGN_MASK
}

/// `0x006405c0`: release a mapped script image.
///
/// The row zeroes the length word only when the base word is nonzero, then
/// enters the non-returning release routine `FUN_00924220(base,
/// &DAT_00d32a20, 0, 0)`.  The model performs the conditional length clear and
/// clears the handle; `true` means a mapping was present.
pub fn a87_unmap_script_image(image: &mut A87MappedImage) -> bool {
    if image.handle != 0 {
        image.size = 0;
        image.handle = 0;
        true
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// Row 0x006406b8: rotate the word and rebind the bwscript length
// ---------------------------------------------------------------------------

/// Result of `0x006406b8`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A87BwScriptRebind {
    /// The rotated dword the row stores back through `in_EAX`.
    pub rotated: u32,
    /// The new `0x00fc870c` length word (`in_ECX`).
    pub size: u32,
}

/// `0x006406b8`: rotate a dword and publish a new bwscript length.
///
/// The recovered body is
/// `*in_EAX = *in_EAX << (in_ECX & 0x1f) | *in_EAX >> (0x20 - (in_ECX & 0x1f))`
/// followed by `0x00fc870c = in_ECX` and a fresh allocation at
/// `0x00fc8708`.  On x86 the shift counts are masked to five bits, which makes
/// the expression exactly `word.rotate_left(count & 0x1f)`.  The allocation
/// is outside this window.
pub fn a87_rotate_and_resize(word: u32, count: u32) -> A87BwScriptRebind {
    A87BwScriptRebind {
        rotated: word.rotate_left(count & 0x1f),
        size: count,
    }
}

// ---------------------------------------------------------------------------
// Rows 0x00640940 / 0x00640999 / 0x006409d3: script-id dispatch
// ---------------------------------------------------------------------------

/// One 8-byte `(id, code offset)` table entry of `aiscript.bin` or
/// `bwscript.bin`.  Both tables are scanned in 8-byte steps and terminated by
/// an entry whose id word is zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A87ScriptEntry {
    pub id: u32,
    pub offset: u32,
}

/// A matched script: the code offset and whether it came from
/// `bwscript.bin` (the row's fourth argument `uVar4`, `0` for
/// `aiscript.bin` and `1` for `bwscript.bin`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A87ScriptHit {
    pub offset: u32,
    pub external: bool,
}

/// Reasons `0x00640940` does not reach its dispatch call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A87DispatchError {
    /// The id is not one of the five words at `0x00f056b8`.
    UnknownId,
    /// Neither table contains a non-zero code offset for the id.
    NotFound,
}

/// `0x00640940`: the five ids the dispatch row accepts, recovered from the
/// literal string `"EnBkEnTrExTrDWHePB3F"` at `0x00f056b8`.
pub fn a87_script_id_accepted(id: u32) -> bool {
    A87_SCRIPT_ID_TABLE.contains(&id)
}

/// Scan one script table for `id`.
///
/// Mirrors the row's loop: stop at the first entry whose id is zero, accept
/// the first id match, and treat a matching entry with a zero code offset as
/// "no dispatch".
pub fn a87_find_script_entry(table: &[A87ScriptEntry], id: u32) -> Option<u32> {
    for entry in table {
        if entry.id == 0 {
            return None;
        }
        if entry.id == id {
            return (entry.offset != 0).then_some(entry.offset);
        }
    }
    None
}

/// `0x00640940`: validate the id, then search `aiscript.bin` before
/// `bwscript.bin`.
///
/// The row masks the id table with the five-word loop and falls into
/// `FUN_00748d40` for an unknown id.  It scans the first mapping only when
/// its length is nonzero, then the second, and passes `0` or `1` as the
/// external flag to `FUN_00642a80`.
pub fn a87_dispatch_script(
    ai_table: &[A87ScriptEntry],
    bw_table: &[A87ScriptEntry],
    id: u32,
) -> Result<A87ScriptHit, A87DispatchError> {
    if !a87_script_id_accepted(id) {
        return Err(A87DispatchError::UnknownId);
    }
    if let Some(offset) = a87_find_script_entry(ai_table, id) {
        return Ok(A87ScriptHit {
            offset,
            external: false,
        });
    }
    if let Some(offset) = a87_find_script_entry(bw_table, id) {
        return Ok(A87ScriptHit {
            offset,
            external: true,
        });
    }
    Err(A87DispatchError::NotFound)
}

/// `0x00640999`: the panel record offset.
///
/// The row writes the raw `EBX` when the object word is zero and
/// `(EBX + 0xc) * 0x24` otherwise.  The multiply wraps like the shipped
/// 32-bit `imul`.
pub fn a87_panel_slot(index: u32, has_object: bool) -> u32 {
    if has_object {
        index
            .wrapping_add(A87_PANEL_SLOT_BIAS)
            .wrapping_mul(A87_PANEL_RECORD_BYTES)
    } else {
        index
    }
}

/// `0x00640999`: read the panel state byte at `slot + 8`.
pub fn a87_panel_state(panel: &[u8], slot: u32) -> Option<u8> {
    panel
        .get(slot.wrapping_add(A87_PANEL_STATE_OFFSET) as usize)
        .copied()
}

/// `0x00640999`: the state byte must be exactly `1`.
pub fn a87_panel_state_running(state: u8) -> bool {
    state == A87_PANEL_STATE_RUNNING
}

/// `0x006409d3`: the special-case test.
///
/// The row continues only when the panel byte at `+8` is `7` and the id word
/// is `0x75637352` (`"Rscu"`); it then searches both script tables and
/// dispatches with the external flag fixed to `0`.
pub fn a87_special_script_matches(panel_kind: u8, id: u32) -> bool {
    panel_kind == A87_PANEL_KIND_SPECIAL && id == A87_SPECIAL_SCRIPT_ID
}

// ---------------------------------------------------------------------------
// Row 0x00640a90: interpreter cost lookup
// ---------------------------------------------------------------------------

/// `0x00640a90`: the `0x1e`/`5` cost special case.
///
/// The row looks `script_type & 0xffff` up in one of two tables
/// (`FUN_00695be0` when `param_3 == 0`, `FUN_00695c10` otherwise), then adds
/// the other special entry when the type is `0x1e` or `5`:
///
/// * type `0x1e` returns `lookup(0x1e) + lookup(5)`;
/// * type `5` returns `lookup(5) + lookup(0x1e)`;
/// * every other type returns `lookup(type)`.
///
/// The two lookups are passed in so the tables stay outside this revision;
/// `slot` is the second register argument the decompiler preserves for the
/// nested call.  The adds wrap like the shipped 32-bit `add`.
pub fn a87_script_cost<F, G>(
    script_type: u32,
    slot: u8,
    absolute: bool,
    relative: F,
    absolute_table: G,
) -> i32
where
    F: Fn(u16, u8) -> i32,
    G: Fn(u16, u8) -> i32,
{
    let masked = (script_type & 0xffff) as u16;
    if !absolute {
        let mut value = relative(masked, slot);
        if script_type == A87_COST_TYPE_DELAY {
            return relative(A87_COST_TYPE_BUILD as u16, slot).wrapping_add(value);
        }
        if script_type == A87_COST_TYPE_BUILD {
            value = relative(A87_COST_TYPE_DELAY as u16, slot).wrapping_add(value);
        }
        value
    } else {
        let value = absolute_table(masked, slot);
        if script_type == A87_COST_TYPE_DELAY {
            return absolute_table(A87_COST_TYPE_BUILD as u16, slot).wrapping_add(value);
        }
        if script_type == A87_COST_TYPE_BUILD {
            return absolute_table(A87_COST_TYPE_DELAY as u16, slot).wrapping_add(value);
        }
        value
    }
}

// ---------------------------------------------------------------------------
// Rows 0x00640be8 / 0x00640e25: interior entries of FUN_00640b20
// ---------------------------------------------------------------------------

/// `0x00640be8`: the shared unwind epilogue of `FUN_00640b20`.
///
/// The four instructions restore `ExceptionList` from the interpreter frame
/// at `[ebp-0xc]` and jump to the non-returning unwinder `FUN_00c86b6e`.
/// Ghidra also lists this address as a function because the interpreter's
/// switch jumps here; there is no independent entry point, so the model
/// returns the recovered exception-list word for the caller to install.
pub fn a87_interpreter_unwind(saved_exception_list: Option<u32>) -> Option<u32> {
    saved_exception_list
}

/// Result of the `0x00640e25` operand guard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A87OperandFetch {
    /// Carry was set and `cursor + 2` is still below the limit; the new
    /// cursor is returned.
    Word { cursor: u32 },
    /// The guard failed and the shipped row falls into its diagnostic tail.
    Truncated,
}

/// `0x00640e25`: guard an already-advanced word operand.
///
/// The recovered block is `if (CF && in_EAX + 2U < in_ECX)` then store
/// `in_EAX + 2` in the record's cursor word and return; otherwise the
/// decompiler loses the flow in the diagnostic string builder.  The unsigned
/// comparison and the wrapping add are reproduced exactly.
pub fn a87_operand_word_advance(carry: bool, cursor: u32, limit: u32) -> A87OperandFetch {
    let advanced = cursor.wrapping_add(2);
    if carry && advanced < limit {
        A87OperandFetch::Word { cursor: advanced }
    } else {
        A87OperandFetch::Truncated
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent87 {
    use super::*;

    /// `0x00640010`: the byte table is the identity sequence.
    #[test]
    fn index_table_matches_0x00640010() {
        let mut table = [0xffu8; A87_INDEX_TABLE_BYTES];
        a87_fill_index_table(&mut table);
        let expected: Vec<u8> = (0..A87_INDEX_TABLE_BYTES as u8).collect();
        assert_eq!(table.to_vec(), expected);
        assert_eq!(table[0], 0);
        assert_eq!(table[0x1b], 0x1b);
    }

    /// `0x00640080`: only a null probe passes.
    #[test]
    fn object_list_check_matches_0x00640080() {
        assert_eq!(a87_check_no_pending_object(None), Ok(()));
        assert_eq!(a87_check_no_pending_object(Some(0)), Ok(()));
        assert_eq!(
            a87_check_no_pending_object(Some(0x00fc_8710)),
            Err(0x00fc_8710)
        );
    }

    /// `0x006400a7`: the three-condition direct match.
    #[test]
    fn selector_matches_0x006400a7() {
        assert!(a87_selector_matches(7, 7, 5, 5, 2));
        assert!(a87_selector_matches(7, 7, 0, 0xff, -100));
        assert!(a87_selector_matches(-1, -1, 0xff, 0xff, -0x60));
        // Query byte is the low byte of the probe result.
        assert!(!a87_selector_matches(7, 7, 6, 5, 2));
        // AX must equal DI.
        assert!(!a87_selector_matches(7, 8, 0, 0xff, 2));
        // Only the three recovered class bytes are accepted.
        assert!(!a87_selector_matches(7, 7, 0, 0xff, 1));
        assert!(!a87_selector_matches(7, 7, 0, 0xff, 7));
        assert_eq!(A87_SELECTOR_CLASSES, [-100, -96, 2]);
    }

    /// `0x00640620`: the full 100-record chain, the free head and the
    /// separately written last record.
    #[test]
    fn pool_init_matches_0x00640620() {
        let runtime = A87ScriptRuntime::a87_init();
        assert_eq!(a87_record_address(0), 0x00fc_8710);
        assert_eq!(a87_record_address(99), 0x00fc_9b2c);
        assert_eq!(a87_record_address(98), 0x00fc_9af8);
        assert_eq!(a87_record_address(100), 0x00fc_9b60);
        assert_eq!(runtime.pool.records[0][A87_REC_NEXT], a87_record_address(1));
        assert_eq!(runtime.pool.records[0][A87_REC_PREV], 0);
        assert_eq!(runtime.pool.records[99][A87_REC_NEXT], 0);
        assert_eq!(
            runtime.pool.records[99][A87_REC_PREV],
            a87_record_address(98)
        );
        assert_eq!(
            runtime.pool.records[98][A87_REC_NEXT],
            a87_record_address(99)
        );
        // Every non-link dword starts zero.
        for index in 0..A87_SCRIPT_POOL_RECORDS {
            for field in 2..A87_SCRIPT_RECORD_DWORDS {
                assert_eq!(runtime.pool.records[index][field], 0);
            }
        }
        assert_eq!(runtime.free_head, a87_record_address(0));
        assert_eq!(runtime.active_head, 0);
        assert_eq!(runtime.active_tail, 0);
        assert_eq!(runtime.a87_free_list().len(), A87_SCRIPT_POOL_RECORDS);
        assert_eq!(runtime.a87_active_list(), Vec::<usize>::new());
        // 0x00fc9b30 is record 99's `previous` word.
        assert_eq!(a87_record_address(99) + 4, 0x00fc_9b30);
        // 0x00fc9b60 is the free-list word at pool offset 0x1450.
        assert_eq!(
            A87_SCRIPT_FREE_LIST,
            A87_SCRIPT_POOL_BASE + A87_SCRIPT_POOL_BYTES
        );
    }

    /// `0x006401a6` / `0x006403f0`: the saved-index formula and the round
    /// trip through both conversions.
    #[test]
    fn save_restore_roundtrip_matches_pool_rows() {
        assert_eq!(a87_saved_index(0), 0);
        assert_eq!(a87_saved_index(a87_record_address(0)), 1);
        assert_eq!(a87_saved_index(a87_record_address(5)), 6);
        assert_eq!(a87_saved_index(a87_record_address(99)), 100);
        assert_eq!(a87_restored_address(0), 0);
        assert_eq!(a87_restored_address(1), a87_record_address(0));
        assert_eq!(a87_restored_address(100), a87_record_address(99));
        assert_eq!(A87_SCRIPT_INDEX_ORIGIN, 0x00fc_86dc);

        let runtime = A87ScriptRuntime::a87_init();
        let saved = a87_save_script_pool(&runtime);
        assert_eq!(saved.free_head, 1);
        assert_eq!(saved.records[0][A87_REC_NEXT], 2);
        assert_eq!(saved.records[99][A87_REC_NEXT], 0);
        assert_eq!(saved.records[99][A87_REC_PREV], 99);

        let restored = a87_restore_script_pool(&saved);
        assert_eq!(restored.pool, runtime.pool);
        assert_eq!(restored.free_head, runtime.free_head);
        assert_eq!(restored.active_head, 0);
    }

    /// `0x00640020`: unlink from the middle, head and tail of the active
    /// list, then push onto the free list.
    #[test]
    fn release_record_matches_0x00640020() {
        let mut runtime = A87ScriptRuntime::a87_init();
        // Build an active list 0,1,2 out of the free list.
        assert!(runtime.a87_push_active(0));
        assert!(runtime.a87_push_active(1));
        assert!(runtime.a87_push_active(2));
        assert_eq!(runtime.a87_active_list(), vec![0, 1, 2]);
        assert_eq!(runtime.active_tail, a87_record_address(2));

        // Middle record.
        assert!(runtime.a87_release_record(1));
        assert_eq!(runtime.a87_active_list(), vec![0, 2]);
        assert_eq!(runtime.free_head, a87_record_address(1));
        // The push links the record to the free head that existed before it:
        // records 0..2 were taken off the free list, so that head is record 3.
        assert_eq!(runtime.pool.records[1][A87_REC_NEXT], a87_record_address(3));
        assert_eq!(runtime.pool.records[1][A87_REC_PREV], 0);

        // Head record: the next record's previous link becomes null.
        assert!(runtime.a87_release_record(0));
        assert_eq!(runtime.a87_active_list(), vec![2]);
        assert_eq!(runtime.free_head, a87_record_address(0));
        assert_eq!(runtime.pool.records[2][A87_REC_PREV], 0);

        // Tail record: the active tail follows the null next link.
        assert!(runtime.a87_release_record(2));
        assert_eq!(runtime.a87_active_list(), Vec::<usize>::new());
        assert_eq!(runtime.active_head, 0);
        assert_eq!(runtime.active_tail, 0);
        assert_eq!(runtime.free_head, a87_record_address(2));
        // Free-list order is LIFO: 2, 0, 1, then the untouched records.
        let free = runtime.a87_free_list();
        assert_eq!(&free[..3], &[2, 0, 1]);
        // `0x006403f0` saves the free head as a 1-based index: record 2 -> 3.
        let saved = a87_save_script_pool(&runtime);
        assert_eq!(saved.free_head, 3);
        assert_eq!(saved.records[1][A87_REC_NEXT], 4);
        assert!(!runtime.a87_release_record(A87_SCRIPT_POOL_RECORDS));
    }

    /// `0x00640390`: the chain clear zeroes dwords 2..=12 and keeps links.
    #[test]
    fn clear_chain_matches_0x00640390() {
        let mut runtime = A87ScriptRuntime::a87_init();
        runtime.pool.records[0][A87_REC_FLAGS] = 0xffff_ffff;
        runtime.pool.records[0][A87_REC_OWNER] = 0x1234_5678;
        runtime.pool.records[57][A87_REC_DELAY] = 9;
        assert_eq!(runtime.a87_clear_chain(0), A87_SCRIPT_POOL_RECORDS);
        for index in 0..A87_SCRIPT_POOL_RECORDS {
            for field in 2..A87_SCRIPT_RECORD_DWORDS {
                assert_eq!(runtime.pool.records[index][field], 0);
            }
        }
        // Links survive the clear.
        assert_eq!(runtime.pool.records[0][A87_REC_NEXT], a87_record_address(1));
        assert_eq!(
            runtime.pool.records[99][A87_REC_PREV],
            a87_record_address(98)
        );
    }

    /// `0x006405c0`: a present image loses its length before the release; an
    /// absent one is left alone.
    #[test]
    fn release_image_matches_0x006405c0() {
        let mut image = A87MappedImage::a87_load(Some(0x1235));
        assert_eq!(image.size, 0x7234);
        assert_ne!(image.handle, 0);
        assert!(a87_unmap_script_image(&mut image));
        assert_eq!(image, A87MappedImage::default());

        let mut absent = A87MappedImage::default();
        assert!(!a87_unmap_script_image(&mut absent));
        assert_eq!(absent, A87MappedImage::default());

        // The length is stored before the handle, so an image whose handle is
        // zero keeps whatever length word was written (the row only clears it
        // inside the nonzero-handle branch).
        let mut zero_handle = A87MappedImage {
            handle: 0,
            size: 0x10,
        };
        assert!(!a87_unmap_script_image(&mut zero_handle));
        assert_eq!(zero_handle.size, 0x10);
    }

    /// `0x00640620`: the `+0x6000` headroom and four-byte alignment.
    #[test]
    fn image_size_matches_0x00640620() {
        assert_eq!(a87_script_image_size(0), 0x6000);
        assert_eq!(a87_script_image_size(1), 0x6000);
        assert_eq!(a87_script_image_size(2), 0x6000);
        assert_eq!(a87_script_image_size(3), 0x6000);
        assert_eq!(a87_script_image_size(4), 0x6004);
        assert_eq!(a87_script_image_size(0x1235), 0x7234);
        assert_eq!(a87_script_image_size(u32::MAX), 0x5ffc);
        assert_eq!(a87_script_image_size(0x1000), 0x7000);
        // Absent files leave the length and handle words at zero.
        assert_eq!(A87MappedImage::a87_load(None), A87MappedImage::default());
    }

    /// `0x006406b8`: the x86-masked rotate plus the published length.
    #[test]
    fn rotate_matches_0x006406b8() {
        assert_eq!(a87_rotate_and_resize(0x8000_0001, 1).rotated, 0x0000_0003);
        assert_eq!(a87_rotate_and_resize(0x1234_5678, 0).rotated, 0x1234_5678);
        assert_eq!(a87_rotate_and_resize(0x1234_5678, 32).rotated, 0x1234_5678);
        assert_eq!(a87_rotate_and_resize(0x1234_5678, 33).rotated, 0x2468_acf0);
        assert_eq!(
            a87_rotate_and_resize(0xdead_beef, 0x1f).rotated,
            0xef56_df77
        );
        let rebound = a87_rotate_and_resize(0x1111_1111, 0x99);
        assert_eq!(rebound.size, 0x99);
        assert_eq!(rebound.rotated, 0x2222_2222);
    }

    /// `0x00640830`: the owner sweep takes every matching record and leaves
    /// the rest linked, in order.
    #[test]
    fn kill_owned_matches_0x00640830() {
        let mut runtime = A87ScriptRuntime::a87_init();
        for index in 0..4 {
            assert!(runtime.a87_push_active(index));
        }
        runtime.pool.records[0][A87_REC_OWNER] = 0xaa;
        runtime.pool.records[1][A87_REC_OWNER] = 0xbb;
        runtime.pool.records[2][A87_REC_OWNER] = 0xaa;
        runtime.pool.records[3][A87_REC_OWNER] = 0xcc;

        assert_eq!(runtime.a87_kill_owned(0xaa), vec![0, 2]);
        assert_eq!(runtime.a87_active_list(), vec![1, 3]);
        assert_eq!(runtime.active_head, a87_record_address(1));
        assert_eq!(runtime.active_tail, a87_record_address(3));
        assert_eq!(runtime.a87_kill_owned(0xaa), Vec::<usize>::new());
        assert_eq!(runtime.a87_kill_owned(0xcc), vec![3]);
        assert_eq!(runtime.a87_active_list(), vec![1]);
        // The removed records are at the head of the free list.
        let free = runtime.a87_free_list();
        assert_eq!(&free[..2], &[3, 2]);
    }

    /// `0x006408b0`: decrement, run at a pre-decrement zero, and unlink the
    /// bit-2 records; traversal follows the saved next word.
    #[test]
    fn tick_matches_0x006408b0() {
        let mut runtime = A87ScriptRuntime::a87_init();
        for index in 0..3 {
            assert!(runtime.a87_push_active(index));
        }
        runtime.pool.records[0][A87_REC_DELAY] = 0;
        runtime.pool.records[1][A87_REC_DELAY] = 1;
        runtime.pool.records[2][A87_REC_DELAY] = 0;
        runtime.pool.records[2][A87_REC_FLAGS] = A87_REC_FLAG_UNLINK;

        assert_eq!(
            runtime.a87_tick(),
            vec![
                A87ScriptStep::Executed(0),
                A87ScriptStep::Decremented(1, 0),
                A87ScriptStep::Unlinked(2),
            ]
        );
        // The executed record wrapped its zero delay.
        assert_eq!(runtime.pool.records[0][A87_REC_DELAY], u32::MAX);
        assert_eq!(runtime.pool.records[1][A87_REC_DELAY], 0);
        assert_eq!(runtime.a87_active_list(), vec![0, 1]);

        // Second pass: record 0 still has a nonzero (wrapped) delay, record 1
        // now runs because its delay reads zero.
        assert_eq!(
            runtime.a87_tick(),
            vec![
                A87ScriptStep::Decremented(0, u32::MAX - 1),
                A87ScriptStep::Executed(1),
            ]
        );
        // The interpreter reload path writes 0x1e and clears the next word.
        assert!(runtime.a87_reload_delay(0));
        assert_eq!(runtime.pool.records[0][A87_REC_DELAY], 0x1e);
        assert_eq!(runtime.pool.records[0][A87_REC_DELAY + 1], 0);
        assert!(!runtime.a87_reload_delay(A87_SCRIPT_POOL_RECORDS));
    }

    /// `0x00640940`: the five-id gate, the aiscript-before-bwscript order and
    /// the external flag.
    #[test]
    fn dispatch_matches_0x00640940() {
        let enbk = A87_SCRIPT_ID_TABLE[0];
        assert_eq!(a87_script_id_accepted(enbk), true);
        assert!(!a87_script_id_accepted(u32::from_le_bytes(*b"Zzzz")));
        assert_eq!(A87_SCRIPT_ID_TABLE.len(), 5);
        assert_eq!(A87_SCRIPT_ID_TABLE[4], u32::from_le_bytes(*b"PB3F"));

        let ai = [
            A87ScriptEntry {
                id: enbk,
                offset: 0x40,
            },
            A87ScriptEntry { id: 0, offset: 0 },
        ];
        let bw = [
            A87ScriptEntry {
                id: enbk,
                offset: 0x80,
            },
            A87ScriptEntry { id: 0, offset: 0 },
        ];
        assert_eq!(
            a87_dispatch_script(&ai, &bw, enbk),
            Ok(A87ScriptHit {
                offset: 0x40,
                external: false
            })
        );
        assert_eq!(
            a87_dispatch_script(&[], &bw, enbk),
            Ok(A87ScriptHit {
                offset: 0x80,
                external: true
            })
        );
        assert_eq!(
            a87_dispatch_script(&ai, &bw, u32::from_le_bytes(*b"Zzzz")),
            Err(A87DispatchError::UnknownId)
        );
        // The terminator stops the scan even when the id appears later.
        let terminated = [
            A87ScriptEntry { id: 0, offset: 0 },
            A87ScriptEntry {
                id: enbk,
                offset: 0x40,
            },
        ];
        assert_eq!(
            a87_dispatch_script(&terminated, &[], enbk),
            Err(A87DispatchError::NotFound)
        );
        // A matching id with a zero code offset is not dispatched.
        let zero_offset = [A87ScriptEntry {
            id: enbk,
            offset: 0,
        }];
        assert_eq!(
            a87_dispatch_script(&zero_offset, &[], enbk),
            Err(A87DispatchError::NotFound)
        );
        assert_eq!(a87_find_script_entry(&zero_offset, enbk), None);
    }

    /// `0x00640999` / `0x006409d3`: the `(index + 0xc) * 0x24` panel offset,
    /// the state byte at `+8`, and the special kind/id test.
    #[test]
    fn panel_rows_match_0x00640999_and_0x006409d3() {
        assert_eq!(a87_panel_slot(5, false), 5);
        assert_eq!(a87_panel_slot(5, true), 0x11 * 0x24);
        assert_eq!(a87_panel_slot(0, true), 0x0c * 0x24);
        // `u32::MAX + 0xc` wraps to `0xb`, so the offset is `0xb * 0x24`.
        assert_eq!(a87_panel_slot(u32::MAX, true), 0x0b * 0x24);

        let mut panel = vec![0u8; 0x200];
        let slot = a87_panel_slot(1, true);
        panel[slot as usize + A87_PANEL_STATE_OFFSET as usize] = 1;
        assert_eq!(a87_panel_state(&panel, slot), Some(1));
        assert!(a87_panel_state_running(1));
        assert!(!a87_panel_state_running(0));
        assert!(!a87_panel_state_running(4));
        assert_eq!(a87_panel_state(&panel, 0xffff_fff0), None);

        assert!(a87_special_script_matches(7, A87_SPECIAL_SCRIPT_ID));
        assert_eq!(A87_SPECIAL_SCRIPT_ID, 0x7563_7352);
        assert!(!a87_special_script_matches(1, A87_SPECIAL_SCRIPT_ID));
        assert!(!a87_special_script_matches(7, 0x1234_5678));
    }

    /// `0x00640a90`: both tables and both special-cased types.
    #[test]
    fn cost_matches_0x00640a90() {
        let relative = |script_type: u16, slot: u8| i32::from(script_type) * 10 + i32::from(slot);
        let absolute = |script_type: u16, slot: u8| i32::from(script_type) * 100 + i32::from(slot);

        assert_eq!(
            a87_script_cost(0x1e, 7, false, relative, absolute),
            0x1e * 10 + 7 + 5 * 10 + 7
        );
        assert_eq!(
            a87_script_cost(5, 7, false, relative, absolute),
            5 * 10 + 7 + 0x1e * 10 + 7
        );
        assert_eq!(
            a87_script_cost(0x1e, 7, true, relative, absolute),
            0x1e * 100 + 7 + 5 * 100 + 7
        );
        assert_eq!(
            a87_script_cost(5, 7, true, relative, absolute),
            5 * 100 + 7 + 0x1e * 100 + 7
        );
        assert_eq!(a87_script_cost(9, 3, false, relative, absolute), 9 * 10 + 3);
        assert_eq!(a87_script_cost(9, 3, true, relative, absolute), 9 * 100 + 3);
        // The lookup receives the masked type word; the special comparisons
        // use the unmasked value, so `0x1_001e` is not special.
        assert_eq!(
            a87_script_cost(0x1_0000 | 9, 0, false, relative, absolute),
            9 * 10
        );
        assert_eq!(
            a87_script_cost(0x1_001e, 0, false, relative, absolute),
            0x1e * 10
        );
        // The adds wrap.
        let max = |_: u16, _: u8| i32::MAX;
        assert_eq!(
            a87_script_cost(0x1e, 0, false, max, absolute),
            i32::MAX.wrapping_add(i32::MAX)
        );
    }

    /// `0x00640320` / `0x00640348`: the frame size, the handler word and the
    /// cookie xor.
    #[test]
    fn seh_rows_match_0x00640320_and_0x00640348() {
        assert_eq!(a87_script_frame_bytes(), 0x1454);
        assert_eq!(A87_SCRIPT_FRAME_BYTES, 0x1454);
        assert_eq!(A87_SCRIPT_SEH_HANDLER, 0x00cc_7258);
        assert_eq!(A87_SECURITY_COOKIE_ADDRESS, 0x00f4_e578);

        let (frame, cookie) =
            a87_push_script_exception_frame(Some(0xdead_0000), 0x1122_3344, 0x5000);
        assert_eq!(
            frame,
            A87ExceptionFrame {
                previous: Some(0xdead_0000),
                handler: 0x00cc_7258
            }
        );
        assert_eq!(cookie, 0x1122_6344);

        let (frame, cookie) = a87_push_script_exception_frame(None, 0, 0xffff_ffff);
        assert_eq!(frame.previous, None);
        assert_eq!(cookie, 0xffff_ffff);
    }

    /// `0x00640be8`: the epilogue hands the saved exception-list word back.
    #[test]
    fn unwind_matches_0x00640be8() {
        assert_eq!(a87_interpreter_unwind(Some(0x00cc_7290)), Some(0x00cc_7290));
        assert_eq!(a87_interpreter_unwind(None), None);
    }

    /// `0x00640e25`: the carry-gated word cursor.
    #[test]
    fn operand_guard_matches_0x00640e25() {
        assert_eq!(
            a87_operand_word_advance(true, 0, 4),
            A87OperandFetch::Word { cursor: 2 }
        );
        assert_eq!(
            a87_operand_word_advance(true, 1, 4),
            A87OperandFetch::Word { cursor: 3 }
        );
        assert_eq!(
            a87_operand_word_advance(true, 2, 4),
            A87OperandFetch::Truncated
        );
        assert_eq!(
            a87_operand_word_advance(false, 0, 4),
            A87OperandFetch::Truncated
        );
        assert_eq!(
            a87_operand_word_advance(true, 0xffff_fffd, u32::MAX),
            A87OperandFetch::Truncated
        );
        assert_eq!(
            a87_operand_word_advance(true, 0xffff_fffe, u32::MAX),
            A87OperandFetch::Word { cursor: 0 }
        );
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// * `FUN_00641021` (`FUN_00641021`, 87 bytes) is not a function entry.  The
//   recovered body opens with `*unaff_EDI = *unaff_EDI >> 3 | *unaff_EDI <<
//   0x1d; out(*unaff_ESI,(short)in_EDX); *(char *)in_EAX = (char)*in_EAX +
//   (char)in_EAX;` and then resumes the `FUN_00640b20` switch at
//   `code_r0x00641028`.  It is a jump-table target inside the already
//   translated interpreter body.
// * `FUN_006416a0` (`FUN_006416a0`, 71 bytes) is the same kind of interior
//   entry: `*pbVar1 = *pbVar1 + (byte)in_ECX` then
//   `code_rFUN_006416a0`, the shared case-0x17/0x18 resumption point of the
//   interpreter.
// * `FUN_0064176c` (`FUN_0064176c`, 23 bytes) is the third one: `*in_EAX =
//   *in_EAX + (char)in_EAX` immediately followed by `code_r0x0064176e`, the
//   `FUN_00635340` case tail of the interpreter.  Translating these three
//   rows separately would duplicate `FUN_00640b20`, whose ledger row is already
//   translated with `src/aiscript.rs` as evidence.
//
// The two rows that are translated above only as their recovered interior
// entry are called out in their doc comments: `0x00640be8` is the interpreter
// epilogue and `0x00640e25` is the operand-truncation guard.  They are listed
// in the ledger as separate `decompiled_uncited` functions, so they are
// reconstructed rather than skipped, but neither is an independent shipped
// entry point.
