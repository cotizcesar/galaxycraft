#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent64.rs
// ===========================================================================
// Translation batch 64.  Only this file is edited by the batch.
//
// Batch 64 owns `0x00612000..=0x00613fff`.  The ledger lists 56
// `decompiled_uncited` rows in that window; the rows reproduced below cover
// 28 of them.  The rest are mid-instruction fragments, RTTI descriptor
// returns, exception-cleanup tails, or bodies whose entire observable work is
// an indirect call into a routine outside the window.  The skip reasons are
// collected at the end of the file.
//
// The window is the icon/asset cache of the shipped client:
//
// * A chained hash table keyed by a 32-bit address.  The bucket is
//   `murmur3_fmix32(key) & (capacity - 1)`: `0x006120a5` and `0x006121e0`
//   both spell the fmix32 finaliser out inline (`* 0x85ebca6b`,
//   `* 0xc2b2ae35`).  [`a64_murmur_mix`] is the exact finaliser.
// * Insertion returns a four-dword descriptor: the key object, the slot
//   (a bucket address for the hashed path, a caller value for the append
//   path), the one-past-the-end element, and a flag byte (`0` appended,
//   `1` hashed).  See [`a64_append_plan`] / [`a64_hashed_plan`].
// * The MSVC-style string header used by the cache lambdas is
//   `{ data, len, capacity }` with capacity `0x8000000f` when empty and the
//   top bit marking an owned buffer.  [`a64_empty_string_header`] is the
//   header written by `0x006122f0`, `0x00612550` and `0x006133d0`.
// * `0x00613fe0` is the map-tile decoder, reproduced byte for byte in
//   [`a64_decode_tile`] / [`a64_decode_tiles`].

// ---------------------------------------------------------------------------
// Shared constants
// ---------------------------------------------------------------------------

/// Capacity word written by every empty string header in this window
/// (`0x8000000f` at `0x006122f0`, `0x00612550`, `0x006133d0`).
pub const A64_EMPTY_CAP: u32 = 0x8000_000f;

/// Mask that keeps the low 31 bits of an owned-buffer capacity word.
pub const A64_CAP_MASK: u32 = 0x7fff_ffff;

/// Low bit of a shared-string header: set when the data word is an indirect
/// reference to the payload rather than the payload itself (`0x00612110`,
/// `0x0061382e`, `0x006138fe`, `0x00613e57`).
pub const A64_SHARED_INDIRECT: u32 = 1;

/// Value stored back into a released shared member (`*(...+0x24) = 1` in
/// `0x0061382e`, `0x006138fe` and `0x00613e57`).
pub const A64_SHARED_RELEASED: u32 = 1;

// ---------------------------------------------------------------------------
// 0x006122f0 / 0x00612550 / 0x006133d0: empty string header
// ---------------------------------------------------------------------------

/// The three fields the empty-string initialiser stores.
///
/// `0x006122f0` writes `param_1[1] = 0`, `*param_1 = param_1 + 3`,
/// `param_1[2] = 0x8000000f`, `*(u8 *)(param_1 + 3) = 0` on the mode-2 path and
/// the identical `{ self + 3, 0, 0x8000000f, 0 }` quadruple on the other
/// branch; `0x00612550` and `0x006133d0` store the same four words into the
/// embedded member.  The self-pointer is not modelled, only the three scalar
/// fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A64StringHeader {
    /// Stored byte count (`param_1[1]` in `0x006122f0`, `param_1[3]` in
    /// `0x00612550`).
    pub len: u32,
    /// Capacity word carrying the owned-buffer top bit (`0x8000000f`).
    pub cap: u32,
    /// First inline byte, cleared by the initialiser.
    pub first: u8,
}

/// The header written by every empty-string initialiser in this batch.
pub const fn a64_empty_string_header() -> A64StringHeader {
    A64StringHeader {
        len: 0,
        cap: A64_EMPTY_CAP,
        first: 0,
    }
}

/// `0x006122f0`: the two branches differ only in which source word they pass
/// on to the shipped duplicate helper; both initialise the destination with
/// the empty header first.  The helper itself is outside the window, so only
/// the header stores are reproduced.
pub fn a64_key_string_header() -> A64StringHeader {
    a64_empty_string_header()
}

/// `0x00612550` and the identical body at `0x006133d0`.
///
/// The copy writes the implementation vtable, carries the source's `+0x04`
/// word, rewrites the embedded string header to empty, then hands the
/// source's `+0x0c` word to the duplicate helper.  [`A64StringFunctor`] holds
/// the recovered words; the pending source word records the value the copy
/// helper consumes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A64StringFunctor {
    /// `+0x04`: copied from the source object.
    pub word1: u32,
    /// `+0x08`: the embedded header, rewired to the empty form.
    pub header: A64StringHeader,
    /// `+0x0c` of the source, passed to the duplicate helper.
    pub pending_source: u32,
}

/// `0x00612550` / `0x006133d0`: the observed stores of the string functor
/// copy constructor.
pub fn a64_string_functor_copy(word1: u32, pending_source: u32) -> A64StringFunctor {
    A64StringFunctor {
        word1,
        header: a64_empty_string_header(),
        pending_source,
    }
}

// ---------------------------------------------------------------------------
// 0x006120a5 / 0x006121e0: MurmurHash3 finaliser
// ---------------------------------------------------------------------------

/// The 32-bit MurmurHash3 finaliser recovered from `0x006120a5`:
///
/// ```text
/// h = (h >> 16 ^ h) * 0x85ebca6b
/// h = (h >> 13 ^ h) * 0xc2b2ae35
/// h =  h >> 16 ^ h
/// ```
///
/// The decompiler's `* -0x7a143595` and `* -0x3d4d51cb` are the two
/// constants above; `0x006121e0` uses the identical sequence.
pub const fn a64_murmur_mix(key: u32) -> u32 {
    let mut h = key;
    h = (h >> 16 ^ h).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    h >> 16 ^ h
}

/// `0x006120a5` / `0x006121e0`: bucket index into a table of `capacity`
/// four-byte pointers.
///
/// The decompiler spells the mask as `(count - 1U & hash)`; `capacity == 0`
/// wraps to `0xffffffff`, which keeps the whole hash.
pub fn a64_bucket_index(key: u32, capacity: u32) -> u32 {
    a64_murmur_mix(key) & capacity.wrapping_sub(1)
}

// ---------------------------------------------------------------------------
// 0x00612029 / 0x006120a5: insertion descriptors
// ---------------------------------------------------------------------------

/// The four-dword descriptor both insertion paths write.
///
/// Layout recovered from `0x00612029` (`*out = {key, slot}`, `out[2] = tail`,
/// `*(u8 *)(out + 3) = flag`):
///
/// * `+0x00`: key object address.
/// * `+0x04`: slot address (bucket cell, or the caller-supplied value on the
///   append path).
/// * `+0x08`: `base + count * 4`, one past the last element.
/// * `+0x0c`: `0` for the append path, `1` for the hashed path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A64InsertPlan {
    /// `+0x00`: first stored dword (the key object).
    pub key: u32,
    /// `+0x04`: second stored dword (bucket cell or caller value).
    pub slot: u32,
    /// `+0x08`: one-past-the-end element.
    pub tail: u32,
    /// `+0x0c`: path flag.
    pub flag: u8,
}

/// `0x00612029`: append path.
///
/// The body stores the caller's `+0x0c` argument as the slot, computes
/// `base + count * 4` as the tail, and writes a zero flag byte.  `key` is the
/// value loaded from the key register before the stores.
pub fn a64_append_plan(key: u32, slot: u32, base: u32, count: u32) -> A64InsertPlan {
    A64InsertPlan {
        key,
        slot,
        tail: base.wrapping_add(count.wrapping_mul(4)),
        flag: 0,
    }
}

/// `0x006120a5`: hashed path.
///
/// The dword at `+0x04` of the key object is run through
/// [`a64_murmur_mix`], masked with `count - 1`, scaled by four and added to
/// the element base to form the bucket slot.  The tail and the flag byte
/// (`1`) are shared with the append path.
pub fn a64_hashed_plan(key: u32, hash_word: u32, base: u32, count: u32) -> A64InsertPlan {
    A64InsertPlan {
        key,
        slot: base.wrapping_add(a64_bucket_index(hash_word, count).wrapping_mul(4)),
        tail: base.wrapping_add(count.wrapping_mul(4)),
        flag: 1,
    }
}

// ---------------------------------------------------------------------------
// 0x00612110 / 0x0061382e / 0x006138fe / 0x00613e57: shared member
// ---------------------------------------------------------------------------

/// Resolve a shared-string flags word to the payload address.
///
/// Every shared member in this window uses the same convention
/// (`0x00612110`, `0x0061382e`, `0x006138fe`, `0x00613e57`): bit 0 set means
/// the stored word is an indirect reference and the payload is
/// `word & 0xfffffffe`, otherwise the word itself is the payload.
pub const fn a64_shared_target(flags: u32) -> u32 {
    if flags & A64_SHARED_INDIRECT != 0 {
        flags & !A64_SHARED_INDIRECT
    } else {
        flags
    }
}

/// `0x00612110`: the validation gate applied before the shared payload is
/// adopted.
///
/// A zero payload is accepted without inspection.  Otherwise the payload's
/// `present` byte must be non-zero, its length word must be at most `0x14`
/// and its kind word at most `4` (`(local_18 == 0) || (0x14 < local_20) ||
/// (4 < local_1c)` in the decompiled body).
pub const fn a64_shared_valid(flags: u32, present: bool, len: u32, kind: u32) -> bool {
    if a64_shared_target(flags) == 0 {
        return true;
    }
    present && len <= 0x14 && kind <= 4
}

/// Result of the shared release prologue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A64SharedRelease {
    /// `a64_shared_target(flags)`: address passed to the release call.
    pub target: u32,
    /// Value stored back into the member's flags word (`1`).
    pub reset: u32,
}

/// `0x0061382e`, `0x006138fe`, `0x00613e57`: the identical release prologue.
///
/// Each body resolves the member with the `& 0xfffffffe` rule, calls the
/// release slot, stores `1` into the flags word and then tail-calls the
/// teardown at `0x00613a30`.  Only the resolution and the stored value are
/// reproduced; the virtual call and the teardown are outside this translation.
pub const fn a64_shared_release(flags: u32) -> A64SharedRelease {
    A64SharedRelease {
        target: a64_shared_target(flags),
        reset: A64_SHARED_RELEASED,
    }
}

// ---------------------------------------------------------------------------
// 0x006121e0: chained hash lookup
// ---------------------------------------------------------------------------

/// One bucket-chain node.
///
/// The decompiler walks `*piVar1` for the successor and compares `piVar2[1]`
/// with the sought key, so the node layout is `{ next, key }` with 32-bit
/// pointers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A64HashNode {
    /// `+0x00`: successor index, `None` for the null terminator.
    pub next: Option<usize>,
    /// `+0x04`: key compared by the walk.
    pub key: u32,
}

/// Result of `0x006121e0`'s chain walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A64ChainLocate {
    /// Node whose `next` field holds the cursor; `None` when the cursor is the
    /// bucket head (or the chain is empty).
    pub predecessor: Option<usize>,
    /// Cursor at loop exit; `Some` exactly when the key was found.
    pub found: Option<usize>,
}

impl A64ChainLocate {
    /// `*piVar1 != 0` in the shipped body, i.e. the duplicate-key rejection.
    pub const fn duplicate(&self) -> bool {
        self.found.is_some()
    }
}

/// `0x006121e0`: locate `key` in one bucket chain.
///
/// The walk is
/// `for (n = *bucket; n != 0 && key != n[1]; n = *n) bucket = n;`
/// so the loop leaves the predecessor in `predecessor` and the matched (or
/// null) node in `found`.  The shipped routine only performs the walk when
/// the table pointer `0x00f04c68` is non-null; callers model the absent table
/// by not calling at all.
pub fn a64_chain_locate(
    nodes: &[A64HashNode],
    bucket_first: Option<usize>,
    key: u32,
) -> A64ChainLocate {
    let mut predecessor = None;
    let mut cursor = bucket_first;
    while let Some(index) = cursor {
        if nodes[index].key == key {
            break;
        }
        predecessor = cursor;
        cursor = nodes[index].next;
    }
    A64ChainLocate {
        predecessor,
        found: cursor,
    }
}

// ---------------------------------------------------------------------------
// 0x00612480 / 0x006124f0 / 0x006132e0: guards and identity wrappers
// ---------------------------------------------------------------------------

/// `0x00612480`: the release guard.
///
/// The body throws when the signed word at `+0x10` is non-negative
/// (`if (-1 < *(int *)(in_ECX + 0x10))`) and only then tests the caller's
/// free flag.  Accepted reference counts are therefore strictly negative.
pub const fn a64_release_allowed(refcount: i32) -> bool {
    refcount < 0
}

/// `0x006124f0`: forward the first dword of the caller's record to
/// `0x006122f0` and return the output pointer unchanged.
pub const fn a64_key_string_for(out: u32, mode: u32) -> u32 {
    let _ = mode;
    out
}

/// `0x006132e0`: forward the first dword of the caller's record to
/// `FUN_00612f20` and return the output pointer unchanged.  `FUN_00612f20` is a
/// noreturn lookup/duplicate wrapper outside this translation, so only the
/// identity return is reproduced.
pub const fn a64_dup_string_for(out: u32, key: u32) -> u32 {
    let _ = key;
    out
}

// ---------------------------------------------------------------------------
// 0x00612510 / 0x00613300: three-row grids
// ---------------------------------------------------------------------------

/// Row count of every grid walked in this window (`iVar2 = 3` in
/// `0x00612510` and `0x00613300`).
pub const A64_GRID_ROWS: u32 = 3;

/// Byte stride of one grid element (`in_ECX + 0x30` end pointer with a
/// `+ 0x10` step in `0x00612510`).
pub const A64_GRID_STRIDE: u32 = 0x10;

/// `0x00612510`: the destructor pass.
///
/// The outer loop runs three times; the inner loop walks `base .. base + 0x30`
/// in `0x10` steps, i.e. three elements per row, calling the shipped element
/// destructor each time.  Returns the element offsets in call order.
pub fn a64_grid_destroy_offsets() -> Vec<u32> {
    let mut offsets = Vec::with_capacity(a64_grid_destroy_calls() as usize);
    for _ in 0..A64_GRID_ROWS {
        let mut offset = 0;
        while offset != 3 * A64_GRID_STRIDE {
            offsets.push(offset);
            offset += A64_GRID_STRIDE;
        }
    }
    offsets
}

/// `0x00612510`: number of element destructor calls (`3 rows * 3 elements`).
pub const fn a64_grid_destroy_calls() -> u32 {
    9
}

/// `0x00613300`: the reset pass.
///
/// Each of the three rows first releases and clears `*row`, then walks the
/// three dword pairs at `row + 1`, `+3`, `+5` (`piVar1 != in_ECX + 7` step
/// `piVar1 + 2`), calling the shipped pair destructor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A64ResetPlan {
    /// Outer iterations.
    pub rounds: u32,
    /// `*row != 0` releases per round.
    pub releases: u32,
    /// Pair destructors per round.
    pub per_round: u32,
}

/// `0x00613300`: the recovered loop bounds.
pub const fn a64_grid_reset_plan() -> A64ResetPlan {
    A64ResetPlan {
        rounds: A64_GRID_ROWS,
        releases: A64_GRID_ROWS,
        per_round: 3,
    }
}

// ---------------------------------------------------------------------------
// 0x00612830 / 0x00612eb0: element addressing
// ---------------------------------------------------------------------------

/// `0x00612830`: `param_1 * 0x10 + in_ECX`, one grid row address.
pub const fn a64_slot16(base: u32, index: u32) -> u32 {
    base.wrapping_add(index.wrapping_mul(0x10))
}

/// `0x00612eb0`: `param_1 * 8 + 4 + in_ECX`, the `+4` offset element address
/// used by the eight-byte record walk.
pub const fn a64_slot8(base: u32, index: u32) -> u32 {
    base.wrapping_add(4).wrapping_add(index.wrapping_mul(8))
}

// ---------------------------------------------------------------------------
// 0x00612d20 / 0x00613030: icon records and the archive path
// ---------------------------------------------------------------------------

/// `0x00612d20`: the three-word record written by the icon table initialiser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A64IconHeader {
    /// `+0x00`: `0x24`, the record stride.
    pub stride: u32,
    /// `+0x04`: `4`, the record count.
    pub count: u32,
    /// `+0x08`: cleared byte.
    pub flag: u8,
}

/// `0x00612d20`: `*param_1 = 0x24; param_1[1] = 4; *(u8 *)(param_1 + 2) = 0`.
pub const fn a64_icon_header() -> A64IconHeader {
    A64IconHeader {
        stride: 0x24,
        count: 4,
        flag: 0,
    }
}

/// `0x00613030`: build the icon archive name.
///
/// The shipped body is `FUN_00c6c360(param_1, "%sgame\\icons.grp", *puVar1)`
/// where `*puVar1` is the first word of the caller's record, i.e. the string
/// header's data pointer.  The result is the prefix followed by the literal
/// `game\icons.grp`.
pub fn a64_icon_archive_path(prefix: &str) -> String {
    format!("{prefix}game\\icons.grp")
}

// ---------------------------------------------------------------------------
// 0x006133a0 / 0x006133b0 / 0x00613440 / 0x00613460: cache functors
// ---------------------------------------------------------------------------

/// The lambda identities named by the functor stores in this window.  The
/// matching RTTI descriptor stubs are `FUN_00613760` (`L2b45`), `FUN_00613770`
/// (`L5f0b`), `FUN_00613790` (`L92e3`) and `FUN_006137a0` (`Ld3e5`);
/// `FUN_00613780` (`lambda_669384bded6b821f72b5754f116318fd`) has no store
/// inside the batch and is not represented.  `L19e2` is the immediate-byte
/// functor stored by `0x00613950`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A64Lambda {
    /// `<lambda_2b45b921844d4445efa9d3fd3765101a>` (`FUN_00613760`).
    L2b45,
    /// `<lambda_5f0b93efd4db5d6b5c2f41e3a838c96b>` (`FUN_00613770`).
    L5f0b,
    /// `<lambda_92e3c88ef4a608ef5dd3d9dee5e876dd>` (`FUN_00613790`).
    L92e3,
    /// `<lambda_d3e57fb348665356f6926ac61acd4194>` (`FUN_006137a0`).
    Ld3e5,
    /// `<lambda_19e2ac969616734601bfbc4079eb0b93>` used by `0x00613950`.
    L19e2,
}

impl A64Lambda {
    /// The mangled lambda identifier as it appears in the RTTI names.
    pub const fn name(self) -> &'static str {
        match self {
            A64Lambda::L2b45 => "lambda_2b45b921844d4445efa9d3fd3765101a",
            A64Lambda::L5f0b => "lambda_5f0b93efd4db5d6b5c2f41e3a838c96b",
            A64Lambda::L92e3 => "lambda_92e3c88ef4a608ef5dd3d9dee5e876dd",
            A64Lambda::Ld3e5 => "lambda_d3e57fb348665356f6926ac61acd4194",
            A64Lambda::L19e2 => "lambda_19e2ac969616734601bfbc4079eb0b93",
        }
    }
}

/// A callable-object copy with a single byte payload.
///
/// `0x006133b0` and `0x00613440` store the implementation vtable, copy the
/// source byte from `+0x04`, then overwrite the vtable with the object
/// implementation for their lambda.  Only the final vtable identity and the
/// byte survive, which is what this type keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A64BoolFunctor {
    /// Final `bcCopyableFunctionObjectImpl` lambda.
    pub lambda: A64Lambda,
    /// `+0x04`: the payload byte.
    pub value: u8,
}

/// `0x006133a0`: `*param_1 = vftable` for the `lambda_2b45` object
/// implementation, nothing else.
pub const fn a64_bare_functor(lambda: A64Lambda) -> A64BoolFunctor {
    A64BoolFunctor { lambda, value: 0 }
}

/// `0x006133b0` / `0x00613440`: the copy constructor's two stores.
pub const fn a64_bool_functor_copy(lambda: A64Lambda, src: u8) -> A64BoolFunctor {
    a64_bool_functor_from(lambda, src)
}

/// `0x006137b0`, `0x00613880`, `0x00613950`: the immediate-byte constructors.
///
/// Each body moves the caller's byte into the local object's `+0x04`, writes
/// the object-implementation vtable and then jumps to the shipped invoke
/// helper `FUN_005426e0`; only the two stores that precede the `noreturn`
/// call are reproduced.  The lambdas are `L92e3`, `L5f0b` and `L19e2`
/// respectively.
pub const fn a64_bool_functor_from(lambda: A64Lambda, value: u8) -> A64BoolFunctor {
    A64BoolFunctor { lambda, value }
}

/// `0x00613460`: `*param_1 = vftable` for `lambda_d3e5`.
pub const fn a64_functor_d3e5() -> A64BoolFunctor {
    A64BoolFunctor {
        lambda: A64Lambda::Ld3e5,
        value: 0,
    }
}

// ---------------------------------------------------------------------------
// 0x00613a30: relocated offsets
// ---------------------------------------------------------------------------

/// `0x00613a30`: relocate the stored offsets of a shared block.
///
/// The recovered loop runs backwards from the count stored in the low 15 bits
/// of the block header (`*(u16 *)*in_ECX & 0x7fff`) and, for every entry,
///
/// ```text
/// w = w & 0x7fffffff
/// w = w + base
/// ```
///
/// The decompiler addresses the same dword twice (`puVar1` and `piVar8` both
/// resolve to `base + 10 + i * 8`), so the mask and the add apply to one word.
/// `words` is the caller-provided view of those entries in index order.
pub fn a64_relocate_offsets(words: &mut [u32], base: u32) {
    for word in words.iter_mut().rev() {
        *word &= A64_CAP_MASK;
        *word = word.wrapping_add(base);
    }
}

// ---------------------------------------------------------------------------
// 0x00613fe0: map-tile decoder
// ---------------------------------------------------------------------------

/// Source tile bits: the terrain/type index is `(word >> 4) & 0x7ff`.
pub const A64_TILE_KIND_SHIFT: u32 = 4;
/// Mask applied after the shift (`>> 4 & 0x7ff`).
pub const A64_TILE_KIND_MASK: u32 = 0x7ff;
/// Source tile bits: the low nibble indexes the 16 sub-tiles.
pub const A64_TILE_LOW_MASK: u32 = 0xf;
/// Stride between the two 26-word blocks of the lookup table (`uVar2 * 0x1a`).
pub const A64_TILE_BLOCK_STRIDE: u32 = 0x1a;
/// Word offset of the 16-entry sub-tile sub-table (`+0x14` bytes).
pub const A64_TILE_TABLE_PREFIX: u32 = 0x14 / 2;
/// Replacement source word when the type index is out of range (`0x20`, the
/// space glyph).
pub const A64_TILE_FALLBACK_WORD: u16 = 0x20;
/// Replacement type index when the source type is out of range (`uVar2 = 2`).
pub const A64_TILE_FALLBACK_KIND: u32 = 2;
/// Mask applied to the per-kind flag word before it is moved to the high half
/// (`& 0xffffd8fa`).
pub const A64_TILE_FLAG_MASK: u16 = 0xd8fa;
/// Source-tile high bit copied to the destination tile.
pub const A64_TILE_HIGH_BIT: u16 = 0x8000;
/// Extra flag bit set when the source tile's high bit was set (`0x400000`).
pub const A64_TILE_FORCED_FLAG: u32 = 0x0040_0000;
/// The destination id is the low 15 bits of the decoded tile (`& 0x7fff`).
pub const A64_TILE_ID_MASK: u16 = 0x7fff;

/// One decoded map cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A64DecodedTile {
    /// Decoded destination word (lookup-table entry, high bit preserved).
    pub tile: u16,
    /// Destination flag dword.
    pub flags: u32,
    /// True when the source type index was out of range and replaced.
    pub out_of_range: bool,
}

/// `0x00613fe0`: decode one source tile word.
///
/// The recovered body, for every cell:
///
/// ```text
/// w     = *src
/// kind  = (*src >> 4) & 0x7ff
/// if (id_limit <= kind) { w = 0x20; kind = 2; low = 0 }
/// dst   = table[10 + (w & 0xf) + kind * 0x1a]
/// flags = (flags & 0xffff) | ((table[kind * 0x1a + 1] & 0xd8fa) << 16)
///                    | ids[dst & 0x7fff]
/// if ((i16)w < 0) { dst |= 0x8000; flags |= 0x400000 }
/// ```
///
/// Note that the out-of-range path rewrites the source word before the sign
/// test, so a clamped cell can never set the high bit.  The caller's `flags`
/// word is the value already present in the destination array; the body clears
/// its high half before the OR.
///
/// Returns `None` when `table` or `ids` is too short for the indices the body
/// would read; the shipped code reads them unconditionally.
pub fn a64_decode_tile(
    word: u16,
    id_limit: u32,
    table: &[u16],
    ids: &[u32],
    flags: u32,
) -> Option<A64DecodedTile> {
    let mut source = word;
    let mut kind = (u32::from(word) >> A64_TILE_KIND_SHIFT) & A64_TILE_KIND_MASK;
    let mut low = u32::from(word) & A64_TILE_LOW_MASK;
    let mut out_of_range = false;
    if id_limit <= kind {
        out_of_range = true;
        source = A64_TILE_FALLBACK_WORD;
        kind = A64_TILE_FALLBACK_KIND;
        low = 0;
    }

    let table_index = (A64_TILE_TABLE_PREFIX + low + kind * A64_TILE_BLOCK_STRIDE) as usize;
    let flag_index = (kind * A64_TILE_BLOCK_STRIDE + 1) as usize;
    let mut tile = *table.get(table_index)?;
    let mut out_flags = (flags & 0x0000_ffff)
        | (u32::from(table.get(flag_index).copied()? & A64_TILE_FLAG_MASK) << 16);
    out_flags |= *ids.get(usize::from(tile & A64_TILE_ID_MASK))?;

    if (source as i16) < 0 {
        tile |= A64_TILE_HIGH_BIT;
        out_flags |= A64_TILE_FORCED_FLAG;
    }

    Some(A64DecodedTile {
        tile,
        flags: out_flags,
        out_of_range,
    })
}

/// `0x00613fe0`: the row/column pair handed to the out-of-range callback.
///
/// The shipped code computes `index % width` and `index / width` from
/// `(cell_count - remaining) - 1`, i.e. the zero-based cell index.
pub fn a64_cell_xy(index: u32, width: u16) -> (u32, u32) {
    let width = u32::from(width);
    assert!(width != 0, "the shipped loop divides by param_7");
    (index % width, index / width)
}

/// `0x00613fe0`: run the decoder over `width * height` cells.
///
/// The cell count is `param_8 * param_7` (`iVar1 = param_8 * uVar3`); the loop
/// consumes exactly that many source words and produces that many destination
/// words and flag dwords.
pub fn a64_decode_tiles(
    words: &[u16],
    id_limit: u32,
    table: &[u16],
    ids: &[u32],
    width: u16,
    height: u16,
) -> Vec<A64DecodedTile> {
    let cells = usize::from(width) * usize::from(height);
    let mut out = Vec::with_capacity(cells);
    for index in 0..cells {
        let decoded = a64_decode_tile(words[index], id_limit, table, ids, 0)
            .expect("table and id array cover every decoded index");
        out.push(decoded);
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent64 {
    use super::*;

    /// `0x006120a5` / `0x006121e0`: the exact fmix32 constants and the bucket
    /// mask, including the zero-capacity wrap.
    #[test]
    fn murmur_mix_and_bucket_index() {
        assert_eq!(a64_murmur_mix(0), 0);
        assert_eq!(a64_murmur_mix(1), 0x514e_28b7);
        assert_eq!(a64_murmur_mix(0x1234_5678), 0xe37c_d1bc);
        assert_eq!(a64_murmur_mix(0xdead_beef), 0x0de5_c6a9);
        assert_eq!(a64_murmur_mix(u32::MAX), 0x81f1_6f39);

        assert_eq!(a64_bucket_index(0x1234_5678, 8), 4);
        assert_eq!(a64_bucket_index(0xdead_beef, 4), 1);
        assert_eq!(a64_bucket_index(0xdead_beef, 1), 0);
        assert_eq!(
            a64_bucket_index(0x1234_5678, 0),
            a64_murmur_mix(0x1234_5678)
        );
    }

    /// `0x00612029` / `0x006120a5`: both descriptors, their flags and the
    /// one-past-the-end tail.
    #[test]
    fn insert_plans_match_the_two_store_paths() {
        assert_eq!(
            a64_append_plan(0xaaaa, 0xbbbb, 0x1000, 3),
            A64InsertPlan {
                key: 0xaaaa,
                slot: 0xbbbb,
                tail: 0x100c,
                flag: 0,
            }
        );

        let hashed = a64_hashed_plan(0x2000, 0xdead_beef, 0x3000, 4);
        assert_eq!(hashed.key, 0x2000);
        assert_eq!(hashed.slot, 0x3004);
        assert_eq!(hashed.tail, 0x3010);
        assert_eq!(hashed.flag, 1);

        // Wrapping pointer arithmetic, as the shipped `lea` does.
        assert_eq!(a64_append_plan(0, 0, 0xffff_fffc, 1).tail, 0);
    }

    /// `0x006122f0`, `0x00612550`, `0x006133d0`: the empty header and the
    /// functor copy stores.
    #[test]
    fn empty_string_header_and_functor_copy() {
        assert_eq!(
            a64_empty_string_header(),
            A64StringHeader {
                len: 0,
                cap: 0x8000_000f,
                first: 0,
            }
        );
        assert_eq!(a64_key_string_header(), a64_empty_string_header());

        let copied = a64_string_functor_copy(0x1234, 0x5678);
        assert_eq!(copied.word1, 0x1234);
        assert_eq!(copied.pending_source, 0x5678);
        assert_eq!(copied.header, a64_empty_string_header());
        assert_eq!(copied.header.cap & A64_CAP_MASK, 0x0f);
    }

    /// `0x00612110`: the indirect resolution and the `0x14` / `4` bounds.
    #[test]
    fn shared_string_gate() {
        assert_eq!(a64_shared_target(0), 0);
        assert_eq!(a64_shared_target(1), 0);
        assert_eq!(a64_shared_target(0x1001), 0x1000);
        assert_eq!(a64_shared_target(0x1000), 0x1000);

        // A null payload skips the inspection entirely.
        assert!(a64_shared_valid(0, false, u32::MAX, u32::MAX));
        assert!(a64_shared_valid(1, false, u32::MAX, u32::MAX));

        assert!(a64_shared_valid(0x1001, true, 0x14, 4));
        assert!(!a64_shared_valid(0x1001, true, 0x15, 4));
        assert!(!a64_shared_valid(0x1001, true, 0x14, 5));
        assert!(!a64_shared_valid(0x1001, false, 0, 0));
        // Flags without bit 0 resolve to themselves and are inspected.
        assert!(a64_shared_valid(0x1000, true, 1, 1));
        assert!(!a64_shared_valid(0x1000, true, 1, 5));
    }

    /// `0x0061382e`, `0x006138fe`, `0x00613e57`: same resolution, `1` stored
    /// back.
    #[test]
    fn shared_release_prologue() {
        assert_eq!(
            a64_shared_release(0x2001),
            A64SharedRelease {
                target: 0x2000,
                reset: 1,
            }
        );
        assert_eq!(
            a64_shared_release(0x2000),
            A64SharedRelease {
                target: 0x2000,
                reset: 1,
            }
        );
        assert_eq!(A64_SHARED_RELEASED, 1);
    }

    /// `0x006121e0`: head match, interior match, miss and empty bucket.
    #[test]
    fn chain_locate_reports_predecessor_and_duplicate() {
        let nodes = [
            A64HashNode {
                next: Some(1),
                key: 5,
            },
            A64HashNode { next: None, key: 9 },
        ];

        let head = a64_chain_locate(&nodes, Some(0), 5);
        assert_eq!(head.predecessor, None);
        assert_eq!(head.found, Some(0));
        assert!(head.duplicate());

        let tail = a64_chain_locate(&nodes, Some(0), 9);
        assert_eq!(tail.predecessor, Some(0));
        assert_eq!(tail.found, Some(1));
        assert!(tail.duplicate());

        let miss = a64_chain_locate(&nodes, Some(0), 7);
        assert_eq!(miss.predecessor, Some(1));
        assert_eq!(miss.found, None);
        assert!(!miss.duplicate());

        let empty = a64_chain_locate(&nodes, None, 7);
        assert_eq!(empty.predecessor, None);
        assert_eq!(empty.found, None);
        assert!(!empty.duplicate());
    }

    /// `0x00612480`, `0x006124f0`, `0x006132e0`: the guard and the two
    /// identity wrappers.
    #[test]
    fn guards_and_identity_wrappers() {
        assert!(a64_release_allowed(-1));
        assert!(a64_release_allowed(i32::MIN));
        assert!(!a64_release_allowed(0));
        assert!(!a64_release_allowed(1));

        assert_eq!(a64_key_string_for(0x1234, 2), 0x1234);
        assert_eq!(a64_key_string_for(0x1234, 0), 0x1234);
        assert_eq!(a64_dup_string_for(0x5678, 0x9abc), 0x5678);
    }

    /// `0x00612510` / `0x00613300`: three rows of three elements.
    #[test]
    fn grid_plans() {
        assert_eq!(
            a64_grid_destroy_offsets(),
            vec![0, 0x10, 0x20, 0, 0x10, 0x20, 0, 0x10, 0x20]
        );
        assert_eq!(
            a64_grid_destroy_offsets().len(),
            a64_grid_destroy_calls() as usize
        );
        assert_eq!(
            a64_grid_reset_plan(),
            A64ResetPlan {
                rounds: 3,
                releases: 3,
                per_round: 3,
            }
        );
    }

    /// `0x00612830` / `0x00612eb0`: the two element addresses.
    #[test]
    fn element_addressing() {
        assert_eq!(a64_slot16(0x1000, 0), 0x1000);
        assert_eq!(a64_slot16(0x1000, 3), 0x1030);
        assert_eq!(a64_slot16(0xffff_fff0, 2), 0x10);

        assert_eq!(a64_slot8(0x1000, 0), 0x1004);
        assert_eq!(a64_slot8(0x1000, 3), 0x101c);
        assert_eq!(a64_slot8(0xffff_fffc, 1), 0x8);
    }

    /// `0x00612d20` / `0x00613030`: the icon record header and archive path.
    #[test]
    fn icon_header_and_path() {
        assert_eq!(
            a64_icon_header(),
            A64IconHeader {
                stride: 0x24,
                count: 4,
                flag: 0,
            }
        );
        assert_eq!(a64_icon_header().stride * a64_icon_header().count, 0x90);

        assert_eq!(
            a64_icon_archive_path("C:\\starcraft\\"),
            "C:\\starcraft\\game\\icons.grp"
        );
        assert_eq!(a64_icon_archive_path(""), "game\\icons.grp");
    }

    /// `0x006133a0`, `0x006133b0`, `0x00613440`, `0x00613460`: vtable identity
    /// and the copied byte.
    #[test]
    fn cache_functors() {
        assert_eq!(
            a64_bare_functor(A64Lambda::L2b45),
            A64BoolFunctor {
                lambda: A64Lambda::L2b45,
                value: 0,
            }
        );
        assert_eq!(
            a64_bool_functor_copy(A64Lambda::L5f0b, 1),
            A64BoolFunctor {
                lambda: A64Lambda::L5f0b,
                value: 1,
            }
        );
        assert_eq!(a64_bool_functor_copy(A64Lambda::L92e3, 0x7f).value, 0x7f);
        assert_eq!(a64_functor_d3e5().lambda, A64Lambda::Ld3e5);
        assert_eq!(
            a64_bool_functor_from(A64Lambda::L19e2, 7),
            A64BoolFunctor {
                lambda: A64Lambda::L19e2,
                value: 7,
            }
        );
        assert_eq!(
            A64Lambda::L19e2.name(),
            "lambda_19e2ac969616734601bfbc4079eb0b93"
        );
        assert_eq!(
            A64Lambda::L2b45.name(),
            "lambda_2b45b921844d4445efa9d3fd3765101a"
        );
    }

    /// `0x00613a30`: mask the top bit, then add the base, newest entry first.
    #[test]
    fn relocate_offsets() {
        let mut words = [0x8000_0010, 0x0000_0020, 0x7fff_ffff];
        a64_relocate_offsets(&mut words, 0x1000);
        assert_eq!(words, [0x1010, 0x1020, 0x8000_0fff]);

        let mut empty: [u32; 0] = [];
        a64_relocate_offsets(&mut empty, 0x1000);
        assert_eq!(empty, [0u32; 0]);
    }

    /// A lookup table wide enough for two kinds: the second block starts at
    /// word 26, the sub-tile block at `+0x0a`.
    fn sample_table() -> Vec<u16> {
        let mut table = vec![0u16; 0x100];
        table[0 * 0x1a + 10 + 3] = 2;
        table[1 * 0x1a + 10 + 3] = 5;
        table[1 * 0x1a + 1] = 0x1234;
        table[2 * 0x1a + 10] = 7;
        table[2 * 0x1a + 1] = 0xffff;
        table
    }

    fn sample_ids() -> Vec<u32> {
        let mut ids = vec![0u32; 16];
        ids[2] = 0xabc;
        ids[5] = 0x1111_2222;
        ids[7] = 0x33;
        ids
    }

    /// `0x00613fe0`: in-range cell, sign bit forwarding and the flag layout.
    #[test]
    fn decode_tile_in_range() {
        let table = sample_table();
        let ids = sample_ids();

        let plain = a64_decode_tile(0x13, 8, &table, &ids, 0).unwrap();
        assert_eq!(
            plain,
            A64DecodedTile {
                tile: 5,
                flags: 0x1030_0000 | 0x1111_2222,
                out_of_range: false,
            }
        );

        let signed = a64_decode_tile(0x8013, 8, &table, &ids, 0).unwrap();
        assert_eq!(signed.tile, 5 | A64_TILE_HIGH_BIT);
        assert_eq!(signed.flags, 0x1070_0000 | 0x1111_2222);
        assert!(!signed.out_of_range);

        // The body keeps only the low half of the incoming flag dword.
        let kept = a64_decode_tile(0x13, 8, &table, &ids, 0xdead_beef).unwrap();
        assert_eq!(kept.flags, 0x1030_beef | 0x1111_2222);
    }

    /// `0x00613fe0`: out-of-range clamping rewrites the source word to `0x20`
    /// and the kind to `2`, so the high bit can never survive.
    #[test]
    fn decode_tile_clamps_out_of_range() {
        let table = sample_table();
        let ids = sample_ids();

        let clamped = a64_decode_tile(0x8013, 1, &table, &ids, 0).unwrap();
        assert!(clamped.out_of_range);
        assert_eq!(clamped.tile, 7);
        assert_eq!(
            clamped.flags,
            u32::from(0xffffu16 & A64_TILE_FLAG_MASK) << 16 | 0x33
        );

        // A zero limit clamps even a zero type index, which is the common case
        // for the shipped empty-table guard.
        let zero = a64_decode_tile(0x8000, 0, &table, &ids, 0).unwrap();
        assert!(zero.out_of_range);
        assert_eq!(zero.tile, 7);
        assert_eq!(zero.flags, clamped.flags);
    }

    /// `0x00613fe0`: the row/column pair and the full `width * height` walk.
    #[test]
    fn decode_tiles_covers_every_cell() {
        assert_eq!(a64_cell_xy(0, 4), (0, 0));
        assert_eq!(a64_cell_xy(9, 4), (1, 2));
        assert_eq!(a64_cell_xy(11, 3), (2, 3));

        let table = sample_table();
        let ids = sample_ids();
        let words = [0x13u16, 0x8013, 0x0003, 0x13];
        let out = a64_decode_tiles(&words, 8, &table, &ids, 2, 2);
        assert_eq!(out.len(), 4);
        assert_eq!(out[0].tile, 5);
        assert_eq!(out[1].tile, 5 | A64_TILE_HIGH_BIT);
        assert_eq!(out[2].tile, 2);
        assert_eq!(out[3].tile, 5);
        assert_eq!(out.iter().filter(|t| t.out_of_range).count(), 0);
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// * FUN_00612110 (tail), FUN_006121e0 (teardown after the lookup),
//   FUN_00612261, FUN_006122a2, FUN_00612480 (tail), FUN_00612cdb,
//   FUN_00613d9d: exception-cleanup tails.  The recovered bodies end in
//   `FUN_00c867a5` / `FUN_00c86b6e`, the shipped `noreturn` runtime helpers,
//   after releasing one vtable slot; only the prologue arithmetic is
//   recoverable and it is already covered by the shared-member helpers above.
// * FUN_00612029 / FUN_006120a5 / FUN_006121e0 first instruction: the indirect
//   call `(**(code **)(*in_EAX + 0x10))()` runs before the stores.  The
//   descriptor stores are reproduced; the callback target is outside the
//   window.
// * FUN_006122f0, FUN_00612550, FUN_006132e0, FUN_006133d0: the final
//   `FUN_00538810` / `FUN_005426e0` is a `noreturn` string-copy/invoke helper
//   from outside the batch, so only the header and functor stores are
//   reproduced.
// * FUN_00612f20, FUN_00612fa0, FUN_00613060, FUN_006130e0, FUN_00613260,
//   FUN_00613ea0: lookup-plus-duplicate wrappers whose only observable work is
//   a call to `FUN_00649fa0` followed by a `noreturn` helper; there is no
//   return value or table to reproduce.
// * FUN_006125c0, FUN_00612a30, FUN_00612aa9, FUN_00612c90, FUN_00613470:
//   the decompiler reports overlapping instructions, removed unreachable
//   blocks and bad control flow.  The recovered text is not a coherent body:
//   it reads and writes addresses such as `0x0061358a` (inside its own code)
//   and mixes `rdtsc`/`spectre` probes into the cache logic.
// * FUN_00612d40, FUN_00613de0, FUN_00613f20: the body stores one dword (or
//   only a vtable) and then jumps to a `noreturn` helper, so the function
//   never returns a value the caller could observe.
// * FUN_006136e0, FUN_006139a8, FUN_00613a6b, FUN_00613d55:
//   mid-instruction fragments, `int3` padding, or bodies that start after
//   `halt_baddata`.
// * FUN_00612880, FUN_00613760, FUN_00613770, FUN_00613780, FUN_00613790,
//   FUN_006137a0: six-byte `return &RTTI_Type_Descriptor` stubs.  They hold no
//   arithmetic, table or string behaviour; the lambda identities they name are
//   recorded in `A64Lambda`.
// * FUN_00613a30 / FUN_00613a6b: the teardown driver is a three-row release
//   plus a shared-block trim whose only self-contained arithmetic is the
//   relocation loop reproduced in `a64_relocate_offsets`; the rest is virtual
//   dispatch and exception state.
