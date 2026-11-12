#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent58.rs
// ===========================================================================
// Translation batch 58.  Only this file is edited by the batch.
//
// Batch 58 owns `0x00606000..=0x00607fff`.  The ledger lists 30 rows in the
// window, all `decompiled_uncited`; 18 are translated below and 12 are
// skipped:
//
// * `FUN_00606eef`, `FUN_0060727e`, `FUN_006074c1`, `FUN_00607512`, `FUN_0060754e`,
//   `FUN_006075c5`, `FUN_006075dd`, `FUN_006075ee`, `FUN_006075fc`, `FUN_00607633`,
//   `FUN_00607752` and `FUN_006079f3` are compiler-generated MSVC exception
//   funclets / split continuations of `0x00606de0`, `0x00607660` and
//   `0x00607a70`.  Every one reads its operands from the parent frame
//   (`unaff_EBP` / `unaff_EDI`), so it has no independent calling contract to
//   reproduce; the parent routines that own those transitions are translated
//   instead.
//
// The window is the save-game path/header object of the 32-bit client.  The
// member offsets recovered from the bodies are:
//
// | offset           | meaning                                                   |
// |------------------|-----------------------------------------------------------|
// | `+0x08`/`+0x0c`  | explicit-name view (`ptr | len` is the "present" test)    |
// | `+0x3c`..`+0x44` | display string (`ptr`, `size`, `cap`)                     |
// | `+0x58`..`+0x60` | save-body string                                          |
// | `+0x74`..`+0x7c` | error string written by `0x00606900`                      |
// | `+0xf0`          | busy flag (1 while a save is in flight)                   |
// | `+0xf4`          | status: 0 ok, 1 running, 2 alternate, otherwise error     |
// | `+0xf8`          | result/tag value                                          |
// | `+0xfc`          | map dword                                                 |
// | `+0x100`         | flag byte (`& 1`, `>>1`, `>>2`, `>>3`, `>>4`, `>>6`)      |
// | `+0x102`/`+0x104`/`+0x106` | three words stored from a result record         |
// | `+0x108`..`+0x10b`| four bytes stored from a result record                   |
// | `+0x10c`         | mode byte                                                 |
// | `+0x114`         | `0xb5`-byte record block copied by `0x00607660`           |
// | `+0x120`         | `0x8d`-byte record block copied by `0x00606de0`           |
// | `+0x16d`         | NUL-terminated map-name buffer                            |
// | `+0x1c9`         | 8-byte tag block                                          |
// | `+0x1d4`         | record dword                                              |
// | `+0x1d8`         | directory string                                          |
// | `+0x1f4`         | file-name string (returned by `0x006068f0`)               |
//
// The class is entirely outside simulation state: the translations below are
// [`World`] helpers over [`A58Save`], a value model of the recovered object.
// Calls whose targets live outside the window (localisation, path
// canonicalisation, the string pool and the file stream) are abstracted as
// value inputs/outcomes, and their contracts are named in the doc comment of
// the routine that consumes them.  Every literal and arithmetic constant that
// the instruction stream does show - the 13-byte tag, the two newline bytes,
// the 8-byte zero tag block, the `0x104` path buffer, the `0x600` open mode,
// the read-only attribute bit and the `0x1f4` file-name offset - is
// reproduced exactly.

/// Inline `std::string` capacity used by every string constructor in the
/// window (`*(+0x8) = 0x8000000f`, i.e. size 0 with the `0xf` inline capacity
/// and the heap flag cleared).
pub const A58_SSO_CAPACITY: u32 = 0xf;

/// Offset of the file-name member returned by `0x006068f0`
/// (`lea eax, [ecx + 0x1f4]; ret`).
pub const A58_FILE_NAME_OFFSET: usize = 0x1f4;

/// Offset of the error string assigned by `0x00606900`
/// (`lea ecx, [ebx + 0x74]`).
pub const A58_ERROR_OFFSET: usize = 0x74;

/// Path buffer size of `0x00606900` (`FUN_006f0090(path, buffer, 0x104)`).
pub const A58_PATH_BUFFER: usize = 0x104;

/// Buffer size handed to the map-name conversion of `0x006064f0`
/// (`FUN_007a6db0` allocates `0x1008` bytes and passes `0x1000`).
pub const A58_MAP_BUFFER: usize = 0x1000;

/// File open mode of `0x00606cb0` (`FUN_00c6d470(name, 0, 0x600)`).
pub const A58_OPEN_MODE: u32 = 0x600;

/// `FILE_ATTRIBUTE_READONLY`, the bit `0x00606cb0` rejects before opening
/// (`uVar10 & 1` after the attribute query).
pub const A58_ATTRIBUTE_READONLY: u32 = 0x1;

/// `INVALID_FILE_ATTRIBUTES`, the attribute result `0x00606cb0` ignores
/// (`uVar10 != 0xffffffff`).
pub const A58_ATTRIBUTE_INVALID: u32 = 0xffff_ffff;

/// The 13-byte save-header tag literal at `0x00d391bc`, copied by
/// `0x006064f0` with two `movq`s, a dword and a byte before being localised.
pub const A58_SAVE_TAG: &[u8; 13] = b"saveMAPNAME_s";

/// The two bytes `0x006064f0` appends after the formatted tag: it reads the
/// word at `0x00d391cc` (`0a 0a`) and appends it with the string growth path.
pub const A58_HEADER_SEPARATOR: [u8; 2] = [0x0a, 0x0a];

/// The zero-filled 8-byte table copied to `+0x1c9` by `0x00607a70`
/// (`FUN_007448d0(object + 0x1c9, 8)` copies from `0x011a30a8`).
pub const A58_TAG_TABLE: [u8; 8] = [0; 8];

/// Offset of the tag table inside the object (`0x00607a70`).
pub const A58_TAG_TABLE_OFFSET: usize = 0x1c9;

/// Error text `0x00606900` stores when the path cannot be made absolute and
/// the "report" flag is set (`0x00d391a8`).
pub const A58_INVALID_SAVE_ERROR: &str = "saveerrorINVALID";

/// First message of `0x00606cb0`: the attribute query succeeded and bit 1 is
/// set (`"Existing file is read only: {0}"`).
pub const A58_READ_ONLY_MESSAGE: &str = "Existing file is read only: {0}";

/// Second message of `0x00606cb0` (`"Failed to open file for writing: {0} |
/// {1} | {2}"`).
pub const A58_OPEN_FAILED_MESSAGE: &str = "Failed to open file for writing: {0} | {1} | {2}";

/// Third message of `0x00606cb0` (`"Failed to write all data (wrote {0} /
/// {1} bytes): {2} | {3} | {4}"`).
pub const A58_SHORT_WRITE_MESSAGE: &str =
    "Failed to write all data (wrote {0} / {1} bytes): {2} | {3} | {4}";

/// Element count of the block `0x00606de0` copies to `+0x120`: `0x23` dwords
/// plus one byte, i.e. `0x8d` bytes.
pub const A58_BLOCK_120_LEN: usize = 0x8d;

/// Element count of the block `0x00607660` copies to `+0x114`: `0x2d` dwords
/// plus one byte, i.e. `0xb5` bytes.
pub const A58_BLOCK_114_LEN: usize = 0xb5;

/// How a write attempt ended, abstracted from the out-of-window file stream
/// of `0x00606cb0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A58WriteOutcome {
    /// The stream opened and the write returned this many bytes.
    Written(u32),
    /// The open call failed (`cVar6 == '\0'`).
    OpenFailed,
}

/// The three non-returning error paths of `0x00606cb0`, carrying only the
/// operands the recovered format strings show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A58SaveError {
    /// Attribute query returned a value other than `0xffffffff` with bit 1
    /// set.
    ReadOnly,
    /// The output stream did not open.
    OpenFailed,
    /// The write returned fewer bytes than the supplied length.
    ShortWrite { wrote: u32, expected: usize },
}

impl A58SaveError {
    /// The literal format string the corresponding `0x00606cb0` path passes
    /// to its message builder.
    pub fn message_template(self) -> &'static str {
        match self {
            A58SaveError::ReadOnly => A58_READ_ONLY_MESSAGE,
            A58SaveError::OpenFailed => A58_OPEN_FAILED_MESSAGE,
            A58SaveError::ShortWrite { .. } => A58_SHORT_WRITE_MESSAGE,
        }
    }
}

/// Result of `0x00606900`: the resolved path plus the `saveerrorINVALID`
/// marker (`true` when no absolute path could be produced).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A58PathResult {
    pub path: A58String,
    pub invalid: bool,
}

/// The 0x18-byte `std::string` model used by the window.  `cap == 0xf` (or
/// less) is the inline buffer, larger capacities are the recovered
/// `len | 0xf` heap class.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A58String {
    pub text: String,
    pub cap: u32,
}

impl A58String {
    /// The empty string written by the constructors of `0x006064f0`,
    /// `0x00606900` and `0x00607a70`: size 0, capacity `0xf`, first byte 0.
    pub fn empty() -> Self {
        Self {
            text: String::new(),
            cap: A58_SSO_CAPACITY,
        }
    }

    /// Builds a string with the capacity class of the recovered
    /// `std::string` growth policy: `len | 0xf` once the inline buffer is
    /// exhausted.
    pub fn from_text(text: &str) -> Self {
        let len = text.len() as u32;
        let cap = if len <= A58_SSO_CAPACITY {
            A58_SSO_CAPACITY
        } else {
            len | A58_SSO_CAPACITY
        };
        Self {
            text: text.to_string(),
            cap,
        }
    }

    /// Size test used by `0x006062a0` (`cmp dword ptr [edi + 0x40], 0`) and by
    /// the `ptr | len` pair test of `0x00606c70`/`0x00606cb0` (`+0x08`/`+0x0c`).
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

/// The save-game object of the window.  Fields are named after the offsets
/// listed in the module doc comment; nothing here is `CUnit`/`World` state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A58Save {
    /// `+0x08`: explicit file name.
    pub name: A58String,
    /// `+0x3c`: display text.
    pub display: A58String,
    /// `+0x58`: save body text.
    pub text: A58String,
    /// `+0x74`: error text.
    pub error: A58String,
    /// `+0xf0`: save in flight.
    pub busy: u8,
    /// `+0xf4`: status.
    pub status: i32,
    /// `+0xf8`: result/tag value.
    pub result: i32,
    /// `+0xfc`: map dword.
    pub map_dword: u32,
    /// `+0x100`: flag byte.
    pub flags: u8,
    /// `+0x102`.
    pub word_102: u16,
    /// `+0x104`.
    pub word_104: u16,
    /// `+0x106`.
    pub word_106: u16,
    /// `+0x108`.
    pub byte_108: u8,
    /// `+0x109`.
    pub byte_109: u8,
    /// `+0x10a`.
    pub byte_10a: u8,
    /// `+0x10b`.
    pub byte_10b: u8,
    /// `+0x10c`.
    pub mode: u8,
    /// `+0x114`: `0xb5`-byte block.
    pub block_114: [u8; A58_BLOCK_114_LEN],
    /// `+0x120`: `0x8d`-byte block.
    pub block_120: [u8; A58_BLOCK_120_LEN],
    /// `+0x16d`: map name, NUL-terminated in retail.
    pub map_name: String,
    /// `+0x1c9`: 8-byte tag block.
    pub tag_table: [u8; 8],
    /// `+0x1d4`: record dword.
    pub dword_1d4: u32,
    /// `+0x1d8`: directory.
    pub dir: A58String,
    /// `+0x1f4`: file name.
    pub file: A58String,
}

impl Default for A58Save {
    fn default() -> Self {
        Self {
            name: A58String::empty(),
            display: A58String::empty(),
            text: A58String::empty(),
            error: A58String::empty(),
            busy: 0,
            status: 0,
            result: 0,
            map_dword: 0,
            flags: 0,
            word_102: 0,
            word_104: 0,
            word_106: 0,
            byte_108: 0,
            byte_109: 0,
            byte_10a: 0,
            byte_10b: 0,
            mode: 0,
            block_114: [0; A58_BLOCK_114_LEN],
            block_120: [0; A58_BLOCK_120_LEN],
            map_name: String::new(),
            tag_table: [0; 8],
            dword_1d4: 0,
            dir: A58String::empty(),
            file: A58String::empty(),
        }
    }
}

/// Result record consumed by `0x00606de0`: the stack locals `local_1d4`
/// (status), `local_1b0`/`local_1b4`/`local_1ac`/`local_1a8` (bytes),
/// `local_17f`/`local_17d`/`local_16f` (words), `local_183`/`local_1a4`
/// (dwords) and the `0x8d`-byte `local_19f` block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A58RecordA {
    pub status: i32,
    pub byte_1b0: u8,
    pub byte_1b4: u8,
    pub byte_1ac: u8,
    pub byte_1a8: u8,
    pub word_17f: u16,
    pub word_17d: u16,
    pub word_16f: u16,
    pub dword_183: u32,
    pub dword_1a4: u32,
    pub block_19f: [u8; A58_BLOCK_120_LEN],
}

/// Result record consumed by `0x00607660`: `local_1c8` (status), the
/// `uStack_1a0`/`uStack_19c` bytes, `uStack_173`/`uStack_171`/`uStack_163`
/// words, `uStack_177` dword and the `0xb5`-byte `auStack_104` block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A58RecordB {
    pub status: i32,
    pub byte_1a0: u8,
    pub byte_19c: u8,
    pub word_173: u16,
    pub word_171: u16,
    pub word_163: u16,
    pub dword_177: u32,
    pub block_104: [u8; A58_BLOCK_114_LEN],
}

/// Result record consumed by `0x00607a70`: `iStack_113` (status), the
/// `uStack_eb`/`uStack_e7`/`uStack_f3`/`uStack_ef` bytes,
/// `uStack_be`/`uStack_bc`/`uStack_ae` words, the `uStack_df` mode byte,
/// `param_2` (tag value) and the two strings `0x006062a0` receives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A58RecordC {
    pub status: i32,
    pub byte_eb: u8,
    pub byte_e7: u8,
    pub byte_f3: u8,
    pub byte_ef: u8,
    pub word_be: u16,
    pub word_bc: u16,
    pub word_ae: u16,
    pub mode_df: u8,
    pub tag_value: u32,
    /// First argument of `0x006062a0` (display string).
    pub display: A58String,
    /// Second argument of `0x006062a0` (body string).
    pub text: A58String,
    /// Path-branch conversion passed to `0x006062a0`.
    pub path_text: A58String,
    /// Name-branch conversion passed to `0x006062a0`.
    pub name_text: A58String,
    /// Result of the out-of-window finaliser applied to `text`.
    pub transformed: A58String,
    /// `0x00606cb0` verdict used by `0x006062a0`.
    pub write_ok: bool,
}

/// Path-branch selection returned by `0x006062a0`:
/// `None` when the initial display string was already non-empty (no branch
/// ran), otherwise `Some(write_ok)` naming the branch that supplied the
/// display text.
pub type A58DisplayBranch = Option<bool>;

/// Inputs of `0x006064f0` that live outside the window.  The routine itself
/// only decides which of the two conversions fills `+0x3c`, measures the
/// `+0x16d` map name, appends the 13-byte tag plus the two newline bytes and
/// stores the finalised body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A58HeaderInputs {
    /// `0x00606cb0` verdict (true selects the path conversion).
    pub write_ok: bool,
    /// `FUN_0074f340(0x00606b70(...))` in the true branch.
    pub path_conversion: A58String,
    /// `FUN_00c6c220(name.ptr, name.len)` in the false branch.
    pub name_conversion: A58String,
    /// The localised 13-byte tag formatted with the map-name conversion
    /// (`FUN_00554080(localised_tag, map_conversion)`).
    pub tag_with_map: A58String,
    /// `FUN_0067dbd0(body)` result stored back into `+0x58`.
    pub finalised: A58String,
}

/// Output of `0x006064f0` besides the mutated object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A58HeaderBuild {
    /// Which conversion supplied the display text.
    pub used_path: bool,
    /// `strlen(object + 0x16d)` computed before the conversions.
    pub map_name_len: usize,
    /// `+0x58` after the tag and the two newline bytes, before finalisation.
    pub rendered: A58String,
}

/// Byte length of the NUL-terminated map-name buffer at `+0x16d`, using the
/// exact loop of `0x006064f0`/`0x00606900` (the terminating byte is the first
/// zero byte, or the end of the modelled buffer).
pub fn a58_c_string_len(text: &str) -> usize {
    text.as_bytes()
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(text.len())
}

/// Path join of `FUN_0074eda0`, the callee of `0x00606b70`'s empty-name branch.
///
/// The directory is copied first; when it is non-empty and its last byte is
/// neither `'\\'` nor `'/'`, one `'\\'` is inserted before the file name:
///
/// ```text
/// if (dir.size == 0)                       no separator
/// else if (dir[dir.size-1] == '\\' || '/') no separator
/// else                                     append '\\'
/// ```
pub fn a58_join_path(dir: &A58String, file: &A58String) -> A58String {
    let mut text = String::with_capacity(dir.text.len() + file.text.len() + 1);
    text.push_str(&dir.text);
    if !dir.text.is_empty() {
        let last = dir.text.as_bytes()[dir.text.len() - 1];
        if last != b'\\' && last != b'/' {
            text.push('\\');
        }
    }
    text.push_str(&file.text);
    A58String::from_text(&text)
}

/// Bit test of the flag byte at `+0x100`, used by the accessors below.
fn a58_flag_bit(flags: u8, bit: u32) -> bool {
    (flags >> bit) & 1 == 1
}

impl World {
    /// `0x006068f0` (`FUN_006068f0`, `decompiled_uncited`).
    ///
    /// ```c
    /// int FUN_006068f0(void) { return in_ECX + 500; }
    /// ```
    ///
    /// The body is exactly `lea eax, [ecx + 0x1f4]; ret`: the address of the
    /// file-name string member.  [`A58_FILE_NAME_OFFSET`] is that 500-byte
    /// displacement.
    pub fn a58_file_name<'a>(&self, state: &'a A58Save) -> &'a A58String {
        &state.file
    }

    /// `0x00606c70` (`FUN_00606c70`, `decompiled_uncited`).
    ///
    /// ```c
    /// uVar1 = *(uint *)(in_ECX + 8) | *(uint *)(in_ECX + 0xc);
    /// if (uVar1 != 0) return CONCAT31((int3)(uVar1 >> 8), 1);
    /// return 0;
    /// ```
    ///
    /// The explicit-name view is the `ptr | len` pair at `+0x08`/`+0x0c`.
    /// The object only ever holds an all-zero pair or a real name, so the
    /// modelled `len != 0` test is the same predicate.
    pub fn a58_has_name(&self, state: &A58Save) -> bool {
        !state.name.is_empty()
    }

    /// `0x00606c80` (`FUN_00606c80`, `decompiled_uncited`).
    ///
    /// ```c
    /// return *(byte *)(in_ECX + 0x100) >> 6 & 1;
    /// ```
    pub fn a58_flag_bit6(&self, state: &A58Save) -> bool {
        a58_flag_bit(state.flags, 6)
    }

    /// `0x00606c90` (`FUN_00606c90`, `decompiled_uncited`).
    ///
    /// ```c
    /// return *(undefined1 *)(in_ECX + 0x10c);
    /// ```
    ///
    /// The mode byte written by `0x00607a70` from `uStack_df`.
    pub fn a58_mode_byte(&self, state: &A58Save) -> u8 {
        state.mode
    }

    /// `0x00606ca0` (`FUN_00606ca0`, `decompiled_uncited`).
    ///
    /// ```c
    /// return *(byte *)(in_ECX + 0x100) >> 3 & 1;
    /// ```
    pub fn a58_flag_bit3(&self, state: &A58Save) -> bool {
        a58_flag_bit(state.flags, 3)
    }

    /// `0x00606d90` (`FUN_00606d90`, `decompiled_uncited`).
    ///
    /// ```c
    /// return *(byte *)(in_ECX + 0x100) >> 1 & 1;
    /// ```
    pub fn a58_flag_bit1(&self, state: &A58Save) -> bool {
        a58_flag_bit(state.flags, 1)
    }

    /// `0x00606da0` (`FUN_00606da0`, `decompiled_uncited`).
    ///
    /// ```c
    /// return *(byte *)(in_ECX + 0x100) >> 4 & 1;
    /// ```
    pub fn a58_flag_bit4(&self, state: &A58Save) -> bool {
        a58_flag_bit(state.flags, 4)
    }

    /// `0x00606db0` (`FUN_00606db0`, `decompiled_uncited`).
    ///
    /// ```c
    /// return (*(byte *)(in_ECX + 0x100) & 0x12) == 0x12;
    /// ```
    ///
    /// True only when both bit 1 and bit 4 are set.
    pub fn a58_flags_1_and_4(&self, state: &A58Save) -> bool {
        state.flags & 0x12 == 0x12
    }

    /// `0x00606dc0` (`FUN_00606dc0`, `decompiled_uncited`).
    ///
    /// ```c
    /// return *(byte *)(in_ECX + 0x100) >> 2 & 1;
    /// ```
    pub fn a58_flag_bit2(&self, state: &A58Save) -> bool {
        a58_flag_bit(state.flags, 2)
    }

    /// `0x00606dd0` (`FUN_00606dd0`, `decompiled_uncited`).
    ///
    /// ```c
    /// return *(byte *)(in_ECX + 0x100) & 1;
    /// ```
    pub fn a58_flag_bit0(&self, state: &A58Save) -> bool {
        a58_flag_bit(state.flags, 0)
    }

    /// `0x00606b70` (`FUN_00606b70`, `decompiled_uncited`).
    ///
    /// Builds the save path into the caller's string:
    ///
    /// ```text
    /// if (name.ptr | name.len) return copy(name)
    /// else                     return join(directory, file_name)
    /// ```
    ///
    /// The explicit-name branch copies the `+0x08` string through the string
    /// pool; the other branch delegates to the path join of `FUN_0074eda0`
    /// ([`a58_join_path`]).  Both branches return the same caller buffer.
    pub fn a58_save_path(&self, state: &A58Save) -> A58String {
        if !state.name.is_empty() {
            return state.name.clone();
        }
        a58_join_path(&state.dir, &state.file)
    }

    /// `0x00606900` (`FUN_00606900`, `decompiled_uncited`).
    ///
    /// Resolves a usable save path:
    ///
    /// ```text
    /// if (name is present)          return path
    /// if (path is already absolute) return path
    /// absolute = make_absolute(path, buffer, 0x104)
    /// if (absolute == 0) {
    ///     if (report) error(+0x74) = localise("saveerrorINVALID")
    ///     return empty
    /// }
    /// return string(buffer)         // strlen(buffer)
    /// ```
    ///
    /// The absolute-path predicate is `FUN_0074fb80` (the root parser of
    /// `FUN_0093f2c0`: drive letter or UNC root) and the canonicaliser is
    /// `FUN_006f0090`, both outside the window; the caller supplies their
    /// verdicts.  On failure `+0x74` receives the exact error literal and the
    /// result is always the empty string, whether or not the error is
    /// reported.
    pub fn a58_resolved_save_path(
        &self,
        state: &mut A58Save,
        path_is_absolute: bool,
        absolute_path: Option<&A58String>,
        report_invalid: bool,
    ) -> A58PathResult {
        let path = self.a58_save_path(state);
        if !state.name.is_empty() || path_is_absolute {
            return A58PathResult {
                path,
                invalid: false,
            };
        }
        match absolute_path {
            Some(absolute) => A58PathResult {
                path: absolute.clone(),
                invalid: false,
            },
            None => {
                if report_invalid {
                    state.error = A58String::from_text(A58_INVALID_SAVE_ERROR);
                }
                A58PathResult {
                    path: A58String::empty(),
                    invalid: true,
                }
            }
        }
    }

    /// `0x00606cb0` (`FUN_00606cb0`, `decompiled_uncited`).
    ///
    /// Writes the save body when an explicit name is present:
    ///
    /// ```text
    /// if ((name.ptr | name.len) == 0) return 1
    /// attrs = get_file_attributes(name)
    /// if (attrs != 0xffffffff && (attrs & 1) != 0) error("read only")
    /// if (!open(name, 0, 0x600))                   error("open failed")
    /// wrote = write(data, data_len)
    /// if (wrote != data_len)                       error("short write")
    /// close()
    /// return lookup(name)     // FUN_00839c40: exists-after-write verdict
    /// ```
    ///
    /// The open/write/lookup calls live outside the window, so their outcome
    /// is supplied by the caller.  The three error strings, the `0x600` open
    /// mode, the `0xffffffff` attribute sentinel and the read-only bit are
    /// reproduced exactly.
    pub fn a58_commit_save_data(
        &self,
        state: &A58Save,
        data: &[u8],
        attributes: Option<u32>,
        outcome: A58WriteOutcome,
        exists_after_write: bool,
    ) -> Result<bool, A58SaveError> {
        if state.name.is_empty() {
            return Ok(true);
        }
        if let Some(attributes) = attributes {
            if attributes != A58_ATTRIBUTE_INVALID && attributes & A58_ATTRIBUTE_READONLY != 0 {
                return Err(A58SaveError::ReadOnly);
            }
        }
        match outcome {
            A58WriteOutcome::OpenFailed => Err(A58SaveError::OpenFailed),
            A58WriteOutcome::Written(wrote) => {
                if wrote as usize != data.len() {
                    Err(A58SaveError::ShortWrite {
                        wrote,
                        expected: data.len(),
                    })
                } else {
                    Ok(exists_after_write)
                }
            }
        }
    }

    /// `0x006062a0` (`FUN_006062a0`, `decompiled_uncited`).
    ///
    /// Fills the display and body strings:
    ///
    /// ```text
    /// display = *param_1
    /// if (display.size == 0) {
    ///     if (write_check()) display = convert_path(save_path())
    ///     else               display = convert_name(name.ptr, name.len)
    /// }
    /// text = *param_2
    /// text = finalise(text)                  // FUN_0067dbd0
    /// ```
    ///
    /// The two conversions and the finaliser are outside the window and are
    /// supplied as values.  Returns the branch that supplied the display
    /// (`None` when the incoming display string was already non-empty).
    pub fn a58_assign_save_texts(
        &self,
        state: &mut A58Save,
        display: &A58String,
        text: &A58String,
        write_ok: bool,
        path_text: &A58String,
        name_text: &A58String,
        transformed: &A58String,
    ) -> A58DisplayBranch {
        state.display = display.clone();
        let branch = if state.display.is_empty() {
            let source = if write_ok { path_text } else { name_text };
            state.display = source.clone();
            Some(write_ok)
        } else {
            None
        };
        state.text = text.clone();
        state.text = transformed.clone();
        branch
    }

    /// `0x006064f0` (`FUN_006064f0`, `decompiled_uncited`).
    ///
    /// Builds the save header:
    ///
    /// ```text
    /// map_len = strlen(object + 0x16d)
    /// conversion = convert(map_name)                    // FUN_007a6db0, 0x1000
    /// if (write_check()) display = convert_path(save_path())
    /// else               display = convert_name(name.ptr, name.len)
    /// body = format(localise("saveMAPNAME_s"), map_conversion)  // FUN_00554080
    /// body += "\n\n"                                    // 0x00d391cc
    /// serialize_body(object + 0x1ad)                    // FUN_00605660
    /// body = finalise(body)                             // FUN_0067dbd0
    /// ```
    ///
    /// The 13-byte tag, the two `0x0a` bytes, the display branch and the
    /// `strlen` loop are exact; the conversion, localisation and finaliser
    /// results are supplied by the caller.
    pub fn a58_build_save_header(
        &self,
        state: &mut A58Save,
        inputs: &A58HeaderInputs,
    ) -> A58HeaderBuild {
        let map_name_len = a58_c_string_len(&state.map_name);
        let display = if inputs.write_ok {
            &inputs.path_conversion
        } else {
            &inputs.name_conversion
        };
        state.display = display.clone();
        let mut rendered = A58String::empty();
        rendered.text.push_str(&inputs.tag_with_map.text);
        rendered
            .text
            .push_str(std::str::from_utf8(&A58_HEADER_SEPARATOR).unwrap());
        state.text = rendered.clone();
        state.text = inputs.finalised.clone();
        A58HeaderBuild {
            used_path: inputs.write_ok,
            map_name_len,
            rendered,
        }
    }

    /// `0x00606de0` (`FUN_00606de0`, `decompiled_uncited`).
    ///
    /// Applies an [`A58RecordA`] result record:
    ///
    /// ```text
    /// if (busy != 0) return
    /// busy = 1; status = 1
    /// if (record.status == 0) {
    ///     +0x10b = b1b0; +0x10a = b1b4; +0x108 = b1ac; +0x109 = b1a8
    ///     +0x102 = w17f; +0x104 = w17d; +0x106 = w16f
    ///     +0xfc  = d183; +0x1d4 = d1a4
    ///     copy 0x8d bytes (0x23 dwords + 1) from local_19f to +0x120
    /// }
    /// status = record.status
    /// ```
    ///
    /// Returns `false` when the busy gate rejected the call (the object is
    /// left untouched), `true` otherwise.
    pub fn a58_begin_save_from_record(&self, state: &mut A58Save, record: &A58RecordA) -> bool {
        if state.busy != 0 {
            return false;
        }
        state.busy = 1;
        state.status = 1;
        if record.status == 0 {
            state.byte_10b = record.byte_1b0;
            state.byte_10a = record.byte_1b4;
            state.byte_108 = record.byte_1ac;
            state.byte_109 = record.byte_1a8;
            state.word_102 = record.word_17f;
            state.word_104 = record.word_17d;
            state.word_106 = record.word_16f;
            state.map_dword = record.dword_183;
            state.dword_1d4 = record.dword_1a4;
            state.block_120.copy_from_slice(&record.block_19f);
        }
        state.status = record.status;
        true
    }

    /// `0x00607660` (`FUN_00607660`, `decompiled_uncited`).
    ///
    /// The alternate save entry, same shape as `0x00606de0` with the second
    /// record layout:
    ///
    /// ```text
    /// if (busy != 0) return
    /// busy = 1; status = 1
    /// if (record.status == 0) {
    ///     +0x108 = b1a0; +0x109 = b19c
    ///     +0x102 = w173; +0x104 = w171; +0x106 = w163
    ///     +0xfc  = d177
    ///     copy 0xb5 bytes (0x2d dwords + 1) from auStack_104 to +0x114
    /// }
    /// status = record.status
    /// ```
    pub fn a58_begin_save_alt(&self, state: &mut A58Save, record: &A58RecordB) -> bool {
        if state.busy != 0 {
            return false;
        }
        state.busy = 1;
        state.status = 1;
        if record.status == 0 {
            state.byte_108 = record.byte_1a0;
            state.byte_109 = record.byte_19c;
            state.word_102 = record.word_173;
            state.word_104 = record.word_171;
            state.word_106 = record.word_163;
            state.map_dword = record.dword_177;
            state.block_114.copy_from_slice(&record.block_104);
        }
        state.status = record.status;
        true
    }

    /// `0x00607a70` (`FUN_00607a70`, `decompiled_uncited`).
    ///
    /// The option-carrying save entry.  On success it stores the record
    /// bytes, copies the 8-byte zero tag table to `+0x1c9`, runs the string
    /// assignment of `0x006062a0`, stores the tag value at `+0xf8` and clears
    /// the status:
    ///
    /// ```text
    /// if (busy != 0) return
    /// busy = 1; status = 1
    /// if (record.status == 0) {
    ///     +0x108 = beb; +0x109 = be7; +0x10a = bf3; +0x10b = bef
    ///     +0x102 = wbe; +0x104 = wbc; +0x106 = wae
    ///     +0x10c = bdf
    ///     assign_texts(display, text)              // 0x006062a0
    ///     copy 8 zero bytes to +0x1c9               // FUN_007448d0
    ///     +0xf8 = tag_value
    ///     status = 0
    /// } else {
    ///     status = record.status
    /// }
    /// ```
    pub fn a58_begin_save_with_tag(&self, state: &mut A58Save, record: &A58RecordC) -> bool {
        if state.busy != 0 {
            return false;
        }
        state.busy = 1;
        state.status = 1;
        if record.status == 0 {
            state.byte_108 = record.byte_eb;
            state.byte_109 = record.byte_e7;
            state.byte_10a = record.byte_f3;
            state.byte_10b = record.byte_ef;
            state.word_102 = record.word_be;
            state.word_104 = record.word_bc;
            state.word_106 = record.word_ae;
            state.mode = record.mode_df;
            let _ = self.a58_assign_save_texts(
                state,
                &record.display,
                &record.text,
                record.write_ok,
                &record.path_text,
                &record.name_text,
                &record.transformed,
            );
            state.tag_table = A58_TAG_TABLE;
            state.result = record.tag_value as i32;
            state.status = 0;
        } else {
            state.status = record.status;
        }
        true
    }
}

#[cfg(test)]
mod tests_agent58 {
    use super::*;
    use crate::data::GameData;
    use crate::map::Terrain;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn s(text: &str) -> A58String {
        A58String::from_text(text)
    }

    fn record_a(status: i32) -> A58RecordA {
        A58RecordA {
            status,
            byte_1b0: 0x10,
            byte_1b4: 0x11,
            byte_1ac: 0x12,
            byte_1a8: 0x13,
            word_17f: 0x1414,
            word_17d: 0x1515,
            word_16f: 0x1616,
            dword_183: 0x1717_1717,
            dword_1a4: 0x1818_1818,
            block_19f: [0x19; A58_BLOCK_120_LEN],
        }
    }

    fn record_b(status: i32) -> A58RecordB {
        A58RecordB {
            status,
            byte_1a0: 0x20,
            byte_19c: 0x21,
            word_173: 0x2222,
            word_171: 0x2323,
            word_163: 0x2424,
            dword_177: 0x2525_2525,
            block_104: [0x26; A58_BLOCK_114_LEN],
        }
    }

    fn record_c(status: i32) -> A58RecordC {
        A58RecordC {
            status,
            byte_eb: 0x30,
            byte_e7: 0x31,
            byte_f3: 0x32,
            byte_ef: 0x33,
            word_be: 0x3434,
            word_bc: 0x3535,
            word_ae: 0x3636,
            mode_df: 0x37,
            tag_value: 0xdead_beef,
            display: s(""),
            text: s("raw"),
            path_text: s("C:\\Saves\\slot.sav"),
            name_text: s("slot.sav"),
            transformed: s("final"),
            write_ok: true,
        }
    }

    #[test]
    fn file_name_offset_is_the_500_byte_displacement() {
        // 0x006068f0
        let w = world();
        let mut state = A58Save::default();
        state.file = s("slot.sav");
        assert_eq!(A58_FILE_NAME_OFFSET, 500);
        assert_eq!(w.a58_file_name(&state).text, "slot.sav");
    }

    #[test]
    fn has_name_tests_the_explicit_name_pair() {
        // 0x00606c70
        let w = world();
        let mut state = A58Save::default();
        assert!(!w.a58_has_name(&state));
        state.name = s("slot.sav");
        assert!(w.a58_has_name(&state));
    }

    #[test]
    fn flag_bits_match_the_recovered_shifts() {
        // 0x00606c80, 0x00606ca0, 0x00606d90, 0x00606da0, 0x00606dc0, 0x00606dd0
        let w = world();
        let mut state = A58Save::default();
        state.flags = 0b0001_1111;
        assert!(w.a58_flag_bit0(&state));
        assert!(w.a58_flag_bit1(&state));
        assert!(w.a58_flag_bit2(&state));
        assert!(w.a58_flag_bit3(&state));
        assert!(w.a58_flag_bit4(&state));
        assert!(!w.a58_flag_bit6(&state));
        state.flags = 0b0100_0000;
        assert!(w.a58_flag_bit6(&state));
        assert!(!w.a58_flag_bit0(&state));
        state.flags = 0;
        assert!(!w.a58_flag_bit4(&state));
    }

    #[test]
    fn both_low_flag_bits_are_required() {
        // 0x00606db0
        let w = world();
        let mut state = A58Save::default();
        for (flags, expected) in [
            (0x00u8, false),
            (0x02, false),
            (0x10, false),
            (0x12, true),
            (0x13, true),
            (0xff, true),
        ] {
            state.flags = flags;
            assert_eq!(w.a58_flags_1_and_4(&state), expected, "flags {flags:#x}");
        }
    }

    #[test]
    fn mode_byte_is_the_10c_byte() {
        // 0x00606c90
        let w = world();
        let mut state = A58Save::default();
        state.mode = 0xa5;
        assert_eq!(w.a58_mode_byte(&state), 0xa5);
    }

    #[test]
    fn save_path_prefers_the_explicit_name() {
        // 0x00606b70
        let w = world();
        let mut state = A58Save::default();
        state.name = s("explicit.sav");
        state.dir = s("C:\\Saves");
        state.file = s("other.sav");
        assert_eq!(w.a58_save_path(&state).text, "explicit.sav");
    }

    #[test]
    fn save_path_joins_directory_and_file() {
        // 0x00606b70 -> FUN_0074eda0
        let w = world();
        let mut state = A58Save::default();
        state.file = s("slot.sav");
        state.dir = s("C:\\Saves");
        assert_eq!(w.a58_save_path(&state).text, "C:\\Saves\\slot.sav");
        state.dir = s("C:\\Saves\\");
        assert_eq!(w.a58_save_path(&state).text, "C:\\Saves\\slot.sav");
        state.dir = s("C:/Saves/");
        assert_eq!(w.a58_save_path(&state).text, "C:/Saves/slot.sav");
        state.dir = s("");
        assert_eq!(w.a58_save_path(&state).text, "slot.sav");
        assert_eq!(a58_join_path(&s("a"), &s("b")).text, "a\\b");
        assert_eq!(a58_join_path(&s(""), &s("b")).text, "b");
        assert_eq!(a58_join_path(&s("a/"), &s("b")).text, "a/b");
    }

    #[test]
    fn resolved_path_short_circuits_for_names_and_absolute_paths() {
        // 0x00606900
        let w = world();
        let mut state = A58Save::default();
        state.name = s("slot.sav");
        let result = w.a58_resolved_save_path(&mut state, false, None, true);
        assert_eq!(result.path.text, "slot.sav");
        assert!(!result.invalid);
        assert!(state.error.is_empty());

        let mut state = A58Save::default();
        state.dir = s("C:\\Saves");
        state.file = s("slot.sav");
        let result = w.a58_resolved_save_path(&mut state, true, None, true);
        assert_eq!(result.path.text, "C:\\Saves\\slot.sav");
        assert!(!result.invalid);
    }

    #[test]
    fn resolved_path_uses_the_canonicalised_buffer() {
        // 0x00606900
        let w = world();
        let mut state = A58Save::default();
        state.dir = s("Saves");
        state.file = s("slot.sav");
        let absolute = s("C:\\Games\\Saves\\slot.sav");
        let result = w.a58_resolved_save_path(&mut state, false, Some(&absolute), true);
        assert_eq!(result.path, absolute);
        assert!(!result.invalid);
    }

    #[test]
    fn resolved_path_reports_the_invalid_literal() {
        // 0x00606900
        let w = world();
        let mut state = A58Save::default();
        state.dir = s("Saves");
        state.file = s("slot.sav");
        let result = w.a58_resolved_save_path(&mut state, false, None, true);
        assert_eq!(result.path, A58String::empty());
        assert!(result.invalid);
        assert_eq!(state.error.text, A58_INVALID_SAVE_ERROR);

        let mut quiet = A58Save::default();
        let result = w.a58_resolved_save_path(&mut quiet, false, None, false);
        assert!(result.invalid);
        assert!(quiet.error.is_empty());
        assert_eq!(A58_ERROR_OFFSET, 0x74);
        assert_eq!(A58_PATH_BUFFER, 0x104);
    }

    #[test]
    fn commit_skips_when_there_is_no_explicit_name() {
        // 0x00606cb0
        let w = world();
        let state = A58Save::default();
        assert_eq!(
            w.a58_commit_save_data(
                &state,
                b"body",
                Some(A58_ATTRIBUTE_READONLY),
                A58WriteOutcome::OpenFailed,
                false,
            ),
            Ok(true)
        );
    }

    #[test]
    fn commit_rejects_read_only_targets() {
        // 0x00606cb0
        let w = world();
        let mut state = A58Save::default();
        state.name = s("slot.sav");
        let error = w
            .a58_commit_save_data(
                &state,
                b"body",
                Some(A58_ATTRIBUTE_READONLY),
                A58WriteOutcome::Written(4),
                true,
            )
            .unwrap_err();
        assert_eq!(error, A58SaveError::ReadOnly);
        assert_eq!(error.message_template(), A58_READ_ONLY_MESSAGE);
        assert_eq!(A58_ATTRIBUTE_INVALID, 0xffff_ffff);
        assert_eq!(A58_OPEN_MODE, 0x600);
    }

    #[test]
    fn commit_reports_open_and_short_write_failures() {
        // 0x00606cb0
        let w = world();
        let mut state = A58Save::default();
        state.name = s("slot.sav");
        let error = w
            .a58_commit_save_data(
                &state,
                b"body",
                Some(A58_ATTRIBUTE_INVALID),
                A58WriteOutcome::OpenFailed,
                false,
            )
            .unwrap_err();
        assert_eq!(error, A58SaveError::OpenFailed);
        assert_eq!(error.message_template(), A58_OPEN_FAILED_MESSAGE);

        let error = w
            .a58_commit_save_data(&state, b"body", None, A58WriteOutcome::Written(2), false)
            .unwrap_err();
        assert_eq!(
            error,
            A58SaveError::ShortWrite {
                wrote: 2,
                expected: 4
            }
        );
        assert_eq!(error.message_template(), A58_SHORT_WRITE_MESSAGE);
    }

    #[test]
    fn commit_returns_the_post_write_lookup() {
        // 0x00606cb0
        let w = world();
        let mut state = A58Save::default();
        state.name = s("slot.sav");
        assert_eq!(
            w.a58_commit_save_data(
                &state,
                b"body",
                Some(0x20),
                A58WriteOutcome::Written(4),
                true,
            ),
            Ok(true)
        );
        assert_eq!(
            w.a58_commit_save_data(
                &state,
                b"body",
                Some(0x20),
                A58WriteOutcome::Written(4),
                false,
            ),
            Ok(false)
        );
    }

    #[test]
    fn assign_texts_selects_the_display_branch() {
        // 0x006062a0
        let w = world();
        let mut state = A58Save::default();
        let branch = w.a58_assign_save_texts(
            &mut state,
            &s(""),
            &s("body"),
            true,
            &s("C:\\Saves\\slot.sav"),
            &s("slot.sav"),
            &s("final"),
        );
        assert_eq!(branch, Some(true));
        assert_eq!(state.display.text, "C:\\Saves\\slot.sav");
        assert_eq!(state.text.text, "final");

        let mut state = A58Save::default();
        let branch = w.a58_assign_save_texts(
            &mut state,
            &s(""),
            &s("body"),
            false,
            &s("path"),
            &s("slot.sav"),
            &s("final"),
        );
        assert_eq!(branch, Some(false));
        assert_eq!(state.display.text, "slot.sav");
    }

    #[test]
    fn assign_texts_keeps_a_non_empty_display() {
        // 0x006062a0
        let w = world();
        let mut state = A58Save::default();
        let branch = w.a58_assign_save_texts(
            &mut state,
            &s("existing"),
            &s("body"),
            true,
            &s("path"),
            &s("name"),
            &s("final"),
        );
        assert_eq!(branch, None);
        assert_eq!(state.display.text, "existing");
        assert_eq!(state.text.text, "final");
    }

    #[test]
    fn build_header_appends_the_tag_and_the_two_newlines() {
        // 0x006064f0
        let w = world();
        let mut state = A58Save::default();
        state.map_name = "Map\0trailing".to_string();
        let inputs = A58HeaderInputs {
            write_ok: true,
            path_conversion: s("C:\\Saves\\slot.sav"),
            name_conversion: s("slot.sav"),
            tag_with_map: s("Saving Map"),
            finalised: s("Saving Map\n\nbody"),
        };
        let build = w.a58_build_save_header(&mut state, &inputs);
        assert_eq!(build.map_name_len, 3, "the NUL ends the map name");
        assert!(build.used_path);
        assert_eq!(build.rendered.text, "Saving Map\n\n");
        assert_eq!(state.display.text, "C:\\Saves\\slot.sav");
        assert_eq!(state.text.text, "Saving Map\n\nbody");
        assert_eq!(A58_SAVE_TAG, b"saveMAPNAME_s");
        assert_eq!(A58_HEADER_SEPARATOR, [0x0a, 0x0a]);
        assert_eq!(a58_c_string_len("abc"), 3);
    }

    #[test]
    fn build_header_uses_the_name_conversion_without_write_access() {
        // 0x006064f0
        let w = world();
        let mut state = A58Save::default();
        let inputs = A58HeaderInputs {
            write_ok: false,
            path_conversion: s("path"),
            name_conversion: s("name"),
            tag_with_map: s("T"),
            finalised: s("T\n\nF"),
        };
        let build = w.a58_build_save_header(&mut state, &inputs);
        assert!(!build.used_path);
        assert_eq!(state.display.text, "name");
        assert_eq!(build.rendered.text, "T\n\n");
        assert_eq!(A58_MAP_BUFFER, 0x1000);
    }

    #[test]
    fn begin_save_from_record_applies_the_first_layout() {
        // 0x00606de0
        let w = world();
        let mut state = A58Save::default();
        assert!(w.a58_begin_save_from_record(&mut state, &record_a(0)));
        assert_eq!(state.busy, 1);
        assert_eq!(state.status, 0);
        assert_eq!(state.byte_10b, 0x10);
        assert_eq!(state.byte_10a, 0x11);
        assert_eq!(state.byte_108, 0x12);
        assert_eq!(state.byte_109, 0x13);
        assert_eq!(state.word_102, 0x1414);
        assert_eq!(state.word_104, 0x1515);
        assert_eq!(state.word_106, 0x1616);
        assert_eq!(state.map_dword, 0x1717_1717);
        assert_eq!(state.dword_1d4, 0x1818_1818);
        assert_eq!(state.block_120, [0x19; A58_BLOCK_120_LEN]);
    }

    #[test]
    fn begin_save_rejects_a_second_call_while_busy() {
        // 0x00606de0, 0x00607660, 0x00607a70
        let w = world();
        let mut state = A58Save::default();
        state.busy = 1;
        assert!(!w.a58_begin_save_from_record(&mut state, &record_a(0)));
        assert_eq!(state.status, 0, "the gate leaves the object untouched");
        state.busy = 0;
        assert!(w.a58_begin_save_alt(&mut state, &record_b(0)));
        assert!(!w.a58_begin_save_alt(&mut state, &record_b(0)));
        state.busy = 0;
        assert!(w.a58_begin_save_with_tag(&mut state, &record_c(0)));
        assert!(!w.a58_begin_save_with_tag(&mut state, &record_c(0)));
    }

    #[test]
    fn begin_save_stores_error_statuses_without_copying() {
        // 0x00606de0, 0x00607660
        let w = world();
        let mut state = A58Save::default();
        assert!(w.a58_begin_save_from_record(&mut state, &record_a(-7)));
        assert_eq!(state.status, -7);
        assert_eq!(state.flags, 0, "record fields stay untouched");
        assert_eq!(state.byte_108, 0);
        assert_eq!(state.map_dword, 0);

        let mut state = A58Save::default();
        assert!(w.a58_begin_save_alt(&mut state, &record_b(5)));
        assert_eq!(state.status, 5);
        assert_eq!(state.map_dword, 0);
        assert_eq!(state.block_114, [0; A58_BLOCK_114_LEN]);
    }

    #[test]
    fn begin_save_alt_applies_the_second_layout() {
        // 0x00607660
        let w = world();
        let mut state = A58Save::default();
        assert!(w.a58_begin_save_alt(&mut state, &record_b(0)));
        assert_eq!(state.byte_108, 0x20);
        assert_eq!(state.byte_109, 0x21);
        assert_eq!(state.word_102, 0x2222);
        assert_eq!(state.word_104, 0x2323);
        assert_eq!(state.word_106, 0x2424);
        assert_eq!(state.map_dword, 0x2525_2525);
        assert_eq!(state.block_114, [0x26; A58_BLOCK_114_LEN]);
        assert_eq!(state.block_120, [0; A58_BLOCK_120_LEN]);
    }

    #[test]
    fn begin_save_with_tag_applies_the_third_layout_and_assigns_texts() {
        // 0x00607a70
        let w = world();
        let mut state = A58Save::default();
        assert!(w.a58_begin_save_with_tag(&mut state, &record_c(0)));
        assert_eq!(state.status, 0);
        assert_eq!(state.byte_108, 0x30);
        assert_eq!(state.byte_109, 0x31);
        assert_eq!(state.byte_10a, 0x32);
        assert_eq!(state.byte_10b, 0x33);
        assert_eq!(state.word_102, 0x3434);
        assert_eq!(state.word_104, 0x3535);
        assert_eq!(state.word_106, 0x3636);
        assert_eq!(state.mode, 0x37);
        assert_eq!(state.display.text, "C:\\Saves\\slot.sav");
        assert_eq!(state.text.text, "final");
        assert_eq!(state.tag_table, A58_TAG_TABLE);
        assert_eq!(state.result, 0xdead_beefu32 as i32);
        assert_eq!(A58_TAG_TABLE_OFFSET, 0x1c9);
        assert_eq!(A58_BLOCK_114_LEN, 0xb5);
        assert_eq!(A58_BLOCK_120_LEN, 0x8d);
    }

    #[test]
    fn begin_save_with_tag_keeps_the_error_status() {
        // 0x00607a70
        let w = world();
        let mut state = A58Save::default();
        assert!(w.a58_begin_save_with_tag(&mut state, &record_c(9)));
        assert_eq!(state.status, 9);
        assert_eq!(state.mode, 0);
        assert_eq!(state.tag_table, [0; 8]);
        assert_eq!(state.result, 0);
        assert!(state.display.is_empty());
        assert!(state.text.is_empty());
    }
}
