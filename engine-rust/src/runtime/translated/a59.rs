#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent59.rs
// ===========================================================================
// Translation batch 59.  Only this file is edited by the batch.
//
// Batch 59 owns `0x00608000..=0x00609fff`.  The ledger lists 43
// `decompiled_uncited` rows in the window; 30 are translated below and the
// 13 rows that are not are listed at the end with their reason.
//
// The window is one `file` support library (retail MSVC, the bundled `fmt`
// v6 formatter and the MSVC `std::list` sort) rather than simulation code.
// Nothing here reads a `CUnit`, a scenario table or an order queue, so the
// translations are pure free functions over local value types and no `sim`
// record gains a field.
//
// * `0x00608430`, `0x006084a0`, `0x006085a0` and `0x006086b0` are `fmt`
//   wrappers with one, three, three and five arguments.  Each passes a
//   packed `types` word built by `fmt`'s type packer: four bits per argument,
//   argument 0 in the least significant nibble, `0x2` = 32-bit unsigned,
//   `0x3` = signed 64-bit and `0xd` = string (pointer + length).  The
//   recovered call sites pin the four shapes: `0xd`, `0x33d`, `0xd3d` and
//   `0xd3d33`.  The format string is a *parameter* of every wrapper (for
//   example `0x00609190` passes `"Failed to open file: {0}"` into
//   `0x00608430`), so the wrappers are translated as arity/type shapes plus
//   the small `{n}` formatter the recovered literals use.
// * `0x006080d0` is the hash-table bucket planner and `0x00609790`,
//   `0x0060980d`, `0x00609c50` are the `std::list::sort` bin machinery.
//   `0x00609c60` writes a nine-word container descriptor.
// * `0x00608780`..`0x006089d0` build and destroy the
//   `file::ReadError`/`file::WriteError`/`exception::StringRuntimeError`
//   objects, and `0x00608ea0`..`0x00609469` are the read/write entry points
//   that turn OS failures into those errors.
// * `0x00608b30`..`0x00608d00` are the `_bcCopyableFunction*` thunks and the
//   predicates used by the file-sharing check.
//
// Recovered layout constants that recur across the window:
//
// | constant | value | use |
// |----------|-------|-----|
// | inline capacity word | `0x8000000f` | bit 31 = inline buffer, low bits = capacity 15 |
// | 16-byte-header string | data at `+0xc` | `0x00608230`, `0x00608c20` |
// | exception string | data at `+0x18`, size `+0x10`, capacity `+0x14` | `0x006087b0`, `0x00608860` |
// | error code | `+0x28` | `0x00608780`, `0x00608900` |
// | sort bin header | `0xc` bytes, 64 bins | `0x00609790`, `0x0060980d` |
//
// Address to function:
//
// * `0x006080d0` [`a59_rehash_plan`]
// * `0x00608230` [`a59_stream_message_init`]
// * `0x00608430` [`a59_format_one_string`] (shape `0xd`)
// * `0x006084a0` [`a59_format_string_ll_ll`] (shape `0x33d`)
// * `0x006085a0` [`a59_format_string_ll_string`] (shape `0xd3d`)
// * `0x006086b0` [`a59_format_ll_ll_string_ll_string`] (shape `0xd3d33`)
// * `0x00608780` [`a59_read_error_layout`]
// * `0x006087b0` [`a59_string_runtime_error_copy_layout`]
// * `0x00608860` [`a59_string_runtime_error_layout`]
// * `0x00608900` [`a59_write_error_from_source_layout`]
// * `0x00608930` [`a59_write_error_layout`]
// * `0x006089d0` [`a59_exception_deleting_dtor`]
// * `0x00608ad0` [`a59_file_output_stream_dtor`]
// * `0x00608b30` [`a59_handle_call_body`]
// * `0x00608b60` [`a59_object_impl_vtable`]
// * `0x00608b70` [`a59_object_impl_with_handle`]
// * `0x00608b90` [`a59_paths_differ`]
// * `0x00608be0` [`a59_file_accepted`]
// * `0x00608c20` [`a59_error_text_message`]
// * `0x00608d00` [`a59_reset_object_impl`]
// * `0x00608ea0` [`a59_open_read`]
// * `0x00609000` [`a59_missing_regular_file_plan`]
// * `0x00609190` [`a59_read_stream_open`]
// * `0x00609250` [`a59_read_stream_open_variant`]
// * `0x00609330` [`a59_write_all_plan`]
// * `0x00609469` [`a59_write_catch_plan`]
// * `0x00609790` [`a59_sort_entry`]
// * `0x0060980d` [`a59_list_sort`]
// * `0x00609c50` [`a59_list_head_init`]
// * `0x00609c60` [`a59_hash_state_init`]
//
// Note on overlapping rows: `0x0060980d` starts inside `0x00609790`'s
// 149-byte body (`0x00609790 + 0x95 == 0x00609825`), so the two ledger rows
// are the `sort()` prologue and the 64-bin merge body it reaches.  Both are
// translated, the first as [`a59_sort_entry`] and the second as
// [`a59_list_sort`].
//
// Rows **not** translated, and why:
//
// * `FUN_00608520`, `FUN_00608630` - the two remaining `fmt` shapes (`0x23d` and
//   `0x3d`).  They are the same wrappers as `0x006084a0`/`0x00608430` with a
//   different context tuple, no cached call site references them, and the
//   window budget stops at 30 rows.
// * `FUN_00608960` - the base `std::exception` destructor: it performs the same
//   capacity-tag check as `0x006089d0` but without the deleting flag, so the
//   translated deleting destructor covers its whole arithmetic.
// * `FUN_006089c6` - an 8-byte jump thunk into `0x00608ad0`; it has no body of
//   its own.
// * `FUN_00608a50` - a byte-identical copy of `0x006089d0` for the second
//   exception class.
// * `FUN_00608b00` - the same teardown as `0x00608ad0` with the
//   `output_stream` destructor, so [`a59_file_output_stream_dtor`] covers the
//   shape.
// * `FUN_00608b50`, `FUN_00609780` - four/six-byte getters returning `*(this+4)`
//   (the captured handle and the stream length); nothing beyond one load.
// * `FUN_00608c10` - an indirect call through the platform import pointer at
//   `0x0249025a` (the "last error" service); the callee is outside the image
//   and has no cached body to reproduce.
// * `FUN_00608ce0`, `FUN_00608cf0` - return the addresses of the two RTTI type
//   descriptors; static-data addresses, no arithmetic or string.
// * `FUN_006092ea` - a catch pad that forwards through two non-returning
//   helpers; it carries no computation.
// * `FUN_006096c0` - an unwind funclet that tail-calls the terminate path.

// ---------------------------------------------------------------------------
// fmt type packing and the `{n}` formatter
// ---------------------------------------------------------------------------

/// `fmt` v6 tag for a 32-bit unsigned argument (`0x2`).
pub const A59_TYPE_UINT: u32 = 0x2;
/// `fmt` v6 tag for a signed 64-bit argument (`0x3`).
pub const A59_TYPE_LONG_LONG: u32 = 0x3;
/// `fmt` v6 tag for a `string_view` argument (`0xd`).
pub const A59_TYPE_STRING: u32 = 0xd;

/// One argument handed to a `fmt` wrapper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A59FmtArg<'a> {
    /// A string argument: pointer and length, copied as two words.
    Str(&'a str),
    /// A signed 64-bit argument.
    Int(i64),
    /// A 32-bit unsigned argument.
    Uint(u32),
}

impl A59FmtArg<'_> {
    /// Packed type tag of the argument.
    pub fn type_nibble(self) -> u32 {
        match self {
            A59FmtArg::Str(_) => A59_TYPE_STRING,
            A59FmtArg::Int(_) => A59_TYPE_LONG_LONG,
            A59FmtArg::Uint(_) => A59_TYPE_UINT,
        }
    }

    fn render(self) -> String {
        match self {
            A59FmtArg::Str(text) => text.to_string(),
            A59FmtArg::Int(value) => value.to_string(),
            A59FmtArg::Uint(value) => value.to_string(),
        }
    }
}

/// Argument-type shape of one recovered `fmt` wrapper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A59FormatShape {
    /// Address of the wrapper.
    pub address: u32,
    /// Packed `fmt` type word.
    pub types: u32,
    /// Number of argument slots the wrapper copies.
    pub arg_count: usize,
}

/// Pack the `fmt` type word: four bits per argument, argument 0 in the least
/// significant nibble.
///
/// The recovered constants are `0x000000d` (one string), `0x000033d`
/// (string, 64-bit, 64-bit), `0x0000d3d` (string, 64-bit, string) and
/// `0x00d3d33` (64-bit, 64-bit, string, 64-bit, string).  More than eight
/// arguments do not fit the 32-bit word the retail wrappers pass; they are
/// truncated at eight.
pub fn a59_pack_arg_types(args: &[A59FmtArg<'_>]) -> u32 {
    let mut packed = 0u32;
    for (index, arg) in args.iter().take(8).enumerate() {
        packed |= arg.type_nibble() << (4 * index);
    }
    packed
}

/// Error reported while parsing a retail `{n}` format string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A59FormatError {
    /// A `}` that is not part of a `{{` escape.
    UnmatchedClose,
    /// A `{` that is not closed by `}`.
    MissingClose,
    /// A mix of automatic `{}` and manual `{n}` indexing.
    MixedIndexing,
    /// A placeholder index beyond the argument list.
    IndexOutOfRange,
}

/// Error reported by a translated wrapper: the shape check or the parser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A59FormatShapeError {
    /// The wrapper was handed the wrong number of argument slots.
    ArgCount { expected: usize, actual: usize },
    /// The packed type word differs from the one the wrapper builds.
    Types { expected: u32, actual: u32 },
    /// The format string itself is malformed.
    Format(A59FormatError),
}

/// Render the `{n}` subset used by every recovered format string.
///
/// * `{{` and `}}` are literal braces.
/// * `{n}` selects argument `n`; `{}` selects the next argument (the retail
///   parser enforces that the two styles are not mixed).
/// * Decimal values are rendered as `fmt` renders them for these shapes.
///
/// Width, fill and alignment specifications are not needed by any recovered
/// literal in the window and are not modelled.
pub fn a59_format(template: &str, args: &[A59FmtArg<'_>]) -> Result<String, A59FormatError> {
    let bytes = template.as_bytes();
    let mut out = String::with_capacity(template.len() + 16);
    let mut index = 0usize;
    let mut next_auto = 0usize;
    let mut manual_seen = false;
    let mut auto_seen = false;
    while index < bytes.len() {
        match bytes[index] {
            b'{' => {
                if bytes.get(index + 1) == Some(&b'{') {
                    out.push('{');
                    index += 2;
                    continue;
                }
                let mut cursor = index + 1;
                let mut literal = 0usize;
                let mut digits = 0usize;
                while let Some(digit) = bytes.get(cursor).copied() {
                    if !digit.is_ascii_digit() {
                        break;
                    }
                    literal = literal
                        .checked_mul(10)
                        .and_then(|value| value.checked_add((digit - b'0') as usize))
                        .ok_or(A59FormatError::IndexOutOfRange)?;
                    cursor += 1;
                    digits += 1;
                }
                if bytes.get(cursor) != Some(&b'}') {
                    return Err(A59FormatError::MissingClose);
                }
                let selected = if digits == 0 {
                    if manual_seen {
                        return Err(A59FormatError::MixedIndexing);
                    }
                    auto_seen = true;
                    let current = next_auto;
                    next_auto += 1;
                    current
                } else {
                    if auto_seen {
                        return Err(A59FormatError::MixedIndexing);
                    }
                    manual_seen = true;
                    literal
                };
                let arg = args.get(selected).ok_or(A59FormatError::IndexOutOfRange)?;
                out.push_str(&arg.render());
                index = cursor + 1;
            }
            b'}' => {
                if bytes.get(index + 1) == Some(&b'}') {
                    out.push('}');
                    index += 2;
                    continue;
                }
                return Err(A59FormatError::UnmatchedClose);
            }
            _ => {
                // Braces and digits are ASCII, so `index` is always on a
                // character boundary here.
                let ch = template[index..]
                    .chars()
                    .next()
                    .expect("index is a character boundary");
                out.push(ch);
                index += ch.len_utf8();
            }
        }
    }
    Ok(out)
}

/// Check a wrapper's argument shape and render its format string.
pub fn a59_format_with_shape(
    shape: &A59FormatShape,
    template: &str,
    args: &[A59FmtArg<'_>],
) -> Result<String, A59FormatShapeError> {
    if args.len() != shape.arg_count {
        return Err(A59FormatShapeError::ArgCount {
            expected: shape.arg_count,
            actual: args.len(),
        });
    }
    let packed = a59_pack_arg_types(args);
    if packed != shape.types {
        return Err(A59FormatShapeError::Types {
            expected: shape.types,
            actual: packed,
        });
    }
    a59_format(template, args).map_err(A59FormatShapeError::Format)
}

/// Shape of `0x00608430` (`FUN_00608430`, 105 bytes): one string argument.
///
/// ```c
/// len = strlen(param_2);                 // param_2 is the format string
/// local_10 = *param_3; local_c = param_3[1];   // string + length
/// FUN_00558ab0(out, param_2, len, 0xd, 0, &local_10);
/// ```
///
/// The wrapper copies the string argument as two words (pointer and length)
/// and passes the packed type word `0xd` to the formatter.
pub fn a59_shape_00608430() -> A59FormatShape {
    A59FormatShape {
        address: 0x0060_8430,
        types: 0xd,
        arg_count: 1,
    }
}

/// `0x00608430`: format one `string_view` argument.
pub fn a59_format_one_string(template: &str, text: &str) -> Result<String, A59FormatShapeError> {
    a59_format_with_shape(&a59_shape_00608430(), template, &[A59FmtArg::Str(text)])
}

/// Shape of `0x006084a0` (`FUN_006084a0`, 121 bytes): string, 64-bit, 64-bit.
///
/// The recovered body copies one two-word string slot and two integer slots
/// from `param_3`..`param_5` and passes the type word `0x33d`.
pub fn a59_shape_006084a0() -> A59FormatShape {
    A59FormatShape {
        address: 0x0060_84a0,
        types: 0x33d,
        arg_count: 3,
    }
}

/// `0x006084a0`: format a string followed by two 64-bit context values.
pub fn a59_format_string_ll_ll(
    template: &str,
    text: &str,
    first: i64,
    second: i64,
) -> Result<String, A59FormatShapeError> {
    a59_format_with_shape(
        &a59_shape_006084a0(),
        template,
        &[
            A59FmtArg::Str(text),
            A59FmtArg::Int(first),
            A59FmtArg::Int(second),
        ],
    )
}

/// Shape of `0x006085a0` (`FUN_006085a0`, 126 bytes): string, 64-bit, string.
///
/// This is the shape of the write-error messages in `0x00609330` and
/// `0x00609469`: `"Failed to create folder(s): {0} | {1} | {2}"` receives the
/// path, the platform error code and the rendered error text.  The packed type
/// word is `0xd3d`.
pub fn a59_shape_006085a0() -> A59FormatShape {
    A59FormatShape {
        address: 0x0060_85a0,
        types: 0xd3d,
        arg_count: 3,
    }
}

/// `0x006085a0`: format a string, a 64-bit value and a second string.
pub fn a59_format_string_ll_string(
    template: &str,
    text: &str,
    number: i64,
    tail: &str,
) -> Result<String, A59FormatShapeError> {
    a59_format_with_shape(
        &a59_shape_006085a0(),
        template,
        &[
            A59FmtArg::Str(text),
            A59FmtArg::Int(number),
            A59FmtArg::Str(tail),
        ],
    )
}

/// Shape of `0x006086b0` (`FUN_006086b0`, 143 bytes): 64-bit, 64-bit, string,
/// 64-bit, string.
///
/// The recovered type word is `0xd3d33`.  The only cached call site is the
/// short-write message in `0x00609330`, which matches the shape exactly:
/// written bytes, expected bytes, path, platform error code, error text.
pub fn a59_shape_006086b0() -> A59FormatShape {
    A59FormatShape {
        address: 0x0060_86b0,
        types: 0xd3d33,
        arg_count: 5,
    }
}

/// `0x006086b0`: format two 64-bit values, a string, a 64-bit value and a
/// second string.
pub fn a59_format_ll_ll_string_ll_string(
    template: &str,
    first: i64,
    second: i64,
    text: &str,
    number: i64,
    tail: &str,
) -> Result<String, A59FormatShapeError> {
    a59_format_with_shape(
        &a59_shape_006086b0(),
        template,
        &[
            A59FmtArg::Int(first),
            A59FmtArg::Int(second),
            A59FmtArg::Str(text),
            A59FmtArg::Int(number),
            A59FmtArg::Str(tail),
        ],
    )
}

// ---------------------------------------------------------------------------
// string headers and the 1.5x capacity policy
// ---------------------------------------------------------------------------

/// Inline capacity word written by every string constructor in the window:
/// bit 31 marks the inline buffer, the low bits hold the 15-byte capacity.
pub const A59_INLINE_CAPACITY_WORD: u32 = 0x8000_000f;

/// Inline capacity in bytes (`0xf`).
pub const A59_INLINE_CAPACITY: u32 = 0xf;

/// Data offset of the sixteen-byte-header strings built by `0x00608230` and
/// `0x00608c20` (`this + 3` dwords).
pub const A59_STRING_DATA_OFFSET: u32 = 0xc;

/// Data offset of the exception strings built by `0x006087b0` and
/// `0x00608860` (`this + 6` dwords).
pub const A59_ERROR_DATA_OFFSET: u32 = 0x18;

/// Capacity tag bit (bit 31) and mask used by the growth helper.
pub const A59_CAPACITY_TAG: u32 = 0x8000_0000;
pub const A59_CAPACITY_MASK: u32 = !A59_CAPACITY_TAG;

/// Plain string header recovered at the construction sites.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A59InlineString {
    /// Offset of the character buffer.
    pub data_offset: u32,
    /// Length in bytes (zero at construction).
    pub size: u32,
    /// Capacity word, tag bit included.
    pub capacity_word: u32,
}

/// Result of the 1.5x capacity helper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A59Capacity {
    /// Capacity word after the request.
    pub word: u32,
    /// Whether the helper had to grow.
    pub grew: bool,
}

/// The growth arithmetic shared by `0x00608230`, `0x006087b0`, `0x00608860`
/// and `0x00608c20` (`FUN_00538810`, cut off at the allocator):
///
/// ```c
/// cap = word & 0x7fffffff;
/// if (cap < requested) {
///     grown = (word >> 1 & 0x3fffffff) + cap;    // 1.5 * cap
///     if (grown < requested) grown = requested;
///     word = (word ^ grown) & 0x7fffffff ^ word; // keep the tag bit
///     FUN_0093ec70();                            // allocate and copy
/// }
/// ```
///
/// `FUN_0093ec70` is the allocator tail; the cached body of every caller is
/// cut there, so the copy is not reproduced.  The returned word is exactly the
/// value the caller stores before the tail call.
pub fn a59_capacity_growth(word: u32, requested: u32) -> A59Capacity {
    let capacity = word & A59_CAPACITY_MASK;
    if capacity < requested {
        let grown = (word >> 1 & 0x3fff_ffff).wrapping_add(capacity);
        let target = grown.max(requested);
        A59Capacity {
            word: ((word ^ target) & A59_CAPACITY_MASK) ^ word,
            grew: true,
        }
    } else {
        A59Capacity { word, grew: false }
    }
}

/// String initialisation reported by `0x00608230`: an empty sixteen-byte-
/// header string that then reserves the length the stream reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A59StringInit {
    /// Length requested from the capacity helper.
    pub reserved_len: u32,
    /// Header written before the request.
    pub header: A59InlineString,
    /// Capacity word after the request.
    pub capacity: A59Capacity,
}

/// `0x00608230` (`FUN_00608230`, 115 bytes): initialise an output string and
/// reserve the length reported by a stream.
///
/// ```c
/// len = (*(stream->vtable + 0xc))(cookie);   // reported length
/// out[1] = 0;                                // size
/// out[2] = 0x8000000f;                       // inline capacity word
/// *out   = out + 3;                          // data at +0xc
/// *(u8 *)(out + 3) = 0;
/// FUN_00538810(len);                         // reserve
/// ```
///
/// The virtual call is the stream's reported length and is taken as the input
/// `stream_len`; the character copy happens after the allocator tail and is
/// not in the cached body.
pub fn a59_stream_message_init(stream_len: u32) -> A59StringInit {
    let header = A59InlineString {
        data_offset: A59_STRING_DATA_OFFSET,
        size: 0,
        capacity_word: A59_INLINE_CAPACITY_WORD,
    };
    A59StringInit {
        reserved_len: stream_len,
        header,
        capacity: a59_capacity_growth(header.capacity_word, stream_len),
    }
}

/// Message built from the platform error text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A59ErrorMessage {
    /// The platform error text.
    pub text: String,
    /// Header with the text length reserved.
    pub init: A59StringInit,
}

/// `0x00608c20` (`FUN_00608c20`, 115 bytes): turn the platform error text into
/// a string.
///
/// The body calls the platform text helper, measures it with the same
/// `do { c = *p++; } while (c)` loop Ghidra shows (an empty string measures
/// zero), builds the `0x8000000f` header and reserves the measured length.
/// The measurement is exactly [`str::len`] for the UTF-8 text.
pub fn a59_error_text_message(text: &str) -> A59ErrorMessage {
    A59ErrorMessage {
        text: text.to_string(),
        init: a59_stream_message_init(text.len() as u32),
    }
}

// ---------------------------------------------------------------------------
// exception object layouts
// ---------------------------------------------------------------------------

/// Class of a recovered file exception.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A59ErrorClass {
    /// `file::ReadError`.
    Read,
    /// `file::WriteError`.
    Write,
    /// `exception::StringRuntimeError`.
    StringRuntime,
}

/// Source object of the copy constructor at `0x006087b0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A59ErrorSource {
    /// The two words at source `+0x4`/`+0x8`, copied verbatim.
    pub prefix: [u32; 2],
    /// Length field at source `+0x10`, passed to the capacity helper.
    pub reserved_len: u32,
    /// Code field at source `+0x28`, read by `0x00608900`.
    pub code: u32,
}

/// Recovered layout of one thrown file exception.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A59ErrorLayout {
    /// Vtable class assigned last by the constructor.
    pub class: A59ErrorClass,
    /// `Some` when the constructor writes the code field at `+0x28`.
    pub code: Option<u32>,
    /// Words at `+0x4`/`+0x8`.
    pub prefix: [u32; 2],
    /// String header at `+0xc` (data at `+0x18`).
    pub header: A59InlineString,
    /// Capacity word after reserving the source length.
    pub capacity: A59Capacity,
}

fn a59_error_header(reserved_len: u32) -> (A59InlineString, A59Capacity) {
    let header = A59InlineString {
        data_offset: A59_ERROR_DATA_OFFSET,
        size: 0,
        capacity_word: A59_INLINE_CAPACITY_WORD,
    };
    (
        header,
        a59_capacity_growth(header.capacity_word, reserved_len),
    )
}

/// `0x00608780` (`FUN_00608780`, 39 bytes): finish a `file::ReadError`.
///
/// ```c
/// FUN_00608860(source);              // base ctor from the source length
/// this[10] = code;                   // +0x28
/// *this = file::ReadError::vftable;
/// ```
///
/// `reserved_len` is the length the base constructor reads from its source.
pub fn a59_read_error_layout(reserved_len: u32, code: u32) -> A59ErrorLayout {
    let (header, capacity) = a59_error_header(reserved_len);
    A59ErrorLayout {
        class: A59ErrorClass::Read,
        code: Some(code),
        prefix: [0, 0],
        header,
        capacity,
    }
}

/// `0x00608860` (`FUN_00608860`, 107 bytes): `exception::StringRuntimeError`
/// base constructor.
///
/// ```c
/// *(u64 *)(this + 1) = 0;                    // +0x4..+0xc
/// *this = exception::StringRuntimeError::vftable;
/// len = *(source + 4);
/// this[4] = 0; this[3] = this + 6; this[5] = 0x8000000f;
/// *(u8 *)(this + 6) = 0;
/// FUN_00538810(len);
/// ```
///
/// The two prefix words are cleared, the string header sits at `+0xc` with its
/// data pointer at `+0x18`, and the capacity helper reserves `reserved_len`.
pub fn a59_string_runtime_error_layout(reserved_len: u32) -> A59ErrorLayout {
    let (header, capacity) = a59_error_header(reserved_len);
    A59ErrorLayout {
        class: A59ErrorClass::StringRuntime,
        code: None,
        prefix: [0, 0],
        header,
        capacity,
    }
}

/// `0x006087b0` (`FUN_006087b0`, 127 bytes): copy-construct an
/// `exception::StringRuntimeError`.
///
/// ```c
/// *this = std::exception::vftable;
/// *(u64 *)(this + 1) = 0;
/// FUN_00c894fe(source + 4, this + 1);   // copy the two prefix words
/// *this = exception::StringRuntimeError::vftable;
/// len = *(source + 0x10);
/// this[4] = 0; this[3] = this + 6; this[5] = 0x8000000f;
/// *(u8 *)(this + 6) = 0;
/// FUN_00538810(len);
/// ```
///
/// The string copy is cut at the allocator; only the layout and the reserved
/// length are recoverable, and the two copied words are returned as the
/// source's `prefix`.
pub fn a59_string_runtime_error_copy_layout(source: A59ErrorSource) -> A59ErrorLayout {
    let (header, capacity) = a59_error_header(source.reserved_len);
    A59ErrorLayout {
        class: A59ErrorClass::StringRuntime,
        code: None,
        prefix: source.prefix,
        header,
        capacity,
    }
}

/// `0x00608900` (`FUN_00608900`, 42 bytes): `file::WriteError` from a
/// `StringRuntimeError`-shaped source.
///
/// ```c
/// FUN_006087b0(source);                          // copy base
/// *this = file::WriteError::vftable;
/// this[10] = *(source + 0x28);                   // error code
/// ```
pub fn a59_write_error_from_source_layout(source: A59ErrorSource) -> A59ErrorLayout {
    let mut layout = a59_string_runtime_error_copy_layout(source);
    layout.class = A59ErrorClass::Write;
    layout.code = Some(source.code);
    layout
}

/// `0x00608930` (`FUN_00608930`, 39 bytes): finish a `file::WriteError`.
///
/// ```c
/// FUN_00608860(source);              // base ctor from the source length
/// this[10] = code;                   // +0x28
/// *this = file::WriteError::vftable;
/// ```
pub fn a59_write_error_layout(reserved_len: u32, code: u32) -> A59ErrorLayout {
    let (header, capacity) = a59_error_header(reserved_len);
    A59ErrorLayout {
        class: A59ErrorClass::Write,
        code: Some(code),
        prefix: [0, 0],
        header,
        capacity,
    }
}

/// One fully rendered error the throw sites pass to the runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A59ThrownError {
    /// Exception class.
    pub class: A59ErrorClass,
    /// Error code stored in the object.
    pub code: u32,
    /// Rendered message.
    pub message: String,
}

// ---------------------------------------------------------------------------
// destructors
// ---------------------------------------------------------------------------

/// Result of a scalar deleting destructor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A59Dtor {
    /// The destructor ran.
    pub destroyed: bool,
    /// The object was released (delete flag set).
    pub deallocated: bool,
}

/// Recovered fault paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A59Fault {
    /// The string capacity word had bit 31 clear before destruction
    /// (`-1 < (int)word` is the retail trap at `FUN_00608960`/`0x006089d0`).
    CapacityWord(u32),
    /// A bucket count above `2^31` cannot be reached by the power-of-two
    /// loop without wrapping to zero.
    UnrepresentableBuckets(u32),
}

/// `0x006089d0` (`FUN_006089d0`, 112 bytes): deleting destructor of the
/// `std::exception`-derived strings.
///
/// ```c
/// if (-1 < (int)this[5]) FUN_0093ec70();   // capacity tag check -> trap
/// *this = std::exception::vftable;
/// FUN_00c89561(this + 1);                  // destroy the string
/// if (flag & 1) FUN_00923e90();            // operator delete(this)
/// ```
///
/// `this[5]` is the capacity word at `+0x14`; every constructed string has bit
/// 31 set (`0x8000000f`), so a word with the sign bit clear is the retail
/// invariant fault.
pub fn a59_exception_deleting_dtor(capacity_word: u32, flag: u8) -> Result<A59Dtor, A59Fault> {
    if (capacity_word as i32) >= 0 {
        return Err(A59Fault::CapacityWord(capacity_word));
    }
    Ok(A59Dtor {
        destroyed: true,
        deallocated: flag & 1 != 0,
    })
}

/// `0x00608ad0` (`FUN_00608ad0`, 32 bytes): deleting destructor of
/// `stdext::file_output_stream`.
///
/// ```c
/// FUN_00c6ce30();          // reset the vtable, close handle at +8, destroy
/// if (flag & 1) FUN_00923e90();
/// ```
///
/// The destructor body closes the handle (`+8 != -1`), stores `-1`, and runs
/// the two base teardowns; the translation reports that teardown and the
/// conditional release.
pub fn a59_file_output_stream_dtor(flag: u8) -> A59Dtor {
    A59Dtor {
        destroyed: true,
        deallocated: flag & 1 != 0,
    }
}

// ---------------------------------------------------------------------------
// copyable-function thunks
// ---------------------------------------------------------------------------

/// `0x00608b30` (`FUN_00608b30`, 24 bytes): body of the
/// `unsigned int(HOSFILE *)` callable.
///
/// ```c
/// FUN_0093ffa0(*param_1, 0, 0);   // platform call with the captured handle
/// ```
///
/// The platform helper is an import (`0x024c029a`), so the call is supplied as
/// a closure; the two trailing zero arguments and the captured handle are
/// reproduced exactly.
pub fn a59_handle_call_body<T>(handle: i32, call: impl FnOnce(i32, u32, u32) -> T) -> T {
    call(handle, 0, 0)
}

/// Vtable of the object impl wrapping the handle lambda
/// (`_bcCopyableFunctionObjectImpl<<lambda_95e7...>&,unsigned int,HOSFILE*>`).
pub const A59_VTABLE_OBJECT_HANDLE: &str = "_bcCopyableFunctionObjectImpl<<lambda_95e70d69b51b22763c90d4eab9ccccfc>&,unsigned_int,HOSFILE__*>::vftable";

/// Vtable of the object impl wrapping the size lambda
/// (`_bcCopyableFunctionObjectImpl<<lambda_e1f5...>&,unsigned int,HOSFILE*>`).
pub const A59_VTABLE_OBJECT_SIZE: &str = "_bcCopyableFunctionObjectImpl<<lambda_e1f546344bc49b26d3017301900dd4c7>&,unsigned_int,HOSFILE__*>::vftable";

/// Vtable of the non-object callable base
/// (`_bcCopyableFunctionImpl<unsigned int,HOSFILE*>`).
pub const A59_VTABLE_FUNCTION_IMPL: &str =
    "_bcCopyableFunctionImpl<unsigned_int,HOSFILE__*>::vftable";

/// One recovered copyable-function object and the fields the thunks write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A59FunctionObject {
    /// Base vtable written first, when the body writes one.
    pub base_vtable: Option<&'static str>,
    /// Object-impl vtable written last.
    pub object_vtable: &'static str,
    /// Captured handle word copied from the source object.
    pub captured: Option<u32>,
}

/// `0x00608b60` (`FUN_00608b60`, 16 bytes): install the object-impl vtable of
/// the `lambda_95e7...` handle callable.
///
/// ```c
/// *param_1 = _bcCopyableFunctionObjectImpl<<lambda_95e7...>&,...>::vftable;
/// ```
pub fn a59_object_impl_vtable() -> A59FunctionObject {
    A59FunctionObject {
        base_vtable: None,
        object_vtable: A59_VTABLE_OBJECT_HANDLE,
        captured: None,
    }
}

/// `0x00608b70` (`FUN_00608b70`, 28 bytes): copy the `lambda_e1f5...` callable
/// and its captured handle.
///
/// ```c
/// *param_1 = _bcCopyableFunctionImpl<unsigned_int,HOSFILE*>::vftable;
/// param_1[1] = *(this + 4);                    // captured handle
/// *param_1 = _bcCopyableFunctionObjectImpl<<lambda_e1f5...>&,...>::vftable;
/// ```
pub fn a59_object_impl_with_handle(captured: u32) -> A59FunctionObject {
    A59FunctionObject {
        base_vtable: Some(A59_VTABLE_FUNCTION_IMPL),
        object_vtable: A59_VTABLE_OBJECT_SIZE,
        captured: Some(captured),
    }
}

/// `0x00608d00` (`FUN_00608d00`, 84 bytes): build the `lambda_95e7...` object
/// impl on the stack and hand it to the callable reset helper.
///
/// The reset helper (`FUN_005426e0`) is outside the window; the recovered body
/// only writes the vtable and calls it, so the translation reports the vtable
/// the reset sees.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A59CallableReset {
    /// Vtable installed on the temporary.
    pub vtable: &'static str,
    /// The reset helper was invoked.
    pub reset: bool,
}

/// `0x00608d00`: see [`A59CallableReset`].
pub fn a59_reset_object_impl() -> A59CallableReset {
    A59CallableReset {
        vtable: A59_VTABLE_OBJECT_HANDLE,
        reset: true,
    }
}

// ---------------------------------------------------------------------------
// file predicates
// ---------------------------------------------------------------------------

/// `0x00608be0` (`FUN_00608be0`, 44 bytes): regular-file probe guard.
///
/// ```c
/// if (is_regular_file(a) && FUN_0093fdb0(*a) == 0) return 0;
/// return 1;
/// ```
///
/// `FUN_0074f0c0` is the `GetFileAttributes`-based test and is true only for an
/// existing non-directory.  `FUN_0093fdb0` converts the path to UTF-16 and calls
/// the one-argument platform service at `0x024d0046`; the body only compares
/// its result with zero, so that result is an input (`probe_zero`) here.
pub fn a59_file_accepted(is_regular_file: bool, probe_zero: bool) -> bool {
    !(is_regular_file && probe_zero)
}

/// `0x00608b90` (`FUN_00608b90`, 65 bytes): the file-replacement difference
/// test.
///
/// ```c
/// if (is_regular_file(b) && FUN_0093fdb0(*b) == 0) return 0;
/// return compare_paths(a, b, 1) != 0;
/// ```
///
/// The first branch is [`a59_file_accepted`]; the second compares the two
/// paths through `FUN_00941150` and returns true unless they compare equal,
/// which is taken as `paths_equal` here.
pub fn a59_paths_differ(
    second_is_regular_file: bool,
    second_probe_zero: bool,
    paths_equal: bool,
) -> bool {
    a59_file_accepted(second_is_regular_file, second_probe_zero) && !paths_equal
}

// ---------------------------------------------------------------------------
// open / write error paths
// ---------------------------------------------------------------------------

/// Platform callable reference encoded in a file-handle word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A59CallableRef {
    /// Bit 0 clear: the callable lives in the caller's slot (`&param_3`).
    StackCopy,
    /// Bit 0 set: the callable is the heap object `word & !1`.
    Heap(u32),
}

/// `0x00608ea0` (`FUN_00608ea0`, 187 bytes) helper: select the callable the
/// handle path invokes at vtable slot 1.
///
/// ```c
/// if (word & 1) callable = (int *)(word & 0xfffffffe);
/// else          callable = (int *)&word;
/// ```
pub fn a59_select_callable(word: u32) -> A59CallableRef {
    if word & 1 != 0 {
        A59CallableRef::Heap(word & !1)
    } else {
        A59CallableRef::StackCopy
    }
}

/// Outcome of the read-open entry point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A59OpenRead {
    /// Handle returned by the platform open call.
    pub handle: i32,
    /// Callable invoked with `&handle` when the open succeeded.
    pub handle_callback: Option<A59CallableRef>,
    /// `file::ReadError` thrown when the open failed.
    pub error: Option<A59ThrownError>,
}

/// `0x00608ea0` (`FUN_00608ea0`, 187 bytes): open for reading, with the
/// missing-file error.
///
/// ```c
/// handle = CreateFileA(path, 0x80000000, 1, 3, 0x80);
/// if (handle != -1) {
///     callable = (param_3 & 1) ? (param_3 & ~1) : &param_3;
///     (*(callable->vtable + 4))(&handle);
///     ...                                 // cached body is cut here
/// } else {
///     msg = format("File does not exist: {0}", path);
///     throw file::ReadError(0, msg);
/// }
/// ```
///
/// The open parameters are `GENERIC_READ`, share-read, `OPEN_EXISTING` and
/// `FILE_ATTRIBUTE_NORMAL`.  The success path's continuation is not in the
/// cached body; the selected callable is reported.
pub fn a59_open_read(
    handle: i32,
    path: &str,
    callable_word: u32,
) -> Result<A59OpenRead, A59FormatShapeError> {
    if handle != -1 {
        Ok(A59OpenRead {
            handle,
            handle_callback: Some(a59_select_callable(callable_word)),
            error: None,
        })
    } else {
        let message = a59_format_one_string("File does not exist: {0}", path)?;
        Ok(A59OpenRead {
            handle,
            handle_callback: None,
            error: Some(A59ThrownError {
                class: A59ErrorClass::Read,
                code: 0,
                message,
            }),
        })
    }
}

/// Plan built by `0x00609000`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A59MissingFilePlan {
    /// The path is an existing regular file (`FUN_0074f0c0`).
    pub regular_file: bool,
    /// Allocation request made on the existing-file path (16 bytes).
    pub allocation: Option<u32>,
    /// The `ReadError(0, "File does not exist: {0}")` that is always built.
    pub error: A59ThrownError,
}

/// `0x00609000` (`FUN_00609000`, 138 bytes): the missing-file throw path.
///
/// ```c
/// if (is_regular_file(path)) FUN_00923df0(0x10);   // operator new(16)
/// msg = format("File does not exist: {0}", path);
/// throw file::ReadError(0, msg);
/// ```
///
/// Both the 16-byte allocation and the error object are reproduced; the
/// allocation result is discarded in the cached body before the throw.
pub fn a59_missing_regular_file_plan(
    path: &str,
    regular_file: bool,
) -> Result<A59MissingFilePlan, A59FormatShapeError> {
    let message = a59_format_one_string("File does not exist: {0}", path)?;
    Ok(A59MissingFilePlan {
        regular_file,
        allocation: regular_file.then_some(0x10),
        error: A59ThrownError {
            class: A59ErrorClass::Read,
            code: 0,
            message,
        },
    })
}

/// Outcome of the read-stream constructors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A59ReadStreamOpen {
    /// The stream constructor reported success.
    pub opened: bool,
    /// Message header reserved from the stream length on success.
    pub message: Option<A59StringInit>,
    /// `ReadError(1, "Failed to open file: {0}")` on failure.
    pub error: Option<A59ThrownError>,
}

fn a59_read_stream_outcome(
    opened: bool,
    stream_len: u32,
    path: &str,
) -> Result<A59ReadStreamOpen, A59FormatShapeError> {
    if opened {
        Ok(A59ReadStreamOpen {
            opened,
            message: Some(a59_stream_message_init(stream_len)),
            error: None,
        })
    } else {
        let message = a59_format_one_string("Failed to open file: {0}", path)?;
        Ok(A59ReadStreamOpen {
            opened,
            message: None,
            error: Some(A59ThrownError {
                class: A59ErrorClass::Read,
                code: 1,
                message,
            }),
        })
    }
}

/// `0x00609190` (`FUN_00609190`, 184 bytes): construct the input stream and
/// either reserve the message or throw `ReadError(1)`.
///
/// ```c
/// FUN_00551020(stream, *path, flags);
/// if (stream.opened) { FUN_00608230(out, stream); return; }
/// msg = format("Failed to open file: {0}", path);
/// throw file::ReadError(1, msg);
/// ```
pub fn a59_read_stream_open(
    opened: bool,
    stream_len: u32,
    path: &str,
) -> Result<A59ReadStreamOpen, A59FormatShapeError> {
    a59_read_stream_outcome(opened, stream_len, path)
}

/// `0x00609250` (`FUN_00609250`, 199 bytes): the second input-stream
/// constructor variant.
///
/// The body differs from `0x00609190` only in its exception frame (a saved
/// stack cookie is restored on the failure path) and in the stream slot it
/// passes; the success path reserves the stream length and the failure path
/// throws the same `ReadError(1, "Failed to open file: {0}")`.
pub fn a59_read_stream_open_variant(
    opened: bool,
    stream_len: u32,
    path: &str,
) -> Result<A59ReadStreamOpen, A59FormatShapeError> {
    a59_read_stream_outcome(opened, stream_len, path)
}

/// Failure selected by the write-all routine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum A59WriteFailure {
    /// `"Failed to create folder(s): {0} | {1} | {2}"`, code 0.
    CreateFolder(A59ThrownError),
    /// `"Existing file is read only: {0}"`, thrown without an error code.
    ReadOnly(String),
    /// `"Failed to open file for writing: {0} | {1} | {2}"`, code 1.
    OpenForWriting(A59ThrownError),
    /// `"Failed to write all data (wrote {0} / {1} bytes): ..."`, code 2.
    ShortWrite(A59ThrownError),
}

impl A59WriteFailure {
    /// The thrown error, when the branch builds a `file::WriteError`.
    pub fn error(&self) -> Option<&A59ThrownError> {
        match self {
            A59WriteFailure::CreateFolder(error)
            | A59WriteFailure::OpenForWriting(error)
            | A59WriteFailure::ShortWrite(error) => Some(error),
            A59WriteFailure::ReadOnly(_) => None,
        }
    }
}

/// Result of the write-all routine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A59WritePlan {
    /// No failure branch fired.
    pub success: bool,
    /// First failure branch that fired.
    pub failure: Option<A59WriteFailure>,
}

/// `0x00609330` (`FUN_00609330`, 681 bytes): write a whole buffer to a file.
///
/// The recovered branch order is:
///
/// 1. the directory-creation helper (`FUN_0074f0a0`) fails ->
///    `WriteError(0, "Failed to create folder(s): {0} | {1} | {2}")`;
/// 2. the path is a regular file (`FUN_0074f0c0`) whose attributes are valid and
///    carry `FILE_ATTRIBUTE_READONLY` -> `"Existing file is read only: {0}"`
///    (thrown without an error code);
/// 3. the stream-open helper at `FUN_00c6d470` fails ->
///    `WriteError(1, "Failed to open file for writing: {0} | {1} | {2}")`;
/// 4. the stream write returns fewer bytes than `param_1[1]` ->
///    `WriteError(2, "Failed to write all data (wrote {0} / {1} bytes): ...")`;
/// 5. otherwise the stream is closed and the handle left unused.
///
/// `last_error` and `error_text` are the values the body obtains from
/// `FUN_00608c10` (the platform last-error service) and `0x00608c20` for the
/// `{1}` and `{2}` slots.  The short-write message uses the stream length at
/// `+4` (`FUN_00609780`) for its `{1}` slot.
pub fn a59_write_all_plan(
    path: &str,
    directory_ready: bool,
    path_is_regular_file: bool,
    existing_attributes: u32,
    open_succeeded: bool,
    written: u32,
    data_len: u32,
    stream_len: u32,
    last_error: u32,
    error_text: &str,
) -> Result<A59WritePlan, A59FormatShapeError> {
    let failure = if !directory_ready {
        let message = a59_format_string_ll_string(
            "Failed to create folder(s): {0} | {1} | {2}",
            path,
            last_error as i64,
            error_text,
        )?;
        Some(A59WriteFailure::CreateFolder(A59ThrownError {
            class: A59ErrorClass::Write,
            code: 0,
            message,
        }))
    } else if path_is_regular_file
        && existing_attributes != 0xffff_ffff
        && existing_attributes & 1 != 0
    {
        let message = a59_format_one_string("Existing file is read only: {0}", path)?;
        Some(A59WriteFailure::ReadOnly(message))
    } else if !open_succeeded {
        let message = a59_format_string_ll_string(
            "Failed to open file for writing: {0} | {1} | {2}",
            path,
            last_error as i64,
            error_text,
        )?;
        Some(A59WriteFailure::OpenForWriting(A59ThrownError {
            class: A59ErrorClass::Write,
            code: 1,
            message,
        }))
    } else if written != data_len {
        let message = a59_format_ll_ll_string_ll_string(
            "Failed to write all data (wrote {0} / {1} bytes): {2} | {3} | {4}",
            written as i64,
            stream_len as i64,
            path,
            last_error as i64,
            error_text,
        )?;
        Some(A59WriteFailure::ShortWrite(A59ThrownError {
            class: A59ErrorClass::Write,
            code: 2,
            message,
        }))
    } else {
        None
    };
    Ok(A59WritePlan {
        success: failure.is_none(),
        failure,
    })
}

/// Result of the write catch-all handler `0x00609469`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A59WriteCatchPlan {
    /// The platform error code was `ERROR_ACCESS_DENIED` (5).
    pub access_denied: bool,
    /// The nested access-denied message thrown directly.
    pub thrown: Option<String>,
    /// The `WriteError(5, ...)` built on the other branch.
    pub error: Option<A59ThrownError>,
}

/// `0x00609469` (`Catch_All@00609469`, 158 bytes): write catch-all handler.
///
/// ```c
/// last = platform_last_error();
/// base = format("Failed to write to file: {0} | {1} | {2}", path, last, text);
/// if (last == 5)
///     throw format("Failed to write to file (access denied): {0} | {1} | {2}",
///                  base, last, text);
/// throw file::WriteError(5, base);
/// ```
///
/// The `{0}` slot of the access-denied message is the already-rendered base
/// message, which is reproduced by compositing the two calls.
pub fn a59_write_catch_plan(
    path: &str,
    last_error: u32,
    error_text: &str,
) -> Result<A59WriteCatchPlan, A59FormatShapeError> {
    let base = a59_format_string_ll_string(
        "Failed to write to file: {0} | {1} | {2}",
        path,
        last_error as i64,
        error_text,
    )?;
    if last_error == 5 {
        let denied = a59_format_string_ll_string(
            "Failed to write to file (access denied): {0} | {1} | {2}",
            &base,
            last_error as i64,
            error_text,
        )?;
        Ok(A59WriteCatchPlan {
            access_denied: true,
            thrown: Some(denied),
            error: None,
        })
    } else {
        Ok(A59WriteCatchPlan {
            access_denied: false,
            thrown: None,
            error: Some(A59ThrownError {
                class: A59ErrorClass::Write,
                code: 5,
                message: base,
            }),
        })
    }
}

// ---------------------------------------------------------------------------
// hash bucket planning
// ---------------------------------------------------------------------------

/// Smallest bucket count the planner can return (`8`).
pub const A59_MIN_BUCKETS: u32 = 8;

/// Largest bucket count reachable by the doubling loop (`2^31`).
pub const A59_BUCKET_LIMIT: u32 = 0x8000_0000;

/// Plan produced by the bucket-count routine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A59RehashPlan {
    /// Power-of-two bucket count.
    pub buckets: u32,
    /// The current table does not already match (`buckets == *table` and the
    /// bucket array is non-null is the only early return).
    pub needs_rehash: bool,
    /// The truncated element count the load-factor comparison saw.
    pub truncated_count: u32,
}

/// The float-to-32-bit truncation helper called at `FUN_00c86e10`.
///
/// The helper converts the `f32` in `XMM0` to a 64-bit integer and returns
/// `0x8000000000000000` outside `[-2^63, 2^63)` (including NaN).  The caller
/// keeps the low 32 bits, so such values become zero.
pub fn a59_trunc_f32_to_u32_low(value: f32) -> u32 {
    let wide = value as f64;
    if wide.is_nan() || wide.abs() >= 9.223_372_036_854_776e18 {
        return 0;
    }
    (wide.trunc() as i64 as u64) as u32
}

/// `0x006080d0` (`FUN_006080d0`, 156 bytes): choose the next bucket count.
///
/// ```c
/// count = table[2];
/// truncated = (u32)(f32)count;               // helper at FUN_00c86e10
/// candidate = truncated + 1;
/// if ((f32)count <= (f32)truncated * max_load) candidate = truncated;
/// if (candidate < requested) candidate = requested;
/// buckets = 8;
/// while (8 < candidate && buckets < candidate) buckets *= 2;
/// if (buckets == table[0] && table[1] != 0) return;   // already sized
/// ...                                                // grow the table
/// ```
///
/// The `(double)value + DAT_00d33180[-(value >> 31)]` expression in the
/// decompile is the compiler's unsigned-to-double conversion (`+0.0` or
/// `+2^32`); the comparisons themselves are single-precision.  A candidate
/// above `2^31` makes the retail doubling loop wrap to zero forever, so the
/// translation reports [`A59Fault::UnrepresentableBuckets`] instead.
pub fn a59_rehash_plan(
    requested: u32,
    element_count: u32,
    max_load: f32,
    current_buckets: u32,
    buckets_allocated: bool,
) -> Result<A59RehashPlan, A59Fault> {
    let truncated = a59_trunc_f32_to_u32_low(element_count as f32);
    let mut candidate = truncated.wrapping_add(1);
    if (element_count as f32) <= (truncated as f32) * max_load {
        candidate = truncated;
    }
    if candidate < requested {
        candidate = requested;
    }
    let mut buckets = A59_MIN_BUCKETS;
    while buckets < candidate {
        if buckets > A59_BUCKET_LIMIT / 2 {
            return Err(A59Fault::UnrepresentableBuckets(candidate));
        }
        buckets = buckets.wrapping_mul(2);
    }
    Ok(A59RehashPlan {
        buckets,
        needs_rehash: !(buckets == current_buckets && buckets_allocated),
        truncated_count: truncated,
    })
}

// ---------------------------------------------------------------------------
// list sort bin machinery
// ---------------------------------------------------------------------------

/// Bins used by the recovered list sort (`0x40` list heads of `0xc` bytes).
pub const A59_SORT_BINS: usize = 0x40;

/// Size of one bin head: next, previous, element count (`0xc`).
pub const A59_SORT_SLOT_STRIDE: u32 = 0xc;

/// One bin head.  The retail stores pointers; indices model the same
/// self-linked list without inventing an address space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A59ListHead {
    /// Next node index (the head itself when empty).
    pub next: u32,
    /// Previous node index (the head itself when empty).
    pub prev: u32,
    /// Number of nodes chained on the head.
    pub count: u32,
}

/// `0x00609c50` (`FUN_00609c50`, 15 bytes): initialise one list head.
///
/// ```c
/// *(int *)this = this;
/// *(int *)(this + 4) = this;
/// *(u32 *)(this + 8) = 0;
/// ```
///
/// `index` is the head's own index, used for both self links.
pub fn a59_list_head_init(index: u32) -> A59ListHead {
    A59ListHead {
        next: index,
        prev: index,
        count: 0,
    }
}

/// `0x00609790` (`FUN_00609790`, 149 bytes): `std::list::sort` prologue.
///
/// ```c
/// if (1 < this[2]) {                     // element count
///     carry[0] = carry[1] = &carry; carry[2] = 0;
///     __ehvec_ctor(counter, 0xc, 0x40, init_head, destroy_head);
///     ...                                // body at 0x0060980d
/// }
/// ```
///
/// `__ehvec_ctor` constructs the 64 `0xc`-byte heads with [`a59_list_head_init`]
/// and registers their destructor; the sort body then merges the runs.  A list
/// of zero or one element skips the whole construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A59SortEntry {
    /// Bins the entry constructs.
    pub bins: usize,
    /// Stride of one bin head.
    pub stride: u32,
    /// Whether the sort body runs (`1 < len`).
    pub sorts: bool,
}

/// `0x00609790`: see [`A59SortEntry`].
pub fn a59_sort_entry(len: usize) -> A59SortEntry {
    A59SortEntry {
        bins: A59_SORT_BINS,
        stride: A59_SORT_SLOT_STRIDE,
        sorts: len > 1,
    }
}

fn a59_merge_runs(
    left: &[(u32, u32)],
    right: &[(u32, u32)],
    less: &impl Fn(u32, u32) -> bool,
) -> Vec<(u32, u32)> {
    let mut merged = Vec::with_capacity(left.len() + right.len());
    let mut left_index = 0usize;
    let mut right_index = 0usize;
    while left_index < left.len() && right_index < right.len() {
        if less(right[right_index].0, left[left_index].0) {
            merged.push(right[right_index]);
            right_index += 1;
        } else {
            merged.push(left[left_index]);
            left_index += 1;
        }
    }
    merged.extend_from_slice(&left[left_index..]);
    merged.extend_from_slice(&right[right_index..]);
    merged
}

/// `0x0060980d` (`FUN_0060980d`, 1028 bytes): the 64-bin merge body of
/// `std::list::sort`, starting inside `0x00609790`'s range.
///
/// The recovered code is the SGI/MSVC counter sort: take the first node, merge
/// it into bin 0 while that bin is occupied, carry the merged run upward, drop
/// the carry into the first free bin and bump the fill count; then merge the
/// remaining bins from low to high and return the last bin.  The comparator
/// (`FUN_0060a070` at `FUN_0060a070` in the same table, outside this window) is
/// an input here, so the translation is the generic stable merge order.
///
/// The payload pair is `(key, value)`; equal keys keep their input order, which
/// is what the list splice order of the recovered body produces.
pub fn a59_list_sort<F: Fn(u32, u32) -> bool>(entries: &[(u32, u32)], less: F) -> Vec<(u32, u32)> {
    let mut bins: [Vec<(u32, u32)>; A59_SORT_BINS] = std::array::from_fn(|_| Vec::new());
    let mut fill = 0usize;
    for &entry in entries {
        let mut carry = vec![entry];
        let mut slot = 0usize;
        while slot < fill && !bins[slot].is_empty() {
            carry = a59_merge_runs(&bins[slot], &carry, &less);
            bins[slot] = Vec::new();
            slot += 1;
        }
        bins[slot] = carry;
        if slot == fill {
            fill += 1;
        }
    }
    for slot in 1..fill {
        let merged = a59_merge_runs(&bins[slot], &bins[slot - 1], &less);
        bins[slot] = merged;
        bins[slot - 1] = Vec::new();
    }
    if fill == 0 {
        Vec::new()
    } else {
        bins[fill - 1].clone()
    }
}

// ---------------------------------------------------------------------------
// container descriptor
// ---------------------------------------------------------------------------

/// Request passed to the capacity helper by `0x00609c60` (`0x19` = 25).
pub const A59_HASH_REQUEST: u32 = 0x19;

/// Nine-word descriptor written by `0x00609c60`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A59HashState {
    /// The nine words in store order (`+0x0`..`+0x20`).
    pub words: [u32; 9],
    /// Capacity request issued after the stores.
    pub request: u32,
    /// Capacity result for the word at `+0x8` (zero before the request).
    pub capacity: A59Capacity,
}

/// `0x00609c60` (`FUN_00609c60`, 170 bytes): initialise the nine-word
/// container state and reserve 25 elements.
///
/// ```c
/// this[0] = 7; this[1] = 8; this[2] = 0; this[3] = 0;
/// this[4] = 0x3f800000; this[5] = 8; this[6] = 0; this[7] = 0;
/// this[8] = 0x3f800000;
/// FUN_00538810(0x19);
/// ```
///
/// `0x3f800000` is `1.0f`; the descriptor therefore carries the mask/count pair
/// `7`/`8`, two empty slots, a load factor of 1.0 and a second `8`; the helper
/// then grows the word at `+0x8` from `0` to `0x19`.  The cached body does not
/// show the `lea ecx` before the helper, so the base object is taken as the
/// capacity owner, matching the `+0x8` word the helper reads.
pub fn a59_hash_state_init() -> A59HashState {
    let words = [7, 8, 0, 0, 0x3f80_0000, 8, 0, 0, 0x3f80_0000];
    A59HashState {
        words,
        request: A59_HASH_REQUEST,
        capacity: a59_capacity_growth(words[2], A59_HASH_REQUEST),
    }
}

#[cfg(test)]
mod tests_agent59 {
    use super::*;

    // -- 0x006080d0 ---------------------------------------------------------

    #[test]
    fn a59_truncation_matches_the_helper() {
        assert_eq!(a59_trunc_f32_to_u32_low(0.0), 0);
        assert_eq!(a59_trunc_f32_to_u32_low(12.75), 12);
        assert_eq!(a59_trunc_f32_to_u32_low(-1.0), u32::MAX);
        assert_eq!(a59_trunc_f32_to_u32_low(f32::NAN), 0);
        assert_eq!(a59_trunc_f32_to_u32_low(f32::INFINITY), 0);
    }

    #[test]
    fn a59_rehash_plan_rounds_up_to_powers_of_two() {
        // load 1.0: candidate = element count, then max with the request.
        let plan = a59_rehash_plan(10, 100, 1.0, 64, true).unwrap();
        assert_eq!(plan.truncated_count, 100);
        assert_eq!(plan.buckets, 128);
        assert!(plan.needs_rehash);

        // Same count, already sized: no rehash.
        let plan = a59_rehash_plan(0, 100, 1.0, 128, true).unwrap();
        assert!(!plan.needs_rehash);
        // ...but a missing bucket array still needs one.
        assert!(
            a59_rehash_plan(0, 100, 1.0, 128, false)
                .unwrap()
                .needs_rehash
        );

        // A load below one adds one to the candidate (truncated + 1).
        let plan = a59_rehash_plan(0, 100, 0.5, 0, false).unwrap();
        assert_eq!(plan.truncated_count, 100);
        assert_eq!(plan.buckets, 128);

        // Small requests clamp to the minimum of eight buckets.
        assert_eq!(a59_rehash_plan(1, 0, 1.0, 0, false).unwrap().buckets, 8);
        assert_eq!(A59_MIN_BUCKETS, 8);
    }

    #[test]
    fn a59_rehash_plan_reproduces_the_large_count_truncation() {
        // 0xffffffff as f32 is 2^32; the helper's low word is 0, so the
        // load comparison fails and the candidate is 0 + 1.
        let plan = a59_rehash_plan(0, u32::MAX, 1.0, 0, false).unwrap();
        assert_eq!(plan.truncated_count, 0);
        assert_eq!(plan.buckets, 8);

        // Above 2^31 the retail doubling loop would wrap; report the fault.
        assert_eq!(
            a59_rehash_plan(0x8000_0001, 0, 1.0, 0, false),
            Err(A59Fault::UnrepresentableBuckets(0x8000_0001))
        );
        // Exactly 2^31 still fits.
        assert_eq!(
            a59_rehash_plan(0x8000_0000, 0, 1.0, 0, false)
                .unwrap()
                .buckets,
            0x8000_0000
        );
    }

    // -- 0x00608230 / 0x00608c20 ------------------------------------------

    #[test]
    fn a59_capacity_growth_uses_the_one_and_a_half_policy() {
        // 15 -> max(15 + 7, 25) = 25, tag bit preserved.
        let grown = a59_capacity_growth(A59_INLINE_CAPACITY_WORD, 0x19);
        assert_eq!(
            grown,
            A59Capacity {
                word: 0x8000_0019,
                grew: true
            }
        );
        // 15 is enough for a 12-byte message: unchanged.
        let same = a59_capacity_growth(A59_INLINE_CAPACITY_WORD, 12);
        assert_eq!(
            same,
            A59Capacity {
                word: A59_INLINE_CAPACITY_WORD,
                grew: false
            }
        );
        // Heap word without the tag: 8 -> max(12, 25) = 25.
        assert_eq!(
            a59_capacity_growth(8, 0x19),
            A59Capacity {
                word: 0x19,
                grew: true
            }
        );
    }

    #[test]
    fn a59_stream_message_init_reads_the_stream_length() {
        let init = a59_stream_message_init(12);
        assert_eq!(init.reserved_len, 12);
        assert_eq!(init.header.size, 0);
        assert_eq!(init.header.data_offset, A59_STRING_DATA_OFFSET);
        assert_eq!(init.header.capacity_word, 0x8000_000f);
        assert!(!init.capacity.grew);
        assert_eq!(init.capacity.word, 0x8000_000f);

        let init = a59_stream_message_init(0x20);
        assert!(init.capacity.grew);
        assert_eq!(init.capacity.word, 0x8000_0020);
    }

    #[test]
    fn a59_error_text_message_measures_the_text() {
        let empty = a59_error_text_message("");
        assert_eq!(empty.text, "");
        assert_eq!(empty.init.reserved_len, 0);
        assert!(!empty.init.capacity.grew);

        let text = a59_error_text_message("access denied");
        assert_eq!(text.init.reserved_len, "access denied".len() as u32);
        let grown = a59_error_text_message("0123456789abcdef");
        assert!(grown.init.capacity.grew);
        // 15 + 15 / 2 = 22 already covers the 16-byte text.
        assert_eq!(grown.init.capacity.word, 0x8000_0016);
        assert_eq!(A59_INLINE_CAPACITY, 0xf);
    }

    // -- fmt wrappers ------------------------------------------------------

    #[test]
    fn a59_pack_arg_types_matches_the_recovered_masks() {
        assert_eq!(a59_pack_arg_types(&[A59FmtArg::Str("p")]), 0xd);
        assert_eq!(
            a59_pack_arg_types(&[A59FmtArg::Str("p"), A59FmtArg::Int(1)]),
            0x3d
        );
        assert_eq!(
            a59_pack_arg_types(&[A59FmtArg::Str("p"), A59FmtArg::Int(1), A59FmtArg::Uint(2)]),
            0x23d
        );
        assert_eq!(
            a59_pack_arg_types(&[A59FmtArg::Str("p"), A59FmtArg::Int(1), A59FmtArg::Int(2)]),
            0x33d
        );
        assert_eq!(
            a59_pack_arg_types(&[A59FmtArg::Str("p"), A59FmtArg::Int(1), A59FmtArg::Str("q")]),
            0xd3d
        );
        assert_eq!(
            a59_pack_arg_types(&[
                A59FmtArg::Int(1),
                A59FmtArg::Int(2),
                A59FmtArg::Str("p"),
                A59FmtArg::Int(3),
                A59FmtArg::Str("q"),
            ]),
            0xd3d33
        );
        assert_eq!(A59_TYPE_UINT, 2);
        assert_eq!(A59_TYPE_LONG_LONG, 3);
        assert_eq!(A59_TYPE_STRING, 0xd);
    }

    #[test]
    fn a59_format_wrappers_pin_the_recovered_shapes() {
        assert_eq!(a59_shape_00608430().address, 0x0060_8430);
        assert_eq!(a59_shape_00608430().types, 0xd);
        assert_eq!(a59_shape_00608430().arg_count, 1);
        assert_eq!(a59_shape_006084a0().address, 0x0060_84a0);
        assert_eq!(a59_shape_006084a0().types, 0x33d);
        assert_eq!(a59_shape_006084a0().arg_count, 3);
        assert_eq!(a59_shape_006085a0().address, 0x0060_85a0);
        assert_eq!(a59_shape_006085a0().types, 0xd3d);
        assert_eq!(a59_shape_006085a0().arg_count, 3);
        assert_eq!(a59_shape_006086b0().address, 0x0060_86b0);
        assert_eq!(a59_shape_006086b0().types, 0xd3d33);
        assert_eq!(a59_shape_006086b0().arg_count, 5);
    }

    #[test]
    fn a59_format_one_string_renders_the_missing_file_text() {
        assert_eq!(
            a59_format_one_string("File does not exist: {0}", "a.scm").unwrap(),
            "File does not exist: a.scm"
        );
        // Wrong arity or wrong type word are rejected by the shape check.
        assert_eq!(
            a59_format_with_shape(&a59_shape_00608430(), "{0}", &[]),
            Err(A59FormatShapeError::ArgCount {
                expected: 1,
                actual: 0
            })
        );
        assert_eq!(
            a59_format_with_shape(&a59_shape_00608430(), "{0}", &[A59FmtArg::Int(1)]),
            Err(A59FormatShapeError::Types {
                expected: 0xd,
                actual: 0x3
            })
        );
    }

    #[test]
    fn a59_format_three_argument_shapes_render() {
        assert_eq!(
            a59_format_string_ll_ll("A {0} {1} {2}", "a", -4, 5).unwrap(),
            "A a -4 5"
        );
        assert_eq!(
            a59_format_string_ll_string(
                "Failed to create folder(s): {0} | {1} | {2}",
                "save/",
                5,
                "access denied"
            )
            .unwrap(),
            "Failed to create folder(s): save/ | 5 | access denied"
        );
    }

    #[test]
    fn a59_format_five_argument_shape_renders_the_short_write_text() {
        let rendered = a59_format_ll_ll_string_ll_string(
            "Failed to write all data (wrote {0} / {1} bytes): {2} | {3} | {4}",
            5,
            8,
            "a.rep",
            5,
            "access denied",
        )
        .unwrap();
        assert_eq!(
            rendered,
            "Failed to write all data (wrote 5 / 8 bytes): a.rep | 5 | access denied"
        );
    }

    #[test]
    fn a59_format_reports_brace_and_index_errors() {
        assert_eq!(
            a59_format("a {0", &[A59FmtArg::Str("x")]),
            Err(A59FormatError::MissingClose)
        );
        assert_eq!(
            a59_format("a } b", &[]),
            Err(A59FormatError::UnmatchedClose)
        );
        assert_eq!(
            a59_format("{} {0}", &[A59FmtArg::Str("x")]),
            Err(A59FormatError::MixedIndexing)
        );
        assert_eq!(
            a59_format("{0} {1}", &[A59FmtArg::Str("x")]),
            Err(A59FormatError::IndexOutOfRange)
        );
        assert_eq!(
            a59_format("{{ {0} }}", &[A59FmtArg::Str("x")]).unwrap(),
            "{ x }"
        );
        assert_eq!(
            a59_format("{} {}", &[A59FmtArg::Uint(7), A59FmtArg::Int(-2)]).unwrap(),
            "7 -2"
        );
    }

    // -- exception layouts and destructors --------------------------------

    #[test]
    fn a59_error_layouts_assign_class_and_code() {
        let read = a59_read_error_layout(0x20, 0);
        assert_eq!(read.class, A59ErrorClass::Read);
        assert_eq!(read.code, Some(0));
        assert_eq!(read.prefix, [0, 0]);
        assert_eq!(read.header.data_offset, A59_ERROR_DATA_OFFSET);
        assert_eq!(read.header.size, 0);
        assert_eq!(read.header.capacity_word, 0x8000_000f);
        assert_eq!(
            read.capacity,
            A59Capacity {
                word: 0x8000_0020,
                grew: true
            }
        );

        let write = a59_write_error_layout(8, 2);
        assert_eq!(write.class, A59ErrorClass::Write);
        assert_eq!(write.code, Some(2));
        assert!(!write.capacity.grew);
        assert_eq!(write.capacity.word, 0x8000_000f);

        let runtime = a59_string_runtime_error_layout(8);
        assert_eq!(runtime.class, A59ErrorClass::StringRuntime);
        assert_eq!(runtime.code, None);
        assert_eq!(runtime.header.data_offset, 0x18);
    }

    #[test]
    fn a59_error_layouts_copy_the_source_prefix() {
        let source = A59ErrorSource {
            prefix: [0x11, 0x22],
            reserved_len: 0x40,
            code: 5,
        };
        let copied = a59_string_runtime_error_copy_layout(source);
        assert_eq!(copied.class, A59ErrorClass::StringRuntime);
        assert_eq!(copied.prefix, [0x11, 0x22]);
        assert_eq!(copied.code, None);
        // 0x40 bytes: 15 + 7 = 22 < 64, so the 1.5x step is clamped up.
        assert_eq!(copied.capacity.word, 0x8000_0040);

        let write = a59_write_error_from_source_layout(source);
        assert_eq!(write.class, A59ErrorClass::Write);
        assert_eq!(write.code, Some(5));
        assert_eq!(write.prefix, [0x11, 0x22]);
    }

    #[test]
    fn a59_exception_deleting_dtor_checks_the_capacity_tag() {
        assert_eq!(
            a59_exception_deleting_dtor(0x8000_000f, 0),
            Ok(A59Dtor {
                destroyed: true,
                deallocated: false
            })
        );
        assert_eq!(
            a59_exception_deleting_dtor(0x8000_000f, 1),
            Ok(A59Dtor {
                destroyed: true,
                deallocated: true
            })
        );
        // Bit 31 clear is the retail invariant trap.
        assert_eq!(
            a59_exception_deleting_dtor(0x0f, 0),
            Err(A59Fault::CapacityWord(0x0f))
        );
    }

    #[test]
    fn a59_file_output_stream_dtor_honours_the_delete_flag() {
        assert_eq!(
            a59_file_output_stream_dtor(0),
            A59Dtor {
                destroyed: true,
                deallocated: false
            }
        );
        assert!(a59_file_output_stream_dtor(3).deallocated);
    }

    // -- copyable function thunks -----------------------------------------

    #[test]
    fn a59_handle_call_body_forwards_the_captured_handle() {
        let mut seen = None;
        let result = a59_handle_call_body(7, |handle, first, second| {
            seen = Some((handle, first, second));
            0x1234_u64
        });
        assert_eq!(seen, Some((7, 0, 0)));
        assert_eq!(result, 0x1234);
    }

    #[test]
    fn a59_function_object_vtables_name_the_lambdas() {
        let handle = a59_object_impl_vtable();
        assert!(
            handle
                .object_vtable
                .contains("lambda_95e70d69b51b22763c90d4eab9ccccfc")
        );
        assert_eq!(handle.base_vtable, None);
        assert_eq!(handle.captured, None);

        let size = a59_object_impl_with_handle(0xabc);
        assert!(
            size.object_vtable
                .contains("lambda_e1f546344bc49b26d3017301900dd4c7")
        );
        assert_eq!(size.base_vtable, Some(A59_VTABLE_FUNCTION_IMPL));
        assert_eq!(size.captured, Some(0xabc));

        assert!(A59_VTABLE_FUNCTION_IMPL.contains("unsigned_int,HOSFILE__*"));
        assert_eq!(
            a59_reset_object_impl(),
            A59CallableReset {
                vtable: A59_VTABLE_OBJECT_HANDLE,
                reset: true
            }
        );
    }

    // -- predicates --------------------------------------------------------

    #[test]
    fn a59_path_predicates_follow_the_branches() {
        assert!(!a59_paths_differ(true, true, false));
        assert!(a59_paths_differ(true, false, false));
        assert!(!a59_paths_differ(true, false, true));
        assert!(a59_paths_differ(false, true, false));

        assert!(!a59_file_accepted(true, true));
        assert!(a59_file_accepted(true, false));
        assert!(a59_file_accepted(false, true));
    }

    // -- open / write paths ------------------------------------------------

    #[test]
    fn a59_open_read_reports_missing_files() {
        let missing = a59_open_read(-1, "maps/a.scm", 0).unwrap();
        assert_eq!(missing.handle, -1);
        assert_eq!(missing.handle_callback, None);
        let error = missing.error.unwrap();
        assert_eq!(error.class, A59ErrorClass::Read);
        assert_eq!(error.code, 0);
        assert_eq!(error.message, "File does not exist: maps/a.scm");

        let opened = a59_open_read(12, "maps/a.scm", 0).unwrap();
        assert_eq!(opened.handle, 12);
        assert_eq!(opened.error, None);
    }

    #[test]
    fn a59_open_read_selects_the_tagged_callable() {
        assert_eq!(a59_select_callable(0), A59CallableRef::StackCopy);
        assert_eq!(a59_select_callable(0xabc), A59CallableRef::StackCopy);
        assert_eq!(a59_select_callable(0xabd), A59CallableRef::Heap(0xabc));
        assert_eq!(a59_select_callable(1), A59CallableRef::Heap(0));
        assert_eq!(
            a59_open_read(3, "p", 0xabd).unwrap().handle_callback,
            Some(A59CallableRef::Heap(0xabc))
        );
    }

    #[test]
    fn a59_missing_regular_file_plan_allocates_for_regular_files() {
        let regular = a59_missing_regular_file_plan("a.txt", true).unwrap();
        assert!(regular.regular_file);
        assert_eq!(regular.allocation, Some(0x10));
        assert_eq!(regular.error.message, "File does not exist: a.txt");
        assert_eq!(regular.error.code, 0);

        let other = a59_missing_regular_file_plan("a.txt", false).unwrap();
        assert_eq!(other.allocation, None);
        assert_eq!(other.error.class, A59ErrorClass::Read);
    }

    #[test]
    fn a59_read_stream_open_renders_the_open_failure() {
        let opened = a59_read_stream_open(true, 0x20, "a.scm").unwrap();
        assert!(opened.opened);
        assert_eq!(opened.error, None);
        let init = opened.message.unwrap();
        assert_eq!(init.reserved_len, 0x20);
        assert!(init.capacity.grew);

        let failed = a59_read_stream_open(false, 0, "a.scm").unwrap();
        assert!(!failed.opened);
        assert_eq!(failed.message, None);
        let error = failed.error.unwrap();
        assert_eq!(error.class, A59ErrorClass::Read);
        assert_eq!(error.code, 1);
        assert_eq!(error.message, "Failed to open file: a.scm");

        // The second variant has the same outcome.
        let variant = a59_read_stream_open_variant(false, 0, "a.scm").unwrap();
        assert_eq!(variant.error, Some(error));
    }

    #[test]
    fn a59_write_all_plan_orders_the_failure_checks() {
        let dir = a59_write_all_plan(
            "save/a.rep",
            false,
            false,
            0,
            false,
            0,
            0,
            8,
            5,
            "access denied",
        )
        .unwrap();
        assert!(!dir.success);
        let error = dir.failure.unwrap().error().cloned().unwrap();
        assert_eq!(error.code, 0);
        assert_eq!(
            error.message,
            "Failed to create folder(s): save/a.rep | 5 | access denied"
        );

        // Read-only regular file: no WriteError code.
        let readonly =
            a59_write_all_plan("save/a.rep", true, true, 1, true, 8, 8, 8, 0, "").unwrap();
        assert_eq!(
            readonly.failure,
            Some(A59WriteFailure::ReadOnly(
                "Existing file is read only: save/a.rep".to_string()
            ))
        );
        // The attribute branch is only reached for a regular file.
        assert!(
            a59_write_all_plan("save/a.rep", true, false, 1, true, 8, 8, 8, 0, "")
                .unwrap()
                .success
        );

        let open =
            a59_write_all_plan("save/a.rep", true, false, 0x80, false, 0, 8, 8, 3, "bad").unwrap();
        assert_eq!(open.failure.unwrap().error().unwrap().code, 1);

        let short =
            a59_write_all_plan("save/a.rep", true, false, 0x80, true, 5, 8, 8, 0, "").unwrap();
        let error = short.failure.unwrap().error().cloned().unwrap();
        assert_eq!(error.code, 2);
        assert_eq!(
            error.message,
            "Failed to write all data (wrote 5 / 8 bytes): save/a.rep | 0 | "
        );

        let ok = a59_write_all_plan("save/a.rep", true, false, 0x80, true, 8, 8, 8, 0, "").unwrap();
        assert!(ok.success);
        assert_eq!(ok.failure, None);
    }

    #[test]
    fn a59_write_catch_plan_nests_the_access_denied_message() {
        let denied = a59_write_catch_plan("a.rep", 5, "denied").unwrap();
        assert!(denied.access_denied);
        assert_eq!(denied.error, None);
        assert_eq!(
            denied.thrown.unwrap(),
            "Failed to write to file (access denied): Failed to write to file: a.rep | 5 | \
             denied | 5 | denied"
        );

        let other = a59_write_catch_plan("a.rep", 13, "disk full").unwrap();
        assert!(!other.access_denied);
        assert_eq!(other.thrown, None);
        let error = other.error.unwrap();
        assert_eq!(error.class, A59ErrorClass::Write);
        assert_eq!(error.code, 5);
        assert_eq!(
            error.message,
            "Failed to write to file: a.rep | 13 | disk full"
        );
    }

    // -- sort machinery ----------------------------------------------------

    #[test]
    fn a59_list_sort_is_stable_and_matches_the_order() {
        let mut entries = [
            (3u32, 0u32),
            (1, 1),
            (3, 2),
            (2, 3),
            (1, 4),
            (3, 5),
            (0, 6),
            (2, 7),
            (1, 8),
        ];
        let sorted = a59_list_sort(&entries, |a, b| a < b);
        assert_eq!(
            sorted,
            vec![
                (0, 6),
                (1, 1),
                (1, 4),
                (1, 8),
                (2, 3),
                (2, 7),
                (3, 0),
                (3, 2),
                (3, 5),
            ]
        );
        // `>=` in the comparator makes the sort keep the reverse order.
        entries.reverse();
        let descending = a59_list_sort(&entries, |a, b| a > b);
        assert!(descending.windows(2).all(|pair| pair[0].0 >= pair[1].0));

        let mut more: Vec<(u32, u32)> = (0..200u32).map(|i| ((i * 37) % 11, i)).collect();
        let expected = {
            let mut copy = more.clone();
            copy.sort_by_key(|entry| entry.0);
            copy
        };
        more = a59_list_sort(&more, |a, b| a < b);
        assert_eq!(more, expected);

        assert!(a59_list_sort(&[], |a, b| a < b).is_empty());
        assert_eq!(a59_list_sort(&[(4, 9)], |a, b| a < b), vec![(4, 9)]);
    }

    #[test]
    fn a59_sort_entry_skips_short_lists() {
        assert!(!a59_sort_entry(0).sorts);
        assert!(!a59_sort_entry(1).sorts);
        assert!(a59_sort_entry(2).sorts);
        assert_eq!(a59_sort_entry(2).bins, 64);
        assert_eq!(a59_sort_entry(2).stride, 0xc);
        assert_eq!(A59_SORT_BINS, 0x40);
        assert_eq!(A59_SORT_SLOT_STRIDE, 0xc);
    }

    #[test]
    fn a59_list_head_and_hash_state_initialisation() {
        assert_eq!(
            a59_list_head_init(3),
            A59ListHead {
                next: 3,
                prev: 3,
                count: 0
            }
        );

        let state = a59_hash_state_init();
        assert_eq!(state.words, [7, 8, 0, 0, 0x3f80_0000, 8, 0, 0, 0x3f80_0000]);
        assert_eq!(state.request, 0x19);
        assert_eq!(
            state.capacity,
            A59Capacity {
                word: 0x19,
                grew: true
            }
        );
        assert_eq!(A59_HASH_REQUEST, 0x19);
    }
}
