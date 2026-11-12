#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave14/agent07.rs
// ===========================================================================
// Wave 14 translation batch 07 (staging).
//
// Window: `0067c000..0067dfff` (hex).  The rows in this window are 1.23.10
// instantiations of small support routines: byte-buffer editing, base64
// encoding, a pair of per-player stat tables, and several equality/validation
// helpers.  Nothing here touches `World`, `SimUnit`, the renderer or the
// client; every translation is a free function over values or slices.
//
// ## Recovered layouts
//
// The per-player stat block used by the `0067cfd0`..`0067d2d0` family is
// three global word arrays indexed by the player byte:
//
// | global | element | use |
// |--------|---------|-----|
// | `DAT_00fe5a58` | `u32` | lower bound (minimum) |
// | `DAT_00fe5a88` | `u32` | upper bound (maximum) |
// | `DAT_00fe5ab8` | `u32` | current coefficient |
//
// The decoded object at `DAT_00f063cc ^ 10fae560` holds two more per-player
// counters, at `+0x00` and `+0x30`.  The `DAT_00f8db08` / `DAT_00f8db60`
// tables are `u16` values indexed by a kind byte; `DAT_00f918c8` /
// `DAT_00f91a90` are `u16` components indexed by a 16-bit kind; and
// `DAT_00f921b8` is a byte coefficient table indexed the same way.  Bit
// `0x400` of `DAT_00f8fa08[kind]` doubles that coefficient.
//
// The `0067d580`..`0067d9f0` family operates on `(pointer, length)` byte
// pairs.  The length is the second word of the pair, which is why the
// translated forms take `&[u8]`.
//
// ## Skipped rows
//
// `FUN_0067c160` is dominated by overlapping-instruction and
// `bad instruction data` warnings.  `FUN_0067c260` indexes a `0x18`-stride
// slot array through the decompiler's misaligned negative-index pointer
// spellings, so its field layout is not recoverable.  `FUN_0067c3c0` and
// `FUN_0067c740` are SEH destructor/throw scaffolds whose only local work is
// a pair of sign tests followed by a non-returning runtime call.
// `FUN_0067c450` and `FUN_0067c5a0` are the MSVC `std::_Tree` erase and
// lower-bound-with-insert-hint operations over a node arena that only exists
// in caller registers; the erase tail also needs the out-of-window rebalance
// callback.  `FUN_0067c690` is the simpler exact-key tree lookup and is
// translated below.
// `FUN_0067c820` is an `unaff_EBP`/`unaff_EBX` frame fragment with a
// `bad instruction data` warning.  `FUN_0067c9d0` is the base64 decoder, but
// its recovered guard is inverted (`(len >> 2) * 3 == 0` encloses the decode
// loop) and every path ends in the non-returning allocator call, so the
// useful result is not present in the body.  `FUN_0067cced` is a duplicate
// mid-function slice of the base64 encoder with `unaff_*` operands.
// `FUN_0067ced0` chains four opaque unit-stat getters and an opaque report
// sink; only the coefficient scaling is local.  `FUN_0067d380` computes a
// string length and immediately throws.  `FUN_0067d5d0`, `FUN_0067d7b0`,
// `FUN_0067d901`, `FUN_0067d982`, `FUN_0067daa0`, `FUN_0067dcc0`,
// `FUN_0067dda0`, `FUN_0067de2a` and `FUN_0067de80` are string/list
// constructor or exception-landing fragments truncated at the same
// non-returning allocator/unwinder calls, with arguments read from
// `unaff_EBP`/`unaff_EBX` rather than the routine's stack frame.

// ---------------------------------------------------------------------------
// Guarded dispatch
// ---------------------------------------------------------------------------

/// `0x0067c380`: guarded callback dispatch.
///
/// The retail routine ignores its first argument, returns zero on every path,
/// and calls `FUN_0067b3c0` (with its fixed `DAT_00f063d8` context in `ECX`)
/// only when the two identifiers are equal and both payload words are
/// non-zero.  The callback receives the five stack arguments in the same
/// order the retail call pushes them.
pub fn w14a07_guarded_emit<F>(
    _unused: u32,
    first: u32,
    second: u32,
    payload: u32,
    left_id: u32,
    right_id: u32,
    emit: F,
) -> u32
where
    F: FnOnce(u32, u32, u32, u32, u32),
{
    if left_id == right_id && first != 0 && second != 0 {
        emit(first, second, payload, left_id, right_id);
    }
    0
}

// ---------------------------------------------------------------------------
// Byte-buffer editing
// ---------------------------------------------------------------------------

/// `0x0067c6f0`: remove every byte equal to `marker` from `buffer` starting at
/// the first occurrence of `marker`.
///
/// Bytes before the first occurrence are never moved.  The retail routine
/// works on a `[first, last)` pointer pair and returns nothing; this form
/// returns the resulting logical length.  When `marker` is absent the buffer
/// is unchanged and its full length is returned.
pub fn w14a07_remove_from_first(buffer: &mut [u8], marker: u8) -> usize {
    let Some(first) = buffer.iter().position(|&byte| byte == marker) else {
        return buffer.len();
    };
    let mut write = first;
    for read in first..buffer.len() {
        let byte = buffer[read];
        if byte != marker {
            buffer[write] = byte;
            write += 1;
        }
    }
    write
}

/// `0x0067c970`: erase a byte range from a NUL-terminated, length-tracked
/// buffer.
///
/// `first` and `last` are the byte offsets the retail caller passes as its
/// two pointers.  The erased count is the `u32` difference `last - first`,
/// clamped to the bytes available from `first`; an inverted range therefore
/// subtracts to a huge unsigned count and erases through the end, exactly as
/// the shipped `cmovae` does.  The retail routine then moves the tail down,
/// writes a NUL at the new length and returns its `first` pointer; this form
/// returns the same offset and keeps the buffer's length authoritative.
///
/// The retail callers guarantee `first <= len`; larger offsets are clamped
/// here instead of reproducing the pointer underflow.
pub fn w14a07_erase_byte_range(buffer: &mut Vec<u8>, first: usize, last: usize) -> usize {
    let offset = first.min(buffer.len());
    let requested = last.wrapping_sub(first);
    let count = requested.min(buffer.len() - offset);
    if count != 0 {
        buffer.drain(offset..offset + count);
    }
    first
}

// ---------------------------------------------------------------------------
// Base64
// ---------------------------------------------------------------------------

/// The 64-character alphabet used by the encoder at `0x0067cc50`.
pub const W14A07_BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `0x0067cc50`: encode a byte slice as base64 with `=` padding.
///
/// The retail routine writes into a caller-owned string buffer whose capacity
/// is `((len + 2) / 3) * 4 + 1`; the extra byte is the NUL terminator.  Three
/// input bytes become four alphabet characters, a one-byte tail emits two
/// characters and `==`, and a two-byte tail emits three characters and `=`.
pub fn w14a07_base64_encode(input: &[u8]) -> String {
    let mut output = String::with_capacity(input.len().div_ceil(3) * 4);
    let alphabet = W14A07_BASE64_ALPHABET;
    for chunk in input.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        output.push(alphabet[usize::from(first >> 2)] as char);
        output.push(alphabet[usize::from((first & 0x03) << 4 | second >> 4)] as char);
        if chunk.len() > 1 {
            output.push(alphabet[usize::from((second & 0x0f) << 2 | third >> 6)] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(alphabet[usize::from(third & 0x3f)] as char);
        } else {
            output.push('=');
        }
    }
    output
}

// ---------------------------------------------------------------------------
// String-keyed tree lookup
// ---------------------------------------------------------------------------

/// One node of the MSVC `std::_Tree` arena used by the `0067c450`..`0067c690`
/// family.
///
/// The shipped node stores its links at `+0x00` (parent), `+0x04` (left) and
/// `+0x08` (right), a color byte at `+0x0c`, and the key's string header at
/// `+0x10`.  This arena model keeps the three links and the key bytes; the
/// color byte is only read by the erase and lower-bound rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14a07TreeNode<'a> {
    /// Parent link (`+0x00`), `None` for a detached root.
    pub parent: Option<usize>,
    /// Left child link (`+0x04`).
    pub left: Option<usize>,
    /// Right child link (`+0x08`).
    pub right: Option<usize>,
    /// Key bytes stored behind the string header at `+0x10`.
    pub key: &'a [u8],
}

/// `0x0067c690`: exact-key lookup in a string-keyed binary-search tree.
///
/// The retail search compares `node.key` with the search key, descends left
/// while `search <= node.key` (remembering the last such node) and right
/// while `search > node.key`, then requires `search >= candidate.key`.  A miss
/// returns the tree's `head` iterator; the arena form returns `None`.
pub fn w14a07_tree_find<'a>(
    nodes: &[W14a07TreeNode<'a>],
    root: Option<usize>,
    key: &[u8],
) -> Option<usize> {
    let mut node = root;
    let mut candidate = None;
    while let Some(index) = node {
        let entry = nodes.get(index)?;
        if entry.key < key {
            node = entry.right;
        } else {
            candidate = Some(index);
            node = entry.left;
        }
    }
    let index = candidate?;
    (nodes[index].key <= key).then_some(index)
}

// ---------------------------------------------------------------------------
// Per-player stat tables
// ---------------------------------------------------------------------------

/// One player's recovered stat triple.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14a07StatValues {
    /// Word stored in the `DAT_00fe5a58` array.
    pub minimum: u32,
    /// Word stored in the `DAT_00fe5a88` array.
    pub maximum: u32,
    /// Word stored in the `DAT_00fe5ab8` array.
    pub current: u32,
}

/// The three components read by `0x0067d2d0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14a07TableLookup {
    /// `DAT_00f918c8[index]`, zero-extended.
    pub first: u32,
    /// `DAT_00f91a90[index]`, zero-extended.
    pub second: u32,
    /// `DAT_00f921b8[index]`, optionally doubled.
    pub coefficient: u8,
}

/// Bit in `DAT_00f8fa08[index]` that doubles the coefficient byte.
pub const W14A07_FLAG_COEFFICIENT_DOUBLED: u32 = 0x400;

/// Report code used by `0x0067cfd0` when the first counter is below its
/// minimum.
pub const W14A07_REPORT_BELOW_MINIMUM: u16 = 0x352;

/// Report code used by `0x0067cfd0` when the second counter is above its
/// maximum.
pub const W14A07_REPORT_ABOVE_MAXIMUM: u16 = 0x353;

/// `0x0067ce30`: add the full `u16` table values to the two per-player
/// counters.
///
/// The first counter lives at `+0x00` of the decoded per-player record and
/// takes `first_table[kind]`; the second lives at `+0x30` and takes
/// `second_table[kind]`.  The retail indexes are unguarded global arrays, so
/// this form returns `false` without writing when any index is out of range.
pub fn w14a07_add_full_values(
    player: u8,
    kind: u16,
    first: &mut [i32],
    second: &mut [i32],
    first_table: &[u16],
    second_table: &[u16],
) -> bool {
    let index = usize::from(player);
    if index >= first.len() || index >= second.len() {
        return false;
    }
    let (Some(&first_delta), Some(&second_delta)) = (
        first_table.get(usize::from(kind)),
        second_table.get(usize::from(kind)),
    ) else {
        return false;
    };
    first[index] = first[index].wrapping_add(i32::from(first_delta));
    second[index] = second[index].wrapping_add(i32::from(second_delta));
    true
}

/// `0x0067ce70`: add three quarters of each `u16` table value to the two
/// per-player counters.
///
/// The retail arithmetic is `(value * 3) >> 2` on a zero-extended `u16`, i.e.
/// floor division by four because the product is non-negative.  Range
/// handling matches [`w14a07_add_full_values`].
pub fn w14a07_add_three_quarters(
    player: u8,
    kind: u16,
    first: &mut [i32],
    second: &mut [i32],
    first_table: &[u16],
    second_table: &[u16],
) -> bool {
    let index = usize::from(player);
    if index >= first.len() || index >= second.len() {
        return false;
    }
    let (Some(&first_value), Some(&second_value)) = (
        first_table.get(usize::from(kind)),
        second_table.get(usize::from(kind)),
    ) else {
        return false;
    };
    let first_delta = (i32::from(first_value) * 3) >> 2;
    let second_delta = (i32::from(second_value) * 3) >> 2;
    first[index] = first[index].wrapping_add(first_delta);
    second[index] = second[index].wrapping_add(second_delta);
    true
}

/// `0x0067cfd0`: validate a player's two counters against the stored bounds.
///
/// The result is `(true, None)` when `current_first >= minimum_first` and
/// `maximum_second <= current_second`.  The first failing comparison decides
/// the report code: `0x352` when the first counter is below its minimum,
/// `0x353` when the second counter is above its maximum.  The retail routine
/// only dispatches the report when its second argument is non-zero; pass
/// `report` to model that argument.
pub fn w14a07_validate_stat_pair(
    current_first: i32,
    minimum_first: i32,
    maximum_second: i32,
    current_second: i32,
    report: bool,
) -> (bool, Option<u16>) {
    if current_first < minimum_first {
        return (false, report.then_some(W14A07_REPORT_BELOW_MINIMUM));
    }
    if maximum_second <= current_second {
        return (true, None);
    }
    (false, report.then_some(W14A07_REPORT_ABOVE_MAXIMUM))
}

/// `0x0067d0a0`: compute a player's stat triple from two `u16` tables indexed
/// by a kind byte.
///
/// The current coefficient is reset to zero.  The retail routine stores the
/// three words into the per-player arrays and then calls `FUN_0067cfd0` with
/// `player` and its report argument; the call's result is discarded, so it is
/// not modelled here.
pub fn w14a07_stat_from_tables(
    kind: u8,
    minimum_table: &[u16],
    maximum_table: &[u16],
) -> Option<W14a07StatValues> {
    let index = usize::from(kind);
    Some(W14a07StatValues {
        minimum: u32::from(*minimum_table.get(index)?),
        maximum: u32::from(*maximum_table.get(index)?),
        current: 0,
    })
}

/// `0x0067d220`: compute a player's stat triple from two opaque probes.
///
/// The retail body calls `FUN_005dac50(player, argument)` for the minimum and
/// `FUN_005dac10(player, argument)` for the maximum, truncating each result to
/// 16 bits, then zeroes the current coefficient and calls `FUN_0067cfd0`.
/// The probes are supplied as callbacks so no opaque global is assumed.
pub fn w14a07_stat_from_probe<F, G>(
    player: u32,
    argument: u32,
    minimum_probe: F,
    maximum_probe: G,
) -> W14a07StatValues
where
    F: FnOnce(u32, u32) -> u32,
    G: FnOnce(u32, u32) -> u32,
{
    let minimum = minimum_probe(player, argument) & 0xffff;
    let maximum = maximum_probe(player, argument) & 0xffff;
    W14a07StatValues {
        minimum,
        maximum,
        current: 0,
    }
}

/// `0x0067d270`: read a player's current coefficient from the
/// `DAT_00fe5ab8` array.
pub fn w14a07_stat_current(table: &[u32], player: usize) -> Option<u32> {
    table.get(player).copied()
}

/// `0x0067d280`: read a player's upper bound from the `DAT_00fe5a88` array.
pub fn w14a07_stat_maximum(table: &[u32], player: usize) -> Option<u32> {
    table.get(player).copied()
}

/// `0x0067d290`: read a player's lower bound from the `DAT_00fe5a58` array.
pub fn w14a07_stat_minimum(table: &[u32], player: usize) -> Option<u32> {
    table.get(player).copied()
}

/// `0x0067d2a0`: read the two `u16` values for `index`.
///
/// The retail routine writes each value only when the corresponding
/// out-pointer is non-null; this form returns both.  The index is a full
/// `int` in the shipped code and is unbounded in `[0, 65535]` terms, hence
/// the `usize` parameter.
pub fn w14a07_table_values(
    index: usize,
    first_table: &[u16],
    second_table: &[u16],
) -> Option<(u32, u32)> {
    Some((
        u32::from(*first_table.get(index)?),
        u32::from(*second_table.get(index)?),
    ))
}

/// `0x0067d2d0`: read both `u16` values and the coefficient byte for a
/// 16-bit kind.
///
/// The coefficient comes from `DAT_00f921b8`; when bit `0x400` is set in
/// `DAT_00f8fa08[kind]` the retail code doubles it with an 8-bit `add al,al`,
/// so the doubling wraps modulo 256.
pub fn w14a07_table_lookup(
    kind: u16,
    first_table: &[u16],
    second_table: &[u16],
    coefficient_table: &[u8],
    flags: &[u32],
) -> Option<W14a07TableLookup> {
    let index = usize::from(kind);
    let first = u32::from(*first_table.get(index)?);
    let second = u32::from(*second_table.get(index)?);
    let mut coefficient = *coefficient_table.get(index)?;
    if flags.get(index)? & W14A07_FLAG_COEFFICIENT_DOUBLED != 0 {
        coefficient = coefficient.wrapping_mul(2);
    }
    Some(W14a07TableLookup {
        first,
        second,
        coefficient,
    })
}

/// `0x0067d320`: query two opaque per-pair probes and truncate each result to
/// 16 bits.
///
/// The retail routine calls `FUN_005dac50(first, second)` only when the first
/// out-pointer is non-null and `FUN_005dac10(first, second)` only when the
/// second is non-null.  `want_first` / `want_second` model those two
/// pointers; a probe is not invoked when its flag is false.
pub fn w14a07_probe_pair<F, G>(
    first: u8,
    second: u8,
    want_first: bool,
    want_second: bool,
    first_probe: F,
    second_probe: G,
) -> (Option<u32>, Option<u32>)
where
    F: FnOnce(u8, u8) -> u32,
    G: FnOnce(u8, u8) -> u32,
{
    let low = want_first.then(|| first_probe(first, second) & 0xffff);
    let high = want_second.then(|| second_probe(first, second) & 0xffff);
    (low, high)
}

// ---------------------------------------------------------------------------
// Equality helpers
// ---------------------------------------------------------------------------

/// `0x0067d580`: compare the trailing `other.len()` bytes of `data` with
/// `other`.
///
/// When `data` is shorter, the retail routine returns
/// `other_len & 0xffff_ff00` (it clears the low byte of the length still in
/// `EAX`); otherwise it delegates to the length-equality routine and returns
/// `0` or `1`.  `word_compare` selects the same two strategies as
/// [`w14a07_bytes_equal`].
pub fn w14a07_compare_tail(data: &[u8], other: &[u8], word_compare: bool) -> u32 {
    if data.len() < other.len() {
        return (other.len() as u32) & 0xffff_ff00;
    }
    let start = data.len() - other.len();
    u32::from(w14a07_bytes_equal(&data[start..], other, word_compare))
}

/// `0x0067d6b0`: exact-length byte equality with the word-strategy disabled.
///
/// The retail routine is a four-argument wrapper that tail-calls the
/// comparator at `0x0067d9f0` with `param_5 == 0`.
pub fn w14a07_bytes_equal_exact(left: &[u8], right: &[u8]) -> bool {
    w14a07_bytes_equal(left, right, false)
}

/// `0x0067d9b0`: compare `data`'s leading `other.len()` bytes with `other`.
///
/// The retail wrapper passes `min(data_len, other_len)` as the first length
/// and leaves the second length unchanged, so unequal lengths fail before the
/// comparison and a shorter `data` always yields `false`.
pub fn w14a07_compare_prefix(data: &[u8], other: &[u8], word_compare: bool) -> bool {
    if data.len() < other.len() {
        return false;
    }
    w14a07_bytes_equal(&data[..other.len()], other, word_compare)
}

/// `0x0067d9f0`: length-checked byte equality with two comparison strategies.
///
/// Unequal lengths return `false` immediately.  With `word_compare == false`
/// the retail routine tail-calls `FUN_00c94358` (`memcmp`).  With
/// `word_compare == true` it compares whole 32-bit words and finishes with
/// the remaining 0..3 bytes; both routes return the same equality answer.
pub fn w14a07_bytes_equal(left: &[u8], right: &[u8], word_compare: bool) -> bool {
    if left.len() != right.len() {
        return false;
    }
    if !word_compare {
        return left == right;
    }
    let mut offset = 0;
    while left.len() - offset > 3 {
        if left[offset..offset + 4] != right[offset..offset + 4] {
            break;
        }
        offset += 4;
    }
    left[offset..] == right[offset..]
}

// ---------------------------------------------------------------------------
// Value predicates and scalar readers
// ---------------------------------------------------------------------------

/// `0x0067d6e0`: the recovered 22-value type set.
///
/// The shipped dispatch does a signed-byte decrement followed by an unsigned
/// comparison against `0x1e`, then consults a 31-byte jump table.  Only the
/// values `1..=8`, `0x0e..=0x11`, `0x15..=0x19` and `0x1b..=0x1f` take the
/// `return 1` arm; every other byte, including `0`, returns zero.
pub fn w14a07_is_tracked_type(value: u8) -> bool {
    matches!(
        value,
        1..=8 | 0x0e..=0x11 | 0x15..=0x19 | 0x1b..=0x1f
    )
}

/// `0x0067d730`: read a byte buffer as a big-endian accumulator.
///
/// The retail loop executes `value = value << 8 | (int)(char)byte`.  The byte
/// is loaded with `movsbl`, so bytes at or above `0x80` sign-extend and set
/// the high 24 bits of the accumulator.  Shifting drops bits above bit 31 and
/// every OR is 32-bit, which [`u32`] arithmetic reproduces.
pub fn w14a07_read_big_endian(bytes: &[u8]) -> u32 {
    let mut value = 0u32;
    for &byte in bytes {
        value = (value << 8) | (byte as i8 as i32 as u32);
    }
    value
}

/// `0x0067d770`: forward two two-word bound records to `FUN_0067e4a0`.
///
/// The retail routine places its first argument in `ECX` and pushes the four
/// words of the two records.  The callback receives the context word first,
/// followed by the four record words.
pub fn w14a07_forward_bounds<F>(context: u32, first: (u32, u32), second: (u32, u32), forward: F)
where
    F: FnOnce(u32, u32, u32, u32, u32),
{
    forward(context, first.0, first.1, second.0, second.1);
}

/// `0x0067dbd0`: validate every byte of a control record.
///
/// The retail loop throws when a byte is printable (`>= 0x20`) or is one of
/// TAB, LF or CR, and also throws when `0x0067d6e0` reports the byte in the
/// tracked-type set.  A record is accepted only when every byte is a control
/// value outside those exclusions; the translation returns that predicate
/// instead of throwing.
pub fn w14a07_valid_control_record(bytes: &[u8]) -> bool {
    bytes.iter().all(|&byte| {
        byte < 0x20 && byte != 9 && byte != 10 && byte != 13 && !w14a07_is_tracked_type(byte)
    })
}

#[cfg(test)]
mod tests_wave14_agent07 {
    use super::*;

    #[test]
    fn guarded_emit_calls_only_for_matching_nonzero_payload() {
        let mut seen = None;
        let result = w14a07_guarded_emit(0, 0x11, 0x22, 0x33, 7, 7, |a, b, c, l, r| {
            seen = Some((a, b, c, l, r));
        });
        assert_eq!(result, 0);
        assert_eq!(seen, Some((0x11, 0x22, 0x33, 7, 7)));

        seen = None;
        assert_eq!(
            w14a07_guarded_emit(0, 0x11, 0x22, 0x33, 7, 8, |_, _, _, _, _| seen =
                Some((1, 2, 3, 4, 5))),
            0
        );
        assert!(seen.is_none());

        assert_eq!(
            w14a07_guarded_emit(0, 0, 0x22, 0x33, 7, 7, |_, _, _, _, _| seen =
                Some((1, 2, 3, 4, 5))),
            0
        );
        assert!(seen.is_none());

        assert_eq!(
            w14a07_guarded_emit(0, 0x11, 0, 0x33, 7, 7, |_, _, _, _, _| seen =
                Some((1, 2, 3, 4, 5))),
            0
        );
        assert!(seen.is_none());
    }

    #[test]
    fn remove_from_first_compacts_the_suffix_only() {
        let mut buffer = *b"ab,cd,,e";
        assert_eq!(w14a07_remove_from_first(&mut buffer, b','), 5);
        assert_eq!(&buffer[..5], b"abcde");

        let mut none = *b"abc";
        assert_eq!(w14a07_remove_from_first(&mut none, b','), 3);
        assert_eq!(&none, b"abc");

        let mut leading = *b",,a,b";
        assert_eq!(w14a07_remove_from_first(&mut leading, b','), 2);
        assert_eq!(&leading[..2], b"ab");

        let mut empty = [];
        assert_eq!(w14a07_remove_from_first(&mut empty, b','), 0);
    }

    #[test]
    fn erase_byte_range_matches_the_pointer_erase() {
        let mut buffer = b"abcdef".to_vec();
        assert_eq!(w14a07_erase_byte_range(&mut buffer, 1, 4), 1);
        assert_eq!(buffer, b"aef");

        let mut whole = b"abc".to_vec();
        assert_eq!(w14a07_erase_byte_range(&mut whole, 0, 3), 0);
        assert!(whole.is_empty());

        let mut tail = b"abc".to_vec();
        assert_eq!(w14a07_erase_byte_range(&mut tail, 2, 100), 2);
        assert_eq!(tail, b"ab");

        let mut noop = b"abc".to_vec();
        assert_eq!(w14a07_erase_byte_range(&mut noop, 2, 2), 2);
        assert_eq!(noop, b"abc");

        // The shipped count is an unsigned pointer difference: an inverted
        // range wraps and erases through the end of the buffer.
        let mut inverted = b"abcdef".to_vec();
        assert_eq!(w14a07_erase_byte_range(&mut inverted, 4, 1), 4);
        assert_eq!(inverted, b"abcd");
    }

    #[test]
    fn base64_encoder_matches_rfc_vectors_and_padding() {
        assert_eq!(w14a07_base64_encode(b""), "");
        assert_eq!(w14a07_base64_encode(b"M"), "TQ==");
        assert_eq!(w14a07_base64_encode(b"Ma"), "TWE=");
        assert_eq!(w14a07_base64_encode(b"Man"), "TWFu");
        assert_eq!(w14a07_base64_encode(b"Many"), "TWFueQ==");
        assert_eq!(w14a07_base64_encode(b"abcde"), "YWJjZGU=");
        assert_eq!(w14a07_base64_encode(&[0x00, 0xff, 0x10]), "AP8Q");
    }

    #[test]
    fn tree_find_matches_the_string_key_search() {
        let nodes = [
            W14a07TreeNode {
                parent: None,
                left: Some(1),
                right: Some(2),
                key: b"m",
            },
            W14a07TreeNode {
                parent: Some(0),
                left: Some(3),
                right: Some(4),
                key: b"c",
            },
            W14a07TreeNode {
                parent: Some(0),
                left: None,
                right: Some(5),
                key: b"x",
            },
            W14a07TreeNode {
                parent: Some(1),
                left: None,
                right: None,
                key: b"a",
            },
            W14a07TreeNode {
                parent: Some(1),
                left: None,
                right: None,
                key: b"e",
            },
            W14a07TreeNode {
                parent: Some(2),
                left: None,
                right: None,
                key: b"z",
            },
        ];
        assert_eq!(w14a07_tree_find(&nodes, Some(0), b"m"), Some(0));
        assert_eq!(w14a07_tree_find(&nodes, Some(0), b"c"), Some(1));
        assert_eq!(w14a07_tree_find(&nodes, Some(0), b"e"), Some(4));
        assert_eq!(w14a07_tree_find(&nodes, Some(0), b"z"), Some(5));
        assert_eq!(w14a07_tree_find(&nodes, Some(0), b"b"), None);
        assert_eq!(w14a07_tree_find(&nodes, Some(0), b"zz"), None);
        assert_eq!(w14a07_tree_find(&nodes, Some(0), b"aa"), None);
        assert_eq!(w14a07_tree_find(&nodes, None, b"m"), None);
    }

    #[test]
    fn add_full_values_updates_both_counters() {
        let mut first = [10i32, 20, 30];
        let mut second = [-1i32, 0, 1];
        let first_table = [7u16, 8, 9];
        let second_table = [100u16, 200, 300];
        assert!(w14a07_add_full_values(
            1,
            2,
            &mut first,
            &mut second,
            &first_table,
            &second_table
        ));
        assert_eq!(first, [10, 29, 30]);
        assert_eq!(second, [-1, 300, 1]);

        assert!(!w14a07_add_full_values(
            3,
            0,
            &mut first,
            &mut second,
            &first_table,
            &second_table
        ));
        assert!(!w14a07_add_full_values(
            0,
            3,
            &mut first,
            &mut second,
            &first_table,
            &second_table
        ));
        assert_eq!(first, [10, 29, 30]);
    }

    #[test]
    fn add_three_quarters_truncates_the_product() {
        let mut first = [0i32, 0];
        let mut second = [0i32, 0];
        let first_table = [1u16, 2, 3, 4, 5];
        let second_table = [6u16, 7, 8, 9, 10];
        assert!(w14a07_add_three_quarters(
            0,
            4,
            &mut first,
            &mut second,
            &first_table,
            &second_table
        ));
        assert_eq!(first[0], 3);
        assert_eq!(second[0], 7);

        let mut first = [0i32];
        let mut second = [0i32];
        assert!(w14a07_add_three_quarters(
            0,
            0,
            &mut first,
            &mut second,
            &first_table,
            &second_table
        ));
        assert_eq!(first[0], 0);
        assert_eq!(second[0], 4);
    }

    #[test]
    fn validate_stat_pair_returns_shipped_report_codes() {
        assert_eq!(w14a07_validate_stat_pair(5, 5, 10, 10, true), (true, None));
        assert_eq!(
            w14a07_validate_stat_pair(4, 5, 10, 10, true),
            (false, Some(W14A07_REPORT_BELOW_MINIMUM))
        );
        assert_eq!(
            w14a07_validate_stat_pair(4, 5, 10, 10, false),
            (false, None)
        );
        assert_eq!(
            w14a07_validate_stat_pair(5, 5, 11, 10, true),
            (false, Some(W14A07_REPORT_ABOVE_MAXIMUM))
        );
        assert_eq!(
            w14a07_validate_stat_pair(5, 5, 11, 10, false),
            (false, None)
        );
        // The first comparison wins when both would fail.
        assert_eq!(
            w14a07_validate_stat_pair(0, 1, 100, 0, true),
            (false, Some(W14A07_REPORT_BELOW_MINIMUM))
        );
    }

    #[test]
    fn stat_from_tables_resets_current_and_checks_indices() {
        let minimums = [1u16, 2, 3];
        let maximums = [10u16, 20, 30];
        assert_eq!(
            w14a07_stat_from_tables(1, &minimums, &maximums),
            Some(W14a07StatValues {
                minimum: 2,
                maximum: 20,
                current: 0
            })
        );
        assert_eq!(w14a07_stat_from_tables(3, &minimums, &maximums), None);
    }

    #[test]
    fn stat_from_probe_calls_both_probes_in_order() {
        use std::cell::RefCell;
        let calls = RefCell::new(Vec::new());
        let values = w14a07_stat_from_probe(
            7,
            0x1234,
            |player, argument| {
                calls.borrow_mut().push((0, player, argument));
                0x1_2345
            },
            |player, argument| {
                calls.borrow_mut().push((1, player, argument));
                0x6_789a
            },
        );
        assert_eq!(
            values,
            W14a07StatValues {
                minimum: 0x2345,
                maximum: 0x789a,
                current: 0
            }
        );
        assert_eq!(*calls.borrow(), vec![(0, 7, 0x1234), (1, 7, 0x1234)]);
    }

    #[test]
    fn stat_getters_read_the_three_parallel_arrays() {
        let table = [11u32, 22, 33];
        assert_eq!(w14a07_stat_current(&table, 1), Some(22));
        assert_eq!(w14a07_stat_maximum(&table, 2), Some(33));
        assert_eq!(w14a07_stat_minimum(&table, 0), Some(11));
        assert_eq!(w14a07_stat_current(&table, 3), None);
        assert_eq!(w14a07_stat_maximum(&table, 9), None);
        assert_eq!(w14a07_stat_minimum(&table, 9), None);
    }

    #[test]
    fn table_values_zero_extend_both_words() {
        let firsts = [0u16, 0x1234, 0xffff];
        let seconds = [1u16, 0x5678, 0xfffe];
        assert_eq!(
            w14a07_table_values(1, &firsts, &seconds),
            Some((0x1234, 0x5678))
        );
        assert_eq!(
            w14a07_table_values(2, &firsts, &seconds),
            Some((0xffff, 0xfffe))
        );
        assert_eq!(w14a07_table_values(3, &firsts, &seconds), None);
    }

    #[test]
    fn table_lookup_applies_the_flag_and_wraps_the_doubling() {
        let firsts = [1u16, 2];
        let seconds = [3u16, 4];
        let coefficients = [5u8, 0x81];
        let flags = [0u32, W14A07_FLAG_COEFFICIENT_DOUBLED];
        assert_eq!(
            w14a07_table_lookup(1, &firsts, &seconds, &coefficients, &flags),
            Some(W14a07TableLookup {
                first: 2,
                second: 4,
                coefficient: 0x02
            })
        );
        assert_eq!(
            w14a07_table_lookup(0, &firsts, &seconds, &coefficients, &flags),
            Some(W14a07TableLookup {
                first: 1,
                second: 3,
                coefficient: 5
            })
        );
        assert_eq!(
            w14a07_table_lookup(2, &firsts, &seconds, &coefficients, &flags),
            None
        );
    }

    #[test]
    fn probe_pair_calls_only_requested_probes() {
        use std::cell::Cell;
        let first_calls = Cell::new(0);
        let second_calls = Cell::new(0);
        let result = w14a07_probe_pair(
            3,
            9,
            true,
            false,
            |a, b| {
                first_calls.set(first_calls.get() + 1);
                assert_eq!((a, b), (3, 9));
                0x1_2345
            },
            |_, _| {
                second_calls.set(second_calls.get() + 1);
                0xffff_ffff
            },
        );
        assert_eq!(result, (Some(0x2345), None));
        assert_eq!(first_calls.get(), 1);
        assert_eq!(second_calls.get(), 0);

        let result = w14a07_probe_pair(0, 0, false, true, |_, _| 1, |_, _| 0x1_0002);
        assert_eq!(result, (None, Some(2)));

        let result = w14a07_probe_pair(0, 0, false, false, |_, _| 1, |_, _| 2);
        assert_eq!(result, (None, None));
    }

    #[test]
    fn compare_tail_handles_short_data_and_suffixes() {
        assert_eq!(w14a07_compare_tail(b"xxabc", b"abc", false), 1);
        assert_eq!(w14a07_compare_tail(b"xxabd", b"abc", false), 0);
        assert_eq!(w14a07_compare_tail(b"xxabc", b"abc", true), 1);
        assert_eq!(w14a07_compare_tail(b"ab", &[0u8; 300], false), 256);
        assert_eq!(w14a07_compare_tail(b"", b"", false), 1);
    }

    #[test]
    fn compare_prefix_and_exact_equality() {
        assert!(w14a07_compare_prefix(b"abcdef", b"abc", false));
        assert!(w14a07_compare_prefix(b"abcdef", b"abc", true));
        assert!(!w14a07_compare_prefix(b"abc", b"abcdef", false));
        assert!(!w14a07_compare_prefix(b"abd", b"abc", false));
        assert!(w14a07_bytes_equal_exact(b"same", b"same"));
        assert!(!w14a07_bytes_equal_exact(b"same", b"samf"));
        assert!(!w14a07_bytes_equal_exact(b"same", b"same!"));
    }

    #[test]
    fn bytes_equal_covers_both_strategies_and_chunk_boundaries() {
        for length in 0..=9usize {
            let left: Vec<u8> = (0..length).map(|value| value as u8).collect();
            let mut right = left.clone();
            assert!(w14a07_bytes_equal(&left, &right, false));
            assert!(w14a07_bytes_equal(&left, &right, true));
            if let Some(last) = right.last_mut() {
                *last ^= 0x80;
                assert!(!w14a07_bytes_equal(&left, &right, false));
                assert!(!w14a07_bytes_equal(&left, &right, true));
            }
        }
        assert!(w14a07_bytes_equal(
            &[1, 2, 3, 4, 5, 6, 7, 8],
            &[1, 2, 3, 4, 5, 6, 7, 8],
            true
        ));
        assert!(!w14a07_bytes_equal(
            &[1, 2, 3, 4, 5, 6, 7, 8],
            &[1, 2, 3, 4, 5, 6, 7, 9],
            true
        ));
        assert!(!w14a07_bytes_equal(b"abc", b"ab", true));
    }

    #[test]
    fn tracked_type_predicate_matches_the_jump_table() {
        for value in 0u8..=255 {
            let expected = matches!(
                value,
                1..=8 | 0x0e..=0x11 | 0x15..=0x19 | 0x1b..=0x1f
            );
            assert_eq!(w14a07_is_tracked_type(value), expected, "value {value}");
        }
        assert!(!w14a07_is_tracked_type(0));
        assert!(w14a07_is_tracked_type(1));
        assert!(w14a07_is_tracked_type(0x1f));
        assert!(!w14a07_is_tracked_type(0x20));
    }

    #[test]
    fn read_big_endian_sign_extends_bytes() {
        assert_eq!(w14a07_read_big_endian(&[]), 0);
        assert_eq!(w14a07_read_big_endian(&[0x01]), 0x01);
        assert_eq!(
            w14a07_read_big_endian(&[0x01, 0x02, 0x03, 0x04]),
            0x0102_0304
        );
        assert_eq!(
            w14a07_read_big_endian(&[0x11, 0x22, 0x33, 0x44, 0x55]),
            0x2233_4455
        );
        assert_eq!(w14a07_read_big_endian(&[0x84]), 0xffff_ff84);
        assert_eq!(
            w14a07_read_big_endian(&[0x01, 0x02, 0x03, 0x84]),
            0xffff_ff84
        );
    }

    #[test]
    fn forward_bounds_preserves_the_context_and_two_pairs() {
        use std::cell::Cell;
        let seen = Cell::new((0u32, 0u32, 0u32, 0u32, 0u32));
        w14a07_forward_bounds(9, (1, 2), (3, 4), |context, a, b, c, d| {
            seen.set((context, a, b, c, d));
        });
        assert_eq!(seen.get(), (9, 1, 2, 3, 4));
    }

    #[test]
    fn control_record_validation_matches_the_throw_conditions() {
        assert!(w14a07_valid_control_record(&[]));
        assert!(w14a07_valid_control_record(&[
            0, 0x0b, 0x0c, 0x12, 0x13, 0x14, 0x1a
        ]));
        assert!(!w14a07_valid_control_record(&[0x20]));
        assert!(!w14a07_valid_control_record(&[9]));
        assert!(!w14a07_valid_control_record(&[10]));
        assert!(!w14a07_valid_control_record(&[13]));
        assert!(!w14a07_valid_control_record(&[1]));
        assert!(!w14a07_valid_control_record(&[0x1f]));
        assert!(!w14a07_valid_control_record(&[0x00, 0x41, 0x00]));
    }
}
