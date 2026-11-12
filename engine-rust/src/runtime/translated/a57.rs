#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent57.rs
// ===========================================================================
// Translation batch 57.  Only this file is edited by the batch.
//
// Batch 57 owns `0x00604000..=0x00605fff`.  The ledger lists 31 rows in the
// window, all `decompiled_uncited`; 30 are translated below and one is
// skipped:
//
// * `FUN_00605ca0` (237 bytes) begins the map-file loader: it constructs the
//   `0x00604f40` subobject, asks the out-of-window parser `FUN_007590b0` for a
//   map, turns the result into one of the two literals `maperrorINVALID` /
//   `maperrorHIGHERVERSION`, then copies a `0x8d`-byte parser state through
//   `FUN_00758d80` into stack locals and enters a non-returning tail
//   (`FUN_00748d40(0)`).  Everything after the two error literals is produced
//   by out-of-window parsers, so the row is skipped instead of inventing
//   their data flow.
//
// The window is the client's tileset/asset cache plus the C++ container
// support its records use.  The batch adds no fields to `crate::sim::World`
// or `crate::sim::SimUnit`; the layouts below are local value types, mirroring the
// recovered retail records.  The string and table bytes are read from the
// shipped image at their exact data addresses, so the names and extensions
// are not guesses:
//
// * `0x00f04758` - eight 0x1c-byte `std::string` objects (the tileset file
//   names), contiguous with the six-dword variant table at `0x00f04838`.
// * `0x00d33588`, `0x00d390c8`, `0x00d390ec` - `.DDS`, `.vr4`, `.grp`;
//   `0x00d390d0` is `Tileset\`, `0x00d390dc` is `{0}{1}{2}{3}{4}` and
//   `0x00d39100` is `{0}{1}{2}`.
// * `0x00f4e578`-style stack cookies and the `ExceptionList` bookkeeping are
//   unwind artefacts of the decompiler's funclets; they carry no data and
//   are not modelled.
//
// Recovered records:
//
// | record | layout |
// |--------|--------|
// | tileset slot, stride `0x520` (base `0x00fa3970`, era at `0x00fa3e8c`, ready at `0x00fa3e8e`) | `+0x000` file #3, `+0x004` file #0, `+0x008` `u16` `#0 size >> 5`, `+0x00c` file #1, `+0x010` `#1 size / 0x34`, `+0x014` file #2, `+0x018` `#4 size`, `+0x518` `u32`-per-`0x20`-byte grid, `+0x51c` `u16` era, `+0x51e` ready byte |
// | per-era asset record, stride `0x90` (base `0x00fa8b70`) | five 0x1c-byte string slots at `+0x00`, `+0x1c`, `+0x38`, `+0x54`, `+0x70`; ready byte at `+0x8c` |
// | string | data pointer `+0x00`, byte size `+0x04`, tagged capacity `+0x08` (`0x8000000f` inline), 16-byte inline buffer at `+0x0c` |
// | `0x00604d80` object | seven strings at `+0x20` `+0x3c` `+0x58` `+0x74` `+0x90` `+0xac` `+0xc8`, list at `+0xe4`, flags `+0xf0..+0x113`, 0xbd zero bytes `+0x114..+0x1d0`, dword `+0x1d4`, strings `+0x1d8`/`+0x1f4` |
// | `0x00604f40` object | string `+0x04`, `[0,0,1,8,0]` at `+0x20`, byte `+0x34`, strings `+0xc4`/`+0xe0`/`+0xfc`, list at `+0x11c` |
//
// The two hash maps use the image's initial state: a counter of 1 at
// `0x00f04850`, eight buckets (`0x00f04854`/`0x00f04864`) and max load
// factor 1.0 (`0x00f04860`/`0x00f04870`).  The string map hashes its key
// with the MurmurHash3 x86-32 body at `FUN_00c62800` (seed 0); the pair map
// uses the Murmur finaliser on the low key dword.

/// One tileset slot: the record size used by `0x00604540`'s array index
/// (`iVar3 = (param_1 & 0xffff) * 0x520`).
pub const A57_TILESET_STRIDE: u32 = 0x520;

/// Base of the slot array that `0x00604540` indexes and returns
/// (`&DAT_00fa3970 + iVar3`).
pub const A57_SLOT_ARRAY_BASE: u32 = 0x00fa_3970;

/// Base of the second 0x520-stride array selected by the global era index in
/// `0x00604340` (`&DAT_00fa6270 + DAT_00fa3968 * 0x520`).
pub const A57_ACTIVE_ARRAY_BASE: u32 = 0x00fa_6270;

/// `+0x518`: the `u32` grid built by `0x00604250`/`0x00604540` and freed by
/// `0x00604290` (`in_ECX[0x146]`).
pub const A57_SLOT_MTX_OFFSET: usize = 0x518;

/// `+0x51c`: the `u16` era stored by `0x00604540` and read by
/// `0x006047b0`/`0x006048b0`.
pub const A57_SLOT_ERA_OFFSET: usize = 0x51c;

/// `+0x51e`: the ready byte cleared by `0x006040e0`/`0x00604290` and set by
/// `0x00604540` (`(&DAT_00fa3e8e)[iVar3]`).
pub const A57_SLOT_READY_OFFSET: usize = 0x51e;

/// Per-era asset record stride (`0x00604540`/`0x006048b0` index by
/// `era * 0x90`).
pub const A57_ASSET_STRIDE: u32 = 0x90;

/// Distance between the five string slots of an asset record
/// (`0x00fa8b70`, `0x00fa8b8c`, `0x00fa8ba8`, `0x00fa8bc4`, `0x00fa8be0`).
pub const A57_ASSET_STRING_STRIDE: u32 = 0x1c;

/// `+0x8c` of an asset record: the ready byte tested by `0x006048b0`
/// (`(&DAT_00fa8bfc)[era * 0x90]`).
pub const A57_ASSET_READY_OFFSET: u32 = 0x8c;

/// One minitile record consumed by the grid builder: 32 bytes = 16 `u16`
/// flags plus the parity bytes counted by the first scan.
pub const A57_MINITILE_STRIDE: usize = 0x20;

/// One megatile record: `param_1 / 0x34` in `0x00604540`.
pub const A57_MEGATILE_STRIDE: u32 = 0x34;

/// Directory prefix written by the path builders (`0x00d390d0`).
pub const A57_TILESET_DIR: &str = "Tileset\\";

/// HD art prefix (`0x00d33588`).
pub const A57_DDS_SUFFIX: &str = ".DDS";

/// HD group suffix (`0x00d390ec`, used by `0x00604450`).
pub const A57_GRP_SUFFIX: &str = ".grp";

/// SD/HD `vr4` suffix (`0x00d390c8`, used by `0x00604690` and the
/// `0x006048b0` error path).
pub const A57_VR4_SUFFIX: &str = ".vr4";

/// The complete `0x00d39120` literal reported by `0x006040c0`.
pub const A57_MTX_BUG_FORMAT: &str = "BUG: mtx %d has mid+high cell";

/// The eight file-name strings copied from the `std::string` table at
/// `0x00f04758` (sizes 8, 8, 7, 8, 6, 6, 3, 8).  Note the mixed spelling:
/// the retail file names are `AshWorld`, `Jungle`, `Desert`, `Ice` and
/// `Twilight`, not the lower-case display names.
pub const A57_TILESET_NAMES: [&str; 8] = [
    "badlands", "platform", "install", "AshWorld", "Jungle", "Desert", "Ice", "Twilight",
];

/// The six dwords at `0x00f04838`, read by `0x00604c40`.  The following
/// dwords in the image belong to the map globals (`0x00f04850` is the
/// counter), so the logical table stops at entry 5.
pub const A57_VARIANT_TABLE: [u32; 6] = [0, 0x24, 0x1d, 0x15, 0xc, 1];

/// Tag bit of a string capacity word: the inline empty string is
/// `0x8000000f`, and the checks in `0x00605010`/`0x00605220` reject any
/// capacity whose sign bit is clear.
pub const A57_CAPACITY_TAG: u32 = 0x8000_0000;

/// Empty inline string capacity written by every constructor in the window
/// (`0x8000000f`).
pub const A57_SSO_CAPACITY: u32 = 0x8000_000f;

/// Initial bucket count of both id maps (`DAT_00f04854 = 8`,
/// `DAT_00f04864 = 8`).
pub const A57_MAP_INITIAL_BUCKETS: u32 = 8;

/// Initial counter of both id maps (`DAT_00f04850 = 1`).
pub const A57_MAP_COUNTER_BASE: u32 = 1;

/// Capacity offsets checked, in order, by `0x00605010` (`local_8` 0, 1
/// and 3..9; state 2 is the list reset between the second and third).
pub const A57_DISPLAY_CAP_OFFSETS: [usize; 9] =
    [0x1fc, 0x1e0, 0xd0, 0xb4, 0x98, 0x7c, 0x60, 0x44, 0x28];

/// Capacity offsets checked, in order, by `0x00605220` and `0x006052f0`
/// (`local_8` 0..3, and 1..4 when the list reset is step 0).
pub const A57_MAP_CAP_OFFSETS: [usize; 4] = [0x104, 0xe8, 0xcc, 0xc];

/// String-object offsets of the `0x00604d80` object, in ascending order.
pub const A57_DISPLAY_STRING_OFFSETS: [usize; 9] =
    [0x20, 0x3c, 0x58, 0x74, 0x90, 0xac, 0xc8, 0x1d8, 0x1f4];

/// String-object offsets of the `0x00604f40` object, in ascending order.
pub const A57_MAP_STRING_OFFSETS: [usize; 4] = [0x04, 0xc4, 0xe0, 0xfc];

/// Observable header of one 0x1c-byte string.
///
/// Retail keeps the data pointer in `+0x00`, the byte size in `+0x04`, the
/// tagged capacity in `+0x08` and the 16-byte inline buffer at `+0x0c`.
/// The model keeps the bytes in [`String`] and only the capacity word is
/// reproduced because the sign bit is what the container checks test.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A57String {
    /// Bytes stored for the string (`+0x04` bytes plus the NUL in retail).
    pub text: String,
    /// The `+0x08` capacity word, including [`A57_CAPACITY_TAG`].
    pub cap: u32,
}

/// The empty inline string written by every constructor in the window:
/// size 0, capacity `0x8000000f`, first buffer byte 0.
pub fn a57_string_empty() -> A57String {
    A57String {
        text: String::new(),
        cap: A57_SSO_CAPACITY,
    }
}

/// The test performed by every `if (-1 < *(int *)(this + off))` check in
/// `0x00605010`, `0x00605220` and `0x006052f0`: a valid header always has
/// the capacity tag set, so a non-negative capacity word is the failure.
pub fn a57_capacity_initialized(cap: u32) -> bool {
    cap & A57_CAPACITY_TAG != 0
}

/// The growth policy of the out-of-window `FUN_00538810`, which
/// `0x006053d0` and `0x00605660` call:
///
/// ```text
/// real = cap & 0x7fffffff
/// if (real >= needed) return;                 // no growth
/// grown = (cap >> 1 & 0x3fffffff) + real      // half the current capacity
/// if (grown < needed) grown = needed
/// cap = (cap & 0x80000000) | (grown & 0x7fffffff)
/// ```
///
/// Returns the new capacity word, or `None` when the current one already
/// covers `needed`.  The allocation itself is not modelled.
pub fn a57_reserve_capacity(cap: u32, needed: u32) -> Option<u32> {
    let real = cap & 0x7fff_ffff;
    if real >= needed {
        return None;
    }
    let mut grown = (cap >> 1 & 0x3fff_ffff) + real;
    if grown < needed {
        grown = needed;
    }
    Some((cap & A57_CAPACITY_TAG) | (grown & 0x7fff_ffff))
}

/// The move of a string header performed by the out-of-window `FUN_0053ae90`,
/// called by `0x00605430`, `0x006054e0` and `0x006055e0`.
///
/// The shipped body copies the data pointer and size, writes the source
/// capacity with its tag masked (`param_1[2] & 0x7fffffff`), then resets the
/// source to the inline header (`self+0xc`, size 0, capacity
/// `0x8000000f`, first byte 0).  The masked destination capacity is
/// reproduced exactly; the decompiled fragment ends at the source reset and
/// does not write the destination tag itself.
pub fn a57_move_string(src: &mut A57String) -> A57String {
    let text = std::mem::take(&mut src.text);
    let cap = src.cap & 0x7fff_ffff;
    *src = a57_string_empty();
    A57String { text, cap }
}

/// An intrusive list header: the two self-referential end links and the
/// element count.  A fresh list has both links pointing at the header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A57List {
    /// `+0x00`: the head link points at the header when the list is empty.
    pub first_self: bool,
    /// `+0x04`: the tail link points at the header when the list is empty.
    pub last_self: bool,
    /// `+0x08`: element count.
    pub size: u32,
}

/// The header state written by `0x00605190` and its identical out-of-window
/// twin `FUN_005e8790`: `first = last = this`, `size = 0`.
pub const A57_LIST_EMPTY: A57List = A57List {
    first_self: true,
    last_self: true,
    size: 0,
};

/// The body shared by `0x00605190` and the out-of-window `FUN_005e8790`:
/// re-link both ends to the header and clear the count.  The old tail is
/// inspected first; when it was not the header the shipped code walks the
/// node teardown and never returns through this path, which the boolean
/// reports (`true` = the list was already empty).
pub fn a57_list_reset(list: &mut A57List) -> bool {
    let was_empty = list.last_self && list.size == 0;
    *list = A57_LIST_EMPTY;
    was_empty
}

/// The five files of one per-era asset record (`0x00fa8b70 + 0x90*era
/// + 0x1c*k`, k = 0..4).
/// The strings themselves are empty in the shipped image; the loaders are
/// handed their contents through this value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct A57AssetFiles {
    /// Ready byte at record `+0x8c` (`0x006048b0` rejects a clear byte).
    pub loaded: bool,
    /// File slot 0, read into slot `+0x004` and counted in 0x20-byte
    /// minitile records (`size >> 5`).
    pub file0: Vec<u8>,
    /// File slot 1, read into slot `+0x00c` and counted in 0x34-byte
    /// megatile records (`size / 0x34`).
    pub file1: Vec<u8>,
    /// File slot 2, read into slot `+0x014`.
    pub file2: Vec<u8>,
    /// File slot 3, read into slot `+0x000`.
    pub file3: Vec<u8>,
    /// File slot 4; only its byte size reaches slot `+0x018`.
    pub file4: Vec<u8>,
}

/// One 0x520-byte tileset slot of the array at `0x00fa3970`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct A57TilesetSlot {
    /// `+0x000`: file slot 3 (`0x00604540`), freed first by
    /// `0x00604290`.
    pub primary: Option<Vec<u8>>,
    /// `+0x004`: file slot 0, the minitile records.
    pub minitiles: Option<Vec<u8>>,
    /// `+0x008`: `u16` record count, `file0.len() >> 5` truncated to 16 bits.
    pub minitile_count: u16,
    /// `+0x00c`: file slot 1, the megatile records.
    pub megatiles: Option<Vec<u8>>,
    /// `+0x010`: `file1.len() / 0x34`.
    pub megatile_count: u32,
    /// `+0x014`: file slot 2.
    pub art: Option<Vec<u8>>,
    /// `+0x018`: byte size of file slot 4 (not owned by the destructor).
    pub probe_size: u32,
    /// `+0x518`: one flags word per minitile record.
    pub mtx_flags: Option<Vec<u32>>,
    /// `+0x51c`: era id.
    pub era: u16,
    /// `+0x51e`: ready byte.
    pub initialized: bool,
}

/// The `u16` count truncation used at both grid sites:
/// `(short)(size >> 5)` in `0x00604540` and `0x006047b0`.
pub fn a57_minitile_count(size: u32) -> u16 {
    (size >> 5) as u16
}

/// The grid allocation size: `(uint)*(ushort *)(this + 8) << 2`.
pub fn a57_mtx_grid_size(count: u16) -> u32 {
    u32::from(count) << 2
}

/// The per-cell flag computation of the out-of-window grid routine
/// `FUN_0060b1b0`, reproduced inline for `0x00604250`/`0x00604540`:
///
/// ```text
/// n = count of even-offset bytes with bit 0 set (0..16)
/// flags = (n > 0xc ? 0x10000 : 0) | (0 < n < 16 ? 0x20000000 : 0)
/// c4, c2, c8 = counts of ushort bit 2, bit 1, bit 3 (0..16)
/// if (c4 < 0xc) { if (c4 + c2 > 0xb) flags |= 0x02000000 }
/// else          { flags |= 0x04000000 }
/// if (c8 != 0) flags |= 0x01000000
/// if (flags & 0x10000) == 0 flags |= 0x00040000
/// ```
///
/// The 16 ushorts are read little-endian from the 32-byte record.
pub fn a57_mtx_flags(record: &[u8; A57_MINITILE_STRIDE]) -> u32 {
    let mut parity = 0u32;
    for block in 0..4 {
        for off in [8 * block, 8 * block + 2, 8 * block + 4, 8 * block + 6] {
            if record[off] & 1 != 0 {
                parity += 1;
            }
        }
    }
    let mut flags = if parity > 0xc { 0x0001_0000 } else { 0 };
    if parity != 0 && parity < 16 {
        flags |= 0x2000_0000;
    }

    let (mut bit2, mut bit1, mut bit3) = (0u32, 0u32, 0u32);
    for i in 0..16 {
        let value = u16::from_le_bytes([record[2 * i], record[2 * i + 1]]);
        if value & 4 != 0 {
            bit2 += 1;
        }
        if value & 2 != 0 {
            bit1 += 1;
        }
        if value & 8 != 0 {
            bit3 += 1;
        }
    }

    if bit2 < 0xc {
        if bit2 + bit1 > 0xb {
            flags |= 0x0200_0000;
        }
    } else {
        flags |= 0x0400_0000;
    }

    if bit3 != 0 {
        flags |= 0x0100_0000;
    }
    if flags & 0x0001_0000 == 0 {
        flags |= 0x0004_0000;
    }
    flags
}

/// Map `count` 0x20-byte records of `records` to grid words.  A short buffer
/// yields only the records that are present; the shipped routine trusts the
/// count and would read past it, which cannot occur when the count comes
/// from the same byte size (`0x00604540`/`0x006047b0`).
pub fn a57_build_mtx_flags(count: u16, records: &[u8]) -> Vec<u32> {
    let available = records.len() / A57_MINITILE_STRIDE;
    let usable = (count as usize).min(available);
    (0..usable)
        .map(|i| {
            let mut record = [0u8; A57_MINITILE_STRIDE];
            let at = i * A57_MINITILE_STRIDE;
            record.copy_from_slice(&records[at..at + A57_MINITILE_STRIDE]);
            a57_mtx_flags(&record)
        })
        .collect()
}

/// `0x00fa3970 + (slot as u32) * 0x520`, the value returned by
/// `0x00604540`.
pub fn a57_slot_data_address(slot: u16) -> u32 {
    A57_SLOT_ARRAY_BASE.wrapping_add(u32::from(slot).wrapping_mul(A57_TILESET_STRIDE))
}

/// `0x00604340`: the slot pointer of the globally selected era,
/// `0x00fa6270 + DAT_00fa3968 * 0x520`.  The global index is passed in so
/// the function is deterministic; the arithmetic is 32-bit wrapping, as in
/// `&DAT_00fa6270 + (uint)DAT_00fa3968 * 0x520`.
pub fn a57_active_slot_address(era: u32) -> u32 {
    A57_ACTIVE_ARRAY_BASE.wrapping_add(era.wrapping_mul(A57_TILESET_STRIDE))
}

/// The file-name table lookup used by the path builders (`0x00604450`,
/// `0x00604690`) and the error path (`0x006048b0`).  The retail table has
/// exactly eight entries; an out-of-range era is returned as `None` instead
/// of reproducing the out-of-bounds read.
pub fn a57_tileset_name(era: u16) -> Option<&'static str> {
    A57_TILESET_NAMES.get(usize::from(era)).copied()
}

/// The `0x006048b0` message: `{0}{1}{2}` with `Tileset\`, the era name and
/// `.vr4`.
pub fn a57_missing_asset_path(era: u16) -> Option<String> {
    a57_tileset_name(era).map(|name| format!("{A57_TILESET_DIR}{name}{A57_VR4_SUFFIX}"))
}

/// `0x00604450` (223 bytes, `decompiled_uncited`).
///
/// Formats `{0}{1}{2}{3}{4}` where argument 0 is the 8-byte
/// `std::string_view` copied from `param_3`/`param_4` (root, length) and the
/// four fixed arguments are `Tileset\` (`0x00d390d0`), the 0x1c-byte name
/// table entry (`0x00f04758 + era * 0x1c`), `.DDS` (`0x00d33588`) and
/// `.grp` (`0x00d390ec`).  The `param_5` sign-bit test and the
/// `ExceptionList` restore at the end of the decompiled body are the
/// exception funclet, not output data.
pub fn a57_hd_path_grp(root: &str, era: u16) -> Option<String> {
    a57_tileset_name(era)
        .map(|name| format!("{root}{A57_TILESET_DIR}{name}{A57_DDS_SUFFIX}{A57_GRP_SUFFIX}"))
}

/// `0x00604690` (223 bytes, `decompiled_uncited`).
///
/// The same five-argument body as `0x00604450`; only the last fixed string
/// differs: `.vr4` (`0x00d390c8`) instead of `.grp`.  The resulting path is
/// `<root>Tileset\<name>.DDS.vr4`.
pub fn a57_hd_path_vr4(root: &str, era: u16) -> Option<String> {
    a57_tileset_name(era)
        .map(|name| format!("{root}{A57_TILESET_DIR}{name}{A57_DDS_SUFFIX}{A57_VR4_SUFFIX}"))
}

/// `0x006040c0` (28 bytes, `decompiled_uncited`).
///
/// The mtx validation reporter formats `BUG: mtx %d has mid+high cell`
/// with `index` (severity 10000, the second argument 0).  The formatted text
/// is the observable value; the 256-byte stack buffer and the report sink
/// are not modelled.
pub fn a57_mtx_bug_message(index: i32) -> String {
    format!("BUG: mtx {index} has mid+high cell")
}

impl A57TilesetSlot {
    /// `0x006040e0` (10 bytes, `decompiled_uncited`).
    ///
    /// `*(byte *)(this + 0x51e) = 0`: clears the ready flag of the slot.
    pub fn a57_clear_initialized(&mut self) {
        self.initialized = false;
    }

    /// `0x00604250` (55 bytes, `decompiled_uncited`).
    ///
    /// Allocates `(u16 at +8) << 2` bytes, stores the pointer at `+0x518`
    /// and fills one word per 0x20-byte record starting at `+4` with
    /// [`a57_mtx_flags`].  The fourth argument `0x006040c0` is the reporter;
    /// the recovered grid body never calls it.  Returns the allocated byte
    /// size.
    pub fn a57_build_mtx_grid(&mut self) -> u32 {
        let words = a57_build_mtx_flags(
            self.minitile_count,
            self.minitiles.as_deref().unwrap_or(&[]),
        );
        self.mtx_flags = Some(words);
        a57_mtx_grid_size(self.minitile_count)
    }

    /// `0x00604290` (174 bytes, `decompiled_uncited`).
    ///
    /// The slot teardown: frees `+0x000`, `+0x004`, `+0x518`, `+0x00c` and
    /// `+0x014`, in that order, skipping null pointers; then clears the
    /// `u16` at `+0x008`, the dword at `+0x010` and the ready byte at
    /// `+0x51e`.  `+0x018` and `+0x51c` are deliberately left alone, and the
    /// returned count is the number of released blocks.
    pub fn a57_release(&mut self) -> usize {
        let mut freed = 0;
        if self.primary.take().is_some() {
            freed += 1;
        }
        if self.minitiles.take().is_some() {
            freed += 1;
        }
        if self.mtx_flags.take().is_some() {
            freed += 1;
        }
        if self.megatiles.take().is_some() {
            freed += 1;
        }
        if self.art.take().is_some() {
            freed += 1;
        }
        self.minitile_count = 0;
        self.megatile_count = 0;
        self.initialized = false;
        freed
    }

    /// `0x00604540` (332 bytes, `decompiled_uncited`).
    ///
    /// Lazily loads the slot at `param_1 * 0x520`.  The already-ready guard
    /// is the byte at `+0x51e`; it returns `0x00fa3970 + slot * 0x520`
    /// without touching the assets.  Otherwise:
    ///
    /// 1. `0x006048b0` requires the asset record's ready byte; a clear byte
    ///    produces the `Tileset\<era>.vr4` error.
    /// 2. File slot 0 goes to `+0x004`; the `u16` at `+0x008` is
    ///    `(size >> 5) as u16`, and `+0x518` receives
    ///    `count * 4` bytes of grid flags.
    /// 3. File slot 1 goes to `+0x00c`; `+0x010` is `size / 0x34`.
    /// 4. File slot 2 goes to `+0x014`; file slot 3 to `+0x000`.
    /// 5. File slot 4 contributes only its size to `+0x018` (the decompiler
    ///    shows the size-out pointer in the second argument position of the
    ///    reader; the buffer itself is not retained).
    /// 6. The era argument is truncated to 16 bits into `+0x51c`, and the
    ///    ready byte is set.
    ///
    /// The asset record is the one selected by the *old* `+0x51c` value:
    /// the rebuild reads the era before writing the argument, which the
    /// passed-in `files` value represents.
    pub fn a57_load(&mut self, slot: u16, files: &A57AssetFiles) -> Result<u32, String> {
        let address = a57_slot_data_address(slot);
        if self.initialized {
            return Ok(address);
        }
        self.a57_require_asset(files.loaded)?;
        self.minitiles = Some(files.file0.clone());
        self.minitile_count = a57_minitile_count(files.file0.len() as u32);
        self.mtx_flags = Some(a57_build_mtx_flags(self.minitile_count, &files.file0));
        self.megatiles = Some(files.file1.clone());
        self.megatile_count = (files.file1.len() as u32) / A57_MEGATILE_STRIDE;
        self.art = Some(files.file2.clone());
        self.primary = Some(files.file3.clone());
        self.probe_size = files.file4.len() as u32;
        self.era = slot;
        self.initialized = true;
        Ok(address)
    }

    /// `0x006047b0` (72 bytes, `decompiled_uncited`).
    ///
    /// Re-reads file slot 0 for the era at `+0x51c`: the reader's buffer
    /// goes to `+0x004` and `(short)(size >> 5)` to `+0x008`.
    pub fn a57_attach_minitiles(&mut self, bytes: Vec<u8>) {
        self.minitile_count = a57_minitile_count(bytes.len() as u32);
        self.minitiles = Some(bytes);
    }

    /// `0x006048b0` (207 bytes, `decompiled_uncited`).
    ///
    /// The missing-asset guard: when the asset record's ready byte is clear
    /// it builds `Tileset\<era>.vr4` with `{0}{1}{2}` and enters the
    /// non-returning throw path; otherwise the routine falls through with no
    /// side effects.  The `Result` reports the two branches.
    pub fn a57_require_asset(&self, loaded: bool) -> Result<(), String> {
        if loaded {
            return Ok(());
        }
        Err(a57_missing_asset_path(self.era).unwrap_or_default())
    }
}

/// The global `#xN` selection state read by `0x00604c30`/`0x00604c40`/
/// `0x00604c50` and written by `0x00604c60`/`0x00604d70`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A57Variant {
    /// The global at `0x00fa8ff0`; `0` means "no `#xN` suffix".
    pub index: u32,
}

impl A57Variant {
    /// `0x00604c30` (6 bytes, `decompiled_uncited`).
    ///
    /// `return DAT_00fa8ff0;`
    pub fn a57_index(&self) -> u32 {
        self.index
    }

    /// `0x00604c40` (13 bytes, `decompiled_uncited`).
    ///
    /// `return *(u32 *)(0x00f04838 + DAT_00fa8ff0 * 4);` over the six-dword
    /// table [`A57_VARIANT_TABLE`].  The parser only stores 0..5; an index
    /// outside that range reads adjacent globals in retail and is reported
    /// as `None` here.
    pub fn a57_mapped(&self) -> Option<u32> {
        A57_VARIANT_TABLE.get(self.index as usize).copied()
    }

    /// `0x00604c50` (11 bytes, `decompiled_uncited`).
    ///
    /// `return DAT_00fa8ff0 != 0;`
    pub fn a57_selected(&self) -> bool {
        self.index != 0
    }

    /// `0x00604c60` (162 bytes, `decompiled_uncited`).
    ///
    /// Scans the name for the marker `#x1`..`#x5` (`x` case-insensitive)
    /// and stores the digit; every other outcome stores 0.  The recovered
    /// loop tests only the first `#`: at position `i` it needs
    /// `len - (i + 1) >= 2`, then requires `name[i + 1]` to be `x`/`X` and
    /// `name[i + 2]` to be `1`..`5`.  The `i == -1` branch in the decompiled
    /// body is unreachable (`i` starts at 0); a null name is the `None`
    /// argument.
    pub fn a57_parse_name(&mut self, name: Option<&str>) {
        let Some(name) = name else {
            self.index = 0;
            return;
        };
        let bytes = name.as_bytes();
        if bytes.is_empty() {
            self.index = 0;
            return;
        }
        for i in 0..bytes.len() {
            if bytes[i] != b'#' {
                continue;
            }
            if bytes.len() - (i + 1) < 2 {
                self.index = 0;
                return;
            }
            if bytes[i + 1] != b'x' && bytes[i + 1] != b'X' {
                self.index = 0;
                return;
            }
            self.index = match bytes[i + 2] {
                digit @ b'1'..=b'5' => u32::from(digit - b'0'),
                _ => 0,
            };
            return;
        }
        self.index = 0;
    }

    /// `0x00604d70` (11 bytes, `decompiled_uncited`).
    ///
    /// `DAT_00fa8ff0 = 0;`
    pub fn a57_reset(&mut self) {
        self.index = 0;
    }
}

/// Fields cleared individually by `0x00604d80` (all zero after the
/// constructor).  The retail stores are unaligned for `+0x102`/`+0x106`;
/// the model keeps them as named words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A57DisplayFlags {
    /// `+0xf0` byte.
    pub at_f0: u8,
    /// `+0xf4` dword.
    pub at_f4: u32,
    /// `+0xf8` dword.
    pub at_f8: u32,
    /// `+0xfc` dword.
    pub at_fc: u32,
    /// `+0x100` byte (the flag `0x00605a20`/`0x00605b40` overwrite).
    pub at_100: u8,
    /// `+0x102` dword (unaligned in retail).
    pub at_102: u32,
    /// `+0x106` dword (unaligned in retail).
    pub at_106: u32,
    /// `+0x10a` word.
    pub at_10a: u16,
    /// `+0x10c` byte.
    pub at_10c: u8,
    /// `+0x110` dword.
    pub at_110: u32,
    /// `+0x1dc` dword.
    pub at_1dc: u32,
}

/// The `0x00604d80` object: nine strings, an intrusive list, eleven cleared
/// fields and a 0xbd-byte zeroed block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A57DisplayObject {
    /// `+0x00..+0x1f`: eight header dwords.  The constructor writes
    /// `+0x00` and `+0x08..+0x1c`; `+0x04` is not written by the recovered
    /// body.
    pub head: [u32; 8],
    /// Seven empty strings at `+0x20`, `+0x3c`, ..., `+0xc8`.
    pub strings: [A57String; 7],
    /// `+0xe4`: the intrusive list header.
    pub list: A57List,
    /// `+0xf0..+0x113`: cleared flags.
    pub flags: A57DisplayFlags,
    /// `+0x114..+0x1d0`: 0xbd zero bytes.
    pub blob: [u8; 0xbd],
    /// `+0x1d4`: cleared dword.
    pub field_1d4: u32,
    /// `+0x1d8` and `+0x1f4`: two more empty strings.
    pub tail_strings: [A57String; 2],
}

impl A57DisplayObject {
    /// A zeroed object with the string buffers already in their legal empty
    /// state; used before [`Self::a57_init`] and by the tests.
    pub fn a57_new() -> Self {
        A57DisplayObject {
            head: [0; 8],
            strings: std::array::from_fn(|_| a57_string_empty()),
            list: A57_LIST_EMPTY,
            flags: A57DisplayFlags::default(),
            blob: [0; 0xbd],
            field_1d4: 0,
            tail_strings: std::array::from_fn(|_| a57_string_empty()),
        }
    }

    /// `0x00604d80` (423 bytes, `decompiled_uncited`).
    ///
    /// The object constructor.  In recovered order: `+0x00 = 0`,
    /// `+0x08..+0x1c = 0`, seven empty strings at `+0x20`..`+0xc8` (each
    /// `{self+0x0c, 0, 0x8000000f, 0}`), self-linked list at `+0xe4`, the
    /// individually cleared fields at `+0x102`, `+0x106`, `+0x10a`, `+0x10c`,
    /// `+0x110`, `+0xf0`, `+0xf4`..`+0xfc`, `+0x100`, `+0x1d4`, `+0x1dc`,
    /// the two trailing strings at `+0x1d8`/`+0x1f4`, and finally
    /// `memset(this + 0x114, 0, 0xbd)`.  `+0x04` is not written.
    pub fn a57_init(&mut self) {
        self.head[0] = 0;
        for word in &mut self.head[2..] {
            *word = 0;
        }
        for slot in &mut self.strings {
            *slot = a57_string_empty();
        }
        self.list = A57_LIST_EMPTY;
        self.flags.at_f0 = 0;
        self.flags.at_f4 = 0;
        self.flags.at_f8 = 0;
        self.flags.at_fc = 0;
        self.flags.at_100 = 0;
        self.flags.at_102 = 0;
        self.flags.at_106 = 0;
        self.flags.at_10a = 0;
        self.flags.at_10c = 0;
        self.flags.at_110 = 0;
        self.flags.at_1dc = 0;
        self.field_1d4 = 0;
        for slot in &mut self.tail_strings {
            *slot = a57_string_empty();
        }
        self.blob = [0; 0xbd];
    }

    /// The nine capacity words in ascending string-offset order
    /// ([`A57_DISPLAY_STRING_OFFSETS`]).
    pub fn a57_string_caps(&self) -> [u32; 9] {
        let mut caps = [0u32; 9];
        for (index, slot) in self.strings.iter().enumerate() {
            caps[index] = slot.cap;
        }
        caps[7] = self.tail_strings[0].cap;
        caps[8] = self.tail_strings[1].cap;
        caps
    }

    /// `0x00605010` (292 bytes, `decompiled_uncited`).
    ///
    /// The teardown chain, in the exact `local_8` order of the shipped
    /// body:
    ///
    /// ```text
    /// local_8 0: check capacity +0x1fc (tail string at +0x1f4)
    /// local_8 1: check capacity +0x1e0 (tail string at +0x1d8)
    /// local_8 2: reset the list at +0xe4 (0x00605190 / FUN_005e8790)
    /// local_8 3: check capacity +0xd0 (string at +0xc8)
    /// local_8 4: +0xb4 (string at +0xac)
    /// local_8 5: +0x98 (string at +0x90)
    /// local_8 6: +0x7c (string at +0x74)
    /// local_8 7: +0x60 (string at +0x58)
    /// local_8 8: +0x44 (string at +0x3c)
    /// local_8 9: +0x28 (string at +0x20)
    /// ```
    ///
    /// Each check rejects a capacity without the `0x80000000` tag.  The
    /// returned value is the shipped `local_8` state of the failing check
    /// (`2` never fails: it is the list reset); `Ok(was_empty)` reports
    /// whether the list reset found an empty list.  The reset happens after
    /// the first two checks, so a failure at 0 or 1 leaves the list
    /// untouched.
    pub fn a57_teardown(&mut self) -> Result<bool, usize> {
        if !a57_capacity_initialized(self.tail_strings[1].cap) {
            return Err(0);
        }
        if !a57_capacity_initialized(self.tail_strings[0].cap) {
            return Err(1);
        }
        let was_empty = a57_list_reset(&mut self.list);
        for (offset, slot) in self.strings.iter().rev().enumerate() {
            if !a57_capacity_initialized(slot.cap) {
                return Err(offset + 3);
            }
        }
        Ok(was_empty)
    }
}

/// The `0x00604f40` object: the name string, five numeric initialisers, the
/// three trailing strings and the list at `+0x11c`.  The `+0x08..+0x1f` span
/// is not written by the recovered body and is not modelled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A57MapObject {
    /// `+0x00`: cleared dword.
    pub head: u32,
    /// `+0x04`: empty string.
    pub name: A57String,
    /// `+0x20`, `+0x24`, `+0x28`, `+0x2c`, `+0x30`: `[0, 0, 1, 8, 0]`.
    pub words: [u32; 5],
    /// `+0x34`: cleared byte.
    pub byte_34: u8,
    /// `+0xc4`, `+0xe0`, `+0xfc`: three empty strings.
    pub strings: [A57String; 3],
    /// `+0x11c`: the intrusive list header.
    pub list: A57List,
}

impl A57MapObject {
    /// A zeroed object with legal empty strings, before [`Self::a57_init`].
    pub fn a57_new() -> Self {
        A57MapObject {
            head: 0,
            name: a57_string_empty(),
            words: [0; 5],
            byte_34: 0,
            strings: std::array::from_fn(|_| a57_string_empty()),
            list: A57_LIST_EMPTY,
        }
    }

    /// `0x00604f40` (204 bytes, `decompiled_uncited`).
    ///
    /// The constructor: `+0x00 = 0`, empty string at `+0x04`, the numeric
    /// run `+0x20 = 0`, `+0x24 = 0`, `+0x28 = 1`, `+0x2c = 8`, `+0x30 = 0`,
    /// byte `+0x34 = 0`, empty strings at `+0xc4`/`+0xe0`/`+0xfc`, and the
    /// self-linked list at `+0x11c`.
    pub fn a57_init(&mut self) {
        self.head = 0;
        self.name = a57_string_empty();
        self.words = [0, 0, 1, 8, 0];
        self.byte_34 = 0;
        for slot in &mut self.strings {
            *slot = a57_string_empty();
        }
        self.list = A57_LIST_EMPTY;
    }

    /// The four capacity words in ascending string-offset order
    /// ([`A57_MAP_STRING_OFFSETS`]).
    pub fn a57_string_caps(&self) -> [u32; 4] {
        [
            self.name.cap,
            self.strings[0].cap,
            self.strings[1].cap,
            self.strings[2].cap,
        ]
    }

    /// `0x00605220` (157 bytes, `decompiled_uncited`).
    ///
    /// Checks, in `local_8` order 0..3: capacity `+0x104` (string at
    /// `+0xfc`), `+0xe8` (string at `+0xe0`), `+0xcc` (string at `+0xc4`)
    /// and `+0x0c` (string at `+0x04`).  `Err(step)` is the first rejected
    /// capacity; every check tests the `0x80000000` tag.
    pub fn a57_validate(&self) -> Result<(), usize> {
        for (step, slot) in self.strings.iter().rev().enumerate() {
            if !a57_capacity_initialized(slot.cap) {
                return Err(step);
            }
        }
        if !a57_capacity_initialized(self.name.cap) {
            return Err(3);
        }
        Ok(())
    }

    /// `0x006052f0` (175 bytes, `decompiled_uncited`).
    ///
    /// `local_8 = 0` resets the list (the out-of-window `FUN_005e8790`, whose
    /// body is the same as `0x00605190`), then `local_8 = 1..4` repeats the
    /// four capacity checks of [`Self::a57_validate`].  A failed check is
    /// reported as `Err(local_8)` (1..4), and `Ok(was_empty)` reports the
    /// list result.
    pub fn a57_teardown(&mut self) -> Result<bool, usize> {
        let was_empty = a57_list_reset(&mut self.list);
        self.a57_validate().map_err(|step| step + 1)?;
        Ok(was_empty)
    }
}

/// An `{id, string}` record moved by `0x00605430`, `0x006054e0` and
/// `0x006055e0`.  The id is copied first (`*in_ECX = *param_1`), then the
/// string at `param_1 + 1` is moved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A57IdString {
    /// The leading dword.
    pub id: u32,
    /// The string at `+0x04`.
    pub text: A57String,
}

/// The shared move: copy the id, move the string and reset the source
/// header.  The capacity masking is [`a57_move_string`]'s recovered store.
fn a57_take_record(src: &mut A57IdString) -> A57IdString {
    let id = src.id;
    let text = a57_move_string(&mut src.text);
    A57IdString { id, text }
}

/// `0x00605430` (31 bytes, `decompiled_uncited`).
///
/// A move constructor of an `{u32, string}` record: `*dest = *src` for the
/// leading dword and the out-of-window `FUN_0053ae90` string move for the
/// member at `+0x04`.
pub fn a57_take_record_5430(src: &mut A57IdString) -> A57IdString {
    a57_take_record(src)
}

/// `0x006054e0` (31 bytes, `decompiled_uncited`).
///
/// Byte-for-byte the same body as `0x00605430` (a second class's move
/// constructor).
pub fn a57_take_record_54e0(src: &mut A57IdString) -> A57IdString {
    a57_take_record(src)
}

/// `0x006055e0` (31 bytes, `decompiled_uncited`).
///
/// Byte-for-byte the same body as `0x00605430` (a third class's move
/// constructor).
pub fn a57_take_record_55e0(src: &mut A57IdString) -> A57IdString {
    a57_take_record(src)
}

/// `0x006053d0` (29 bytes, `decompiled_uncited`).
///
/// Reserves room for the size at `param_1 + 4` in the destination string
/// (`ECX`): this is the out-of-window `FUN_00538810` growth policy applied
/// with the destination's `+0x08` capacity.  Returns the replacement
/// capacity word or `None` when no growth is needed.
pub fn a57_prepare_copy(cap: u32, source_len: u32) -> Option<u32> {
    a57_reserve_capacity(cap, source_len)
}

/// `0x00605660` (58 bytes, `decompiled_uncited`).
///
/// `string::append(const char *)`: measures `param_1` (the shipped loop is
/// `strlen`), then reserves `*(this + 4) + length` bytes in the string at
/// `ECX`.  The returned requirement is the value handed to the growth
/// policy of [`a57_reserve_capacity`]; the addition is 32-bit wrapping.
pub fn a57_append_required_size(current_size: u32, text: &str) -> u32 {
    current_size.wrapping_add(text.len() as u32)
}

/// MurmurHash3 x86-32, the body of `FUN_00c62800` used as the string map's
/// hasher.  The shipped code takes `(ptr, len, seed)`; the key is hashed with
/// seed 0.  The multiply/or pairs in the decompiled body are the rotated
/// `0xcc9e2d51` product and `0x1b873593` mix, and
/// `0xfaddaf14 * 5 == 0xe6546b64` is the tail constant.
pub fn a57_murmur3_32(bytes: &[u8], seed: u32) -> u32 {
    let mut h = seed;
    let (blocks, tail) = bytes.as_chunks::<4>();
    for block in blocks {
        let k = u32::from_le_bytes(*block);
        let k = k
            .wrapping_mul(0xcc9e_2d51)
            .rotate_left(15)
            .wrapping_mul(0x1b87_3593);
        h ^= k;
        h = h.rotate_left(13).wrapping_mul(5).wrapping_add(0xe654_6b64);
    }
    let mut t = 0u32;
    if tail.len() >= 3 {
        t ^= u32::from(tail[2]) << 16;
    }
    if tail.len() >= 2 {
        t ^= u32::from(tail[1]) << 8;
    }
    if let Some(&first) = tail.first() {
        t ^= u32::from(first);
        h ^= t
            .wrapping_mul(0xcc9e_2d51)
            .rotate_left(15)
            .wrapping_mul(0x1b87_3593);
    }
    h ^= bytes.len() as u32;
    a57_fmix32(h)
}

/// The MurmurHash3 32-bit finaliser (`x ^= x >> 16; x *= 0x85ebca6b;
/// x ^= x >> 13; x *= 0xc2b2ae35; x ^= x >> 16`), used verbatim by
/// `0x00605890`'s bucket hash and by [`a57_murmur3_32`].
pub fn a57_fmix32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x85eb_ca6b);
    x ^= x >> 13;
    x = x.wrapping_mul(0xc2b2_ae35);
    x ^= x >> 16;
    x
}

/// `hash & (buckets - 1)`, the bucket selection of both maps.  The maps
/// start at [`A57_MAP_INITIAL_BUCKETS`] (a power of two) and grow by
/// doubling (the out-of-window grow helper's `uVar5 = uVar5 * 2`).
pub fn a57_bucket_index(hash: u32, buckets: u32) -> u32 {
    hash & buckets.wrapping_sub(1)
}

/// The string-keyed id map of `0x006056e0`: keys are 8-byte string views at
/// node `+0x04`, values at node `+0x20`, and new keys receive the next
/// counter value.  Insertion order is modelled deterministically.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A57StringIntern {
    entries: Vec<(Vec<u8>, u32)>,
    next_id: u32,
}

impl A57StringIntern {
    /// A map with the image's counter (`1`) and no nodes.
    pub fn a57_new() -> Self {
        A57StringIntern {
            entries: Vec::new(),
            next_id: A57_MAP_COUNTER_BASE,
        }
    }

    /// The value stored at node `+0x20` for an existing key.
    pub fn a57_lookup(&self, key: &[u8]) -> Option<u32> {
        self.entries
            .iter()
            .find(|(stored, _)| stored.as_slice() == key)
            .map(|(_, value)| *value)
    }

    /// `0x006056e0` (217 bytes, `decompiled_uncited`).
    ///
    /// Looks the key up in the murmur3-hashed map; an existing entry returns
    /// its stored value unchanged.  A new key takes `DAT_00f04850` as its
    /// value and leaves the counter incremented.  The shipped lookup hashes
    /// `(key.ptr, key.len)` with the MurmurHash3 body at `FUN_00c62800` and
    /// indexes with `& (buckets - 1)`; [`a57_murmur3_32`] and
    /// [`a57_bucket_index`] reproduce those operations.
    pub fn a57_intern(&mut self, key: &[u8]) -> u32 {
        if let Some(value) = self.a57_lookup(key) {
            return value;
        }
        let value = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.entries.push((key.to_vec(), value));
        value
    }

    /// The `DAT_00f04850` counter.
    pub fn a57_next_id(&self) -> u32 {
        self.next_id
    }

    /// Number of stored keys.
    pub fn a57_len(&self) -> usize {
        self.entries.len()
    }

    /// The bucket occupied by `key` in the initial table.
    pub fn a57_bucket(&self, key: &[u8]) -> u32 {
        a57_bucket_index(a57_murmur3_32(key, 0), A57_MAP_INITIAL_BUCKETS)
    }
}

/// The pair-keyed id map of `0x00605890`: the key is the two dwords at node
/// `+0x08`/`+0x0c`, the value at node `+0x10`, and the bucket hash is
/// `fmix32(low_key) & (buckets - 1)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A57PairIntern {
    entries: Vec<((u32, u32), u32)>,
    next_id: u32,
    buckets: u32,
}

impl A57PairIntern {
    /// A map with the image's counter (`1`) and no bucket array
    /// (`DAT_00f04868 = 0`).
    pub fn a57_new() -> Self {
        A57PairIntern {
            entries: Vec::new(),
            next_id: A57_MAP_COUNTER_BASE,
            buckets: 0,
        }
    }

    /// The value stored at node `+0x10` for an existing key.
    pub fn a57_lookup(&self, first: u32, second: u32) -> Option<u32> {
        self.entries
            .iter()
            .find(|((a, b), _)| *a == first && *b == second)
            .map(|(_, value)| *value)
    }

    /// `0x00605890` (259 bytes, `decompiled_uncited`).
    ///
    /// Hash lookup then the counter update.  The shipped body computes
    /// `fmix32(param_1) & (DAT_00f04864 - 1)` for the bucket, walks the chain
    /// comparing both key dwords, and returns the existing value when the
    /// node is found.  The insert path reads `iVar1 = DAT_00f04850 + 1`,
    /// stores the *current* counter in the node (`iVar3 = DAT_00f04850`),
    /// then commits `DAT_00f04850 = iVar1`; the lazy bucket-array creation
    /// and a missing node both end in the decompile's non-returning calls.
    /// The translation keeps the stores of the returning path: value =
    /// counter, counter = counter + 1, so a fresh map assigns 1, 2, 3...
    pub fn a57_intern(&mut self, first: u32, second: u32) -> u32 {
        if self.buckets != 0 {
            if let Some(value) = self.a57_lookup(first, second) {
                return value;
            }
        } else {
            self.buckets = A57_MAP_INITIAL_BUCKETS;
        }
        let value = self.next_id;
        self.next_id = value.wrapping_add(1);
        self.entries.push(((first, second), value));
        value
    }

    /// The `DAT_00f04850` counter.
    pub fn a57_next_id(&self) -> u32 {
        self.next_id
    }

    /// The current bucket count (`0` before the array is created).
    pub fn a57_buckets(&self) -> u32 {
        self.buckets
    }

    /// Number of stored keys.
    pub fn a57_len(&self) -> usize {
        self.entries.len()
    }

    /// The initial-table bucket of `first`.
    pub fn a57_bucket(&self, first: u32) -> u32 {
        a57_bucket_index(a57_fmix32(first), A57_MAP_INITIAL_BUCKETS)
    }
}

/// The object built by `0x00605a20`/`0x00605b40`: the `0x00604d80`
/// constructor's object with the leading dword replaced by the map id and
/// the `+0x100` byte replaced by the flag, plus the capacity reserved for
/// the copied source string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A57MapInfo {
    /// The `0x00604d80` object (`head[0]` is the id, `flags.at_100` the
    /// flag).
    pub object: A57DisplayObject,
    /// The capacity word requested for the source string's size.
    pub text_capacity: u32,
}

fn a57_map_info_from_id(id: u32, flag: u8, source: &A57String) -> A57MapInfo {
    let mut object = A57DisplayObject::a57_new();
    object.a57_init();
    object.head[0] = id;
    object.flags.at_100 = flag;
    let text_capacity = a57_reserve_capacity(A57_SSO_CAPACITY, source.text.len() as u32)
        .unwrap_or(A57_SSO_CAPACITY);
    A57MapInfo {
        object,
        text_capacity,
    }
}

/// `0x00605a20` (119 bytes, `decompiled_uncited`).
///
/// Constructs the `0x00604d80` object, stores the pair map's id in the
/// leading dword (`*param_1 = id`), stores `param_4` in the `+0x100` byte
/// (`*(param_1 + 0x40) = param_4`), then reserves
/// `*(source + 4)` bytes in the destination string.  The destination
/// string subobject is passed in `ECX` and its offset is not part of the
/// decompiled body; the requested capacity is returned separately.
pub fn a57_map_info_pair(
    map: &mut A57PairIntern,
    first: u32,
    second: u32,
    flag: u8,
    source: &A57String,
) -> A57MapInfo {
    let id = map.a57_intern(first, second);
    a57_map_info_from_id(id, flag, source)
}

/// `0x00605b40` (129 bytes, `decompiled_uncited`).
///
/// The string-keyed twin of `0x00605a20`: the leading dword receives the
/// value from `0x006056e0`, the `+0x100` byte receives `param_3`, and the
/// destination string is reserved for `*(source + 4)` bytes.
pub fn a57_map_info_string(
    map: &mut A57StringIntern,
    key: &[u8],
    flag: u8,
    source: &A57String,
) -> A57MapInfo {
    let id = map.a57_intern(key);
    a57_map_info_from_id(id, flag, source)
}

#[cfg(test)]
mod tests_agent57 {
    use super::*;

    fn record(byte: u8) -> [u8; A57_MINITILE_STRIDE] {
        [byte; A57_MINITILE_STRIDE]
    }

    fn new_slot(era: u16) -> A57TilesetSlot {
        A57TilesetSlot {
            era,
            ..A57TilesetSlot::default()
        }
    }

    fn display() -> A57DisplayObject {
        let mut object = A57DisplayObject::a57_new();
        object.a57_init();
        object
    }

    fn map_object() -> A57MapObject {
        let mut object = A57MapObject::a57_new();
        object.a57_init();
        object
    }

    fn string(text: &str) -> A57String {
        A57String {
            text: text.to_string(),
            cap: A57_SSO_CAPACITY,
        }
    }

    #[test]
    fn mtx_bug_message_is_the_recovered_literal() {
        // 0x006040c0, 0x00d39120
        assert_eq!(a57_mtx_bug_message(7), "BUG: mtx 7 has mid+high cell");
        assert_eq!(a57_mtx_bug_message(-1), "BUG: mtx -1 has mid+high cell");
        assert_eq!(A57_MTX_BUG_FORMAT, "BUG: mtx %d has mid+high cell");
    }

    #[test]
    fn clear_initialized_only_clears_the_ready_byte() {
        // 0x006040e0
        let mut slot = new_slot(3);
        slot.initialized = true;
        slot.probe_size = 0x55;
        slot.a57_clear_initialized();
        assert!(!slot.initialized);
        assert_eq!(slot.era, 3);
        assert_eq!(slot.probe_size, 0x55);
        assert_eq!(A57_SLOT_READY_OFFSET, 0x51e);
    }

    #[test]
    fn mtx_flags_reproduces_the_record_scan() {
        // 0x00604250 / FUN_0060b1b0
        assert_eq!(a57_mtx_flags(&record(0)), 0x0004_0000);
        assert_eq!(a57_mtx_flags(&record(0xff)), 0x0501_0000);

        let mut all_parity = record(0);
        for offset in 0..16 {
            all_parity[offset * 2] = 1;
        }
        assert_eq!(a57_mtx_flags(&all_parity), 0x0001_0000);

        let mut half_parity = record(0);
        for offset in 0..8 {
            half_parity[offset * 2] = 1;
        }
        assert_eq!(a57_mtx_flags(&half_parity), 0x2004_0000);

        let mut all_bit2 = record(0);
        for index in 0..16 {
            all_bit2[index * 2] = 4;
        }
        assert_eq!(a57_mtx_flags(&all_bit2), 0x0404_0000);

        let mut all_bit1 = record(0);
        for index in 0..16 {
            all_bit1[index * 2] = 2;
        }
        assert_eq!(a57_mtx_flags(&all_bit1), 0x0204_0000);

        let mut all_bit3 = record(0);
        for index in 0..16 {
            all_bit3[index * 2] = 8;
        }
        assert_eq!(a57_mtx_flags(&all_bit3), 0x0104_0000);

        let mut twelve_four = record(0);
        for index in 0..12 {
            twelve_four[index * 2] = 4;
        }
        for index in 0..16 {
            twelve_four[index * 2] |= 2;
        }
        assert_eq!(a57_mtx_flags(&twelve_four), 0x0404_0000);
    }

    #[test]
    fn mtx_grid_counts_four_bytes_per_record() {
        // 0x00604250: (uint)*(ushort *)(this + 8) << 2
        assert_eq!(a57_mtx_grid_size(0), 0);
        assert_eq!(a57_mtx_grid_size(1), 4);
        assert_eq!(a57_mtx_grid_size(0xffff), 0x0003_fffc);

        let first = record(0);
        let second = record(0xff);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&first);
        bytes.extend_from_slice(&second);
        assert_eq!(
            a57_build_mtx_flags(2, &bytes),
            vec![0x0004_0000, 0x0501_0000]
        );
        assert_eq!(a57_build_mtx_flags(3, &bytes).len(), 2);
    }

    #[test]
    fn build_mtx_grid_uses_the_stored_count_and_source() {
        // 0x00604250
        let mut slot = new_slot(0);
        slot.minitile_count = 2;
        slot.minitiles = Some({
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&record(0));
            bytes.extend_from_slice(&record(0xff));
            bytes
        });
        assert_eq!(slot.a57_build_mtx_grid(), 8);
        assert_eq!(
            slot.mtx_flags.as_deref(),
            Some(&[0x0004_0000, 0x0501_0000][..])
        );
        assert_eq!(A57_SLOT_MTX_OFFSET, 0x518);

        let mut empty = new_slot(0);
        empty.minitile_count = 0;
        assert_eq!(empty.a57_build_mtx_grid(), 0);
        assert_eq!(empty.mtx_flags.as_deref(), Some(&[][..]));
    }

    #[test]
    fn release_frees_five_blocks_and_keeps_probe_and_era() {
        // 0x00604290
        let mut slot = A57TilesetSlot {
            primary: Some(vec![1]),
            minitiles: Some(vec![0; A57_MINITILE_STRIDE]),
            minitile_count: 1,
            megatiles: Some(vec![0; A57_MEGATILE_STRIDE as usize]),
            megatile_count: 1,
            art: Some(vec![2]),
            probe_size: 0x99,
            mtx_flags: Some(vec![0x0004_0000]),
            era: 6,
            initialized: true,
        };
        assert_eq!(slot.a57_release(), 5);
        assert!(slot.primary.is_none());
        assert!(slot.minitiles.is_none());
        assert!(slot.mtx_flags.is_none());
        assert!(slot.megatiles.is_none());
        assert!(slot.art.is_none());
        assert_eq!(slot.minitile_count, 0);
        assert_eq!(slot.megatile_count, 0);
        assert!(!slot.initialized);
        assert_eq!(slot.probe_size, 0x99);
        assert_eq!(slot.era, 6);

        assert_eq!(new_slot(0).a57_release(), 0);
    }

    #[test]
    fn active_slot_address_is_the_second_array_plus_stride() {
        // 0x00604340
        assert_eq!(a57_active_slot_address(0), 0x00fa_6270);
        assert_eq!(a57_active_slot_address(7), 0x00fa_8650);
        assert_eq!(a57_active_slot_address(0x00cc_0000), 0x167a_6270);
    }

    #[test]
    fn hd_paths_use_the_recovered_name_table_and_suffixes() {
        // 0x00604450, 0x00604690, 0x00f04758
        assert_eq!(
            a57_hd_path_grp("", 0).as_deref(),
            Some("Tileset\\badlands.DDS.grp")
        );
        assert_eq!(
            a57_hd_path_vr4("", 0).as_deref(),
            Some("Tileset\\badlands.DDS.vr4")
        );
        assert_eq!(
            a57_hd_path_vr4("hd2/", 3).as_deref(),
            Some("hd2/Tileset\\AshWorld.DDS.vr4")
        );
        assert_eq!(a57_hd_path_grp("", 8), None);
        assert_eq!(
            A57_TILESET_NAMES,
            [
                "badlands", "platform", "install", "AshWorld", "Jungle", "Desert", "Ice",
                "Twilight"
            ]
        );
        assert_eq!(a57_tileset_name(3), Some("AshWorld"));
        assert_eq!(a57_tileset_name(7), Some("Twilight"));
        assert_eq!(a57_tileset_name(8), None);
        assert_eq!(A57_TILESET_DIR, "Tileset\\");
        assert_eq!(A57_DDS_SUFFIX, ".DDS");
        assert_eq!(A57_GRP_SUFFIX, ".grp");
        assert_eq!(A57_VR4_SUFFIX, ".vr4");
    }

    #[test]
    fn load_fills_the_slot_from_the_five_asset_slots() {
        // 0x00604540
        let mut files = A57AssetFiles::default();
        files.loaded = true;
        files.file0 = vec![0; A57_MINITILE_STRIDE * 3 + 7];
        files.file1 = vec![0; A57_MEGATILE_STRIDE as usize * 2 + 3];
        files.file2 = vec![7; 4];
        files.file3 = vec![8; 5];
        files.file4 = vec![9; 0x11];

        let mut slot = new_slot(5);
        let address = slot.a57_load(2, &files).unwrap();
        assert_eq!(address, 0x00fa_3970 + 2 * A57_TILESET_STRIDE);
        assert_eq!(slot.era, 2, "the argument is stored after the asset read");
        assert_eq!(slot.minitile_count, 3);
        assert_eq!(slot.megatile_count, 2);
        assert_eq!(slot.probe_size, 0x11);
        assert_eq!(slot.mtx_flags.as_deref().map(<[u32]>::len), Some(3));
        assert_eq!(slot.minitiles.as_deref().map(<[u8]>::len), Some(0x67));
        assert_eq!(slot.megatiles.as_deref().map(<[u8]>::len), Some(0x6b));
        assert_eq!(slot.art.as_deref(), Some(&[7, 7, 7, 7][..]));
        assert_eq!(slot.primary.as_deref(), Some(&[8, 8, 8, 8, 8][..]));
        assert!(slot.initialized);

        // The ready guard runs before the asset check: a second call with an
        // unready bundle still returns the address and changes nothing.
        let again = slot.a57_load(2, &A57AssetFiles::default()).unwrap();
        assert_eq!(again, address);
        assert_eq!(slot.era, 2);
    }

    #[test]
    fn load_requires_the_asset_ready_byte() {
        // 0x00604540 -> 0x006048b0
        let mut slot = new_slot(3);
        let error = slot.a57_load(1, &A57AssetFiles::default()).unwrap_err();
        assert_eq!(error, "Tileset\\AshWorld.vr4");
        assert!(!slot.initialized);
        assert!(a57_missing_asset_path(3).is_some());
        assert_eq!(
            a57_missing_asset_path(0).as_deref(),
            Some("Tileset\\badlands.vr4")
        );
    }

    #[test]
    fn require_asset_reports_the_two_branches() {
        // 0x006048b0
        assert_eq!(new_slot(0).a57_require_asset(true), Ok(()));
        assert_eq!(
            new_slot(0).a57_require_asset(false),
            Err("Tileset\\badlands.vr4".to_string())
        );
    }

    #[test]
    fn reload_minitiles_stores_the_pointer_and_truncated_count() {
        // 0x006047b0
        let mut slot = new_slot(0);
        slot.a57_attach_minitiles(vec![0; A57_MINITILE_STRIDE * 4 + 9]);
        assert_eq!(slot.minitile_count, 4);
        assert_eq!(
            slot.minitiles.as_deref().map(<[u8]>::len),
            Some(A57_MINITILE_STRIDE * 4 + 9)
        );
        assert_eq!(a57_minitile_count(0), 0);
        assert_eq!(a57_minitile_count(0x1f), 0);
        assert_eq!(a57_minitile_count(0x20), 1);
        assert_eq!(a57_minitile_count(0x20 * 0x1_0000), 0);
    }

    #[test]
    fn slot_data_address_is_the_first_array_plus_stride() {
        // 0x00604540 return
        assert_eq!(a57_slot_data_address(0), 0x00fa_3970);
        assert_eq!(a57_slot_data_address(1), 0x00fa_3e90);
        assert_eq!(a57_slot_data_address(0xffff), 0x061a_3450);
        assert_eq!(A57_TILESET_STRIDE, 0x520);
        assert_eq!(A57_SLOT_ARRAY_BASE, 0x00fa_3970);
        assert_eq!(A57_ACTIVE_ARRAY_BASE, 0x00fa_6270);
    }

    #[test]
    fn variant_accessors_use_the_recovered_table() {
        // 0x00604c30, 0x00604c40, 0x00604c50
        let mut variant = A57Variant::default();
        assert_eq!(variant.a57_index(), 0);
        assert_eq!(variant.a57_mapped(), Some(0));
        assert!(!variant.a57_selected());

        variant.index = 1;
        assert_eq!(variant.a57_index(), 1);
        assert_eq!(variant.a57_mapped(), Some(0x24));
        assert!(variant.a57_selected());

        variant.index = 5;
        assert_eq!(variant.a57_mapped(), Some(1));
        variant.index = 6;
        assert_eq!(variant.a57_mapped(), None);
        assert_eq!(A57_VARIANT_TABLE, [0, 0x24, 0x1d, 0x15, 0xc, 1]);
    }

    #[test]
    fn parse_name_finds_the_first_hash_marker() {
        // 0x00604c60
        let mut variant = A57Variant::default();
        for (name, expected) in [
            (None, 0),
            (Some(""), 0),
            (Some("foo"), 0),
            (Some("foo#x1"), 1),
            (Some("foo#X2"), 2),
            (Some("foo#x5"), 5),
            (Some("foo#x6"), 0),
            (Some("foo#y3"), 0),
            (Some("#x1"), 1),
            (Some("a#x"), 0),
            (Some("a#x12"), 1),
            (Some("#x1#x4"), 1),
            (Some("x#x2"), 2),
            (Some("#X"), 0),
        ] {
            variant.index = 9;
            variant.a57_parse_name(name);
            assert_eq!(variant.index, expected, "name {name:?}");
        }
    }

    #[test]
    fn reset_clears_the_variant_index() {
        // 0x00604d70
        let mut variant = A57Variant { index: 4 };
        variant.a57_reset();
        assert_eq!(variant.index, 0);
        assert!(!variant.a57_selected());
    }

    #[test]
    fn display_object_constructor_zeroes_every_member() {
        // 0x00604d80
        let mut object = A57DisplayObject::a57_new();
        object.head[1] = 0xdead_beef;
        object.strings[0].text = "junk".to_string();
        object.strings[0].cap = 0;
        object.flags.at_10a = 0xffff;
        object.flags.at_100 = 1;
        object.blob[0] = 1;
        object.field_1d4 = 3;
        object.list = A57List {
            first_self: false,
            last_self: false,
            size: 2,
        };
        object.a57_init();

        assert_eq!(object.head[0], 0);
        assert_eq!(object.head[1], 0xdead_beef, "+0x04 is not written");
        assert_eq!(
            object
                .strings
                .iter()
                .filter(|s| s.text.is_empty() && s.cap == A57_SSO_CAPACITY)
                .count(),
            7
        );
        assert_eq!(
            object
                .tail_strings
                .iter()
                .filter(|s| s.text.is_empty() && s.cap == A57_SSO_CAPACITY)
                .count(),
            2
        );
        assert_eq!(object.list, A57_LIST_EMPTY);
        assert_eq!(object.flags, A57DisplayFlags::default());
        assert_eq!(object.blob, [0u8; 0xbd]);
        assert_eq!(object.field_1d4, 0);
        assert_eq!(
            A57_DISPLAY_STRING_OFFSETS,
            [0x20, 0x3c, 0x58, 0x74, 0x90, 0xac, 0xc8, 0x1d8, 0x1f4]
        );
        assert_eq!(
            A57_DISPLAY_CAP_OFFSETS,
            [0x1fc, 0x1e0, 0xd0, 0xb4, 0x98, 0x7c, 0x60, 0x44, 0x28]
        );
    }

    #[test]
    fn display_teardown_checks_in_recovered_order() {
        // 0x00605010
        let mut object = display();
        assert_eq!(object.a57_teardown(), Ok(true));

        object.list = A57List {
            first_self: false,
            last_self: false,
            size: 1,
        };
        assert_eq!(object.a57_teardown(), Ok(false));
        assert_eq!(object.list, A57_LIST_EMPTY);

        // The first two checks precede the list reset.
        object.list = A57List {
            first_self: false,
            last_self: false,
            size: 1,
        };
        object.tail_strings[1].cap = 0;
        assert_eq!(object.a57_teardown(), Err(0));
        assert_eq!(object.list.size, 1);
        object.tail_strings[1].cap = A57_SSO_CAPACITY;

        object.tail_strings[0].cap = 0;
        assert_eq!(object.a57_teardown(), Err(1));
        assert_eq!(object.list.size, 1);
        object.tail_strings[0].cap = A57_SSO_CAPACITY;

        // State 3 is the first string after the reset (+0xc8, strings[6]);
        // state 2 is the list reset itself.
        object.strings[6].cap = 0;
        assert_eq!(object.a57_teardown(), Err(3));
        assert_eq!(object.list, A57_LIST_EMPTY);
        object.strings[6].cap = A57_SSO_CAPACITY;

        // State 6 is strings[3] (+0x74).
        object.strings[3].cap = 0;
        assert_eq!(object.a57_teardown(), Err(6));
        assert_eq!(object.strings[0].cap, A57_SSO_CAPACITY);
        assert_eq!(object.a57_string_caps()[3], 0);
    }

    #[test]
    fn list_reset_relinks_and_reports_the_old_state() {
        // 0x00605190
        let mut list = A57List {
            first_self: false,
            last_self: false,
            size: 3,
        };
        assert!(!a57_list_reset(&mut list));
        assert_eq!(list, A57_LIST_EMPTY);
        assert!(a57_list_reset(&mut list));
        assert_eq!(list, A57_LIST_EMPTY);
    }

    #[test]
    fn map_object_constructor_installs_the_numeric_run() {
        // 0x00604f40
        let mut object = A57MapObject::a57_new();
        object.head = 0x77;
        object.words = [9; 5];
        object.byte_34 = 9;
        object.name = string("junk");
        object.strings[0] = string("junk");
        object.list.size = 2;
        object.a57_init();

        assert_eq!(object.head, 0);
        assert_eq!(object.name, a57_string_empty());
        assert_eq!(object.words, [0, 0, 1, 8, 0]);
        assert_eq!(object.byte_34, 0);
        assert_eq!(
            object
                .strings
                .iter()
                .filter(|s| s.text.is_empty() && s.cap == A57_SSO_CAPACITY)
                .count(),
            3
        );
        assert_eq!(object.list, A57_LIST_EMPTY);
        assert_eq!(object.a57_string_caps(), [A57_SSO_CAPACITY; 4]);
        assert_eq!(A57_MAP_STRING_OFFSETS, [0x04, 0xc4, 0xe0, 0xfc]);
        assert_eq!(A57_MAP_CAP_OFFSETS, [0x104, 0xe8, 0xcc, 0xc]);
    }

    #[test]
    fn map_object_validation_reports_the_recovered_steps() {
        // 0x00605220
        let mut object = map_object();
        assert_eq!(object.a57_validate(), Ok(()));

        object.strings[2].cap = 0;
        assert_eq!(object.a57_validate(), Err(0));
        object.strings[2].cap = A57_SSO_CAPACITY;

        object.strings[1].cap = 0x10;
        assert_eq!(object.a57_validate(), Err(1));
        object.strings[1].cap = A57_SSO_CAPACITY;

        object.strings[0].cap = 0;
        assert_eq!(object.a57_validate(), Err(2));
        object.strings[0].cap = A57_SSO_CAPACITY;

        object.name.cap = 0;
        assert_eq!(object.a57_validate(), Err(3));
    }

    #[test]
    fn map_object_teardown_resets_the_list_then_validates() {
        // 0x006052f0
        let mut object = map_object();
        assert_eq!(object.a57_teardown(), Ok(true));

        object.list = A57List {
            first_self: false,
            last_self: false,
            size: 4,
        };
        assert_eq!(object.a57_teardown(), Ok(false));
        assert_eq!(object.list, A57_LIST_EMPTY);

        object.strings[2].cap = 0;
        assert_eq!(object.a57_teardown(), Err(1));
        assert_eq!(object.list, A57_LIST_EMPTY, "the reset is local_8 = 0");

        object.strings[2].cap = A57_SSO_CAPACITY;
        object.name.cap = 0;
        assert_eq!(object.a57_teardown(), Err(4));
    }

    #[test]
    fn reserve_capacity_reproduces_the_growth_formula() {
        // 0x006053d0 / FUN_00538810
        assert_eq!(a57_prepare_copy(A57_SSO_CAPACITY, 0), None);
        assert_eq!(a57_prepare_copy(A57_SSO_CAPACITY, 8), None);
        assert_eq!(a57_prepare_copy(A57_SSO_CAPACITY, 0x10), Some(0x8000_0016));
        assert_eq!(a57_prepare_copy(A57_SSO_CAPACITY, 0x100), Some(0x8000_0100));
        assert_eq!(a57_prepare_copy(0x20, 0x40), Some(0x40));
        assert_eq!(a57_prepare_copy(0x8000_0020, 0x30), Some(0x8000_0030));
        assert_eq!(
            a57_prepare_copy(0x7fff_fffe, 0x7fff_ffff),
            Some(0x3fff_fffd)
        );
    }

    #[test]
    fn record_move_copies_id_and_moves_the_string() {
        // 0x00605430, 0x006054e0, 0x006055e0
        for take in [
            a57_take_record_5430,
            a57_take_record_54e0,
            a57_take_record_55e0,
        ] {
            let mut source = A57IdString {
                id: 9,
                text: string("abc"),
            };
            let moved = take(&mut source);
            assert_eq!(moved.id, 9);
            assert_eq!(moved.text.text, "abc");
            assert_eq!(moved.text.cap, 0xf, "the source capacity is masked");
            assert_eq!(source.text, a57_string_empty());
            assert_eq!(source.id, 9);
        }
    }

    #[test]
    fn append_required_size_adds_the_measured_length() {
        // 0x00605660
        assert_eq!(a57_append_required_size(0, ""), 0);
        assert_eq!(a57_append_required_size(5, "abc"), 8);
        assert_eq!(a57_append_required_size(5, ""), 5);
        assert_eq!(a57_append_required_size(u32::MAX - 2, "abc"), 0);
    }

    #[test]
    fn murmur3_matches_the_reference_vectors() {
        // 0x006056e0 hasher (FUN_00c62800)
        assert_eq!(a57_murmur3_32(b"", 0), 0);
        assert_eq!(a57_murmur3_32(b"a", 0), 0x3c25_69b2);
        assert_eq!(a57_murmur3_32(b"abc", 0), 0xb3dd_93fa);
        assert_eq!(a57_murmur3_32(b"Hello, world!", 0), 0xc036_3e43);
        assert_eq!(a57_murmur3_32(b"a", 1), a57_murmur3_32(b"a", 1));
        assert_eq!(a57_bucket_index(a57_murmur3_32(b"1v1", 0), 8), 2);
        assert_eq!(a57_bucket_index(0, 8), 0);
        assert_eq!(a57_bucket_index(0xffff_ffff, 8), 7);
    }

    #[test]
    fn fmix32_is_the_recovered_finaliser() {
        // 0x00605890
        assert_eq!(a57_fmix32(0), 0);
        assert_eq!(a57_fmix32(1), 0x514e_28b7);
        assert_eq!(a57_fmix32(0x1234_5678), 0xe37c_d1bc);
        assert_eq!(a57_fmix32(0xdead_beef), 0x0de5_c6a9);
    }

    #[test]
    fn string_intern_assigns_counter_ids_from_one() {
        // 0x006056e0
        let mut map = A57StringIntern::a57_new();
        assert_eq!(map.a57_next_id(), 1);
        assert_eq!(map.a57_intern(b"1v1"), 1);
        assert_eq!(map.a57_intern(b"2v2"), 2);
        assert_eq!(map.a57_intern(b"1v1"), 1);
        assert_eq!(map.a57_intern(b""), 3);
        assert_eq!(map.a57_len(), 3);
        assert_eq!(map.a57_next_id(), 4);
        assert_eq!(map.a57_lookup(b"2v2"), Some(2));
        assert_eq!(map.a57_lookup(b"missing"), None);
        assert_eq!(map.a57_bucket(b"1v1"), 2);
        assert_eq!(A57_MAP_INITIAL_BUCKETS, 8);
        assert_eq!(A57_MAP_COUNTER_BASE, 1);
    }

    #[test]
    fn pair_intern_assigns_counter_ids_from_one() {
        // 0x00605890
        let mut map = A57PairIntern::a57_new();
        assert_eq!(map.a57_buckets(), 0);
        assert_eq!(map.a57_intern(7, 9), 1);
        assert_eq!(map.a57_buckets(), 8);
        assert_eq!(map.a57_intern(7, 9), 1);
        assert_eq!(map.a57_intern(1, 2), 2);
        assert_eq!(map.a57_next_id(), 3);
        assert_eq!(map.a57_len(), 2);
        assert_eq!(map.a57_lookup(1, 2), Some(2));
        assert_eq!(map.a57_lookup(1, 3), None);
        assert_eq!(map.a57_bucket(0xdead_beef), 1);
    }

    #[test]
    fn map_info_pair_stores_id_flag_and_reservation() {
        // 0x00605a20
        let mut map = A57PairIntern::a57_new();
        let source = A57String {
            text: "x".repeat(0x20),
            cap: 0x8000_0020,
        };
        let info = a57_map_info_pair(&mut map, 4, 5, 0x2a, &source);
        assert_eq!(info.object.head[0], 1);
        assert_eq!(info.object.flags.at_100, 0x2a);
        assert_eq!(info.text_capacity, 0x8000_0020);
        assert_eq!(info.object.strings[0].cap, A57_SSO_CAPACITY);
        assert_eq!(info.object.tail_strings[0].cap, A57_SSO_CAPACITY);
        assert_eq!(info.object.list, A57_LIST_EMPTY);
        assert_eq!(map.a57_len(), 1);
    }

    #[test]
    fn map_info_string_stores_id_flag_and_reservation() {
        // 0x00605b40
        let mut map = A57StringIntern::a57_new();
        let source = string("short");
        let info = a57_map_info_string(&mut map, b"1v1", 7, &source);
        assert_eq!(info.object.head[0], 1);
        assert_eq!(info.object.flags.at_100, 7);
        assert_eq!(info.text_capacity, A57_SSO_CAPACITY);
        assert_eq!(map.a57_lookup(b"1v1"), Some(1));

        let long = A57String {
            text: "y".repeat(0x100),
            cap: A57_SSO_CAPACITY,
        };
        let info = a57_map_info_string(&mut map, b"2v2", 0, &long);
        assert_eq!(info.object.head[0], 2);
        assert_eq!(info.text_capacity, 0x8000_0100);
    }

    #[test]
    fn asset_record_layout_constants_match_the_image() {
        // 0x00604540 / 0x006048b0 offsets
        assert_eq!(A57_ASSET_STRIDE, 0x90);
        assert_eq!(A57_ASSET_STRING_STRIDE, 0x1c);
        assert_eq!(A57_ASSET_READY_OFFSET, 0x8c);
        assert_eq!(A57_MINITILE_STRIDE, 0x20);
        assert_eq!(A57_MEGATILE_STRIDE, 0x34);
        assert_eq!(A57_SLOT_MTX_OFFSET, 0x518);
        assert_eq!(A57_SLOT_ERA_OFFSET, 0x51c);
        assert_eq!(A57_SLOT_READY_OFFSET, 0x51e);
        assert_eq!(A57_CAPACITY_TAG, 0x8000_0000);
        assert_eq!(A57_SSO_CAPACITY, 0x8000_000f);
        assert!(a57_capacity_initialized(A57_SSO_CAPACITY));
        assert!(!a57_capacity_initialized(0x10));
    }
}
