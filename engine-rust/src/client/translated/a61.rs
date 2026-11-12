#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent61.rs
// ===========================================================================
// Translation batch 61.  Only this file is edited by the batch.
//
// Batch 61 owns `0x0060c000..=0x0060dfff` of the shipped StarCraft 1.23
// executable.  The ledger lists 22 `decompiled_uncited` rows in that window;
// 18 are translated below and four are skipped (reasons at the end of this
// file).
//
// The window is the map-view / zoom controller:
//
// * `0x00f04ac0..0x00f04b03` is one view record: viewport width
//   (`0x00f04ac0`), the height rounded up by the current scale
//   (`0x00f04ac4`), the requested height (`0x00f04ac8`), the scale
//   (`0x00f04acc`) and the zoom interpolation state
//   (`0x00f04ad0..=0x00f04b00`).
// * `0x00fa947c` is a screen/world rectangle; `0x0060ce40` tests a point
//   against it.
// * The zoom-step table holds `2.0, 1.5, 1.0, 0.75, 0.5, 0.33, 0.25`;
//   `0x0060dce0` special-cases the `2.0` level.
// * The per-map tile buffers created by `0x0060d1a0` and released by
//   `0x0060d050` hold the `0x400`-byte tile blocks and the `0x20000`-byte
//   light and tile-index arrays.
//
// The helper at `FUN_00c9c5ae` is implemented here as [`a61_round_away`]
// because `0x0060d940` depends on its exact float behaviour.  Every other
// helper invoked by these rows (for example `FUN_0060c0e0`, the zoom-level
// mapper) is either outside this window or was not decompiled; those results
// are explicit parameters or returned action variants, never cited as Rust
// functions.

// ---------------------------------------------------------------------------
// Float helpers used by the view rows
// ---------------------------------------------------------------------------

/// The comparison tolerance of the zoom rows: `0x34800000` = 2^-22,
/// printed by the decompiler as `2.3841858e-07` (used by `0x0060da50`,
/// `0x0060dce0` and `FUN_0060da5f`).
pub const A61_EPS: f32 = f32::from_bits(0x3480_0000);

/// `FUN_00c9c5ae` (`FUN_00c9c5ae`, 82 bytes): the shipped float adjustment
/// called by `0x0060d940` before its `cvttss2si`.
///
/// The routine asks the bit classifier at `FUN_00c93a21` whether the value has
/// bits below 2^-1, then below 2^0.  When both answers say "bits present" it
/// adds `+1.0` (positive sign) or `-1.0` (negative sign).  Integers and
/// non-finite values are returned unchanged, while every non-zero value below
/// `1.0` (including exact powers of two such as `0.5`) has fractional bits
/// and is moved.  A later `(int)` cast therefore rounds away from zero:
/// `ceil` for positive non-integers and `floor` for negative ones.
/// Half-integers are returned as `x ± 1` (for example `2.5 -> 3.5`) but
/// truncate the same way.
pub fn a61_round_away(x: f32) -> f32 {
    let bits = x.to_bits();
    let exponent = (bits >> 23) & 0xff;
    let mantissa = bits & 0x007f_ffff;
    // 0xff is inf/NaN: `FUN_00c93a21` returns the 1/2 class codes and the
    // caller leaves the value alone.
    if exponent == 0xff {
        return x;
    }
    // Signed zero keeps its sign and is already an integer.
    if bits & 0x7fff_ffff == 0 {
        return x;
    }
    // (0x96 - biased_exponent) - 0, the bit count the classifier shifts by.
    let shift = 150i32 - exponent as i32;
    if shift <= 0 {
        // |x| >= 2^23: every mantissa bit is an integer bit.
        return x;
    }
    // Shifts above 23 discard the whole mantissa; any non-zero value there is
    // fractional (this includes powers of two below 1.0 such as 0.5).
    let has_fraction = shift > 23 || (mantissa & ((1u32 << shift) - 1)) != 0;
    if !has_fraction {
        return x;
    }
    if bits & 0x8000_0000 == 0 {
        x + 1.0
    } else {
        x - 1.0
    }
}

/// x86 `cvttss2si`: truncate toward zero, returning the integer indefinite
/// value `0x80000000` for NaN and out-of-range inputs.
pub fn a61_cvttss2si(x: f32) -> i32 {
    if x.is_nan() || x >= 2_147_483_648.0 || x < -2_147_483_648.0 {
        i32::MIN
    } else {
        x as i32
    }
}

/// The height computation shared by `0x0060d940` and `0x0060d8f0`:
/// `(int)FUN_00c9c5ae((float)height * scale)`.
pub fn a61_scaled_height(height: i32, scale: f32) -> i32 {
    a61_cvttss2si(a61_round_away(height as f32 * scale))
}

// ---------------------------------------------------------------------------
// 0x0060ce40 - point in the view rectangle
// ---------------------------------------------------------------------------

/// The four dwords loaded from `0x00fa947c` by `0x0060ce40`:
/// `left`, `top`, `right`, `bottom`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A61ViewRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// `0x0060ce40` (`FUN_0060ce40`, 126 bytes): membership test of one
/// transformed point in the view rectangle.
///
/// The row first converts its two arguments through `FUN_007b2f10` into the
/// point `(x, y)`; that mapper is outside this batch, so the translated
/// function takes its result directly.  The rectangle test itself is exact:
///
/// ```text
/// 0 <= x - left < right - left  and  0 <= y - top < bottom - top
/// ```
///
/// with 32-bit wrapping subtraction and signed compares.
pub fn a61_point_in_rect(x: i32, y: i32, rect: A61ViewRect) -> bool {
    let dx = x.wrapping_sub(rect.left);
    let dy = y.wrapping_sub(rect.top);
    dx >= 0
        && dy >= 0
        && dx < rect.right.wrapping_sub(rect.left)
        && dy < rect.bottom.wrapping_sub(rect.top)
}

// ---------------------------------------------------------------------------
// 0x0060cec0 - recentre the camera on the stored tile
// ---------------------------------------------------------------------------

/// `0x0060cec0` (`FUN_0060cec0`, 208 bytes): request for the new camera
/// pixel position derived from the two view-tile words at `+0xe0`/`+0xe2`.
///
/// Only the camera branch of the row is modelled here.  It runs when
/// `FUN_00748d60()` is zero and `0x011a4580` is zero (see
/// [`a61_recenter_active`]); the other branch scans the eight player slots
/// through `FUN_00748d40`/`FUN_00748da0`, which are outside this window.
///
/// The two words are the camera origin in tiles.  `0xffff` in either word is
/// the sentinel for "no stored position": the row then calls the centring
/// helper with `(0, 0)`.  Otherwise the request is `(tile_x << 5,
/// tile_y << 5)` (tile size 32).  `None` is the sentinel branch; the shipped
/// call passes `(0, 0)` in that case.
pub fn a61_recenter_request(tile_x: u16, tile_y: u16) -> Option<(u32, u32)> {
    if tile_x == 0xffff || tile_y == 0xffff {
        None
    } else {
        Some(((tile_x as u32) << 5, (tile_y as u32) << 5))
    }
}

/// The guard at the head of `0x0060cec0`:
/// `FUN_00748d60()` is `DAT_00f2acf8 - 0x80 < 4` (unsigned) and the camera
/// branch also requires `DAT_011a4580 == 0`.
pub fn a61_recenter_active(state_word: u32, other: u32) -> bool {
    !(state_word.wrapping_sub(0x80) < 4) && other == 0
}

/// The clamp performed by the centring helper called from `0x0060cec0`
/// (`FUN_00681ad0`): each requested pixel is limited to the map scroll word
/// `0x00fe5b4c` / `0x00fe5b50`, after a negative value is raised to zero.
/// The request produced by [`a61_recenter_request`] is never negative, so the
/// clamp is `min(request, limit)`.
pub fn a61_recenter_clamp(request: (u32, u32), limit: (u32, u32)) -> (u32, u32) {
    (request.0.min(limit.0), request.1.min(limit.1))
}

/// The tail loop of `0x0060cec0`: three dwords at `+0x180`, `+0x184` and
/// `+0x188` each receive the low word of the new camera x and y.  The loop
/// increments the byte offset by four and stops at `0x0c`.
pub fn a61_recenter_pairs(pixel: (u32, u32)) -> [(u16, u16); 3] {
    [(pixel.0 as u16, pixel.1 as u16); 3]
}

// ---------------------------------------------------------------------------
// 0x0060cfd0 / 0x0060d190 / 0x0060d8e0 - view state accessors
// ---------------------------------------------------------------------------

/// The recovered view state at `0x00f04ac0..=0x00f04b03`.  Byte fields are
/// modelled as `bool`, the eight-byte counter at `0x00f04ae8` as `u64`.
/// No existing type gains fields; this record is local to the batch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct A61View {
    /// `0x00f04ac0`: viewport width in pixels.
    pub width: i32,
    /// `0x00f04ac4`: height scaled by `scale` and rounded up.
    pub scaled_height: i32,
    /// `0x00f04ac8`: requested height in pixels.
    pub height: i32,
    /// `0x00f04acc`: scale, clamped to `[0.5, 1.0]` by `0x0060d8f0`.
    pub scale: f32,
    /// `0x00f04ad0`: committed zoom level.
    pub zoom: f32,
    /// `0x00f04ad4`: zoom at the start of the interpolation.
    pub zoom_start: f32,
    /// `0x00f04ad8`: zoom target of the interpolation.
    pub zoom_target: f32,
    /// `0x00f04adc`: non-zero while an interpolation is active.
    pub interpolating: bool,
    /// `0x00f04ae0`: interpolation fraction in `[0.0, 1.0]`.
    pub fraction: f32,
    /// `0x00f04ae8` low dword plus `0x00f04aec` high dword.
    pub stamp: u64,
    /// `0x00f04af0`: view mode (0..3).
    pub mode: i32,
    /// `0x00f04af4`: non-zero while an animated zoom owns the view.
    pub animating: bool,
    /// `0x00f04af8`: last zoom direction, `-1` or `1`.
    pub direction: i32,
    /// `0x00f04afc`: cleared dword.
    pub word_afc: u32,
    /// `0x00f04b00`: cleared dword.
    pub word_b00: u32,
}

/// A zero-initialised record with the shipped default scale of 1.0.  The
/// executable starts every other member at zero; `0x0060cfd0` restores the
/// `fraction = 1.0` default.
pub fn a61_default_view() -> A61View {
    A61View {
        width: 0,
        scaled_height: 0,
        height: 0,
        scale: 1.0,
        zoom: 0.0,
        zoom_start: 0.0,
        zoom_target: 0.0,
        interpolating: false,
        fraction: 0.0,
        stamp: 0,
        mode: 0,
        animating: false,
        direction: 0,
        word_afc: 0,
        word_b00: 0,
    }
}

impl A61View {
    /// `0x0060d190` (`FUN_0060d190`, 6 bytes): `return DAT_00f04ac4`.
    pub fn a61_scaled_height(&self) -> i32 {
        self.scaled_height
    }

    /// `0x0060d8e0` (`FUN_0060d8e0`, 6 bytes): `return DAT_00f04ac8`.
    pub fn a61_height(&self) -> i32 {
        self.height
    }

    /// `0x0060cfd0` (`FUN_0060cfd0`, 88 bytes): collapse the interpolation
    /// onto the current zoom.
    ///
    /// The row stores `0` to `0x00f04afc`, copies `0x00f04ad0` to both
    /// `0x00f04ad4` and `0x00f04ad8`, writes `1.0` to `0x00f04ae0`, clears
    /// the two flags at `0x00f04adc`/`0x00f04af4` and clears
    /// `0x00f04b00`.  `mode`, `scale` and the stamp are untouched.
    pub fn a61_reset_interpolation(&mut self) {
        self.word_afc = 0;
        self.zoom_start = self.zoom;
        self.zoom_target = self.zoom;
        self.fraction = 1.0;
        self.interpolating = false;
        self.animating = false;
        self.word_b00 = 0;
    }
}

// ---------------------------------------------------------------------------
// 0x0060d030 - one-time subsystem-5 marker
// ---------------------------------------------------------------------------

/// `0x0060d030` (`FUN_0060d030`, 18 bytes): mark subsystem 5 initialised.
///
/// The guard tests bit 0 of `0x011cb56d`, which is exactly
/// `(&DAT_011cb509)[5 * 0x14]` (the byte `FUN_009546d0` writes).  When the bit
/// is clear the row calls `FUN_009546d0(5)`; that helper ORs `1` into the same
/// byte.  The translated function performs the same test and OR and reports
/// whether it changed the byte.
pub fn a61_mark_subsystem5(flags: &mut u8) -> bool {
    if *flags & 1 == 0 {
        *flags |= 1;
        true
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// 0x0060d050 - map teardown
// ---------------------------------------------------------------------------

/// The five heap blocks released by `0x0060d050`, plus the five scalar words
/// it clears and `0x00fa949c`, which is cleared but not released.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A61MapBuffers {
    /// `0x00fa9478`: the `0x40000`-byte cleared buffer.
    pub flags: usize,
    /// `0x00fa9468`: buffer cleared here (not created by `0x0060d1a0`).
    pub buffer68: usize,
    /// `0x00fe5b70`: per-tile `0x400`-byte flag blocks.
    pub tile_flags: usize,
    /// `0x00fe5b6c`: the `0x20000`-byte tile-index array.
    pub tile_data: usize,
    /// `0x00fa948c`: the `0x20000`-byte light array.
    pub light: usize,
    /// `0x00fa949c`: cleared but never released.
    pub misc9c: usize,
    /// `0x00fe5b74`.
    pub word_5b74: u32,
    /// `0x00fe5b68`.
    pub word_5b68: u32,
    /// `0x011d3020`: tile-type table pointer.
    pub word_3020: u32,
    /// `0x00fa9470`.
    pub word_9470: u32,
    /// `0x00fa9474`.
    pub word_9474: u32,
}

/// The helper calls made by `0x0060d050`, in execution order.  `FUN_00604340`
/// supplies the object consumed by `FUN_00604290`; `FUN_00680290` supplies the
/// object consumed by `FUN_006c31f0`.
pub const A61_TEARDOWN_STEPS: [u32; 11] = [
    0x0075_d1d0,
    0x0067_ee60,
    0x0060_4340,
    0x0060_4290,
    0x0071_55a0,
    0x0077_4dd0,
    0x0068_1360,
    0x0070_8000,
    0x0070_8030,
    0x0068_0290,
    0x006c_31f0,
];

/// Result of `0x0060d050`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A61MapRelease {
    /// Non-null buffers in release order.
    pub freed: Vec<usize>,
    /// The zeroed record after the row.
    pub buffers: A61MapBuffers,
    /// The helper sequence from [`A61_TEARDOWN_STEPS`].
    pub steps: Vec<u32>,
}

/// `0x0060d050` (`FUN_0060d050`, 290 bytes): release the map subsystem.
///
/// Five `FUN_00924220` releases run in the order `0x00fa9478`, `0x00fa9468`,
/// `0x00fe5b70`, `0x00fe5b6c`, `0x00fa948c`, each guarded by a non-null test.
/// `0x00fa949c` is cleared without a release.  The row clears
/// `0x00fe5b74`, `0x00fe5b68`, `0x011d3020`, `0x00fa9470` and `0x00fa9474`
/// before the releases, and the five released pointers afterwards.  The
/// eleven helper calls interleave around the releases exactly as listed in
/// [`A61_TEARDOWN_STEPS`].
pub fn a61_release_map(buffers: A61MapBuffers) -> A61MapRelease {
    let mut freed = Vec::new();
    for pointer in [
        buffers.flags,
        buffers.buffer68,
        buffers.tile_flags,
        buffers.tile_data,
        buffers.light,
    ] {
        if pointer != 0 {
            freed.push(pointer);
        }
    }
    A61MapRelease {
        freed,
        buffers: A61MapBuffers::default(),
        steps: A61_TEARDOWN_STEPS.to_vec(),
    }
}

// ---------------------------------------------------------------------------
// 0x0060d1a0 - map subsystem creation and buffer sizes
// ---------------------------------------------------------------------------

/// C's truncating `(x + 0x1f) / 0x20`, as emitted by `0x0060d1a0`
/// (`add`, `cdq`, `and edx,0x1f`, `add`, `sar 5`).
fn a61_c_div32(x: i32) -> i32 {
    let y = x.wrapping_add(0x1f);
    let y = if y < 0 { y.wrapping_add(0x1f) } else { y };
    y >> 5
}

/// `0x0060d1a0` (`FUN_0060d1a0`, 262 bytes): size of the per-tile flag
/// buffer:
///
/// ```text
/// (((height + 0x1f) / 32) + 1) * (((width + 0x1f) / 32) + 1) << 10
/// ```
///
/// The divisions are C integer divisions (truncating toward zero) and the
/// product and shift wrap at 32 bits, exactly like the `imul`/`shl` pair at
/// `0x0060d34a`.  The extra row and column are the border tiles.
pub fn a61_tile_buffer_size(width: i32, height: i32) -> u32 {
    let rows = a61_c_div32(height).wrapping_add(1);
    let columns = a61_c_div32(width).wrapping_add(1);
    rows.wrapping_mul(columns).wrapping_shl(10) as u32
}

/// The state left by `0x0060d1a0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A61MapInit {
    /// True when the row created a new subsystem object (the object word was
    /// zero on entry).
    pub created: bool,
    /// `0x00fa94a0`: object created by `FUN_007b2040`.
    pub object: u32,
    /// `0x00fa9490`: value read through `FUN_00551600` after creation.
    pub field_490: u32,
    /// `0x01177478`: word cleared to zero.
    pub word_1177478: u16,
    /// `0x011b8678`: byte cleared to zero.
    pub byte_11b8678: u8,
    /// `0x00fa948c`: `0x20000` bytes.
    pub light_size: usize,
    /// `0x00fe5b6c`: `0x20000` bytes.
    pub tile_data_size: usize,
    /// `0x00fe5b70`: [`a61_tile_buffer_size`] bytes.
    pub tile_flags_size: usize,
    /// `0x00fa9478`: `0x40000` bytes.
    pub buffer_size: usize,
    /// Bytes zeroed in `0x00fa9478` by `FUN_00c8a080`.
    pub buffer_cleared: usize,
}

/// `0x0060d1a0` (`FUN_0060d1a0`, 262 bytes): create the map subsystem.
///
/// When `0x00fa94a0` is zero the row creates the object through
/// `FUN_007b2040` and stores the dword returned through the `FUN_00551600`
/// factory at `0x00fa9490`; two guarded callable blocks surround those
/// calls.  It then clears `0x01177478` (word) and `0x011b8678` (byte),
/// allocates `0x00fa948c` (0x20000), `0x00fe5b6c` (0x20000),
/// `0x00fe5b70` ([`a61_tile_buffer_size`]) and `0x00fa9478` (0x40000), and
/// clears the last block with `FUN_00c8a080`.
///
/// The two callable blocks only run on the creation path and their callbacks
/// are not part of the row's returned data; `object` and `field_490` are the
/// values those calls produce.  Passing `Some((object, field_490))` models a
/// second call with the subsystem already present, which skips creation but
/// still re-runs the clears and allocations.
pub fn a61_create_map_buffers(
    existing: Option<(u32, u32)>,
    object: u32,
    field_490: u32,
    width: i32,
    height: i32,
) -> A61MapInit {
    let (created, object, field_490) = match existing {
        Some((object, field_490)) => (false, object, field_490),
        None => (true, object, field_490),
    };
    A61MapInit {
        created,
        object,
        field_490,
        word_1177478: 0,
        byte_11b8678: 0,
        light_size: 0x20000,
        tile_data_size: 0x20000,
        tile_flags_size: a61_tile_buffer_size(width, height) as usize,
        buffer_size: 0x40000,
        buffer_cleared: 0x40000,
    }
}

// ---------------------------------------------------------------------------
// 0x0060d6b0 - rebuild terrain / light data
// ---------------------------------------------------------------------------

/// Result of `0x0060d6b0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A61TerrainRebuild {
    /// Bytes cleared in the light array by `FUN_00719e40` (0x20000).
    pub light_bytes_cleared: u32,
    /// Size reported by `FUN_00983510`: `minimap_width * minimap_height * 2`,
    /// or zero when its guard word is clear.
    pub tile_bytes: u32,
    /// `+0xe4`: map width in tiles.
    pub width_tiles: u16,
    /// `+0xe6`: map height in tiles.
    pub height_tiles: u16,
    /// The `FUN_006c3290` / `FUN_006c2b90` branch, taken when the first byte of
    /// the object returned by `FUN_00680290` is non-zero.
    pub ui_rebuilt: bool,
    /// `FUN_006813a0` (water overlay rebuild) always runs.
    pub water_rebuilt: bool,
}

/// `0x0060d6b0` (`FUN_0060d6b0`, 145 bytes): rebuild the terrain upload.
///
/// The row clears `0x20000` bytes at `0x00fa948c`, asks `FUN_00983510` for the
/// packed tile size (0 when `0x011cf174` is clear, otherwise
/// `0x011cf180 * 0x011cf17c * 2`), calls `FUN_00680a90` with that size and the
/// two tile dimensions, optionally rebuilds the interface overlay through
/// `FUN_006c3290`/`FUN_006c2b90`, and always calls `FUN_006813a0`.
pub fn a61_terrain_rebuild(
    minimap: Option<(i32, i32)>,
    width_tiles: u16,
    height_tiles: u16,
    ui_present: bool,
) -> A61TerrainRebuild {
    A61TerrainRebuild {
        light_bytes_cleared: 0x20000,
        tile_bytes: minimap
            .map(|(width, height)| width.wrapping_mul(height).wrapping_mul(2) as u32)
            .unwrap_or(0),
        width_tiles,
        height_tiles,
        ui_rebuilt: ui_present,
        water_rebuilt: true,
    }
}

// ---------------------------------------------------------------------------
// 0x0060d750 - vertical viewport pixel range
// ---------------------------------------------------------------------------

/// `0x0060d750` (`FUN_0060d750`, 91 bytes): pixel range used by the viewport
/// refresh.
///
/// With `0x0117758d` non-zero the row computes
///
/// ```text
/// low  = max(0, view_tile_y - 4)
/// high = scaled_height + 4 + view_tile_y
/// if map_height_tiles <= high { high = map_height_tiles - 1 }
/// ```
///
/// and passes `(low, high)` to `FUN_0057c5b0`.  `None` is the disabled path.
pub fn a61_view_pixel_range(
    scaled_height: i32,
    view_tile_y: u16,
    map_height_tiles: u16,
    enabled: bool,
) -> Option<(i32, i32)> {
    if !enabled {
        return None;
    }
    let mut low = view_tile_y as i32 - 4;
    if low < 0 {
        low = 0;
    }
    let mut high = scaled_height
        .wrapping_add(4)
        .wrapping_add(view_tile_y as i32);
    if (map_height_tiles as i32) <= high {
        high = map_height_tiles as i32 - 1;
    }
    Some((low, high))
}

// ---------------------------------------------------------------------------
// 0x0060d8f0 / 0x0060d940 - scale and viewport size
// ---------------------------------------------------------------------------

/// Result of `0x0060d940`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A61SizeApply {
    pub scaled_height: i32,
    pub width: i32,
    pub height: i32,
    /// True when width, scaled height and height all matched and the row
    /// returned at `0x0060d9b3` without updating anything.
    pub early_return: bool,
}

impl A61View {
    /// `0x0060d8f0` (`FUN_0060d8f0`, 70 bytes): clamp the scale to
    /// `[0.5, 1.0]`, store it and reapply the view size.
    ///
    /// The two SSE operations are `minss` with `1.0` and a `comiss`/`ja`
    /// against `0.5`; as a result NaN becomes `0.5` (not `1.0` as the
    /// decompiled C suggests).  The clamped value is both stored at
    /// `0x00f04acc` and returned.
    pub fn a61_set_scale(&mut self, value: f32) -> f32 {
        // minss dest=1.0, src=value: NaN selects the source.
        let limited = if 1.0f32 < value { 1.0 } else { value };
        let clamped = if limited > 0.5 { limited } else { 0.5 };
        self.scale = clamped;
        self.a61_apply_size(self.width, self.height);
        clamped
    }

    /// `0x0060d940` (`FUN_0060d940`, 118 bytes): store a new viewport width
    /// and height and recompute the scaled height.
    ///
    /// `scaled = (int)FUN_00c9c5ae((float)height * scale)` is computed first.
    /// The row returns without changes when all three of width, scaled height
    /// and height already match.  A positive width replaces the stored width;
    /// a positive height replaces the stored height and scaled height.  The
    /// tail then calls `FUN_00681cc0` and `FUN_0060d7b0`; those viewport
    /// consumers are reported through `early_return == false`.
    pub fn a61_apply_size(&mut self, width: i32, height: i32) -> A61SizeApply {
        let scaled = a61_scaled_height(height, self.scale);
        if width == self.width && self.scaled_height == scaled && self.height == height {
            return A61SizeApply {
                scaled_height: self.scaled_height,
                width: self.width,
                height: self.height,
                early_return: true,
            };
        }
        if width > 0 {
            self.width = width;
        }
        if height > 0 {
            self.height = height;
            self.scaled_height = scaled;
        }
        A61SizeApply {
            scaled_height: self.scaled_height,
            width: self.width,
            height: self.height,
            early_return: false,
        }
    }

    /// `0x0060d9c0` (`FUN_0060d9c0`, 138 bytes): reset the zoom to a new
    /// base scale.
    ///
    /// The row sets `mode = 1`, stores the mapper result at `0x00f04ad0`,
    /// clears `0x00f04afc`, copies the new value to the start and target
    /// words, sets the fraction to `1.0`, clears both flags and clears
    /// `0x00f04b00`.  `base` is the result of the zoom mapper at
    /// `FUN_0060c0e0` (outside this window); the shipped tail `FUN_0060fba0`
    /// commits the request and returns the value read back from
    /// `0x00f04ad0`, which can be clamped there.  The value stored by this
    /// row is `base`.
    pub fn a61_reset_zoom_from_base(&mut self, base: f32) -> f32 {
        self.mode = 1;
        self.zoom = base;
        self.word_afc = 0;
        self.zoom_start = base;
        self.zoom_target = base;
        self.fraction = 1.0;
        self.interpolating = false;
        self.animating = false;
        self.word_b00 = 0;
        base
    }
}

// ---------------------------------------------------------------------------
// 0x0060da50 / 0x0060dad0 / 0x0060db10 - zoom requests
// ---------------------------------------------------------------------------

/// The action requested by the zoom entry points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum A61ZoomAction {
    /// Paused or the interface owns the input; the row returns immediately.
    Ignored,
    /// `FUN_0060fba0` followed by `0x0060cfd0`: commit `target` and collapse
    /// the interpolation.
    Snap { target: f32 },
    /// `FUN_0060ff50(direction, amount, mode)`: start an animated zoom.
    Request {
        direction: i32,
        amount: f32,
        mode: i32,
    },
}

impl A61View {
    /// `0x0060da50` (`FUN_0060da50`, 121 bytes): request an absolute zoom
    /// level.
    ///
    /// The two guard helpers (`FUN_007744b0` and `FUN_007494e0`) are supplied as
    /// `paused`/`ui_busy`.  When the request is within [`A61_EPS`] of the
    /// committed zoom the row commits it through `FUN_0060fba0` and calls
    /// `0x0060cfd0`; otherwise it calls
    /// `FUN_0060ff50(direction, target, 1)` with
    /// `direction = (zoom <= target) ? 1 : -1`.  The direction test is
    /// `setbe`, so a NaN target also selects `+1`.
    pub fn a61_request_zoom(&self, target: f32, paused: bool, ui_busy: bool) -> A61ZoomAction {
        if paused || ui_busy {
            return A61ZoomAction::Ignored;
        }
        let delta = (target - self.zoom).abs();
        if delta < A61_EPS {
            A61ZoomAction::Snap { target }
        } else {
            let direction = if !(self.zoom > target) { 1 } else { -1 };
            A61ZoomAction::Request {
                direction,
                amount: target,
                mode: 1,
            }
        }
    }

    /// `0x0060dad0` (`FUN_0060dad0`, 61 bytes): zoom-in shortcut.
    ///
    /// The row returns zero if paused, if the interface owns the input, or if
    /// an animated zoom is already running in the `+1` direction.  Otherwise
    /// it calls `FUN_0060ff50(1, 2.0, 1)` and returns one.  The translated
    /// function returns the request (or [`A61ZoomAction::Ignored`]).
    pub fn a61_zoom_in(&self, paused: bool, ui_busy: bool) -> A61ZoomAction {
        if paused || ui_busy {
            return A61ZoomAction::Ignored;
        }
        if self.animating && self.direction == 1 {
            return A61ZoomAction::Ignored;
        }
        A61ZoomAction::Request {
            direction: 1,
            amount: 2.0,
            mode: 1,
        }
    }

    /// `0x0060db10` (`FUN_0060db10`, 61 bytes): zoom-out shortcut.
    ///
    /// Same guards as [`A61View::a61_zoom_in`], but an animation already
    /// running in the `-1` direction blocks the request, and the amount is
    /// `0.25` (`0x3e800000`).
    pub fn a61_zoom_out(&self, paused: bool, ui_busy: bool) -> A61ZoomAction {
        if paused || ui_busy {
            return A61ZoomAction::Ignored;
        }
        if self.animating && self.direction == -1 {
            return A61ZoomAction::Ignored;
        }
        A61ZoomAction::Request {
            direction: -1,
            amount: 0.25,
            mode: 1,
        }
    }
}

// ---------------------------------------------------------------------------
// 0x0060db50 - accumulated wheel zoom
// ---------------------------------------------------------------------------

/// Result of `0x0060db50`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum A61WheelOutcome {
    /// The row returned before touching the state (animated zoom running, or
    /// paused/UI-owned).
    Ignored,
    /// The direction reversed; the row ran `0x0060cfd0` and then committed
    /// `request`.
    Reset { request: f32 },
    /// The target was scaled and the fraction recomputed; `request` is the
    /// interpolated zoom handed to `FUN_0060fba0`.
    Update { request: f32 },
}

impl A61View {
    /// `0x0060db50` (`FUN_0060db50`, 392 bytes): accumulate one wheel notch.
    ///
    /// `input` is the row's float argument (`param_1`), `mode` its second
    /// argument.  The mapper `FUN_0060c0e0`, which converts a scaled request
    /// into a legal zoom level, is supplied as `base`; `stamp` is the 64-bit
    /// value `FUN_0093f7b0` returns on the start path.
    ///
    /// * A running animation (`0x00f04af4`), `paused` or `ui_busy` leaves
    ///   everything untouched.
    /// * Without an active interpolation the target is
    ///   `base((input + 1.0) * target)`, the fraction is zeroed and the
    ///   stamp is stored.
    /// * A reversal (`target > start && input < 0`, or
    ///   `start > target && input > 0`) resets through `0x0060cfd0`.
    /// * Otherwise the target is rescaled and the fraction becomes
    ///   `(((old_target - start) * fraction + start) - start) /
    ///   (new_target - start)`, clamped to `[0.0, 1.0]`.  The nested
    ///   add/subtract of `start` is part of the shipped float sequence.
    ///
    /// The returned request is always
    /// `fraction * (target - start) + start`, computed in `f32`.
    pub fn a61_wheel<F: Fn(f32) -> f32>(
        &mut self,
        input: f32,
        mode: i32,
        base: F,
        stamp: u64,
        paused: bool,
        ui_busy: bool,
    ) -> A61WheelOutcome {
        if self.animating || paused || ui_busy {
            return A61WheelOutcome::Ignored;
        }
        self.mode = mode;

        if !self.interpolating {
            self.interpolating = true;
            let scaled = (input + 1.0) * self.zoom_target;
            self.zoom_target = base(scaled);
            self.stamp = stamp;
            self.fraction = 0.0;
            let request = self.fraction * (self.zoom_target - self.zoom_start) + self.zoom_start;
            return A61WheelOutcome::Update { request };
        }

        let start = self.zoom_start;
        let old_target = self.zoom_target;
        if (old_target > start && input < 0.0) || (start > old_target && input > 0.0) {
            self.a61_reset_interpolation();
            let request = self.fraction * (self.zoom_target - self.zoom_start) + self.zoom_start;
            return A61WheelOutcome::Reset { request };
        }

        let scaled = (input + 1.0) * old_target;
        let new_target = base(scaled);
        self.zoom_target = new_target;
        if new_target - start == 0.0 {
            self.fraction = 1.0;
        } else {
            let numerator = ((old_target - start) * self.fraction + start) - start;
            let ratio = numerator / (new_target - start);
            // minss dest=1.0, src=ratio: NaN selects the source, which the
            // following comiss/ja then maps to zero.
            let limited = if 1.0f32 < ratio { 1.0 } else { ratio };
            self.fraction = if limited > 0.0 { limited } else { 0.0 };
        }
        let request = self.fraction * (self.zoom_target - self.zoom_start) + self.zoom_start;
        A61WheelOutcome::Update { request }
    }
}

// ---------------------------------------------------------------------------
// 0x0060dce0 - zoom level from the step table
// ---------------------------------------------------------------------------

/// The seven zoom levels at `0x00d39a00`: `2.0, 1.5, 1.0, 0.75, 0.5, 0.33,
/// 0.25` (the literal `0.33` is the f32 `0x3ea8f5c3`).
pub const A61_ZOOM_STEPS: [f32; 7] = [2.0, 1.5, 1.0, 0.75, 0.5, 0.33, 0.25];

/// Outcome of `0x0060dce0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum A61StepOutcome {
    /// The interpolated zoom was outside the table (`z > 2.0` or
    /// `z < 0.25`); the row returned at `0x0060dd98`.
    NoMatch,
    /// Paused or the interface owns the input; the row returned after
    /// computing the index.
    Ignored,
    /// `FUN_0060fba0` plus `0x0060cfd0`: commit the selected level.
    Commit { target: f32 },
    /// `FUN_0060ff50(direction, target, 2)`.
    Request { direction: i32, target: f32 },
}

impl A61View {
    /// `0x0060dce0` (`FUN_0060dce0`, 370 bytes): pick a zoom level from the
    /// recovered step table.
    ///
    /// The current interpolated zoom is
    /// `z = fraction * (target - start) + start`.  `z` within [`A61_EPS`] of
    /// `2.0` selects index 0; otherwise the code scans from index 1 for the
    /// first `i` with `table[i] <= z <= table[i - 1]`, returning
    /// [`A61StepOutcome::NoMatch`] when the scan reaches index 7.  The
    /// selected index is then reduced by the truncated `steps` argument and
    /// clamped to `[0, 6]`.  The row commits the level when it is within
    /// [`A61_EPS`] of the committed zoom and otherwise requests
    /// `FUN_0060ff50(direction, level, 2)`.
    pub fn a61_zoom_step(&self, steps: f32, paused: bool, ui_busy: bool) -> A61StepOutcome {
        let z = self.fraction * (self.zoom_target - self.zoom_start) + self.zoom_start;
        let index = if (z - 2.0).abs() < A61_EPS {
            0usize
        } else {
            let mut candidate = 1usize;
            loop {
                if candidate >= A61_ZOOM_STEPS.len() {
                    return A61StepOutcome::NoMatch;
                }
                let below = z < A61_ZOOM_STEPS[candidate];
                let above = A61_ZOOM_STEPS[candidate - 1] >= z;
                if !below && above {
                    break candidate;
                }
                candidate += 1;
            }
        };

        let truncated = a61_cvttss2si(steps);
        let mut offset = (index as i32).wrapping_sub(truncated);
        if offset > 6 {
            offset = 6;
        }
        let selected = if offset > 0 { offset } else { 0 } as usize;

        if paused || ui_busy {
            return A61StepOutcome::Ignored;
        }

        let target = A61_ZOOM_STEPS[selected];
        if (target - self.zoom).abs() < A61_EPS {
            return A61StepOutcome::Commit { target };
        }
        let direction = if !(self.zoom > target) { 1 } else { -1 };
        A61StepOutcome::Request { direction, target }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent61 {
    use super::*;

    fn view() -> A61View {
        a61_default_view()
    }

    fn close(actual: f32, expected: f32) -> bool {
        (actual - expected).abs() <= 1e-6 * expected.abs().max(1.0)
    }

    /// `0x0060ce40`: all four rectangle inequalities.
    #[test]
    fn point_in_rect_uses_the_recovered_rectangle() {
        let rect = A61ViewRect {
            left: 10,
            top: 20,
            right: 110,
            bottom: 220,
        };
        assert!(a61_point_in_rect(10, 20, rect));
        assert!(a61_point_in_rect(109, 219, rect));
        assert!(a61_point_in_rect(55, 120, rect));
        assert!(!a61_point_in_rect(9, 20, rect));
        assert!(!a61_point_in_rect(10, 19, rect));
        assert!(!a61_point_in_rect(110, 20, rect));
        assert!(!a61_point_in_rect(10, 220, rect));
        assert!(!a61_point_in_rect(-1, -1, rect));

        let empty = A61ViewRect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        assert!(!a61_point_in_rect(0, 0, empty));
    }

    /// `0x0060cec0`: sentinel request, guard, tile shift, clamp and the
    /// three-pair tail loop.
    #[test]
    fn recenter_request_clamps_and_repeats_the_pixel() {
        assert!(!a61_recenter_active(0x80, 0));
        assert!(!a61_recenter_active(0x81, 0));
        assert!(!a61_recenter_active(0x83, 0));
        assert!(a61_recenter_active(0x7f, 0));
        assert!(a61_recenter_active(0x84, 0));
        assert!(!a61_recenter_active(0x84, 1));

        assert_eq!(a61_recenter_request(0xffff, 3), None);
        assert_eq!(a61_recenter_request(3, 0xffff), None);
        assert_eq!(a61_recenter_request(0xffff, 0xffff), None);
        assert_eq!(a61_recenter_request(3, 4), Some((96, 128)));
        assert_eq!(a61_recenter_request(0, 0), Some((0, 0)));

        let request = a61_recenter_request(3, 4).unwrap();
        assert_eq!(a61_recenter_clamp(request, (1000, 1000)), (96, 128));
        assert_eq!(a61_recenter_clamp(request, (64, 100)), (64, 100));
        assert_eq!(a61_recenter_clamp(request, (0, 0)), (0, 0));

        let pairs = a61_recenter_pairs((96, 128));
        assert_eq!(pairs, [(96, 128), (96, 128), (96, 128)]);
        // The loop stores the low 16 bits of each dword.
        assert_eq!(
            a61_recenter_pairs((0x0001_0060, 0x0002_0080)),
            [(0x0060, 0x0080); 3]
        );
    }

    /// `0x0060cfd0`: every field of the collapse, and the untouched ones.
    #[test]
    fn reset_interpolation_collapses_onto_the_committed_zoom() {
        let mut state = A61View {
            width: 640,
            scaled_height: 480,
            height: 480,
            scale: 0.75,
            zoom: 1.25,
            zoom_start: 0.5,
            zoom_target: 2.0,
            interpolating: true,
            fraction: 0.5,
            stamp: 0x1122_3344_5566_7788,
            mode: 2,
            animating: true,
            direction: -1,
            word_afc: 0xdead_beef,
            word_b00: 0xfeed_face,
        };
        state.a61_reset_interpolation();
        assert_eq!(state.zoom, 1.25);
        assert_eq!(state.zoom_start, 1.25);
        assert_eq!(state.zoom_target, 1.25);
        assert_eq!(state.fraction, 1.0);
        assert!(!state.interpolating);
        assert!(!state.animating);
        assert_eq!(state.word_afc, 0);
        assert_eq!(state.word_b00, 0);
        // Not part of the row.
        assert_eq!(state.mode, 2);
        assert_eq!(state.scale, 0.75);
        assert_eq!(state.stamp, 0x1122_3344_5566_7788);
        assert_eq!(state.direction, -1);
        assert_eq!(state.width, 640);
        assert_eq!(state.height, 480);
        assert_eq!(state.scaled_height, 480);
    }

    /// `0x0060d030`: bit 0 of the subsystem-5 byte.
    #[test]
    fn subsystem5_marker_sets_only_the_low_bit() {
        let mut flags = 0u8;
        assert!(a61_mark_subsystem5(&mut flags));
        assert_eq!(flags, 1);
        assert!(!a61_mark_subsystem5(&mut flags));
        assert_eq!(flags, 1);

        let mut other = 0x40u8;
        assert!(a61_mark_subsystem5(&mut other));
        assert_eq!(other, 0x41);
        assert!(!a61_mark_subsystem5(&mut other));
        assert_eq!(other, 0x41);
    }

    /// `0x0060d050`: release order, null skipping, zeroing and call order.
    #[test]
    fn map_release_frees_in_order_and_clears_every_word() {
        let buffers = A61MapBuffers {
            flags: 0x1000,
            buffer68: 0,
            tile_flags: 0x3000,
            tile_data: 0x4000,
            light: 0x5000,
            misc9c: 0x6000,
            word_5b74: 1,
            word_5b68: 2,
            word_3020: 3,
            word_9470: 4,
            word_9474: 5,
        };
        let released = a61_release_map(buffers);
        assert_eq!(released.freed, vec![0x1000, 0x3000, 0x4000, 0x5000]);
        assert_eq!(released.buffers, A61MapBuffers::default());
        assert_eq!(released.steps, A61_TEARDOWN_STEPS.to_vec());
        assert_eq!(released.steps.first(), Some(&0x0075_d1d0));
        assert_eq!(released.steps.last(), Some(&0x006c_31f0));
        // `0x00fa949c` is cleared but never released.
        assert!(!released.freed.contains(&0x6000));

        let empty = a61_release_map(A61MapBuffers::default());
        assert!(empty.freed.is_empty());
        assert_eq!(empty.buffers, A61MapBuffers::default());
    }

    /// `0x0060d190` / `0x0060d8e0`: the two raw getters.
    #[test]
    fn view_getters_read_the_recovered_words() {
        let state = A61View {
            scaled_height: 0x7fff_0001,
            height: -2,
            ..view()
        };
        assert_eq!(state.a61_scaled_height(), 0x7fff_0001);
        assert_eq!(state.a61_height(), -2);
    }

    /// `0x0060d1a0`: the border-tile size formula and the init record.
    #[test]
    fn tile_buffer_size_and_map_creation() {
        assert_eq!(a61_tile_buffer_size(640, 480), 344_064);
        // (n + 31) / 32 + 1, so even a 1x1 map needs a 2x2 block border.
        assert_eq!(a61_tile_buffer_size(0, 0), 0x400);
        assert_eq!(a61_tile_buffer_size(1, 1), 4 * 0x400);
        assert_eq!(a61_tile_buffer_size(32, 32), 4 * 0x400);
        assert_eq!(a61_tile_buffer_size(33, 33), 9 * 0x400);

        let created = a61_create_map_buffers(None, 0x1234, 0x5678, 640, 480);
        assert!(created.created);
        assert_eq!(created.object, 0x1234);
        assert_eq!(created.field_490, 0x5678);
        assert_eq!(created.word_1177478, 0);
        assert_eq!(created.byte_11b8678, 0);
        assert_eq!(created.light_size, 0x20000);
        assert_eq!(created.tile_data_size, 0x20000);
        assert_eq!(created.tile_flags_size, 344_064);
        assert_eq!(created.buffer_size, 0x40000);
        assert_eq!(created.buffer_cleared, 0x40000);

        let existing = a61_create_map_buffers(Some((0x99, 0xaa)), 0x1234, 0x5678, 0, 0);
        assert!(!existing.created);
        assert_eq!(existing.object, 0x99);
        assert_eq!(existing.field_490, 0xaa);
        assert_eq!(existing.tile_flags_size, 0x400);
    }

    /// `0x0060d6b0`: the three sizes/dimensions and the optional UI branch.
    #[test]
    fn terrain_rebuild_reports_sizes_and_branches() {
        let none = a61_terrain_rebuild(None, 16, 8, false);
        assert_eq!(
            none,
            A61TerrainRebuild {
                light_bytes_cleared: 0x20000,
                tile_bytes: 0,
                width_tiles: 16,
                height_tiles: 8,
                ui_rebuilt: false,
                water_rebuilt: true,
            }
        );

        let some = a61_terrain_rebuild(Some((320, 200)), 16, 8, true);
        assert_eq!(some.tile_bytes, 128_000);
        assert!(some.ui_rebuilt);

        let wrapped = a61_terrain_rebuild(Some((0x1_0000, 0x1_0000)), 0, 0, false);
        assert_eq!(wrapped.tile_bytes, 0);
    }

    /// `0x0060d750`: the low/high viewport range and its three clamps.
    #[test]
    fn view_pixel_range_matches_the_shipped_clamps() {
        assert_eq!(a61_view_pixel_range(480, 10, 64, false), None);
        assert_eq!(a61_view_pixel_range(480, 10, 64, true), Some((6, 63)));
        assert_eq!(a61_view_pixel_range(480, 2, 64, true), Some((0, 63)));
        assert_eq!(a61_view_pixel_range(0, 100, 50, true), Some((96, 49)));
        assert_eq!(a61_view_pixel_range(0, 0, 0, true), Some((0, -1)));
        // high = -1 + 4 + 4 = 7, which is below the 0xffff map height.
        assert_eq!(a61_view_pixel_range(-1, 4, 0xffff, true), Some((0, 7)));
    }

    /// `FUN_00c9c5ae` and `0x0060d940`: the round-away helper feeds a
    /// truncating cast, so the scaled height is a ceiling.
    #[test]
    fn scaled_height_rounds_away_from_zero() {
        assert_eq!(a61_round_away(0.0), 0.0);
        assert_eq!(a61_round_away(-0.0), -0.0);
        assert_eq!(a61_round_away(1.0), 1.0);
        assert_eq!(a61_round_away(-2.0), -2.0);
        assert_eq!(a61_round_away(0.25), 1.25);
        assert_eq!(a61_round_away(0.5), 1.5);
        assert_eq!(a61_round_away(-0.5), -1.5);
        assert_eq!(a61_round_away(1.25), 2.25);
        assert_eq!(a61_round_away(1.5), 2.5);
        assert_eq!(a61_round_away(2.5), 3.5);
        assert_eq!(a61_round_away(-1.25), -2.25);
        assert_eq!(a61_round_away(-1.5), -2.5);
        assert_eq!(a61_round_away(-2.5), -3.5);
        assert_eq!(a61_round_away(8_388_608.0), 8_388_608.0);
        assert!(a61_round_away(f32::INFINITY).is_infinite());
        assert!(a61_round_away(f32::NAN).is_nan());

        assert_eq!(a61_scaled_height(480, 1.0), 480);
        assert_eq!(a61_scaled_height(481, 0.5), 241);
        assert_eq!(a61_scaled_height(480, 0.5), 240);
        assert_eq!(a61_scaled_height(1, 0.5), 1);
        assert_eq!(a61_scaled_height(3, 0.5), 2);
        // Negative values round away from zero, so -1.5 truncates to -2.
        assert_eq!(a61_scaled_height(-3, 0.5), -2);
        assert_eq!(a61_scaled_height(-1, 0.5), -1);
        assert_eq!(a61_scaled_height(-480, 0.5), -240);
        assert_eq!(a61_cvttss2si(f32::NAN), i32::MIN);
        assert_eq!(a61_cvttss2si(2_147_483_648.0), i32::MIN);
    }

    /// `0x0060d8f0`: the two SSE clamps, including the NaN result.
    #[test]
    fn set_scale_clamps_to_half_and_one() {
        let mut state = A61View {
            width: 640,
            height: 480,
            scaled_height: 480,
            ..view()
        };
        assert_eq!(state.a61_set_scale(0.3), 0.5);
        assert_eq!(state.scale, 0.5);
        assert_eq!(state.scaled_height, 240);

        assert_eq!(state.a61_set_scale(0.75), 0.75);
        assert_eq!(state.scale, 0.75);
        assert_eq!(state.scaled_height, 360);

        assert_eq!(state.a61_set_scale(1.5), 1.0);
        assert_eq!(state.scale, 1.0);
        assert_eq!(state.scaled_height, 480);

        assert_eq!(state.a61_set_scale(f32::NAN), 0.5);
        assert_eq!(state.scale, 0.5);
    }

    /// `0x0060d940`: the early return and the two positive guards.
    #[test]
    fn apply_size_updates_only_positive_arguments() {
        let mut state = view();
        let first = state.a61_apply_size(640, 480);
        assert!(!first.early_return);
        assert_eq!(
            (first.width, first.height, first.scaled_height),
            (640, 480, 480)
        );

        let same = state.a61_apply_size(640, 480);
        assert!(same.early_return);
        assert_eq!(
            (state.width, state.height, state.scaled_height),
            (640, 480, 480)
        );

        let width_zero = state.a61_apply_size(0, 481);
        assert!(!width_zero.early_return);
        assert_eq!((state.width, state.height), (640, 481));
        assert_eq!(state.scaled_height, 481);

        let negatives = state.a61_apply_size(-5, -5);
        assert!(!negatives.early_return);
        // Negative values never replace the stored size.
        assert_eq!((state.width, state.height), (640, 481));
        assert_eq!(state.scaled_height, 481);

        // A negative height still feeds the scaled value into the early
        // comparison, so the next identical call is not an early return.
        let repeat = state.a61_apply_size(-5, -5);
        assert!(!repeat.early_return);
    }

    /// `0x0060d9c0`: `mode = 1` plus the full reset around the base value.
    #[test]
    fn reset_zoom_from_base_sets_mode_one() {
        let mut state = A61View {
            mode: 0,
            zoom: 0.3,
            zoom_start: 9.0,
            zoom_target: 8.0,
            fraction: 0.2,
            interpolating: true,
            animating: true,
            word_afc: 7,
            word_b00: 9,
            ..view()
        };
        assert_eq!(state.a61_reset_zoom_from_base(1.25), 1.25);
        assert_eq!(state.mode, 1);
        assert_eq!(state.zoom, 1.25);
        assert_eq!(state.zoom_start, 1.25);
        assert_eq!(state.zoom_target, 1.25);
        assert_eq!(state.fraction, 1.0);
        assert!(!state.interpolating);
        assert!(!state.animating);
        assert_eq!(state.word_afc, 0);
        assert_eq!(state.word_b00, 0);
    }

    /// `0x0060da50`: guards, the epsilon snap and both directions.
    #[test]
    fn request_zoom_snaps_then_requests_a_direction() {
        let mut state = view();
        state.zoom = 1.0;

        assert_eq!(
            state.a61_request_zoom(1.5, true, false),
            A61ZoomAction::Ignored
        );
        assert_eq!(
            state.a61_request_zoom(1.5, false, true),
            A61ZoomAction::Ignored
        );

        assert_eq!(
            state.a61_request_zoom(1.0 + A61_EPS * 0.5, false, false),
            A61ZoomAction::Snap {
                target: 1.0 + A61_EPS * 0.5
            }
        );
        assert_eq!(
            state.a61_request_zoom(1.5, false, false),
            A61ZoomAction::Request {
                direction: 1,
                amount: 1.5,
                mode: 1
            }
        );
        assert_eq!(
            state.a61_request_zoom(0.5, false, false),
            A61ZoomAction::Request {
                direction: -1,
                amount: 0.5,
                mode: 1
            }
        );
        // The `setbe` direction test selects +1 for an unordered compare.
        match state.a61_request_zoom(f32::NAN, false, false) {
            A61ZoomAction::Request {
                direction,
                amount,
                mode,
            } => {
                assert_eq!((direction, mode), (1, 1));
                assert!(amount.is_nan());
            }
            other => panic!("expected NaN request, got {other:?}"),
        }
    }

    /// `0x0060dad0` / `0x0060db10`: the animation-direction guards and the
    /// two fixed amounts.
    #[test]
    fn zoom_shortcuts_block_matching_animations() {
        let mut state = view();
        assert_eq!(
            state.a61_zoom_in(false, false),
            A61ZoomAction::Request {
                direction: 1,
                amount: 2.0,
                mode: 1
            }
        );
        assert_eq!(
            state.a61_zoom_out(false, false),
            A61ZoomAction::Request {
                direction: -1,
                amount: 0.25,
                mode: 1
            }
        );

        state.animating = true;
        state.direction = 1;
        assert_eq!(state.a61_zoom_in(false, false), A61ZoomAction::Ignored);
        assert_eq!(
            state.a61_zoom_out(false, false),
            A61ZoomAction::Request {
                direction: -1,
                amount: 0.25,
                mode: 1
            }
        );
        state.direction = -1;
        assert_eq!(state.a61_zoom_out(false, false), A61ZoomAction::Ignored);
        assert_eq!(
            state.a61_zoom_in(false, false),
            A61ZoomAction::Request {
                direction: 1,
                amount: 2.0,
                mode: 1
            }
        );

        state.animating = false;
        assert_eq!(state.a61_zoom_in(true, false), A61ZoomAction::Ignored);
        assert_eq!(state.a61_zoom_out(false, true), A61ZoomAction::Ignored);
    }

    /// `0x0060db50`: the start path, the reversal path and the interpolated
    /// recomputation.
    #[test]
    fn wheel_zoom_starts_resets_and_interpolates() {
        let mut state = A61View {
            zoom: 1.0,
            zoom_start: 0.75,
            zoom_target: 1.0,
            fraction: 0.5,
            ..view()
        };
        let started = state.a61_wheel(0.5, 1, |x| x, 0x1234_5678_9abc_def0, false, false);
        assert_eq!(started, A61WheelOutcome::Update { request: 0.75 });
        assert!(state.interpolating);
        assert_eq!(state.zoom_target, 1.5);
        assert_eq!(state.fraction, 0.0);
        assert_eq!(state.mode, 1);
        assert_eq!(state.stamp, 0x1234_5678_9abc_def0);

        // Reversal: target above start and a negative notch.
        let reset = state.a61_wheel(-0.1, 2, |x| x, 0, false, false);
        assert_eq!(reset, A61WheelOutcome::Reset { request: 1.0 });
        assert_eq!(state.zoom_start, 1.0);
        assert_eq!(state.zoom_target, 1.0);
        assert_eq!(state.fraction, 1.0);
        assert_eq!(state.mode, 2);

        // Normal interpolation: the fraction is rescaled against the new
        // target before the request is formed.
        let mut interp = A61View {
            zoom: 1.2,
            zoom_start: 1.0,
            zoom_target: 1.5,
            fraction: 0.4,
            interpolating: true,
            ..view()
        };
        let outcome = interp.a61_wheel(0.1, 3, |x| x, 0, false, false);
        assert!(close(interp.zoom_target, 1.65));
        assert!(close(interp.fraction, 0.2 / 0.65));
        match outcome {
            A61WheelOutcome::Update { request } => {
                assert!(close(request, 1.0 + (0.2 / 0.65) * 0.65));
            }
            other => panic!("expected update, got {other:?}"),
        }
        assert_eq!(interp.mode, 3);
    }

    /// `0x0060db50`: the zero-denominator case and the clamp to zero.
    #[test]
    fn wheel_zoom_handles_equal_targets_and_negative_ratios() {
        let mut equal = A61View {
            zoom: 1.0,
            zoom_start: 1.0,
            zoom_target: 1.2,
            fraction: 0.5,
            interpolating: true,
            ..view()
        };
        let outcome = equal.a61_wheel(0.0, 1, |_| 1.0, 0, false, false);
        assert_eq!(outcome, A61WheelOutcome::Update { request: 1.0 });
        assert_eq!(equal.fraction, 1.0);
        assert_eq!(equal.zoom_target, 1.0);

        let mut negative = A61View {
            zoom: 1.5,
            zoom_start: 1.0,
            zoom_target: 1.5,
            fraction: 0.5,
            interpolating: true,
            ..view()
        };
        let outcome = negative.a61_wheel(0.5, 1, |_| 0.5, 0, false, false);
        assert_eq!(outcome, A61WheelOutcome::Update { request: 1.0 });
        assert_eq!(negative.fraction, 0.0);

        // A running animation ignores the notch and leaves mode untouched.
        let mut animated = A61View {
            animating: true,
            mode: 7,
            zoom_target: 2.0,
            ..view()
        };
        assert_eq!(
            animated.a61_wheel(1.0, 9, |x| x, 5, false, false),
            A61WheelOutcome::Ignored
        );
        assert_eq!(animated.mode, 7);
        assert_eq!(animated.zoom_target, 2.0);
    }

    /// `0x0060dce0`: the table scan, its out-of-range return and the steps
    /// offset clamp.
    #[test]
    fn zoom_step_selects_the_recovered_table_entry() {
        let mut state = A61View {
            zoom_start: 1.0,
            zoom_target: 1.0,
            fraction: 0.0,
            zoom: 1.0,
            ..view()
        };
        assert_eq!(
            state.a61_zoom_step(1.0, false, false),
            A61StepOutcome::Request {
                direction: 1,
                target: 1.5
            }
        );
        state.zoom = 1.5;
        assert_eq!(
            state.a61_zoom_step(1.0, false, false),
            A61StepOutcome::Commit { target: 1.5 }
        );

        // 2.0 is the special-cased index 0.
        state.zoom_start = 2.0;
        state.zoom_target = 2.0;
        state.zoom = 1.0;
        assert_eq!(
            state.a61_zoom_step(0.0, false, false),
            A61StepOutcome::Request {
                direction: 1,
                target: 2.0
            }
        );

        // A 0.5 zoom is table index 4.  Negative step counts raise the
        // index, positive ones lower it and the result clamps to [0, 6].
        state.zoom = 0.0;
        state.zoom_start = 0.5;
        state.zoom_target = 0.5;
        assert_eq!(
            state.a61_zoom_step(0.0, false, false),
            A61StepOutcome::Request {
                direction: 1,
                target: 0.5
            }
        );
        assert_eq!(
            state.a61_zoom_step(-2.0, false, false),
            A61StepOutcome::Request {
                direction: 1,
                target: 0.25
            }
        );
        assert_eq!(
            state.a61_zoom_step(3.0, false, false),
            A61StepOutcome::Request {
                direction: 1,
                target: 1.5
            }
        );
        assert_eq!(
            state.a61_zoom_step(5.0, false, false),
            A61StepOutcome::Request {
                direction: 1,
                target: 2.0
            }
        );
        assert_eq!(
            state.a61_zoom_step(-10.0, false, false),
            A61StepOutcome::Request {
                direction: 1,
                target: 0.25
            }
        );

        // Table boundaries.
        state.zoom_start = 1.6;
        state.zoom_target = 1.6;
        assert_eq!(
            state.a61_zoom_step(0.0, false, false),
            A61StepOutcome::Request {
                direction: 1,
                target: 1.5
            }
        );
        state.zoom_start = 0.25;
        state.zoom_target = 0.25;
        assert_eq!(
            state.a61_zoom_step(0.0, false, false),
            A61StepOutcome::Request {
                direction: 1,
                target: 0.25
            }
        );

        // Outside the table the row returns without a request.
        state.zoom_start = 2.5;
        state.zoom_target = 2.5;
        assert_eq!(
            state.a61_zoom_step(0.0, false, false),
            A61StepOutcome::NoMatch
        );
        state.zoom_start = 0.2;
        state.zoom_target = 0.2;
        assert_eq!(
            state.a61_zoom_step(0.0, false, false),
            A61StepOutcome::NoMatch
        );

        // The pause/UI gate is checked after the index calculation.
        state.zoom_start = 1.0;
        state.zoom_target = 1.0;
        assert_eq!(
            state.a61_zoom_step(1.0, true, false),
            A61StepOutcome::Ignored
        );
        assert_eq!(
            state.a61_zoom_step(1.0, false, true),
            A61StepOutcome::Ignored
        );
        state.zoom_start = 2.5;
        state.zoom_target = 2.5;
        assert_eq!(
            state.a61_zoom_step(0.0, true, false),
            A61StepOutcome::NoMatch
        );

        // An unordered interpolated zoom also fails the `comiss` special
        // case and the scan, so it returns without a request.
        let non_finite = A61View {
            fraction: f32::NAN,
            ..view()
        };
        assert_eq!(
            non_finite.a61_zoom_step(0.0, false, false),
            A61StepOutcome::NoMatch
        );

        // A NaN step count truncates to 0x80000000, the wrapped offset is
        // negative and the selection clamps to index 0 (2.0).
        state.zoom = 1.0;
        state.zoom_start = 1.0;
        state.zoom_target = 1.0;
        assert_eq!(
            state.a61_zoom_step(f32::NAN, false, false),
            A61StepOutcome::Request {
                direction: 1,
                target: 2.0
            }
        );
    }

    /// The step table itself.
    #[test]
    fn zoom_step_table_matches_the_shipped_constants() {
        assert_eq!(A61_ZOOM_STEPS, [2.0, 1.5, 1.0, 0.75, 0.5, 0.33, 0.25]);
        assert_eq!(A61_ZOOM_STEPS[5].to_bits(), 0x3ea8_f5c3);
        assert_eq!(A61_EPS.to_bits(), 0x3480_0000);
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// * `FUN_0060d42b` (`FUN_0060d42b`, 34 bytes) starts in the middle of the
//   `push 0` at `0x0060d42a`; the decompiler decodes `add [edi], cl`,
//   `in al, dx`, an LDT read and other instructions that cannot come from
//   this function's real body.  It is the tail of the long initialiser whose
//   ledger row is `0x0060d1a0`, not a function entry.
// * `FUN_0060d5c7` (`FUN_0060d5c7`, 32 bytes) is likewise a mid-body fragment:
//   `push esi; mov ecx, eax; call [eax+0x10]; mov fs:[0] ...` followed by the
//   `FUN_00c86b6e` security-cookie epilogue.  There is no independent entry
//   point to translate.
// * `FUN_0060d7b0` (`FUN_0060d7b0`, 84 bytes) is truncated by the decompiler at
//   a call to `FUN_007b3e10` that it mislabels `noreturn`.  The machine code
//   continues past `0x0060d810` with a second coordinate transform, a
//   screen-size multiply and further untranslated calls, so the recorded
//   body is not the whole row.
// * `FUN_0060da5f` (`FUN_0060da5f`, 107 bytes) lies inside `0x0060da50`
//   (which ends at `0x0060dac9`).  It is the same body reached at its real
//   entry, not a second function; translating it separately would duplicate
//   `0x0060da50`.
