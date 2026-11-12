#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent86.rs
// ===========================================================================
// Translation batch 86.  Only this file is edited by the batch.
//
// Batch 86 owns `0x0063e000..=0x0063ffff`.  The ledger lists 23
// `decompiled_uncited` rows in that window; 22 are translated below and the
// single row that is not is listed at the end with its reason.
//
// The window sits in the AI-script interpreter and the `CUnit` command
// helpers that feed it.  Seven of the rows are whole-function obfuscated
// bodies that Ghidra split into `~2.5k`-line listings: `0x0063e2e2`,
// `0x0063e35a`, `0x0063e565` and `0x0063eb0a`/`0x0063ed3c`/`0x0063efa0`/
// `0x0063f269`.  Those listings contain three things that survive the damage
// and are reproduced exactly here as pure arithmetic:
//
// * the integrity / cookie preamble (`0x93af10f8` seed, `0x93af` fold,
//   `0x00f7ef98` table index pair, `0x112da` bias, always-taken guard);
// * the `units.dat` records they walk (`0x34` stride, `u16` limits at
//   `+0x0c`/`+0x0e`, two counters at `+0x14`/`+0x16`);
// * the shared opcode dispatch (112 cases, `0x24` absent, `0x3e`/`0x46`/
//   `0x4c` aliased, `0xe4` unit-type operand bound) and the pool scan
//   (`count = raw ^ (key + 0x4163f040)`, `0xdce84bd8` slot tag,
//   `(i + 1) % count` ring advance) plus the `FNV-1a` recurrence with
//   basis `0x811c9dc5` and prime `0x01000193`.
//
// The other 15 translated rows are ordinary `CUnit` / command handlers.
// Recovered state used by them:
//
// | owner          | words                                                        |
// |----------------|--------------------------------------------------------------|
// | `CUnit`        | `+0x4c` owner, `+0x4d` type byte, `+0x5c`, `+0x64` type u16, `+0x6c` next, `+0x74`, `+0x7c`, `+0x92` mask, `+0x93` flags, `+0xa2` energy `Q8`, `+0xa3` byte, `+0xa6` state byte, `+0xdc` status dword, `+0x134` order target |
// | region chunks  | `map+0x0c` `u16[0x100]` grid, `map+0x1400c` 6-byte split entries      |
// | tile records   | `map+tile*0x40+0x44a0c` / `+0x44a10`, world = `>> 8`                 |
// | per-player     | `0x00faa410[player*0x4e8] & 0x20`, `0x00f8fa08[type*4]` masks       |
//
// Address to function:
//
// * `0x0063e2e2` [`a86_cookie_indices`], [`a86_cookie_mix`],
//   [`a86_cookie_gate`], [`a86_completed_flag`], [`a86_completed_gate`]
// * `0x0063e35a` [`a86_entry_tag_compare`], [`a86_entry_tag_taken`]
// * `0x0063e565` [`a86_units_dat_record`], [`a86_units_dat_top_up`]
// * `0x0063e838` [`a86_retry_guard`]
// * `0x0063e875` [`a86_returns_immediately`]
// * `0x0063e880` [`a86_unit_kind_or_player_flag`]
// * `0x0063eb0a` [`a86_opcode_valid`], [`a86_opcode_group`],
//   [`a86_unit_type_in_range`], [`a86_opcode_reads_unit_type`],
//   [`a86_decode_stride`]
// * `0x0063ed3c` [`a86_script_table_base`], [`a86_stride_3c_fits`]
// * `0x0063efa0` [`a86_fnv1a32`]
// * `0x0063f269` [`a86_pool_count`], [`a86_pool_tag_matches`],
//   [`a86_pool_ring_next`], [`a86_pool_pair_index`], [`a86_pool_not_found`]
// * `0x0063f7b0` [`a86_udivmod64`], [`a86_f7b0_divisor`]
// * `0x0063f7e0` [`a86_orphan_len`], [`a86_orphan_sweep`]
// * `0x0063f874` [`a86_orphan_sweep_from`]
// * `0x0063f8b0` [`a86_validate_target_list`]
// * `0x0063f950` [`a86_tile_world`], [`a86_chunk_index`],
//   [`a86_subtile_bit`], [`a86_region_at`], [`a86_region_unit_query`],
//   [`a86_region_unit_return`]
// * `0x0063fa20` [`a86_point_unit_return`]
// * `0x0063fad0` [`a86_area_mark_slot`]
// * `0x0063fb10` [`a86_lift_transition`]
// * `0x0063fb70` [`a86_ac_continues`], [`a86_ac_scan`],
//   [`a86_command_dispatch`]
// * `0x0063ff60` [`a86_voice_word`], [`a86_ability_gate`]
// * `0x0063ffa0` [`a86_mark_once`]
// * `0x0063fff0` [`a86_release_slot`]
//
// No `sim` record gains a field; every item below is local to this batch.

// ---------------------------------------------------------------------------
// 0x0063e2e2 - integrity preamble and the frame-cookie fold
// ---------------------------------------------------------------------------

/// The read-only probe address the preamble of `0x0063e2e2` and `0x0063e565`
/// stores at `EBP-0x64` (the listing prints the slot as `-100`, decimal) as
/// `&DAT_0f81369a`, and later feeds to `FUN_004a97c0`.
pub const A86_GUARD_PROBE: u32 = 0x0f81_369a;

/// The plaintext word the same two rows store at `EBP-0x80` (`0x93af10f8`)
/// and xor into the recomputed cookie.
pub const A86_GUARD_SEED: u32 = 0x93af_10f8;

/// The low-half fold constant (`uVar17 - 0x93af`).
pub const A86_GUARD_FOLD: u32 = 0x93af;

/// `0x0063e2e2`/`0x0063e565`: the two `0x00f7ef98` table indices taken out of
/// the probe word.
///
/// ```c
/// table[probe & 0xfff]        // low half of the cookie table entry
/// table[probe >> 0x14]        // entry used by the second guard
/// *(ushort *)(&DAT_00f7ef9a + (probe & 0xfff))   // high half, same entry
/// ```
///
/// `0x00f7ef9a` is `0x00f7ef98 + 2`, i.e. the second `u16` of the same
/// 4-byte cookie-table entry.
pub fn a86_cookie_indices(probe: u32) -> (usize, usize) {
    ((probe & 0x0fff) as usize, (probe >> 20) as usize)
}

/// `0x0063e2e2`/`0x0063e565`: recompute the cookie word held at `EBP-0x0c`.
///
/// ```c
/// low   = field & 0xffff;
/// field = ((field >> 0x10 ^ table[probe & 0xfff] ^ low ^ (low - 0x93af)) << 0x10
///          | low) ^ seed;
/// ```
///
/// The `|` sits inside explicit parentheses in the listing, so the shift and
/// the `or` bind before the final `^ seed`.  `table` is the `u16` read at
/// `&DAT_00f7ef98 + (probe & 0xfff)` (or `&DAT_00f7ef9a` for the high half,
/// which is the caller's choice, not the fold's).
pub fn a86_cookie_mix(field: u32, table_word: u16, seed: u32) -> u32 {
    let low = field & 0xffff;
    let mixed = (field >> 16) ^ (table_word as u32) ^ low ^ low.wrapping_sub(A86_GUARD_FOLD);
    ((mixed << 16) | low) ^ seed
}

/// The verdict of the preamble's second guard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A86CookieGate {
    /// The guard compared equal and the low half was zero, so the row's body
    /// is skipped.
    Clean,
    /// The shipped branch was taken; the row falls into the integrity abort.
    Dirty,
}

/// `0x0063e2e2`/`0x0063e565`: the second guard, verbatim.
///
/// ```c
/// if (((field >> 0x10 ^ low * 2 + table[probe >> 0x14] * -0x200) & 0xffff) !=
///     (table[probe & 0xfff] ^ low) || low != 0)
/// ```
///
/// `low_word` is `table[probe & 0xfff]` and `high_word` is
/// `table[probe >> 0x14]`; [`a86_cookie_indices`] produces the two indices.
/// `-0x200` is `0xfffffe00`, i.e. the table entry shifted into the high half
/// and subtracted, which is how the compiler expressed `- (entry << 9)`.
pub fn a86_cookie_gate(field: u32, low_word: u16, high_word: u16) -> A86CookieGate {
    let low = field & 0xffff;
    let mixed = (field >> 16)
        ^ low
            .wrapping_mul(2)
            .wrapping_add((high_word as u32).wrapping_mul(0xffff_fe00));
    let equal = (mixed & 0xffff) == ((low_word as u32) ^ low);
    if equal && low == 0 {
        A86CookieGate::Clean
    } else {
        A86CookieGate::Dirty
    }
}

/// `CUnit+0xdc >> 4 & 1`, the completed-construction bit, which is the value
/// `FUN_005b5840` returns at the tail of `0x0063e2e2`, `0x0063e35a` and
/// `0x0063e565`.
pub fn a86_completed_flag(field_dc: u32) -> u32 {
    (field_dc >> 4) & 1
}

/// The tail of `0x0063e2e2`, `0x0063e35a` and `0x0063e565`: the flag is
/// returned when set, and a clear flag reaches the `0x0063e838` continuation
/// that the row pushes as the return address immediately before `FUN_005b4cd0`.
pub fn a86_completed_gate(field_dc: u32) -> Result<u32, u32> {
    match a86_completed_flag(field_dc) {
        0 => Err(A86_CONTINUATION),
        flag => Ok(flag),
    }
}

/// The continuation address both guards leave on the stack (`0x0063e838`).
pub const A86_CONTINUATION: u32 = 0x0063_e838;

// ---------------------------------------------------------------------------
// 0x0063e35a - the second entry tag check
// ---------------------------------------------------------------------------

/// The literal low half `0x0063e35a` builds with `CONCAT31(uVar6, 0x6c)`.
pub const A86_ENTRY_TAG: u32 = 0x6c;

/// The bias the same row adds before comparing (`0x112da`).
pub const A86_ENTRY_TAG_BIAS: u32 = 0x0001_12da;

/// `0x0063e35a`: the comparison half of the entry guard.
///
/// ```c
/// ((high ^ (0x6c + 0x112da)) & 0xffff) != (table[probe >> 0x14] ^ 0x6c)
/// ```
///
/// Returns `true` when the two halves differ, i.e. when the guard fires.
pub fn a86_entry_tag_compare(high: u32, table_word: u16) -> bool {
    let low = A86_ENTRY_TAG;
    ((high ^ low.wrapping_add(A86_ENTRY_TAG_BIAS)) & 0xffff) != ((table_word as u32) ^ low)
}

/// `0x0063e35a`: the whole guard.
///
/// The listing ends the condition with `|| uVar5 != 0` and `uVar5` is the
/// literal `0x6c`, so the `0x0063e35a` guard is a tautology and the row
/// always runs its integrity path regardless of the tag values.  The constant
/// is returned as data instead of a `true` literal so the recovery stays
/// visible.
pub fn a86_entry_tag_taken(high: u32, table_word: u16) -> bool {
    a86_entry_tag_compare(high, table_word) || A86_ENTRY_TAG != 0
}

// ---------------------------------------------------------------------------
// 0x0063e565 - the units.dat record pair it tops up
// ---------------------------------------------------------------------------

/// `units.dat` record stride (`uVar24 * 0x34 + iVar9`).
pub const A86_UNITS_DAT_STRIDE: u32 = 0x34;

/// The row's payload offset: the first statement is
/// `*(int *)(EBP - 0x9c) = in_EAX + 0x1c`.
pub const A86_E565_PAYLOAD: u32 = 0x1c;

/// `0x0063e565`: `units.dat` record address for a unit type.
///
/// ```c
/// pcVar34 = (code *)((uint)uVar24 * 0x34 + iVar9);   // iVar9 = table base
/// ```
pub fn a86_units_dat_record(unit_type: u16, table_base: u32) -> u32 {
    (unit_type as u32)
        .wrapping_mul(A86_UNITS_DAT_STRIDE)
        .wrapping_add(table_base)
}

/// `0x0063e565`: one of the two counter top-up loops.
///
/// ```c
/// uVar1  = *(ushort *)(record + 0xe);      // limit
/// uVar11 = *(ushort *)(unit + 0x16);       // counter
/// if (uVar1 != 0 && uVar11 < uVar1) {
///     do {
///         iVar9  = FUN_006523f0(...);      // step
///         uVar11 = uVar11 + iVar9;
///         uVar17 = *(ushort *)(record + 0xe);   // limit re-read
///     } while (uVar11 < uVar17);
/// }
/// ```
///
/// The second loop is identical with `+0xc` and the unit's `+0x14` counter.
/// The limit is reloaded from the record on every pass, so a step of zero
/// spins rather than terminating; that is reproduced rather than hidden.
pub fn a86_units_dat_top_up(current: u16, limit: u16, mut step: impl FnMut() -> u32) -> u32 {
    let mut value = current as u32;
    let mut bound = limit as u32;
    if bound == 0 || value >= bound {
        return value;
    }
    loop {
        value = value.wrapping_add(step());
        bound = limit as u32;
        if value >= bound {
            return value;
        }
    }
}

// ---------------------------------------------------------------------------
// 0x0063e838 - the continuation the three guards jump to
// ---------------------------------------------------------------------------

/// The two unit type bytes the continuation matches (`0x9c`, i.e. `-100` as a
/// signed byte, and `2`).
pub const A86_RETRY_TYPES: [u8; 2] = [0x9c, 0x02];

/// The `in_AX` value the continuation skips (`0x29`).
pub const A86_RETRY_SKIP_AX: u16 = 0x29;

/// The command byte and the order the unreachable half would post
/// (`FUN_00534aa0(0xb, owner)` and `FUN_005b7d00(0x74)`).
pub const A86_RETRY_COMMAND: u8 = 0x0b;
pub const A86_RETRY_ORDER: u16 = 0x74;

/// `0x0063e838`: the retry gate.
///
/// ```c
/// if (in_AX != 0x29 && (*(char *)(unit + 0x4d) == 0x9c || == 2)) {
///     FUN_005bb740();        // FUN_005bb740 tail-calls FUN_0058b640, noreturn
///     ...                    // dead
/// }
/// ```
///
/// `FUN_005bb740` is an eight-byte tail call into the noreturn `FUN_0058b640`, so
/// the `FUN_00534aa0(0xb, ...)` / `FUN_005b7d00(0x74)` half can never run.
/// The recovered predicate is what the row actually decides.
pub fn a86_retry_guard(ax: u16, unit_type: u8) -> bool {
    ax != A86_RETRY_SKIP_AX && A86_RETRY_TYPES.contains(&unit_type)
}

// ---------------------------------------------------------------------------
// 0x0063e875 - four-byte empty body
// ---------------------------------------------------------------------------

/// `0x0063e875`: the whole row is `retn` (four bytes, no stack adjustment).
/// It is kept as an explicit no-op so callers that model the dispatch slot
/// have a defined target.
pub fn a86_returns_immediately() {}

// ---------------------------------------------------------------------------
// 0x0063e880 - owner flag fallback
// ---------------------------------------------------------------------------

/// `0x00faa410` per-player stride used by `FUN_00635720`.
pub const A86_PLAYER_STRIDE: u32 = 0x4e8;

/// The bit `FUN_00635720` masks out (`0x20`).
pub const A86_PLAYER_FLAG_BIT: u16 = 0x20;

/// `0x0063e880`: the row's return value.
///
/// ```c
/// uVar3 = FUN_00635720(*(byte *)(unit + 0x4c));   // player word & 0x20
/// if (uVar3 == 0) {
///     bVar1 = *(byte *)(unit + 0x4d);             // unit type byte
///     uVar3 = bVar1;
///     ...                                         // 0x9c/2 branch aborts
/// }
/// return uVar3;
/// ```
///
/// `FUN_00635720` is `*(ushort *)(&DAT_00faa410 + player * 0x4e8) & 0x20`, so
/// a set bit yields `0x20` and a clear bit yields the unit type byte widened
/// to `u32`.
pub fn a86_unit_kind_or_player_flag(player_word: u16, unit_type: u8) -> u32 {
    let flag = (player_word & A86_PLAYER_FLAG_BIT) as u32;
    if flag == 0 { unit_type as u32 } else { flag }
}

// ---------------------------------------------------------------------------
// 0x0063eb0a / 0x0063ed3c / 0x0063efa0 / 0x0063f269 - opcode dispatch
// ---------------------------------------------------------------------------

/// Highest opcode byte the dispatch accepts (`0x70`).
pub const A86_OPCODE_MAX: u8 = 0x70;

/// The one gap in the dispatch table: `0x24` has no `case`.
pub const A86_OPCODE_HOLE: u8 = 0x24;

/// The cases that share one body (`case 0x3e: case 0x46: case 0x4c:`).
pub const A86_OPCODE_ALIASES: [u8; 3] = [0x3e, 0x46, 0x4c];

/// Number of `case` labels the listing carries, aliases included.
pub const A86_OPCODE_CASES: usize = 112;

/// The unit-type operand bound the dispatch tests (`< 0xe4`).
pub const A86_UNIT_TYPE_LIMIT: u16 = 0xe4;

/// The case labels whose body validates a `u16` unit-type operand against
/// `0xe4`, in table order.  Extracted from the `&DAT_000000e4` stores in
/// `0x0063eb0a`, `0x0063efa0` and `0x0063f269`; all three listings agree.
pub const A86_UNIT_TYPE_BOUNDED: [u8; 15] = [
    0x0a, 0x0c, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x45, 0x4b, 0x4c, 0x4e, 0x53,
];

/// `0x0063eb0a`: is the opcode byte a `case` of the dispatch?
///
/// The listing has `0x00..=0x23` and `0x25..=0x70`, which is 112 labels.
pub fn a86_opcode_valid(opcode: u8) -> bool {
    opcode <= A86_OPCODE_MAX && opcode != A86_OPCODE_HOLE
}

/// `0x0063eb0a`: the canonical body for an opcode.
///
/// `0x46` and `0x4c` have no body of their own; they fall into `0x3e`.
pub fn a86_opcode_group(opcode: u8) -> u8 {
    if A86_OPCODE_ALIASES[1..].contains(&opcode) {
        A86_OPCODE_ALIASES[0]
    } else {
        opcode
    }
}

/// `0x0063eb0a`: `if (uVar2 < 0xe4)` - the unit-type operand bound.
pub fn a86_unit_type_in_range(unit_type: u16) -> bool {
    unit_type < A86_UNIT_TYPE_LIMIT
}

/// `0x0063eb0a`: does this opcode validate its first operand as a unit type?
pub fn a86_opcode_reads_unit_type(opcode: u8) -> bool {
    A86_UNIT_TYPE_BOUNDED.contains(&opcode)
}

/// `0x0063eb0a`: the repeat/stride word the prologue installs.
///
/// ```c
/// n = 1;
/// if ((DAT_00f7ea50 & 0x500) == 0) {
///     if (iVar27 != 0xfd57f28b) n = 8;
/// } else {
///     n = 0x10;
/// }
/// ```
///
/// `iVar27` is `CONCAT31(in_EAX >> 8, DAT_00f7ea50._1_1_)`, i.e. a value the
/// obfuscation seeds from caller state; the comparison is against the 32-bit
/// form of `-0x2a80d75`, which is `0xfd57f28b`.
pub fn a86_decode_stride(flags: u16, seeded: u32) -> u32 {
    if flags & 0x0500 != 0 {
        0x10
    } else if seeded != 0xfd57_f28b {
        8
    } else {
        1
    }
}

/// `0x0063ed3c`: the tagged script-table pointer base.
///
/// The row reads the table through `DAT_00f063cc ^ 0x10fae560`; the same
/// xor-obfuscated global is used by every unit-table read in the window.
pub fn a86_script_table_base(stored: u32) -> u32 {
    stored ^ 0x10fa_e560
}

/// `0x0063ed3c`, `case 0x31`: the record bound test.
///
/// ```c
/// if ((uint)bVar12 * 0x3c <= *(uint *)(table + 0xe608)) { ... }
/// ```
///
/// `0x3c` is the stride of the record the byte indexes; `+0xe608` is the
/// stored record count/limit in the same tagged table.
pub fn a86_stride_3c_fits(index: u8, limit: u32) -> bool {
    (index as u32).wrapping_mul(0x3c) <= limit
}

/// `0x0063efa0`: the 32-bit `FNV-1a` recurrence the row runs over the head
/// (up to a 16-byte boundary) and the tail of the buffer.
///
/// ```c
/// uVar13 = 0x811c9dc5;
/// do { uVar13 = (*pbVar20 ^ uVar13) * 0x1000193; } while (++i < len);
/// ```
///
/// The middle of the row is the compiler's 16-byte unrolled form of the same
/// recurrence: it xors a whole 16-byte block and multiplies each `u16` lane by
/// the constant vector at `0x00d3c560`, then folds four dwords with `^`.  That
/// constant vector is outside the row, so only the scalar recurrence is
/// reproduced; it is the one the row uses for every byte before the first
/// aligned block and after the last one.
pub const A86_FNV_OFFSET: u32 = 0x811c_9dc5;

/// `FNV-1a` prime for the same row.
pub const A86_FNV_PRIME: u32 = 0x0100_0193;

/// `0x0063efa0`: scalar `FNV-1a` over `bytes`, from `A86_FNV_OFFSET`.
pub fn a86_fnv1a32(bytes: &[u8]) -> u32 {
    let mut hash = A86_FNV_OFFSET;
    for &byte in bytes {
        hash = (byte as u32 ^ hash).wrapping_mul(A86_FNV_PRIME);
    }
    hash
}

/// `0x0063f269`: the slot tag the pool scan looks for (`0xdce84bd8`).
pub const A86_POOL_TAG: u32 = 0xdce8_4bd8;

/// `0x0063f269`: the bias added to the pool key before the xor
/// (`0x4163f040`).
pub const A86_POOL_COUNT_BIAS: u32 = 0x4163_f040;

/// `0x0063f269`/`0x0063eb0a`: the ring length.
///
/// ```c
/// count = *(uint *)(pool + 0xfc) ^ (DAT_00f7f79e + 0x4163f040);
/// ```
///
/// `+` binds tighter than `^`, so the bias is added to the key first.
pub fn a86_pool_count(raw: u32, pool_key: u32) -> u32 {
    raw ^ pool_key.wrapping_add(A86_POOL_COUNT_BIAS)
}

/// `0x0063f269`: the slot tag test.
///
/// ```c
/// if ((*(uint *)(slot * 8 + 4 + base) ^ _DAT_00f7fcf6 ^ 0xdce84bd8) != 0) break;
/// ```
///
/// The scan keeps walking while the expression is non-zero, so a slot is valid
/// exactly when the tag equals `key ^ 0xdce84bd8`.
pub fn a86_pool_tag_matches(raw_tag: u32, tag_key: u32) -> bool {
    raw_tag ^ tag_key ^ A86_POOL_TAG == 0
}

/// `0x0063f269`: the ring advance `(index + 1) % count`.
///
/// The listing computes this modulo with the tag-stripped count in two places
/// (`uStack_24` and `EBP-0x20`); a zero count would fault in the shipped code
/// and is reported as `None` here.
pub fn a86_pool_ring_next(index: u32, count: u32) -> Option<u32> {
    if count == 0 {
        None
    } else {
        Some(index.wrapping_add(1) % count)
    }
}

/// `0x0063f269`: the composite index the row multiplies once it settles on a
/// slot (`*(int *)(EBP-0x20) * *(int *)(EBP-0x24)`) and the value it returns
/// when the scan found nothing (`DAT_00f7f79e + 0x4163f040`).
pub fn a86_pool_pair_index(inner: u32, outer: u32) -> u32 {
    inner.wrapping_mul(outer)
}

/// `0x0063f269`: the scan's "nothing found" return value.
pub fn a86_pool_not_found(pool_key: u32) -> u32 {
    pool_key.wrapping_add(A86_POOL_COUNT_BIAS)
}

// ---------------------------------------------------------------------------
// 0x0063f7b0 - the frame sweep
// ---------------------------------------------------------------------------

/// `0x0063f7b0`: the mask applied to the frame word (`0x82b`).
pub const A86_F7B0_MASK: u32 = 0x082b;

/// `0x0063f7b0`: the second frame word, `0x83b ^ 0x82b`, which folds to
/// `0x10`.
pub const A86_F7B0_SECOND: u32 = 0x083b ^ 0x082b;

/// `0x0063f7b0`: the 64-bit divisor the row derives from the masked word.
///
/// ```c
/// *(uint *)(EBP-8) ^= 0x82b;
/// _aulldvrm(rdtsc_lo, rdtsc_hi, *(int *)(EBP-8), *(int *)(EBP-8) >> 0x1f);
/// ```
///
/// `FUN_00c87420` is `_aulldvrm`: it returns the quotient in `EAX` and the
/// remainder in `EDX` for a 64-by-64 unsigned divide, and the fourth argument
/// supplies the divisor's high half, so the divisor is the masked word
/// sign-extended to 64 bits.
pub fn a86_f7b0_divisor(word: u32) -> u64 {
    (word ^ A86_F7B0_MASK) as i32 as i64 as u64
}

/// `0x0063f7b0`: the divide itself, with `_aulldvrm`'s `(quotient,
/// remainder)` result.  A zero divisor raises `#DE` on the shipped CPU and is
/// reported as `None`.
pub fn a86_udivmod64(numerator: u64, divisor: u64) -> Option<(u64, u64)> {
    let quotient = numerator.checked_div(divisor)?;
    let remainder = numerator.checked_rem(divisor)?;
    Some((quotient, remainder))
}

// ---------------------------------------------------------------------------
// 0x0063f7e0 / 0x0063f874 - the orphan sweeps
// ---------------------------------------------------------------------------

/// `0x0063f7e0`: `0x182b ^ 0x82b`, the bias subtracted from the range length.
pub const A86_ORPHAN_BIAS: i32 = 0x182b ^ 0x082b;

/// `0x0063f7e0`/`0x0063f874`: `0x8d4 ^ 0x82b`, the failure code.
pub const A86_ORPHAN_FAIL: u32 = 0x08d4 ^ 0x082b;

/// The six stack slots a sweep probe must avoid (the `EBP-0x28`, `-0x68`,
/// `-0x6c`, `-0x70`, `-0x74` and `-0x78` addresses of the frame that owns the
/// container).
pub const A86_ORPHAN_SLOTS: usize = 6;

/// `0x0063f7e0`: the sweep length.
///
/// ```c
/// end = *(int *)(container + 8) + offset + count;      // EBP-0x28
/// begin = *(int *)(container + 4);
/// len = (begin - end) - 0x1000;
/// ```
///
/// The `0x1000` is a debug-range bias, not a byte count, so `len` is usually
/// negative; the loop below compares it as an unsigned word exactly as the
/// listing does.
pub fn a86_orphan_len(begin: i32, end: i32) -> i32 {
    begin.wrapping_sub(end).wrapping_sub(A86_ORPHAN_BIAS)
}

/// `0x0063f7e0`: walk the sweep and report the first probe that collides with
/// one of the six frame slots.
///
/// ```c
/// for (i = 0; i != len; i += 4) {
///     p = end + i;
///     if (p == EBP-0x28 || p == EBP-0x68 || ... ) abort(0xff);
/// }
/// ```
///
/// Returns `Err(A86_ORPHAN_FAIL)` on the collision and `Ok(probes)` with the
/// number of probes taken otherwise.
pub fn a86_orphan_sweep(end: u32, len: i32, slots: &[u32; A86_ORPHAN_SLOTS]) -> Result<u32, u32> {
    if len == 0 {
        return Ok(0);
    }
    let mut index: u32 = 0;
    let mut probes = 0;
    loop {
        let probe = end.wrapping_add(index);
        if slots.contains(&probe) {
            return Err(A86_ORPHAN_FAIL);
        }
        probes += 1;
        index = index.wrapping_add(4);
        if index >= len as u32 {
            return Ok(probes);
        }
    }
}

/// `0x0063f874`: the same sweep entered from a stored index.
///
/// ```c
/// *out = in_AL;                       // one byte stored before the walk
/// i = *(int *)(EBP-0xc);
/// loop {
///     next = i + 1;
///     if (len <= next) { unlock(); return; }
///     p = first + next;
///     if (p collides) abort(0xff);
///     i += 4;
/// }
/// ```
///
/// The counter advances by four but the probe uses `i + 1`, so the probes are
/// `first + start + 1 + 4k` while `start + 1 + 4k < len`.
pub fn a86_orphan_sweep_from(
    start: u32,
    len: u32,
    first: u32,
    slots: &[u32; A86_ORPHAN_SLOTS],
) -> Result<u32, u32> {
    let mut counter = start;
    let mut probes = 0;
    loop {
        let next = counter.wrapping_add(1);
        if len <= next {
            return Ok(probes);
        }
        let probe = first.wrapping_add(next);
        if slots.contains(&probe) {
            return Err(A86_ORPHAN_FAIL);
        }
        probes += 1;
        counter = counter.wrapping_add(4);
    }
}

// ---------------------------------------------------------------------------
// 0x0063f8b0 - target list validation walk
// ---------------------------------------------------------------------------

/// One record of the list rows `0x0063f8b0`, `0x0063f950` and `0x0063fa20`
/// walk.  Each field is one of the predicates the shipped loop calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A86ListUnit {
    /// `FUN_005a3180`: the record is gone (dead, or `+0x4d == 0`).
    pub gone: bool,
    /// `FUN_005b6370`: `0x00f8fa08[+0x64] & 0x10`, the row's skip mask.
    pub hidden: bool,
    /// `FUN_005b5b90(player, unit)`: the owner test.
    pub matched: bool,
    /// `FUN_0075d940(FUN_0075d7b0(unit))`: the tile marker is `0x1ffd`.
    pub has_target: bool,
}

/// `0x0063f8b0`: the walk's outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A86ListVerdict {
    /// The walk reached the null link and the row returns `0`.
    Exhausted,
    /// A matching record failed the target test and the row reaches
    /// `FUN_0058b650`.
    TargetMissing(usize),
}

/// `0x0063f8b0`: the validation walk.
///
/// ```c
/// while (true) {
///     if (unit == 0) return 0;
///     if (!gone && !hidden && match(player, unit)) {
///         tile = FUN_0075d7b0(unit);
///         if (!FUN_0075d940(tile)) FUN_0058b650();
///     }
///     unit = *(int *)(unit + 0x6c);          // FUN_005b4bb0
/// }
/// ```
///
/// The match does not terminate the walk: `0x0063f8b0` is a checker, not a
/// finder, and only the `FUN_0058b650` failure exits early.  The link word is
/// `+0x6c`, unlike the `+4` link the two nearest-unit rows follow.
pub fn a86_validate_target_list(units: &[A86ListUnit]) -> A86ListVerdict {
    for (index, unit) in units.iter().enumerate() {
        if unit.gone || unit.hidden || !unit.matched {
            continue;
        }
        if !unit.has_target {
            return A86ListVerdict::TargetMissing(index);
        }
    }
    A86ListVerdict::Exhausted
}

// ---------------------------------------------------------------------------
// 0x0063f950 / 0x0063fa20 - region and point queries
// ---------------------------------------------------------------------------

/// One 64-byte tile record as `FUN_0075d630` reads it: the two scaled position
/// words at `+0x0c` and `+0x10` of `map + tile * 0x40 + 0x44a0c`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A86Tile {
    /// `*(int *)(map + tile * 0x40 + 0x44a0c)`.
    pub x_word: i32,
    /// `*(int *)(map + tile * 0x40 + 0x44a10)`.
    pub y_word: i32,
}

/// `0x0063f950`: tile to world.  `FUN_0075d630` shifts both stored words right
/// by eight to drop the `1/256` sub-pixel fraction.
pub fn a86_tile_world(tile: &A86Tile) -> (i32, i32) {
    (tile.x_word >> 8, tile.y_word >> 8)
}

/// `0x0063f950`: chunk-grid width (`0x100` entries of `u16` at `map + 0x0c`).
pub const A86_CHUNK_WIDTH: u32 = 0x100;

/// `0x0063f950`: `FUN_0075d720`'s split threshold.  A chunk word above `0x1fff`
/// is an index into the six-byte split entries, not a region id.
pub const A86_REGION_SPLIT_MIN: u16 = 0x2000;

/// One six-byte split entry at `map + 0x1400c + index * 6`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A86RegionSplit {
    /// `+0x00`: the 4x4 sub-tile bitmap.
    pub mask: u16,
    /// `+0x02`: the region returned when the bit is clear.
    pub outside: u16,
    /// `+0x04`: the region returned when the bit is set.
    pub inside: u16,
}

/// `0x0063f950`: the chunk index.
///
/// ```c
/// ((y + (y >> 31 & 0x1f)) >> 5) * 0x100 + ((x + (x >> 31 & 0x1f)) >> 5)
/// ```
///
/// The `>> 31 & 0x1f` term is the compiler's signed-divide bias.  It is not a
/// floor for every negative input: `-1` maps to chunk `0`, and only `-32`
/// reaches chunk `-1`.
pub fn a86_chunk_index(x: i32, y: i32) -> u32 {
    let cx = (x + ((x >> 31) & 0x1f)) >> 5;
    let cy = (y + ((y >> 31) & 0x1f)) >> 5;
    (cy as u32)
        .wrapping_mul(A86_CHUNK_WIDTH)
        .wrapping_add(cx as u32)
}

/// `0x0063f950`: the 4x4 sub-tile bit, `((x >> 3) & 3) + ((y >> 3) & 3) * 4`.
pub fn a86_subtile_bit(x: i32, y: i32) -> u32 {
    let bx = ((x + ((x >> 31) & 0x07)) >> 3) & 3;
    let by = ((y + ((y >> 31) & 0x07)) >> 3) & 3;
    (bx + by * 4) as u32
}

/// `0x0063f950`: `FUN_0075d720`, the region id for a world point.
///
/// ```c
/// uVar2 = chunks[(y >> 5) * 0x100 + (x >> 5)];
/// if (0x1fff < uVar2) {
///     entry = splits[uVar2];
///     if (entry.mask & (1 << bit)) return entry.inside;
///     return entry.outside;
/// }
/// return uVar2;
/// ```
///
/// An out-of-range chunk index or split index would read past the map in the
/// shipped code; the Rust form reports `None` instead of inventing a value.
pub fn a86_region_at(chunks: &[u16], splits: &[A86RegionSplit], x: i32, y: i32) -> Option<u16> {
    let chunk = *chunks.get(a86_chunk_index(x, y) as usize)?;
    if chunk < A86_REGION_SPLIT_MIN {
        return Some(chunk);
    }
    let split = *splits.get(chunk as usize)?;
    if split.mask & (1 << a86_subtile_bit(x, y)) != 0 {
        Some(split.inside)
    } else {
        Some(split.outside)
    }
}

/// The sentinel both query rows write into the "best distance" slot
/// (`local_8 = 99999999`).
pub const A86_BEST_DISTANCE_INIT: i32 = 99_999_999;

/// The initial "best unit" slot (`local_c = 0`).
pub const A86_BEST_UNIT_INIT: u32 = 0;

/// `0x0063f950`'s "nothing found" return (`return 0xffffffff`).
pub const A86_NO_UNIT: u32 = 0xffff_ffff;

/// `0x0063fa20`'s "nothing found" return (`return 0`).
pub const A86_NO_POINT_UNIT: u32 = 0;

/// One candidate of the two nearest-unit rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A86CandidateUnit {
    /// `FUN_005a3180`.
    pub gone: bool,
    /// `FUN_005b5810`: `CUnit+0xdc >> 1 & 1`.
    pub active: bool,
    /// `FUN_005b5b90(player, unit)`.
    pub matched: bool,
}

/// `0x0063f950`/`0x0063fa20`: the filter chain.
///
/// ```c
/// for (unit = head; unit != 0; unit = *(int *)(unit + 4)) {
///     if (!gone && active && matched) break;
/// }
/// if (unit == 0) return <row sentinel>;
/// FUN_0058b650();                    // the listing's noreturn cut
/// ```
///
/// Both rows follow the `+4` link (unlike `0x0063f8b0`'s `+0x6c`), and both
/// initialise the `99999999` / `0` best-distance pair *before* the walk.  The
/// distance minimisation itself sits past the cut, so only the filter and the
/// sentinels are reproduced.
pub fn a86_region_unit_query(units: &[A86CandidateUnit]) -> Option<usize> {
    units
        .iter()
        .position(|unit| !unit.gone && unit.active && unit.matched)
}

/// `0x0063f950`: the whole row's return value given the candidate list.
///
/// `None` maps to `0xffffffff`, a hit to the matching unit's slot.
pub fn a86_region_unit_return(units: &[A86CandidateUnit]) -> u32 {
    match a86_region_unit_query(units) {
        None => A86_NO_UNIT,
        Some(index) => index as u32,
    }
}

/// `0x0063fa20`: the whole row's return value given the candidate list.
///
/// Identical filter, but the empty case returns `0` and the row takes the
/// point coordinates directly instead of converting a tile first.
pub fn a86_point_unit_return(units: &[A86CandidateUnit]) -> u32 {
    match a86_region_unit_query(units) {
        None => A86_NO_POINT_UNIT,
        Some(index) => index as u32,
    }
}

// ---------------------------------------------------------------------------
// 0x0063fad0 - area mark
// ---------------------------------------------------------------------------

/// `0x0063fad0`: the third argument the row forwards, `FUN_0063cdd0(a, b, 2)`.
pub const A86_AREA_MARK_VALUE: u8 = 2;

/// `FUN_0063cdd0`'s mark-table stride (`(param_1 + 0x131b) * 0xc`).
pub const A86_AREA_MARK_STRIDE: u32 = 0x0c;

/// `FUN_0063cdd0`'s mark-table base offset (`0x131b`).
pub const A86_AREA_MARK_BASE: u32 = 0x131b;

/// `0x0063fad0`: the offset the called helper writes for one unit.
///
/// ```c
/// *(byte *)(unit_owner + (player + 0x131b) * 0xc + table) = 2;
/// ```
///
/// The helper skips units whose `+0x4c` equals `player`, so only other owners
/// are marked.
pub fn a86_area_mark_slot(player: u8, unit_owner: u8) -> Option<u32> {
    if player == unit_owner {
        None
    } else {
        Some(
            (player as u32)
                .wrapping_add(A86_AREA_MARK_BASE)
                .wrapping_mul(A86_AREA_MARK_STRIDE)
                .wrapping_add(unit_owner as u32),
        )
    }
}

// ---------------------------------------------------------------------------
// 0x0063fb10 - lift state transition
// ---------------------------------------------------------------------------

/// `0x0063fb10`: the state byte the row requires (`'m'`).
pub const A86_LIFT_STATE_UP: u8 = 0x6d;

/// `0x0063fb10`: the state byte the row writes (`'n'`, via `FUN_005b7cd0`).
pub const A86_LIFT_STATE_DOWN: u8 = 0x6e;

/// `0x0063fb10`: the inclusive unit-type window that suppresses the write.
pub const A86_LIFT_TYPE_WINDOW: (u8, u8) = (0x7b, 0x81);

/// `0x0063fb10`: the mask `FUN_005b5950` tests (`0x00f8fa08[type] & 0x200`).
pub const A86_LIFT_FLAG: u32 = 0x200;

/// `FUN_005b7cd0`'s side effects: the three words cleared when the state byte
/// actually changes.
pub const A86_LIFT_CLEARED_FIELDS: [u16; 3] = [0x00e8, 0x00ec, 0x00e2];

/// The `CUnit` facts `0x0063fb10` reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A86LiftFacts {
    /// `+0xa6`, the animation/state byte.
    pub state: u8,
    /// `0x00f8fa08[+0x64] & 0x200`, from `FUN_005b5950`.
    pub flags: u32,
    /// `FUN_005a30c0(0) != 0`.
    pub pred_a: bool,
    /// `FUN_005b6a30()` - `*(int *)(+0x7c) != 0`.
    pub pred_b: bool,
    /// `FUN_005c7870(unit) != 0`.
    pub pred_c: bool,
    /// `+0x4d`, the unit type byte.
    pub unit_type: u8,
}

/// `0x0063fb10`: the state transition.
///
/// ```c
/// if (*(char *)(unit + 0xa6) == 'm') {
///     if (FUN_005b5950() != 0)
///         if (FUN_005a30c0(0) == 0)
///             if (FUN_005b6a30() == 0)
///                 if (FUN_005c7870(unit) == 0 &&
///                     (type < 0x7b || 0x81 < type))
///                     FUN_005b7cd0(0x6e);
/// }
/// ```
///
/// Returns the byte to write (`0x6e`) when every gate opens.  The write helper
/// `FUN_005b7cd0` only stores when the byte differs, which the `0x6d`/`0x6e`
/// gate guarantees, so it also clears `+0xe8`, `+0xec` and `+0xe2`.
pub fn a86_lift_transition(facts: &A86LiftFacts) -> Option<u8> {
    if facts.state != A86_LIFT_STATE_UP {
        return None;
    }
    if facts.flags & A86_LIFT_FLAG == 0 {
        return None;
    }
    if facts.pred_a || facts.pred_b || facts.pred_c {
        return None;
    }
    if A86_LIFT_TYPE_WINDOW.0 <= facts.unit_type && facts.unit_type <= A86_LIFT_TYPE_WINDOW.1 {
        return None;
    }
    Some(A86_LIFT_STATE_DOWN)
}

// ---------------------------------------------------------------------------
// 0x0063fb70 - command dispatch
// ---------------------------------------------------------------------------

/// An order/message the dispatch posts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A86Action {
    /// `FUN_005b7d00(order)`.
    Order(u16),
    /// `FUN_005b7dd0(order, operand)`.
    OrderTyped { order: u16, operand: u32 },
    /// `FUN_005b7d80(message, x, y)`.
    Message { message: u16, x: u16, y: u16 },
    /// `FUN_005b7c70(message)`.
    Post(u16),
    /// The row returns its value without posting anything.
    Nothing,
}

/// The dispatch result: the row's `return` value plus the posted action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A86DispatchResult {
    /// The row's return value (`0` or `1`).
    pub ret: u32,
    /// The posted action, if any.
    pub action: A86Action,
}

/// `0x0063fb70`: one entry of the `0xac` walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A86AcEntry {
    /// `FUN_005c1f10(unit) == 0`.
    pub pred_a_clear: bool,
    /// `FUN_005b49d0(unit) != 0`.
    pub pred_b_set: bool,
    /// `FUN_00656b00(order, 0) != 0`.
    pub pred_c_set: bool,
    /// `FUN_005b4a20()` value, compared against `0x28`.
    pub value: u32,
}

/// `0x0063fb70`: the facts the row reads for one `AX` message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A86DispatchInput {
    /// The message id (`in_AX`).
    pub ax: u16,
    /// `+0x4c`, the owner byte.
    pub owner: u8,
    /// `+0x4d`, the unit type byte.
    pub unit_type: u8,
    /// `+0x74`.
    pub field_74: u32,
    /// `FUN_00534aa0(3, owner) == 1`.
    pub can_3: bool,
    /// `FUN_006510c0() != 0`.
    pub pred_510c0: bool,
    /// `*(char *)(*(int *)(unit + 0x134) + 8) == 1`.
    pub target_kind_1: bool,
    /// `FUN_00659030(...) != 0`.
    pub pred_59030: bool,
    /// `FUN_00534aa0(5, owner) == 1`.
    pub can_5: bool,
    /// `FUN_00588d60()`.
    pub cmp_a: u32,
    /// `FUN_005b4b30()`.
    pub cmp_b: u32,
    /// `FUN_005c7850()` value; a non-zero result is the row's third gate and
    /// is also the operand of the typed order it posts.
    pub c7850: u32,
    /// `FUN_005d5d10() != 0`.
    pub pred_d5d10: bool,
    /// `FUN_006560f0() != 0`.
    pub pred_560f0: bool,
    /// `FUN_00591ba0()`: `0x49`, `0x55` or `0`.
    pub msg_kind: u16,
    /// `FUN_0065b2b0(msg_kind, owner) == 1`.
    pub can_msg: bool,
    /// `FUN_0058e040(msg_kind) != 0`.
    pub pred_e040: bool,
    /// `FUN_00590f50() != 0`.
    pub pred_90f50: bool,
    /// `+0xa3`.
    pub state_a3: u8,
    /// The message's two coordinate words (`EBP-4` / `EBP+8`).
    pub x: u16,
    /// The message's second coordinate word.
    pub y: u16,
}

/// The non-posting outcome of `0x0063fb70`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A86Dispatch {
    /// The row posted something.
    Posted(A86DispatchResult),
    /// The row fell through the switch and returned `0`.
    Ignored,
    /// The row reached the `0x47` fatal continuation (`FUN_0058b640`).
    Fatal,
    /// The `0xac` walk found an entry that fails its continue test, which
    /// reaches `FUN_0058b650`.
    NotFound,
}

/// `0x0063fb70`: the `0xac` continue test.
///
/// ```c
/// while (a == 0 || b != 0 || c != 0 || d > 0x28) { next; if (next == 0) return 0; }
/// FUN_0058b650();
/// ```
pub fn a86_ac_continues(entry: &A86AcEntry) -> bool {
    entry.pred_a_clear || entry.pred_b_set || entry.pred_c_set || entry.value > 0x28
}

/// `0x0063fb70`: the `0xac` walk.
///
/// Returns `true` when the walk runs off the list (the row returns `0`) and
/// `false` when it stops on an entry (`FUN_0058b650`).
pub fn a86_ac_scan(entries: &[A86AcEntry]) -> bool {
    entries.iter().all(a86_ac_continues)
}

/// `0x0063fb70`: the message dispatch.
///
/// The `case` bodies, with the `0x0b -> 0x45 -> 0x2a` cascade reproduced as
/// the fallthroughs the listing shows:
///
/// * `0x02`: `can_3 && pred_510c0` -> message `0x84` with the two words.
/// * `0x05`: target kind `1`, `!pred_59030`, `can_5`, `+0x74 == 0` -> order
///   `0x62`.
/// * `0x0b`: `cmp_a != cmp_b && c7850 != 0`, then either the typed order
///   `0x31` carrying the `FUN_005c7850` result or, when `pred_d5d10`, the plain
///   order `0x6f`.
/// * `0x45`: `pred_560f0` -> return `1` without posting; else fall into
///   `0x2a`.
/// * `0x2a`: `pred_d5d10` -> order `0x6f`.
/// * `0x47`: fatal.
/// * `0x48`/`0x53`: `can_msg && pred_e040 && !pred_90f50` -> post `0x3f`
///   and return `0`.
/// * `0x7c`/`0x90`/`0x92`/`0xa2`: order `0x12`.
/// * `0xac`: only when `+0xa3 > 0x31`; walks the list.
pub fn a86_command_dispatch(input: &A86DispatchInput, ac_entries: &[A86AcEntry]) -> A86Dispatch {
    match input.ax {
        0x02 => {
            if input.can_3 && input.pred_510c0 {
                A86Dispatch::Posted(A86DispatchResult {
                    ret: 1,
                    action: A86Action::Message {
                        message: 0x84,
                        x: input.x,
                        y: input.y,
                    },
                })
            } else {
                A86Dispatch::Ignored
            }
        }
        0x05 => {
            if input.target_kind_1 && !input.pred_59030 && input.can_5 && input.field_74 == 0 {
                A86Dispatch::Posted(A86DispatchResult {
                    ret: 1,
                    action: A86Action::Order(0x62),
                })
            } else {
                A86Dispatch::Ignored
            }
        }
        0x0b => {
            if input.cmp_a != input.cmp_b && input.c7850 != 0 {
                if input.pred_d5d10 {
                    A86Dispatch::Posted(A86DispatchResult {
                        ret: 1,
                        action: A86Action::Order(0x6f),
                    })
                } else {
                    // `FUN_005b7dd0(0x31, iVar3)` reuses the `FUN_005c7850`
                    // result, which the decompiler assigned back into the same
                    // local as the `FUN_00588d60` comparison.
                    A86Dispatch::Posted(A86DispatchResult {
                        ret: 1,
                        action: A86Action::OrderTyped {
                            order: 0x31,
                            operand: input.c7850,
                        },
                    })
                }
            } else {
                a86_command_dispatch_group_45(input)
            }
        }
        0x45 => a86_command_dispatch_group_45(input),
        0x2a => {
            if input.pred_d5d10 {
                A86Dispatch::Posted(A86DispatchResult {
                    ret: 1,
                    action: A86Action::Order(0x6f),
                })
            } else {
                A86Dispatch::Ignored
            }
        }
        0x47 => A86Dispatch::Fatal,
        0x48 | 0x53 => {
            if input.can_msg && input.pred_e040 && !input.pred_90f50 {
                A86Dispatch::Posted(A86DispatchResult {
                    ret: 0,
                    action: A86Action::Post(0x3f),
                })
            } else {
                A86Dispatch::Ignored
            }
        }
        0x7c | 0x90 | 0x92 | 0xa2 => A86Dispatch::Posted(A86DispatchResult {
            ret: 1,
            action: A86Action::Order(0x12),
        }),
        0xac => {
            if input.state_a3 > 0x31 {
                if a86_ac_scan(ac_entries) {
                    // The walk ran off the list and the row returns 0 without
                    // posting anything.
                    A86Dispatch::Ignored
                } else {
                    A86Dispatch::NotFound
                }
            } else {
                A86Dispatch::Ignored
            }
        }
        _ => A86Dispatch::Ignored,
    }
}

/// The `0x45`/`0x2a` half of `0x0063fb70`, shared by the `0x0b` fallthrough.
fn a86_command_dispatch_group_45(input: &A86DispatchInput) -> A86Dispatch {
    if input.pred_560f0 {
        A86Dispatch::Posted(A86DispatchResult {
            ret: 1,
            action: A86Action::Nothing,
        })
    } else if input.pred_d5d10 {
        A86Dispatch::Posted(A86DispatchResult {
            ret: 1,
            action: A86Action::Order(0x6f),
        })
    } else {
        A86Dispatch::Ignored
    }
}

// ---------------------------------------------------------------------------
// 0x0063ff60 - ability gate
// ---------------------------------------------------------------------------

/// `0x0063ff60`: the per-player table `FUN_005b2510` indexes (`0x00f8abd0`).
pub const A86_VOICE_LIMIT: u8 = 0x2c;

/// `0x0063ff60`: `FUN_005cc8e0`'s "ignore energy" switch
/// (`DAT_01165d00 & 0x800`).
pub const A86_ENERGY_BYPASS: u16 = 0x0800;

/// `0x0063ff60`: `FUN_005b2510`'s table walk (`&DAT_00f8abd0` ->
/// `&DAT_00f8dc10`, `u16 << 8`).
///
/// ```c
/// if ((byte)(&DAT_00f8abd0)[param_1] < 0x2c)
///     return *(short *)(&DAT_00f8dc10 + table[param_1] * 2) << 8;
/// return 0;
/// ```
pub fn a86_voice_word(index_table: &[u8], words: &[i16], player: u8) -> i16 {
    let index = *index_table.get(player as usize).unwrap_or(&0xff);
    if index < A86_VOICE_LIMIT {
        *words.get(index as usize).unwrap_or(&0) << 8
    } else {
        0
    }
}

/// `0x0063ff60`: the gate.
///
/// ```c
/// if (FUN_005cc8e0(FUN_005b2510(param_2))) return false;      // not enough
/// return FUN_00534aa0(param_3, *(byte *)(unit + 0x4c), unit) == 1;
/// ```
///
/// `FUN_005cc8e0` reads `if (DAT_01165d00 & 0x800) return false;` first, so the
/// energy comparison only runs when that bypass is clear; it then returns
/// `energy < cost`.
pub fn a86_ability_gate(energy_q8: u16, cost_q8: u16, bypass: bool, can_execute: bool) -> bool {
    let blocked = if bypass { false } else { energy_q8 < cost_q8 };
    !blocked && can_execute
}

// ---------------------------------------------------------------------------
// 0x0063ffa0 - one-shot mark
// ---------------------------------------------------------------------------

/// `0x0063ffa0`: the mark state the row reads and updates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A86Mark {
    /// `+0x4d`, the byte compared with the row's `param_2`.
    pub type_byte: u8,
    /// `+0x92`, the accumulated mask.
    pub bits: u8,
}

/// The outcome of one `0x0063ffa0` call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A86MarkOutcome {
    /// The row's return value.
    pub ret: bool,
    /// `true` when `FUN_005b7dd0(value, unit)` is posted.
    pub posted: bool,
}

/// `0x0063ffa0`: the one-shot mark.
///
/// ```c
/// if (unit != 0 && (*(byte *)(unit + 0x92) & mask) == 0) {
///     if (*(byte *)(unit + 0x4d) != value) {
///         FUN_005b7dd0(value, unit);
///         *(byte *)(unit + 0x92) |= mask;
///     }
///     return 1;
/// }
/// return 0;
/// ```
///
/// The bit is only accumulated inside the value-changed branch, which is the
/// non-obvious part: a call whose value already matches returns `1` without
/// touching `+0x92`.  `FUN_005b7dd0` posts the order but writes neither `+0x4d`
/// nor `+0x92`; the `or` in the row is the only store.
pub fn a86_mark_once(unit: Option<&mut A86Mark>, value: u8, mask: u8) -> A86MarkOutcome {
    let Some(unit) = unit else {
        return A86MarkOutcome {
            ret: false,
            posted: false,
        };
    };
    if unit.bits & mask != 0 {
        return A86MarkOutcome {
            ret: false,
            posted: false,
        };
    }
    let posted = unit.type_byte != value;
    if posted {
        unit.bits |= mask;
    }
    A86MarkOutcome { ret: true, posted }
}

// ---------------------------------------------------------------------------
// 0x0063fff0 - owned-slot release guard
// ---------------------------------------------------------------------------

/// `0x0063fff0`: the second argument of the release call (`0x1454`).
pub const A86_RELEASE_CODE: u32 = 0x1454;

/// `0x0063fff0`: the release decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A86Release {
    /// The slot is null; the row returns without calling anything.
    Empty,
    /// The slot is non-null; the row reaches `FUN_00923e90(slot, 0x1454)`.
    Leak(u32),
}

/// `0x0063fff0`: `if (*slot != 0) FUN_00923e90(*slot, 0x1454);`.
///
/// `FUN_00923e90` forwards to the noreturn `FUN_00924220` with the diagnostic
/// descriptor at `0x00d32a20`, so the row's only decision is null versus
/// non-null.
pub fn a86_release_slot(slot: u32) -> A86Release {
    if slot == 0 {
        A86Release::Empty
    } else {
        A86Release::Leak(slot)
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// One row is not translated:
//
// * `FUN_0063ff05` (`FUN_0063ff05`, 79 bytes): the whole body is 34
//   `*in_ECX = *in_ECX | (uint)in_ECX;` stores, four
//   `*unaff_EDI = *unaff_EDI | (uint)in_EAX;` stores, one
//   `*in_EAX = *in_EAX | (uint)in_ECX;` store and `swi(3)`.  Every store ORs a
//   register into the word it is writing, the source and destination alias,
//   and Ghidra reports truncated control flow at the end, so the row is an
//   overlapping-instruction artefact with no arithmetic, table or string
//   behaviour to reproduce.  It stays `decompiled_uncited`.
//
// Four rows in the window share one 112-case dispatch listing
// (`0x0063eb0a`, `0x0063ed3c`, `0x0063efa0`, `0x0063f269`).  Only the
// dispatch facts that are stable across all four copies are translated above;
// the per-opcode call targets are ordinary `CUnit` helpers outside the window
// and are not claimed.

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent86 {
    use super::*;

    #[test]
    fn cookie_indices_split_the_probe_word() {
        let probe = A86_GUARD_PROBE;
        let (low, high) = a86_cookie_indices(probe);
        assert_eq!(low, (probe & 0x0fff) as usize);
        assert_eq!(high, (probe >> 20) as usize);
        assert_eq!(low, 0x69a);
        assert_eq!(high, 0x0f8);
    }

    #[test]
    fn cookie_mix_folds_the_low_half_and_seed() {
        // low = 0x1234, high = 0xabcd, table entry 0x0102.
        let field = 0xabcd_1234;
        let table = 0x0102u16;
        let low = 0x1234u32;
        let mixed = (field >> 16) ^ (table as u32) ^ low ^ low.wrapping_sub(0x93af);
        let expected = ((mixed << 16) | low) ^ A86_GUARD_SEED;
        assert_eq!(a86_cookie_mix(field, table, A86_GUARD_SEED), expected);
        // The low half comes back xor-ed with the seed's own low half.
        assert_eq!(
            a86_cookie_mix(field, table, A86_GUARD_SEED) & 0xffff,
            low ^ (A86_GUARD_SEED & 0xffff)
        );
    }

    #[test]
    fn cookie_mix_uses_the_plain_low_word_twice() {
        // The low half is folded and then re-emitted unchanged.
        let a = a86_cookie_mix(0x0000_0001, 0, 0);
        assert_eq!(a & 0xffff, 1);
        // Changing only the seed changes only the high half.
        let b = a86_cookie_mix(0x0000_0001, 0, 0xffff_ffff);
        assert_eq!(a ^ b, 0xffff_ffff);
    }

    #[test]
    fn cookie_gate_is_clean_only_for_a_zero_low_half() {
        // With low == 0 the comparison reduces to `(field >> 16) == table`.
        let field = 0x1111_0000;
        let table = 0x1111u16;
        assert_eq!(a86_cookie_gate(field, table, 0), A86CookieGate::Clean);
        assert_eq!(a86_cookie_gate(0x2222_0000, table, 0), A86CookieGate::Dirty);
        // A non-zero low half always takes the branch.
        assert_eq!(a86_cookie_gate(field, 0x1111, 0), A86CookieGate::Clean);
        assert_eq!(
            a86_cookie_gate(0x1111_0001, 0x1111, 0),
            A86CookieGate::Dirty
        );
    }

    #[test]
    fn completed_gate_matches_the_bit_four_flag() {
        assert_eq!(a86_completed_flag(0x0000_0010), 1);
        assert_eq!(a86_completed_flag(0x0000_000f), 0);
        assert_eq!(a86_completed_flag(0xffff_ffff), 1);
        assert_eq!(a86_completed_gate(0x10), Ok(1));
        assert_eq!(a86_completed_gate(0x0), Err(A86_CONTINUATION));
    }

    #[test]
    fn entry_tag_compare_is_the_low_half_test() {
        // low = 0x6c, bias 0x112da, so the wrapped sum is 0x11346.
        let biased = A86_ENTRY_TAG.wrapping_add(A86_ENTRY_TAG_BIAS);
        assert_eq!(biased, 0x1_1346);
        let table = 0x0012u16;
        // Pick `high` so that the masked xor equals table ^ 0x6c.
        let wanted = (table as u32) ^ A86_ENTRY_TAG;
        let high = 0xdead_0000 | (wanted ^ (biased & 0xffff)) & 0xffff;
        assert!(!a86_entry_tag_compare(high, table));
        assert!(a86_entry_tag_compare(high ^ 1, table));
    }

    #[test]
    fn entry_tag_guard_always_fires() {
        // The trailing `|| 0x6c != 0` makes the shipped guard a tautology.
        assert!(a86_entry_tag_taken(0, 0));
        assert!(a86_entry_tag_taken(0xffff_ffff, 0xffff));
        assert_eq!(A86_ENTRY_TAG, 0x6c);
    }

    #[test]
    fn units_dat_records_use_the_52_byte_stride() {
        assert_eq!(a86_units_dat_record(0, 0x1000), 0x1000);
        assert_eq!(a86_units_dat_record(1, 0x1000), 0x1034);
        assert_eq!(a86_units_dat_record(3, 0), 0x9c);
        assert_eq!(A86_UNITS_DAT_STRIDE, 0x34);
        assert_eq!(A86_E565_PAYLOAD, 0x1c);
    }

    #[test]
    fn units_dat_top_up_stops_at_the_reloaded_limit() {
        // Four fixed steps of 3 from 10 against a limit of 20.
        let mut calls = 0;
        let value = a86_units_dat_top_up(10, 20, || {
            calls += 1;
            3
        });
        assert_eq!(value, 22);
        assert_eq!(calls, 4);
        // A zero limit never enters the loop.
        assert_eq!(a86_units_dat_top_up(99, 0, || 1), 99);
        // Already at the limit: no step.
        assert_eq!(a86_units_dat_top_up(20, 20, || panic!("no step")), 20);
    }

    #[test]
    fn retry_guard_skips_everything_for_ax_29() {
        assert!(!a86_retry_guard(0x29, 0x9c));
        assert!(a86_retry_guard(0x00, 0x9c));
        assert!(a86_retry_guard(0x2a, 0x02));
        assert!(!a86_retry_guard(0x2a, 0x03));
        assert_eq!(A86_RETRY_COMMAND, 0x0b);
        assert_eq!(A86_RETRY_ORDER, 0x74);
    }

    #[test]
    fn immediate_return_is_a_no_op() {
        a86_returns_immediately();
    }

    #[test]
    fn unit_kind_falls_back_to_the_type_byte() {
        // A clear bit 5 returns the type byte.
        assert_eq!(a86_unit_kind_or_player_flag(0x0000, 0x55), 0x55);
        assert_eq!(a86_unit_kind_or_player_flag(0x00df, 0x55), 0x55);
        // A set bit 5 returns 0x20, not the byte.
        assert_eq!(a86_unit_kind_or_player_flag(0x0020, 0x55), 0x20);
        assert_eq!(a86_unit_kind_or_player_flag(0xffff, 0x00), 0x20);
    }

    #[test]
    fn opcode_window_has_the_0x24_hole() {
        assert!(a86_opcode_valid(0x00));
        assert!(a86_opcode_valid(0x23));
        assert!(!a86_opcode_valid(0x24));
        assert!(a86_opcode_valid(0x25));
        assert!(a86_opcode_valid(0x70));
        assert!(!a86_opcode_valid(0x71));
        assert!(!a86_opcode_valid(0xff));
        assert_eq!(A86_OPCODE_CASES, 112);
        // 0x00..=0x23 is 36 labels and 0x25..=0x70 is 76.
        assert_eq!(0x24 + (0x70 - 0x25 + 1), A86_OPCODE_CASES);
    }

    #[test]
    fn alias_opcodes_share_the_0x3e_body() {
        assert_eq!(a86_opcode_group(0x3e), 0x3e);
        assert_eq!(a86_opcode_group(0x46), 0x3e);
        assert_eq!(a86_opcode_group(0x4c), 0x3e);
        assert_eq!(a86_opcode_group(0x0a), 0x0a);
        assert_eq!(a86_opcode_group(0x70), 0x70);
    }

    #[test]
    fn unit_type_bound_is_0xe4() {
        assert!(a86_unit_type_in_range(0));
        assert!(a86_unit_type_in_range(0xe3));
        assert!(!a86_unit_type_in_range(0xe4));
        assert!(!a86_unit_type_in_range(0xffff));
        assert!(a86_opcode_reads_unit_type(0x0a));
        assert!(a86_opcode_reads_unit_type(0x1a));
        assert!(a86_opcode_reads_unit_type(0x4c));
        assert!(!a86_opcode_reads_unit_type(0x00));
        assert!(!a86_opcode_reads_unit_type(0x24));
        for opcode in A86_UNIT_TYPE_BOUNDED {
            assert!(a86_opcode_valid(opcode), "opcode {opcode:#04x} must exist");
        }
    }

    #[test]
    fn decode_stride_prefers_the_0x500_flag() {
        assert_eq!(a86_decode_stride(0x0500, 0xfd57_f28b), 0x10);
        assert_eq!(a86_decode_stride(0x0100, 0x0000_0000), 0x10);
        assert_eq!(a86_decode_stride(0x0000, 0x0000_0000), 8);
        assert_eq!(a86_decode_stride(0x0000, 0xfd57_f28b), 1);
        assert_eq!(a86_decode_stride(0x00ff, 0x1234_5678), 8);
    }

    #[test]
    fn script_table_base_round_trips() {
        assert_eq!(
            a86_script_table_base(0x00f0_63cc),
            0x00f0_63cc ^ 0x10fa_e560
        );
        assert_eq!(
            a86_script_table_base(a86_script_table_base(0x1234_5678)),
            0x1234_5678
        );
    }

    #[test]
    fn stride_3c_bound_matches_a_60_byte_record() {
        assert!(a86_stride_3c_fits(0, 0));
        assert!(a86_stride_3c_fits(1, 0x3c));
        assert!(!a86_stride_3c_fits(1, 0x3b));
        assert!(a86_stride_3c_fits(0xff, 0xff * 0x3c));
        assert!(!a86_stride_3c_fits(0xff, 0xff * 0x3c - 1));
    }

    #[test]
    fn fnv1a_matches_the_published_vectors() {
        assert_eq!(a86_fnv1a32(b""), 0x811c_9dc5);
        assert_eq!(a86_fnv1a32(b"a"), 0xe40c_292c);
        assert_eq!(a86_fnv1a32(b"foobar"), 0xbf9c_f968);
        // The recurrence is byte-oriented, not word-oriented.
        assert_eq!(
            a86_fnv1a32(b"ab"),
            (b'b' as u32 ^ (b'a' as u32 ^ 0x811c_9dc5).wrapping_mul(0x0100_0193))
                .wrapping_mul(0x0100_0193)
        );
    }

    #[test]
    fn pool_count_and_tag_arithmetic() {
        let key = 0x1122_3344;
        assert_eq!(
            a86_pool_count(0x0000_00ff, key),
            0x00ff ^ (key + 0x4163_f040)
        );
        // A tag is valid exactly when it equals key ^ tag.
        let tag = key ^ A86_POOL_TAG;
        assert!(a86_pool_tag_matches(tag, key));
        assert!(!a86_pool_tag_matches(tag ^ 1, key));
        // With a zero key the bare tag constant is the valid value.
        assert!(a86_pool_tag_matches(A86_POOL_TAG, 0));
        assert!(!a86_pool_tag_matches(0, 0));
        assert_eq!(a86_pool_not_found(key), key.wrapping_add(0x4163_f040));
    }

    #[test]
    fn pool_ring_wraps_and_rejects_zero_count() {
        assert_eq!(a86_pool_ring_next(0, 4), Some(1));
        assert_eq!(a86_pool_ring_next(3, 4), Some(0));
        assert_eq!(a86_pool_ring_next(0, 1), Some(0));
        assert_eq!(a86_pool_ring_next(u32::MAX, 0), None);
        assert_eq!(a86_pool_pair_index(3, 5), 15);
        assert_eq!(a86_pool_pair_index(u32::MAX, 2), 0xffff_fffe);
    }

    #[test]
    fn udivmod64_is_unsigned_and_sign_extends_the_divisor() {
        assert_eq!(a86_udivmod64(100, 7), Some((14, 2)));
        assert_eq!(a86_udivmod64(u64::MAX, 2), Some((u64::MAX / 2, 1)));
        assert_eq!(a86_udivmod64(1, 0), None);
        // 0x83b ^ 0x82b folds to 0x10.
        assert_eq!(A86_F7B0_SECOND, 0x10);
        assert_eq!(a86_f7b0_divisor(0x083b), 0x10);
        assert_eq!(a86_f7b0_divisor(0), 0x082b);
        // A word whose masked form has bit 31 set becomes a huge unsigned
        // divisor, which is the whole point of the sign extension.
        assert_eq!(a86_f7b0_divisor(0xffff_ffff), 0xffff_ffff_ffff_f7d4);
    }

    #[test]
    fn orphan_sweep_steps_by_four_and_reports_the_bias() {
        assert_eq!(A86_ORPHAN_BIAS, 0x1000);
        assert_eq!(A86_ORPHAN_FAIL, 0xff);
        let slots = [0u32; A86_ORPHAN_SLOTS];
        // len == 0 takes the early out.
        assert_eq!(a86_orphan_sweep(0x1000, 0, &slots), Ok(0));
        // 12 bytes of range is three probes at +0, +4, +8.
        assert_eq!(a86_orphan_sweep(0x1000, 12, &slots), Ok(3));
        // The first probe collides.
        let hit = [0x2000u32, 0, 0, 0, 0, 0];
        assert_eq!(a86_orphan_sweep(0x2000, 12, &hit), Err(0xff));
        // A collision on the third probe still reports the same code.
        let mut late = [0u32; A86_ORPHAN_SLOTS];
        late[5] = 0x2008;
        assert_eq!(a86_orphan_sweep(0x2000, 12, &late), Err(0xff));
        // len is compared as an unsigned word, so a "negative" length runs.
        assert_eq!(a86_orphan_len(0x1000, 0x2000), -0x2000);
        assert_eq!(a86_orphan_len(0x0000_0010, 0), 0x10 - 0x1000);
    }

    #[test]
    fn orphan_sweep_from_uses_the_plus_one_probe() {
        let slots = [0u32; A86_ORPHAN_SLOTS];
        // start = 0, len = 3: probes are first+1 (index 0), then 0 + 4 + 1 = 5
        // which is >= len so the walk stops after one probe.
        assert_eq!(a86_orphan_sweep_from(0, 3, 0x8000, &slots), Ok(1));
        // start = 0, len = 10: probes at +1, +5 and +9; the next counter is
        // 12, so `len <= next` ends the walk after three probes.
        assert_eq!(a86_orphan_sweep_from(0, 10, 0x8000, &slots), Ok(3));
        // The colliding probe is the third one, first + 9.
        let mut hit = [0u32; A86_ORPHAN_SLOTS];
        hit[2] = 0x8009;
        assert_eq!(a86_orphan_sweep_from(0, 10, 0x8000, &hit), Err(0xff));
        // start past the length: no probes at all.
        assert_eq!(a86_orphan_sweep_from(3, 3, 0x8000, &slots), Ok(0));
    }

    #[test]
    fn target_list_walk_reports_the_first_missing_target() {
        let ok = A86ListUnit {
            gone: false,
            hidden: false,
            matched: true,
            has_target: true,
        };
        assert_eq!(a86_validate_target_list(&[]), A86ListVerdict::Exhausted);
        assert_eq!(
            a86_validate_target_list(&[ok, ok]),
            A86ListVerdict::Exhausted
        );
        // A matching record without a target stops the walk.
        let missing = A86ListUnit {
            has_target: false,
            ..ok
        };
        assert_eq!(
            a86_validate_target_list(&[ok, missing, ok]),
            A86ListVerdict::TargetMissing(1)
        );
        // Skipped records never produce the failure.
        let skipped = A86ListUnit {
            hidden: true,
            has_target: false,
            ..ok
        };
        assert_eq!(
            a86_validate_target_list(&[skipped]),
            A86ListVerdict::Exhausted
        );
        let gone = A86ListUnit {
            gone: true,
            ..skipped
        };
        assert_eq!(a86_validate_target_list(&[gone]), A86ListVerdict::Exhausted);
    }

    #[test]
    fn tile_world_drops_the_subpixel_fraction() {
        let tile = A86Tile {
            x_word: 0x1234_5600,
            y_word: -0x0000_0100,
        };
        assert_eq!(a86_tile_world(&tile), (0x1234_56, -1));
        assert_eq!(a86_tile_world(&A86Tile::default()), (0, 0));
    }

    #[test]
    fn chunk_and_subtile_indices() {
        assert_eq!(a86_chunk_index(0, 0), 0);
        assert_eq!(a86_chunk_index(31, 0), 0);
        assert_eq!(a86_chunk_index(32, 0), 1);
        assert_eq!(a86_chunk_index(0, 32), 0x100);
        // The `>> 31 & 0x1f` bias makes -1 land in chunk 0, not chunk -1.
        assert_eq!(a86_chunk_index(-1, 0), 0);
        // -32 is the first negative coordinate that floors to -1.
        assert_eq!(a86_chunk_index(-32, 0), 0xffff_ffff);
        assert_eq!(a86_chunk_index(-32, -32), 0xffff_feff);
        assert_eq!(a86_subtile_bit(0, 0), 0);
        assert_eq!(a86_subtile_bit(8, 0), 1);
        assert_eq!(a86_subtile_bit(0, 8), 4);
        assert_eq!(a86_subtile_bit(24, 24), 15);
        assert_eq!(a86_subtile_bit(32, 32), 0);
    }

    #[test]
    fn region_lookup_uses_the_split_bitmap() {
        let mut chunks = vec![0u16; 0x100];
        chunks[0] = 7;
        chunks[1] = 0x2000;
        // The split table is indexed by the raw chunk word, so entry 0x2000.
        let mut splits = vec![A86RegionSplit::default(); 0x2001];
        splits[0x2000] = A86RegionSplit {
            mask: 1 << 3,
            outside: 0x0011,
            inside: 0x0022,
        };
        // Plain chunk id.
        assert_eq!(a86_region_at(&chunks, &splits, 0, 0), Some(7));
        // Split with sub-tile bit 0 clear.
        assert_eq!(a86_region_at(&chunks, &splits, 32, 0), Some(0x0011));
        // x = 56 puts the point in sub-tile column 3, so bit 3 is set.
        assert_eq!(a86_region_at(&chunks, &splits, 56, 0), Some(0x0022));
        // y = 8 moves the bit to the second row, which the mask omits.
        assert_eq!(a86_region_at(&chunks, &splits, 32, 8), Some(0x0011));
        // Out-of-range chunk reports None rather than reading past the table.
        assert_eq!(a86_region_at(&chunks, &splits, 32 * 0x100, 0), None);
        assert_eq!(A86_REGION_SPLIT_MIN, 0x2000);
    }

    #[test]
    fn region_unit_query_returns_the_row_sentinels() {
        let live = A86CandidateUnit {
            gone: false,
            active: true,
            matched: true,
        };
        assert_eq!(a86_region_unit_query(&[]), None);
        assert_eq!(a86_region_unit_return(&[]), A86_NO_UNIT);
        assert_eq!(a86_point_unit_return(&[]), A86_NO_POINT_UNIT);
        assert_eq!(A86_NO_UNIT, 0xffff_ffff);
        assert_eq!(A86_NO_POINT_UNIT, 0);
        assert_eq!(A86_BEST_DISTANCE_INIT, 99_999_999);
        assert_eq!(A86_BEST_UNIT_INIT, 0);
        assert_eq!(a86_region_unit_query(&[live]), Some(0));
        // An inactive record is skipped even when it matches.
        let inactive = A86CandidateUnit {
            active: false,
            ..live
        };
        assert_eq!(a86_region_unit_query(&[inactive, live]), Some(1));
        // A record that is gone is skipped as well.
        let gone = A86CandidateUnit { gone: true, ..live };
        assert_eq!(a86_region_unit_return(&[gone, inactive]), A86_NO_UNIT);
    }

    #[test]
    fn area_mark_skips_the_owning_player() {
        assert_eq!(a86_area_mark_slot(3, 3), None);
        // (player + 0x131b) * 0xc + owner
        assert_eq!(a86_area_mark_slot(0, 1), Some(0x131b * 0x0c + 1));
        assert_eq!(a86_area_mark_slot(1, 0), Some(0x131c * 0x0c));
        assert_eq!(A86_AREA_MARK_VALUE, 2);
    }

    #[test]
    fn lift_transition_opens_only_outside_the_type_window() {
        let base = A86LiftFacts {
            state: A86_LIFT_STATE_UP,
            flags: A86_LIFT_FLAG,
            pred_a: false,
            pred_b: false,
            pred_c: false,
            unit_type: 0x00,
        };
        assert_eq!(a86_lift_transition(&base), Some(0x6e));
        // State must be 'm'.
        assert_eq!(
            a86_lift_transition(&A86LiftFacts {
                state: 0x00,
                ..base
            }),
            None
        );
        // The 0x200 flag is required.
        assert_eq!(
            a86_lift_transition(&A86LiftFacts {
                flags: 0x100,
                ..base
            }),
            None
        );
        for pred in 0..3 {
            let mut facts = base;
            match pred {
                0 => facts.pred_a = true,
                1 => facts.pred_b = true,
                _ => facts.pred_c = true,
            }
            assert_eq!(a86_lift_transition(&facts), None);
        }
        // The excluded type window is inclusive at both ends.
        assert_eq!(
            a86_lift_transition(&A86LiftFacts {
                unit_type: 0x7b,
                ..base
            }),
            None
        );
        assert_eq!(
            a86_lift_transition(&A86LiftFacts {
                unit_type: 0x81,
                ..base
            }),
            None
        );
        assert_eq!(
            a86_lift_transition(&A86LiftFacts {
                unit_type: 0x7a,
                ..base
            }),
            Some(0x6e)
        );
        assert_eq!(
            a86_lift_transition(&A86LiftFacts {
                unit_type: 0x82,
                ..base
            }),
            Some(0x6e)
        );
        assert_eq!(A86_LIFT_CLEARED_FIELDS, [0xe8, 0xec, 0xe2]);
    }

    #[test]
    fn dispatch_handles_the_default_and_fatal_cases() {
        let input = A86DispatchInput {
            ax: 0x00,
            ..A86DispatchInput::default()
        };
        assert_eq!(a86_command_dispatch(&input, &[]), A86Dispatch::Ignored);
        let fatal = A86DispatchInput {
            ax: 0x47,
            ..A86DispatchInput::default()
        };
        assert_eq!(a86_command_dispatch(&fatal, &[]), A86Dispatch::Fatal);
        // The four order-0x12 messages.
        for ax in [0x7c, 0x90, 0x92, 0xa2] {
            let input = A86DispatchInput {
                ax,
                ..A86DispatchInput::default()
            };
            assert_eq!(
                a86_command_dispatch(&input, &[]),
                A86Dispatch::Posted(A86DispatchResult {
                    ret: 1,
                    action: A86Action::Order(0x12),
                })
            );
        }
    }

    #[test]
    fn dispatch_case_02_and_05_gates() {
        let mut input = A86DispatchInput {
            ax: 0x02,
            x: 0x10,
            y: 0x20,
            can_3: true,
            pred_510c0: true,
            ..A86DispatchInput::default()
        };
        assert_eq!(
            a86_command_dispatch(&input, &[]),
            A86Dispatch::Posted(A86DispatchResult {
                ret: 1,
                action: A86Action::Message {
                    message: 0x84,
                    x: 0x10,
                    y: 0x20,
                },
            })
        );
        input.can_3 = false;
        assert_eq!(a86_command_dispatch(&input, &[]), A86Dispatch::Ignored);

        let mut five = A86DispatchInput {
            ax: 0x05,
            target_kind_1: true,
            can_5: true,
            ..A86DispatchInput::default()
        };
        assert_eq!(
            a86_command_dispatch(&five, &[]),
            A86Dispatch::Posted(A86DispatchResult {
                ret: 1,
                action: A86Action::Order(0x62),
            })
        );
        // A non-zero +0x74 or a positive FUN_00659030 both block the order.
        five.field_74 = 1;
        assert_eq!(a86_command_dispatch(&five, &[]), A86Dispatch::Ignored);
        five.field_74 = 0;
        five.pred_59030 = true;
        assert_eq!(a86_command_dispatch(&five, &[]), A86Dispatch::Ignored);
    }

    #[test]
    fn dispatch_case_0b_falls_through_to_the_45_group() {
        let mut input = A86DispatchInput {
            ax: 0x0b,
            cmp_a: 7,
            cmp_b: 9,
            c7850: 0x5a,
            ..A86DispatchInput::default()
        };
        // Without FUN_005d5d10 the typed order is posted.
        assert_eq!(
            a86_command_dispatch(&input, &[]),
            A86Dispatch::Posted(A86DispatchResult {
                ret: 1,
                action: A86Action::OrderTyped {
                    order: 0x31,
                    operand: 0x5a,
                },
            })
        );
        // With it, the plain order wins.
        input.pred_d5d10 = true;
        assert_eq!(
            a86_command_dispatch(&input, &[]),
            A86Dispatch::Posted(A86DispatchResult {
                ret: 1,
                action: A86Action::Order(0x6f),
            })
        );
        // Equal ids skip the case body and fall into 0x45.
        input.cmp_a = 9;
        input.pred_560f0 = true;
        assert_eq!(
            a86_command_dispatch(&input, &[]),
            A86Dispatch::Posted(A86DispatchResult {
                ret: 1,
                action: A86Action::Nothing,
            })
        );
        input.pred_560f0 = false;
        assert_eq!(
            a86_command_dispatch(&input, &[]),
            A86Dispatch::Posted(A86DispatchResult {
                ret: 1,
                action: A86Action::Order(0x6f),
            })
        );
        input.pred_d5d10 = false;
        assert_eq!(a86_command_dispatch(&input, &[]), A86Dispatch::Ignored);
    }

    #[test]
    fn dispatch_case_48_posts_and_returns_zero() {
        let input = A86DispatchInput {
            ax: 0x48,
            msg_kind: 0x49,
            can_msg: true,
            pred_e040: true,
            ..A86DispatchInput::default()
        };
        assert_eq!(
            a86_command_dispatch(&input, &[]),
            A86Dispatch::Posted(A86DispatchResult {
                ret: 0,
                action: A86Action::Post(0x3f),
            })
        );
        let blocked = A86DispatchInput {
            pred_90f50: true,
            ..input
        };
        assert_eq!(a86_command_dispatch(&blocked, &[]), A86Dispatch::Ignored);
        assert_eq!(
            a86_command_dispatch(&A86DispatchInput { ax: 0x53, ..input }, &[]),
            A86Dispatch::Posted(A86DispatchResult {
                ret: 0,
                action: A86Action::Post(0x3f)
            })
        );
    }

    #[test]
    fn dispatch_case_ac_walks_only_for_big_state_bytes() {
        let low = A86DispatchInput {
            ax: 0xac,
            state_a3: 0x31,
            ..A86DispatchInput::default()
        };
        assert_eq!(a86_command_dispatch(&low, &[]), A86Dispatch::Ignored);
        let high = A86DispatchInput {
            state_a3: 0x32,
            ..low
        };
        // An empty list runs off the end and the row returns 0 unposted.
        assert_eq!(a86_command_dispatch(&high, &[]), A86Dispatch::Ignored);
        // An entry that fails the continue test stops the walk.
        let stop = [A86AcEntry::default()];
        assert_eq!(a86_command_dispatch(&high, &stop), A86Dispatch::NotFound);
        // Any of the four continue terms keeps the walk going.
        for entry in [
            A86AcEntry {
                pred_a_clear: true,
                ..A86AcEntry::default()
            },
            A86AcEntry {
                pred_b_set: true,
                ..A86AcEntry::default()
            },
            A86AcEntry {
                pred_c_set: true,
                ..A86AcEntry::default()
            },
            A86AcEntry {
                value: 0x29,
                ..A86AcEntry::default()
            },
        ] {
            assert!(a86_ac_continues(&entry));
            assert_eq!(a86_command_dispatch(&high, &[entry]), A86Dispatch::Ignored);
        }
        // `0x28` is the last value inside the range: `0x28 < d` is false.
        assert!(!a86_ac_continues(&A86AcEntry {
            value: 0x28,
            ..A86AcEntry::default()
        }));
    }

    #[test]
    fn voice_word_uses_the_index_table_and_shift() {
        let index = [3u8, 0x2c, 0x2b, 0];
        let words: Vec<i16> = (0..0x2c).map(|i| i as i16).collect();
        assert_eq!(a86_voice_word(&index, &words, 0), 3 << 8);
        // 0x2c is the first index the row rejects, so the result is zero.
        assert_eq!(a86_voice_word(&index, &words, 1), 0);
        // 0x2b is the last accepted index.
        assert_eq!(a86_voice_word(&index, &words, 2), 0x2b00i16);
        assert_eq!(a86_voice_word(&index, &words, 3), 0);
        // A missing player also yields zero.
        assert_eq!(a86_voice_word(&index, &words, 9), 0);
    }

    #[test]
    fn ability_gate_combines_energy_and_the_command_check() {
        // Enough energy, command allowed.
        assert!(a86_ability_gate(100, 50, false, true));
        // Not enough energy.
        assert!(!a86_ability_gate(49, 50, false, true));
        // The 0x800 bypass skips the energy test.
        assert!(a86_ability_gate(0, 50, true, true));
        // The command check still applies.
        assert!(!a86_ability_gate(100, 50, false, false));
        assert_eq!(A86_ENERGY_BYPASS, 0x0800);
    }

    #[test]
    fn mark_once_tracks_the_bit_and_the_value() {
        let mut unit = A86Mark {
            type_byte: 0x10,
            bits: 0,
        };
        // First call changes the value and accumulates the bit.
        let first = a86_mark_once(Some(&mut unit), 0x20, 0x04);
        assert!(first.ret);
        assert!(first.posted);
        // `FUN_005b7dd0` posts the order; it does not write `+0x4d`.
        assert_eq!(unit.type_byte, 0x10);
        assert_eq!(unit.bits, 0x04);
        // Second call is a no-op because the bit is already set.
        let second = a86_mark_once(Some(&mut unit), 0x30, 0x04);
        assert!(!second.ret);
        assert!(!second.posted);
        assert_eq!(unit.bits, 0x04);
        // Same value with a fresh mask returns 1 without posting.
        let third = a86_mark_once(Some(&mut unit), 0x10, 0x08);
        assert!(third.ret);
        assert!(!third.posted);
        assert_eq!(unit.bits, 0x04);
        // A different value with a fresh mask posts and sets the bit.
        let fourth = a86_mark_once(Some(&mut unit), 0x40, 0x08);
        assert!(fourth.ret);
        assert!(fourth.posted);
        assert_eq!(unit.bits, 0x0c);
        // A null unit returns 0.
        let none = a86_mark_once(None, 0x20, 0x08);
        assert!(!none.ret);
        assert!(!none.posted);
    }

    #[test]
    fn release_slot_distinguishes_null() {
        assert_eq!(a86_release_slot(0), A86Release::Empty);
        assert_eq!(a86_release_slot(0x1234), A86Release::Leak(0x1234));
        assert_eq!(A86_RELEASE_CODE, 0x1454);
    }
}
