#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent51.rs
// ===========================================================================
// Translation batch 51.  Only this file is edited by the batch.
//
// Batch 51 owns `0x005f8000..=0x005f9fff`.  The ledger lists 60
// `decompiled_uncited` rows in the window: 30 are translated here and the
// other 30 are skipped for the reasons in [Skipped rows](#skipped-rows).
//
// The window is the launcher's command-line layer: the TCLAP-style `args::`
// classes (`Arg`, `Group`, `Subparser`, `CmdLine`) together with the MSVC
// `std::string` / `std::vector` support their inline bodies instantiate.
// Nothing here reads or writes simulation state, so the translations are
// deterministic [`World`] helpers over local value types; virtual calls whose
// targets live outside the window enter as caller-supplied closures or as
// value inputs, and every recovered constant is reproduced exactly.
//
// Recovered layouts:
//
// | offset | meaning |
// |--------|---------|
// | `+0x00` | `std::string` union (16-byte inline buffer or heap pointer) |
// | `+0x04` | per-object flags dword: bit `0x1` "only once", bit `0x2` "required" |
// | `+0x08` | byte state: seen/parsed (`0x005f8c10`, `FUN_005f93f0`, `0x005f9470`) |
// | `+0x10`/`+0x14` | `std::string` size / capacity |
// | `+0x24`/`+0x28` | child pointer vector, 4-byte slots (`0x005f8000`, `0x005f89c0`) |
// | `+0x54` | provider torn down by `0x005f8eb0` |
// | `+0x5c` | argument triple taken by `0x005f8aa0` |
// | `+0x68` | byte that selects the `+0x5c` triple in `0x005f8aa0` |
// | `+0xac`/`+0xb0` | 0x18-stride vector triple used by `0x005f93a0` |
// | `+0xc4` (`+0xd4` size, `+0xd8` cap) | string released by `0x005f9410` |
// | `+0xfc` | own provider probed by `0x005f8b70`/`0x005f98f0` |
// | `+0x100`/`+0x104` | 0x34-stride pair list cleared by `0x005f9470` |
// | `+0x10c`/`+0x110` | vector range destroyed by `0x005f9470` |
// | `+0x118`/`+0x119`/`+0x11a` | command/validation gate bytes |
// | `+0x11c` | current subparser head pushed by `0x005f9740` |
// | `+0x150` -> `+0x1f8` | member string copied by `0x005f9570` (`+0x208` size, `+0x20c` cap) |
// | `+0x180..+0x183` | render flags set by `0x005f9570` |
// | `+0x188` | reset flag cleared by `0x005f9470` |
//
// An MSVC `std::string` is 24 bytes: the union at `+0x00`, `size` at `+0x10`
// and `capacity` at `+0x14`.  Capacities above `0xf` select the heap pointer
// (`0x005f8d40`'s `0xf < *(uint *)(param_1 + 0x18)`), otherwise the bytes
// are inline.  [`A51String`] keeps the bytes and the capacity; the storage
// choice is [`A51String::is_heap`].
//
// ## Skipped rows
//
// * `FUN_005f82b4`, `FUN_005f83b4`, `FUN_005f84a6` - three 15-byte catch-handler
//   steps (`*(edi + 4) += 1`, `*(ebp - 4) = 2/4/6`, then `jmp 0x005f853a`).
//   Only the stored state constant differs; the shared tail is translated as
//   [`World::a51_unwind_tail`] and the incremented register plus the state
//   slot are caller-frame locals of the funclet, not routine arguments.
// * `FUN_005f8675`, `FUN_005f86a0`, `FUN_005f8719`, `FUN_005f87aa`, `FUN_005f8823`,
//   `FUN_005f88a6` - exception landing pads of the same parser routines.  Every
//   operand is an uninitialised `unaff_EBP` slot (`-0x24`, `-0x40`, `-0x54`,
//   `-0x100`) owned by the caller's frame; the only self-contained literal is
//   the `0xfffffffd` mask that clears bit 1 of a flag dword, and each body
//   ends in the translated `0x005f853a` tail.
// * `FUN_005f8784` - landing pad that stores the 10-byte literal `"--profile="`
//   (`0x6c69666f72702d2d`, `0x3d65`); the destination buffer, the insertion
//   index and the bounds word are all uninitialised caller-frame slots, so
//   only the literal survives (recorded here rather than re-emitted as a
//   function with no arguments).
// * `FUN_005f88dd` - the `"--noGame"` guard; the flag arrives in `AL` and the
//   `FUN_0067d9b0(*ebx)` argument is an uninitialised slot, the remainder is
//   the shared tail.
// * `FUN_005f8c60` (1 byte) and `FUN_005f93ed` (`__break`, 1 byte) - the
//   decoder's `halt_baddata()` and `int3` markers; the cached bodies contain
//   no instruction stream.
// * `FUN_005f8ea0` - 4-byte accessor, `return *(byte *)(this + 8)`; the same
//   byte is the `active` field of [`A51GroupState`], so a second translation
//   would only duplicate it.
// * `FUN_005f8ea7` - 9-byte SEH prologue that rebuilds the trap frame from the
//   individual `in_OF`/`in_NT`/`in_VIF` flag inputs and composes a fresh
//   `ExceptionList` chain before calling `(*(*this + 0x54))[2]`.  The frame
//   arithmetic is not recoverable from the artefact.
// * `FUN_005f8f20`, `FUN_005f8f40` - 23-byte `{0, 0}` / `{1, 1}` pair stores.
//   They are the inline constructors of the `+0xac` triple whose teardown is
//   [`World::a51_destroy_vector_18`]; the batch budget of 30 rows was spent
//   on the routines with recoverable control flow.
// * `FUN_005f8f60`, `FUN_005f90e0` - two non-returning tails
//   (`FUN_00538810(*(param + 4))` behind an SEH frame, and
//   `FUN_0067d6b0(begin, end, &DAT_00d386b0, 2)`) whose only datum is a
//   caller-frame slot / a constant report site already represented by
//   [`A51Tail::Report`].
// * `FUN_005f93f0` - 17-byte reset (`*(this + 8) = 0`, `*(this + 0xac) =
//   *(this + 0xb0)`); represented by [`World::a51_reset_and_release_c4`] and
//   [`A51GroupState`], whose `active` field is the same byte.
// * `FUN_005f988d`, `FUN_005f98dc` - the landing pad and `Catch` thunk of
//   `0x005f9740`; the pad only restores the saved `+0x11c` head (translated
//   as [`World::a51_install_subparser`]'s post-condition) and the thunk is a
//   bare `return 0x5f9891` label.
// * `FUN_005f9be0`, `FUN_005f9c0d`, `FUN_005f9c30`, `FUN_005f9c60`, `FUN_005f9d7b` -
//   five tail-only stubs: `__purecall`, a `ZF`-selected trap pair, two
//   `FUN_00923df0(0xc)` / `FUN_00923df0(0x20)` allocation-size constants and
//   the `if (size < 0x20)` free guard.  They carry no arithmetic beyond the
//   constant named in this bullet.
// * `FUN_005f9ef0` - byte-for-byte the same body as `0x005f9e50` except for the
//   rehash helper it tail-calls (`0x005ee7f0` instead of `FUN_005eeb50`); the
//   arithmetic is translated once in [`World::a51_grow_buckets`].
// * `FUN_005f9f90`, `FUN_005f9fb0` - `std::_Func_impl` vtable copy and the
//   `std::list` splice half of a hash-table copy: both bodies start from
//   uninitialised `in_ECX`/`param_1` pairs and only move vtable pointers and
//   eight-byte node headers, which carry no numeric or string behaviour.

/// Inline (`std::string`) capacity written by every constructor in the
/// window (`+0x14 = 0xf` in `0x005f80c0`, the temporaries of `0x005f8aa0`).
pub const A51_SSO_CAPACITY: u32 = 0xf;

/// Element size larger than this takes the `begin - 4` block-header path
/// (`0x005f9c90`, `0x005f9d30`, `0x005f9dc0`: `if (0xfff < byte_len)`).
pub const A51_LARGE_BLOCK: u32 = 0xfff;

/// Added to the block byte length on the large path (`uVar2 + 0x23`).
pub const A51_BLOCK_HEADER: u32 = 0x23;

/// Allowed slack between a large block's stored pointer and its data
/// (`0x1f < (begin - stored) - 4` is the `FUN_00c9474b` trap).
pub const A51_ALIGN_SLACK: u32 = 0x1f;

/// Stride of the requirement vector (`0x005f8130`, `0x005f9c90`).
pub const A51_ELEMENT_1C: u32 = 0x1c;

/// Stride of the `+0xac` message vector (`0x005f93a0`, `0x005f9d30`).
pub const A51_ELEMENT_18: u32 = 0x18;

/// Stride of the `+0x100` pair list (`0x005f9470`, `0x005f9dc0`).
pub const A51_ELEMENT_34: u32 = 0x34;

/// Size of one child pointer slot in the `+0x24`/`+0x28` vector; the count
/// formula `(end - begin + 3) >> 2` equals the element count for aligned
/// ranges.
pub const A51_CHILD_SLOT: u32 = 4;

/// FNV-1a basis used by both hash-table probes of `0x005f8d40`
/// (`0x811c9dc5`).
pub const A51_FNV_BASIS: u32 = 0x811c_9dc5;

/// FNV-1a prime (`0x1000193`).
pub const A51_FNV_PRIME: u32 = 0x0100_0193;

/// Flags bit `0x1`: the argument may be passed only once (`0x005f8c10`).
pub const A51_FLAG_ONLY_ONCE: u32 = 0x1;

/// Flags bit `0x2`: the argument is required (`0x005f9a80`'s
/// `((uint)in_ECX[1] >> 1 & 1)`).
pub const A51_FLAG_REQUIRED: u32 = 0x2;

/// Node flags bit `0x4`: excluded by the `0x005f8000` collector.
pub const A51_FLAG_SKIP: u8 = 0x4;

/// Node flags bit `0x10`: the only children probed by the linked arm of
/// `0x005f8b70`.
pub const A51_FLAG_LINKED: u8 = 0x10;

/// Linear bucket threshold of the grow policy: below `0x200` the capacity is
/// multiplied by eight (`(int)cap << 3`).
pub const A51_BUCKET_LINEAR: u32 = 0x200;

/// Doubling threshold: below `0x1fffffff` the capacity is doubled.
pub const A51_BUCKET_LIMIT: u32 = 0x1fff_ffff;

/// Value forced by `0x005f91fe` when its flag byte is non-zero.
pub const A51_VERBOSITY_OVERRIDE: u32 = 5;

/// The `+0x1c` addend of the grow ratio (`(double)x + *(&DAT_00d33180 -
/// sign bit)`); the table holds the conventional `0.5`.
pub const A51_HALF: f64 = 0.5;

/// Message built by `0x005f98f0` before it throws.
pub const A51_GROUP_VALIDATION_FAILED: &str = "Group validation failed somewhere!";

/// Message built by `0x005f98f0` when a required command is missing.
pub const A51_COMMAND_REQUIRED: &str = "Command is required";

/// Source literal passed to `FUN_0067d6b0` by `0x005f853a`
/// (`&DAT_00d38aa4`, count `3`).
pub const A51_REPORT_SITE: u32 = 0x00d3_8aa4;

/// The bits reported by the short-key path of `0x005f8c10`
/// (`*(byte *)(in_ECX + 4) & 1`).
pub const A51_ONLY_ONCE_MASK: u32 = 0x1;

/// The shared `std::string` value model: `text` holds the bytes, `cap` is the
/// value at `+0x14`.  A capacity of `0xf` or less means the inline buffer is
/// live, a larger one means the union at `+0x00` holds a heap pointer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A51String {
    pub text: String,
    pub cap: u32,
}

impl A51String {
    /// `0xf < capacity` selects the pointer at `+0x00` rather than the inline
    /// buffer (`0x005f8d40`, `0x005f93a0`, `0x005f9410`).
    pub fn is_heap(&self) -> bool {
        self.cap > A51_SSO_CAPACITY
    }
}

/// A child-list node of the `+0x24`/`+0x28` vector: flags byte at `+4` and an
/// integer code the out-of-window probes return.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A51Child {
    pub flags: u8,
    pub text: A51String,
    pub code: i32,
}

/// The 0x1c requirement record written by `0x005f80c0`: kind byte at `+0`,
/// value byte at `+1` and a `std::string` at `+4` (`+0x14` size, `+0x18`
/// capacity).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A51Requirement {
    pub kind: u8,
    pub value: u8,
    pub text: A51String,
}

/// The three selector inputs of `0x005f80c0`: the dword at `+8`, whether the
/// `+0x28` child end pointer is non-zero, the first child's `+8` string and
/// the byte reached through `**(this + 4) + 8`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A51RequirementSource {
    pub selector: u32,
    pub has_children: bool,
    pub child_text: Option<A51String>,
    pub own_value: Option<u8>,
}

/// Result of `0x005f8130`: the zeroed triple plus the two addresses the body
/// compares (`begin + 0x1c` against `begin + count * 0x1c`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A51TripleCheck {
    pub triple: [u32; 3],
    pub one_past_first: u32,
    pub end: u32,
    pub matched: bool,
}

/// The non-returning tails shared by the catch funclets of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A51Tail {
    /// `FUN_0093ec70`: `_invalid_parameter_noinfo_noreturn` (the recovered
    /// guard is `if ((state & 0x80000000) == 0)`).
    InvalidParameter,
    /// `FUN_00c86b6e`: restore the exception list and rethrow; no value
    /// reaches the caller.
    Rethrow,
    /// `FUN_0067d6b0(first, second, &DAT_00d38aa4, 3)`.
    Report {
        first: u32,
        second: u32,
        source: u32,
        line: u32,
    },
}

/// Outcome of the `+0x54` teardown of `0x005f8eb0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A51Teardown {
    /// The virtual `+8` slot ran; the routine returns to its caller.
    Dispatched,
    /// `FUN_00c84194` followed by `int3`: the field was null.
    InvalidParameter,
}

/// Observable header of the `std::string` element the vector destructors
/// release: the union pointer at `+0x00`, the size at `+0x10` and the
/// capacity at `+0x14`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A51StringSlot {
    pub data: u32,
    pub size: u32,
    pub cap: u32,
}

impl A51StringSlot {
    pub fn is_heap(&self) -> bool {
        self.cap > A51_SSO_CAPACITY
    }
}

/// Result of the `0x18`-stride vector destructor `0x005f93a0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A51Dtor18 {
    /// `(end - begin) / 0x18 + sign == sign` - the recovered guard calls
    /// `FUN_005fd630` and then hits `int3`.
    ZeroCountTrap,
    /// `begin == this + 0xac`: the member aliases the vector start, so
    /// nothing is released.
    Aliased,
    /// `FUN_005fdb20(pointer, size)`.
    Released { pointer: u32, size: u32 },
}

/// Outcome of the subparser installation `0x005f9740`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A51Install {
    /// `this + 0xf4 == 0`: the whole body is skipped and the head is left
    /// untouched.
    Skipped,
    /// The `+0x11c` head was replaced by the new node, dispatched through
    /// `vtable + 8`, then restored to `saved`.
    Dispatched { saved: Option<u32> },
}

/// The four flags `0x005f9570` stores at `+0x180..+0x183` plus the separator
/// its local string receives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A51RenderSetup {
    /// `[+0x180, +0x181, +0x182, +0x183]` in store order
    /// (`param_1, param_2, param_3, param_4`).
    pub flags: [u8; 4],
    /// `" "` when `+0x180 == 0`, otherwise the empty string.
    pub separator: &'static str,
    /// The copy of the `+0x150` member that lands at `+0x1f8`.
    pub copied: A51String,
}

/// The mutable part of the group object reset by `0x005f9470`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A51GroupState {
    /// `+0x08`: cleared on entry, set to `1` at the end of the reset.
    pub active: u8,
    /// `+0x24`: first child pointer slot.
    pub child_begin: u32,
    /// `+0x28`: one past the last child pointer slot.
    pub child_end: u32,
    /// `+0xfc`: own provider reference.
    pub provider: u32,
    /// `+0x100`: first `0x34` pair record.
    pub list_begin: u32,
    /// `+0x104`: one past the last `0x34` pair record.
    pub list_end: u32,
    /// `+0x10c`: first element of the destroyed vector range.
    pub vec_begin: u32,
    /// `+0x110`: one past that range.
    pub vec_end: u32,
    /// `+0x118`: the two gate bytes (`+0x118`, `+0x119`).
    pub flags: u16,
    /// `+0x11a`: the "command is required" gate.
    pub extra: u8,
    /// `+0x188`: the trailing reset byte.
    pub tail: u8,
}

/// What `0x005f9470` did to the fields of [`A51GroupState`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A51ResetReport {
    /// Number of `vtable + 0x34` child probes run.
    pub children: u32,
    /// The `[begin, end)` range handed to `FUN_005ed2b0`, before `+0x110` was
    /// collapsed onto `+0x10c`.
    pub released: (u32, u32),
    /// Number of `0x34`-stride records destroyed (`FUN_005f34c0`).
    pub list_elements: u32,
}

/// Per-child results of the three virtual probes `0x005f98f0` runs before it
/// visits a group: `vtable + 0x14` (validate), `vtable + 4` (ok) and
/// `vtable + 0x2c` (requires a command).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A51GroupChild {
    pub validates: bool,
    pub ok: bool,
    pub requires_command: bool,
}

/// What `0x005f98f0` did before returning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A51VisitReport {
    /// Number of `vtable + 8` visits performed.
    pub visited: u32,
    /// Whether the `+0x11c` subparser received its `vtable + 8` call.
    pub subparser_dispatched: bool,
}

/// The 0x1c key variant probed by `0x005f8d40`: tag byte at `+0x00`, short
/// key byte at `+0x01`, and a `std::string` at `+0x04` (`+0x14` size,
/// `+0x18` capacity).  A non-zero tag takes the short-key path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A51FlagQuery {
    pub tag: u8,
    pub key: u8,
    pub text: A51String,
}

/// Node of the short-key table (`this + 0xc`): each bucket stores
/// `{first, last}` and the key byte sits at node `+8`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A51ShortFlag {
    pub bucket: u32,
    pub key: u8,
}

/// Node of the long-key table (`this + 0x2c`): a `std::string` at node `+8`
/// (`size` at `+0x18`, `capacity` at `+0x1c`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A51LongFlag {
    pub bucket: u32,
    pub text: A51String,
}

/// Result of the reserve paths `0x005f9c90` / `0x005f9d30`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A51Reserve {
    /// `*this == 0`: the triple was installed from the arguments.
    Initialized { begin: u32, end: u32, cap: u32 },
    /// The old block was reported to `FUN_00923e90(data, size)`.
    Released { data: u32, size: u32 },
    /// `0x1f < (begin - stored) - 4`: `FUN_00c9474b` traps.
    Misaligned { begin: u32, stored: u32 },
}

/// Result of the `0x34`-stride reserve path `0x005f9dc0`, which destroys the
/// old elements before releasing the block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A51Reserve34 {
    Initialized {
        begin: u32,
        end: u32,
        cap: u32,
    },
    Released {
        data: u32,
        size: u32,
        destroyed: u32,
    },
    Misaligned {
        begin: u32,
        stored: u32,
    },
}

/// The block-class decision shared by the three reserve paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A51LargeBlock {
    /// `byte_len <= 0xfff`: `FUN_00923e90(begin, byte_len)`.
    Small { bytes: u32 },
    /// `byte_len > 0xfff`: `FUN_00923e90(*(begin - 4), byte_len + 0x23)`.
    Large { stored: u32, bytes: u32 },
    /// `0x1f < (begin - stored) - 4`: `FUN_00c9474b` traps.
    Misaligned { begin: u32, stored: u32 },
}

/// Result of the bucket-grow trigger of `0x005f9e50`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct A51Grow {
    /// `(count + 0.5) / (capacity + 0.5)` computed in double, stored as float.
    pub ratio: f32,
    /// The capacity the grow policy selects.
    pub target: u32,
    /// `load <= ratio && ratio != load`.
    pub grow: bool,
}

/// A fresh inline string: no bytes, capacity `0xf`.
pub fn a51_string_empty() -> A51String {
    A51String {
        text: String::new(),
        cap: A51_SSO_CAPACITY,
    }
}

/// One FNV-1a round per byte with the recovered `0x811c9dc5` /
/// `0x1000193` pair, as used by both arms of `0x005f8d40`
/// (`(*pbVar1 ^ h) * 0x1000193`).
pub fn a51_fnv1a(bytes: &[u8]) -> u32 {
    bytes.iter().fold(A51_FNV_BASIS, |hash, byte| {
        (*byte as u32 ^ hash).wrapping_mul(A51_FNV_PRIME)
    })
}

/// The bucket index `0x005f8d40` computes: `mask & hash`, with `mask` read
/// from `this + 0x18` for the short table and `this + 0x38` for the long one.
pub fn a51_hash_bucket(mask: u32, hash: u32) -> u32 {
    mask & hash
}

/// Child count recovered from the `+0x24`/`+0x28` vector loops
/// (`0x005f9470`, `0x005f9530`, `0x005f9ba0`):
/// `(end - begin + 3) >> 2`, forced to zero when `end < begin`.
pub fn a51_child_count(begin: u32, end: u32) -> u32 {
    if end < begin {
        0
    } else {
        end.wrapping_sub(begin).wrapping_add(3) >> 2
    }
}

/// Capacity bytes of a vector triple, as the reserve paths compute it:
/// `((cap - begin) / stride) * stride` with a truncating signed division
/// (`0x005f9c90`, `0x005f9d30`, `0x005f9dc0`).
pub fn a51_capacity_bytes(begin: u32, cap: u32, stride: u32) -> u32 {
    let diff = cap.wrapping_sub(begin) as i32;
    let stride = stride as i32;
    (diff / stride).wrapping_mul(stride) as u32
}

/// The large-block arm shared by the reserve paths:
///
/// ```c
/// if (0xfff < bytes) {
///   stored = *(begin - 4);
///   bytes  = bytes + 0x23;
///   if (0x1f < (begin - stored) - 4) trap;
/// }
/// ```
pub fn a51_large_block(begin: u32, stored: u32, bytes: u32) -> A51LargeBlock {
    if bytes <= A51_LARGE_BLOCK {
        return A51LargeBlock::Small { bytes };
    }
    let grown = bytes.wrapping_add(A51_BLOCK_HEADER);
    if A51_ALIGN_SLACK < begin.wrapping_sub(stored).wrapping_sub(4) {
        A51LargeBlock::Misaligned { begin, stored }
    } else {
        A51LargeBlock::Large {
            stored,
            bytes: grown,
        }
    }
}

/// The ratio `0x005f9e50` computes: the numerator and denominator each add
/// the `&DAT_00d33180` entry selected by their sign bit (the conventional
/// `0.5`), the division happens in double and the result is stored as float.
pub fn a51_grow_ratio(count: i32, capacity: i32) -> f32 {
    ((count as f64 + A51_HALF) / (capacity as f64 + A51_HALF)) as f32
}

/// The capacity the grow policy selects:
///
/// ```c
/// if ((uint)cap < 0x200)      cap = (int)cap << 3;
/// else if ((uint)cap < 0x1fffffff) cap = (int)cap * 2;
/// ```
pub fn a51_grow_target(capacity: u32) -> u32 {
    if capacity < A51_BUCKET_LINEAR {
        (capacity as i32).wrapping_shl(3) as u32
    } else if capacity < A51_BUCKET_LIMIT {
        capacity.wrapping_mul(2)
    } else {
        capacity
    }
}

/// `0x005f91fe` (`FUN_005f91fe`, 19 bytes, `decompiled_uncited`).
///
/// ```c
/// if (in_AL != '\0') in_ECX = 5;
/// DAT_00f04754 = in_ECX;
/// ```
///
/// The flag byte selects the hard-coded `5`; the incoming value is otherwise
/// stored unchanged into the boot verbosity global.
pub fn a51_boot_verbosity(value: u32, override_flag: bool) -> u32 {
    if override_flag {
        A51_VERBOSITY_OVERRIDE
    } else {
        value
    }
}

impl World {
    /// `0x005f8000` (`FUN_005f8000`, 183 bytes, `decompiled_uncited`).
    ///
    /// Appends one 8-byte probe result per child whose flags byte at node
    /// `+4` lacks bit `0x4`:
    ///
    /// ```c
    /// *param_1 = 0; param_1[1] = 0; param_1[2] = 0;      /* result vector */
    /// for (p = *(int **)(this + 0x24); p != *(int **)(this + 0x28); ++p)
    ///   if ((*(byte *)((int *)*p + 1) & 4) == 0) {       /* node + 4 */
    ///     (**(code **)(*(int *)*p + 0x30))(&local_20, param_2, cookie);
    ///     FUN_005ef420(param_1[1], local_20, local_1c, param_1);
    ///     FUN_005fd300();                                /* temp destructor */
    ///   }
    /// ```
    ///
    /// The vtable slot `+0x30` lives outside the window, so `probe` receives
    /// the child index and the caller's argument and returns the two dwords
    /// (`local_20`, `local_1c`) that the retail `push_back` copies.
    pub fn a51_collect_probe_pairs<F>(
        &self,
        children: &[A51Child],
        arg: u32,
        mut probe: F,
    ) -> Vec<[u32; 2]>
    where
        F: FnMut(usize, u32) -> [u32; 2],
    {
        let mut out = Vec::new();
        for (index, child) in children.iter().enumerate() {
            if child.flags & A51_FLAG_SKIP != 0 {
                continue;
            }
            out.push(probe(index, arg));
        }
        out
    }

    /// `0x005f80c0` (`FUN_005f80c0`, 97 bytes, `decompiled_uncited`).
    ///
    /// Builds the 0x1c requirement record from the three selector inputs:
    ///
    /// ```c
    /// if (*(int *)(this + 8) == 0) {
    ///   if (*(int *)(this + 0x28) != 0) { *param_1 = 0; FUN_005f12e0(first + 8); }
    ///   else *param_1 = 0x2001;                    /* kind 1, value 0x20 */
    /// } else {
    ///   *(byte *)param_1 = 1;
    ///   *(byte *)((char *)param_1 + 1) = *(byte *)(**(int **)(this + 4) + 8);
    /// }
    /// *(u32 *)((char *)param_1 + 0x14) = 0;        /* size */
    /// *(u32 *)((char *)param_1 + 0x18) = 0xf;      /* capacity */
    /// *(byte *)((char *)param_1 + 4) = 0;          /* inline buffer */
    /// ```
    ///
    /// `0x2001` is the little-endian pair `kind = 1`, `value = 0x20`; the
    /// `this + 8 == 0` arm instead selects kind `0` (the named form) and
    /// copies the first child's `+8` string.
    pub fn a51_requirement_from_source(&self, source: &A51RequirementSource) -> A51Requirement {
        if source.selector == 0 {
            if source.has_children {
                A51Requirement {
                    kind: 0,
                    value: 0,
                    text: source.child_text.clone().unwrap_or_else(a51_string_empty),
                }
            } else {
                A51Requirement {
                    kind: 1,
                    value: 0x20,
                    text: a51_string_empty(),
                }
            }
        } else {
            A51Requirement {
                kind: 1,
                value: source.own_value.unwrap_or(0),
                text: a51_string_empty(),
            }
        }
    }

    /// `0x005f8130` (`FUN_005f8130`, 185 bytes, `decompiled_uncited`).
    ///
    /// Zeroes the destination triple, then validates a one-element vector:
    ///
    /// ```c
    /// *param_1 = 0; param_1[1] = 0; param_1[2] = 0;
    /// iVar1 = *param_2;                       /* begin */
    /// if (iVar1 + 0x1c != *param_2 + param_2[1] * 0x1c)   /* count */
    ///   FUN_0067d6b0(*(iVar1 + 0x1c), *(iVar1 + 0x20), &DAT_00d38aa4, 3);
    /// FUN_00c86b6e();
    /// ```
    ///
    /// The comparison addresses are reported so the caller can see both the
    /// `begin + 0x1c` form and the `begin + count * 0x1c` form; `matched` is
    /// false exactly when the debug report fires.
    pub fn a51_init_one_element_triple(&self, begin: u32, count: u32) -> A51TripleCheck {
        let one_past_first = begin.wrapping_add(A51_ELEMENT_1C);
        let end = begin.wrapping_add(count.wrapping_mul(A51_ELEMENT_1C));
        A51TripleCheck {
            triple: [0; 3],
            one_past_first,
            end,
            matched: one_past_first == end,
        }
    }

    /// `0x005f853a` (`FUN_005f853a`, 21 bytes, `decompiled_uncited`).
    ///
    /// The tail every catch funclet of the window shares:
    ///
    /// ```c
    /// if ((*(uint *)(ebp - 0x24) & 0x80000000) == 0) FUN_0093ec70();  /* trap */
    /// *(byte *)(ebp - 4) = 0;                         /* state = 0 */
    /// if (ebx + 0x1c == begin + count * 0x1c) { restore; FUN_00c86b6e(); }
    /// FUN_0067d6b0(*(ebx + 0x1c), *(ebx + 0x20), &DAT_00d38aa4, 3);
    /// ```
    ///
    /// `state` is the cookie dword the funclet tests for its high bit,
    /// `element_end` the `ebx + 0x1c` address, `element_second` the
    /// `ebx + 0x20` dword handed to the report, and `tuple_end` the
    /// `begin + count * 0x1c` end pointer.  The recovered body also stores
    /// `0` into the funclet's state byte before the comparison.
    pub fn a51_unwind_tail(
        &self,
        state: u32,
        element_end: u32,
        tuple_end: u32,
        element_second: u32,
    ) -> A51Tail {
        if state & 0x8000_0000 == 0 {
            return A51Tail::InvalidParameter;
        }
        if element_end == tuple_end {
            return A51Tail::Rethrow;
        }
        A51Tail::Report {
            first: element_end,
            second: element_second,
            source: A51_REPORT_SITE,
            line: 3,
        }
    }

    /// `0x005f8990` (`FUN_005f8990`, 46 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// for (p = *(int **)(this + 0x24); p != *(int **)(this + 0x28); ++p)
    ///   if ((**(code **)(*(int *)*p + 0x2c))() != '\0') return 1;
    /// return 0;
    /// ```
    ///
    /// The first child whose `vtable + 0x2c` predicate is true ends the scan.
    pub fn a51_any_child_flag_2c<F>(&self, children: &[A51Child], mut probe: F) -> bool
    where
        F: FnMut(&A51Child) -> bool,
    {
        children.iter().any(|child| probe(child))
    }

    /// `0x005f89c0` (`FUN_005f89c0`, 51 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// if (*(char *)(this + 0x118) == '\0') {
    ///   for (p = *(int **)(this + 0x24); p != *(int **)(this + 0x28); ++p)
    ///     if ((**(code **)(*(int *)*p + 0x24))() != '\0') break;
    ///   else return 0;
    /// }
    /// return 1;
    /// ```
    ///
    /// A non-zero `+0x118` gate short-circuits to true without probing.
    pub fn a51_any_child_flag_24_unless_118<F>(
        &self,
        gate_118: bool,
        children: &[A51Child],
        mut probe: F,
    ) -> bool
    where
        F: FnMut(&A51Child) -> bool,
    {
        if gate_118 {
            return true;
        }
        children.iter().any(|child| probe(child))
    }

    /// `0x005f8a00` (`FUN_005f8a00`, 46 bytes, `decompiled_uncited`).
    ///
    /// The ungated twin of `0x005f89c0`: scan the child vector and stop at the
    /// first `vtable + 0x24` hit.
    pub fn a51_any_child_flag_24<F>(&self, children: &[A51Child], mut probe: F) -> bool
    where
        F: FnMut(&A51Child) -> bool,
    {
        children.iter().any(|child| probe(child))
    }

    /// `0x005f8a30` (`FUN_005f8a30`, 51 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// if (*(char *)(this + 0x119) == '\0') { ... vtable + 0x28 ... }
    /// return 1;
    /// ```
    ///
    /// The gate is the *high* byte of the `+0x118` word.
    pub fn a51_any_child_flag_28_unless_119<F>(
        &self,
        gate_119: bool,
        children: &[A51Child],
        mut probe: F,
    ) -> bool
    where
        F: FnMut(&A51Child) -> bool,
    {
        if gate_119 {
            return true;
        }
        children.iter().any(|child| probe(child))
    }

    /// `0x005f8a70` (`FUN_005f8a70`, 46 bytes, `decompiled_uncited`).
    ///
    /// The ungated twin of `0x005f8a30` (`vtable + 0x28`).
    pub fn a51_any_child_flag_28<F>(&self, children: &[A51Child], mut probe: F) -> bool
    where
        F: FnMut(&A51Child) -> bool,
    {
        children.iter().any(|child| probe(child))
    }

    /// `0x005f8aa0` (`FUN_005f8aa0`, 204 bytes, `decompiled_uncited`).
    ///
    /// Moves a 3-dword argument triple out of either a vtable result or the
    /// member at `+0x5c`:
    ///
    /// ```c
    /// bVar2 = *(byte *)(this + 0x68) == 0;
    /// puVar1 = bVar2 ? (vtable + 0x3c)(&tmp, param_2) : (this + 0x5c);
    /// *param_1 = 0; param_1[1] = 0; param_1[2] = 0;
    /// *param_1 = *puVar1; param_1[1] = puVar1[1]; param_1[2] = puVar1[2];
    /// *puVar1 = 0; puVar1[1] = 0; puVar1[2] = 0;
    /// ```
    ///
    /// The returned pair is `(destination, source after the move)`; the
    /// member is only cleared when it is the source, which is exactly the
    /// `byte +0x68 != 0` arm.
    pub fn a51_take_argument_triple<F>(
        &self,
        member_present: bool,
        member: [u32; 3],
        mut probe: F,
    ) -> ([u32; 3], [u32; 3])
    where
        F: FnMut() -> [u32; 3],
    {
        if member_present {
            (member, [0; 3])
        } else {
            (probe(), member)
        }
    }

    /// `0x005f8b70` (`FUN_005f8b70`, 156 bytes, `decompiled_uncited`).
    ///
    /// Three-level validation walk:
    ///
    /// ```c
    /// if (this->own == 0) {                       /* this + 0xfc */
    ///   if (*(int **)(this + 0x11c) != 0) return subparser->vtable_0x18(arg);
    ///   if (!(*vtable + 4)()) return 0;
    ///   for (p = this->children; p != end; ++p)
    ///     if ((code = (*p)->vtable_0x18(arg)) != 0) return code;
    /// } else {
    ///   if ((code = own->vtable_0x18(arg)) != 0) return code;
    ///   for (p = children; p != end; ++p)
    ///     if ((flags(*p) & 0x10) != 0 && (code = (*p)->vtable_0x18(arg)) != 0)
    ///       return code;
    /// }
    /// return 0;
    /// ```
    ///
    /// The own/subparser nodes are opaque ids, the children are
    /// [`A51Child`] values (their flags byte carries the `0x10` bit).
    pub fn a51_validate_node<FO, FC>(
        &self,
        own: Option<u32>,
        subparser: Option<u32>,
        self_ok: bool,
        children: &[A51Child],
        arg: u32,
        mut probe_node: FO,
        mut probe_child: FC,
    ) -> i32
    where
        FO: FnMut(u32, u32) -> i32,
        FC: FnMut(&A51Child, u32) -> i32,
    {
        if let Some(node) = own {
            let code = probe_node(node, arg);
            if code != 0 {
                return code;
            }
            for child in children {
                if child.flags & A51_FLAG_LINKED == 0 {
                    continue;
                }
                let code = probe_child(child, arg);
                if code != 0 {
                    return code;
                }
            }
            return 0;
        }
        if let Some(node) = subparser {
            return probe_node(node, arg);
        }
        if !self_ok {
            return 0;
        }
        for child in children {
            let code = probe_child(child, arg);
            if code != 0 {
                return code;
            }
        }
        0
    }

    /// `0x005f8c10` (`FUN_005f8c10`, 244 bytes, `decompiled_uncited`).
    ///
    /// Duplicate-flag bookkeeping:
    ///
    /// ```c
    /// if (FUN_005f8d40(key) == 0) return;         /* the flag is not in the map */
    /// if ((*(byte *)(this + 4) & 1) != 0 && *(char *)(this + 8) != 0) {
    ///   "Flag '" + unparse(arg) +
    ///   "' was passed multiple times, but is only allowed to be passed once"
    ///   -> throw
    /// }
    /// *(char *)(this + 8) = 1;
    /// ```
    ///
    /// Returns the exact message when the error path fires; otherwise marks
    /// the `+8` byte (when the flag exists) and returns `None`.
    pub fn a51_mark_flag_seen(
        &self,
        exists: bool,
        flags: u32,
        seen: &mut bool,
        label: &str,
    ) -> Option<String> {
        if !exists {
            return None;
        }
        if flags & A51_ONLY_ONCE_MASK != 0 && *seen {
            return Some(format!(
                "Flag '{label}' was passed multiple times, but is only allowed to be passed once"
            ));
        }
        *seen = true;
        None
    }

    /// `0x005f8d10` (`FUN_005f8d10`, 47 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// for (p = this->children; p != end; ++p)
    ///   if ((code = (*p)->vtable_0x18(arg)) != 0) return code;
    /// return 0;
    /// ```
    ///
    /// Returns the first non-zero probe code.
    pub fn a51_first_child_status<F>(&self, children: &[A51Child], mut probe: F) -> i32
    where
        F: FnMut(&A51Child) -> i32,
    {
        for child in children {
            let code = probe(child);
            if code != 0 {
                return code;
            }
        }
        0
    }

    /// `0x005f8d40` (`FUN_005f8d40`, 320 bytes, `decompiled_uncited`).
    ///
    /// The flag-table membership probe.  The key object is a 0x1c variant
    /// ([`A51FlagQuery`]): a non-zero tag at `+0x00` selects the short-key
    /// table, a zero tag selects the long-key table.
    ///
    /// Short path (`*param_1 != '\0'`):
    ///
    /// ```c
    /// h = mask_18 & ((byte)param_1[1] ^ 0x811c9dc5) * 0x1000193;
    /// bucket = *(this + 0xc) + h * 8;                 /* {first, last} */
    /// walk the chain comparing *(byte *)(node + 8) == param_1[1];
    /// ```
    ///
    /// Long path:
    ///
    /// ```c
    /// size = *(uint *)(param_1 + 0x14);
    /// data = (0xf < *(uint *)(param_1 + 0x18)) ? *(char **)(param_1 + 4) : param_1 + 4;
    /// for (i = 0; i < size; ++i) h = (data[i] ^ h) * 0x1000193;
    /// bucket = *(this + 0x38) & h;                    /* *(this + 0x2c) array */
    /// walk the chain comparing with FUN_005f02b0 in both directions;
    /// ```
    ///
    /// Both comparisons reduce to byte equality, so the model walks the nodes
    /// of the selected table in bucket `mask & hash`.
    pub fn a51_lookup_flag(
        &self,
        query: &A51FlagQuery,
        short_mask: u32,
        short_nodes: &[A51ShortFlag],
        long_mask: u32,
        long_nodes: &[A51LongFlag],
    ) -> bool {
        if query.tag != 0 {
            let hash = a51_hash_bucket(short_mask, a51_fnv1a(&[query.key]));
            short_nodes
                .iter()
                .any(|node| node.bucket == hash && node.key == query.key)
        } else {
            let hash = a51_hash_bucket(long_mask, a51_fnv1a(query.text.text.as_bytes()));
            long_nodes
                .iter()
                .any(|node| node.bucket == hash && node.text.text == query.text.text)
        }
    }

    /// `0x005f8eb0` (`FUN_005f8eb0`, 72 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// if (*(int **)(this + 0x54) != 0) { (*(*ptr + 8))(); return; }
    /// FUN_00c84194(cookie); int3;
    /// ```
    ///
    /// The virtual slot is the caller's `dispose`; a null field takes the
    /// non-returning invalid-parameter path.
    pub fn a51_dispose_member_54<F>(&self, present: bool, mut dispose: F) -> A51Teardown
    where
        F: FnMut(),
    {
        if present {
            dispose();
            A51Teardown::Dispatched
        } else {
            A51Teardown::InvalidParameter
        }
    }

    /// `0x005f8f00` (`FUN_005f8f00`, 22 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// FUN_005f12e0(this + 0x24);   /* std::string copy ctor, destination in EAX */
    /// return param_1;
    /// ```
    ///
    /// The destination is the returned object, so the translation is a copy
    /// that preserves the source capacity (`FUN_005f12e0` duplicates the header
    /// and the bytes).
    pub fn a51_copy_member_string(&self, member: &A51String) -> A51String {
        member.clone()
    }

    /// `0x005f93a0` (`FUN_005f93a0`, 72 bytes, `decompiled_uncited`).
    ///
    /// Destroys the 0x18-stride vector whose triple starts at `this + 0xac`:
    ///
    /// ```c
    /// begin = *param_1;
    /// sign  = (end - begin) >> 0x1f;
    /// if ((end - begin) / 0x18 + sign == sign) { FUN_005fd630(); int3; }
    /// if ((int *)(this + 0xac) != begin) {
    ///   size = *(int *)(begin + 0x10);
    ///   ptr  = (0xf < *(uint *)(begin + 0x14)) ? *(int *)begin : begin;
    ///   FUN_005fdb20(ptr, size);
    /// }
    /// ```
    ///
    /// The signed guard traps when the truncating element count is zero;
    /// `element` describes the string header at `begin`.
    pub fn a51_destroy_vector_18(
        &self,
        begin: u32,
        end: u32,
        member: u32,
        element: A51StringSlot,
    ) -> A51Dtor18 {
        let diff = end.wrapping_sub(begin) as i32;
        let sign = diff >> 31;
        let stride = A51_ELEMENT_18 as i32;
        if diff / stride + sign == sign {
            return A51Dtor18::ZeroCountTrap;
        }
        if begin == member {
            return A51Dtor18::Aliased;
        }
        let pointer = if element.is_heap() {
            element.data
        } else {
            begin
        };
        A51Dtor18::Released {
            pointer,
            size: element.size,
        }
    }

    /// `0x005f9410` (`FUN_005f9410`, 87 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// *(byte *)(this + 8) = 0;
    /// if ((int *)(this + 0xac) != (int *)(this + 0xc4)) {
    ///   ptr = (0xf < *(uint *)(this + 0xd8)) ? *(int *)(this + 0xc4) : this + 0xc4;
    ///   FUN_005fdb20(ptr, *(uint *)(this + 0xd4));
    /// }
    /// ```
    ///
    /// Returns the released `(pointer, size)` pair, or `None` when the `+0xac`
    /// triple already pointed at the `+0xc4` string.
    pub fn a51_reset_and_release_c4(
        &self,
        active: &mut u8,
        aliased: bool,
        member: u32,
        slot: A51StringSlot,
    ) -> Option<(u32, u32)> {
        *active = 0;
        if aliased {
            return None;
        }
        let pointer = if slot.is_heap() { slot.data } else { member };
        Some((pointer, slot.size))
    }

    /// `0x005f9470` (`FUN_005f9470`, 177 bytes, `decompiled_uncited`).
    ///
    /// Full group reset:
    ///
    /// ```c
    /// *(byte *)(this + 8) = 0;
    /// count = (end - begin + 3) >> 2;      /* 0 when end < begin */
    /// for (i = 0; i != count; ++i) (*p)->vtable_0x34();
    /// *(u32 *)(this + 0xfc) = 0;
    /// FUN_005ed2b0(*(u32 *)(this + 0x10c), *(u32 *)(this + 0x110), this + 0x10c);
    /// *(u32 *)(this + 0x110) = *(u32 *)(this + 0x10c);
    /// for (p = this + 0x100; p != *(this + 0x104); p += 0x34) FUN_005f34c0();
    /// *(u32 *)(this + 0x104) = *(u32 *)(this + 0x100);
    /// *(u16 *)(this + 0x118) = 0;
    /// *(byte *)(this + 0x11a) = 0;
    /// *(byte *)(this + 8) = 1;
    /// *(byte *)(this + 0x188) = 0;
    /// ```
    pub fn a51_reset_group<F>(
        &self,
        state: &mut A51GroupState,
        mut probe_child: F,
    ) -> A51ResetReport
    where
        F: FnMut(usize),
    {
        state.active = 0;
        let children = a51_child_count(state.child_begin, state.child_end);
        for index in 0..children as usize {
            probe_child(index);
        }
        state.provider = 0;
        let released = (state.vec_begin, state.vec_end);
        state.vec_end = state.vec_begin;
        let list_elements = if state.list_begin == state.list_end {
            0
        } else {
            state.list_end.wrapping_sub(state.list_begin) / A51_ELEMENT_34
        };
        state.list_end = state.list_begin;
        state.flags = 0;
        state.extra = 0;
        state.active = 1;
        state.tail = 0;
        A51ResetReport {
            children,
            released,
            list_elements,
        }
    }

    /// `0x005f9530` (`FUN_005f9530`, 55 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// *(byte *)(this + 8) = 0;
    /// count = (uint)((int)end + (3 - (int)begin)) >> 2;
    /// if (end < begin) count = 0;
    /// for (i = 0; i != count; ++i) (*p)->vtable_0x34();
    /// ```
    ///
    /// Same count formula as `0x005f9470` (`(end - begin + 3) >> 2`), but
    /// only the child probes run.  Returns the number of `vtable + 0x34`
    /// calls.
    pub fn a51_reset_children<F>(&self, state: &mut A51GroupState, mut probe_child: F) -> u32
    where
        F: FnMut(usize),
    {
        state.active = 0;
        let children = a51_child_count(state.child_begin, state.child_end);
        for index in 0..children as usize {
            probe_child(index);
        }
        children
    }

    /// `0x005f9570` (`FUN_005f9570`, 415 bytes, `decompiled_uncited`).
    ///
    /// Stores the four render flags and prepares the separator string:
    ///
    /// ```c
    /// *(byte *)(this + 0x180) = param_1;
    /// *(byte *)(this + 0x182) = param_3;
    /// *(byte *)(this + 0x181) = param_2;
    /// *(byte *)(this + 0x183) = param_4;
    /// if (param_2 == 0) FUN_005fdb20(&DAT_00d38618, 1);      /* non-returning */
    /// copy the +0x150 string into +0x1f8 (+0x208 size, +0x20c cap), then clear it;
    /// separator = (*(byte *)(this + 0x180) == 0) ? " " : "";
    /// ```
    ///
    /// `None` reports the non-returning `param_2 == 0` path; otherwise the
    /// member is moved into `copied` and left empty.
    pub fn a51_setup_render(
        &self,
        p1: u8,
        p2: u8,
        p3: u8,
        p4: u8,
        member: &mut A51String,
    ) -> Option<A51RenderSetup> {
        if p2 == 0 {
            return None;
        }
        let flags = [p1, p2, p3, p4];
        let copied = member.clone();
        member.text.clear();
        member.cap = A51_SSO_CAPACITY;
        let separator = if p1 == 0 { " " } else { "" };
        Some(A51RenderSetup {
            flags,
            separator,
            copied,
        })
    }

    /// `0x005f9740` (`FUN_005f9740`, 340 bytes, `decompiled_uncited`).
    ///
    /// Installs a temporary subparser at the head of the `+0x11c` chain:
    ///
    /// ```c
    /// if (*(int *)(this + 0xf4) == 0) return;      /* whole body skipped */
    /// saved = *(u32 *)(this + 0x11c);
    /// *(u32 *)(this + 0x11c) = new_node;
    /// (**(code **)(**(int **)(this + 0xf4) + 8))(new_node);
    /// *(u32 *)(this + 0x11c) = saved;
    /// ```
    ///
    /// The `std::function` local (`local_e0[0] = Subparser::vftable`) and the
    /// two `FUN_005fd300` destructor calls carry no data, so they are not
    /// represented.  The second `+0xf4 == 0` test in the body is unreachable
    /// (the same field was already tested non-zero).
    pub fn a51_install_subparser<F>(
        &self,
        target_present: bool,
        head: &mut Option<u32>,
        node: u32,
        mut dispatch: F,
    ) -> A51Install
    where
        F: FnMut(u32),
    {
        if !target_present {
            return A51Install::Skipped;
        }
        let saved = *head;
        *head = Some(node);
        dispatch(node);
        *head = saved;
        A51Install::Dispatched { saved }
    }

    /// `0x005f98f0` (`FUN_005f98f0`, 392 bytes, `decompiled_uncited`).
    ///
    /// Validates and visits the group:
    ///
    /// ```c
    /// if (!(*vtable + 4)()) return;
    /// for (p = children; p != end; ++p) {
    ///   if ((*p)->vtable_0x14() && !(*p)->vtable_4()) {
    ///     "Group validation failed somewhere!" -> throw
    ///   }
    ///   (*p)->vtable_8(arg_1, arg_2);
    /// }
    /// if (*(int *)(this + 0x11c) != 0) (*(*sub + 8))(arg_1, arg_2);
    /// if (*(int *)(this + 0xfc) == 0 && *(char *)(this + 0xf8) != 0) {
    ///   for (p = children; p != end; ++p)
    ///     if ((*p)->vtable_0x2c()) goto required;
    ///   if (*(char *)(this + 0x11a) == 0) return;
    /// required:
    ///   "Command is required" -> throw
    /// }
    /// ```
    ///
    /// The per-child virtual results are [`A51GroupChild`] values; `visit`
    /// receives the child index and `dispatch` runs the subparser slot.
    pub fn a51_validate_and_visit<F, G>(
        &self,
        enabled: bool,
        children: &[A51GroupChild],
        subparser_present: bool,
        own_present: bool,
        command_required: bool,
        extra_required: bool,
        mut visit: F,
        mut dispatch: G,
    ) -> Result<A51VisitReport, &'static str>
    where
        F: FnMut(usize),
        G: FnMut(),
    {
        if !enabled {
            return Ok(A51VisitReport {
                visited: 0,
                subparser_dispatched: false,
            });
        }
        let mut visited = 0;
        for (index, child) in children.iter().enumerate() {
            if child.validates && !child.ok {
                return Err(A51_GROUP_VALIDATION_FAILED);
            }
            visit(index);
            visited += 1;
        }
        let subparser_dispatched = if subparser_present {
            dispatch();
            true
        } else {
            false
        };
        if !own_present && command_required {
            if children.iter().any(|child| child.requires_command) || extra_required {
                return Err(A51_COMMAND_REQUIRED);
            }
        }
        Ok(A51VisitReport {
            visited,
            subparser_dispatched,
        })
    }

    /// `0x005f9a80` (`FUN_005f9a80`, 266 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// if ((*vtable + 4)() == 0 && ((uint)in_ECX[1] >> 1 & 1) != 0) {
    ///   "Flag '" + unparse(arg) + "' is required" -> throw
    /// }
    /// ```
    ///
    /// Both literals are reproduced verbatim; the unparse call is the
    /// `label` input.
    pub fn a51_required_message(&self, valid: bool, flags: u32, label: &str) -> Option<String> {
        if valid || (flags >> 1) & 1 == 0 {
            return None;
        }
        Some(format!("Flag '{label}' is required"))
    }

    /// `0x005f9ba0` (`FUN_005f9ba0`, 63 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// count = (uint)((int)end + (3 - (int)begin)) >> 2;
    /// if (end < begin) count = 0;
    /// for (i = 0; i != count; ++i) (*p)->vtable_8(param_1, param_2);
    /// ```
    ///
    /// Returns the number of visits.
    pub fn a51_visit_children<F>(&self, children: &[A51Child], arg: u32, mut visit: F) -> u32
    where
        F: FnMut(usize, u32),
    {
        let mut visited = 0;
        for (index, _) in children.iter().enumerate() {
            visit(index, arg);
            visited += 1;
        }
        visited
    }

    /// `0x005f9c90` (`FUN_005f9c90`, 148 bytes, `decompiled_uncited`).
    ///
    /// Reserve/reallocate of the 0x1c requirement vector:
    ///
    /// ```c
    /// if (*this == 0) {
    ///   *this = param_1;
    ///   this[1] = param_1 + param_2 * 0x1c;
    ///   this[2] = param_1 + param_3 * 0x1c;
    ///   return;
    /// }
    /// FUN_005ed240(*this, this[1]);
    /// byte_len = ((this[2] - *this) / 0x1c) * 0x1c;
    /// if (0xfff < byte_len) { stored = *(*this - 4); byte_len += 0x23;
    ///                         if (0x1f < (*this - stored) - 4) trap; }
    /// FUN_00923e90(stored_or_begin, byte_len);
    /// ```
    ///
    /// `header` is the dword at `begin - 4` (the size-class header the
    /// allocator stores before a large block).
    pub fn a51_reserve_1c(
        &self,
        triple: &mut [u32; 3],
        begin: u32,
        count: u32,
        capacity: u32,
        header: u32,
    ) -> A51Reserve {
        if triple[0] == 0 {
            *triple = [
                begin,
                begin.wrapping_add(count.wrapping_mul(A51_ELEMENT_1C)),
                begin.wrapping_add(capacity.wrapping_mul(A51_ELEMENT_1C)),
            ];
            return A51Reserve::Initialized {
                begin: triple[0],
                end: triple[1],
                cap: triple[2],
            };
        }
        let bytes = a51_capacity_bytes(triple[0], triple[2], A51_ELEMENT_1C);
        match a51_large_block(triple[0], header, bytes) {
            A51LargeBlock::Small { bytes } => A51Reserve::Released {
                data: triple[0],
                size: bytes,
            },
            A51LargeBlock::Large { stored, bytes } => A51Reserve::Released {
                data: stored,
                size: bytes,
            },
            A51LargeBlock::Misaligned { begin, stored } => A51Reserve::Misaligned { begin, stored },
        }
    }

    /// `0x005f9d30` (`FUN_005f9d30`, 75 bytes, `decompiled_uncited`).
    ///
    /// The 0x18-stride twin of `0x005f9c90`; the old range is destroyed with
    /// `FUN_005ed2b0` instead of `FUN_005ed240` and the divisor is `0x18`.
    pub fn a51_reserve_18(
        &self,
        triple: &mut [u32; 3],
        begin: u32,
        count: u32,
        capacity: u32,
        header: u32,
    ) -> A51Reserve {
        if triple[0] == 0 {
            *triple = [
                begin,
                begin.wrapping_add(count.wrapping_mul(A51_ELEMENT_18)),
                begin.wrapping_add(capacity.wrapping_mul(A51_ELEMENT_18)),
            ];
            return A51Reserve::Initialized {
                begin: triple[0],
                end: triple[1],
                cap: triple[2],
            };
        }
        let bytes = a51_capacity_bytes(triple[0], triple[2], A51_ELEMENT_18);
        match a51_large_block(triple[0], header, bytes) {
            A51LargeBlock::Small { bytes } => A51Reserve::Released {
                data: triple[0],
                size: bytes,
            },
            A51LargeBlock::Large { stored, bytes } => A51Reserve::Released {
                data: stored,
                size: bytes,
            },
            A51LargeBlock::Misaligned { begin, stored } => A51Reserve::Misaligned { begin, stored },
        }
    }

    /// `0x005f9dc0` (`FUN_005f9dc0`, 129 bytes, `decompiled_uncited`).
    ///
    /// The 0x34-stride reserve path, which first destroys every record of the
    /// old range:
    ///
    /// ```c
    /// if (*this == 0) { install param_1 / +param_2*0x34 / +param_3*0x34; return; }
    /// if (*this != this[1]) do { FUN_005f34c0(); p += 0x34; } while (p != this[1]);
    /// byte_len = ((this[2] - *this) / 0x34) * 0x34;
    /// ... same 0xfff / +0x23 / 0x1f guard ...
    /// ```
    pub fn a51_reserve_34(
        &self,
        triple: &mut [u32; 3],
        begin: u32,
        count: u32,
        capacity: u32,
        header: u32,
    ) -> A51Reserve34 {
        if triple[0] == 0 {
            *triple = [
                begin,
                begin.wrapping_add(count.wrapping_mul(A51_ELEMENT_34)),
                begin.wrapping_add(capacity.wrapping_mul(A51_ELEMENT_34)),
            ];
            return A51Reserve34::Initialized {
                begin: triple[0],
                end: triple[1],
                cap: triple[2],
            };
        }
        let destroyed = if triple[0] == triple[1] {
            0
        } else {
            triple[1].wrapping_sub(triple[0]) / A51_ELEMENT_34
        };
        let bytes = a51_capacity_bytes(triple[0], triple[2], A51_ELEMENT_34);
        match a51_large_block(triple[0], header, bytes) {
            A51LargeBlock::Small { bytes } => A51Reserve34::Released {
                data: triple[0],
                size: bytes,
                destroyed,
            },
            A51LargeBlock::Large { stored, bytes } => A51Reserve34::Released {
                data: stored,
                size: bytes,
                destroyed,
            },
            A51LargeBlock::Misaligned { begin, stored } => {
                A51Reserve34::Misaligned { begin, stored }
            }
        }
    }

    /// `0x005f9e50` (`FUN_005f9e50`, 149 bytes, `decompiled_uncited`).
    ///
    /// The bucket-grow trigger of the hash container:
    ///
    /// ```c
    /// ratio = ((double)(int)load_count + table[sign])
    ///       / ((double)(int)capacity  + table[sign]);
    /// if (load <= ratio && ratio != load) {
    ///   if ((uint)capacity < 0x200)        target = (int)capacity << 3;
    ///   else if ((uint)capacity < 0x1fffffff) target = (int)capacity * 2;
    ///   else                               target = capacity;
    ///   FUN_005fc710(target);              /* resize, then rehash each node */
    /// }
    /// ```
    ///
    /// `FUN_005f9ef0` repeats the same arithmetic with another rehash helper;
    /// it is recorded as a skipped duplicate in the module comment.
    pub fn a51_grow_buckets(&self, load: f32, count: i32, capacity: i32) -> A51Grow {
        let ratio = a51_grow_ratio(count, capacity);
        let grow = load <= ratio && ratio != load;
        A51Grow {
            ratio,
            target: a51_grow_target(capacity as u32),
            grow,
        }
    }
}

#[cfg(test)]
mod tests_agent51 {
    use super::*;
    use crate::data::GameData;
    use crate::map::Terrain;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn s(text: &str) -> A51String {
        A51String {
            text: text.to_string(),
            cap: A51_SSO_CAPACITY,
        }
    }

    fn child(flags: u8, code: i32) -> A51Child {
        A51Child {
            flags,
            text: a51_string_empty(),
            code,
        }
    }

    #[test]
    fn collect_probe_pairs_skips_bit_four() {
        // 0x005f8000
        let w = world();
        let kids = [
            child(0, 0),
            child(A51_FLAG_SKIP, 1),
            child(0x10, 2),
            child(A51_FLAG_SKIP | 0x10, 3),
        ];
        let pairs = w.a51_collect_probe_pairs(&kids, 7, |index, arg| [index as u32, arg]);
        assert_eq!(pairs, vec![[0, 7], [2, 7]]);
        assert!(w.a51_collect_probe_pairs(&[], 1, |_, _| [0, 0]).is_empty());
    }

    #[test]
    fn requirement_selects_named_char_and_default_forms() {
        // 0x005f80c0
        let w = world();
        let mut named = A51RequirementSource {
            selector: 0,
            has_children: true,
            child_text: Some(s("Minerals")),
            own_value: None,
        };
        assert_eq!(
            w.a51_requirement_from_source(&named),
            A51Requirement {
                kind: 0,
                value: 0,
                text: s("Minerals"),
            }
        );

        named.has_children = false;
        assert_eq!(
            w.a51_requirement_from_source(&named),
            A51Requirement {
                kind: 1,
                value: 0x20,
                text: s(""),
            },
            "0x2001 is the little-endian pair kind 1 / value 0x20"
        );

        let char_form = A51RequirementSource {
            selector: 3,
            has_children: true,
            child_text: Some(s("ignored")),
            own_value: Some(b'Z'),
        };
        let record = w.a51_requirement_from_source(&char_form);
        assert_eq!(record.kind, 1);
        assert_eq!(record.value, b'Z');
        assert!(record.text.text.is_empty());
        assert_eq!(record.text.cap, A51_SSO_CAPACITY);
    }

    #[test]
    fn one_element_triple_compares_begin_plus_0x1c_with_count_times_stride() {
        // 0x005f8130
        let w = world();
        let check = w.a51_init_one_element_triple(0x1000, 1);
        assert_eq!(check.triple, [0, 0, 0]);
        assert_eq!(check.one_past_first, 0x101c);
        assert_eq!(check.end, 0x101c);
        assert!(check.matched);

        let check = w.a51_init_one_element_triple(0x1000, 2);
        assert_eq!(check.end, 0x1038);
        assert!(!check.matched, "the debug report fires for count != 1");
    }

    #[test]
    fn unwind_tail_reports_the_recovered_site() {
        // 0x005f853a
        let w = world();
        assert_eq!(
            w.a51_unwind_tail(0x7fff_ffff, 1, 2, 3),
            A51Tail::InvalidParameter,
            "a clear high bit traps first"
        );
        assert_eq!(
            w.a51_unwind_tail(0x8000_0000, 0x40, 0x40, 0x44),
            A51Tail::Rethrow
        );
        assert_eq!(
            w.a51_unwind_tail(0x8000_0000, 0x40, 0x60, 0x44),
            A51Tail::Report {
                first: 0x40,
                second: 0x44,
                source: 0xd3_8aa4,
                line: 3,
            }
        );
    }

    #[test]
    fn child_predicates_gate_on_the_recovered_bytes() {
        // 0x005f8990, 0x005f89c0, 0x005f8a00, 0x005f8a30, 0x005f8a70
        let w = world();
        let kids = [child(0, 1), child(0, 2)];
        assert!(w.a51_any_child_flag_2c(&kids, |c| c.code == 2));
        assert!(!w.a51_any_child_flag_2c(&kids, |c| c.code == 9));

        assert!(w.a51_any_child_flag_24_unless_118(true, &kids, |_| false));
        assert!(!w.a51_any_child_flag_24_unless_118(false, &kids, |_| false));
        assert!(w.a51_any_child_flag_24(&kids, |c| c.code == 1));

        assert!(w.a51_any_child_flag_28_unless_119(true, &kids, |_| false));
        assert!(w.a51_any_child_flag_28(&kids, |c| c.code == 2));
        assert!(!w.a51_any_child_flag_28(&[], |_| true));
    }

    #[test]
    fn take_argument_triple_moves_member_only_when_present() {
        // 0x005f8aa0
        let w = world();
        let member = [1, 2, 3];
        let (dest, source) = w.a51_take_argument_triple(true, member, || [9, 9, 9]);
        assert_eq!(dest, member);
        assert_eq!(source, [0, 0, 0], "the member triple is zeroed");

        let (dest, source) = w.a51_take_argument_triple(false, member, || [4, 5, 6]);
        assert_eq!(dest, [4, 5, 6]);
        assert_eq!(
            source, member,
            "the probed temporary leaves the member alone"
        );
    }

    #[test]
    fn validate_node_walks_own_then_linked_children() {
        // 0x005f8b70
        let w = world();
        let kids = [
            child(0, 1),
            child(A51_FLAG_LINKED, 2),
            child(A51_FLAG_LINKED, 3),
        ];
        let log = std::cell::RefCell::new(Vec::new());
        let code = w.a51_validate_node(
            Some(0xaa),
            None,
            true,
            &kids,
            5,
            |node, arg| {
                log.borrow_mut().push((node, arg));
                0
            },
            |c, arg| {
                log.borrow_mut().push((c.code as u32, arg));
                0
            },
        );
        assert_eq!(code, 0);
        assert_eq!(log.into_inner(), vec![(0xaa, 5), (2, 5), (3, 5)]);

        let code = w.a51_validate_node(Some(1), None, true, &kids, 0, |_, _| 0, |c, _| c.code);
        assert_eq!(code, 2, "the first linked child wins");

        let code = w.a51_validate_node(
            None,
            Some(0xbb),
            false,
            &kids,
            9,
            |node, _| node as i32,
            |_, _| 0,
        );
        assert_eq!(
            code, 0xbb,
            "the subparser short-circuits before the children"
        );
    }

    #[test]
    fn validate_node_self_predicate_gates_the_child_scan() {
        // 0x005f8b70
        let w = world();
        let kids = [child(0, 0), child(0, 4)];
        let mut visited = Vec::new();
        let code = w.a51_validate_node(
            None,
            None,
            false,
            &kids,
            0,
            |_, _| 0,
            |c, _| {
                visited.push(c.code);
                0
            },
        );
        assert_eq!(code, 0);
        assert!(visited.is_empty(), "vtable+4 zero skips the whole scan");

        let code = w.a51_validate_node(None, None, true, &kids, 0, |_, _| 0, |c, _| c.code + 1);
        assert_eq!(code, 1, "the first child status is returned");
    }

    #[test]
    fn mark_flag_seen_reports_the_exact_duplicate_message() {
        // 0x005f8c10
        let w = world();
        let mut seen = false;
        assert_eq!(w.a51_mark_flag_seen(false, 1, &mut seen, "--x"), None);
        assert!(!seen, "an unknown flag is not marked");

        assert_eq!(w.a51_mark_flag_seen(true, 1, &mut seen, "--x"), None);
        assert!(seen);

        let message = w.a51_mark_flag_seen(true, 1, &mut seen, "--x").unwrap();
        assert_eq!(
            message,
            "Flag '--x' was passed multiple times, but is only allowed to be passed once"
        );

        seen = false;
        assert_eq!(w.a51_mark_flag_seen(true, 0, &mut seen, "--y"), None);
        assert!(seen, "without bit 0 the duplicate check never fires");
    }

    #[test]
    fn lookup_flag_uses_fnv1a_and_the_recovered_masks() {
        // 0x005f8d40
        let w = world();
        let short_hash = 0xe40c_292c; // FNV-1a of b"a"
        assert_eq!(a51_fnv1a(b"a"), short_hash);
        assert_eq!(a51_fnv1a(b""), A51_FNV_BASIS);
        assert_eq!(a51_fnv1a(b"foobar"), 0xbf9c_f968);

        let short_key = A51FlagQuery {
            tag: 1,
            key: b'a',
            text: a51_string_empty(),
        };
        let nodes = [
            A51ShortFlag {
                bucket: a51_hash_bucket(0xffff_ffff, short_hash),
                key: b'a',
            },
            A51ShortFlag {
                bucket: 1,
                key: b'b',
            },
        ];
        assert!(w.a51_lookup_flag(&short_key, 0xffff_ffff, &nodes, 0, &[]));
        assert!(!w.a51_lookup_flag(
            &A51FlagQuery {
                tag: 1,
                key: b'q',
                text: a51_string_empty(),
            },
            0xffff_ffff,
            &nodes,
            0,
            &[]
        ));

        let long_key = A51FlagQuery {
            tag: 0,
            key: 0,
            text: s("foobar"),
        };
        let long_nodes = [
            A51LongFlag {
                bucket: a51_hash_bucket(0x1ff, 0xbf9c_f968),
                text: s("foobar"),
            },
            A51LongFlag {
                bucket: 0,
                text: s("other"),
            },
        ];
        assert!(w.a51_lookup_flag(&long_key, 0, &[], 0x1ff, &long_nodes));
        let miss = A51FlagQuery {
            tag: 0,
            key: 0,
            text: s("nothing"),
        };
        assert!(!w.a51_lookup_flag(&miss, 0, &[], 0x1ff, &long_nodes));
    }

    #[test]
    fn hash_bucket_masks_the_low_bits() {
        // 0x005f8d40
        assert_eq!(a51_hash_bucket(0xf, 0x1234_5678), 0x8);
        assert_eq!(a51_hash_bucket(0xffff_ffff, 0x1234_5678), 0x1234_5678);
    }

    #[test]
    fn dispose_member_and_copy_string_follow_the_retail_branches() {
        // 0x005f8eb0, 0x005f8f00
        let w = world();
        let mut disposed = false;
        assert_eq!(
            w.a51_dispose_member_54(false, || {}),
            A51Teardown::InvalidParameter
        );
        assert_eq!(
            w.a51_dispose_member_54(true, || disposed = true),
            A51Teardown::Dispatched
        );
        assert!(disposed);

        let member = A51String {
            text: "0123456789abcdef".to_string(),
            cap: 0x1f,
        };
        assert_eq!(w.a51_copy_member_string(&member), member);
        assert_eq!(member.cap, 0x1f, "the capacity is duplicated");
    }

    #[test]
    fn boot_verbosity_overrides_with_five() {
        // 0x005f91fe
        assert_eq!(a51_boot_verbosity(2, false), 2);
        assert_eq!(a51_boot_verbosity(2, true), A51_VERBOSITY_OVERRIDE);
        assert_eq!(a51_boot_verbosity(0, true), 5);
    }

    #[test]
    fn destroy_vector_18_checks_count_then_alias_then_storage() {
        // 0x005f93a0
        let w = world();
        let element = A51StringSlot {
            data: 0x7000,
            size: 5,
            cap: A51_SSO_CAPACITY,
        };
        assert_eq!(
            w.a51_destroy_vector_18(0x1000, 0x1000, 0x2000, element),
            A51Dtor18::ZeroCountTrap,
            "the truncating element count is zero"
        );
        assert_eq!(
            w.a51_destroy_vector_18(0x1000, 0x1030, 0x1000, element),
            A51Dtor18::Aliased
        );
        assert_eq!(
            w.a51_destroy_vector_18(0x1000, 0x1030, 0x2000, element),
            A51Dtor18::Released {
                pointer: 0x1000,
                size: 5,
            },
            "an inline element releases the vector start"
        );
        let heap_element = A51StringSlot {
            data: 0x9000,
            size: 0x21,
            cap: 0x3f,
        };
        assert_eq!(
            w.a51_destroy_vector_18(0x1000, 0x1018, 0x2000, heap_element),
            A51Dtor18::Released {
                pointer: 0x9000,
                size: 0x21,
            }
        );
    }

    #[test]
    fn reset_and_release_c4_tracks_the_alias() {
        // 0x005f9410
        let w = world();
        let mut active = 1;
        let slot = A51StringSlot {
            data: 0x5000,
            size: 9,
            cap: 0x10,
        };
        assert_eq!(
            w.a51_reset_and_release_c4(&mut active, true, 0x40, slot),
            None
        );
        assert_eq!(active, 0);
        assert_eq!(
            w.a51_reset_and_release_c4(&mut active, false, 0x40, slot),
            Some((0x5000, 9))
        );
        let inline = A51StringSlot {
            data: 0,
            size: 3,
            cap: A51_SSO_CAPACITY,
        };
        assert_eq!(
            w.a51_reset_and_release_c4(&mut active, false, 0x40, inline),
            Some((0x40, 3))
        );
    }

    #[test]
    fn reset_group_clears_every_recovered_field() {
        // 0x005f9470
        let w = world();
        let mut state = A51GroupState {
            active: 0,
            child_begin: 0x1000,
            child_end: 0x1008,
            provider: 0x999,
            list_begin: 0x4000,
            list_end: 0x4068,
            vec_begin: 0x3000,
            vec_end: 0x3020,
            flags: 0x0101,
            extra: 1,
            tail: 1,
        };
        let mut probed = Vec::new();
        let report = w.a51_reset_group(&mut state, |index| probed.push(index));
        assert_eq!(probed, vec![0, 1], "one vtable+0x34 probe per child");
        assert_eq!(report.children, 2);
        assert_eq!(report.released, (0x3000, 0x3020));
        assert_eq!(report.list_elements, 2);
        assert_eq!(state.provider, 0);
        assert_eq!(state.vec_end, 0x3000, "+0x110 collapses onto +0x10c");
        assert_eq!(state.list_end, 0x4000);
        assert_eq!(state.flags, 0);
        assert_eq!(state.extra, 0);
        assert_eq!(state.active, 1);
        assert_eq!(state.tail, 0);
    }

    #[test]
    fn reset_group_ignores_a_reversed_child_range() {
        // 0x005f9470
        let w = world();
        let mut state = A51GroupState {
            active: 0,
            child_begin: 0x2000,
            child_end: 0x1000,
            provider: 1,
            list_begin: 0x10,
            list_end: 0x10,
            vec_begin: 0x20,
            vec_end: 0x20,
            flags: 0xffff,
            extra: 1,
            tail: 1,
        };
        let report = w.a51_reset_group(&mut state, |_| {});
        assert_eq!(report.children, 0, "end < begin forces the count to zero");
        assert_eq!(report.list_elements, 0);
        assert_eq!(report.released, (0x20, 0x20));
    }

    #[test]
    fn reset_children_uses_the_rounded_count() {
        // 0x005f9530
        let w = world();
        let mut state = A51GroupState {
            active: 1,
            child_begin: 0x1000,
            child_end: 0x1004,
            provider: 0,
            list_begin: 0,
            list_end: 0,
            vec_begin: 0,
            vec_end: 0,
            flags: 0,
            extra: 0,
            tail: 0,
        };
        let mut probed = Vec::new();
        assert_eq!(
            w.a51_reset_children(&mut state, |index| probed.push(index)),
            1
        );
        assert_eq!(probed, vec![0]);
        assert_eq!(state.active, 0);
        assert_eq!(a51_child_count(0x1000, 0x1000), 0);
        assert_eq!(a51_child_count(0x1000, 0x1002), 1, "(2 + 3) >> 2");
        assert_eq!(a51_child_count(0x1000, 0x1006), 2);
    }

    #[test]
    fn setup_render_stores_flags_and_moves_the_member() {
        // 0x005f9570
        let w = world();
        let mut member = s("Minerals");
        let setup = w.a51_setup_render(0, 1, 2, 3, &mut member).unwrap();
        assert_eq!(setup.flags, [0, 1, 2, 3]);
        assert_eq!(setup.separator, " ", "flag +0x180 == 0 selects the space");
        assert_eq!(setup.copied, s("Minerals"));
        assert!(member.text.is_empty());
        assert_eq!(member.cap, A51_SSO_CAPACITY);

        let mut member = s("x");
        let setup = w.a51_setup_render(1, 1, 0, 0, &mut member).unwrap();
        assert_eq!(setup.separator, "");
        assert_eq!(setup.flags, [1, 1, 0, 0]);
    }

    #[test]
    fn setup_render_returns_none_for_the_non_returning_path() {
        // 0x005f9570
        let w = world();
        let mut member = s("x");
        assert_eq!(w.a51_setup_render(0, 0, 0, 0, &mut member), None);
        assert_eq!(member, s("x"), "the member is untouched");
    }

    #[test]
    fn install_subparser_pushes_dispatches_and_pops() {
        // 0x005f9740
        let w = world();
        let mut head = Some(0x10);
        let mut dispatched = Vec::new();
        assert_eq!(
            w.a51_install_subparser(false, &mut head, 0x20, |n| dispatched.push(n)),
            A51Install::Skipped
        );
        assert_eq!(head, Some(0x10));
        assert!(dispatched.is_empty());

        assert_eq!(
            w.a51_install_subparser(true, &mut head, 0x20, |n| dispatched.push(n)),
            A51Install::Dispatched { saved: Some(0x10) }
        );
        assert_eq!(dispatched, vec![0x20]);
        assert_eq!(head, Some(0x10), "the saved head is restored");

        let mut empty = None;
        let result = w.a51_install_subparser(true, &mut empty, 0x30, |_| {});
        assert_eq!(result, A51Install::Dispatched { saved: None });
        assert_eq!(empty, None);
    }

    #[test]
    fn validate_and_visit_reports_the_two_recovered_messages() {
        // 0x005f98f0
        let w = world();
        let kids = [
            A51GroupChild {
                validates: true,
                ok: false,
                requires_command: false,
            },
            A51GroupChild {
                validates: false,
                ok: true,
                requires_command: false,
            },
        ];
        let mut visits = Vec::new();
        let error = w
            .a51_validate_and_visit(
                true,
                &kids,
                false,
                true,
                false,
                false,
                |i| visits.push(i),
                || {},
            )
            .unwrap_err();
        assert_eq!(error, "Group validation failed somewhere!");
        assert!(visits.is_empty(), "the failing child is not visited");

        let good = [
            A51GroupChild {
                validates: true,
                ok: true,
                requires_command: true,
            },
            A51GroupChild {
                validates: false,
                ok: false,
                requires_command: false,
            },
        ];
        let mut dispatched = 0;
        let report = w
            .a51_validate_and_visit(
                true,
                &good,
                true,
                false,
                false,
                false,
                |i| visits.push(i),
                || dispatched += 1,
            )
            .unwrap();
        assert_eq!(report.visited, 2);
        assert!(report.subparser_dispatched);
        assert_eq!(dispatched, 1);
        assert_eq!(visits, vec![0, 1]);

        let error = w
            .a51_validate_and_visit(true, &good, false, false, true, false, |_| {}, || {})
            .unwrap_err();
        assert_eq!(error, "Command is required");

        let error = w
            .a51_validate_and_visit(true, &[], false, false, true, true, |_| {}, || {})
            .unwrap_err();
        assert_eq!(error, "Command is required", "the +0x11a gate also fires");

        let report = w
            .a51_validate_and_visit(false, &good, true, false, true, true, |_| {}, || {})
            .unwrap();
        assert_eq!(report.visited, 0);
        assert!(!report.subparser_dispatched);

        let error = w
            .a51_validate_and_visit(true, &good, false, false, true, false, |_| {}, || {})
            .unwrap_err();
        assert_eq!(
            error, "Command is required",
            "a child whose vtable+0x2c predicate is true jumps to the error label"
        );

        let quiet = [
            A51GroupChild {
                validates: false,
                ok: true,
                requires_command: false,
            },
            A51GroupChild {
                validates: false,
                ok: true,
                requires_command: false,
            },
        ];
        let report = w
            .a51_validate_and_visit(true, &quiet, false, false, true, false, |_| {}, || {})
            .unwrap();
        assert_eq!(
            report.visited, 2,
            "no child requires a command, so the check passes"
        );
    }

    #[test]
    fn required_message_uses_the_two_literals() {
        // 0x005f9a80
        let w = world();
        assert_eq!(
            w.a51_required_message(false, A51_FLAG_REQUIRED, "--profile"),
            Some("Flag '--profile' is required".to_string())
        );
        assert_eq!(w.a51_required_message(true, A51_FLAG_REQUIRED, "--x"), None);
        assert_eq!(w.a51_required_message(false, 0, "--x"), None);
        assert_eq!(
            w.a51_required_message(false, 0x4, "--x"),
            None,
            "bit 1 is the required bit"
        );
    }

    #[test]
    fn visit_children_walks_the_whole_list() {
        // 0x005f9ba0
        let w = world();
        let kids = [child(0, 0), child(0, 1), child(A51_FLAG_SKIP, 2)];
        let mut order = Vec::new();
        let count = w.a51_visit_children(&kids, 0x42, |index, arg| {
            order.push((index, arg));
        });
        assert_eq!(count, 3, "the visit loop has no flags filter");
        assert_eq!(order, vec![(0, 0x42), (1, 0x42), (2, 0x42)]);
    }

    #[test]
    fn capacity_bytes_truncates_to_whole_elements() {
        // 0x005f9c90 / 0x005f9d30 / 0x005f9dc0
        assert_eq!(a51_capacity_bytes(0x2000, 0x2038, A51_ELEMENT_1C), 0x38);
        assert_eq!(a51_capacity_bytes(0x2000, 0x2010, A51_ELEMENT_1C), 0);
        assert_eq!(a51_capacity_bytes(0x2000, 0x2060, A51_ELEMENT_18), 0x60);
        assert_eq!(a51_capacity_bytes(0x2000, 0x2068, A51_ELEMENT_34), 0x68);
        assert_eq!(
            a51_capacity_bytes(0x2000, 0x2000, A51_ELEMENT_34),
            0,
            "a zero span truncates to zero"
        );
    }

    #[test]
    fn reserve_1c_installs_or_reports_the_old_block() {
        // 0x005f9c90
        let w = world();
        let mut triple = [0, 0, 0];
        assert_eq!(
            w.a51_reserve_1c(&mut triple, 0x2000, 2, 4, 0),
            A51Reserve::Initialized {
                begin: 0x2000,
                end: 0x2038,
                cap: 0x2070,
            }
        );
        assert_eq!(triple, [0x2000, 0x2038, 0x2070]);

        assert_eq!(
            w.a51_reserve_1c(&mut triple, 0x9000, 1, 1, 0),
            A51Reserve::Released {
                data: 0x2000,
                size: 0x70,
            },
            "0x70 <= 0xfff keeps the inline block header"
        );

        let mut big = [0x2000, 0x2000, 0x5000];
        assert_eq!(
            w.a51_reserve_1c(&mut big, 0x9000, 1, 1, 0x1ffc),
            A51Reserve::Released {
                data: 0x1ffc,
                size: 0x300b,
            },
            "0x3000 rounds to 0x2fe8 then adds 0x23"
        );

        let mut bad = [0x2000, 0x2000, 0x5038];
        assert_eq!(
            w.a51_reserve_1c(&mut bad, 0x9000, 1, 1, 0x1f00),
            A51Reserve::Misaligned {
                begin: 0x2000,
                stored: 0x1f00,
            }
        );
    }

    #[test]
    fn reserve_18_uses_the_0x18_divisor() {
        // 0x005f9d30
        let w = world();
        let mut triple = [0, 0, 0];
        assert_eq!(
            w.a51_reserve_18(&mut triple, 0x100, 3, 3, 0),
            A51Reserve::Initialized {
                begin: 0x100,
                end: 0x148,
                cap: 0x148,
            }
        );
        assert_eq!(
            w.a51_reserve_18(&mut triple, 0, 1, 1, 0),
            A51Reserve::Released {
                data: 0x100,
                size: 0x48,
            }
        );
    }

    #[test]
    fn reserve_34_destroys_the_old_records() {
        // 0x005f9dc0
        let w = world();
        let mut triple = [0, 0, 0];
        assert_eq!(
            w.a51_reserve_34(&mut triple, 0x1000, 2, 2, 0),
            A51Reserve34::Initialized {
                begin: 0x1000,
                end: 0x1068,
                cap: 0x1068,
            }
        );
        assert_eq!(
            w.a51_reserve_34(&mut triple, 0, 1, 1, 0),
            A51Reserve34::Released {
                data: 0x1000,
                size: 0x68,
                destroyed: 2,
            }
        );
        let mut big = [0x1000, 0x1000, 0x5000];
        assert_eq!(
            w.a51_reserve_34(&mut big, 0, 1, 1, 0x1f00),
            A51Reserve34::Misaligned {
                begin: 0x1000,
                stored: 0x1f00,
            },
            "the alignment guard only runs on the large-block path"
        );
    }

    #[test]
    fn grow_buckets_applies_the_two_thresholds() {
        // 0x005f9e50
        let w = world();
        let grow = w.a51_grow_buckets(0.5, 3, 6);
        assert!((grow.ratio - 7.0 / 13.0).abs() < 1e-6);
        assert!(grow.grow);
        assert_eq!(grow.target, 48, "6 < 0x200 multiplies by eight");

        let no = w.a51_grow_buckets(0.9, 3, 6);
        assert!(!no.grow);
        assert_eq!(no.target, 48);

        let equal = w.a51_grow_buckets(7.0 / 13.0, 3, 6);
        assert!(!equal.grow, "ratio == load is not a grow");

        assert_eq!(a51_grow_target(0x1ff), 0xff8);
        assert_eq!(a51_grow_target(0x200), 0x400);
        assert_eq!(a51_grow_target(0x1fff_fffe), 0x3fff_fffc);
        assert_eq!(a51_grow_target(0x1fff_ffff), 0x1fff_ffff);
    }

    #[test]
    fn grow_ratio_uses_the_half_addends() {
        // 0x005f9e50
        assert!((a51_grow_ratio(0, 0) - 1.0).abs() < 1e-6);
        assert!((a51_grow_ratio(1, 2) - 0.6).abs() < 1e-6);
        assert!((a51_grow_ratio(-1, 1) - (-0.5 / 1.5)).abs() < 1e-6);
    }
}
