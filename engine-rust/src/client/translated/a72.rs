#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent72.rs
// ===========================================================================
// Translation batch 72.  Only this file is edited by the batch.
//
// Batch 72 owns `0x00622000..=0x00623fff`.  The ledger lists 72 rows in the
// window; one is a `thunk` (`FUN_006226de`) and the other 71 are
// `decompiled_uncited`.  Thirty-four of the 71 are translated below and the
// rest are listed at the end with their reason.
//
// The window is not simulation code.  It is one C++ runtime / configuration
// cluster in four groups:
//
// * the `DAT_00faa150` / `DAT_00f04f2c` status probe and the two predicates
//   that read it (`0x00622878`, `0x00623610`, `0x00623620`);
// * the upper-cased config-key scan that the SEH bodies repeat
//   (`0x00622acd`, `0x0062303c`, `0x006232ea`) and the two inline-string head
//   resets (`0x00623119`, `0x0062312e`);
// * the `std::string` / hash-table helpers: substring find
//   (`0x00623680`), bucket placement (`0x00623700`, `0x006237c4`) and the
//   thirty-two-character hash decoder (`0x00623890`);
// * the `iostream` / `stringbuf` destruction sequence
//   (`0x00623a10`..`0x00623cf0`), the `debugLevel: ` stream report
//   (`0x00623d20`, `0x00623db6`, `0x00623e1c`) and the allocation report
//   (`0x00623ee0`).
//
// Recovered layouts:
//
// | layout | words |
// |--------|-------|
// | `ios_base` state | one word; only bit 0 (`\| 1`) is touched in this window |
// | `ostream` sink | one state word plus a `{put, end}` pair; `put == end` routes the byte through `overflow` |
// | inline string head | `+0x00` data word, `+0x04` size, `+0x08` capacity `0x8000000f`, `+0x0c` first byte |
// | hash table | `+0x00` bucket count, `+0x04` bucket array pointer, `+0x08` element count, `+0x0c` max load (`f32`) |
// | `stringbuf` (first form) | `+0x10` begin, `+0x14` end, `+0x1c` data, `+0x20` capacity, `+0x24` buffer tag |
// | `stringbuf` (second form) | `+0x28` begin, `+0x2c` end, `+0x34` data, `+0x38` capacity, `+0x3c` buffer tag |
//
// Every recovered body addresses its object through `ECX` and has no stack
// object argument, so the translations are value types and free functions.
// No `sim` record gains a field.
//
// Address to function:
//
// * `0x00622730` [`a72_client_sdk_literal`]
// * `0x00622878` [`a72_status_probe`]
// * `0x00622acd`, `0x0062303c`, `0x006232ea` [`a72_key_scan`]
// * `0x00623119`, `0x0062312e` [`a72_reset_inline_string`]
// * `0x00623610` [`a72_status_is_set`]
// * `0x00623620` [`a72_status_code`]
// * `0x00623680` [`a72_find_from`]
// * `0x00623700` [`a72_bucket_pair`]
// * `0x006237c4` [`a72_insert_and_maybe_grow`]
// * `0x00623890` [`a72_hash16_from_text`]
// * `0x00623900` [`A72Ostream::a72_print_cstr`]
// * `0x0062390b` [`A72Ostream::a72_print_cstr_unchecked`]
// * `0x0062394d` [`A72Ostream::a72_print_null`]
// * `0x00623990` [`A72Ostream::a72_endl`]
// * `0x00623a10` [`a72_assign_ios_base`]
// * `0x00623a20` [`a72_assign_istream`]
// * `0x00623a30` [`a72_assign_ostream`]
// * `0x00623a40` [`a72_assign_streambuf`]
// * `0x00623a4c` [`a72_iostream_destroy_sequence`]
// * `0x00623b00` [`a72_ios_base_dtor`]
// * `0x00623b30` [`a72_iostream_deleting_dtor`]
// * `0x00623b80` [`a72_istream_deleting_dtor`]
// * `0x00623bc0` [`a72_ostream_deleting_dtor`]
// * `0x00623c00` [`a72_streambuf_dtor`]
// * `0x00623c30` [`a72_stringbuf_destroy`]
// * `0x00623cc0` [`a72_stringstream_deleting_dtor`]
// * `0x00623cf0` [`a72_ios_base_dtor_alt`]
// * `0x00623d20` [`a72_debug_level_report`]
// * `0x00623db6` [`a72_debug_level_report64`]
// * `0x00623e1c` [`a72_badbit_after_write`]
// * `0x00623ee0` [`a72_memory_allocation_report`]
//
// [`A72HighWater`] / [`a72_high_water`] is the capacity update shared by the
// two `stringbuf` destructors (`0x00623a4c` uses the `+0x38` capacity word,
// `0x00623c30` the `+0x20` one).

// ---------------------------------------------------------------------------
// Literal contract: 0x00622730
// ---------------------------------------------------------------------------

/// Literal built by `0x00622730` before its `noreturn` call.
pub const A72_CLIENT_SDK: &str = "ClientSdk";

/// Length passed with [`A72_CLIENT_SDK`].
pub const A72_CLIENT_SDK_LEN: u32 = 9;

/// `0x00622730`: `FUN_00678940(local_68, "ClientSdk", 9);` then the frame
/// enters the `noreturn` helper `FUN_00538810` with the constructed object.
///
/// Only the literal and the explicit length are local to the row; the
/// constructor it feeds is out of batch.
pub fn a72_client_sdk_literal() -> (&'static str, u32) {
    (A72_CLIENT_SDK, A72_CLIENT_SDK_LEN)
}

// ---------------------------------------------------------------------------
// Status probe: 0x00622878, 0x00623610, 0x00623620
// ---------------------------------------------------------------------------

/// The two words `0x00622878` writes and the predicates read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A72StatusProbe {
    /// `DAT_00faa150`.
    pub status: u32,
    /// `DAT_00f04f2c`.
    pub mode: u32,
}

/// `0x00622878`: promote a successful probe, otherwise default the status.
///
/// The row terminates a buffer (`*buf = 0`, `len = i`) and runs two finders
/// on it:
///
/// ```text
/// if (find(...) == -1) {
///     if (find(...) == -1) {
///         if (DAT_00faa150 == 0) DAT_00faa150 = 2;
///         return;                     // DAT_00f04f2c untouched
///     }
/// }
/// DAT_00faa150 = 1;
/// DAT_00f04f2c = 5;
/// ```
///
/// `first` and `second` are those two results (`true` == found).  The unwind
/// blocks that follow only release the finders' temporaries.
pub fn a72_status_probe(first: bool, second: bool, status: u32, mode: u32) -> A72StatusProbe {
    if first || second {
        A72StatusProbe { status: 1, mode: 5 }
    } else {
        A72StatusProbe {
            status: if status == 0 { 2 } else { status },
            mode,
        }
    }
}

/// `0x00623610`: `return DAT_00faa150 != 0;`
pub fn a72_status_is_set(status: u32) -> bool {
    status != 0
}

/// `0x00623620`:
///
/// ```text
/// if (DAT_00faa150 != 0)
///     return CONCAT31((int3)(DAT_00faa150 >> 8), DAT_00faa150 == 1);
/// return (uint)(DAT_00f04f2c == 5);
/// ```
///
/// `CONCAT31` keeps the high 24 bits of the status word and replaces the low
/// byte with the equality result, so the arithmetic is a mask, not a plain
/// boolean.
pub fn a72_status_code(status: u32, mode: u32) -> u32 {
    if status != 0 {
        (status & 0xffff_ff00) | u32::from(status == 1)
    } else {
        u32::from(mode == 5)
    }
}

// ---------------------------------------------------------------------------
// Upper-cased key scan: 0x00622acd, 0x0062303c, 0x006232ea
// ---------------------------------------------------------------------------

/// Result of the three-part scan the three catch bodies share.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A72KeyScan {
    /// `(p + len) - p`, `0` when the 32-bit add wraps.
    pub span: u32,
    /// `FUN_00623680` result; `0xffff_ffff` when the key is absent.
    pub found_at: u32,
    /// `total - (found_at + 3)`, `None` when the key is absent.
    pub tail: Option<u32>,
}

/// The trip count of the byte loop: `(int)(p + len) - (int)p`, forced to `0`
/// when `p + len` wraps below `p`.
pub fn a72_span_len(base: u32, len: u32) -> u32 {
    if base.wrapping_add(len) < base {
        0
    } else {
        len
    }
}

/// `0x00622acd`, `0x0062303c`, `0x006232ea`: upper-case a config key, find
/// the literal at `0x00d3b290`, and keep the bytes after the key.
///
/// All three rows contain the same three-part body, differing only in the
/// length word they take from their frame:
///
/// ```text
/// span = (p + len) - p;  if (p + len < p) span = 0;
/// for (i = 0; i != span; ++i) p[i] = transform(p[i]);
/// pos = find(key_string, literal, 0);
/// if (pos == -1) { ...empty-string path... }
/// remaining = total - (pos + 3);
/// ```
///
/// * `0x0062303c` writes `buf[2] = 0` and `len = 2` first, so its `len_slot`
///   is 2;
/// * `0x006232ea` writes `buf[0x1a] = 0` and `len = 0x1a`, so its `len_slot`
///   is 0x1a;
/// * `0x00622acd` terminates at a length it receives and upper-cases the
///   string's own length word.
///
/// The transform helper at `FUN_00c93789` is not part of the row; the recovered
/// contract is the standard byte-wise upper-case, which is what the
/// translation applies.  `bytes` is the key string itself, so its length is
/// the haystack length the find uses.  The slice bound is a Rust-side bound:
/// the shipped buffer always holds at least `span` bytes.
pub fn a72_key_scan(
    bytes: &mut [u8],
    base: u32,
    len_slot: u32,
    needle: &[u8],
    total_len: u32,
) -> A72KeyScan {
    let span = a72_span_len(base, len_slot);
    let visited = (span as usize).min(bytes.len());
    for byte in &mut bytes[..visited] {
        *byte = byte.to_ascii_uppercase();
    }
    let found_at = a72_find_from(bytes, needle, 0);
    let tail = if found_at == u32::MAX {
        None
    } else {
        Some(total_len.wrapping_sub(found_at.wrapping_add(3)))
    };
    A72KeyScan {
        span,
        found_at,
        tail,
    }
}

// ---------------------------------------------------------------------------
// Inline string head: 0x00623119, 0x0062312e
// ---------------------------------------------------------------------------

/// Capacity word written by the two resets: 15 inline bytes plus the high bit.
pub const A72_INLINE_CAPACITY_WORD: u32 = 0x8000_000f;

/// The five stores the two reset rows perform on one string object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A72InlineStringHead {
    /// `+0x00`: `puVar1 + 3` words, i.e. the inline buffer at `+0x0c`.
    pub data_offset: u32,
    /// `+0x04`: zeroed.
    pub size: u32,
    /// `+0x08`: [`A72_INLINE_CAPACITY_WORD`].
    pub capacity: u32,
    /// `+0x0c`: the terminator byte.
    pub first_byte: u8,
    /// The frame word at `-0xf4`, which receives the pending length.
    pub pending_len: u32,
}

/// `0x00623119` and `0x0062312e`: reset a string to the inline form.
///
/// ```text
/// string[1] = 0;
/// string[0] = string + 3;      // +0x0c
/// string[2] = 0x8000000f;
/// *(char *)(string + 3) = 0;
/// frame[-0xf4] = pending_len;
/// ```
///
/// `0x0062312e` additionally calls the string's virtual destructor through
/// `*(*in_EAX + 0x10)` before the same five stores; `0x00623119` reaches them
/// from the adjacent unwind state.  Both rows end in a `noreturn` call with
/// `pending_len`, so it is returned as a field rather than dropped.
pub fn a72_reset_inline_string(pending_len: u32) -> A72InlineStringHead {
    A72InlineStringHead {
        data_offset: 0x0c,
        size: 0,
        capacity: A72_INLINE_CAPACITY_WORD,
        first_byte: 0,
        pending_len,
    }
}

// ---------------------------------------------------------------------------
// Substring find: 0x00623680
// ---------------------------------------------------------------------------

/// `0x00623680`: `std::string::find(const char *, pos)`.
///
/// The shipped body computes the needle length with the usual `do { } while
/// (*p++)` walk (an empty needle has length 0), then:
///
/// ```text
/// if (pos <= this->size) {
///     if (needle_len == 0) return pos;
///     end = needle_len + pos;
///     if (end <= this->size) do {
///         i = 0;
///         while (i < needle_len && this->data[pos + i] == needle[i]) ++i;
///         if (i == needle_len) return pos;
///         ++pos; ++end;
///     } while (end <= this->size);
/// }
/// return -1;
/// ```
///
/// `pos` is a `u32` because the row's length word and return value are
/// 32-bit; a miss returns `0xffff_ffff`.  Both increments wrap like the
/// shipped `uVar6 = uVar5 + param_2` add.
pub fn a72_find_from(haystack: &[u8], needle: &[u8], pos: u32) -> u32 {
    let hay_len = haystack.len() as u32;
    if pos > hay_len {
        return u32::MAX;
    }
    let needle_len = needle.len() as u32;
    if needle_len == 0 {
        return pos;
    }
    let mut start = pos;
    let mut end = needle_len.wrapping_add(start);
    if end <= hay_len {
        loop {
            if &haystack[start as usize..end as usize] == needle {
                return start;
            }
            start = start.wrapping_add(1);
            end = end.wrapping_add(1);
            if end > hay_len {
                break;
            }
        }
    }
    u32::MAX
}

// ---------------------------------------------------------------------------
// Hash bucket placement: 0x00623700, 0x006237c4
// ---------------------------------------------------------------------------

/// The two bucket-array offsets `0x00623700` returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A72BucketPair {
    /// `(count - 1) & hash`.
    pub bucket: u32,
    /// `bucket * 4`, the slot within the bucket array.
    pub slot_offset: u32,
    /// `count * 4`, the end of the bucket array.
    pub end_offset: u32,
}

/// `0x00623700`: locate the chain head for `hash`.
///
/// ```text
/// count = this[1];
/// if (count == 0) { grow(this[0]); count = this[1]; }
/// slot = this[0] + ((count - 1) & hash) * 4;
/// pair = *slot;  if (!pair) abort;
/// result = { slot, this[0] + count * 4 };
/// ```
///
/// `this[0]` is the bucket count and `this[1]` the bucket array pointer, so
/// the two returned words are the chain-head address and the array end.
/// `None` is the `count == 0` case: the row calls its out-of-batch grow
/// helper first and reloads the count, which cannot be predicted here.
pub fn a72_bucket_pair(count: u32, hash: u32) -> Option<A72BucketPair> {
    if count == 0 {
        return None;
    }
    let bucket = count.wrapping_sub(1) & hash;
    Some(A72BucketPair {
        bucket,
        slot_offset: bucket.wrapping_mul(4),
        end_offset: count.wrapping_mul(4),
    })
}

/// `0x006237c4`: truncate a string, count one element, grow on load.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct A72InsertOutcome {
    /// Index the row terminates the string at (`data[new_len] = 0`).
    pub terminator_at: u32,
    /// Size word written after the terminator (`this[1] = new_len`).
    pub string_size: u32,
    /// Element count after the increment.
    pub count: u32,
    /// Whether the capacity was doubled.
    pub grew: bool,
    /// Capacity used for the bucket and end offsets.
    pub capacity: u32,
    /// `(capacity - 1) & hash`.
    pub bucket: u32,
    /// `capacity * 4`.
    pub end_offset: u32,
    /// `count / capacity`, in `f32` like the row.
    pub load: f32,
}

/// `0x006237c4`: insert one element and double the bucket count when the
/// load exceeds the table's maximum.
///
/// ```text
/// data[new_len] = 0;  this[1] = new_len;
/// count = ++this[2];
/// capacity = this[0];
/// load = (float)count / (float)capacity;
/// if (load <= this[3]) { keep capacity; }
/// else { capacity *= 2; grow(); capacity = this[0]; }
/// bucket = (capacity - 1) & hash;
/// result = { array + bucket * 4, array + capacity * 4, flag = 1 };
/// ```
///
/// The comparison is the row's `fVar5 < max || fVar5 == max`, so a NaN load
/// takes the grow arm; Rust's `load <= max` has the same truth table.  A zero
/// capacity divides to infinity, takes the grow arm and stays zero in the
/// translation because the row's grow helper (which would install the first
/// real capacity) is out of batch.
pub fn a72_insert_and_maybe_grow(
    buckets: u32,
    elements: u32,
    max_load: f32,
    hash: u32,
    new_len: u32,
) -> A72InsertOutcome {
    let count = elements.wrapping_add(1);
    let load = count as f32 / buckets as f32;
    let grew = !(load <= max_load);
    let capacity = if grew {
        buckets.wrapping_mul(2)
    } else {
        buckets
    };
    let bucket = capacity.wrapping_sub(1) & hash;
    A72InsertOutcome {
        terminator_at: new_len,
        string_size: new_len,
        count,
        grew,
        capacity,
        bucket,
        end_offset: capacity.wrapping_mul(4),
        load,
    }
}

// ---------------------------------------------------------------------------
// Map hash text: 0x00623890
// ---------------------------------------------------------------------------

/// Message the wrong-length branch passes to `FUN_005f2dd0`.
pub const A72_MAP_HASH_LEN_MESSAGE: &str = "Unrecognized map file hash string length";

/// Why [`a72_hash16_from_text`] rejected its input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A72HashTextError {
    /// The record's length word was not `0x20`; carries the exact message.
    Length(&'static str),
    /// A byte outside `0-9a-fA-F`.  The row's converter body is not part of
    /// the row, so this arm is the translation's boundary.
    Digit(u8),
}

/// `0x00623890`: convert a 32-character hash string to 16 bytes.
///
/// ```text
/// if (record[1] == 0x20) { out[0..16] = convert(record); return; }
/// FUN_005f2dd0("Unrecognized map file hash string length");
/// ```
///
/// The row's contract is 32 characters in, 16 bytes out; the converter call
/// itself is out of batch, so the translation performs the standard hex-pair
/// decode that contract implies.  `text` is a fixed-size array because the
/// only length the row checks is the record's own length word.
pub fn a72_hash16_from_text(len: u32, text: &[u8; 32]) -> Result<[u8; 16], A72HashTextError> {
    if len != 0x20 {
        return Err(A72HashTextError::Length(A72_MAP_HASH_LEN_MESSAGE));
    }
    let mut out = [0u8; 16];
    for (index, pair) in text.chunks_exact(2).enumerate() {
        let hi = match a72_hex_nibble(pair[0]) {
            Some(value) => value,
            None => return Err(A72HashTextError::Digit(pair[0])),
        };
        let lo = match a72_hex_nibble(pair[1]) {
            Some(value) => value,
            None => return Err(A72HashTextError::Digit(pair[1])),
        };
        out[index] = (hi << 4) | lo;
    }
    Ok(out)
}

fn a72_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// ostream sink: 0x00623900, 0x0062390b, 0x0062394d, 0x00623990
// ---------------------------------------------------------------------------

/// The only `ios_base` state bit this window touches.
pub const A72_BADBIT: u32 = 0x1;

/// Literal the null-`char *` path writes: `"(null)"`, six bytes.
pub const A72_NULL_TEXT: &[u8] = b"(null)";

/// One `ostream` as the four rows above see it.
#[derive(Clone, Debug, PartialEq)]
pub struct A72Ostream {
    /// The `ios_base` state word read and written at `stream + 8`.
    pub state: u32,
    /// Bytes the sink accepted.
    pub bytes: Vec<u8>,
    /// `put != end` in the attached `streambuf`.
    pub space: bool,
    /// Return value of `sync()`; `false` is the `-1` case.
    pub sync_ok: bool,
    /// Number of `sync()` calls made.
    pub syncs: u32,
}

impl A72Ostream {
    /// Fresh sink with the given buffer room and flush result.
    pub fn a72_new(space: bool, sync_ok: bool) -> Self {
        A72Ostream {
            state: 0,
            bytes: Vec::new(),
            space,
            sync_ok,
            syncs: 0,
        }
    }

    /// `0x00623900`: `operator<<(ostream &, const char *)`.
    ///
    /// ```text
    /// if (s) { write(s, strlen(s)); return; }
    /// if (state == 0) {
    ///     if (write("(null)", 6) != 6) state |= 1;
    /// } else {
    ///     state |= 1;
    /// }
    /// ```
    ///
    /// The non-null arm forwards to `FUN_00624280`, whose body is out of
    /// batch; only the length computation is local, so the translation moves
    /// exactly `s.len()` bytes through the same sink and claims nothing about
    /// the flags on the non-null path.  The null arm is fully recovered and
    /// sets the badbit on a short write.
    pub fn a72_print_cstr(&mut self, s: Option<&[u8]>) {
        match s {
            Some(text) => {
                self.a72_print_cstr_unchecked(text);
            }
            None => self.a72_print_null(),
        }
    }

    /// `0x0062390b`: `strlen(s)` then the shipped `operator<<(const char *)`.
    ///
    /// ```text
    /// for (p = s; *p; ++p) { }
    /// FUN_00624280(s, p - (s + 1), 0);
    /// ```
    ///
    /// The translation performs the same byte count and returns how many
    /// bytes the sink accepted.
    pub fn a72_print_cstr_unchecked(&mut self, s: &[u8]) -> usize {
        self.a72_sink(s)
    }

    /// `0x0062394d`: the null-`char *` arm in isolation.
    ///
    /// `write("(null)", 6)`; a short write or an already-set state word sets
    /// `state |= 1`.  The decompiler's `if (*(int *)(... + 4) == 0) state |= 1;`
    /// repeats the same store after the first one, so it is not a second
    /// check.
    pub fn a72_print_null(&mut self) {
        if self.state != 0 {
            self.state |= A72_BADBIT;
            return;
        }
        let written = self.a72_sink(A72_NULL_TEXT);
        if written != A72_NULL_TEXT.len() {
            self.state |= A72_BADBIT;
        }
    }

    /// `0x00623990`: `operator<<(ostream &, endl)`.
    ///
    /// ```text
    /// if (state == 0) {
    ///     if (put != end) { *put++ = '\n'; }
    ///     else if (overflow('\n') == -1) state |= 1;
    /// } else state |= 1;
    /// if (rdbuf && state == 0) {
    ///     if (sync() == -1) state |= 1;
    /// }
    /// ```
    ///
    /// `space` models `put != end`; a full buffer takes the `overflow` arm,
    /// whose `-1` return is the only `overflow` behaviour this row shows.
    pub fn a72_endl(&mut self) {
        if self.state == 0 {
            if self.space {
                self.bytes.push(b'\n');
            } else {
                self.state |= A72_BADBIT;
            }
        } else {
            self.state |= A72_BADBIT;
        }
        if self.state == 0 {
            self.syncs = self.syncs.wrapping_add(1);
            if !self.sync_ok {
                self.state |= A72_BADBIT;
            }
        }
    }

    fn a72_sink(&mut self, data: &[u8]) -> usize {
        if !self.space {
            return 0;
        }
        self.bytes.extend_from_slice(data);
        data.len()
    }
}

// ---------------------------------------------------------------------------
// Virtual table identities: 0x00623a10, 0x00623a20, 0x00623a30, 0x00623a40
// ---------------------------------------------------------------------------

/// The virtual tables the destruction rows install.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A72Vtable {
    /// `blz::ios_base::vftable`.
    IosBase,
    /// `blz::basic_istream<char, ...>::vftable`.
    Istream,
    /// `blz::basic_ostream<char, ...>::vftable`.
    Ostream,
    /// `blz::basic_streambuf<char, ...>::vftable`.
    Streambuf,
    /// `blz::basic_iostream<char, ...>::vftable`.
    Iostream,
    /// `blz::basic_stringbuf<char, ...>::vftable`.
    Stringbuf,
    /// `blz::basic_stringstream<char, ...>::vftable`.
    Stringstream,
}

/// `0x00623a10`: `*in_ECX = blz::ios_base::vftable;`
pub fn a72_assign_ios_base() -> A72Vtable {
    A72Vtable::IosBase
}

/// `0x00623a20`: installs the `basic_istream` vtable at the virtual-base
/// adjusted address `*(*(ECX - 0x10) + 4) - 0x10 + ECX`.
pub fn a72_assign_istream() -> A72Vtable {
    A72Vtable::Istream
}

/// `0x00623a30`: installs the `basic_ostream` vtable at
/// `*(*(ECX - 4) + 4) - 4 + ECX`.
pub fn a72_assign_ostream() -> A72Vtable {
    A72Vtable::Ostream
}

/// `0x00623a40`: `*in_ECX = blz::basic_streambuf<char, ...>::vftable;`
pub fn a72_assign_streambuf() -> A72Vtable {
    A72Vtable::Streambuf
}

// ---------------------------------------------------------------------------
// stringbuf capacity high-water marks: shared by 0x00623a4c and 0x00623c30
// ---------------------------------------------------------------------------

/// The capacity update the two `stringbuf` destructors perform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A72HighWater {
    /// Capacity word after the update.
    pub capacity: u32,
    /// `Some(used)` when the row wrote a terminator at `used`.
    pub terminator_at: Option<u32>,
}

/// ```text
/// used = end - begin;
/// if (capacity < used) { data[used] = 0; capacity = used; }
/// ```
///
/// `0x00623a4c` uses `used = [0x2c] - [0x28]` with the capacity at `[0x38]`;
/// `0x00623c30` uses `used = [0x14] - [0x10]` with the capacity at `[0x20]`.
pub fn a72_high_water(used: u32, capacity: u32) -> A72HighWater {
    if capacity < used {
        A72HighWater {
            capacity: used,
            terminator_at: Some(used),
        }
    } else {
        A72HighWater {
            capacity,
            terminator_at: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Destructor plans: 0x00623a4c .. 0x00623cf0
// ---------------------------------------------------------------------------

/// What a deleting destructor does with its object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A72Delete {
    /// Plain destructor: no release.
    None,
    /// `FUN_00923e90(this)` without a size argument.
    Unsized,
    /// `FUN_00923e90(this, size)`.
    Sized(u32),
}

/// One recovered destructor body.
#[derive(Clone, Debug, PartialEq)]
pub struct A72DtorPlan {
    /// Byte offset from the row's `ECX` to the object the stores address.
    pub base_offset: i32,
    /// Vtable stores in program order, as `(byte offset, vtable)`.
    pub stores: Vec<(u32, A72Vtable)>,
    /// Capacity update, when the row performs one.
    pub high_water: Option<A72HighWater>,
    /// `Some(true)` when the row's buffer word has the sign bit clear (heap
    /// buffer released on the normal path), `Some(false)` for the inline
    /// buffer, `None` when the row has no buffer word.
    pub release_heap_buffer: Option<bool>,
    /// The row's release action.
    pub delete: A72Delete,
}

/// `0x00623a4c`: the `basic_iostream` destructor body.
///
/// ```text
/// used = [0x2c] - [0x28];
/// [0x18] = stringbuf vftable;
/// if ([0x38] < used) { data[used] = 0; [0x38] = used; }
/// if ([0x3c] >= 0) release buffer;              // normal path
/// [0x18] = streambuf vftable;
/// [*(*this + 4) + this]             = iostream vftable;
/// [*(this[4] + 4) + 0x10 + this]    = ostream vftable;
/// [*(*this + 4) + this]             = istream vftable;   // overwrites the first
/// [0x58] = ios_base vftable;
/// ```
///
/// The two base-object stores are in that order, so the final base vtable is
/// `istream`; the store list keeps the order rather than collapsing it.
pub fn a72_iostream_destroy_sequence(used: u32, capacity: u32, heap_buffer: bool) -> A72DtorPlan {
    A72DtorPlan {
        base_offset: 0,
        stores: vec![
            (0x18, A72Vtable::Stringbuf),
            (0x18, A72Vtable::Streambuf),
            (0x00, A72Vtable::Iostream),
            (0x10, A72Vtable::Ostream),
            (0x00, A72Vtable::Istream),
            (0x58, A72Vtable::IosBase),
        ],
        high_water: Some(a72_high_water(used, capacity)),
        release_heap_buffer: Some(heap_buffer),
        delete: A72Delete::None,
    }
}

/// `0x00623b00`: `*in_ECX = ios_base vftable;` then an unsized release when
/// bit 0 of the flags byte is set.
pub fn a72_ios_base_dtor(flags: u8) -> A72DtorPlan {
    A72DtorPlan {
        base_offset: 0,
        stores: vec![(0, A72Vtable::IosBase)],
        high_water: None,
        release_heap_buffer: None,
        delete: a72_delete_for(flags, A72Delete::Unsized),
    }
}

/// `0x00623b30`: deleting destructor of the `0x24`-byte `basic_iostream`.
///
/// `ECX` is the `ios_base` subobject at `base + 0x18`; the stores are
/// `iostream` and then `istream` at `base`, `ostream` at `base + 0x10` and
/// `ios_base` at `base + 0x18`, in that order, and the release is
/// `FUN_00923e90(base, 0x24)`.
pub fn a72_iostream_deleting_dtor(flags: u8) -> A72DtorPlan {
    A72DtorPlan {
        base_offset: -0x18,
        stores: vec![
            (0x00, A72Vtable::Iostream),
            (0x10, A72Vtable::Ostream),
            (0x00, A72Vtable::Istream),
            (0x18, A72Vtable::IosBase),
        ],
        high_water: None,
        release_heap_buffer: None,
        delete: a72_delete_for(flags, A72Delete::Sized(0x24)),
    }
}

/// `0x00623b80`: deleting destructor of the `0x1c`-byte `basic_istream`.
///
/// Base is `ECX - 0x10`; `istream` is stored at the base and `ios_base` at
/// `base + 0x10`; release is `FUN_00923e90(base, 0x1c)`.
pub fn a72_istream_deleting_dtor(flags: u8) -> A72DtorPlan {
    A72DtorPlan {
        base_offset: -0x10,
        stores: vec![(0x00, A72Vtable::Istream), (0x10, A72Vtable::IosBase)],
        high_water: None,
        release_heap_buffer: None,
        delete: a72_delete_for(flags, A72Delete::Sized(0x1c)),
    }
}

/// `0x00623bc0`: deleting destructor of the `0x10`-byte `basic_ostream`.
///
/// Base is `ECX - 4`; `ostream` is stored at the base and `ios_base` at
/// `base + 4`; release is `FUN_00923e90(base, 0x10)`.
pub fn a72_ostream_deleting_dtor(flags: u8) -> A72DtorPlan {
    A72DtorPlan {
        base_offset: -0x04,
        stores: vec![(0x00, A72Vtable::Ostream), (0x04, A72Vtable::IosBase)],
        high_water: None,
        release_heap_buffer: None,
        delete: a72_delete_for(flags, A72Delete::Sized(0x10)),
    }
}

/// `0x00623c00`: `*in_ECX = streambuf vftable;` then an unsized release.
pub fn a72_streambuf_dtor(flags: u8) -> A72DtorPlan {
    A72DtorPlan {
        base_offset: 0,
        stores: vec![(0, A72Vtable::Streambuf)],
        high_water: None,
        release_heap_buffer: None,
        delete: a72_delete_for(flags, A72Delete::Unsized),
    }
}

/// `0x00623c30`: `basic_stringbuf` destructor.
///
/// ```text
/// used = [0x14] - [0x10];
/// [0] = stringbuf vftable;
/// if ([0x20] < used) { data[used] = 0; [0x20] = used; }
/// if ([0x24] >= 0) release buffer;
/// [0] = streambuf vftable;
/// if (flags & 1) FUN_00923e90(this);            // unsized
/// ```
pub fn a72_stringbuf_destroy(
    used: u32,
    capacity: u32,
    heap_buffer: bool,
    flags: u8,
) -> A72DtorPlan {
    A72DtorPlan {
        base_offset: 0,
        stores: vec![(0x00, A72Vtable::Stringbuf), (0x00, A72Vtable::Streambuf)],
        high_water: Some(a72_high_water(used, capacity)),
        release_heap_buffer: Some(heap_buffer),
        delete: a72_delete_for(flags, A72Delete::Unsized),
    }
}

/// `0x00623cc0`: deleting destructor of the `0x64`-byte `basic_stringstream`.
///
/// The row calls the chained destructor at `0x00623a50` and then
/// `FUN_00923e90(ECX - 0x58, 100)`.  Nothing else is recovered from it.
pub fn a72_stringstream_deleting_dtor(flags: u8) -> A72DtorPlan {
    A72DtorPlan {
        base_offset: -0x58,
        stores: Vec::new(),
        high_water: None,
        release_heap_buffer: None,
        delete: a72_delete_for(flags, A72Delete::Sized(0x64)),
    }
}

/// `0x00623cf0`: `*in_ECX = ios_base vftable;` then an unsized release.
///
/// Same two stores as `0x00623b00`; the ledger keeps them as separate rows.
pub fn a72_ios_base_dtor_alt(flags: u8) -> A72DtorPlan {
    A72DtorPlan {
        base_offset: 0,
        stores: vec![(0, A72Vtable::IosBase)],
        high_water: None,
        release_heap_buffer: None,
        delete: a72_delete_for(flags, A72Delete::Unsized),
    }
}

fn a72_delete_for(flags: u8, action: A72Delete) -> A72Delete {
    if flags & 1 != 0 {
        action
    } else {
        A72Delete::None
    }
}

// ---------------------------------------------------------------------------
// debugLevel report: 0x00623d20, 0x00623db6, 0x00623e1c
// ---------------------------------------------------------------------------

/// Label written before the parsed level.
pub const A72_DEBUG_LEVEL_LABEL: &[u8] = b"debugLevel: ";

/// One `debugLevel: ` report as `0x00623d20` builds it.
#[derive(Clone, Debug, PartialEq)]
pub struct A72DebugLevelReport {
    /// `basic_stringstream` vtable constructed for the report.
    pub vtable: A72Vtable,
    /// The parsed level, kept only when it is positive.
    pub level: Option<i32>,
    /// `prefix || "debugLevel: " || level? || '\n'`.
    pub text: Vec<u8>,
}

/// `0x00623d20`: build the `debugLevel: ` line and report the positive level.
///
/// ```text
/// write(prefix, 4);
/// write("debugLevel: ", 12);
/// parsed = parse(..., "%d", ...);
/// if (parsed > 0) write(parsed);
/// endl(stream);
/// text = stream.str();
/// ```
///
/// The sign test is the decompiler's
/// `-1 < (iVar3 >> 31) && (iVar3 < 0 || iVar3 != 0)`, which for a 32-bit
/// value is exactly `iVar3 > 0`.  The row's first write is the four bytes at
/// `0x00d3b3e8`, which are not part of the row, so the prefix is a slice
/// argument here.
pub fn a72_debug_level_report(prefix: &[u8], parsed: i32) -> A72DebugLevelReport {
    let level = if parsed > 0 { Some(parsed) } else { None };
    let mut text = Vec::new();
    text.extend_from_slice(prefix);
    text.extend_from_slice(A72_DEBUG_LEVEL_LABEL);
    if let Some(value) = level {
        text.extend_from_slice(value.to_string().as_bytes());
    }
    text.push(b'\n');
    A72DebugLevelReport {
        vtable: A72Vtable::Stringstream,
        level,
        text,
    }
}

/// One `debugLevel: ` report from the 64-bit frame.
#[derive(Clone, Debug, PartialEq)]
pub struct A72DebugLevelReport64 {
    /// `basic_stringstream` vtable constructed for the report.
    pub vtable: A72Vtable,
    /// The parsed level, kept only when it is positive.
    pub level: Option<i64>,
    /// `prefix || "debugLevel: " || level? || '\n'`.
    pub text: Vec<u8>,
}

/// `0x00623db6`: the 64-bit form of [`a72_debug_level_report`].
///
/// The row compares the low word and its sign extension separately
/// (`(int)value == (int)result && (int)(value >> 32) == result >> 31`), which
/// is the 64-bit `value > 0` test; the text arithmetic is the same.
pub fn a72_debug_level_report64(prefix: &[u8], parsed: i64) -> A72DebugLevelReport64 {
    let level = if parsed > 0 { Some(parsed) } else { None };
    let mut text = Vec::new();
    text.extend_from_slice(prefix);
    text.extend_from_slice(A72_DEBUG_LEVEL_LABEL);
    if let Some(value) = level {
        text.extend_from_slice(value.to_string().as_bytes());
    }
    text.push(b'\n');
    A72DebugLevelReport64 {
        vtable: A72Vtable::Stringstream,
        level,
        text,
    }
}

/// `0x00623e1c`: the flag update that follows the level write.
///
/// ```text
/// if (state == 0) {
///     if (write(level) == level) goto done;
///     state = reload(state);
/// }
/// state |= 1;
/// if (next_word == 0) state |= 1;    // same store, repeated
/// done: endl(stream); text = stream.str();
/// ```
///
/// `wrote_entire` is the exact-write test.  The row's remaining work is the
/// `endl` and the string length, both already reproduced by
/// [`A72Ostream::a72_endl`] and [`a72_debug_level_report`].
pub fn a72_badbit_after_write(state: u32, wrote_entire: bool) -> u32 {
    if state == 0 && wrote_entire {
        0
    } else {
        state | A72_BADBIT
    }
}

// ---------------------------------------------------------------------------
// Memory allocation report: 0x00623ee0
// ---------------------------------------------------------------------------

/// Title literal passed to the report sink.
pub const A72_MEMORY_INFO_TITLE: &str = "Memory allocation info:";

/// Second argument address of the report sink call (`&DAT_00d3b408`).
pub const A72_MEMORY_INFO_TAG_ADDRESS: u32 = 0x00d3_b408;

/// One allocation report as `0x00623ee0` composes it.
#[derive(Clone, Debug, PartialEq)]
pub struct A72MemoryAllocationReport {
    /// The fixed title.
    pub title: &'static str,
    /// The tag pointer the row passes as the second sink argument.
    pub tag_address: u32,
    /// The `debugLevel: ` text from [`a72_debug_level_report`].
    pub level_text: Vec<u8>,
}

/// `0x00623ee0`: `"Memory allocation info:"` plus the `debugLevel: ` line.
///
/// ```text
/// FUN_00633bd0(this, "Memory allocation info:", cookie);
/// level_text = FUN_00623d20();
/// (*vtable[0])(this, &DAT_00d3b408, *level_text);
/// ```
///
/// The sink call is out of batch; what the row fixes is the title and the tag
/// address.  The level text is produced by the translated `0x00623d20`
/// helper, so the composition is reproduced without the sink.
pub fn a72_memory_allocation_report(prefix: &[u8], parsed: i32) -> A72MemoryAllocationReport {
    A72MemoryAllocationReport {
        title: A72_MEMORY_INFO_TITLE,
        tag_address: A72_MEMORY_INFO_TAG_ADDRESS,
        level_text: a72_debug_level_report(prefix, parsed).text,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent72 {
    use super::*;

    // -- 0x00622730 -------------------------------------------------------

    #[test]
    fn client_sdk_literal_is_exact() {
        assert_eq!(a72_client_sdk_literal(), ("ClientSdk", 9));
        assert_eq!(A72_CLIENT_SDK.len(), 9);
    }

    // -- 0x00622878 / 0x00623610 / 0x00623620 -----------------------------

    #[test]
    fn status_probe_promotes_either_hit() {
        assert_eq!(
            a72_status_probe(true, false, 0, 0),
            A72StatusProbe { status: 1, mode: 5 }
        );
        assert_eq!(
            a72_status_probe(false, true, 7, 9),
            A72StatusProbe { status: 1, mode: 5 }
        );
        assert_eq!(
            a72_status_probe(true, true, 0, 0),
            A72StatusProbe { status: 1, mode: 5 }
        );
    }

    #[test]
    fn status_probe_defaults_only_from_zero() {
        assert_eq!(
            a72_status_probe(false, false, 0, 0),
            A72StatusProbe { status: 2, mode: 0 }
        );
        assert_eq!(
            a72_status_probe(false, false, 3, 4),
            A72StatusProbe { status: 3, mode: 4 }
        );
        assert_eq!(
            a72_status_probe(false, false, 2, 5),
            A72StatusProbe { status: 2, mode: 5 }
        );
    }

    #[test]
    fn status_predicates_use_the_raw_word() {
        assert!(!a72_status_is_set(0));
        assert!(a72_status_is_set(1));
        assert!(a72_status_is_set(2));

        assert_eq!(a72_status_code(0, 5), 1);
        assert_eq!(a72_status_code(0, 4), 0);
        assert_eq!(a72_status_code(1, 9), 1);
        assert_eq!(a72_status_code(2, 9), 0);
        // CONCAT31 keeps the high 24 bits of the status word.
        assert_eq!(a72_status_code(0x0000_0102, 9), 0x0000_0100);
        assert_eq!(a72_status_code(0xabcd_ef01, 0), 0xabcd_ef00);
    }

    // -- 0x00622acd / 0x0062303c / 0x006232ea -----------------------------

    #[test]
    fn span_len_matches_the_pointer_difference() {
        assert_eq!(a72_span_len(0x1000, 2), 2);
        assert_eq!(a72_span_len(0x1000, 0x1a), 0x1a);
        assert_eq!(a72_span_len(0xffff_fffe, 4), 0);
        assert_eq!(a72_span_len(0xffff_ffff, 1), 0);
    }

    #[test]
    fn key_scan_uppercases_then_keeps_the_tail() {
        let mut key = *b"debugLevel: 7\0extra";
        let scan = a72_key_scan(&mut key, 0x1000, 0x0d, b"DEBUGLEVEL:", 0x10);
        assert_eq!(scan.span, 0x0d);
        assert_eq!(&key[..0x0d], b"DEBUGLEVEL: 7");
        // The key occupies thirteen bytes and starts at zero.
        assert_eq!(scan.found_at, 0);
        assert_eq!(scan.tail, Some(0x10u32.wrapping_sub(3)));
    }

    #[test]
    fn key_scan_prefix_1a_and_absent_key() {
        let mut key = [0u8; 0x1a];
        key[..4].copy_from_slice(b"abca");
        let scan = a72_key_scan(&mut key, 0x2000, 0x1a, b"BCA", 0x1a);
        assert_eq!(scan.span, 0x1a);
        assert_eq!(scan.found_at, 1);
        assert_eq!(scan.tail, Some(0x1au32.wrapping_sub(4)));

        let mut absent = [0u8; 2];
        absent.copy_from_slice(b"zz");
        let scan = a72_key_scan(&mut absent, 0, 2, b"QQ", 9);
        assert_eq!(scan.found_at, u32::MAX);
        assert_eq!(scan.tail, None);
    }

    #[test]
    fn key_scan_tail_wraps_like_the_row() {
        let mut key = *b"ab";
        let scan = a72_key_scan(&mut key, 0, 2, b"AB", 2);
        assert_eq!(scan.found_at, 0);
        assert_eq!(scan.tail, Some(u32::MAX)); // 2 - 3
    }

    // -- 0x00623119 / 0x0062312e ------------------------------------------

    #[test]
    fn inline_string_reset_head() {
        let head = a72_reset_inline_string(0x1234);
        assert_eq!(
            head,
            A72InlineStringHead {
                data_offset: 0x0c,
                size: 0,
                capacity: A72_INLINE_CAPACITY_WORD,
                first_byte: 0,
                pending_len: 0x1234,
            }
        );
        assert_eq!(head.capacity, 0x8000_000f);
    }

    // -- 0x00623680 -------------------------------------------------------

    #[test]
    fn find_from_matches_and_misses() {
        assert_eq!(a72_find_from(b"abcabc", b"abc", 0), 0);
        assert_eq!(a72_find_from(b"abcabc", b"abc", 1), 3);
        assert_eq!(a72_find_from(b"abcabc", b"cab", 0), 2);
        assert_eq!(a72_find_from(b"abcabc", b"abc", 4), u32::MAX);
        assert_eq!(a72_find_from(b"abc", b"abcd", 0), u32::MAX);
        assert_eq!(a72_find_from(b"aaaa", b"aa", 2), 2);
        assert_eq!(a72_find_from(b"aaaa", b"aa", 3), u32::MAX);
        assert_eq!(a72_find_from(b"", b"a", 0), u32::MAX);
    }

    #[test]
    fn find_from_empty_needle_returns_pos() {
        assert_eq!(a72_find_from(b"abc", b"", 0), 0);
        assert_eq!(a72_find_from(b"abc", b"", 3), 3);
        assert_eq!(a72_find_from(b"abc", b"", 4), u32::MAX);
    }

    // -- 0x00623700 / 0x006237c4 ------------------------------------------

    #[test]
    fn bucket_pair_masks_the_hash() {
        assert_eq!(a72_bucket_pair(0, 0x1234), None);

        let pair = a72_bucket_pair(8, 0x1234).unwrap();
        assert_eq!(pair.bucket, 0x1234 & 7);
        assert_eq!(pair.slot_offset, (0x1234 & 7) * 4);
        assert_eq!(pair.end_offset, 32);

        // The mask is `count - 1`, so a non power of two count is no mask.
        let pair = a72_bucket_pair(6, 0x1234).unwrap();
        assert_eq!(pair.bucket, 0x1234 & 5);
        assert_eq!(pair.end_offset, 24);
    }

    #[test]
    fn insert_without_growth_keeps_the_capacity() {
        let out = a72_insert_and_maybe_grow(8, 2, 0.75, 0x1234, 0x2a);
        assert_eq!(out.terminator_at, 0x2a);
        assert_eq!(out.string_size, 0x2a);
        assert_eq!(out.count, 3);
        assert!(!out.grew);
        assert_eq!(out.capacity, 8);
        assert_eq!(out.bucket, 0x1234 & 7);
        assert_eq!(out.end_offset, 32);
        assert_eq!(out.load, 3.0 / 8.0);
    }

    #[test]
    fn insert_at_the_exact_load_does_not_grow() {
        // 6 / 8 == 0.75, and the row grows only when the load is greater.
        let out = a72_insert_and_maybe_grow(8, 5, 0.75, 0x1234, 0);
        assert_eq!(out.count, 6);
        assert_eq!(out.load, 0.75);
        assert!(!out.grew);
        assert_eq!(out.capacity, 8);
    }

    #[test]
    fn insert_grows_past_the_load() {
        let out = a72_insert_and_maybe_grow(8, 6, 0.75, 0x1234, 0);
        assert_eq!(out.count, 7);
        assert!(out.grew);
        assert_eq!(out.capacity, 16);
        assert_eq!(out.bucket, 0x1234 & 15);
        assert_eq!(out.end_offset, 64);
    }

    #[test]
    fn insert_with_nan_load_takes_the_grow_arm() {
        let out = a72_insert_and_maybe_grow(8, 6, f32::NAN, 0x1234, 0);
        assert!(out.grew);
        assert_eq!(out.capacity, 16);
        assert_eq!(out.end_offset, 64);
    }

    // -- 0x00623890 -------------------------------------------------------

    #[test]
    fn hash16_decodes_thirty_two_hex_digits() {
        let text = *b"000102030405060708090a0b0c0d0e0f";
        assert_eq!(
            a72_hash16_from_text(0x20, &text),
            Ok([
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f,
            ])
        );
        let upper = *b"FFEEDDCCBBAA99887766554433221100";
        assert_eq!(
            a72_hash16_from_text(0x20, &upper).unwrap()[..4],
            [0xff, 0xee, 0xdd, 0xcc]
        );
    }

    #[test]
    fn hash16_reports_the_exact_length_message() {
        let text = *b"000102030405060708090a0b0c0d0e0f";
        assert_eq!(
            a72_hash16_from_text(0x1f, &text),
            Err(A72HashTextError::Length(
                "Unrecognized map file hash string length"
            ))
        );
        assert_eq!(A72_MAP_HASH_LEN_MESSAGE.len(), 40);
    }

    #[test]
    fn hash16_flags_a_non_hex_digit() {
        let text = *b"000102030405060708090a0b0c0d0e0z";
        assert_eq!(
            a72_hash16_from_text(0x20, &text),
            Err(A72HashTextError::Digit(b'z'))
        );
    }

    // -- 0x00623900 / 0x0062390b / 0x0062394d / 0x00623990 ----------------

    #[test]
    fn print_cstr_writes_the_text() {
        let mut stream = A72Ostream::a72_new(true, true);
        stream.a72_print_cstr(Some(b"abc"));
        assert_eq!(stream.bytes, b"abc");
        assert_eq!(stream.state, 0);
    }

    #[test]
    fn print_null_writes_the_six_byte_literal() {
        let mut stream = A72Ostream::a72_new(true, true);
        stream.a72_print_cstr(None);
        assert_eq!(stream.bytes, b"(null)");
        assert_eq!(A72_NULL_TEXT.len(), 6);
        assert_eq!(stream.state, 0);
    }

    #[test]
    fn null_short_write_sets_the_badbit() {
        let mut stream = A72Ostream::a72_new(false, true);
        stream.a72_print_null();
        assert!(stream.bytes.is_empty());
        assert_eq!(stream.state, A72_BADBIT);

        // A stream that is already bad is left bad and writes nothing.
        let mut bad = A72Ostream::a72_new(true, true);
        bad.state = 0x10;
        bad.a72_print_null();
        assert_eq!(bad.state, 0x11);
        assert!(bad.bytes.is_empty());
    }

    #[test]
    fn endl_writes_newline_then_syncs() {
        let mut stream = A72Ostream::a72_new(true, true);
        stream.a72_endl();
        assert_eq!(stream.bytes, b"\n");
        assert_eq!(stream.state, 0);
        assert_eq!(stream.syncs, 1);
    }

    #[test]
    fn endl_with_a_full_buffer_sets_the_badbit() {
        let mut stream = A72Ostream::a72_new(false, true);
        stream.a72_endl();
        assert!(stream.bytes.is_empty());
        assert_eq!(stream.state, A72_BADBIT);
        assert_eq!(stream.syncs, 0);
    }

    #[test]
    fn endl_sync_failure_sets_the_badbit_after_the_newline() {
        let mut stream = A72Ostream::a72_new(true, false);
        stream.a72_endl();
        assert_eq!(stream.bytes, b"\n");
        assert_eq!(stream.state, A72_BADBIT);
        assert_eq!(stream.syncs, 1);
    }

    #[test]
    fn endl_on_a_bad_stream_only_keeps_the_flag() {
        let mut stream = A72Ostream::a72_new(true, true);
        stream.state = 0x2;
        stream.a72_endl();
        assert!(stream.bytes.is_empty());
        assert_eq!(stream.state, 0x3);
        assert_eq!(stream.syncs, 0);
    }

    // -- 0x00623e1c -------------------------------------------------------

    #[test]
    fn badbit_after_write_truth_table() {
        assert_eq!(a72_badbit_after_write(0, true), 0);
        assert_eq!(a72_badbit_after_write(0, false), A72_BADBIT);
        assert_eq!(a72_badbit_after_write(4, true), 5);
        assert_eq!(a72_badbit_after_write(4, false), 5);
    }

    // -- 0x00623a10 .. 0x00623a40 -----------------------------------------

    #[test]
    fn vtable_assignments_name_the_four_tables() {
        assert_eq!(a72_assign_ios_base(), A72Vtable::IosBase);
        assert_eq!(a72_assign_istream(), A72Vtable::Istream);
        assert_eq!(a72_assign_ostream(), A72Vtable::Ostream);
        assert_eq!(a72_assign_streambuf(), A72Vtable::Streambuf);
    }

    // -- 0x00623a4c / 0x00623c30 ------------------------------------------

    #[test]
    fn high_water_updates_only_when_capacity_is_short() {
        assert_eq!(
            a72_high_water(0x20, 0x10),
            A72HighWater {
                capacity: 0x20,
                terminator_at: Some(0x20),
            }
        );
        assert_eq!(
            a72_high_water(0x10, 0x20),
            A72HighWater {
                capacity: 0x20,
                terminator_at: None,
            }
        );
        assert_eq!(
            a72_high_water(0x10, 0x10),
            A72HighWater {
                capacity: 0x10,
                terminator_at: None,
            }
        );
    }

    #[test]
    fn iostream_destroy_sequence_keeps_the_store_order() {
        let plan = a72_iostream_destroy_sequence(0x30, 0x20, true);
        assert_eq!(
            plan.stores,
            vec![
                (0x18, A72Vtable::Stringbuf),
                (0x18, A72Vtable::Streambuf),
                (0x00, A72Vtable::Iostream),
                (0x10, A72Vtable::Ostream),
                (0x00, A72Vtable::Istream),
                (0x58, A72Vtable::IosBase),
            ]
        );
        assert_eq!(plan.high_water.unwrap().capacity, 0x30);
        assert_eq!(plan.high_water.unwrap().terminator_at, Some(0x30));
        assert_eq!(plan.release_heap_buffer, Some(true));
        assert_eq!(plan.delete, A72Delete::None);
    }

    #[test]
    fn stringbuf_destroy_reports_capacity_and_release() {
        let plan = a72_stringbuf_destroy(0x18, 0x10, false, 0);
        assert_eq!(
            plan.stores,
            vec![(0x00, A72Vtable::Stringbuf), (0x00, A72Vtable::Streambuf),]
        );
        assert_eq!(
            plan.high_water,
            Some(A72HighWater {
                capacity: 0x18,
                terminator_at: Some(0x18),
            })
        );
        assert_eq!(plan.release_heap_buffer, Some(false));
        assert_eq!(plan.delete, A72Delete::None);
        assert_eq!(
            a72_stringbuf_destroy(0x18, 0x10, false, 1).delete,
            A72Delete::Unsized
        );
    }

    // -- 0x00623b00 .. 0x00623cf0 -----------------------------------------

    #[test]
    fn base_destructors_use_unsized_release() {
        for (plan, vtable) in [
            (a72_ios_base_dtor(0), A72Vtable::IosBase),
            (a72_ios_base_dtor_alt(0), A72Vtable::IosBase),
            (a72_streambuf_dtor(0), A72Vtable::Streambuf),
        ] {
            assert_eq!(plan.base_offset, 0);
            assert_eq!(plan.stores, vec![(0, vtable)]);
            assert_eq!(plan.delete, A72Delete::None);
        }
        assert_eq!(a72_ios_base_dtor(1).delete, A72Delete::Unsized);
        assert_eq!(a72_streambuf_dtor(0x81).delete, A72Delete::Unsized);
        assert_eq!(a72_ios_base_dtor_alt(0x80).delete, A72Delete::None);
    }

    #[test]
    fn deleting_destructors_carry_the_recovered_sizes() {
        let iostream = a72_iostream_deleting_dtor(1);
        assert_eq!(iostream.base_offset, -0x18);
        assert_eq!(iostream.delete, A72Delete::Sized(0x24));
        assert_eq!(
            iostream.stores,
            vec![
                (0x00, A72Vtable::Iostream),
                (0x10, A72Vtable::Ostream),
                (0x00, A72Vtable::Istream),
                (0x18, A72Vtable::IosBase),
            ]
        );

        let istream = a72_istream_deleting_dtor(1);
        assert_eq!(istream.base_offset, -0x10);
        assert_eq!(istream.delete, A72Delete::Sized(0x1c));
        assert_eq!(
            istream.stores,
            vec![(0x00, A72Vtable::Istream), (0x10, A72Vtable::IosBase)]
        );

        let ostream = a72_ostream_deleting_dtor(1);
        assert_eq!(ostream.base_offset, -0x04);
        assert_eq!(ostream.delete, A72Delete::Sized(0x10));
        assert_eq!(
            ostream.stores,
            vec![(0x00, A72Vtable::Ostream), (0x04, A72Vtable::IosBase)]
        );

        let stringstream = a72_stringstream_deleting_dtor(1);
        assert_eq!(stringstream.base_offset, -0x58);
        assert_eq!(stringstream.delete, A72Delete::Sized(0x64));
        assert!(stringstream.stores.is_empty());

        assert_eq!(a72_stringstream_deleting_dtor(0).delete, A72Delete::None);
    }

    // -- 0x00623d20 / 0x00623db6 / 0x00623ee0 -----------------------------

    #[test]
    fn debug_level_report_keeps_only_positive_levels() {
        let report = a72_debug_level_report(b"log:", 7);
        assert_eq!(report.vtable, A72Vtable::Stringstream);
        assert_eq!(report.level, Some(7));
        assert_eq!(report.text, b"log:debugLevel: 7\n");

        assert_eq!(a72_debug_level_report(b"log:", 0).level, None);
        assert_eq!(a72_debug_level_report(b"log:", -3).level, None);
        assert_eq!(
            a72_debug_level_report(b"log:", 0).text,
            b"log:debugLevel: \n"
        );
        assert_eq!(
            a72_debug_level_report(b"log:", i32::MIN).text,
            b"log:debugLevel: \n"
        );
        assert_eq!(
            a72_debug_level_report(b"log:", i32::MAX).text,
            b"log:debugLevel: 2147483647\n"
        );
    }

    #[test]
    fn debug_level_report64_uses_the_same_rule() {
        let report = a72_debug_level_report64(b"", 1_i64 << 40);
        assert_eq!(report.level, Some(1_i64 << 40));
        assert_eq!(report.text, b"debugLevel: 1099511627776\n");
        assert_eq!(a72_debug_level_report64(b"", 0).level, None);
        assert_eq!(a72_debug_level_report64(b"", i64::MIN).level, None);
    }

    #[test]
    fn memory_allocation_report_composes_the_level_line() {
        let report = a72_memory_allocation_report(b"mem:", 5);
        assert_eq!(report.title, "Memory allocation info:");
        assert_eq!(report.tag_address, 0x00d3_b408);
        assert_eq!(report.level_text, b"mem:debugLevel: 5\n");
        assert_eq!(A72_MEMORY_INFO_TITLE.len(), 23);
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// The 37 `decompiled_uncited` rows not translated above, grouped by why:
//
// * `FUN_00622097`, `FUN_0062234f`, `FUN_00622505`: the decompiler hit bad
//   instruction data immediately (`halt_baddata`); the rows start at a
//   truncated entry point, so no behaviour is recoverable.
// * `FUN_00622134`, `FUN_00622256`: stems whose first visible store is
//   `*in_EAX = *in_EAX + (char)in_EAX` followed by a `noreturn` throw; the
//   overlap means the operand bytes are already the next instruction.
// * `FUN_006226ea`, `FUN_006229ab`: unwind-list restores and a terminate call;
//   no local stores besides the frame words the unwinder consumes.
// * `FUN_006228ec`, `FUN_0062290b`, `FUN_00622919`, `FUN_00622927`,
//   `FUN_00622949`, `FUN_0062296b`, `FUN_0062322b`, `FUN_00623248`,
//   `FUN_0062325b`, `FUN_00623265`, `FUN_00623285`: SEH destructor
//   funclets.  Each calls one virtual `*(*in_EAX + 0x10)` and then
//   `FUN_0093ec70`/`FUN_00c86b6e` with a stack state byte from a caller
//   outside the page; there is no local arithmetic to reproduce.
// * `FUN_00622cea`, `FUN_00622d05`, `FUN_00622d28`, `FUN_00622d45`,
//   `FUN_00622d62`, `FUN_00622f18`, `FUN_00622f33`, `FUN_00622f5a`,
//   `FUN_00622f7d`, `FUN_00622fa0`: ten copies of the catch body translated
//   by [`a72_key_scan`] (upper-case span, find, `total - (pos + 3)`).  They
//   differ only in the unwind state byte and frame slot numbers, so they are
//   not duplicated as separate functions.
// * `FUN_00622c87`: catch body that formats `"CONNECTION_STRING_"` through
//   three out-of-batch helpers (`FUN_00555760`, `FUN_0056d3c0`, `FUN_007554b0`) and
//   then enters the `noreturn` path; only the literal is local, and a literal
//   alone is not the row's behaviour.
// * `FUN_0062301c`: one store (`frame[-0x60] = 0x14`) immediately before the
//   `noreturn` call; no computation.
// * `FUN_006231f2`: truncation plus `frame[-0xf8] = EDI | 9`; `EDI` is a
//   caller register, so the tag word cannot be reconstructed.
// * `FUN_00623320`, `FUN_006233e9`: jump-table bodies the decompiler marked
//   as bad instruction data (dozens of removed unreachable blocks, overlapping
//   instructions, `swi`).  The visible text contains non-canonical stores such
//   as `_DAT_12cd501f`, so no source-level behaviour can be claimed.
// * `FUN_00623870`, `FUN_006238e0`: two-instruction forwarders to
//   `FUN_00715a60` / `FUN_00715e10` (both out of batch) that return `param_1`
//   unchanged; nothing local beyond the call.
// * `FUN_00623916`: starts with `swi(4)` and a mid-instruction stack add.
// * `FUN_00623f80`: varargs forwarder to `FUN_00c661d0` with the caller's
//   stack pointer; the argument list is not part of the row.
// * `FUN_006226de`: ledger status `thunk`, not `decompiled_uncited`.
