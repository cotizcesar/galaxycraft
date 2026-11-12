#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent67.rs
// ===========================================================================
// Translation batch 67.  Only this file is edited by the batch.
//
// Batch 67 owns `0x00618000..=0x00619fff`.  The ledger lists ten
// `decompiled_uncited` rows in that window and every one of them is covered
// below by twenty `a67_` functions:
//
// | address      | rows -> recovered work                                        |
// |--------------|---------------------------------------------------------------|
// | `0x0061803b` | loop tail of the flattened walker -> [`a67_loop_tail`]         |
// | `0x00618493` | bias add and accumulator store -> [`a67_bias_8af9`], [`a67_accumulate`] |
// | `0x00618850` | dispatch-address mixer -> [`a67_dispatch_mix`]                 |
// | `0x00618b80` | kind gate and 64-bit scaled product -> [`a67_kind_gate`], [`a67_scaled`], [`a67_shift24`] |
// | `0x00618dfe` | address fold, state block and counters -> [`a67_fold_chain`], [`a67_state_base`], [`a67_state_counters`] |
// | `0x0061902a` | third fold step -> [`a67_fold_step_354`]                       |
// | `0x006196c6` | two table mixers, cache window, record index -> [`a67_mix_index`], [`a67_mix_word16`], [`a67_mix_pair`], [`a67_cache_hit`], [`a67_cache_stamp`], [`a67_container_index`] |
// | `0x00619b90` | frame local xor -> [`a67_local_xor`]                            |
// | `0x00619ee7` | one-byte `ret` -> [`a67_bare_ret`]                             |
// | `0x00619f94` | eight-bit rotate -> [`a67_ror5`]                               |
//
// ## Shape of the window
//
// The cached bodies are control-flow-flattened: Ghidra's linear sweep sees
// overlapping instructions (`0x00618c20`/`0x00618c23`, `0x00618b82`,
// `0x00619ee2`), nine of the ten bodies contain a `halt_baddata()` marker,
// and several rows begin one byte inside the previous instruction.  The real
// instruction stream is nevertheless intact: it is a chain of basic blocks
// that share a stack frame whose locals are `[ebp-0x34]` (scale),
// `[ebp-0x3c]` (kind), `[ebp-0x48]` (record pointer), `[ebp-0x4c]` (sub
// count), `[ebp-0x40]` (64-bit product) and `[ebp-0xa0]` (second 64-bit
// product).  Junk bytes between the blocks are `mov reg,imm` filler and
// `rdtsc`/`pop esp` noise; every helper below is taken from a coherent
// instruction sequence, not from the filler.
//
// ## Recovered constants
//
// * Every "table base" in this window is the obfuscator's folded address
//   `&DAT_000010b7 + k - 0xc93 - 0x354 - 0xeb1`, which reduces to `k -
//   0xde1`.  That is why `0xde1` maps to offset 0, `0x1101` to `0x320`,
//   `0xdf9` to `0x18`, `0x1165` to `0x384`, `0xe45` to `0x64`, `0xde5` to
//   `0x4`, `0xe21` to `0x40` and `0x1de1` to `0x1000`
//   ([`a67_fold_chain`], `0x00618be4`..`0x00618c17`).
// * `0x7ffe0de1` folds to the fixed state block base `0x7ffe0000`
//   (`0x00618e27`); `0x00618ec2` loads `0x7ffdfc43` and `0x00618ecc` adds
//   `0x3bd`, producing the same base.  The block's kind/count fields are
//   loaded at `base+0x26c`/`base+0x270` (`0x00618f48`, `0x00618f9b`).
// * The table at `0x00f7ef98` is a run of 32-bit pseudo-random constants
//   (`0xa6dc0af8`, `0x9b5ac947`, ...); its 16-bit halves are read
//   separately (`0x00618888`, `0x006197e0`, `0x00619896`) and the low 12
//   bits of an index select the entry (`0x00619887`).
// * `0x00f7ef90` is a 32-bit accumulator: the `90 ef f7 00` displacement
//   appears 160 times in `.text`, and `0x00618520` stores `edx + eax*ecx`
//   into it.
// * `0x00f04c74`/`0x00f04c78` are the two cache slots of the throttled
//   block at `0x006199f6`; both start at `0xffffffff` in `.data` and the
//   block compares them with the incoming id pair and the elapsed ticks
//   against `0x1f4`.
//
// All helpers are pure: no `World`/`SimUnit` state is touched by any row in
// the window, so no simulation field is added and no sibling file changes.

// ---------------------------------------------------------------------------
// Recovered constants
// ---------------------------------------------------------------------------

/// Loop back edge of the flattened walker (`0x00618042` jumps here only when
/// the counter is below the bound).
pub const A67_LOOP_BACK_EDGE: u32 = 0x0061_75c1;

/// Target of `0x00618042`: the throttled cache block at `0x006199f6`.
pub const A67_CACHE_BLOCK: u32 = 0x0061_99f6;

/// Seed added first by every folded-address chain (`0x00618be4` stores
/// `0xde1`; `0x00618bee` adds `0x10b7`).
pub const A67_FOLD_SEED: u32 = 0x0000_10b7;

/// The constant the fold reduces to: `seed - 0xc93 - 0x354 - 0xeb1`.
pub const A67_FOLD_BIAS: u32 = 0x0000_0de1;

/// Second fold step (`0x00618bf9`: `add eax, 0xfffff36d`).
pub const A67_FOLD_STEP_1: u32 = 0xffff_f36d;

/// Third fold step (`0x00618c04`: `add eax, 0xfffffcac`).
pub const A67_FOLD_STEP_2: u32 = 0xffff_fcac;

/// Fourth fold step (`0x00618c0f`: `add eax, 0xfffff14f`).
pub const A67_FOLD_STEP_3: u32 = 0xffff_f14f;

/// Fixed base of the state block (`0x00618e27`: `mov [ebp-0x2c], 0x7ffe0de1`).
pub const A67_STATE_BASE_SEED: u32 = 0x7ffe_0de1;

/// Second spelling of the same base (`0x00618ec2`: `mov [ebp-0x44], 0x7ffdfc43`).
pub const A67_STATE_BASE_ALT: u32 = 0x7ffd_fc43;

/// Addend that turns [`A67_STATE_BASE_ALT`] into `0x7ffe0000`
/// (`0x00618ecc`: `add eax, 0x3bd`).
pub const A67_STATE_BASE_ALT_STEP: u32 = 0x0000_03bd;

/// `base+0x26c`: kind field loaded at `0x00618f48`
/// (`8b 80 6c 02 00 00`) and by the block labelled `code_r0x00618f45`.
pub const A67_STATE_KIND: usize = 0x26c;

/// `base+0x270`: count field loaded at `0x00618f9b`
/// (`8b 80 70 02 00 00`) and by the block labelled `code_r0x00618f9a`.
pub const A67_STATE_COUNT: usize = 0x270;

/// Upper bound of the kind gate (`0x00618bc9`: `cmp eax, 6`).
pub const A67_KIND_LIMIT: u32 = 6;

/// Special kind that additionally requires a small sub count (`0x00618bd2`).
pub const A67_KIND_LAST: u32 = 5;

/// Sub-count bound applied to [`A67_KIND_LAST`] (`0x00618bd7`: `cmp [ebp-0x4c], 2`).
pub const A67_KIND_LAST_SUB: u32 = 2;

/// Right shift applied to the 64-bit product (`0x00618deb`: `psrlq xmm1, xmm0`
/// with shift `0x18` supplied by the `0xdf9` fold at `0x00618d3e`).
pub const A67_SCALE_SHIFT: u32 = 0x18;

/// Start of the 32-bit constant table whose 16-bit halves feed the mixers.
pub const A67_TABLE_BASE: u32 = 0x00f7_ef98;

/// Start of the sibling half-word table (`0x0061940c`: `[edi+0xf7ef9a]`).
pub const A67_TABLE_PAIR_BASE: u32 = 0x00f7_ef9a;

/// Index mask of the table (`0x00619887`: `and esi, 0xfff`).
pub const A67_TABLE_MASK: u32 = 0x0000_0fff;

/// Subtracted from the low half in [`a67_mix_word16`]
/// (`0x006197f6`: `lea ecx, [eax-0x93af]`).
pub const A67_MIX_BIAS: u32 = 0x0000_93af;

/// XOR applied to the combined half before the second fold in
/// [`a67_dispatch_mix`] (`0x006188a9`: `xor esi, 0x6a080000`).
pub const A67_HASH_XOR: u32 = 0x6a08_0000;

/// First cache slot (`0x006199ff`: `cmp ecx, [0xf04c74]`).
pub const A67_CACHE_SLOT0: u32 = 0x00f0_4c74;

/// Second cache slot (`0x00619a0b`: `cmp edi, [0xf04c78]`).
pub const A67_CACHE_SLOT1: u32 = 0x00f0_4c78;

/// Cache window in elapsed ticks (`0x00619a1f`: `cmp eax, 0x1f4`).
pub const A67_CACHE_WINDOW: u32 = 0x0000_01f4;

/// Node stride used by the container index the `0x006196c6` body computes as
/// `(ptr - base) / 0x1cc + 1`.
pub const A67_STRIDE_NODE: u32 = 0x0000_01cc;

/// Pair stride of the second index in the same body (`... / 0x18 + 1`).
pub const A67_STRIDE_PAIR: u32 = 0x0000_0018;

/// Size of the record copied by `rep movsb` (`0x00619954`: `mov ecx, 0x1c`).
pub const A67_RECORD_COPY: usize = 0x1c;

/// Global accumulator written by `0x00618520`.
pub const A67_ACCUMULATOR: u32 = 0x00f7_ef90;

/// Bias added at `0x00618497` (`add eax, 0x8af90000`).
pub const A67_BIAS_8AF9: u32 = 0x8af9_0000;

/// Locals reserved by the prologue at `0x00619b93` (`sub esp, 0x24`).
pub const A67_FRAME_LOCALS: u32 = 0x24;

/// XOR applied to the local at `[ebp-0xc]` (`0x00619bb7`).
pub const A67_FRAME_XOR: u32 = 0x0000_0bbf;

/// The one-byte function at `0x00619ee7` is `ret` (`0xc3`).
pub const A67_BARE_RET_OPCODE: u8 = 0xc3;

/// Rotate count of the byte operation at `0x00619fa8` (`ror ..., 0x45`).
pub const A67_ROR_COUNT: u8 = 0x45;

// ---------------------------------------------------------------------------
// Recovered value types
// ---------------------------------------------------------------------------

/// The `edx:eax` pair left by the `mul` sites of the window
/// (`0x00618c6b`, `0x0061916c`, `0x00618d33`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A67Scaled {
    /// Low dword (`eax` after `mul`).
    pub lo: u32,
    /// High dword (`edx` after `mul`).
    pub hi: u32,
}

impl A67Scaled {
    /// The 64-bit value the pair represents; used to shift products exactly
    /// as `psrlq xmm1, xmm0` does at `0x00618deb`.
    pub fn a67_join(&self) -> u64 {
        (u64::from(self.hi) << 32) | u64::from(self.lo)
    }
}

/// Result of the loop-tail compare at `0x0061803b`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A67LoopTail {
    /// `true` when the unsigned counter is still below the bound.
    pub take: bool,
    /// Address the block branches to: [`A67_LOOP_BACK_EDGE`] or
    /// [`A67_CACHE_BLOCK`].
    pub target: u32,
}

// ---------------------------------------------------------------------------
// `0x0061803b` - loop tail of the flattened walker
// ---------------------------------------------------------------------------

/// `0x0061803b` (`FUN_0061803b`, 5 bytes) is the last byte of
/// `0x00618039 cmp eax, [ebp-0x44]` plus `0x0061803c jb 0x006175c1` and
/// `0x00618042 jmp 0x006199f6`.  Decoded from `0x0061803b` the five bytes
/// `bc 0f 82 7f f5` read as `mov esp, 0xf57f820f`, which the cache reports as
/// bad instruction data; the enclosing compare is what this block performs.
/// `[ebp-0x38]` was just incremented (`0x00618036`) and the walker continues
/// while it is below the bound.  The descending edge feeds the cache block
/// that later rows translate.
pub fn a67_loop_tail(counter: u32, bound: u32) -> A67LoopTail {
    if counter < bound {
        A67LoopTail {
            take: true,
            target: A67_LOOP_BACK_EDGE,
        }
    } else {
        A67LoopTail {
            take: false,
            target: A67_CACHE_BLOCK,
        }
    }
}

// ---------------------------------------------------------------------------
// `0x00618493` - bias add and accumulator store
// ---------------------------------------------------------------------------

/// `0x00618493` (`FUN_00618493`, 139 bytes) begins one byte inside
/// `0x00618491 mov [ebp-0x3c], eax`.  The first coherent statement of its
/// body is `0x00618497 add eax, 0x8af90000`; the bias is reproduced modulo
/// `2^32`, matching the 32-bit `add`.  The body's coherent tail is the
/// accumulator store translated by [`a67_accumulate`].
pub fn a67_bias_8af9(eax: u32) -> u32 {
    eax.wrapping_add(A67_BIAS_8AF9)
}

/// `0x0061851b`: `imul eax, ecx`; `0x0061851e`: `add edx, eax`;
/// `0x00618520`: `mov [0xf7ef90], edx`.  The product is the low 32 bits of
/// `eax*ecx` and is added into the incoming `edx` with wrapping; the result is
/// the dword this block stores into the global accumulator.
pub fn a67_accumulate(eax: u32, ecx: u32, edx: u32) -> u32 {
    edx.wrapping_add(eax.wrapping_mul(ecx))
}

// ---------------------------------------------------------------------------
// `0x00618850` - dispatch-address mixer
// ---------------------------------------------------------------------------

/// `0x00618850` (`FUN_00618850`, 12 bytes) is entered through the call at
/// `0x0060ee00`; the coherent mixing sequence sits at
/// `0x00618882`..`0x006188b9`:
///
/// ```text
/// movzx ecx, dx          ; low half of the hash return
/// shr   edx, 0x10        ; high half
/// mov   eax, [eax+0xf7ef98]   ; 32-bit table word
/// shr   eax, 0x10
/// ror   eax, 3
/// lea   esi, [eax+eax]
/// sub   esi, ecx
/// xor   esi, edx
/// shl   esi, 0x10
/// or    esi, ecx
/// xor   esi, [ebp-0xec]  ; salt
/// movzx eax, si
/// xor   esi, 0x6a080000
/// shr   esi, 0x10
/// xor   esi, eax
/// shl   esi, 0x10
/// or    esi, eax
/// mov   [ebp-0x3c], esi
/// ```
///
/// `table_word` is the dword the caller loads from `A67_TABLE_BASE` with the
/// low half of the hash return as index; `salt` is `[ebp-0xec]`.  The result
/// is the dispatch address the block stores at `[ebp-0x3c]` and jumps
/// through.
pub fn a67_dispatch_mix(table_word: u32, key_lo: u16, key_hi: u16, salt: u32) -> u32 {
    let table_half = (table_word >> 16).rotate_right(3);
    let lo = u32::from(key_lo);
    let hi = u32::from(key_hi);
    let diff = table_half.wrapping_mul(2).wrapping_sub(lo) ^ hi;
    let combined = (diff << 16) | lo;
    let salted = combined ^ salt;
    let low = salted & 0xffff;
    (((salted ^ A67_HASH_XOR) >> 16) ^ low) << 16 | low
}

// ---------------------------------------------------------------------------
// `0x00618b80` - kind gate and 64-bit scaled product
// ---------------------------------------------------------------------------

/// `0x00618bc9`..`0x00618bdb`: accept when `kind < 6` and, for kind `5`
/// only, when the sub count is below `2`.  The same gate is duplicated in
/// the `0x00618850` body (`if (*(uint*)(ebp-0x3b) < 6) &&
/// (*(uint*)(ebp-0x3b) != 5 || *(uint*)(ebp-0x4b) < 2))`).
pub fn a67_kind_gate(kind: u32, sub: u32) -> bool {
    kind < A67_KIND_LIMIT && (kind != A67_KIND_LAST || sub < A67_KIND_LAST_SUB)
}

/// The two branches of `0x00618b80` produce a `mul` pair that is stored as
/// the qword at `[ebp-0x40]`:
///
/// * `take_first` (gate accepted, `0x00618c1a`): `kind = [base+0]`, then
///   `0x00618c6b mul [ebp-0x34]` gives `kind * scale` in `edx:eax`.
/// * otherwise (`0x00618cac`..`0x00618cb9`): `sub = [base+0x320]`,
///   `kind = [base+0x324]`, and `0x00618d33 mul [ebp-0x34]` gives
///   `sub * scale`; the low dword becomes `lo`, and the high dword is added
///   to `kind * scale` (`0x00618d30`..`0x00618d38`) to form `hi`.
///
/// `record0` is the dword at `base+0`, `sub_320`/`kind_324` the pair at
/// `base+0x320`, and `scale` the local at `[ebp-0x34]`.
pub fn a67_scaled(
    take_first: bool,
    record0: u32,
    sub_320: u32,
    kind_324: u32,
    scale: u32,
) -> A67Scaled {
    if take_first {
        let product = u64::from(record0) * u64::from(scale);
        A67Scaled {
            lo: product as u32,
            hi: (product >> 32) as u32,
        }
    } else {
        let product = u64::from(sub_320) * u64::from(scale);
        let hi = ((product >> 32) as u32).wrapping_add(kind_324.wrapping_mul(scale));
        A67Scaled {
            lo: product as u32,
            hi,
        }
    }
}

/// `0x00618d3e`..`0x00618df0`: the qword product is loaded into `xmm1`
/// (`0x00618d4d movq`), the folded `0xdf9` supplies shift `0x18` in `xmm0`,
/// `0x00618deb psrlq` shifts the pair right by [`A67_SCALE_SHIFT`] and
/// `0x00618df0 movq` stores it back.  A 64-bit logical shift moves the low
/// bits of `hi` into the top of `lo`.
pub fn a67_shift24(value: A67Scaled) -> A67Scaled {
    A67Scaled {
        lo: (value.lo >> A67_SCALE_SHIFT) | (value.hi << (32 - A67_SCALE_SHIFT)),
        hi: value.hi >> A67_SCALE_SHIFT,
    }
}

// ---------------------------------------------------------------------------
// `0x00618dfe` / `0x0061902a` - folded addresses, state block, counters
// ---------------------------------------------------------------------------

/// The address chain every row repeats.  `0x00618be4`..`0x00618c17` (and the
/// identical sequence at `0x00618e27`..`0x00618e57`) do
/// `p = &DAT_000010b7 + k; p += -0xc93; p += -0x354; p += -0xeb1`, all in
/// 32-bit arithmetic, which is exactly `k - 0xde1` because
/// `0x10b7 - 0xc93 - 0x354 - 0xeb1 = -0xde1`.
pub fn a67_fold_chain(k: u32) -> u32 {
    k.wrapping_add(A67_FOLD_SEED)
        .wrapping_add(A67_FOLD_STEP_1)
        .wrapping_add(A67_FOLD_STEP_2)
        .wrapping_add(A67_FOLD_STEP_3)
}

/// One fold step on its own: `0x00619029 add eax, 0xfffffcac` (`-0x354`).
/// Row `0x0061902a` starts one byte inside that immediate, which is why its
/// cached body is `inc ebp; aam 5; lodsb; cld`; the enclosing step is the
/// second subtract of [`a67_fold_chain`].
pub fn a67_fold_step_354(value: u32) -> u32 {
    value.wrapping_add(A67_FOLD_STEP_2)
}

/// `0x00618e27`: `mov [ebp-0x2c], 0x7ffe0de1`, then the fold chain of
/// `0x00618e2e`..`0x00618e57`.  `0x00618ec2` loads `0x7ffdfc43` and
/// `0x00618ecc` adds `0x3bd`, reaching the same base.  Both spellings are
/// asserted equal in the tests.
pub fn a67_state_base() -> u32 {
    a67_fold_chain(A67_STATE_BASE_SEED)
}

/// `0x00618f48`: `mov eax, [eax+0x26c]` (kind) and `0x00618f9b`:
/// `mov eax, [eax+0x270]` (count) — the blocks the decompiler labels
/// `code_r0x00618f45` and `code_r0x00618f9a`.  `block` is the memory starting
/// at [`a67_state_base`]; the pair is returned as `(kind, count)`.  `None`
/// when the slice is shorter than the count field.
pub fn a67_state_counters(block: &[u8]) -> Option<(u32, u32)> {
    let kind = block.get(A67_STATE_KIND..A67_STATE_KIND + 4)?;
    let count = block.get(A67_STATE_COUNT..A67_STATE_COUNT + 4)?;
    Some((
        u32::from_le_bytes([kind[0], kind[1], kind[2], kind[3]]),
        u32::from_le_bytes([count[0], count[1], count[2], count[3]]),
    ))
}

// ---------------------------------------------------------------------------
// `0x006196c6` - table mixers, cache window and record index
// ---------------------------------------------------------------------------

/// `0x00619887` (`and esi, 0xfff`) and `0x006193e7` (`and edi, 0xfff`)
/// mask the two table indices before the loads.  The table is 0x1000 dwords;
/// the mask keeps the index inside it.
pub fn a67_mix_index(value: u32) -> u32 {
    value & A67_TABLE_MASK
}

/// `0x006197da`..`0x00619809` mixes a 16-bit table word into the id pair:
///
/// ```text
/// movzx ecx, dx
/// shr   edx, 0x10
/// movzx esi, word [eax+0xf7ef98]
/// xor   esi, ecx
/// xor   esi, edx
/// shl   esi, 0x10
/// or    esi, ecx
/// movzx eax, si
/// shr   esi, 0x10
/// lea   ecx, [eax-0x93af]
/// xor   ecx, esi
/// shl   ecx, 0x10
/// or    ecx, eax
/// xor   ecx, [ebp-0xf8]
/// ```
///
/// `word` is the half-word loaded from [`A67_TABLE_BASE`], `lo`/`hi` the two
/// halves of the incoming dword and `salt` the local at `[ebp-0xf8]`.
pub fn a67_mix_word16(word: u16, lo: u16, hi: u16, salt: u32) -> u32 {
    let lo = u32::from(lo);
    let hi = u32::from(hi);
    let combined = ((u32::from(word) ^ lo ^ hi) << 16) | lo;
    let low = combined & 0xffff;
    let high = combined >> 16;
    let mixed = ((low.wrapping_sub(A67_MIX_BIAS) ^ high) << 16) | low;
    mixed ^ salt
}

/// `0x00619887`..`0x006198c7` (the same sequence appears at
/// `0x006193e7`..`0x0061941b`):
///
/// ```text
/// movzx esi, word [esi+0xf7ef9a]   ; word_plus2, index = v & 0xfff
/// movzx eax, word [eax+0xf7ef98]   ; word_index, index = v >> 20
/// rol   eax, 9
/// lea   edx, [ecx+ecx]
/// sub   edx, eax
/// xor   edx, edi
/// shl   edx, 0x10
/// or    edx, ecx
/// movzx eax, dx
/// xor   esi, eax
/// shr   edx, 0x10
/// xor   esi, edx
/// shl   esi, 0x10
/// or    esi, eax
/// ```
///
/// `word_plus2` comes from [`A67_TABLE_PAIR_BASE`] (`base+2`) and
/// `word_index` from [`A67_TABLE_BASE`]; `lo`/`hi` are the halves of the
/// incoming dword.
pub fn a67_mix_pair(word_plus2: u16, word_index: u16, lo: u16, hi: u16) -> u32 {
    let rotated = u32::from(word_index).rotate_left(9);
    let lo = u32::from(lo);
    let diff = lo.wrapping_mul(2).wrapping_sub(rotated) ^ u32::from(hi);
    let combined = (diff << 16) | lo;
    let low = combined & 0xffff;
    let high = combined >> 16;
    (((u32::from(word_plus2) ^ low) ^ high) << 16) | low
}

/// `0x006199f6`: the throttled block compares the cached id pair with the
/// incoming one (`0x006199ff`/`0x00619a0b`) and the elapsed ticks against
/// `0x1f4` (`0x00619a1f`: `cmp eax, 0x1f4; jae refresh`).  A hit needs all
/// three conditions, with the elapsed delta computed by `sub eax, [esi]`.
pub fn a67_cache_hit(slot0: u32, slot1: u32, id0: u32, id1: u32, now: u32, last: u32) -> bool {
    slot0 == id0 && slot1 == id1 && now.wrapping_sub(last) < A67_CACHE_WINDOW
}

/// Timestamp the block writes back through `[esi]`.  On a hit
/// (`0x00619a58`) it stores `now - 0x1f4`, which forces the next call to
/// refresh; on a miss (`0x00619ac9`) it stores `now` unchanged.
pub fn a67_cache_stamp(now: u32, hit: bool) -> u32 {
    if hit {
        now.wrapping_sub(A67_CACHE_WINDOW)
    } else {
        now
    }
}

/// Index computed by the decompiled body from a pointer difference:
/// `(ptr - base) / stride + 1`, with the two strides used in the window
/// ([`A67_STRIDE_NODE`] and [`A67_STRIDE_PAIR`]).  The division is the
/// unsigned `div` the original emits; `stride` must be non-zero.
pub fn a67_container_index(difference: u32, stride: u32) -> u32 {
    assert!(stride != 0, "container stride is a non-zero constant");
    difference / stride + 1
}

// ---------------------------------------------------------------------------
// `0x00619b90` - frame local xor
// ---------------------------------------------------------------------------

/// `0x00619b90` (`FUN_00619b90`, 91 bytes) is a complete frame:
/// `0x00619b90 push ebp`, `0x00619b91 mov ebp, esp`,
/// `0x00619b93 sub esp, 0x24`, three callee-saved pushes.  The only coherent
/// statement before the filler is `0x00619bb4 mov eax, [ebp-0xc]`,
/// `0x00619bb7 xor eax, 0xbbf`, `0x00619bbc mov [ebp-0x24], eax`: the dword
/// local at `-0xc` is xor-ed and written to the first local slot.
pub fn a67_local_xor(local_0c: u32) -> u32 {
    local_0c ^ A67_FRAME_XOR
}

// ---------------------------------------------------------------------------
// `0x00619ee7` - one-byte return
// ---------------------------------------------------------------------------

/// `0x00619ee7` (`FUN_00619ee7`, 1 byte) is the single byte `0xc3`, reached
/// by the call at `0x00617571`.  The body stores nothing, so the translation
/// is the empty function plus the opcode constant.
pub fn a67_bare_ret() {}

// ---------------------------------------------------------------------------
// `0x00619f94` - eight-bit rotate
// ---------------------------------------------------------------------------

/// `0x00619f94` (27 bytes) is entered from the call at `0x00618e13` and its
/// last instruction is `0x00619fa8 ror byte ptr [ecx-0x74c817a1], 0x45`.
/// A byte rotate count is taken modulo eight, so `0x45` is five; the cached
/// body spells the same operation `*p = *p >> 5 | *p << 3`.
pub fn a67_ror5(value: u8) -> u8 {
    value.rotate_right(u32::from(A67_ROR_COUNT & 7))
}

#[cfg(test)]
mod tests_agent67 {
    use super::*;

    /// `0x0061803b`: unsigned compare of the incrementing walker counter.
    #[test]
    fn loop_tail_continues_below_the_bound() {
        let below = a67_loop_tail(0, 1);
        assert!(below.take);
        assert_eq!(below.target, A67_LOOP_BACK_EDGE);

        let equal = a67_loop_tail(5, 5);
        assert!(!equal.take);
        assert_eq!(equal.target, A67_CACHE_BLOCK);

        let wrap = a67_loop_tail(0xffff_ffff, 0);
        assert!(!wrap.take);
        assert_eq!(wrap.target, 0x0061_99f6);
    }

    /// `0x00618497`: the bias add is a 32-bit wrap.
    #[test]
    fn bias_adds_8af90000() {
        assert_eq!(a67_bias_8af9(0), 0x8af9_0000);
        assert_eq!(a67_bias_8af9(0x0000_1234), 0x8af9_1234);
        assert_eq!(a67_bias_8af9(0x8000_0000), 0x0af9_0000);
    }

    /// `0x0061851b`: `edx + eax*ecx`, low 32 bits, stored to `0xf7ef90`.
    #[test]
    fn accumulator_adds_the_low_product() {
        assert_eq!(a67_accumulate(2, 3, 7), 13);
        assert_eq!(a67_accumulate(0x8000_0000, 2, 0), 0);
        assert_eq!(
            a67_accumulate(0x1234_5678, 0x1001, 0xffff_ffff),
            0xffff_ffff_u32.wrapping_add(0x1234_5678_u32.wrapping_mul(0x1001))
        );
        assert_eq!(A67_ACCUMULATOR, 0x00f7_ef90);
    }

    /// `0x00618882`..`0x006188b9`: exact dispatch-address mixer.
    #[test]
    fn dispatch_mix_matches_the_recovered_sequence() {
        // table word 0x12345678 -> half 0x1234 ror 3 = 0x4000136b
        assert_eq!(
            a67_dispatch_mix(0x1234_5678, 0xabcd, 0x1234, 0),
            0x8b4e_abcd
        );
        // second table constant, zero halves and an all-ones salt
        assert_eq!(
            a67_dispatch_mix(0x9b5a_c947, 0, 0, 0xffff_ffff),
            0x4cde_ffff
        );
        assert_eq!(A67_TABLE_BASE, 0x00f7_ef98);
        assert_eq!(A67_HASH_XOR, 0x6a08_0000);
    }

    /// `0x00618bc9`..`0x00618bdb`: kind gate.
    #[test]
    fn kind_gate_accepts_below_six_and_small_five() {
        assert!(a67_kind_gate(0, 0));
        assert!(a67_kind_gate(4, 9));
        assert!(a67_kind_gate(5, 1));
        assert!(!a67_kind_gate(5, 2));
        assert!(!a67_kind_gate(6, 0));
        assert!(!a67_kind_gate(u32::MAX, 0));
    }

    /// `0x00618c17`..`0x00618c71` and `0x00618caf`..`0x00618d38`: both `mul`
    /// paths of the 64-bit scaled product.
    #[test]
    fn scaled_product_takes_a_branch_of_the_mul() {
        // accepted path: kind at +0 times the scale
        let first = a67_scaled(true, 3, 0, 0, 0x18);
        assert_eq!(first, A67Scaled { lo: 0x48, hi: 0 });
        assert_eq!(first.a67_join(), 0x48);

        // rejected path: sub at +0x320 times the scale, plus kind*scale in hi
        let second = a67_scaled(false, 0, 0x1000, 3, 0x18);
        assert_eq!(
            second,
            A67Scaled {
                lo: 0x1_8000,
                hi: 0x48
            }
        );
        assert_eq!(second.a67_join(), 0x48_0001_8000);

        // carry from the sub product reaches the high dword
        let carry = a67_scaled(false, 0, 0xffff_ffff, 0x10, 2);
        assert_eq!(
            carry,
            A67Scaled {
                lo: 0xffff_fffe,
                hi: 0x21
            }
        );
        assert_eq!(carry.a67_join(), 0x21_ffff_fffe);
    }

    /// `0x00618d3e`..`0x00618df0`: the `psrlq` by `0x18`.
    #[test]
    fn shift24_is_a_64_bit_logical_shift() {
        let shifted = a67_shift24(A67Scaled {
            lo: 0x1234_5678,
            hi: 0x9abc_def0,
        });
        assert_eq!(
            shifted,
            A67Scaled {
                lo: 0xbcde_f012,
                hi: 0x0000_009a,
            }
        );
        assert_eq!(
            shifted.a67_join(),
            A67Scaled {
                lo: 0x1234_5678,
                hi: 0x9abc_def0
            }
            .a67_join()
                >> A67_SCALE_SHIFT
        );
        assert_eq!(A67_SCALE_SHIFT, 0x18);
    }

    /// `0x00618be4`..`0x00618c17` / `0x00618e27`: the folded offsets every
    /// row indexes with.
    #[test]
    fn fold_chain_reduces_to_the_recovered_offsets() {
        assert_eq!(a67_fold_chain(0x0de1), 0x0000);
        assert_eq!(a67_fold_chain(0x1101), 0x0320);
        assert_eq!(a67_fold_chain(0x0df9), 0x0018);
        assert_eq!(a67_fold_chain(0x1165), 0x0384);
        assert_eq!(a67_fold_chain(0x0e45), 0x0064);
        assert_eq!(a67_fold_chain(0x0de5), 0x0004);
        assert_eq!(a67_fold_chain(0x0e21), 0x0040);
        assert_eq!(a67_fold_chain(0x1de1), 0x1000);
        assert_eq!(a67_fold_chain(0x7ffe_0de1), 0x7ffe_0000);
        assert_eq!(A67_FOLD_BIAS, 0x0de1);
    }

    /// `0x00619029`: the `-0x354` step inside the fold, and the row that
    /// starts in its immediate.
    #[test]
    fn fold_step_354_is_the_third_chain_element() {
        // 0xde1 + 0x10b7 - 0xc93 = 0x1205, -0x354 = 0x0eb1
        assert_eq!(a67_fold_step_354(0x1205), 0x0eb1);
        assert_eq!(a67_fold_step_354(0x0eb1), 0x0b5d);
        assert_eq!(a67_fold_step_354(0x0b5d), 0x0809);
        assert_eq!(A67_FOLD_STEP_1, 0xffff_f36d);
        assert_eq!(A67_FOLD_STEP_2, 0xffff_fcac);
    }

    /// `0x00618e27` and `0x00618ec2`/`0x00618ecc`: the two spellings of the
    /// state base.
    #[test]
    fn state_base_has_two_equal_spellings() {
        assert_eq!(a67_state_base(), 0x7ffe_0000);
        assert_eq!(
            A67_STATE_BASE_ALT.wrapping_add(A67_STATE_BASE_ALT_STEP),
            0x7ffe_0000
        );
        assert_eq!(A67_STATE_BASE_SEED, 0x7ffe_0de1);
    }

    /// `0x00618f48` / `0x00618f9b`: the kind and count fields of the block.
    #[test]
    fn state_counters_read_offsets_26c_and_270() {
        assert_eq!(a67_state_counters(&[]), None);
        let mut block = vec![0u8; A67_STATE_COUNT + 4];
        block[A67_STATE_KIND..A67_STATE_KIND + 4].copy_from_slice(&7u32.to_le_bytes());
        block[A67_STATE_COUNT..A67_STATE_COUNT + 4].copy_from_slice(&0x1234u32.to_le_bytes());
        assert_eq!(a67_state_counters(&block), Some((7, 0x1234)));
        assert_eq!(a67_state_counters(&block[..A67_STATE_COUNT]), None);
        assert_eq!((A67_STATE_KIND, A67_STATE_COUNT), (0x26c, 0x270));
    }

    /// `0x00619887`: the table index mask.
    #[test]
    fn mix_index_masks_to_the_table_size() {
        assert_eq!(a67_mix_index(0x1234_5678), 0x678);
        assert_eq!(a67_mix_index(0xffff_ffff), 0xfff);
        assert_eq!(a67_mix_index(0x1000), 0);
        assert_eq!(A67_TABLE_MASK, 0xfff);
    }

    /// `0x006197da`..`0x00619809`: first 16-bit table mixer.
    #[test]
    fn mix_word16_matches_the_recovered_sequence() {
        assert_eq!(a67_mix_word16(0x1234, 0x5678, 0x9abc, 0), 0x1c39_5678);
        assert_eq!(
            a67_mix_word16(0xa6dc, 0x0af8, 0x0000, 0xdead_beef),
            0x05c0_b417
        );
        // the bias is a 16-bit wrap: 0x0000 - 0x93af
        assert_eq!(a67_mix_word16(0x0000, 0x0000, 0x0000, 0), 0x6c51_0000);
        assert_eq!(A67_MIX_BIAS, 0x93af);
    }

    /// `0x00619887`..`0x006198c7`: second table mixer, including the
    /// `rol 9`.
    #[test]
    fn mix_pair_matches_the_recovered_sequence() {
        assert_eq!(a67_mix_pair(0x1111, 0x2222, 0x3333, 0x4444), 0x4400_3333);
        assert_eq!(a67_mix_pair(0xa6dc, 0x0af8, 0xffff, 0x0001), 0x56dc_ffff);
        // rol 9 of 0x8000 is 0, of 0x0001 is 0x0200
        assert_eq!(a67_mix_pair(0, 0x8000, 0, 0), 0);
        assert_eq!(a67_mix_pair(0, 0x0001, 0, 0), 0xfe00_0000);
        assert_eq!(A67_TABLE_PAIR_BASE, 0x00f7_ef9a);
    }

    /// `0x006199ff`..`0x00619a24`: the cache window and its two slots.
    #[test]
    fn cache_hit_needs_ids_and_the_500_tick_window() {
        assert!(a67_cache_hit(1, 2, 1, 2, 1000, 900));
        assert!(!a67_cache_hit(1, 2, 1, 3, 1000, 900));
        assert!(!a67_cache_hit(1, 2, 0, 2, 1000, 900));
        // exactly 0x1f4 elapsed takes the refresh path (jae)
        assert!(!a67_cache_hit(1, 2, 1, 2, 1400, 900));
        assert!(a67_cache_hit(1, 2, 1, 2, 1399, 900));
        // wrapping elapsed time
        assert!(a67_cache_hit(1, 2, 1, 2, 0x10, 0xffff_ffff));
        assert_eq!(A67_CACHE_WINDOW, 500);
        assert_eq!(
            (A67_CACHE_SLOT0, A67_CACHE_SLOT1),
            (0x00f0_4c74, 0x00f0_4c78)
        );
    }

    /// `0x00619a58` / `0x00619ac9`: the timestamp written back.
    #[test]
    fn cache_stamp_rewinds_on_a_hit() {
        assert_eq!(a67_cache_stamp(0x1234, true), 0x1040);
        assert_eq!(a67_cache_stamp(0x1234, false), 0x1234);
        assert_eq!(a67_cache_stamp(0x1f3, true), 0xffff_ffff);
    }

    /// Decompiled container index `(ptr - base) / stride + 1`.
    #[test]
    fn container_index_divides_by_the_two_strides() {
        assert_eq!(a67_container_index(0x1cc, A67_STRIDE_NODE), 2);
        assert_eq!(a67_container_index(0x3 * 0x1cc, A67_STRIDE_NODE), 4);
        assert_eq!(a67_container_index(0x1cc * 7 + 0x1cb, A67_STRIDE_NODE), 8);
        assert_eq!(a67_container_index(0x18 * 5, A67_STRIDE_PAIR), 6);
        assert_eq!(a67_container_index(0, A67_STRIDE_PAIR), 1);
        assert_eq!(A67_RECORD_COPY, 0x1c);
    }

    /// `0x00619b93` / `0x00619bb4`..`0x00619bbc`: frame size and local xor.
    #[test]
    fn frame_local_is_xor_ed_with_bbf() {
        assert_eq!(a67_local_xor(0), 0xbbf);
        assert_eq!(a67_local_xor(0xffff_ffff), 0xffff_f440);
        assert_eq!(a67_local_xor(0x0bbf), 0);
        assert_eq!(A67_FRAME_LOCALS, 0x24);
        assert_eq!(A67_FRAME_XOR, 0xbbf);
    }

    /// `0x00619ee7`: the single-byte `ret`.
    #[test]
    fn bare_ret_is_the_single_c3_byte() {
        assert_eq!(A67_BARE_RET_OPCODE, 0xc3);
        a67_bare_ret();
    }

    /// `0x00619fa8`: `ror byte ptr [...], 0x45` rotates by five.
    #[test]
    fn ror5_is_a_byte_rotate_by_the_masked_count() {
        assert_eq!(A67_ROR_COUNT, 0x45);
        assert_eq!(A67_ROR_COUNT & 7, 5);
        assert_eq!(a67_ror5(0x3d), 0xe9);
        assert_eq!(a67_ror5(0x01), 0x08);
        assert_eq!(a67_ror5(0x00), 0x00);
        assert_eq!(a67_ror5(0xff), 0xff);
        // eight five-bit rotations are the identity for every byte value
        for value in 0u8..=255 {
            let mut rotated = value;
            for _ in 0..8 {
                rotated = a67_ror5(rotated);
            }
            assert_eq!(rotated, value);
        }
    }
}
