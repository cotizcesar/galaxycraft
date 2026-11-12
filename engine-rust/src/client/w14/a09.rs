#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave14/agent09.rs
// ===========================================================================
// Wave 14 translation batch 09 (staging).
//
// Window: `680000..681fff`.
//
// The window is the terrain/minimap scroll block of the 1.23.10 client, plus
// the small intrusive tree/hash containers that the band cache uses.  Fifteen
// rows carry a recoverable contract and are translated below; the remaining
// candidate rows are thunks, `noreturn` assertion tails, or mid-function
// fragments and are listed under [Skipped rows](#skipped-rows).
//
// Everything here is a free helper over values, slices and indices.  Nothing
// touches `World`, `SimUnit`, the renderer or the client, and no state is
// added to save/replay.
//
// ## Recovered layouts
//
// | location | layout |
// |----------|--------|
// | intrusive tree node | `+0x00` parent, `+0x04` left, `+0x08` right, `+0x0c` colour byte (`0` = black), `+0x10` `u16` key, `+0x14` dword-array pointer, `+0x18` element count (`-1` entries are empty slots) |
// | tree header | `+0x00` root, `+0x04` leftmost in-order node, `+0x08` rightmost in-order node |
// | hash header | `+0x00` bucket count, `+0x04` bucket-array pointer, `+0x08` element count, `+0x0c` maximum load factor (`1.0`, i.e. dword `3f800000`) |
// | hash chain node | `+0x00` next node, `+0x04` key dword, `+0x08` value |
// | terrain context | pointer `f063cc ^ 10fae560`: `+0xe0` tile column, `+0xe2` tile row, `+0xe4` width in tiles, `+0xe6` height in tiles, `+0xec` water variant flags (low three bits) |
// | scroll state | `fe5b88` x, `fe5b8c` y, `fe5b80` previous x, `fe5b84` previous y, `fe5b78` cached left column, `fe5b7c` cached top row, `fe5b4c` x limit, `fe5b50` y limit, `fe5af4`/`fe5af8` parallax wrap |
// | parallax layers | five dwords at `fe5b0c` (x) and `fe5b20` (y); speeds at `f06500` |
// | tile id table | base `11d3020`, `+0x14`, stride `0x1a` `u16` words |
//
// Bare hex names abbreviate image globals; the low bytes are the exact
// offsets.
//
// ## Skipped rows
//
// * `FUN_00680290` - once-guard around an opaque constructor plus a cleanup
//   callback; the returned object at `fe5c08` has no visible fields.
// * `FUN_006803a0` - start-up path with `unaff_EDI/ESI/EBX` register inputs
//   that reads opaque globals and ends in non-returning calls; no value
//   contract survives.
// * `FUN_006804be` - one-byte empty `ret`.
// * `FUN_00680742` - mid-function fragment over `in_EAX/in_ECX/unaff_ESI/
//   unaff_EBX` and the x87 `2^52` integer-conversion constants; no
//   self-contained signature.
// * `FUN_0068090b` - one-byte do-nothing infinite loop.
// * `FUN_006809df` - fragment writing through `unaff_EDI/in_EAX` and then
//   calling two opaque routines.
// * `FUN_00680a90` - `DECOMPILE_FAILED` (decompiler timeout).
// * `FUN_00680c30` - recursive tree destructor whose recursive and fallthrough
//   arms both end in the non-returning range-check handler; no observable
//   result.
// * `FUN_00680cb0` - SEH unwind fragment over `unaff_EBP` frame slots.
// * `FUN_00680d21` - one-byte empty `ret`.
// * `FUN_00680da0` - ten-byte tail call to the non-returning handler.
// * `FUN_00680df0` - bounds assertion over decrypted context dwords; both
//   arms end in the non-returning handler, so no value survives.
// * `FUN_00680f90` - single tail call to the non-returning handler.
// * `FUN_006811c0` - ten-byte tail call to the non-returning handler (the band
//   filler invoked by the two scroll routines).
// * `FUN_006811eb` - mid-function block over `unaff_EBP` stack arguments;
//   only a width range check remains.
// * `FUN_00681360` - five back-to-back calls into the texture layer with no
//   visible arguments or results.
// * `FUN_006819af` - second copy of the tile-id band loop with an unrecovered
//   `unaff_EBP` frame; its shared lookup/flag arithmetic is translated with
//   the sibling at `0x006819c5`.
// * `FUN_00681d90` - non-returning handler call only.

// ---------------------------------------------------------------------------
// Intrusive tree: in-order successor and insert search
// ---------------------------------------------------------------------------

/// `0x00680190`: in-order successor in the intrusive tree used by the band
/// cache destructor.
///
/// The destructor starts at the header's leftmost node and advances until it
/// reaches the header again; each visit releases the node's `+0x14` dword
/// array and zeroes the `+0x18` count before the successor step.  This helper
/// models the step itself: with a right child the successor is that subtree's
/// leftmost node, otherwise the walk climbs until the current node is not its
/// parent's right child and returns that parent.  `None` is the header, i.e.
/// the end of the traversal (the retail body compares against the header
/// pointer and rewrites the rightmost node's successor to the header).
pub fn w14a09_inorder_successor(
    left: &[Option<usize>],
    right: &[Option<usize>],
    parent: &[Option<usize>],
    node: usize,
) -> Option<usize> {
    if let Some(right_node) = right[node] {
        let mut current = right_node;
        while let Some(next) = left[current] {
            current = next;
        }
        return Some(current);
    }

    let mut current = node;
    let mut up = parent[current];
    while let Some(parent_node) = up {
        if right[parent_node] != Some(current) {
            return Some(parent_node);
        }
        current = parent_node;
        up = parent[parent_node];
    }
    None
}

/// Which link of the returned node the insert search selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W14a09InsertHint {
    /// The key already exists in the returned node.
    AtNode,
    /// The key belongs in the returned node's left child link.
    LeftChild,
    /// The key belongs in the returned node's right child link.
    RightChild,
}

/// `0x00680cd0`: unique-insert search over the intrusive tree.
///
/// The body walks the tree comparing the caller's `u16` key with the node key
/// at `+0x10`: strictly smaller keys descend left, equal and larger keys
/// descend right.  When the last visited node's key is `<=` the wanted key the
/// result is that node with [`W14a09InsertHint::AtNode`] on equality or
/// [`W14a09InsertHint::RightChild`] when the key is strictly larger.  When the
/// last node's key is larger, the inlined predecessor walk decides between
/// [`W14a09InsertHint::AtNode`] (the predecessor's key already equals or
/// exceeds the wanted key, so the predecessor is the lower bound) and
/// [`W14a09InsertHint::LeftChild`].  A node equal to `leftmost` skips the
/// predecessor check, exactly as the retail body's `header->left` comparison
/// does.
///
/// `keys`, `left`, `right` and `parent` describe the same nodes; `root` is the
/// header's `+0x00` link.  `None` is returned only for an empty tree, where the
/// retail body returns the header and its left link.
pub fn w14a09_find_insert_position(
    keys: &[u16],
    left: &[Option<usize>],
    right: &[Option<usize>],
    parent: &[Option<usize>],
    leftmost: usize,
    root: Option<usize>,
    key: u16,
) -> Option<(usize, W14a09InsertHint)> {
    let mut where_node = None;
    if let Some(mut node) = root {
        loop {
            where_node = Some(node);
            let next = if key < keys[node] {
                left[node]
            } else {
                right[node]
            };
            match next {
                Some(child) => node = child,
                None => break,
            }
        }

        let leaf = where_node.expect("root is some");
        let leaf_key = keys[leaf];
        if leaf_key <= key {
            let hint = if leaf_key < key {
                W14a09InsertHint::RightChild
            } else {
                W14a09InsertHint::AtNode
            };
            return Some((leaf, hint));
        }
    }

    let node = where_node?;
    if node != leftmost
        && let Some(pred) = w14a09_inorder_predecessor(left, right, parent, node)
        && key <= keys[pred]
    {
        return Some((pred, W14a09InsertHint::AtNode));
    }
    Some((node, W14a09InsertHint::LeftChild))
}

/// In-order predecessor, the inlined `_Prev` walk of `0x00680cd0`.
///
/// `None` means "no predecessor" (the header's rightmost link in the retail
/// body); the caller only uses the result when the node is not the leftmost
/// one, where a predecessor always exists.
fn w14a09_inorder_predecessor(
    left: &[Option<usize>],
    right: &[Option<usize>],
    parent: &[Option<usize>],
    node: usize,
) -> Option<usize> {
    if let Some(left_node) = left[node] {
        let mut current = left_node;
        while let Some(next) = right[current] {
            current = next;
        }
        return Some(current);
    }

    let mut current = node;
    let mut up = parent[current];
    while let Some(parent_node) = up {
        if right[parent_node] == Some(current) {
            return Some(parent_node);
        }
        current = parent_node;
        up = parent[parent_node];
    }
    None
}

// ---------------------------------------------------------------------------
// Hash header and checked field
// ---------------------------------------------------------------------------

/// Bucket count written by the lazy initialiser of `0x00680210`.
pub const W14A09_HASH_INITIAL_BUCKETS: u32 = 8;

/// Maximum load factor written at header `+0x0c` (`1.0f`).
pub const W14A09_HASH_LOAD_FACTOR: f32 = 1.0;

/// Byte offset of the value inside one hash chain node.
pub const W14A09_HASH_VALUE_OFFSET: usize = 8;

/// First hash multiplier (`-0x7a143595` as an unsigned dword).
pub const W14A09_HASH_MULTIPLIER_1: u32 = 0x85eb_ca6b;

/// Second hash multiplier (`-0x3d4d51cb` as an unsigned dword).
pub const W14A09_HASH_MULTIPLIER_2: u32 = 0xc2b2_ae35;

/// `0x00680210`: initial contents of the four-dword hash header.
///
/// The retail body is a thread-local once-guard: when the guard says the
/// object at `fe5bf4` is uninitialised it writes `8` at `+0x00`, `0` at
/// `+0x04` and `+0x08`, `0x3f800000` (`1.0f`) at `+0x0c`, registers the
/// object's cleanup callback and returns its address.  The guard and the
/// callback registration are not modelled; the initial field values are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct W14a09HashHeader {
    /// `+0x00`: bucket count; the lookup masks with `bucket_count - 1`.
    pub bucket_count: u32,
    /// `+0x04`: bucket-array pointer, zero until the first insertion.
    pub buckets: u32,
    /// `+0x08`: element count.
    pub elements: u32,
    /// `+0x0c`: maximum load factor (`1.0`).
    pub load_factor: f32,
}

/// `0x00680210`: the header values installed by the lazy initialiser.
pub fn w14a09_hash_header_init() -> W14a09HashHeader {
    W14a09HashHeader {
        bucket_count: W14A09_HASH_INITIAL_BUCKETS,
        buckets: 0,
        elements: 0,
        load_factor: W14A09_HASH_LOAD_FACTOR,
    }
}

// ---------------------------------------------------------------------------
// Checked field read
// ---------------------------------------------------------------------------

/// Value returned by the `+0x18` getter when the field is unset.
pub const W14A09_INVALID_FIELD: i32 = -1;

/// `0x00680330`: read the signed dword at `this + 0x18`.
///
/// The retail body checks the field against `-1` and, when it matches, calls
/// the non-returning handler; otherwise it copies the field to the caller's
/// output.  The helper returns `None` for that sentinel and the value
/// otherwise, leaving the load to the caller.
pub fn w14a09_checked_field(value: i32) -> Option<i32> {
    if value == W14A09_INVALID_FIELD {
        None
    } else {
        Some(value)
    }
}

// ---------------------------------------------------------------------------
// Hash bucket lookup
// ---------------------------------------------------------------------------

/// `0x006806a0`: the two-round hash of one lookup key.
///
/// Both rounds fold the high half into the low half before multiplying, and
/// the final fold is masked with `bucket_count - 1` by the caller.  The
/// multipliers are recovered dwords and the shift counts are `16`, `13` and
/// `16`.
pub fn w14a09_hash_key(key: u32) -> u32 {
    let mut value = (key >> 16) ^ key;
    value = value.wrapping_mul(W14A09_HASH_MULTIPLIER_1);
    value = (value >> 13) ^ value;
    value = value.wrapping_mul(W14A09_HASH_MULTIPLIER_2);
    (value >> 16) ^ value
}

/// `0x006806a0`: bucket index for `key` in a `bucket_count`-bucket table.
///
/// The retail body requires a non-null bucket array and masks with
/// `*piVar2 - 1`, i.e. the bucket count is a power of two (eight in the
/// initialiser above).  A zero bucket count is not representable in the
/// retail body either.
pub fn w14a09_hash_bucket_index(key: u32, bucket_count: u32) -> usize {
    (w14a09_hash_key(key) & bucket_count.wrapping_sub(1)) as usize
}

/// `0x006806a0`: follow one bucket chain to the node whose key matches.
///
/// `head` is the bucket's first node, `next[i]` is the node's `+0x00` link and
/// `keys[i]` its `+0x04` dword.  The match returns the node index; the value
/// lives at `node + 8`.  A miss (the retail body reaches a null `next` and
/// calls the non-returning handler) is `None`.
pub fn w14a09_hash_find(
    keys: &[u32],
    next: &[Option<usize>],
    head: Option<usize>,
    key: u32,
) -> Option<usize> {
    let mut current = head;
    while let Some(node) = current {
        if keys[node] == key {
            return Some(node);
        }
        current = next[node];
    }
    None
}

// ---------------------------------------------------------------------------
// Presence scan and selection predicate
// ---------------------------------------------------------------------------

/// `0x00680a00`: linear membership over the dword vector at `fe5ba0`
/// (`fe5ba4` elements).
///
/// The retail body walks `piVar3` from the begin pointer to
/// `begin + count` and stops on the first equal dword; the boolean it keeps is
/// inverted later, so this helper returns the plain "found" answer.
pub fn w14a09_contains_i32(values: &[i32], wanted: i32) -> bool {
    values.contains(&wanted)
}

/// `0x00680a00`: the compound visibility predicate around the presence scan.
///
/// `present` is [`w14a09_contains_i32`]'s result.  The body then tests two
/// option bytes: with `fe5b28` non-zero and the out-of-window query reporting
/// true it returns `true` immediately; with `fe5b2c` non-zero it queries again
/// and returns `!query || present`; otherwise it returns `present`.  `probe` is
/// the query and keeps the retail body's lazy evaluation, including the second
/// call on the `fe5b2c` path.
pub fn w14a09_selection_predicate<F: FnMut() -> bool>(
    present: bool,
    primary_flag: bool,
    secondary_flag: bool,
    mut probe: F,
) -> bool {
    if primary_flag && probe() {
        return true;
    }
    if secondary_flag {
        return !probe() || present;
    }
    present
}

// ---------------------------------------------------------------------------
// Water/noise texture variant
// ---------------------------------------------------------------------------

/// Eight-entry selector indexed by the low three bits of terrain `+0xec`.
pub const W14A09_WATER_VARIANTS: [u8; 8] = [0, 2, 2, 1, 0, 2, 0, 0];

/// Texture pair installed by the water initialiser at `0x006813a0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W14a09WaterTextures {
    /// Both normal-water maps (`effect/water_normal_2.dds.grp` and
    /// `effect/water_normal_1.dds.grp`).
    Water,
    /// The noise map (`effect/noise.dds`).
    Noise,
    /// No texture load for this variant (selector value `2`).
    None,
}

/// `0x006813a0`: choose the water texture set for the map's variant flags.
///
/// The selector is `(*(u16 *)(context + 0xec)) & 7` and the recovered table is
/// [`W14A09_WATER_VARIANTS`].  A table value of `0` loads the two normal water
/// maps, `1` loads the noise map and `2` loads nothing.  The surrounding body
/// also creates the two surface banks and fills the `0x400`/`0x40`-entry id
/// arrays with `-1`; only the variant decision is modelled here.
pub fn w14a09_water_texture_variant(terrain_flags: u16) -> W14a09WaterTextures {
    match W14A09_WATER_VARIANTS[(terrain_flags & 7) as usize] {
        0 => W14a09WaterTextures::Water,
        1 => W14a09WaterTextures::Noise,
        _ => W14a09WaterTextures::None,
    }
}

// ---------------------------------------------------------------------------
// Tile flag validation scan
// ---------------------------------------------------------------------------

/// `0x006815b0`: per-tile bitmask predicate of the terrain validation loop.
///
/// For a tile reported special by the out-of-window per-tile predicate the
/// low byte must contain every bit of `all_bits` (`(all_bits & !low_byte) ==
/// 0`).  For every other tile at least one bit of `any_bits` must be present
/// (`(any_bits & flags) != 0`).  The masks are the two globals read by the
/// body (`119f7c0` and `11a4578`).
pub fn w14a09_tile_flags_ok(flags: u32, special: bool, all_bits: u32, any_bits: u32) -> bool {
    if special {
        (all_bits & !(flags & 0xff)) == 0
    } else {
        (any_bits & flags) != 0
    }
}

/// `0x006815b0`: scan the whole `width * height` tile array and return the
/// first failing tile index.
///
/// The retail loop runs the recovered tile count down to zero and fails on
/// the first tile that violates [`w14a09_tile_flags_ok`]; the column counter
/// it maintains wraps at the map width but is not read again inside the
/// visible body.  `None` means every tile passed (the retail body returns
/// without reaching the non-returning handler).
pub fn w14a09_scan_tile_flags(
    tiles: &[u32],
    special: &[bool],
    all_bits: u32,
    any_bits: u32,
) -> Option<usize> {
    for (index, (&flags, &is_special)) in tiles.iter().zip(special.iter()).enumerate() {
        if !w14a09_tile_flags_ok(flags, is_special, all_bits, any_bits) {
            return Some(index);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Pixel extents and visible row count
// ---------------------------------------------------------------------------

/// Tile-to-pixel shift recovered from the map-size setup.
pub const W14A09_TILE_SHIFT: u32 = 5;

/// `0x00681750`: the two pixel extents of the tile map.
///
/// The body sign-extends the 16-bit tile counts at context `+0xe4`/`+0xe6`
/// and shifts each left by five (32 pixels per tile) before storing them at
/// `fe5b58`, `fe5b5a`, `fe5b5c` and `fe5b60`.  Tiles counts above `0x7fff`
/// therefore produce negative extents, matching the retail sign extension.
pub fn w14a09_tile_extent_to_pixels(width_tiles: u16, height_tiles: u16) -> (i32, i32) {
    (
        ((width_tiles as i16 as i32) << W14A09_TILE_SHIFT),
        ((height_tiles as i16 as i32) << W14A09_TILE_SHIFT),
    )
}

/// `0x00681900`: number of 32-pixel rows covered by `screen_px`, plus one.
///
/// The body computes
/// `((screen_px + 31 + (((screen_px + 31) >> 31) & 31)) >> 5) + 1`, the signed
/// ceiling division by 32 with one extra row, and then runs a bounds assertion
/// that only ends in the non-returning handler.  Only the recovered count is
/// modelled; it is also the loop bound used by the band refresh at
/// `0x00681ad0` and `0x00681e70`.
pub fn w14a09_visible_rows(screen_px: i32) -> i32 {
    let biased = screen_px.wrapping_add(0x1f);
    ((biased.wrapping_add((biased >> 31) & 0x1f)) >> 5).wrapping_add(1)
}

// ---------------------------------------------------------------------------
// Tile id lookup and flag update
// ---------------------------------------------------------------------------

/// `u16` stride of one entry group in the tile id table.
pub const W14A09_TILE_LOOKUP_STRIDE: usize = 0x1a;

/// Word index of the table's `+0x14` byte bias.
pub const W14A09_TILE_LOOKUP_BIAS: usize = 0x14 / 2;

/// Bit 30 of the parallel flag dword, raised when the out-of-window coverage
/// query at `983b60` reports non-zero.
pub const W14A09_TILE_EXTENT_BIT: u32 = 0x4000_0000;

/// Mask that clears the extent bit before it is rewritten.
pub const W14A09_TILE_EXTENT_CLEAR: u32 = 0xbfff_ffff;

/// Mask applied to the destination id during the "did it change" test.
pub const W14A09_TILE_COMPARE_MASK: u32 = 0x7fff;

/// `0x006819c5`: look one tile id up in the `0x1a`-stride table at `11d3020`.
///
/// The word index is `10 + ((tile >> 4 & 0x7ff) * 0x1a + (tile & 0xf))`: the
/// body adds the table's `+0x14` byte bias and scales the two-part index by
/// two.  `table` must therefore cover `10 + 0x7ff * 0x1a + 0xf` words.
pub fn w14a09_lookup_tile_id(table: &[u16], tile: u16) -> u16 {
    let index = ((tile as usize >> 4) & 0x7ff) * W14A09_TILE_LOOKUP_STRIDE + (tile as usize & 0xf);
    table[W14A09_TILE_LOOKUP_BIAS + index]
}

/// Result of one tile-id band step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14a09TileIdUpdate {
    /// Whether the looked-up id differs from the destination's low 15 bits.
    pub changed: bool,
    /// The looked-up id, to be stored into the parallel `u16` array.
    pub id: u16,
    /// Updated flag dword for the parallel `u32` array.
    pub flags: u32,
}

/// `0x006819c5`: compare and apply one tile id from the lookup table.
///
/// The looked-up id is compared with the destination's low 15 bits
/// (`*dst & 0x7fff`).  On a match nothing is written.  Otherwise the id is
/// stored into the `u16` array, bit 30 of the separate flag dword is cleared
/// and re-raised only when the out-of-window coverage query reports true, and
/// the caller refreshes the tile through the band call that follows.  The
/// flag dword's other bits, including its own low 16 bits, are preserved.
pub fn w14a09_update_tile_id(
    flags: u32,
    destination: u16,
    new_id: u16,
    has_extent: bool,
) -> W14a09TileIdUpdate {
    if new_id == (u32::from(destination) & W14A09_TILE_COMPARE_MASK) as u16 {
        return W14a09TileIdUpdate {
            changed: false,
            id: new_id,
            flags,
        };
    }

    let mut updated = flags & W14A09_TILE_EXTENT_CLEAR;
    if has_extent {
        updated |= W14A09_TILE_EXTENT_BIT;
    }
    W14a09TileIdUpdate {
        changed: true,
        id: new_id,
        flags: updated,
    }
}

// ---------------------------------------------------------------------------
// Scroll state and band bookkeeping
// ---------------------------------------------------------------------------

/// Number of parallax layers advanced by every scroll routine.
pub const W14A09_LAYER_COUNT: usize = 5;

/// The recovered scroll globals.
///
/// `x`/`y` are `fe5b88`/`fe5b8c`, the previous request is `fe5b80`/`fe5b84`,
/// the tile coordinates are context `+0xe0`/`+0xe2` and the cached band origin
/// is `fe5b78`/`fe5b7c`.  Offsets are in map pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a09ScrollState {
    /// `fe5b88`: current horizontal offset in map pixels.
    pub x: i32,
    /// `fe5b8c`: current vertical offset in map pixels.
    pub y: i32,
    /// `fe5b80`: horizontal offset of the previous update.
    pub prev_x: i32,
    /// `fe5b84`: vertical offset of the previous update.
    pub prev_y: i32,
    /// Context `+0xe0`: current tile column (`x >> 5`).
    pub tile_x: u16,
    /// Context `+0xe2`: current tile row (`y >> 5`).
    pub tile_y: u16,
    /// `fe5b78`: leftmost tile column of the cached band.
    pub last_tile_x: u16,
    /// `fe5b7c`: topmost tile row of the cached band.
    pub last_tile_y: u16,
}

/// The five parallax layer offsets stored at `fe5b0c`/`fe5b20`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a09Parallax {
    /// Horizontal offsets, one per layer.
    pub offset_x: [i32; W14A09_LAYER_COUNT],
    /// Vertical offsets, one per layer.
    pub offset_y: [i32; W14A09_LAYER_COUNT],
}

/// Outcome of an absolute scroll request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a09ScrollResult {
    /// Whether the clamped position moved (the layer offsets were advanced).
    pub scrolled: bool,
    /// Whether the tile coordinate crossed out of the cached band.
    pub tile_changed: bool,
    /// Rows the retail body would hand to the band filler (`0` when the tile
    /// coordinate stayed inside the cached band).
    pub refresh_rows: i32,
}

/// `0x00681ad0`: clamp an absolute scroll request and advance the parallax.
///
/// The request is clamped to `[0, limit_x]`/`[0, limit_y]` (the limit itself is
/// reachable, as in the retail `limit <= value` upper test), then converted to
/// tile coordinates with an arithmetic `>> 5`.  When the clamped position
/// differs from the previous one the five layer offsets advance by
/// `(velocity * (previous - new) + offset) % wrap`, using the previous request
/// stored in the state.  The tile-band refresh that follows in the retail body
/// calls the non-returning filler at `6811c0`; its row count is returned
/// instead.
///
/// The two leading out-of-window guards (the `7744b0`/`7490d0` pair) are the
/// caller's responsibility; `wrap_x`/`wrap_y` must be non-zero because the
/// retail body divides by them directly.
#[allow(clippy::too_many_arguments)]
pub fn w14a09_scroll_to(
    state: &mut W14a09ScrollState,
    layers: &mut W14a09Parallax,
    target_x: i32,
    target_y: i32,
    limit_x: i32,
    limit_y: i32,
    velocity_x: &[i32; W14A09_LAYER_COUNT],
    velocity_y: &[i32; W14A09_LAYER_COUNT],
    wrap_x: i32,
    wrap_y: i32,
    viewport_height_px: i32,
) -> W14a09ScrollResult {
    let x = if target_x < 0 {
        0
    } else if target_x >= limit_x {
        limit_x
    } else {
        target_x
    };
    let y = if target_y < 0 {
        0
    } else if target_y >= limit_y {
        limit_y
    } else {
        target_y
    };

    state.tile_x = (x >> 5) as u16;
    state.x = x;
    state.tile_y = (y >> 5) as u16;
    state.y = y;

    if x == state.prev_x && y == state.prev_y {
        return W14a09ScrollResult::default();
    }

    let delta_x = state.prev_x - x;
    let delta_y = state.prev_y - y;
    for layer in 0..W14A09_LAYER_COUNT {
        layers.offset_x[layer] = (velocity_x[layer] * delta_x + layers.offset_x[layer]) % wrap_x;
        layers.offset_y[layer] = (velocity_y[layer] * delta_y + layers.offset_y[layer]) % wrap_y;
    }

    state.prev_x = x;
    state.prev_y = y;

    let tile_changed = state.tile_x != state.last_tile_x || state.tile_y != state.last_tile_y;
    if tile_changed {
        state.last_tile_y = state.tile_y;
        state.last_tile_x = state.tile_x;
    }

    W14a09ScrollResult {
        scrolled: true,
        tile_changed,
        refresh_rows: if tile_changed {
            w14a09_visible_rows(viewport_height_px)
        } else {
            0
        },
    }
}

/// `0x00681cc0`: recompute the vertical scroll limit.
///
/// The body adds the map height in pixels (`fe5b60`) to the converted
/// viewport/screen term (`fe5b64`) and subtracts the current screen height
/// (`iVar1` from the out-of-window query), storing the wrapped unsigned result
/// at `fe5b50` before the non-returning tail.  The two value-producing calls
/// themselves are not modelled.
pub fn w14a09_vertical_scroll_limit(map_height_px: u32, viewport_px: u32, current_px: u32) -> u32 {
    map_height_px
        .wrapping_add(viewport_px)
        .wrapping_sub(current_px)
}

/// Outcome of a relative vertical scroll.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a09VerticalScroll {
    /// Whether the position moved (the layer offsets were advanced).
    pub scrolled: bool,
    /// Rows the retail body would hand to the band filler.
    pub rows_to_refresh: i32,
}

/// `0x00681e70`: scroll down by `delta_px` scaled through the frame factor.
///
/// The body clamps the scale to at least `1.0`, divides the requested pixel
/// delta by it with a truncating float-to-int conversion, adds the result to
/// the vertical offset and, when the limit is crossed, folds the overshoot
/// back into the effective delta before clamping.  Moving the position updates
/// the five layer offsets by `offset - velocity * delta`, adding the wrap value
/// once when the result is negative.  Rows between the cached band origin and
/// the new tile row are refreshed by the non-returning filler at `6811c0`;
/// their count is returned and the band origin is advanced.
pub fn w14a09_scroll_down(
    state: &mut W14a09ScrollState,
    layers: &mut W14a09Parallax,
    delta_px: i32,
    limit_y: i32,
    velocity_y: &[i32; W14A09_LAYER_COUNT],
    wrap_y: i32,
    factor: f32,
) -> W14a09VerticalScroll {
    let scale = if factor <= 1.0 { 1.0 } else { factor };
    let mut delta = (delta_px as f32 / scale) as i32;

    state.y = state.y.wrapping_add(delta);
    if limit_y < state.y {
        delta += limit_y - state.y;
        state.y = limit_y;
    }
    state.tile_y = ((state.y as u32) >> 5) as u16;

    if state.prev_y == state.y {
        return W14a09VerticalScroll::default();
    }
    state.prev_y = state.y;

    for (layer, &velocity) in velocity_y.iter().enumerate() {
        let mut value = layers.offset_y[layer] - velocity * delta;
        if value < 0 {
            value += wrap_y;
        }
        layers.offset_y[layer] = value;
    }

    if state.last_tile_y != state.tile_y {
        let rows = i32::from(state.tile_y) - i32::from(state.last_tile_y);
        state.last_tile_y = state.tile_y;
        return W14a09VerticalScroll {
            scrolled: true,
            rows_to_refresh: rows,
        };
    }
    W14a09VerticalScroll {
        scrolled: true,
        rows_to_refresh: 0,
    }
}

/// Outcome of a relative horizontal scroll.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a09HorizontalScroll {
    /// Whether the position moved (the layer offsets were advanced).
    pub scrolled: bool,
    /// Columns the retail body would hand to the band filler.
    pub columns_to_refresh: i32,
}

/// `0x00681ff0`: scroll left by `delta_px` scaled through the frame factor.
///
/// The mirror of the vertical routine: the scaled delta is subtracted from the
/// horizontal offset, an underflow at zero is folded back into the effective
/// delta before clamping, and the five layer offsets advance by
/// `offset + velocity * delta`, subtracting the wrap value once when the result
/// is larger.  When the tile column leaves the cached band the body walks the
/// band origin down to the new column through the non-returning filler at
/// `680df0`; the column count is returned and the origin is moved.
pub fn w14a09_scroll_left(
    state: &mut W14a09ScrollState,
    layers: &mut W14a09Parallax,
    delta_px: i32,
    velocity_x: &[i32; W14A09_LAYER_COUNT],
    wrap_x: i32,
    factor: f32,
) -> W14a09HorizontalScroll {
    let scale = if factor <= 1.0 { 1.0 } else { factor };
    let mut delta = (delta_px as f32 / scale) as i32;

    state.x = state.x.wrapping_sub(delta);
    if state.x < 0 {
        delta += state.x;
        state.x = 0;
    }
    state.tile_x = ((state.x as u32) >> 5) as u16;

    if state.prev_x == state.x {
        return W14a09HorizontalScroll::default();
    }
    state.prev_x = state.x;

    for (layer, &velocity) in velocity_x.iter().enumerate() {
        let mut value = layers.offset_x[layer] + velocity * delta;
        if wrap_x < value {
            value -= wrap_x;
        }
        layers.offset_x[layer] = value;
    }

    if state.last_tile_x != state.tile_x {
        let columns = i32::from(state.last_tile_x) - i32::from(state.tile_x);
        state.last_tile_x = state.tile_x;
        return W14a09HorizontalScroll {
            scrolled: true,
            columns_to_refresh: columns,
        };
    }
    W14a09HorizontalScroll {
        scrolled: true,
        columns_to_refresh: 0,
    }
}

#[cfg(test)]
mod tests_wave14_agent09 {
    use super::*;
    use std::cell::Cell;

    // -- 0x00680190 --------------------------------------------------------

    /// `(keys, left, right, parent)` for the seven-node fixture tree.
    type W14a09TreeFixture = (
        [u16; 7],
        [Option<usize>; 7],
        [Option<usize>; 7],
        [Option<usize>; 7],
    );

    /// A seven-node tree: root `50`, `30` with `20`/`40`, `70` with `60`/`80`.
    fn sample_tree() -> W14a09TreeFixture {
        let keys = [50, 30, 70, 20, 40, 60, 80];
        let left = [Some(1), Some(3), Some(5), None, None, None, None];
        let right = [Some(2), Some(4), Some(6), None, None, None, None];
        let parent = [None, Some(0), Some(0), Some(1), Some(1), Some(2), Some(2)];
        (keys, left, right, parent)
    }

    #[test]
    fn successor_walks_the_whole_tree_in_order() {
        let (_, left, right, parent) = sample_tree();
        let order = [3usize, 1, 4, 0, 5, 2, 6];

        for pair in order.windows(2) {
            assert_eq!(
                w14a09_inorder_successor(&left, &right, &parent, pair[0]),
                Some(pair[1]),
            );
        }
        assert_eq!(
            w14a09_inorder_successor(&left, &right, &parent, order[6]),
            None,
        );
    }

    #[test]
    fn successor_starts_at_the_right_subtrees_leftmost_node() {
        let (_, left, right, parent) = sample_tree();
        // 50 -> right child 70 -> left child 60.
        assert_eq!(w14a09_inorder_successor(&left, &right, &parent, 0), Some(5),);
        // 40 has no right child, so it climbs back to 50.
        assert_eq!(w14a09_inorder_successor(&left, &right, &parent, 4), Some(0),);
    }

    // -- 0x00680cd0 --------------------------------------------------------

    #[test]
    fn insert_search_selects_the_node_or_the_child_link() {
        let (keys, left, right, parent) = sample_tree();
        let find =
            |key| w14a09_find_insert_position(&keys, &left, &right, &parent, 3, Some(0), key);

        assert_eq!(find(50), Some((0, W14a09InsertHint::AtNode)));
        assert_eq!(find(30), Some((1, W14a09InsertHint::AtNode)));
        assert_eq!(find(20), Some((3, W14a09InsertHint::AtNode)));
        assert_eq!(find(80), Some((6, W14a09InsertHint::AtNode)));
        // Below the leftmost node: insert into its left link.
        assert_eq!(find(10), Some((3, W14a09InsertHint::LeftChild)));
        assert_eq!(find(35), Some((4, W14a09InsertHint::LeftChild)));
        assert_eq!(find(45), Some((4, W14a09InsertHint::RightChild)));
        assert_eq!(find(65), Some((5, W14a09InsertHint::RightChild)));
        assert_eq!(find(75), Some((6, W14a09InsertHint::LeftChild)));
        assert_eq!(find(90), Some((6, W14a09InsertHint::RightChild)));
        assert_eq!(find(55), Some((5, W14a09InsertHint::LeftChild)));
        // An empty tree is the header case and has no slice representation.
        assert_eq!(
            w14a09_find_insert_position(&keys, &left, &right, &parent, 3, None, 42),
            None,
        );
    }

    #[test]
    fn insert_search_finds_a_duplicate_through_the_predecessor() {
        // 30 -> right 50 -> left 40.  Looking up 30 descends to 40 and the
        // predecessor walk returns the duplicate at 30.
        let keys = [30u16, 50, 40];
        let left = [None, Some(2), None];
        let right = [Some(1), None, None];
        let parent = [None, Some(0), Some(1)];

        assert_eq!(
            w14a09_find_insert_position(&keys, &left, &right, &parent, 0, Some(0), 30),
            Some((0, W14a09InsertHint::AtNode)),
        );
        assert_eq!(
            w14a09_find_insert_position(&keys, &left, &right, &parent, 0, Some(0), 35),
            Some((2, W14a09InsertHint::LeftChild)),
        );
        assert_eq!(
            w14a09_find_insert_position(&keys, &left, &right, &parent, 0, Some(0), 45),
            Some((2, W14a09InsertHint::RightChild)),
        );
    }

    // -- 0x00680210 --------------------------------------------------------

    #[test]
    fn hash_header_matches_the_initialiser_words() {
        let header = w14a09_hash_header_init();
        assert_eq!(header.bucket_count, 8);
        assert_eq!(header.buckets, 0);
        assert_eq!(header.elements, 0);
        assert_eq!(header.load_factor, 1.0);
        assert_eq!(header.load_factor.to_bits(), 0x3f80_0000);
    }

    // -- 0x00680330 --------------------------------------------------------

    #[test]
    fn checked_field_rejects_the_all_ones_sentinel() {
        assert_eq!(w14a09_checked_field(0), Some(0));
        assert_eq!(w14a09_checked_field(-1), None);
        assert_eq!(w14a09_checked_field(0x7fff_ffff), Some(0x7fff_ffff));
        assert_eq!(w14a09_checked_field(i32::MIN), Some(i32::MIN));
    }

    // -- 0x006806a0 --------------------------------------------------------

    #[test]
    fn hash_key_matches_the_two_recovered_mixing_rounds() {
        assert_eq!(w14a09_hash_key(0), 0);
        assert_eq!(w14a09_hash_key(1), 0x514e_28b7);
        assert_eq!(w14a09_hash_key(2), 0x30f4_c306);
        assert_eq!(w14a09_hash_key(0x10), 0x5525_53bc);
        assert_eq!(w14a09_hash_key(0x1234_5678), 0xe37c_d1bc);
        assert_eq!(w14a09_hash_key(0xffff_ffff), 0x81f1_6f39);
    }

    #[test]
    fn hash_bucket_masks_with_the_bucket_count() {
        assert_eq!(w14a09_hash_bucket_index(0x1234_5678, 16), 12);
        assert_eq!(w14a09_hash_bucket_index(0x1234_5678, 32), 28);
        // One bucket degenerates to the zero mask.
        assert_eq!(w14a09_hash_bucket_index(0xffff_ffff, 1), 0);
    }

    #[test]
    fn hash_find_walks_the_bucket_chain_to_the_matching_key() {
        let keys = [7u32, 9, 11];
        let next = [Some(1), Some(2), None];

        assert_eq!(w14a09_hash_find(&keys, &next, Some(0), 7), Some(0));
        assert_eq!(w14a09_hash_find(&keys, &next, Some(0), 11), Some(2));
        assert_eq!(w14a09_hash_find(&keys, &next, Some(0), 13), None);
        assert_eq!(w14a09_hash_find(&keys, &next, None, 7), None);
        assert_eq!(W14A09_HASH_VALUE_OFFSET, 8);
    }

    // -- 0x00680a00 --------------------------------------------------------

    #[test]
    fn presence_scan_compares_signed_dwords() {
        let values = [-2, -1, 0, 0x7fff_ffff];
        assert!(w14a09_contains_i32(&values, -1));
        assert!(w14a09_contains_i32(&values, 0x7fff_ffff));
        assert!(!w14a09_contains_i32(&values, 1));
        assert!(!w14a09_contains_i32(&[], 0));
    }

    #[test]
    fn selection_predicate_follows_the_two_option_bytes() {
        // Neither option byte set: the plain presence answer.
        assert!(w14a09_selection_predicate(true, false, false, || {
            panic!("the query must not run")
        }));
        assert!(!w14a09_selection_predicate(false, false, false, || {
            panic!("the query must not run")
        }));

        // Primary byte set: a true query wins, a false one falls through.
        assert!(w14a09_selection_predicate(false, true, false, || true));
        assert!(w14a09_selection_predicate(true, true, false, || false));
        assert!(!w14a09_selection_predicate(false, true, false, || false));

        // Secondary byte set: a false query exposes the presence answer.
        assert!(!w14a09_selection_predicate(false, false, true, || true));
        assert!(w14a09_selection_predicate(false, false, true, || false));
        assert!(w14a09_selection_predicate(true, false, true, || true));
    }

    #[test]
    fn selection_predicate_queries_lazily_and_twice_on_the_secondary_path() {
        let calls = Cell::new(0);
        let predicate = w14a09_selection_predicate(false, true, true, || {
            calls.set(calls.get() + 1);
            false
        });
        // The primary query missed, so the secondary path runs and its false
        // query exposes the (absent) presence answer as true.
        assert!(predicate);
        assert_eq!(calls.get(), 2);

        let calls = Cell::new(0);
        let predicate = w14a09_selection_predicate(false, true, false, || {
            calls.set(calls.get() + 1);
            true
        });
        assert!(predicate);
        assert_eq!(calls.get(), 1);
    }

    // -- 0x006813a0 --------------------------------------------------------

    #[test]
    fn water_variant_uses_the_low_three_bits() {
        for flags in [0u16, 4, 6, 7, 8, 0x0100, 0x0104] {
            assert_eq!(
                w14a09_water_texture_variant(flags),
                W14a09WaterTextures::Water,
                "flags {flags:#06x}",
            );
        }
        assert_eq!(w14a09_water_texture_variant(3), W14a09WaterTextures::Noise,);
        assert_eq!(
            w14a09_water_texture_variant(0x0103),
            W14a09WaterTextures::Noise,
        );
        for flags in [1u16, 2, 5, 9, 0x0101] {
            assert_eq!(
                w14a09_water_texture_variant(flags),
                W14a09WaterTextures::None,
                "flags {flags:#06x}",
            );
        }
        assert_eq!(W14A09_WATER_VARIANTS, [0, 2, 2, 1, 0, 2, 0, 0]);
    }

    // -- 0x006815b0 --------------------------------------------------------

    #[test]
    fn tile_flags_require_all_bits_for_special_tiles() {
        let all = 0x0000_00f0;
        let any = 0x8000_0000;

        assert!(w14a09_tile_flags_ok(0x0000_00f0, true, all, any));
        // Bit 0x10 missing.
        assert!(!w14a09_tile_flags_ok(0x0000_00e0, true, all, any));
        // Bits above the low byte do not satisfy the special test.
        assert!(!w14a09_tile_flags_ok(0xffff_ff0f, true, all, any));

        assert!(w14a09_tile_flags_ok(0x8000_0001, false, all, any));
        assert!(!w14a09_tile_flags_ok(0x0000_00f0, false, all, any));
    }

    #[test]
    fn scan_reports_the_first_failing_tile() {
        let all = 0x0000_00ff;
        let any = 0x4000_0000;
        let tiles = [0x4000_0000, 0x0000_0000, 0x4000_0000];
        let special = [false, false, false];
        assert_eq!(w14a09_scan_tile_flags(&tiles, &special, all, any), Some(1));

        let tiles = [0x4000_0000, 0x4000_0000];
        assert_eq!(
            w14a09_scan_tile_flags(&tiles, &special[..2], all, any),
            None
        );

        // Empty map: the retail counter starts at zero and returns at once.
        assert_eq!(w14a09_scan_tile_flags(&[], &[], all, any), None,);

        // A special tile with an incomplete mask fails in the other arm.
        let tiles = [0x0000_0003];
        let special = [true];
        assert_eq!(
            w14a09_scan_tile_flags(&tiles, &special, 0x0000_000f, any),
            Some(0),
        );
    }

    // -- 0x00681750 --------------------------------------------------------

    #[test]
    fn tile_extents_shift_by_five_and_sign_extend() {
        assert_eq!(w14a09_tile_extent_to_pixels(0, 0), (0, 0));
        assert_eq!(w14a09_tile_extent_to_pixels(1, 1), (32, 32));
        assert_eq!(w14a09_tile_extent_to_pixels(64, 32), (2048, 1024));
        assert_eq!(
            w14a09_tile_extent_to_pixels(0x7fff, 0x8000),
            (0x0f_ffe0, -0x10_0000),
        );
        assert_eq!(W14A09_TILE_SHIFT, 5);
    }

    // -- 0x00681900 --------------------------------------------------------

    #[test]
    fn visible_rows_rounds_up_after_adding_one_row() {
        assert_eq!(w14a09_visible_rows(0), 1);
        assert_eq!(w14a09_visible_rows(1), 2);
        assert_eq!(w14a09_visible_rows(31), 2);
        assert_eq!(w14a09_visible_rows(32), 2);
        assert_eq!(w14a09_visible_rows(33), 3);
        assert_eq!(w14a09_visible_rows(64), 3);
        assert_eq!(w14a09_visible_rows(-1), 1);
        assert_eq!(w14a09_visible_rows(-32), 1);
        assert_eq!(w14a09_visible_rows(-64), 0);
    }

    // -- 0x006819c5 --------------------------------------------------------

    #[test]
    fn tile_lookup_uses_the_recovered_stride_and_bias() {
        let mut table = vec![0u16; W14A09_TILE_LOOKUP_BIAS + 0x7ff * 0x1a + 0xf + 1];
        table[10] = 0x1234;
        table[11] = 0x2345;
        table[36] = 0x3456;
        table[25] = 0x4567;
        table[10 + 0x7ff * 0x1a + 0xf] = 0x5678;

        assert_eq!(w14a09_lookup_tile_id(&table, 0), 0x1234);
        assert_eq!(w14a09_lookup_tile_id(&table, 1), 0x2345);
        assert_eq!(w14a09_lookup_tile_id(&table, 0x10), 0x3456);
        assert_eq!(w14a09_lookup_tile_id(&table, 0xf), 0x4567);
        // Bit 15 is outside the 11-bit group and the low nibble stays.
        assert_eq!(w14a09_lookup_tile_id(&table, 0x8001), 0x2345);
        assert_eq!(w14a09_lookup_tile_id(&table, 0xffff), 0x5678);
    }

    #[test]
    fn tile_update_compares_only_the_low_fifteen_bits() {
        let update = w14a09_update_tile_id(0x8000_0001, 0x8001, 1, true);
        assert!(!update.changed);
        assert_eq!(update.flags, 0x8000_0001);
        assert_eq!(update.id, 1);

        let update = w14a09_update_tile_id(0x8000_0001, 0x8001, 2, false);
        assert!(update.changed);
        assert_eq!(update.id, 2);
        // Only bit 30 is rewritten; the low half is not the id.
        assert_eq!(update.flags, 0x8000_0001);

        let update = w14a09_update_tile_id(0x4000_0007, 0, 3, true);
        assert!(update.changed);
        assert_eq!(update.flags, 0x4000_0007);
    }

    // -- 0x00681ad0 --------------------------------------------------------

    fn scroll_state() -> W14a09ScrollState {
        W14a09ScrollState {
            x: 0,
            y: 0,
            prev_x: 200,
            prev_y: 200,
            tile_x: 0,
            tile_y: 0,
            last_tile_x: 0,
            last_tile_y: 0,
        }
    }

    #[test]
    fn scroll_to_clamps_converts_and_advances_the_five_layers() {
        let mut state = scroll_state();
        let mut layers = W14a09Parallax::default();
        let velocity = [1, 2, 3, 4, 5];

        let result = w14a09_scroll_to(
            &mut state,
            &mut layers,
            100,
            100,
            640,
            480,
            &velocity,
            &velocity,
            97,
            89,
            64,
        );

        assert!(result.scrolled);
        assert!(result.tile_changed);
        assert_eq!(result.refresh_rows, 3);
        assert_eq!((state.x, state.y), (100, 100));
        assert_eq!((state.tile_x, state.tile_y), (3, 3));
        assert_eq!((state.prev_x, state.prev_y), (100, 100));
        assert_eq!((state.last_tile_x, state.last_tile_y), (3, 3));
        assert_eq!(layers.offset_x, [3, 6, 9, 12, 15]);
        assert_eq!(layers.offset_y, [11, 22, 33, 44, 55]);
    }

    #[test]
    fn scroll_to_clamps_both_limits_and_keeps_the_previous_request() {
        let mut state = scroll_state();
        let mut layers = W14a09Parallax::default();
        let velocity = [0, 0, 0, 0, 0];

        let result = w14a09_scroll_to(
            &mut state,
            &mut layers,
            10_000,
            -10,
            640,
            480,
            &velocity,
            &velocity,
            97,
            89,
            64,
        );
        assert_eq!((state.x, state.y), (640, 0));
        assert_eq!((state.tile_x, state.tile_y), (20, 0));
        assert!(result.tile_changed);
    }

    #[test]
    fn scroll_to_returns_early_when_the_position_did_not_move() {
        let mut state = scroll_state();
        state.prev_x = 3;
        state.prev_y = 3;
        state.last_tile_x = 3;
        state.last_tile_y = 3;

        let mut layers = W14a09Parallax {
            offset_x: [1, 2, 3, 4, 5],
            offset_y: [1, 2, 3, 4, 5],
        };
        let velocity = [9, 9, 9, 9, 9];
        let result = w14a09_scroll_to(
            &mut state,
            &mut layers,
            3,
            3,
            640,
            480,
            &velocity,
            &velocity,
            97,
            89,
            64,
        );

        assert!(!result.scrolled);
        assert!(!result.tile_changed);
        assert_eq!(result.refresh_rows, 0);
        assert_eq!(layers.offset_x, [1, 2, 3, 4, 5]);
        assert_eq!(layers.offset_y, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn scroll_to_reports_an_in_band_tile_move_without_rows() {
        let mut state = scroll_state();
        state.prev_x = 100;
        state.prev_y = 100;
        state.last_tile_x = 2;
        state.last_tile_y = 3;
        let mut layers = W14a09Parallax::default();
        let velocity = [0, 0, 0, 0, 0];

        let result = w14a09_scroll_to(
            &mut state,
            &mut layers,
            110,
            100,
            640,
            480,
            &velocity,
            &velocity,
            97,
            89,
            64,
        );
        assert!(result.scrolled);
        assert!(result.tile_changed);
        assert_eq!((state.tile_x, state.tile_y), (3, 3));
        assert_eq!(state.last_tile_x, 3);
    }

    // -- 0x00681cc0 --------------------------------------------------------

    #[test]
    fn vertical_limit_wraps_like_the_unsigned_body() {
        assert_eq!(w14a09_vertical_scroll_limit(1024, 32, 64), 992);
        assert_eq!(w14a09_vertical_scroll_limit(0, 0, 1), u32::MAX);
        assert_eq!(w14a09_vertical_scroll_limit(u32::MAX, 2, 0), 1,);
    }

    // -- 0x00681e70 --------------------------------------------------------

    #[test]
    fn scroll_down_scales_the_delta_and_moves_the_layers() {
        let mut state = W14a09ScrollState {
            y: 100,
            prev_y: 100,
            tile_y: 3,
            last_tile_y: 3,
            ..W14a09ScrollState::default()
        };
        let mut layers = W14a09Parallax {
            offset_y: [10, 20, 30, 40, 50],
            ..W14a09Parallax::default()
        };

        let result =
            w14a09_scroll_down(&mut state, &mut layers, 4, 1000, &[1, 2, 3, 4, 5], 89, 1.0);

        assert!(result.scrolled);
        assert_eq!(result.rows_to_refresh, 0);
        assert_eq!(state.y, 104);
        assert_eq!(state.prev_y, 104);
        assert_eq!(state.tile_y, 3);
        assert_eq!(layers.offset_y, [6, 12, 18, 24, 30]);
    }

    #[test]
    fn scroll_down_folds_the_clamped_overshoot_into_the_delta() {
        let mut state = W14a09ScrollState {
            y: 995,
            prev_y: 995,
            tile_y: 31,
            last_tile_y: 3,
            ..W14a09ScrollState::default()
        };
        let mut layers = W14a09Parallax {
            offset_y: [10, 20, 30, 40, 50],
            ..W14a09Parallax::default()
        };

        let result =
            w14a09_scroll_down(&mut state, &mut layers, 64, 1000, &[1, 2, 3, 4, 5], 89, 1.0);

        assert_eq!(state.y, 1000);
        assert_eq!(state.tile_y, 31);
        // The effective delta is 5 after the overshoot is folded back.
        assert_eq!(layers.offset_y, [5, 10, 15, 20, 25]);
        assert_eq!(result.rows_to_refresh, 28);
    }

    #[test]
    fn scroll_down_adds_the_wrap_value_only_once_and_honours_the_factor() {
        let mut state = W14a09ScrollState {
            y: 0,
            prev_y: 0,
            tile_y: 0,
            last_tile_y: 0,
            ..W14a09ScrollState::default()
        };
        let mut layers = W14a09Parallax {
            offset_y: [10, 20, 30, 40, 50],
            ..W14a09Parallax::default()
        };

        // factor 2.0 halves a 64-pixel request to 32.
        let result =
            w14a09_scroll_down(&mut state, &mut layers, 64, 1000, &[1, 2, 3, 4, 5], 89, 2.0);
        assert_eq!(state.y, 32);
        // One wrap add per layer; the last layer stays negative because the
        // retail body adds the wrap value only once.
        assert_eq!(layers.offset_y, [67, 45, 23, 1, -21]);
        assert_eq!(result.rows_to_refresh, 1);

        // factor below one is clamped to one.
        let mut state = W14a09ScrollState::default();
        let mut layers = W14a09Parallax::default();
        w14a09_scroll_down(&mut state, &mut layers, 64, 1000, &[0; 5], 89, 0.25);
        assert_eq!(state.y, 64);
    }

    #[test]
    fn scroll_down_reports_no_movement_for_a_zero_delta() {
        let mut state = W14a09ScrollState {
            y: 10,
            prev_y: 10,
            ..W14a09ScrollState::default()
        };
        let mut layers = W14a09Parallax {
            offset_y: [1, 2, 3, 4, 5],
            ..W14a09Parallax::default()
        };
        let result = w14a09_scroll_down(&mut state, &mut layers, 0, 1000, &[1; 5], 89, 1.0);
        assert!(!result.scrolled);
        assert_eq!(layers.offset_y, [1, 2, 3, 4, 5]);
    }

    // -- 0x00681ff0 --------------------------------------------------------

    #[test]
    fn scroll_left_clamps_at_zero_and_wraps_the_layers() {
        let mut state = W14a09ScrollState {
            x: 300,
            prev_x: 300,
            tile_x: 9,
            last_tile_x: 9,
            ..W14a09ScrollState::default()
        };
        let mut layers = W14a09Parallax {
            offset_x: [10, 20, 30, 40, 50],
            ..W14a09Parallax::default()
        };

        let result = w14a09_scroll_left(&mut state, &mut layers, 20, &[1, 2, 3, 4, 5], 97, 1.0);

        assert!(result.scrolled);
        assert_eq!(result.columns_to_refresh, 1);
        assert_eq!(state.x, 280);
        assert_eq!(state.tile_x, 8);
        assert_eq!(state.last_tile_x, 8);
        assert_eq!(layers.offset_x, [30, 60, 90, 23, 53]);
    }

    #[test]
    fn scroll_left_folds_the_underflow_into_the_delta() {
        let mut state = W14a09ScrollState {
            x: 5,
            prev_x: 5,
            tile_x: 0,
            last_tile_x: 9,
            ..W14a09ScrollState::default()
        };
        let mut layers = W14a09Parallax::default();

        let result = w14a09_scroll_left(&mut state, &mut layers, 20, &[1; 5], 97, 1.0);

        assert_eq!(state.x, 0);
        assert_eq!(state.tile_x, 0);
        assert_eq!(result.columns_to_refresh, 9);
        assert_eq!(layers.offset_x, [5, 5, 5, 5, 5]);
    }

    #[test]
    fn scroll_left_reports_no_movement_when_the_position_is_unchanged() {
        let mut state = W14a09ScrollState {
            x: 0,
            prev_x: 0,
            ..W14a09ScrollState::default()
        };
        let mut layers = W14a09Parallax {
            offset_x: [7, 8, 9, 10, 11],
            ..W14a09Parallax::default()
        };
        let result = w14a09_scroll_left(&mut state, &mut layers, 0, &[1; 5], 97, 1.0);
        assert!(!result.scrolled);
        assert_eq!(result.columns_to_refresh, 0);
        assert_eq!(layers.offset_x, [7, 8, 9, 10, 11]);
    }

    #[test]
    fn scroll_left_scales_a_large_factor_down_to_one() {
        let mut state = W14a09ScrollState {
            x: 100,
            prev_x: 100,
            ..W14a09ScrollState::default()
        };
        let mut layers = W14a09Parallax::default();
        w14a09_scroll_left(&mut state, &mut layers, 10, &[0; 5], 97, 0.5);
        assert_eq!(state.x, 90);
    }
}
