#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent54.rs
// ===========================================================================
// Translation batch 54.  Only this file is edited by the batch.
//
// Batch 54 owns `0x005fe000..=0x005fffff`.  The ledger lists 48
// `decompiled_uncited` rows in the window: 30 are translated below and 18
// are skipped (the list with reasons closes this comment).
//
// The window is statically linked MSVC C++ library support, not gameplay
// code.  Three families appear:
//
// * The `std::num_get<char>` integer rows (`0x005fe160`..`0x005fed70`).  Each
//   one locks the stream's locale, scans the field with the recovered
//   `_Getifld` (`FUN_005fc150`), converts the scanned text with `_Stol`,
//   `_Stoul`, `_Stoll` or `_Stoull` (`FUN_00c84c6b`, `FUN_00c84d10`,
//   `FUN_00c84e83`, `FUN_00c84f51`), stores the value and reports `failbit` when
//   the scan converted nothing or raised `ERANGE`.  The three float rows add
//   the recovered decimal-exponent multiply and the `ldexp` rescale.
// * The `std::num_put<char>` float rows (`0x005ff300`, `0x005ff4b0`), which
//   size a temporary string from the stream precision, grow it by
//   `|frexp exponent| * 30103 / 100000` for `fixed` values above `1e10`,
//   build the `printf` specification and emit the field.
// * The container support veneer: the small-block deallocation rows
//   (`0x005fe030`, `0x005fe070`, `0x005fe0b0`), the `std::string` assignment
//   wrappers (`0x005fe120`, `0x005ff1b0`, `0x005ffaa0`), `char_traits::copy`
//   (`0x005ff200`), the two `ctype` case-mapping overrides with their range
//   forms (`0x005ff9c0`, `0x005ff9e0`, `0x005ffa30`, `0x005ffa50`), the
//   `_Hash` node unlink rows (`0x005ffb90`, `0x005ffc10`), `string::find`
//   (`0x005ffd90`) and the `basic_ios` field initialiser (`0x005fff70`).
//
// Recovered layout used throughout the batch:
//
// | shape | meaning |
// |-------|---------|
// | `ios_base` | `_Mystate` `+0x8`, `_Except` `+0xc`, `_Wide` `+0x10`, flags `+0x14`, precision `+0x18` (`i64`), width `+0x20` (`i64`), `+0x28`/`+0x2c`/`+0x30` zeroed |
// | `std::string` | union `+0x0`, size `+0x10`, capacity `+0x14`; `capacity < 0x10` selects the 16-byte inline buffer |
// | `_Hash` | end node `+0x4`, count `+0x8`, bucket array `+0xc`, mask `+0x18`; a bucket is `{first, last}` eight bytes wide and an empty bucket points at the end node |
// | `_Hash` node | next `+0x0`, prev `+0x4`, key bytes `+0x8`; the string-keyed form keeps the usual 32-byte `std::string` header at `+0x8` (data `+0x8`, size `+0x18`, capacity `+0x1c`) and is released with a 12-byte delete |
// | locale facet | `numpunct`/`ctype` at `+0x8` of the `_Locinfo` returned by `FUN_005f09e0`; its vtable slots `+0x10` group separator, `+0x14` grouping, `+0x18` `truename`, `+0x1c` `falsename` |
//
// Conventions:
//
// * Every row below is a pure free function.  Nothing in the window reads or
//   writes `CUnit`/`World` simulation state, so this batch adds no fields to
//   [`crate::sim::World`] or [`crate::sim::SimUnit`] and no `a54_` inherent
//   method.
// * Callees that live outside the window are abstracted as value inputs and
//   outputs: the field scanner (`_Getifld` `FUN_005fc150`), the decimal scans
//   (`_Stof` `FUN_00c980b9`, `_Stod` `FUN_00c980a2`), `frexp` (`FUN_00c94828`),
//   `pow` (`FUN_00c9b940`), `sprintf` (`FUN_00603450`) and the field emitter
//   (`_Put` `FUN_005fa2c0`).  Each doc comment names the abstracted input.
// * The decompiler split bodies at their deallocation/unwind calls.  The
//   translations stop at the same point and report the computed values
//   instead of running an allocator.
//
// Skipped rows, and why:
//
// * `FUN_005fe6ea`, `FUN_005fea3a`: unwind/SSE tail fragments of the float scan
//   body.  Both read the enclosing frame (`unaff_EBP`, `in_ST0`,
//   `in_XMM0`), so their inputs do not exist as a standalone call.
// * `FUN_005feeb0`, `FUN_005ff7a0`: the `bool` get/put pair.  Their non
//   `boolalpha` paths are simple, but the `boolalpha` paths read the locale
//   `truename`/`falsename` through vtable slots and compare them with the
//   unanalysed `func_0x005ee2d0`, so the branch that decides `true`/`false`
//   cannot be reproduced from the cache.
// * `FUN_005ff1f0`, `FUN_005ffae0`: `return param_1` one-byte `_Widen`/`_Narrow`
//   passthroughs.  There is no arithmetic, table or state to reproduce.
// * `FUN_005ff220`, `FUN_005ff290`, `FUN_005ff660`, `FUN_005ff6c0`, `FUN_005ff730`:
//   integer `num_put` overloads.  The length/conversion spec handed to the
//   `_Ifmt` builder (`FUN_005fc670`) travels in registers the decompiled body
//   does not show, and `FUN_005ff660` formats with a static string at
//   `0x00d38c8c` that is not in the cache; a translation would have to guess
//   which overload each row is.
// * `FUN_005ffaf0`: the same `char_traits::copy` body as `0x005ff200`, only the
//   destination argument position differs (three arguments instead of four).
// * `FUN_005ffb10`: stores `[ecx+4]` into `*param_1`; a one-dword iterator read
//   with no arithmetic or state.
// * `FUN_005ffb20`, `FUN_005ffb50`: `_Hash` iterator comparisons.  Both compare
//   the node pointer and the owner's end-node pointer, and `FUN_005ffb50` gets
//   one operand from a virtual `_Find`; the two pointer comparisons are
//   already covered by the erase rows and there is no further arithmetic.
// * `FUN_005ffcc0`, `FUN_005ffd10`: `std::list` node insert/pop steps.  Both call
//   the node link helpers `FUN_005efcc0`/`FUN_005ed2b0` outside the window and
//   the `FUN_005ffd10` body ends in the deallocation call; without the node
//   allocator the remaining statements are the two string-header stores
//   modelled by the assignment rows.
// * `FUN_005ffeb0`: `ios_base` locale teardown.  Every observable step is a
//   virtual call on the locale facet (`+0x34`, `+0x8`) plus the `_Lockit`
//   thunk, so nothing but the reference count can be reconstructed.

/// `ios_base::failbit`, the state the rows below store on a failed parse.
pub const A54_FAILBIT: u32 = 2;

/// `ios_base::eofbit`, set when the input range was exhausted.
pub const A54_EOFBIT: u32 = 1;

/// `_Mystate`/`_Except` initial value (`0x005fff70` stores zero dwords).
pub const A54_GOODBIT: u32 = 0;

/// `skipws`, part of the `0x201` flags word written by `0x005fff70`.
pub const A54_SKIPWS: u32 = 0x0001;

/// `dec`, the other half of the `0x201` flags word.
pub const A54_DEC: u32 = 0x0200;

/// The flags word `0x005fff70` stores at `+0x14` (`skipws | dec`).
pub const A54_DEFAULT_FLAGS: u32 = A54_SKIPWS | A54_DEC;

/// `uppercase`, selects `A`/`E`/`G`/`X` conversions.
pub const A54_UPPERCASE: u32 = 0x0004;

/// `showpoint`, the `#` flag of the `printf` specification.
pub const A54_SHOWPOINT: u32 = 0x0010;

/// `showpos`, the `+` flag of the `printf` specification.
pub const A54_SHOWPOS: u32 = 0x0020;

/// `scientific`, one of the `floatfield` bits.
pub const A54_SCIENTIFIC: u32 = 0x1000;

/// `fixed`, one of the `floatfield` bits.
pub const A54_FIXED: u32 = 0x2000;

/// `hexfloat` (`fixed | scientific`).
pub const A54_HEXFLOAT: u32 = 0x3000;

/// `floatfield` (`scientific | fixed`).
pub const A54_FLOATFIELD: u32 = 0x3000;

/// The precision sentinel of the `num_get` float rows: the temporary buffer
/// is sized for `0x300` significant digits when the limit still holds this
/// value (`FUN_005fa8e0` compares the pointed-to dword with `1000000000`).
pub const A54_NO_PRECISION_LIMIT: i32 = 1_000_000_000;

/// The default stream precision written by `0x005fff70` (`+0x18 = 6`).
pub const A54_DEFAULT_PRECISION: i64 = 6;

/// Inline `std::string` capacity (`+0x14 = 0xf`).
pub const A54_SSO_CAPACITY: u32 = 0xf;

/// Largest representable string size (`0x7fffffff`).
pub const A54_STRING_MAX: u32 = 0x7fff_ffff;

/// Allocation class boundary: byte counts above `0xfff` take the block-header
/// path.
pub const A54_LARGE_BLOCK: u32 = 0xfff;

/// Bytes charged for a large block (`size + 0x23`, see `0x005fe030`).
pub const A54_BLOCK_OVERHEAD: u32 = 0x23;

/// Largest allowed distance from the stored header pointer to the data minus
/// four (`0x1f < (ptr - stored) - 4` is the `0x005fe030` corruption trap).
pub const A54_ALIGN_SLACK: u32 = 0x1f;

/// The locale scan limit branch of `0x005fe650`/`0x005fe7d0`: any other value
/// rescales the parsed value by `2^(limit << 2)` with `ldexp`.
pub const A54_LIMIT_SHIFT: u32 = 2;

/// FNV-1a 32-bit basis used by the `_Hash` rows (`0x005ffb90`, `0x005ffc10`).
pub const A54_HASH_BASIS: u32 = 0x811c_9dc5;

/// FNV-1a 32-bit prime used by the `_Hash` rows.
pub const A54_HASH_PRIME: u32 = 0x0100_0193;

/// Bytes of the block `0x005fff70` allocates after zeroing its fields
/// (`FUN_00923df0(8)`).
pub const A54_IOS_INIT_ALLOCATION: usize = 8;

/// Header of one 24-byte MSVC `std::string` member.
///
/// The union at `+0x0` is either the 16-byte inline buffer or a heap pointer;
/// the translation only needs to know which storage the header selects, so
/// that union is represented by `heap`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A54String {
    /// Retail `+0x10`: number of stored bytes.
    pub len: u32,
    /// Retail `+0x14`: allocation size; below `0x10` means inline storage.
    pub cap: u32,
    /// Retail `+0x0`: `true` when the union points at a heap block.
    pub heap: bool,
}

/// The three stores every assignment wrapper performs before copying:
/// `size = 0` (`+0x10`), `capacity = 0xf` (`+0x14`), `buf[0] = 0` (`+0x0`).
pub fn a54_empty_string() -> A54String {
    A54String {
        len: 0,
        cap: A54_SSO_CAPACITY,
        heap: false,
    }
}

/// Result of one `std::string::assign(const char*)` row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A54Assign {
    /// Number of bytes the source C string contains (`strlen`).
    pub len: u32,
    /// Capacity the header holds after the assignment.
    pub cap: u32,
    /// `true` when the assignment left the inline buffer for a heap block.
    pub heap: bool,
    /// `true` when the retail body took its `_Reallocate` branch.
    pub reallocated: bool,
}

/// Capacity selected by the `FUN_005fdb20` growth policy.
///
/// The requested `len | 0xf` is raised to `old_cap + old_cap / 2` when that is
/// larger, clamped to `0x7fffffff` once `0x7fffffff - old_cap / 2 < old_cap`,
/// and `len >= 0x80000000` takes the `length_error` thrower
/// (`FUN_005fd600`), reported here as `None`.
fn a54_grow_capacity(len: u32, old_cap: u32) -> Option<u32> {
    if len >= 0x8000_0000 {
        return None;
    }
    let mut cap = len | A54_SSO_CAPACITY;
    if cap < 0x8000_0000 {
        if A54_STRING_MAX - (old_cap >> 1) < old_cap {
            cap = A54_STRING_MAX;
        } else {
            let grown = (old_cap >> 1) + old_cap;
            if cap < grown {
                cap = grown;
            }
        }
    } else {
        cap = A54_STRING_MAX;
    }
    Some(cap)
}

/// One `assign(const char*)` after the three inline-clear stores.
///
/// `src` is the C string the row reads from the member pointer (`strlen`
/// supplies the length).  The capacity arithmetic is the exact `FUN_005fdb20`
/// policy: a length that fits the freshly cleared `capacity = 0xf` only stores
/// the new size, a longer one allocates `len | 0xf` or one-and-a-half times
/// the old capacity, whichever is larger.
fn a54_assign_c_str(src: &[u8]) -> Option<A54Assign> {
    let len = src.len() as u32;
    let cleared = a54_empty_string();
    if len <= cleared.cap {
        return Some(A54Assign {
            len,
            cap: cleared.cap,
            heap: false,
            reallocated: false,
        });
    }
    let cap = a54_grow_capacity(len, cleared.cap)?;
    Some(A54Assign {
        len,
        cap,
        heap: true,
        reallocated: true,
    })
}

/// `0x005fe120`: assign the C string held by the member pointer at `+0x10`.
///
/// The body clears the destination header (`+0x10 = 0`, `+0x14 = 0xf`,
/// `buf[0] = 0`), measures the member with a `strlen` loop and calls the
/// assignment helper `FUN_005fdb20`.  Returns the resulting header and the
/// `strlen` result; `None` is the `length_error` path.
pub fn a54_assign_member_10(src: &[u8]) -> Option<A54Assign> {
    a54_assign_c_str(src)
}

/// `0x005ff1b0`: the same assignment with the member pointer at `+0x8`.
pub fn a54_assign_member_08(src: &[u8]) -> Option<A54Assign> {
    a54_assign_c_str(src)
}

/// `0x005ffaa0`: the same assignment with the member pointer at `+0x14`.
pub fn a54_assign_member_14(src: &[u8]) -> Option<A54Assign> {
    a54_assign_c_str(src)
}

/// Outcome of one small-block deallocation row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A54Free {
    /// Pointer the row hands to the deallocator (the block header pointer for
    /// a large block, the data pointer otherwise).
    pub block: usize,
    /// Byte count passed along with it (`count * stride`, `+ 0x23` for a
    /// large block).
    pub bytes: u32,
    /// `true` when the `0xfff` size test selected the block-header path.
    pub large: bool,
    /// `false` when the header-pointer distance check would have raised the
    /// invalid-parameter trap (`FUN_00c9474b`).
    pub valid: bool,
}

/// The shared body of `0x005fe030`, `0x005fe070` and `0x005fe0b0`.
///
/// `count * stride` is formed in 32 bits.  Below `0x1000` the data pointer is
/// released with that byte count.  Otherwise the block pointer is read from
/// `[ptr-4]` (passed in as `stored`), the count grows by `0x23`, and the
/// distance `(ptr - stored) - 4` must be at most `0x1f`.
fn a54_deallocate(ptr: usize, stored: usize, count: u32, stride: u32) -> A54Free {
    let mut bytes = count.wrapping_mul(stride);
    let mut block = ptr;
    let mut large = false;
    let mut valid = true;
    if bytes > A54_LARGE_BLOCK {
        large = true;
        block = stored;
        bytes = bytes.wrapping_add(A54_BLOCK_OVERHEAD);
        let distance = ptr.wrapping_sub(block).wrapping_sub(4);
        if distance > A54_ALIGN_SLACK as usize {
            valid = false;
        }
    }
    A54Free {
        block,
        bytes,
        large,
        valid,
    }
}

/// `0x005fe030`: deallocate an array of four-byte elements.
pub fn a54_deallocate_stride_4(ptr: usize, stored: usize, count: u32) -> A54Free {
    a54_deallocate(ptr, stored, count, 4)
}

/// `0x005fe070`: deallocate an array of 24-byte elements.
pub fn a54_deallocate_stride_18(ptr: usize, stored: usize, count: u32) -> A54Free {
    a54_deallocate(ptr, stored, count, 0x18)
}

/// `0x005fe0b0`: deallocate an array of 52-byte elements.
pub fn a54_deallocate_stride_34(ptr: usize, stored: usize, count: u32) -> A54Free {
    a54_deallocate(ptr, stored, count, 0x34)
}

/// One two-dword iterator value (`_Ptr` at `+0x0`, owner/container at `+0x4`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A54IterPair {
    /// Retail `*param_1`.
    pub node: u32,
    /// Retail `param_1[1]`, written from the calling object (`in_ECX`).
    pub owner: u32,
}

/// `0x005fe0f0`: build an iterator from a node and the owning object.
///
/// The two stores are `*param_1 = param_2` and `param_1[1] = in_ECX`.
pub fn a54_make_iter(node: u32, owner: u32) -> A54IterPair {
    A54IterPair { node, owner }
}

/// `0x005fe110`: the byte at `+0xc` of the calling object.
pub fn a54_byte_at_0c(object: &[u8]) -> u8 {
    object[0x0c]
}

/// `0x005ff9b0`: the byte at `+0xd` of the calling object.
pub fn a54_byte_at_0d(object: &[u8]) -> u8 {
    object[0x0d]
}

/// `ios_base` state recovered from the `0x005fff70` field stores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A54Ios {
    /// `+0x8`: `_Mystate`.
    pub state: u32,
    /// `+0xc`: `_Except`.
    pub exceptions: u32,
    /// `+0x10`: widening flag.
    pub wide: u32,
    /// `+0x14`: `_Fmtflags`.
    pub flags: u32,
    /// `+0x18`: precision as the 64-bit `streamsize` the rows read.
    pub precision: i64,
    /// `+0x20`: width as the 64-bit `streamsize` `FUN_005ff7a0` subtracts.
    pub width: i64,
    /// `+0x28`, `+0x2c`, `+0x30`: the three trailing zeroed dwords.
    pub spare: [u32; 3],
}

/// `0x005fff70`: initialise the stream state object.
///
/// The body stores zero to `+0x30`, `+0x8`, `+0x10`, `+0x18`..`+0x2c` and
/// `+0xc`, sets the flags to `0x201` (`skipws | dec`), the precision to `6`,
/// and then allocates eight bytes (`FUN_00923df0(8)`).  Returns the state and
/// the size of that allocation.
pub fn a54_ios_init() -> (A54Ios, usize) {
    (
        A54Ios {
            state: A54_GOODBIT,
            exceptions: A54_GOODBIT,
            wide: 0,
            flags: A54_DEFAULT_FLAGS,
            precision: A54_DEFAULT_PRECISION,
            width: 0,
            spare: [0; 3],
        },
        A54_IOS_INIT_ALLOCATION,
    )
}

/// The `sign` and magnitude one `strto*` scan recovered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct A54Scan {
    /// `'+'` or `'-'`; the retail normalises an absent sign to `'+'`.
    sign: u8,
    /// Radix actually used after the base-0/`0x` resolution.
    radix: u32,
    /// Accumulated magnitude, wrapping at the destination width.
    value: u64,
    /// Index one past the last converted character, `0` when nothing was.
    end: usize,
    /// `true` when at least one digit (including a leading `'0'`) was
    /// consumed after the optional sign and prefix.
    converted: bool,
    /// `true` when the scan would have set `errno = 0x22` (`ERANGE`).
    erange: bool,
}

/// C-locale `isspace`, the class the `strto*` rows skip
/// (`FUN_00c983ad` against the default table).
fn a54_is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Digit value of `c` in `radix`, or `None` when `c` is not a digit below the
/// radix (the `_Tolower` + `"0123456789abc..."` lookup of `FUN_00c84d10`).
fn a54_digit_value(c: u8, radix: u32) -> Option<u32> {
    let value = match c {
        b'0'..=b'9' => (c - b'0') as u32,
        b'a'..=b'z' => (c - b'a') as u32 + 10,
        b'A'..=b'Z' => (c - b'A') as u32 + 10,
        _ => return None,
    };
    if value < radix { Some(value) } else { None }
}

/// Digits of the largest value of `bits` width in `radix`.
///
/// This reproduces the per-radix table `FUN_00c84d10` indexes at
/// `0x00e4c008` (`FUN_00c84f51` uses `0x00e4c058` for 64 bits): 32 digits for
/// base 2, 11 for base 8, 10 for base 10 and 8 for base 16 at 32 bits.
fn a54_max_digits(radix: u32, bits: u32) -> usize {
    let max = if bits == 32 {
        u32::MAX as u64
    } else {
        u64::MAX
    };
    let mut digits = 0;
    let mut rest = max;
    while rest > 0 {
        rest /= radix as u64;
        digits += 1;
    }
    digits
}

/// The shared `strtoul`/`strtoull` scan: whitespace, sign, base resolution,
/// digit accumulation, and the digit-count/`ERANGE` decision.
fn a54_scan_unsigned(text: &[u8], base: i32, bits: u32) -> A54Scan {
    let mut pos = 0;
    while pos < text.len() && a54_is_space(text[pos]) {
        pos += 1;
    }
    let mut sign = b'+';
    if pos < text.len() && (text[pos] == b'+' || text[pos] == b'-') {
        sign = text[pos];
        pos += 1;
    }
    let mut scan = A54Scan {
        sign,
        radix: 0,
        value: 0,
        end: 0,
        converted: false,
        erange: false,
    };
    if base < 0 || base == 1 || base > 0x24 {
        return scan;
    }
    let mut radix = base as u32;
    if radix == 0 {
        if pos < text.len() && text[pos] == b'0' {
            if pos + 1 < text.len() && (text[pos + 1] | 0x20) == b'x' {
                radix = 16;
                pos += 2;
            } else {
                radix = 8;
            }
        } else {
            radix = 10;
        }
    } else if radix == 16
        && pos + 1 < text.len()
        && text[pos] == b'0'
        && (text[pos + 1] | 0x20) == b'x'
    {
        pos += 2;
    }
    scan.radix = radix;
    let prefix_end = pos;
    while pos < text.len() && text[pos] == b'0' {
        pos += 1;
    }
    let digit_start = pos;
    let width_mask = if bits == 32 {
        u64::from(u32::MAX)
    } else {
        u64::MAX
    };
    let mut value = 0u64;
    let mut previous = 0u64;
    let mut last_digit = 0u64;
    while pos < text.len() {
        let Some(digit) = a54_digit_value(text[pos], radix) else {
            break;
        };
        previous = value;
        last_digit = digit as u64;
        value = value.wrapping_mul(radix as u64).wrapping_add(digit as u64) & width_mask;
        pos += 1;
    }
    if pos == prefix_end {
        return scan;
    }
    scan.converted = true;
    scan.end = pos;
    scan.value = value;
    let digits = pos - digit_start;
    let max_digits = a54_max_digits(radix, bits);
    if digits > max_digits {
        scan.erange = true;
    } else if digits == max_digits {
        let restored = value.wrapping_sub(last_digit);
        if value < restored || restored / radix as u64 != previous {
            scan.erange = true;
        }
    }
    scan
}

/// `FUN_00c84d10` (`_Stoul`): unsigned scan with the `ERANGE` clamp.
///
/// Returns the value, the end index (`0` when nothing was converted) and the
/// `errno` flag.  A negative field wraps, as the retail body negates the
/// clamped magnitude at the end.
pub fn a54_strtoul(text: &[u8], base: i32) -> (u32, usize, bool) {
    let scan = a54_scan_unsigned(text, base, 32);
    if !scan.converted {
        return (0, 0, false);
    }
    let mut value = if scan.erange {
        u32::MAX
    } else {
        scan.value as u32
    };
    if scan.sign == b'-' {
        value = value.wrapping_neg();
    }
    (value, scan.end, scan.erange)
}

/// `FUN_00c84c6b` (`_Stol`): signed 32-bit scan.
///
/// The body calls `_Stoul` on the text after the sign and then applies the
/// recovered acceptance test: a magnitude of `0x80000000` or more is only kept
/// for a negative field, and the failure path returns `LONG_MAX`/`LONG_MIN`
/// with `errno` set.
pub fn a54_strtol(text: &[u8], base: i32) -> (i32, usize, bool) {
    let scan = a54_scan_unsigned(text, base, 32);
    if !scan.converted {
        return (0, 0, false);
    }
    let magnitude = if scan.erange {
        u32::MAX
    } else {
        scan.value as u32
    };
    if scan.sign == b'-' || magnitude < 0x8000_0000 {
        if scan.sign != b'-' {
            return (magnitude as i32, scan.end, scan.erange);
        }
        if magnitude < 0x8000_0001 {
            return ((magnitude as i32).wrapping_neg(), scan.end, scan.erange);
        }
    }
    (
        if scan.sign == b'-' {
            i32::MIN
        } else {
            i32::MAX
        },
        scan.end,
        true,
    )
}

/// `FUN_00c84f51` (`_Stoull`): unsigned 64-bit scan with the `ERANGE` clamp.
pub fn a54_strtoull(text: &[u8], base: i32) -> (u64, usize, bool) {
    let scan = a54_scan_unsigned(text, base, 64);
    if !scan.converted {
        return (0, 0, false);
    }
    let mut value = if scan.erange { u64::MAX } else { scan.value };
    if scan.sign == b'-' {
        value = value.wrapping_neg();
    }
    (value, scan.end, scan.erange)
}

/// `FUN_00c84e83` (`_Stoll`): signed 64-bit scan.
///
/// The cached body prints the final returns as 32-bit casts, but the callers
/// store eight bytes and the `_Stoll` contract is 64-bit, so the full value is
/// carried here.
pub fn a54_strtoll(text: &[u8], base: i32) -> (i64, usize, bool) {
    let scan = a54_scan_unsigned(text, base, 64);
    if !scan.converted {
        return (0, 0, false);
    }
    let magnitude = if scan.erange { u64::MAX } else { scan.value };
    if scan.sign == b'-' || magnitude < 0x8000_0000_0000_0000 {
        if scan.sign != b'-' {
            return (magnitude as i64, scan.end, scan.erange);
        }
        if magnitude < 0x8000_0000_0000_0001 {
            return ((magnitude as i64).wrapping_neg(), scan.end, scan.erange);
        }
    }
    (
        if scan.sign == b'-' {
            i64::MIN
        } else {
            i64::MAX
        },
        scan.end,
        true,
    )
}

/// `0x005fe160`: `num_get` row for a two-byte signed field.
///
/// The scanned text keeps the sign (`_Getifld` writes it first), the body
/// strips a leading `'-'`, scans the rest with `_Stoul`, checks the magnitude
/// against `0xffff`, truncates to 16 bits and negates when the sign was
/// negative.  A failed scan stores `-1` and sets `failbit`; an exhausted
/// input range adds `eofbit`.
pub fn a54_num_get_short(field: &[u8], base: i32, at_end: bool) -> (i16, u32) {
    let mut value: i16 = 0;
    let mut state = A54_GOODBIT;
    if field.is_empty() {
        state = A54_FAILBIT;
    } else {
        let negative = field[0] == b'-';
        let digits = if negative { &field[1..] } else { field };
        let (magnitude, end, erange) = a54_strtoul(digits, base);
        value = magnitude as i16;
        if end == 0 || erange || magnitude > 0xffff {
            state = A54_FAILBIT;
            value = -1;
        } else if negative {
            value = value.wrapping_neg();
        }
    }
    if at_end {
        state |= A54_EOFBIT;
    }
    (value, state)
}

/// `0x005fe2b0`: four-byte unsigned `num_get` row.
///
/// The body scans with `_Stoul`, stores the converted value even when the scan
/// failed (`endptr == buffer` or `ERANGE` only raise `failbit`), and leaves
/// `0` for an empty field.
pub fn a54_num_get_u32(field: &[u8], base: i32, at_end: bool) -> (u32, u32) {
    let mut value = 0;
    let mut state = A54_GOODBIT;
    if field.is_empty() {
        state = A54_FAILBIT;
    } else {
        let (magnitude, end, erange) = a54_strtoul(field, base);
        value = magnitude;
        if end == 0 || erange {
            state = A54_FAILBIT;
        }
    }
    if at_end {
        state |= A54_EOFBIT;
    }
    (value, state)
}

/// `0x005fe3f0`: four-byte signed `num_get` row.
///
/// The body scans with `_Stol`; a failed scan keeps whatever the clamp
/// returned and raises `failbit`.
pub fn a54_num_get_i32(field: &[u8], base: i32, at_end: bool) -> (i32, u32) {
    let mut value = 0;
    let mut state = A54_GOODBIT;
    if field.is_empty() {
        state = A54_FAILBIT;
    } else {
        let (parsed, end, erange) = a54_strtol(field, base);
        value = parsed;
        if end == 0 || erange {
            state = A54_FAILBIT;
        }
    }
    if at_end {
        state |= A54_EOFBIT;
    }
    (value, state)
}

/// `0x005fe520`: four-byte unsigned `num_get` row scanned with `_Stoul`.
///
/// The body is the same statement sequence as `0x005fe2b0` (only the virtual
/// overload differs), so both rows return the same result: the clamped value
/// is stored and `failbit` reports the failure.
pub fn a54_num_get_u32_keep(field: &[u8], base: i32, at_end: bool) -> (u32, u32) {
    let mut value = 0;
    let mut state = A54_GOODBIT;
    if field.is_empty() {
        state = A54_FAILBIT;
    } else {
        let (magnitude, end, erange) = a54_strtoul(field, base);
        value = magnitude;
        if end == 0 || erange {
            state = A54_FAILBIT;
        }
    }
    if at_end {
        state |= A54_EOFBIT;
    }
    (value, state)
}

/// `0x005feb10`: four-byte unsigned `num_get` row that clears on failure.
///
/// The body stores the `_Stoul` result, then overwrites both the value (`0`)
/// and `failbit` when the scan converted nothing or raised `ERANGE`.
pub fn a54_num_get_u32_reset(field: &[u8], base: i32, at_end: bool) -> (u32, u32) {
    let mut value = 0;
    let mut state = A54_GOODBIT;
    let mut ok = false;
    if !field.is_empty() {
        let (magnitude, end, erange) = a54_strtoul(field, base);
        value = magnitude;
        ok = end != 0 && !erange;
    }
    if !ok {
        state = A54_FAILBIT;
        value = 0;
    }
    if at_end {
        state |= A54_EOFBIT;
    }
    (value, state)
}

/// `0x005fec30`: eight-byte signed `num_get` row.
pub fn a54_num_get_i64(field: &[u8], base: i32, at_end: bool) -> (i64, u32) {
    let mut value = 0;
    let mut state = A54_GOODBIT;
    if field.is_empty() {
        state = A54_FAILBIT;
    } else {
        let (parsed, end, erange) = a54_strtoll(field, base);
        value = parsed;
        if end == 0 || erange {
            state = A54_FAILBIT;
        }
    }
    if at_end {
        state |= A54_EOFBIT;
    }
    (value, state)
}

/// `0x005fed70`: eight-byte unsigned `num_get` row.
pub fn a54_num_get_u64(field: &[u8], base: i32, at_end: bool) -> (u64, u32) {
    let mut value = 0;
    let mut state = A54_GOODBIT;
    if field.is_empty() {
        state = A54_FAILBIT;
    } else {
        let (magnitude, end, erange) = a54_strtoull(field, base);
        value = magnitude;
        if end == 0 || erange {
            state = A54_FAILBIT;
        }
    }
    if at_end {
        state |= A54_EOFBIT;
    }
    (value, state)
}

/// Outcome of the CRT decimal scan (`_Stof` `FUN_00c980b9`) of the temporary
/// field buffer.  The scanner is outside the window, so its result is an
/// input of the row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct A54ScanF32 {
    /// `true` when the field buffer's first byte is non-zero.
    pub present: bool,
    /// `true` when the scan converted at least one character.
    pub converted: bool,
    /// `true` when the scan set `errno` (`ERANGE`).
    pub erange: bool,
    /// The converted single-precision value.
    pub value: f32,
}

/// The same scan result for `_Stod` (`FUN_00c980a2`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct A54ScanF64 {
    /// `true` when the field buffer's first byte is non-zero.
    pub present: bool,
    /// `true` when the scan converted at least one character.
    pub converted: bool,
    /// `true` when the scan set `errno` (`ERANGE`).
    pub erange: bool,
    /// The converted double-precision value.
    pub value: f64,
}

/// `10.0^n`, the form the float rows reach the CRT `pow` (`FUN_00c9b940`) with.
fn a54_pow10(exponent: i32) -> f64 {
    10f64.powi(exponent)
}

/// `ldexp(x, n)`, the scale applied by `FUN_00c94924`.
fn a54_ldexp_f64(value: f64, exponent: i32) -> f64 {
    value * 2f64.powi(exponent)
}

/// `0x005fe650`: `num_get` row for a four-byte float field.
///
/// The body parses the field with `_Stof`, multiplies by `10^N` when the
/// scanner returned a non-zero `N` (the exact `pow` call of the retail body),
/// and, while the precision limit still holds a value other than the
/// `1000000000` sentinel, rescales the product by `2^(limit << 2)` with
/// `ldexp`.  An empty field, an unconverted scan or `ERANGE` stores `0.0` and
/// `failbit`.
pub fn a54_num_get_float(
    scan: A54ScanF32,
    scanner_exponent: i32,
    limit: i32,
    at_end: bool,
) -> (f32, u32) {
    let mut value = 0.0f32;
    let mut state = A54_GOODBIT;
    if scan.present {
        let mut parsed = scan.value;
        if scanner_exponent != 0 {
            parsed = (a54_pow10(scanner_exponent) as f32) * parsed;
        }
        value = parsed;
        if scan.converted && !scan.erange {
            if limit != A54_NO_PRECISION_LIMIT && limit != 0 {
                let exponent = limit.wrapping_shl(A54_LIMIT_SHIFT);
                value = a54_ldexp_f64(parsed as f64, exponent) as f32;
            }
        } else {
            state = A54_FAILBIT;
            value = 0.0;
        }
    } else {
        state = A54_FAILBIT;
    }
    if at_end {
        state |= A54_EOFBIT;
    }
    (value, state)
}

/// `0x005fe7d0`: `num_get` row for a double field with the precision limit.
///
/// Identical to `0x005fe650` except that the parse is `_Stod` and the product
/// stays in double precision.
pub fn a54_num_get_double(
    scan: A54ScanF64,
    scanner_exponent: i32,
    limit: i32,
    at_end: bool,
) -> (f64, u32) {
    let mut value = 0.0f64;
    let mut state = A54_GOODBIT;
    if scan.present {
        let mut parsed = scan.value;
        if scanner_exponent != 0 {
            parsed = a54_pow10(scanner_exponent) * parsed;
        }
        value = parsed;
        if scan.converted && !scan.erange {
            if limit != A54_NO_PRECISION_LIMIT && limit != 0 {
                value = a54_ldexp_f64(parsed, limit.wrapping_shl(A54_LIMIT_SHIFT));
            }
        } else {
            state = A54_FAILBIT;
            value = 0.0;
        }
    } else {
        state = A54_FAILBIT;
    }
    if at_end {
        state |= A54_EOFBIT;
    }
    (value, state)
}

/// `0x005fe940`: the double row without the precision-limit rescale.
///
/// This body never reads a limit, so the `ldexp` branch of
/// `a54_num_get_double` is absent; everything else matches.
pub fn a54_num_get_double_unlimited(
    scan: A54ScanF64,
    scanner_exponent: i32,
    at_end: bool,
) -> (f64, u32) {
    let mut value = 0.0f64;
    let mut state = A54_GOODBIT;
    if scan.present {
        let mut parsed = scan.value;
        if scanner_exponent != 0 {
            parsed = a54_pow10(scanner_exponent) * parsed;
        }
        value = parsed;
        if scan.converted && !scan.erange {
            // The retail body stores the parsed value and jumps to the eof
            // check, leaving it in place.
        } else {
            state = A54_FAILBIT;
            value = 0.0;
        }
    } else {
        state = A54_FAILBIT;
    }
    if at_end {
        state |= A54_EOFBIT;
    }
    (value, state)
}

/// `frexp` exponent of `value`, the side value `FUN_00c94828` writes.
///
/// Zero yields `0`, infinities and NaNs yield `-1` (`0xffffffff` in the
/// retail frame), normal values yield `exponent - 1022` and subnormals are
/// normalised by shifting the significand up.  The `FUN_00c94828` row itself is
/// outside this window and is not claimed here; only the exponent the
/// `fixed` branch consumes is reproduced.
fn a54_frexp_exponent(value: f64) -> i32 {
    if !value.is_finite() {
        return -1;
    }
    if value == 0.0 {
        return 0;
    }
    let bits = value.abs().to_bits();
    let biased = ((bits >> 52) & 0x7ff) as i32;
    if biased != 0 {
        return biased - 1022;
    }
    let mut significand = bits & 0x000f_ffff_ffff_ffff;
    let mut exponent = -1021;
    while significand & 0x0010_0000_0000_0000 == 0 {
        significand <<= 1;
        exponent -= 1;
    }
    exponent
}

/// A `printf` float specification, the output of the `_Ffmt` builder the two
/// float `num_put` rows call.
///
/// The bytes are the exact ones the builder writes: `%`, `+` when `showpos`,
/// `#` when `showpoint`, `.*`, the optional length specifier, and the
/// conversion letter recovered from the `floatfield` bits (`f` for `fixed`,
/// `a`/`A` for `hexfloat`, `e`/`E` for `scientific`, `g`/`G` otherwise).
fn a54_float_format(flags: u32, spec: u8) -> [u8; 8] {
    let mut format = [0u8; 8];
    let mut len = 0;
    format[len] = b'%';
    len += 1;
    if flags & A54_SHOWPOS != 0 {
        format[len] = b'+';
        len += 1;
    }
    if flags & A54_SHOWPOINT != 0 {
        format[len] = b'#';
        len += 1;
    }
    format[len] = b'.';
    format[len + 1] = b'*';
    len += 2;
    if spec != 0 {
        format[len] = spec;
        len += 1;
    }
    let field = flags & A54_FLOATFIELD;
    let uppercase = flags & A54_UPPERCASE != 0;
    format[len] = if field == A54_HEXFLOAT {
        if uppercase { b'A' } else { b'a' }
    } else if field == A54_FIXED {
        b'f'
    } else if field == A54_SCIENTIFIC {
        if uppercase { b'E' } else { b'e' }
    } else if uppercase {
        b'G'
    } else {
        b'g'
    };
    format
}

/// The plan one float `num_put` row builds for its temporary string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A54FloatFieldPlan {
    /// Digits handed to `sprintf` as the `*` precision (`iVar6`).
    pub precision: u32,
    /// Size the temporary string is resized to (`precision + 0x32`, raised by
    /// the `fixed` headroom for magnitudes above `1e10`).
    pub size: u32,
    /// The `printf` specification bytes.
    pub format: [u8; 8],
}

/// The shared body of `0x005ff300` and `0x005ff4b0`.
///
/// The default-precision rule is the exact recovered test: when the 64-bit
/// precision is not positive (high dword below one and either negative or a
/// zero low dword) and the field is not `fixed`, the low dword becomes `6`.
/// For `fixed` fields above `1e10` the body calls `frexp`, multiplies the
/// absolute exponent by `0x7597` (`30103`), divides by `100000` and adds the
/// result to the precision, only to allocate that much headroom: the
/// `sprintf` precision stays the unadjusted value.
fn a54_float_field_plan(ios: &A54Ios, value: f64, spec: u8) -> A54FloatFieldPlan {
    let field = ios.flags & A54_FLOATFIELD;
    let precision_high = (ios.precision >> 32) as i32;
    let mut precision = ios.precision as u32;
    if precision_high < 1 && (precision_high < 0 || precision == 0) && field != A54_FIXED {
        precision = 6;
    }
    let mut size = precision.wrapping_add(0x32);
    if field == A54_FIXED && value.abs() > 10_000_000_000.0 {
        let exponent = a54_frexp_exponent(value).wrapping_abs();
        let headroom = exponent.wrapping_mul(0x7597) / 100_000;
        let adjusted = headroom.wrapping_add(precision as i32);
        size = (adjusted as u32).wrapping_add(0x32);
    }
    A54FloatFieldPlan {
        precision,
        size,
        format: a54_float_format(ios.flags, spec),
    }
}

/// `0x005ff300`: `num_put` row for a `double` field.
///
/// The value arrives in `param_6`; the row sizes the temporary string, builds
/// `%.*g`-style specifications and calls `sprintf` (`FUN_00603450`) with the
/// unadjusted precision before the field is handed to `_Put` (`FUN_005fa2c0`).
/// Both callees are outside the window, so this row reports the plan they
/// receive.
pub fn a54_num_put_double(ios: &A54Ios, value: f64) -> A54FloatFieldPlan {
    a54_float_field_plan(ios, value, 0)
}

/// `0x005ff4b0`: the `long double` twin of `0x005ff300`.
///
/// The decompiled body reads the eight-byte slice of the 80-bit argument at
/// its stack slot, so the same precision arithmetic is reproduced on that
/// slice; the length specifier `'L'` the row passes to the format builder is
/// carried into the returned specification.
pub fn a54_num_put_long_double(ios: &A54Ios, value: f64) -> A54FloatFieldPlan {
    a54_float_field_plan(ios, value, b'L')
}

/// `0x005ff9c0`: `ctype` override mapping one character with `tolower`.
///
/// The retail body calls the table-driven `FUN_00c8427f`; the recovered classic
/// branch maps `A`..`Z` to `+0x20` and leaves every other byte alone.
pub fn a54_tolower_char(c: u8) -> u8 {
    if (0x41..=0x5a).contains(&c) {
        c + 0x20
    } else {
        c
    }
}

/// `0x005ff9e0`: `tolower` over the range `[first, last)` in place.
///
/// A reversed range (`last < first`) converts nothing, exactly as the body
/// clamps the count to zero.  Returns the advanced first index.
pub fn a54_tolower_range(buf: &mut [u8], first: usize, last: usize) -> usize {
    let count = if last < first { 0 } else { last - first };
    for index in 0..count {
        let at = first + index;
        buf[at] = a54_tolower_char(buf[at]);
    }
    first + count
}

/// `0x005ffa30`: `ctype` override mapping one character with `toupper`.
pub fn a54_toupper_char(c: u8) -> u8 {
    if (0x61..=0x7a).contains(&c) {
        c - 0x20
    } else {
        c
    }
}

/// `0x005ffa50`: `toupper` over the range `[first, last)` in place.
pub fn a54_toupper_range(buf: &mut [u8], first: usize, last: usize) -> usize {
    let count = if last < first { 0 } else { last - first };
    for index in 0..count {
        let at = first + index;
        buf[at] = a54_toupper_char(buf[at]);
    }
    first + count
}

/// `0x005ff200`: `char_traits::copy` of the range `[first, last)`.
///
/// The body is `memcpy(dst, first, last - first)`; `src` is that range, the
/// caller guarantees `dst` holds at least `src.len()` bytes, and the return
/// value is the byte count moved.
pub fn a54_copy_range(dst: &mut [u8], src: &[u8]) -> usize {
    let count = src.len();
    dst[..count].copy_from_slice(src);
    count
}

/// FNV-1a 32-bit over `bytes` (`0x005ffc10` hashes the node's string key with
/// this loop; `0x005ffb90` performs the single step of the same recurrence).
pub fn a54_fnv1a32(bytes: &[u8]) -> u32 {
    let mut hash = A54_HASH_BASIS;
    for byte in bytes {
        hash = (u32::from(*byte) ^ hash).wrapping_mul(A54_HASH_PRIME);
    }
    hash
}

/// One `{first, last}` `_Hash` bucket entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A54Bucket {
    /// `+0x0`: first node of the bucket.
    pub first: usize,
    /// `+0x4`: last node of the bucket.
    pub last: usize,
}

/// One 12-byte `_Hash` node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A54HashNode {
    /// `+0x0`: next node in the iterator order.
    pub next: usize,
    /// `+0x4`: previous node.
    pub prev: usize,
    /// `+0x8`: key bytes (a one-byte key for `0x005ffb90`, the `std::string`
    /// payload for `0x005ffc10`).
    pub key: Vec<u8>,
}

/// The `_Hash` header fields the unlink rows touch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A54Hash {
    /// `+0x4`: end node every emptied bucket points at.
    pub end_node: usize,
    /// `+0x8`: element count.
    pub len: u32,
    /// `+0xc`: bucket array selected by `hash & mask`.
    pub buckets: Vec<A54Bucket>,
    /// `+0x18`: bucket mask.
    pub mask: u32,
    /// Model storage for the intrusive nodes (the retail allocator is not
    /// reproduced).
    pub nodes: Vec<A54HashNode>,
}

/// Bucket selected by `hash & mask` (`(mask & hash) * 8` in the retail rows).
pub fn a54_bucket_index(hash: u32, mask: u32) -> usize {
    (hash & mask) as usize
}

/// Shared unlink of the intrusive node `node` from bucket `bucket`.
fn a54_hash_unlink(table: &mut A54Hash, node: usize, bucket: usize) {
    let next = table.nodes[node].next;
    let prev = table.nodes[node].prev;
    if table.buckets[bucket].last == node {
        if table.buckets[bucket].first == node {
            table.buckets[bucket].first = table.end_node;
            table.buckets[bucket].last = table.end_node;
        } else {
            table.buckets[bucket].last = prev;
        }
    } else if table.buckets[bucket].first == node {
        table.buckets[bucket].first = next;
    }
    table.nodes[prev].next = next;
    table.nodes[next].prev = prev;
    table.len = table.len.wrapping_sub(1);
}

/// `0x005ffb90`: erase a node whose key is the single byte at `+0x8`.
///
/// The bucket is `((key ^ 0x811c9dc5) * 0x1000193) & mask`, the single FNV
/// step the body performs inline.  The four branch cases of the retail unlink
/// (sole node, tail, head, middle) and the count decrement are reproduced.
pub fn a54_hash_erase_byte_key(table: &mut A54Hash, node: usize) {
    let key = table.nodes[node].key[0];
    let hash = (u32::from(key) ^ A54_HASH_BASIS).wrapping_mul(A54_HASH_PRIME);
    let bucket = a54_bucket_index(hash, table.mask);
    a54_hash_unlink(table, node, bucket);
}

/// `0x005ffc10`: erase a node whose key is the `std::string` at `+0x8`.
///
/// The FNV-1a loop runs over the first `size` bytes of the key
/// (`capacity > 0xf` means the payload lives behind the header's first dword,
/// which the model stores directly in [`A54HashNode::key`]).  Returns the node
/// the retail writes back as the next iterator.
pub fn a54_hash_erase_string_key(table: &mut A54Hash, node: usize) -> usize {
    let hash = a54_fnv1a32(&table.nodes[node].key);
    let bucket = a54_bucket_index(hash, table.mask);
    let next = table.nodes[node].next;
    a54_hash_unlink(table, node, bucket);
    next
}

/// `0x005ffd90`: `std::string::find(const string&, pos)`.
///
/// The body rejects a needle longer than the haystack or a `pos` beyond
/// `len - needle.len()`, returns `pos` for an empty needle and otherwise
/// scans with `memchr` for the first byte and compares the candidate with the
/// remaining needle bytes.  `u32::MAX` is `npos`.
pub fn a54_string_find(haystack: &[u8], needle: &[u8], pos: u32) -> u32 {
    if haystack.len() < needle.len() {
        return u32::MAX;
    }
    let span = haystack.len() - needle.len();
    if span < pos as usize {
        return u32::MAX;
    }
    if needle.is_empty() {
        return pos;
    }
    for start in pos as usize..=span {
        if haystack[start..start + needle.len()] == *needle {
            return start as u32;
        }
    }
    u32::MAX
}

#[cfg(test)]
mod tests_agent54 {
    use super::*;

    fn ios_with(flags: u32, precision: i64) -> A54Ios {
        let (mut ios, _) = a54_ios_init();
        ios.flags = flags;
        ios.precision = precision;
        ios
    }

    fn format_bytes(plan: &A54FloatFieldPlan) -> &[u8] {
        let end = plan
            .format
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(plan.format.len());
        &plan.format[..end]
    }

    fn hash_table() -> A54Hash {
        A54Hash {
            end_node: 0,
            len: 3,
            buckets: vec![
                A54Bucket { first: 1, last: 3 },
                A54Bucket { first: 2, last: 2 },
            ],
            mask: 1,
            nodes: vec![
                A54HashNode {
                    next: 1,
                    prev: 3,
                    key: Vec::new(),
                },
                A54HashNode {
                    next: 2,
                    prev: 0,
                    key: b"a".to_vec(),
                },
                A54HashNode {
                    next: 3,
                    prev: 1,
                    key: b"b".to_vec(),
                },
                A54HashNode {
                    next: 0,
                    prev: 2,
                    key: b"c".to_vec(),
                },
            ],
        }
    }

    #[test]
    fn deallocate_stride_4_uses_the_data_pointer_below_one_page() {
        // 0x005fe030
        let freed = a54_deallocate_stride_4(0x1000, 0x0ffc, 0x3ff);
        assert_eq!(
            freed,
            A54Free {
                block: 0x1000,
                bytes: 0xffc,
                large: false,
                valid: true
            }
        );
    }

    #[test]
    fn deallocate_stride_4_reads_the_block_header_above_one_page() {
        // 0x005fe030: 0x400 * 4 = 0x1000 > 0xfff
        let freed = a54_deallocate_stride_4(0x1020, 0x1000, 0x400);
        assert_eq!(freed.bytes, 0x1023);
        assert_eq!(freed.block, 0x1000);
        assert!(freed.large);
        assert!(freed.valid, "gap 0x20 is inside the 4..=0x23 window");
        let bad = a54_deallocate_stride_4(0x1024, 0x1000, 0x400);
        assert!(!bad.valid, "gap 0x24 trips FUN_00c9474b");
    }

    #[test]
    fn deallocate_stride_18_and_34_scale_the_count() {
        // 0x005fe070, 0x005fe0b0
        assert_eq!(a54_deallocate_stride_18(0x2000, 0, 0x100).bytes, 0x1823);
        assert!(a54_deallocate_stride_18(0x2000, 0x1fe0, 0x100).valid);
        let wide = a54_deallocate_stride_34(0x3000, 0x2fe0, 0x100);
        assert_eq!(wide.bytes, 0x3400 + 0x23);
        assert!(wide.large);
        assert_eq!(a54_deallocate_stride_34(0x3000, 0x3000, 0).bytes, 0);
    }

    #[test]
    fn assignment_keeps_the_inline_capacity_for_short_sources() {
        // 0x005fe120 / 0x005ff1b0 / 0x005ffaa0
        let short = a54_assign_member_10(b"abc").unwrap();
        assert_eq!(
            short,
            A54Assign {
                len: 3,
                cap: A54_SSO_CAPACITY,
                heap: false,
                reallocated: false
            }
        );
        assert_eq!(a54_assign_member_08(b"abc").unwrap(), short);
        assert_eq!(a54_assign_member_14(b"abc").unwrap(), short);
    }

    #[test]
    fn assignment_grows_to_the_next_capacity_class() {
        // FUN_005fdb20: capacity = len | 0xf
        let grown = a54_assign_member_10(b"0123456789abcdef").unwrap();
        assert_eq!(grown.len, 16);
        assert_eq!(grown.cap, 16 | 0xf);
        assert!(grown.heap && grown.reallocated);
        let exactly_15 = a54_assign_member_10(b"0123456789abcde").unwrap();
        assert_eq!(exactly_15.cap, A54_SSO_CAPACITY);
        assert!(!exactly_15.reallocated);
        assert_eq!(
            a54_assign_member_10(b"").unwrap(),
            A54Assign {
                len: 0,
                cap: A54_SSO_CAPACITY,
                heap: false,
                reallocated: false
            }
        );
    }

    #[test]
    fn strtoul_handles_bases_prefixes_and_signs() {
        // FUN_00c84d10
        assert_eq!(a54_strtoul(b"123", 10), (123, 3, false));
        assert_eq!(a54_strtoul(b"  42x", 10), (42, 4, false));
        assert_eq!(a54_strtoul(b"0x1f", 0), (31, 4, false));
        assert_eq!(a54_strtoul(b"010", 0), (8, 3, false));
        assert_eq!(a54_strtoul(b"0", 0), (0, 1, false));
        assert_eq!(a54_strtoul(b"0x", 0), (0, 0, false));
        assert_eq!(a54_strtoul(b"zz", 10), (0, 0, false));
        assert_eq!(a54_strtoul(b"-5", 10), (u32::MAX - 4, 2, false));
        assert_eq!(a54_strtoul(b"77", 8), (63, 2, false));
        assert_eq!(a54_strtoul(b"8", 8), (0, 0, false));
    }

    #[test]
    fn strtoul_clamps_on_erange() {
        // FUN_00c84d10: 0x22 is stored and the result saturates
        assert_eq!(a54_strtoul(b"4294967295", 10), (u32::MAX, 10, false));
        assert_eq!(a54_strtoul(b"4294967296", 10), (u32::MAX, 10, true));
        assert_eq!(a54_strtoul(b"9999999999", 10), (u32::MAX, 10, true));
        assert_eq!(a54_strtoul(b"0x100000000", 16), (u32::MAX, 11, true));
    }

    #[test]
    fn strtol_applies_the_sign_and_the_long_clamps() {
        // FUN_00c84c6b
        assert_eq!(a54_strtol(b"-2147483648", 10), (i32::MIN, 11, false));
        assert_eq!(a54_strtol(b"2147483647", 10), (i32::MAX, 10, false));
        assert_eq!(a54_strtol(b"2147483648", 10), (i32::MAX, 10, true));
        assert_eq!(a54_strtol(b"-2147483649", 10), (i32::MIN, 11, true));
        assert_eq!(a54_strtol(b"-0", 10), (0, 2, false));
        assert_eq!(a54_strtol(b"+7", 10), (7, 2, false));
        assert_eq!(a54_strtol(b"junk", 10), (0, 0, false));
    }

    #[test]
    fn strtoull_and_strtoll_cover_the_sixty_four_bit_bounds() {
        // FUN_00c84f51, FUN_00c84e83
        assert_eq!(
            a54_strtoull(b"18446744073709551615", 10),
            (u64::MAX, 20, false)
        );
        assert_eq!(
            a54_strtoull(b"18446744073709551616", 10),
            (u64::MAX, 20, true)
        );
        assert_eq!(
            a54_strtoll(b"-9223372036854775808", 10),
            (i64::MIN, 20, false)
        );
        assert_eq!(
            a54_strtoll(b"9223372036854775807", 10),
            (i64::MAX, 19, false)
        );
        assert_eq!(
            a54_strtoll(b"9223372036854775808", 10),
            (i64::MAX, 19, true)
        );
        assert_eq!(
            a54_strtoll(b"-9223372036854775809", 10),
            (i64::MIN, 20, true)
        );
    }

    #[test]
    fn num_get_short_truncates_and_fails_out_of_range() {
        // 0x005fe160
        assert_eq!(a54_num_get_short(b"7", 10, false), (7, 0));
        assert_eq!(a54_num_get_short(b"-7", 10, false), (-7, 0));
        assert_eq!(a54_num_get_short(b"65535", 10, false), (-1, 0));
        assert_eq!(a54_num_get_short(b"65536", 10, false), (-1, A54_FAILBIT));
        assert_eq!(a54_num_get_short(b"", 10, false), (0, A54_FAILBIT));
        assert_eq!(a54_num_get_short(b"3", 10, true), (3, A54_EOFBIT));
        assert_eq!(a54_num_get_short(b"-1", 10, true), (-1, A54_EOFBIT));
    }

    #[test]
    fn num_get_u32_keeps_the_parsed_value_on_failure() {
        // 0x005fe2b0
        assert_eq!(a54_num_get_u32(b"42", 10, false), (42, 0));
        assert_eq!(
            a54_num_get_u32(b"-1", 10, false),
            (u32::MAX, 0),
            "a negative field wraps through _Stoul"
        );
        assert_eq!(
            a54_num_get_u32(b"4294967296", 10, false),
            (u32::MAX, A54_FAILBIT)
        );
        assert_eq!(a54_num_get_u32(b"", 10, false), (0, A54_FAILBIT));
        assert_eq!(a54_num_get_u32(b"7", 10, true), (7, A54_EOFBIT));
    }

    #[test]
    fn num_get_i32_and_u32_keep_report_the_clamped_value() {
        // 0x005fe3f0, 0x005fe520
        assert_eq!(a54_num_get_i32(b"-12", 10, false), (-12, 0));
        assert_eq!(
            a54_num_get_i32(b"2147483648", 10, false),
            (i32::MAX, A54_FAILBIT)
        );
        assert_eq!(a54_num_get_i32(b"", 10, false), (0, A54_FAILBIT));
        assert_eq!(a54_num_get_u32_keep(b"99", 10, false), (99, 0));
        assert_eq!(a54_num_get_u32_keep(b"-99", 10, false), (u32::MAX - 98, 0));
    }

    #[test]
    fn num_get_u32_reset_clears_the_value_on_failure() {
        // 0x005feb10
        assert_eq!(a54_num_get_u32_reset(b"17", 10, false), (17, 0));
        assert_eq!(
            a54_num_get_u32_reset(b"4294967296", 10, false),
            (0, A54_FAILBIT)
        );
        assert_eq!(a54_num_get_u32_reset(b"", 10, false), (0, A54_FAILBIT));
        assert_eq!(a54_num_get_u32_reset(b"17", 10, true), (17, A54_EOFBIT));
    }

    #[test]
    fn num_get_i64_and_u64_store_eight_bytes() {
        // 0x005fec30, 0x005fed70
        assert_eq!(
            a54_num_get_i64(b"-9000000000", 10, false),
            (-9_000_000_000, 0)
        );
        assert_eq!(
            a54_num_get_i64(b"99999999999999999999", 10, false),
            (i64::MAX, A54_FAILBIT)
        );
        assert_eq!(
            a54_num_get_u64(b"18000000000", 10, false),
            (18_000_000_000, 0)
        );
        assert_eq!(
            a54_num_get_u64(b"18446744073709551616", 10, true),
            (u64::MAX, A54_FAILBIT | A54_EOFBIT)
        );
    }

    #[test]
    fn num_get_float_applies_the_decimal_and_limit_scales() {
        // 0x005fe650
        let scan = A54ScanF32 {
            present: true,
            converted: true,
            erange: false,
            value: 1.5,
        };
        assert_eq!(
            a54_num_get_float(scan, 0, A54_NO_PRECISION_LIMIT, false),
            (1.5, 0)
        );
        assert_eq!(
            a54_num_get_float(scan, 2, A54_NO_PRECISION_LIMIT, false),
            (150.0, 0)
        );
        assert_eq!(
            a54_num_get_float(scan, 2, 1, false),
            (2400.0, 0),
            "ldexp(150, 1 << 2) = 2400"
        );
        let failed = A54ScanF32 {
            present: true,
            converted: true,
            erange: true,
            value: 1.5,
        };
        assert_eq!(
            a54_num_get_float(failed, 0, A54_NO_PRECISION_LIMIT, false),
            (0.0, A54_FAILBIT)
        );
        let absent = A54ScanF32 {
            present: false,
            converted: false,
            erange: false,
            value: 0.0,
        };
        assert_eq!(
            a54_num_get_float(absent, 0, A54_NO_PRECISION_LIMIT, true),
            (0.0, A54_FAILBIT | A54_EOFBIT)
        );
    }

    #[test]
    fn num_get_double_scales_and_the_unlimited_twin_does_not() {
        // 0x005fe7d0, 0x005fe940
        let scan = A54ScanF64 {
            present: true,
            converted: true,
            erange: false,
            value: 2.5,
        };
        assert_eq!(
            a54_num_get_double(scan, 3, A54_NO_PRECISION_LIMIT, false),
            (2500.0, 0)
        );
        assert_eq!(
            a54_num_get_double(scan, 0, 1, false),
            (40.0, 0),
            "ldexp(2.5, 1 << 2) = 40"
        );
        assert_eq!(a54_num_get_double_unlimited(scan, 3, false), (2500.0, 0));
        let unconverted = A54ScanF64 {
            present: true,
            converted: false,
            erange: false,
            value: 2.5,
        };
        assert_eq!(
            a54_num_get_double_unlimited(unconverted, 0, false),
            (0.0, A54_FAILBIT)
        );
    }

    #[test]
    fn frexp_exponent_matches_the_recovered_side_value() {
        // FUN_00c94828
        assert_eq!(a54_frexp_exponent(0.0), 0);
        assert_eq!(a54_frexp_exponent(-0.0), 0);
        assert_eq!(a54_frexp_exponent(1.0), 1);
        assert_eq!(a54_frexp_exponent(0.5), 0);
        assert_eq!(a54_frexp_exponent(1e20), 67);
        assert_eq!(a54_frexp_exponent(f64::INFINITY), -1);
        assert_eq!(a54_frexp_exponent(f64::NAN), -1);
        assert_eq!(a54_frexp_exponent(f64::MIN_POSITIVE / 2.0), -1022);
    }

    #[test]
    fn ios_init_writes_the_recovered_field_values() {
        // 0x005fff70
        let (ios, allocation) = a54_ios_init();
        assert_eq!(allocation, 8);
        assert_eq!(ios.state, A54_GOODBIT);
        assert_eq!(ios.exceptions, A54_GOODBIT);
        assert_eq!(ios.wide, 0);
        assert_eq!(ios.flags, 0x201);
        assert_eq!(ios.flags, A54_SKIPWS | A54_DEC);
        assert_eq!(ios.precision, 6);
        assert_eq!(ios.width, 0);
        assert_eq!(ios.spare, [0, 0, 0]);
    }

    #[test]
    fn float_format_builder_orders_flags_specifier_and_conversion() {
        // 0x005ff300 / 0x005ff4b0 format builder
        assert_eq!(
            format_bytes(&a54_num_put_double(&ios_with(0x201, 6), 1.0)),
            b"%.*g"
        );
        assert_eq!(
            format_bytes(&a54_num_put_double(&ios_with(0x235, 6), 1.0)),
            b"%+#.*G"
        );
        assert_eq!(
            format_bytes(&a54_num_put_double(&ios_with(0x2000, 6), 1.0)),
            b"%.*f"
        );
        assert_eq!(
            format_bytes(&a54_num_put_double(&ios_with(0x3000, 6), 1.0)),
            b"%.*a"
        );
        assert_eq!(
            format_bytes(&a54_num_put_double(&ios_with(0x1004, 6), 1.0)),
            b"%.*E"
        );
        assert_eq!(
            format_bytes(&a54_num_put_long_double(&ios_with(0x2000, 6), 1.0)),
            b"%.*Lf"
        );
    }

    #[test]
    fn num_put_double_defaults_and_sizes_the_temporary_string() {
        // 0x005ff300
        let plan = a54_num_put_double(&ios_with(0x201, 6), 1.0);
        assert_eq!(plan.precision, 6);
        assert_eq!(plan.size, 6 + 0x32);
        let zero_precision = a54_num_put_double(&ios_with(0x201, 0), 1.0);
        assert_eq!(zero_precision.precision, 6, "default precision applies");
        assert_eq!(zero_precision.size, 0x38);
        let fixed_zero = a54_num_put_double(&ios_with(0x2000, 0), 1.0);
        assert_eq!(fixed_zero.precision, 0, "a fixed field keeps precision 0");
        assert_eq!(fixed_zero.size, 0x32);
    }

    #[test]
    fn num_put_double_grows_the_buffer_for_large_fixed_values() {
        // 0x005ff300: (|frexp exponent| * 30103) / 100000 + precision
        let large = a54_num_put_double(&ios_with(0x2000, 6), 1e20);
        assert_eq!(large.precision, 6);
        assert_eq!(large.size, 6 + 20 + 0x32);
        let boundary = a54_num_put_double(&ios_with(0x2000, 6), 1e10);
        assert_eq!(boundary.size, 6 + 0x32, "1e10 is not above the threshold");
        let huge = a54_num_put_double(&ios_with(0x2000, 6), -1e100);
        assert_eq!(huge.size, 6 + 100 + 0x32);
        let scientific = a54_num_put_double(&ios_with(0x1000, 6), 1e20);
        assert_eq!(scientific.size, 6 + 0x32);
    }

    #[test]
    fn ctype_overrides_map_ascii_letters_only() {
        // 0x005ff9c0, 0x005ffa30
        assert_eq!(a54_tolower_char(b'A'), b'a');
        assert_eq!(a54_tolower_char(b'a'), b'a');
        assert_eq!(a54_tolower_char(0x80), 0x80);
        assert_eq!(a54_toupper_char(b'a'), b'A');
        assert_eq!(a54_toupper_char(b'A'), b'A');
        assert_eq!(a54_toupper_char(b'0'), b'0');
    }

    #[test]
    fn ctype_range_rows_convert_in_place_and_clamp_reversed_ranges() {
        // 0x005ff9e0, 0x005ffa50
        let mut lower = *b"AbC";
        assert_eq!(a54_tolower_range(&mut lower, 0, 3), 3);
        assert_eq!(&lower, b"abc");
        let mut upper = *b"xYz";
        assert_eq!(a54_toupper_range(&mut upper, 0, 3), 3);
        assert_eq!(&upper, b"XYZ");
        let mut untouched = *b"Ab";
        assert_eq!(a54_tolower_range(&mut untouched, 2, 1), 2);
        assert_eq!(&untouched, b"Ab");
        let mut tail = *b"aAbB";
        assert_eq!(a54_tolower_range(&mut tail, 2, 4), 4);
        assert_eq!(&tail, b"aAbb");
    }

    #[test]
    fn copy_range_moves_the_source_prefix_and_reports_the_count() {
        // 0x005ff200
        let mut dst = [0u8; 4];
        assert_eq!(a54_copy_range(&mut dst, b"xy"), 2);
        assert_eq!(&dst, b"xy\0\0");
        let mut exact = [0u8; 3];
        assert_eq!(a54_copy_range(&mut exact, b"abc"), 3);
        assert_eq!(&exact, b"abc");
    }

    #[test]
    fn iterator_and_byte_accessors_read_the_recovered_offsets() {
        // 0x005fe0f0, 0x005fe110, 0x005ff9b0
        assert_eq!(
            a54_make_iter(0x1234, 0x5678),
            A54IterPair {
                node: 0x1234,
                owner: 0x5678
            }
        );
        let mut object = [0u8; 0x10];
        object[0x0c] = 0xaa;
        object[0x0d] = 0xbb;
        assert_eq!(a54_byte_at_0c(&object), 0xaa);
        assert_eq!(a54_byte_at_0d(&object), 0xbb);
    }

    #[test]
    fn fnv1a32_matches_the_published_vectors() {
        // 0x005ffc10 / 0x005ffb90 key hash
        assert_eq!(a54_fnv1a32(b""), A54_HASH_BASIS);
        assert_eq!(a54_fnv1a32(b"a"), 0xe40c_292c);
        assert_eq!(a54_fnv1a32(b"b"), 0xe70c_2de5);
        assert_eq!(a54_fnv1a32(b"c"), 0xe60c_2c52);
        assert_eq!(a54_fnv1a32(b"foobar"), 0xbf9c_f968);
        assert_eq!(
            a54_fnv1a32(b"a"),
            (u32::from(b'a') ^ A54_HASH_BASIS).wrapping_mul(A54_HASH_PRIME)
        );
        assert_eq!(a54_bucket_index(0xe40c_292c, 1), 0);
    }

    #[test]
    fn hash_erase_byte_key_resets_a_sole_bucket_member() {
        // 0x005ffb90
        let mut table = hash_table();
        a54_hash_erase_byte_key(&mut table, 2);
        assert_eq!(table.len, 2);
        assert_eq!(
            table.buckets[1],
            A54Bucket {
                first: table.end_node,
                last: table.end_node
            }
        );
        assert_eq!(table.nodes[1].next, 3);
        assert_eq!(table.nodes[3].prev, 1);
    }

    #[test]
    fn hash_erase_string_key_relinks_the_bucket_tail_and_returns_next() {
        // 0x005ffc10
        let mut table = hash_table();
        let next = a54_hash_erase_string_key(&mut table, 3);
        assert_eq!(next, 0);
        assert_eq!(table.len, 2);
        assert_eq!(table.buckets[0], A54Bucket { first: 1, last: 2 });
        assert_eq!(table.nodes[2].next, 0);
        assert_eq!(table.nodes[0].prev, 2);
        assert_eq!(a54_fnv1a32(b"c") & 1, 0);
    }

    #[test]
    fn hash_erase_covers_the_head_and_middle_branches() {
        // 0x005ffb90: bucket-head and non-tail cases
        let mut table = A54Hash {
            end_node: 0,
            len: 4,
            buckets: vec![A54Bucket { first: 1, last: 4 }],
            mask: 0,
            nodes: vec![
                A54HashNode {
                    next: 1,
                    prev: 4,
                    key: Vec::new(),
                },
                A54HashNode {
                    next: 2,
                    prev: 0,
                    key: b"a".to_vec(),
                },
                A54HashNode {
                    next: 3,
                    prev: 1,
                    key: b"b".to_vec(),
                },
                A54HashNode {
                    next: 4,
                    prev: 2,
                    key: b"c".to_vec(),
                },
                A54HashNode {
                    next: 0,
                    prev: 3,
                    key: b"d".to_vec(),
                },
            ],
        };
        a54_hash_erase_byte_key(&mut table, 1);
        assert_eq!(table.buckets[0], A54Bucket { first: 2, last: 4 });
        assert_eq!(table.nodes[0].next, 2);
        assert_eq!(table.nodes[2].prev, 0);
        a54_hash_erase_byte_key(&mut table, 3);
        assert_eq!(
            table.buckets[0],
            A54Bucket { first: 2, last: 4 },
            "a middle node leaves both bucket ends alone"
        );
        assert_eq!(table.nodes[2].next, 4);
        assert_eq!(table.nodes[4].prev, 2);
        assert_eq!(table.len, 2);
    }

    #[test]
    fn string_find_reproduces_the_retail_bounds() {
        // 0x005ffd90
        assert_eq!(a54_string_find(b"abcabc", b"bc", 0), 1);
        assert_eq!(a54_string_find(b"abcabc", b"bc", 2), 4);
        assert_eq!(a54_string_find(b"abcabc", b"bc", 5), u32::MAX);
        assert_eq!(a54_string_find(b"abcabc", b"", 3), 3);
        assert_eq!(a54_string_find(b"abc", b"abcd", 0), u32::MAX);
        assert_eq!(a54_string_find(b"abc", b"", 4), u32::MAX);
        assert_eq!(a54_string_find(b"aaaa", b"aaa", 1), 1);
    }
}
