#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent78.rs
// ===========================================================================
// Translation batch 78.  Only this file is edited by the batch.
//
// Batch 78 owns `0x0062e000..=0x0062ffff`.  The ledger lists 55
// `decompiled_uncited` rows in that window; 26 are translated below and the
// other 29 are exception-cleanup fragments, `int3` padding, mid-instruction
// tails or bodies whose only remaining work is an indirect call into a
// routine outside the window.  The skip reasons are recorded in the
// [batch skip notes](#batch-skip-notes) at the end of this file.
//
// The window is a terrain-surface reader plus the MSVC container support its
// property objects instantiate.  Two records dominate:
//
// ## The 8-byte quad record (`0x0062e370`, `0x0062e330`, `0x0062e4b0`)
//
// | offset | meaning |
// |--------|---------|
// | `+0x00` | `u16` low bound, compared against `+0x02` by `0x0062e4b0` |
// | `+0x02` | `u16` high bound |
// | `+0x04` | 16 packed 2-bit fields, one per 4x4 sub-cell |
//
// The record index is `(width / 4 * (y / 4) + x / 4) * 8`.  `0x0062e330`
// selects the 2-bit field at `((x % 4) + (y % 4) * 4) * 2` inside the dword
// at `+0x04`; the `%` is C truncating remainder, so a negative coordinate
// wraps through the byte-sized intermediate exactly as the shipped code does.
//
// ## The 16-byte tile record (`0x0062e3b0`, `0x0062e3f0`, `0x0062e780`)
//
// | offset | meaning |
// |--------|---------|
// | `+0x00` | `u16` first palette selector, low byte also the first blend byte |
// | `+0x02` | `u16` second palette selector, low byte the second blend byte |
// | `+0x0c` | 16 packed 2-bit fields, read as `0x0062e330(record + 8, …)` |
//
// The record index is `((x / 4) + (width / 4) * (y / 4)) * 0x10`.
//
// ## The surface object (`in_ECX`)
//
// | offset | meaning |
// |--------|---------|
// | `+0x04` | base address of the pixel/record block |
// | `+0x10` | width |
// | `+0x14` | height |
// | `+0x18` | row stride in bytes |
//
// `0x0062e550` and `0x0062e840` address pixels as
// `base + stride * y + x * 4`; `0x0062e510`, `0x0062e3f0` and `0x0062e780`
// read the 16-byte tile record instead.
//
// ## Recovered colour helpers used by the window
//
// `0x0062e180`, `0x0062e720`, `0x0062e780` and `0x0062e840` call the shipped
// colour routines `FUN_00a8f9d0` (scale a 4-float vector), `FUN_00a8fa00` (add
// two 4-float vectors), `FUN_00a8fb10` (four bytes over 255 into a 4-float
// vector) and `FUN_00a8fb80` (copy RGB, overwrite alpha).  Every one of those
// bodies is short and fully recovered, so the translations perform the same
// `f32` multiplies, adds and divides directly and cite the helper address in
// the comment of the method that uses it.
//
// Nothing here reads or writes simulation state; every translation is a
// deterministic [`World`] helper over value types.

// ---------------------------------------------------------------------------
// Recovered constants
// ---------------------------------------------------------------------------

/// `0x3f2aaaab`, the factor passed to `FUN_00a8f9d0` by `0x0062e180` for the
/// two-thirds share of a palette blend.
pub const A78_TWO_THIRDS: f32 = f32::from_bits(0x3f2a_aaab);

/// `0x3eaaaaab`, the one-thirds factor of `0x0062e180`.
pub const A78_ONE_THIRD: f32 = f32::from_bits(0x3eaa_aaab);

/// `0x3f000000`, the half factor used by the `param_2[1] >= *param_2` path of
/// `0x0062e180`.
pub const A78_HALF: f32 = f32::from_bits(0x3f00_0000);

/// Capacity word written by the empty `std::string` initialiser in
/// `0x0062ecb0`: top bit set (inline storage) and 15 usable bytes.
pub const A78_SSO_CAP: u32 = 0x8000_000f;

/// Inline byte count of an SSO string (`A78_SSO_CAP & 0x7fffffff`).
pub const A78_SSO_INLINE: u32 = 0x0000_000f;

/// `0x00f3ca68` is the four-dword default colour copied by `0x0062e820`,
/// `0x0062e720`, `0x0062e780` and `0x0062e840`.  The value itself lives in
/// the data section and is supplied by the caller.
pub const A78_DEFAULT_COLOR_SITE: u32 = 0x00f3_ca68;

/// `0x00d33180` holds the two `double`s `0.0` and `4294967296.0`; `0x0062f850`
/// indexes it with `-(value >> 31)` to widen an unsigned 32-bit payload.
pub const A78_U32_BIAS_SITE: u32 = 0x00d3_3180;

/// `0x00f05618`, the static object returned by `0x0062fd60`.
pub const A78_TLS_STATIC: u32 = 0x00f0_5618;

/// Tag written by the byte-valued `push` at `0x0062f0d0` (`local_30 = 1`).
pub const A78_TAG_BYTE: u32 = 1;

/// Tag written by the dword-valued `push` at `0x0062edb0` (`local_30 = 2`).
pub const A78_TAG_DWORD: u32 = 2;

/// Tag written by the string-valued `push` at `0x0062efc0` (`local_30 = 5`).
pub const A78_TAG_STRING: u32 = 5;

// ---------------------------------------------------------------------------
// Recovered value types
// ---------------------------------------------------------------------------

/// One node of the chained hash table walked by `0x0062fcf0` / `0x0062fd00`.
///
/// The shipped node stores `next` at `+0x00` and the key block at `+0x04`;
/// `FUN_00539640` compares the query key with that block.  The translations
/// compare a caller-supplied `u32` key instead, because the length-dependent
/// string compare lives outside the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A78ChainNode {
    /// `+0x00`: next node index, `None` for the null link.
    pub next: Option<u32>,
    /// `+0x04`: recovered key block (retail compares 4 bytes).
    pub key: u32,
}

/// Header written by the empty-string initialiser of `0x0062ecb0`.
///
/// Retail stores `this + 0xc` into `+0x00`; the header type records only the
/// three scalar members and the first inline byte, so the pointer is not
/// modelled.  Everything else matches the shipped stores exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A78StringInit {
    /// `+0x04`: stored byte count, always zero after the initialiser.
    pub len: u32,
    /// `+0x08`: capacity word with the inline-storage bit.
    pub cap: u32,
    /// `+0x0c`: first inline byte.
    pub first: u8,
}

/// Result of `0x0062ecb0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A78StringRange {
    /// Empty header written before the range is assigned.
    pub header: A78StringInit,
    /// `param_3 - param_2`, clamped to zero when `param_3 < param_2`.  The
    /// value is the recovered range length; it gates the tail call.
    pub range_len: i32,
}

/// Tagged value pushed through the visitor protocol of
/// `0x0062edb0` / `0x0062efc0` / `0x0062f0d0` / `0x0062f025`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum A78Variant {
    /// `0x0062f0d0`: tag 1, one payload byte at `+0x04`.
    Byte(u8),
    /// `0x0062edb0`: tag 2, one payload dword at `+0x04`.
    Dword(u32),
    /// `0x0062efc0`: tag 5, twelve payload bytes holding an MSVC string.
    Text(Vec<u8>),
    /// `0x0062f025`: the variant lives in the caller's frame; only the tag is
    /// observed by the wrapper.
    Caller(u32),
}

impl A78Variant {
    /// Variant tag word written at `+0x00` (`local_30` in retail).
    pub fn tag(&self) -> u32 {
        match self {
            A78Variant::Byte(_) => A78_TAG_BYTE,
            A78Variant::Dword(_) => A78_TAG_DWORD,
            A78Variant::Text(_) => A78_TAG_STRING,
            A78Variant::Caller(tag) => *tag,
        }
    }
}

/// Outcome of one push through the visitor protocol.
///
/// Retail returns a byte: `0` when the visitor refused the value, `1`
/// otherwise.  The three success/rejection shapes below carry the flag byte at
/// `+0x18` that the wrapper maintains.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A78Push {
    /// Return byte: `false` for the visitor-refusal path, `true` otherwise.
    pub accepted: bool,
    /// `false` when the tag dispatch routed the value instead of applying it.
    pub pushed: bool,
    /// `true` when a previously armed flag was re-applied through
    /// `FUN_0062f1d0`.
    pub rearmed: bool,
    /// Value left in the `+0x18` flag byte.
    pub flag: bool,
}

/// Numeric payload class selected by `0x0062f850` from the flags word at
/// `+0x0e` of the source variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A78NumberKind {
    /// `0x0200`: the payload already is a `double` (`*pdVar3`).
    F64,
    /// `0x0020`: `(double)*(int *)payload`.
    I32,
    /// `0x0040`: unsigned 32-bit, widened with the `0x00d33180` bias.
    U32,
    /// `0x0080`: the wide conversion at `FUN_00c87160`.  Both wide conversions
    /// decompile to empty bodies (`void f(void) { return; }`), so the signed
    /// width is inferred from the pairing with the unsigned arm.
    I64,
    /// Otherwise: the wide conversion at `FUN_00c87120` (same caveat).
    U64,
}

/// Action selected by `0x0062fdc0` after the value extraction succeeds or
/// fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A78DefaultAction {
    /// `+0x48` was null: the wrapper did nothing.
    Missing,
    /// `0x0062f850` returned non-zero; only `+0x60` was cleared.
    Extracted,
    /// Kinds 1..=5 fall into the `FUN_0053a640` noreturn tail.
    Failed(u32),
    /// Default arm: lock `+0x24`, set `+0x60`, unlock.
    Restored,
}

/// Full result of `0x0062fdc0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A78DefaultOutcome {
    /// `+0x60` is cleared as soon as `+0x48` is non-null.
    pub cleared: bool,
    /// What the wrapper did next.
    pub action: A78DefaultAction,
}

/// Result of the static-guard sequence in `0x0062fd60`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A78TlsStatic {
    /// `tls_block[0xc] < DAT_00faa1c8`: the per-thread header needs building.
    pub init_header: bool,
    /// `DAT_00faa1c8 == -1` after the header call: register the destructor.
    pub register_destructor: bool,
    /// Address of the guarded static (`0x00f05618`).
    pub address: u32,
}

// ---------------------------------------------------------------------------
// Shared arithmetic helpers
// ---------------------------------------------------------------------------

/// Recovered `(value + (value >> 31 & 3)) >> 2` idiom: C truncating division
/// by four, used by the two record-index builders.
fn a78_div4(value: i32) -> i32 {
    (value.wrapping_add((value >> 31) & 3)) >> 2
}

/// The four-sevenths / three-sevenths narrowing of `0x0062e590`, written
/// exactly as the shipped body writes it.
///
/// The shipped body is `(((v - v / 7) >> 1) + v / 7) >> 2`; for every
/// `0 <= v <= 0xffff` that equals `v / 7` (asserted in the tests), and the
/// expression is kept visible because the shift/add sequence is what the
/// decompiler recovered.
fn a78_sevenths(value: u32) -> u32 {
    let q = value / 7;
    (((value - q) >> 1) + q) >> 2
}

/// Scale two 4-float colours and sum them, matching `FUN_00a8f9d0` (scale) and
/// `FUN_00a8fa00` (add): `first * first_weight + second * second_weight`.
fn a78_scaled_sum(
    first: [f32; 4],
    first_weight: f32,
    second: [f32; 4],
    second_weight: f32,
) -> [f32; 4] {
    let mut out = [0.0f32; 4];
    for (index, slot) in out.iter_mut().enumerate() {
        *slot = first[index] * first_weight + second[index] * second_weight;
    }
    out
}

impl World {
    // -----------------------------------------------------------------------
    // Terrain-surface readers
    // -----------------------------------------------------------------------

    /// `0x0062e330` (`FUN_0062e330`, 19 bytes, `decompiled_uncited`).
    ///
    /// Extracts one 2-bit field from the packed quadrant word:
    ///
    /// ```c
    /// param_3 = param_3 & 0x80000003;                  /* y */
    /// if ((int)param_3 < 0) param_3 = (param_3 - 1 | 0xfffffffc) + 1;
    /// param_2 = param_2 & 0x80000003;                  /* x */
    /// if ((int)param_2 < 0) param_2 = (param_2 - 1 | 0xfffffffc) + 1;
    /// return *(uint *)(param_1 + 4)
    ///        >> (((char)param_2 + (char)param_3 * 4) * 2 & 0x1f) & 3;
    /// ```
    ///
    /// The masking dance is C's truncating `% 4`; the byte-sized intermediate
    /// keeps the negative coordinates wrapping through the low five bits of
    /// the shift count.
    pub fn a78_quad_field(&self, word: u32, x: i32, y: i32) -> u32 {
        let x_mod = x % 4;
        let y_mod = y % 4;
        let shift = ((x_mod as i8 as i32 + (y_mod as i8 as i32) * 4) * 2) & 0x1f;
        (word >> (shift as u32)) & 3
    }

    /// `0x0062e370` (`FUN_0062e370`, 58 bytes, `decompiled_uncited`).
    ///
    /// Address of the 8-byte quadrant record holding pixel `(x, y)`:
    ///
    /// ```c
    /// return *(int *)(this + 4)
    ///      + ((width / 4) * (y / 4) + (x / 4)) * 8;
    /// ```
    ///
    /// All three divisions are the recovered truncating `>> 2` idiom, so
    /// negative coordinates round toward zero like the shipped code.
    pub fn a78_quad_index8(&self, base: i32, width: i32, x: i32, y: i32) -> i32 {
        let columns = a78_div4(width);
        let row = a78_div4(y);
        let column = a78_div4(x);
        base.wrapping_add(
            columns
                .wrapping_mul(row)
                .wrapping_add(column)
                .wrapping_mul(8),
        )
    }

    /// `0x0062e3b0` (`FUN_0062e3b0`, 58 bytes, `decompiled_uncited`).
    ///
    /// Address of the 16-byte tile record holding pixel `(x, y)`:
    ///
    /// ```c
    /// return ((x / 4) + (width / 4) * (y / 4)) * 0x10 + *(int *)(this + 4);
    /// ```
    pub fn a78_tile_index16(&self, base: i32, width: i32, x: i32, y: i32) -> i32 {
        let columns = a78_div4(width);
        let row = a78_div4(y);
        let column = a78_div4(x);
        column
            .wrapping_add(columns.wrapping_mul(row))
            .wrapping_mul(0x10)
            .wrapping_add(base)
    }

    /// `0x0062e590` (`FUN_0062e590`, 322 bytes, `decompiled_uncited`).
    ///
    /// Blends the two bytes at the head of a tile record into one byte:
    ///
    /// ```c
    /// if (b < a) { v = b * 3 + a * 4; v = (((v - v / 7) >> 1) + v / 7) >> 2; }
    /// else       { v = (a * 2 + b * 3) / 5; }
    /// return v > 0xff ? 0xff : (byte)v;
    /// ```
    ///
    /// The `b < a` branch is the four-sevenths / three-sevenths mix; the
    /// `>=` branch is the two-fifths / three-fifths mix.  Both are exact
    /// integer arithmetic and the clamp is retained even though the recovered
    /// weights cannot exceed 255.
    pub fn a78_blend_alpha_bytes(&self, first: u8, second: u8) -> u8 {
        let a = first as u32;
        let b = second as u32;
        let value = if b < a {
            a78_sevenths(b * 3 + a * 4)
        } else {
            (a * 2 + b * 3) / 5
        };
        if value > 0xff { 0xff } else { value as u8 }
    }

    /// `0x0062e650` (`FUN_0062e650`, 20 bytes, `decompiled_uncited`).
    ///
    /// The tail of a second byte pair blend.  The body reads three
    /// caller-frame registers and stores two bytes into the caller's frame
    /// (`[ebp - 5]`, `[ebp - 4]`); only the arithmetic is recoverable, so the
    /// three unnamed inputs stay explicit parameters:
    ///
    /// ```c
    /// [ebp-5] = min(whole / 7, 0xff);
    /// [ebp-4] = min((left + right * 6) / 7, 0xff);
    /// ```
    ///
    /// This is the seven-part split (`1 : 6`) recovered from the fragment; the
    /// caller-frame destination bytes and the enclosing routine are unknown,
    /// so no relationship to the `0x0062e590` weights is claimed.
    pub fn a78_blend_alpha_sevenths(&self, whole: u32, left: u32, right: u32) -> (u8, u8) {
        let low = whole / 7;
        let high = left.wrapping_add(right.wrapping_mul(6)) / 7;
        let clamp = |value: u32| if value > 0xff { 0xff } else { value as u8 };
        (clamp(low), clamp(high))
    }

    /// `0x0062e3f0` (`FUN_0062e3f0`, 180 bytes, `decompiled_uncited`).
    ///
    /// Blended byte of the 16-byte tile record for `(x, y)`:
    ///
    /// ```c
    /// iVar2 = ((x / 4) + (width / 4) * (y / 4)) * 0x10 + base;
    /// uVar4 = *(uint6 *)(iVar2 + 2);      /* discarded tail load */
    /// return FUN_0062e590(iVar2);         /* blend of bytes +0 and +1 */
    /// ```
    ///
    /// The six-byte load at `+0x02` and the `FUN_00c86b6e` tail are decompiler
    /// artefacts of the epilogue: `FUN_00c86b6e` is the 17-byte
    /// `__security_check_cookie` stub (`if (in_ECX == DAT_00f4e578) return;`).
    /// Every caller consumes the low byte of the return (`0x0062e510`,
    /// `0x0062e780`), which [`World::a78_blend_alpha_bytes`] supplies.  There
    /// is no bounds check in retail; `None` is returned when the record falls
    /// outside the supplied block.
    pub fn a78_tile_alpha(&self, data: &[u8], base: i32, width: i32, x: i32, y: i32) -> Option<u8> {
        let offset = self.a78_tile_index16(base, width, x, y);
        let at = usize::try_from(offset).ok()?;
        let first = *data.get(at)?;
        let second = *data.get(at.checked_add(1)?)?;
        Some(self.a78_blend_alpha_bytes(first, second))
    }

    /// `0x0062e4b0` (`FUN_0062e4b0`, 92 bytes, `decompiled_uncited`).
    ///
    /// Passability of one pixel:
    ///
    /// ```c
    /// if (x < width && y < height && x >= 0 && y >= 0) {
    ///   puVar1 = (ushort *)FUN_0062e370(x, y);
    ///   if (*puVar1 <= puVar1[1])
    ///     return FUN_0062e330(puVar1, x, y) != 3;
    ///   return 1;
    /// }
    /// return 0;
    /// ```
    ///
    /// So a quadrant whose low bound exceeds its high bound is always
    /// passable, and the packed 2-bit field marks blocked sub-cells with the
    /// value 3.  A record outside the supplied block is treated as absent.
    pub fn a78_tile_passable(
        &self,
        data: &[u8],
        base: i32,
        width: i32,
        height: i32,
        x: i32,
        y: i32,
    ) -> bool {
        if !(x < width && y < height && x >= 0 && y >= 0) {
            return false;
        }
        let offset = self.a78_quad_index8(base, width, x, y);
        let read = || -> Option<bool> {
            let at = usize::try_from(offset).ok()?;
            let low = u16::from_le_bytes([*data.get(at)?, *data.get(at.checked_add(1)?)?]);
            let high = u16::from_le_bytes([*data.get(at + 2)?, *data.get(at + 3)?]);
            if low > high {
                return Some(true);
            }
            let word = u32::from_le_bytes([
                *data.get(at + 4)?,
                *data.get(at + 5)?,
                *data.get(at + 6)?,
                *data.get(at + 7)?,
            ]);
            Some(self.a78_quad_field(word, x, y) != 3)
        };
        read().unwrap_or(false)
    }

    /// `0x0062e510` (`FUN_0062e510`, 54 bytes, `decompiled_uncited`).
    ///
    /// `threshold <= alpha(x, y)` using the blended tile byte:
    ///
    /// ```c
    /// if (x < width && y < height && x >= 0 && y >= 0) {
    ///   bVar1 = FUN_0062e3f0(x, y);
    ///   return param_3 <= bVar1;
    /// }
    /// return false;
    /// ```
    pub fn a78_tile_alpha_covers(
        &self,
        data: &[u8],
        base: i32,
        width: i32,
        height: i32,
        x: i32,
        y: i32,
        threshold: u8,
    ) -> bool {
        if !(x < width && y < height && x >= 0 && y >= 0) {
            return false;
        }
        match self.a78_tile_alpha(data, base, width, x, y) {
            Some(alpha) => threshold <= alpha,
            None => false,
        }
    }

    /// `0x0062e550` (`FUN_0062e550`, 64 bytes, `decompiled_uncited`).
    ///
    /// `threshold <= pixel_byte[3]` of the RGBA surface:
    ///
    /// ```c
    /// if (x < width && y < height && x >= 0 && y >= 0)
    ///   return param_3 <= *(byte *)(stride * y + x * 4 + 3 + base);
    /// return false;
    /// ```
    ///
    /// The `CONCAT31(... & 0xffffff00, …)` in the recovered body is the
    /// caller's stale upper `EAX`; the function really returns a `bool`.
    pub fn a78_tile_rgba_covers(
        &self,
        data: &[u8],
        base: i32,
        width: i32,
        height: i32,
        stride: i32,
        x: i32,
        y: i32,
        threshold: u8,
    ) -> bool {
        if !(x < width && y < height && x >= 0 && y >= 0) {
            return false;
        }
        let offset = (base as i64)
            .wrapping_add((stride as i64).wrapping_mul(y as i64))
            .wrapping_add((x as i64).wrapping_mul(4))
            .wrapping_add(3);
        match usize::try_from(offset) {
            Ok(at) => match data.get(at) {
                Some(byte) => *byte >= threshold,
                None => false,
            },
            Err(_) => false,
        }
    }

    /// `0x0062e840` (`FUN_0062e840`, 51 bytes, `decompiled_uncited`).
    ///
    /// Four-byte RGBA pixel widened to a 4-float colour by the shipped
    /// `FUN_00a8fb10`: every component is `byte / 255.0`:
    ///
    /// ```c
    /// puVar4 = (byte *)(stride * param_3 + param_2 * 4 + base);
    /// FUN_00a8fb10(param_1, puVar4[0], puVar4[1], puVar4[2], puVar4[3]);
    /// ```
    ///
    /// Out-of-range pixels copy the four default dwords from `0x00f3ca68`.
    pub fn a78_tile_rgba_color(
        &self,
        data: &[u8],
        base: i32,
        width: i32,
        height: i32,
        stride: i32,
        x: i32,
        y: i32,
        default: [f32; 4],
    ) -> [f32; 4] {
        if !(x < width && y < height && x >= 0 && y >= 0) {
            return default;
        }
        let offset = (base as i64)
            .wrapping_add((stride as i64).wrapping_mul(y as i64))
            .wrapping_add((x as i64).wrapping_mul(4));
        let at = match usize::try_from(offset) {
            Ok(at) => at,
            Err(_) => return default,
        };
        match data.get(at..at.saturating_add(4)) {
            Some(bytes) => [
                bytes[0] as f32 / 255.0,
                bytes[1] as f32 / 255.0,
                bytes[2] as f32 / 255.0,
                bytes[3] as f32 / 255.0,
            ],
            None => default,
        }
    }

    /// `0x0062e820` (`FUN_0062e820`, 20 bytes, `decompiled_uncited`).
    ///
    /// Copies the four default dwords into the caller's colour slot:
    ///
    /// ```c
    /// *param_1 = _DAT_00f3ca68;
    /// param_1[1] = uRam00f3ca6c;
    /// param_1[2] = uRam00f3ca70;
    /// param_1[3] = uRam00f3ca74;
    /// ```
    ///
    /// The dwords live at [`A78_DEFAULT_COLOR_SITE`] and are supplied by the
    /// caller; the translation is the identity on them.
    pub fn a78_tile_default_color(&self, default: [f32; 4]) -> [f32; 4] {
        default
    }

    /// `0x0062e180` (`FUN_0062e180`, 75 bytes, `decompiled_uncited`).
    ///
    /// Blends the two palette entries of one selector pair:
    ///
    /// ```c
    /// switch (mode) {
    /// case 0: return palette(sel[0]);
    /// case 1: return palette(sel[1]);
    /// case 2: if (sel[1] < sel[0])  return sel[1] * 2/3 + sel[0] * 1/3;
    ///         else                  return sel[1] * 1/2 + sel[0] * 1/2;
    /// case 3: if (sel[0] <= sel[1]) { out = default; out.a = 0; return out; }
    ///         else                  return sel[1] * 1/3 + sel[0] * 2/3;
    /// default: return the zeroed output slot;
    /// }
    /// ```
    ///
    /// The recovered order of the two `FUN_00a8f9d0` scales is preserved: the
    /// first scaled temporary follows the second palette constructor
    /// (`FUN_0062dec0` leaves `in_ECX` pointing at `sel[1]`'s colour), so the
    /// factors are listed with `colors[1]` first.  `colors` are the entries
    /// produced by the 5-6-5 decoder (`FUN_0062dec0` →
    /// `FUN_00a8fbb0(out, r, g, b)`, alpha `0x3f800000`); mode 0 and mode 1
    /// return them verbatim.
    ///
    /// Mode 3's first arm calls `FUN_00a8fb80(out, &0x00f3ca68, 0)`, which
    /// copies the default RGB and overwrites alpha with zero.
    pub fn a78_blend_palette_pair(
        &self,
        mode: u32,
        selectors: [u16; 2],
        colors: [[f32; 4]; 2],
        default: [f32; 4],
    ) -> [f32; 4] {
        match mode {
            0 => colors[0],
            1 => colors[1],
            2 => {
                if selectors[1] < selectors[0] {
                    a78_scaled_sum(colors[1], A78_TWO_THIRDS, colors[0], A78_ONE_THIRD)
                } else {
                    a78_scaled_sum(colors[1], A78_HALF, colors[0], A78_HALF)
                }
            }
            3 => {
                if selectors[0] <= selectors[1] {
                    [default[0], default[1], default[2], 0.0]
                } else {
                    a78_scaled_sum(colors[1], A78_ONE_THIRD, colors[0], A78_TWO_THIRDS)
                }
            }
            _ => [0.0; 4],
        }
    }

    /// `0x0062e720` (`FUN_0062e720`, 8 bytes, `decompiled_uncited`).
    ///
    /// Blended colour of the quadrant record holding `(x, y)`:
    ///
    /// ```c
    /// if (x < width && y < height && x >= 0 && y >= 0) {
    ///   rec  = FUN_0062e370(x, y);
    ///   mode = FUN_0062e330(rec, x, y);
    ///   return FUN_0062e180(rec, mode);
    /// }
    /// return 0x00f3ca68;
    /// ```
    ///
    /// `rec` holds the selector pair at `+0x00`/`+0x02` and the packed mode
    /// word at `+0x04`; the decompiler dropped the pointer argument of
    /// `0x0062e180` and printed only the mode word.
    pub fn a78_tile_quad_color<F>(
        &self,
        data: &[u8],
        base: i32,
        width: i32,
        height: i32,
        x: i32,
        y: i32,
        palette: F,
        default: [f32; 4],
    ) -> [f32; 4]
    where
        F: Fn(u16) -> [f32; 4],
    {
        if !(x < width && y < height && x >= 0 && y >= 0) {
            return default;
        }
        let offset = self.a78_quad_index8(base, width, x, y);
        let record = || -> Option<(u16, u16, u32)> {
            let at = usize::try_from(offset).ok()?;
            let selectors = [
                u16::from_le_bytes([*data.get(at)?, *data.get(at.checked_add(1)?)?]),
                u16::from_le_bytes([*data.get(at + 2)?, *data.get(at + 3)?]),
            ];
            let word = u32::from_le_bytes([
                *data.get(at + 4)?,
                *data.get(at + 5)?,
                *data.get(at + 6)?,
                *data.get(at + 7)?,
            ]);
            Some((selectors[0], selectors[1], word))
        };
        let (first, second, word) = match record() {
            Some(record) => record,
            None => return default,
        };
        let mode = self.a78_quad_field(word, x, y);
        self.a78_blend_palette_pair(
            mode,
            [first, second],
            [palette(first), palette(second)],
            default,
        )
    }

    /// `0x0062e780` (`FUN_0062e780`, 148 bytes, `decompiled_uncited`).
    ///
    /// Blended tile colour with the tile's blended alpha attached:
    ///
    /// ```c
    /// if (x < width && y < height && x >= 0 && y >= 0) {
    ///   rec   = FUN_0062e3b0(x, y);
    ///   mode  = FUN_0062e330(rec + 8, x, y);      /* dword at rec + 0xc */
    ///   blend = FUN_0062e180(rec, mode);          /* selectors rec + 0/+2 */
    ///   alpha = FUN_0062e3f0(x, y);               /* blend of rec + 0/+1 */
    ///   return FUN_00a8fb80(blend, (float)alpha / 255.0);
    /// }
    /// return 0x00f3ca68;
    /// ```
    ///
    /// `FUN_00a8fb80(out, colour, alpha)` copies the three RGB floats and
    /// replaces the fourth, so the recovered result keeps the blended RGB and
    /// takes `alpha / 255` as its alpha.
    pub fn a78_tile_quad_color_with_alpha<F>(
        &self,
        data: &[u8],
        base: i32,
        width: i32,
        height: i32,
        x: i32,
        y: i32,
        palette: F,
        default: [f32; 4],
    ) -> [f32; 4]
    where
        F: Fn(u16) -> [f32; 4],
    {
        if !(x < width && y < height && x >= 0 && y >= 0) {
            return default;
        }
        let offset = self.a78_tile_index16(base, width, x, y);
        let record = || -> Option<([u16; 2], u32, u8)> {
            let at = usize::try_from(offset).ok()?;
            let selectors = [
                u16::from_le_bytes([*data.get(at)?, *data.get(at.checked_add(1)?)?]),
                u16::from_le_bytes([*data.get(at + 2)?, *data.get(at + 3)?]),
            ];
            let word = u32::from_le_bytes([
                *data.get(at + 0x0c)?,
                *data.get(at + 0x0d)?,
                *data.get(at + 0x0e)?,
                *data.get(at + 0x0f)?,
            ]);
            let alpha = self.a78_blend_alpha_bytes(*data.get(at)?, *data.get(at + 1)?);
            Some((selectors, word, alpha))
        };
        let (selectors, word, alpha) = match record() {
            Some(record) => record,
            None => return default,
        };
        let mode = self.a78_quad_field(word, x, y);
        let mut blend = self.a78_blend_palette_pair(
            mode,
            selectors,
            [palette(selectors[0]), palette(selectors[1])],
            default,
        );
        blend[3] = alpha as f32 / 255.0;
        blend
    }

    // -----------------------------------------------------------------------
    // MSVC container support
    // -----------------------------------------------------------------------

    /// `0x0062ecb0` (`FUN_0062ecb0`, 85 bytes, `decompiled_uncited`).
    ///
    /// Range constructor of an SSO `std::string`:
    ///
    /// ```c
    /// param_1[1] = 0;                    /* +0x04 length */
    /// *param_1 = param_1 + 3;            /* +0x00 pointer to the inline buffer */
    /// param_1[2] = 0x8000000f;           /* +0x08 capacity with the SSO bit */
    /// *(byte *)(param_1 + 3) = 0;        /* +0x0c first inline byte */
    /// iVar2 = param_3 - param_2;
    /// if (param_3 < param_2) iVar2 = 0;  /* unsigned compare */
    /// if (iVar2 != 0) FUN_00538810(param_1[1] + 1);
    /// ```
    ///
    /// The returned [`A78StringRange`] carries the empty header and the
    /// recovered range length.  `FUN_00538810` is the 66-byte capacity request
    /// (`if ((cap & 0x7fffffff) < n) cap = (cap >> 1 & 0x3fffffff) + cap;`
    /// clamped up to `n`, then reallocate through `FUN_0093ec70`); the
    /// reallocation continuation is the same static-allocator accessor that the
    /// decompiler renders as `noreturn`, so it stays outside the window.
    pub fn a78_string_from_range(&self, begin: u32, end: u32) -> A78StringRange {
        let range_len = if end < begin {
            0
        } else {
            end.wrapping_sub(begin) as i32
        };
        A78StringRange {
            header: A78StringInit {
                len: 0,
                cap: A78_SSO_CAP,
                first: 0,
            },
            range_len,
        }
    }

    /// `0x0062edb0` (`FUN_0062edb0`, 249 bytes, `decompiled_uncited`).
    ///
    /// Pushes a dword-valued variant (`{tag 2, value}` at `local_30`) through
    /// the visitor protocol.  The body performs three checks in order:
    ///
    /// ```c
    /// slot = *(uint **)(this + 0x78);                 /* low bit = tagged ptr */
    /// if (slot) if (!(**(code **)(masked(slot) + 4))()) {
    ///     lock(this + 0x24); *(byte *)(this + 0x18) = 1; unlock();
    ///     return 0;
    /// }
    /// if (!FUN_0053b1e0(&variant)) {                  /* tag dispatch */
    ///     FUN_0053af40(&variant);                     /* apply */
    ///     if (!sticky) {
    ///         if (*(char *)(this + 0x18)) FUN_0062f1d0(*this, &variant);
    ///         *(byte *)(this + 0x18) = 0;
    ///     } else *(byte *)(this + 0x18) = 1;
    /// }
    /// unlock();
    /// return 1;
    /// ```
    ///
    /// The visitor and tag dispatch live outside the window, so their results
    /// enter as `visitor` (`None` when `+0x78` is null) and `dispatch` (the
    /// `FUN_0053b1e0` result).  The lock/unlock pair is elided; the flag byte
    /// transitions and the return value are exact.
    pub fn a78_push_dword(
        &self,
        value: u32,
        visitor: Option<bool>,
        dispatch: bool,
        sticky: bool,
        flag: bool,
    ) -> (A78Variant, A78Push) {
        let push = self.a78_push_outcome(visitor, dispatch, sticky, flag);
        (A78Variant::Dword(value), push)
    }

    /// `0x0062f0d0` (`FUN_0062f0d0`, 249 bytes, `decompiled_uncited`).
    ///
    /// Same protocol as [`World::a78_push_dword`] with the byte-valued
    /// envelope `{tag 1, byte}` built at `local_30`/`local_2c`.
    pub fn a78_push_byte(
        &self,
        value: u8,
        visitor: Option<bool>,
        dispatch: bool,
        sticky: bool,
        flag: bool,
    ) -> (A78Variant, A78Push) {
        let push = self.a78_push_outcome(visitor, dispatch, sticky, flag);
        (A78Variant::Byte(value), push)
    }

    /// `0x0062efc0` (`FUN_0062efc0`, 101 bytes, `decompiled_uncited`).
    ///
    /// Same protocol with the string-valued envelope: the four dwords at
    /// `local_30` are zeroed, `FUN_0053cd10` copy-assigns the caller's
    /// `std::string` into the payload, and the tag 5 is written last.
    ///
    /// `FUN_0053cd10` is the shipped MSVC assign fast path: a source whose
    /// capacity word has the top bit clear is *moved* (header copied, source
    /// zeroed); an inline source is copied byte-wise only when
    /// `len < 15` and the destination is inline.  Any other source size leaves
    /// the fast path and continues through the `FUN_0093ec70` allocator
    /// accessor (`BcAllocatorWrap`, the static at `0x00f38738`), which the
    /// decompiler renders as `noreturn`; that heap-growth continuation is not
    /// recovered, so the translation returns `None` for it.
    pub fn a78_push_string(
        &self,
        source: &[u8],
        source_is_heap: bool,
        visitor: Option<bool>,
        dispatch: bool,
        sticky: bool,
        flag: bool,
    ) -> Option<(A78Variant, A78Push)> {
        if !source_is_heap && source.len() as u32 >= A78_SSO_INLINE {
            return None;
        }
        let push = self.a78_push_outcome(visitor, dispatch, sticky, flag);
        Some((A78Variant::Text(source.to_vec()), push))
    }

    /// `0x0062f025` (`FUN_0062f025`, 157 bytes, `decompiled_uncited`).
    ///
    /// The same apply protocol reached without a locally built envelope: the
    /// variant pointer and the `sticky` byte are caller-frame values, and the
    /// visitor slot is `*this & 0xfffffffe` (the first member, not `+0x78`):
    ///
    /// ```c
    /// cVar1 = (**(code **)(*(int *)(*in_ECX & 0xfffffffe) + 4))();
    /// if (cVar1 == '\0') { lock; *(byte *)(this + 0x18) = 1; unlock; return 0; }
    /// if (!FUN_0053b1e0()) { FUN_0053af40(caller_variant); … }
    /// unlock(); return 1;
    /// ```
    ///
    /// There is no null check on the visitor (the first member is asserted
    /// non-null by construction), so `visitor` is a plain `bool` here.
    pub fn a78_push_caller_variant(
        &self,
        visitor_accepts: bool,
        dispatch: bool,
        sticky: bool,
        flag: bool,
    ) -> A78Push {
        self.a78_push_outcome(Some(visitor_accepts), dispatch, sticky, flag)
    }

    /// Shared tail of the four push wrappers: the visitor gate, the tag
    /// dispatch gate and the two flag transitions.
    fn a78_push_outcome(
        &self,
        visitor: Option<bool>,
        dispatch: bool,
        sticky: bool,
        flag: bool,
    ) -> A78Push {
        if let Some(accepted) = visitor {
            if !accepted {
                return A78Push {
                    accepted: false,
                    pushed: false,
                    rearmed: false,
                    flag: true,
                };
            }
        }
        if dispatch {
            return A78Push {
                accepted: true,
                pushed: false,
                rearmed: false,
                flag,
            };
        }
        if sticky {
            A78Push {
                accepted: true,
                pushed: true,
                rearmed: false,
                flag: true,
            }
        } else {
            A78Push {
                accepted: true,
                pushed: true,
                rearmed: flag,
                flag: false,
            }
        }
    }

    /// `0x0062fdc0` (`FUN_0062fdc0`, 94 bytes, `decompiled_uncited`).
    ///
    /// Restores the property object's default value:
    ///
    /// ```c
    /// if (*(int *)(this + 0x48) != 0) {
    ///   *(byte *)(this + 0x60) = 0;
    ///   if (FUN_0062f850() == '\0') {
    ///     switch (*(uint *)(this + 0x20)) {
    ///     case 1: case 2: case 3: case 4: case 5:   /* FUN_0053a640, noreturn */
    ///     default:
    ///       FUN_0053b060(this + 0x24); *(byte *)(this + 0x60) = 1; FUN_0053cac0();
    ///     }
    ///   }
    /// }
    /// ```
    ///
    /// `+0x20` is the value class; classes 1..=5 all fall into the same
    /// `FUN_0053a640` tail, everything else locks the `+0x24` guard and marks
    /// `+0x60` as default again.
    pub fn a78_default_dispatch(
        &self,
        has_slot: bool,
        extract_ok: bool,
        kind: u32,
    ) -> A78DefaultOutcome {
        if !has_slot {
            return A78DefaultOutcome {
                cleared: false,
                action: A78DefaultAction::Missing,
            };
        }
        let action = if extract_ok {
            A78DefaultAction::Extracted
        } else if (1..=5).contains(&kind) {
            A78DefaultAction::Failed(kind)
        } else {
            A78DefaultAction::Restored
        };
        A78DefaultOutcome {
            cleared: true,
            action,
        }
    }

    /// `0x0062f850` (`FUN_0062f850`, 139 bytes, `decompiled_uncited`).
    ///
    /// Numeric payload class of the current value, selected from the flags
    /// word at `+0x0e` of the source variant:
    ///
    /// ```c
    /// uVar1 = *(ushort *)((int)pdVar3 + 0xe);
    /// if (uVar1 & 0x200)      d = *pdVar3;                       /* F64 */
    /// else if (uVar1 & 0x20)  d = (double)*(int *)pdVar3;        /* I32 */
    /// else if (uVar1 & 0x40)  d = (double)*(int *)pdVar3
    ///                             + (double)(&0x00d33180)[-(v >> 31)];  /* U32 */
    /// else if ((char)uVar1 < 0)                                  /* I64 */
    /// else                                                       /* U64 */
    /// ```
    ///
    /// The remaining work of `0x0062f850` builds the tagged dword envelope
    /// and runs the same apply protocol as the push wrappers; only the class
    /// selection is self-contained, so that is what is claimed here.
    pub fn a78_number_kind(&self, flags: u16) -> A78NumberKind {
        if flags & 0x0200 != 0 {
            A78NumberKind::F64
        } else if flags & 0x0020 != 0 {
            A78NumberKind::I32
        } else if flags & 0x0040 != 0 {
            A78NumberKind::U32
        } else if flags & 0x0080 != 0 {
            A78NumberKind::I64
        } else {
            A78NumberKind::U64
        }
    }

    /// `0x0062f850` unsigned-32 widening (the `0x0040` arm):
    /// `(double)(int32)value + (double)(&0x00d33180)[-(value >> 31)]`, where
    /// the two doubles at [`A78_U32_BIAS_SITE`] are `0.0` and `4294967296.0`.
    pub fn a78_unsigned_i32_to_f64(&self, value: i32) -> f64 {
        let bias = if value < 0 { 4294967296.0 } else { 0.0 };
        value as f64 + bias
    }

    /// `0x0062fd60` (`FUN_0062fd60`, 81 bytes, `decompiled_uncited`).
    ///
    /// MSVC magic-static guard for the object at `0x00f05618`:
    ///
    /// ```c
    /// if (tls_block[0xc] < DAT_00faa1c8) {
    ///   FUN_00c87327(&DAT_00faa1c8);              /* __Init_thread_header */
    ///   if (DAT_00faa1c8 == -1) {
    ///     FUN_00c86b59(&LAB_00d0f840);            /* atexit registration */
    ///     FUN_00c872dd(&DAT_00faa1c8);            /* __Init_thread_footer */
    ///   }
    /// }
    /// return &DAT_00f05618;
    /// ```
    ///
    /// `tls_header` is the per-thread word at `[tls_block + 0xc]`,
    /// `thread_seq` is the process-wide counter (`DAT_00faa1c8`), and
    /// `cookie` is that counter's value observed *after* the header call
    /// (`-1` means the guard still needs its destructor registered).
    pub fn a78_tls_static(&self, tls_header: i32, thread_seq: i32, cookie: i32) -> A78TlsStatic {
        let init_header = tls_header < thread_seq;
        A78TlsStatic {
            init_header,
            register_destructor: init_header && cookie == -1,
            address: A78_TLS_STATIC,
        }
    }

    /// `0x0062fcf0` (`FUN_0062fcf0`, 16 bytes, `decompiled_uncited`).
    ///
    /// Chained hash-table lookup:
    ///
    /// ```c
    /// if (in_ECX[1] != 0) {
    ///   hash  = FUN_00c62800(key, key_len, 0);
    ///   index = *in_ECX - 1 & hash;
    ///   node  = buckets[index];
    ///   while (node && !FUN_00539640(key, node + 1)) node = *node;
    ///   if (buckets_or_prev_link != 0) return node + 0x20;
    /// }
    /// return 0;
    /// ```
    ///
    /// The bucket hash is the shipped 32-bit hash (`FUN_00c62800`, the
    /// MurmurHash3-style mixer with constants `0xcc9e2d51` / `0x1b873593` /
    /// `0x85ebca6b` / `0xc2b2ae35`) and the key compare is the shipped
    /// 4-byte block compare (`FUN_00539640`); both are outside the window, so
    /// the hash enters as `hash` and the compare as `key == node.key`.  The
    /// returned `u32` is the node index; retail returns `node + 0x20`.
    pub fn a78_hash_chain_lookup(
        &self,
        table_present: bool,
        bucket_count: u32,
        buckets: &[Option<u32>],
        nodes: &[A78ChainNode],
        hash: u32,
        key: u32,
    ) -> Option<u32> {
        if !table_present {
            return None;
        }
        self.a78_hash_chain_walk(bucket_count, buckets, nodes, hash, key)
    }

    /// `0x0062fd00` (`FUN_0062fd00`, 82 bytes, `decompiled_uncited`).
    ///
    /// The same walk as [`World::a78_hash_chain_lookup`] without the
    /// `in_ECX[1] != 0` guard: `0x0062fd00` is entered with the table and
    /// bucket count held in caller-frame registers (`unaff_ESI`, `unaff_EBX`)
    /// after the caller has already established that the table exists.
    pub fn a78_hash_chain_lookup_unguarded(
        &self,
        bucket_count: u32,
        buckets: &[Option<u32>],
        nodes: &[A78ChainNode],
        hash: u32,
        key: u32,
    ) -> Option<u32> {
        self.a78_hash_chain_walk(bucket_count, buckets, nodes, hash, key)
    }

    /// Shared bucket-select and chain walk of `0x0062fcf0` / `0x0062fd00`.
    fn a78_hash_chain_walk(
        &self,
        bucket_count: u32,
        buckets: &[Option<u32>],
        nodes: &[A78ChainNode],
        hash: u32,
        key: u32,
    ) -> Option<u32> {
        let index = bucket_count.wrapping_sub(1) & hash;
        let mut slot = *buckets.get(index as usize)?;
        while let Some(node) = slot {
            let entry = nodes.get(node as usize)?;
            if entry.key == key {
                return Some(node);
            }
            slot = entry.next;
        }
        None
    }

    /// `0x0062f700` (`FUN_0062f700`, 92 bytes, `decompiled_uncited`).
    ///
    /// Bucket index of the get-or-create lookup:
    ///
    /// ```c
    /// if (in_ECX[1] == 0) FUN_0053cdd0(*in_ECX);      /* allocate buckets */
    /// h    = FUN_00c62800(*param_1, param_1[1], 0);   /* bucket hash */
    /// slot = FUN_00553f50(in_ECX[1] + ((*in_ECX - 1) & h) * 4, param_1);
    /// if (*slot == 0) FUN_0093ec70();                 /* insert / allocate */
    /// return *slot + 0x20;
    /// ```
    ///
    /// The bucket count is `*in_ECX`; the mask is `count - 1`.  The
    /// get-or-insert (`FUN_00553f50`) and the empty-slot path (which continues
    /// through the `FUN_0093ec70` allocator accessor) are outside the window, so
    /// only the index arithmetic is claimed.
    pub fn a78_hash_slot_index(&self, bucket_count: u32, hash: u32) -> u32 {
        bucket_count.wrapping_sub(1) & hash
    }

    /// `0x0062f620` (`FUN_0062f620`, 136 bytes, `decompiled_uncited`).
    ///
    /// Releases every live element of a slot array:
    ///
    /// ```c
    /// if (in_ECX[1] == 0) return;
    /// for (i = 0; i < *in_ECX; ++i) {
    ///   p = *(int **)(in_ECX[1] + i * 4);
    ///   if (p != 0) {
    ///     FUN_006306e0(p);                 /* element destructor */
    ///     if (*(int *)(p + 0xc) < 0) { … } /* reference count below zero */
    ///   }
    /// }
    /// ```
    ///
    /// `FUN_006306e0` is the shipped element destructor (286 bytes) and both
    /// recovered tails leave through the `FUN_0093ec70` allocator accessor, so
    /// the code after the destructor is not recovered.  The translation counts
    /// the destroyed elements and reports `None` when one of them carries a
    /// negative reference count at `+0xc`, which is the only observable state
    /// transition in the body.
    pub fn a78_release_nodes(&self, count: u32, slots: &[Option<i32>]) -> Option<usize> {
        let limit = usize::try_from(count).ok()?;
        if limit > slots.len() {
            return None;
        }
        let mut released = 0usize;
        for slot in slots.iter().take(limit) {
            let Some(refcount) = *slot else { continue };
            released += 1;
            if refcount < 0 {
                return None;
            }
        }
        Some(released)
    }
}

#[cfg(test)]
mod tests_agent78 {
    use super::*;
    use crate::data::GameData;
    use crate::map::Terrain;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(8, 8),
            2,
        )
    }

    // -- 0x0062e330 / 0x0062e370 / 0x0062e3b0 -------------------------------

    /// `0x0062e370`: the 8-byte record index uses `width / 4 * (y / 4) + x / 4`.
    #[test]
    fn quad_index8_matches_the_recovered_stride() {
        let world = world();
        assert_eq!(world.a78_quad_index8(0, 8, 0, 0), 0);
        assert_eq!(world.a78_quad_index8(0, 8, 3, 0), 0);
        assert_eq!(world.a78_quad_index8(0, 8, 4, 0), 8);
        // width / 4 = 2 columns; (2 * 1 + 1) * 8.
        assert_eq!(world.a78_quad_index8(0, 8, 7, 4), 24);
        assert_eq!(world.a78_quad_index8(0x1000, 8, 5, 9), 0x1000 + 40);
        // Negative coordinates divide toward zero, as the shipped `>> 2` does.
        assert_eq!(world.a78_quad_index8(0, 8, -1, -1), 0);
        assert_eq!(world.a78_quad_index8(0, 8, -5, 0), -8);
    }

    /// `0x0062e3b0`: the 16-byte record index swaps the multiply order but
    /// keeps the truncating quarters.
    #[test]
    fn tile_index16_matches_the_recovered_stride() {
        let world = world();
        assert_eq!(world.a78_tile_index16(0, 8, 0, 0), 0);
        assert_eq!(world.a78_tile_index16(0, 8, 4, 0), 0x10);
        assert_eq!(world.a78_tile_index16(0, 8, 0, 4), 0x20);
        assert_eq!(world.a78_tile_index16(0, 8, 7, 7), 0x10 + 0x20);
        assert_eq!(world.a78_tile_index16(0x40, 8, 3, 3), 0x40);
    }

    /// `0x0062e330`: the packed 2-bit field is selected by
    /// `((x % 4) + (y % 4) * 4) * 2` in a `u32`.
    #[test]
    fn quad_field_reads_each_sub_cell() {
        let world = world();
        for y in 0..4 {
            for x in 0..4 {
                let shift = (x + y * 4) * 2;
                let word = 1u32 << shift;
                assert_eq!(world.a78_quad_field(word, x, y), 1);
                assert_eq!(world.a78_quad_field(word, x + 4, y + 8), 1);
            }
        }
        assert_eq!(world.a78_quad_field(0xffff_ffff, 1, 1), 3);
        // Negative coordinates wrap through the byte-sized intermediate:
        // (-1) + (-1) * 4 = -5, * 2 = -10, & 0x1f = 22.
        assert_eq!(world.a78_quad_field(1 << 22, -1, -1), 1);
        assert_eq!(world.a78_quad_field(1 << 22, -5, -5), 1);
    }

    // -- 0x0062e590 / 0x0062e650 -------------------------------------------

    /// `0x0062e590`: both recovered weight sets, including the equality edge.
    #[test]
    fn alpha_byte_blend_uses_the_recovered_weights() {
        let world = world();
        // b < a: (4 * a + 3 * b) / 7.
        assert_eq!(world.a78_blend_alpha_bytes(255, 0), 145);
        // b >= a: (2 * a + 3 * b) / 5.
        assert_eq!(world.a78_blend_alpha_bytes(0, 255), 153);
        assert_eq!(world.a78_blend_alpha_bytes(10, 20), 16);
        assert_eq!(world.a78_blend_alpha_bytes(34, 17), 26);
        // a == b takes the second arm and returns the value unchanged.
        for value in 0u8..=255 {
            assert_eq!(world.a78_blend_alpha_bytes(value, value), value);
        }
    }

    /// `0x0062e590`: the shipped shift/add sequence equals `v / 7` for every
    /// value the two bytes can produce.
    #[test]
    fn sevenths_sequence_equals_integer_division() {
        for value in 0u32..=0xffff {
            assert_eq!(a78_sevenths(value), value / 7, "value {value}");
        }
        for a in 0u32..=255 {
            for b in 0u32..=255 {
                assert_eq!(a78_sevenths(a * 4 + b * 3), (a * 4 + b * 3) / 7);
            }
        }
    }

    /// `0x0062e650`: the seven-part split clamps both bytes at 255.
    #[test]
    fn seventh_split_clamps_both_bytes() {
        let world = world();
        assert_eq!(world.a78_blend_alpha_sevenths(14, 1, 2), (2, 1));
        assert_eq!(world.a78_blend_alpha_sevenths(0, 0, 0), (0, 0));
        assert_eq!(
            world.a78_blend_alpha_sevenths(u32::MAX, u32::MAX, u32::MAX),
            (255, 255)
        );
        assert_eq!(
            world.a78_blend_alpha_sevenths(7 * 300, 7 * 300, 0),
            (255, 255)
        );
    }

    // -- 0x0062e3f0 / 0x0062e4b0 / 0x0062e510 / 0x0062e550 ----------------

    /// `0x0062e3f0` blends the two bytes at the 16-byte record head.
    #[test]
    fn tile_alpha_blends_the_record_head() {
        let world = world();
        let mut data = vec![0u8; 0x40];
        // (x = 4, y = 0) -> (1 + 0) * 0x10 = 0x10.
        data[0x10] = 34;
        data[0x11] = 17;
        assert_eq!(world.a78_tile_alpha(&data, 0, 8, 4, 0), Some(26));
        // Unchecked by retail: a record past the surface edge still resolves
        // while it stays inside the supplied block.
        assert_eq!(world.a78_tile_alpha(&data, 0, 8, 8, 0), Some(0));
        assert_eq!(world.a78_tile_alpha(&data, 0, 8, 32, 0), None);
    }

    /// `0x0062e4b0`: bounds, the `lo <= hi` gate and the "3 means blocked"
    /// field value.
    #[test]
    fn passability_follows_the_quad_record() {
        let world = world();
        let mut data = vec![0u8; 0x80];
        // Quadrant (0, 0) is record 0; quadrant (1, 0) is record 1.
        data[0..2].copy_from_slice(&1u16.to_le_bytes());
        data[2..4].copy_from_slice(&2u16.to_le_bytes());
        // Cell (1, 1) -> shift 10.
        data[4..8].copy_from_slice(&(3u32 << 10).to_le_bytes());
        assert!(world.a78_tile_passable(&data, 0, 8, 8, 0, 0));
        assert!(!world.a78_tile_passable(&data, 0, 8, 8, 1, 1));
        // A quadrant whose low bound exceeds its high bound is passable.
        data[0..2].copy_from_slice(&5u16.to_le_bytes());
        data[2..4].copy_from_slice(&4u16.to_le_bytes());
        assert!(world.a78_tile_passable(&data, 0, 8, 8, 1, 1));
        // Out-of-range pixels are never passable.
        assert!(!world.a78_tile_passable(&data, 0, 8, 8, 8, 0));
        assert!(!world.a78_tile_passable(&data, 0, 8, 8, 0, -1));
    }

    /// `0x0062e510` compares the threshold against the blended record byte.
    #[test]
    fn tile_alpha_covers_uses_the_blend() {
        let world = world();
        let mut data = vec![0u8; 0x40];
        data[0x10] = 34;
        data[0x11] = 17;
        assert!(world.a78_tile_alpha_covers(&data, 0, 8, 8, 4, 0, 26));
        assert!(!world.a78_tile_alpha_covers(&data, 0, 8, 8, 4, 0, 27));
        assert!(!world.a78_tile_alpha_covers(&data, 0, 8, 8, 4, 8, 0));
    }

    /// `0x0062e550` reads the alpha byte of the RGBA surface.
    #[test]
    fn rgba_alpha_covers_reads_byte_three() {
        let world = world();
        let mut data = vec![0u8; 8 * 32];
        // Pixel (2, 3) -> 3 * 32 + 2 * 4.
        data[3 * 32 + 2 * 4 + 3] = 200;
        assert!(world.a78_tile_rgba_covers(&data, 0, 8, 8, 32, 2, 3, 200));
        assert!(!world.a78_tile_rgba_covers(&data, 0, 8, 8, 32, 2, 3, 201));
        // Pixel (2, 4) has alpha 0, so only a zero threshold passes.
        assert!(world.a78_tile_rgba_covers(&data, 0, 8, 8, 32, 2, 4, 0));
        assert!(!world.a78_tile_rgba_covers(&data, 0, 8, 8, 32, 2, 4, 1));
        assert!(!world.a78_tile_rgba_covers(&data, 0, 8, 8, 32, -1, 0, 0));
    }

    /// `0x0062e840`: the four bytes become `byte / 255` floats, and the
    /// single-precision divide agrees with the double-rounded one for every
    /// possible byte.
    #[test]
    fn rgba_pixel_becomes_four_floats() {
        let world = world();
        let mut data = vec![0u8; 8 * 32];
        data[32 + 4] = 0;
        data[32 + 5] = 51;
        data[32 + 6] = 128;
        data[32 + 7] = 255;
        let color = world.a78_tile_rgba_color(&data, 0, 8, 8, 32, 1, 1, [9.0; 4]);
        assert_eq!(color, [0.0, 51.0 / 255.0, 128.0 / 255.0, 1.0]);
        let default = [7.0, 8.0, 9.0, 10.0];
        assert_eq!(
            world.a78_tile_rgba_color(&data, 0, 8, 8, 32, 8, 0, default),
            default
        );
        for byte in 0u8..=255 {
            let single = byte as f32 / 255.0;
            let double = (byte as f64 / 255.0) as f32;
            assert_eq!(single.to_bits(), double.to_bits(), "byte {byte}");
        }
    }

    /// `0x0062e820` copies the four default dwords unchanged.
    #[test]
    fn default_color_is_copied() {
        let world = world();
        let default = [0.25, 0.5, 0.75, 1.0];
        assert_eq!(world.a78_tile_default_color(default), default);
    }

    // -- 0x0062e180 / 0x0062e720 / 0x0062e780 ------------------------------

    fn selector_color(selector: u16) -> [f32; 4] {
        // The 5-6-5 decode of FUN_0062dec0, kept in the test so the translation
        // only models the blend.
        let red = (selector >> 11) & 0x1f;
        let green = (selector >> 5) & 0x3f;
        let blue = selector & 0x1f;
        [
            red as f32 / 31.0,
            green as f32 / 63.0,
            blue as f32 / 31.0,
            1.0,
        ]
    }

    /// `0x0062e180`: the four mode arms and the zeroed default arm.
    #[test]
    fn palette_blend_arms() {
        let world = world();
        let first = [1.0, 0.0, 0.0, 1.0];
        let second = [0.0, 1.0, 0.0, 1.0];
        let colors = [first, second];
        let default = [0.5, 0.25, 0.125, 1.0];

        assert_eq!(
            world.a78_blend_palette_pair(0, [1, 2], colors, default),
            first
        );
        assert_eq!(
            world.a78_blend_palette_pair(1, [1, 2], colors, default),
            second
        );

        // sel[1] < sel[0]: second * 2/3 + first * 1/3.
        let mixed = world.a78_blend_palette_pair(2, [0x2222, 0x1111], colors, default);
        assert_eq!(mixed[0].to_bits(), A78_ONE_THIRD.to_bits());
        assert_eq!(mixed[1].to_bits(), A78_TWO_THIRDS.to_bits());
        assert_eq!(mixed[2], 0.0);
        assert_eq!(mixed[3], 1.0);

        // sel[1] >= sel[0]: half each.
        let mixed = world.a78_blend_palette_pair(2, [1, 2], colors, default);
        assert_eq!(mixed, [0.5, 0.5, 0.0, 1.0]);

        // Mode 3 with sel[0] <= sel[1] keeps the default RGB and clears alpha.
        assert_eq!(
            world.a78_blend_palette_pair(3, [1, 2], colors, default),
            [0.5, 0.25, 0.125, 0.0]
        );
        // Mode 3 with sel[0] > sel[1]: second * 1/3 + first * 2/3.
        let mixed = world.a78_blend_palette_pair(3, [2, 1], colors, default);
        assert_eq!(mixed[0].to_bits(), A78_TWO_THIRDS.to_bits());
        assert_eq!(mixed[1].to_bits(), A78_ONE_THIRD.to_bits());

        // Unknown modes leave the zeroed output slot untouched.
        assert_eq!(
            world.a78_blend_palette_pair(4, [1, 2], colors, default),
            [0.0; 4]
        );
    }

    /// `0x0062e720`: quadrant record selectors plus the packed mode word.
    #[test]
    fn quad_color_reads_selectors_and_mode() {
        let world = world();
        let mut data = vec![0u8; 0x80];
        let record = world.a78_quad_index8(0, 8, 4, 4) as usize;
        data[record..record + 2].copy_from_slice(&0x2222u16.to_le_bytes());
        data[record + 2..record + 4].copy_from_slice(&0x1111u16.to_le_bytes());
        // Cell (0, 0) of that quadrant: mode 2.
        data[record + 4..record + 8].copy_from_slice(&2u32.to_le_bytes());

        let color = world.a78_tile_quad_color(
            &data,
            0,
            8,
            8,
            4,
            4,
            |selector| match selector {
                0x2222 => [1.0, 0.0, 0.0, 1.0],
                0x1111 => [0.0, 1.0, 0.0, 1.0],
                _ => [0.0; 4],
            },
            [1.0, 2.0, 3.0, 4.0],
        );
        assert_eq!(color[0].to_bits(), A78_ONE_THIRD.to_bits());
        assert_eq!(color[1].to_bits(), A78_TWO_THIRDS.to_bits());

        // Out of range returns the four default dwords.
        assert_eq!(
            world.a78_tile_quad_color(&data, 0, 8, 8, 8, 0, selector_color, [5.0; 4]),
            [5.0; 4]
        );
    }

    /// `0x0062e780`: the tile record supplies selectors, the dword at `+0xc`
    /// supplies the mode, and the record head supplies the alpha pair.
    #[test]
    fn tile_color_with_alpha_attaches_the_blended_alpha() {
        let world = world();
        let mut data = vec![0u8; 0x100];
        let record = world.a78_tile_index16(0, 8, 4, 4) as usize;
        // Head bytes 0x22 / 0x11 are also the low bytes of the selectors
        // 0x1122 and 0x1111.
        data[record..record + 4].copy_from_slice(&[0x22, 0x11, 0x11, 0x11]);
        // Mode 2 in cell (0, 0) of the packed word at +0xc.
        data[record + 0x0c..record + 0x10].copy_from_slice(&2u32.to_le_bytes());

        let color = world.a78_tile_quad_color_with_alpha(
            &data,
            0,
            8,
            8,
            4,
            4,
            |selector| match selector {
                0x1122 => [1.0, 0.0, 0.0, 1.0],
                0x1111 => [0.0, 1.0, 0.0, 1.0],
                _ => [0.0; 4],
            },
            [1.0, 2.0, 3.0, 4.0],
        );
        // Alpha bytes are 0x22 and 0x11 -> 26 -> 26 / 255.
        assert_eq!(color[3], 26.0 / 255.0);
        // sel[1] = 0x1111 < sel[0] = 0x1122: second * 2/3 + first * 1/3.
        assert_eq!(color[0].to_bits(), A78_ONE_THIRD.to_bits());
        assert_eq!(color[1].to_bits(), A78_TWO_THIRDS.to_bits());

        // Out of range keeps the default, alpha included.
        assert_eq!(
            world.a78_tile_quad_color_with_alpha(&data, 0, 8, 8, 8, 0, selector_color, [5.0; 4]),
            [5.0; 4]
        );
    }

    // -- 0x0062ecb0 / the push family / 0x0062fdc0 ------------------------

    /// `0x0062ecb0`: the empty SSO header plus the unsigned range clamp.
    #[test]
    fn string_range_initialiser() {
        let world = world();
        let init = world.a78_string_from_range(0, 4);
        assert_eq!(init.header.len, 0);
        assert_eq!(init.header.cap, A78_SSO_CAP);
        assert_eq!(init.header.first, 0);
        assert_eq!(init.range_len, 4);
        assert_eq!(world.a78_string_from_range(4, 4).range_len, 0);
        // `param_3 < param_2` is an unsigned comparison, so a reversed range
        // yields zero even though the difference is negative.
        assert_eq!(world.a78_string_from_range(5, 2).range_len, 0);
        assert_eq!(
            world.a78_string_from_range(0, 0x8000_0001).range_len,
            i32::MIN + 1
        );
    }

    /// `0x0062edb0`, `0x0062f0d0`: envelope tags and the apply transitions.
    #[test]
    fn push_envelopes_and_outcomes() {
        let world = world();
        let (variant, push) = world.a78_push_dword(7, None, false, false, true);
        assert_eq!(variant, A78Variant::Dword(7));
        assert_eq!(variant.tag(), A78_TAG_DWORD);
        assert_eq!(
            push,
            A78Push {
                accepted: true,
                pushed: true,
                rearmed: true,
                flag: false
            }
        );

        let (variant, push) = world.a78_push_byte(3, None, false, true, false);
        assert_eq!(variant, A78Variant::Byte(3));
        assert_eq!(variant.tag(), A78_TAG_BYTE);
        assert_eq!(
            push,
            A78Push {
                accepted: true,
                pushed: true,
                rearmed: false,
                flag: true
            }
        );

        // The visitor refuses: the flag is armed and the wrapper returns 0.
        let (_, push) = world.a78_push_dword(7, Some(false), false, false, false);
        assert_eq!(
            push,
            A78Push {
                accepted: false,
                pushed: false,
                rearmed: false,
                flag: true
            }
        );

        // The tag dispatch routes the value: nothing is applied and the flag
        // is left alone.
        let (_, push) = world.a78_push_dword(7, Some(true), true, true, true);
        assert_eq!(
            push,
            A78Push {
                accepted: true,
                pushed: false,
                rearmed: false,
                flag: true
            }
        );
    }

    /// `0x0062efc0`: the string envelope and the `FUN_0053cd10` inline gate.
    #[test]
    fn push_string_respects_the_inline_gate() {
        let world = world();
        let (variant, push) = world
            .a78_push_string(b"abc", false, None, false, false, false)
            .unwrap();
        assert_eq!(variant.tag(), A78_TAG_STRING);
        assert_eq!(variant, A78Variant::Text(b"abc".to_vec()));
        assert!(push.pushed);

        // 15 inline bytes are the first size the fast path rejects.
        assert!(
            world
                .a78_push_string(&[b'x'; 14], false, None, false, false, false)
                .is_some()
        );
        assert!(
            world
                .a78_push_string(&[b'x'; 15], false, None, false, false, false)
                .is_none()
        );
        // Heap sources are moved, so the length gate does not apply.
        assert!(
            world
                .a78_push_string(&[b'x'; 64], true, None, false, false, false)
                .is_some()
        );
    }

    /// `0x0062f025`: the caller-frame variant path has no visitor null check.
    #[test]
    fn caller_variant_push() {
        let world = world();
        let push = world.a78_push_caller_variant(true, false, false, false);
        assert!(push.accepted && push.pushed && !push.flag);
        let push = world.a78_push_caller_variant(false, false, false, true);
        assert!(!push.accepted && push.flag);
    }

    /// `0x0062fdc0`: the value-class dispatch and the cleared `+0x60` byte.
    #[test]
    fn default_dispatch_arms() {
        let world = world();
        assert_eq!(
            world.a78_default_dispatch(false, true, 9),
            A78DefaultOutcome {
                cleared: false,
                action: A78DefaultAction::Missing
            }
        );
        assert_eq!(
            world.a78_default_dispatch(true, true, 9),
            A78DefaultOutcome {
                cleared: true,
                action: A78DefaultAction::Extracted
            }
        );
        for kind in 1..=5 {
            assert_eq!(
                world.a78_default_dispatch(true, false, kind).action,
                A78DefaultAction::Failed(kind)
            );
        }
        assert_eq!(
            world.a78_default_dispatch(true, false, 0).action,
            A78DefaultAction::Restored
        );
        assert_eq!(
            world.a78_default_dispatch(true, false, 6).action,
            A78DefaultAction::Restored
        );
    }

    // -- 0x0062f850 / 0x0062fd60 / 0x0062fcf0 / 0x0062f700 / 0x0062f620 --

    /// `0x0062f850`: class precedence is `0x200`, `0x20`, `0x40`, `0x80`,
    /// then the unsigned-64 fallback.
    #[test]
    fn numeric_class_selection() {
        let world = world();
        assert_eq!(world.a78_number_kind(0x0200), A78NumberKind::F64);
        assert_eq!(world.a78_number_kind(0x0220), A78NumberKind::F64);
        assert_eq!(world.a78_number_kind(0x0020), A78NumberKind::I32);
        assert_eq!(world.a78_number_kind(0x0040), A78NumberKind::U32);
        assert_eq!(world.a78_number_kind(0x00c0), A78NumberKind::U32);
        assert_eq!(world.a78_number_kind(0x0080), A78NumberKind::I64);
        assert_eq!(world.a78_number_kind(0x0000), A78NumberKind::U64);
        assert_eq!(world.a78_number_kind(0x0001), A78NumberKind::U64);
    }

    /// `0x0062f850`: the `0x00d33180` bias widens the negative half.
    #[test]
    fn unsigned_i32_widening() {
        let world = world();
        assert_eq!(world.a78_unsigned_i32_to_f64(0), 0.0);
        assert_eq!(world.a78_unsigned_i32_to_f64(5), 5.0);
        assert_eq!(world.a78_unsigned_i32_to_f64(-1), 4294967295.0);
        assert_eq!(world.a78_unsigned_i32_to_f64(i32::MIN), 2147483648.0);
    }

    /// `0x0062fd60`: the guard decision plus the fixed static address.
    #[test]
    fn tls_static_guard() {
        let world = world();
        let state = world.a78_tls_static(3, 5, -1);
        assert_eq!(
            state,
            A78TlsStatic {
                init_header: true,
                register_destructor: true,
                address: 0x00f0_5618
            }
        );
        let state = world.a78_tls_static(5, 5, 1);
        assert!(!state.init_header && !state.register_destructor);
        let state = world.a78_tls_static(1, 5, 1);
        assert!(state.init_header && !state.register_destructor);
    }

    fn chain_fixture() -> (Vec<Option<u32>>, Vec<A78ChainNode>) {
        // bucket 3 -> node 0 -> node 1, bucket 0 -> node 2.
        let buckets = vec![Some(2), None, None, Some(0)];
        let nodes = vec![
            A78ChainNode {
                next: Some(1),
                key: 10,
            },
            A78ChainNode {
                next: None,
                key: 11,
            },
            A78ChainNode {
                next: None,
                key: 20,
            },
        ];
        (buckets, nodes)
    }

    /// `0x0062fcf0`: bucket mask `count - 1 & hash` then the chain walk.
    #[test]
    fn hash_chain_lookup_walks_the_bucket() {
        let world = world();
        let (buckets, nodes) = chain_fixture();
        assert_eq!(
            world.a78_hash_chain_lookup(true, 4, &buckets, &nodes, 3, 11),
            Some(1)
        );
        assert_eq!(
            world.a78_hash_chain_lookup(true, 4, &buckets, &nodes, 3, 10),
            Some(0)
        );
        assert_eq!(
            world.a78_hash_chain_lookup(true, 4, &buckets, &nodes, 0, 20),
            Some(2)
        );
        assert_eq!(
            world.a78_hash_chain_lookup(true, 4, &buckets, &nodes, 3, 99),
            None
        );
        // Empty bucket and absent table both miss.
        assert_eq!(
            world.a78_hash_chain_lookup(true, 4, &buckets, &nodes, 1, 10),
            None
        );
        assert_eq!(
            world.a78_hash_chain_lookup(false, 4, &buckets, &nodes, 3, 10),
            None
        );
    }

    /// `0x0062fd00`: the ungated walk is the same algorithm.
    #[test]
    fn unguarded_hash_chain_lookup() {
        let world = world();
        let (buckets, nodes) = chain_fixture();
        assert_eq!(
            world.a78_hash_chain_lookup_unguarded(4, &buckets, &nodes, 3, 11),
            Some(1)
        );
        assert_eq!(
            world.a78_hash_chain_lookup_unguarded(4, &buckets, &nodes, 2, 11),
            None
        );
    }

    /// `0x0062f700`: the bucket mask wraps like the shipped `count - 1`.
    #[test]
    fn hash_slot_index() {
        let world = world();
        assert_eq!(world.a78_hash_slot_index(8, 0x1234), 0x1234 & 7);
        assert_eq!(world.a78_hash_slot_index(1, 0xffff_ffff), 0);
        assert_eq!(world.a78_hash_slot_index(0, 0xffff_ffff), 0xffff_ffff);
    }

    /// `0x0062f620`: the release loop skips nulls and aborts on a negative
    /// reference count.
    #[test]
    fn release_loop_counts_live_slots() {
        let world = world();
        assert_eq!(
            world.a78_release_nodes(3, &[Some(2), None, Some(0)]),
            Some(2)
        );
        assert_eq!(world.a78_release_nodes(0, &[Some(2)]), Some(0));
        assert_eq!(world.a78_release_nodes(1, &[Some(-1)]), None);
        // A count beyond the supplied slot array cannot be walked.
        assert_eq!(world.a78_release_nodes(4, &[Some(1)]), None);
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// * FUN_0062e040: texture-cache conversion probe.  It validates through
//   `FUN_0053f840`, constructs through `FUN_00551020`, then switches on the
//   `0x31545844`/`0x35545844` (`DXT1`/`DXT5`) tag and calls `FUN_0062da60` /
//   `FUN_0062dae0` (both `noreturn` conversion tails).  Every step is an
//   indirect call or a routine outside the window.
// * FUN_0062e06a, FUN_0062f370, FUN_0062f26b, FUN_0062fb22, FUN_0062fe64:
//   bodies that begin inside a previous instruction stream ("control flow
//   encountered bad instruction data", overlapping instructions) or that
//   operate only on `unaff_*` registers with arithmetic that has no recovered
//   operands.  No faithful source-level behaviour survives.
// * FUN_0062e345, FUN_0062e72a, FUN_0062e8d3, FUN_0062f5e1, FUN_0062f696,
//   FUN_0062f6ac: mid-instruction fragments and duplicated tails
//   (`FUN_0062e345` is `0x0062e330`'s body with a garbled prologue, `FUN_0062e72a`
//   is `0x0062e720` with a shifted caller frame, `FUN_0062e8d3` is an empty spin
//   loop).
// * FUN_0062e816, FUN_0062fbbe: `int3` padding (`__break`), zero behaviour.
// * FUN_0062e8b0, FUN_0062ead0: inline `std::string` assignment bodies.  The
//   only self-contained work is the C-string length walk (`end - (start + 1)`);
//   the two arms then continue into `FUN_00538810` (the 66-byte capacity
//   request) and the 1208-byte `FUN_00c89580` copy helper, both outside the
//   window and both rendered as `noreturn` by the decompiler.
// * FUN_0062e9ac, FUN_0062e9e2, FUN_0062ea04, FUN_0062ea80, FUN_0062ea8c,
//   FUN_0062eb6f, FUN_0062ebad, FUN_0062ec51, FUN_0062ec70, FUN_0062eeb0,
//   FUN_0062eece: MSVC exception-cleanup funclets.  Each body is a sequence of
//   caller-frame stores (`[ebp - 4] = state`, inline `0x1c05` string headers)
//   that ends in `FUN_00c86b6e` / `FUN_0093ec70`; the frame slots are owned by the
//   enclosing routine and the recovered stores carry no independent
//   behaviour.
// * FUN_0062f1d0: tag dispatch through the jump table at `0x0062f600`.  The
//   decompiler could not recover the table ("too many branches"), so the
//   callee set is unknown.
// * FUN_0062fbd0: walks the bucket array and both link levels, calling
//   `0x0062f850` for each node.  The body has no return value and every
//   interesting step is an indirect call into `0x0062f850`'s closure state.
