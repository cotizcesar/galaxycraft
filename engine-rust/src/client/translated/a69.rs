#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent69.rs
// ===========================================================================
// Translation batch 69.  Only this file is edited by the batch.
//
// The `decompiled_uncited` rows in `0x0061c000..=0x0061dfff` (55 rows plus
// one thunk) are the low-level half of the bundled JSON reader.  Retail
// keeps three pieces of state that the rows below touch:
//
// * a downward-growing **value stack**: base at `+0x20`, cursor at `+0x24`,
//   end at `+0x28`.  A completed value is a 16-byte (scalar/array) or
//   32-byte (object) frame written immediately *below* the cursor, with the
//   type tag in the frame's last two bytes;
// * a **byte builder** for the decoded string, base at `+0x08` and cursor at
//   `+0x0c`, grown one byte at a time through the guard below;
// * an **input cursor** of two words, `{pos, base}`, where `pos` is the
//   current read position and `base` the start of the whole document.
//   Recorded error offsets are always `pos - base`.
//
// Every function below reproduces one row's exact arithmetic, table lookup or
// byte sequence as a free function (the rows are pure helpers with no
// `CUnit`/`CWorld` state, so nothing is added to [`crate::sim::SimUnit`] or
// [`crate::sim::World`]).  Positions are modelled as indices into the input
// slice instead of raw pointers; the pointer subtraction used for error
// offsets is then the plain index difference.
//
// Rows in this page that are *not* translated are listed next.
//
// # Skipped rows
//
// The remaining 38 `decompiled_uncited` rows fall into four groups.
//
// **Unwinding funclets and misaligned entry points** (28 rows).  These have
// no recovered body beyond an `ExceptionList` restore, a terminate call or an
// overlapping-instruction decode: `FUN_0061c050`, `FUN_0061c081`, `FUN_0061c311`,
// `FUN_0061c735`, `FUN_0061ce6e`, `FUN_0061cebe`, `FUN_0061cf2f`, `FUN_0061d630`,
// `FUN_0061d690`, `FUN_0061d700`, `FUN_0061d767`, `FUN_0061d77d`, `FUN_0061d7d0`,
// `FUN_0061d820`, `FUN_0061d87f`, `FUN_0061da4e`, `FUN_0061da70`, `FUN_0061dcf6`,
// `FUN_0061dd00`, `FUN_0061dd60`, `FUN_0061dd66`, `FUN_0061dddc`, `FUN_0061ddf9`,
// `FUN_0061de2c`, `FUN_0061debe`, `FUN_0061def8`, `FUN_0061df0d`, `FUN_0061df40`.
//
// **Parser bodies that need helpers outside this page** (4 rows).
// `FUN_0061c3f0` / `FUN_0061cdb0` / `FUN_0061cf90` / `FUN_0061d450` are the array,
// object and document bodies.  `FUN_0061d450` dispatches on the first byte and
// its `true`/`false` arms call `0x0061d970`, which is not in this batch; the
// same bodies also call `FUN_0061ef80` (string validation) and `FUN_00562fd0`
// (allocation), so their control flow cannot be reproduced without citing
// helpers that do not exist yet.
//
// **Bodies whose data tables are absent from the decompiled text** (4 rows).
// `FUN_0061c5b0` is the number scanner.  Its digit loops and overflow
// thresholds are fully visible, but every arm ends by writing a `CValue`
// header whose tag bits, and whose final scaling, come from the power-of-ten
// table at `0x00d3a378`; without that table the result would be a guess.
// `FUN_0061d100` / `FUN_0061d16a` / `FUN_0061d2a7` are the string unescapers.
// `FUN_0061d100` indexes the escape table at `0x00d3ad78`, splices surrogate
// pairs through `0x0061c1b0`/`0x0061c540` and finishes in
// `FUN_0061ef80`/`FUN_00562fd0`; `FUN_0061d16a` and `FUN_0061d2a7` are its two
// register-specialised restarts, and neither the escape table nor the
// validation helper is recoverable from the C text.
//
// **Hash-map walkers** (2 rows).  `FUN_0061df80` and `FUN_0061df8f` walk an
// opaque bucket vector and end in a virtual call through `*(*node + 0x10)`
// whose target is not part of this batch.

/// `{pos, base}` input cursor.  Retail stores the two words at `+0`/`+4` of
/// every parser context; `base` is subtracted from `pos` to form an error
/// offset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct A69Cursor {
    /// Current read position, as an index into the document bytes.
    pub pos: usize,
    /// Start of the document; error offsets are `pos - base`.
    pub base: usize,
}

/// Value-stack header.  Field offsets mirror the retail object: `+0x20` base,
/// `+0x24` cursor, `+0x28` end.  The three addresses are 32-bit retail
/// pointers, so the frame arithmetic below wraps at 32 bits exactly as the
/// retail `add`/`lea` do.  `tag` and `words` model the frame that the two
/// `pop_*_frame` builders write below the cursor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct A69ValueStack {
    /// `+0x20`: first byte of the value buffer.
    pub begin: u32,
    /// `+0x24`: next free byte.
    pub cursor: u32,
    /// `+0x28`: end of the value buffer.
    pub end: u32,
    /// Type tag written at `frame_base + size - 2`.
    pub tag: u16,
    /// The three payload words zeroed at `frame_base + 0`, `+4`, `+8`.
    pub words: [u32; 3],
}

/// Decoded-string byte builder.  Field offsets mirror retail: `+0x08` base,
/// `+0x0c` cursor.  The object's `+0x10` byte is the flag cleared by
/// [`a69_clear_flag`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct A69ByteBuf {
    /// `+0x08`: start of the buffer.
    pub begin: u32,
    /// `+0x0c`: next byte to write.
    pub cursor: u32,
}

/// Size of one array/string frame on the value stack (`param_1 * -0x10`).
pub const A69_ARRAY_FRAME_BYTES: u32 = 0x10;
/// Size of one object frame on the value stack (`param_1 * -0x20`).
pub const A69_OBJECT_FRAME_BYTES: u32 = 0x20;
/// Type tag of an array frame (`0x0061d9b0` writes `4`).
pub const A69_ARRAY_TAG: u16 = 4;
/// Type tag of an object frame (`0x0061da10` writes `3`).
pub const A69_OBJECT_TAG: u16 = 3;
/// The static descriptor table whose address `0x0061df70` returns.
pub const A69_STATIC_DESCRIPTOR_TABLE: usize = 0x00f0_4d24;

/// `0x0061c030` (`FUN_0061c030`, size 27): initialise a fresh 12-byte value
/// header.  Retail stores `0x10` at `+0`, `4` at `+4` and the byte `1` at
/// `+8`; bytes `+9..+12` are left untouched.
pub fn a69_value_header_init(header: &mut [u8; 12]) {
    header[0..4].copy_from_slice(&0x10u32.to_le_bytes());
    header[4..8].copy_from_slice(&4u32.to_le_bytes());
    header[8] = 1;
}

/// `0x0061c1b0` (`FUN_0061c1b0`, size 142): append one scalar to the decoded
/// string as UTF-8 and return how many bytes were appended.  Retail grows the
/// builder one byte at a time through the guard at `0x0061c2f0`; the `Vec`
/// growth here is equivalent, so only the byte sequence is reproduced.
///
/// The branch boundaries are the retail comparisons `0x80`, `0x800` and
/// `0x10000`; the four-byte form shifts the lead byte by `0x12` (18) and the
/// three continuation bytes by `0xc`, `6` and `0`.  No range check is done on
/// the scalar, exactly as retail does not do one.
pub fn a69_utf8_append(out: &mut Vec<u8>, scalar: u32) -> usize {
    if scalar < 0x80 {
        out.push(scalar as u8);
        1
    } else if scalar < 0x800 {
        out.push(((scalar >> 6) as u8) | 0xc0);
        out.push((scalar as u8 & 0x3f) | 0x80);
        2
    } else if scalar < 0x10000 {
        out.push(((scalar >> 0xc) as u8) | 0xe0);
        out.push((((scalar >> 6) as u8) & 0x3f) | 0x80);
        out.push((scalar as u8 & 0x3f) | 0x80);
        3
    } else {
        out.push(((scalar >> 0x12) as u8) | 0xf0);
        out.push((((scalar >> 0xc) as u8) & 0x3f) | 0x80);
        out.push((((scalar >> 6) as u8) & 0x3f) | 0x80);
        out.push((scalar as u8 & 0x3f) | 0x80);
        4
    }
}

/// `0x0061c240` (`FUN_0061c240`, size 173): the stand-alone four-byte emitter.
/// Retail derives its bytes from `input - 1` (`uVar3 = unaff_EBX - 1`) and
/// always writes four bytes, so it is exposed with the already-decremented
/// scalar spelled out.  Returns 4, the number of counter steps retail makes.
pub fn a69_utf8_append_offset4(out: &mut Vec<u8>, value: u32) -> usize {
    let v = value.wrapping_sub(1);
    out.push(((v >> 0x12) as u8) | 0xf0);
    out.push((((v >> 0xc) as u8) & 0x3f) | 0x80);
    out.push((((v >> 6) as u8) & 0x3f) | 0x80);
    out.push(((v as u8) & 0x3f) | 0x80);
    4
}

/// The two terminate paths behind the string-append guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A69BufferAbort {
    /// `capacity == 0 && begin == 0`: the builder has no buffer at all, which
    /// retail reports with the argument-`1` abort.
    NullBuffer,
    /// Every other failed growth: retail reports it with the no-argument
    /// abort.
    Exhausted,
}

/// `0x0061c2f0` (`FUN_0061c2f0`, size 19): classify the string builder's
/// failed append.  The retail body is non-returning; the distinction it makes
/// is `in_ECX[2] == 0 && *in_ECX == 0` (capacity and base both zero) versus
/// everything else.
pub fn a69_append_guard_abort(capacity: usize, begin: usize) -> A69BufferAbort {
    if capacity == 0 && begin == 0 {
        A69BufferAbort::NullBuffer
    } else {
        A69BufferAbort::Exhausted
    }
}

/// `0x0061c370` (`FUN_0061c370`, size 73): the value stack's copy of the same
/// guard.  Its decompiled body is byte-identical to `0x0061c2f0`; only the
/// call sites differ (this one is entered when a value-stack grow fails).
pub fn a69_value_stack_guard_abort(capacity: usize, begin: usize) -> A69BufferAbort {
    a69_append_guard_abort(capacity, begin)
}

/// `0x0061c540` (`FUN_0061c540`, size 103): read exactly four hexadecimal
/// digits, accumulating in an `i32` (returned as the low 32 bits).  Digit
/// values come from the three retail range tests: `'0'..'9'` subtract `0x30`,
/// `'A'..'F'` subtract `0x37`, `'a'..'f'` subtract `0x57`.  Anything else
/// records error code `8` at `error_offset` (the caller's escape offset),
/// leaves the cursor on the bad byte and returns `Err`.
pub fn a69_hex_quad(
    bytes: &[u8],
    cur: &mut A69Cursor,
    error_offset: usize,
) -> Result<u32, (u8, usize)> {
    let mut value: i32 = 0;
    for _ in 0..4 {
        let byte = bytes.get(cur.pos).copied().unwrap_or(0);
        // Retail sign-extends the `char` before adding it to the accumulator.
        value = value.wrapping_mul(0x10).wrapping_add(byte as i8 as i32);
        if byte.wrapping_sub(0x30) < 10 {
            value = value.wrapping_add(-0x30);
        } else if byte.wrapping_add(0xbf) < 6 {
            value = value.wrapping_add(-0x37);
        } else if byte.wrapping_add(0x9f) <= 5 {
            value = value.wrapping_add(-0x57);
        } else {
            return Err((8, error_offset));
        }
        cur.pos += 1;
    }
    Ok(value as u32)
}

/// `0x0061d600` (`FUN_0061d600`, size 47): advance over ASCII whitespace.
/// Retail's loop tests `' '`, `'\n'`, `'\r'` and `'\t'` in that order and
/// stops on the first other byte (including NUL).
pub fn a69_skip_whitespace(bytes: &[u8], cur: &mut A69Cursor) {
    let mut pos = cur.pos;
    while matches!(bytes.get(pos).copied(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
        pos += 1;
    }
    cur.pos = pos;
}

/// `0x0061d402` (`FUN_0061d402`, size 26): write the terminating NUL of the
/// decoded string at `length` and then publish the input cursor, whose two
/// words retail copies through the caller's out pointer.  Returns the written
/// index.
pub fn a69_terminate_string(
    out: &mut [u8],
    length: usize,
    cur: A69Cursor,
    published: &mut A69Cursor,
) -> usize {
    out[length] = 0;
    *published = cur;
    length
}

/// `0x0061d43b` (`FUN_0061d43b`, size 19): epilogue-only fragment.  The body
/// restores `ExceptionList` and returns, so the modelled behaviour is a no-op.
pub fn a69_unwind_epilogue() {}

/// `0x0061d6f0` (`FUN_0061d6f0`, size 14): copy the two cursor words into the
/// cursor object reached through the source's third word (`this+0x8`).
pub fn a69_store_cursor_pair(dst: &mut A69Cursor, src: A69Cursor) {
    *dst = src;
}

/// `0x0061d810` (`FUN_0061d810`, size 5): clear the byte flag at `+0x10`.
pub fn a69_clear_flag(flag: &mut u8) {
    *flag = 0;
}

/// `0x0061d8b0` (`FUN_0061d8b0`, size 72): the parser's sign guard on the
/// status word at `+0x1c`.  Retail restores `ExceptionList` and calls the
/// terminate helper whenever the word is `>= 0`, and returns only for a
/// negative value; the predicate reports that accepted condition.
pub fn a69_error_status_accepted(status: i32) -> bool {
    status < 0
}

/// `0x0061d900` (`FUN_0061d900`, size 49): reset the value stack by copying
/// base (`+0x20`) into the cursor (`+0x24`).
pub fn a69_reset_value_cursor(stack: &mut A69ValueStack) {
    stack.cursor = stack.begin;
}

/// `0x0061d960` (`FUN_0061d960`, size 5): reset the decoded-string builder by
/// copying base (`+0x8`) into the cursor (`+0xc`).
pub fn a69_reset_byte_cursor(buf: &mut A69ByteBuf) {
    buf.cursor = buf.begin;
}

/// `0x0061d9b0` (`FUN_0061d9b0`, size 77): close an array value.  The cursor
/// steps back by `count * 0x10`, the frame's tag becomes `4` and the three
/// payload words are zeroed.  Retail also allocates and copies `count * 0x10`
/// bytes when `count != 0` before storing the same zeros; that heap
/// relocation does not change the cursor or header, so it is not modelled.
/// Always returns true (retail `return 1`).
pub fn a69_pop_array_frame(stack: &mut A69ValueStack, count: u32) -> bool {
    let frame = stack
        .cursor
        .wrapping_sub(count.wrapping_mul(A69_ARRAY_FRAME_BYTES));
    stack.cursor = frame;
    stack.tag = A69_ARRAY_TAG;
    stack.words = [0; 3];
    true
}

/// `0x0061da10` (`FUN_0061da10`, size 77): close an object value.  Same
/// sequence as [`a69_pop_array_frame`] with `count * 0x20` and tag `3`.
pub fn a69_pop_object_frame(stack: &mut A69ValueStack, count: u32) -> bool {
    let frame = stack
        .cursor
        .wrapping_sub(count.wrapping_mul(A69_OBJECT_FRAME_BYTES));
    stack.cursor = frame;
    stack.tag = A69_OBJECT_TAG;
    stack.words = [0; 3];
    true
}

/// `0x0061df70` (`FUN_0061df70`, size 6): return the address of the reader's
/// static descriptor table `0x00f04d24`.
pub fn a69_static_descriptor_table() -> usize {
    A69_STATIC_DESCRIPTOR_TABLE
}

#[cfg(test)]
mod tests_agent69 {
    use super::*;

    /// `0x0061c030`: the fresh header is `0x10 / 4 / 1` in the first nine
    /// bytes and the trailing three bytes are left alone.
    #[test]
    fn value_header_writes_three_constants() {
        let mut header = [0xffu8; 12];
        a69_value_header_init(&mut header);
        assert_eq!(&header[0..4], &[0x10, 0x00, 0x00, 0x00]);
        assert_eq!(&header[4..8], &[0x04, 0x00, 0x00, 0x00]);
        assert_eq!(header[8], 1);
        assert_eq!(&header[9..12], &[0xff, 0xff, 0xff]);
    }

    /// `0x0061c1b0`: one, two, three or four bytes at the retail thresholds.
    #[test]
    fn utf8_append_uses_the_retail_thresholds() {
        let mut ascii = Vec::new();
        assert_eq!(a69_utf8_append(&mut ascii, 0x24), 1);
        assert_eq!(ascii, [0x24]);
        assert_eq!(a69_utf8_append(&mut ascii, 0x7f), 1);
        assert_eq!(ascii, [0x24, 0x7f]);

        let mut two = Vec::new();
        assert_eq!(a69_utf8_append(&mut two, 0x80), 2);
        assert_eq!(two, [0xc2, 0x80]);
        two.clear();
        assert_eq!(a69_utf8_append(&mut two, 0x7ff), 2);
        assert_eq!(two, [0xdf, 0xbf]);

        let mut three = Vec::new();
        assert_eq!(a69_utf8_append(&mut three, 0x800), 3);
        assert_eq!(three, [0xe0, 0xa0, 0x80]);
        three.clear();
        assert_eq!(a69_utf8_append(&mut three, 0xffff), 3);
        assert_eq!(three, [0xef, 0xbf, 0xbf]);

        let mut four = Vec::new();
        assert_eq!(a69_utf8_append(&mut four, 0x10000), 4);
        assert_eq!(four, [0xf0, 0x90, 0x80, 0x80]);
        four.clear();
        assert_eq!(a69_utf8_append(&mut four, 0x1f600), 4);
        assert_eq!(four, [0xf0, 0x9f, 0x98, 0x80]);
    }

    /// `0x0061c240`: the stand-alone emitter always writes four bytes and
    /// encodes `input - 1`.
    #[test]
    fn utf8_offset4_matches_the_four_byte_form_of_input_minus_one() {
        let mut from_offset = Vec::new();
        assert_eq!(a69_utf8_append_offset4(&mut from_offset, 0x1f601), 4);
        let mut direct = Vec::new();
        assert_eq!(a69_utf8_append(&mut direct, 0x1f600), 4);
        assert_eq!(from_offset, direct);
        assert_eq!(from_offset, [0xf0, 0x9f, 0x98, 0x80]);

        // Even below the four-byte threshold the emitter keeps its shape.
        let mut small = Vec::new();
        assert_eq!(a69_utf8_append_offset4(&mut small, 0x40), 4);
        assert_eq!(small, [0xf0, 0x80, 0x80, 0xbf]);
    }

    /// `0x0061c2f0` / `0x0061c370`: the two abort classifications.
    #[test]
    fn guard_abort_separates_null_buffer_from_exhaustion() {
        assert_eq!(a69_append_guard_abort(0, 0), A69BufferAbort::NullBuffer);
        assert_eq!(a69_append_guard_abort(0, 0x1000), A69BufferAbort::Exhausted);
        assert_eq!(a69_append_guard_abort(0x40, 0), A69BufferAbort::Exhausted);
        // The value-stack guard is byte-identical in retail.
        assert_eq!(
            a69_value_stack_guard_abort(0, 0),
            a69_append_guard_abort(0, 0)
        );
        assert_eq!(
            a69_value_stack_guard_abort(0x20, 0x3000),
            A69BufferAbort::Exhausted
        );
    }

    /// `0x0061c540`: four digits, both letter cases, and the error path that
    /// leaves the cursor on the offending byte.
    #[test]
    fn hex_quad_reads_four_digits_and_reports_code_8() {
        let doc = b"1a2Brest";
        let mut cur = A69Cursor { pos: 0, base: 0 };
        assert_eq!(a69_hex_quad(doc, &mut cur, 0), Ok(0x1a2b));
        assert_eq!(cur.pos, 4);

        let doc = b"00Ff";
        let mut cur = A69Cursor { pos: 0, base: 0 };
        assert_eq!(a69_hex_quad(doc, &mut cur, 7), Ok(0x00ff));
        assert_eq!(cur.pos, 4);

        // An invalid digit stops on the bad byte; the third good digit stays
        // consumed because retail advances after every accepted digit.
        let doc = b"12g4";
        let mut cur = A69Cursor { pos: 0, base: 0 };
        assert_eq!(a69_hex_quad(doc, &mut cur, 9), Err((8, 9)));
        assert_eq!(cur.pos, 2);

        // NUL / end of input is just another invalid digit.
        let doc = b"1";
        let mut cur = A69Cursor { pos: 0, base: 0 };
        assert_eq!(a69_hex_quad(doc, &mut cur, 3), Err((8, 3)));
        assert_eq!(cur.pos, 1);
    }

    /// `0x0061d600`: exactly the four whitespace bytes are skipped.
    #[test]
    fn whitespace_scan_covers_space_newline_return_tab_only() {
        let doc = b" \n\r\t  x";
        let mut cur = A69Cursor { pos: 0, base: 0 };
        a69_skip_whitespace(doc, &mut cur);
        assert_eq!(cur.pos, 6);
        assert_eq!(doc[cur.pos], b'x');

        let doc = b"\x0b \x0c";
        let mut cur = A69Cursor { pos: 0, base: 0 };
        a69_skip_whitespace(doc, &mut cur);
        assert_eq!(cur.pos, 0, "vertical tab is not whitespace here");

        let mut cur = A69Cursor { pos: 3, base: 0 };
        a69_skip_whitespace(b"", &mut cur);
        assert_eq!(cur.pos, 3, "an empty tail stops the scan");
    }

    /// `0x0061d402`: the NUL goes at `length` and the cursor pair follows it.
    #[test]
    fn terminate_string_writes_nul_and_publishes_the_cursor() {
        let mut buf = *b"abcdXX";
        let mut published = A69Cursor::default();
        let len = a69_terminate_string(&mut buf, 4, A69Cursor { pos: 11, base: 2 }, &mut published);
        assert_eq!(len, 4);
        assert_eq!(&buf[..], b"abcd\0X");
        assert_eq!(published, A69Cursor { pos: 11, base: 2 });
    }

    /// `0x0061d6f0` / `0x0061d810`: the cursor pair copy and the flag clear.
    #[test]
    fn cursor_pair_copy_and_flag_clear() {
        let mut dst = A69Cursor { pos: 1, base: 2 };
        a69_store_cursor_pair(
            &mut dst,
            A69Cursor {
                pos: 0x30,
                base: 0x10,
            },
        );
        assert_eq!(
            dst,
            A69Cursor {
                pos: 0x30,
                base: 0x10
            }
        );

        let mut flag = 1u8;
        a69_clear_flag(&mut flag);
        assert_eq!(flag, 0);
        a69_clear_flag(&mut flag);
        assert_eq!(flag, 0);
    }

    /// `0x0061d8b0`: only a negative status is accepted.
    #[test]
    fn negative_status_is_the_accepted_path() {
        assert!(a69_error_status_accepted(-1));
        assert!(a69_error_status_accepted(i32::MIN));
        assert!(!a69_error_status_accepted(0));
        assert!(!a69_error_status_accepted(1));
    }

    /// `0x0061d900` / `0x0061d960`: both cursors rewind to their base.
    #[test]
    fn cursor_resets_copy_base_over_cursor() {
        let mut stack = A69ValueStack {
            begin: 0x1000,
            cursor: 0x1040,
            end: 0x2000,
            tag: 7,
            words: [1, 2, 3],
        };
        a69_reset_value_cursor(&mut stack);
        assert_eq!(stack.cursor, 0x1000);
        assert_eq!(stack.tag, 7, "the reset must not touch the frame");
        assert_eq!(stack.words, [1, 2, 3]);

        let mut buf = A69ByteBuf {
            begin: 0x800,
            cursor: 0x812,
        };
        a69_reset_byte_cursor(&mut buf);
        assert_eq!(buf.cursor, 0x800);
        assert_eq!(buf.begin, 0x800);
    }

    /// `0x0061d9b0`: an array frame is 16 bytes per element and tagged 4.
    #[test]
    fn array_frame_steps_back_sixteen_bytes_per_element() {
        let mut stack = A69ValueStack {
            begin: 0x1000,
            cursor: 0x1100,
            end: 0x2000,
            tag: 0,
            words: [9, 9, 9],
        };
        assert!(a69_pop_array_frame(&mut stack, 3));
        assert_eq!(stack.cursor, 0x1100 - 3 * 0x10);
        assert_eq!(stack.tag, A69_ARRAY_TAG);
        assert_eq!(stack.words, [0, 0, 0]);

        assert!(a69_pop_array_frame(&mut stack, 0));
        assert_eq!(stack.cursor, 0x10d0, "an empty array does not move");

        // The 32-bit wrap is part of the retail arithmetic.
        let mut wrapped = A69ValueStack {
            begin: 0,
            cursor: 8,
            end: 0,
            tag: 0,
            words: [1, 1, 1],
        };
        assert!(a69_pop_array_frame(&mut wrapped, 1));
        assert_eq!(wrapped.cursor, 0xffff_fff8);
    }

    /// `0x0061da10`: an object frame is 32 bytes per member and tagged 3.
    #[test]
    fn object_frame_steps_back_thirty_two_bytes_per_member() {
        let mut stack = A69ValueStack {
            begin: 0x1000,
            cursor: 0x1200,
            end: 0x2000,
            tag: 0,
            words: [0xaa; 3],
        };
        assert!(a69_pop_object_frame(&mut stack, 2));
        assert_eq!(stack.cursor, 0x1200 - 2 * 0x20);
        assert_eq!(stack.tag, A69_OBJECT_TAG);
        assert_eq!(stack.words, [0, 0, 0]);
    }

    /// `0x0061df70`: the returned table address is `0x00f04d24`.
    #[test]
    fn static_descriptor_table_address() {
        assert_eq!(a69_static_descriptor_table(), 0x00f0_4d24);
    }

    /// `0x0061d43b`: the epilogue fragment has no visible effect.
    #[test]
    fn unwind_epilogue_is_a_no_op() {
        let before = A69Cursor { pos: 5, base: 1 };
        a69_unwind_epilogue();
        let after = before;
        assert_eq!(before, after);
    }
}
