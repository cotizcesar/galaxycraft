#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave11/agent02.rs
// ===========================================================================
// Wave 11, batch 02 — console/UI settings records in
// `0x0064a000..=0x0064bfff`.
//
// The window is the front-end half of the console-skin machinery: three
// `0x78`-byte player records, a "current selection" word that lives in the
// default record, a lazily initialised settings singleton, and a small
// memo table that caches a per-record byte.  The callers are the console
// screen, the options screen and the skin picker.
//
// ## Recovered layout
//
// | address      | meaning                                                  |
// |--------------|----------------------------------------------------------|
// | `0x00fc9b88` | first of three `0x78`-byte records (slots 0..3)           |
// | `0x00fc9cf0` | the default record, `0x00fc9b88 + 3 * 0x78`               |
// | `+0x24`      | 16-bit record id (the word the setters compare)           |
// | `0x00fc9d14` | the default record's id word = current selection          |
// | `0x00f05848` | memo table bucket count (a power of two)                  |
// | `0x00f0584c` | memo table bucket chains (`next` at `+0`, key at `+4`)     |
//
// The records and the singleton are process globals in the shipped build;
// the translations here model the *state* they hold and keep the caller's
// buffers explicit, so every routine is a free function over borrowed data.
//
// ## Rows translated
//
// `0x0064a000`, `0x0064a010`, `0x0064a9c0`, `0x0064ae80`, `0x0064b2b0`,
// `0x0064b2d0`, `0x0064b510`, `0x0064b790`, `0x0064b7e7`, `0x0064b840`,
// `0x0064b930`, `0x0064b9d0`, `0x0064bd50`, `0x0064bfb0`.
//
// The remaining rows of the window are not translated: their decompiled
// bodies are obfuscated jump-table soup (bad-instruction warnings, overlapping
// instructions and an unrecovered stack spacebase), compiler-generated
// destructor thunks whose only content is a virtual-slot dispatch under a
// structured-exception frame, or a straight-line sequence of out-of-window
// calls followed by a non-returning abort.

// ---------------------------------------------------------------------------
// Recovered layout: the three console records
// ---------------------------------------------------------------------------

/// `0x00fc9b88`: first of the three `0x78`-byte console/player records.
///
/// The slot setters index this array with the requested slot and fall back to
/// the default record for every value outside `0..3`.
pub const W11A02_RECORD_BASE: u32 = 0x00fc_9b88;

/// `0x78`: stride between two console/player records.
pub const W11A02_RECORD_STRIDE: u32 = 0x0078;

/// `3`: number of real slot records; larger slots fall back to the default.
pub const W11A02_RECORD_SLOTS: u16 = 3;

/// `0x00fc9cf0`: the default record, `0x00fc9b88 + 3 * 0x78`.
pub const W11A02_RECORD_DEFAULT: u32 =
    W11A02_RECORD_BASE + W11A02_RECORD_SLOTS as u32 * W11A02_RECORD_STRIDE;

/// `+0x24`: the 16-bit record id inside a console/player record.
///
/// The memo lookup in `0x0064b2d0` passes this word as the key
/// (`*(undefined2 *)(record + 0x24)`).
pub const W11A02_RECORD_ID_OFFSET: u32 = 0x24;

/// `0x00fc9d14`: the "current selection" word.
///
/// The address is exactly the default record's id field
/// (`0x00fc9cf0 + 0x24`); the slot setters compare the requested slot against
/// it and the front-end commands pass its value straight back as a slot index.
pub const W11A02_SELECTION_WORD: u32 = W11A02_RECORD_DEFAULT + W11A02_RECORD_ID_OFFSET;

// ---------------------------------------------------------------------------
// Row 0x0064a000 / 0x0064a010: the two lambda RTTI descriptors
// ---------------------------------------------------------------------------

/// A recovered `TypeDescriptor` for one of the closures the compiler emitted
/// in this window.
///
/// The shipped routines are six-byte lea/ret pairs that return the address of
/// a static descriptor; the descriptor name below is the mangled lambda tag
/// the decompiler printed for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W11A02TypeDescriptor {
    /// Mangled lambda name the `TypeDescriptor` describes.
    pub mangled_name: &'static str,
    /// Address of the descriptor the row returns (`0x0064a000`/`0x0064a010`).
    pub descriptor: u32,
}

static W11A02_DESCRIPTOR_288F: W11A02TypeDescriptor = W11A02TypeDescriptor {
    mangled_name: "lambda_288f98074645ff1ea1cf1ecbd259e14d",
    descriptor: 0x0064_a000,
};

static W11A02_DESCRIPTOR_3B36: W11A02TypeDescriptor = W11A02TypeDescriptor {
    mangled_name: "lambda_3b365bb9b7028c2eef8edd2e8a7a3c1c",
    descriptor: 0x0064_a010,
};

/// `0x0064a000`: descriptor of `lambda_288f98074645ff1ea1cf1ecbd259e14d`.
///
/// The body is `return &<lambda_288f...>::RTTI_Type_Descriptor;`, so the
/// address handed out is a process-lifetime static and stays identical across
/// calls.
pub fn w11a02_type_descriptor_288f() -> &'static W11A02TypeDescriptor {
    &W11A02_DESCRIPTOR_288F
}

/// `0x0064a010`: descriptor of `lambda_3b365bb9b7028c2eef8edd2e8a7a3c1c`.
///
/// Same six-byte lea/ret shape as `0x0064a000`; the two rows differ only in
/// the descriptor they hand back.
pub fn w11a02_type_descriptor_3b36() -> &'static W11A02TypeDescriptor {
    &W11A02_DESCRIPTOR_3B36
}

// ---------------------------------------------------------------------------
// Row 0x0064a9c0: initialiser of the `forcedCarbot` local static
// ---------------------------------------------------------------------------

/// `"forcedCarbot"`: the key literal walked by the initialiser `0x0064a9c0`.
///
/// It is the only piece of state the row exposes; `0x0064bd50` registers the
/// sibling key `"skinsEnabled"` the same way.
pub const W11A02_FORCED_CARBOT_KEY: &str = "forcedCarbot";

/// `0x0064a9c0`: run the dynamic initialiser of the `forcedCarbot` static.
///
/// The row builds a structured-exception frame, calls its out-of-window
/// initialiser with the security-cookie word, then walks the `"forcedCarbot"`
/// literal to compute its length before entering a non-returning diagnostic
/// routine.  Because the walk is a plain `strlen`, the model runs the caller's
/// initialiser with the same cookie and returns the literal together with the
/// length the scan computes.
pub fn w11a02_init_forced_carbot(
    cookie: u32,
    initialise: impl FnOnce(u32),
) -> (&'static str, usize) {
    initialise(cookie);
    (W11A02_FORCED_CARBOT_KEY, W11A02_FORCED_CARBOT_KEY.len())
}

// ---------------------------------------------------------------------------
// Row 0x0064ae80: memoised per-record byte lookup
// ---------------------------------------------------------------------------

/// `0x85ebca6b` = `-0x7a143595`: first multiplier of the lookup hash.
pub const W11A02_MEMO_HASH_MUL: u32 = 0x85eb_ca6b;

/// `0xc2b2ae35` = `-0x3d4d51cb`: second multiplier of the lookup hash.
pub const W11A02_MEMO_HASH_MUL2: u32 = 0xc2b2_ae35;

/// `0x00f05848`: memo table bucket count (the code masks with `count - 1`).
pub const W11A02_MEMO_BUCKETS: u32 = 0x00f0_5848;

/// `0x00f0584c`: memo table bucket-chain heads.
pub const W11A02_MEMO_CHAINS: u32 = 0x00f0_584c;

/// The chained memo table behind `0x00f05848`/`0x00f0584c`.
///
/// A node is `{ next: *mut Node, key: u16 @ +4, .. }`; the probe in
/// `0x0064ae80` walks the chain while comparing the 16-bit key, then reads the
/// byte through the table's `operator[]`.  The model keeps the same chained
/// buckets but owns the entries, so no raw pointers are involved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W11A02ByteMemo {
    buckets: Vec<Vec<(u16, u8)>>,
    active: bool,
}

/// Create an empty memo table with `bucket_count` chained buckets.
///
/// The shipped code masks the hash with the bucket-count word minus one, which
/// only yields a valid index for a power-of-two count.
pub fn w11a02_byte_memo(bucket_count: u32) -> W11A02ByteMemo {
    assert!(
        bucket_count.is_power_of_two(),
        "the shipped index mask needs a power-of-two bucket count"
    );
    W11A02ByteMemo {
        buckets: vec![Vec::new(); bucket_count as usize],
        active: false,
    }
}

/// Whether the table exists yet (`0x00f0584c != 0` in the shipped build).
///
/// The table is created by the first insertion, which is why the probe in
/// `0x0064ae80` is skipped while the chain head is null.
pub fn w11a02_memo_active(memo: &W11A02ByteMemo) -> bool {
    memo.active
}

/// Bucket index for `key`, exactly as `0x0064ae80` computes it.
///
/// `h = key * 0x85ebca6b; h = (h >> 13 ^ h) * 0xc2b2ae35;`
/// `index = (h >> 16 ^ h) & (buckets - 1)`.
pub fn w11a02_memo_bucket(key: u16, buckets: u32) -> u32 {
    let scaled = u32::from(key).wrapping_mul(W11A02_MEMO_HASH_MUL);
    let folded = (scaled >> 13) ^ scaled;
    let mixed = folded.wrapping_mul(W11A02_MEMO_HASH_MUL2);
    ((mixed >> 16) ^ mixed) & buckets.wrapping_sub(1)
}

/// Probe the chain for `key`; the first matching node wins.
pub fn w11a02_memo_probe(memo: &W11A02ByteMemo, key: u16) -> Option<u8> {
    let bucket = w11a02_memo_bucket(key, memo.buckets.len() as u32) as usize;
    memo.buckets[bucket]
        .iter()
        .find(|(entry_key, _)| *entry_key == key)
        .map(|(_, value)| *value)
}

fn w11a02_memo_insert(memo: &mut W11A02ByteMemo, key: u16, value: u8) {
    let bucket = w11a02_memo_bucket(key, memo.buckets.len() as u32) as usize;
    memo.active = true;
    match memo.buckets[bucket]
        .iter_mut()
        .find(|(entry_key, _)| *entry_key == key)
    {
        Some(entry) => entry.1 = value,
        None => memo.buckets[bucket].push((key, value)),
    }
}

/// `0x0064ae80`: the cached per-record byte.
///
/// The shipped routine:
///
/// * returns `0` immediately for the zero key, without touching the table;
/// * probes the chained table when `param_2 == 0` **and** the table exists,
///   returning the cached byte on a key hit;
/// * otherwise calls the per-key handler (a `ushort`-indexed call through a
///   code table, i.e. out-of-window code), stores its result with the map's
///   `operator[]`, and returns the stored byte.
///
/// `bypass_cache` is the `param_2` flag: a nonzero value skips the probe, so
/// the handler runs again even for a key that is already cached (the result is
/// still written back).  All callers pass the record id at `+0x24` as the key.
pub fn w11a02_cached_record_byte(
    memo: &mut W11A02ByteMemo,
    key: u16,
    bypass_cache: bool,
    compute: impl FnOnce(u16) -> u8,
) -> u8 {
    if key == 0 {
        return 0;
    }
    if !bypass_cache && memo.active {
        if let Some(cached) = w11a02_memo_probe(memo, key) {
            return cached;
        }
    }
    let value = compute(key);
    w11a02_memo_insert(memo, key, value);
    value
}

// ---------------------------------------------------------------------------
// Rows 0x0064b2b0 / 0x0064b2d0: the two state predicates
// ---------------------------------------------------------------------------

/// The primary state answer returned by the query used here (`1`).
pub const W11A02_STATE_PRIMARY: u32 = 1;

/// The alternate state answer (`2`) that `0x0064b2b0` accepts.
pub const W11A02_STATE_ALTERNATE: u32 = 2;

/// `0x0064b2b0`: "is the record in the alternate state?".
///
/// The body forwards the query word to a state routine that lives outside this
/// window (it folds the settings flag at `+4`, an option check and the memo
/// lookup into a `1`/`2` answer) and compares the answer with `2`.  The
/// recovered predicate is that comparison, so it takes the resolved answer.
pub fn w11a02_state_is_alternate(state: u32) -> bool {
    state == W11A02_STATE_ALTERNATE
}

/// `0x0064b2d0`: the layered "special variant available" gate.
///
/// The shipped conjunction is the settings byte at `+4` of the singleton, an
/// option query on the caller's table, and the memoised per-record byte for
/// the selected record's id at `+0x24` -- all three have to be nonzero.  The
/// three sources are passed in already resolved; the shipped short-circuit
/// order is preserved (the option query is only reached when the settings byte
/// is set).
pub fn w11a02_skin_gate(settings_flag: u8, option_enabled: bool, variant_available: bool) -> bool {
    settings_flag != 0 && option_enabled && variant_available
}

// ---------------------------------------------------------------------------
// Row 0x0064b930: console record selection
// ---------------------------------------------------------------------------

/// `0x0064b930`: index of the record selected for `slot`.
///
/// The routine replaces the console record pointer with
/// `&0x00fc9b88 + slot * 0x78` for `slot < 3` and with `&0x00fc9cf0`
/// (the default record) for every larger slot, so the effective index is
/// `min(slot, 3)`.  `0x0064b510` starts with the same selection block.
pub fn w11a02_record_index(slot: u16) -> u16 {
    slot.min(W11A02_RECORD_SLOTS)
}

/// `0x0064b930`: address of the record selected for `slot`.
///
/// Same block as [`w11a02_record_index`]: slot 0..=2 selects
/// `0x00fc9b88 ... 0x00fc9c78`, everything else selects `0x00fc9cf0`.
pub fn w11a02_record_address(slot: u16) -> u32 {
    W11A02_RECORD_BASE + u32::from(w11a02_record_index(slot)) * W11A02_RECORD_STRIDE
}

// ---------------------------------------------------------------------------
// Row 0x0064b510: selection flag written by the slot setter
// ---------------------------------------------------------------------------

/// `0x0064b510` / `0x0064b9d0` / `0x0064b840`: byte `+0x60` of the settings
/// singleton, which records that the last write settled.
pub const W11A02_SETTINGS_APPLIED_OFFSET: u32 = 0x60;

/// `0x0064b510`: the flag a slot request asks for.
///
/// The setter prepares its request from
/// `(ushort)param_1 != DAT_00fc9d14`, where `DAT_00fc9d14` is the current
/// selection word ([`W11A02_SELECTION_WORD`]).  The flag therefore means
/// "the requested slot differs from the current one".
pub fn w11a02_selection_flag(current_selection: u16, requested_slot: u16) -> bool {
    requested_slot != current_selection
}

/// `0x0064b510`: whether the request must trigger the refresh path.
///
/// Once the request has been written, the routine compares the settings byte
/// at `+4` (read before the write) against the flag of
/// [`w11a02_selection_flag`]:
///
/// ```text
/// if (old_flag != (requested_slot != DAT_00fc9d14)) {
///     write the new flag; run the refresh path;
/// }
/// ```
///
/// Only a *change* of the derived flag refreshes, so re-requesting the current
/// slot with the flag already stored is a no-op.
pub fn w11a02_selection_refresh_needed(
    current_selection: u16,
    requested_slot: u16,
    stored_flag: bool,
) -> bool {
    stored_flag != w11a02_selection_flag(current_selection, requested_slot)
}

// ---------------------------------------------------------------------------
// Row 0x0064b790: unlink a console node and destroy its payload
// ---------------------------------------------------------------------------

/// `1`: the word the destructor stores into a released payload slot.
pub const W11A02_PAYLOAD_EMPTY: u32 = 1;

/// The low bit of a payload word: `1` means the payload lives on the heap and
/// the word holds `pointer | 1`; `0` means the payload is stored inline.
pub const W11A02_PAYLOAD_HEAP_TAG: u32 = 1;

/// The two link words at the head of a console node.
///
/// The shipped links are absolute addresses in a circular list whose sentinel
/// is the list header; the model uses indices into a node array, node `0`
/// being the sentinel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W11A02NodeLinks {
    /// Index of the next node (the word at `+0x00`).
    pub next: usize,
    /// Index of the previous node (the word at `+0x04`).
    pub prev: usize,
}

/// Decoded state of the payload word at node `+0x08`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W11A02Payload {
    /// The word is [`W11A02_PAYLOAD_EMPTY`]: the payload was already released.
    Empty,
    /// The low bit is clear: the payload object lives inline in the node.
    Inline,
    /// The low bit is set: the payload lives at `word & !1`.
    Heap(u32),
}

/// What [`w11a02_release_payload`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W11A02Release {
    /// The word was already the empty marker.
    AlreadyEmpty,
    /// The inline payload's virtual destructor ran in place.
    InlineDestroyed,
    /// The heap payload's virtual destructor ran and its storage was freed.
    HeapDestroyed(u32),
}

/// `0x0064b790`: unlink `node` from its circular list.
///
/// The shipped body performs `node->next->prev = node->prev` and
/// `node->prev->next = node->next`, which works for a circular list with a
/// sentinel header and for a middle node alike.
pub fn w11a02_unlink_node(nodes: &mut [W11A02NodeLinks], node: usize) {
    let next = nodes[node].next;
    let prev = nodes[node].prev;
    nodes[next].prev = prev;
    nodes[prev].next = next;
}

/// Decode the payload word at node `+0x08`.
///
/// The empty marker is checked first: it is `1`, so the plain low-bit test
/// would otherwise read it as a heap pointer to null.  The shipped destructor
/// can only reach a released node through a stale list entry, which the
/// unlink step prevents, so the ordering is unobservable there.
pub fn w11a02_payload_word(raw: u32) -> W11A02Payload {
    if raw == W11A02_PAYLOAD_EMPTY {
        W11A02Payload::Empty
    } else if raw & W11A02_PAYLOAD_HEAP_TAG != 0 {
        W11A02Payload::Heap(raw & !W11A02_PAYLOAD_HEAP_TAG)
    } else {
        W11A02Payload::Inline
    }
}

/// `0x0064b790`: destroy the payload and mark the slot empty.
///
/// The routine picks the object to destroy (the node itself for an inline
/// payload, `word & !1` for a heap payload), calls its virtual destructor
/// slot 0 with a zero delete flag, frees the storage when the low bit was set,
/// and stores `1` into the payload word.
pub fn w11a02_release_payload(raw: &mut u32) -> W11A02Release {
    let outcome = match w11a02_payload_word(*raw) {
        W11A02Payload::Empty => W11A02Release::AlreadyEmpty,
        W11A02Payload::Inline => W11A02Release::InlineDestroyed,
        W11A02Payload::Heap(storage) => W11A02Release::HeapDestroyed(storage),
    };
    *raw = W11A02_PAYLOAD_EMPTY;
    outcome
}

// ---------------------------------------------------------------------------
// Row 0x0064b7e7: fold a byte into the settings state word
// ---------------------------------------------------------------------------

/// `0x0064b7e7`: OR `bits` into the state byte and accept only zero.
///
/// The row is two instructions of real work -- `*in_ECX |= in_AL` -- followed
/// by a guard that takes the non-returning error path for every nonzero merged
/// value.  The `Ok` arm therefore only covers the all-zero result; the merged
/// byte is reported either way.
pub fn w11a02_fold_state_bits(state: u8, bits: u8) -> Result<u8, u8> {
    let merged = state | bits;
    if merged == 0 { Ok(merged) } else { Err(merged) }
}

// ---------------------------------------------------------------------------
// Row 0x0064b840: write a setting value and settle the applied byte
// ---------------------------------------------------------------------------

/// The settings state touched by the value writer `0x0064b840`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W11A02WriteTarget {
    /// The value the request carries (`*param_1`); the writer owns the actual
    /// store, so it is not modified by the dispatch below.
    pub value: u8,
    /// Byte `+0x60`: set while the last write is considered settled.
    pub applied: bool,
}

/// Result of [`w11a02_apply_setting_write`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W11A02WriteOutcome {
    /// The optional writer object at `+0x78` handled the request; `+0x60`
    /// became `1`.
    OverrideHandled,
    /// The base writer accepted the request; `+0x60` is unchanged.
    Written,
    /// The base writer rejected the request; `+0x60` was forced to the
    /// keep/clear policy and the disable notification ran when it cleared.
    Rejected { applied: bool },
}

/// `0x0064b840`: apply one setting write and update the settled byte.
///
/// The dispatch is:
///
/// 1. if the settings object has the optional writer at `+0x78`, call its
///    virtual slot 1 with the request; a **zero** return means handled, and
///    `+0x60` is set to `1`;
/// 2. otherwise call the base writer; a nonzero return means accepted and
///    `+0x60` is left alone;
/// 3. a zero return from the base writer rejects the request: the request is
///    cleaned up, then `param_2` decides -- `0` runs the disable notification
///    and clears `+0x60`, nonzero keeps `+0x60` set.
///
/// Both writer results are passed in already resolved (the polarity above is
/// converted to `accepted`), and `notify_disabled` stands for the out-of-window
/// call the rejection path makes.
pub fn w11a02_apply_setting_write(
    target: &mut W11A02WriteTarget,
    keep_when_rejected: bool,
    override_accepted: Option<bool>,
    base_accepted: bool,
    notify_disabled: impl FnOnce(),
) -> W11A02WriteOutcome {
    if override_accepted == Some(true) {
        target.applied = true;
        return W11A02WriteOutcome::OverrideHandled;
    }
    if base_accepted {
        return W11A02WriteOutcome::Written;
    }
    if keep_when_rejected {
        target.applied = true;
        W11A02WriteOutcome::Rejected { applied: true }
    } else {
        notify_disabled();
        target.applied = false;
        W11A02WriteOutcome::Rejected { applied: false }
    }
}

// ---------------------------------------------------------------------------
// Row 0x0064b9d0: one-byte setting change detection
// ---------------------------------------------------------------------------

/// `0x0064b9d0`: whether a one-byte setting request must refresh.
///
/// The row reads the settings byte at `+4` before writing, then compares it
/// with the requested byte (`(short *)(param_1 + 0xe) == 10` and the constant
/// `1` are the two call sites).  A difference writes the new value and runs
/// the refresh path; an equal value does nothing.
pub fn w11a02_byte_setting_refresh_needed(stored: u8, requested: u8) -> bool {
    stored != requested
}

// ---------------------------------------------------------------------------
// Row 0x0064bd50: the lazily initialised skins singleton
// ---------------------------------------------------------------------------

/// `"skinsEnabled"`: profile key of the settings the singleton loads.
pub const W11A02_SKINS_ENABLED_KEY: &str = "skinsEnabled";

/// The skins singleton returned by `0x0064bd50`.
///
/// The shipped object is larger; the byte this window reads is `+4` (tested by
/// `0x0064b2d0` and by the state query behind `0x0064b2b0`), so the model keeps
/// that fact and nothing else.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W11A02SkinSettings {
    /// Byte `+4` of the singleton: the "skins enabled" flag.
    pub skins_enabled: bool,
}

/// `0x0064bd50`: get the skins singleton, initialising it once.
///
/// The row is the compiler's function-local-static guard: it tests the
/// thread-local magic-static counter and runs the dynamic initialiser (which
/// registers the `"skinsEnabled"` key and reads the profile) before returning
/// the singleton pointer.  A [`OnceLock`] reproduces the exactly-once
/// semantics without the guard words.
pub fn w11a02_skin_settings(
    cell: &OnceLock<W11A02SkinSettings>,
    load: impl FnOnce() -> W11A02SkinSettings,
) -> &W11A02SkinSettings {
    cell.get_or_init(load)
}

// ---------------------------------------------------------------------------
// Row 0x0064bfb0: the empty inline string constructor
// ---------------------------------------------------------------------------

/// `0x0f`: capacity of the inline buffer, in bytes.
pub const W11A02_SSO_CAPACITY: u32 = 0x0f;

/// `0x80000000`: the bit the empty constructor ORs into the capacity word.
pub const W11A02_SSO_CAPACITY_FLAG: u32 = 0x8000_0000;

/// `0x8000000f`: the capacity word of a freshly constructed empty string.
pub const W11A02_SSO_CAPACITY_WORD: u32 = W11A02_SSO_CAPACITY_FLAG | W11A02_SSO_CAPACITY;

/// Bytes in the inline buffer (`0x0f`).
pub const W11A02_SSO_BUFFER_BYTES: usize = W11A02_SSO_CAPACITY as usize;

/// The small-string object built by `0x0064bfb0`.
///
/// The shipped layout is `{ pointer, size, capacity, inline buffer }` with the
/// pointer aimed at the object's own buffer; the model drops the self pointer
/// and keeps the size, the capacity word and the buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W11A02SmallString {
    /// Inline storage (`+0x0c`, `0x0f` bytes).
    pub buffer: [u8; W11A02_SSO_BUFFER_BYTES],
    /// Current length (`+0x04`), zero for the freshly built string.
    pub len: u32,
    /// Capacity word (`+0x08`), [`W11A02_SSO_CAPACITY_WORD`] when empty.
    pub capacity: u32,
}

/// `0x0064bfb0`: construct the empty inline string.
///
/// The body stores `ptr = self + 0x0c`, `size = 0`, `capacity = 0x8000000f`,
/// `buffer[0] = 0`, and then enters a non-returning routine with the length of
/// a 0x800-byte path template (the constructor is reached from a failed path
/// conversion).  Only the constructor's own state is recoverable, so that is
/// what the model builds.
pub fn w11a02_empty_small_string() -> W11A02SmallString {
    W11A02SmallString {
        buffer: [0; W11A02_SSO_BUFFER_BYTES],
        len: 0,
        capacity: W11A02_SSO_CAPACITY_WORD,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_wave11_agent02 {
    use super::*;
    use std::cell::Cell;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// `0x0064a000` / `0x0064a010`: stable, distinct descriptors.
    #[test]
    fn type_descriptors_match_0x0064a000_and_0x0064a010() {
        let first = w11a02_type_descriptor_288f();
        let again = w11a02_type_descriptor_288f();
        assert!(std::ptr::eq(first, again));
        assert_eq!(first.descriptor, 0x0064_a000);
        assert_eq!(
            first.mangled_name,
            "lambda_288f98074645ff1ea1cf1ecbd259e14d"
        );

        let second = w11a02_type_descriptor_3b36();
        assert!(!std::ptr::eq(first, second));
        assert_eq!(second.descriptor, 0x0064_a010);
        assert_eq!(
            second.mangled_name,
            "lambda_3b365bb9b7028c2eef8edd2e8a7a3c1c"
        );
        assert_ne!(first, second);
    }

    /// `0x0064a9c0`: the initialiser runs once with the cookie and reports the
    /// 12-byte key literal.
    #[test]
    fn forced_carbot_initialiser_matches_0x0064a9c0() {
        let seen = Cell::new(0u32);
        let (key, len) = w11a02_init_forced_carbot(0x00f4_e578, |cookie| seen.set(cookie));
        assert_eq!(seen.get(), 0x00f4_e578);
        assert_eq!(key, "forcedCarbot");
        assert_eq!(len, 12);
        assert_eq!(len, key.len());
    }

    /// `0x0064ae80`: the recovered hash lands on the expected buckets.
    #[test]
    fn memo_hash_matches_0x0064ae80() {
        assert_eq!(w11a02_memo_bucket(0x0001, 8), 7);
        assert_eq!(w11a02_memo_bucket(0x0002, 8), 6);
        assert_eq!(w11a02_memo_bucket(0x0003, 8), 7);
        assert_eq!(w11a02_memo_bucket(0x0024, 8), 0);
        assert_eq!(w11a02_memo_bucket(0x1234, 8), 5);
        assert_eq!(w11a02_memo_bucket(0xffff, 8), 7);
        // The id word of the default record maps to bucket 0 at both sizes.
        assert_eq!(w11a02_memo_bucket(0x0024, 16), 0);
        assert_eq!(w11a02_memo_bucket(0x1234, 16), 5);
        // Multiplying by the (wrapping) negative constant is the recovered
        // `* -0x7a143595`.
        assert_eq!(
            u32::from(0x0024u16).wrapping_mul(W11A02_MEMO_HASH_MUL),
            0xd528_770c
        );
    }

    /// `0x0064ae80`: memoised value, the zero-key short circuit and the
    /// `param_2` bypass.
    #[test]
    fn cached_lookup_matches_0x0064ae80() {
        let mut memo = w11a02_byte_memo(8);
        assert!(!w11a02_memo_active(&memo));
        assert_eq!(w11a02_memo_probe(&memo, 0x0024), None);

        let calls = Cell::new(0);
        let compute = |key: u16| {
            calls.set(calls.get() + 1);
            key as u8 ^ 0x5a
        };
        let first = w11a02_cached_record_byte(&mut memo, 0x0024, false, compute);
        assert_eq!(first, 0x24 ^ 0x5a);
        assert_eq!(calls.get(), 1);
        assert!(w11a02_memo_active(&memo));
        assert_eq!(w11a02_memo_probe(&memo, 0x0024), Some(0x7e));

        // A cached key is served without running the handler.
        let again = w11a02_cached_record_byte(&mut memo, 0x0024, false, compute);
        assert_eq!(again, first);
        assert_eq!(calls.get(), 1);

        // The zero key never reaches the table or the handler.
        let zero = w11a02_cached_record_byte(&mut memo, 0, false, compute);
        assert_eq!(zero, 0);
        assert_eq!(calls.get(), 1);
        assert_eq!(w11a02_memo_probe(&memo, 0), None);

        // `param_2 != 0` skips the probe: the handler runs again and its new
        // value replaces the cache entry.
        let bypassed = w11a02_cached_record_byte(&mut memo, 0x0024, true, |key| {
            calls.set(calls.get() + 1);
            key as u8
        });
        assert_eq!(bypassed, 0x24);
        assert_eq!(calls.get(), 2);
        assert_eq!(w11a02_memo_probe(&memo, 0x0024), Some(0x24));
    }

    /// `0x0064ae80`: colliding keys stay distinct inside one chain.
    #[test]
    fn memo_chain_keeps_colliding_keys_apart() {
        let mut memo = w11a02_byte_memo(8);
        assert_eq!(w11a02_memo_bucket(1, 8), w11a02_memo_bucket(3, 8));
        assert_eq!(w11a02_memo_bucket(1, 8), w11a02_memo_bucket(0xffff, 8));
        for key in [1u16, 3, 0xffff] {
            assert_eq!(
                w11a02_cached_record_byte(&mut memo, key, false, |k| (k & 0xff) as u8),
                (key & 0xff) as u8
            );
        }
        assert_eq!(w11a02_memo_probe(&memo, 1), Some(1));
        assert_eq!(w11a02_memo_probe(&memo, 3), Some(3));
        assert_eq!(w11a02_memo_probe(&memo, 0xffff), Some(0xff));
        assert_eq!(w11a02_memo_probe(&memo, 2), None);
    }

    /// `0x0064b2b0`: only the alternate answer passes.
    #[test]
    fn state_predicate_matches_0x0064b2b0() {
        assert!(w11a02_state_is_alternate(W11A02_STATE_ALTERNATE));
        assert!(!w11a02_state_is_alternate(W11A02_STATE_PRIMARY));
        assert!(!w11a02_state_is_alternate(0));
        assert!(!w11a02_state_is_alternate(3));
        assert_eq!((W11A02_STATE_PRIMARY, W11A02_STATE_ALTERNATE), (1, 2));
    }

    /// `0x0064b2d0`: all three layers must agree, in the shipped order.
    #[test]
    fn skin_gate_matches_0x0064b2d0() {
        assert!(w11a02_skin_gate(1, true, true));
        assert!(w11a02_skin_gate(0xff, true, true));
        assert!(!w11a02_skin_gate(0, true, true));
        assert!(!w11a02_skin_gate(1, false, true));
        assert!(!w11a02_skin_gate(1, true, false));
        assert!(!w11a02_skin_gate(0, false, false));
    }

    /// `0x0064b930`: `min(slot, 3)` selects the record.
    #[test]
    fn record_selection_matches_0x0064b930() {
        assert_eq!(w11a02_record_index(0), 0);
        assert_eq!(w11a02_record_index(1), 1);
        assert_eq!(w11a02_record_index(2), 2);
        assert_eq!(w11a02_record_index(3), 3);
        assert_eq!(w11a02_record_index(0xffff), 3);

        assert_eq!(w11a02_record_address(0), 0x00fc_9b88);
        assert_eq!(w11a02_record_address(1), 0x00fc_9c00);
        assert_eq!(w11a02_record_address(2), 0x00fc_9c78);
        assert_eq!(w11a02_record_address(3), 0x00fc_9cf0);
        assert_eq!(w11a02_record_address(0x1234), 0x00fc_9cf0);
        assert_eq!(W11A02_RECORD_DEFAULT, w11a02_record_address(3));
        // The selection word is the default record's `+0x24` id field.
        assert_eq!(W11A02_SELECTION_WORD, 0x00fc_9d14);
        assert_eq!(
            W11A02_SELECTION_WORD,
            W11A02_RECORD_DEFAULT + W11A02_RECORD_ID_OFFSET
        );
    }

    /// `0x0064b510`: the request flag is "different from the selection", and
    /// only a changed flag refreshes.
    #[test]
    fn selection_flag_matches_0x0064b510() {
        assert!(!w11a02_selection_flag(2, 2));
        assert!(w11a02_selection_flag(2, 1));
        assert!(w11a02_selection_flag(3, 0));

        // Stored flag already equals the requested one: nothing to do.
        assert!(!w11a02_selection_refresh_needed(2, 1, true));
        assert!(!w11a02_selection_refresh_needed(2, 2, false));
        // Request flips the derived flag: refresh.
        assert!(w11a02_selection_refresh_needed(2, 2, true));
        assert!(w11a02_selection_refresh_needed(2, 1, false));
    }

    /// `0x0064b790`: unlink keeps the circular list intact.
    #[test]
    fn unlink_matches_0x0064b790() {
        // Node 0 is the sentinel header; 1..=3 are live.
        let mut nodes = [
            W11A02NodeLinks { next: 1, prev: 3 },
            W11A02NodeLinks { next: 2, prev: 0 },
            W11A02NodeLinks { next: 3, prev: 1 },
            W11A02NodeLinks { next: 0, prev: 2 },
        ];
        w11a02_unlink_node(&mut nodes, 2);
        assert_eq!(nodes[0].next, 1);
        assert_eq!(nodes[0].prev, 3);
        assert_eq!(nodes[1].next, 3);
        assert_eq!(nodes[3].prev, 1);
        // Node 2's own links are untouched, exactly like the shipped copy.
        assert_eq!(nodes[2], W11A02NodeLinks { next: 3, prev: 1 });

        // Removing the sentinel's neighbour keeps the ring closed.
        w11a02_unlink_node(&mut nodes, 1);
        assert_eq!(nodes[0].next, 3);
        assert_eq!(nodes[3].prev, 0);
        assert_eq!(nodes[3].next, 0);

        // A single-element ring unlinks back to itself.
        let mut single = [
            W11A02NodeLinks { next: 1, prev: 1 },
            W11A02NodeLinks { next: 0, prev: 0 },
        ];
        w11a02_unlink_node(&mut single, 1);
        assert_eq!(single[0], W11A02NodeLinks { next: 0, prev: 0 });
    }

    /// `0x0064b790`: the payload tag decode and the release bookkeeping.
    #[test]
    fn payload_release_matches_0x0064b790() {
        assert_eq!(w11a02_payload_word(1), W11A02Payload::Empty);
        assert_eq!(w11a02_payload_word(0x00fc_9b90), W11A02Payload::Inline);
        assert_eq!(
            w11a02_payload_word(0x00fc_9b91),
            W11A02Payload::Heap(0x00fc_9b90)
        );
        assert_eq!(
            w11a02_payload_word(0x0000_0003),
            W11A02Payload::Heap(0x0000_0002)
        );

        let mut inline = 0x00fc_9b90;
        assert_eq!(
            w11a02_release_payload(&mut inline),
            W11A02Release::InlineDestroyed
        );
        assert_eq!(inline, W11A02_PAYLOAD_EMPTY);

        let mut heap = 0x00fc_9b91;
        assert_eq!(
            w11a02_release_payload(&mut heap),
            W11A02Release::HeapDestroyed(0x00fc_9b90)
        );
        assert_eq!(heap, W11A02_PAYLOAD_EMPTY);
        assert_eq!(w11a02_payload_word(heap), W11A02Payload::Empty);

        let mut empty = W11A02_PAYLOAD_EMPTY;
        assert_eq!(
            w11a02_release_payload(&mut empty),
            W11A02Release::AlreadyEmpty
        );
        assert_eq!(empty, W11A02_PAYLOAD_EMPTY);
    }

    /// `0x0064b7e7`: the merged byte is the returned value and only zero is
    /// accepted.
    #[test]
    fn state_fold_matches_0x0064b7e7() {
        assert_eq!(w11a02_fold_state_bits(0, 0), Ok(0));
        assert_eq!(w11a02_fold_state_bits(0x01, 0x02), Err(0x03));
        assert_eq!(w11a02_fold_state_bits(0x80, 0x80), Err(0x80));
        assert_eq!(w11a02_fold_state_bits(0xff, 0x00), Err(0xff));
    }

    /// `0x0064b840`: the three write outcomes and the `+0x60` byte.
    #[test]
    fn setting_write_matches_0x0064b840() {
        let notified = Cell::new(0);

        // The optional writer handled the request.
        let mut target = W11A02WriteTarget {
            value: 0x11,
            applied: false,
        };
        let outcome = w11a02_apply_setting_write(&mut target, false, Some(true), false, || {
            notified.set(notified.get() + 1)
        });
        assert_eq!(outcome, W11A02WriteOutcome::OverrideHandled);
        assert!(target.applied);
        assert_eq!(notified.get(), 0);
        assert_eq!(target.value, 0x11);

        // The optional writer declined; the base writer accepted, so `+0x60`
        // keeps its previous value.
        let mut target = W11A02WriteTarget {
            value: 0x22,
            applied: false,
        };
        let outcome = w11a02_apply_setting_write(&mut target, false, Some(false), true, || {
            notified.set(notified.get() + 1)
        });
        assert_eq!(outcome, W11A02WriteOutcome::Written);
        assert!(!target.applied);
        assert_eq!(notified.get(), 0);

        // Rejected with `param_2 == 0`: notify and clear the applied byte.
        let outcome = w11a02_apply_setting_write(&mut target, false, None, false, || {
            notified.set(notified.get() + 1)
        });
        assert_eq!(outcome, W11A02WriteOutcome::Rejected { applied: false });
        assert!(!target.applied);
        assert_eq!(notified.get(), 1);

        // Rejected with `param_2 != 0`: keep the applied byte, no notification.
        let outcome = w11a02_apply_setting_write(&mut target, true, None, false, || {
            notified.set(notified.get() + 1)
        });
        assert_eq!(outcome, W11A02WriteOutcome::Rejected { applied: true });
        assert!(target.applied);
        assert_eq!(notified.get(), 1);
        assert_eq!(W11A02_SETTINGS_APPLIED_OFFSET, 0x60);
    }

    /// `0x0064b9d0`: a byte setting refreshes only when it changes.
    #[test]
    fn byte_setting_change_matches_0x0064b9d0() {
        assert!(!w11a02_byte_setting_refresh_needed(1, 1));
        assert!(!w11a02_byte_setting_refresh_needed(0, 0));
        assert!(w11a02_byte_setting_refresh_needed(0, 1));
        assert!(w11a02_byte_setting_refresh_needed(1, 0));
        // The call site passes `(short *)(param_1 + 0xe) == 10`, i.e. 0/1.
        assert!(w11a02_byte_setting_refresh_needed(0, u8::from(10u16 == 10)));
    }

    /// `0x0064bd50`: the singleton is loaded exactly once.
    #[test]
    fn skin_settings_loads_once_matches_0x0064bd50() {
        let cell = OnceLock::new();
        let loads = AtomicUsize::new(0);
        let load = || {
            loads.fetch_add(1, Ordering::SeqCst);
            W11A02SkinSettings {
                skins_enabled: true,
            }
        };
        let first = w11a02_skin_settings(&cell, load);
        let second = w11a02_skin_settings(&cell, load);
        assert_eq!(loads.load(Ordering::SeqCst), 1);
        assert!(std::ptr::eq(first, second));
        assert!(first.skins_enabled);
        assert_eq!(W11A02_SKINS_ENABLED_KEY, "skinsEnabled");
        assert_eq!(W11A02_SKINS_ENABLED_KEY.len(), 12);
    }

    /// `0x0064bfb0`: the empty inline string keeps size 0 and capacity
    /// `0x8000000f`.
    #[test]
    fn empty_small_string_matches_0x0064bfb0() {
        let string = w11a02_empty_small_string();
        assert_eq!(string.len, 0);
        assert_eq!(string.capacity, 0x8000_000f);
        assert_eq!(string.capacity, W11A02_SSO_CAPACITY_WORD);
        assert_eq!(string.capacity & 0x7fff_ffff, W11A02_SSO_CAPACITY);
        assert_eq!(string.buffer, [0u8; 15]);
        assert_eq!(string.buffer.len(), W11A02_SSO_BUFFER_BYTES);
        assert_eq!(W11A02_SSO_CAPACITY_WORD, W11A02_SSO_CAPACITY_FLAG | 0x0f);
    }
}
