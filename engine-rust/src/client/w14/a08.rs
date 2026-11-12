#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave14/agent08.rs
// ===========================================================================
// Wave 14 translation batch 08.
//
// Window: `67e000..67ffff`.  This window is library code, not combat code:
// the shipped runtime keeps two small-string classes and the containers that
// hold them here.
//
// * `0067e030`, `0067e150`, `0067e1c0`, `0067e2e0`, `0067e360` and
//   `0067e440` are the UTF-32 side: the `String` object stores 32-bit code
//   points with fifteen inline slots, and the routines trim, search and
//   assign it.  The two `...030`/`...1c0` rows are the by-value wrappers: they
//   widen their two 8-bit arguments through the process-locale conversion
//   helper, trim, and return the new string through the hidden result
//   pointer.
// * `0067e4a0` is the byte-oriented twin: a 15-byte inline capacity plus a
//   single-byte terminator, and a replace-every-occurrence loop.  The
//   decompiled row stops after the first 118 bytes; the loop was recovered
//   from the shipped machine code (the function ends with `ret 0x10`).
// * `0067ecb0`, `0067ed90` and `0067ec70` operate on the client's ordered
//   `std::map`-style tree keyed by a `u16`; each value is a 12-byte
//   `vector<u32>`.
// * `0067ebd0` and `0067ee60` release and clear the client's global chained
//   hash map.
// * `0067f450` is the terrain walk around the camera.  Its decompiled body is
//   cut at a mis-identified non-returning getter, but the clamp, the block
//   granularity and the half-open tile window are complete in the machine
//   code and are reproduced here as three helpers.  The subsequent map and
//   unit walks are not part of this batch.
// * `0067ff80` is the tile-buffer setup that walks the map in thirty-two
//   tile blocks.  Its header reads, cursor seed, row advance, cell predicate
//   and visit order are recovered; the sprite stamp it calls for every
//   visible cell is outside this window and is not modelled.
//
// ## Recovered layouts
//
// ### UTF-32 string (`0x4c` bytes)
//
// | offset | field |
// |--------|-------|
// | `+0x00` | data pointer |
// | `+0x04` | length in code points |
// | `+0x08` | capacity word: low 31 bits hold the capacity, bit 31 is set for the inline buffer |
// | `+0x0c` | inline buffer: fifteen code points plus a four-byte terminator |
//
// ### Byte string (`0x1c` bytes)
//
// | offset | field |
// |--------|-------|
// | `+0x00` | data pointer |
// | `+0x04` | length in bytes |
// | `+0x08` | capacity word, same bit-31 convention |
// | `+0x0c` | inline buffer: fifteen bytes plus a one-byte terminator |
//
// ### Ordered map
//
// | object | offset | field |
// |--------|--------|-------|
// | map | `+0x00` | tree root |
// | map | `+0x10` | entry count |
// | node | `+0x00` | parent |
// | node | `+0x04` | left child |
// | node | `+0x08` | right child |
// | node | `+0x0c` | colour byte |
// | node | `+0x10` | `u16` key |
// | node | `+0x14` | twelve-byte value (`vector<u32>`) |
// | node | size | `0x20` bytes |
//
// The value vector uses the same three-word header as the byte string:
// data, length, capacity word.
//
// ### Global chained hash map
//
// | offset | field |
// |--------|-------|
// | `+0x00` | bucket count |
// | `+0x04` | bucket-array pointer, null until the first insert |
// | `+0x08` | live entry count |
// | `+0x0c` | load limit, `1.0f` |
//
// A bucket is the head of a singly linked chain; a node keeps its next
// pointer at `+0x00` and a pointer to a thirty-six-byte payload object at
// `+0x08`.
//
// ## Rows not translated
//
// | row | reason |
// |-----|--------|
// | `67e650` | Ghidra's row is only the first twelve bytes of the terrain-column copy; it ends at a getter that the decompiler marked non-returning, and the remainder of the routine is not part of the candidate body. |
// | `67e7c9` | inline row-stride fragment inside that copy loop (`in_EAX`, `in_EDX` and a stack slot); no recoverable arguments. |
// | `67e9d4` | overlapping-instruction fragment that writes through `unaff_EBX` and consumes `in_ECX`/`in_EAX`; no recoverable signature. |
// | `67ea45` | mid-function tile-slot store over `unaff_EBX`/`unaff_ESI`; no recoverable arguments. |
// | `67eb10` | Ghidra's seventeen-byte row ends at an allocator call; the real routine continues past the declared boundary and the candidate body has no behaviour. |
// | `67fc30` | decompile_failed. |
//
// The two partial rows (`67f450` and `67ff80`) are listed with the translated
// helpers below rather than in this table; only their fully recovered
// arithmetic is reproduced.
//
// Everything below is a free function over caller-owned data.  No `World`,
// `SimUnit`, renderer or client state is touched, and no save/replay state is
// added.

// ---------------------------------------------------------------------------
// Recovered constants
// ---------------------------------------------------------------------------

/// Capacity word of an empty string: fifteen inline elements plus the inline
/// tag bit.
pub const W14A08_EMPTY_CAPACITY_WORD: u32 = 0x8000_000f;

/// Mask that keeps the capacity bits and drops the inline/heap tag.
pub const W14A08_CAPACITY_MASK: u32 = 0x7fff_ffff;

/// Number of elements stored inline by both string forms.
pub const W14A08_INLINE_CAPACITY: u32 = 15;

/// Size of one UTF-32 string object.
pub const W14A08_U32_STRING_BYTES: u32 = 0x4c;

/// Offset of the UTF-32 inline buffer.
pub const W14A08_U32_INLINE_BUFFER_OFFSET: u32 = 0x0c;

/// Size of one byte string object.
pub const W14A08_BYTE_STRING_BYTES: u32 = 0x1c;

/// Offset of the byte-string inline buffer.
pub const W14A08_BYTE_INLINE_BUFFER_OFFSET: u32 = 0x0c;

/// Size of one ordered-map node.
pub const W14A08_ORDERED_NODE_BYTES: u32 = 0x20;

/// Offset of the `u16` key inside an ordered-map node.
pub const W14A08_ORDERED_NODE_KEY_OFFSET: u32 = 0x10;

/// Offset of the value vector inside an ordered-map node.
pub const W14A08_ORDERED_NODE_VALUE_OFFSET: u32 = 0x14;

/// Offset of the tree root in an ordered map.
pub const W14A08_ORDERED_ROOT_OFFSET: u32 = 0x00;

/// Offset of the entry count in an ordered map.
pub const W14A08_ORDERED_SIZE_OFFSET: u32 = 0x10;

/// Offset of the bucket count in the global hash map.
pub const W14A08_HASH_BUCKET_COUNT_OFFSET: u32 = 0x00;

/// Offset of the bucket-array pointer in the global hash map.
pub const W14A08_HASH_BUCKET_ARRAY_OFFSET: u32 = 0x04;

/// Offset of the live entry count in the global hash map.
pub const W14A08_HASH_SIZE_OFFSET: u32 = 0x08;

/// Offset of the load limit in the global hash map.
pub const W14A08_HASH_LOAD_OFFSET: u32 = 0x0c;

/// Load limit stored by the hash-map constructor.
pub const W14A08_HASH_LOAD_LIMIT: f32 = 1.0;

/// Offset of the chain link in a hash-map node.
pub const W14A08_HASH_NODE_NEXT_OFFSET: u32 = 0x00;

/// Offset of the payload pointer in a hash-map node.
pub const W14A08_HASH_NODE_PAYLOAD_OFFSET: u32 = 0x08;

// ---------------------------------------------------------------------------
// UTF-32 string: assign and grow (`0067e360`)
// ---------------------------------------------------------------------------

/// `0x0067e360`: the capacity the shipped string assign selects when the
/// current capacity word cannot hold `needed` code points.
///
/// The recovered sequence is:
///
/// ```text
/// current = capacity_word & 0x7fffffff
/// if needed <= current: no reallocation
/// grown = ((capacity_word >> 1) & 0x3fffffff) + current
/// if grown < needed: grown = needed
/// new capacity word = ((capacity_word ^ grown) & 0x7fffffff) ^ capacity_word
/// ```
///
/// The last expression reduces to `grown` once the tag bit is masked off, so
/// the helper returns `None` when no reallocation is needed and
/// `Some(grown)` otherwise.  For the empty header `0x8000000f` the first
/// growth target is `7 + 15 = 22`.
pub fn w14a08_u32_growth_capacity(capacity_word: u32, needed: u32) -> Option<u32> {
    let current = capacity_word & W14A08_CAPACITY_MASK;
    if needed <= current {
        return None;
    }
    let mut grown = ((capacity_word >> 1) & 0x3fff_ffff) + current;
    if grown < needed {
        grown = needed;
    }
    Some(grown)
}

/// `0x0067e360`: replace `destination` with `source`.
///
/// The shipped member function first resets its header to the empty inline
/// form (data at `this + 0x0c`, length zero, capacity word `0x8000000f`),
/// grows the buffer with [`w14a08_u32_growth_capacity`] when the source needs
/// more than fifteen code points, copies the code points, writes a four-byte
/// terminator after the last one and stores the length.  The Rust model uses
/// a `Vec<u32>` for the same payload; the terminator is implicit.
pub fn w14a08_u32_assign(destination: &mut Vec<u32>, source: &[u32]) {
    destination.clear();
    let needed = u32::try_from(source.len()).unwrap_or(u32::MAX);
    if let Some(capacity) = w14a08_u32_growth_capacity(W14A08_EMPTY_CAPACITY_WORD, needed) {
        destination.reserve(capacity as usize);
    }
    destination.extend_from_slice(source);
}

// ---------------------------------------------------------------------------
// UTF-32 string: find (`0067e440`)
// ---------------------------------------------------------------------------

/// `0x0067e440`: the first index at or after `position` whose code point is
/// `wanted`, or `None`.
///
/// The shipped loop rejects `position > length` with `0xffffffff`, requires
/// `position + 1 <= length` for the first comparison, and then compares one
/// 32-bit code point per step.  `None` is therefore returned both for an
/// out-of-range position and for `position == length`.
pub fn w14a08_u32_find(haystack: &[u32], wanted: u32, position: u32) -> Option<u32> {
    let start = position as usize;
    if start >= haystack.len() {
        return None;
    }
    haystack[start..]
        .iter()
        .position(|&code_point| code_point == wanted)
        .map(|offset| (start + offset) as u32)
}

// ---------------------------------------------------------------------------
// UTF-32 string: trim right (`0067e150`) and its wrapper (`0067e030`)
// ---------------------------------------------------------------------------

/// `0x0067e150`: assign the longest prefix of `source` that does not end in a
/// code point contained in `set` to `destination`.
///
/// The shipped loop walks `source` from its last code point towards the front
/// and stops at the first code point for which `set.find(code_point, 0)`
/// returns `-1`; when every code point is a member of `set` the result is
/// empty.  The destination is written through `0067e360`, so the store
/// always replaces the previous contents.
pub fn w14a08_u32_trim_end_into(destination: &mut Vec<u32>, source: &[u32], set: &[u32]) {
    let mut end = source.len();
    while end > 0 {
        if w14a08_u32_find(set, source[end - 1], 0).is_none() {
            break;
        }
        end -= 1;
    }
    w14a08_u32_assign(destination, &source[..end]);
}

/// `0x0067e030`: widen both 8-bit arguments and return the right-trimmed
/// UTF-32 string.
///
/// The shipped row is the by-value wrapper: it constructs two temporary
/// UTF-32 strings from the 8-bit arguments through the process-locale
/// conversion helper, calls the in-place trim, and then copy-assigns the
/// temporary into the hidden result pointer.  The copies make the wrapper
/// safe when the result aliases an argument.  Here the two inputs are taken
/// as octets and widened one-to-one; that is the conversion the process
/// locale performs for the ASCII trim sets used by the callers, while the
/// trim itself is exact.
pub fn w14a08_u32_trim_end_bytes(source: &[u8], set: &[u8]) -> Vec<u32> {
    let widened_source: Vec<u32> = source.iter().map(|&byte| u32::from(byte)).collect();
    let widened_set: Vec<u32> = set.iter().map(|&byte| u32::from(byte)).collect();
    let mut result = Vec::new();
    w14a08_u32_trim_end_into(&mut result, &widened_source, &widened_set);
    result
}

// ---------------------------------------------------------------------------
// UTF-32 string: trim left (`0067e2e0`) and its wrapper (`0067e1c0`)
// ---------------------------------------------------------------------------

/// `0x0067e2e0`: assign the longest suffix of `source` that does not start
/// with a code point contained in `set` to `destination`.
///
/// This is the mirror image of the right trim: the loop advances from the
/// first code point while the code point is a member of `set`, then assigns
/// `source[start..]`.  When every code point is a member of `set` the result
/// is empty.
pub fn w14a08_u32_trim_start_into(destination: &mut Vec<u32>, source: &[u32], set: &[u32]) {
    let mut start = 0;
    while start < source.len() {
        if w14a08_u32_find(set, source[start], 0).is_none() {
            break;
        }
        start += 1;
    }
    w14a08_u32_assign(destination, &source[start..]);
}

/// `0x0067e1c0`: widen both 8-bit arguments and return the left-trimmed
/// UTF-32 string.
///
/// Same wrapper shape as [`w14a08_u32_trim_end_bytes`]: two temporary UTF-32
/// strings are built through the process-locale conversion helper, the
/// in-place left trim runs, and the result is copied into the hidden result
/// pointer.  The ASCII widening used here matches the recovery model of the
/// right-trim wrapper.
pub fn w14a08_u32_trim_start_bytes(source: &[u8], set: &[u8]) -> Vec<u32> {
    let widened_source: Vec<u32> = source.iter().map(|&byte| u32::from(byte)).collect();
    let widened_set: Vec<u32> = set.iter().map(|&byte| u32::from(byte)).collect();
    let mut result = Vec::new();
    w14a08_u32_trim_start_into(&mut result, &widened_source, &widened_set);
    result
}

// ---------------------------------------------------------------------------
// Byte string: replace every occurrence (`0067e4a0`)
// ---------------------------------------------------------------------------

/// The byte-search primitive used by the replace loop.
///
/// The shipped routine delegates to a byte-search helper outside this window
/// with the same contract: `position > length` misses, a zero-length needle
/// matches at `position`, and `position + needle.len() > length` misses.
fn w14a08_byte_find(haystack: &[u8], needle: &[u8], position: u32) -> Option<usize> {
    let start = position as usize;
    if start > haystack.len() {
        return None;
    }
    if needle.is_empty() {
        return Some(start);
    }
    if needle.len() > haystack.len() - start {
        return None;
    }
    haystack[start..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| start + offset)
}

/// `0x0067e4a0`: replace every occurrence of `needle` with `replacement` in
/// the byte string `haystack`, left to right, and return the substitution
/// count.
///
/// The recovered loop is:
///
/// ```text
/// pos = find(haystack, needle, 0)
/// while pos fits:
///     removed = min(needle.len(), length - pos)
///     reserve(length - removed + replacement.len())
///     move the tail behind the match by replacement.len() - removed bytes
///     copy replacement to pos
///     length = length - removed + replacement.len()
///     pos = pos + replacement.len()
///     pos = find(haystack, needle, pos)
/// ```
///
/// A match is therefore never re-scanned as part of the inserted text.  The
/// shipped function returns the string itself; the count is returned only so
/// the tests can observe the loop.  The only guard added here is the
/// empty-needle case: without it the recovered loop advances `pos` and the
/// length by the same amount for ever, so the helper leaves the string
/// unchanged instead (the shipped callers pass the three fixed two- or
/// three-byte literals).
pub fn w14a08_byte_replace_all(haystack: &mut Vec<u8>, needle: &[u8], replacement: &[u8]) -> usize {
    if needle.is_empty() {
        return 0;
    }
    let mut position = match w14a08_byte_find(haystack, needle, 0) {
        Some(found) => found,
        None => return 0,
    };
    let mut substitutions = 0usize;
    loop {
        let removed = needle.len().min(haystack.len() - position);
        haystack.drain(position..position + removed);
        haystack.splice(position..position, replacement.iter().copied());
        substitutions += 1;
        position += replacement.len();
        match w14a08_byte_find(haystack, needle, position as u32) {
            Some(found) => position = found,
            None => break,
        }
    }
    substitutions
}

// ---------------------------------------------------------------------------
// Ordered map (`0067ecb0`, `0067ed90`, `0067ec70`)
// ---------------------------------------------------------------------------

/// The recovered `std::map<u16, std::vector<u32>>` value type.
///
/// A `BTreeMap` models the ordered tree: keys are unique and visited in key
/// order, and each value is the twelve-byte vector recovered from node
/// `+0x14`.
pub type W14a08OrderedMap = std::collections::BTreeMap<u16, Vec<u32>>;

/// `0x0067ecb0`: return the value vector for `key`, inserting an empty entry
/// when the key is absent.
///
/// The shipped member is the tree's `operator[]`: it descends the red-black
/// tree through the lookup helper `FUN_00680cd0`, allocates a `0x20`-byte
/// node on a miss (parent, left, right, colour, the `u16` key at `+0x10` and
/// a zeroed twelve-byte value vector at `+0x14`), links and rebalances it,
/// bumps the entry count at `+0x10` of the map, and returns `node + 0x14`.
/// The `BTreeMap` entry API performs the same lookup-or-insert.
pub fn w14a08_ordered_map_entry(map: &mut W14a08OrderedMap, key: u16) -> &mut Vec<u32> {
    map.entry(key).or_default()
}

/// `0x0067ed90`: assign a fresh value vector to `map[key]` and return it.
///
/// The shipped function looks the entry up, releases the value vector it
/// already held (a heap buffer is freed and the three-word header is zeroed),
/// re-obtains the entry and calls the vector resize helper `FUN_00682410`
/// with `count`.  That resize fills every new slot with `0xffffffff`, so the
/// observable result is `count` copies of `u32::MAX`.  The temporary empty
/// vector and the intermediate clear are folded into the final assignment
/// here.
pub fn w14a08_ordered_map_assign_minus_one(
    map: &mut W14a08OrderedMap,
    key: u16,
    count: u32,
) -> &mut Vec<u32> {
    let value = w14a08_ordered_map_entry(map, key);
    value.clear();
    value.resize(count as usize, u32::MAX);
    value
}

/// `0x0067ec70`: release the ordered tree rooted at the map's first word.
///
/// The body is a single call to the recursive node destructor
/// `FUN_00680c30` with `*(this + 0x00)`; that destructor walks the node link
/// at `+0x04`, destroys the twelve-byte vector value and releases the node.
/// The `BTreeMap` model reproduces the resulting empty map and reports how
/// many entries were released.
pub fn w14a08_ordered_map_release(map: &mut W14a08OrderedMap) -> usize {
    let released = map.len();
    map.clear();
    released
}

// ---------------------------------------------------------------------------
// Global chained hash map (`0067ebd0`, `0067ee60`)
// ---------------------------------------------------------------------------

/// The recovered shape of the client's global chained hash map.
///
/// The bucket array is optional because the shipped routines distinguish a
/// null array pointer from an allocated one.  When present, the vector holds
/// `bucket_count` buckets, and each bucket holds the chain of node
/// identifiers from head to tail.  Node identity and payload destruction are
/// abstracted: releasing a chain drops its identifiers, exactly as the
/// shipped loop destroys the payload object at node `+0x08` and frees the
/// node.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W14a08HashTable {
    /// `+0x00`: bucket count.
    pub bucket_count: u32,
    /// `+0x04`: bucket array, `None` while the pointer is null.
    pub buckets: Option<Vec<Vec<u32>>>,
    /// `+0x08`: live entry count.
    pub size: u32,
}

/// Total number of chained nodes in a bucket array.
fn w14a08_hash_chain_nodes(buckets: &[Vec<u32>]) -> usize {
    buckets.iter().map(Vec::len).sum()
}

/// `0x0067ebd0`: destroy the hash table.
///
/// The shipped body returns immediately when the bucket-array pointer is
/// null.  Otherwise it walks every bucket `0..bucket_count`, and for each
/// node in the chain it saves the `+0x00` link, destroys the payload pointer
/// stored at `+0x08` through `FUN_0065bf90`, and frees the node.  It then
/// frees the bucket array itself and writes null to `+0x04`.  The bucket
/// count at `+0x00` and the entry count at `+0x08` are left untouched.
///
/// The model releases the chain identifiers and sets `buckets` to `None`;
/// the returned value is the number of nodes released.
pub fn w14a08_hash_table_destroy(table: &mut W14a08HashTable) -> usize {
    let released = table
        .buckets
        .as_deref()
        .map(w14a08_hash_chain_nodes)
        .unwrap_or(0);
    table.buckets = None;
    released
}

/// `0x0067ee60`: clear the hash table while keeping its bucket array.
///
/// The shipped body obtains the global table pointer, returns early when the
/// bucket array is null (after storing zero in the size field), and otherwise
/// walks each bucket, releases the chain exactly like the destructor, writes
/// zero into that bucket head, and finally stores zero in the size field.
///
/// The model empties every bucket, keeps the array allocated, and resets
/// `size` to zero.  The returned value is the number of nodes released.
pub fn w14a08_hash_table_clear(table: &mut W14a08HashTable) -> usize {
    let released = match table.buckets.as_mut() {
        Some(buckets) => {
            let released = w14a08_hash_chain_nodes(buckets);
            for chain in buckets.iter_mut() {
                chain.clear();
            }
            released
        }
        None => 0,
    };
    table.size = 0;
    released
}

// ---------------------------------------------------------------------------
// Camera tile window (`0067f450`)
// ---------------------------------------------------------------------------

/// `0x0067f450`: the screen extent used by the camera tile walk.
///
/// The machine code converts the floating-point screen dimension with
/// `cvttss2si`, subtracts one and clamps the result to zero:
///
/// ```text
/// value = cvttss2si(pixels) - 1
/// if value <= 0: value = 0
/// ```
///
/// `cvttss2si` returns `0x80000000` for NaN and out-of-range inputs; the
/// helper reproduces that conversion before the wrapping decrement.  The
/// callers pass a finite screen dimension.
pub fn w14a08_screen_tile_extent(pixels: f32) -> i32 {
    let converted = if !pixels.is_finite() || pixels >= 2_147_483_648.0 || pixels < -2_147_483_648.0
    {
        i32::MIN
    } else {
        pixels.trunc() as i32
    };
    let extent = converted.wrapping_sub(1);
    if extent <= 0 { 0 } else { extent }
}

/// `0x0067f450`: the recovered tile-block count for one map dimension.
///
/// The shipped expression is the compiler's signed-division bias around a
/// thirty-two tile block, followed by one extra block:
///
/// ```text
/// biased = dimension + 0x1f
/// count  = ((biased + ((biased >> 31) & 0x1f)) >> 5) + 1
/// ```
///
/// For a non-negative dimension this is `ceil(dimension / 32) + 1`.
pub fn w14a08_tile_block_span(dimension: i32) -> i32 {
    let biased = dimension.wrapping_add(0x1f);
    (biased.wrapping_add((biased >> 31) & 0x1f) >> 5).wrapping_add(1)
}

/// The half-open tile rectangle recovered from the camera walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14a08TileWindow {
    /// `[ebp-0x18]`: first column, `-extent_x`.
    pub x_start: i32,
    /// `[ebp-0x14]`: one past the last column.
    pub x_end: i32,
    /// `[ebp-0x10]`: first row, `-extent_y`.
    pub y_start: i32,
    /// `[ebp-0x20]`: one past the last row.
    pub y_end: i32,
}

/// `0x0067f450`: the half-open tile window the camera walk iterates.
///
/// The recovered setup is:
///
/// ```text
/// w = clamp_extent(screen_width)      // 0 .. screen_width - 1
/// h = clamp_extent(screen_height)
/// x_start = -w
/// x_end   = block_span(map_width)  + w
/// y_start = -h
/// y_end   = block_span(map_height) + h
/// ```
///
/// The rows are visited for `y_start .. y_end` and the columns for
/// `x_start .. x_end`; the inner walk and the tile stamp are not modelled.
pub fn w14a08_camera_tile_window(
    screen_width: f32,
    screen_height: f32,
    map_width: i32,
    map_height: i32,
) -> W14a08TileWindow {
    let extent_x = w14a08_screen_tile_extent(screen_width);
    let extent_y = w14a08_screen_tile_extent(screen_height);
    W14a08TileWindow {
        x_start: extent_x.wrapping_neg(),
        x_end: w14a08_tile_block_span(map_width).wrapping_add(extent_x),
        y_start: extent_y.wrapping_neg(),
        y_end: w14a08_tile_block_span(map_height).wrapping_add(extent_y),
    }
}

// ---------------------------------------------------------------------------
// Tile-buffer walk (`0067ff80`)
// ---------------------------------------------------------------------------

/// `0x0067ff80`: the map-header fields consumed by the tile walk.
///
/// The routine obtains the header through an obfuscated map-pointer global
/// and reads four unsigned words.  The column loop compares
/// `column_offset + column < row_width`; the row loop compares
/// `row_offset + row < row_limit`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a08TileMapHeader {
    /// `+0xe0`: column offset added to the visited column.
    pub column_offset: u16,
    /// `+0xe2`: row offset added to the visited row.
    pub row_offset: u16,
    /// `+0xe4`: row width in tiles, also the exclusive column limit.
    pub row_width: u16,
    /// `+0xe6`: exclusive row limit.
    pub row_limit: u16,
}

/// `0x0067ff80`: the first destination cursor of the tile walk.
///
/// The recovered seed is
/// `array_base + (row_width * row_offset + column_offset) * 2`; the factor of
/// two is the size of one tile word.
pub fn w14a08_tile_cursor_seed(array_base: u32, header: &W14a08TileMapHeader) -> u32 {
    let tile_offset = u32::from(header.row_width)
        .wrapping_mul(u32::from(header.row_offset))
        .wrapping_add(u32::from(header.column_offset));
    array_base.wrapping_add(tile_offset.wrapping_mul(2))
}

/// `0x0067ff80`: the destination cursor advance between two visited rows.
///
/// The row cursor is reset from the previous row seed and advanced by
/// `row_width * 2` bytes after every row.
pub fn w14a08_tile_row_advance(header: &W14a08TileMapHeader) -> u32 {
    u32::from(header.row_width).wrapping_mul(2)
}

/// `0x0067ff80`: whether the tile walk visits `(row, column)`.
///
/// Both comparisons are the unsigned ones from the shipped loop; the walk
/// index starts at zero and is offset by the header's row and column bases.
pub fn w14a08_tile_cell_visible(header: &W14a08TileMapHeader, row: u32, column: u32) -> bool {
    u32::from(header.row_offset).wrapping_add(row) < u32::from(header.row_limit)
        && u32::from(header.column_offset).wrapping_add(column) < u32::from(header.row_width)
}

/// `0x0067ff80`: the visited `(row, column)` pairs, in shipped order.
///
/// The row count comes from the map height with [`w14a08_tile_block_span`]
/// and the column count from the map width; the shipped loop re-reads those
/// globals for every iteration, which is equivalent to the parameter values
/// used here.  A failing row predicate skips that row's column loop and
/// continues with the next row; a failing column predicate leaves the column
/// loop and continues with the next row.  A non-positive count skips the
/// corresponding loop.
///
/// Each visited cell is handed to the sprite stamp with the cell cursor, the
/// row and the column; that callee is outside this window and is not
/// modelled.
pub fn w14a08_tile_walk(
    header: &W14a08TileMapHeader,
    map_width: i32,
    map_height: i32,
) -> Vec<(u32, u32)> {
    let mut cells = Vec::new();
    let rows = w14a08_tile_block_span(map_height);
    if rows <= 0 {
        return cells;
    }
    for row in 0..rows as u32 {
        let columns = w14a08_tile_block_span(map_width);
        if columns <= 0 {
            continue;
        }
        for column in 0..columns as u32 {
            if !w14a08_tile_cell_visible(header, row, column) {
                break;
            }
            cells.push((row, column));
        }
    }
    cells
}

/// `0x0067ff80`: the byte address of a visited tile word.
///
/// The row seed advances by [`w14a08_tile_row_advance`] and the column cursor
/// by two bytes, so the address is
/// `seed + row * row_width * 2 + column * 2`.  `row` and `column` are walk
/// indices, not the biased map coordinates.
pub fn w14a08_tile_cell_address(
    array_base: u32,
    header: &W14a08TileMapHeader,
    row: u32,
    column: u32,
) -> u32 {
    w14a08_tile_cursor_seed(array_base, header)
        .wrapping_add(
            row.wrapping_mul(u32::from(header.row_width))
                .wrapping_mul(2),
        )
        .wrapping_add(column.wrapping_mul(2))
}

#[cfg(test)]
mod tests_wave14_agent08 {
    use super::*;

    // -- UTF-32 assign and growth -------------------------------------------

    #[test]
    fn growth_uses_the_recovered_double_plus_fifteen_rule() {
        assert_eq!(
            w14a08_u32_growth_capacity(W14A08_EMPTY_CAPACITY_WORD, 0),
            None
        );
        assert_eq!(
            w14a08_u32_growth_capacity(W14A08_EMPTY_CAPACITY_WORD, 15),
            None
        );
        assert_eq!(
            w14a08_u32_growth_capacity(W14A08_EMPTY_CAPACITY_WORD, 16),
            Some(22)
        );
        assert_eq!(
            w14a08_u32_growth_capacity(W14A08_EMPTY_CAPACITY_WORD, 22),
            Some(22)
        );
        assert_eq!(
            w14a08_u32_growth_capacity(W14A08_EMPTY_CAPACITY_WORD, 23),
            Some(23)
        );
        // A heap header without the tag bit grows by half of its capacity.
        assert_eq!(w14a08_u32_growth_capacity(100, 100), None);
        assert_eq!(w14a08_u32_growth_capacity(100, 101), Some(150));
        assert_eq!(w14a08_u32_growth_capacity(100, 200), Some(200));
    }

    #[test]
    fn assign_replaces_every_element_and_grows_inline() {
        let source: Vec<u32> = (1..=20).collect();
        let mut destination = vec![0xdead_beef; 3];
        w14a08_u32_assign(&mut destination, &source);
        assert_eq!(destination, source);
        assert!(
            destination.capacity() >= 22,
            "growth target must be reserved"
        );

        w14a08_u32_assign(&mut destination, &[]);
        assert!(destination.is_empty());
    }

    #[test]
    fn assign_keeps_the_fifteen_inline_elements_without_reallocation() {
        for length in 0..=15usize {
            let source: Vec<u32> = (0..length as u32).collect();
            let mut destination = Vec::new();
            w14a08_u32_assign(&mut destination, &source);
            assert_eq!(destination, source, "length {length}");
        }
    }

    // -- UTF-32 find ---------------------------------------------------------

    #[test]
    fn find_honours_position_and_returns_none_at_or_past_the_end() {
        let haystack = [10, 20, 30, 20, 0x1f600];
        assert_eq!(w14a08_u32_find(&haystack, 20, 0), Some(1));
        assert_eq!(w14a08_u32_find(&haystack, 20, 1), Some(1));
        assert_eq!(w14a08_u32_find(&haystack, 20, 2), Some(3));
        assert_eq!(w14a08_u32_find(&haystack, 20, 3), Some(3));
        assert_eq!(w14a08_u32_find(&haystack, 20, 4), None);
        assert_eq!(w14a08_u32_find(&haystack, 20, 5), None);
        assert_eq!(w14a08_u32_find(&haystack, 20, 999), None);
        assert_eq!(w14a08_u32_find(&haystack, 99, 0), None);
        assert_eq!(w14a08_u32_find(&haystack, 0x1f600, 0), Some(4));
        assert_eq!(w14a08_u32_find(&[], 1, 0), None);
    }

    // -- UTF-32 trim right ---------------------------------------------------

    #[test]
    fn trim_end_removes_only_the_trailing_members_of_the_set() {
        let source = ['a' as u32, 'b' as u32, 'x' as u32, 'x' as u32];
        let set = ['x' as u32];
        let mut destination = vec![0xffff_ffff];
        w14a08_u32_trim_end_into(&mut destination, &source, &set);
        assert_eq!(destination, vec!['a' as u32, 'b' as u32]);

        // Every code point is in the set: the result is empty.
        let all = ['x' as u32, 'x' as u32];
        w14a08_u32_trim_end_into(&mut destination, &all, &set);
        assert!(destination.is_empty());

        // An empty set and an empty source both leave the source prefix.
        w14a08_u32_trim_end_into(&mut destination, &source, &[]);
        assert_eq!(destination, source);
        w14a08_u32_trim_end_into(&mut destination, &[], &set);
        assert!(destination.is_empty());

        // A mismatching code point at the front keeps the whole string.
        let no_trailing = ['a' as u32, 'x' as u32, 'b' as u32];
        w14a08_u32_trim_end_into(&mut destination, &no_trailing, &set);
        assert_eq!(destination, no_trailing);
    }

    #[test]
    fn trim_end_bytes_widens_octets_and_trims_the_right_edge() {
        let result = w14a08_u32_trim_end_bytes(b"value \t\r\n", b" \t\r\n");
        assert_eq!(result, "value".chars().map(u32::from).collect::<Vec<_>>());
        assert!(w14a08_u32_trim_end_bytes(b"", b" ").is_empty());
        assert_eq!(
            w14a08_u32_trim_end_bytes(b"  ab  ", b" "),
            "  ab".chars().map(u32::from).collect::<Vec<_>>()
        );
        assert_eq!(
            w14a08_u32_trim_end_bytes(b"ab", b""),
            "ab".chars().map(u32::from).collect::<Vec<_>>()
        );
    }

    // -- UTF-32 trim left ----------------------------------------------------

    #[test]
    fn trim_start_removes_only_the_leading_members_of_the_set() {
        let source = ['x' as u32, 'x' as u32, 'a' as u32, 'b' as u32];
        let set = ['x' as u32];
        let mut destination = vec![0xdead_beef];
        w14a08_u32_trim_start_into(&mut destination, &source, &set);
        assert_eq!(destination, vec!['a' as u32, 'b' as u32]);

        let all = ['x' as u32, 'x' as u32];
        w14a08_u32_trim_start_into(&mut destination, &all, &set);
        assert!(destination.is_empty());

        w14a08_u32_trim_start_into(&mut destination, &source, &[]);
        assert_eq!(destination, source);
        w14a08_u32_trim_start_into(&mut destination, &[], &set);
        assert!(destination.is_empty());
    }

    #[test]
    fn trim_start_and_trim_end_are_mirror_images() {
        let mixed = ['x' as u32, 'a' as u32, 'y' as u32, 'x' as u32];
        let set = ['x' as u32];
        assert_eq!(
            w14a08_u32_trim_start_bytes(b"xayx", b"x"),
            vec!['a' as u32, 'y' as u32, 'x' as u32]
        );
        let mut left = Vec::new();
        let mut right = Vec::new();
        w14a08_u32_trim_start_into(&mut left, &mixed, &set);
        w14a08_u32_trim_end_into(&mut right, &mixed, &set);
        assert_eq!(left, vec!['a' as u32, 'y' as u32, 'x' as u32]);
        assert_eq!(right, vec!['x' as u32, 'a' as u32, 'y' as u32]);
    }

    #[test]
    fn both_trim_wrappers_match_their_in_place_helpers() {
        let source: Vec<u32> = "xy middle yx".chars().map(u32::from).collect();
        let set: Vec<u32> = "xy".chars().map(u32::from).collect();
        let mut expected = Vec::new();
        w14a08_u32_trim_end_into(&mut expected, &source, &set);
        assert_eq!(w14a08_u32_trim_end_bytes(b"xy middle yx", b"xy"), expected);

        let mut expected = Vec::new();
        w14a08_u32_trim_start_into(&mut expected, &source, &set);
        assert_eq!(
            w14a08_u32_trim_start_bytes(b"xy middle yx", b"xy"),
            expected
        );
    }

    // -- Byte-string replace-all --------------------------------------------

    #[test]
    fn replace_all_substitutes_every_occurrence_left_to_right() {
        let mut text = Vec::from(&b"a\\ub"[..]);
        assert_eq!(w14a08_byte_replace_all(&mut text, b"\\u", b"X"), 1);
        assert_eq!(text, b"aXb");

        let mut text = Vec::from(&b"\\u\\u"[..]);
        assert_eq!(w14a08_byte_replace_all(&mut text, b"\\u", b"Q"), 2);
        assert_eq!(text, b"QQ");

        let mut text = Vec::from(&b"\\n\\t\\u"[..]);
        assert_eq!(w14a08_byte_replace_all(&mut text, b"\\n", b"\\\\n"), 1);
        assert_eq!(w14a08_byte_replace_all(&mut text, b"\\t", b"\\\\t"), 1);
        assert_eq!(text, b"\\\\n\\\\t\\u");
    }

    #[test]
    fn replace_all_leaves_non_matching_text_alone() {
        let mut text = Vec::from(&b"plain text"[..]);
        assert_eq!(w14a08_byte_replace_all(&mut text, b"\\u", b"x"), 0);
        assert_eq!(text, b"plain text");

        let mut text = Vec::from(&b"abc"[..]);
        assert_eq!(w14a08_byte_replace_all(&mut text, b"", b"x"), 0);
        assert_eq!(text, b"abc");
    }

    #[test]
    fn replace_all_handles_the_truncated_tail_at_the_end() {
        // The last occurrence is shorter than the needle only when the match
        // starts inside the string; the shipped loop clamps the removal with
        // min(needle.len(), length - position).
        let mut text = Vec::from(&b"xx"[..]);
        assert_eq!(w14a08_byte_replace_all(&mut text, b"xx", b"y"), 1);
        assert_eq!(text, b"y");

        let mut text = Vec::from(&b"aaaa"[..]);
        assert_eq!(w14a08_byte_replace_all(&mut text, b"aa", b"b"), 2);
        assert_eq!(text, b"bb");
    }

    // -- Ordered map ---------------------------------------------------------

    #[test]
    fn ordered_map_entry_inserts_missing_keys_and_keeps_existing_values() {
        let mut map = W14a08OrderedMap::new();
        assert!(w14a08_ordered_map_entry(&mut map, 7).is_empty());
        assert_eq!(map.len(), 1);
        w14a08_ordered_map_entry(&mut map, 7).extend_from_slice(&[1, 2, 3]);
        assert_eq!(map.get(&7).map(Vec::as_slice), Some(&[1u32, 2, 3][..]));
        let entry = w14a08_ordered_map_entry(&mut map, 7);
        assert_eq!(entry.as_slice(), &[1u32, 2, 3][..]);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn ordered_map_assign_fills_with_minus_one() {
        let mut map = W14a08OrderedMap::new();
        let value = w14a08_ordered_map_assign_minus_one(&mut map, 2, 4);
        assert_eq!(value.len(), 4);
        assert!(value.iter().all(|&element| element == u32::MAX));
        assert_eq!(map.get(&2).map(Vec::as_slice), Some(&[u32::MAX; 4][..]));

        // The previous value is replaced, other keys are untouched.
        map.insert(6, vec![9]);
        let value = w14a08_ordered_map_assign_minus_one(&mut map, 2, 1);
        assert_eq!(value.as_slice(), &[u32::MAX][..]);
        assert_eq!(map.get(&2).map(Vec::as_slice), Some(&[u32::MAX][..]));
        assert_eq!(map.get(&6).map(Vec::as_slice), Some(&[9u32][..]));

        // A zero count assigns an empty value.
        assert!(w14a08_ordered_map_assign_minus_one(&mut map, 6, 0).is_empty());
        assert_eq!(map.get(&6).map(Vec::as_slice), Some(&[][..]));
    }

    #[test]
    fn ordered_map_release_empties_the_tree_and_counts_entries() {
        let mut map = W14a08OrderedMap::new();
        map.insert(1, vec![10]);
        map.insert(2, vec![20, 21]);
        assert_eq!(w14a08_ordered_map_release(&mut map), 2);
        assert!(map.is_empty());
        assert_eq!(w14a08_ordered_map_release(&mut map), 0);
    }

    // -- Hash-map cleanup ----------------------------------------------------

    fn sample_hash_table() -> W14a08HashTable {
        W14a08HashTable {
            bucket_count: 4,
            buckets: Some(vec![vec![0, 1], vec![2], Vec::new(), vec![3, 4, 5]]),
            size: 6,
        }
    }

    #[test]
    fn hash_destroy_releases_chains_and_the_bucket_array() {
        let mut table = sample_hash_table();
        assert_eq!(w14a08_hash_table_destroy(&mut table), 6);
        assert_eq!(table.buckets, None);
        // The shipped destructor leaves the count and size fields alone.
        assert_eq!(table.bucket_count, 4);
        assert_eq!(table.size, 6);

        // A null bucket array is a no-op.
        let mut empty = W14a08HashTable {
            bucket_count: 8,
            buckets: None,
            size: 0,
        };
        assert_eq!(w14a08_hash_table_destroy(&mut empty), 0);
        assert_eq!(empty.buckets, None);
    }

    #[test]
    fn hash_clear_keeps_the_array_and_resets_the_size() {
        let mut table = sample_hash_table();
        assert_eq!(w14a08_hash_table_clear(&mut table), 6);
        assert_eq!(
            table.buckets,
            Some(vec![Vec::new(), Vec::new(), Vec::new(), Vec::new()])
        );
        assert_eq!(table.bucket_count, 4);
        assert_eq!(table.size, 0);

        let mut empty = W14a08HashTable {
            bucket_count: 8,
            buckets: None,
            size: 7,
        };
        assert_eq!(w14a08_hash_table_clear(&mut empty), 0);
        assert_eq!(empty.buckets, None);
        assert_eq!(empty.size, 0);
    }

    // -- Camera tile window --------------------------------------------------

    #[test]
    fn screen_extent_truncates_decrements_and_clamps() {
        assert_eq!(w14a08_screen_tile_extent(0.0), 0);
        assert_eq!(w14a08_screen_tile_extent(1.0), 0);
        assert_eq!(w14a08_screen_tile_extent(1.9), 0);
        assert_eq!(w14a08_screen_tile_extent(2.0), 1);
        assert_eq!(w14a08_screen_tile_extent(32.9), 31);
        assert_eq!(w14a08_screen_tile_extent(-5.0), 0);
        // `cvttss2si` turns NaN into 0x80000000; the decrement wraps to
        // 0x7fffffff, which is positive and therefore survives the clamp.
        assert_eq!(w14a08_screen_tile_extent(f32::NAN), i32::MAX);
    }

    #[test]
    fn tile_block_span_is_ceil_over_thirty_two_plus_one() {
        assert_eq!(w14a08_tile_block_span(0), 1);
        assert_eq!(w14a08_tile_block_span(1), 2);
        assert_eq!(w14a08_tile_block_span(31), 2);
        assert_eq!(w14a08_tile_block_span(32), 2);
        assert_eq!(w14a08_tile_block_span(33), 3);
        assert_eq!(w14a08_tile_block_span(1024), 33);
    }

    #[test]
    fn camera_window_is_half_open_and_uses_the_block_counts() {
        let window = w14a08_camera_tile_window(640.0, 480.0, 32, 32);
        assert_eq!(window.x_start, -639);
        assert_eq!(window.x_end, 641);
        assert_eq!(window.y_start, -479);
        assert_eq!(window.y_end, 481);

        let single_block = w14a08_camera_tile_window(1.0, 1.0, 0, 0);
        assert_eq!(single_block.x_start, 0);
        assert_eq!(single_block.x_end, 1);
        assert_eq!(single_block.y_start, 0);
        assert_eq!(single_block.y_end, 1);
    }

    // -- Tile-buffer walk ----------------------------------------------------

    fn sample_tile_header() -> W14a08TileMapHeader {
        W14a08TileMapHeader {
            column_offset: 2,
            row_offset: 3,
            row_width: 10,
            row_limit: 8,
        }
    }

    #[test]
    fn tile_cursor_seed_row_advance_and_cell_address_agree() {
        let header = sample_tile_header();
        let seed = w14a08_tile_cursor_seed(0x1000, &header);
        assert_eq!(seed, 0x1000 + (10 * 3 + 2) * 2);
        assert_eq!(w14a08_tile_row_advance(&header), 20);
        assert_eq!(w14a08_tile_cell_address(0x1000, &header, 0, 0), seed);
        assert_eq!(w14a08_tile_cell_address(0x1000, &header, 1, 1), seed + 22);
        assert_eq!(
            w14a08_tile_cell_address(0x1000, &header, 2, 3),
            seed + 2 * 20 + 3 * 2
        );
    }

    #[test]
    fn tile_cells_respect_both_header_bounds() {
        let header = sample_tile_header();
        assert!(w14a08_tile_cell_visible(&header, 0, 0));
        // row 3 + 4 = 7 < 8 and column 2 + 7 = 9 < 10.
        assert!(w14a08_tile_cell_visible(&header, 4, 7));
        // row 3 + 5 = 8 is the exclusive limit.
        assert!(!w14a08_tile_cell_visible(&header, 5, 0));
        // column 2 + 8 = 10 is the exclusive limit.
        assert!(!w14a08_tile_cell_visible(&header, 0, 8));
    }

    #[test]
    fn tile_walk_follows_the_recovered_row_and_column_breaks() {
        let header = sample_tile_header();
        assert_eq!(
            w14a08_tile_walk(&header, 32, 32),
            vec![(0, 0), (0, 1), (1, 0), (1, 1)]
        );

        // The second row starts at the exclusive limit and is skipped, but
        // its row cursor still advances.
        let first_row_only = W14a08TileMapHeader {
            row_limit: 4,
            ..header
        };
        assert_eq!(
            w14a08_tile_walk(&first_row_only, 32, 32),
            vec![(0, 0), (0, 1)]
        );

        // The second column is at the exclusive width limit and ends the
        // column loop for every row.
        let narrow = W14a08TileMapHeader {
            column_offset: 9,
            ..header
        };
        assert_eq!(w14a08_tile_walk(&narrow, 32, 32), vec![(0, 0), (1, 0)]);

        // A negative map dimension produces a non-positive block count.
        assert!(w14a08_tile_walk(&header, 32, i32::MIN).is_empty());
    }
}
