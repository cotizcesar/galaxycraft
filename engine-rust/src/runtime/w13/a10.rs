#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave13/agent10.rs
// ===========================================================================
// Wave 13 translation batch 10: the JSON-subset data parser in
// `0x0066e000..=0x0066ffff`.
//
// The retail executable contains a small JSON-like reader used by the data
// document loader.  It is not a general parser: values are a subset of JSON
// (objects, strings, numbers, `true`, `false`, `null`, and arrays), and the
// object keys are byte strings.  The routines translated below are the
// lexical core: cursor access, whitespace skipping, string escapes, number
// conversion, object assembly, and the one-value document check.
//
// ## Recovered layout
//
// The input cursor (a `ZIO` in the original) is three pointers:
//
// | offset | field |
// |--------|-------|
// | `+0x00` | `p`, next byte to read |
// | `+0x04` | `base`, start of the buffer (used for error offsets) |
// | `+0x08` | `end`, one past the last byte |
//
// A byte at `p == end` reads as zero but is *not* advanced; this is how the
// original distinguishes "end of buffer" from a NUL byte in the data.
//
// The lexer state keeps a byte buffer at `+0x0c` (write pointer) and
// `+0x10` (end pointer), and the parser status at `+0x18` (code) and
// `+0x1c` (byte offset from `base`).  Code `0` means success.  The codes
// observed in this window are:
//
// | code | meaning |
// |------|---------|
// | `3`  | malformed literal / bad keyword / bad number start |
// | `4`  | object key is not a quoted string |
// | `5`  | missing `:` after an object key |
// | `6`  | missing `,` or `}` after an object member |
// | `8`  | bad digit in a `\uXXXX` escape |
// | `9`  | bad or missing UTF-16 surrogate |
// | `10` | unknown backslash escape |
// | `11` | end of input inside a string |
// | `12` | raw control byte inside a string |
// | `13` | exponent out of range |
// | `14` | missing digit after the decimal point |
// | `15` | missing digit after the exponent marker |
// | `16` | string interning failure (not reachable in this pure model) |
//
// A numeric value is a 16-byte record: an eight-byte payload at `+0x00` and
// a two-byte tag at `+0x0e` (the remaining bytes are zero).  [`W13a10Number`]
// keeps exactly those two meaningful pieces so the tags written by the
// number routine can be asserted directly.  The raw tag literals are:
//
// | tag | written for |
// |-----|-------------|
// | `0x00b6` | negative value that fits in `i32` |
// | `0x01f6` | non-negative value below `0x8000_0000` |
// | `0x01d6` | non-negative value in `0x8000_0000..=0xffff_ffff` |
// | `0x0096` | negative 64-bit value |
// | `0x0116` | positive 64-bit value with bit 63 set |
// | `0x0196` | positive 64-bit value with bit 63 clear and a non-zero high word |
// | `0x0216` | double |
//
// Seven rows in this window are compiler-generated SEH fragments, bare
// frame-pointer fragments, or bodies that stop at a call Ghidra mislabelled
// as non-returning; they are listed in the batch report rather than guessed
// at here.
//
// ## Translation notes
//
// * The integer-to-double step in the number routine is an inlined
//   conversion helper whose decompiled body is empty.  The translation
//   converts the accumulated `u64`/`u32` magnitude with Rust's `as f64`,
//   which rounds to nearest, and applies the sign only after scaling.
// * The static powers-of-ten table is reconstructed from the recovered
//   index range `0..=308`; every entry is the correctly rounded decimal
//   literal the compiler would emit for `1eN`.
// * The string routine's trailing interning helper always reports success,
//   so status `16` is not reachable and only the decoded bytes are
//   returned.
// * The object routine returns its key/value list instead of building a Lua
//   table node array; the table finalisation has no failure path in the
//   recovered body.
// * The document routine's final stack-restore/allocator tail is outside the
//   recovered status logic and is not modelled.
// * The two getters apply their second lookup to the first value's table
//   header.  The safe model returns nil (or an empty byte string) when that
//   first value is not a table, instead of reinterpreting a non-table
//   payload as a node array.
//
// ## Style note
//
// The original parser pushes values on a caller-owned stack.  These
// translations return values instead, and the object/document routines take
// a [`W13a10ValueParser`] so the caller can supply the array arm (whose
// routine lies outside this window).  Nothing here touches `World`,
// `SimUnit`, the renderer, or the client.
//
/// Status code for a malformed literal.
pub const W13A10_ERR_SYNTAX: u32 = 3;
/// Status code for a non-string object key.
pub const W13A10_ERR_EXPECTED_STRING: u32 = 4;
/// Status code for a missing `:`.
pub const W13A10_ERR_EXPECTED_COLON: u32 = 5;
/// Status code for a missing `,` or `}`.
pub const W13A10_ERR_EXPECTED_BRACE_OR_COMMA: u32 = 6;
/// Status code for a bad hexadecimal digit in `\uXXXX`.
pub const W13A10_ERR_INVALID_UNICODE_ESCAPE: u32 = 8;
/// Status code for a bad UTF-16 surrogate pair.
pub const W13A10_ERR_INVALID_SURROGATE: u32 = 9;
/// Status code for an unknown backslash escape.
pub const W13A10_ERR_INVALID_ESCAPE: u32 = 10;
/// Status code for end of input inside a string.
pub const W13A10_ERR_UNFINISHED_STRING: u32 = 11;
/// Status code for a raw control byte inside a string.
pub const W13A10_ERR_CONTROL_CHARACTER: u32 = 12;
/// Status code for an exponent that is too large.
pub const W13A10_ERR_EXPONENT_OVERFLOW: u32 = 13;
/// Status code for a missing fraction digit.
pub const W13A10_ERR_MISSING_FRACTION_DIGIT: u32 = 14;
/// Status code for a missing exponent digit.
pub const W13A10_ERR_MISSING_EXPONENT_DIGIT: u32 = 15;

/// Number tag: negative value that fits in `i32`.
pub const W13A10_TAG_NEG_I32: u16 = 0x00b6;
/// Number tag: non-negative value below `0x8000_0000`.
pub const W13A10_TAG_NONNEG_I32: u16 = 0x01f6;
/// Number tag: non-negative value in `0x8000_0000..=0xffff_ffff`.
pub const W13A10_TAG_POS_U32: u16 = 0x01d6;
/// Number tag: negative 64-bit value.
pub const W13A10_TAG_NEG_I64: u16 = 0x0096;
/// Number tag: positive 64-bit value with bit 63 set.
pub const W13A10_TAG_POS_U64_HIGH: u16 = 0x0116;
/// Number tag: positive 64-bit value with bit 63 clear and a non-zero high word.
pub const W13A10_TAG_POS_I64: u16 = 0x0196;
/// Number tag: double.
pub const W13A10_TAG_DOUBLE: u16 = 0x0216;

/// Short-string tag written when a string is at most thirteen bytes.
pub const W13A10_TAG_SHORT_STRING: u16 = 0x1c05;
/// Long-string tag written when a string is at least fourteen bytes.
pub const W13A10_TAG_LONG_STRING: u16 = 0x0c05;

/// Number of bytes of a short string that fit in the value record.
pub const W13A10_SHORT_STRING_BYTES: usize = 0x0d;

/// Maximum decimal exponent handled by the number routine.
pub const W13A10_MAX_EXPONENT: i32 = 0x134;

/// A byte cursor shaped like the original `ZIO` input stream.
///
/// Offsets reported by the parser are byte offsets from the start of `data`,
/// exactly like the original's `p - base` arithmetic.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a10Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> W13a10Cursor<'a> {
    /// Creates a cursor at the start of `data`.
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Current byte offset from the start of the buffer.
    pub const fn position(&self) -> usize {
        self.pos
    }

    /// Whether `p` has reached `end`.
    pub const fn at_end(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Raw cursor byte without the end-of-buffer special case.
    fn byte_at_cursor(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    /// Advances `p` by one byte unless it already equals `end`.
    fn advance(&mut self) {
        if !self.at_end() {
            self.pos += 1;
        }
    }
}

/// Parser status: a code (`0` = success) and the byte offset of the failure.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a10Error {
    /// Status code.  `0` means success.
    pub code: u32,
    /// Byte offset from the start of the input.
    pub offset: usize,
}

impl W13a10Error {
    /// Whether the status is success.
    pub const fn is_ok(&self) -> bool {
        self.code == 0
    }

    /// Records `code` at `offset`.  The original writes the failure site
    /// unconditionally once it decides to fail.
    pub fn set(&mut self, code: u32, offset: usize) {
        self.code = code;
        self.offset = offset;
    }
}

/// `0x0066fe20`: returns the byte under the cursor, or zero at `end`.
///
/// The original never advances here, so a NUL byte and the end of the buffer
/// are distinguishable only by the separate cursor-position checks performed
/// by the callers.
pub fn w13a10_peek(cur: &W13a10Cursor<'_>) -> u8 {
    cur.byte_at_cursor().unwrap_or(0)
}

/// `0x0066ff40`: returns the byte under the cursor and advances `p`.
///
/// Returns zero without advancing when `p == end`.  This is the original's
/// "read one character" primitive; [`w13a10_peek`] is the non-consuming
/// variant.
pub fn w13a10_getc(cur: &mut W13a10Cursor<'_>) -> u8 {
    let value = w13a10_peek(cur);
    cur.advance();
    value
}

/// `0x0066f3c0`: consumes spaces, newlines, carriage returns and tabs.
///
/// The original returns nothing and leaves the cursor on the first other
/// byte; this translation returns that byte for convenience.  A NUL byte and
/// the end of the buffer both stop the loop.
pub fn w13a10_skip_whitespace(cur: &mut W13a10Cursor<'_>) -> u8 {
    loop {
        let c = w13a10_peek(cur);
        if !matches!(c, b' ' | b'\n' | b'\r' | b'\t') {
            return c;
        }
        cur.advance();
    }
}

/// Decoded replacement for a JSON backslash escape, or `None` for the
/// `\uXXXX` form and for escapes the table does not know.
///
/// The original indexes a 256-byte lookup table.  The table contents are not
/// present in the decompiled body, but the call sites and the surrounding
/// string syntax identify the standard JSON set: `"`, `\`, `/`, `b`, `f`,
/// `n`, `r`, `t`.  The `u` entry must be zero because the routine takes a
/// separate branch for it.
const fn w13a10_escape_value(c: u8) -> Option<u8> {
    match c {
        b'"' => Some(b'"'),
        b'\\' => Some(b'\\'),
        b'/' => Some(b'/'),
        b'b' => Some(0x08),
        b'f' => Some(0x0c),
        b'n' => Some(b'\n'),
        b'r' => Some(b'\r'),
        b't' => Some(b'\t'),
        _ => None,
    }
}

/// Appends the UTF-8 encoding of `cp` to `out`.
///
/// This is the encoding shape used by the string routine: one byte below
/// `0x80`, two below `0x800`, three below `0x10000`, four otherwise.  It
/// deliberately takes a `u32` rather than a `char`, because the original
/// encodes a lone low surrogate without rejecting it.
fn w13a10_push_utf8(out: &mut Vec<u8>, cp: u32) {
    if cp < 0x80 {
        out.push(cp as u8);
    } else if cp < 0x800 {
        out.push(0xc0 | (cp >> 6) as u8);
        out.push(0x80 | (cp & 0x3f) as u8);
    } else if cp < 0x10000 {
        out.push(0xe0 | (cp >> 12) as u8);
        out.push(0x80 | ((cp >> 6) & 0x3f) as u8);
        out.push(0x80 | (cp & 0x3f) as u8);
    } else {
        out.push(0xf0 | (cp >> 18) as u8);
        out.push(0x80 | ((cp >> 12) & 0x3f) as u8);
        out.push(0x80 | ((cp >> 6) & 0x3f) as u8);
        out.push(0x80 | (cp & 0x3f) as u8);
    }
}

/// `0x0066e120`: reads exactly four hexadecimal digits and returns their
/// value.
///
/// The cursor is on the first digit on entry and is left after the fourth
/// digit on success.  On a bad digit the routine does not advance past it,
/// writes status `8` at `token_start`, and returns `None`.  End of input
/// reads as zero and therefore also fails with status `8`.  The 32-bit
/// accumulator wraps exactly like the original.
pub fn w13a10_parse_hex4(
    cur: &mut W13a10Cursor<'_>,
    token_start: usize,
    err: &mut W13a10Error,
) -> Option<u32> {
    let mut value = 0u32;
    for _ in 0..4 {
        let c = w13a10_peek(cur);
        let digit = match c {
            b'0'..=b'9' => u32::from(c - b'0'),
            b'A'..=b'F' => u32::from(c - b'A') + 10,
            b'a'..=b'f' => u32::from(c - b'a') + 10,
            _ => {
                err.set(W13A10_ERR_INVALID_UNICODE_ESCAPE, token_start);
                return None;
            }
        };
        cur.advance();
        value = value.wrapping_mul(16).wrapping_add(digit);
    }
    Some(value)
}

/// One numeric value as the original stores it: the eight-byte payload and
/// the two-byte tag at record offset `+0x0e`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct W13a10Number {
    /// `+0x00`: raw eight-byte payload.  For a double this is the IEEE-754
    /// bit pattern; for an integer it is the two's-complement value.
    pub payload: u64,
    /// `+0x0e`: the exact 16-bit tag written by the routine.
    pub tag: u16,
}

impl W13a10Number {
    /// Builds a number record from its raw payload and tag.
    pub const fn from_parts(payload: u64, tag: u16) -> Self {
        Self { payload, tag }
    }

    /// Builds a double record (tag `0x0216`).
    pub fn from_double(value: f64) -> Self {
        Self {
            payload: value.to_bits(),
            tag: W13A10_TAG_DOUBLE,
        }
    }

    /// Returns the payload as a signed integer.
    pub const fn as_i64(&self) -> i64 {
        self.payload as i64
    }

    /// Returns the payload as an unsigned integer.
    pub const fn as_u64(&self) -> u64 {
        self.payload
    }

    /// Returns the double payload when the tag is `0x0216`.
    pub fn as_f64(&self) -> Option<f64> {
        if self.tag == W13A10_TAG_DOUBLE {
            Some(f64::from_bits(self.payload))
        } else {
            None
        }
    }
}

/// Powers of ten used by the exponent-scaling step of
/// [`w13a10_parse_number`].
///
/// The original reads a static table of correctly rounded decimal literals
/// indexed by `0..=308`.  Reproducing the literals keeps the scaling step
/// bit-exact with the retail routine.
const W13A10_POW10: [f64; 309] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22, 1e23, 1e24, 1e25, 1e26, 1e27, 1e28, 1e29, 1e30, 1e31, 1e32,
    1e33, 1e34, 1e35, 1e36, 1e37, 1e38, 1e39, 1e40, 1e41, 1e42, 1e43, 1e44, 1e45, 1e46, 1e47, 1e48,
    1e49, 1e50, 1e51, 1e52, 1e53, 1e54, 1e55, 1e56, 1e57, 1e58, 1e59, 1e60, 1e61, 1e62, 1e63, 1e64,
    1e65, 1e66, 1e67, 1e68, 1e69, 1e70, 1e71, 1e72, 1e73, 1e74, 1e75, 1e76, 1e77, 1e78, 1e79, 1e80,
    1e81, 1e82, 1e83, 1e84, 1e85, 1e86, 1e87, 1e88, 1e89, 1e90, 1e91, 1e92, 1e93, 1e94, 1e95, 1e96,
    1e97, 1e98, 1e99, 1e100, 1e101, 1e102, 1e103, 1e104, 1e105, 1e106, 1e107, 1e108, 1e109, 1e110,
    1e111, 1e112, 1e113, 1e114, 1e115, 1e116, 1e117, 1e118, 1e119, 1e120, 1e121, 1e122, 1e123,
    1e124, 1e125, 1e126, 1e127, 1e128, 1e129, 1e130, 1e131, 1e132, 1e133, 1e134, 1e135, 1e136,
    1e137, 1e138, 1e139, 1e140, 1e141, 1e142, 1e143, 1e144, 1e145, 1e146, 1e147, 1e148, 1e149,
    1e150, 1e151, 1e152, 1e153, 1e154, 1e155, 1e156, 1e157, 1e158, 1e159, 1e160, 1e161, 1e162,
    1e163, 1e164, 1e165, 1e166, 1e167, 1e168, 1e169, 1e170, 1e171, 1e172, 1e173, 1e174, 1e175,
    1e176, 1e177, 1e178, 1e179, 1e180, 1e181, 1e182, 1e183, 1e184, 1e185, 1e186, 1e187, 1e188,
    1e189, 1e190, 1e191, 1e192, 1e193, 1e194, 1e195, 1e196, 1e197, 1e198, 1e199, 1e200, 1e201,
    1e202, 1e203, 1e204, 1e205, 1e206, 1e207, 1e208, 1e209, 1e210, 1e211, 1e212, 1e213, 1e214,
    1e215, 1e216, 1e217, 1e218, 1e219, 1e220, 1e221, 1e222, 1e223, 1e224, 1e225, 1e226, 1e227,
    1e228, 1e229, 1e230, 1e231, 1e232, 1e233, 1e234, 1e235, 1e236, 1e237, 1e238, 1e239, 1e240,
    1e241, 1e242, 1e243, 1e244, 1e245, 1e246, 1e247, 1e248, 1e249, 1e250, 1e251, 1e252, 1e253,
    1e254, 1e255, 1e256, 1e257, 1e258, 1e259, 1e260, 1e261, 1e262, 1e263, 1e264, 1e265, 1e266,
    1e267, 1e268, 1e269, 1e270, 1e271, 1e272, 1e273, 1e274, 1e275, 1e276, 1e277, 1e278, 1e279,
    1e280, 1e281, 1e282, 1e283, 1e284, 1e285, 1e286, 1e287, 1e288, 1e289, 1e290, 1e291, 1e292,
    1e293, 1e294, 1e295, 1e296, 1e297, 1e298, 1e299, 1e300, 1e301, 1e302, 1e303, 1e304, 1e305,
    1e306, 1e307, 1e308,
];

/// Returns the tag written for a 32-bit integer payload.
///
/// The negative branch writes `0x00b6` unless the negated value is
/// non-negative (the `-0` corner), in which case it writes `0x01f6`.  The
/// positive branch tests bit 31 to distinguish `0x01f6` from `0x01d6`.
const fn w13a10_narrow_tag(value: u32, negative: bool) -> u16 {
    if negative {
        if (value as i32) < 0 {
            W13A10_TAG_NEG_I32
        } else {
            W13A10_TAG_NONNEG_I32
        }
    } else if value & 0x8000_0000 == 0 {
        W13A10_TAG_NONNEG_I32
    } else {
        W13A10_TAG_POS_U32
    }
}

/// Returns the tag written for a wide integer payload.
///
/// `payload` is the final two's-complement 64-bit value.  The original
/// applies the same decision tree to both signs; the branches below are the
/// literal comparisons from the routine.
const fn w13a10_wide_tag(payload: u64, negative: bool) -> u16 {
    let high = (payload >> 32) as u32;
    let low = payload as u32;
    if negative {
        // The routine starts at 0x0096 and only shortens the tag when the
        // negated value happens to fit the 32-bit ranges.
        if (high as i32) < 0 {
            if (high as i32) > -2 && low > 0x7fff_ffff {
                W13A10_TAG_NEG_I32
            } else {
                W13A10_TAG_NEG_I64
            }
        } else if high == 0 {
            if low & 0x8000_0000 == 0 {
                W13A10_TAG_NONNEG_I32
            } else {
                W13A10_TAG_POS_U32
            }
        } else {
            W13A10_TAG_POS_I64
        }
    } else if (high as i32) < 0 {
        W13A10_TAG_POS_U64_HIGH
    } else if high == 0 {
        if low & 0x8000_0000 == 0 {
            W13A10_TAG_NONNEG_I32
        } else {
            W13A10_TAG_POS_U32
        }
    } else {
        W13A10_TAG_POS_I64
    }
}

/// `0x0066e1a0`: parses one numeric literal and returns the 16-byte value
/// payload plus its exact tag.
///
/// The accepted grammar is
/// `-? (0 | [1-9][0-9]*) ( '.' [0-9]+ )? ( [eE] [+-]? [0-9]+ )?`.
/// The routine first accumulates into a 32-bit integer, then into a 64-bit
/// integer when the 32-bit bound would be crossed, and finally into a
/// `double` when the 64-bit bound would be crossed.  The narrow bounds are
/// `i32::MAX` for a negative literal and `u32::MAX` for a non-negative one;
/// the wide bounds are `i64::MAX` and `u64::MAX`.  The fractional accumulator
/// keeps at most seventeen significant digits, and the exponent is applied
/// through the recovered powers-of-ten table.  A total exponent below
/// `-616` underflows to zero; `[-616, -309]` is scaled in two steps through
/// `1e308`.
///
/// On failure the cursor is left at the byte that caused the failure and the
/// status code and offset are written to `err`:
///
/// * the first character is not `0` or `1..=9` -> `3` at the literal start;
/// * no digit after `.` -> `14` at the byte after the dot;
/// * no digit after `e`/`E` and an optional sign -> `15` at the current byte;
/// * a positive exponent above `308 + kept_fraction_digits` -> `13` at the
///   literal start (the original reports the token start for this case).
///
/// `1e309` fails with status `13`; `1e-400` returns `+0.0`.  The routine
/// stops at the first byte that cannot continue a number, so `0x10` parses as
/// the value `0` and leaves the cursor on `x`.
pub fn w13a10_parse_number(
    cur: &mut W13a10Cursor<'_>,
    err: &mut W13a10Error,
) -> Option<W13a10Number> {
    let token_start = cur.position();
    let negative = if w13a10_peek(cur) == b'-' {
        cur.advance();
        true
    } else {
        false
    };

    let mut narrow: u32 = 0;
    let mut wide = false;
    let mut wide_bits: u64 = 0;
    let mut is_double = false;
    let mut double_value = 0.0f64;
    let mut significant_digits: u32 = 0;
    let mut fraction_shift: i32 = 0;

    let first = w13a10_peek(cur);
    if first == b'0' {
        cur.advance();
    } else {
        if !(b'1'..=b'9').contains(&first) {
            err.set(W13A10_ERR_SYNTAX, token_start);
            return None;
        }
        cur.advance();
        narrow = u32::from(first - b'0');

        // Narrow accumulation.  A negative literal may reach 2^31, a
        // non-negative literal may reach 2^32 - 1.
        let (narrow_limit, max_last_digit) = if negative {
            (0x0ccc_ccccu32, 8u8)
        } else {
            (0x1999_9999u32, 5u8)
        };
        loop {
            if cur.at_end() {
                break;
            }
            let c = w13a10_peek(cur);
            if !c.is_ascii_digit() {
                break;
            }
            let digit = c - b'0';
            if narrow > narrow_limit || (narrow == narrow_limit && digit > max_last_digit) {
                wide = true;
                wide_bits = u64::from(narrow);
                break;
            }
            cur.advance();
            significant_digits = significant_digits.wrapping_add(1);
            narrow = narrow.wrapping_mul(10).wrapping_add(u32::from(digit));
        }

        if wide {
            // The 64-bit bound is i64::MAX / 10 for a negative literal and
            // u64::MAX / 10 otherwise.  Crossing it starts the double path.
            let (high_limit, low_limit, max_last_digit) = if negative {
                (0x0ccc_ccccu32, 0xcccc_ccccu32, 8u8)
            } else {
                (0x1999_9999u32, 0x9999_9999u32, 5u8)
            };
            loop {
                if cur.at_end() {
                    break;
                }
                let c = w13a10_peek(cur);
                if !c.is_ascii_digit() {
                    break;
                }
                let digit = c - b'0';
                let high = (wide_bits >> 32) as u32;
                let low = wide_bits as u32;
                let crosses = high > high_limit
                    || (high == high_limit
                        && (low > low_limit || (low == low_limit && digit > max_last_digit)));
                if crosses {
                    is_double = true;
                    double_value = wide_bits as f64;
                    break;
                }
                cur.advance();
                significant_digits = significant_digits.wrapping_add(1);
                wide_bits = wide_bits.wrapping_mul(10).wrapping_add(u64::from(digit));
            }

            if is_double {
                loop {
                    if cur.at_end() {
                        break;
                    }
                    let c = w13a10_peek(cur);
                    if !c.is_ascii_digit() {
                        break;
                    }
                    if double_value >= 1.797_693_134_862_315_8e307 {
                        err.set(W13A10_ERR_EXPONENT_OVERFLOW, token_start);
                        return None;
                    }
                    cur.advance();
                    double_value = double_value * 10.0 + f64::from(c - b'0');
                }
            }
        }
    }

    // Fractional part.  The first fraction digit forces the value into the
    // double path, even when the integer part was still narrow.
    if w13a10_peek(cur) == b'.' {
        cur.advance();
        if !w13a10_peek(cur).is_ascii_digit() {
            err.set(W13A10_ERR_MISSING_FRACTION_DIGIT, cur.position());
            return None;
        }
        if !is_double {
            double_value = if wide {
                wide_bits as f64
            } else {
                f64::from(narrow)
            };
            is_double = true;
        }
        loop {
            if cur.at_end() {
                break;
            }
            let c = w13a10_peek(cur);
            if !c.is_ascii_digit() {
                break;
            }
            cur.advance();
            if significant_digits < 0x11 {
                fraction_shift = fraction_shift.wrapping_sub(1);
                double_value = double_value * 10.0 + f64::from(c - b'0');
                if double_value > 0.0 {
                    significant_digits = significant_digits.wrapping_add(1);
                }
            }
        }
    }

    // Optional decimal exponent.
    let mut exponent: i32 = 0;
    let marker = w13a10_peek(cur);
    if marker == b'e' || marker == b'E' {
        cur.advance();
        if !is_double {
            double_value = if wide {
                wide_bits as f64
            } else {
                f64::from(narrow)
            };
            is_double = true;
        }
        let mut negative_exponent = false;
        match w13a10_peek(cur) {
            b'+' => cur.advance(),
            b'-' => {
                negative_exponent = true;
                cur.advance();
            }
            _ => {}
        }
        if !w13a10_peek(cur).is_ascii_digit() {
            err.set(W13A10_ERR_MISSING_EXPONENT_DIGIT, cur.position());
            return None;
        }
        exponent = i32::from(w13a10_getc(cur) - b'0');
        if negative_exponent {
            loop {
                if cur.at_end() {
                    break;
                }
                let c = w13a10_peek(cur);
                if !c.is_ascii_digit() {
                    break;
                }
                cur.advance();
                exponent = exponent.wrapping_mul(10).wrapping_add(i32::from(c - b'0'));
                if exponent > 0x0ccc_cccb {
                    // The original stops accumulating and only consumes the
                    // remaining exponent digits.
                    while !cur.at_end() && w13a10_peek(cur).is_ascii_digit() {
                        cur.advance();
                    }
                    break;
                }
            }
            exponent = exponent.wrapping_neg();
        } else {
            loop {
                if cur.at_end() {
                    break;
                }
                let c = w13a10_peek(cur);
                if !c.is_ascii_digit() {
                    break;
                }
                cur.advance();
                exponent = exponent.wrapping_mul(10).wrapping_add(i32::from(c - b'0'));
                if W13A10_MAX_EXPONENT - fraction_shift < exponent {
                    err.set(W13A10_ERR_EXPONENT_OVERFLOW, token_start);
                    return None;
                }
            }
        }
    }

    if is_double {
        let mut value = double_value;
        let mut total = exponent.wrapping_add(fraction_shift);
        if total < -W13A10_MAX_EXPONENT {
            value /= 1e308;
            total = total.wrapping_add(W13A10_MAX_EXPONENT);
            if total < -W13A10_MAX_EXPONENT {
                value = 0.0;
                if negative {
                    value = -value;
                }
                return Some(W13a10Number::from_double(value));
            }
        }
        if total < 0 {
            value /= W13A10_POW10[total.unsigned_abs() as usize];
        } else if total > 0 {
            value *= W13A10_POW10[total as usize];
        }
        if negative {
            value = -value;
        }
        return Some(W13a10Number::from_double(value));
    }

    if wide {
        let payload = if negative {
            (!wide_bits).wrapping_add(1)
        } else {
            wide_bits
        };
        let tag = w13a10_wide_tag(payload, negative);
        return Some(W13a10Number::from_parts(payload, tag));
    }

    if negative {
        let payload = narrow.wrapping_neg();
        let extended = u64::from(payload) | (((payload as i32) >> 31) as u32 as u64) << 32;
        let tag = w13a10_narrow_tag(payload, true);
        Some(W13a10Number::from_parts(extended, tag))
    } else {
        let tag = w13a10_narrow_tag(narrow, false);
        Some(W13a10Number::from_parts(u64::from(narrow), tag))
    }
}

/// `0x0066edc0`: decodes one quoted string and returns its bytes.
///
/// The cursor must be on the opening `"`; the routine consumes it, decodes
/// backslash escapes, encodes `\uXXXX` (including surrogate pairs) as UTF-8,
/// and consumes the closing quote.  The original then materialises a Lua
/// string value and optionally interns it; interning is not observable in a
/// pure parser, so only the byte sequence is returned here.  The original's
/// interning call cannot fail (its helper always reports success), so the
/// `16` status is not reachable.
///
/// Error sites:
///
/// * unknown escape -> code `10` at the backslash;
/// * bad `\u` digit -> code `8` at the backslash (via
///   [`w13a10_parse_hex4`]);
/// * missing or out-of-range surrogate -> code `9` at the backslash;
/// * NUL or end of input -> code `11` at the current byte;
/// * another raw byte below `0x20` -> code `12` at that byte.
pub fn w13a10_parse_string(cur: &mut W13a10Cursor<'_>, err: &mut W13a10Error) -> Option<Vec<u8>> {
    // Consume the opening quote.
    cur.advance();

    let mut out = Vec::new();
    loop {
        let c = w13a10_peek(cur);
        if c == b'\\' {
            let escape_start = cur.position();
            cur.advance();
            let escape = w13a10_peek(cur);
            if let Some(replacement) = w13a10_escape_value(escape) {
                cur.advance();
                out.push(replacement);
            } else if escape == b'u' {
                cur.advance();
                let mut code_point = w13a10_parse_hex4(cur, escape_start, err)?;
                if (0xd800..0xdc00).contains(&code_point) {
                    if w13a10_peek(cur) != b'\\' {
                        err.set(W13A10_ERR_INVALID_SURROGATE, escape_start);
                        return None;
                    }
                    cur.advance();
                    if w13a10_peek(cur) != b'u' {
                        err.set(W13A10_ERR_INVALID_SURROGATE, escape_start);
                        return None;
                    }
                    cur.advance();
                    let low = w13a10_parse_hex4(cur, escape_start, err)?;
                    if !(0xdc00..0xe000).contains(&low) {
                        err.set(W13A10_ERR_INVALID_SURROGATE, escape_start);
                        return None;
                    }
                    code_point = ((code_point - 0xd800) * 0x400 | (low - 0xdc00)) + 0x1_0000;
                }
                w13a10_push_utf8(&mut out, code_point);
            } else {
                err.set(W13A10_ERR_INVALID_ESCAPE, escape_start);
                return None;
            }
        } else if c == b'"' {
            cur.advance();
            return Some(out);
        } else if c < 0x20 {
            if c == 0 {
                err.set(W13A10_ERR_UNFINISHED_STRING, cur.position());
            } else {
                err.set(W13A10_ERR_CONTROL_CHARACTER, cur.position());
            }
            return None;
        } else {
            cur.advance();
            out.push(c);
        }
    }
}

/// The value kind selected by [`w13a10_dispatch`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a10Dispatch {
    /// A quoted string; the cursor is left on the opening quote.
    String,
    /// An array; the cursor is left on `[`.
    Array,
    /// The keyword `false`, already consumed.
    False,
    /// The keyword `null`, already consumed.
    Null,
    /// The keyword `true`, already consumed.
    True,
    /// An object; the cursor is left on `{`.
    Object,
    /// A numeric literal; the cursor is left on its first byte.
    Number,
    /// A partial `false`/`null`/`true` keyword; status `3` was written.
    Invalid,
}

/// `0x0066f130`: selects the value routine from the first byte.
///
/// The string, array, object and number arms do not consume the first byte;
/// their routines do.  The keyword arms consume the letters one at a time,
/// exactly like the original, so a partial keyword leaves the cursor after
/// the letters it matched and writes status `3` at the first unmatched byte.
/// Any other byte (including end of input) is handed to the number routine,
/// which then reports status `3` if it is not a valid number start.  The
/// boolean and nil helpers called by the keyword arms always report success
/// in the recovered body and are represented by the returned variants.
pub fn w13a10_dispatch(cur: &mut W13a10Cursor<'_>, err: &mut W13a10Error) -> W13a10Dispatch {
    match w13a10_peek(cur) {
        b'"' => W13a10Dispatch::String,
        b'[' => W13a10Dispatch::Array,
        b'{' => W13a10Dispatch::Object,
        b'f' => {
            cur.advance();
            if w13a10_peek(cur) == b'a' {
                cur.advance();
                if w13a10_peek(cur) == b'l' {
                    cur.advance();
                    if w13a10_peek(cur) == b's' {
                        cur.advance();
                        if w13a10_peek(cur) == b'e' {
                            cur.advance();
                            return W13a10Dispatch::False;
                        }
                    }
                }
            }
            err.set(W13A10_ERR_SYNTAX, cur.position());
            W13a10Dispatch::Invalid
        }
        b'n' => {
            cur.advance();
            if w13a10_peek(cur) == b'u' {
                cur.advance();
                if w13a10_peek(cur) == b'l' {
                    cur.advance();
                    if w13a10_peek(cur) == b'l' {
                        cur.advance();
                        return W13a10Dispatch::Null;
                    }
                }
            }
            err.set(W13A10_ERR_SYNTAX, cur.position());
            W13a10Dispatch::Invalid
        }
        b't' => {
            cur.advance();
            if w13a10_peek(cur) == b'r' {
                cur.advance();
                if w13a10_peek(cur) == b'u' {
                    cur.advance();
                    if w13a10_peek(cur) == b'e' {
                        cur.advance();
                        return W13a10Dispatch::True;
                    }
                }
            }
            err.set(W13A10_ERR_SYNTAX, cur.position());
            W13a10Dispatch::Invalid
        }
        _ => W13a10Dispatch::Number,
    }
}

/// A parsed value.  This mirrors the five value shapes the dispatcher can
/// produce; the array variant is only constructed by the caller-supplied
/// array routine because that routine lies outside this window.
#[derive(Clone, Debug, PartialEq)]
pub enum W13a10Value {
    /// `null` (an all-zero record).
    Nil,
    /// `true` or `false`.
    Bool(bool),
    /// A numeric literal with its exact payload and tag.
    Number(W13a10Number),
    /// A decoded quoted string.
    String(Vec<u8>),
    /// An object, preserving member order and duplicate keys.
    Table(Vec<(Vec<u8>, W13a10Value)>),
    /// An array returned by the caller-supplied array parser.
    Array(Vec<W13a10Value>),
}

/// Parser facade that owns the caller-supplied array arm.
///
/// The original dispatcher calls an array routine whose body is outside this
/// window.  Passing it in as a callback keeps the translated routines
/// self-contained and lets the caller decide whether arrays are supported.
pub struct W13a10ValueParser<'a> {
    /// Array arm for the `[` dispatch case.
    pub array:
        &'a mut dyn for<'c> FnMut(&mut W13a10Cursor<'c>, &mut W13a10Error) -> Option<W13a10Value>,
}

impl W13a10ValueParser<'_> {
    /// `0x0066f130`: parses one value using the recovered dispatch.
    ///
    /// String, number, object and array values are delegated to their
    /// routines; `false`, `null` and `true` are consumed and returned
    /// directly.  An invalid keyword leaves `err` set and returns `None`.
    pub fn parse_value(
        &mut self,
        cur: &mut W13a10Cursor<'_>,
        err: &mut W13a10Error,
    ) -> Option<W13a10Value> {
        match w13a10_dispatch(cur, err) {
            W13a10Dispatch::String => w13a10_parse_string(cur, err).map(W13a10Value::String),
            W13a10Dispatch::Array => (self.array)(cur, err),
            W13a10Dispatch::False => Some(W13a10Value::Bool(false)),
            W13a10Dispatch::Null => Some(W13a10Value::Nil),
            W13a10Dispatch::True => Some(W13a10Value::Bool(true)),
            W13a10Dispatch::Object => w13a10_parse_object(cur, err, self).map(W13a10Value::Table),
            W13a10Dispatch::Number => w13a10_parse_number(cur, err).map(W13a10Value::Number),
            W13a10Dispatch::Invalid => None,
        }
    }
}

/// `0x0066ea60`: parses an object body and returns its key/value pairs.
///
/// The cursor must be on `{`.  The routine consumes the brace, consumes an
/// immediate `}`, or loops over `"key" : value` members separated by commas.
/// Member order and duplicate keys are preserved because the original fills
/// a table node array in insertion order.  The final table-build call cannot
/// fail in this model, so the `16` status is unreachable.
///
/// Errors are reported at the current byte: `4` when a key does not start
/// with `"`, `5` when `:` is missing, and `6` when neither `,` nor `}` follows
/// a member.  A trailing comma therefore reaches the key check and reports
/// `4` at the closing brace, matching the original's loop structure.
pub fn w13a10_parse_object(
    cur: &mut W13a10Cursor<'_>,
    err: &mut W13a10Error,
    parser: &mut W13a10ValueParser<'_>,
) -> Option<Vec<(Vec<u8>, W13a10Value)>> {
    // Consume the opening brace.
    cur.advance();
    w13a10_skip_whitespace(cur);
    if !err.is_ok() {
        return None;
    }
    if w13a10_peek(cur) == b'}' {
        cur.advance();
        return Some(Vec::new());
    }

    let mut pairs = Vec::new();
    loop {
        if w13a10_peek(cur) != b'"' {
            err.set(W13A10_ERR_EXPECTED_STRING, cur.position());
            return None;
        }
        let key = w13a10_parse_string(cur, err)?;
        w13a10_skip_whitespace(cur);
        if !err.is_ok() {
            return None;
        }
        if w13a10_peek(cur) != b':' {
            err.set(W13A10_ERR_EXPECTED_COLON, cur.position());
            return None;
        }
        cur.advance();
        w13a10_skip_whitespace(cur);
        if !err.is_ok() {
            return None;
        }
        let value = parser.parse_value(cur, err)?;
        w13a10_skip_whitespace(cur);
        if !err.is_ok() {
            return None;
        }
        pairs.push((key, value));

        match w13a10_peek(cur) {
            b',' => {
                cur.advance();
                w13a10_skip_whitespace(cur);
                if !err.is_ok() {
                    return None;
                }
            }
            b'}' => {
                cur.advance();
                return Some(pairs);
            }
            _ => {
                err.set(W13A10_ERR_EXPECTED_BRACE_OR_COMMA, cur.position());
                return None;
            }
        }
    }
}

/// `0x0066ec60`: parses exactly one value and classifies the remainder.
///
/// The original works on a stack-local status record, so this translation
/// starts from a clean [`W13a10Error`].  After skipping leading whitespace:
///
/// * end of input or a NUL byte yields [`W13a10DocumentStatus::Empty`] with
///   the current offset;
/// * otherwise one value is parsed; if it succeeds, trailing whitespace is
///   skipped and any remaining byte yields
///   [`W13a10DocumentStatus::Trailing`] with that offset;
/// * a parse failure returns `None` with `err` set.
///
/// On success the original pops the parsed value from its result stack; this
/// translation returns the status only, because the value is produced by the
/// caller-supplied parser.
pub fn w13a10_parse_document(
    cur: &mut W13a10Cursor<'_>,
    err: &mut W13a10Error,
    parser: &mut W13a10ValueParser<'_>,
) -> Option<W13a10DocumentStatus> {
    *err = W13a10Error::default();
    w13a10_skip_whitespace(cur);
    if !err.is_ok() {
        return None;
    }
    if cur.at_end() || w13a10_peek(cur) == 0 {
        return Some(W13a10DocumentStatus::Empty {
            offset: cur.position(),
        });
    }
    parser.parse_value(cur, err)?;
    w13a10_skip_whitespace(cur);
    if !err.is_ok() {
        return None;
    }
    if !cur.at_end() && w13a10_peek(cur) != 0 {
        return Some(W13a10DocumentStatus::Trailing {
            offset: cur.position(),
        });
    }
    Some(W13a10DocumentStatus::Complete)
}

/// Result of [`w13a10_parse_document`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a10DocumentStatus {
    /// Exactly one value was consumed.
    Complete,
    /// The input was empty (end of buffer or NUL) at this offset.
    Empty {
        /// Byte offset of the empty position.
        offset: usize,
    },
    /// Extra data followed the value at this offset.
    Trailing {
        /// Byte offset of the first trailing byte.
        offset: usize,
    },
}

/// `0x0066f3f0`: moves a fourteen-word (`0x38`-byte) parser record.
///
/// The record contains the four-word value header followed by a table
/// pointer, a terminator word, six owned words and two status words.  The
/// original copies the whole record to the destination and then clears the
/// source's tag (the high half of word `3`, at `+0x0e`) and the owned fields
/// so the source will not release them a second time.
pub fn w13a10_move_record(src: &mut [u32; 14]) -> [u32; 14] {
    let dst = *src;
    // Clear the two-byte tag at +0x0e, which is the high half of word 3.
    src[3] &= 0x0000_ffff;
    // The destination took ownership of these fields.
    src[4] = 0;
    src[5] = 0;
    for word in &mut src[6..12] {
        *word = 0;
    }
    src[12] = 0;
    src[13] = 0;
    dst
}

/// A 16-byte string value in the parser's value record shape.
///
/// Short strings (at most thirteen bytes) store their bytes inline starting
/// at `+0x00`, the length encoding at `+0x0d`, and tag `0x1c05` at `+0x0e`.
/// Long strings store the byte count at `+0x00`, a payload pointer at
/// `+0x08`, and tag `0x0c05` at `+0x0e`.  The pointer cannot be represented
/// directly in safe Rust, so [`bytes`](Self::bytes) keeps the decoded payload
/// alongside the raw words.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W13a10StringSlot {
    /// Raw `+0x00..+0x0f` record words.
    pub words: [u32; 4],
    /// Decoded string bytes (inline bytes or the long-string payload).
    pub bytes: Vec<u8>,
}

impl W13a10StringSlot {
    /// Builds the record exactly as the store routine lays it out.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut words = [0u32; 4];
        if bytes.len() <= W13A10_SHORT_STRING_BYTES {
            let mut record = [0u8; 16];
            record[..bytes.len()].copy_from_slice(bytes);
            record[0x0d] = 0x0d - bytes.len() as u8;
            record[0x0e] = 0x05;
            record[0x0f] = 0x1c;
            for (index, word) in words.iter_mut().enumerate() {
                let start = index * 4;
                *word = u32::from_le_bytes([
                    record[start],
                    record[start + 1],
                    record[start + 2],
                    record[start + 3],
                ]);
            }
        } else {
            words[0] = bytes.len() as u32;
            words[3] = 0x0c05_0000;
        }
        Self {
            words,
            bytes: bytes.to_vec(),
        }
    }

    /// The tag stored at `+0x0e`.
    pub const fn tag(&self) -> u16 {
        (self.words[3] >> 16) as u16
    }

    /// The short-string length encoding at `+0x0d`, if this is a short
    /// string.
    pub const fn short_length_byte(&self) -> Option<u8> {
        if self.tag() == W13A10_TAG_SHORT_STRING {
            Some((self.words[3] >> 8) as u8)
        } else {
            None
        }
    }
}

/// One value of the minimal document tree used by the endpoint accessors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum W13a10StoredValue {
    /// The table lookup produced the shared nil value.
    Nil,
    /// A table of key/value entries, in insertion order.
    Table(Vec<(Vec<u8>, W13a10StoredValue)>),
    /// A string slot.
    String(W13a10StringSlot),
}

/// Shared nil value standing in for the static nil scratch record the retail
/// lookups return on a miss.
pub static W13A10_NIL: W13a10StoredValue = W13a10StoredValue::Nil;

/// Looks up a byte-string key in insertion order.
///
/// The retail table is a `0x20`-byte node array searched by the shared lookup
/// helper; order is not observable for a miss, and duplicate keys resolve to
/// the first entry, so a linear scan over the translated table model matches
/// the recovered length-plus-byte comparisons.
fn w13a10_lookup<'a>(
    table: &'a [(Vec<u8>, W13a10StoredValue)],
    key: &[u8],
) -> Option<&'a W13a10StoredValue> {
    table
        .iter()
        .find_map(|(name, value)| (name.as_slice() == key).then_some(value))
}

/// `0x0066f800`: returns the value of the nested `data.data` entry.
///
/// The routine performs two lookups: first `data` on the caller's table,
/// then `data` again on the first result.  A miss at either step returns the
/// shared nil record.  The second lookup is applied to the first value's
/// table header in the retail code; this safe model returns nil when the
/// first `data` value is not a table, rather than reinterpreting a non-table
/// payload as a node array.
pub fn w13a10_get_nested_data(document: &[(Vec<u8>, W13a10StoredValue)]) -> &W13a10StoredValue {
    let Some(W13a10StoredValue::Table(data)) = w13a10_lookup(document, b"data") else {
        return &W13A10_NIL;
    };
    w13a10_lookup(data, b"data").unwrap_or(&W13A10_NIL)
}

/// `0x0066fd00`: returns a copy of the `data.endpoint` string.
///
/// The getter resolves `data`, then `endpoint`, and copies the resulting
/// string into a fresh standard string object.  The retail code distinguishes
/// the short-string layout (tag bit `0x1000` set; the length is
/// `0xd - byte[+0xd]` and the bytes are inline) from the long-string layout
/// (length at `+0x00`, payload pointer at `+0x08`).  Both layouts are
/// represented by [`W13a10StringSlot`], so the decoded bytes are returned
/// directly.  A missing `data`/`endpoint` entry, or a non-string value, yields
/// an empty string; the original would read a non-string payload through its
/// unchecked long-string branch, which safe Rust cannot reproduce.
pub fn w13a10_get_endpoint(document: &[(Vec<u8>, W13a10StoredValue)]) -> Vec<u8> {
    let Some(W13a10StoredValue::Table(data)) = w13a10_lookup(document, b"data") else {
        return Vec::new();
    };
    match w13a10_lookup(data, b"endpoint") {
        Some(W13a10StoredValue::String(slot)) => slot.bytes.clone(),
        _ => Vec::new(),
    }
}

/// `0x0066fe30`: stores `bytes` in the `data.endpoint` string slot.
///
/// The retail routine resolves the global document table's `data` entry,
/// then the `endpoint` entry of that table, and writes a string value into
/// the resolved slot.  A missing `data` entry or a `data` entry that is not a
/// table makes the following lookup fail, in which case the original writes
/// into a shared nil scratch value and the document is unchanged; the same
/// happens when `endpoint` is absent.  Returns `true` only when a real slot
/// was updated.
///
/// The value layout is the one produced by the string routine: at most
/// thirteen bytes are inline with tag `0x1c05`; longer strings get a
/// count/pointer record with tag `0x0c05`.
pub fn w13a10_store_endpoint(document: &mut [(Vec<u8>, W13a10StoredValue)], bytes: &[u8]) -> bool {
    let Some((_, W13a10StoredValue::Table(data))) = document
        .iter_mut()
        .find(|(key, _)| key.as_slice() == b"data")
    else {
        return false;
    };
    let Some((_, value)) = data
        .iter_mut()
        .find(|(key, _)| key.as_slice() == b"endpoint")
    else {
        return false;
    };
    *value = W13a10StoredValue::String(W13a10StringSlot::from_bytes(bytes));
    true
}

#[cfg(test)]
mod tests_wave13_agent10 {
    use super::*;

    fn number(text: &[u8]) -> (W13a10Number, usize) {
        let mut cur = W13a10Cursor::new(text);
        let mut err = W13a10Error::default();
        let value = w13a10_parse_number(&mut cur, &mut err).expect("number");
        assert!(err.is_ok(), "unexpected status {err:?} for {text:?}");
        (value, cur.position())
    }

    fn number_error(text: &[u8]) -> (u32, usize) {
        let mut cur = W13a10Cursor::new(text);
        let mut err = W13a10Error::default();
        assert!(w13a10_parse_number(&mut cur, &mut err).is_none());
        (err.code, err.offset)
    }

    #[test]
    fn cursor_peek_and_getc_stop_at_end() {
        let mut cur = W13a10Cursor::new(b"ab");
        assert_eq!(w13a10_peek(&cur), b'a');
        assert_eq!(cur.position(), 0);
        assert_eq!(w13a10_getc(&mut cur), b'a');
        assert_eq!(cur.position(), 1);
        assert_eq!(w13a10_peek(&cur), b'b');
        assert_eq!(w13a10_getc(&mut cur), b'b');
        assert_eq!(cur.position(), 2);
        assert!(cur.at_end());
        assert_eq!(w13a10_peek(&cur), 0);
        assert_eq!(w13a10_getc(&mut cur), 0);
        assert_eq!(cur.position(), 2);
    }

    #[test]
    fn skip_whitespace_stops_on_nul_and_end() {
        let mut cur = W13a10Cursor::new(b" \n\r\tX");
        assert_eq!(w13a10_skip_whitespace(&mut cur), b'X');
        assert_eq!(cur.position(), 4);

        let mut empty = W13a10Cursor::new(b"");
        assert_eq!(w13a10_skip_whitespace(&mut empty), 0);
        assert_eq!(empty.position(), 0);

        let mut nul = W13a10Cursor::new(b"  \0x");
        assert_eq!(w13a10_skip_whitespace(&mut nul), 0);
        assert_eq!(nul.position(), 2);
    }

    #[test]
    fn hex4_accepts_both_cases_and_reports_bad_digits() {
        let mut cur = W13a10Cursor::new(b"00Af!");
        let mut err = W13a10Error::default();
        assert_eq!(w13a10_parse_hex4(&mut cur, 7, &mut err), Some(0x00af));
        assert_eq!(cur.position(), 4);
        assert!(err.is_ok());

        let mut cur = W13a10Cursor::new(b"12G4");
        let mut err = W13a10Error::default();
        assert_eq!(w13a10_parse_hex4(&mut cur, 5, &mut err), None);
        assert_eq!(err, W13a10Error { code: 8, offset: 5 });
        assert_eq!(cur.position(), 2);

        let mut cur = W13a10Cursor::new(b"123");
        let mut err = W13a10Error::default();
        assert_eq!(w13a10_parse_hex4(&mut cur, 0, &mut err), None);
        assert_eq!(err.code, 8);
        assert_eq!(cur.position(), 3);
    }

    #[test]
    fn number_narrow_ranges_and_tags() {
        let (zero, end) = number(b"0");
        assert_eq!(zero, W13a10Number::from_parts(0, W13A10_TAG_NONNEG_I32));
        assert_eq!(end, 1);

        let (value, _) = number(b"42");
        assert_eq!(value.as_i64(), 42);
        assert_eq!(value.tag, W13A10_TAG_NONNEG_I32);

        let (value, _) = number(b"2147483647");
        assert_eq!(value.as_i64(), 2_147_483_647);
        assert_eq!(value.tag, W13A10_TAG_NONNEG_I32);

        let (value, _) = number(b"2147483648");
        assert_eq!(value.as_u64(), 2_147_483_648);
        assert_eq!(value.tag, W13A10_TAG_POS_U32);

        let (value, _) = number(b"4294967295");
        assert_eq!(value.as_u64(), 4_294_967_295);
        assert_eq!(value.tag, W13A10_TAG_POS_U32);

        let (value, _) = number(b"4294967296");
        assert_eq!(value.as_u64(), 4_294_967_296);
        assert_eq!(value.tag, W13A10_TAG_POS_I64);
    }

    #[test]
    fn number_negative_ranges_and_minus_zero() {
        let (neg_zero, _) = number(b"-0");
        assert_eq!(neg_zero.as_u64(), 0);
        assert_eq!(neg_zero.tag, W13A10_TAG_NONNEG_I32);

        let (value, _) = number(b"-1");
        assert_eq!(value.as_i64(), -1);
        assert_eq!(value.tag, W13A10_TAG_NEG_I32);

        let (value, _) = number(b"-2147483648");
        assert_eq!(value.as_i64(), -2_147_483_648);
        assert_eq!(value.tag, W13A10_TAG_NEG_I32);

        let (value, _) = number(b"-2147483649");
        assert_eq!(value.as_i64(), -2_147_483_649);
        assert_eq!(value.tag, W13A10_TAG_NEG_I64);

        let (value, _) = number(b"-4294967295");
        assert_eq!(value.as_i64(), -4_294_967_295);
        assert_eq!(value.tag, W13A10_TAG_NEG_I64);

        let (value, _) = number(b"-9223372036854775808");
        assert_eq!(value.as_i64(), i64::MIN);
        assert_eq!(value.tag, W13A10_TAG_NEG_I64);
    }

    #[test]
    fn number_wide_positive_tags() {
        let (value, _) = number(b"9223372036854775807");
        assert_eq!(value.as_u64(), i64::MAX as u64);
        assert_eq!(value.tag, W13A10_TAG_POS_I64);

        let (value, _) = number(b"9223372036854775808");
        assert_eq!(value.as_u64(), 1u64 << 63);
        assert_eq!(value.tag, W13A10_TAG_POS_U64_HIGH);

        let (value, _) = number(b"18446744073709551615");
        assert_eq!(value.as_u64(), u64::MAX);
        assert_eq!(value.tag, W13A10_TAG_POS_U64_HIGH);

        let (value, _) = number(b"18446744073709551616");
        assert_eq!(
            value.as_f64().expect("double"),
            18_446_744_073_709_551_616.0
        );
    }

    #[test]
    fn number_fraction_exponent_and_stop_rules() {
        let (value, end) = number(b"1.5");
        assert_eq!(value.as_f64(), Some(1.5));
        assert_eq!(value.tag, W13A10_TAG_DOUBLE);
        assert_eq!(end, 3);

        let (value, _) = number(b"12.34");
        let parsed = value.as_f64().expect("double");
        assert!((parsed - 12.34).abs() < 1e-12);

        let (value, _) = number(b"1e3");
        assert_eq!(value.as_f64(), Some(1000.0));

        let (value, _) = number(b"1.25e-2");
        let parsed = value.as_f64().expect("double");
        assert!((parsed - 0.0125).abs() < 1e-15);

        let (value, end) = number(b"0x10");
        assert_eq!(value.as_i64(), 0);
        assert_eq!(value.tag, W13A10_TAG_NONNEG_I32);
        assert_eq!(end, 1);

        let (value, end) = number(b"007");
        assert_eq!(value.as_i64(), 0);
        assert_eq!(end, 1);

        let (value, _) = number(b"1e-400");
        assert_eq!(value.as_f64(), Some(0.0));

        let (value, _) = number(b"-1e-400");
        let parsed = value.as_f64().expect("double");
        assert_eq!(parsed, 0.0);
        assert!(parsed.is_sign_negative());

        let (value, _) = number(b"1e308");
        assert_eq!(value.as_f64(), Some(1e308));
    }

    #[test]
    fn number_failures_report_exact_sites() {
        assert_eq!(number_error(b""), (W13A10_ERR_SYNTAX, 0));
        assert_eq!(number_error(b"-"), (W13A10_ERR_SYNTAX, 0));
        assert_eq!(number_error(b"+1"), (W13A10_ERR_SYNTAX, 0));
        assert_eq!(number_error(b".5"), (W13A10_ERR_SYNTAX, 0));
        assert_eq!(number_error(b"1."), (W13A10_ERR_MISSING_FRACTION_DIGIT, 2));
        assert_eq!(number_error(b"1e"), (W13A10_ERR_MISSING_EXPONENT_DIGIT, 2));
        assert_eq!(number_error(b"1e+"), (W13A10_ERR_MISSING_EXPONENT_DIGIT, 3));
        assert_eq!(number_error(b"1e309"), (W13A10_ERR_EXPONENT_OVERFLOW, 0));
        assert_eq!(number_error(b"-2e309"), (W13A10_ERR_EXPONENT_OVERFLOW, 0));
    }

    #[test]
    fn string_plain_and_escaped() {
        let mut cur = W13a10Cursor::new(br#""abc""#);
        let mut err = W13a10Error::default();
        let out = w13a10_parse_string(&mut cur, &mut err).expect("string");
        assert_eq!(out.as_slice(), b"abc");
        assert!(err.is_ok());
        assert_eq!(cur.position(), 5);

        let mut cur = W13a10Cursor::new(br#""a\n\t\"\\\/b""#);
        let mut err = W13a10Error::default();
        let out = w13a10_parse_string(&mut cur, &mut err).expect("string");
        assert_eq!(out.as_slice(), b"a\n\t\"\\/b");
        assert!(err.is_ok());

        let mut cur = W13a10Cursor::new(br#""\b\f\r""#);
        let mut err = W13a10Error::default();
        let out = w13a10_parse_string(&mut cur, &mut err).expect("string");
        assert_eq!(out.as_slice(), b"\x08\x0c\r");
    }

    #[test]
    fn string_unicode_and_surrogates() {
        let mut cur = W13a10Cursor::new(br#""\u0041""#);
        let mut err = W13a10Error::default();
        let out = w13a10_parse_string(&mut cur, &mut err).expect("string");
        assert_eq!(out.as_slice(), b"A");

        let mut cur = W13a10Cursor::new(br#""\u00e9""#);
        let mut err = W13a10Error::default();
        let out = w13a10_parse_string(&mut cur, &mut err).expect("string");
        assert_eq!(out.as_slice(), b"\xc3\xa9");

        let mut cur = W13a10Cursor::new(br#""\ud83d\ude00""#);
        let mut err = W13a10Error::default();
        let out = w13a10_parse_string(&mut cur, &mut err).expect("string");
        assert_eq!(out.as_slice(), b"\xf0\x9f\x98\x80");

        let mut cur = W13a10Cursor::new(br#""\ud83dX""#);
        let mut err = W13a10Error::default();
        assert!(w13a10_parse_string(&mut cur, &mut err).is_none());
        assert_eq!(err, W13a10Error { code: 9, offset: 1 });
    }

    #[test]
    fn string_failures_report_exact_sites() {
        let mut cur = W13a10Cursor::new(br#""\q""#);
        let mut err = W13a10Error::default();
        assert!(w13a10_parse_string(&mut cur, &mut err).is_none());
        assert_eq!(
            err,
            W13a10Error {
                code: 10,
                offset: 1
            }
        );

        let mut cur = W13a10Cursor::new(b"\"abc");
        let mut err = W13a10Error::default();
        assert!(w13a10_parse_string(&mut cur, &mut err).is_none());
        assert_eq!(
            err,
            W13a10Error {
                code: 11,
                offset: 4
            }
        );

        let mut cur = W13a10Cursor::new(b"\"a\x01b\"");
        let mut err = W13a10Error::default();
        assert!(w13a10_parse_string(&mut cur, &mut err).is_none());
        assert_eq!(
            err,
            W13a10Error {
                code: 12,
                offset: 2
            }
        );
    }

    #[test]
    fn dispatch_consumes_keywords_and_leaves_other_bytes() {
        let mut cur = W13a10Cursor::new(b"\"x\"");
        let mut err = W13a10Error::default();
        assert_eq!(w13a10_dispatch(&mut cur, &mut err), W13a10Dispatch::String);
        assert_eq!(cur.position(), 0);

        let mut cur = W13a10Cursor::new(b"[1]");
        let mut err = W13a10Error::default();
        assert_eq!(w13a10_dispatch(&mut cur, &mut err), W13a10Dispatch::Array);
        assert_eq!(cur.position(), 0);

        let mut cur = W13a10Cursor::new(b"{");
        let mut err = W13a10Error::default();
        assert_eq!(w13a10_dispatch(&mut cur, &mut err), W13a10Dispatch::Object);
        assert_eq!(cur.position(), 0);

        let mut cur = W13a10Cursor::new(b"false!");
        let mut err = W13a10Error::default();
        assert_eq!(w13a10_dispatch(&mut cur, &mut err), W13a10Dispatch::False);
        assert_eq!(cur.position(), 5);
        assert!(err.is_ok());

        let mut cur = W13a10Cursor::new(b"null");
        let mut err = W13a10Error::default();
        assert_eq!(w13a10_dispatch(&mut cur, &mut err), W13a10Dispatch::Null);
        assert_eq!(cur.position(), 4);

        let mut cur = W13a10Cursor::new(b"true");
        let mut err = W13a10Error::default();
        assert_eq!(w13a10_dispatch(&mut cur, &mut err), W13a10Dispatch::True);
        assert_eq!(cur.position(), 4);

        let mut cur = W13a10Cursor::new(b"falsX");
        let mut err = W13a10Error::default();
        assert_eq!(w13a10_dispatch(&mut cur, &mut err), W13a10Dispatch::Invalid);
        assert_eq!(err, W13a10Error { code: 3, offset: 4 });
        assert_eq!(cur.position(), 4);

        let mut cur = W13a10Cursor::new(b"nul");
        let mut err = W13a10Error::default();
        assert_eq!(w13a10_dispatch(&mut cur, &mut err), W13a10Dispatch::Invalid);
        assert_eq!(err, W13a10Error { code: 3, offset: 3 });

        let mut cur = W13a10Cursor::new(b"?");
        let mut err = W13a10Error::default();
        assert_eq!(w13a10_dispatch(&mut cur, &mut err), W13a10Dispatch::Number);
        assert_eq!(cur.position(), 0);
        assert!(err.is_ok());
    }

    fn unsupported_array(
        _cur: &mut W13a10Cursor<'_>,
        err: &mut W13a10Error,
    ) -> Option<W13a10Value> {
        err.set(W13A10_ERR_SYNTAX, 0);
        None
    }

    fn parse_object_value(text: &[u8]) -> (W13a10Value, usize) {
        let mut cur = W13a10Cursor::new(text);
        let mut err = W13a10Error::default();
        let mut array = unsupported_array;
        let mut parser = W13a10ValueParser { array: &mut array };
        let value = parser.parse_value(&mut cur, &mut err).expect("value");
        assert!(err.is_ok(), "unexpected status {err:?}");
        (value, cur.position())
    }

    #[test]
    fn value_parser_scalars() {
        assert_eq!(
            parse_object_value(b"\"hi\"").0,
            W13a10Value::String(b"hi".to_vec())
        );

        let (value, end) = parse_object_value(b"false");
        assert_eq!(value, W13a10Value::Bool(false));
        assert_eq!(end, 5);

        assert_eq!(parse_object_value(b"null").0, W13a10Value::Nil);
        assert_eq!(parse_object_value(b"true").0, W13a10Value::Bool(true));

        let (value, end) = parse_object_value(b"12");
        assert_eq!(
            value,
            W13a10Value::Number(W13a10Number::from_parts(12, W13A10_TAG_NONNEG_I32))
        );
        assert_eq!(end, 2);
    }

    #[test]
    fn object_parser_preserves_order_and_nests() {
        let (value, end) = parse_object_value(br#"{"a":1,"b":true,"c":{"d":"x"}}"#);
        let expected = W13a10Value::Table(vec![
            (
                b"a".to_vec(),
                W13a10Value::Number(W13a10Number::from_parts(1, W13A10_TAG_NONNEG_I32)),
            ),
            (b"b".to_vec(), W13a10Value::Bool(true)),
            (
                b"c".to_vec(),
                W13a10Value::Table(vec![(b"d".to_vec(), W13a10Value::String(b"x".to_vec()))]),
            ),
        ]);
        assert_eq!(value, expected);
        assert_eq!(end, 30);

        let (value, end) = parse_object_value(b"{}");
        assert_eq!(value, W13a10Value::Table(Vec::new()));
        assert_eq!(end, 2);

        let (value, _) = parse_object_value(br#"{"a":1,"a":2}"#);
        assert_eq!(
            value,
            W13a10Value::Table(vec![
                (
                    b"a".to_vec(),
                    W13a10Value::Number(W13a10Number::from_parts(1, W13A10_TAG_NONNEG_I32)),
                ),
                (
                    b"a".to_vec(),
                    W13a10Value::Number(W13a10Number::from_parts(2, W13A10_TAG_NONNEG_I32)),
                ),
            ])
        );
    }

    fn object_error(text: &[u8]) -> (u32, usize) {
        let mut cur = W13a10Cursor::new(text);
        let mut err = W13a10Error::default();
        let mut array = unsupported_array;
        let mut parser = W13a10ValueParser { array: &mut array };
        assert!(parser.parse_value(&mut cur, &mut err).is_none());
        (err.code, err.offset)
    }

    #[test]
    fn object_parser_errors() {
        assert_eq!(object_error(b"{"), (4, 1));
        assert_eq!(object_error(br#"{"a" 1}"#), (5, 5));
        assert_eq!(object_error(br#"{"a":1 "b":2}"#), (6, 7));
        assert_eq!(object_error(br#"{"a":1,}"#), (4, 7));
        assert_eq!(object_error(br#"{"a":}"#), (3, 5));
        assert_eq!(object_error(br#"{"a":tru}"#), (3, 8));
    }

    #[test]
    fn array_callback_is_used_for_brackets() {
        let mut cur = W13a10Cursor::new(b"[ignored]");
        let mut err = W13a10Error::default();
        let mut array = |cur: &mut W13a10Cursor<'_>, _err: &mut W13a10Error| {
            while !cur.at_end() && w13a10_peek(cur) != b']' {
                cur.advance();
            }
            cur.advance();
            Some(W13a10Value::Array(vec![W13a10Value::Nil]))
        };
        let mut parser = W13a10ValueParser { array: &mut array };
        let value = parser.parse_value(&mut cur, &mut err).expect("array");
        assert_eq!(value, W13a10Value::Array(vec![W13a10Value::Nil]));
        assert!(err.is_ok());
        assert_eq!(cur.position(), 9);
    }

    #[test]
    fn document_status_classifies_empty_trailing_and_complete() {
        let mut cur = W13a10Cursor::new(b"  {} ");
        let mut err = W13a10Error::default();
        let mut array = unsupported_array;
        let mut parser = W13a10ValueParser { array: &mut array };
        assert_eq!(
            w13a10_parse_document(&mut cur, &mut err, &mut parser),
            Some(W13a10DocumentStatus::Complete)
        );
        assert!(err.is_ok());

        let mut cur = W13a10Cursor::new(b"   ");
        let mut err = W13a10Error::default();
        assert_eq!(
            w13a10_parse_document(&mut cur, &mut err, &mut parser),
            Some(W13a10DocumentStatus::Empty { offset: 3 })
        );

        let mut cur = W13a10Cursor::new(b"{} x");
        let mut err = W13a10Error::default();
        assert_eq!(
            w13a10_parse_document(&mut cur, &mut err, &mut parser),
            Some(W13a10DocumentStatus::Trailing { offset: 3 })
        );

        let mut cur = W13a10Cursor::new(b"");
        let mut err = W13a10Error::default();
        assert_eq!(
            w13a10_parse_document(&mut cur, &mut err, &mut parser),
            Some(W13a10DocumentStatus::Empty { offset: 0 })
        );

        let mut cur = W13a10Cursor::new(b"{");
        let mut err = W13a10Error::default();
        assert_eq!(w13a10_parse_document(&mut cur, &mut err, &mut parser), None);
        assert_eq!(err, W13a10Error { code: 4, offset: 1 });
    }

    #[test]
    fn move_record_transfers_ownership_and_clears_source() {
        let mut src = [0u32; 14];
        for (index, word) in src.iter_mut().enumerate() {
            *word = 0x1000 + index as u32;
        }
        src[3] = 0x1c05_abcd;
        let original = src;
        let dst = w13a10_move_record(&mut src);
        assert_eq!(dst, original);
        assert_eq!(src[3], 0x0000_abcd);
        assert_eq!(&src[4..6], &[0, 0]);
        assert_eq!(&src[6..12], &[0, 0, 0, 0, 0, 0]);
        assert_eq!(&src[12..14], &[0, 0]);
        assert_eq!(&dst[4..6], &[0x1004, 0x1005]);
    }

    #[test]
    fn string_slot_short_and_long_layouts() {
        let slot = W13a10StringSlot::from_bytes(b"abc");
        assert_eq!(slot.tag(), W13A10_TAG_SHORT_STRING);
        assert_eq!(slot.short_length_byte(), Some(10));
        assert_eq!(slot.bytes.as_slice(), b"abc");
        assert_eq!(slot.words[3] & 0xffff_0000, 0x1c05_0000);

        let thirteen = b"0123456789abc";
        let slot = W13a10StringSlot::from_bytes(thirteen);
        assert_eq!(slot.tag(), W13A10_TAG_SHORT_STRING);
        assert_eq!(slot.short_length_byte(), Some(0));
        assert_eq!(slot.bytes.as_slice(), thirteen);

        let fourteen = b"0123456789abcd";
        let slot = W13a10StringSlot::from_bytes(fourteen);
        assert_eq!(slot.tag(), W13A10_TAG_LONG_STRING);
        assert_eq!(slot.words[0], 14);
        assert_eq!(slot.short_length_byte(), None);
        assert_eq!(slot.bytes.as_slice(), fourteen);

        let slot = W13a10StringSlot::from_bytes(b"");
        assert_eq!(slot.tag(), W13A10_TAG_SHORT_STRING);
        assert_eq!(slot.short_length_byte(), Some(13));
        assert!(slot.bytes.is_empty());
    }

    #[test]
    fn endpoint_store_updates_only_the_resolved_slot() {
        let mut document = vec![(
            b"data".to_vec(),
            W13a10StoredValue::Table(vec![(b"endpoint".to_vec(), W13a10StoredValue::Nil)]),
        )];
        assert!(w13a10_store_endpoint(&mut document, b"new value"));
        let W13a10StoredValue::Table(data) = &document[0].1 else {
            panic!("data table");
        };
        let W13a10StoredValue::String(slot) = &data[0].1 else {
            panic!("endpoint string");
        };
        assert_eq!(slot.bytes.as_slice(), b"new value");
        assert_eq!(slot.tag(), W13A10_TAG_SHORT_STRING);

        let mut missing = vec![(b"other".to_vec(), W13a10StoredValue::Table(Vec::new()))];
        assert!(!w13a10_store_endpoint(&mut missing, b"x"));

        let mut not_table = vec![(
            b"data".to_vec(),
            W13a10StoredValue::String(W13a10StringSlot::from_bytes(b"x")),
        )];
        assert!(!w13a10_store_endpoint(&mut not_table, b"x"));

        let mut no_endpoint = vec![(b"data".to_vec(), W13a10StoredValue::Table(Vec::new()))];
        assert!(!w13a10_store_endpoint(&mut no_endpoint, b"x"));
    }

    #[test]
    fn endpoint_getter_returns_decoded_bytes_and_empty_on_misses() {
        let mut document = vec![(
            b"data".to_vec(),
            W13a10StoredValue::Table(vec![(
                b"endpoint".to_vec(),
                W13a10StoredValue::String(W13a10StringSlot::from_bytes(b"endpoint value")),
            )]),
        )];
        assert_eq!(w13a10_get_endpoint(&document).as_slice(), b"endpoint value");

        assert!(w13a10_store_endpoint(&mut document, b"0123456789abcd"));
        assert_eq!(w13a10_get_endpoint(&document).as_slice(), b"0123456789abcd");

        let missing = vec![(b"other".to_vec(), W13a10StoredValue::Table(Vec::new()))];
        assert!(w13a10_get_endpoint(&missing).is_empty());

        let not_table = vec![(
            b"data".to_vec(),
            W13a10StoredValue::String(W13a10StringSlot::from_bytes(b"x")),
        )];
        assert!(w13a10_get_endpoint(&not_table).is_empty());

        let no_endpoint = vec![(b"data".to_vec(), W13a10StoredValue::Table(Vec::new()))];
        assert!(w13a10_get_endpoint(&no_endpoint).is_empty());

        let numeric_endpoint = vec![(
            b"data".to_vec(),
            W13a10StoredValue::Table(vec![(b"endpoint".to_vec(), W13a10StoredValue::Nil)]),
        )];
        assert!(w13a10_get_endpoint(&numeric_endpoint).is_empty());
    }

    #[test]
    fn nested_data_getter_returns_inner_value_or_nil() {
        let document = vec![(
            b"data".to_vec(),
            W13a10StoredValue::Table(vec![(
                b"data".to_vec(),
                W13a10StoredValue::String(W13a10StringSlot::from_bytes(b"inner")),
            )]),
        )];
        assert_eq!(
            w13a10_get_nested_data(&document),
            &W13a10StoredValue::String(W13a10StringSlot::from_bytes(b"inner"))
        );

        let missing = vec![(b"other".to_vec(), W13a10StoredValue::Nil)];
        assert_eq!(w13a10_get_nested_data(&missing), &W13A10_NIL);

        let not_table = vec![(
            b"data".to_vec(),
            W13a10StoredValue::String(W13a10StringSlot::from_bytes(b"x")),
        )];
        assert_eq!(w13a10_get_nested_data(&not_table), &W13A10_NIL);

        let no_inner = vec![(b"data".to_vec(), W13a10StoredValue::Table(Vec::new()))];
        assert_eq!(w13a10_get_nested_data(&no_inner), &W13A10_NIL);
    }
}
