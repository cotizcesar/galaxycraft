#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent90.rs
// ===========================================================================
// Translation batch 90.  Only this file is edited by the batch.
//
// Batch 90 owns `0x00646000..=0x00647fff`.  The window is C++ support code
// around the sound/localisation tables, the `std::map`/`std::unordered_map`
// helpers used by those tables, one text-line layout routine and the
// surrounding container plumbing.  The bodies below reproduce the exact
// recovered arithmetic, key strings and container-layout rules.  Where the
// shipped routine calls a manager that lives outside this window, the
// manager enters the translation as an explicit lookup input, never as a
// citation of an unimplemented helper.
//
// Recovered layouts
// -----------------
//
// * **String/vector header**: `+0x00` data, `+0x04` count, `+0x08` capacity
//   word.  Bit 31 set means inline storage; clear means heap storage.
// * **`std::map` header**: `+0x00` root link, `+0x04` size.  A node has
//   `+0x00` left, `+0x04` right, `+0x08` parent, `+0x10` the string key
//   (data then length) and `+0x2c` the mapped value.
// * **`std::unordered_map` header**: `+0x00` bucket count (power of two),
//   `+0x04` bucket array, `+0x08` element count, `+0x0c` maximum load
//   factor as an `f32`.  A bucket is a singly linked chain through a node's
//   `+0x00` next link; the `u16` key lives at `+0x04`.
// * **Draw record** (`0x40` bytes): `+0x00` word, `+0x04` `u16` frame id,
//   `+0x08..=+0x17` four bounds words, `+0x18..=+0x2f` six zero words,
//   `+0x30..=+0x3f` four colour words.
// * **Text state**: `+0x00` colour word, `+0x04` font word, `+0x08` x
//   origin, `+0x0c` y origin, `+0x10` width, `+0x14` height,
//   `+0x18..=+0x27` clip left/top/width/height, `+0x28` x offset,
//   `+0x2c` y offset and `+0x30..=+0x3f` four draw colour words.
//
// Address to function
// -------------------
//
// * `0x00646140` [`a90_sound_offset_00646140`]
// * `0x00646250` [`a90_research_complete_00646250`]
// * `0x00646380` [`a90_map_first_value_00646380`]
// * `0x006463f0` [`a90_collect_name_match_indices_006463f0`]
// * `0x006465c0` [`a90_two_level_map_lookup_006465c0`]
// * `0x006466a0` [`a90_start_of_game_006466a0`]
// * `0x006467e0` [`a90_sound_offset_006467e0`]
// * `0x00646b00` [`a90_upgrade_complete_00646b00`]
// * `0x00646c70` [`a90_tree_destroy_plan_00646c70`]
// * `0x00646d00` [`a90_insert_position_00646d00`]
// * `0x00646d60` [`a90_string_compare_00646d60`]
// * `0x00646df0` [`a90_reserve_capacity_00646df0`]
// * `0x00646fd0` [`a90_signature_tail_plan_00646fd0`]
// * `0x00647390` [`a90_init_draw_record_00647390`]
// * `0x006473f0` [`a90_draw_record_blit_006473f0`]
// * `0x006474a0` [`a90_text_block_006474a0`]
// * `0x00647531` [`a90_free_heap_strings_00647531`]
// * `0x00647557` [`a90_free_vector_buffer_00647557`]
// * `0x00647580` [`a90_text_runs_layout_00647580`]
// * `0x00647750` [`a90_reset_bounds_00647750`]
// * `0x00647770` [`a90_write_functor_vtable_00647770`]
// * `0x00647780` [`a90_lambda_type_descriptor_00647780`]
// * `0x00647932` [`a90_hash_insert_plan_00647932`]
// * `0x00647997` [`a90_hash_chain_lookup_00647997`]
// * `0x00647f50` [`a90_copy_variant_00647f50`]
//
// Rows not translated
// -------------------
//
// `0x00646140`-style small entries are translated even when they are
// secondary entry points.  The following rows are SEH epilogues, register
// fragments, indirect jump tables, or container bodies whose recovered
// stores contradict their own header (documented per row in the batch
// report): `FUN_006466a4`, `FUN_00646763`, `FUN_0064696d`, `FUN_00646be9`,
// `FUN_00647126`, `FUN_0064780f`, `FUN_006478f0`, `FUN_00647a24`,
// `FUN_00647ae3`, `FUN_00647b30`, `FUN_00647de0`, `FUN_00647eb0`.

// ---------------------------------------------------------------------------
// 0x00646d60 - std::string three-way comparison
// ---------------------------------------------------------------------------

/// `0x00646d60` (`FUN_00646d60`, 137 bytes): lexicographic comparison of two
/// `std::string` headers.
///
/// The shipped body compares `min(len_left, len_right)` bytes in four-byte
/// chunks and then byte-by-byte.  At the first differing byte it returns `-1`
/// when the left byte is smaller and `+1` when it is larger.  If all compared
/// bytes are equal it returns `left_len - right_len` as a wrapping `int`.
pub fn a90_string_compare_00646d60(left: &[u8], right: &[u8]) -> i32 {
    let shared = left.len().min(right.len());
    for index in 0..shared {
        if left[index] != right[index] {
            return if left[index] < right[index] { -1 } else { 1 };
        }
    }
    (left.len() as i32).wrapping_sub(right.len() as i32)
}

// ---------------------------------------------------------------------------
// Multimap helpers (FUN_00644430 / 0x00646d00 / 0x006465c0)
// ---------------------------------------------------------------------------

/// A sorted `std::map` node view used by the insertion-position translation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A90BstNode<'a> {
    /// Key stored at node `+0x10`.
    pub key: &'a str,
    /// Left child link (`+0x00`), or `None` for a null link.
    pub left: Option<usize>,
    /// Right child link (`+0x04`), or `None` for a null link.
    pub right: Option<usize>,
}

/// Which child link the insertion-position walk would take after the returned node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A90InsertLink {
    /// `key < node.key`; the returned iterator is the node's left link.
    Left,
    /// `key >= node.key`; the returned iterator is the node's right link.
    Right,
}

/// `0x00646d00` (`FUN_00646d00`, 89 bytes): `std::map` insertion-position
/// search, the routine used by `FUN_0064476c` to place a new node.
///
/// The walk starts at the header's root link, keeps the last visited node and
/// descends left while `compare(key, node.key) < 0`, otherwise right.  The
/// returned pair is `(last_node, last_node + (left ? 4 : 8))`; the translation
/// exposes the chosen link as [`A90InsertLink`].  An empty tree returns
/// `(None, Left)` because the shipped `al` starts at `1`.
pub fn a90_insert_position_00646d00<'a>(
    nodes: &[A90BstNode<'a>],
    root: Option<usize>,
    key: &[u8],
) -> (Option<usize>, A90InsertLink) {
    let mut last = None;
    let mut step = A90InsertLink::Left;
    let mut cursor = root;
    while let Some(index) = cursor {
        let node = &nodes[index];
        last = Some(index);
        if a90_string_compare_00646d60(key, node.key.as_bytes()) < 0 {
            step = A90InsertLink::Left;
            cursor = node.left;
        } else {
            step = A90InsertLink::Right;
            cursor = node.right;
        }
    }
    (last, step)
}

/// Collects every mapped value whose key compares equal to `key`.
///
/// This is the value-copy loop of `FUN_00644430`: lower-bound the first equal
/// key, then copy the node value (`+0x2c`) until the comparator stops
/// returning zero.  The translation accepts the already-sorted table and
/// therefore reproduces the same equal-range.
pub fn a90_multimap_values(table: &[(&str, u32)], key: &str) -> Vec<u32> {
    let mut low = 0usize;
    let mut high = table.len();
    while low < high {
        let middle = (low + high) / 2;
        if a90_string_compare_00646d60(table[middle].0.as_bytes(), key.as_bytes()) < 0 {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    let mut values = Vec::new();
    for &(candidate, value) in &table[low..] {
        if a90_string_compare_00646d60(candidate.as_bytes(), key.as_bytes()) != 0 {
            break;
        }
        values.push(value);
    }
    values
}

/// `0x006465c0` (`FUN_006465c0`, 191 bytes): two-level map lookup.
///
/// The routine first collects the equal range from the map at `0x00f05820`.
/// When that vector is empty it collects the equal range from the fallback
/// map at `0x00f0580c` and move-assigns it into the result.  The translation
/// returns the resulting value vector.
pub fn a90_two_level_map_lookup_006465c0(
    primary: &[(&str, u32)],
    fallback: &[(&str, u32)],
    key: &str,
) -> Vec<u32> {
    let values = a90_multimap_values(primary, key);
    if values.is_empty() {
        a90_multimap_values(fallback, key)
    } else {
        values
    }
}

/// `0x00646380` (`FUN_00646380`, 99 bytes): first mapped value for `key`.
///
/// The routine builds the two-level lookup above, then reads `*first` from
/// the returned vector.  A missing key dereferences the null data pointer in
/// the shipped code; here it yields `None`.
pub fn a90_map_first_value_00646380(
    primary: &[(&str, u32)],
    fallback: &[(&str, u32)],
    key: &str,
) -> Option<u32> {
    a90_two_level_map_lookup_006465c0(primary, fallback, key)
        .first()
        .copied()
}

/// `0x00646140` (`FUN_00646140`, 76 bytes): sound-table offset for the
/// fixed key `SND_ZERG_RESCUE`.
///
/// The shipped body builds that 15-byte key, performs the two-level lookup
/// and returns `first_value + argument`.  The argument is the byte offset
/// added to the table entry.
pub fn a90_sound_offset_00646140(
    primary: &[(&str, u32)],
    fallback: &[(&str, u32)],
    byte_offset: u32,
) -> Option<u32> {
    a90_map_first_value_00646380(primary, fallback, "SND_ZERG_RESCUE")
        .map(|value| value.wrapping_add(byte_offset))
}

/// `0x006467e0` (`FUN_006467e0`, 76 bytes): sound-table offset for the
/// fixed key `SND_Z_THUNK`.  Same `first_value + argument` shape as
/// `0x00646140`, with an 11-byte key.
pub fn a90_sound_offset_006467e0(
    primary: &[(&str, u32)],
    fallback: &[(&str, u32)],
    byte_offset: u32,
) -> Option<u32> {
    a90_map_first_value_00646380(primary, fallback, "SND_Z_THUNK")
        .map(|value| value.wrapping_add(byte_offset))
}

// ---------------------------------------------------------------------------
// Localised sound lookups (0x00646250 / 0x006466a0 / 0x00646b00)
// ---------------------------------------------------------------------------

/// The manager object at `0x00f028e8` owns the localised sound map used by
/// the three wrappers below.  Its lookup body is outside this batch, so each
/// translation threads it in as `lookup(category, key, argument)`.
pub fn a90_research_complete_00646250<F>(argument: u32, lookup: F) -> u32
where
    F: FnOnce(&str, &str, u32) -> u32,
{
    lookup("researchComplete", "SND_Z_RESEARCH_COMPLETE", argument)
}

/// `0x006466a0` (`FUN_006466a0`, 116 bytes): `startOfGame` lookup.  The
/// shipped routine builds an empty key and the 11-byte `startOfGame`
/// category before calling the manager.
pub fn a90_start_of_game_006466a0<F>(argument: u32, lookup: F) -> u32
where
    F: FnOnce(&str, &str, u32) -> u32,
{
    lookup("startOfGame", "", argument)
}

/// `0x00646b00` (`FUN_00646b00`, 116 bytes): `upgradeComplete` lookup for
/// the 22-byte `SND_Z_UPGRADE_COMPLETE` key.
pub fn a90_upgrade_complete_00646b00<F>(argument: u32, lookup: F) -> u32
where
    F: FnOnce(&str, &str, u32) -> u32,
{
    lookup("upgradeComplete", "SND_Z_UPGRADE_COMPLETE", argument)
}

// ---------------------------------------------------------------------------
// 0x006463f0 - filter unit records by name substring
// ---------------------------------------------------------------------------

/// `0x006463f0` (`FUN_006463f0`, 341 bytes): collect the indices of unit
/// records whose name contains the filter.
///
/// The global array base is `0x00fc9b74`, the count is `0x00fc9b78`, and the
/// stride is `0xb0`.  For each record the shipped code passes the name pair at
/// `+0x64`/`+0x68` and the filter pair to `FUN_0067d380` with its fifth
/// argument `1`; that path reduces to a C-string substring search (the flag
/// skips the two conversions and `FUN_00549a60` scans the first string for the
/// second, returning `-1` when absent and the match offset otherwise).
///
/// The result is a `std::vector<u16>` with the recovered growth rule
/// `new_capacity = max(requested, old_capacity / 2 + old_capacity)`.  The
/// decompiled element expression is a stack-pointer difference that falls
/// outside the vector; the observable store is the running record index.
pub fn a90_collect_name_match_indices_006463f0(names: &[&str], filter: &str) -> Vec<u16> {
    let mut result: Vec<u16> = Vec::new();
    let mut capacity: u32 = 0;
    let needle = filter.split('\0').next().unwrap_or("");
    for (index, name) in names.iter().enumerate() {
        let trimmed = name.split('\0').next().unwrap_or("");
        if !trimmed.contains(needle) {
            continue;
        }
        let count = result.len() as u32;
        if count == capacity {
            let requested = count.wrapping_add(1);
            let grown = (capacity >> 1).wrapping_add(capacity);
            capacity = if grown < requested { requested } else { grown };
        }
        result.push(index as u16);
    }
    result
}

// ---------------------------------------------------------------------------
// 0x00646c70 - tree destruction order
// ---------------------------------------------------------------------------

/// A `std::map` node view for the destructive walk at `0x00646c70`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A90TreeNode {
    /// Left child link at `+0x04`.
    pub left: Option<usize>,
    /// Right child link at `+0x08`.
    pub right: Option<usize>,
    /// `true` when the node's string payload (`+0x10`, capacity word at
    /// `+0x18`) is heap-backed and must be freed.
    pub payload_heap: bool,
}

/// One node visit in the destruction plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A90TreeDestroyVisit {
    /// Arena index of the node.
    pub node: usize,
    /// Whether the string payload at `+0x10` is freed before the node.
    pub free_payload: bool,
}

/// `0x00646c70` (`FUN_00646c70`, 116 bytes): recursive node destructor.
///
/// The shipped body destroys the left subtree, frees the node's string
/// payload when its capacity word at `+0x18` is non-negative (sign bit clear),
/// frees the node itself, then iterates down the right link.  The resulting
/// order is left subtree, node, right subtree (in-order).
pub fn a90_tree_destroy_plan_00646c70(
    nodes: &[A90TreeNode],
    root: Option<usize>,
) -> Vec<A90TreeDestroyVisit> {
    fn visit(nodes: &[A90TreeNode], node: Option<usize>, out: &mut Vec<A90TreeDestroyVisit>) {
        let Some(index) = node else {
            return;
        };
        let entry = nodes[index];
        visit(nodes, entry.left, out);
        out.push(A90TreeDestroyVisit {
            node: index,
            free_payload: entry.payload_heap,
        });
        visit(nodes, entry.right, out);
    }

    let mut out = Vec::new();
    visit(nodes, root, &mut out);
    out
}

// ---------------------------------------------------------------------------
// 0x00646df0 - vector reserve growth
// ---------------------------------------------------------------------------

/// `0x00646df0` (`FUN_00646df0`, 122 bytes): reserve growth rule.
///
/// With `old = flags & 0x7fff_ffff`, the routine does nothing when
/// `old >= requested`.  Otherwise it computes
/// `grown = old / 2 + old`, takes `max(requested, grown)` and writes the new
/// capacity through the XOR sequence
/// `((flags ^ new) & 0x7fff_ffff) ^ flags`, which preserves bit 31.
pub fn a90_reserve_capacity_00646df0(capacity_flags: u32, requested: u32) -> u32 {
    let old = capacity_flags & 0x7fff_ffff;
    if old >= requested {
        return capacity_flags;
    }
    let grown = (capacity_flags >> 1 & 0x3fff_ffff).wrapping_add(old);
    let new_capacity = if grown < requested { requested } else { grown };
    ((capacity_flags ^ new_capacity) & 0x7fff_ffff) ^ capacity_flags
}

// ---------------------------------------------------------------------------
// 0x00646fd0 - trailing signature check and body load
// ---------------------------------------------------------------------------

/// `NGIS` is the literal byte order stored at the end of the file; it is the
/// byte-reversed spelling of `SIGN` and is compared as the little-endian
/// `u32` `0x5349474e`.
pub const A90_SIGNATURE_MAGIC: [u8; 4] = *b"NGIS";

/// Result of the `0x00646fd0` seek/read plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A90SignatureTailPlan {
    /// Byte offset of the four-byte trailer (`file_size - offset - 4`).
    pub seek_offset: u32,
    /// Four bytes read at the trailer.
    pub trailer: [u8; 4],
    /// Number of body bytes to load from offset zero.
    pub body_len: u32,
    /// Whether the trailer matched and the body is loaded.
    pub load_body: bool,
}

/// `0x00646fd0` (`FUN_00646fd0`, 225 bytes): verify a trailing `NGIS`
/// signature and prepare the load of the preceding body.
///
/// `FUN_0093ffa0` returns the 64-bit file size.  The shipped code rejects a
/// nonzero high word or `offset + 4 > file_size` (both unsigned), seeks to
/// `(file_size - offset) - 4`, reads four bytes, and requires the little
/// endian word `0x5349474e`.  On success it seeks to zero and reads
/// `file_size - offset - 4` bytes.
pub fn a90_signature_tail_plan_00646fd0(
    file_size: u64,
    offset: u32,
    trailer: [u8; 4],
) -> A90SignatureTailPlan {
    let high = (file_size >> 32) as u32;
    let low = file_size as u32;
    let end = offset.wrapping_add(4);
    let seek_offset = low.wrapping_sub(offset).wrapping_sub(4);
    let in_range = high == 0 && low >= end;
    let load_body = in_range && trailer == A90_SIGNATURE_MAGIC;
    A90SignatureTailPlan {
        seek_offset,
        trailer,
        body_len: if in_range { seek_offset } else { 0 },
        load_body,
    }
}

// ---------------------------------------------------------------------------
// 0x00647390 / 0x006473f0 - draw record initialisation and blit arguments
// ---------------------------------------------------------------------------

/// Recovered `0x40`-byte draw record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A90DrawRecord {
    /// `+0x00`: first word.
    pub word0: u32,
    /// `+0x04`: frame id written as a `u16`.
    pub frame: u16,
    /// `+0x06`: high half of the frame word; `0x00647390` does not write it,
    /// so the caller's value is threaded through here.
    pub word6: u16,
    /// `+0x08..=+0x17`: four bounds words.
    pub bounds: [u32; 4],
    /// `+0x18..=+0x2f`: six words cleared by the initialiser.
    pub zeros: [u32; 6],
    /// `+0x30..=+0x3f`: four colour words.
    pub colors: [u32; 4],
}

impl A90DrawRecord {
    /// Flat `u32` view of the record, used by the copy-style helpers.
    pub fn a90_words(&self) -> [u32; 16] {
        [
            self.word0,
            (self.frame as u32) | ((self.word6 as u32) << 16),
            self.bounds[0],
            self.bounds[1],
            self.bounds[2],
            self.bounds[3],
            self.zeros[0],
            self.zeros[1],
            self.zeros[2],
            self.zeros[3],
            self.zeros[4],
            self.zeros[5],
            self.colors[0],
            self.colors[1],
            self.colors[2],
            self.colors[3],
        ]
    }
}

/// `0x00647390` (`FUN_00647390`, 83 bytes): initialise a draw record.
///
/// Stores the first argument at `+0x00`, the `u16` at `+0x04`, copies the
/// four words of `bounds_source` into `+0x08`, clears `+0x18..=+0x2f`, then
/// copies `colours_source` into `+0x30`.  The high half of the frame word is
/// not touched by the shipped stores and enters as `word6`.
pub fn a90_init_draw_record_00647390(
    word0: u32,
    frame: u16,
    word6: u16,
    colours_source: [u32; 4],
    bounds_source: [u32; 4],
) -> A90DrawRecord {
    A90DrawRecord {
        word0,
        frame,
        word6,
        bounds: bounds_source,
        zeros: [0; 6],
        colors: colours_source,
    }
}

/// The nine arguments passed to the rectangle blit at `FUN_00a8bbc0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A90BlitCall {
    /// `+0x08`: left edge.
    pub x: i32,
    /// `+0x0c`: top edge.
    pub y: i32,
    /// `+0x08 + +0x10`: right edge.
    pub x2: i32,
    /// `+0x0c + +0x14`: bottom edge.
    pub y2: i32,
    /// `(u16)(+0x04) - 1`, zero-extended to 32 bits.
    pub frame: u32,
    /// `+0x30..=+0x3f`: four colour/flag words.
    pub colors: [u32; 4],
}

/// `0x006473f0` (`FUN_006473f0`, 164 bytes): build the blit arguments.
///
/// The shipped body computes `(x, y, x + width, y + height)` from
/// `+0x08..=+0x14`, decrements the frame word with 16-bit wrap
/// (`dec ax; movzx eax,ax`) and forwards `+0x30..=+0x3c` unchanged.
pub fn a90_draw_record_blit_006473f0(record: &A90DrawRecord) -> A90BlitCall {
    let x = record.bounds[0] as i32;
    let y = record.bounds[1] as i32;
    A90BlitCall {
        x,
        y,
        x2: x.wrapping_add(record.bounds[2] as i32),
        y2: y.wrapping_add(record.bounds[3] as i32),
        frame: record.frame.wrapping_sub(1) as u32,
        colors: record.colors,
    }
}

// ---------------------------------------------------------------------------
// 0x00647580 / 0x006474a0 - text runs and line splitting
// ---------------------------------------------------------------------------

/// Clip rectangle stored as left/top/width/height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A90Clip {
    /// `+0x18`
    pub left: i32,
    /// `+0x1c`
    pub top: i32,
    /// `+0x20`
    pub width: i32,
    /// `+0x24`
    pub height: i32,
}

/// The two packed screen-rectangle words read by `0x00647580`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A90ScreenRect {
    /// `0x011cb5f4` low half.
    pub left: i32,
    /// `0x011cb5f4` high half.
    pub top: i32,
    /// `0x011cb5f8` low half.
    pub right: i32,
    /// `0x011cb5f8` high half.
    pub bottom: i32,
}

impl A90ScreenRect {
    /// Expand the two packed `(low, high)` words exactly as the shipped body
    /// does with `movsx` of the low and high halves.
    pub fn a90_from_words(xy_word: u32, right_bottom_word: u32) -> Self {
        Self {
            left: (xy_word as u16 as i16) as i32,
            top: ((xy_word >> 16) as u16 as i16) as i32,
            right: (right_bottom_word as u16 as i16) as i32,
            bottom: ((right_bottom_word >> 16) as u16 as i16) as i32,
        }
    }
}

/// Full text layout state at the entry of `0x00647580`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A90TextState {
    /// `+0x00`: colour word pushed to `FUN_0065b920`.
    pub colour: u32,
    /// `+0x04`: font word pushed to `FUN_0065b920`.
    pub font: u16,
    /// `+0x08`: x origin.
    pub origin_x: i32,
    /// `+0x0c`: y origin.
    pub origin_y: i32,
    /// `+0x10`: width used by centred runs.
    pub width: i32,
    /// `+0x14`: height.
    pub height: i32,
    /// `+0x18..=+0x27`: clip left/top/width/height.
    pub clip: A90Clip,
    /// `+0x28`: x offset.
    pub x_offset: i32,
    /// `+0x2c`: y offset.
    pub y_offset: i32,
    /// `+0x30..=+0x3f`: draw colour words.
    pub colors: [u32; 4],
}

/// One line handed to `0x00647580` together with the width returned by the
/// text measurement at `FUN_009585d0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct A90TextLine<'a> {
    /// Line bytes (the original `std::string` data).
    pub text: &'a str,
    /// Result of `FUN_009585d0` as a single-precision float.
    pub measured_width: f32,
}

/// One draw command produced by `0x00647580`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A90TextDraw {
    /// x argument to `FUN_00957d90`.
    pub x: i32,
    /// y argument to `FUN_00957d90`.
    pub y: i32,
    /// Index into the input line slice.
    pub line: usize,
}

/// `0x00647580` (`FUN_00647580`, 456 bytes): lay out and draw text lines.
///
/// For every line the routine pushes its colour/font pair, measures the line,
/// computes:
///
/// * centred (`param_2 == 1`):
///   `x = origin_x + (width - trunc(floor(f32_width))) / 2`,
///   `y = origin_y + y_offset`;
/// * otherwise:
///   `x = origin_x + x_offset`, `y = origin_y + y_offset`.
///
/// It then intersects the stored clip with the screen rectangle.  The stored
/// `+0x20`/`+0x24` values are width/height, so the intersection is
/// `left = min(screen.left, clip.left)`,
/// `top = min(screen.top, clip.top)`,
/// `width = max(screen.right, clip.left + clip.width) - left`,
/// `height = max(screen.bottom, clip.top + clip.height) - top`.
/// After every line except the last, `y_offset += line_height` and
/// `x_offset = 0`.
pub fn a90_text_runs_layout_00647580<'a>(
    state: &A90TextState,
    lines: &[A90TextLine<'a>],
    centred: bool,
    screen: A90ScreenRect,
    line_height: u8,
) -> (Vec<A90TextDraw>, A90TextState) {
    let mut next = *state;
    let mut draws = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().enumerate() {
        let (x, y) = if centred {
            let measured = a90_text_width_i32(line.measured_width);
            (
                next.origin_x
                    .wrapping_add(next.width.wrapping_sub(measured) / 2),
                next.origin_y.wrapping_add(next.y_offset),
            )
        } else {
            (
                next.origin_x.wrapping_add(next.x_offset),
                next.origin_y.wrapping_add(next.y_offset),
            )
        };
        draws.push(A90TextDraw { x, y, line: index });

        if next.clip.width != 0 || next.clip.height != 0 {
            let left = screen.left.min(next.clip.left);
            let top = screen.top.min(next.clip.top);
            let right = screen
                .right
                .max(next.clip.left.wrapping_add(next.clip.width));
            let bottom = screen
                .bottom
                .max(next.clip.top.wrapping_add(next.clip.height));
            next.clip = A90Clip {
                left,
                top,
                width: right.wrapping_sub(left),
                height: bottom.wrapping_sub(top),
            };
        }

        if index + 1 < lines.len() {
            next.y_offset = next.y_offset.wrapping_add(line_height as i32);
            next.x_offset = 0;
        }
    }
    (draws, next)
}

/// Convert the measurement to the integer used by the centred branch.
///
/// The shipped sequence is `fstp f32; cvtps2pd; call floor; cvtpd2ps;
/// cvttss2si`.  `floor` of a finite value is already integral, so the
/// truncating conversion is reproduced by [`f32::floor`] and Rust's
/// saturating `as i32` cast.
pub fn a90_text_width_i32(measured_width: f32) -> i32 {
    measured_width.floor() as i32
}

/// `0x006474a0` (`FUN_006474a0`, 144 bytes): split a string at newline and
/// lay out the resulting lines.
///
/// The constructor call at `FUN_0067d7b0` is invoked with delimiter `10`, the
/// keep-first-empty flag `0` and the input pair `(*param_1, param_1[1])`;
/// the loop pushes every segment including empty ones.  The vector is then
/// passed to `0x00647580`, and the tail destroys each 0x1c-byte string.
pub fn a90_text_block_006474a0<'a>(
    text: &'a str,
    measured_widths: &[f32],
    state: &A90TextState,
    centred: bool,
    screen: A90ScreenRect,
    line_height: u8,
) -> (Vec<A90TextDraw>, A90TextState) {
    let lines: Vec<A90TextLine<'a>> = text
        .split('\n')
        .enumerate()
        .map(|(index, text)| A90TextLine {
            text,
            measured_width: measured_widths.get(index).copied().unwrap_or(0.0),
        })
        .collect();
    a90_text_runs_layout_00647580(state, &lines, centred, screen, line_height)
}

/// One `0x1c`-byte string slot in the line vector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A90StringSlot {
    /// `+0x00`: heap data pointer.
    pub data: u32,
    /// `+0x04`: byte length.
    pub count: u32,
    /// `+0x08`: capacity flags; bit 31 set means inline.
    pub flags: u32,
}

/// `0x00647531` (`FUN_00647531`, 38 bytes): free the heap-backed line
/// strings.
///
/// The fragment walks 0x1c-byte slots from `+0x00` to `count * 0x1c` and
/// frees `slot.data` when `slot.flags` has bit 31 clear.  It returns the
/// freed pointers in visit order.
pub fn a90_free_heap_strings_00647531(slots: &[A90StringSlot]) -> Vec<u32> {
    let mut freed = Vec::new();
    for slot in slots {
        if slot.flags & 0x8000_0000 == 0 {
            freed.push(slot.data);
        }
    }
    freed
}

/// `0x00647557` (`FUN_00647557`, 28 bytes): free the vector's own buffer.
///
/// The fragment tests the vector header's capacity word and frees the data
/// pointer when bit 31 is clear.  `None` means the buffer is inline.
pub fn a90_free_vector_buffer_00647557(data: u32, flags: u32) -> Option<u32> {
    if flags & 0x8000_0000 == 0 {
        Some(data)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// 0x00647750 / 0x00647770 / 0x00647780 - small record helpers
// ---------------------------------------------------------------------------

/// `0x00647750` (`FUN_00647750`, 9 bytes): copy the saved bounds at
/// `+0x18..=+0x24` down to `+0x08..=+0x14`.
///
/// The argument is the ten-word `0x28`-byte record; words 6..9 are copied to
/// words 2..5 with two `movups` stores.  The helper returns the new bounds.
pub fn a90_reset_bounds_00647750(words: &mut [u32; 10]) -> [u32; 4] {
    words[2] = words[6];
    words[3] = words[7];
    words[4] = words[8];
    words[5] = words[9];
    [words[2], words[3], words[4], words[5]]
}

/// Copyable-function-object vtable written by `0x00647770`
/// (`0x00d3cafc`).
pub const A90_FUNCTOR_VTABLE_00647770: u32 = 0x00d3_cafc;

/// `0x00647770` (`FUN_00647770`, 16 bytes): store the lambda's vtable
/// pointer into the object.
pub fn a90_write_functor_vtable_00647770(out: &mut u32) {
    *out = A90_FUNCTOR_VTABLE_00647770;
}

/// RTTI type descriptor returned by `0x00647780` (`0x00f535f4`).
pub const A90_LAMBDA_TYPE_DESCRIPTOR_00647780: u32 = 0x00f5_35f4;

/// `0x00647780` (`FUN_00647780`, 6 bytes): `mov eax,0x00f535f4; ret`.
pub fn a90_lambda_type_descriptor_00647780() -> u32 {
    A90_LAMBDA_TYPE_DESCRIPTOR_00647780
}

// ---------------------------------------------------------------------------
// 0x00647932 / 0x00647997 - unordered-map hash and chain helpers
// ---------------------------------------------------------------------------

/// First Murmur-style multiplier used by both hash sites.
pub const A90_HASH_MULTIPLIER_1: u32 = 0x85eb_ca6b;
/// Second Murmur-style multiplier used by both hash sites.
pub const A90_HASH_MULTIPLIER_2: u32 = 0xc2b2_ae35;

/// The exact finaliser recovered at `0x00647932` and `0x00647997`.
///
/// `h ^= h >> 16; h *= 0x85ebca6b; h ^= h >> 13; h *= 0xc2b2ae35;
/// h ^= h >> 16`.
pub fn a90_hash_u32_00647932(mut value: u32) -> u32 {
    value ^= value >> 16;
    value = value.wrapping_mul(A90_HASH_MULTIPLIER_1);
    value ^= value >> 13;
    value = value.wrapping_mul(A90_HASH_MULTIPLIER_2);
    value ^= value >> 16;
    value
}

/// Bucket index after the hash: `hash & (bucket_count - 1)`.
pub fn a90_bucket_index_00647932(hash: u32, bucket_count: u32) -> u32 {
    hash & bucket_count.wrapping_sub(1)
}

/// Insert plan for the `u16`-keyed map at `0x00647932`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct A90HashInsertPlan {
    /// Finaliser output.
    pub hash: u32,
    /// `hash & (bucket_count - 1)`.
    pub bucket_index: u32,
    /// Whether the bucket chain already contains the key.
    pub duplicate: bool,
    /// Element count after a successful insert.
    pub count_after: u32,
    /// New bucket count when the load factor grows, else `None`.
    pub rehash_to: Option<u32>,
}

/// `0x00647932` (`FUN_00647932`, 101 bytes): `unordered_map<u16, ...>`
/// insert path.
///
/// The routine hashes the `u16` key at bucket-node `+0x04`, masks the hash
/// with `bucket_count - 1`, and walks the bucket chain comparing the `u16`
/// key.  A duplicate is destroyed rather than inserted.  A successful insert
/// links a 0x1c-byte node, increments the count, and doubles the bucket count
/// when `count / bucket_count > max_load` (the shipped `comiss`/`jbe`
/// sequence; unordered results skip the growth).
pub fn a90_hash_insert_plan_00647932(
    bucket_count: u32,
    count: u32,
    max_load: f32,
    key: u16,
    chain_contains_key: bool,
) -> A90HashInsertPlan {
    let hash = a90_hash_u32_00647932(key as u32);
    let bucket_index = a90_bucket_index_00647932(hash, bucket_count);
    if chain_contains_key {
        return A90HashInsertPlan {
            hash,
            bucket_index,
            duplicate: true,
            count_after: count,
            rehash_to: None,
        };
    }
    let count_after = count.wrapping_add(1);
    let ratio = count_after as f32 / bucket_count as f32;
    let rehash_to = if ratio > max_load {
        Some(bucket_count.wrapping_mul(2))
    } else {
        None
    };
    A90HashInsertPlan {
        hash,
        bucket_index,
        duplicate: false,
        count_after,
        rehash_to,
    }
}

/// One `u32`-keyed bucket-chain node for `0x00647997`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A90HashU32Node {
    /// Key at `+0x04`.
    pub key: u32,
    /// Next link at `+0x00`.
    pub next: Option<usize>,
}

/// `0x00647997` (`FUN_00647997`, 228 bytes): lookup path of the `u32`-keyed
/// map.
///
/// The fragment hashes the key with the same finaliser, masks with
/// `bucket_count - 1`, and walks the chain comparing the full `u32` key.  The
/// returned index is the matching node; `None` means the key is absent and
/// the shipped code takes the insertion path.
pub fn a90_hash_chain_lookup_00647997(
    nodes: &[A90HashU32Node],
    bucket_count: u32,
    head: Option<usize>,
    key: u32,
) -> Option<usize> {
    let hash = a90_hash_u32_00647932(key);
    let _bucket_index = a90_bucket_index_00647932(hash, bucket_count);
    let mut cursor = head;
    while let Some(index) = cursor {
        let node = nodes[index];
        if node.key == key {
            return Some(index);
        }
        cursor = node.next;
    }
    None
}

// ---------------------------------------------------------------------------
// 0x00647f50 - variant copy dispatch
// ---------------------------------------------------------------------------

/// Payload copied by the `0x00647f50` variant dispatcher.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A90VariantPayload<'a> {
    /// Tags `0`, `2` and `3`: one dword copied from `+0x04`.
    Word(u32),
    /// Tag `1`: only the low byte at `+0x04` is copied.
    Byte(u8),
    /// Tag `4`: `FUN_00539990` copies an inline string.
    InlineString(&'a str),
    /// Tag `5`: `FUN_005399f0` zeroes the three-word vector header at `+0x04`.
    EmptyVector,
}

/// Source variant value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A90Variant<'a> {
    /// Tag word at `+0x00`.
    pub tag: u32,
    /// Payload at `+0x04`.
    pub payload: A90VariantPayload<'a>,
}

/// `0x00647f50` (`FUN_00647f50`, 24 bytes): small variant copy dispatcher.
///
/// The destination tag is zeroed first and then selected by the source tag
/// through the jump table at `0x00647fd8`:
///
/// * tag `0`: copy the dword payload, tag stays `0`;
/// * tag `1`: copy the low payload byte, tag becomes `1`;
/// * tag `2`: copy the dword payload, tag becomes `2`;
/// * tag `3`: copy the dword payload, tag becomes `3`;
/// * tag `4`: `FUN_00539990` copy-constructs an inline string payload;
/// * tag `5`: `FUN_005399f0` zero-initialises a three-word vector header.
///
/// Tags outside `0..=5` are not handled by the jump table; the translation
/// keeps the zero tag and `Word(0)` so the result remains defined.
pub fn a90_copy_variant_00647f50<'a>(source: &A90Variant<'a>) -> A90Variant<'a> {
    match source.tag {
        0 => A90Variant {
            tag: 0,
            payload: match source.payload {
                A90VariantPayload::Word(value) => A90VariantPayload::Word(value),
                A90VariantPayload::Byte(value) => A90VariantPayload::Word(value as u32),
                A90VariantPayload::InlineString(_) | A90VariantPayload::EmptyVector => {
                    A90VariantPayload::Word(0)
                }
            },
        },
        1 => A90Variant {
            tag: 1,
            payload: A90VariantPayload::Byte(match source.payload {
                A90VariantPayload::Word(value) => value as u8,
                A90VariantPayload::Byte(value) => value,
                A90VariantPayload::InlineString(_) | A90VariantPayload::EmptyVector => 0,
            }),
        },
        2 => A90Variant {
            tag: 2,
            payload: match source.payload {
                A90VariantPayload::Word(value) => A90VariantPayload::Word(value),
                A90VariantPayload::Byte(value) => A90VariantPayload::Word(value as u32),
                A90VariantPayload::InlineString(_) | A90VariantPayload::EmptyVector => {
                    A90VariantPayload::Word(0)
                }
            },
        },
        3 => A90Variant {
            tag: 3,
            payload: match source.payload {
                A90VariantPayload::Word(value) => A90VariantPayload::Word(value),
                A90VariantPayload::Byte(value) => A90VariantPayload::Word(value as u32),
                A90VariantPayload::InlineString(_) | A90VariantPayload::EmptyVector => {
                    A90VariantPayload::Word(0)
                }
            },
        },
        4 => A90Variant {
            tag: 4,
            payload: match source.payload {
                A90VariantPayload::InlineString(value) => A90VariantPayload::InlineString(value),
                A90VariantPayload::Word(_)
                | A90VariantPayload::Byte(_)
                | A90VariantPayload::EmptyVector => A90VariantPayload::InlineString(""),
            },
        },
        5 => A90Variant {
            tag: 5,
            payload: A90VariantPayload::EmptyVector,
        },
        _ => A90Variant {
            tag: 0,
            payload: A90VariantPayload::Word(0),
        },
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent90 {
    use super::*;

    #[test]
    fn string_compare_matches_lexicographic_rules() {
        assert_eq!(a90_string_compare_00646d60(b"abc", b"abc"), 0);
        assert_eq!(a90_string_compare_00646d60(b"abc", b"abd"), -1);
        assert_eq!(a90_string_compare_00646d60(b"abd", b"abc"), 1);
        assert_eq!(a90_string_compare_00646d60(b"ab", b"abc"), -1);
        assert_eq!(a90_string_compare_00646d60(b"abcd", b"abc"), 1);
        assert_eq!(a90_string_compare_00646d60(b"", b"a"), -1);
        assert_eq!(a90_string_compare_00646d60(b"", b""), 0);
        assert_eq!(a90_string_compare_00646d60(b"Z", b"a"), -1);
    }

    #[test]
    fn multimap_values_returns_only_the_equal_range() {
        let table = [("a", 1), ("b", 2), ("b", 3), ("b", 4), ("c", 5)];
        assert_eq!(a90_multimap_values(&table, "b"), vec![2, 3, 4]);
        assert_eq!(a90_multimap_values(&table, "a"), vec![1]);
        assert_eq!(a90_multimap_values(&table, "d"), Vec::<u32>::new());
        assert_eq!(a90_multimap_values(&table, ""), Vec::<u32>::new());
    }

    #[test]
    fn two_level_lookup_prefers_primary_and_falls_back() {
        let primary = [("SND_Z_THUNK", 0x20), ("SND_Z_THUNK", 0x21)];
        let fallback = [("SND_ZERG_RESCUE", 0x40), ("SND_Z_THUNK", 0x99)];
        assert_eq!(
            a90_two_level_map_lookup_006465c0(&primary, &fallback, "SND_Z_THUNK"),
            vec![0x20, 0x21]
        );
        assert_eq!(
            a90_two_level_map_lookup_006465c0(&primary, &fallback, "SND_ZERG_RESCUE"),
            vec![0x40]
        );
        assert_eq!(
            a90_map_first_value_00646380(&primary, &fallback, "SND_ZERG_RESCUE"),
            Some(0x40)
        );
    }

    #[test]
    fn fixed_sound_keys_add_the_byte_offset() {
        let primary = [("SND_ZERG_RESCUE", 0x100), ("SND_Z_THUNK", 0x200)];
        assert_eq!(a90_sound_offset_00646140(&primary, &[], 0x1c), Some(0x11c));
        assert_eq!(a90_sound_offset_006467e0(&primary, &[], 0x1c), Some(0x21c));
        assert_eq!(a90_sound_offset_00646140(&[], &[], 4), None);
    }

    #[test]
    fn localised_lookup_passes_exact_key_pairs_in_manager_order() {
        let research = a90_research_complete_00646250(7, |category, key, argument| {
            assert_eq!(category, "researchComplete");
            assert_eq!(key, "SND_Z_RESEARCH_COMPLETE");
            assert_eq!(argument, 7);
            0x31
        });
        assert_eq!(research, 0x31);

        let start = a90_start_of_game_006466a0(9, |category, key, argument| {
            assert_eq!(category, "startOfGame");
            assert_eq!(key, "");
            assert_eq!(argument, 9);
            0x32
        });
        assert_eq!(start, 0x32);

        let upgrade = a90_upgrade_complete_00646b00(11, |category, key, argument| {
            assert_eq!(category, "upgradeComplete");
            assert_eq!(key, "SND_Z_UPGRADE_COMPLETE");
            assert_eq!(argument, 11);
            0x33
        });
        assert_eq!(upgrade, 0x33);
    }

    #[test]
    fn name_substring_filter_collects_record_indices() {
        let names = ["Zergling", "Zealot", "zergling", "Marine"];
        assert_eq!(
            a90_collect_name_match_indices_006463f0(&names, "Zerg"),
            vec![0]
        );
        assert_eq!(
            a90_collect_name_match_indices_006463f0(&names, "zerg"),
            vec![2]
        );
        assert_eq!(
            a90_collect_name_match_indices_006463f0(&names, ""),
            vec![0, 1, 2, 3]
        );
        assert_eq!(
            a90_collect_name_match_indices_006463f0(&names, "erg"),
            vec![0, 2]
        );
        assert_eq!(
            a90_collect_name_match_indices_006463f0(&names, "Q"),
            Vec::<u16>::new()
        );
        assert_eq!(
            a90_collect_name_match_indices_006463f0(&["Zerg\0hidden"], "Zerg"),
            vec![0]
        );
    }

    #[test]
    fn insert_position_walk_matches_map_semantics() {
        let nodes = [
            A90BstNode {
                key: "b",
                left: Some(1),
                right: Some(2),
            },
            A90BstNode {
                key: "a",
                left: None,
                right: None,
            },
            A90BstNode {
                key: "c",
                left: None,
                right: None,
            },
        ];
        assert_eq!(
            a90_insert_position_00646d00(&nodes, Some(0), b"b"),
            (Some(2), A90InsertLink::Left)
        );
        assert_eq!(
            a90_insert_position_00646d00(&nodes, Some(0), b"a"),
            (Some(1), A90InsertLink::Right)
        );
        assert_eq!(
            a90_insert_position_00646d00(&nodes, Some(0), b"0"),
            (Some(1), A90InsertLink::Left)
        );
        assert_eq!(
            a90_insert_position_00646d00(&nodes, None, b"x"),
            (None, A90InsertLink::Left)
        );
    }

    #[test]
    fn tree_destroy_plan_is_in_order_and_frees_heap_payloads() {
        let nodes = [
            A90TreeNode {
                left: Some(1),
                right: Some(2),
                payload_heap: true,
            },
            A90TreeNode {
                left: None,
                right: None,
                payload_heap: false,
            },
            A90TreeNode {
                left: None,
                right: None,
                payload_heap: true,
            },
        ];
        assert_eq!(
            a90_tree_destroy_plan_00646c70(&nodes, Some(0)),
            vec![
                A90TreeDestroyVisit {
                    node: 1,
                    free_payload: false
                },
                A90TreeDestroyVisit {
                    node: 0,
                    free_payload: true
                },
                A90TreeDestroyVisit {
                    node: 2,
                    free_payload: true
                },
            ]
        );
        assert_eq!(
            a90_tree_destroy_plan_00646c70(&nodes, None),
            Vec::<A90TreeDestroyVisit>::new()
        );
    }

    #[test]
    fn reserve_capacity_reproduces_the_xor_growth_rule() {
        assert_eq!(
            a90_reserve_capacity_00646df0(0x8000_000f, 0x17),
            0x8000_0017
        );
        assert_eq!(
            a90_reserve_capacity_00646df0(0x8000_000f, 0x10),
            0x8000_0016
        );
        assert_eq!(
            a90_reserve_capacity_00646df0(0x0000_0010, 0x10),
            0x0000_0010
        );
        assert_eq!(
            a90_reserve_capacity_00646df0(0x8000_0010, 0x20),
            0x8000_0020
        );
        assert_eq!(a90_reserve_capacity_00646df0(0x0000_0000, 0x1), 0x0000_0001);
    }

    #[test]
    fn signature_tail_plan_matches_signed_trailer_rule() {
        let plan = a90_signature_tail_plan_00646fd0(0x100, 0x10, *b"NGIS");
        assert_eq!(plan.seek_offset, 0xec);
        assert_eq!(plan.body_len, 0xec);
        assert!(plan.load_body);

        let plan = a90_signature_tail_plan_00646fd0(0x100, 0x10, *b"SIGN");
        assert_eq!(plan.seek_offset, 0xec);
        assert!(!plan.load_body);

        let plan = a90_signature_tail_plan_00646fd0(0x1_0000_0000, 0, *b"NGIS");
        assert!(!plan.load_body);
        assert_eq!(plan.body_len, 0);

        let plan = a90_signature_tail_plan_00646fd0(4, 4, *b"NGIS");
        assert_eq!(plan.seek_offset, 0xffff_fffc);
        assert!(!plan.load_body);
    }

    #[test]
    fn draw_record_initialisation_and_blit_arguments() {
        let record =
            a90_init_draw_record_00647390(0x11, 7, 0xdead, [0xa, 0xb, 0xc, 0xd], [10, 20, 30, 40]);
        assert_eq!(record.word0, 0x11);
        assert_eq!(record.frame, 7);
        assert_eq!(record.word6, 0xdead);
        assert_eq!(record.bounds, [10, 20, 30, 40]);
        assert_eq!(record.zeros, [0; 6]);
        assert_eq!(record.colors, [0xa, 0xb, 0xc, 0xd]);
        assert_eq!(record.a90_words()[1], 0xdead_0007);

        let call = a90_draw_record_blit_006473f0(&record);
        assert_eq!(call.x, 10);
        assert_eq!(call.y, 20);
        assert_eq!(call.x2, 40);
        assert_eq!(call.y2, 60);
        assert_eq!(call.frame, 6);
        assert_eq!(call.colors, [0xa, 0xb, 0xc, 0xd]);

        let wrapped = a90_init_draw_record_00647390(0, 0, 0, [0; 4], [0; 4]);
        assert_eq!(a90_draw_record_blit_006473f0(&wrapped).frame, 0xffff);
    }

    fn text_state() -> A90TextState {
        A90TextState {
            colour: 0x01,
            font: 0x02,
            origin_x: 10,
            origin_y: 20,
            width: 100,
            height: 30,
            clip: A90Clip {
                left: 5,
                top: 6,
                width: 50,
                height: 10,
            },
            x_offset: 3,
            y_offset: 4,
            colors: [1, 2, 3, 4],
        }
    }

    #[test]
    fn text_layout_uses_floor_and_truncating_centre_division() {
        assert_eq!(a90_text_width_i32(19.9), 19);
        assert_eq!(a90_text_width_i32(20.0), 20);
        assert_eq!(a90_text_width_i32(-0.5), -1);

        let lines = [
            A90TextLine {
                text: "abc",
                measured_width: 20.0,
            },
            A90TextLine {
                text: "de",
                measured_width: 11.9,
            },
        ];
        let screen = A90ScreenRect {
            left: 0,
            top: 0,
            right: 200,
            bottom: 100,
        };
        let (draws, state) = a90_text_runs_layout_00647580(&text_state(), &lines, true, screen, 7);
        assert_eq!(draws.len(), 2);
        assert_eq!(
            draws[0],
            A90TextDraw {
                x: 50,
                y: 24,
                line: 0
            }
        );
        assert_eq!(
            draws[1],
            A90TextDraw {
                x: 54,
                y: 31,
                line: 1
            }
        );
        assert_eq!(
            state.clip,
            A90Clip {
                left: 0,
                top: 0,
                width: 200,
                height: 100
            }
        );
        assert_eq!(state.y_offset, 11);
        assert_eq!(state.x_offset, 0);
    }

    #[test]
    fn text_layout_unextended_clip_and_left_aligned_offsets() {
        let mut state = text_state();
        state.clip = A90Clip {
            left: 10,
            top: 5,
            width: 200,
            height: 90,
        };
        let lines = [A90TextLine {
            text: "x",
            measured_width: 1.0,
        }];
        let screen = A90ScreenRect {
            left: 0,
            top: 0,
            right: 100,
            bottom: 50,
        };
        let (draws, next) = a90_text_runs_layout_00647580(&state, &lines, false, screen, 8);
        assert_eq!(
            draws[0],
            A90TextDraw {
                x: 13,
                y: 24,
                line: 0
            }
        );
        assert_eq!(
            next.clip,
            A90Clip {
                left: 0,
                top: 0,
                width: 210,
                height: 95
            }
        );
        assert_eq!(next.y_offset, 4);
    }

    #[test]
    fn text_block_splits_newlines_and_uses_measured_widths() {
        let screen = A90ScreenRect {
            left: 0,
            top: 0,
            right: 100,
            bottom: 100,
        };
        let (draws, _) = a90_text_block_006474a0(
            "one\n\nthree\n",
            &[9.0, 0.0, 15.0, 0.0],
            &text_state(),
            false,
            screen,
            5,
        );
        assert_eq!(draws.len(), 4);
        assert_eq!(draws[0].line, 0);
        assert_eq!(draws[1].line, 1);
        assert_eq!(draws[2].line, 2);
        assert_eq!(draws[3].line, 3);

        let (draws, _) = a90_text_block_006474a0("a\nb", &[10.0], &text_state(), false, screen, 5);
        assert_eq!(draws.len(), 2);
        assert_eq!(draws[1].y, 29);
    }

    #[test]
    fn string_vector_cleanup_frees_only_heap_slots() {
        let slots = [
            A90StringSlot {
                data: 0x1000,
                count: 1,
                flags: 0x8000_000f,
            },
            A90StringSlot {
                data: 0x2000,
                count: 4,
                flags: 0x0000_000f,
            },
            A90StringSlot {
                data: 0x3000,
                count: 0,
                flags: 0x0000_0000,
            },
        ];
        assert_eq!(a90_free_heap_strings_00647531(&slots), vec![0x2000, 0x3000]);
        assert_eq!(a90_free_vector_buffer_00647557(0x4000, 0x8000_0004), None);
        assert_eq!(
            a90_free_vector_buffer_00647557(0x4000, 0x0000_0004),
            Some(0x4000)
        );
    }

    #[test]
    fn small_record_helpers_keep_exact_constants() {
        let mut words = [0u32; 10];
        words[6] = 0x10;
        words[7] = 0x11;
        words[8] = 0x12;
        words[9] = 0x13;
        assert_eq!(
            a90_reset_bounds_00647750(&mut words),
            [0x10, 0x11, 0x12, 0x13]
        );
        assert_eq!(&words[2..6], &[0x10, 0x11, 0x12, 0x13]);

        let mut vtable = 0;
        a90_write_functor_vtable_00647770(&mut vtable);
        assert_eq!(vtable, 0x00d3_cafc);
        assert_eq!(a90_lambda_type_descriptor_00647780(), 0x00f5_35f4);
    }

    #[test]
    fn hash_finaliser_and_bucket_mask_match_retail_values() {
        assert_eq!(a90_hash_u32_00647932(0), 0);
        assert_eq!(a90_hash_u32_00647932(1), 0x514e_28b7);
        assert_eq!(a90_hash_u32_00647932(0x1234), 0xb4c8_1a85);
        assert_eq!(a90_hash_u32_00647932(0xabcd), 0x4422_1e35);
        assert_eq!(a90_hash_u32_00647932(0xffff), 0xa23b_ae67);
        assert_eq!(a90_hash_u32_00647932(0xdead_beef), 0x0de5_c6a9);
        assert_eq!(a90_bucket_index_00647932(0xffff_ffff, 16), 15);
        assert_eq!(a90_bucket_index_00647932(0x1234_5678, 8), 0);
    }

    #[test]
    fn hash_insert_plan_reports_duplicate_count_and_rehash() {
        let plan = a90_hash_insert_plan_00647932(16, 16, 1.0, 0xabcd, false);
        assert_eq!(plan.hash, 0x4422_1e35);
        assert_eq!(plan.bucket_index, 5);
        assert!(!plan.duplicate);
        assert_eq!(plan.count_after, 17);
        assert_eq!(plan.rehash_to, Some(32));

        let plan = a90_hash_insert_plan_00647932(16, 15, 1.0, 0xabcd, false);
        assert_eq!(plan.count_after, 16);
        assert_eq!(plan.rehash_to, None);

        let plan = a90_hash_insert_plan_00647932(16, 15, 1.0, 0xabcd, true);
        assert!(plan.duplicate);
        assert_eq!(plan.count_after, 15);
        assert_eq!(plan.rehash_to, None);

        let plan = a90_hash_insert_plan_00647932(16, 0, f32::NAN, 1, false);
        assert_eq!(plan.rehash_to, None);
    }

    #[test]
    fn hash_chain_lookup_walks_the_u32_chain() {
        let nodes = [
            A90HashU32Node {
                key: 0x99,
                next: Some(1),
            },
            A90HashU32Node {
                key: 0xabcd,
                next: Some(2),
            },
            A90HashU32Node {
                key: 0x1234,
                next: None,
            },
        ];
        assert_eq!(
            a90_hash_chain_lookup_00647997(&nodes, 16, Some(0), 0xabcd),
            Some(1)
        );
        assert_eq!(
            a90_hash_chain_lookup_00647997(&nodes, 16, Some(0), 0x1234),
            Some(2)
        );
        assert_eq!(a90_hash_chain_lookup_00647997(&nodes, 16, Some(0), 7), None);
        assert_eq!(a90_hash_chain_lookup_00647997(&nodes, 16, None, 7), None);
    }

    #[test]
    fn variant_copy_dispatches_on_the_tag_table() {
        assert_eq!(
            a90_copy_variant_00647f50(&A90Variant {
                tag: 0,
                payload: A90VariantPayload::Word(0x1234_5678)
            }),
            A90Variant {
                tag: 0,
                payload: A90VariantPayload::Word(0x1234_5678)
            }
        );
        assert_eq!(
            a90_copy_variant_00647f50(&A90Variant {
                tag: 1,
                payload: A90VariantPayload::Byte(0xab)
            }),
            A90Variant {
                tag: 1,
                payload: A90VariantPayload::Byte(0xab)
            }
        );
        assert_eq!(
            a90_copy_variant_00647f50(&A90Variant {
                tag: 2,
                payload: A90VariantPayload::Word(7)
            }),
            A90Variant {
                tag: 2,
                payload: A90VariantPayload::Word(7)
            }
        );
        assert_eq!(
            a90_copy_variant_00647f50(&A90Variant {
                tag: 3,
                payload: A90VariantPayload::Word(8)
            }),
            A90Variant {
                tag: 3,
                payload: A90VariantPayload::Word(8)
            }
        );
        assert_eq!(
            a90_copy_variant_00647f50(&A90Variant {
                tag: 4,
                payload: A90VariantPayload::InlineString("abc")
            }),
            A90Variant {
                tag: 4,
                payload: A90VariantPayload::InlineString("abc")
            }
        );
        assert_eq!(
            a90_copy_variant_00647f50(&A90Variant {
                tag: 5,
                payload: A90VariantPayload::EmptyVector
            }),
            A90Variant {
                tag: 5,
                payload: A90VariantPayload::EmptyVector
            }
        );
    }
}
