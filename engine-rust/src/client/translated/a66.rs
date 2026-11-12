#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent66.rs
// ===========================================================================
// Translation batch 66.  Only this file is edited by the batch.
//
// Batch 66 owns `0x00616000..=0x00617fff`.  The ledger lists **12** rows in
// that window, all `decompiled_uncited`, and all 12 are covered below.  The
// next row after `0x00617c69` is `FUN_0061803b` (batch 67), and the row before
// `0x00616007` is `FUN_00615ee3` (batch 65), so 15 rows cannot be reached
// without leaving the window; every row present is translated.
//
// | address      | recovered role                                            |
// |--------------|-----------------------------------------------------------|
// | `0x00616007` | protector constant decode + 64x32 multiply/shift sequence  |
// | `0x006168d5` | notify descriptor, record-kind 8 "extended" path           |
// | `0x00616b3e` | tagged entry pointer + x87/SSE conversion selector         |
// | `0x00616b5c` | notify descriptor, generic path                            |
// | `0x00616db7` | 0x1c-byte descriptor gate/copy                             |
// | `0x006170d0` | owner slot clear and derived-byte refresh                  |
// | `0x0061729a` | mode/seed table entry + string slot layout + loop guard    |
// | `0x00617551` | ISA unroll stride + zero-skipping entry scan               |
// | `0x00617600` | recursive zero-skip variant of the same entry scan         |
// | `0x00617690` | `rol 21` / `/198` and the 9-7-8 init record                |
// | `0x00617773` | entry decode, effective count, 500-frame notify throttle    |
// | `0x00617c69` | byte `divmod`                                              |
//
// ## What is recoverable, and what is not
//
// Every body in this window is protected code: the cached decompilations open
// with `WARNING: Control flow encountered bad instruction data`, contain
// `rdtsc`/`swi`/`halt_baddata` filler, and in four rows the decompiler folds a
// shared out-of-window blob (`0x0061ff48`, `0x00628e50`, `0x00639427`) into the
// listing.  Those merged regions belong to other functions and are *not*
// translated here.
//
// The surviving arithmetic is nevertheless exact and is what this module
// reproduces:
//
// * a **constant decoder** shared by all 12 rows.  Every immediate in this
//   window is encoded; `0x00616007` shows the full chain
//   `v -= 0x7b0; v -= 0x12d8; v ^= 0xf5f; v += 0x576`, and the same chain
//   turns `0x105d` into `0`, `0x1085` into `24`, `0x205d` into `0x1000`.
//   [`a66_decode_const`] is that chain; `0x7ffdfc43 + 0x3bd` is the short
//   two-step form and lands on the same `0x7ffe0000` base.
// * the **word unmangler** used by `0x00616b3e`, `0x00616b5c`,
//   `0x00616db7`, `0x00617551`, `0x00617600` and `0x00617773`
//   ([`a66_unmangle_word`]) together with its two checksum forms
//   ([`a66_check_word_93af`], [`a66_check_word_112da`]).
// * the **notify descriptor** stores shared by `0x006168d5`, `0x00616b3e`,
//   `0x00616b5c` and `0x00616db7`: id word at `+0x00`, decoded class word at
//   `+0x02`, type `2` at `+0x10`, user argument at `+0x18`; the kind-8 path
//   instead writes `0xf` at `+0x08` and ORs `0x100` into the class word when
//   bit `0x40000000` is set ([`a66_notify`]).
// * the **entry scan** shared by `0x00617551`, `0x00617600` and
//   `0x00617773`: effective count `count ^ (global + 0x4163f040)`, next index
//   `(index + 1) % count`, emptiness encoded as `word == key ^ 0xdce84bd8`,
//   and a `processed` counter that terminates the scan
//   ([`a66_scan_entries`]).
// * the **notify throttle** of `0x00617773`: the same target within 500
//   frames rewinds the stamp to `now - 500`.
//
// Not recovered: the indirect thunks (`FUN_00538810` throw, `FUN_009736b0`
// registry lookup, `FUN_006145d0`/`FUN_00614620` class-word decode,
// `FUN_005a3180` object test) and the virtual slots at `+0x10`/`+0x04`.
// Where a row's only remaining work is such a call, the translation models
// the descriptor/gate that the call receives and returns the *decision*,
// never a fabricated callee.

// ---------------------------------------------------------------------------
// Protector constant decoder
// ---------------------------------------------------------------------------

/// First step of the decode chain (`-0x7b0`).
pub const A66_CONST_SUB_A: u32 = 0x7b0;
/// Second step (`-0x12d8`).
pub const A66_CONST_SUB_B: u32 = 0x12d8;
/// Third step (`^0xf5f`).
pub const A66_CONST_XOR: u32 = 0xf5f;
/// Fourth step (`+0x576`).
pub const A66_CONST_ADD: u32 = 0x576;
/// Short-form addend that turns `0x7ffdfc43` into `0x7ffe0000`.
pub const A66_CONST_SHORT_ADD: u32 = 0x3bd;
/// Base all frame-relative encodings resolve to (`0x7ffe0000`).
pub const A66_FRAME_BASE: u32 = 0x7ffe_0000;

/// Decode one encoded immediate.
///
/// ```text
/// v = enc
/// v -= 0x7b0
/// v -= 0x12d8
/// v ^= 0xf5f
/// v += 0x576
/// ```
///
/// `0x00616007` applies this to `0x7ffe105d` (→ `0x7ffe0000`), `0x105d` (→ 0),
/// `0x1085` (→ 24), `0xd7d` (→ 800), `0xbd9` (→ 900) and `0x1039` (→ 100).
/// `0x0061729a` reuses it for `0xde5`/`0xde1`/`0xdf9`/`0x1165`/`0xe45`
/// (376/380/420/-264/600).
pub fn a66_decode_const(enc: u32) -> u32 {
    let v = enc.wrapping_sub(A66_CONST_SUB_A);
    let v = v.wrapping_sub(A66_CONST_SUB_B);
    let v = v ^ A66_CONST_XOR;
    v.wrapping_add(A66_CONST_ADD)
}

/// Exact inverse of [`a66_decode_const`].
pub fn a66_encode_const(value: u32) -> u32 {
    let v = value.wrapping_sub(A66_CONST_ADD) ^ A66_CONST_XOR;
    v.wrapping_add(A66_CONST_SUB_B)
        .wrapping_add(A66_CONST_SUB_A)
}

/// Short two-step decode used at `0x00616007` for the frame base
/// (`0x7ffdfc43 + 0x3bd == 0x7ffe0000`).
pub fn a66_decode_const_short(enc: u32) -> u32 {
    enc.wrapping_add(A66_CONST_SHORT_ADD)
}

// ---------------------------------------------------------------------------
// Flag tests and multiply sequences
// ---------------------------------------------------------------------------

/// The byte-sized "wrapped or zero" test at the head of `0x00616007`:
/// `sum == 0 || SCARRY1(a, b) != (sum < 0)`.
///
/// `SCARRY1` is the x86 signed-overflow flag of the 8-bit add, so the test is
/// true when the sum is zero or when the overflow flag and the sign of the sum
/// disagree.
pub fn a66_i8_add_wraps_or_zero(a: i8, b: i8) -> bool {
    let sum = a.wrapping_add(b);
    let overflow = a.checked_add(b).is_none();
    sum == 0 || overflow != (sum < 0)
}

/// Dword twin of [`a66_i8_add_wraps_or_zero`], from the head of
/// `0x00617773` (`*p += (int)p; if (*p == 0 || SCARRY4(old, p) != (*p < 0))`).
pub fn a66_u32_add_wraps_or_zero(a: u32, b: u32) -> bool {
    let sum = a.wrapping_add(b);
    // `SCARRY4` is the signed-overflow flag, not the carry out of bit 31.
    let overflow = (a as i32).checked_add(b as i32).is_none();
    sum == 0 || overflow != ((sum as i32) < 0)
}

/// `(a * b) >> shift` with the 64-bit product of `0x00616007`.
///
/// `0x00616007` keeps the low half at `ebp-0x1c`, the high half at `ebp-0x18`
/// and shifts the qword by the decoded count `24`
/// (`0x00616007` uses the decoded `0x1085`).
pub fn a66_mul_shift_u64(a: u32, b: u32, shift: u32) -> u64 {
    ((u64::from(a) * u64::from(b)) >> shift) as u64
}

/// The wide branch of the same multiply: a 0x68-stride entry supplies a
/// `(low, high)` dword pair, and the scale multiplies both halves.
///
/// ```text
/// product = (u64)low * scale
/// out_lo  = (u32)product
/// out_hi  = high * scale + (u32)(product >> 32)
/// ```
pub fn a66_mul_acc_64x32(low: u32, high: u32, scale: u32) -> (u32, u32) {
    let product = u64::from(low) * u64::from(scale);
    let out_lo = product as u32;
    let out_hi = high
        .wrapping_mul(scale)
        .wrapping_add((product >> 32) as u32);
    (out_lo, out_hi)
}

// ---------------------------------------------------------------------------
// Word unmangler
// ---------------------------------------------------------------------------

/// `0xffff_fe00`: `-0x200` as the unmangler's second table multiplier.
pub const A66_MANGLE_TABLE_SCALE: u32 = 0xffff_fe00;
/// `0x93af`: the low-word bias subtracted before the high word is folded in.
pub const A66_MANGLE_BIAS: u32 = 0x93af;
/// Bias of the alternate checksum (`0x006168d5`: `lo + 0x112da`).
pub const A66_CHECK_BIAS: u32 = 0x112da;

/// Rebuild one mangled dword.
///
/// `0x00616b5c` and `0x00616db7` compute
///
/// ```text
/// lo  = x & 0xffff
/// mid = ((table_line ^ x ^ (y >> 16)) << 16 | x) >> 16
/// w   = ((lo - 0x93af ^ mid) << 16 | lo) ^ key
/// ```
///
/// with `x` the mangled source, `y` the second dword of the pair, `table_line`
/// the `DAT_00f7ef98` line selected by the low 12 bits of the stack key and
/// `key` the stack-resident xor pad.
pub fn a66_unmangle_word(x: u32, y: u32, table_line: u16, key: u32) -> u32 {
    let lo = x & 0xffff;
    let mid = ((u32::from(table_line) ^ x ^ (y >> 16)) << 16 | x) >> 16;
    let hi = lo.wrapping_sub(A66_MANGLE_BIAS) ^ mid;
    (hi << 16 | lo) ^ key
}

/// Checksum form A, used after every unmangle in `0x00616b3e`, `0x00616b5c`
/// and `0x00616db7`:
///
/// ```text
/// lo = w & 0xffff
/// (table_lo ^ lo) == ((lo * 2 + table_hi * -0x200 ^ (w >> 16)) & 0xffff)
///     && lo == 0
/// ```
pub fn a66_check_word_93af(w: u32, table_lo: u16, table_hi: u16) -> bool {
    let lo = w & 0xffff;
    let lhs = u32::from(table_lo) ^ lo;
    let rhs = (lo
        .wrapping_mul(2)
        .wrapping_add(u32::from(table_hi).wrapping_mul(A66_MANGLE_TABLE_SCALE))
        ^ (w >> 16))
        & 0xffff;
    lhs == rhs && lo == 0
}

/// Checksum form B, the first validation in `0x006168d5`:
///
/// ```text
/// lo = w & 0xffff
/// ((w >> 16 ^ lo + 0x112da) & 0xffff) == (lo ^ table_lo) && lo == 0
/// ```
pub fn a66_check_word_112da(w: u32, table_lo: u16) -> bool {
    let lo = w & 0xffff;
    ((w >> 16 ^ lo.wrapping_add(A66_CHECK_BIAS)) & 0xffff) == (lo ^ u32::from(table_lo)) && lo == 0
}

// ---------------------------------------------------------------------------
// Notify descriptors: 0x006168d5, 0x00616b3e, 0x00616b5c, 0x00616db7
// ---------------------------------------------------------------------------

/// `+0x08` of the kind-8 ("extended") descriptor.
pub const A66_NOTIFY_EXTENDED_MODE: u16 = 0xf;
/// Bit that selects the alternate callback at `_DAT_01176d94` instead of
/// `_DAT_01176d90`.
pub const A66_NOTIFY_BIT30: u32 = 0x4000_0000;
/// Bit ORed into the class word of the extended descriptor when
/// [`A66_NOTIFY_BIT30`] is set.
pub const A66_NOTIFY_WORD_FLAG: u16 = 0x100;
/// Descriptor type written at `+0x10` by the generic path.
pub const A66_DESC_TYPE: u16 = 2;
/// Descriptor size copied by `0x00616db7` (`0x1c` bytes, seven dwords).
pub const A66_DESC_BYTES: usize = 0x1c;
/// Record kind that takes the extended path (`*(ebp-0xc4) == 8`).
pub const A66_RECORD_KIND_EXTENDED: i32 = 8;

/// The three observable fields of the notify descriptor built by
/// `0x006168d5`/`0x00616b3e`/`0x00616b5c`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A66Notify {
    /// `+0x00`: low 16 bits of the id argument.
    pub id_low: u16,
    /// `+0x02`: class word returned by the decode call, ORed with `0x100`
    /// when bit `0x40000000` of the id is set.
    pub word: u16,
    /// Mode word: `0xf` for record kind 8, otherwise `1` when bit
    /// `0x40000000` is set and `0` otherwise.
    pub mode: u16,
    /// `true` when the extended (kind 8) path built the descriptor.
    pub extended: bool,
}

/// Build the notify descriptor.
///
/// `0x006168d5` first calls `FUN_005614b0`; when that returns non-zero it calls
/// `FUN_006145d0(id)` and then branches on `*(ebp-0xc4) == 8`:
///
/// * kind 8: store `id`, then `0xf` at `+0x08`, and `decoded | 0x100` at
///   `+0x02` when `id & 0x40000000`.  If the registry lookup
///   (`FUN_009736b0`) fails, `_DAT_01176dcc` is invoked as the fallback.
/// * otherwise: store `id`, `decoded`, and mode `1` (bit 30 set) or `0`; the
///   registry lookup then falls back to `_DAT_01176d94` / `_DAT_01176d90`.
///
/// `decoded` is the value produced by the class-word decode (`FUN_00614620`);
/// the callee itself is not part of this window, so the caller supplies it.
pub fn a66_notify(id: u32, record_kind: i32, decoded: u16) -> A66Notify {
    let bit30 = id & A66_NOTIFY_BIT30 != 0;
    let word = if bit30 {
        decoded | A66_NOTIFY_WORD_FLAG
    } else {
        decoded
    };
    let extended = record_kind == A66_RECORD_KIND_EXTENDED;
    let mode = if extended {
        A66_NOTIFY_EXTENDED_MODE
    } else if bit30 {
        1
    } else {
        0
    };
    A66Notify {
        id_low: id as u16,
        word,
        mode,
        extended,
    }
}

/// Tagged entry pointers: `0x00616b3e` unmasks a low-bit tag with
/// `*puVar11 & 0xfffffffe` before the virtual call.
pub fn a66_clear_tag(pointer: u32) -> u32 {
    pointer & 0xffff_fffe
}

/// `0x00616b3e` selects the floating-point conversion helper by the sign of
/// `(char)(in_EAX - 0x51cb0958)`: `FUN_00c87160` for negative, `FUN_00c87120`
/// otherwise.
pub const A66_CONVERT_X87: u32 = 0x00c8_7160;
/// Non-negative branch helper of `0x00616b3e`.
pub const A66_CONVERT_SSE: u32 = 0x00c8_7120;

/// Which conversion helper `0x00616b3e` dispatches to.
pub fn a66_convert_helper(negative: bool) -> u32 {
    if negative {
        A66_CONVERT_X87
    } else {
        A66_CONVERT_SSE
    }
}

/// `0x00616b3e`'s entry bias (`in_EDX + 0xb665f9b9`) and the
/// `0x499a0646` bound it is compared against.
pub const A66_ENTRY_BIAS_3E: u32 = 0xb665_f9b9;
/// Inclusive-exclusive bound of the `0x00616b3e` bias test.
pub const A66_ENTRY_BOUND_3E: u32 = 0x499a_0646;

/// `edx + 0xb665f9b9` from `0x00616b3e`.
pub fn a66_entry_bias_3e(edx: u32) -> u32 {
    edx.wrapping_add(A66_ENTRY_BIAS_3E)
}

/// `0x00616db7` copies seven dwords out of its 0x1c-byte descriptor and
/// dispatches only when the masked virtual-call result is non-negative
/// (`if (-1 < iVar5)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A66DescGate {
    /// `true` when the virtual call returned `>= 0`.
    pub dispatch: bool,
    /// Bytes copied from the local descriptor (`0x1c`).
    pub bytes: usize,
}

/// Reproduce the `0x00616db7` gate.
pub fn a66_desc_gate(call_result: i32) -> A66DescGate {
    A66DescGate {
        dispatch: call_result >= 0,
        bytes: A66_DESC_BYTES,
    }
}

// ---------------------------------------------------------------------------
// 0x006170d0: owner slot clear
// ---------------------------------------------------------------------------

/// `param & 0x3fffffff` mask from `0x006170d0`.
pub const A66_SLOT_MASK: u32 = 0x3fff_ffff;
/// Exclusive slot bound (`uVar4 < 0xff`).
pub const A66_SLOT_LIMIT: u32 = 0xff;
/// Slot that takes the early-out (`uVar4 == 0x2c`).
pub const A66_SLOT_SPECIAL: u32 = 0x2c;

/// Result of the `0x006170d0` store sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A66SlotClear {
    /// `true` when `param & 0x3fffffff` is below `0xff`.
    pub in_range: bool,
    /// The masked slot index, valid when [`Self::in_range`].
    pub index: u32,
    /// `+0x14..+0x16` refresh, present only when the slot test succeeded
    /// (`FUN_006148b0` returned non-zero).
    pub derived: Option<[u8; 3]>,
    /// `true` for slot `0x2c`, which returns before the notify.
    pub special: bool,
}

/// `0x006170d0`'s owner-side byte table.
///
/// The routine writes `owner + 4 + index` (index `0..=0xfe`) and reads the flag
/// bytes `+0xa4..+0xa8` to refresh `+0x14..+0x16`:
///
/// ```text
/// owner[+0x14] = owner[+0xa5] | owner[+0xa4]
/// owner[+0x15] = owner[+0xa7] | owner[+0xa6]
/// owner[+0x16] = owner[+0xa8]
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A66SlotTable {
    /// `owner+4..owner+0x103`, one byte per slot index.
    pub slots: [u8; A66_SLOT_LIMIT as usize],
    /// `owner+0xa4`.
    pub a4: u8,
    /// `owner+0xa5`.
    pub a5: u8,
    /// `owner+0xa6`.
    pub a6: u8,
    /// `owner+0xa7`.
    pub a7: u8,
    /// `owner+0xa8`.
    pub a8: u8,
}

impl A66SlotTable {
    /// Zero-filled owner view (`0x006170d0` does not initialise the table;
    /// the constructor only exists so the tests can build one).
    pub fn a66_new() -> Self {
        Self {
            slots: [0; A66_SLOT_LIMIT as usize],
            a4: 0,
            a5: 0,
            a6: 0,
            a7: 0,
            a8: 0,
        }
    }

    /// `0x006170d0`: clear one slot byte and refresh the derived triple.
    ///
    /// `slot_test` is the result of the shipped predicate (`FUN_006148b0`);
    /// the routine's own arithmetic — masking, the `< 0xff` bound, the three
    /// OR/overwrite stores and the `0x2c` early-out — is reproduced exactly.
    pub fn a66_clear(&mut self, param: u32, slot_test: bool) -> A66SlotClear {
        let index = param & A66_SLOT_MASK;
        if index >= A66_SLOT_LIMIT {
            return A66SlotClear::default();
        }
        self.slots[index as usize] = 0;
        let derived = if slot_test {
            Some([self.a5 | self.a4, self.a7 | self.a6, self.a8])
        } else {
            None
        };
        A66SlotClear {
            in_range: true,
            index,
            derived,
            special: index == A66_SLOT_SPECIAL,
        }
    }
}

impl Default for A66SlotTable {
    fn default() -> Self {
        Self::a66_new()
    }
}

// ---------------------------------------------------------------------------
// 0x0061729a: mode/seed table, string slots, loop guard
// ---------------------------------------------------------------------------

/// Mode table entries selected by the `unaff_ESI` argument of `0x0061729a`.
pub const A66_MODE_TABLE_1: u32 = 0x00fa_94b0;
/// Second entry (`unaff_ESI == 4`).
pub const A66_MODE_TABLE_4: u32 = 0x00fa_94b4;
/// Third entry (every other selector).
pub const A66_MODE_TABLE_OTHER: u32 = 0x00fa_94b8;

/// One recovered mode-table entry.
///
/// `0x0061729a` seeds a stack block with the packed word `0x00060004`
/// (selector 1), `0x00090007` (selector 4) or `0x000c000a` (other), and points
/// three frame slots into it.  The packed word holds two `u16`s: selector 1 is
/// `(4, 6)`, selector 4 is `(7, 9)`, the default is `(10, 12)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A66ModeSeed {
    /// Address stored at `ebp-0x60` (one of the three `0x00fa94xx` tables).
    pub table: u32,
    /// The packed `u16` pair written at the key slot.
    pub word: u32,
    /// `ebp-0xc4`: pointer slot for the packed word.
    pub key_off: i32,
    /// `ebp-0xb8`: pointer slot for the upper `u16` of the word.
    pub x_off: i32,
    /// `ebp-0xbc`: pointer slot for the class dword.
    pub c_off: i32,
    /// The remaining seeded pointer (`puVar7`).
    pub sub_off: i32,
}

/// Build the `0x0061729a` mode seed for a selector.
///
/// ```text
/// esi == 1 : table 0x00fa94b0, word 0x00060004, slots -0x28/-0x26/-0x20/-0x24
/// esi == 4 : table 0x00fa94b4, word 0x00090007, slots -0x1c/-0x1a/-0x14/-0x18
/// otherwise: table 0x00fa94b8, word 0x000c000a, slots -0x10/-0x0e/-0x08/-0x0c
/// ```
pub fn a66_mode_seed(selector: i32) -> A66ModeSeed {
    match selector {
        1 => A66ModeSeed {
            table: A66_MODE_TABLE_1,
            word: 0x0006_0004,
            key_off: -0x28,
            x_off: -0x26,
            c_off: -0x20,
            sub_off: -0x24,
        },
        4 => A66ModeSeed {
            table: A66_MODE_TABLE_4,
            word: 0x0009_0007,
            key_off: -0x1c,
            x_off: -0x1a,
            c_off: -0x14,
            sub_off: -0x18,
        },
        _ => A66ModeSeed {
            table: A66_MODE_TABLE_OTHER,
            word: 0x000c_000a,
            key_off: -0x10,
            x_off: -0x0e,
            c_off: -0x08,
            sub_off: -0x0c,
        },
    }
}

/// Split a mode seed word into its `(low, high)` `u16` pair.
pub fn a66_mode_pair(word: u32) -> (u16, u16) {
    (word as u16, (word >> 16) as u16)
}

/// Capacity word written by both empty-string initialisers of `0x0061729a`
/// (`*(edi + 0x40) = 0x8000000f`, `*(edi + 0x5c) = 0x8000000f`).
pub const A66_STRING_CAP: u32 = 0x8000_000f;

/// `(data, size, capacity)` offsets of the two string members of the
/// `0x0061729a` object.
///
/// Selector 1 writes the trio at `+0x38/+0x3c/+0x40`; selector 2 writes the
/// same trio at `+0x54/+0x58/+0x5c`.  Both initialisers set `size = 0`,
/// `capacity = 0x8000000f`, and the first content byte to zero, then call the
/// out-of-window throw helper `FUN_00538810` (the recovered prefix stops at
/// that non-returning call).
pub fn a66_string_slot(which: u8) -> Option<(u32, u32, u32)> {
    match which {
        1 => Some((0x38, 0x3c, 0x40)),
        2 => Some((0x54, 0x58, 0x5c)),
        _ => None,
    }
}

/// Loop guard of `0x0061729a`:
/// `*(char *)(ebp+…aaa0) != 0 && *(uint *)(…aa98) < 0x15 && *(uint *)(…aa9c) < 5`.
pub const A66_GUARD_COUNT: u32 = 0x15;
/// Second guard bound (`< 5`).
pub const A66_GUARD_SUB: u32 = 5;

/// The three-field guard that gates the `+0x74` virtual call.
pub fn a66_iter_guard(flag: bool, count: u32, sub: u32) -> bool {
    flag && count < A66_GUARD_COUNT && sub < A66_GUARD_SUB
}

// ---------------------------------------------------------------------------
// Entry scan: 0x00617551, 0x00617600, 0x00617773
// ---------------------------------------------------------------------------

/// `DAT_00f7ea50 & 0x500 != 0`: the ISA feature pair that selects the widest
/// unroll (`0x00617551` writes `0x10` into `ebp-0x11`).
pub const A66_ISA_WIDE: u32 = 0x500;
/// `DAT_00f7ea50 & 0x200`: the narrower ISA bit that selects unroll `8`.
pub const A66_ISA_NARROW: u32 = 0x200;

/// Unroll/stride word chosen by `0x00617551` before the scan:
///
/// ```text
/// stride = 1
/// if (isa & 0x500) != 0      { stride = 0x10 }
/// else if (isa & 0x200) != 0 { stride = 8 }
/// ```
pub fn a66_unroll_stride(isa: u32) -> u32 {
    if isa & A66_ISA_WIDE != 0 {
        0x10
    } else if isa & A66_ISA_NARROW != 0 {
        8
    } else {
        1
    }
}

/// Addend that turns the count word into the effective limit
/// (`count ^ (DAT_00f7f79e + 0x4163f040)`).
pub const A66_COUNT_BIAS: u32 = 0x4163_f040;
/// The `DAT_00f7f79e` value that makes the bias cancel (`-0x4163f040`).
pub const A66_COUNT_BIAS_CANCEL: u32 = 0xbe9c_0fc0;
/// Tag xor applied to every stored entry word before the zero test.
pub const A66_ENTRY_TAG: u32 = 0xdce8_4bd8;

/// `count ^ (global + 0x4163f040)`.
pub fn a66_effective_count(count: u32, global: u32) -> u32 {
    count ^ global.wrapping_add(A66_COUNT_BIAS)
}

/// `(index + 1) % limit`, with the wrap suppressed for a zero limit (the
/// shipping code divides by zero in that case; the model returns `0`).
pub fn a66_next_index_limit(index: u32, limit: u32) -> u32 {
    if limit == 0 {
        0
    } else {
        index.wrapping_add(1) % limit
    }
}

/// Entry-word decode: `word ^ key ^ 0xdce84bd8`; an empty slot stores exactly
/// `key ^ 0xdce84bd8` so the decoded value is zero.
pub fn a66_decode_entry(raw: u32, key: u32) -> u32 {
    raw ^ key ^ A66_ENTRY_TAG
}

/// Outcome of the zero-skipping scan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A66EntryScan {
    /// Index of the first non-empty entry, or the wrap position when the scan
    /// exhausted the table.
    pub index: u32,
    /// Number of empty entries skipped.
    pub processed: u32,
    /// Decoded entry, `Some` only when a non-empty entry was found.
    pub entry: Option<u32>,
}

/// Reproduce the `0x00617551` / `0x00617600` scan.
///
/// Both rows are recursive: on an empty entry they do
/// `processed += 1; index = (index + 1) % count;` and call themselves.  The
/// iteration stops when `processed == effective_count` — the tail
/// `if (processed == (count ^ bias)) break;` in both bodies.
///
/// `words` is the entry array the mangled base points at; every entry is eight
/// bytes wide, and the decoded dword sits at `+4`.
pub fn a66_scan_entries(
    count: u32,
    global: u32,
    words: &[u32],
    key: u32,
    start: u32,
) -> A66EntryScan {
    let limit = a66_effective_count(count, global);
    let mut index = if limit == 0 { 0 } else { start % limit };
    let mut processed = 0u32;
    while processed < limit {
        let raw = words.get(index as usize).copied().unwrap_or(0);
        let entry = a66_decode_entry(raw, key);
        if entry != 0 {
            return A66EntryScan {
                index,
                processed,
                entry: Some(entry),
            };
        }
        processed = processed.wrapping_add(1);
        index = a66_next_index_limit(index, limit);
    }
    A66EntryScan {
        index,
        processed,
        entry: None,
    }
}

/// `0x00617773`'s notify throttle window (`… - *stamp < 500`).
pub const A66_THROTTLE_WINDOW: u32 = 500;

/// `0x00617773`'s same-target decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A66Throttle {
    /// `last == target && now - stamp < 500`.
    pub same_target: bool,
    /// New stamp: `now - 500` when throttled, otherwise `now`.
    pub stamp: u32,
}

/// Reproduce the cached-target throttle at `0x00617773` (`LAB_006199f6`).
///
/// ```text
/// if (a == DAT_00f04c74 && b == DAT_00f04c78 && (now - *stamp) < 500) {
///     DAT_00f04c74 = a; DAT_00f04c78 = b;
///     *stamp = now - 500;
/// } else {
///     DAT_00f04c74 = a; DAT_00f04c78 = b;
///     *stamp = now;
/// }
/// ```
pub fn a66_throttle(last: (i32, i32), target: (i32, i32), now: u32, stamp: u32) -> A66Throttle {
    let same_target = last == target && now.wrapping_sub(stamp) < A66_THROTTLE_WINDOW;
    A66Throttle {
        same_target,
        stamp: if same_target {
            now.wrapping_sub(A66_THROTTLE_WINDOW)
        } else {
            now
        },
    }
}

// ---------------------------------------------------------------------------
// 0x00617690: rotate, byte division and the init record
// ---------------------------------------------------------------------------

/// `*p = *p << 0x15 | *p >> 0xb`: rotate left by 21.
pub const A66_ROTL_BITS: u32 = 21;
/// Byte divisor of `0x00617690` (`bVar3 / 0xc6`, `bVar3 % 0xc6`).
pub const A66_DIVISOR_198: u8 = 0xc6;
/// `in_EDX + 99` bias of the second address computation.
pub const A66_PTR_BIAS_690: u32 = 99;
/// `-0x7449d0e5`, the addend applied after the byte-1 rewrite.
pub const A66_PTR_ADD_690: u32 = 0x8bb6_2f1b;

/// Rotate a dword left by 21 bits (`0x00617690`).
pub fn a66_rotl21(x: u32) -> u32 {
    x.rotate_left(A66_ROTL_BITS)
}

/// `divmod` by 198 (`0x00617690`: quotient and remainder of a byte).
pub fn a66_divmod_198(value: u8) -> (u8, u8) {
    (value / A66_DIVISOR_198, value % A66_DIVISOR_198)
}

/// Replace bits `8..16` of `x` with `0xc8`, as `0x00617690` does with
/// `CONCAT22(hi16, CONCAT11(200, lo8))` before adding `-0x7449d0e5`.
pub fn a66_force_byte1(x: u32) -> u32 {
    (x & !0xff00) | 0xc800
}

/// Full address arithmetic of `0x00617690`'s second branch:
/// `(force_byte1(edx + 99) + -0x7449d0e5)`.
pub fn a66_biased_ptr_690(edx: u32) -> u32 {
    a66_force_byte1(edx.wrapping_add(A66_PTR_BIAS_690)).wrapping_add(A66_PTR_ADD_690)
}

/// The init record `0x00617690` stores before running its builder:
/// `*(ebp-0x28) = 9`, `*(ebp-4) = 7`, then `*(ebp-4) = 8` after the builder
/// returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A66InitState {
    /// `*(ebp-0x28)`: element count passed to the builder.
    pub count: u32,
    /// `*(ebp-4)` before the builder call.
    pub state_before: u8,
    /// `*(ebp-4)` after the builder call.
    pub state_after: u8,
}

/// The recovered `0x00617690` state triple.
pub fn a66_init_state() -> A66InitState {
    A66InitState {
        count: 9,
        state_before: 7,
        state_after: 8,
    }
}

// ---------------------------------------------------------------------------
// 0x00617c69: byte divmod
// ---------------------------------------------------------------------------

/// `0x00617c69` returns `CONCAT11(ax % d, ax / d)`, i.e. the **remainder in the
/// high byte and the quotient in the low byte**.  The divisor is a byte read
/// from `ebp - 0x75 + edx`; a zero divisor would fault in retail, and the model
/// returns `0` for that case.
pub fn a66_divmod_u8(ax: u16, divisor: u8) -> u16 {
    if divisor == 0 {
        return 0;
    }
    let quotient = ax / u16::from(divisor);
    let remainder = ax % u16::from(divisor);
    (u16::from(remainder as u8) << 8) | u16::from(quotient as u8)
}

#[cfg(test)]
mod tests_agent66 {
    use super::*;

    #[test]
    fn constant_decoder_matches_the_shipped_chain() {
        // 0x00616007: 0x7ffe105d - 0x7b0 - 0x12d8 ^ 0xf5f + 0x576.
        assert_eq!(a66_decode_const(0x7ffe105d), A66_FRAME_BASE);
        assert_eq!(a66_decode_const(0x105d), 0);
        assert_eq!(a66_decode_const(0x1059), 4);
        assert_eq!(a66_decode_const(0x1085), 24);
        assert_eq!(a66_decode_const(0xd7d), 800);
        assert_eq!(a66_decode_const(0xbd9), 900);
        assert_eq!(a66_decode_const(0x1039), 100);
        assert_eq!(a66_decode_const(0x205d), 0x1000);
        // 0x0061729a's constants.
        assert_eq!(a66_decode_const(0xde5), 376);
        assert_eq!(a66_decode_const(0xde1), 380);
        assert_eq!(a66_decode_const(0xdf9), 420);
        assert_eq!(a66_decode_const(0xe45), 600);
        assert_eq!(a66_decode_const(0x1165), 0xffff_fef8);
        assert_eq!(a66_decode_const(0x1101), 0xffff_fe9c);
    }

    #[test]
    fn constant_decoder_round_trips_and_short_form_lands_on_base() {
        for v in [0u32, 4, 24, 100, 420, 800, 900, 600, 0x1000, 0xffff_fef8] {
            assert_eq!(a66_decode_const(a66_encode_const(v)), v);
        }
        assert_eq!(a66_decode_const_short(0x7ffdfc43), A66_FRAME_BASE);
        assert_eq!(a66_decode_const_short(0x1039), 0x1039 + 0x3bd);
    }

    #[test]
    fn wrapped_or_zero_flag_tests() {
        // i8: 0 + 0 is zero, so the test fires.
        assert!(a66_i8_add_wraps_or_zero(0, 0));
        // 0x7f + 1 overflows to -0x80 (negative), so overflow == sign.
        assert!(!a66_i8_add_wraps_or_zero(0x7f, 1));
        // 0x80 + 0x80 overflows to 0 -> zero.
        assert!(a66_i8_add_wraps_or_zero(-128, -128));
        assert!(a66_u32_add_wraps_or_zero(0, 0));
        assert!(!a66_u32_add_wraps_or_zero(0x7fff_ffff, 1));
        assert!(a66_u32_add_wraps_or_zero(u32::MAX, 1));
    }

    #[test]
    fn multiply_sequences_reproduce_the_64x32_accumulator() {
        assert_eq!(a66_mul_shift_u64(0x1000_0000, 0x40, 24), 0x400u64);
        assert_eq!(a66_mul_shift_u64(3, 5, 0), 15);
        let (lo, hi) = a66_mul_acc_64x32(0xffff_ffff, 3, 5);
        assert_eq!(lo, 0xffff_fffb);
        assert_eq!(hi, 3 * 5 + 4);
        assert_eq!(a66_mul_acc_64x32(7, 9, 1), (7, 9));
    }

    #[test]
    fn unmangler_and_checksums_are_exact() {
        // 0x00616b5c: lo = x & 0xffff, mid folds table ^ x ^ (y >> 16).
        let x = 0x1234_5678;
        let y = 0x9abc_def0;
        let table = 0x1357;
        // Without a pad the low half is preserved exactly.
        let plain = a66_unmangle_word(x, y, table, 0);
        assert_eq!(plain & 0xffff, x & 0xffff, "the low half is preserved");
        assert_eq!(plain, {
            let lo = x & 0xffff;
            let mid = ((u32::from(table) ^ x ^ (y >> 16)) << 16 | x) >> 16;
            ((lo.wrapping_sub(0x93af) ^ mid) << 16) | lo
        });
        // The stack pad is xored over the whole dword.
        assert_eq!(a66_unmangle_word(x, y, table, 0x5a), plain ^ 0x5a);
        // The validator fires only when the low half is zero.
        let lo0 = a66_unmangle_word(0x2000_0000, 0x0000_0000, 0, 0);
        assert_eq!(lo0 & 0xffff, 0);
        let hi = lo0 >> 16;
        assert!(
            a66_check_word_93af(lo0, hi as u16, 0),
            "table_lo = hi and table_hi = 0 satisfy form A"
        );
        assert!(!a66_check_word_93af(lo0, (hi as u16) ^ 1, 0));
        assert!(!a66_check_word_112da(lo0, hi as u16));
        assert!(!a66_check_word_93af(0x0000_0001, 1, 0));
    }

    #[test]
    fn notify_descriptor_selects_the_three_modes() {
        // kind 8 -> 0xf and the 0x100 flag when bit 30 is set.
        let ext = a66_notify(0x4000_0001, 8, 0x20);
        assert_eq!(
            ext,
            A66Notify {
                id_low: 1,
                word: 0x120,
                mode: 0xf,
                extended: true
            }
        );
        // bit 30 without kind 8 -> mode 1, word gets 0x100.
        let wide = a66_notify(0x4000_0001, 3, 0x20);
        assert_eq!(wide.mode, 1);
        assert_eq!(wide.word, 0x120);
        assert!(!wide.extended);
        // plain id -> mode 0, word untouched.
        let plain = a66_notify(0x0000_0007, 3, 0x20);
        assert_eq!(plain.mode, 0);
        assert_eq!(plain.word, 0x20);
        assert_eq!(plain.id_low, 7);
        // bit 30 is also visible on the extended path's mode word.
        assert_eq!(a66_clear_tag(0x1234_5679), 0x1234_5678);
        assert_eq!(a66_clear_tag(0x1234_5678), 0x1234_5678);
        assert_eq!(a66_convert_helper(true), A66_CONVERT_X87);
        assert_eq!(a66_convert_helper(false), A66_CONVERT_SSE);
        assert_eq!(a66_entry_bias_3e(0x499a_0647), 0x0);
        assert_eq!(a66_entry_bias_3e(0), A66_ENTRY_BIAS_3E);
    }

    #[test]
    fn descriptor_gate_copies_0x1c_bytes_only_on_non_negative() {
        let hit = a66_desc_gate(0);
        assert!(hit.dispatch);
        assert_eq!(hit.bytes, A66_DESC_BYTES);
        assert_eq!(hit.bytes, 0x1c);
        let miss = a66_desc_gate(-1);
        assert!(!miss.dispatch);
        assert!(!a66_desc_gate(-0x8000_0000).dispatch);
    }

    #[test]
    fn slot_clear_masks_bounds_and_derives() {
        let mut table = A66SlotTable::a66_new();
        table.a4 = 0x10;
        table.a5 = 0x01;
        table.a6 = 0x80;
        table.a7 = 0x02;
        table.a8 = 0x5a;
        table.slots[0x2c] = 0xff;
        let cleared = table.a66_clear(0x2c, true);
        assert!(cleared.in_range);
        assert_eq!(cleared.index, 0x2c);
        assert_eq!(cleared.derived, Some([0x11, 0x82, 0x5a]));
        assert!(cleared.special, "0x2c returns before the notify");
        assert_eq!(table.slots[0x2c], 0);

        // The high bits are masked off, not rejected (0x40000067 -> 0x67).
        table.slots[0x67] = 1;
        let masked = table.a66_clear(0x4000_0067, false);
        assert!(masked.in_range);
        assert_eq!(masked.index, 0x67);
        assert_eq!(masked.derived, None);

        // 0xff and above are out of range.
        assert!(!table.a66_clear(0xff, true).in_range);
        assert!(!table.a66_clear(0xffff_ffff, true).in_range);
    }

    #[test]
    fn mode_seed_table_matches_the_three_entries() {
        let one = a66_mode_seed(1);
        assert_eq!(one.table, A66_MODE_TABLE_1);
        assert_eq!(one.word, 0x0006_0004);
        assert_eq!(a66_mode_pair(one.word), (4, 6));
        assert_eq!(
            (one.key_off, one.x_off, one.c_off, one.sub_off),
            (-0x28, -0x26, -0x20, -0x24)
        );

        let four = a66_mode_seed(4);
        assert_eq!(four.table, A66_MODE_TABLE_4);
        assert_eq!(a66_mode_pair(four.word), (7, 9));
        assert_eq!(
            (four.key_off, four.x_off, four.c_off, four.sub_off),
            (-0x1c, -0x1a, -0x14, -0x18)
        );

        let other = a66_mode_seed(9);
        assert_eq!(other.table, A66_MODE_TABLE_OTHER);
        assert_eq!(a66_mode_pair(other.word), (10, 12));
        assert_eq!(
            (other.key_off, other.x_off, other.c_off, other.sub_off),
            (-0x10, -0x0e, -0x08, -0x0c)
        );
    }

    #[test]
    fn string_slots_and_loop_guard() {
        assert_eq!(a66_string_slot(1), Some((0x38, 0x3c, 0x40)));
        assert_eq!(a66_string_slot(2), Some((0x54, 0x58, 0x5c)));
        assert_eq!(a66_string_slot(0), None);
        assert_eq!(a66_string_slot(3), None);
        assert_eq!(A66_STRING_CAP, 0x8000_000f);

        assert!(a66_iter_guard(true, 20, 4));
        assert!(!a66_iter_guard(false, 20, 4));
        assert!(!a66_iter_guard(true, 21, 4), "count < 0x15 is strict");
        assert!(!a66_iter_guard(true, 20, 5), "sub < 5 is strict");
    }

    #[test]
    fn unroll_stride_and_effective_count() {
        assert_eq!(a66_unroll_stride(0), 1);
        assert_eq!(a66_unroll_stride(0x40), 1);
        assert_eq!(a66_unroll_stride(A66_ISA_NARROW), 8);
        // Bit 8 is part of the 0x500 mask, so 0x100 alone selects the wide 0x10.
        assert_eq!(a66_unroll_stride(0x100), 0x10);
        assert_eq!(a66_unroll_stride(A66_ISA_WIDE), 0x10);
        assert_eq!(a66_effective_count(0x10, A66_COUNT_BIAS_CANCEL), 0x10);
        assert_eq!(a66_effective_count(0x10, 0), 0x10 ^ A66_COUNT_BIAS);
        assert_eq!(a66_next_index_limit(2, 3), 0);
        assert_eq!(a66_next_index_limit(1, 3), 2);
        assert_eq!(a66_next_index_limit(0, 0), 0);
    }

    #[test]
    fn entry_scan_skips_empty_slots_and_counts_them() {
        let key = 0x1122_3344;
        let empty = key ^ A66_ENTRY_TAG;
        let live = 0x99 ^ key ^ A66_ENTRY_TAG;
        // Three slots, the third holds a live entry.
        let words = [empty, empty, live];
        let scan = a66_scan_entries(3, A66_COUNT_BIAS_CANCEL, &words, key, 0);
        assert_eq!(scan.processed, 2);
        assert_eq!(scan.index, 2);
        assert_eq!(scan.entry, Some(0x99));

        // A full sweep wraps back to the start and reports no entry.
        let all_empty = [empty; 3];
        let none = a66_scan_entries(3, A66_COUNT_BIAS_CANCEL, &all_empty, key, 1);
        assert_eq!(none.processed, 3);
        assert_eq!(none.entry, None);
        assert_eq!(none.index, 1, "wrapped back to the start index");

        // The first slot is live: nothing is skipped.
        let hit = a66_scan_entries(3, A66_COUNT_BIAS_CANCEL, &[0x7, empty, empty], key, 0);
        assert_eq!(hit.processed, 0);
        assert_eq!(hit.entry, Some(0x7 ^ key ^ A66_ENTRY_TAG));
        // decode_entry is the identity on zero tags.
        assert_eq!(a66_decode_entry(empty, key), 0);
    }

    #[test]
    fn throttle_rewinds_only_inside_the_window() {
        let last = (0x11, 0x22);
        let hot = a66_throttle(last, last, 1000, 600);
        assert!(hot.same_target, "400 frames < 500");
        assert_eq!(hot.stamp, 500);

        let cold = a66_throttle(last, last, 1200, 600);
        assert!(!cold.same_target, "600 frames is outside the window");
        assert_eq!(cold.stamp, 1200);

        let moved = a66_throttle(last, (0x11, 0x23), 1000, 600);
        assert!(!moved.same_target);
        assert_eq!(moved.stamp, 1000);

        // Exactly 500 frames is not inside the window.
        assert!(!a66_throttle(last, last, 1100, 600).same_target);
        assert_eq!(A66_THROTTLE_WINDOW, 500);
    }

    #[test]
    fn rotate_division_and_init_record() {
        assert_eq!(a66_rotl21(1), 0x20_0000);
        assert_eq!(a66_rotl21(0x8000_0000), 0x10_0000);
        assert_eq!(a66_rotl21(0), 0);
        assert_eq!(a66_rotl21(0x1234_5678).rotate_right(21), 0x1234_5678);

        assert_eq!(a66_divmod_198(0xc6), (1, 0));
        assert_eq!(a66_divmod_198(0xff), (1, 0x39));
        assert_eq!(a66_divmod_198(0xb3), (0, 0xb3));
        assert_eq!(a66_force_byte1(0x1234_5678), 0x1234_c878);
        assert_eq!(
            a66_biased_ptr_690(0),
            (0xc863u32).wrapping_add(A66_PTR_ADD_690)
        );

        let init = a66_init_state();
        assert_eq!(init.count, 9);
        assert_eq!(init.state_before, 7);
        assert_eq!(init.state_after, 8);
    }

    #[test]
    fn byte_divmod_puts_remainder_high_and_quotient_low() {
        // 0x00617c69: CONCAT11(ax % d, ax / d).
        assert_eq!(a66_divmod_u8(0x0123, 0x10), 0x0312);
        assert_eq!(a66_divmod_u8(0x00ff, 0x10), 0x0f0f);
        assert_eq!(a66_divmod_u8(0x1234, 0x01), 0x0034);
        assert_eq!(a66_divmod_u8(0x1234, 0xff), 0x4612);
        assert_eq!(a66_divmod_u8(0x1234, 0), 0, "zero divisor is guarded");
    }
}
