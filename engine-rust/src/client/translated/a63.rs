#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent63.rs
// ===========================================================================
// Translation batch 63.  Only this file is edited by the batch.
//
// Batch 63 owns `0x00610000..=0x00611fff`.  The ledger lists 50
// `decompiled_uncited` rows in that window; 29 are translated below, the
// other 21 are recorded with their skip reason in the closing section.
//
// Two object families dominate the window:
//
// * An 8-byte-payload tagged value (`tag` at `+0x00`, payload at `+0x08`).
//   `0x00610620`, `0x00610640`, `0x00610660` and `0x00610730` are the four
//   `operator=` overloads (byte, qword, two-dword pair); `0x00610410` is the
//   tag-copying assignment and `0x00610520` its equality comparison.  Every
//   `switch` in this family is a bare indirect call through
//   `*(code **)(base + index * 4)`; [`a63_slot_target`] reproduces that
//   address arithmetic and each row keeps its own table base.
// * An MSVC `std::string`-shaped header (`+0x08` data, `+0x0c` length,
//   `+0x10` capacity/flags, inline buffer at `+0x14`) that `0x006103c0` and
//   `0x00610680` initialise to the empty form `0x8000000f` / length 0, and
//   `0x006101c3` terminates by storing the new length back into `+0x0c`.
//
// The richest pure routines are [`a63_mix_dwords`] (`0x00610970`, an
// eight-mode dword mixer with byte-rotate modes) and [`a63_hash_insert`]
// (`0x00611fbd`, MurmurHash3 `fmix32` bucket selection plus a load-factor
// check against `count / capacity`).

// ---------------------------------------------------------------------------
// Tagged value (tag at +0x00, 8-byte payload at +0x08)
// ---------------------------------------------------------------------------

/// Width of one recovered jump-table slot: all tables in this window hold
/// 32-bit code pointers (`*(code **)(base + index * 4)`).
pub const A63_SLOT_BYTES: u32 = 4;

/// Tag stored by `0x00610620` (`operator=(bool)`).
pub const A63_TAG_BYTE: u32 = 5;
/// Tag stored by `0x00610640` (`operator=(double)`/qword).
pub const A63_TAG_QWORD: u32 = 4;
/// Tag stored by `0x00610660` (`operator=(pair)`, first overload).
pub const A63_TAG_PAIR_A: u32 = 2;
/// Tag stored by `0x00610730` (`operator=(pair)`, second overload).
pub const A63_TAG_PAIR_B: u32 = 3;

/// Recovered tagged value: `tag` at `+0x00`, the union payload at
/// `+0x08..+0x10`.  Only the payload bytes each setter writes are modelled;
/// the value at `+0x04` is never touched by the four setters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A63Value {
    /// `+0x00`: active member tag.
    pub tag: u32,
    /// `+0x08`: payload bytes, little-endian where a setter stores a number.
    pub payload: [u8; 8],
}

impl A63Value {
    /// `0x00610620` (`FUN_00610620`, 20 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// *(undefined1 *)(param_1 + 2) = param_2;   /* byte at +0x08 */
    /// *param_1 = 5;                             /* tag */
    /// ```
    ///
    /// Only `payload[0]` is written; the other seven bytes keep whatever the
    /// destination held.
    pub fn a63_set_tag5_byte(&mut self, value: u8) {
        self.tag = A63_TAG_BYTE;
        self.payload[0] = value;
    }

    /// `0x00610640` (`FUN_00610640`, 24 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// *(undefined8 *)(param_1 + 2) = param_2;   /* qword at +0x08 */
    /// *param_1 = 4;
    /// ```
    pub fn a63_set_tag4_qword(&mut self, value: u64) {
        self.tag = A63_TAG_QWORD;
        self.payload = value.to_le_bytes();
    }

    /// `0x00610660` (`FUN_00610660`, 26 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// param_1[2] = param_2;   /* dword at +0x08 */
    /// param_1[3] = param_3;   /* dword at +0x0c */
    /// *param_1 = 2;
    /// ```
    pub fn a63_set_tag2_pair(&mut self, low: u32, high: u32) {
        self.tag = A63_TAG_PAIR_A;
        self.payload[0..4].copy_from_slice(&low.to_le_bytes());
        self.payload[4..8].copy_from_slice(&high.to_le_bytes());
    }

    /// `0x00610730` (`FUN_00610730`, 26 bytes, `decompiled_uncited`).
    ///
    /// Byte-for-byte the same body as `0x00610660` except for the tag, which
    /// is `3` instead of `2`.
    pub fn a63_set_tag3_pair(&mut self, low: u32, high: u32) {
        self.tag = A63_TAG_PAIR_B;
        self.payload[0..4].copy_from_slice(&low.to_le_bytes());
        self.payload[4..8].copy_from_slice(&high.to_le_bytes());
    }

    /// The two pair setters only differ in the tag they store.
    pub fn a63_pair(&self) -> Option<(u32, u32)> {
        match self.tag {
            A63_TAG_PAIR_A | A63_TAG_PAIR_B => {
                let low = u32::from_le_bytes([
                    self.payload[0],
                    self.payload[1],
                    self.payload[2],
                    self.payload[3],
                ]);
                let high = u32::from_le_bytes([
                    self.payload[4],
                    self.payload[5],
                    self.payload[6],
                    self.payload[7],
                ]);
                Some((low, high))
            }
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Jump-table slot arithmetic
// ---------------------------------------------------------------------------

/// Recovered slot address of a `switch`-on-tag body: `base + index * 4`.
///
/// The shipped bodies index a table of code pointers with a signed 32-bit
/// index; the addition and the scaling both wrap in 32 bits.
pub fn a63_slot_target(base: u32, index: i32) -> u32 {
    base.wrapping_add((index as u32).wrapping_mul(A63_SLOT_BYTES))
}

/// `0x00610070` (`FUN_00610070`, 58 bytes, `decompiled_uncited`).
///
/// ```c
/// (**(code **)(&DAT_00610340 + (*in_ECX + -1) * 4))();
/// ```
///
/// The dispatcher reads the tag from `+0x00` and uses `tag - 1` as the slot
/// index into the table at `0x00610340`.
pub fn a63_dispatch_00610070(tag: i32) -> u32 {
    a63_slot_target(0x0061_0340, tag.wrapping_sub(1))
}

/// `0x00610750` (`FUN_00610750`, 12 bytes, `decompiled_uncited`); table at
/// `0x006107b4`, index is the tag itself.
pub fn a63_dispatch_00610750(tag: i32) -> u32 {
    a63_slot_target(0x0061_07b4, tag)
}

/// `0x006107d0` (`FUN_006107d0`, 9 bytes, `decompiled_uncited`); table at
/// `0x0061080c`, index is the tag itself.
pub fn a63_dispatch_006107d0(tag: i32) -> u32 {
    a63_slot_target(0x0061_080c, tag)
}

/// `0x00610830` (`FUN_00610830`, 14 bytes, `decompiled_uncited`); table at
/// `0x006108f0`, index is the tag itself.
pub fn a63_dispatch_00610830(tag: i32) -> u32 {
    a63_slot_target(0x0061_08f0, tag)
}

/// `0x00610910` (`FUN_00610910`, 9 bytes, `decompiled_uncited`); table at
/// `0x0061094c`, index is the tag itself.
pub fn a63_dispatch_00610910(tag: i32) -> u32 {
    a63_slot_target(0x0061_094c, tag)
}

// ---------------------------------------------------------------------------
// Tag copy / tag compare (0x00610410, 0x00610520 + wrappers)
// ---------------------------------------------------------------------------

/// Successful tag copy from `0x00610410`: the destination tag is cleared and
/// then loaded with the source tag before the indirect dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A63Assign {
    /// Value written to `+0x00` before the source tag is copied.
    pub cleared: u32,
    /// Source tag loaded into `+0x00`.
    pub tag: u32,
    /// `0x00610500 + tag * 4`, the slot that is then called.
    pub jump_target: u32,
}

/// The range-check failure taken by `0x00610410` before the copy
/// (`FUN_0093ec70`, non-returning in the shipped image).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A63AssignFatal {
    /// `old_tag - 1`, the value compared against zero.
    pub tag_minus_one: u32,
    /// `+0x10` read as a signed dword; the check requires it to be negative
    /// to avoid the failure.
    pub word10: i32,
}

/// `0x00610410` (`FUN_00610410`, 88 bytes, `decompiled_uncited`).
///
/// ```c
/// local_8 = *in_ECX + -1;
/// if (local_8 == 0 && -1 < in_ECX[4]) FUN_0093ec70(...);  /* non-returning */
/// *in_ECX = 0;
/// iVar1 = *param_1;
/// *in_ECX = iVar1;
/// (**(code **)(&DAT_00610500 + iVar1 * 4))();
/// ```
///
/// The guard therefore fires only when the destination tag is `1` *and* the
/// dword at `+0x10` is non-negative.
pub fn a63_variant_assign(
    old_tag: u32,
    word10: u32,
    src_tag: u32,
) -> Result<A63Assign, A63AssignFatal> {
    let tag_minus_one = old_tag.wrapping_sub(1);
    if tag_minus_one == 0 && (word10 as i32) >= 0 {
        return Err(A63AssignFatal {
            tag_minus_one,
            word10: word10 as i32,
        });
    }
    Ok(A63Assign {
        cleared: 0,
        tag: src_tag,
        jump_target: a63_slot_target(0x0061_0500, src_tag as i32),
    })
}

/// `0x00610040` (`FUN_00610040`, 21 bytes, `decompiled_uncited`): the body is
/// `FUN_00610410(param_1);` only, so the wrapper forwards all three recovered
/// inputs unchanged.
pub fn a63_variant_assign_00610040(
    old_tag: u32,
    word10: u32,
    src_tag: u32,
) -> Result<A63Assign, A63AssignFatal> {
    a63_variant_assign(old_tag, word10, src_tag)
}

/// Result of `0x00610520`: either the masked "not equal" answer or the slot
/// that compares the two values of the same tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A63TagCompare {
    /// Tags differ: the first dword is returned with its low byte cleared.
    Different(u32),
    /// Tags match: `0x00610608 + word * 4` is called.
    Same {
        /// Full first dword that matched.
        word: u32,
        /// Slot holding the same-tag comparator.
        jump_target: u32,
    },
}

/// `0x00610520` (`FUN_00610520`, 27 bytes, `decompiled_uncited`).
///
/// ```c
/// uVar1 = *in_ECX;
/// if (uVar1 != *param_1) return uVar1 & 0xffffff00;
/// uVar1 = (**(code **)(&DAT_00610608 + uVar1 * 4))();
/// ```
pub fn a63_tag_compare(self_word: u32, other_word: u32) -> A63TagCompare {
    if self_word != other_word {
        A63TagCompare::Different(self_word & 0xffff_ff00)
    } else {
        A63TagCompare::Same {
            word: self_word,
            jump_target: a63_slot_target(0x0061_0608, self_word as i32),
        }
    }
}

/// `0x00610060` (`FUN_00610060`, 9 bytes, `decompiled_uncited`): a plain
/// forward of `FUN_00610520`, keeping the comparison contract identical.
pub fn a63_tag_compare_00610060(self_word: u32, other_word: u32) -> A63TagCompare {
    a63_tag_compare(self_word, other_word)
}

// ---------------------------------------------------------------------------
// Inline string header (0x006103c0, 0x00610680, 0x006101c3)
// ---------------------------------------------------------------------------

/// Capacity/flags word stored by every inline-string initialiser in this
/// window (`+0x10 = 0x8000000f`).
pub const A63_STRING_CAP: u32 = 0x8000_000f;

/// Recovered `std::string` header: `+0x08` data pointer (inline at `+0x14`),
/// `+0x0c` length, `+0x10` capacity with the top flag bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A63StringHeader {
    /// `+0x0c`: stored byte count.
    pub len: u32,
    /// `+0x10`: capacity with the top flag bit.
    pub cap: u32,
    /// `+0x14`: first inline byte written by the initialiser.
    pub first: u8,
    /// `+0x08`: `true` when the data pointer is `this + 0x14`.
    pub inline_at_14: bool,
}

impl A63StringHeader {
    /// Empty header shared by `0x006103c0` and `0x00610680`: length `0`,
    /// capacity `0x8000000f`, data pointer `this + 0x14`, first byte `0`.
    pub fn a63_empty_inline() -> Self {
        Self {
            len: 0,
            cap: A63_STRING_CAP,
            first: 0,
            inline_at_14: true,
        }
    }
}

/// Result of `0x006103c0`: the reset header plus the capacity request the
/// shipped body forwards to its (out-of-window) growth helper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A63StringReset {
    /// Header after the four stores.
    pub header: A63StringHeader,
    /// `*(u32 *)(src + 4)` — the value read before the stores and forwarded.
    pub requested: u32,
}

/// `0x006103c0` (`FUN_006103c0`, 47 bytes, `decompiled_uncited`).
///
/// ```c
/// uVar1 = *(undefined4 *)(param_1 + 4);
/// *(undefined4 *)(in_ECX + 0xc) = 0;                 /* length   */
/// *(undefined1 **)(in_ECX + 8) = (in_ECX + 0x14);    /* data     */
/// *(undefined4 *)(in_ECX + 0x10) = 0x8000000f;       /* capacity */
/// *(undefined1 *)(in_ECX + 0x14) = 0;                /* first    */
/// FUN_00538810(uVar1);                               /* non-returning */
/// ```
///
/// Only the request value is recovered from the tail; the helper itself is
/// outside the window and is not modelled.
pub fn a63_string_reset_006103c0(requested: u32) -> A63StringReset {
    A63StringReset {
        header: A63StringHeader::a63_empty_inline(),
        requested,
    }
}

/// Result of `0x00610680`: the cleared tag plus the initialised header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A63RecordInit {
    /// `+0x00`: cleared to zero by the constructor.
    pub tag: u32,
    /// `+0x08..+0x15`: the inline string header.
    pub header: A63StringHeader,
    /// `*(u32 *)(src + 4)`, forwarded to the growth helper.
    pub requested: u32,
}

/// `0x00610680` (`FUN_00610680`, 111 bytes, `decompiled_uncited`).
///
/// ```c
/// *param_1 = 0;                                /* tag            */
/// param_1[3] = 0;                              /* length  (+0x0c) */
/// param_1[2] = param_1 + 5;                    /* data    (+0x08) */
/// param_1[4] = 0x8000000f;                     /* capacity(+0x10) */
/// *(undefined1 *)(param_1 + 5) = 0;            /* first   (+0x14) */
/// FUN_00538810(*(undefined4 *)(param_2 + 4));
/// ```
pub fn a63_record_init_00610680(requested: u32) -> A63RecordInit {
    A63RecordInit {
        tag: 0,
        header: A63StringHeader::a63_empty_inline(),
        requested,
    }
}

/// Result of `0x006101c3`: where the terminator was written and whether the
/// debug guard at `+0x10` was satisfied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A63Terminate {
    /// Index the zero byte was written at, also the new stored length.
    pub index: u32,
    /// `+0x0c` after the store.
    pub len: u32,
    /// `true` when `buf[index]` was in range and received the zero byte.
    pub wrote: bool,
    /// `true` when `guard & 0x80000000 == 0`, i.e. the shipped range-check
    /// failure helper would run (non-returning).
    pub fatal: bool,
}

/// `0x006101c3` (`FUN_006101c3`, 44 bytes, `decompiled_uncited`).
///
/// ```c
/// iVar1 = new_length;
/// *(undefined1 *)(iVar1 + *(int *)(unaff_EDI + 8)) = 0;  /* data[len] = 0 */
/// *(int *)(unaff_EDI + 0xc) = iVar1;                     /* +0x0c = len  */
/// if ((*(uint *)(frame - 0x24) & 0x80000000) == 0) FUN_0093ec70();
/// ```
///
/// The write and the length store both happen before the guard is tested.
pub fn a63_terminate_006101c3(buf: &mut [u8], new_len: u32, guard: u32) -> A63Terminate {
    let index = new_len;
    let wrote = match buf.get_mut(index as usize) {
        Some(byte) => {
            *byte = 0;
            true
        }
        None => false,
    };
    A63Terminate {
        index,
        len: index,
        wrote,
        fatal: guard & 0x8000_0000 == 0,
    }
}

// ---------------------------------------------------------------------------
// Eight-mode dword mixer (0x00610970)
// ---------------------------------------------------------------------------

/// Mode bytes consumed by [`a63_mix_dwords`]: the shipped table keeps 256
/// dwords of substitution data in `table[0..0x400]` and the per-index mode
/// byte in the byte at `table + i`, so the walk starts at `i = 0x400`.
pub const A63_MIX_FIRST_MODE: i32 = 0x400;

/// `0x00610970` (`FUN_00610970`, 28 bytes, `decompiled_uncited`).
///
/// The body mixes `n >> 2` dwords from `data` into a running accumulator,
/// applying one operation per mode byte while `0x400 <= i < len`:
///
/// | mode | operation (accumulator `a`, word `w`)                        |
/// |------|--------------------------------------------------------------|
/// | 0    | `a ^= w`                                                      |
/// | 1    | `a = a + w`                                                   |
/// | 2    | `a = a - w`                                                   |
/// | 3-5  | `a ^= S[w>>24] ^ S[w & 0xff] ^ S[(w>>8)&0xff] ^ S[(w>>16)&0xff]` |
/// | 6    | `a = rol(a, w & 0x1f)`, then the other three bytes in order      |
/// | 7    | same four rotations to the right                              |
/// | other| no change                                                    |
///
/// The rotate modes use the low five bits of each byte (`& 0x1f`), matching
/// the `rol`/`ror` encodings in the shipped body.  The dword walk stops after
/// `n >> 2` words, so a trailing partial word is never consumed; `n <= 3`
/// leaves the accumulator at zero.
pub fn a63_mix_dwords(sbox: &[u32; 256], modes: &[u8], len: i32, data: &[u32], n: u32) -> u32 {
    let mut acc: u32 = 0;
    if n <= 3 {
        return 0;
    }
    let words = (n >> 2) as usize;
    for &word in data.iter().take(words) {
        if len > A63_MIX_FIRST_MODE {
            let mut i = A63_MIX_FIRST_MODE;
            while i < len {
                match modes[i as usize] {
                    0 => acc ^= word,
                    1 => acc = acc.wrapping_add(word),
                    2 => acc = acc.wrapping_sub(word),
                    3 | 4 | 5 => {
                        acc ^= sbox[(word >> 24) as usize]
                            ^ sbox[(word & 0xff) as usize]
                            ^ sbox[((word >> 8) & 0xff) as usize]
                            ^ sbox[((word >> 16) & 0xff) as usize];
                    }
                    6 => {
                        for shift in [0u32, 8, 16, 24] {
                            acc = acc.rotate_left((word >> shift) & 0x1f);
                        }
                    }
                    7 => {
                        for shift in [0u32, 8, 16, 24] {
                            acc = acc.rotate_right((word >> shift) & 0x1f);
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        }
    }
    acc
}

// ---------------------------------------------------------------------------
// Clamped string length (0x00610d30)
// ---------------------------------------------------------------------------

/// Result of `0x00610d30`: the clamped length and whether a terminator was
/// actually found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A63ClampedLen {
    /// `min(strlen, 0x7f)`.
    pub len: u32,
    /// `false` when the scan ran off the end of the slice; the shipped body
    /// would then keep reading past the buffer.
    pub terminated: bool,
}

/// `0x00610d30` (`FUN_00610d30`, 46 bytes, `decompiled_uncited`).
///
/// ```c
/// do { cVar1 = *pcVar3; pcVar3 = pcVar3 + 1; } while (cVar1 != '\0');
/// uVar2 = 0x7f;
/// if ((uint)((int)pcVar3 - (int)(param_2 + 1)) < 0x7f)
///     uVar2 = (int)pcVar3 - (int)(param_2 + 1);
/// FUN_00c89580(param_1, param_2, uVar2);
/// ```
///
/// The recovered count is the `strlen` of the source clamped to `0x7f`; it is
/// the length argument of the out-of-window copy helper.
pub fn a63_strlen_00610d30(bytes: &[u8]) -> A63ClampedLen {
    match bytes.iter().position(|&b| b == 0) {
        Some(index) => A63ClampedLen {
            len: (index as u32).min(0x7f),
            terminated: true,
        },
        None => A63ClampedLen {
            len: 0x7f,
            terminated: false,
        },
    }
}

// ---------------------------------------------------------------------------
// Path selection (0x006116f3)
// ---------------------------------------------------------------------------

/// Which of the two exits `0x006116f3` takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A63AbortPath {
    /// `FUN_0092d4b0` runs first, then the shared non-returning tail.
    StateResetThenAbort,
    /// Straight to the non-returning tail.
    Abort,
}

/// Mask built from the out-of-window query result in `0x006116f3`:
///
/// ```c
/// iVar2 = FUN_0065f590(*(undefined4 *)(unaff_ESI + 0xfc));
/// uVar3 = -(uint)(iVar2 != 0) & 0x7ffffffa;
/// ```
///
/// The mask is therefore always `0` or `0x7ffffffa`, which is exactly the pair
/// the following guard compares against; that guard can never fall through.
pub fn a63_state_mask(query: i32) -> u32 {
    0u32.wrapping_sub(u32::from(query != 0)) & 0x7fff_fffa
}

/// `0x006116f3` (`FUN_006116f3`, 161 bytes, `decompiled_uncited`).
///
/// The three predicates are the results of the `FUN_00606d90` /
/// `FUN_00606dd0` / `FUN_00606dc0` probes; `flag` is the byte at
/// `frame - 0xae5`.  Both exits end in the same non-returning tail, so the
/// only recovered difference is whether the `FUN_0092d4b0` step runs:
///
/// ```text
/// flag && (a || b || d)   -> Abort
/// otherwise               -> StateResetThenAbort
/// ```
///
/// `d` is only *read* when `b` is clear, but a set `b` already aborts on its
/// own, so the disjunction is the whole recovered condition.
pub fn a63_path_006116f3(a: bool, b: bool, d: bool, flag: bool) -> A63AbortPath {
    if flag && (a || b || d) {
        A63AbortPath::Abort
    } else {
        A63AbortPath::StateResetThenAbort
    }
}

// ---------------------------------------------------------------------------
// Interface vtable / global pointer stores (0x006117f0, 0x00611820, 0x00611830)
// ---------------------------------------------------------------------------

/// `PTR_PTR_00f04c60`: the global pointer written by all three bodies below.
pub const A63_GLOBAL_PTR_PTR_00F04C60: u32 = 0x00f0_4c60;

/// `PTR_vftable_00f04c5c`: the static pointer whose address is stored into
/// the global by `0x006117f0` and `0x00611830`.
pub const A63_PTR_VFTABLE_00F04C5C: u32 = 0x00f0_4c5c;

/// Recovered interface identity; the shipped bodies store the symbol
/// `IGameLobby::vftable` into `+0x00`, whose numeric address is not part of
/// the decompiled body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A63Interface {
    /// `IGameLobby::vftable`.
    GameLobby,
}

/// Result of the three vtable stores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A63VtableStore {
    /// Value stored into the object's `+0x00`.
    pub this_vtable: A63Interface,
    /// Value stored into the global at `0x00f04c60`.
    pub global: u32,
    /// `0x006117f0` only: `true` when `param_1 & 1` selects the free call.
    pub free: bool,
}

/// `0x006117f0` (`FUN_006117f0`, 43 bytes, `decompiled_uncited`).
///
/// ```c
/// PTR_PTR_00f04c60 = (undefined *)&PTR_vftable_00f04c5c;
/// *in_ECX = IGameLobby::vftable;
/// if ((param_1 & 1) != 0) FUN_00923e90();   /* operator delete */
/// ```
pub fn a63_vtable_store_006117f0(flags: u8) -> A63VtableStore {
    A63VtableStore {
        this_vtable: A63Interface::GameLobby,
        global: A63_PTR_VFTABLE_00F04C5C,
        free: flags & 1 != 0,
    }
}

/// `0x00611820` (`FUN_00611820`, 15 bytes, `decompiled_uncited`).
///
/// ```c
/// *in_ECX = IGameLobby::vftable;
/// PTR_PTR_00f04c60 = (undefined *)in_ECX;
/// ```
///
/// `this_address` is the value the global receives.
pub fn a63_vtable_store_00611820(this_address: u32) -> A63VtableStore {
    A63VtableStore {
        this_vtable: A63Interface::GameLobby,
        global: this_address,
        free: false,
    }
}

/// `0x00611830` (`FUN_00611830`, 17 bytes, `decompiled_uncited`).
///
/// ```c
/// *in_ECX = IGameLobby::vftable;
/// PTR_PTR_00f04c60 = (undefined *)&PTR_vftable_00f04c5c;
/// ```
pub fn a63_vtable_store_00611830() -> A63VtableStore {
    A63VtableStore {
        this_vtable: A63Interface::GameLobby,
        global: A63_PTR_VFTABLE_00F04C5C,
        free: false,
    }
}

// ---------------------------------------------------------------------------
// Low-bit encoded slot release (0x00611850, 0x00611d60)
// ---------------------------------------------------------------------------

/// What the low bit of the slot word selects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A63SlotTarget {
    /// `word & 1 != 0` and `word & !1 == 0`: nothing is called.
    Empty,
    /// `word & 1 == 0`: the object lives inline at `this + 0x10`, so the
    /// address is always non-null and the virtual call is always issued.
    Inline,
    /// `word & 1 != 0`: heap object at `word & 0xfffffffe`.
    Heap(u32),
}

/// One pass of the recovered release block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A63SlotRelease {
    /// Object selected by the low bit before the call.
    pub target: A63SlotTarget,
    /// `true` when the virtual destructor was invoked.
    pub called: bool,
    /// Slot word afterwards: untouched when nothing was called, the value
    /// produced by the call when it succeeded, and the raw post-call word
    /// when the guard fired.
    pub word: u32,
    /// `true` when the post-call word still had its low bit set; the shipped
    /// body then takes the non-returning range-check failure.
    pub fatal: bool,
}

/// One release block, exactly as written twice in `0x00611850` and once in
/// `0x00611d60`:
///
/// ```c
/// if ((*(byte *)(slot) & 1) == 0) p = slot;             /* inline address */
/// else                            p = *(int *)slot & 0xfffffffe;
/// if (p != 0) {
///     (**(code **)*p)(0);
///     if ((*(byte *)(slot) & 1) != 0) FUN_0093ec70();   /* non-returning */
///     *(int *)slot = 1;
/// }
/// ```
///
/// The post-call flag test re-reads the slot, so `after` returns the word the
/// destructor left behind.
pub fn a63_release_slot(word: u32, mut after: impl FnMut() -> u32) -> A63SlotRelease {
    let target = if word & 1 != 0 {
        match word & 0xffff_fffe {
            0 => A63SlotTarget::Empty,
            heap => A63SlotTarget::Heap(heap),
        }
    } else {
        A63SlotTarget::Inline
    };
    if target == A63SlotTarget::Empty {
        return A63SlotRelease {
            target,
            called: false,
            word,
            fatal: false,
        };
    }
    let produced = after();
    if produced & 1 != 0 {
        A63SlotRelease {
            target,
            called: true,
            word: produced,
            fatal: true,
        }
    } else {
        A63SlotRelease {
            target,
            called: true,
            word: 1,
            fatal: false,
        }
    }
}

/// Result of `0x00611850`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A63EntityCleanup {
    /// `+0x08`: cleared before anything else.
    pub byte8: u8,
    /// `+0x00`: handle after the conditional close (`-1` once closed).
    pub handle: i32,
    /// `true` when the close helper was consulted and reported success.
    pub closed: bool,
    /// Release passes that ran; the shipped body stops at a fatal one.
    pub releases: Vec<A63SlotRelease>,
    /// `+0x04`: cleared between the two release blocks; left at the incoming
    /// value when the first release takes the non-returning guard.
    pub word4: u32,
}

impl A63EntityCleanup {
    /// `true` when a release ended in the non-returning guard.
    pub fn a63_fatal(&self) -> bool {
        self.releases.iter().any(|r| r.fatal)
    }
}

/// `0x00611850` (`FUN_00611850`, 197 bytes, `decompiled_uncited`).
///
/// ```c
/// *(byte *)(in_ECX + 2) = 0;                       /* +0x08          */
/// if (*in_ECX != -1 && FUN_00609780(...) != 0) {   /* close possible */
///     FUN_0087be60((short)*in_ECX); *in_ECX = -1;
/// }
/// <release block>                                   /* slot at +0x10  */
/// in_ECX[1] = 0;                                    /* +0x04          */
/// <release block again>
/// ```
///
/// `can_close` is the `FUN_00609780` result; `after` produces the slot word
/// the virtual destructor leaves behind.  `word4_before` is only kept when the
/// first release takes the non-returning guard, because the clear of `+0x04`
/// sits between the two blocks.
pub fn a63_cleanup_00611850(
    handle: i32,
    word4_before: u32,
    slot_word: u32,
    can_close: bool,
    mut after: impl FnMut() -> u32,
) -> A63EntityCleanup {
    let mut handle = handle;
    let mut closed = false;
    if handle != -1 && can_close {
        closed = true;
        handle = -1;
    }
    let mut word = slot_word;
    let mut releases = Vec::new();
    let first = a63_release_slot(word, &mut after);
    word = first.word;
    let fatal = first.fatal;
    releases.push(first);
    if fatal {
        return A63EntityCleanup {
            byte8: 0,
            handle,
            closed,
            releases,
            word4: word4_before,
        };
    }
    let second = a63_release_slot(word, &mut after);
    releases.push(second);
    A63EntityCleanup {
        byte8: 0,
        handle,
        closed,
        releases,
        word4: 0,
    }
}

/// Result of `0x00611d60`: the shortened single-release variant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A63EntityCleanupShort {
    /// `+0x08`: cleared first.
    pub byte8: u8,
    /// `+0x00` after the conditional close.
    pub handle: i32,
    /// `true` when the close helper was consulted and reported success.
    pub closed: bool,
    /// The single release pass.
    pub release: A63SlotRelease,
    /// `+0x04`: cleared at the end when the release survives.
    pub word4: u32,
}

/// `0x00611d60` (`FUN_00611d60`, 98 bytes, `decompiled_uncited`).
///
/// The same body as the first half of `0x00611850`: clear the byte at
/// `+0x08`, close the handle when the query allows it and the handle is not
/// `-1`, run one release block and clear `+0x04`.  The clear only happens when
/// the release survives, so `word4_before` is kept on the fatal path.
pub fn a63_cleanup_00611d60(
    handle: i32,
    word4_before: u32,
    slot_word: u32,
    can_close: bool,
    mut after: impl FnMut() -> u32,
) -> A63EntityCleanupShort {
    let mut handle = handle;
    let mut closed = false;
    if handle != -1 && can_close {
        closed = true;
        handle = -1;
    }
    let release = a63_release_slot(slot_word, &mut after);
    A63EntityCleanupShort {
        byte8: 0,
        handle,
        closed,
        word4: if release.fatal { word4_before } else { 0 },
        release,
    }
}

// ---------------------------------------------------------------------------
// Layout validation (0x00611950)
// ---------------------------------------------------------------------------

/// Triple produced by the out-of-window `vtable + 8` query in `0x00611950`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A63SourceProbe {
    /// Frame slot `-0x34`.
    pub id: u32,
    /// Frame slot `-0x30`.
    pub kind: u32,
    /// Frame slot `-0x2c`; only `0` is rejected.
    pub valid: u8,
}

/// Rejection reasons recovered from `0x00611950`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A63SourceReject {
    /// `param_1 == in_ECX`, reported with the non-returning helper.
    SelfAssignment,
    /// `!valid || id > 0x14 || kind > 4`, reported with the range-check
    /// helper.
    RangeCheck,
}

/// Accepted state of the copy in `0x00611950`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A63SourceCopy {
    /// `raw & 0xfffffffe` when the low bit was set, otherwise `raw`.
    pub deref: u32,
    /// Builder word initialised to `1` before the query.
    pub builder: u32,
    /// `+0x0c` stage byte: `0` until the query succeeds, then `1`.
    pub stage: u8,
    /// `true` when the `vtable + 0x10` call received the builder.
    pub dispatched: bool,
}

/// `0x00611950` (`FUN_00611950`, 164 bytes, `decompiled_uncited`).
///
/// ```c
/// if (param_1 == in_ECX) FUN_00c86b6e();            /* non-returning */
/// local_28[0] = 1;                                  /* builder       */
/// if ((*param_1 & 1) != 0) param_1 = (uint *)(*param_1 & 0xfffffffe);
/// if (param_1 != 0) {
///     (**(code **)(*param_1 + 8))(&local_34, cookie);   /* id/kind/valid */
///     if ((local_2c == 0) || (0x14 < local_34) || (4 < local_30)) FUN_0093ec70();
///     local_8 = CONCAT31(local_8._1_3_, 1);             /* stage = 1     */
///     (**(code **)(*param_1 + 0x10))(local_28);        /* builder       */
/// }
/// FUN_005426e0(local_28);                            /* non-returning */
/// ```
///
/// A null source after the low-bit mask skips both calls and leaves the stage
/// at zero.
pub fn a63_source_check_00611950(
    same_object: bool,
    raw: u32,
    probe: A63SourceProbe,
) -> Result<A63SourceCopy, A63SourceReject> {
    if same_object {
        return Err(A63SourceReject::SelfAssignment);
    }
    let deref = if raw & 1 != 0 { raw & 0xffff_fffe } else { raw };
    if deref == 0 {
        return Ok(A63SourceCopy {
            deref,
            builder: 1,
            stage: 0,
            dispatched: false,
        });
    }
    if probe.valid == 0 || probe.id > 0x14 || probe.kind > 4 {
        return Err(A63SourceReject::RangeCheck);
    }
    Ok(A63SourceCopy {
        deref,
        builder: 1,
        stage: 1,
        dispatched: true,
    })
}

// ---------------------------------------------------------------------------
// Callable-object copy (0x00611de0)
// ---------------------------------------------------------------------------

/// The two vtable symbols the copy writes in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A63CopyableVtable {
    /// `_bcCopyableFunctionImpl<void>::vftable`
    CopyableFunctionImplVoid,
    /// `_bcCopyableFunctionObjectImpl<<lambda...>,void>::vftable`
    CopyableFunctionObjectLambda,
}

/// `0x00611de0` (`FUN_00611de0`, 28 bytes, `decompiled_uncited`).
///
/// ```c
/// *param_1 = _bcCopyableFunctionImpl<void>::vftable;
/// param_1[1] = *(undefined4 *)(in_ECX + 4);
/// *param_1 = _bcCopyableFunctionObjectImpl<<lambda_147ac83e5...>&,void>::vftable;
/// ```
///
/// The source word at `+0x04` is copied verbatim between the two vtable
/// stores; the second store overwrites the first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A63CopyableObject {
    /// Vtable installed first.
    pub base_vtable: A63CopyableVtable,
    /// Copied `+0x04` word.
    pub word4: u32,
    /// Vtable installed last.
    pub final_vtable: A63CopyableVtable,
}

/// See [`A63CopyableObject`].
pub fn a63_copyable_object_00611de0(source_word4: u32) -> A63CopyableObject {
    A63CopyableObject {
        base_vtable: A63CopyableVtable::CopyableFunctionImplVoid,
        word4: source_word4,
        final_vtable: A63CopyableVtable::CopyableFunctionObjectLambda,
    }
}

// ---------------------------------------------------------------------------
// Slot-1 emptiness branch (0x00611fa0)
// ---------------------------------------------------------------------------

/// The two non-returning outcomes of `0x00611fa0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A63SlotOutcome {
    /// `+0x04 == 0`: the report helper is called with the word at `+0x00`.
    ReportCode(u32),
    /// `+0x04 != 0`: the range-check failure helper is called.
    InvalidArgument,
}

/// `0x00611fa0` (`FUN_00611fa0`, 29 bytes, `decompiled_uncited`).
///
/// ```c
/// if (in_ECX[1] == 0) FUN_0053e930(*in_ECX);
/// else                FUN_0093ec70();
/// ```
pub fn a63_slot_outcome_00611fa0(slot0: u32, slot1: u32) -> A63SlotOutcome {
    if slot1 == 0 {
        A63SlotOutcome::ReportCode(slot0)
    } else {
        A63SlotOutcome::InvalidArgument
    }
}

// ---------------------------------------------------------------------------
// Hash table insert (0x00611fbd)
// ---------------------------------------------------------------------------

/// Recovered chain node: `next` at `+0x00`, key dword at `+0x04`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A63ChainNode {
    /// `+0x00`: next node index, `None` for the end of the chain.
    pub next: Option<usize>,
    /// `+0x04`: key compared by the walk.
    pub key: u32,
}

/// Link that receives the new node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A63PrevLink {
    /// The bucket head itself was empty.
    Bucket,
    /// The tail node of the bucket chain.
    Node(usize),
}

/// Failure exits of `0x00611fbd`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A63InsertError {
    /// Key already present; the shipped body takes the range-check helper.
    Duplicate,
    /// Load factor above the stored limit; the shipped body calls its growth
    /// helper with `capacity * 2`.
    Grow(u32),
}

/// Accepted insert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A63Insert {
    /// Index the fresh node would occupy (the body allocates 8 bytes).
    pub node: usize,
    /// Link that receives the node.
    pub prev: A63PrevLink,
    /// Selected bucket.
    pub bucket: usize,
    /// `+0x08` count after the increment.
    pub count: u32,
    /// `table + capacity * 4`, the one-past-the-end pointer written to the
    /// second result dword.
    pub table_end: u32,
    /// Byte written to `result + 0x0c` (`1`).
    pub incorporated: u8,
}

/// `0x00611fbd` (`FUN_00611fbd`, 201 bytes, `decompiled_uncited`).
///
/// Bucket selection is MurmurHash3's 32-bit finaliser followed by the usual
/// power-of-two mask:
///
/// ```c
/// uVar9 = (key >> 0x10 ^ key) * -0x7a143595;      /* 0x85ebca6b */
/// uVar9 = (uVar9 >> 0xd ^ uVar9) * -0x3d4d51cb;   /* 0xc2b2ae35 */
/// bucket = (uVar9 >> 0x10 ^ uVar9) & (capacity - 1);
/// ```
///
/// The chain walks while `key != node->key`, keeps the last link, and inserts
/// only when that link is null.  The load factor is recomputed as
/// `(float)(double)count / (float)(double)capacity` and compared with the
/// stored float limit; above it the body requests `capacity * 2`.
pub fn a63_hash_insert(
    capacity: u32,
    table: u32,
    count: u32,
    limit: f32,
    key: u32,
    heads: &[Option<usize>],
    nodes: &[A63ChainNode],
) -> Result<A63Insert, A63InsertError> {
    let bucket = (a63_fmix32(key) & capacity.wrapping_sub(1)) as usize;
    let mut prev = A63PrevLink::Bucket;
    let mut cursor = heads.get(bucket).copied().flatten();
    while let Some(index) = cursor {
        if nodes[index].key == key {
            return Err(A63InsertError::Duplicate);
        }
        prev = A63PrevLink::Node(index);
        cursor = nodes[index].next;
    }
    let new_count = count.wrapping_add(1);
    let load = ((new_count as f64) as f32) / ((capacity as f64) as f32);
    if !(load <= limit) {
        return Err(A63InsertError::Grow(capacity.wrapping_mul(2)));
    }
    Ok(A63Insert {
        node: nodes.len(),
        prev,
        bucket,
        count: new_count,
        table_end: table.wrapping_add(capacity.wrapping_mul(4)),
        incorporated: 1,
    })
}

/// MurmurHash3 x86-32 finaliser, exactly as used by `0x00611fbd`.
pub fn a63_fmix32(mut h: u32) -> u32 {
    h ^= h >> 16;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    h
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent63 {
    use super::*;

    /// `0x00610620` / `0x00610640`: the byte setter leaves the other seven
    /// payload bytes alone, the qword setter writes all eight little-endian.
    #[test]
    fn a63_value_tag5_and_tag4() {
        let mut byte = A63Value {
            tag: 9,
            payload: [0xaa; 8],
        };
        byte.a63_set_tag5_byte(0xab);
        assert_eq!(byte.tag, 5);
        assert_eq!(
            byte.payload,
            [0xab, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa]
        );
        assert_eq!(byte.a63_pair(), None);

        let mut qword = A63Value {
            tag: 9,
            payload: [0xaa; 8],
        };
        qword.a63_set_tag4_qword(0x1122_3344_5566_7788);
        assert_eq!(qword.tag, 4);
        assert_eq!(
            qword.payload,
            [0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]
        );
        assert_eq!(qword.a63_pair(), None);
    }

    /// `0x00610660` / `0x00610730`: identical payload, tags 2 and 3.
    #[test]
    fn a63_value_pair_tags() {
        let mut a = A63Value {
            tag: 0,
            payload: [0xaa; 8],
        };
        a.a63_set_tag2_pair(0x0102_0304, 0x0506_0708);
        assert_eq!(a.tag, 2);
        assert_eq!(a.a63_pair(), Some((0x0102_0304, 0x0506_0708)));
        assert_eq!(
            a.payload,
            [4, 3, 2, 1, 8, 7, 6, 5],
            "both dwords are stored little-endian"
        );

        let mut b = A63Value {
            tag: 0,
            payload: [0xaa; 8],
        };
        b.a63_set_tag3_pair(0xdead_beef, 0);
        assert_eq!(b.tag, 3);
        assert_eq!(b.a63_pair(), Some((0xdead_beef, 0)));
        assert_eq!(b.payload, [0xef, 0xbe, 0xad, 0xde, 0, 0, 0, 0]);
    }

    /// `0x00610070`: the slot index is `tag - 1` from the table at
    /// `0x00610340`.
    #[test]
    fn a63_dispatch_00610070_uses_tag_minus_one() {
        assert_eq!(a63_dispatch_00610070(1), 0x0061_0340);
        assert_eq!(a63_dispatch_00610070(2), 0x0061_0344);
        assert_eq!(a63_dispatch_00610070(5), 0x0061_0350);
        assert_eq!(a63_dispatch_00610070(0), 0x0061_033c);
        assert_eq!(a63_dispatch_00610070(-1), 0x0061_0338);
    }

    /// `0x00610750`, `0x006107d0`, `0x00610830`, `0x00610910`: four tables
    /// indexed by the tag with no bias.
    #[test]
    fn a63_tag_tables_are_indexed_directly() {
        assert_eq!(a63_dispatch_00610750(0), 0x0061_07b4);
        assert_eq!(a63_dispatch_00610750(3), 0x0061_07c0);
        assert_eq!(a63_dispatch_006107d0(3), 0x0061_0818);
        assert_eq!(a63_dispatch_00610830(2), 0x0061_08f8);
        assert_eq!(a63_dispatch_00610910(3), 0x0061_0958);
        assert_eq!(a63_dispatch_00610910(-1), 0x0061_0948);
    }

    /// `0x00610410` with its `0x00610040` forwarder: the guard only fires for
    /// destination tag `1` with a non-negative `+0x10`.
    #[test]
    fn a63_variant_assign_guard_and_slot() {
        let ok = a63_variant_assign(0, 0, 3).unwrap();
        assert_eq!(ok.cleared, 0);
        assert_eq!(ok.tag, 3);
        assert_eq!(ok.jump_target, 0x0061_0500 + 3 * 4);

        let err = a63_variant_assign(1, 0, 0).unwrap_err();
        assert_eq!(err.tag_minus_one, 0);
        assert_eq!(err.word10, 0);

        // Tag 1 is fine when +0x10 is negative.
        assert!(a63_variant_assign(1, 0x8000_0000, 2).is_ok());
        // Any other tag never checks +0x10.
        assert!(a63_variant_assign(4, 0, 2).is_ok());
        assert!(a63_variant_assign(0xffff_ffff, 0, 2).is_ok());

        assert_eq!(a63_variant_assign_00610040(0, 0, 4).unwrap().tag, 4);
        assert_eq!(
            a63_variant_assign_00610040(1, 0x7fff_ffff, 4),
            Err(A63AssignFatal {
                tag_minus_one: 0,
                word10: 0x7fff_ffff,
            })
        );
    }

    /// `0x00610520` with its `0x00610060` forwarder.
    #[test]
    fn a63_tag_compare_masks_or_dispatches() {
        assert_eq!(
            a63_tag_compare(0x0000_0001, 0x0000_0002),
            A63TagCompare::Different(0)
        );
        assert_eq!(
            a63_tag_compare(0x1234_5678, 0),
            A63TagCompare::Different(0x1234_5600)
        );
        assert_eq!(
            a63_tag_compare(3, 3),
            A63TagCompare::Same {
                word: 3,
                jump_target: 0x0061_0614,
            }
        );
        assert_eq!(
            a63_tag_compare_00610060(0xffff_ffff, 0xffff_ffff),
            A63TagCompare::Same {
                word: 0xffff_ffff,
                jump_target: a63_slot_target(0x0061_0608, -1),
            }
        );
    }

    /// `0x006103c0`: the empty inline header plus the request read from the
    /// source at `+0x04`.
    #[test]
    fn a63_string_reset_header() {
        let reset = a63_string_reset_006103c0(0x40);
        assert_eq!(reset.requested, 0x40);
        assert_eq!(reset.header, A63StringHeader::a63_empty_inline());
        assert_eq!(reset.header.len, 0);
        assert_eq!(reset.header.cap, 0x8000_000f);
        assert_eq!(reset.header.first, 0);
        assert!(reset.header.inline_at_14);

        assert_eq!(a63_string_reset_006103c0(0).header, reset.header);
    }

    /// `0x00610680`: the same header with a cleared tag in front of it.
    #[test]
    fn a63_record_init_clears_tag_and_string() {
        let init = a63_record_init_00610680(23);
        assert_eq!(init.tag, 0);
        assert_eq!(init.requested, 23);
        assert_eq!(init.header.cap, A63_STRING_CAP);
        assert_eq!(init.header.len, 0);
        assert!(init.header.inline_at_14);
    }

    /// `0x006101c3`: the zero byte lands at the new length and the length is
    /// stored back, both before the guard is tested.
    #[test]
    fn a63_terminate_writes_then_checks() {
        let mut buf = [1u8, 2, 3, 4];
        let ok = a63_terminate_006101c3(&mut buf, 2, 0x8000_0000);
        assert_eq!((ok.index, ok.len), (2, 2));
        assert!(ok.wrote);
        assert!(!ok.fatal);
        assert_eq!(buf, [1, 2, 0, 4]);

        let mut short = [9u8];
        let bad = a63_terminate_006101c3(&mut short, 3, 0);
        assert!(!bad.wrote, "index past the buffer is not written");
        assert!(bad.fatal);
        assert_eq!(bad.len, 3);
    }

    /// `0x00610970`: modes 0..2 on a one-word payload.
    #[test]
    fn a63_mix_arithmetic_modes() {
        let mut sbox = [0u32; 256];
        for (i, slot) in sbox.iter_mut().enumerate() {
            *slot = i as u32;
        }
        let mut modes = vec![0u8; 0x401];
        let data = [0xdead_beefu32];

        modes[0x400] = 0;
        assert_eq!(a63_mix_dwords(&sbox, &modes, 0x401, &data, 4), 0xdead_beef);

        modes[0x400] = 1;
        assert_eq!(a63_mix_dwords(&sbox, &modes, 0x401, &data, 4), 0xdead_beef);

        modes[0x400] = 2;
        assert_eq!(
            a63_mix_dwords(&sbox, &modes, 0x401, &data, 4),
            0u32.wrapping_sub(0xdead_beef)
        );

        modes[0x400] = 3;
        let w: u32 = 0x0102_0304;
        assert_eq!(a63_mix_dwords(&sbox, &modes, 0x401, &[w], 4), 4 ^ 3 ^ 2 ^ 1);

        // Modes 3..5 share one body.
        for mode in [3u8, 4, 5] {
            modes[0x400] = mode;
            assert_eq!(a63_mix_dwords(&sbox, &modes, 0x401, &[w], 4), 4);
        }

        // Bytes with an unhandled mode leave the accumulator alone.
        modes[0x400] = 8;
        assert_eq!(a63_mix_dwords(&sbox, &modes, 0x401, &data, 4), 0);
        modes[0x400] = 0xff;
        assert_eq!(a63_mix_dwords(&sbox, &modes, 0x401, &data, 4), 0);
    }

    /// `0x00610970`: the rotate modes go byte 0 to byte 3 and mask the count
    /// to five bits; `len <= 0x400` skips the whole mode walk.
    #[test]
    fn a63_mix_rotate_modes_and_short_lengths() {
        let sbox = [0u32; 256];
        let mut modes = vec![0u8; 0x402];
        modes[0x400] = 1;
        modes[0x401] = 6;

        let w: u32 = 0x0000_0101;
        assert_eq!(a63_mix_dwords(&sbox, &modes, 0x402, &[w], 4), 0x404);
        // Word 1: `+= 1`, then four left rotations of 1, 0, 0 and 0 bits.
        assert_eq!(a63_mix_dwords(&sbox, &modes, 0x402, &[0x0000_0001], 4), 2);
        // Two words: 0x101 -> 0x404, then +0x101 -> 0x505 -> 0x1414.
        assert_eq!(a63_mix_dwords(&sbox, &modes, 0x402, &[w, w], 8), 0x1414);

        modes[0x401] = 7;
        assert_eq!(a63_mix_dwords(&sbox, &modes, 0x402, &[w], 4), 0x4000_0040);
        // Two words, right rotations: 0x101 -> 0x40000040, +0x101 -> 0x50000050.
        assert_eq!(
            a63_mix_dwords(&sbox, &modes, 0x402, &[w, w], 8),
            0x5000_0050
        );

        // Exactly 0x400 is not "greater than 0x400".
        assert_eq!(a63_mix_dwords(&sbox, &modes, 0x400, &[w], 4), 0);

        // n >> 2 words are consumed; n <= 3 leaves the accumulator at zero.
        assert_eq!(
            a63_mix_dwords(&sbox, &modes, 0x402, &[w, w], 7),
            0x4000_0040
        );
        assert_eq!(a63_mix_dwords(&sbox, &modes, 0x402, &[w], 3), 0);
    }

    /// `0x00610d30`: `min(strlen, 0x7f)` and the unterminated case.
    #[test]
    fn a63_strlen_clamps_at_0x7f() {
        assert_eq!(
            a63_strlen_00610d30(b"abc\0"),
            A63ClampedLen {
                len: 3,
                terminated: true,
            }
        );
        assert_eq!(a63_strlen_00610d30(b"\0").len, 0);
        assert_eq!(a63_strlen_00610d30(&[b'x'; 200]).len, 0x7f);
        assert_eq!(
            a63_strlen_00610d30(&[b'x'; 200]),
            A63ClampedLen {
                len: 0x7f,
                terminated: false,
            }
        );
        let mut long = vec![b'y'; 0x80];
        long.push(0);
        assert_eq!(
            a63_strlen_00610d30(&long),
            A63ClampedLen {
                len: 0x7f,
                terminated: true,
            }
        );
        let mut short = vec![b'y'; 0x7e];
        short.push(0);
        assert_eq!(a63_strlen_00610d30(&short).len, 0x7e);
    }

    /// `0x006116f3`: the flag-gated abort and the always-taken mask guard.
    #[test]
    fn a63_path_selection_and_mask() {
        assert_eq!(a63_state_mask(0), 0);
        assert_eq!(a63_state_mask(1), 0x7fff_fffa);
        assert_eq!(a63_state_mask(-1), 0x7fff_fffa);

        // flag clear: always the reset step, whatever the probes say.
        for a in [false, true] {
            for b in [false, true] {
                for d in [false, true] {
                    assert_eq!(
                        a63_path_006116f3(a, b, d, false),
                        A63AbortPath::StateResetThenAbort
                    );
                }
            }
        }

        // flag set: abort as soon as any of the three probes is set.
        assert_eq!(
            a63_path_006116f3(true, false, false, true),
            A63AbortPath::Abort
        );
        assert_eq!(
            a63_path_006116f3(true, true, true, true),
            A63AbortPath::Abort
        );
        assert_eq!(
            a63_path_006116f3(false, false, true, true),
            A63AbortPath::Abort
        );
        assert_eq!(
            a63_path_006116f3(false, true, false, true),
            A63AbortPath::Abort
        );
        assert_eq!(
            a63_path_006116f3(false, false, false, true),
            A63AbortPath::StateResetThenAbort
        );
        assert_eq!(
            a63_path_006116f3(false, true, true, true),
            A63AbortPath::Abort,
            "a set b aborts before d is ever read"
        );
    }

    /// `0x006117f0`, `0x00611820`, `0x00611830`: vtable and global stores.
    #[test]
    fn a63_vtable_and_global_stores() {
        assert_eq!(
            a63_vtable_store_006117f0(0),
            A63VtableStore {
                this_vtable: A63Interface::GameLobby,
                global: 0x00f0_4c5c,
                free: false,
            }
        );
        assert!(a63_vtable_store_006117f0(1).free);
        assert!(!a63_vtable_store_006117f0(0xfe).free);

        assert_eq!(a63_vtable_store_00611820(0x1234_5678).global, 0x1234_5678);
        assert_eq!(
            a63_vtable_store_00611830().global,
            A63_GLOBAL_PTR_PTR_00F04C60 - 4
        );
        assert_eq!(a63_vtable_store_00611830().global, A63_PTR_VFTABLE_00F04C5C);
    }

    /// `0x00611850`: close once, then two release passes; the second pass is a
    /// no-op because the first one reset the slot to `1`.
    #[test]
    fn a63_cleanup_runs_two_release_passes() {
        let cleanup = a63_cleanup_00611850(5, 0x55, 0x21, true, || 0);
        assert_eq!(cleanup.byte8, 0);
        assert_eq!(cleanup.handle, -1);
        assert!(cleanup.closed);
        assert_eq!(cleanup.word4, 0);
        assert_eq!(cleanup.releases.len(), 2);
        assert_eq!(cleanup.releases[0].target, A63SlotTarget::Heap(0x20));
        assert!(cleanup.releases[0].called);
        assert_eq!(cleanup.releases[0].word, 1);
        assert!(!cleanup.releases[0].fatal);
        assert_eq!(cleanup.releases[1].target, A63SlotTarget::Empty);
        assert!(!cleanup.releases[1].called);
        assert_eq!(cleanup.releases[1].word, 1);
        assert!(!cleanup.a63_fatal());

        // Handle already -1: no close call.
        let skipped = a63_cleanup_00611850(-1, 0x55, 0x21, true, || 0);
        assert!(!skipped.closed);
        assert_eq!(skipped.handle, -1);

        // Query refused the close.
        let refused = a63_cleanup_00611850(5, 0x55, 0x21, false, || 0);
        assert!(!refused.closed);
        assert_eq!(refused.handle, 5);

        // A fatal first release stops before `+0x04` is cleared.
        let fatal = a63_cleanup_00611850(5, 0x55, 0x21, true, || 0x31);
        assert_eq!(fatal.releases.len(), 1);
        assert!(fatal.a63_fatal());
        assert_eq!(fatal.word4, 0x55);
    }

    /// `0x00611850` / `0x00611d60`: inline targets always dispatch, and a
    /// returned odd word is the fatal case.
    #[test]
    fn a63_slot_release_targets_and_fatal() {
        let inline = a63_release_slot(0x40, || 0);
        assert_eq!(inline.target, A63SlotTarget::Inline);
        assert!(inline.called);
        assert_eq!(inline.word, 1);
        assert!(!inline.fatal);

        let empty = a63_release_slot(1, || panic!("must not run"));
        assert_eq!(empty.target, A63SlotTarget::Empty);
        assert!(!empty.called);
        assert_eq!(empty.word, 1);

        let zero = a63_release_slot(0, || 0);
        assert_eq!(zero.target, A63SlotTarget::Inline);
        assert!(zero.called);

        let fatal = a63_release_slot(0x21, || 0x31);
        assert!(fatal.fatal);
        assert_eq!(fatal.word, 0x31);

        // A destructor that leaves an even word is reset to 1.
        let reset = a63_release_slot(0x21, || 0x30);
        assert!(!reset.fatal);
        assert_eq!(reset.word, 1);
    }

    /// `0x00611d60`: one release pass only.
    #[test]
    fn a63_cleanup_short_has_one_release() {
        let cleanup = a63_cleanup_00611d60(9, 0x55, 0x21, true, || 0);
        assert_eq!(cleanup.byte8, 0);
        assert_eq!(cleanup.handle, -1);
        assert!(cleanup.closed);
        assert_eq!(cleanup.word4, 0);
        assert_eq!(cleanup.release.target, A63SlotTarget::Heap(0x20));
        assert_eq!(cleanup.release.word, 1);

        let untouched = a63_cleanup_00611d60(-1, 0x55, 1, false, || panic!("no call"));
        assert!(!untouched.closed);
        assert!(!untouched.release.called);
        assert_eq!(untouched.word4, 0, "cleared when the release survives");

        let fatal = a63_cleanup_00611d60(5, 0x55, 0x21, true, || 0x31);
        assert!(fatal.release.fatal);
        assert_eq!(fatal.word4, 0x55, "the clear is skipped on the fatal path");
    }

    /// `0x00611950`: self-assignment, the low-bit mask and the two limits.
    #[test]
    fn a63_source_check_limits() {
        let probe = A63SourceProbe {
            id: 0x14,
            kind: 4,
            valid: 1,
        };
        assert_eq!(
            a63_source_check_00611950(true, 0x21, probe),
            Err(A63SourceReject::SelfAssignment)
        );

        let ok = a63_source_check_00611950(false, 0x21, probe).unwrap();
        assert_eq!(ok.deref, 0x20);
        assert_eq!(ok.builder, 1);
        assert_eq!(ok.stage, 1);
        assert!(ok.dispatched);

        // Low bit already clear: the raw word is the object address.
        assert_eq!(
            a63_source_check_00611950(false, 0x20, probe).unwrap().deref,
            0x20
        );

        // Null source: no query, no dispatch, stage stays 0.
        let null = a63_source_check_00611950(
            false,
            0,
            A63SourceProbe {
                id: 0,
                kind: 0,
                valid: 0,
            },
        )
        .unwrap();
        assert_eq!(null.deref, 0);
        assert_eq!(null.stage, 0);
        assert!(!null.dispatched);

        for bad in [
            A63SourceProbe {
                id: 0x15,
                kind: 0,
                valid: 1,
            },
            A63SourceProbe {
                id: 0,
                kind: 5,
                valid: 1,
            },
            A63SourceProbe {
                id: 0,
                kind: 0,
                valid: 0,
            },
        ] {
            assert_eq!(
                a63_source_check_00611950(false, 0x21, bad),
                Err(A63SourceReject::RangeCheck)
            );
        }
    }

    /// `0x00611de0`: both vtable stores are recorded, the copy happens
    /// between them.
    #[test]
    fn a63_copyable_object_order() {
        let object = a63_copyable_object_00611de0(0x1122_3344);
        assert_eq!(
            object.base_vtable,
            A63CopyableVtable::CopyableFunctionImplVoid
        );
        assert_eq!(object.word4, 0x1122_3344);
        assert_eq!(
            object.final_vtable,
            A63CopyableVtable::CopyableFunctionObjectLambda
        );
    }

    /// `0x00611fa0`: the branch on the word at `+0x04`.
    #[test]
    fn a63_slot_outcome_branches_on_slot1() {
        assert_eq!(
            a63_slot_outcome_00611fa0(0x1234, 0),
            A63SlotOutcome::ReportCode(0x1234)
        );
        assert_eq!(
            a63_slot_outcome_00611fa0(0, 1),
            A63SlotOutcome::InvalidArgument
        );
    }

    /// `0x00611fbd`: the MurmurHash3 finaliser values used for bucket
    /// selection.
    #[test]
    fn a63_fmix32_matches_murmur3() {
        assert_eq!(a63_fmix32(0), 0);
        assert_eq!(a63_fmix32(1), 0x514e_28b7);
        assert_eq!(a63_fmix32(2), 0x30f4_c306);
        assert_eq!(a63_fmix32(0xdead_beef), 0x0de5_c6a9);
        assert_eq!(a63_fmix32(0xffff_ffff), 0x81f1_6f39);
        assert_eq!(a63_fmix32(1) & 7, 7);
    }

    /// `0x00611fbd`: empty bucket insert, the `table_end` pointer and the
    /// `0x0c` marker byte.
    #[test]
    fn a63_hash_insert_into_empty_bucket() {
        let heads = [None; 8];
        let insert = a63_hash_insert(8, 0x1000, 0, 0.75, 1, &heads, &[]).unwrap();
        assert_eq!(insert.node, 0);
        assert_eq!(insert.prev, A63PrevLink::Bucket);
        assert_eq!(insert.bucket, 7);
        assert_eq!(insert.count, 1);
        assert_eq!(insert.table_end, 0x1020);
        assert_eq!(insert.incorporated, 1);
    }

    /// `0x00611fbd`: the chain walk stops at a matching key and appends after
    /// the tail otherwise.
    #[test]
    fn a63_hash_insert_walks_the_chain() {
        let mut heads = [None; 8];
        heads[7] = Some(0);
        let nodes = [
            A63ChainNode {
                next: Some(1),
                key: 0x99,
            },
            A63ChainNode {
                next: None,
                key: 0x100,
            },
        ];

        let insert = a63_hash_insert(8, 0x2000, 2, 0.75, 1, &heads, &nodes).unwrap();
        assert_eq!(insert.prev, A63PrevLink::Node(1));
        assert_eq!(insert.node, 2);
        assert_eq!(insert.count, 3);

        // A matching key in the same bucket reports the duplicate.
        let duplicate = a63_hash_insert(
            8,
            0x2000,
            2,
            0.75,
            1,
            &heads,
            &[A63ChainNode { next: None, key: 1 }],
        );
        assert_eq!(duplicate, Err(A63InsertError::Duplicate));
    }

    /// `0x00611fbd`: `count / capacity <= limit` inserts, above it the body
    /// asks for `capacity * 2`.
    #[test]
    fn a63_hash_insert_load_factor() {
        let heads = [None; 8];
        // 6 / 8 == 0.75 exactly, and the comparison includes equality.
        assert!(a63_hash_insert(8, 0, 5, 0.75, 2, &heads, &[]).is_ok());
        assert_eq!(
            a63_hash_insert(8, 0, 6, 0.75, 2, &heads, &[]),
            Err(A63InsertError::Grow(16))
        );
        // NaN never satisfies `load <= limit`.
        assert_eq!(
            a63_hash_insert(8, 0, 0, f32::NAN, 2, &heads, &[]),
            Err(A63InsertError::Grow(16))
        );
        // A limit of 1.0 accepts the full table.
        assert!(a63_hash_insert(8, 0, 7, 1.0, 2, &heads, &[]).is_ok());
    }
}

// ---------------------------------------------------------------------------
// Rows in 0x00610000..=0x00611fff that this batch does not translate
// ---------------------------------------------------------------------------
//
// | address      | reason                                                    |
// |--------------|-----------------------------------------------------------|
// | `FUN_0061006e` | 1-byte `__break` (int3 padding), no body.                 |
// | `FUN_0061010e` | overlapping instructions at `0x0061011b`/`0x0061011d`;    |
// |              | body reads `unaff_EBP`/`unaff_EDI` and turns into a       |
// |              | non-returning helper with no recoverable inputs.          |
// | `FUN_006101ef` | single out-of-window virtual call at `vtable + 0x10`      |
// |              | followed by the unwind tail; no arithmetic to reproduce.  |
// | `FUN_0061098e` | decompiler emitted `out` and self-modifying char stores   |
// |              | (overlap at `0x0061099f`/`0x006109a0`); the same eight-    |
// |              | mode switch is reproduced exactly by `0x00610970`.        |
// | `FUN_00610d70` | SEH frame plus six out-of-window calls; ends in a         |
// |              | non-returning 0x8d-byte memset.                           |
// | `FUN_00610ea5` | frame-based body with three out-of-window calls and       |
// |              | non-returning tails.                                      |
// | `FUN_0061101a` | decompiled blob spans `0x0061..0x0063` (1842 lines) with  |
// |              | no recoverable entry body.                                |
// | `FUN_0061111a` | same blob.                                                |
// | `FUN_0061112f` | same blob.                                                |
// | `FUN_00611151` | same blob.                                                |
// | `FUN_0061123d` | stack-only stores inside that blob; the forwarded value   |
// |              | is `unaff_ESI`, so the request cannot be recovered.       |
// | `FUN_006114d6` | two out-of-window calls then the non-returning unwind;    |
// |              | no arithmetic, table or string content.                   |
// | `FUN_006117ab` | identical body to `FUN_006114d6`.                           |
// | `FUN_00611a5f` | mid-instruction fragment (byte add at `+0x5f`) followed   |
// |              | by the non-returning unwind.                              |
// | `FUN_00611cd2` | depends on `unaff_EBP`/`unaff_ESI`; only the tail call is |
// |              | recoverable.                                              |
// | `FUN_00611d4f` | 1-byte `__break` (int3 padding).                          |
// | `FUN_00611d50` | 2-byte tail call to `func_0x00611a80`; no body.           |
// | `FUN_00611e00` | 6-byte getter returning the global pointer; no            |
// |              | arithmetic, table or string behaviour.                    |
// | `FUN_00611e10` | 6-byte getter returning the lambda RTTI descriptor;       |
// |              | symbol address only.                                      |
// | `FUN_00611e20` | guarded state initialiser (byte+8 = 1, dword+4 = 5,       |
// |              | handle = request(1000)); the tail is a flag-driven         |
// |              | indirect call on an unreadable local smart pointer.       |
// | `FUN_00611ea4` | depends on `in_ZF` and `unaff_ESI`; the same release      |
// |              | pattern is covered by `0x00611850`/`0x00611d60`.          |
