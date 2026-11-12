#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent71.rs
// ===========================================================================
// Translation batch 71.  Only this file is edited by the batch.
//
// Batch 71 owns `0x00620000..=0x00621fff`.  The ledger lists 20
// `decompiled_uncited` rows in that window and every one of them is
// represented below.  The window's remaining row is the five-byte thunk at
// `FUN_00621b35` (`thunk_FUN_00538810`), which is not a `decompiled_uncited`
// body.
//
// The window is CRT/C++ support glue, not simulation state: the TLS static
// guard, two RTTI type-descriptor getters, a one-word predicate, several
// inline-string helpers, the small vector/string move-assign and insert
// bodies, two SEH-wrapped shutdown frames and the locale-publish frame used
// by the shutdown path.  Nothing here touches [`crate::sim::SimUnit`] or
// [`crate::sim::World`], so the batch uses free functions and value types
// only; no `sim` record gains a field.
//
// Recovered layouts
// -----------------
//
// * **Vector/string header** (12 bytes): `+0x00` data, `+0x04` element
//   count, `+0x08` capacity word.  Bit 31 of the capacity word is the
//   *inline* flag: set means the data lives in the object (or on the
//   caller's frame) and must not be freed, clear means a heap allocation.
//   This is the test used by `0x00621050` (`test eax,eax; js skip-free`) and
//   by `0x00620f70`.
// * **Inline string object**: `+0x00` data pointer, `+0x04` length,
//   `+0x08` capacity word `0x8000000f` (15 inline bytes), `+0x0c..=+0x1b`
//   the sixteen inline bytes, `+0x1c` one extra word copied by
//   `0x00621280`.  The reserve target grows to
//   `max(request, (flags >> 1 & 0x3fffffff) + (flags & 0x7fffffff))` and
//   allocates `new_capacity + 1` bytes at alignment 8.
// * **SEH frame**: `fs:0` chain word at `[ebp - 0xc]`, handler literal,
//   saved `ExceptionList` and the `0x00f4e578 ^ ebp` stack cookie.  The
//   cookie check is modelled by [`a71_stack_cookie`].
//
// Address to function
// -------------------
//
// * `0x00620890` [`a71_tls_guard_action`]
// * `0x006208f0` [`a71_type_descriptor_lambda_4f30617c`]
// * `0x00620900` [`a71_type_descriptor_lambda_dbfea748`]
// * `0x00620910` [`a71_word18_nonzero`]
// * `0x006209f5` [`a71_memmove_size_006209f5`]
// * `0x00620a67` [`a71_adc_store_00620a67`]
// * `0x00620b40` [`a71_seh_functor_frame_00620b40`]
// * `0x00620d50` [`a71_locale_publish_00620d50`]
// * `0x00620d6a` [`a71_locale_publish_00620d6a`]
// * `0x00620f49` [`a71_virtual_call_00620f49`]
// * `0x00620f70` [`a71_move_assign_00620f70`]
// * `0x00621050` [`a71_vector_insert_00621050`]
// * `0x00621280` [`a71_locale_from_cstr_00621280`]
// * `0x00621297` [`a71_misaligned_string_00621297`]
// * `0x00621300` [`a71_destructor_plan_00621300`]
// * `0x0062134a` [`a71_misaligned_dealloc_0062134a`]
// * `0x00621461` [`a71_stack_string_00621461`]
// * `0x00621511` [`a71_fragment_byte_00621511`]
// * `0x00621825` [`a71_stack_string_00621825`]
// * `0x006218d1` [`a71_fragment_byte_006218d1`]
//
// Rows with partial evidence
// --------------------------
//
// Seven rows start in the middle of another instruction or function body.
// Their translations reproduce exactly the arithmetic, stores or bytes
// that the row contains, and do not invent a larger body:
//
// * `0x006209f5` is the tail of a `memmove` setup at `0x006209e8`.
// * `0x00620a67` is one byte before the indirect call at `0x00620a68`.
// * `0x00621297` is `0x17` bytes into `0x00621280`.
// * `0x0062134a` is `0x4a` bytes into the `0x00621300` epilogue.
// * `0x00621511` is a displacement byte of `0x0062150d`.
// * `0x00621825` is a displacement byte of `0x00621823`.
// * `0x006218d1` is a byte of the `call` at `0x006218ce`.
//
// `0x00620b40` contains overlapping/bad instruction data between
// `0x00620b6e` and `0x00620c13`; the recovered SEH frame and the two
// copyable-function-object constants are modelled, and the unreadable bytes
// are not guessed.
//
// `0x00620d50` and `0x00620d6a` are two entries into the same body
// (`0x00620d6a` is `0x1a` bytes into the `0x00620d50` prologue).  The
// publish plan is shared; the unaligned entry additionally records the
// corrupted six-byte base arithmetic.

// ---------------------------------------------------------------------------
// Shared frame and header arithmetic
// ---------------------------------------------------------------------------

/// Global stack cookie XOR word used by every SEH prologue in this window
/// (`mov eax, ds:0x00f4e578; xor eax, ebp`).
pub const A71_STACK_COOKIE_GLOBAL: u32 = 0x00f4_e578;

/// `xor eax, ebp` stack-cookie value.
///
/// The shipped prologues store `0x00f4e578 ^ frame` and the epilogues
/// recompute it before the non-returning cookie check.
pub fn a71_stack_cookie(frame: u32) -> u32 {
    A71_STACK_COOKIE_GLOBAL ^ frame
}

/// The 12-byte header used by the string/vector bodies below.
///
/// `capacity_flags & 0x8000_0000 != 0` means inline storage (no free);
/// clear means a heap allocation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A71VectorHeader {
    /// `+0x00`: data pointer (or inline-buffer address).
    pub data: u32,
    /// `+0x04`: element count.
    pub count: u32,
    /// `+0x08`: capacity word including the inline flag.
    pub capacity_flags: u32,
}

impl A71VectorHeader {
    /// `+0x08 & 0x7fff_ffff`: the capacity in elements.
    pub fn a71_capacity(&self) -> u32 {
        self.capacity_flags & 0x7fff_ffff
    }

    /// The bit-31 inline flag.
    pub fn a71_is_inline(&self) -> bool {
        self.capacity_flags & 0x8000_0000 != 0
    }
}

/// Inline-string flag word written by `0x00621280`, `0x00621297`,
/// `0x00621461` and `0x00621825`: capacity 15, inline bit set.
pub const A71_STRING_INLINE_FLAGS: u32 = 0x8000_000f;

/// Offset of the inline byte buffer inside the inline string object.
pub const A71_STRING_INLINE_OFFSET: u32 = 0x0c;

/// Capacity of the inline buffer (15 bytes plus the terminator).
pub const A71_STRING_INLINE_CAPACITY: u32 = 0x0f;

// ---------------------------------------------------------------------------
// 0x00620890 - TLS-guarded static initialization
// ---------------------------------------------------------------------------

/// The static object whose address `0x00620890` returns (`0x00faa0f8`).
pub const A71_TLS_STATIC_OBJECT: u32 = 0x00fa_a0f8;

/// Result of the `0x00620890` guard decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A71TlsGuardAction {
    /// Use the already-initialized static object.
    UseStatic,
    /// The header call left the guard at `-1`, so run the initializer and
    /// the footer before using the static object.
    InitializeThenUseStatic,
}

/// `0x00620890` (`FUN_00620890`, 81 bytes): per-thread static-init guard.
///
/// The shipped body is:
///
/// ```text
/// tls   = ThreadLocalStoragePointer[_tls_index]
/// epoch = *(i32 *)(tls + 0xc)
/// guard = *(i32 *)0x00faa114
/// if (guard > epoch) {
///     FUN_00c87327(&guard);          // header
///     if (guard == -1) {             // this thread won the race
///         FUN_00c86b59(&LAB_00d0ec70); // initializer
///         FUN_00c872dd(&guard);      // footer
///     }
/// }
/// return (void *)0x00faa0f8;
/// ```
///
/// `header_result` is the guard word observed after the header call.  The
/// header and footer helpers are outside this batch, so their effects enter
/// through that argument.
pub fn a71_tls_guard_action(guard: i32, tls_epoch: i32, header_result: i32) -> A71TlsGuardAction {
    if guard <= tls_epoch {
        return A71TlsGuardAction::UseStatic;
    }
    if header_result == -1 {
        A71TlsGuardAction::InitializeThenUseStatic
    } else {
        A71TlsGuardAction::UseStatic
    }
}

/// The address returned by every `0x00620890` path.
pub fn a71_tls_static_object() -> u32 {
    A71_TLS_STATIC_OBJECT
}

// ---------------------------------------------------------------------------
// 0x006208f0 / 0x00620900 - RTTI type descriptors
// ---------------------------------------------------------------------------

/// Common RTTI type-descriptor vtable (`0x00e4e184`).
pub const A71_RTTI_TYPE_DESCRIPTOR_VTABLE: u32 = 0x00e4_e184;

/// One MSVC RTTI type descriptor: `vfptr`, spare word, then the inline
/// mangled name at `+0x08`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A71TypeDescriptor {
    /// Address of the descriptor itself.
    pub address: u32,
    /// `+0x00`: type-descriptor vtable.
    pub vtable: u32,
    /// `+0x04`: spare word.
    pub spare: u32,
    /// Address of the inline name string (`address + 8`).
    pub name_address: u32,
    /// The inline name.
    pub name: &'static str,
}

/// `0x006208f0` (`FUN_006208f0`, 6 bytes): `mov eax,0x00f5246c; ret`.
///
/// The decompiled row names the target
/// `<lambda_4f30617cb2c63d2b36d46e4c12668551>::RTTI_Type_Descriptor`; the
/// inline name at `0x00f52474` is
/// `.?AV<lambda_4f30617cb2c63d2b36d46e4c12668551>@@`.
pub fn a71_type_descriptor_lambda_4f30617c() -> A71TypeDescriptor {
    A71TypeDescriptor {
        address: 0x00f5_246c,
        vtable: A71_RTTI_TYPE_DESCRIPTOR_VTABLE,
        spare: 0,
        name_address: 0x00f5_2474,
        name: ".?AV<lambda_4f30617cb2c63d2b36d46e4c12668551>@@",
    }
}

/// `0x00620900` (`FUN_00620900`, 6 bytes): `mov eax,0x00f524a4; ret`.
///
/// The decompiled row names the target
/// `<lambda_dbfea748a930bb5dfd67feba18932425>::RTTI_Type_Descriptor`; the
/// inline name at `0x00f524ac` is
/// `.?AV<lambda_dbfea748a930bb5dfd67feba18932425>@@`.
pub fn a71_type_descriptor_lambda_dbfea748() -> A71TypeDescriptor {
    A71TypeDescriptor {
        address: 0x00f5_24a4,
        vtable: A71_RTTI_TYPE_DESCRIPTOR_VTABLE,
        spare: 0,
        name_address: 0x00f5_24ac,
        name: ".?AV<lambda_dbfea748a930bb5dfd67feba18932425>@@",
    }
}

// ---------------------------------------------------------------------------
// 0x00620910 - one-word predicate
// ---------------------------------------------------------------------------

/// `0x00620910` (`FUN_00620910`, 8 bytes): `return *(int *)(this + 0x18) != 0;`
///
/// The shipped body is `cmp dword ptr [ecx+0x18],0; setne al; ret`.
pub fn a71_word18_nonzero(word: u32) -> bool {
    word != 0
}

// ---------------------------------------------------------------------------
// 0x006209f5 - memmove tail arithmetic
// ---------------------------------------------------------------------------

/// `0x006209f5` (`FUN_006209f5`, 19 bytes): the `memmove` size expression.
///
/// This entry is `0x0d` bytes into the copy sequence at `0x006209e8`:
///
/// ```text
/// add  esp, 0xc
/// lea  eax, [ebp - 0x10]
/// sub  eax, esi
/// and  eax, 0xfffffffc
/// push eax                  ; size
/// push esi                  ; source
/// push ebx                  ; destination
/// call FUN_00c89b00
/// ```
///
/// Returns `(ebp - 0x10 - esi) & ~3`, wrapping at 32 bits.
pub fn a71_memmove_size_006209f5(ebp_minus_0x10: u32, esi: u32) -> u32 {
    ebp_minus_0x10.wrapping_sub(esi) & 0xffff_fffc
}

/// The `(destination, source, size)` triple passed by `0x006209f5`.
pub fn a71_memmove_args_006209f5(ebx: u32, esi: u32, ebp_minus_0x10: u32) -> (u32, u32, u32) {
    (ebx, esi, a71_memmove_size_006209f5(ebp_minus_0x10, esi))
}

// ---------------------------------------------------------------------------
// 0x00620a67 - decoded adc read-modify-write
// ---------------------------------------------------------------------------

/// The decoded store performed by `0x00620a67`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A71AdcStore {
    /// Effective byte address after `adc bh, bh` changed `EBX`'s second byte.
    pub address: u32,
    /// `cl + carry_out`.
    pub addend: u8,
    /// `BH` after `adc bh, bh` and the incoming carry.
    pub doubled_bh: u8,
    /// Carry out of `bh + bh + carry_in`.
    pub carry_out: bool,
}

/// `0x00620a67` (`FUN_00620a67`, 28 bytes): decoded `adc [ebx+disp], cl`.
///
/// The row begins one byte before the indirect call at `0x00620a68`, so the
/// decompiler decoded a fragment:
///
/// ```text
/// adc  bh, bh
/// push edx
/// adc  byte ptr [ebx - 0x769b0bb3], cl
/// ```
///
/// The translation reproduces the exact byte arithmetic.  `bh` becomes
/// `bh + bh + carry_in`, the effective address uses the modified `BH`, and
/// the stored addend is `cl + carry_out`.
pub fn a71_adc_store_00620a67(ebx: u32, cl: u8, carry_in: bool) -> A71AdcStore {
    let bh_old = (ebx >> 8) as u8;
    let (bh_double, carry1) = bh_old.overflowing_add(bh_old);
    let (bh_new, carry2) = bh_double.overflowing_add(carry_in as u8);
    let carry_out = carry1 || carry2;
    let ebx_new = (ebx & 0xffff_0000) | ((bh_new as u32) << 8) | (ebx & 0x0000_00ff);
    A71AdcStore {
        address: ebx_new.wrapping_add(0x8964_f44d),
        addend: cl.wrapping_add(carry_out as u8),
        doubled_bh: bh_new,
        carry_out,
    }
}

/// Apply the decoded `0x00620a67` addend to the old byte value.
pub fn a71_adc_apply_00620a67(old: u8, store: &A71AdcStore) -> u8 {
    old.wrapping_add(store.addend)
}

// ---------------------------------------------------------------------------
// 0x00620b40 - SEH-wrapped copyable function object
// ---------------------------------------------------------------------------

/// `0x00620b40` handler literal (`push 0x00cc5750`).
pub const A71_SEH_HANDLER_00620B40: u32 = 0x00cc_5750;
/// Copyable function object for `lambda_dbfea748...` (`0x00d3b004`).
pub const A71_FUNCTOR_VPTR_LAMBDA_DBFEA748: u32 = 0x00d3_b004;
/// Copyable function object for `lambda_4f30617c...` (`0x00d3b024`).
pub const A71_FUNCTOR_VPTR_LAMBDA_4F30617C: u32 = 0x00d3_b024;
/// Cleanup thunk called with the first functor object (`FUN_005426e0`).
pub const A71_FUNCTOR_CLEANUP_005426E0: u32 = 0x0054_26e0;
/// Call made after the first functor (`FUN_007b5aa0`).
pub const A71_FUNCTOR_POST_CALL_007B5AA0: u32 = 0x007b_5aa0;
/// Final call made after the second functor (`FUN_007b5bd0`).
pub const A71_FUNCTOR_FINAL_CALL_007B5BD0: u32 = 0x007b_5bd0;
/// The `1` stored in the 20-byte handle block before each functor call.
pub const A71_FUNCTOR_HANDLE_MARKER: u32 = 1;
/// `sub esp, 0x14`: bytes of the handle block.
pub const A71_FUNCTOR_HANDLE_BYTES: u32 = 0x14;

/// Recovered state of the `0x00620b40` shutdown frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A71SehFunctorFrame00620B40 {
    /// SEH handler literal.
    pub handler: u32,
    /// Saved `fs:0` link.
    pub saved_exception_list: u32,
    /// `0x00f4e578 ^ frame`.
    pub cookie: u32,
    /// `ecx` captured into the first functor object.
    pub captured_this: u32,
    /// First functor's vptr (`lambda_dbfea748`).
    pub first_functor_vptr: u32,
    /// First functor's handle word (`1`).
    pub first_functor_handle: u32,
    /// First functor's cleanup thunk.
    pub first_functor_cleanup: u32,
    /// Call made after the first functor.
    pub post_call: u32,
    /// Second functor's vptr (`lambda_4f30617c`).
    pub second_functor_vptr: u32,
    /// Final call made after the second functor.
    pub final_call: u32,
    /// `sub esp, 0x14` size of each handle block.
    pub handle_bytes: u32,
}

/// `0x00620b40` (`FUN_00620b40`, 134 bytes): SEH-wrapped functor call.
///
/// The row contains the full MSVC SEH prologue:
///
/// ```text
/// push 0xffffffff
/// push 0x00cc5750
/// mov  eax, fs:0
/// push eax
/// sub  esp, 0x24
/// mov  eax, ds:0x00f4e578
/// xor  eax, ebp
/// mov  [ebp-0x10], eax
/// ...
/// mov  fs:0, &[ebp-0xc]
/// mov  [ebp-0x28], ecx
/// ```
///
/// After the overlapping/bad-instruction bytes the decompiler recovered a
/// copyable-function-object store at `[ebp-0x24]` (`0x00d3b004`, the
/// `lambda_dbfea748...` object) with the captured `ecx` at `[ebp-0x20]`,
/// the `1` handle marker, and the cleanup call
/// `FUN_005426e0(&object)`.  The shipped continuation also builds the
/// `lambda_4f30617c...` object (`0x00d3b024`) between calls
/// `FUN_007b5aa0` and `FUN_007b5bd0`.
pub fn a71_seh_functor_frame_00620b40(
    this_ptr: u32,
    prev_exception_list: u32,
    frame: u32,
) -> A71SehFunctorFrame00620B40 {
    A71SehFunctorFrame00620B40 {
        handler: A71_SEH_HANDLER_00620B40,
        saved_exception_list: prev_exception_list,
        cookie: a71_stack_cookie(frame),
        captured_this: this_ptr,
        first_functor_vptr: A71_FUNCTOR_VPTR_LAMBDA_DBFEA748,
        first_functor_handle: A71_FUNCTOR_HANDLE_MARKER,
        first_functor_cleanup: A71_FUNCTOR_CLEANUP_005426E0,
        post_call: A71_FUNCTOR_POST_CALL_007B5AA0,
        second_functor_vptr: A71_FUNCTOR_VPTR_LAMBDA_4F30617C,
        final_call: A71_FUNCTOR_FINAL_CALL_007B5BD0,
        handle_bytes: A71_FUNCTOR_HANDLE_BYTES,
    }
}

// ---------------------------------------------------------------------------
// 0x00620d50 / 0x00620d6a - locale publish frame
// ---------------------------------------------------------------------------

/// Locale global selected by the `FUN_0061f820` predicate (`DAT_00faa11c`).
pub const A71_LOCALE_GLOBAL_00FAA11C: u32 = 0x00fa_a11c;
/// Locale global selected by the `FUN_0061f890` predicate (`DAT_00faa118`).
pub const A71_LOCALE_GLOBAL_00FAA118: u32 = 0x00fa_a118;
/// Offset of the published id word inside the object (`+0x18`).
pub const A71_LOCALE_SLOT_OFFSET: u32 = 0x18;
/// Offset of the corrupted six-byte base read in `0x00620d6a`.
pub const A71_LOCALE_UNALIGNED_BASE: u32 = 0x5653_f045;

/// Inputs of the shared `0x00620d50` / `0x00620d6a` publish body.
///
/// The two predicate results are the values returned by the out-of-window
/// `FUN_0061f820` and `FUN_0061f890` helpers for `id`; the helper addresses are
/// recorded on [`a71_locale_publish_00620d50`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A71LocalePublishInput {
    /// `*param_1`: the id stored at `this + 0x18`.
    pub id: u32,
    /// Result of `FUN_0061f820(id)` (true when the id is not reserved).
    pub not_reserved: bool,
    /// Result of `FUN_0061f890(id)` (true when the id is `DAT_00faa148`).
    pub is_00faa148: bool,
    /// `param_2`: the byte that enables the second half of the body.
    pub flag: bool,
    /// True when the object at `+0x78` yields a non-null guard object.
    pub guard_present: bool,
    /// Result of the virtual guard precheck at `[vtable + 4]`.
    pub precheck_result: bool,
    /// Result of `FUN_0053b1e0` when the precheck did not take over.
    pub second_check_result: bool,
    /// Capacity word of the stack string built at `[ebp - 0x2c]`.
    pub stack_string_flags: u32,
}

/// The recovered publish plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A71LocalePublishPlan {
    /// `this + 0x18` after the first store.
    pub slot18: u32,
    /// The id passed to `FUN_0092b970`.
    pub main_id: u32,
    /// The id selected by the `param_2` half, when that half runs.
    pub selected_id: Option<u32>,
    /// `1` for the precheck takeover, `2` for the normal path.
    pub lock_state: u8,
    /// True when `[object + 0x60]` was set to `1`.
    pub object_flag_60_set: bool,
    /// Out-of-window call targets in shipped order.
    pub call_targets: Vec<u32>,
    /// True when the stack string at `[ebp - 0x2c]` is heap-backed and must
    /// be released through `allocator[vtable + 0x10]`.
    pub stack_string_dealloc: bool,
}

/// `0x00620d50` (`FUN_00620d50`, 25-byte ledger row; body continues to
/// `0x00620f6c`): shutdown locale publish.
///
/// Recovered control flow:
///
/// ```text
/// this[0x18] = param_1[0]
/// obj = FUN_0061b1b0()
/// is_148 = FUN_0061f890(param_1)
/// if (guard object present && !guard.vtbl[1](obj, &lock)) {
///     FUN_0053b060(obj + 0x24); obj[0x60] = 1;
///     FUN_0053cac0(); state = 1;
/// } else {
///     if (!FUN_0053b1e0(obj, &lock)) { FUN_0053af40(&lock); obj[0x60] = 1; }
///     FUN_0053cac0(); state = 2;
/// }
/// FUN_0053bda0(&lock)
/// main = FUN_0061f820(param_1) ? DAT_00faa11c : this[0x18]
/// FUN_0092b970(main); FUN_007c1d10(1)
/// if (param_2) {
///     sel = FUN_0061f890(param_1) ? DAT_00faa118 : param_1[0]
///     FUN_00634ab4(); FUN_005b4bb0();
///     s = FUN_0061f560(&stack_string); FUN_007b5e30(s)
///     if (!(stack_string[8] & 0x80000000)) free(stack_string[0])
/// }
/// ```
///
/// The helper calls are outside this batch and remain opaque; the plan
/// records their targets and the recovered branch state.
pub fn a71_locale_publish_00620d50(input: A71LocalePublishInput) -> A71LocalePublishPlan {
    let mut call_targets = Vec::new();
    let (lock_state, object_flag_60_set) = if input.guard_present && !input.precheck_result {
        call_targets.push(0x0053_b060);
        (1, true)
    } else {
        call_targets.push(0x0053_b1e0);
        if !input.second_check_result {
            call_targets.push(0x0053_af40);
            (2, true)
        } else {
            (2, false)
        }
    };
    call_targets.push(0x0053_cac0);
    call_targets.push(0x0053_bda0);

    let main_id = if input.not_reserved {
        A71_LOCALE_GLOBAL_00FAA11C
    } else {
        input.id
    };
    call_targets.push(0x0092_b970);
    call_targets.push(0x007c_1d10);

    let mut selected_id = None;
    let mut stack_string_dealloc = false;
    if input.flag {
        selected_id = Some(if input.is_00faa148 {
            A71_LOCALE_GLOBAL_00FAA118
        } else {
            input.id
        });
        call_targets.push(0x0063_4ab4);
        call_targets.push(0x005b_4bb0);
        call_targets.push(0x0061_f560);
        call_targets.push(0x007b_5e30);
        stack_string_dealloc = input.stack_string_flags & 0x8000_0000 == 0;
    }

    A71LocalePublishPlan {
        slot18: input.id,
        main_id,
        selected_id,
        lock_state,
        object_flag_60_set,
        call_targets,
        stack_string_dealloc,
    }
}

/// `0x00620d6a` (`FUN_00620d6a`, 635 bytes): unaligned entry into
/// `0x00620d50`.
///
/// The row starts `0x1a` bytes into the `0x00620d50` SEH prologue, so its
/// first store uses the corrupted base
/// `(int)*(undefined6 *)(this + 0x5653f045) + 0x18`.  `six_byte_value` is
/// that six-byte value; the rest of the body is the shared publish plan.
///
/// Returns `(slot_address, plan)`.
pub fn a71_locale_publish_00620d6a(
    _this_ptr: u32,
    six_byte_value: u32,
    input: A71LocalePublishInput,
) -> (u32, A71LocalePublishPlan) {
    let slot_address = six_byte_value.wrapping_add(A71_LOCALE_SLOT_OFFSET);
    (slot_address, a71_locale_publish_00620d50(input))
}

// ---------------------------------------------------------------------------
// 0x00620f49 - indirect slot dispatch
// ---------------------------------------------------------------------------

/// The recovered `0x00620f49` virtual-call coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A71VirtualCall00620f49 {
    /// `ecx` / first argument (`eax` in the row).
    pub receiver: u32,
    /// `*receiver`: the vtable pointer.
    pub vtable: u32,
    /// Address of the function pointer: `vtable + 0x10`.
    pub slot_address: u32,
    /// The `push esi` argument.
    pub stack_arg: u32,
}

/// `0x00620f49` (`FUN_00620f49`, 32 bytes): indirect call through
/// `[vtable + 0x10]`.
///
/// The shipped bytes are:
///
/// ```text
/// push esi
/// mov  ecx, eax
/// mov  edx, [eax]
/// call [edx + 0x10]
/// ```
///
/// followed by the SEH epilogue and the stack-cookie check.  The translation
/// models the receiver/vtable/slot arithmetic; the call itself is opaque.
pub fn a71_virtual_call_00620f49(
    receiver: u32,
    vtable: u32,
    stack_arg: u32,
) -> A71VirtualCall00620f49 {
    A71VirtualCall00620f49 {
        receiver,
        vtable,
        slot_address: vtable.wrapping_add(0x10),
        stack_arg,
    }
}

// ---------------------------------------------------------------------------
// 0x00620f70 - vector/string move assignment
// ---------------------------------------------------------------------------

/// Result of the `0x00620f70` move assignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A71MoveAssign00620f70 {
    /// Destination header after the assignment.
    pub dest: A71VectorHeader,
    /// Source header after the assignment.
    pub source: A71VectorHeader,
    /// `Some(bytes)` when the inline-source path allocated a new buffer.
    pub allocation_bytes: Option<u32>,
    /// Elements copied by the `rep movsd`-style loop; `0` for the heap-steal
    /// path, which moves the header instead.
    pub copied: u32,
}

/// `0x00620f70` (`FUN_00620f70`, 178 bytes): move-assign a
/// vector/string header.
///
/// The first branch tests the **source** inline flag:
///
/// * source heap (`source.capacity_flags >= 0`): steal the header.  The
///   destination gets `source[0..0xc]` with the inline bit cleared; the
///   source becomes `{0, 0, source_flags & 0x80000000}`.
/// * source inline (`source.capacity_flags < 0`): if the destination is
///   inline with enough capacity, copy `source.count` dwords into its inline
///   buffer.  Otherwise allocate `source.count * 4` bytes at alignment 8,
///   copy the elements, and set the destination capacity word to
///   `source.count & 0x7fffffff`.  The row contains no release of the old
///   destination buffer.
///
/// Both copy paths compute the element count as the source count unless
/// `source.data + count * 4` wraps, in which case the shipped loop count is
/// zero.
pub fn a71_move_assign_00620f70(
    dest: A71VectorHeader,
    source: A71VectorHeader,
) -> A71MoveAssign00620f70 {
    if source.capacity_flags & 0x8000_0000 == 0 {
        return A71MoveAssign00620f70 {
            dest: A71VectorHeader {
                data: source.data,
                count: source.count,
                capacity_flags: source.capacity_flags & 0x7fff_ffff,
            },
            source: A71VectorHeader {
                data: 0,
                count: 0,
                capacity_flags: source.capacity_flags & 0x8000_0000,
            },
            allocation_bytes: None,
            copied: 0,
        };
    }

    let copy_count = if source.data.wrapping_add(source.count.wrapping_mul(4)) < source.data {
        0
    } else {
        source.count
    };

    if source.count < dest.a71_capacity() && dest.a71_is_inline() {
        return A71MoveAssign00620f70 {
            dest: A71VectorHeader {
                data: dest.data,
                count: source.count,
                capacity_flags: dest.capacity_flags,
            },
            source,
            allocation_bytes: None,
            copied: copy_count,
        };
    }

    A71MoveAssign00620f70 {
        dest: A71VectorHeader {
            data: 0,
            count: source.count,
            capacity_flags: source.count & 0x7fff_ffff,
        },
        source,
        allocation_bytes: Some(source.count.wrapping_mul(4)),
        copied: copy_count,
    }
}

// ---------------------------------------------------------------------------
// 0x00621050 - vector insert with an output fill plan
// ---------------------------------------------------------------------------

/// Result of the `0x00621050` insert.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A71Insert00621050 {
    /// `old_count + n`.
    pub new_count: u32,
    /// Capacity word after the growth branch (unchanged otherwise).
    pub new_capacity_flags: u32,
    /// First index the caller fills from its source range.
    pub fill_start: u32,
    /// Split index between the two caller `memcpy` calls.
    pub split: u32,
    /// One past the last index the caller fills.
    pub fill_end: u32,
    /// `Some(bytes)` when the capacity branch allocates a new buffer.
    pub allocation_bytes: Option<u32>,
    /// Old buffer released by the growth branch when it was heap-backed.
    pub freed_old_data: Option<u32>,
    /// Resulting logical elements, with the inserted range zero-filled.
    pub elements: Vec<u32>,
}

/// `0x00621050` (`FUN_00621050`, 16-byte ledger row; body continues to
/// `0x0062117d`): insert `n` default elements at `pos`.
///
/// The function writes an output triple `(fill_start, split, fill_end)` to
/// its first argument.  The caller copies its source range into
/// `[fill_start, split)` with one `memmove` and into `[split, fill_end)` with
/// a second one.  The split is chosen by the shipped branches:
///
/// * growth: `(pos, pos, pos + n)`;
/// * middle (`pos + n < old_count`): `(pos, pos + n, pos + n)`;
/// * tail (`pos + n >= old_count`): `(pos, old_count, pos + n)`.
///
/// Growth uses
/// `max(new_count, (flags >> 1 & 0x3fffffff) + (flags & 0x7fffffff))` and
/// preserves the inline bit in the updated capacity word.
pub fn a71_vector_insert_00621050(
    elements: &[u32],
    capacity_flags: u32,
    data_base: u32,
    pos_index: u32,
    n: u32,
) -> A71Insert00621050 {
    let old_count = elements.len() as u32;
    assert!(
        pos_index <= old_count,
        "0x00621050 position must be inside the vector"
    );
    let new_count = old_count.wrapping_add(n);
    let capacity = capacity_flags & 0x7fff_ffff;
    let mut new_capacity_flags = capacity_flags;
    let mut allocation_bytes = None;
    let mut freed_old_data = None;

    let (fill_start, split, fill_end) = if capacity < new_count {
        let grown = (capacity_flags >> 1 & 0x3fff_ffff).wrapping_add(capacity);
        let new_capacity = if new_count <= grown { grown } else { new_count };
        new_capacity_flags = ((capacity_flags ^ new_capacity) & 0x7fff_ffff) ^ capacity_flags;
        allocation_bytes = Some(new_capacity.wrapping_mul(4));
        if capacity_flags & 0x8000_0000 == 0 {
            freed_old_data = Some(data_base);
        }
        (pos_index, pos_index, pos_index.wrapping_add(n))
    } else {
        let pos_n = pos_index.wrapping_add(n);
        if pos_n < old_count {
            (pos_index, pos_n, pos_n)
        } else {
            (pos_index, old_count, pos_n)
        }
    };

    // The shipped tail copy begins at `data + pos * 4`; valid headers never
    // wrap, so the logical element sequence is the direct insert below.
    let mut out = Vec::with_capacity(elements.len() + n as usize);
    out.extend_from_slice(&elements[..pos_index as usize]);
    out.extend(std::iter::repeat(0).take(n as usize));
    out.extend_from_slice(&elements[pos_index as usize..]);

    A71Insert00621050 {
        new_count,
        new_capacity_flags,
        fill_start,
        split,
        fill_end,
        allocation_bytes,
        freed_old_data,
        elements: out,
    }
}

// ---------------------------------------------------------------------------
// 0x00621280 - inline-string constructor from C string
// ---------------------------------------------------------------------------

/// Result of the `0x00621280` constructor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A71LocaleInit00621280 {
    /// `+0x00`: `0x0c` for the inline buffer, `0` for a fresh allocation.
    pub data_offset: u32,
    /// `+0x04`: string length excluding the terminator.
    pub count: u32,
    /// `+0x08`: capacity word.
    pub capacity_flags: u32,
    /// The string bytes plus the terminating zero.
    pub buffer: Vec<u8>,
    /// `+0x1c`: the word copied from the second stack argument.
    pub extra: u32,
    /// `Some(bytes)` when the reserve target reallocated the string.
    pub allocation_bytes: Option<u32>,
}

/// `0x00621280` (`FUN_00621280`, 24-byte ledger row; body continues to
/// `0x006212f4`): construct an inline string from `param_1` and copy the
/// word at `param_2` to `+0x1c`.
///
/// The shipped body computes `strlen(param_1)`, writes
/// `{data = this + 0x0c, count = 0, capacity_flags = 0x8000000f}` and a zero
/// terminator, calls the reserve target with the length, copies the bytes
/// plus the terminator, stores the length, stores `*param_2`, and returns
/// `this`.  The reserve target's arithmetic is inlined here: it keeps the
/// inline form while `len <= 15`; otherwise it grows to
/// `max(len, (0x8000000f >> 1 & 0x3fffffff) + 15)` and allocates
/// `new_capacity + 1` bytes.
pub fn a71_locale_from_cstr_00621280(source: &[u8], extra: u32) -> A71LocaleInit00621280 {
    let len = source
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(source.len());
    let count = len as u32;
    let mut buffer = Vec::with_capacity(len + 1);
    buffer.extend_from_slice(&source[..len]);
    buffer.push(0);

    if count <= A71_STRING_INLINE_CAPACITY {
        A71LocaleInit00621280 {
            data_offset: A71_STRING_INLINE_OFFSET,
            count,
            capacity_flags: A71_STRING_INLINE_FLAGS,
            buffer,
            extra,
            allocation_bytes: None,
        }
    } else {
        let grown =
            (A71_STRING_INLINE_FLAGS >> 1 & 0x3fff_ffff).wrapping_add(A71_STRING_INLINE_CAPACITY);
        let new_capacity = if count <= grown { grown } else { count };
        let capacity_flags =
            ((A71_STRING_INLINE_FLAGS ^ new_capacity) & 0x7fff_ffff) ^ A71_STRING_INLINE_FLAGS;
        A71LocaleInit00621280 {
            data_offset: 0,
            count,
            capacity_flags,
            buffer,
            extra,
            allocation_bytes: Some(new_capacity.wrapping_add(1)),
        }
    }
}

// ---------------------------------------------------------------------------
// 0x00621297 - unaligned string-header entry
// ---------------------------------------------------------------------------

/// Recovered state of the `0x00621297` unaligned entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A71MisalignedString00621297 {
    /// `*this += this` from the overlapping decode.
    pub first_word: u32,
    /// `*eax += (char)eax` from the overlapping decode.
    pub eax_word: u32,
    /// `esi - 1`: the scan start.
    pub scan_start: u32,
    /// The string header written at `this`.
    pub header: A71VectorHeader,
    /// First inline byte written at `this + 0x0c`.
    pub inline_first_byte: u8,
    /// Length produced by the scan loop (excluding the terminator).
    pub reserve_len: u32,
}

/// `0x00621297` (`FUN_00621297`, 50 bytes): `0x17` bytes into
/// `0x00621280`.
///
/// The decompiled row contains the overlapping stores
/// `*edi += ecx`, `*eax += (char)eax`, the scan from `esi - 1`, and the
/// inline-string header stores
/// `{this + 0x0c, 0, 0x8000000f}`.  The translation reproduces those
/// arithmetic pieces; the actual constructor body is
/// [`a71_locale_from_cstr_00621280`].
pub fn a71_misaligned_string_00621297(
    this_ptr: u32,
    this_word: u32,
    eax_word: u32,
    scan_ptr: u32,
    c_string: &[u8],
) -> A71MisalignedString00621297 {
    let reserve_len = c_string
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(c_string.len()) as u32;
    A71MisalignedString00621297 {
        first_word: this_word.wrapping_add(this_ptr),
        eax_word: eax_word.wrapping_add(((eax_word as u8) as i8 as i32) as u32),
        scan_start: scan_ptr.wrapping_sub(1),
        header: A71VectorHeader {
            data: this_ptr.wrapping_add(A71_STRING_INLINE_OFFSET),
            count: 0,
            capacity_flags: A71_STRING_INLINE_FLAGS,
        },
        inline_first_byte: 0,
        reserve_len,
    }
}

// ---------------------------------------------------------------------------
// 0x00621300 - destructor deallocation plan
// ---------------------------------------------------------------------------

/// `0x00621300` handler literal (`push 0x00cbf8f0`).
pub const A71_SEH_HANDLER_00621300: u32 = 0x00cb_f8f0;

/// Recovered destructor state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A71DestructorPlan00621300 {
    /// SEH handler literal.
    pub handler: u32,
    /// `0x00f4e578 ^ frame`.
    pub cookie: u32,
    /// Heap-backed pointers released, in shipped order.
    pub frees: Vec<u32>,
}

/// `0x00621300` (`FUN_00621300`, 63-byte ledger row; body continues to
/// `0x00621371`): two conditional releases.
///
/// ```text
/// if (this[0x24] >= 0) allocator->free(this[0x1c]);
/// if (this[8]    >= 0) allocator->free(this[0]);
/// ```
///
/// The sign bit is the inline flag, so a negative word skips the free.
pub fn a71_destructor_plan_00621300(
    frame: u32,
    word24: i32,
    ptr1c: u32,
    word8: i32,
    ptr0: u32,
) -> A71DestructorPlan00621300 {
    let mut frees = Vec::new();
    if word24 >= 0 {
        frees.push(ptr1c);
    }
    if word8 >= 0 {
        frees.push(ptr0);
    }
    A71DestructorPlan00621300 {
        handler: A71_SEH_HANDLER_00621300,
        cookie: a71_stack_cookie(frame),
        frees,
    }
}

// ---------------------------------------------------------------------------
// 0x0062134a - unaligned destructor epilogue arithmetic
// ---------------------------------------------------------------------------

/// Recovered arithmetic of the `0x0062134a` unaligned entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A71MisalignedDealloc0062134a {
    /// `*eax + *eax` (the first overlapping store).
    pub value: u32,
    /// The low byte after `+ (char)eax`.
    pub low_byte: u8,
    /// The decompiler's `SCARRY1(...) == (char)*eax < 0` guard.
    pub trap: bool,
}

/// `0x0062134a` (`FUN_0062134a`, 32 bytes): `0x4a` bytes into the
/// `0x00621300` epilogue.
///
/// The row decodes the overlapping stores
/// `*eax += *eax` and `*(char *)eax += (char)eax`, then takes the
/// `FUN_0093ec70` branch when
/// `signed_carry(low8(value), low8(pointer)) == (low8(result) < 0)`.
/// That branch is the second destructor release in the real body; only the
/// decoded arithmetic is reproduced here.
pub fn a71_misaligned_dealloc_0062134a(word: u32, pointer: u32) -> A71MisalignedDealloc0062134a {
    let value = word.wrapping_add(word);
    let left = value as u8;
    let right = pointer as u8;
    let low_byte = left.wrapping_add(right);
    let overflow = (left as i8).checked_add(right as i8).is_none();
    A71MisalignedDealloc0062134a {
        value,
        low_byte,
        trap: overflow == ((low_byte as i8) < 0),
    }
}

// ---------------------------------------------------------------------------
// 0x00621461 - stack string header fragment
// ---------------------------------------------------------------------------

/// Recovered stack string header of `0x00621461`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A71StackString00621461 {
    /// `[ebp - 0x6ac]`: data pointer (the incoming `eax`).
    pub data: u32,
    /// `[ebp - 0x6a8]`: length, zero.
    pub count: u32,
    /// `[ebp - 0x6a4]`: capacity word.
    pub capacity_flags: u32,
    /// `[ebp - 0x6a0]`: first inline byte, zero.
    pub first_byte: u8,
}

/// `0x00621461` (`FUN_00621461`, 38 bytes): stack-string header stores.
///
/// ```text
/// [ebp - 0x6a8] = 0
/// [ebp - 0x6ac] = eax
/// [ebp - 0x6a4] = 0x8000000f
/// [ebp - 0x6a0] = 0
/// call reserve
/// ```
pub fn a71_stack_string_00621461(data: u32) -> A71StackString00621461 {
    A71StackString00621461 {
        data,
        count: 0,
        capacity_flags: A71_STRING_INLINE_FLAGS,
        first_byte: 0,
    }
}

// ---------------------------------------------------------------------------
// 0x00621511 - embedded displacement byte
// ---------------------------------------------------------------------------

/// The byte at `0x00621511` (`0xff`), part of the displacement of
/// `mov dword ptr [ebp-0x670], 4` at `0x0062150d`.
pub const A71_FRAGMENT_BYTE_00621511: u8 = 0xff;

/// `0x00621511` (`FUN_00621511`, 1 byte): the row is a single `0xff` byte
/// inside an instruction displacement, not an executable body.
///
/// The translation's only recoverable behaviour is the byte value.
pub fn a71_fragment_byte_00621511() -> u8 {
    A71_FRAGMENT_BYTE_00621511
}

// ---------------------------------------------------------------------------
// 0x00621825 - stack string header fragment with a trap guard
// ---------------------------------------------------------------------------

/// Recovered stack string state of the `0x00621825` fragment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A71StackString00621825 {
    /// `[ebp - 0x588]`: 4.
    pub word588: u32,
    /// `[ebp - 0x570]`: 2.
    pub word570: u32,
    /// `[ebp - 4]`: 0x0b.
    pub state_byte: u8,
    /// `[ebp - 0x568]`: 0.
    pub word568: u32,
    /// `[ebp - 0x56c]`: `ebp - 0x560`.
    pub data: u32,
    /// `[ebp - 0x564]`: `0x8000000f`.
    pub capacity_flags: u32,
    /// `[ebp - 0x560]`: 0.
    pub first_byte: u8,
    /// `push 4`: the reserve argument.
    pub reserve_arg: u32,
}

/// `0x00621825` (`FUN_00621825`, 4 bytes): the four-byte displacement
/// `78 fa ff ff` of `mov dword ptr [ebp-0x588], 4` at `0x00621823`.
///
/// The decompiler recovered the block that follows the displacement:
/// the stores `4`, `2`, `0x0b`, `0`, the inline data pointer
/// `ebp - 0x560`, the capacity word `0x8000000f`, the zero terminator and
/// `push 4`.  `sign_flag` is the `SF` test on the row; a clear flag is the
/// `halt_baddata` arm and returns `None`.
pub fn a71_stack_string_00621825(sign_flag: bool, frame: u32) -> Option<A71StackString00621825> {
    if !sign_flag {
        return None;
    }
    Some(A71StackString00621825 {
        word588: 4,
        word570: 2,
        state_byte: 0x0b,
        word568: 0,
        data: frame.wrapping_sub(0x560),
        capacity_flags: A71_STRING_INLINE_FLAGS,
        first_byte: 0,
        reserve_arg: 4,
    })
}

// ---------------------------------------------------------------------------
// 0x006218d1 - int1 byte
// ---------------------------------------------------------------------------

/// The byte at `0x006218d1` (`0xf1`), part of the relative `call` at
/// `0x006218ce`.
pub const A71_FRAGMENT_BYTE_006218D1: u8 = 0xf1;

/// The trap kind decoded from the `0x006218d1` byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A71Trap006218d1 {
    /// x86 `int1` / ICEBP (`0xf1`).
    Int1,
}

/// `0x006218d1` (`FUN_006218d1`, 1 byte): a single `0xf1` byte inside the
/// `call FUN_00538810` displacement at `0x006218ce`.
///
/// The decompiler decoded it as `swi(1)`, i.e. the `int1` trap.  The
/// translation records the byte and the trap kind; no call target is
/// recoverable from the one-byte row.
pub fn a71_fragment_byte_006218d1() -> u8 {
    A71_FRAGMENT_BYTE_006218D1
}

/// The `int1` trap kind of `0x006218d1`.
pub fn a71_icebp_trap_006218d1() -> A71Trap006218d1 {
    A71Trap006218d1::Int1
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent71 {
    use super::*;

    #[test]
    fn tls_guard_matches_signed_epoch_comparison() {
        assert_eq!(a71_tls_guard_action(0, 1, -1), A71TlsGuardAction::UseStatic);
        assert_eq!(a71_tls_guard_action(5, 5, -1), A71TlsGuardAction::UseStatic);
        assert_eq!(
            a71_tls_guard_action(6, 5, -1),
            A71TlsGuardAction::InitializeThenUseStatic
        );
        assert_eq!(a71_tls_guard_action(6, 5, 0), A71TlsGuardAction::UseStatic);
        assert_eq!(a71_tls_guard_action(6, 5, -2), A71TlsGuardAction::UseStatic);
        assert_eq!(a71_tls_static_object(), 0x00fa_a0f8);
    }

    #[test]
    fn rtti_descriptors_are_the_two_lambda_objects() {
        let a = a71_type_descriptor_lambda_4f30617c();
        assert_eq!(a.address, 0x00f5_246c);
        assert_eq!(a.name_address, a.address + 8);
        assert_eq!(a.vtable, 0x00e4_e184);
        assert_eq!(a.spare, 0);
        assert!(a.name.contains("lambda_4f30617c"));

        let b = a71_type_descriptor_lambda_dbfea748();
        assert_eq!(b.address, 0x00f5_24a4);
        assert_eq!(b.name_address, b.address + 8);
        assert_eq!(b.vtable, 0x00e4_e184);
        assert_eq!(b.spare, 0);
        assert!(b.name.contains("lambda_dbfea748"));
        assert_ne!(a.address, b.address);
    }

    #[test]
    fn word18_predicate_is_nonzero() {
        assert!(!a71_word18_nonzero(0));
        assert!(a71_word18_nonzero(1));
        assert!(a71_word18_nonzero(0x8000_0000));
    }

    #[test]
    fn memmove_size_and_arguments() {
        assert_eq!(a71_memmove_size_006209f5(0x1000, 0x0ff3), 0x0c);
        assert_eq!(a71_memmove_size_006209f5(0x10, 0x20), 0xffff_fff0);
        assert_eq!(
            a71_memmove_args_006209f5(0xaaaa, 0x0ff3, 0x1000),
            (0xaaaa, 0x0ff3, 0x0c)
        );
        assert_eq!(
            a71_memmove_args_006209f5(0xaaaa, 0xbbbb, 0x1000),
            (0xaaaa, 0xbbbb, 0xffff_5444)
        );
    }

    #[test]
    fn adc_store_doubles_bh_and_adds_cl_with_carry() {
        let store = a71_adc_store_00620a67(0x1122_3344, 0x10, false);
        assert_eq!(store.doubled_bh, 0x66);
        assert!(!store.carry_out);
        assert_eq!(store.address, 0x1122_6644u32.wrapping_add(0x8964_f44d));
        assert_eq!(store.addend, 0x10);
        assert_eq!(a71_adc_apply_00620a67(0xf0, &store), 0x00);

        let store = a71_adc_store_00620a67(0x1122_8044, 0x01, true);
        assert_eq!(store.doubled_bh, 0x01);
        assert!(store.carry_out);
        assert_eq!(store.address, 0x1122_0144u32.wrapping_add(0x8964_f44d));
        assert_eq!(store.addend, 0x02);
        assert_eq!(a71_adc_apply_00620a67(0xfe, &store), 0x00);

        let store = a71_adc_store_00620a67(0x1122_8044, 0x00, false);
        assert_eq!(store.doubled_bh, 0x00);
        assert!(store.carry_out);
        assert_eq!(store.addend, 0x01);
        assert_eq!(a71_adc_apply_00620a67(0xf0, &store), 0xf1);
    }

    #[test]
    fn seh_functor_frame_has_both_functor_constants() {
        let frame = a71_seh_functor_frame_00620b40(0x1234, 0x5678, 0x1000);
        assert_eq!(frame.handler, 0x00cc_5750);
        assert_eq!(frame.saved_exception_list, 0x5678);
        assert_eq!(frame.cookie, 0x00f4_e578 ^ 0x1000);
        assert_eq!(frame.captured_this, 0x1234);
        assert_eq!(frame.first_functor_vptr, 0x00d3_b004);
        assert_eq!(frame.first_functor_handle, 1);
        assert_eq!(frame.first_functor_cleanup, 0x0054_26e0);
        assert_eq!(frame.post_call, 0x007b_5aa0);
        assert_eq!(frame.second_functor_vptr, 0x00d3_b024);
        assert_eq!(frame.final_call, 0x007b_5bd0);
        assert_eq!(frame.handle_bytes, 0x14);
    }

    fn locale_input() -> A71LocalePublishInput {
        A71LocalePublishInput {
            id: 0x1111,
            not_reserved: false,
            is_00faa148: false,
            flag: false,
            guard_present: false,
            precheck_result: true,
            second_check_result: true,
            stack_string_flags: A71_STRING_INLINE_FLAGS,
        }
    }

    #[test]
    fn locale_publish_takes_the_precheck_takeover() {
        let input = A71LocalePublishInput {
            not_reserved: true,
            guard_present: true,
            precheck_result: false,
            ..locale_input()
        };
        let plan = a71_locale_publish_00620d50(input);
        assert_eq!(plan.slot18, 0x1111);
        assert_eq!(plan.main_id, A71_LOCALE_GLOBAL_00FAA11C);
        assert_eq!(plan.selected_id, None);
        assert_eq!(plan.lock_state, 1);
        assert!(plan.object_flag_60_set);
        assert!(!plan.stack_string_dealloc);
        assert_eq!(
            plan.call_targets,
            vec![
                0x0053_b060,
                0x0053_cac0,
                0x0053_bda0,
                0x0092_b970,
                0x007c_1d10
            ]
        );
    }

    #[test]
    fn locale_publish_normal_path_and_second_check() {
        let input = A71LocalePublishInput {
            guard_present: false,
            second_check_result: false,
            ..locale_input()
        };
        let plan = a71_locale_publish_00620d50(input);
        assert_eq!(plan.main_id, 0x1111);
        assert_eq!(plan.selected_id, None);
        assert_eq!(plan.lock_state, 2);
        assert!(plan.object_flag_60_set);
        assert_eq!(
            plan.call_targets,
            vec![
                0x0053_b1e0,
                0x0053_af40,
                0x0053_cac0,
                0x0053_bda0,
                0x0092_b970,
                0x007c_1d10
            ]
        );
    }

    #[test]
    fn locale_publish_second_half_selects_global_or_source_id() {
        let input = A71LocalePublishInput {
            id: 0x2222,
            is_00faa148: true,
            flag: true,
            stack_string_flags: 0x0000_000f,
            ..locale_input()
        };
        let plan = a71_locale_publish_00620d50(input);
        assert_eq!(plan.selected_id, Some(A71_LOCALE_GLOBAL_00FAA118));
        assert!(plan.stack_string_dealloc);
        assert_eq!(
            plan.call_targets,
            vec![
                0x0053_b1e0,
                0x0053_cac0,
                0x0053_bda0,
                0x0092_b970,
                0x007c_1d10,
                0x0063_4ab4,
                0x005b_4bb0,
                0x0061_f560,
                0x007b_5e30
            ]
        );

        let input = A71LocalePublishInput {
            id: 0x3333,
            is_00faa148: false,
            flag: true,
            stack_string_flags: A71_STRING_INLINE_FLAGS,
            ..locale_input()
        };
        let plan = a71_locale_publish_00620d50(input);
        assert_eq!(plan.selected_id, Some(0x3333));
        assert!(!plan.stack_string_dealloc);
    }

    #[test]
    fn locale_publish_unaligned_entry_uses_corrupt_base() {
        let (slot, plan) = a71_locale_publish_00620d6a(0x1000, 0x2000, locale_input());
        assert_eq!(slot, 0x2018);
        assert_eq!(plan.slot18, 0x1111);

        let (slot, _) = a71_locale_publish_00620d6a(0x1000, 0xffff_fff0, locale_input());
        assert_eq!(slot, 0x08);
    }

    #[test]
    fn virtual_call_slot_is_vtable_plus_sixteen() {
        let call = a71_virtual_call_00620f49(0x4000, 0x00d3_b004, 0x99);
        assert_eq!(call.receiver, 0x4000);
        assert_eq!(call.vtable, 0x00d3_b004);
        assert_eq!(call.slot_address, 0x00d3_b014);
        assert_eq!(call.stack_arg, 0x99);
    }

    #[test]
    fn move_assign_steals_a_heap_source() {
        let result = a71_move_assign_00620f70(
            A71VectorHeader {
                data: 0x3000,
                count: 9,
                capacity_flags: A71_STRING_INLINE_FLAGS,
            },
            A71VectorHeader {
                data: 0x4000,
                count: 3,
                capacity_flags: 0x0000_0004,
            },
        );
        assert_eq!(
            result.dest,
            A71VectorHeader {
                data: 0x4000,
                count: 3,
                capacity_flags: 0x0000_0004,
            }
        );
        assert_eq!(
            result.source,
            A71VectorHeader {
                data: 0,
                count: 0,
                capacity_flags: 0,
            }
        );
        assert_eq!(result.allocation_bytes, None);
        assert_eq!(result.copied, 0);
    }

    #[test]
    fn move_assign_copies_inline_source_into_inline_destination() {
        let result = a71_move_assign_00620f70(
            A71VectorHeader {
                data: 0x3000,
                count: 9,
                capacity_flags: A71_STRING_INLINE_FLAGS,
            },
            A71VectorHeader {
                data: 0x4000,
                count: 2,
                capacity_flags: A71_STRING_INLINE_FLAGS,
            },
        );
        assert_eq!(
            result.dest,
            A71VectorHeader {
                data: 0x3000,
                count: 2,
                capacity_flags: A71_STRING_INLINE_FLAGS,
            }
        );
        assert_eq!(result.source.count, 2);
        assert_eq!(result.allocation_bytes, None);
        assert_eq!(result.copied, 2);
    }

    #[test]
    fn move_assign_allocates_for_inline_source() {
        let result = a71_move_assign_00620f70(
            A71VectorHeader {
                data: 0x3000,
                count: 9,
                capacity_flags: 0x0000_0020,
            },
            A71VectorHeader {
                data: 0x4000,
                count: 2,
                capacity_flags: A71_STRING_INLINE_FLAGS,
            },
        );
        assert_eq!(
            result.dest,
            A71VectorHeader {
                data: 0,
                count: 2,
                capacity_flags: 0x0000_0002,
            }
        );
        assert_eq!(result.allocation_bytes, Some(8));
        assert_eq!(result.copied, 2);
    }

    #[test]
    fn move_assign_inline_destination_too_small_allocates_without_free() {
        let result = a71_move_assign_00620f70(
            A71VectorHeader {
                data: 0x3000,
                count: 9,
                capacity_flags: 0x8000_0002,
            },
            A71VectorHeader {
                data: 0x4000,
                count: 5,
                capacity_flags: A71_STRING_INLINE_FLAGS,
            },
        );
        assert_eq!(
            result.dest,
            A71VectorHeader {
                data: 0,
                count: 5,
                capacity_flags: 5,
            }
        );
        assert_eq!(result.allocation_bytes, Some(20));
        assert_eq!(result.copied, 5);
    }

    #[test]
    fn move_assign_pointer_wrap_zeroes_the_copy_count() {
        let result = a71_move_assign_00620f70(
            A71VectorHeader {
                data: 0x3000,
                count: 0,
                capacity_flags: A71_STRING_INLINE_FLAGS,
            },
            A71VectorHeader {
                data: 0xffff_fff0,
                count: 8,
                capacity_flags: A71_STRING_INLINE_FLAGS,
            },
        );
        assert_eq!(result.dest.count, 8);
        assert_eq!(result.allocation_bytes, None);
        assert_eq!(result.copied, 0);
    }

    #[test]
    fn insert_empty_vector_at_front_matches_the_shutdown_fill() {
        let result = a71_vector_insert_00621050(&[], A71_STRING_INLINE_FLAGS, 0x1000, 0, 14);
        assert_eq!(result.new_count, 14);
        assert_eq!(result.new_capacity_flags, A71_STRING_INLINE_FLAGS);
        assert_eq!(
            (result.fill_start, result.split, result.fill_end),
            (0, 0, 14)
        );
        assert_eq!(result.allocation_bytes, None);
        assert_eq!(result.freed_old_data, None);
        assert_eq!(result.elements, vec![0; 14]);
    }

    #[test]
    fn insert_middle_vector_uses_pos_plus_n_split() {
        let result = a71_vector_insert_00621050(&[1, 2, 3, 4], 0x8000_0008, 0x1000, 1, 2);
        assert_eq!(result.new_count, 6);
        assert_eq!(
            (result.fill_start, result.split, result.fill_end),
            (1, 3, 3)
        );
        assert_eq!(result.elements, vec![1, 0, 0, 2, 3, 4]);
    }

    #[test]
    fn insert_near_tail_splits_at_old_count() {
        let result = a71_vector_insert_00621050(&[1, 2, 3, 4], 0x8000_0008, 0x1000, 3, 2);
        assert_eq!(result.new_count, 6);
        assert_eq!(
            (result.fill_start, result.split, result.fill_end),
            (3, 4, 5)
        );
        assert_eq!(result.elements, vec![1, 2, 3, 0, 0, 4]);
    }

    #[test]
    fn insert_grows_capacity_and_preserves_inline_bit() {
        let result = a71_vector_insert_00621050(&[1, 2, 3, 4], 0x0000_0004, 0x1000, 1, 3);
        assert_eq!(result.new_count, 7);
        assert_eq!(result.new_capacity_flags, 0x0000_0007);
        assert_eq!(result.allocation_bytes, Some(28));
        assert_eq!(result.freed_old_data, Some(0x1000));
        assert_eq!(
            (result.fill_start, result.split, result.fill_end),
            (1, 1, 4)
        );
        assert_eq!(result.elements, vec![1, 0, 0, 0, 2, 3, 4]);

        let result = a71_vector_insert_00621050(&[1, 2, 3, 4], A71_STRING_INLINE_FLAGS, 0, 1, 20);
        assert_eq!(result.new_count, 24);
        assert_eq!(result.new_capacity_flags, 0x8000_0018);
        assert_eq!(result.allocation_bytes, Some(96));
        assert_eq!(result.freed_old_data, None);
        assert_eq!(result.elements.len(), 24);
    }

    #[test]
    fn locale_constructor_keeps_short_string_inline() {
        let init = a71_locale_from_cstr_00621280(b"abc\0def", 0x77);
        assert_eq!(init.data_offset, A71_STRING_INLINE_OFFSET);
        assert_eq!(init.count, 3);
        assert_eq!(init.capacity_flags, A71_STRING_INLINE_FLAGS);
        assert_eq!(init.buffer, b"abc\0");
        assert_eq!(init.extra, 0x77);
        assert_eq!(init.allocation_bytes, None);
    }

    #[test]
    fn locale_constructor_keeps_exact_inline_capacity() {
        let source = b"0123456789abcde";
        let init = a71_locale_from_cstr_00621280(source, 0);
        assert_eq!(init.count, 15);
        assert_eq!(init.capacity_flags, A71_STRING_INLINE_FLAGS);
        assert_eq!(init.allocation_bytes, None);
        assert_eq!(init.buffer.len(), 16);
        assert_eq!(init.buffer[15], 0);
    }

    #[test]
    fn locale_constructor_grows_long_strings() {
        let init = a71_locale_from_cstr_00621280(b"0123456789abcdef", 1);
        assert_eq!(init.count, 16);
        assert_eq!(init.capacity_flags, 0x8000_0016);
        assert_eq!(init.allocation_bytes, Some(23));
        assert_eq!(init.data_offset, 0);
        assert_eq!(init.buffer.len(), 17);

        let source = vec![b'a'; 100];
        let init = a71_locale_from_cstr_00621280(&source, 2);
        assert_eq!(init.count, 100);
        assert_eq!(init.capacity_flags, 0x8000_0064);
        assert_eq!(init.allocation_bytes, Some(101));
    }

    #[test]
    fn misaligned_string_entry_reproduces_overlapping_arithmetic() {
        let result = a71_misaligned_string_00621297(0x1000, 0x10, 0x20, 0x2000, b"abc\0");
        assert_eq!(result.first_word, 0x1010);
        assert_eq!(result.eax_word, 0x40);
        assert_eq!(result.scan_start, 0x1fff);
        assert_eq!(
            result.header,
            A71VectorHeader {
                data: 0x100c,
                count: 0,
                capacity_flags: A71_STRING_INLINE_FLAGS,
            }
        );
        assert_eq!(result.inline_first_byte, 0);
        assert_eq!(result.reserve_len, 3);

        let result = a71_misaligned_string_00621297(0xffff_fff0, 0x20, 0x100, 0, b"");
        assert_eq!(result.first_word, 0x10);
        assert_eq!(result.scan_start, 0xffff_ffff);
        assert_eq!(result.reserve_len, 0);

        let result = a71_misaligned_string_00621297(0, 0, 0x80, 0, b"");
        assert_eq!(result.eax_word, 0);
    }

    #[test]
    fn destructor_plan_frees_only_heap_backed_words() {
        let plan = a71_destructor_plan_00621300(0x1000, -1, 0x2000, 0, 0x3000);
        assert_eq!(plan.handler, 0x00cb_f8f0);
        assert_eq!(plan.cookie, 0x00f4_e578 ^ 0x1000);
        assert_eq!(plan.frees, vec![0x3000]);

        let plan = a71_destructor_plan_00621300(0x1000, 0, 0x2000, -1, 0x3000);
        assert_eq!(plan.frees, vec![0x2000]);

        let plan = a71_destructor_plan_00621300(0x1000, -1, 0x2000, -1, 0x3000);
        assert!(plan.frees.is_empty());
    }

    #[test]
    fn misaligned_dealloc_arithmetic_matches_decoded_guard() {
        let result = a71_misaligned_dealloc_0062134a(0, 0);
        assert_eq!(result.value, 0);
        assert_eq!(result.low_byte, 0);
        assert!(result.trap);

        let result = a71_misaligned_dealloc_0062134a(0x10, 0);
        assert_eq!(result.value, 0x20);
        assert_eq!(result.low_byte, 0x20);
        assert!(result.trap);

        let result = a71_misaligned_dealloc_0062134a(0x40, 0);
        assert_eq!(result.value, 0x80);
        assert_eq!(result.low_byte, 0x80);
        assert!(!result.trap);

        let result = a71_misaligned_dealloc_0062134a(0x40, 0x80);
        assert_eq!(result.value, 0x80);
        assert_eq!(result.low_byte, 0);
        assert!(!result.trap);
    }

    #[test]
    fn stack_string_fragments_have_the_expected_stores() {
        let header = a71_stack_string_00621461(0x1234_5678);
        assert_eq!(header.data, 0x1234_5678);
        assert_eq!(header.count, 0);
        assert_eq!(header.capacity_flags, A71_STRING_INLINE_FLAGS);
        assert_eq!(header.first_byte, 0);

        let header = a71_stack_string_00621825(true, 0x1000).unwrap();
        assert_eq!(header.word588, 4);
        assert_eq!(header.word570, 2);
        assert_eq!(header.state_byte, 0x0b);
        assert_eq!(header.word568, 0);
        assert_eq!(header.data, 0x0aa0);
        assert_eq!(header.capacity_flags, A71_STRING_INLINE_FLAGS);
        assert_eq!(header.first_byte, 0);
        assert_eq!(header.reserve_arg, 4);
        assert!(a71_stack_string_00621825(false, 0x1000).is_none());
    }

    #[test]
    fn embedded_bytes_are_reproduced() {
        assert_eq!(a71_fragment_byte_00621511(), 0xff);
        assert_eq!(a71_fragment_byte_006218d1(), 0xf1);
        assert_eq!(a71_icebp_trap_006218d1(), A71Trap006218d1::Int1);
    }
}
