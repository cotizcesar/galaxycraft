#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent52.rs
// ===========================================================================
// Translation batch 52.  Only this file is edited by the batch.
//
// Batch 52 owns `0x005fa000..=0x005fbfff`.  The ledger lists 26 rows in the
// window, all `decompiled_uncited`.  Nineteen are translated below and seven
// are skipped:
//
// * `FUN_005fa507`, `FUN_005fa630`, `FUN_005fa668` are three copies of the
//   same continuation inside the decimal `num_get` overload.  Their inputs
//   are the middleware registers of the enclosing body (`unaff_EBP`,
//   `unaff_ESI`, `unaff_EBX`), so they have no standalone call signature; the
//   only arithmetic they add is the width/precision padding reproduced from
//   `0x005fa2c0` and the 1.5x growth reproduced from `0x005fa4b0`.
// * `FUN_005fa847`, `FUN_005fb49d` are `__finally`/SEH epilogue stubs: restore
//   the saved `ExceptionList`, then tail-call the CRT epilogue thunk.  They
//   compute nothing and never return to their caller.
// * `FUN_005fa8e0` is the body of the decimal `num_get<char>::do_get`
//   overload.  It drives caller-owned istream/locale objects through indirect
//   calls with register-passed state.  Its self-contained pieces are the
//   prefix scan and width padding of `0x005fa2c0` and the rounding loop of
//   `0x005fafb5`.
// * `FUN_005fb4d0` is the 1908-byte body of the hexadecimal
//   `num_get<char>::do_get` overload, for the same reason.  Its character
//   table and eight-digit exponent window are reproduced from
//   `0x005fbc42`/`0x005fbeac`.
//
// The window is CRT / iostream support code, not simulation code.  Recovered
// shapes that recur across the rows:
//
// | offset                 | meaning                                            |
// |------------------------|----------------------------------------------------|
// | `string +0x0` (SSO)    | 16-byte inline buffer; first byte is the string     |
// | `string +0x10` / `+0x14` | `size` / `capacity` (MSVC `std::string`)          |
// | node stride `0x18`     | `{std::string}`; string at `+0`                     |
// | node stride `0x1c`     | `{u32 key, std::string}`; string at `+4`            |
// | node stride `0x34`     | a destructor-called record used by `0x005fa1b0`     |
//
// `std::string` capacity below `0x10` is the inline case; `capacity + 1`
// bytes are released above it, or `capacity + 0x24` once `capacity + 1`
// exceeds `0xfff` (the extra `0x23` accounts for the debug-heap header and
// 32-byte rounding).  The large-block path validates
// `(data - *(data - 4)) - 4 <= 0x1f`; a violation is the retail corruption
// trap.  Those constants are shared by every release row in the window.
//
// Where a row only differs from an already-reconstructed helper in its range
// walk, the translation delegates to `a45_release_text_node` from batch 45
// (`FUN_005ed240`/`FUN_005ed2b0`) rather than inventing a second release policy.

/// Inline (`std::string`) capacity written by every reset in the window
/// (`*(+0x14) = 0xf`, `*(+0x1c) = 0xf`).
pub const A52_SSO_CAPACITY: u32 = 0xf;

/// `capacity + 1` above this value takes the `capacity + 0x24` release path.
pub const A52_SMALL_BLOCK_LIMIT: u32 = 0xfff;

/// Debug-heap header bytes charged by the large-block path (`+0x24`).
pub const A52_CHUNK_HEADER: u32 = 0x24;

/// Allowed distance between the stored block pointer and the data pointer
/// (`(data - stored) - 4` must not exceed this).
pub const A52_ALIGN_SLACK: u32 = 0x1f;

/// Largest `std::string` size: `0x005fa4b0` rejects `size == 0x7fffffff` and
/// clamps the growth candidate to this value.
pub const A52_STRING_MAX: u32 = 0x7fff_ffff;

/// Record stride of the range destroyed by `0x005fa170` (`FUN_005ed240`).
pub const A52_NODE_1C_STRIDE: u32 = 0x1c;

/// Record stride of the range destroyed by `0x005fa190` (`FUN_005ed2b0`).
pub const A52_NODE_18_STRIDE: u32 = 0x18;

/// Record stride of the range destroyed by `0x005fa1b0` (`FUN_005f34c0`).
pub const A52_NODE_34_STRIDE: u32 = 0x34;

/// The 28-byte float character table built at `FUN_005fb4d0`
/// (`"0123456789ABCDEFabcdef-+XxPp"`).  Indices `0..=0x15` are accepted as
/// digits; `-`, `+`, `X`, `x`, `P`, `p` occupy `0x16..=0x1b`.
pub const A52_HEX_FLOAT_TABLE: &[u8; 28] = b"0123456789ABCDEFabcdef-+XxPp";

/// The decimal float character table used by `FUN_005fa8e0`/`0x005fafb5`
/// (`"0123456789-+Ee"`); only indices `0..=9` are digits.
pub const A52_DECIMAL_FLOAT_TABLE: &[u8; 14] = b"0123456789-+Ee";

/// A rejected element count (the retail `length_error` thrower of
/// `FUN_005fd600`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A52LengthError;

/// A corrupt debug-heap block (the `FUN_00c9474b` trap in every large release
/// path).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A52BlockError {
    /// `(data - stored) - 4` exceeded [`A52_ALIGN_SLACK`].
    Corrupt,
}

/// Outcome of a scalar deleting destructor, `0x005fa150`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A52DeleteAction {
    /// The delete flag was zero: the object is only destroyed.
    Keep,
    /// The flag was non-zero: retail tail-calls the deallocator.
    Free,
}

/// Tail reached by the `Catch_All@005fa10f` funclet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A52UnwindTail {
    /// `FUN_005fdde0` cleans the exception state and `FUN_00aad4a0(0, 0)`
    /// rethrows; no value reaches the caller.
    Rethrow,
}

/// One `0x1c`-byte record walked by `0x005fa170`: a key at `+0`, then the
/// embedded `std::string`'s data pointer at `+4`, size at `+0x14` and
/// capacity at `+0x18`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A52Node1C {
    /// The key dword at `+0`; untouched by the destructor.
    pub key: u32,
    /// Capacity word at `+0x18`.
    pub capacity: u32,
    /// Size word at `+0x14`.
    pub size: u32,
    /// First string byte at `+4` (zeroed by the reset).
    pub first_byte: u8,
}

/// One `0x18`-byte record walked by `0x005fa190`: a bare `std::string` with
/// its first byte at `+0`, size at `+0x10` and capacity at `+0x14`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A52Node18 {
    /// Capacity word at `+0x14`.
    pub capacity: u32,
    /// Size word at `+0x10`.
    pub size: u32,
    /// First string byte at `+0` (zeroed by the reset).
    pub first_byte: u8,
}

/// The four scalar stores `0x005fa080` performs before its delegated copies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A52CopyPlan {
    /// `dst[0] = src[0]`.
    pub word0: u32,
    /// `dst[6] = src[6]`.
    pub word6: u32,
    /// `dst[7] = src[7]`.
    pub word7: u32,
    /// `(int)(src[4] - src[3]) >> 2`, the dword count handed to the range
    /// assigner.
    pub element_count: i32,
}

/// A rounding-carry result for the decimal mantissa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A52Round {
    /// True when the carry left every candidate digit zero and the caller
    /// must bump the decimal exponent (`*(ebp-0x60) += 1`).
    pub bump_exponent: bool,
    /// True when the leading slot was rewritten with `'1'`.
    pub wrote_leading_one: bool,
}

/// Fixed-size allocation accounting of `0x005fa589`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A52AllocBlock {
    /// The requested byte count, also stored in the block header.
    pub request: u32,
    /// `(request + 0x23) & !0x1f`, the rounded block size.
    pub block: u32,
    /// `block - 4`, where the request is written.
    pub header: u32,
}

/// Source buffer selected by the copy committed at `0x005fa5b2`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A52CopySource {
    /// Old capacity below `0x10`: the bytes live in the inline buffer.
    Inline,
    /// Old capacity of at least `0x10`: the bytes live behind the first dword.
    Heap,
}

/// The state committed by `0x005fa5b2` after the character copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A52Commit {
    /// New length word: the previous length plus one.
    pub length: u32,
    /// New capacity word taken from the caller's frame.
    pub capacity: u32,
    /// Which source buffer the copy reads.
    pub source: A52CopySource,
}

/// The small-string fields reset by `0x005fa805`/`0x005fa880`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A52TempReset {
    /// Size word `(+0x18` or `(+0x10` depending on the caller).
    pub size: u32,
    /// Capacity word, always [`A52_SSO_CAPACITY`].
    pub capacity: u32,
    /// First byte of the inline buffer, always zero.
    pub first_byte: u8,
}

/// What the `0x005fa805` cleanup does after resetting the first temporary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A52Cleanup {
    /// The second temporary is inline: only the reset happens, then the
    /// routine returns through the SEH epilogue.
    Return(A52TempReset),
    /// The second temporary owns a heap block: release `bytes` after the
    /// reset.
    Free {
        /// The reset applied to the first temporary.
        reset: A52TempReset,
        /// Byte count handed to the deallocator.
        bytes: u32,
    },
}

/// What `0x005fa880` releases for a `std::string` whose buffer starts at
/// `+8`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A52SsoRelease {
    /// Inline capacity: the fields are reset (size 0, capacity `0xf`, first
    /// byte 0) and the object itself is released with the fixed `0x20`-byte
    /// request.
    Object {
        /// Fields written before the release.
        reset: A52TempReset,
        /// Fixed deallocation request (`0x20`).
        bytes: u32,
    },
    /// Heap capacity: release the character buffer.
    Heap {
        /// Capacity word that selected the request.
        capacity: u32,
        /// `capacity + 1`, or `capacity + 0x24` above the small-block limit.
        bytes: u32,
    },
}

/// `0x005fa080` (`FUN_005fa080`, 143 bytes, `decompiled_uncited`).
///
/// ```c
/// in_ECX[6] = param_1[6];
/// in_ECX[7] = param_1[7];
/// FUN_005fde60();
/// *in_ECX = *puVar1;
/// param_1 = (undefined4 *)in_ECX[1];
/// FUN_005fdc10((int)(puVar1[4] - puVar1[3]) >> 2, &param_1);
/// FUN_005f07d0(*(undefined4 *)puVar1[1], (undefined4 *)puVar1[1]);
/// ```
///
/// The four leading scalar copies and the dword count are the self-contained
/// computation.  `FUN_005fde60` (locale-facet copy), `FUN_005fdc10` (range
/// assignment) and `FUN_005f07d0` (single-node release) live outside the
/// window, so the plan reports the values they are handed instead of calling
/// them.  The count uses the C cast-then-shift order: subtract as unsigned,
/// reinterpret as signed, then arithmetic-shift right by two.
pub fn a52_copy_plan(src: &[u32; 8]) -> A52CopyPlan {
    A52CopyPlan {
        word0: src[0],
        word6: src[6],
        word7: src[7],
        element_count: (src[4].wrapping_sub(src[3]) as i32) >> 2,
    }
}

/// `0x005fa10f` (`Catch_All@005fa10f`, 17 bytes, `decompiled_uncited`).
///
/// ```c
/// FUN_005fdde0();
/// FUN_00aad4a0(0,0);   /* no return */
/// ```
///
/// A `__finally` funclet: clean the exception state, then rethrow with a null
/// exception object.  It never returns, so the translation exposes the
/// contract as [`A52UnwindTail::Rethrow`].
pub fn a52_catch_all_tail() -> A52UnwindTail {
    A52UnwindTail::Rethrow
}

/// `0x005fa130` (`FUN_005fa130`, 17 bytes, `decompiled_uncited`).
///
/// ```c
/// LOCK();
/// *(int *)(in_ECX + 4) = *(int *)(in_ECX + 4) + -1;
/// UNLOCK();
/// ```
///
/// An atomic refcount decrement with no result read.  The 32-bit wrap is part
/// of the behaviour (`0` becomes `-1`), so the new value is returned with
/// wrapping arithmetic and the caller can observe the `LOCK` result.
pub fn a52_refcount_release(current: i32) -> i32 {
    current.wrapping_sub(1)
}

/// `0x005fa150` (`FUN_005fa150`, 21 bytes, `decompiled_uncited`).
///
/// ```c
/// if (param_1 != '\0') {
///   FUN_00923e90();          /* scalar deleting destructor, no return */
/// }
/// ```
///
/// The delete-flag test of a scalar deleting destructor: zero destroys the
/// object only, any non-zero byte additionally calls the deallocator.
pub fn a52_delete_action(flag: u8) -> A52DeleteAction {
    if flag == 0 {
        A52DeleteAction::Keep
    } else {
        A52DeleteAction::Free
    }
}

/// `0x005fa170` (`FUN_005fa170`, 22 bytes, `decompiled_uncited`).
///
/// ```c
/// FUN_005ed240(param_1, param_2);
/// ```
///
/// A tail forwarder to the `0x1c`-byte range destructor already reconstructed
/// as [`a45_release_text_node`] (`FUN_005ed240`).  This wrapper adds the record
/// stride; each element's embedded string is reset to size `0`, capacity
/// `0xf`, first byte `0`, and its old heap block is released with `cap + 1`
/// or `cap + 0x24` bytes.  The returned value is the sum of the release
/// requests so the walk is observable.
pub fn a52_destroy_text_nodes_1c(nodes: &mut [A52Node1C]) -> u32 {
    let mut released = 0u32;
    for node in nodes.iter_mut() {
        released = released.wrapping_add(a52_release_text_node(node.capacity));
        node.size = 0;
        node.capacity = A52_SSO_CAPACITY;
        node.first_byte = 0;
    }
    released
}

/// `0x005fa190` (`FUN_005fa190`, 22 bytes, `decompiled_uncited`).
///
/// ```c
/// FUN_005ed2b0(param_1, param_2);
/// ```
///
/// The `0x18`-byte companion of [`a52_destroy_text_nodes_1c`]; the range
/// destructor is the `FUN_005ed2b0` translation, the same release policy with a
/// 4-byte shorter record (string at `+0`).
pub fn a52_destroy_text_nodes_18(nodes: &mut [A52Node18]) -> u32 {
    let mut released = 0u32;
    for node in nodes.iter_mut() {
        released = released.wrapping_add(a52_release_text_node(node.capacity));
        node.size = 0;
        node.capacity = A52_SSO_CAPACITY;
        node.first_byte = 0;
    }
    released
}

/// `0x005fa1b0` (`FUN_005fa1b0`, 36 bytes, `decompiled_uncited`).
///
/// ```c
/// for (; param_1 != param_2; param_1 = param_1 + 0x34)
///   FUN_005f34c0();
/// ```
///
/// A range destructor with no count precomputed: it steps `0x34` bytes and
/// stops on pointer equality.  A byte distance that is not a multiple of
/// `0x34` never compares equal, so it walks away from the range forever; that
/// is reported as [`A52RangeError::NonTerminating`] instead of looping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A52RangeError {
    /// `end - start` is not a multiple of the record stride.
    NonTerminating,
}

/// Counts the records `0x005fa1b0` would destroy for `start..end`.
pub fn a52_range_steps_34(start: u32, end: u32) -> Result<u32, A52RangeError> {
    let delta = end.wrapping_sub(start);
    if delta % A52_NODE_34_STRIDE == 0 {
        Ok(delta / A52_NODE_34_STRIDE)
    } else {
        Err(A52RangeError::NonTerminating)
    }
}

/// `0x005fa1e0` (`FUN_005fa1e0`, 18 bytes, `decompiled_uncited`).
///
/// ```c
/// (**(code **)(in_ECX + 4))(param_1);
/// ```
///
/// Calls the first virtual slot of the object stored at `this + 4`, passing
/// `param_1` unchanged and returning the callee's result.  The target is
/// outside the window, so the slot is abstracted as the closure argument.
pub fn a52_dispatch_slot0<T, R>(arg: T, target: impl FnOnce(T) -> R) -> R {
    target(arg)
}

/// `0x005fa200` (`FUN_005fa200`, 190 bytes, `decompiled_uncited`).
///
/// Builds the `printf` conversion specifier used by the float formatter:
///
/// ```c
/// *out = '%';
/// if (flags & 0x20) { *p++ = '+'; }
/// if (flags & 0x10) { *p++ = '#'; }
/// *p++ = '.'; *p++ = '*';              /* the 16-bit store 0x2a2e */
/// if (precision != 0) { *p++ = precision; }
/// switch (flags & 0x3000) {
///   case 0x3000: *p = (flags & 4) ? 'A' : 'a'; break;
///   case 0x2000: *p = 'f'; break;       /* lowercase even with flags & 4 */
///   case 0x1000: *p = (flags & 4) ? 'E' : 'e'; break;
///   default:     *p = (flags & 4) ? 'G' : 'g'; break;
/// }
/// *++p = 0;
/// ```
///
/// The returned vector includes the trailing NUL, exactly as the eight-byte
/// stack buffer is left by retail.  Note the recovered `0x2000` case: the
/// upper-case flag does **not** select `'F'`, the shared tail always stores
/// `'f'`.
pub fn a52_format_float(flags: u32, precision: Option<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(8);
    out.push(b'%');
    if flags & 0x20 != 0 {
        out.push(b'+');
    }
    if flags & 0x10 != 0 {
        out.push(b'#');
    }
    out.push(b'.');
    out.push(b'*');
    if let Some(p) = precision {
        if p != 0 {
            out.push(p);
        }
    }
    let class = flags & 0x3000;
    let conversion = if flags & 4 == 0 {
        match class {
            0x3000 => b'a',
            0x2000 => b'f',
            0x1000 => b'e',
            _ => b'g',
        }
    } else {
        match class {
            0x3000 => b'A',
            0x2000 => b'f',
            0x1000 => b'E',
            _ => b'G',
        }
    };
    out.push(conversion);
    out.push(0);
    out
}

/// `0x005fa2c0` (`FUN_005fa2c0`, 496 bytes, `decompiled_uncited`).
///
/// The sign and `0x` prefix scan at the start of the decimal `num_get` body:
///
/// ```c
/// offset = (length == 0 || (s[0] != '+' && s[0] != '-')) ? 0 : 1;
/// if ((flags & 0x3000) == 0x3000) {
///   if (offset + 2 <= length && s[offset] == '0' &&
///       (s[offset + 1] == 'x' || s[offset + 1] == 'X'))
///     offset += 2;
/// }
/// ```
///
/// The `0x` skip only happens when the conversion class is the hexadecimal
/// one (`flags & 0x3000 == 0x3000`); a lone `0` is left for the digit loop.
pub fn a52_float_prefix_len(bytes: &[u8], flags: u32) -> usize {
    let mut offset = match bytes.first() {
        Some(b'+') | Some(b'-') => 1,
        _ => 0,
    };
    if flags & 0x3000 == 0x3000
        && offset + 2 <= bytes.len()
        && bytes[offset] == b'0'
        && (bytes[offset + 1] == b'x' || bytes[offset + 1] == b'X')
    {
        offset += 2;
    }
    offset
}

/// `0x005fa2c0` (`FUN_005fa2c0`, 496 bytes, `decompiled_uncited`).
///
/// The width/precision padding expression, the second self-contained piece of
/// the decimal `num_get` body:
///
/// ```c
/// width = *(uint *)(state + 0x20);
/// precision = *(int *)(state + 0x24);
/// if (precision < 0 || (precision < 1 && width == 0) || width <= length)
///   pad = 0;
/// else
///   pad = width - length;
/// ```
///
/// `length` is the current temporary-string size (`uStack_1c`).  The three
/// disjuncts are reproduced in their original order; in particular a zero
/// precision does not suppress padding unless the width is also zero.
pub fn a52_width_padding(width: u32, precision: i32, length: u32) -> u32 {
    if precision < 0 || (precision < 1 && width == 0) || width <= length {
        0
    } else {
        width - length
    }
}

/// `0x005fa4b0` (`FUN_005fa4b0`, 99 bytes, `decompiled_uncited`).
///
/// The `std::string` growth policy inlined in the decimal parser:
///
/// ```c
/// if (size == 0x7fffffff) FUN_005fd600();        /* length_error */
/// cap = (size + 1) | 0xf;
/// if (cap < 0x80000000) {
///   half = capacity >> 1;
///   if (0x7fffffff - half < capacity) cap = 0x7fffffff;
///   else if (cap < half + capacity) cap = half + capacity;
/// } else {
///   cap = 0x7fffffff;
/// }
/// ```
///
/// `size` is the current length (`[ebp-0x18]`) and `capacity` the current
/// allocation (`[ebp-0x14]`).  The overflow test is the exact
/// `0x7fffffff - (capacity >> 1) < capacity` comparison, not a checked add.
pub fn a52_grow_capacity(size: u32, capacity: u32) -> Result<u32, A52LengthError> {
    if size == A52_STRING_MAX {
        return Err(A52LengthError);
    }
    let mut candidate = (size + 1) | 0xf;
    if candidate < 0x8000_0000 {
        let half = capacity >> 1;
        if A52_STRING_MAX - half < capacity {
            candidate = A52_STRING_MAX;
        } else {
            let grown = half + capacity;
            if candidate < grown {
                candidate = grown;
            }
        }
    } else {
        candidate = A52_STRING_MAX;
    }
    Ok(candidate)
}

/// `0x005fa589` (`FUN_005fa589`, 31 bytes, `decompiled_uncited`).
///
/// ```c
/// if (request == 0) FUN_00c9474b();              /* bad allocation, no return */
/// block = (request + 0x23) & 0xffffffe0;
/// *(int *)(block - 4) = request;
/// size = old_size + 1;
/// capacity = [ebp-0x60];
/// ```
///
/// The debug-heap block calculation: request plus the `0x23` header/padding
/// bytes, rounded down to a 32-byte boundary, with the original request stored
/// immediately before the returned pointer.  A zero request is rejected rather
/// than rounded.
pub fn a52_alloc_block(request: u32) -> Option<A52AllocBlock> {
    if request == 0 {
        return None;
    }
    let block = request.wrapping_add(0x23) & !0x1f;
    Some(A52AllocBlock {
        request,
        block,
        header: block.wrapping_sub(4),
    })
}

/// `0x005fa5b2` (`FUN_005fa5b2`, 41 bytes, `decompiled_uncited`).
///
/// ```c
/// [ebp-0x18] = edi + 1;          /* new size */
/// [ebp-0x14] = [ebp-0x60];       /* new capacity */
/// if (0xf < [ebp-0x5c])          /* old capacity selects the source */
///   memcpy(heap, new, ...);
/// else
///   memcpy(inline, new, ...);
/// ```
///
/// The scalar commit of an `assign` tail.  The copy itself is caller-owned;
/// the translation reports the new length/capacity and which source buffer the
/// `0xf < old_capacity` test selects.
pub fn a52_commit_copy(length_before: u32, old_capacity: u32, new_capacity: u32) -> A52Commit {
    A52Commit {
        length: length_before.wrapping_add(1),
        capacity: new_capacity,
        source: if old_capacity > A52_SSO_CAPACITY {
            A52CopySource::Heap
        } else {
            A52CopySource::Inline
        },
    }
}

/// `0x005fa604` (`FUN_005fa604`, 44 bytes, `decompiled_uncited`).
///
/// ```c
/// bytes = capacity + 1;
/// if (0xfff < bytes) {
///   bytes = capacity + 0x24;
///   if (0x1f < (data - *(int *)(data - 4)) - 4) FUN_00c9474b();
/// }
/// FUN_00923e90();                 /* deallocate */
/// ```
///
/// The release request and its large-block alignment guard.  Note the
/// comparison is against `0xfff < capacity + 1`, i.e. the `+0x24` path starts
/// at capacity `0xfff`; capacity `0xffe` still releases `0xfff` bytes.
pub fn a52_dealloc_request(capacity: u32, data: u32, stored: u32) -> Result<u32, A52BlockError> {
    let bytes = capacity.wrapping_add(1);
    if bytes > A52_SMALL_BLOCK_LIMIT {
        if data.wrapping_sub(stored).wrapping_sub(4) > A52_ALIGN_SLACK {
            return Err(A52BlockError::Corrupt);
        }
        Ok(capacity.wrapping_add(A52_CHUNK_HEADER))
    } else {
        Ok(bytes)
    }
}

/// `0x005fa805` (`FUN_005fa805`, 71 bytes, `decompiled_uncited`).
///
/// The second temporary destructor of the decimal parser:
///
/// ```c
/// first.size = 0; first.capacity = 0xf; first.byte = 0;
/// if (second.capacity < 0x10) return;            /* SEH epilogue */
/// if (0xfff < second.capacity + 1) {
///   second.bytes = second.capacity + 0x24;
///   if (0x1f < (second.data - *(int *)(second.data - 4)) - 4) FUN_00c9474b();
/// } else {
///   second.bytes = second.capacity + 1;
/// }
/// FUN_00923e90(second.bytes);
/// ```
///
/// Two temporaries are invalidated: the first is reset unconditionally, the
/// second is released only when it owns a heap block.  The reset fields and
/// the release request are returned so the branch is observable.
pub fn a52_cleanup_temp(
    capacity: u32,
    data: u32,
    stored: u32,
) -> Result<A52Cleanup, A52BlockError> {
    let reset = A52TempReset {
        size: 0,
        capacity: A52_SSO_CAPACITY,
        first_byte: 0,
    };
    if capacity < A52_SSO_CAPACITY + 1 {
        return Ok(A52Cleanup::Return(reset));
    }
    let bytes = a52_dealloc_request(capacity, data, stored)?;
    Ok(A52Cleanup::Free { reset, bytes })
}

/// `0x005fa880` (`FUN_005fa880`, 82 bytes, `decompiled_uncited`).
///
/// ```c
/// capacity = *(uint *)(this + 0x1c);
/// if (capacity < 0x10) {
///   *(uint *)(this + 0x18) = 0;
///   *(uint *)(this + 0x1c) = 0xf;
///   *(byte *)(this + 8) = 0;
///   FUN_00923e90(this, 0x20);
/// } else {
///   data = *(int *)(this + 8);
///   bytes = capacity + 1;
///   if (0xfff < bytes) {
///     stored = *(int *)(data - 4);
///     bytes = capacity + 0x24;
///     if (0x1f < (data - stored) - 4) FUN_00c9474b();
///   }
///   FUN_00923e90(data, bytes);
/// }
/// ```
///
/// A `std::string`-shaped object with its inline buffer at `+8`, size at
/// `+0x18` and capacity at `+0x1c`.  The inline branch resets the three fields
/// before its fixed `0x20`-byte object release; the heap branch only computes
/// the character-buffer release.
pub fn a52_sso_release(
    capacity: u32,
    data: u32,
    stored: u32,
) -> Result<A52SsoRelease, A52BlockError> {
    if capacity < A52_SSO_CAPACITY + 1 {
        return Ok(A52SsoRelease::Object {
            reset: A52TempReset {
                size: 0,
                capacity: A52_SSO_CAPACITY,
                first_byte: 0,
            },
            bytes: 0x20,
        });
    }
    let bytes = a52_dealloc_request(capacity, data, stored)?;
    Ok(A52SsoRelease::Heap { capacity, bytes })
}

/// `0x005fafb5` (`FUN_005fafb5`, 1329 bytes, `decompiled_uncited`).
///
/// The decimal rounding carry inside the `num_get` parser.  When the digit
/// window overflowed with a non-zero digit, retail walks the mantissa
/// backwards from the last stored character:
///
/// ```c
/// p = last;
/// if (p != first) {
///   do {
///     c = *p;
///     if (c != decimal_point) {
///       if (c != '9') { *p = c + 1; goto done; }
///       *p = '0';
///     }
///     p--;
///   } while (p != first);
/// }
/// exp++;
/// *p = '1';
/// done:
/// ```
///
/// `digits[0]` is the retained leading slot (`first`): the do-while decrements
/// before testing, so index 0 is never incremented; it is overwritten with
/// `'1'` only when every examined digit carried.  The locale decimal-point
/// byte is skipped without being incremented.  The returned flag says whether
/// the caller must add one to the exponent.
pub fn a52_round_decimal_digits(digits: &mut [u8], decimal_point: u8) -> A52Round {
    if digits.is_empty() {
        return A52Round {
            bump_exponent: false,
            wrote_leading_one: false,
        };
    }
    if digits.len() > 1 {
        let mut p = digits.len() - 1;
        while p != 0 {
            let c = digits[p];
            if c != decimal_point {
                if c != b'9' {
                    digits[p] = c.wrapping_add(1);
                    return A52Round {
                        bump_exponent: false,
                        wrote_leading_one: false,
                    };
                }
                digits[p] = b'0';
            }
            p -= 1;
        }
    }
    digits[0] = b'1';
    A52Round {
        bump_exponent: true,
        wrote_leading_one: true,
    }
}

/// `0x005fbc42` (`FUN_005fbc42`, 618 bytes, `decompiled_uncited`).
///
/// The hexadecimal float parser classifies each input byte through the
/// 28-byte table [`A52_HEX_FLOAT_TABLE`] with `FUN_00c8a380(table, byte,
/// 0x1c)`.  The returned pointer minus the table base is the position; only
/// positions `0..=0x15` are accepted as digits (`0x15 < index` is the reject
/// test).  Positions `0x16..=0x1b` are `-`, `+`, `X`, `x`, `P`, `p`.
///
/// This exposes the exact table lookup: it is not a hexadecimal digit-value
/// map, because the six lower-case letters start at index `0x10` rather than
/// at `0x0a`.
pub fn a52_hex_float_char(c: u8) -> Option<u8> {
    A52_HEX_FLOAT_TABLE
        .iter()
        .position(|&entry| entry == c)
        .map(|index| index as u8)
}

/// The table character for an accepted hex-float digit index `0..=0x15`.
pub fn a52_hex_float_digit(index: u8) -> Option<u8> {
    if index <= 0x15 {
        Some(A52_HEX_FLOAT_TABLE[index as usize])
    } else {
        None
    }
}

/// `0x005fbeac` (`FUN_005fbeac`, 629 bytes, `decompiled_uncited`).
///
/// The sign/exponent writer shared by the hexadecimal parser tail:
///
/// ```c
/// if (first_byte == '-') { *out++ = '-'; consume(); }
/// /* ... skip the decimal point ... */
/// if (count < 8) { *out++ = "0123456789ABCDEFabcdef-+XxPp"[index]; count++; }
/// ```
///
/// The digit window is eight characters wide; further accepted digits are
/// consumed but not stored, and the first rejected byte ends the window.  The
/// translation takes the already-classified table indices and reports exactly
/// the bytes retail would store.
pub fn a52_exponent_digits(negative: bool, digits: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(9);
    if negative {
        out.push(b'-');
    }
    for &index in digits.iter().take(8) {
        match a52_hex_float_digit(index) {
            Some(byte) => out.push(byte),
            None => break,
        }
    }
    out
}

/// Shared private form of the two `0x005fa170`/`0x005fa190` release calls.
fn a52_release_text_node(capacity: u32) -> u32 {
    a45_release_text_node(capacity).unwrap_or(0)
}

#[cfg(test)]
mod tests_agent52 {
    use super::*;

    #[test]
    fn copy_plan_reproduces_the_scalar_stores_and_shift() {
        // 0x005fa080
        let src = [0xa0, 1, 2, 0x100, 0x10c, 5, 0xc0, 0xc1];
        let plan = a52_copy_plan(&src);
        assert_eq!(plan.word0, 0xa0);
        assert_eq!(plan.word6, 0xc0);
        assert_eq!(plan.word7, 0xc1);
        assert_eq!(plan.element_count, 3, "(0x10c - 0x100) >> 2");

        let mut odd = src;
        odd[4] = 0x10b;
        assert_eq!(a52_copy_plan(&odd).element_count, 2, "15 >> 2 truncates");

        let mut negative = src;
        negative[3] = 0;
        negative[4] = 0x8000_0000;
        assert_eq!(
            a52_copy_plan(&negative).element_count,
            i32::MIN >> 2,
            "unsigned subtract then signed arithmetic shift"
        );

        let mut equal = src;
        equal[4] = equal[3];
        assert_eq!(a52_copy_plan(&equal).element_count, 0);
    }

    #[test]
    fn catch_all_tail_is_the_only_non_returning_tail() {
        // 0x005fa10f
        assert_eq!(a52_catch_all_tail(), A52UnwindTail::Rethrow);
    }

    #[test]
    fn refcount_release_wraps_at_the_signed_boundary() {
        // 0x005fa130
        assert_eq!(a52_refcount_release(3), 2);
        assert_eq!(a52_refcount_release(1), 0);
        assert_eq!(a52_refcount_release(0), -1);
        assert_eq!(a52_refcount_release(i32::MIN), i32::MAX);
    }

    #[test]
    fn delete_flag_zero_keeps_the_object() {
        // 0x005fa150
        assert_eq!(a52_delete_action(0), A52DeleteAction::Keep);
        assert_eq!(a52_delete_action(1), A52DeleteAction::Free);
        assert_eq!(a52_delete_action(0x80), A52DeleteAction::Free);
        assert_eq!(a52_delete_action(0xff), A52DeleteAction::Free);
    }

    #[test]
    fn destroy_1c_range_resets_strings_and_sums_release_requests() {
        // 0x005fa170 (callee FUN_005ed240)
        let mut nodes = [
            A52Node1C {
                key: 7,
                capacity: A52_SSO_CAPACITY,
                size: 3,
                first_byte: b'x',
            },
            A52Node1C {
                key: 8,
                capacity: 0x10,
                size: 9,
                first_byte: b'y',
            },
            A52Node1C {
                key: 9,
                capacity: 0xffe,
                size: 9,
                first_byte: b'z',
            },
            A52Node1C {
                key: 10,
                capacity: 0xfff,
                size: 9,
                first_byte: b'w',
            },
        ];
        assert_eq!(a52_destroy_text_nodes_1c(&mut nodes), 0x2033);
        for (index, node) in nodes.iter().enumerate() {
            assert_eq!(node.key, 7 + index as u32, "the key dword is untouched");
            assert_eq!(node.size, 0);
            assert_eq!(node.capacity, A52_SSO_CAPACITY);
            assert_eq!(node.first_byte, 0);
        }
    }

    #[test]
    fn destroy_18_range_uses_the_same_release_policy() {
        // 0x005fa190 (callee FUN_005ed2b0)
        let mut nodes = [
            A52Node18 {
                capacity: 0,
                size: 5,
                first_byte: b'a',
            },
            A52Node18 {
                capacity: 0x10,
                size: 5,
                first_byte: b'b',
            },
            A52Node18 {
                capacity: 0xfff,
                size: 5,
                first_byte: b'c',
            },
        ];
        assert_eq!(a52_destroy_text_nodes_18(&mut nodes), 0x1034);
        for node in &nodes {
            assert_eq!(node.size, 0);
            assert_eq!(node.capacity, A52_SSO_CAPACITY);
            assert_eq!(node.first_byte, 0);
        }
    }

    #[test]
    fn range_steps_34_needs_an_exact_stride_multiple() {
        // 0x005fa1b0
        assert_eq!(a52_range_steps_34(0x1000, 0x1068), Ok(2));
        assert_eq!(a52_range_steps_34(0x1000, 0x1000), Ok(0));
        assert_eq!(
            a52_range_steps_34(0x1000, 0x1001),
            Err(A52RangeError::NonTerminating)
        );
        assert_eq!(
            a52_range_steps_34(0x1000, 0x1033),
            Err(A52RangeError::NonTerminating)
        );
        assert_eq!(a52_range_steps_34(0, 0x34), Ok(1));
    }

    #[test]
    fn slot0_dispatch_forwards_its_argument() {
        // 0x005fa1e0
        let mut seen = 0u32;
        let result = a52_dispatch_slot0(41u32, |arg| {
            seen = arg;
            arg + 1
        });
        assert_eq!(seen, 41);
        assert_eq!(result, 42);
    }

    #[test]
    fn format_float_writes_the_recovered_specifier_bytes() {
        // 0x005fa200
        assert_eq!(a52_format_float(0, None), b"%.*g\0");
        assert_eq!(a52_format_float(0x20, None), b"%+.*g\0");
        assert_eq!(a52_format_float(0x10, None), b"%#.*g\0");
        assert_eq!(a52_format_float(0x20 | 0x10, None), b"%+#.*g\0");
        assert_eq!(a52_format_float(0x1000, None), b"%.*e\0");
        assert_eq!(a52_format_float(0x2000, None), b"%.*f\0");
        assert_eq!(a52_format_float(0x3000, None), b"%.*a\0");
        assert_eq!(a52_format_float(4, None), b"%.*G\0");
        assert_eq!(a52_format_float(4 | 0x1000, None), b"%.*E\0");
        assert_eq!(a52_format_float(4 | 0x3000, None), b"%.*A\0");
        assert_eq!(
            a52_format_float(4 | 0x2000, None),
            b"%.*f\0",
            "the 0x2000 case shares the lowercase 'f' tail"
        );
        assert_eq!(
            a52_format_float(0x20 | 0x10 | 4 | 0x3000, Some(b'9')),
            b"%+#.*9A\0"
        );
        assert_eq!(
            a52_format_float(0x3000, Some(0)),
            b"%.*a\0",
            "a zero precision byte is not stored"
        );
        assert_eq!(a52_format_float(0, Some(b'L')).len(), 6);
    }

    #[test]
    fn float_prefix_scan_skips_sign_and_only_hex_0x() {
        // 0x005fa2c0
        assert_eq!(a52_float_prefix_len(b"-0x1.8p3", 0x3000), 3);
        assert_eq!(a52_float_prefix_len(b"+0X1", 0x3000), 3);
        assert_eq!(a52_float_prefix_len(b"0x1", 0x3000), 2);
        assert_eq!(a52_float_prefix_len(b"-1", 0x3000), 1);
        assert_eq!(a52_float_prefix_len(b"0x1", 0), 0);
        assert_eq!(a52_float_prefix_len(b"0x1", 0x1000), 0);
        assert_eq!(a52_float_prefix_len(b"-0x", 0x3000), 3);
        assert_eq!(a52_float_prefix_len(b"-0", 0x3000), 1);
        assert_eq!(a52_float_prefix_len(b"", 0x3000), 0);
        assert_eq!(a52_float_prefix_len(b"+", 0x3000), 1);
        assert_eq!(a52_float_prefix_len(b"x0", 0x3000), 0);
    }

    #[test]
    fn width_padding_reproduces_the_three_disjuncts() {
        // 0x005fa2c0
        assert_eq!(a52_width_padding(10, 3, 4), 6);
        assert_eq!(a52_width_padding(4, 3, 4), 0);
        assert_eq!(a52_width_padding(3, 3, 4), 0);
        assert_eq!(a52_width_padding(5, 0, 0), 5);
        assert_eq!(a52_width_padding(0, 0, 0), 0);
        assert_eq!(a52_width_padding(10, -1, 0), 0);
        assert_eq!(a52_width_padding(10, -1, 20), 0);
        assert_eq!(a52_width_padding(u32::MAX, 1, 0), u32::MAX);
    }

    #[test]
    fn grow_capacity_follows_the_one_and_a_half_policy() {
        // 0x005fa4b0
        assert_eq!(a52_grow_capacity(0, 0), Ok(0xf));
        assert_eq!(a52_grow_capacity(0x10, 0xf), Ok(0x1f));
        assert_eq!(
            a52_grow_capacity(0x100, 0x200),
            Ok(0x300),
            "capacity/2 + capacity wins over size+1"
        );
        assert_eq!(
            a52_grow_capacity(0x10, 0x6000_0000),
            Ok(A52_STRING_MAX),
            "0x7fffffff - half < capacity clamps"
        );
        assert_eq!(a52_grow_capacity(0x7fff_fffe, 0), Ok(A52_STRING_MAX));
        assert_eq!(a52_grow_capacity(A52_STRING_MAX, 0), Err(A52LengthError));
    }

    #[test]
    fn alloc_block_rounds_and_stores_the_request() {
        // 0x005fa589
        assert_eq!(a52_alloc_block(0), None);
        assert_eq!(
            a52_alloc_block(1),
            Some(A52AllocBlock {
                request: 1,
                block: 0x20,
                header: 0x1c
            })
        );
        assert_eq!(a52_alloc_block(0x1d).unwrap().block, 0x40);
        assert_eq!(a52_alloc_block(0x1e).unwrap().block, 0x40);
        assert_eq!(a52_alloc_block(0x20).unwrap().block, 0x40);
        assert_eq!(
            a52_alloc_block(0x100).unwrap(),
            A52AllocBlock {
                request: 0x100,
                block: 0x120,
                header: 0x11c
            }
        );
        for request in 1..=0x100u32 {
            let block = a52_alloc_block(request).unwrap();
            assert_eq!(block.header + 4, block.block);
            assert_eq!(block.block & 0x1f, 0);
            assert!(block.block >= 0x20);
            assert_eq!(block.request, request);
        }
    }

    #[test]
    fn commit_copy_reports_length_capacity_and_source() {
        // 0x005fa5b2
        assert_eq!(
            a52_commit_copy(4, 0xf, 0x40),
            A52Commit {
                length: 5,
                capacity: 0x40,
                source: A52CopySource::Inline
            }
        );
        assert_eq!(
            a52_commit_copy(4, 0x10, 0x40),
            A52Commit {
                length: 5,
                capacity: 0x40,
                source: A52CopySource::Heap
            }
        );
        assert_eq!(a52_commit_copy(u32::MAX, 0, 0).length, 0);
    }

    #[test]
    fn dealloc_request_switches_at_the_small_block_limit() {
        // 0x005fa604
        assert_eq!(a52_dealloc_request(0, 0, 0), Ok(1));
        assert_eq!(a52_dealloc_request(0xffe, 0, 0), Ok(0xfff));
        assert_eq!(a52_dealloc_request(0xfff, 0x1004, 0x1000), Ok(0x1023));
        assert_eq!(a52_dealloc_request(0x1000, 0x1004, 0x1000), Ok(0x1024));
        assert_eq!(
            a52_dealloc_request(0xfff, 0x1004, 0x0fe0),
            Err(A52BlockError::Corrupt),
            "(0x24 - 4) exceeds 0x1f"
        );
        assert_eq!(
            a52_dealloc_request(0xfff, 0x1004, 0x1001),
            Err(A52BlockError::Corrupt),
            "(3 - 4) wraps and exceeds 0x1f"
        );
        assert_eq!(
            a52_dealloc_request(0xfff, 0x1004, 0x0fe1),
            Ok(0x1023),
            "(0x23 - 4) is exactly 0x1f"
        );
    }

    #[test]
    fn cleanup_temp_resets_the_first_and_frees_the_second() {
        // 0x005fa805
        let reset = A52TempReset {
            size: 0,
            capacity: A52_SSO_CAPACITY,
            first_byte: 0,
        };
        assert_eq!(a52_cleanup_temp(0xf, 0, 0), Ok(A52Cleanup::Return(reset)));
        assert_eq!(
            a52_cleanup_temp(0x10, 0, 0),
            Ok(A52Cleanup::Free { reset, bytes: 0x11 })
        );
        assert_eq!(
            a52_cleanup_temp(0xfff, 0x1004, 0x1000),
            Ok(A52Cleanup::Free {
                reset,
                bytes: 0x1023
            })
        );
        assert_eq!(
            a52_cleanup_temp(0xfff, 0x1004, 0x0fe0),
            Err(A52BlockError::Corrupt)
        );
    }

    #[test]
    fn sso_release_resets_inline_objects_and_frees_heap_buffers() {
        // 0x005fa880
        assert_eq!(
            a52_sso_release(0, 0, 0),
            Ok(A52SsoRelease::Object {
                reset: A52TempReset {
                    size: 0,
                    capacity: A52_SSO_CAPACITY,
                    first_byte: 0
                },
                bytes: 0x20
            })
        );
        assert_eq!(
            a52_sso_release(0x10, 0, 0),
            Ok(A52SsoRelease::Heap {
                capacity: 0x10,
                bytes: 0x11
            })
        );
        assert_eq!(
            a52_sso_release(0xffe, 0, 0),
            Ok(A52SsoRelease::Heap {
                capacity: 0xffe,
                bytes: 0xfff
            })
        );
        assert_eq!(
            a52_sso_release(0xfff, 0x1000, 0x0fdd),
            Ok(A52SsoRelease::Heap {
                capacity: 0xfff,
                bytes: 0x1023
            })
        );
        assert_eq!(
            a52_sso_release(0xfff, 0x1000, 0x0fdc),
            Err(A52BlockError::Corrupt)
        );
    }

    #[test]
    fn decimal_rounding_increments_the_last_non_nine() {
        // 0x005fafb5
        let mut digits = *b"012349";
        let round = a52_round_decimal_digits(&mut digits, b'.');
        assert_eq!(digits, *b"012350");
        assert_eq!(
            round,
            A52Round {
                bump_exponent: false,
                wrote_leading_one: false
            }
        );

        let mut negative = *b"0123-9";
        assert!(!a52_round_decimal_digits(&mut negative, b'.').bump_exponent);
        assert_eq!(negative, *b"0123.0", "the decimal point is skipped");
    }

    #[test]
    fn decimal_rounding_carries_into_the_leading_slot() {
        // 0x005fafb5
        let mut digits = *b"09999";
        let round = a52_round_decimal_digits(&mut digits, b'.');
        assert_eq!(digits, *b"10000");
        assert_eq!(
            round,
            A52Round {
                bump_exponent: true,
                wrote_leading_one: true
            }
        );

        let mut retained = *b"01999";
        let round = a52_round_decimal_digits(&mut retained, b'.');
        assert_eq!(
            retained, *b"02000",
            "index 0 is the retained slot and is not incremented"
        );
        assert!(!round.bump_exponent);

        let mut point = *b"01.99";
        let round = a52_round_decimal_digits(&mut point, b'.');
        assert_eq!(point, *b"02.00");
        assert!(!round.bump_exponent);

        let mut single = *b"9";
        let round = a52_round_decimal_digits(&mut single, b'.');
        assert_eq!(single, *b"1");
        assert!(round.bump_exponent);

        let mut empty: [u8; 0] = [];
        assert!(!a52_round_decimal_digits(&mut empty, b'.').bump_exponent);
    }

    #[test]
    fn hex_float_table_lookup_preserves_lowercase_positions() {
        // 0x005fbc42
        assert_eq!(A52_HEX_FLOAT_TABLE.len(), 28);
        for (index, &byte) in A52_HEX_FLOAT_TABLE.iter().enumerate() {
            assert_eq!(a52_hex_float_char(byte), Some(index as u8));
        }
        assert_eq!(a52_hex_float_char(b'0'), Some(0));
        assert_eq!(a52_hex_float_char(b'9'), Some(9));
        assert_eq!(a52_hex_float_char(b'A'), Some(10));
        assert_eq!(a52_hex_float_char(b'F'), Some(15));
        assert_eq!(a52_hex_float_char(b'a'), Some(16));
        assert_eq!(a52_hex_float_char(b'f'), Some(21));
        assert_eq!(a52_hex_float_char(b'-'), Some(22));
        assert_eq!(a52_hex_float_char(b'+'), Some(23));
        assert_eq!(a52_hex_float_char(b'X'), Some(24));
        assert_eq!(a52_hex_float_char(b'x'), Some(25));
        assert_eq!(a52_hex_float_char(b'P'), Some(26));
        assert_eq!(a52_hex_float_char(b'p'), Some(27));
        assert_eq!(a52_hex_float_char(b'g'), None);
        assert_eq!(a52_hex_float_char(0), None);
        assert_eq!(a52_hex_float_digit(0x15), Some(b'f'));
        assert_eq!(a52_hex_float_digit(0x16), None);
    }

    #[test]
    fn exponent_window_writes_a_sign_and_at_most_eight_digits() {
        // 0x005fbeac
        assert_eq!(a52_exponent_digits(false, &[]), b"");
        assert_eq!(a52_exponent_digits(true, &[0, 1, 2]), b"-012");
        assert_eq!(a52_exponent_digits(false, &[10, 16, 21]), b"Aaf");
        assert_eq!(
            a52_exponent_digits(false, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
            b"01234567",
            "only the first eight digits are stored"
        );
        assert_eq!(
            a52_exponent_digits(false, &[1, 2, 0x16, 3]),
            b"12",
            "the first rejected index ends the window"
        );
    }
}
