#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent79.rs
// ===========================================================================
// Translation batch 79.  Only this file is edited by the batch.
//
// Batch 79 owns `0x00630000..=0x00631fff`.  The ledger lists 57
// `decompiled_uncited` rows in that window (plus two `thunk` rows and one row
// that is already `translated`); 37 are translated below and the 20 rows that
// are not are listed at the end with their reason.
//
// The window is client-side plumbing, not simulation code: the
// `OsFileLogMarshaller` (the name the decompiler recovered at `0x00630a20`),
// the option-tree report printer, the key-down counter table, a block-backed
// deque of 28-byte strings, the heap-sort helpers, the frame-time statistics
// that feed the performance report, and that report's open/retry loop.
//
// Recovered layouts (all offsets in bytes):
//
// | layout                     | words                                                       |
// |----------------------------|-------------------------------------------------------------|
// | 0x1c string                | `+0x00` data, `+0x04` size, `+0x08` capacity, `+0x0c..+0x1b` inline buffer |
// | log stream                 | `+0x00` vftable, `+0x04` path string, `+0x20` handle, `+0x24` append byte, `+0x28` pending deque |
// | deque                      | `+0x00` block array, `+0x04` block capacity, `+0x08` front, `+0x0c` size |
// | option group               | `+0x00` next, `+0x04` name, `+0x20` entry count, `+0x24` entry array  |
// | option entry               | `+0x00` next, `+0x04` name, `+0x20` payload, `+0x80` dirty byte        |
// | sample (frame timestamp)   | `+0x00` low dword, `+0x04` high dword                        |
//
// The 0x1c string is the same record as the parser records of batch 41: the
// capacity word carries bit 31 as the "inline buffer" marker and its low bits
// hold the inline capacity `0xf`.  `0x00630940` asserts exactly that bit on
// every in-range deque element, and `0x00630be6` combines the capacity words
// with the formula batch 41 reproduces as `a41_transfer_capacity`.
//
// Address to function:
//
// * `0x006300d0` [`a79_option_report`]
// * `0x00630255` [`a79_option_entries`]
// * `0x00630350` [`a79_clear_entry_flags`]
// * `0x006304a0` [`a79_release_dirty_slot`]
// * `0x00630870` [`A79LogStream::a79_commit`]
// * `0x00630890` [`a79_read_position`]
// * `0x00630892` [`a79_read_position_indirect`]
// * `0x00630940` [`A79Ring::a79_check_range`]
// * `0x00630a10` [`A79Ring::a79_check_range_tail`]
// * `0x00630a20` [`A79LogStream::a79_teardown`]
// * `0x00630ab0` [`A79LogStream::a79_flush`]
// * `0x00630ac0` [`a79_error_unwind`]
// * `0x00630be6` [`A79String::a79_steal`]
// * `0x00630bf2` [`A79String::a79_steal_flagged`]
// * `0x00630cb4` [`A79LogStream::a79_release_and_reopen`]
// * `0x00630cd6` [`A79LogStream::a79_release_and_reopen_tail`]
// * `0x00630d20` [`A79LogStream::a79_open`]
// * `0x00630d9a` [`a79_seek_end_call`]
// * `0x00630df0` [`A79LogStream::a79_write_formatted`]
// * `0x00630ea0` [`A79LogStream::a79_write`]
// * `0x00631120` [`A79Ring::a79_reserve_slot`]
// * `0x00631222` [`A79Ring::a79_allocate_block`]
// * `0x00631260` [`a79_language_gate`]
// * `0x006312f0` [`A79KeyState::a79_initialised`]
// * `0x00631300` [`A79KeyState::a79_flag_state`]
// * `0x00631320` [`a79_default_key_table`]
// * `0x00631340` [`A79KeyState::a79_init_once`]
// * `0x00631380` [`A79KeyState::a79_get`]
// * `0x006313a0` [`A79KeyState::a79_shutdown`]
// * `0x00631400` [`A79KeyState::a79_decay`]
// * `0x0063159f` [`a79_heap_sort`]
// * `0x00631790` [`a79_adjust_heap`]
// * `0x006318f0` [`a79_timing_stats`]
// * `0x00631bc0` [`a79_timing_stats_or_zero`]
// * `0x00631e00` [`a79_report_steps`]
// * `0x00631ee8` [`a79_report_steps_retried`]
// * `0x00631ff0` [`a79_write_record`]
//
// Every row reproduced here is a fragment of a larger inlined template.  The
// decompiler cut the bodies at the capacity helper `FUN_00538810`, at the
// non-returning `_invalid_parameter_noinfo_noreturn` `FUN_0093ec70`, and at
// the CRT file layer (`FUN_0093fd10` open, `FUN_009406d0` seek,
// `FUN_00940720` write, `FUN_0093fe80` flush, `FUN_0093fb70` close,
// `FUN_0093ffe0` commit, `FUN_00940060`/`FUN_0093f610` position query).  The
// functions below reproduce the recovered arithmetic, string constants and
// call arguments; the CRT calls are recorded as [`A79FileCall`] values so the
// exact arguments stay testable instead of being guessed.
//
// Nothing here touches `sim` state: the batches own their own types, so no
// simulation record gains a field.

// ---------------------------------------------------------------------------
// Shared constants
// ---------------------------------------------------------------------------

/// Size of one `0x1c`-byte string record.
pub const A79_STRING_SIZE: u32 = 0x1c;
/// Inline capacity word written by every string reset: bit 31 marks the inline
/// buffer, the low bits hold the 15-byte capacity.
pub const A79_INLINE_CAPACITY: u32 = 0x8000_000f;
/// Bit 31 of the capacity word, the "data lives in the inline buffer" marker.
pub const A79_INLINE_MARKER: u32 = 0x8000_0000;
/// Number of `0x1c` records in one deque block (`0x00631222` allocates
/// `0xe0` bytes = 8 * 0x1c).
pub const A79_DEQUE_FANOUT: u32 = 8;
/// Bytes in one deque block: `0xe0`.
pub const A79_DEQUE_BLOCK_BYTES: u32 = A79_DEQUE_FANOUT * A79_STRING_SIZE;
/// Divisor applied to the recovered frame deltas (`0x006318f0`).
pub const A79_TIMESTAMP_DIVISOR: u64 = 1000;
/// Byte offset of the key-down counter table inside its owning global
/// (`0x00631320`: `*(u32 *)(base + 0xe628)` and `+0xe62c`).
pub const A79_KEY_TABLE_OFFSET: u32 = 0xe628;
/// The two dwords `0x00631320` writes: eight bytes of `3`.
pub const A79_KEY_TABLE_WORDS: [u32; 2] = [0x0303_0303, 0x0303_0303];
/// Number of bytes those two stores cover: `0xe628..=0xe62f`.
///
/// `0x00631380` applies no bounds check, but these are the only bytes the
/// window's rows prove; the record is therefore exactly this range.
pub const A79_KEY_TABLE_LEN: usize = 4 * A79_KEY_TABLE_WORDS.len();
/// `DAT_00f063cc`, xor-ed with [`A79_KEY_TABLE_XOR`] to obtain the key table's
/// owning global address.
pub const A79_KEY_TABLE_GLOBAL: u32 = 0x00f0_63cc;
/// `0x10fae560`, the second operand of that xor.
pub const A79_KEY_TABLE_XOR: u32 = 0x10fa_e560;
/// Byte offset of the "live" byte the key-state predicates read
/// (`DAT_0119f7bc`).
pub const A79_KEY_GATE_BYTE: u32 = 0x0119_f7bc;

/// `0x00630890`/`0x00630892`.
pub const A79_POSITION_QUERY_ARGS: (u32, u32) = (0, 0);
/// First argument of every recovered `open` call (`0x00630d20`,
/// `0x00631e00`): `0x40000000`.
pub const A79_OPEN_FLAGS: u32 = 0x4000_0000;
/// CRT buffer size passed to `open` (`0x80`).
pub const A79_OPEN_BUFFER: u32 = 0x80;
/// Share word of the performance-report `open` calls (`0x00631e00`,
/// `0x00631ee8`): `4`.
pub const A79_REPORT_OPEN_SHARE: u32 = 4;
/// First `open` mode the report tries: `3`.
pub const A79_REPORT_MODE_FIRST: u32 = 3;
/// Second `open` mode the report tries: `2`.
pub const A79_REPORT_MODE_SECOND: u32 = 2;
/// Format string of the failure message (`0x00631e00`).
pub const A79_REPORT_MESSAGE_FORMAT: &str = "Could not open file: {0}";
/// Length argument handed to the message builder with that format: `0x18`.
pub const A79_REPORT_MESSAGE_FORMAT_LEN: u32 = 0x18;
/// Caption of the failure message box.
pub const A79_REPORT_CAPTION: &str = "Performance Report Generation error";
/// Third argument of the message-box call: `4`.
pub const A79_REPORT_MESSAGE_FLAGS: u32 = 4;
/// Format string of the backend line.
pub const A79_REPORT_API_FORMAT: &str = "Rendering backend API: {0}";
/// Length argument handed to the string builder with that format: `0x1a`.
pub const A79_REPORT_API_FORMAT_LEN: u32 = 0x1a;
/// The `0xd` third/fourth argument both format-builder calls carry.
pub const A79_REPORT_FORMAT_METADATA: u32 = 0xd;
/// Stand-in for the CRT handle the report loop obtains from `open`; retail
/// compares it with `-1` only.
pub const A79_REPORT_HANDLE: i32 = 1;

// ---------------------------------------------------------------------------
// The 0x1c-byte string record: 0x00630be6, 0x00630bf2
// ---------------------------------------------------------------------------

/// One `0x1c`-byte string: `+0x00` data pointer, `+0x04` size, `+0x08`
/// capacity, `+0x0c..=+0x1b` the sixteen inline bytes.
///
/// `heap == None` models the pointer at `+0x00` pointing at the record's own
/// `+0x0c` buffer; `Some(bytes)` models a heap block.  The capacity word is
/// kept verbatim, because the recovered routines test and combine bit 31
/// rather than a separate flag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A79String {
    /// `+0x00`: `None` = points at the inline buffer at `+0x0c`.
    pub heap: Option<Vec<u8>>,
    /// `+0x04`: size in bytes.
    pub size: u32,
    /// `+0x08`: capacity word; bit 31 is the inline marker.
    pub capacity: u32,
    /// `+0x0c..=+0x1b`: inline buffer (15 bytes plus a terminator).
    pub inline: [u8; 16],
}

impl A79String {
    /// The reset every string routine writes: `size = 0`, `data = inline`,
    /// `capacity = 0x8000_000f`, first inline byte `0`.
    pub fn a79_new() -> Self {
        A79String {
            heap: None,
            size: 0,
            capacity: A79_INLINE_CAPACITY,
            inline: [0; 16],
        }
    }

    /// Build a record holding `bytes`, inline when they fit the 15-byte
    /// buffer, on the heap otherwise (capacity = exact length, marker clear).
    pub fn a79_from_bytes(bytes: &[u8]) -> Self {
        if bytes.len() <= 15 {
            let mut record = A79String::a79_new();
            record.inline[..bytes.len()].copy_from_slice(bytes);
            record.size = bytes.len() as u32;
            record
        } else {
            A79String {
                heap: Some(bytes.to_vec()),
                size: bytes.len() as u32,
                capacity: bytes.len() as u32,
                inline: [0; 16],
            }
        }
    }

    /// The bytes the data pointer addresses.
    pub fn a79_bytes(&self) -> &[u8] {
        match &self.heap {
            Some(bytes) => bytes,
            None => &self.inline[..(self.size as usize).min(16)],
        }
    }

    /// The same bytes as a UTF-8 string (the recovered records are ASCII).
    pub fn a79_text(&self) -> String {
        String::from_utf8_lossy(self.a79_bytes()).into_owned()
    }

    /// `capacity & 0x8000_0000 != 0`: the record owns no heap block.
    pub fn a79_is_inline(&self) -> bool {
        self.capacity & A79_INLINE_MARKER != 0
    }

    /// The reset stored by the moved-from half of `0x00630be6`/`0x00630bf2`
    /// (`size = 0`, `data = inline`, `capacity = 0x8000_000f`, `inline[0] = 0`).
    pub fn a79_reset(&mut self) {
        self.heap = None;
        self.size = 0;
        self.capacity = A79_INLINE_CAPACITY;
        self.inline = [0; 16];
    }

    /// `0x00630be6`: move assignment of a heap-mode source into an inline-mode
    /// destination.
    ///
    /// The recovered body stores the destination's three record words at
    /// `EBX + 4` (data), `EBX + 8` (size) and `EBX + 0xc` (capacity) — the
    /// path record of the log stream — and the source through `EAX`:
    ///
    /// ```text
    /// iVar1 = src[2];                          /* source capacity word */
    /// if (iVar1 < 0) {                         /* source lives inline */
    ///   local = src[0] - dest.size;            /* reserve-failure report */
    ///   FUN_00538810(src[1]);                  /* noreturn, source size */
    /// }
    /// if (-1 < (int)dest.capacity) FUN_0093ec70();   /* dest must be inline */
    /// uVar2 = dest.capacity & 0x7fffffff;
    /// dest.data = src[0];
    /// dest.capacity = uVar2;
    /// dest.size = src[1];
    /// dest.capacity = (src[2] ^ uVar2) & 0x7fffffff ^ uVar2;
    /// src[1] = 0; src[0] = (int)(src + 3);
    /// src[2] = 0x8000000f; src.inline[0] = 0;
    /// ```
    ///
    /// The capacity expression is the transfer formula of batch 41: with an
    /// inline destination (`uVar2 = 0xf`) and a heap source it yields the
    /// source capacity with bit 31 clear.
    pub fn a79_steal(&mut self, src: &mut A79String) -> Result<(), A79StealFault> {
        if src.a79_is_inline() {
            return Err(A79StealFault::SourceInline { size: src.size });
        }
        if !self.a79_is_inline() {
            return Err(A79StealFault::DestinationHeap {
                capacity: self.capacity,
            });
        }
        let low = self.capacity & !A79_INLINE_MARKER;
        let capacity = (src.capacity ^ low) & !A79_INLINE_MARKER ^ low;
        self.heap = src.heap.take();
        self.size = src.size;
        self.capacity = capacity;
        src.a79_reset();
        Ok(())
    }

    /// `0x00630bf2`: the same transfer, entered through a flag byte.
    ///
    /// The row reaches the transfer through `if (in_OF != in_SF)`, i.e. the
    /// caller's comparison said "less", and then passes the source size to the
    /// reserve path.  Reproduced as an extra `flagged` guard in front of the
    /// [`A79String::a79_steal`] body; `ready` is that comparison result.
    pub fn a79_steal_flagged(
        &mut self,
        src: &mut A79String,
        ready: bool,
    ) -> Result<(), A79StealFault> {
        if !ready {
            return Err(A79StealFault::FlaggedNotReady { size: src.size });
        }
        self.a79_steal(src)
    }
}

/// Why a `0x1c` string transfer stopped in the reserve/invalid-parameter path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A79StealFault {
    /// `0x00630be6`: `if (src->capacity < 0) FUN_00538810(src->size)`.  The
    /// recovered report also forms `src->data - dest->size`; the model has no
    /// raw pointers, so only the size is carried.
    SourceInline { size: u32 },
    /// `0x00630be6`: `if (dest->capacity >= 0) FUN_0093ec70()`.
    DestinationHeap { capacity: u32 },
    /// `0x00630bf2`: the flag byte said the transfer was not ready.
    FlaggedNotReady { size: u32 },
}

// ---------------------------------------------------------------------------
// Block-backed deque of 0x1c strings: 0x00630940, 0x00630a10, 0x00631120,
// 0x00631222
// ---------------------------------------------------------------------------

/// A deque whose storage is an array of blocks, each holding eight `0x1c`
/// records (`0xe0` bytes).
///
/// The four header dwords are `+0x00` block array, `+0x04` block capacity,
/// `+0x08` front index and `+0x0c` size.  A logical index `i` lives in block
/// `(capacity - 1) & (i >> 3)` at slot `i & 7`, which is exactly what
/// `0x00631120`, `0x00630940` and `0x00630ea0` compute.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A79Ring {
    /// `+0x00`: block array; `None` models a null slot.
    pub blocks: Vec<Option<Vec<A79String>>>,
    /// `+0x04`: number of block slots.
    pub capacity: u32,
    /// `+0x08`: front index.
    pub front: u32,
    /// `+0x0c`: element count.
    pub size: u32,
}

impl A79Ring {
    /// An empty deque with `capacity` block slots.
    pub fn a79_new(capacity: u32) -> Self {
        A79Ring {
            blocks: vec![None; capacity as usize],
            capacity,
            front: 0,
            size: 0,
        }
    }

    /// Block slot of logical index `index`.
    pub fn a79_block_index(&self, index: u32) -> u32 {
        self.capacity.wrapping_sub(1) & (index >> 3)
    }

    /// Byte offset of logical index `index` inside its block.
    pub fn a79_element_offset(index: u32) -> u32 {
        (index & 7) * A79_STRING_SIZE
    }

    /// The next block capacity `0x00631120` computes before it gives up:
    /// start at the current capacity (or 8 when it is zero) and double while
    /// the value is still below `capacity + 1`.
    pub fn a79_grown_capacity(&self) -> u32 {
        let mut value = if self.capacity != 0 { self.capacity } else { 8 };
        while value < self.capacity.wrapping_add(1) {
            value = value.wrapping_mul(2);
        }
        value
    }

    /// `0x00631120`: reserve the slot for one more element and return
    /// `(index, block slot, byte offset)`.
    ///
    /// ```text
    /// if (((front + size) & 7) == 0 && capacity <= (size + 8) >> 3) grow();
    /// index = front + size;
    /// size = size + 1;
    /// block = blocks[(capacity - 1) & (index >> 3)];   /* must be non-null */
    /// return block + (index & 7) * 0x1c;
    /// ```
    ///
    /// The grow tail is the non-returning reserve path; its computed value is
    /// reported as [`A79RingFault::NeedsGrowth`].
    pub fn a79_reserve_slot(&mut self) -> Result<A79Slot, A79RingFault> {
        let index = self.front.wrapping_add(self.size);
        if index & 7 == 0 && self.capacity <= (self.size.wrapping_add(8)) >> 3 {
            return Err(A79RingFault::NeedsGrowth {
                capacity: self.a79_grown_capacity(),
            });
        }
        self.size = self.size.wrapping_add(1);
        let block = self.a79_block_index(index);
        let slot = A79Slot {
            index,
            block,
            offset: Self::a79_element_offset(index),
        };
        if self
            .blocks
            .get(block as usize)
            .map_or(true, |b| b.is_none())
        {
            return Err(A79RingFault::NullBlock { block });
        }
        Ok(slot)
    }

    /// `0x00631222`: allocate the missing `0xe0`-byte block for `index` and
    /// return `(allocated bytes, byte offset of the element)`.
    ///
    /// ```text
    /// uVar1 = (*vftable[1])(0xe0, 8);        /* alignment 8 */
    /// blocks[block] = uVar1;
    /// return uVar1 + (index & 7) * 0x1c;
    /// ```
    ///
    /// The recovered head `*in_EAX = *in_EAX ^ (byte)in_EAX` is an instruction
    /// the decompiler split; the allocation tail is coherent.
    pub fn a79_allocate_block(&mut self, index: u32) -> Result<A79BlockAllocation, A79RingFault> {
        let block = self.a79_block_index(index);
        let Some(slot) = self.blocks.get_mut(block as usize) else {
            return Err(A79RingFault::NullBlock { block });
        };
        *slot = Some(vec![A79String::a79_new(); A79_DEQUE_FANOUT as usize]);
        Ok(A79BlockAllocation {
            bytes: A79_DEQUE_BLOCK_BYTES,
            block,
            offset: Self::a79_element_offset(index),
        })
    }

    /// `0x00630940`: validate that every element in `[front, front + size)`
    /// still carries the inline capacity word.
    ///
    /// ```text
    /// for (i = front; i != front + size; ++i)
    ///   if (*(int *)(blocks[(capacity - 1) & (i >> 3)] + 8 + (i & 7) * 0x1c) >= 0)
    ///     FUN_0093ec70();                  /* no-heap invariant violated */
    /// if (capacity == 0) FUN_0093ec70();
    /// ```
    pub fn a79_check_range(&self) -> Result<(), A79CheckFault> {
        let end = self.front.wrapping_add(self.size);
        let mut index = self.front;
        while index != end {
            let block = self.a79_block_index(index);
            let offset = Self::a79_element_offset(index) as usize;
            let element = self
                .blocks
                .get(block as usize)
                .and_then(|b| b.as_ref())
                .ok_or(A79CheckFault::NullBlock {
                    block,
                    index: index & 7,
                })?;
            let capacity = element
                .get((offset / A79_STRING_SIZE as usize) as usize)
                .map_or(0, |e| e.capacity);
            if capacity & A79_INLINE_MARKER == 0 {
                return Err(A79CheckFault::HeapElement { index, capacity });
            }
            index = index.wrapping_add(1);
        }
        if self.capacity == 0 {
            return Err(A79CheckFault::EmptyRing);
        }
        Ok(())
    }

    /// `0x00630a10`: the second copy of the range check (the window carries
    /// the same seven-instruction body twice, sizes 156 and 5).
    pub fn a79_check_range_tail(&self) -> Result<(), A79CheckFault> {
        self.a79_check_range()
    }
}

/// Slot selected by [`A79Ring::a79_reserve_slot`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A79Slot {
    /// Logical index `front + size`.
    pub index: u32,
    /// Block slot `(capacity - 1) & (index >> 3)`.
    pub block: u32,
    /// Byte offset `(index & 7) * 0x1c`.
    pub offset: u32,
}

/// Result of [`A79Ring::a79_allocate_block`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A79BlockAllocation {
    /// Bytes the allocator is asked for: `0xe0`.
    pub bytes: u32,
    /// Block slot filled in.
    pub block: u32,
    /// Byte offset of the element inside the new block.
    pub offset: u32,
}

/// Why a deque operation stopped in a non-returning path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A79RingFault {
    /// `0x00631120`: the block array must grow to this capacity first.
    NeedsGrowth { capacity: u32 },
    /// The block slot is null.
    NullBlock { block: u32 },
}

/// Why [`A79Ring::a79_check_range`] traps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A79CheckFault {
    /// `0x00630940`: `if (capacity == 0) FUN_0093ec70()`.
    EmptyRing,
    /// An element's capacity word lost bit 31.
    HeapElement { index: u32, capacity: u32 },
    /// A block slot of an in-range element is null.
    NullBlock { block: u32, index: u32 },
}

// ---------------------------------------------------------------------------
// The log stream, 0x00630870..=0x00630ea0
// ---------------------------------------------------------------------------

/// One recovered call into the CRT file layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum A79FileCall {
    /// `FUN_0093ffe0(handle, 0, 0)` — commit (`0x00630870`).
    Commit {
        handle: i32,
        offset: u32,
        origin: u32,
    },
    /// `FUN_00940060(handle, 0, 0, &raw)` — position query (`0x00630890`).
    QueryPosition {
        handle: i32,
        offset: u32,
        origin: u32,
    },
    /// `FUN_0093f610(&raw, &out)` — widen the queried value (`0x00630890`).
    WidenPosition { raw: u64 },
    /// `FUN_0093fd10(path, 0x40000000, share, mode, 0x80)` (`0x00630d20`).
    Open {
        path: String,
        flags: u32,
        share: u32,
        mode: u32,
        buffer: u32,
    },
    /// `FUN_009406d0(handle, 0, 0, 2)` — seek to the end (`0x00630d20`).
    SeekEnd {
        handle: i32,
        offset: u32,
        origin: u32,
    },
    /// `FUN_00940720(handle, ptr, len, &out)` — one write (`0x00630ea0`).
    Write { handle: i32, len: u32 },
    /// `FUN_0093fe80(handle)` — flush (`0x00630a20`, `0x00630ab0`).
    Flush { handle: i32 },
    /// `FUN_0093fb70(handle)` — close (`0x00630a20`, `0x00630ea0`).
    Close { handle: i32 },
}

/// `0x00630870`: commit the stream the handle pointer addresses.
///
/// ```text
/// FUN_0093ffe0(*param_1, 0, 0);
/// ```
pub fn a79_commit(handle: i32) -> A79FileCall {
    A79FileCall::Commit {
        handle,
        offset: A79_POSITION_QUERY_ARGS.0,
        origin: A79_POSITION_QUERY_ARGS.1,
    }
}

/// `0x00630890`: read the 64-bit position of the stream the handle pointer
/// addresses.
///
/// ```text
/// uStack_c = 0;
/// iVar1 = FUN_00940060(*param_1, 0, 0, &raw);
/// if (iVar1 != 0) FUN_0093f610(&raw, &uStack_c);
/// return uStack_c;
/// ```
///
/// A failed query leaves the zero-initialised result untouched.
pub fn a79_read_position(handle: i32, query_ok: bool, raw: u64) -> (u64, Vec<A79FileCall>) {
    let mut calls = vec![A79FileCall::QueryPosition {
        handle,
        offset: A79_POSITION_QUERY_ARGS.0,
        origin: A79_POSITION_QUERY_ARGS.1,
    }];
    if query_ok {
        calls.push(A79FileCall::WidenPosition { raw });
        (raw, calls)
    } else {
        (0, calls)
    }
}

/// `0x00630892`: the same query, with the handle reached through a pointer to
/// the handle slot (`**(undefined4 **)(ebp + 8)`) and the result returned from
/// the caller's stack slot at `ebp - 8`.
pub fn a79_read_position_indirect(
    handle_slot: &i32,
    query_ok: bool,
    raw: u64,
) -> (u64, Vec<A79FileCall>) {
    a79_read_position(*handle_slot, query_ok, raw)
}

/// `0x00630d9a`: the seek tail of the open routine, split off by the
/// decompiler.
///
/// The body is `FUN_009406d0();` with the handle and arguments still in the
/// caller's registers; the only open site that reaches it passes
/// `(handle, 0, 0, 2)`.  The trailing cookie test is the epilogue's
/// `_invalid_parameter_noinfo_noreturn` guard.
pub fn a79_seek_end_call(handle: i32) -> A79FileCall {
    A79FileCall::SeekEnd {
        handle,
        offset: 0,
        origin: 2,
    }
}

/// `OsFileLogMarshaller`: the stream record the window's destructor names.
///
/// Layout: `+0x00` vftable, `+0x04` path string (`0x1c` record: data at
/// `+0x04`, size at `+0x08`, capacity at `+0x0c`), `+0x20` handle, `+0x24`
/// append byte, `+0x28` the pending deque.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A79LogStream {
    /// Path record at `+0x04`.
    pub path: A79String,
    /// `+0x20`: handle, `-1` when closed.
    pub handle: i32,
    /// `+0x24`: the mode byte `FUN_00630b10` stores and `0x00630ea0` tests.
    pub append: bool,
    /// `+0x28`: queued records, written out by `0x00630ea0`.
    pub pending: A79Ring,
    /// Every CRT call in the order the recovered bodies emit it.
    pub calls: Vec<A79FileCall>,
    /// Bytes the writes delivered (the model of the opened file).
    pub written: Vec<u8>,
    /// Next handle the model hands out; retail only compares with `-1`.
    pub next_handle: i32,
}

impl A79LogStream {
    /// A closed stream with no queued records.
    pub fn a79_new(path: &str) -> Self {
        A79LogStream {
            path: A79String::a79_from_bytes(path.as_bytes()),
            handle: -1,
            append: false,
            pending: A79Ring::a79_new(8),
            calls: Vec::new(),
            written: Vec::new(),
            next_handle: 0x100,
        }
    }

    /// `0x00630d20`: open (or reopen) the stream.
    ///
    /// ```text
    /// FUN_0074f3f0(tmp, this + 4, cookie);        /* build the path */
    /// FUN_0074f0a0(tmp);
    /// uVar1 = FUN_0093fd10(*(u32 *)(this + 4), 0x40000000, 0, (param ^ 1) * 2 + 2, 0x80);
    /// *(u32 *)(this + 0x20) = uVar1;
    /// if (param == 0) FUN_009406d0(uVar1, 0, 0, 2);
    /// ```
    ///
    /// `param` is the append flag: `1` gives mode `2` and no seek, `0` gives
    /// mode `4` followed by a seek to the end.
    pub fn a79_open(&mut self, append: bool) -> i32 {
        let mode = ((append as u32) ^ 1) * 2 + 2;
        self.calls.push(A79FileCall::Open {
            path: self.path.a79_text(),
            flags: A79_OPEN_FLAGS,
            share: 0,
            mode,
            buffer: A79_OPEN_BUFFER,
        });
        let handle = self.next_handle;
        self.next_handle = self.next_handle.wrapping_add(1);
        self.handle = handle;
        if !append {
            self.calls.push(a79_seek_end_call(handle));
        }
        handle
    }

    /// `0x00630ab0`: flush the handle at `+0x20`.
    ///
    /// ```text
    /// FUN_0093fe80(*(undefined4 *)(this + 0x20));
    /// ```
    pub fn a79_flush(&mut self) -> A79FileCall {
        let call = A79FileCall::Flush {
            handle: self.handle,
        };
        self.calls.push(call.clone());
        call
    }

    /// `0x00630a20`: the destructor sequence.
    ///
    /// ```text
    /// *this = OsFileLogMarshaller::vftable;
    /// FUN_0093fe80(this[8]);                    /* flush, even when closed */
    /// if (this[8] != -1) { FUN_0093fb70(this[8]); this[8] = -1; }
    /// FUN_00630940();                           /* the deque range check */
    /// if (-1 < (int)this[3]) FUN_0093ec70();    /* path capacity at +0xc */
    /// ```
    pub fn a79_teardown(&mut self) -> Result<(), A79TeardownFault> {
        self.calls.push(A79FileCall::Flush {
            handle: self.handle,
        });
        if self.handle != -1 {
            self.calls.push(A79FileCall::Close {
                handle: self.handle,
            });
            self.handle = -1;
        }
        if let Err(fault) = self.pending.a79_check_range() {
            return Err(A79TeardownFault::Ring(fault));
        }
        if !self.path.a79_is_inline() {
            return Err(A79TeardownFault::HeapPathCapacity {
                capacity: self.path.capacity,
            });
        }
        Ok(())
    }

    /// The four-instruction tail shared by `0x00630cb4` and `0x00630cd6`:
    ///
    /// ```text
    /// if (*(int *)(this + 8) != 0 && *(char *)(this + 0x24) != 0)
    ///     FUN_00630d20(1);
    /// ```
    pub fn a79_reopen_append(&mut self) -> Option<i32> {
        if self.path.size != 0 && self.append {
            Some(self.a79_open(true))
        } else {
            None
        }
    }

    /// `0x00630cb4`: release the formatted record through its `+0x10` slot,
    /// check the released capacity word, then run the reopen tail.
    ///
    /// The row's `if ((*(uint *)(ebp - 0x24) & 0x80000000) == 0)
    /// FUN_0093ec70()` is the destroyed temporary's capacity invariant.
    pub fn a79_release_and_reopen(
        &mut self,
        record: &mut A79String,
    ) -> Result<Option<i32>, A79TeardownFault> {
        if !record.a79_is_inline() {
            return Err(A79TeardownFault::HeapPathCapacity {
                capacity: record.capacity,
            });
        }
        record.a79_reset();
        Ok(self.a79_reopen_append())
    }

    /// `0x00630cd6`: the same release and reopen tail without the extra
    /// capacity test (the row consists of the virtual `+0x10` call and the
    /// four-instruction tail only).
    pub fn a79_release_and_reopen_tail(&mut self, record: &mut A79String) -> Option<i32> {
        record.a79_reset();
        self.a79_reopen_append()
    }

    /// `0x00630ea0`: write one record, flushing the queued ones first.
    ///
    /// ```text
    /// if (append_byte == 0 && path.size != 0) FUN_00630d20(0);
    /// if (handle == -1) { ... FUN_00538810(strlen(param_1)); }
    /// while (pending.size != 0) {
    ///   element = blocks[(cap - 1) & (front >> 3)] + (front & 7) * 0x1c;
    ///   FUN_00940720(handle, element->data, element->size, &out);
    ///   if (element->capacity >= 0) FUN_0093ec70();
    ///   --pending.size; ++pending.front;
    /// }
    /// FUN_00940720(handle, param_1, param_2, &out);
    /// if (append_byte == 0) { FUN_0093fe80(handle); close(handle); handle = -1; }
    /// ```
    pub fn a79_write(&mut self, data: &[u8]) -> Result<(), A79WriteFault> {
        if !self.append && self.path.size != 0 {
            self.a79_open(false);
        }
        if self.handle == -1 {
            return Err(A79WriteFault::Closed {
                size: data.len() as u32,
            });
        }
        while self.pending.size != 0 {
            let index = self.pending.front;
            let block = self.pending.a79_block_index(index);
            let slot = (index & 7) as usize;
            let (len, capacity, bytes) = self
                .pending
                .blocks
                .get(block as usize)
                .and_then(|b| b.as_ref())
                .and_then(|b| b.get(slot))
                .map(|element| (element.size, element.capacity, element.a79_bytes().to_vec()))
                .ok_or(A79WriteFault::NullBlock { block })?;
            self.calls.push(A79FileCall::Write {
                handle: self.handle,
                len,
            });
            self.written.extend_from_slice(&bytes);
            if capacity & A79_INLINE_MARKER == 0 {
                return Err(A79WriteFault::HeapPending { index, capacity });
            }
            self.pending.front = index.wrapping_add(1);
            self.pending.size -= 1;
        }
        self.calls.push(A79FileCall::Write {
            handle: self.handle,
            len: data.len() as u32,
        });
        self.written.extend_from_slice(data);
        if !self.append {
            self.calls.push(A79FileCall::Flush {
                handle: self.handle,
            });
            self.calls.push(A79FileCall::Close {
                handle: self.handle,
            });
            self.handle = -1;
        }
        Ok(())
    }

    /// `0x00630df0`: format the caller's arguments into a `0x1c` string
    /// (`FUN_00c6c400`, outside the window) and hand pointer plus size to
    /// `0x00630ea0`.
    ///
    /// ```text
    /// FUN_00c6c400(&record, param_2, &stack + 0xc, cookie);
    /// FUN_00630ea0(record.data, record.size);
    /// ```
    pub fn a79_write_formatted(&mut self, text: &[u8]) -> Result<(), A79WriteFault> {
        self.a79_write(text)
    }
}

/// Why a write stopped in a non-returning path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A79WriteFault {
    /// `0x00630ea0`: the handle was `-1`; retail unwinds through the reserve
    /// path with the record's length.
    Closed { size: u32 },
    /// A queued element's capacity word lost bit 31 after its write.
    HeapPending { index: u32, capacity: u32 },
    /// A queued element's block slot is null.
    NullBlock { block: u32 },
}

/// Why [`A79LogStream::a79_teardown`] traps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A79TeardownFault {
    Ring(A79CheckFault),
    /// `0x00630a20`: `if (-1 < (int)this[3]) FUN_0093ec70()`.
    HeapPathCapacity {
        capacity: u32,
    },
}

/// `0x00630ac0`: build the empty inline string and unwind with the dword at
/// `+0x08`.
///
/// ```text
/// uVar1 = *(undefined4 *)(this + 8);
/// dst[1] = 0; *dst = dst + 3; dst[2] = 0x8000000f; dst[3].byte[0] = 0;
/// FUN_00538810(uVar1);                      /* noreturn */
/// ```
pub fn a79_error_unwind(code: u32) -> A79Unwind {
    A79Unwind {
        message: A79String::a79_new(),
        code,
    }
}

/// Result of [`a79_error_unwind`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A79Unwind {
    /// The string the row leaves behind: the inline-empty reset.
    pub message: A79String,
    /// `*(u32 *)(this + 8)`, the value the reserve path receives.
    pub code: u32,
}

// ---------------------------------------------------------------------------
// Key-down table: 0x00631260, 0x006312f0, 0x00631300, 0x00631320,
// 0x00631340, 0x00631380, 0x006313a0, 0x00631400
// ---------------------------------------------------------------------------

/// The eight key-down counters and the three module flags the window's
/// predicates read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A79KeyState {
    /// The bytes at `base + 0xe628`; `0x00631320` writes them all as `3`.
    pub table: [u8; A79_KEY_TABLE_LEN],
    /// `DAT_00faa1cc`.
    pub initialised: u32,
    /// `DAT_0119f7bc`: the byte that gates the decay and the language probe.
    pub gate: bool,
    /// `DAT_011a4580`, read only by `0x006313a0`.
    pub audio_present: bool,
    /// `DAT_00faa1d4`, read only by `0x006313a0`.
    pub muted: bool,
}

impl A79KeyState {
    /// All counters zero and all flags clear.
    pub fn a79_new() -> Self {
        A79KeyState {
            table: [0; A79_KEY_TABLE_LEN],
            initialised: 0,
            gate: false,
            audio_present: false,
            muted: false,
        }
    }

    /// Install the two `0x03030303` dwords `0x00631320` stores.
    pub fn a79_install_default_table(&mut self) {
        self.table = a79_default_key_table();
    }

    /// `0x006312f0`: `return DAT_00faa1cc;`
    pub fn a79_initialised(&self) -> u32 {
        self.initialised
    }

    /// `0x00631300`: `return (DAT_00faa1cc != 0) && (DAT_0119f7bc == 0);`
    pub fn a79_flag_state(&self) -> u32 {
        if self.initialised != 0 && !self.gate {
            1
        } else {
            0
        }
    }

    /// `0x00631380`: `return *(u8 *)(base + 0xe628 + index);`
    ///
    /// The shipped body has no bounds check.
    pub fn a79_get(&self, index: u32) -> u8 {
        self.table[index as usize]
    }

    /// `0x00631400`: decrement one counter when the gate byte is set and the
    /// counter is non-zero.
    ///
    /// ```text
    /// if (DAT_0119f7bc != 0) {
    ///   c = *(char *)(base + 0xe628 + index);
    ///   if (c != 0) *(char *)(base + 0xe628 + index) = c - 1;
    /// }
    /// ```
    pub fn a79_decay(&mut self, index: u32) -> Option<u8> {
        if !self.gate {
            return None;
        }
        let current = self.table[index as usize];
        if current == 0 {
            return None;
        }
        let next = current - 1;
        self.table[index as usize] = next;
        Some(next)
    }

    /// `0x00631340`: initialise once.
    ///
    /// ```text
    /// if (DAT_00faa1cc == 0) { DAT_00faa1cc = 1; FUN_007a89e0(); FUN_0093f6c0(); }
    /// ```
    ///
    /// `true` reports the branch that ran the two calls.
    pub fn a79_init_once(&mut self) -> bool {
        if self.initialised != 0 {
            return false;
        }
        self.initialised = 1;
        true
    }

    /// `0x006313a0`: release the module.
    ///
    /// ```text
    /// if (DAT_00faa1cc == 0) return;
    /// if (param_1 != 0) FUN_0093f6c0();          /* noreturn */
    /// DAT_00faa1cc = 0;
    /// if (DAT_011a4580 != 0 && DAT_00faa1d4 == 0) FUN_0075c110();
    /// ```
    pub fn a79_shutdown(&mut self, force: bool) -> Result<A79Shutdown, A79ShutdownFault> {
        if self.initialised == 0 {
            return Ok(A79Shutdown::Idle);
        }
        if force {
            return Err(A79ShutdownFault::Forced);
        }
        self.initialised = 0;
        if self.audio_present && !self.muted {
            Ok(A79Shutdown::Released { resume_audio: true })
        } else {
            Ok(A79Shutdown::Released {
                resume_audio: false,
            })
        }
    }
}

/// `0x00631320`: the default key table.
///
/// ```text
/// uVar1 = DAT_00f063cc ^ 0x10fae560;
/// *(u32 *)(uVar1 + 0xe628) = 0x03030303;
/// *(u32 *)(uVar1 + 0xe62c) = 0x03030303;
/// ```
pub fn a79_default_key_table() -> [u8; A79_KEY_TABLE_LEN] {
    let mut table = [0u8; A79_KEY_TABLE_LEN];
    for (offset, word) in A79_KEY_TABLE_WORDS.iter().enumerate() {
        let start = offset * 4;
        table[start..start + 4].copy_from_slice(&word.to_le_bytes());
    }
    table
}

/// The base the key table lives at: `DAT_00f063cc ^ 0x10fae560`.
pub fn a79_key_table_base() -> u32 {
    A79_KEY_TABLE_GLOBAL ^ A79_KEY_TABLE_XOR
}

/// Outcome of [`A79KeyState::a79_shutdown`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A79Shutdown {
    /// `DAT_00faa1cc` was already zero: nothing happens.
    Idle,
    /// The flag was cleared; `resume_audio` reports the `FUN_0075c110` call.
    Released { resume_audio: bool },
}

/// Why [`A79KeyState::a79_shutdown`] does not return.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A79ShutdownFault {
    /// `if (param_1 != 0) FUN_0093f6c0();`
    Forced,
}

/// `0x00631260`: the language/UI probe gate.
///
/// ```text
/// if (DAT_0119f7bc == 0) return 1;
/// b = FUN_00722450();
/// if (b < 9) return 0;
/// FUN_00748d40(param_1);                     /* noreturn */
/// ```
///
/// `probe` is the value the out-of-window `FUN_00722450` returns.
pub fn a79_language_gate(gate: bool, probe: u8, arg: u32) -> A79LanguageGate {
    if !gate {
        return A79LanguageGate::Allowed;
    }
    if probe < 9 {
        return A79LanguageGate::Rejected;
    }
    A79LanguageGate::Report { arg }
}

/// Result of [`a79_language_gate`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A79LanguageGate {
    /// The gate byte is clear: the probe returns 1.
    Allowed,
    /// The probe was below 9: the function returns 0.
    Rejected,
    /// The report path `FUN_00748d40(param_1)` runs.
    Report { arg: u32 },
}

// ---------------------------------------------------------------------------
// Heap sort: 0x00631790, 0x0063159f
// ---------------------------------------------------------------------------

/// `0x00631790`: the adjust-heap sift used by both sort routines.
///
/// ```text
/// for (;;) {
///   left = hole * 2; right = left + 2;
///   if (len <= right) break;
///   child = left + 1;
///   if (values[right - 1] <= values[right]) child = right;
///   values[hole] = values[child];
///   hole = child;
/// }
/// if (right == len) { values[hole] = values[right - 1]; hole = left + 1; }
/// if (hole <= start) { values[hole] = key; return; }
/// do {
///   parent = (hole - 1) >> 1;
///   if (key <= values[parent]) break;
///   values[hole] = values[parent];
///   hole = parent;
/// } while (start < parent);
/// values[hole] = key;
/// ```
///
/// `start` is the call's own hole index, which is also the push-up limit.
pub fn a79_adjust_heap(values: &mut [u32], hole: usize, len: usize, key: u32) {
    let start = hole;
    let mut hole = hole;
    loop {
        let left = hole * 2;
        let right = left + 2;
        if len <= right {
            break;
        }
        let mut child = left + 1;
        if values[right - 1] <= values[right] {
            child = right;
        }
        values[hole] = values[child];
        hole = child;
    }
    let right = hole * 2 + 2;
    if right == len {
        values[hole] = values[right - 1];
        hole = right - 1;
    }
    if hole <= start {
        values[hole] = key;
        return;
    }
    loop {
        let parent = ((hole as i32 - 1) >> 1) as usize;
        let parent_value = values[parent];
        if key <= parent_value {
            break;
        }
        values[hole] = parent_value;
        hole = parent;
        if !(start < parent) {
            break;
        }
    }
    values[hole] = key;
}

/// `0x0063159f`: `__make_heap` followed by `__sort_heap`.
///
/// The row is a fragment whose registers are `unaff_*` (it opens with a wild
/// global byte add the decompiler misdecoded).  What survives is the two-loop
/// shape that `FUN_00631430` carries inline:
///
/// ```text
/// for (parent = len / 2 - 1; ; --parent) {
///   key = first[parent];
///   FUN_00631790(first, parent, len, &key);
///   if (parent == 0) break;
/// }
/// while (last - first > 1) {
///   --last;
///   key = *last; *last = *first;
///   FUN_00631790(first, 0, last - first, &key);
/// }
/// ```
///
/// The start index is `floor(len / 2) - 1`, the same value `FUN_00631430`
/// computes as `(byte_length >> 3) - 1`.
pub fn a79_heap_sort(values: &mut [u32]) {
    let len = values.len();
    if len < 2 {
        return;
    }
    let mut parent = len / 2 - 1;
    loop {
        let key = values[parent];
        a79_adjust_heap(values, parent, len, key);
        if parent == 0 {
            break;
        }
        parent -= 1;
    }
    let mut end = len;
    while end > 1 {
        end -= 1;
        let key = values[end];
        values[end] = values[0];
        a79_adjust_heap(values, 0, end, key);
    }
}

// ---------------------------------------------------------------------------
// Frame-time statistics: 0x006318f0, 0x00631bc0
// ---------------------------------------------------------------------------

/// The unsigned-dword-to-double sequence the window uses: cast through `i32`
/// and add `2^32` when the sign bit was set.
///
/// `(double)(&DAT_00d33180)[-(x >> 0x1f)]` is that table lookup, with
/// `DAT_00d33180[0] = 0.0` and `DAT_00d33180[-1] = 4294967296.0`.
pub fn a79_uint_to_double(value: u32) -> f64 {
    let signed = value as i32;
    let bias = if signed < 0 { 4294967296.0 } else { 0.0 };
    signed as f64 + bias
}

/// One recovered frame delta is divided by this and stored as an `f32`.
fn a79_scaled(value: u32) -> f32 {
    let as_float = a79_uint_to_double(value) as f32;
    (as_float as f64 / A79_TIMESTAMP_DIVISOR as f64) as f32
}

/// `0x006318f0`: min, max, mean and median of the frame deltas.
///
/// Each sample is a 64-bit timestamp stored as `(low, high)` dwords at
/// `+0x00`/`+0x04`.  For every adjacent pair the row computes
///
/// ```text
/// delta = (u64)(FUN_00c87550(cur.low - prev.low,
///                           cur.high - prev.high - (cur.low < prev.low),
///                           1000, 0));            /* 64-bit divide, low 32 */
/// min = min(delta, min); max = max(delta, max); sum += delta;
/// ```
///
/// then sorts the deltas (`FUN_00631430`, ascending) and returns
///
/// ```text
/// out[0] = (float)min    / 1000.0;
/// out[1] = (float)max    / 1000.0;
/// out[2] = (float)(sum / count) / 1000.0;
/// out[3] = (float)delta[count / 2] / 1000.0;
/// ```
///
/// The shipped row reads its delta array and its divisor from uninitialised
/// stack slots when fewer than two samples reach it (the wrapper
/// `0x00631bc0` only filters `count == 0`), so this translation returns zeros
/// for that case instead of reproducing stack garbage.
pub fn a79_timing_stats(samples: &[(u32, u32)]) -> [f32; 4] {
    if samples.len() < 2 {
        return [0.0; 4];
    }
    let mut deltas: Vec<u32> = Vec::with_capacity(samples.len() - 1);
    let mut min = u32::MAX;
    let mut max = 0u32;
    let mut sum: u64 = 0;
    for window in samples.windows(2) {
        let previous = ((window[0].1 as u64) << 32) | window[0].0 as u64;
        let current = ((window[1].1 as u64) << 32) | window[1].0 as u64;
        let delta = current.wrapping_sub(previous) / A79_TIMESTAMP_DIVISOR;
        let delta = delta as u32;
        if delta < min {
            min = delta;
        }
        if delta > max {
            max = delta;
        }
        sum = sum.wrapping_add(delta as u64);
        deltas.push(delta);
    }
    deltas.sort_unstable();
    let count = deltas.len() as u64;
    let mean = (sum / count) as u32;
    let median = deltas[deltas.len() / 2];
    [
        a79_scaled(min),
        a79_scaled(max),
        a79_scaled(mean),
        a79_scaled(median),
    ]
}

/// `0x00631bc0`: report the statistics of the sample vector at `+0x24`, or
/// four zeros when the count at `+0x28` is zero.
///
/// ```text
/// if (*(int *)(this + 0x28) == 0) { *out = out[1] = out[2] = out[3] = 0; return; }
/// FUN_006318f0(out, this + 0x24);
/// ```
pub fn a79_timing_stats_or_zero(samples: &[(u32, u32)], count: u32) -> [f32; 4] {
    if count == 0 {
        return [0.0; 4];
    }
    a79_timing_stats(&samples[..(count as usize).min(samples.len())])
}

// ---------------------------------------------------------------------------
// Option tree: 0x006300d0, 0x00630255, 0x00630350, 0x006304a0
// ---------------------------------------------------------------------------

/// One option entry: `+0x04` name, `+0x20` payload pointer, `+0x80` dirty byte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A79OptionEntry {
    /// `+0x04`: the name printed with `%-30s`.
    pub name: String,
    /// The already-formatted variant text (`FUN_006304d0` is not translated, so
    /// the caller supplies the string its `%s` argument would hold).
    pub value: String,
    /// `+0x20`: pointer handed to the destructor.
    pub payload: u32,
    /// `+0x80`: the byte `0x00630350` clears after destroying the payload.
    pub dirty: bool,
}

/// One option group: `+0x04` name, `+0x20` entry count, `+0x24` entry array.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A79OptionGroup {
    /// `+0x04`: the name printed with `"    %s:"`.
    pub name: String,
    /// The entries of `+0x24`, in bucket order and then chain order.
    pub entries: Vec<A79OptionEntry>,
    /// `+0x24 == 0` for a group whose entry array is null.
    pub entry_array_present: bool,
}

/// Pads like the recovered `%-30s`.
fn a79_pad_30(name: &str) -> String {
    let mut padded = String::from(name);
    while padded.len() < 30 {
        padded.push(' ');
    }
    padded
}

/// The entry line `0x006300d0` and `0x00630255` print: `"      %-30s: %s"`.
pub fn a79_option_entry_line(name: &str, value: &str) -> String {
    format!("      {}: {}", a79_pad_30(name), value)
}

/// The group line `0x006300d0` prints: `"    %s:"`.
pub fn a79_option_group_line(name: &str) -> String {
    format!("    {}:", name)
}

/// `0x006300d0`: walk the option hash table and print it.
///
/// ```text
/// (*report)[0](report, "Options");
/// for (group = first bucket; group != end; group = next bucket) do {
///   for (; group != 0; group = group->next) {
///     (*report)[0](report, "    %s:", group->name);
///     for (entry = first non-null of group->entries[0 .. group->count];
///          entry != 0; entry = next bucket or entry->next) {
///       record = variant_to_string(entry->value);
///       (*report)[0](report, "      %-30s: %s", entry->name, record.data);
///     }
///   }
/// } while (more buckets);
/// ```
///
/// The bucket scan and the chain walk both visit their elements in increasing
/// order, so the linear order of [`A79OptionGroup`] and [`A79OptionEntry`] is
/// the printed order.
pub fn a79_option_report(groups: &[A79OptionGroup]) -> Vec<String> {
    let mut lines = vec!["Options".to_string()];
    for group in groups {
        lines.push(a79_option_group_line(&group.name));
        if group.entry_array_present {
            for entry in &group.entries {
                lines.push(a79_option_entry_line(&entry.name, &entry.value));
            }
        }
    }
    lines
}

/// `0x00630255`: the entry-only half of the report.
///
/// The row opens with a vtable `+0x10` call and then re-walks the same hash
/// table, printing only `"      %-30s: %s"` for each entry — the group line of
/// `0x006300d0` is absent.
pub fn a79_option_entries(groups: &[A79OptionGroup]) -> Vec<String> {
    let mut lines = Vec::new();
    for group in groups {
        if group.entry_array_present {
            for entry in &group.entries {
                lines.push(a79_option_entry_line(&entry.name, &entry.value));
            }
        }
    }
    lines
}

/// `0x00630350`: destroy and unflag every dirty entry.
///
/// ```text
/// for (group = first bucket; group != end; group = next bucket)
///   for (entry = first of group->entries; entry != 0; entry = next)
///     if (entry->dirty != 0) {
///       FUN_0062f1d0(entry->payload, &payload);
///       entry->dirty = 0;
///     }
/// ```
///
/// Returns the payloads destroyed, in visit order.
pub fn a79_clear_entry_flags(groups: &mut [A79OptionGroup]) -> Vec<u32> {
    let mut destroyed = Vec::new();
    for group in groups.iter_mut() {
        for entry in group.entries.iter_mut() {
            if entry.dirty {
                destroyed.push(entry.payload);
                entry.dirty = false;
            }
        }
    }
    destroyed
}

/// Byte offset of the dirty flag inside the `0x006304a0` slot record.
pub const A79_DIRTY_FLAG_OFFSET: u32 = 0x60;

/// The slot `0x006304a0` takes by `ECX`: payload at `+0x00`, flag at `+0x60`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A79DirtySlot {
    /// `+0x00`: pointer handed to `FUN_0062f1d0`.
    pub payload: u32,
    /// `+0x60`: the byte that gates the destruction.
    pub dirty: bool,
}

/// `0x006304a0`: destroy the pending object once.
///
/// ```text
/// if (*(char *)(this + 0x60) != 0) {
///   FUN_0062f1d0(*(undefined4 *)this, &local);
///   *(char *)(this + 0x60) = 0;
/// }
/// ```
pub fn a79_release_dirty_slot(slot: &mut A79DirtySlot) -> Option<u32> {
    if !slot.dirty {
        return None;
    }
    slot.dirty = false;
    Some(slot.payload)
}

// ---------------------------------------------------------------------------
// Performance report: 0x00631e00, 0x00631ee8, 0x00631ff0
// ---------------------------------------------------------------------------

/// One step of the recovered report routine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum A79ReportAction {
    /// `FUN_0093fd10(path, 0x40000000, 4, mode, 0x80)`.
    Open {
        path: String,
        flags: u32,
        share: u32,
        mode: u32,
        buffer: u32,
    },
    /// `FUN_009406d0(handle, 0, 0, 2)` after a successful mode-3 open.
    SeekEnd {
        handle: i32,
        offset: u32,
        origin: u32,
    },
    /// `FUN_00631ff0(handle, format, name)` — the backend line.
    WriteApi {
        handle: i32,
        text: String,
        format_len: u32,
        metadata: u32,
    },
    /// `FUN_00558ab0(&text, "Could not open file: {0}", 0x18, 0xd, 0, &pair)`
    /// followed by `FUN_00959280(text, "Performance Report Generation error", 4)`.
    Message {
        text: String,
        format_len: u32,
        metadata: u32,
        caption: String,
        flags: u32,
    },
    /// `FUN_00632780(handle)`.
    Close { handle: i32 },
}

/// What [`a79_report_steps`] and [`a79_report_steps_retried`] ended with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A79ReportRun {
    /// The recovered steps, in order.
    pub actions: Vec<A79ReportAction>,
    /// `Some(3)` or `Some(2)`: the mode whose open succeeded.
    pub opened_mode: Option<u32>,
    /// `true` when the loop gave up without an open.
    pub failed: bool,
}

/// The open/retry loop both report rows carry.
///
/// ```text
/// loop {
///   if (open(mode 3) != -1) { FUN_009406d0(handle, 0, 0, 2); return; }
///   if (open(mode 2) != -1) {
///     name = FUN_00633ca0();                    /* backend API string */
///     FUN_00558ab0(&line, "Rendering backend API: {0}", 0x1a, 0xd, 0, &name);
///     FUN_00631ff0(handle, &DAT_00d3c19c, line);
///     FUN_00632780(handle);
///     return;
///   }
///   FUN_00558ab0(&text, "Could not open file: {0}", 0x18, 0xd, 0, &path_pair);
///   if (FUN_00959280(text, "Performance Report Generation error", 4) == 0) return;
/// }
/// ```
fn a79_report_run(
    path: &str,
    api: &str,
    enter: bool,
    open_results: &[bool],
    retry_results: &[bool],
) -> A79ReportRun {
    let mut run = A79ReportRun {
        actions: Vec::new(),
        opened_mode: None,
        failed: !enter,
    };
    if !enter {
        return run;
    }
    let mut opens = open_results.iter();
    let mut retries = retry_results.iter();
    loop {
        let Some(&first) = opens.next() else {
            run.failed = true;
            return run;
        };
        run.actions.push(A79ReportAction::Open {
            path: path.to_string(),
            flags: A79_OPEN_FLAGS,
            share: A79_REPORT_OPEN_SHARE,
            mode: A79_REPORT_MODE_FIRST,
            buffer: A79_OPEN_BUFFER,
        });
        if first {
            run.actions.push(A79ReportAction::SeekEnd {
                handle: A79_REPORT_HANDLE,
                offset: 0,
                origin: 2,
            });
            run.opened_mode = Some(A79_REPORT_MODE_FIRST);
            run.failed = false;
            return run;
        }
        let Some(&second) = opens.next() else {
            run.failed = true;
            return run;
        };
        run.actions.push(A79ReportAction::Open {
            path: path.to_string(),
            flags: A79_OPEN_FLAGS,
            share: A79_REPORT_OPEN_SHARE,
            mode: A79_REPORT_MODE_SECOND,
            buffer: A79_OPEN_BUFFER,
        });
        if second {
            run.actions.push(A79ReportAction::WriteApi {
                handle: A79_REPORT_HANDLE,
                text: A79_REPORT_API_FORMAT.replace("{0}", api),
                format_len: A79_REPORT_API_FORMAT_LEN,
                metadata: A79_REPORT_FORMAT_METADATA,
            });
            run.actions.push(A79ReportAction::Close {
                handle: A79_REPORT_HANDLE,
            });
            run.opened_mode = Some(A79_REPORT_MODE_SECOND);
            run.failed = false;
            return run;
        }
        run.actions.push(A79ReportAction::Message {
            text: A79_REPORT_MESSAGE_FORMAT.replace("{0}", path),
            format_len: A79_REPORT_MESSAGE_FORMAT_LEN,
            metadata: A79_REPORT_FORMAT_METADATA,
            caption: A79_REPORT_CAPTION.to_string(),
            flags: A79_REPORT_MESSAGE_FLAGS,
        });
        let retry = retries.next().copied().unwrap_or(false);
        if !retry {
            run.failed = true;
            return run;
        }
    }
}

/// `0x00631e00`: open the performance report, entering the loop
/// unconditionally.
///
/// The row is the exception-handling fragment of the generator: it tries mode
/// `3` (and seeks to the end), falls back to mode `2`, and otherwise reports
/// `"Could not open file: {0}"` with the caption
/// `"Performance Report Generation error"` before retrying when the message
/// box returns non-zero.  `open_results` supplies the two `open` outcomes per
/// iteration, `retry_results` the message-box outcomes.
pub fn a79_report_steps(
    path: &str,
    api: &str,
    open_results: &[bool],
    retry_results: &[bool],
) -> A79ReportRun {
    a79_report_run(path, api, true, open_results, retry_results)
}

/// `0x00631ee8`: the same loop, but gated on the caller's byte at
/// `ebp - 0x75` before the first open attempt.
///
/// ```text
/// cVar4 = *(char *)(ebp - 0x75);
/// do {
///   if (cVar4 == 0) break;
///   ... same open/report loop ...
/// } while (true);
/// ```
pub fn a79_report_steps_retried(
    path: &str,
    api: &str,
    enter: bool,
    open_results: &[bool],
    retry_results: &[bool],
) -> A79ReportRun {
    a79_report_run(path, api, enter, open_results, retry_results)
}

/// One written record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A79RecordWrite {
    /// Handle the record went to.
    pub handle: i32,
    /// `record.size`, the length argument of `FUN_00940720`.
    pub len: u32,
    /// The formatted record is released after the write.
    pub released: bool,
}

/// `0x00631ff0`: write one formatted record.
///
/// ```text
/// FUN_00c6c400(&record, param_2, &stack + 0xc, cookie);   /* format */
/// FUN_00940720(handle, record.data, record.size, &out);   /* write */
/// if ((record.capacity & 0x80000000) == 0) FUN_0093ec70();
/// ```
///
/// `param_2` is the format string the caller passes (`&DAT_00d3c19c`, not
/// recoverable from the row); the translation takes the formatted text.
pub fn a79_write_record(handle: i32, text: &str) -> A79RecordWrite {
    A79RecordWrite {
        handle,
        len: text.len() as u32,
        released: true,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent79 {
    use super::*;

    // -- 0x006300d0 / 0x00630255 ------------------------------------------

    fn option_tree() -> Vec<A79OptionGroup> {
        vec![
            A79OptionGroup {
                name: "Graphics".to_string(),
                entries: vec![
                    A79OptionEntry {
                        name: "Resolution".to_string(),
                        value: "1024x768".to_string(),
                        payload: 0x11,
                        dirty: false,
                    },
                    A79OptionEntry {
                        name: "Windowed".to_string(),
                        value: "false".to_string(),
                        payload: 0x12,
                        dirty: true,
                    },
                ],
                entry_array_present: true,
            },
            A79OptionGroup {
                name: "Sound".to_string(),
                entries: Vec::new(),
                entry_array_present: false,
            },
        ]
    }

    #[test]
    fn option_report_prints_the_recovered_format_strings() {
        let lines = a79_option_report(&option_tree());
        assert_eq!(lines[0], "Options");
        assert_eq!(lines[1], "    Graphics:");
        assert_eq!(lines[2], "      Resolution                    : 1024x768");
        assert_eq!(lines[3], "      Windowed                      : false");
        assert_eq!(lines[4], "    Sound:");
        assert_eq!(lines.len(), 5);
    }

    #[test]
    fn option_entries_skips_the_group_lines() {
        let lines = a79_option_entries(&option_tree());
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], a79_option_entry_line("Resolution", "1024x768"));
        assert_eq!(lines[1], a79_option_entry_line("Windowed", "false"));
    }

    #[test]
    fn percent_30s_pads_to_thirty_bytes_and_never_truncates() {
        assert_eq!(a79_pad_30("abc").len(), 30);
        assert_eq!(a79_pad_30("abc"), "abc                           ");
        assert_eq!(
            a79_pad_30("a-name-that-is-longer-than-thirty"),
            "a-name-that-is-longer-than-thirty"
        );
    }

    // -- 0x00630350 -------------------------------------------------------

    #[test]
    fn clear_entry_flags_destroys_only_dirty_entries() {
        let mut groups = option_tree();
        assert_eq!(a79_clear_entry_flags(&mut groups), vec![0x12]);
        assert!(!groups[0].entries[1].dirty);
        assert!(!groups[0].entries[0].dirty);
        assert!(a79_clear_entry_flags(&mut groups).is_empty());
    }

    // -- 0x006304a0 -------------------------------------------------------

    #[test]
    fn dirty_slot_releases_once() {
        let mut slot = A79DirtySlot {
            payload: 0x2a,
            dirty: true,
        };
        assert_eq!(A79_DIRTY_FLAG_OFFSET, 0x60);
        assert_eq!(a79_release_dirty_slot(&mut slot), Some(0x2a));
        assert!(!slot.dirty);
        assert_eq!(a79_release_dirty_slot(&mut slot), None);
    }

    // -- 0x00630870 / 0x00630890 / 0x00630892 -----------------------------

    #[test]
    fn commit_uses_the_zero_offset_pair() {
        assert_eq!(
            a79_commit(5),
            A79FileCall::Commit {
                handle: 5,
                offset: 0,
                origin: 0
            }
        );
    }

    #[test]
    fn position_query_returns_zero_when_the_query_fails() {
        let (value, calls) = a79_read_position(7, true, 0x1234_5678_9abc);
        assert_eq!(value, 0x1234_5678_9abc);
        assert_eq!(
            calls,
            vec![
                A79FileCall::QueryPosition {
                    handle: 7,
                    offset: 0,
                    origin: 0
                },
                A79FileCall::WidenPosition {
                    raw: 0x1234_5678_9abc
                },
            ]
        );

        let (value, calls) = a79_read_position(7, false, 0x99);
        assert_eq!(value, 0);
        assert_eq!(calls.len(), 1);

        let handle = 9;
        let (value, _) = a79_read_position_indirect(&handle, true, 42);
        assert_eq!(value, 42);
    }

    // -- 0x00630940 / 0x00630a10 / 0x00631120 / 0x00631222 ----------------

    fn ring_with_two_inline_elements() -> A79Ring {
        let mut ring = A79Ring::a79_new(8);
        let mut block = vec![A79String::a79_new(); A79_DEQUE_FANOUT as usize];
        block[0] = A79String::a79_from_bytes(b"alpha");
        block[1] = A79String::a79_from_bytes(b"beta");
        // Block slot (capacity - 1) & (index >> 3) of index 0 is 0.
        ring.blocks[0] = Some(block);
        ring.size = 2;
        ring
    }

    #[test]
    fn reserve_slot_uses_the_block_modulo_and_the_low_three_bits() {
        let mut ring = A79Ring::a79_new(8);
        ring.blocks[0] = Some(vec![A79String::a79_new(); 8]);
        assert_eq!(
            ring.a79_reserve_slot().unwrap(),
            A79Slot {
                index: 0,
                block: 0,
                offset: 0
            }
        );
        assert_eq!(ring.size, 1);
        // A null block slot still selected and counted the element first.
        let mut null = A79Ring::a79_new(8);
        assert_eq!(
            null.a79_reserve_slot(),
            Err(A79RingFault::NullBlock { block: 0 })
        );
        assert_eq!(null.size, 1);
        assert_eq!(ring.blocks.len(), 8);
        assert_eq!(A79_DEQUE_BLOCK_BYTES, 0xe0);
        assert_eq!(A79_DEQUE_FANOUT * A79_STRING_SIZE, 0xe0);

        // Index 8 wraps into the second block of an eight-block map.
        let mut wrapped = A79Ring::a79_new(8);
        wrapped.blocks[1] = Some(vec![A79String::a79_new(); 8]);
        wrapped.front = 8;
        assert_eq!(
            wrapped.a79_reserve_slot().unwrap(),
            A79Slot {
                index: 8,
                block: 1,
                offset: 0
            }
        );
    }

    #[test]
    fn reserve_slot_reports_the_grown_capacity() {
        let mut ring = A79Ring::a79_new(1);
        // (0 + 0) & 7 == 0 and capacity 1 <= (0 + 8) >> 3 == 1.
        assert_eq!(
            ring.a79_reserve_slot(),
            Err(A79RingFault::NeedsGrowth { capacity: 2 })
        );
        let mut empty = A79Ring::a79_new(0);
        assert_eq!(
            empty.a79_reserve_slot(),
            Err(A79RingFault::NeedsGrowth { capacity: 8 })
        );
    }

    #[test]
    fn reserve_slot_indexes_through_the_front() {
        let mut ring = A79Ring::a79_new(8);
        ring.blocks[0] = Some(vec![A79String::a79_new(); 8]);
        ring.front = 5;
        ring.size = 2;
        assert_eq!(
            ring.a79_reserve_slot().unwrap(),
            A79Slot {
                index: 7,
                block: 0,
                offset: 7 * 0x1c
            }
        );
        assert_eq!(ring.size, 3);
    }

    #[test]
    fn allocate_block_claims_e0_bytes() {
        let mut ring = A79Ring::a79_new(8);
        let allocation = ring.a79_allocate_block(9).unwrap();
        assert_eq!(
            allocation,
            A79BlockAllocation {
                bytes: 0xe0,
                block: 1,
                offset: 0x1c
            }
        );
        assert_eq!(ring.blocks[1].as_ref().unwrap().len(), 8);
    }

    #[test]
    fn check_range_requires_inline_capacities() {
        let ring = ring_with_two_inline_elements();
        assert_eq!(ring.a79_check_range(), Ok(()));
        assert_eq!(ring.a79_check_range_tail(), Ok(()));

        let mut broken = ring.clone();
        let long = b"a heap string longer than fifteen";
        broken.blocks[0].as_mut().unwrap()[1] = A79String::a79_from_bytes(long);
        assert_eq!(
            broken.a79_check_range(),
            Err(A79CheckFault::HeapElement {
                index: 1,
                capacity: long.len() as u32
            })
        );

        assert_eq!(
            A79Ring::a79_new(0).a79_check_range(),
            Err(A79CheckFault::EmptyRing)
        );
    }

    // -- 0x00630be6 / 0x00630bf2 -----------------------------------------

    #[test]
    fn steal_moves_a_heap_source_into_an_inline_destination() {
        let text = b"a heap string longer than fifteen";
        let mut dest = A79String::a79_new();
        let mut src = A79String::a79_from_bytes(text);
        assert!(!src.a79_is_inline());
        assert_eq!(src.capacity, text.len() as u32);

        assert_eq!(dest.a79_steal(&mut src), Ok(()));
        assert_eq!(dest.a79_text(), "a heap string longer than fifteen");
        assert_eq!(dest.size, text.len() as u32);
        // (src.capacity ^ 0xf) & 0x7fffffff ^ 0xf == src.capacity for a heap source.
        assert_eq!(dest.capacity, text.len() as u32);
        assert!(!dest.a79_is_inline());

        // The moved-from record is the inline-empty reset.
        assert_eq!(src, A79String::a79_new());
    }

    #[test]
    fn steal_rejects_an_inline_source_and_a_heap_destination() {
        let mut dest = A79String::a79_new();
        let mut inline_src = A79String::a79_from_bytes(b"short");
        assert_eq!(
            dest.a79_steal(&mut inline_src),
            Err(A79StealFault::SourceInline { size: 5 })
        );

        let mut heap_dest = A79String::a79_from_bytes(b"another heap string over fifteen bytes");
        let mut heap_src = A79String::a79_from_bytes(b"a heap string longer than fifteen");
        assert_eq!(
            heap_dest.a79_steal(&mut heap_src),
            Err(A79StealFault::DestinationHeap { capacity: 38 })
        );
    }

    #[test]
    fn flagged_steal_needs_the_caller_comparison() {
        let text = b"a heap string longer than fifteen";
        let mut dest = A79String::a79_new();
        let mut src = A79String::a79_from_bytes(text);
        assert_eq!(
            dest.a79_steal_flagged(&mut src, false),
            Err(A79StealFault::FlaggedNotReady {
                size: text.len() as u32
            })
        );
        assert_eq!(dest.a79_steal_flagged(&mut src, true), Ok(()));
        assert_eq!(dest.size, text.len() as u32);
    }

    // -- 0x00630d20 / 0x00630d9a -----------------------------------------

    #[test]
    fn open_uses_the_recovered_mode_formula() {
        let mut stream = A79LogStream::a79_new("errors.log");
        let handle = stream.a79_open(false);
        assert_eq!(handle, 0x100);
        assert_eq!(
            stream.calls,
            vec![
                A79FileCall::Open {
                    path: "errors.log".to_string(),
                    flags: 0x4000_0000,
                    share: 0,
                    mode: 4,
                    buffer: 0x80,
                },
                A79FileCall::SeekEnd {
                    handle: 0x100,
                    offset: 0,
                    origin: 2,
                },
            ]
        );

        let mut append = A79LogStream::a79_new("errors.log");
        assert_eq!(append.a79_open(true), 0x100);
        assert_eq!(
            append.calls,
            vec![A79FileCall::Open {
                path: "errors.log".to_string(),
                flags: 0x4000_0000,
                share: 0,
                mode: 2,
                buffer: 0x80,
            }]
        );
    }

    #[test]
    fn seek_tail_emits_origin_two() {
        assert_eq!(
            a79_seek_end_call(3),
            A79FileCall::SeekEnd {
                handle: 3,
                offset: 0,
                origin: 2
            }
        );
    }

    // -- 0x00630ea0 / 0x00630df0 / 0x00630ab0 / 0x00630a20 ----------------

    #[test]
    fn write_flushes_pending_records_then_closes() {
        let mut stream = A79LogStream::a79_new("errors.log");
        stream.pending = ring_with_two_inline_elements();
        assert_eq!(stream.a79_write(b"tail"), Ok(()));

        let mut expected = String::from("alphabeta");
        expected.push_str("tail");
        assert_eq!(stream.written, expected.as_bytes());

        let writes: Vec<u32> = stream
            .calls
            .iter()
            .filter_map(|call| match call {
                A79FileCall::Write { len, .. } => Some(*len),
                _ => None,
            })
            .collect();
        assert_eq!(writes, vec![5, 4, 4]);
        assert_eq!(stream.pending.size, 0);
        assert_eq!(stream.pending.front, 2);
        assert_eq!(stream.handle, -1);
        assert!(matches!(
            stream.calls.last(),
            Some(A79FileCall::Close { handle: 0x100 })
        ));
    }

    #[test]
    fn append_mode_keeps_the_handle_open() {
        let mut stream = A79LogStream::a79_new("errors.log");
        stream.append = true;
        stream.handle = 0x100;
        assert_eq!(stream.a79_write_formatted(b"line\n"), Ok(()));
        assert_eq!(stream.handle, 0x100);
        assert_eq!(stream.written, b"line\n");
        assert_eq!(stream.calls.len(), 1); // one write, no open/flush/close
    }

    #[test]
    fn non_append_write_opens_first() {
        let mut stream = A79LogStream::a79_new("errors.log");
        assert_eq!(stream.a79_write(b"x"), Ok(()));
        assert!(matches!(
            stream.calls.first(),
            Some(A79FileCall::Open { mode: 4, .. })
        ));
    }

    #[test]
    fn write_without_a_path_reports_a_closed_handle() {
        let mut stream = A79LogStream::a79_new("");
        stream.handle = -1;
        assert_eq!(
            stream.a79_write(b"abc"),
            Err(A79WriteFault::Closed { size: 3 })
        );
    }

    #[test]
    fn flush_and_teardown_follow_the_recovered_order() {
        let mut stream = A79LogStream::a79_new("errors.log");
        stream.a79_open(false);
        stream.calls.clear();
        assert_eq!(stream.a79_flush(), A79FileCall::Flush { handle: 0x100 });
        stream.calls.clear();
        assert_eq!(stream.a79_teardown(), Ok(()));
        assert_eq!(
            stream.calls,
            vec![
                A79FileCall::Flush { handle: 0x100 },
                A79FileCall::Close { handle: 0x100 },
            ]
        );
        assert_eq!(stream.handle, -1);

        // The path capacity is the 0x1c string's word at +0x0c.
        let mut broken = A79LogStream::a79_new("errors.log");
        let long_path = b"a heap path longer than fifteen";
        broken.path = A79String::a79_from_bytes(long_path);
        assert_eq!(
            broken.a79_teardown(),
            Err(A79TeardownFault::HeapPathCapacity {
                capacity: long_path.len() as u32
            })
        );
    }

    #[test]
    fn teardown_checks_the_pending_deque() {
        let mut stream = A79LogStream::a79_new("errors.log");
        stream.pending = A79Ring::a79_new(0);
        assert_eq!(
            stream.a79_teardown(),
            Err(A79TeardownFault::Ring(A79CheckFault::EmptyRing))
        );
    }

    #[test]
    fn release_and_reopen_tail_needs_a_path_and_append_mode() {
        let mut stream = A79LogStream::a79_new("errors.log");
        let mut record = A79String::a79_from_bytes(b"record");
        assert_eq!(stream.a79_release_and_reopen(&mut record), Ok(None));
        assert_eq!(record, A79String::a79_new());

        // A heap-mode temporary trips the capacity invariant before release.
        let heap_text = b"a heap record longer than fifteen";
        let mut heap = A79String::a79_from_bytes(heap_text);
        assert_eq!(
            stream.a79_release_and_reopen(&mut heap),
            Err(A79TeardownFault::HeapPathCapacity {
                capacity: heap_text.len() as u32
            })
        );
        assert!(!heap.a79_is_inline());

        stream.append = true;
        assert_eq!(stream.a79_release_and_reopen_tail(&mut record), Some(0x100));
        assert_eq!(stream.calls.len(), 1);
    }

    #[test]
    fn error_unwind_builds_the_inline_empty_string() {
        let unwind = a79_error_unwind(0x19);
        assert_eq!(unwind.code, 0x19);
        assert_eq!(unwind.message, A79String::a79_new());
        assert_eq!(unwind.message.capacity, 0x8000_000f);
    }

    // -- 0x006312f0 / 0x00631300 / 0x00631320 / 0x00631340 ----------------

    #[test]
    fn default_key_table_is_eight_threes() {
        assert_eq!(a79_default_key_table(), [3u8; 8]);
        assert_eq!(A79_KEY_TABLE_WORDS, [0x0303_0303, 0x0303_0303]);
        assert_eq!(A79_KEY_TABLE_LEN, 8);
        assert_eq!(a79_key_table_base(), 0x100a_86ac);
        assert_eq!(a79_key_table_base() + A79_KEY_TABLE_OFFSET, 0x100b_6cd4);

        let mut state = A79KeyState::a79_new();
        state.a79_install_default_table();
        assert_eq!(state.table, [3u8; 8]);
    }

    #[test]
    fn initialised_flag_and_state_predicate() {
        let mut state = A79KeyState::a79_new();
        assert_eq!(state.a79_initialised(), 0);
        assert_eq!(state.a79_flag_state(), 0);
        assert!(state.a79_init_once());
        assert_eq!(state.a79_initialised(), 1);
        assert_eq!(state.a79_flag_state(), 1);
        assert!(!state.a79_init_once());
        state.gate = true;
        assert_eq!(state.a79_flag_state(), 0);
    }

    #[test]
    fn key_counters_decay_only_behind_the_gate() {
        let mut state = A79KeyState::a79_new();
        state.a79_install_default_table();
        assert_eq!(state.a79_get(0), 3);
        assert_eq!(state.a79_decay(0), None);
        state.gate = true;
        assert_eq!(state.a79_decay(0), Some(2));
        assert_eq!(state.a79_decay(0), Some(1));
        assert_eq!(state.a79_decay(0), Some(0));
        assert_eq!(state.a79_decay(0), None);
        assert_eq!(state.a79_get(7), 3);
    }

    #[test]
    fn shutdown_clears_the_flag_and_may_resume_audio() {
        let mut state = A79KeyState::a79_new();
        assert_eq!(state.a79_shutdown(false), Ok(A79Shutdown::Idle));

        state.a79_init_once();
        assert_eq!(
            state.a79_shutdown(false),
            Ok(A79Shutdown::Released {
                resume_audio: false
            })
        );
        assert_eq!(state.a79_initialised(), 0);

        let mut forcing = A79KeyState::a79_new();
        forcing.a79_init_once();
        assert_eq!(forcing.a79_shutdown(true), Err(A79ShutdownFault::Forced));
        assert_eq!(forcing.a79_initialised(), 1);

        let mut audio = A79KeyState::a79_new();
        audio.a79_init_once();
        audio.audio_present = true;
        assert_eq!(
            audio.a79_shutdown(false),
            Ok(A79Shutdown::Released { resume_audio: true })
        );
    }

    // -- 0x00631260 -------------------------------------------------------

    #[test]
    fn language_gate_reproduces_the_probe_comparison() {
        assert_eq!(a79_language_gate(false, 0, 7), A79LanguageGate::Allowed);
        assert_eq!(a79_language_gate(true, 0, 7), A79LanguageGate::Rejected);
        assert_eq!(a79_language_gate(true, 8, 7), A79LanguageGate::Rejected);
        assert_eq!(
            a79_language_gate(true, 9, 7),
            A79LanguageGate::Report { arg: 7 }
        );
        assert_eq!(
            a79_language_gate(true, 0xff, 7),
            A79LanguageGate::Report { arg: 7 }
        );
    }

    // -- 0x00631790 / 0x0063159f -----------------------------------------

    #[test]
    fn adjust_heap_sifts_a_single_hole() {
        // A three-element heap whose root is being replaced.
        let mut values = vec![1u32, 5, 9];
        a79_adjust_heap(&mut values, 0, 3, 0);
        assert_eq!(values, vec![9, 5, 0]);
        assert!(values[0] >= values[1]);
    }

    #[test]
    fn heap_sort_matches_an_ascending_sort() {
        let cases: [&[u32]; 5] = [
            &[],
            &[42],
            &[2, 1],
            &[5, 5, 1, 9, 3, 3, 0],
            &[9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 11, 12],
        ];
        for case in cases {
            let mut sorted = case.to_vec();
            sorted.sort_unstable();
            let mut heaped = case.to_vec();
            a79_heap_sort(&mut heaped);
            assert_eq!(heaped, sorted, "input {case:?}");
        }
    }

    #[test]
    fn heap_sort_handles_duplicates_and_the_two_element_floor() {
        let mut values = vec![7u32, 7];
        a79_heap_sort(&mut values);
        assert_eq!(values, vec![7, 7]);
        // len/2 - 1 == 0 for both two and three elements.
        assert_eq!(3usize / 2 - 1, 0);
    }

    // -- 0x006318f0 / 0x00631bc0 -----------------------------------------

    #[test]
    fn timing_stats_report_min_max_mean_and_median() {
        // Deltas 1000/2000/3000 are divided by 1000 first, so the reported
        // milliseconds are 1/2/3 and the seconds 0.001/0.002/0.003.
        let samples = [(0u32, 0u32), (1000, 0), (3000, 0), (6000, 0)];
        assert_eq!(a79_timing_stats(&samples), [0.001, 0.003, 0.002, 0.002]);
    }

    #[test]
    fn timing_stats_carry_across_the_high_dword() {
        // 0x1_0000_03e8 - 0xffff_f000 == 5096, divided by 1000 == 5.
        let samples = [(0xffff_f000u32, 0u32), (0x0000_03e8, 1)];
        assert_eq!(a79_timing_stats(&samples), [0.005, 0.005, 0.005, 0.005]);
    }

    #[test]
    fn timing_stats_or_zero_matches_the_wrapper() {
        let samples = [(0u32, 0u32), (2000, 0)];
        assert_eq!(a79_timing_stats_or_zero(&samples, 0), [0.0; 4]);
        assert_eq!(a79_timing_stats_or_zero(&samples, 2), [0.002; 4]);
    }

    #[test]
    fn unsigned_to_double_uses_the_recovered_bias() {
        assert_eq!(a79_uint_to_double(0), 0.0);
        assert_eq!(a79_uint_to_double(0x7fff_ffff), 2147483647.0);
        assert_eq!(a79_uint_to_double(0xffff_ffff), 4294967295.0);
        assert_eq!(a79_uint_to_double(0x8000_0000), 2147483648.0);
    }

    // -- 0x00631e00 / 0x00631ee8 / 0x00631ff0 ----------------------------

    #[test]
    fn report_seeks_when_the_first_mode_opens() {
        let run = a79_report_steps("perf.txt", "D3D9", &[true], &[]);
        assert_eq!(run.opened_mode, Some(3));
        assert!(!run.failed);
        assert_eq!(run.actions.len(), 2);
        assert_eq!(
            run.actions[0],
            A79ReportAction::Open {
                path: "perf.txt".to_string(),
                flags: 0x4000_0000,
                share: 4,
                mode: 3,
                buffer: 0x80,
            }
        );
        assert_eq!(
            run.actions[1],
            A79ReportAction::SeekEnd {
                handle: A79_REPORT_HANDLE,
                offset: 0,
                origin: 2
            }
        );
    }

    #[test]
    fn report_writes_the_backend_line_on_the_second_mode() {
        let run = a79_report_steps("perf.txt", "D3D9", &[false, true], &[]);
        assert_eq!(run.opened_mode, Some(2));
        assert_eq!(
            run.actions[2],
            A79ReportAction::WriteApi {
                handle: A79_REPORT_HANDLE,
                text: "Rendering backend API: D3D9".to_string(),
                format_len: 0x1a,
                metadata: 0xd,
            }
        );
        assert_eq!(run.actions[3], A79ReportAction::Close { handle: 1 });
    }

    #[test]
    fn report_reports_and_retries_the_whole_pair() {
        let run = a79_report_steps("perf.txt", "D3D9", &[false, false, true], &[true]);
        // Open(3), Open(2), Message, then the retried Open(3) and its seek.
        assert_eq!(run.actions.len(), 5);
        assert_eq!(
            run.actions[2],
            A79ReportAction::Message {
                text: "Could not open file: perf.txt".to_string(),
                format_len: 0x18,
                metadata: 0xd,
                caption: "Performance Report Generation error".to_string(),
                flags: 4,
            }
        );
        assert_eq!(run.opened_mode, Some(3));

        let failed = a79_report_steps("perf.txt", "D3D9", &[false, false], &[false]);
        assert!(failed.failed);
        assert_eq!(failed.actions.len(), 3);

        let exhausted = a79_report_steps("perf.txt", "D3D9", &[false, false], &[true]);
        assert!(exhausted.failed);
    }

    #[test]
    fn report_retry_entry_byte_gates_the_loop() {
        let run = a79_report_steps_retried("perf.txt", "D3D9", false, &[true], &[]);
        assert!(run.failed);
        assert!(run.actions.is_empty());

        let run = a79_report_steps_retried("perf.txt", "D3D9", true, &[true], &[]);
        assert_eq!(run.opened_mode, Some(3));
    }

    #[test]
    fn write_record_reports_the_formatted_length() {
        assert_eq!(
            a79_write_record(4, "Rendering backend API: D3D9"),
            A79RecordWrite {
                handle: 4,
                len: 27,
                released: true
            }
        );
        assert_eq!(
            A79_REPORT_API_FORMAT.len(),
            A79_REPORT_API_FORMAT_LEN as usize
        );
        assert_eq!(
            A79_REPORT_MESSAGE_FORMAT.len(),
            A79_REPORT_MESSAGE_FORMAT_LEN as usize
        );
        assert_eq!(A79_REPORT_CAPTION.len(), 35);
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// The 20 rows not translated above, grouped by why:
//
// * `FUN_0063014c`, `FUN_0063016b`: the decompiler's bodies start mid
//   instruction (`*pbVar1 = *pbVar1 << 1 | (char)*pbVar1 < '\0'`, a 16-dword
//   stack copy of garbage, `cVar2 = (char)in_EAX + -0x7b + in_CF`).  Both are
//   overlapping-instruction artefacts whose real behaviour is not present in
//   the row.
// * `FUN_006304d0`, `FUN_006304e6`: the variant-to-string switch.  The tag-1
//   arm (`"true"`/`"false"` plus `strlen` and an empty-string reserve) and the
//   tag-5 overflow test are readable, but tags 2 and 4 call the out-of-window
//   formatters `FUN_00c6be00` / `FUN_0056d6b0`, so the produced text cannot be
//   reproduced from this window.  The option report takes the formatted value
//   as data instead.
// * `FUN_006306e0`: object-array destructor; its work is the out-of-window
//   `FUN_0053aa00`, `FUN_0053bda0` and two negative-length guards whose
//   operands live at `+0x6c`/`+0xc` of an object this window never defines.
// * `FUN_00630af0`: one instruction, `FUN_00c89580(*unaff_ESI)` through an
//   `ESI` that belongs to the caller; nothing is recovered.
// * `FUN_00630b10`: sets the `+0x24` mode byte and then calls the
//   out-of-window `FUN_0074fb80` (path validation) and `FUN_0064c190` /
//   `FUN_0074eda0` (the string builder).  Only the assignment and the trailing
//   reopen condition survive; both are already covered by
//   [`A79LogStream::a79_open`] and [`A79LogStream::a79_reopen_append`].
// * `FUN_00630b89`, `FUN_00630ba9`, `FUN_00630c92`: three catch funclets that
//   write a NUL at `buffer + *(int *)(ebx + 4)`, store the buffer at `ebx + 8`
//   and then test `ebx + 8`/`ebx + 0x24`.  Their `EBX` is neither the stream of
//   `0x00630d20` (path data at `+0x04`, size at `+0x08`, capacity at `+0x0c`)
//   nor a string with that layout, so the row does not name the object whose
//   fields it writes; the reopen tail they end with is
//   [`A79LogStream::a79_reopen_append`].
// * `FUN_00630e90`: `(**(code **)*in_ECX)()` — one virtual dispatch with no
//   visible argument or effect; the object's vtable is not in the row.
// * `FUN_00631374`: `return CONCAT31(..., (char)pcVar1 + *pcVar1 + in_CF)`; the
//   decompiler lost the instruction boundary, so this is an overlapping-
//   instruction artefact.
// * `FUN_00631430`: the introsort.  The small-range arms are readable
//   (insertion sort below 33 elements, the heap pass through `0x00631790`),
//   but the partition calls `func_0x00631680(begin, end, context)` — an
//   indirect callee with no ledger row and no decompilation — so the pivot
//   choice and therefore the swap sequence are not recoverable.  The sort
//   helpers it calls are translated above ([`a79_adjust_heap`],
//   [`a79_heap_sort`]); the statistics routine's ascending-order requirement is
//   what [`a79_timing_stats`] relies on.
// * `FUN_00631820`: only `FUN_00c867a5(this, 0x28, 0xd, FUN_0060ffe0)` — an
//   SEH frame registration; the handler address is outside the window.
// * `FUN_00631860`: `DAT_00f05658 = 1; FUN_00538810(*(param_1 + 4))`.  The
//   later half is the same reserve path [`a79_error_unwind`] models; the flag
//   it sets is never read in this window, so there is no behaviour to pin.
// * `FUN_00631d49`: frame-rate calculation over `_DAT_00f05690`,
//   `_DAT_00f05694`, `DAT_00f05678` and `FUN_00c87160`; the row keeps the
//   numerator in a register the decompiler did not name (`ZEXT416` of a float
//   built from `unaff_ESI`), so the division it performs cannot be stated
//   exactly.
// * `FUN_00631dd3`, `FUN_00631de4`: store the 64-bit tick base and forward to
//   `FUN_00633050`, which has no ledger row and is not translated.
// * `FUN_00631f7b`, `FUN_00631f9d`: the cleanup funclets of the report loop —
//   a virtual `+0x10` call, one capacity trap, and `FUN_00632780(handle)`.
//   [`a79_report_steps`] already models the close they perform.
