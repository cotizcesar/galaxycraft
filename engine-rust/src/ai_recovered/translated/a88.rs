#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent88.rs
// ===========================================================================
// Translation batch 88.  Only this file is edited by the batch.
//
// Batch 88 owns `0x00642000..=0x00643fff`.  The ledger lists 34
// `decompiled_uncited` rows in that window; 20 are translated below and the
// 14 rows that are not are listed at the end with their reason.
//
// Most of the window is the retail AI-script runtime (`FUN_00640080`'s
// opcode interpreter) plus the spawn/region list it drives.  Ghidra split
// that one interpreter into dozens of "functions" whose bodies are the same
// dispatch loop entered at a case label; those rows address their state
// through the caller's `ESI`/`EDI`/`EBP` and carry no self-contained
// operation.  Where a row *does* contain a closed operation (a cursor
// fetch, a string build, a list link, a rect copy) it is translated here as
// a value type plus free functions.  No `sim` record gains a field.
//
// Recovered state:
//
// | owner                 | words                                                                    |
// |-----------------------|--------------------------------------------------------------------------|
// | script state          | `+0x04` byte cursor, `+0x08` unit, `+0x16` location, `+0x18` bank flag   |
// | spawn pool            | `0x1450` free head, `0x00fc9b68` list head, node `{next, prev, ...}`     |
// | spawn node            | `+0x08` location, `+0x10` unit type, `+0x14..0x20` rect, `+0x24/28` center, `+0x30` flags |
// | region node           | `+0x00/04` links, `+0x08/0c` keys, `+0x10` destructor tag, `+0x34..40` rect, `+0x44` valid, `+0x4d/4f/51` bytes |
// | region list           | `+0x00` head, `+0x04` tail, `+0x08` count                                 |
//
// Address to function:
//
// * `0x00642037` [`a88_script_read_byte`]
// * `0x00642484` [`a88_script_store_cursor`]
// * `0x00642687` [`a88_add_al_twice`]
// * `0x00642696` [`a88_invalid_ai_script_words`]
// * `0x006427c6` [`a88_tsc_carry_add`]
// * `0x00642a00` [`a88_spawn_request`]
// * `0x00642a80` [`A88SpawnPool::a88_alloc`]
// * `0x00642b50` [`a88_spawn_at_location`]
// * `0x00642bf0` [`a88_spawn_gate`]
// * `0x00642e9b` [`a88_sso_string_init`]
// * `0x006433d7` [`a88_rotate_right_through_carry`]
// * `0x00643860` [`A88RegionList::a88_reset`]
// * `0x00643910` [`A88RegionList::a88_clear_head`]
// * `0x006439eb` [`A88RegionList::a88_unlink_tail`]
// * `0x00643aa0` [`a88_region_list_present`]
// * `0x00643b10` [`a88_region_snapshot`]
// * `0x00643b30` [`a88_region_scratch`]
// * `0x00643ca4` [`a88_sso_string_init_catch_frame`]
// * `0x00643f40` [`a88_region_marshal_point`]
// * `0x00643fe0` [`a88_region_rect_store`]

// ---------------------------------------------------------------------------
// 0x00642037 - AI-script byte fetch
// ---------------------------------------------------------------------------

/// One of the two script banks selected by the state's `+0x18` flag.
///
/// Retail keeps two pointer pairs: `DAT_00fc8700`/`DAT_00fc8704` for flag
/// bit 0 clear and `DAT_00fc8708`/`DAT_00fc870c` when it is set.  The first
/// word of each pair is the bank base, the second one past its last byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A88ScriptBank {
    /// First script byte (`DAT_00fc8700` / `DAT_00fc8708`).
    pub base: u32,
    /// One past the last script byte (`DAT_00fc8704` / `DAT_00fc870c`).
    pub end: u32,
}

/// Address of the primary pointer pair (`base`, `end`).
pub const A88_SCRIPT_PAIR_PRIMARY_ADDR: u32 = 0x00fc8700;

/// Address of the alternate pointer pair (`base`, `end`).
pub const A88_SCRIPT_PAIR_ALTERNATE_ADDR: u32 = 0x00fc8708;

/// Why the fetch failed.  Retail calls `0x00642696`, which raises
/// `"Invalid AI script"` (see [`a88_invalid_ai_script_words`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A88ScriptFault {
    /// `0x00642696`: `end <= cursor` or `end <= cursor + 1`.
    InvalidScript,
}

/// Result of one successful fetch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A88ScriptRead {
    /// Byte at `base + cursor`.
    pub byte: u8,
    /// The stored cursor, `cursor + 1`.
    pub cursor: u32,
}

/// `0x00642037`: fetch the next script byte.
///
/// ```c
/// puVar18 = *(ushort **)(ESI + 0x18);           // bank flag
/// iVar15  = DAT_00fc8700;
/// if (((uint)puVar18 & 1) != 0) iVar15 = DAT_00fc8708;
/// puVar20 = *(ushort **)(ESI + 4);              // cursor
/// puVar22 = DAT_00fc8704;
/// if (((uint)puVar18 & 1) != 0) puVar22 = DAT_00fc870c;
/// if (puVar22 <= puVar20) return FUN_00642696();
/// if (puVar22 <= (ushort *)((int)puVar20 + 1)) return FUN_00642696();
/// *(undefined1 **)(ESI + 4) = (undefined1 *)((int)puVar20 + 1);
/// puVar28 = *(undefined1 *)((int)puVar20 + iVar15);
/// ```
///
/// Both comparisons are unsigned pointer comparisons on 32-bit values, so
/// the cursor arithmetic is reproduced with `u32` wraparound.  The second
/// test means the *last* byte of a bank can never be fetched; that is the
/// shipped check, not an artefact.  `memory` is the bank's byte image, so
/// the byte index is `cursor - bank.base`.
pub fn a88_script_read_byte(
    cursor: u32,
    flags: u32,
    primary: A88ScriptBank,
    alternate: A88ScriptBank,
    memory: &[u8],
) -> Result<A88ScriptRead, A88ScriptFault> {
    let bank = if flags & 1 != 0 { alternate } else { primary };
    if bank.end <= cursor || bank.end <= cursor.wrapping_add(1) {
        return Err(A88ScriptFault::InvalidScript);
    }
    let offset = cursor.wrapping_sub(bank.base) as usize;
    let byte = *memory.get(offset).ok_or(A88ScriptFault::InvalidScript)?;
    Ok(A88ScriptRead {
        byte,
        cursor: cursor.wrapping_add(1),
    })
}

/// `0x00642484`: commit the next instruction pointer.
///
/// The row is the common epilogue label
/// `*(undefined4 **)(ESI + 4) = in_EAX;` followed by the fetch loop above.
/// It stores a 32-bit pointer into the script state's cursor slot.
pub fn a88_script_store_cursor(cursor: &mut u32, next: u32) {
    *cursor = next;
}

// ---------------------------------------------------------------------------
// 0x00642687 / 0x00642696 - fatal diagnostics
// ---------------------------------------------------------------------------

/// `0x00642687`: add `AL` into the byte addressed by `EAX`, twice.
///
/// ```c
/// *in_EAX = *in_EAX + (char)in_EAX;
/// *in_EAX = *in_EAX + (char)in_EAX;
/// FUN_00952b40(&DAT_00d3b408);          // does not return
/// ```
///
/// The addend is the low byte of the *address* register, so the translated
/// value keeps it as an explicit argument.  The tail is a fatal error
/// (`FUN_00952b40`), reproduced by the caller, not by this arithmetic helper.
pub fn a88_add_al_twice(value: u8, address_low: u8) -> u8 {
    value.wrapping_add(address_low).wrapping_add(address_low)
}

/// `0x00642696`: the five words that spell `"Invalid AI script"`.
///
/// The row builds the message in place instead of pointing at `.rdata`:
///
/// ```c
/// *(u32 *)(EBP - 0x44) = 0x70695c67;
/// *(u32 *)(EBP - 0x44) = *(u32 *)(EBP - 0x44) + 0x5ff;
/// *(u32 *)(EBP - 0x44) = *(u32 *)(EBP - 0x44) ^ 0x1005;   // "crip"
/// *(u32 *)(EBP - 0x18) = *(u32 *)(EBP - 0x44);
/// *(u32 *)(EBP - 0x14) = 0x74;                            // "t\0\0\0"
/// *(u32 *)(EBP - 0x20) = 0x2064696c;                      // "lid "
/// *(u32 *)(EBP - 0x24) = 0x61766e49;                      // "Inva"
/// *(u32 *)(EBP - 0x44) = 0x732033ad;
/// *(u32 *)(EBP - 0x44) += 0xae9;
/// *(u32 *)(EBP - 0x44) += 0x1011;
/// *(u32 *)(EBP - 0x44) -= 0x566;                          // "AI s"
/// *(u32 *)(EBP - 0x1c) = *(u32 *)(EBP - 0x44);
/// FUN_00952b40(&DAT_00d3b408, EBP - 0x24, 0xa4046452);
/// ```
///
/// The returned array is in ascending memory order, exactly the 20 bytes
/// passed to the fatal-error reporter.  The message is 17 characters plus
/// the NUL at byte 17; bytes 18 and 19 are the padding of the last dword.
pub fn a88_invalid_ai_script_words() -> [u32; 5] {
    let mut word = 0x7069_5c67u32;
    word = word.wrapping_add(0x05ff);
    word ^= 0x1005;

    let tail = 0x7320_33adu32
        .wrapping_add(0x0ae9)
        .wrapping_add(0x1011)
        .wrapping_sub(0x0566);

    [
        0x6176_6e49, // "Inva"  (EBP - 0x24)
        0x2064_696c, // "lid "  (EBP - 0x20)
        tail,        // "AI s"  (EBP - 0x1c)
        word,        // "crip"  (EBP - 0x18)
        0x0000_0074, // "t\0\0\0" (EBP - 0x14)
    ]
}

/// The decoded message built by [`a88_invalid_ai_script_words`].
pub const A88_INVALID_AI_SCRIPT: &[u8] = b"Invalid AI script";

// ---------------------------------------------------------------------------
// 0x006427c6 - TSC compare / carry add
// ---------------------------------------------------------------------------

/// Compare constant in `0x006427c6` (`cmp eax, 0xfe98531c`).
pub const A88_TSC_COMPARE: u32 = 0xfe98_531c;

/// Addend in `0x006427c6` (`adc dword [mem], 0x33adbc45`).
pub const A88_TSC_ADDEND: u32 = 0x33ad_bc45;

/// `0x006427c6`: recoverable arithmetic of the row.
///
/// ```c
/// uVar1 = rdtsc();
/// bVar2 = (uint)uVar1 < 0xfe98531c;
/// *(int *)(ptr) = *(int *)(ptr) + 0x33adbc45 + (uint)bVar2;
/// ```
///
/// The `rdtsc` low word is compared with `0xfe98531c`; only the resulting
/// borrow/carry feeds the 32-bit add.  The rest of the row is decompiled as
/// bad-instruction data and is not modelled.  Returns the stored value and
/// the carry that was added.
pub fn a88_tsc_carry_add(value: u32, tsc_low: u32) -> (u32, bool) {
    let carry = tsc_low < A88_TSC_COMPARE;
    (
        value
            .wrapping_add(A88_TSC_ADDEND)
            .wrapping_add(carry as u32),
        carry,
    )
}

// ---------------------------------------------------------------------------
// 0x00642a00 / 0x00642a80 - spawn request and pool
// ---------------------------------------------------------------------------

/// The 0x34-byte request record built by `0x00642a00`.
///
/// Offsets are the record's own: `+0x00`/`+0x04` are not written by the row
/// (they are the caller stack slots it inherits), `+0x08`..`+0x0c` are
/// cleared, `+0x10` receives the unit type, `+0x24`/`+0x28`/`+0x2c` receive
/// the three arguments, and `+0x14`/`+0x18`/`+0x1c`/`+0x20`/`+0x30` are
/// cleared.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A88SpawnRequest {
    /// `+0x00`, inherited (not stored by the row).
    pub head_00: u32,
    /// `+0x04`, inherited (not stored by the row).
    pub head_04: u32,
    /// `+0x08`, cleared.
    pub clear_08: u32,
    /// `+0x0c`, cleared.
    pub clear_0c: u32,
    /// `+0x10`, the unit-type index (`FUN_006351b0`'s argument).
    pub unit_type: u32,
    /// `+0x14`, cleared.
    pub clear_14: u32,
    /// `+0x18`, cleared.
    pub clear_18: u32,
    /// `+0x1c`, cleared.
    pub clear_1c: u32,
    /// `+0x20`, cleared.
    pub clear_20: u32,
    /// `+0x24`, first argument.
    pub arg_24: u32,
    /// `+0x28`, second argument.
    pub arg_28: u32,
    /// `+0x2c`, third argument (the record's region tag source).
    pub arg_2c: u32,
    /// `+0x30`, cleared.
    pub clear_30: u32,
}

impl A88SpawnRequest {
    /// The record as the 13 dwords `0x00642a00` lays out on its stack.
    pub fn a88_words(&self) -> [u32; 13] {
        [
            self.head_00,
            self.head_04,
            self.clear_08,
            self.clear_0c,
            self.unit_type,
            self.clear_14,
            self.clear_18,
            self.clear_1c,
            self.clear_20,
            self.arg_24,
            self.arg_28,
            self.arg_2c,
            self.clear_30,
        ]
    }
}

/// Third argument `0x00642a00` passes to `0x00642bf0`.
pub const A88_SPAWN_REQUEST_SELECTOR: u32 = 2;

/// `0x00642a00`: assemble the spawn request.
///
/// ```c
/// iVar1 = FUN_006351b0(param_1);          // units.dat u16 at
///                                         // 0x00faa412 + type * 0x4e8
/// if (iVar1 == 0) return;
/// local_14 = param_2; local_10 = param_3; local_c = param_4;
/// local_30 = 0; local_2c = 0; local_24 = 0; local_1c = 0;
/// local_20 = 0; local_18 = 0; local_8 = 0; local_28 = param_1;
/// FUN_00642bf0(&local_38, iVar1, 2);
/// ```
///
/// `FUN_006351b0` is the units.dat lookup (a `u16` at a fixed stride); the
/// caller supplies its result as `resolved_unit`, so this function returns
/// `None` exactly when that lookup returns zero.  The request is emitted
/// only in that non-zero case.
pub fn a88_spawn_request(
    head: [u32; 2],
    unit_type: u32,
    resolved_unit: u16,
    arg_24: u32,
    arg_28: u32,
    arg_2c: u32,
) -> Option<A88SpawnRequest> {
    if resolved_unit == 0 {
        return None;
    }
    Some(A88SpawnRequest {
        head_00: head[0],
        head_04: head[1],
        clear_08: 0,
        clear_0c: 0,
        unit_type,
        clear_14: 0,
        clear_18: 0,
        clear_1c: 0,
        clear_20: 0,
        arg_24,
        arg_28,
        arg_2c,
        clear_30: 0,
    })
}

/// One node of the spawn list.  Offsets are the retail node's dwords.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A88SpawnNode {
    /// `+0x00`: free-list next while the node is free, list next once live.
    pub next: u32,
    /// `+0x04`: list previous.
    pub prev: u32,
    /// `+0x08`: location index (`FUN_00642a80`'s second argument).
    pub location: u32,
    /// `+0x0c`: cleared.
    pub reserved_0c: u32,
    /// `+0x10`: unit type.
    pub unit_type: u32,
    /// `+0x14..+0x20`: left, top, right, bottom.
    pub rect: [i32; 4],
    /// `+0x24`/`+0x28`: rect centre.
    pub center: [i32; 2],
    /// `+0x2c`: cleared.
    pub reserved_2c: u32,
    /// `+0x30`: flag bit 0 (low three bits are preserved from the old node).
    pub flags: u32,
}

/// The spawn list at `0x00fc9b64 + 0x1450` (free head) and `0x00fc9b68`
/// (live head).
///
/// Node indices are 1-based because the retail code uses a null pointer as
/// "no node"; index 0 is therefore the null link.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct A88SpawnPool {
    /// Free-list head, `*(DAT_00fc9b64 + 0x1450)`.
    pub free_head: u32,
    /// Live-list head, `DAT_00fc9b68`.
    pub head: u32,
    /// Backing nodes.
    pub nodes: Vec<A88SpawnNode>,
}

impl A88SpawnPool {
    /// Build a pool of `capacity` free nodes chained through `+0x00`.
    pub fn a88_new(capacity: usize) -> Self {
        let mut nodes = vec![A88SpawnNode::default(); capacity];
        for (i, node) in nodes.iter_mut().enumerate() {
            node.next = if i + 1 < capacity { (i + 2) as u32 } else { 0 };
        }
        Self {
            free_head: if capacity == 0 { 0 } else { 1 },
            head: 0,
            nodes,
        }
    }

    /// Node by 1-based index.
    pub fn a88_node(&self, index: u32) -> Option<&A88SpawnNode> {
        if index == 0 {
            return None;
        }
        self.nodes.get((index - 1) as usize)
    }

    /// `0x00642a80`: take a free node, push it on the live list and fill it.
    ///
    /// ```c
    /// piVar1 = *(int **)(DAT_00fc9b64 + 0x1450);
    /// if (piVar1 == 0) return 0;
    /// iVar4 = *piVar1;
    /// *(DAT_00fc9b64 + 0x1450) = iVar4;
    /// if (iVar4 != 0) *(iVar4 + 4) = 0;
    /// piVar1[1] = 0;
    /// piVar1[0] = DAT_00fc9b68;
    /// if (DAT_00fc9b68 != 0) *(DAT_00fc9b68 + 4) = piVar1;
    /// DAT_00fc9b68 = piVar1;
    /// piVar1[2] = param_2;  piVar1[4] = param_1;  piVar1[3] = 0;
    /// piVar1[0xc] = (param_4 != 0) | piVar1[0xc] & 0xfffffffe;
    /// if (param_3 == 0) { piVar1[5..9] = 0; iVar4 = 0; }
    /// else {
    ///   iVar4 = param_3[1]; iVar2 = param_3[2]; iVar3 = param_3[3];
    ///   piVar1[5] = param_3[0]; piVar1[6] = iVar4;
    ///   piVar1[7] = iVar2;      piVar1[8] = iVar3;
    ///   piVar1[9] = (param_3[2] - param_3[0]) / 2 + param_3[0];
    ///   iVar4 = (param_3[3] - param_3[1]) / 2 + param_3[1];
    /// }
    /// piVar1[10] = iVar4;
    /// piVar1[0xc] = piVar1[0xc] & 7;
    /// piVar1[0xb] = 0;
    /// ```
    ///
    /// The division is C signed division (towards zero); `wrapping_*` keeps
    /// the 32-bit arithmetic defined for the extrema C leaves undefined.
    pub fn a88_alloc(
        &mut self,
        unit_type: u32,
        location: u32,
        rect: Option<[i32; 4]>,
        flag: bool,
    ) -> Option<u32> {
        let index = self.free_head;
        if index == 0 {
            return None;
        }
        let slot = (index - 1) as usize;
        let mut node = self.nodes[slot];
        let next_free = node.next;
        self.free_head = next_free;
        if next_free != 0 {
            self.nodes[(next_free - 1) as usize].prev = 0;
        }

        let old_head = self.head;
        node.prev = 0;
        node.next = old_head;
        if old_head != 0 {
            self.nodes[(old_head - 1) as usize].prev = index;
        }
        self.head = index;

        node.location = location;
        node.unit_type = unit_type;
        node.reserved_0c = 0;
        node.flags = ((flag as u32) | (node.flags & 0xffff_fffe)) & 7;
        match rect {
            None => {
                node.rect = [0; 4];
                node.center = [0; 2];
            }
            Some([left, top, right, bottom]) => {
                node.rect = [left, top, right, bottom];
                node.center = [
                    right.wrapping_sub(left) / 2 + left,
                    bottom.wrapping_sub(top) / 2 + top,
                ];
            }
        }
        node.reserved_2c = 0;
        self.nodes[slot] = node;
        Some(index)
    }
}

// ---------------------------------------------------------------------------
// 0x00642b50 / 0x00642bf0 - spawn placement
// ---------------------------------------------------------------------------

/// Upper bound of the location table used by `FUN_0076a4d0`/`FUN_0076a500`.
pub const A88_LOCATION_LIMIT: u32 = 0xfa;

/// Rect and word built by `0x00642b50` for one spawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A88LocationSpawn {
    /// `{x - 1, y - 1, x + 1, y + 1}`.
    pub rect: [i32; 4],
    /// `(location << 3) | (old_flags & 7)`.
    pub flags_word: u32,
    /// Retail's final `FUN_00635720(...) == 0` test: the unit still needs
    /// the `FUN_00657260(unit, x, y)` placement call.
    pub needs_placement: bool,
}

/// `FUN_0076a4d0` / `FUN_0076a500`: location centre from the 0x30-byte table.
///
/// Both helpers return `0` for `location == 0` or `location >= 0xfa`, and
/// otherwise read the `i16` at `0x011ac918 + location * 0x30` (`x`) or
/// `0x011ac91a + location * 0x30` (`y`).  The table is supplied by the
/// caller; a missing entry reproduces the out-of-range zero.
pub fn a88_location_center(location: u32, table: &[(i16, i16)]) -> (i32, i32) {
    if location != 0 && location < A88_LOCATION_LIMIT {
        if let Some(&(x, y)) = table.get(location as usize) {
            return (x as i32, y as i32);
        }
    }
    (0, 0)
}

/// `0x00642b50`: turn a location index into the spawn rect and flag word.
///
/// ```c
/// iVar2 = FUN_0076a4d0(param_2);          // x
/// iVar3 = FUN_0076a500(param_2);          // y
/// local_14 = iVar2 - 1; local_10 = iVar3 - 1;
/// local_c  = iVar2 + 1; local_8  = iVar3 + 1;
/// iVar4 = FUN_00642a80(*(u32 *)(param_1 + 0x10), param_3, &local_14,
///                      *(u32 *)(param_1 + 0x30) & 1);
/// if (iVar4 != 0) {
///   *(u32 *)(iVar4 + 0x30) = param_2 << 3 | *(u32 *)(iVar4 + 0x30) & 7;
///   uVar1 = FUN_0075d720(iVar2, iVar3);   // zone lookup, out of batch
///   FUN_00656540(*(u32 *)(param_1 + 0x10), uVar1, 0);
///   if (FUN_00635720(*(u32 *)(param_1 + 0x10)) == 0)
///       FUN_00657260(*(u32 *)(param_1 + 0x10), iVar2, iVar3);
/// }
/// ```
///
/// The three callees are engine state (zone lookup, zone assignment and the
/// placement predicate); the caller supplies `needs_placement`, which is
/// the `FUN_00635720(...) == 0` result.
pub fn a88_spawn_at_location(
    location: u32,
    center: (i32, i32),
    old_flags: u32,
    needs_placement: bool,
) -> A88LocationSpawn {
    let (x, y) = center;
    A88LocationSpawn {
        rect: [
            x.wrapping_sub(1),
            y.wrapping_sub(1),
            x.wrapping_add(1),
            y.wrapping_add(1),
        ],
        flags_word: location.wrapping_shl(3) | (old_flags & 7),
        needs_placement,
    }
}

/// `0x00642bf0`: the per-unit-type size gate.
///
/// ```c
/// iVar1 = *(int *)(param_1 + 0x10);                     // unit type
/// if ((4999 < *(int *)((DAT_00f063cc ^ 0x10fae560) + iVar1 * 4)) &&
///     (2999 < *(int *)((DAT_00f063cc ^ 0x10fae560) + 0x30 + iVar1 * 4)) &&
///     (FUN_00635720(iVar1) == 0)) return;
/// iVar1 = FUN_0063b420(*(u32 *)(param_1 + 0x2c));       // byte at +0x1c
/// if (iVar1 != 0 && (iVar1 = FUN_0076a530(type, iVar1, param_3)) != 0)
///     FUN_00642b50(param_1, iVar1, param_2);
/// ```
///
/// The two arrays at `(DAT_00f063cc ^ 0x10fae560)` and `+0x30` are indexed
/// by unit type; callers supply the two `i32` entries as `width`/`height`.
/// `skip` is the early return (a large unit type that already owns a
/// placement).  `region_tag` is the `FUN_0063b420` byte, zero meaning "no
/// location spawn".
pub fn a88_spawn_gate(width: i32, height: i32, placed: bool, region_tag: u8) -> A88SpawnGate {
    A88SpawnGate {
        skip: 4999 < width && 2999 < height && !placed,
        region_tag,
    }
}

/// Result of [`a88_spawn_gate`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A88SpawnGate {
    /// The `if (...) return;` early exit.
    pub skip: bool,
    /// `FUN_0063b420`'s byte, `*(u8 *)(*(u32 *)(record + 0x2c) + 0x1c)`.
    pub region_tag: u8,
}

// ---------------------------------------------------------------------------
// 0x00642e9b / 0x00643ca4 - MSVC `std::string` SSO constructor
// ---------------------------------------------------------------------------

/// Offset of the inline buffer in the 28-byte MSVC string object.
pub const A88_SSO_PTR_OFFSET: u32 = 0x0c;

/// Capacity dword of a 15-byte SSO string: `0x80000000 | 15`.
pub const A88_SSO_CAPACITY_TAG: u32 = 0x8000_000f;

/// Bytes of the inline buffer.
pub const A88_SSO_BUFFER_BYTES: usize = 16;

/// The MSVC `std::string` prologue recovered from the window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A88SsoString {
    /// `+0x00`: pointer to the inline buffer (`this + 0x0c`).
    pub ptr_offset: u32,
    /// `+0x04`: size, zero.
    pub size: u32,
    /// `+0x08`: capacity/flags dword.
    pub capacity: u32,
    /// `+0x0c..+0x1b`: inline buffer, first byte zero.
    pub buffer: [u8; A88_SSO_BUFFER_BYTES],
}

impl Default for A88SsoString {
    fn default() -> Self {
        Self {
            ptr_offset: A88_SSO_PTR_OFFSET,
            size: 0,
            capacity: A88_SSO_CAPACITY_TAG,
            buffer: [0; A88_SSO_BUFFER_BYTES],
        }
    }
}

/// `0x00642e9b`: construct an empty SSO string.
///
/// ```c
/// extraout_ECX[1] = 0;
/// *extraout_ECX   = extraout_ECX + 3;     // dwords: this + 0x0c
/// extraout_ECX[2] = 0x8000000f;
/// *(u8 *)(extraout_ECX + 3) = 0;
/// ```
///
/// `+3` is in dword units, so the buffer begins at `+0x0c`; `0x8000000f`
/// is MSVC's "capacity 15, heap flag clear" SSO tag.
pub fn a88_sso_string_init() -> A88SsoString {
    A88SsoString::default()
}

/// `0x00643ca4`: the same constructor in a catch frame.
///
/// ```c
/// *(u32 *)(EBP - 0x44) = 0;
/// *(u32 *)(EBP - 0x48) = EBP - 0x3c;
/// *(u32 *)(EBP - 0x40) = 0x8000000f;
/// *(u8  *)(EBP - 0x3c) = 0;
/// ```
///
/// The pointer/store order differs (the buffer is `EBP - 0x3c` and the
/// object starts at `EBP - 0x48`) but the resulting object is identical.
pub fn a88_sso_string_init_catch_frame() -> A88SsoString {
    A88SsoString::default()
}

// ---------------------------------------------------------------------------
// 0x006433d7 - 33-bit rotate
// ---------------------------------------------------------------------------

/// `0x006433d7`: rotate a 32-bit value with the carry flag as bit 32.
///
/// ```c
/// uVar3 = *(u32 *)(*(int *)(EBP - 0x3c) + 8);
/// bVar5 = (byte)uVar3 & 0x1f;
/// uVar2 = *(u32 *)(EBX + 0x90664989);
/// *(u32 *)(EBX + 0x90664989) =
///     (u32)(CONCAT14(EBX < 0x1c0272d7, uVar2) >> bVar5) |
///     uVar2 << 0x21 - bVar5;
/// ```
///
/// This is `rcr dword [ebx+0x90664989], cl` with the pre-existing carry
/// (`EBX < 0x1c0272d7`) as bit 32.  The x86 count is masked to five bits
/// and a zero count is a no-op, so both are reproduced here rather than the
/// decompiler's `0x21 - count` expression (which is only valid for
/// non-zero counts).
pub fn a88_rotate_right_through_carry(value: u32, carry: bool, count: u32) -> (u32, bool) {
    let n = count & 31;
    if n == 0 {
        return (value, carry);
    }
    let ring = ((carry as u64) << 32) | value as u64;
    let rotated = ((ring >> n) | (ring << (33 - n))) & 0x1_ffff_ffff;
    ((rotated & 0xffff_ffff) as u32, rotated >> 32 != 0)
}

// ---------------------------------------------------------------------------
// 0x00643860 .. 0x00643aa0 - region list
// ---------------------------------------------------------------------------

/// One region-list node.  Only the fields the batch's rows touch are kept.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct A88RegionNode {
    /// `+0x00`: next node.
    pub next: Option<u32>,
    /// `+0x04`: previous node.
    pub prev: Option<u32>,
    /// `+0x08`: first key (`0x00643f40` forwards it).
    pub key_08: u32,
    /// `+0x0c`: second key (`0x00643f40` forwards it).
    pub key_0c: u32,
    /// `+0x10`: destructor tag checked by `0x00643860`/`0x006439eb`.
    pub key_10: i32,
    /// `+0x34..+0x40`: the 16-byte rect.
    pub rect: [u32; 4],
    /// `+0x44`: "rect valid" byte.
    pub valid: u8,
    /// `+0x4d`: cleared by the unlink/reset paths.
    pub byte_4d: u8,
    /// `+0x4f`: cleared by the unlink/reset paths.
    pub byte_4f: u8,
    /// `+0x51`: cleared by the unlink/reset paths.
    pub byte_51: u8,
}

/// Clear the four bytes the region teardown rows zero
/// (`0x00643860` at `+0x51/+0x4f/+0x4d/+0x44` and `0x006439eb` with the
/// same four offsets).
fn a88_clear_region_bytes(node: &mut A88RegionNode) {
    node.byte_51 = 0;
    node.byte_4f = 0;
    node.byte_4d = 0;
    node.valid = 0;
}

/// The region list whose sentinel object holds `+0x00` head, `+0x04` tail
/// and `+0x08` count.
///
/// Retail stores both links in one sentinel object and compares the tail
/// pointer with the sentinel to detect an empty list; here the links are
/// `Option<u32>` indices into `nodes`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct A88RegionList {
    /// `+0x00`.
    pub head: Option<u32>,
    /// `+0x04`.
    pub tail: Option<u32>,
    /// `+0x08`.
    pub count: u32,
    /// Nodes addressed by the two links.
    pub nodes: Vec<A88RegionNode>,
}

impl A88RegionList {
    /// An empty list (empty lists have no node to clear).
    pub fn a88_new() -> Self {
        Self::default()
    }

    /// Node by index.
    pub fn a88_node(&self, index: u32) -> Option<&A88RegionNode> {
        self.nodes.get(index as usize)
    }

    /// `0x00643860`: reset the sentinel and tear down the old tail node.
    ///
    /// ```c
    /// iVar1 = *(int *)(ECX + 4);        // old tail
    /// *(int *)(ECX + 4) = ECX;          // tail = sentinel
    /// *(int *)ECX = ECX;                // head = sentinel
    /// *(u32 *)(ECX + 8) = 0;            // count = 0
    /// if (iVar1 == ECX) return;         // was empty
    /// *(u8 *)(iVar1 + 0x51) = 0;
    /// *(u8 *)(iVar1 + 0x4f) = 0;
    /// *(u8 *)(iVar1 + 0x4d) = 0;
    /// *(u8 *)(iVar1 + 0x44) = 0;
    /// ```
    ///
    /// Returns the node that was torn down, if the list was not empty.  The
    /// row's two `FUN_0093ec70` calls are frame unwinds (one per destructor
    /// branch), not engine state.
    pub fn a88_reset(&mut self) -> Option<u32> {
        let old_tail = if self.head.is_none() { None } else { self.tail };
        self.head = None;
        self.tail = None;
        self.count = 0;
        if let Some(index) = old_tail {
            if let Some(node) = self.nodes.get_mut(index as usize) {
                a88_clear_region_bytes(node);
            }
        }
        old_tail
    }

    /// `0x00643910`: clear only the list head.
    ///
    /// The row is the single store `*in_ECX = 0;`; the tail and count are
    /// deliberately left untouched, matching the shipped code.
    pub fn a88_clear_head(&mut self) {
        self.head = None;
    }

    /// `0x006439eb`: unlink the tail node and clear its flag bytes.
    ///
    /// ```c
    /// piVar1 = *(int **)(EDI + 4);          // tail
    /// *(int *)(*piVar1 + 4) = piVar1[1];    // node->next->prev = node->prev
    /// *(int *)piVar1[1] = *piVar1;          // node->prev->next = node->next
    /// *(u8 *)((int)piVar1 + 0x51) = 0;
    /// *(u8 *)((int)piVar1 + 0x4f) = 0;
    /// *(u8 *)((int)piVar1 + 0x4d) = 0;
    /// *(u8 *)(piVar1 + 0x11) = 0;           // +0x44
    /// iVar2 = piVar1[4];                    // +0x10
    /// if (-1 < iVar2) FUN_0093ec70(...);    // element destructor
    /// ```
    ///
    /// The `+0x00`/`+0x04` stores target the sentinel when the removed node
    /// is the first/last, which in this model is the list head/tail slot.
    pub fn a88_unlink_tail(&mut self) -> A88RegionUnlink {
        let Some(index) = self.tail else {
            return A88RegionUnlink {
                removed: None,
                destructor_ran: false,
            };
        };
        let mut node = self.nodes[index as usize].clone();
        match node.next {
            Some(next) => self.nodes[next as usize].prev = node.prev,
            None => self.tail = node.prev,
        }
        match node.prev {
            Some(prev) => self.nodes[prev as usize].next = node.next,
            None => self.head = node.next,
        }
        a88_clear_region_bytes(&mut node);
        let destructor_ran = node.key_10 >= 0;
        self.nodes[index as usize] = node;
        A88RegionUnlink {
            removed: Some(index),
            destructor_ran,
        }
    }
}

/// Result of [`A88RegionList::a88_unlink_tail`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A88RegionUnlink {
    /// The unlinked node, `None` for an empty list.
    pub removed: Option<u32>,
    /// The `-1 < node->+0x10` destructor branch.
    pub destructor_ran: bool,
}

/// `0x00643aa0`: the region-list non-null gate.
///
/// ```c
/// if (DAT_00fc9b70 == 0) FUN_00923df0(0xc);
/// ```
///
/// Returns `true` when the list exists; a null list takes the abort path
/// with code `0xc`.
pub fn a88_region_list_present(pointer: u32) -> bool {
    pointer != 0
}

/// A rect snapshot produced by `0x00643b10`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A88RegionSnapshot {
    /// The node's `+0x44` byte.
    pub valid: u8,
    /// The caller's rect when the node is invalid, else the node's rect.
    pub rect: [u32; 4],
}

/// `0x00643b10`: copy the node's rect out when `+0x44` is set.
///
/// ```c
/// iVar2 = *in_ECX;
/// cVar1 = *(char *)(iVar2 + 0x44);
/// *(char *)(param_1 + 4) = cVar1;          // byte at out + 0x10
/// if (cVar1 != '\0') {
///     uVar3 = *(u32 *)(iVar2 + 0x38);  uVar4 = *(u32 *)(iVar2 + 0x3c);
///     uVar5 = *(u32 *)(iVar2 + 0x40);
///     *param_1 = *(u32 *)(iVar2 + 0x34);
///     param_1[1] = uVar3;  param_1[2] = uVar4;  param_1[3] = uVar5;
/// }
/// ```
///
/// `out_rect` is the destination buffer as the caller left it: it is only
/// overwritten when the node's valid byte is non-zero.
pub fn a88_region_snapshot(node: &A88RegionNode, out_rect: [u32; 4]) -> A88RegionSnapshot {
    A88RegionSnapshot {
        valid: node.valid,
        rect: if node.valid != 0 { node.rect } else { out_rect },
    }
}

/// Scratch record built by `0x00643b30`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A88RegionScratch {
    /// `param_1[0..3]`, zeroed by the row.
    pub cleared: [u32; 3],
    /// The two dwords read at `ECX + 0x1c`.
    pub probe: [u32; 2],
    /// The `std::string` constructed at `EBP - 0x30`.
    pub string: A88SsoString,
}

/// `0x00643b30`: zero the output triple, probe `ECX + 0x1c` and set up the
/// scratch string.
///
/// ```c
/// *param_1 = 0; param_1[1] = 0; param_1[2] = 0;
/// FUN_0074f3f0(local_30); FUN_0074fb10(local_68, ...);
/// FUN_0074f340(local_30); FUN_006025e0(...);
/// puVar3 = (u32 *)FUN_00715e10(local_68, ECX + 0x1c);   // two dwords
/// FUN_0067dcc0(local_30, *puVar3, puVar3[1]);
/// FUN_00538810(0);                                      // does not return
/// ```
///
/// The two dwords at `ECX + 0x1c` are the only engine input the row reads;
/// everything else is MFC string construction and the terminal report.
pub fn a88_region_scratch(probe: [u32; 2]) -> A88RegionScratch {
    A88RegionScratch {
        cleared: [0; 3],
        probe,
        string: a88_sso_string_init(),
    }
}

/// `0x00643f40`: forward the point and the node's two keys.
///
/// ```c
/// FUN_0067d6b0(*param_1, param_1[1], *(*in_ECX + 8), *(*in_ECX + 0xc));
/// ```
///
/// The callee is out of batch; the row's whole contribution is the argument
/// order, returned here as `[x, y, key_08, key_0c]`.
pub fn a88_region_marshal_point(point: [u32; 2], node: &A88RegionNode) -> [u32; 4] {
    [point[0], point[1], node.key_08, node.key_0c]
}

/// Result of [`a88_region_rect_store`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A88RegionRectStore {
    /// The output buffer after the row: the old node rect on swap, else the
    /// incoming rect unchanged.
    pub buffer: [u32; 4],
    /// `true` when the node already had a valid rect and the two buffers
    /// were exchanged; `false` when the rect was installed.
    pub swapped: bool,
    /// The node's `+0x44` byte after the row (always set).
    pub valid: u8,
}

/// `0x00643fe0`: store a 16-byte rect, or swap it with the existing one.
///
/// ```c
/// puVar3 = FUN_00715a40(local_2c, param_1, param_2);   // 16-byte rect
/// local_1c = *puVar3; uStack_18 = puVar3[1];
/// uStack_14 = puVar3[2]; uStack_10 = puVar3[3];
/// puVar1 = (u32 *)(node + 0x34);
/// if (*(char *)(node + 0x44) != '\0') {
///     iVar4 = 0x10;
///     do { uVar2 = *(u8 *)((int)&local_1c + -(int)puVar1 + (int)puVar3);
///          (local_2c + -(int)puVar1 + 0xf)[(int)puVar3 + 1] = *(u8 *)puVar3;
///          *(u8 *)puVar3 = uVar2; puVar3++; } while (--iVar4);
///     FUN_00c86b6e();                                  // does not return
/// }
/// *puVar1 = *puVar3;
/// *(u32 *)(node + 0x38) = uStack_18;
/// *(u32 *)(node + 0x3c) = uStack_14;
/// *(u32 *)(node + 0x40) = uStack_10;
/// *(u8  *)(node + 0x44) = 1;
/// ```
///
/// The 16-byte byte-wise exchange is equivalent to swapping the two
/// `[u32; 4]` values.
pub fn a88_region_rect_store(node: &mut A88RegionNode, incoming: [u32; 4]) -> A88RegionRectStore {
    if node.valid != 0 {
        let old = node.rect;
        node.rect = incoming;
        A88RegionRectStore {
            buffer: old,
            swapped: true,
            valid: node.valid,
        }
    } else {
        node.rect = incoming;
        node.valid = 1;
        A88RegionRectStore {
            buffer: incoming,
            swapped: false,
            valid: 1,
        }
    }
}

#[cfg(test)]
mod tests_agent88 {
    use super::*;

    // -- 0x00642037 ---------------------------------------------------------

    #[test]
    fn script_fetch_uses_the_selected_bank_and_advances_one_byte() {
        let primary = A88ScriptBank {
            base: 0x1000,
            end: 0x1004,
        };
        let alternate = A88ScriptBank {
            base: 0x2000,
            end: 0x2002,
        };
        let primary_bytes = b"abcd";
        let alternate_bytes = b"xy";

        assert_eq!(
            a88_script_read_byte(0x1000, 0, primary, alternate, primary_bytes),
            Ok(A88ScriptRead {
                byte: b'a',
                cursor: 0x1001
            })
        );
        assert_eq!(
            a88_script_read_byte(0x1002, 0, primary, alternate, primary_bytes),
            Ok(A88ScriptRead {
                byte: b'c',
                cursor: 0x1003
            })
        );
        // Flag bit 0 selects the second pointer pair.
        assert_eq!(
            a88_script_read_byte(0x2000, 1, primary, alternate, alternate_bytes),
            Ok(A88ScriptRead {
                byte: b'x',
                cursor: 0x2001
            })
        );
        // The last byte is rejected by `end <= cursor + 1`.
        assert_eq!(
            a88_script_read_byte(0x1003, 0, primary, alternate, primary_bytes),
            Err(A88ScriptFault::InvalidScript)
        );
        assert_eq!(
            a88_script_read_byte(0x2001, 1, primary, alternate, alternate_bytes),
            Err(A88ScriptFault::InvalidScript)
        );
        // An empty bank fails the first test.
        assert_eq!(
            a88_script_read_byte(0x1004, 0, primary, alternate, primary_bytes),
            Err(A88ScriptFault::InvalidScript)
        );
    }

    #[test]
    fn script_cursor_store_replaces_the_slot() {
        let mut cursor = 0x1000u32;
        a88_script_store_cursor(&mut cursor, 0x2222_3333);
        assert_eq!(cursor, 0x2222_3333);
    }

    // -- 0x00642687 ---------------------------------------------------------

    #[test]
    fn al_is_added_twice_and_wraps() {
        assert_eq!(a88_add_al_twice(0x10, 0xff), 0x0e);
        assert_eq!(a88_add_al_twice(0x00, 0x00), 0x00);
        assert_eq!(a88_add_al_twice(0x80, 0x80), 0x80);
        assert_eq!(a88_add_al_twice(0xff, 0x01), 0x01);
    }

    // -- 0x00642696 ---------------------------------------------------------

    #[test]
    fn invalid_ai_script_words_spell_the_message() {
        let words = a88_invalid_ai_script_words();
        let mut bytes = [0u8; 20];
        for (i, word) in words.iter().enumerate() {
            bytes[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        assert_eq!(&bytes[..17], A88_INVALID_AI_SCRIPT);
        assert_eq!(A88_INVALID_AI_SCRIPT.len(), 17);
        assert_eq!(bytes[17], 0);
        assert_eq!(&bytes[18..], &[0, 0]);
        assert_eq!(words[0], 0x6176_6e49);
        assert_eq!(words[1], 0x2064_696c);
        assert_eq!(words[2], 0x7320_4941);
    }

    // -- 0x006427c6 ---------------------------------------------------------

    #[test]
    fn tsc_carry_add_uses_the_compare_borrow() {
        assert_eq!(a88_tsc_carry_add(0, 0), (A88_TSC_ADDEND + 1, true));
        assert_eq!(
            a88_tsc_carry_add(0, A88_TSC_COMPARE),
            (A88_TSC_ADDEND, false)
        );
        assert_eq!(
            a88_tsc_carry_add(0xffff_ffff, 0xffff_ffff),
            (0xffff_ffffu32.wrapping_add(A88_TSC_ADDEND), false)
        );
        assert_eq!(A88_TSC_COMPARE, 0xfe98_531c);
        assert_eq!(A88_TSC_ADDEND, 0x33ad_bc45);
    }

    // -- 0x00642a00 ---------------------------------------------------------

    #[test]
    fn spawn_request_layout_and_gate() {
        let request = a88_spawn_request([0xaaaa, 0xbbbb], 7, 1, 2, 3, 4).unwrap();
        assert_eq!(
            request.a88_words(),
            [0xaaaa, 0xbbbb, 0, 0, 7, 0, 0, 0, 0, 2, 3, 4, 0]
        );
        assert_eq!(request.unit_type, 7);
        assert_eq!(request.arg_24, 2);
        assert_eq!(request.arg_28, 3);
        assert_eq!(request.arg_2c, 4);
        assert_eq!(A88_SPAWN_REQUEST_SELECTOR, 2);

        // `FUN_006351b0` returning zero skips the request entirely.
        assert_eq!(a88_spawn_request([1, 2], 7, 0, 2, 3, 4), None);
    }

    // -- 0x00642a80 ---------------------------------------------------------

    #[test]
    fn pool_alloc_pops_free_list_and_pushes_live_list() {
        let mut pool = A88SpawnPool::a88_new(3);
        assert_eq!(pool.free_head, 1);
        assert_eq!(pool.head, 0);
        assert_eq!(pool.nodes[0].next, 2);
        assert_eq!(pool.nodes[2].next, 0);

        let first = pool.a88_alloc(7, 4, Some([10, 20, 30, 40]), true).unwrap();
        assert_eq!(first, 1);
        assert_eq!(pool.head, 1);
        assert_eq!(pool.free_head, 2);
        let node = pool.a88_node(first).unwrap();
        assert_eq!(node.next, 0);
        assert_eq!(node.prev, 0);
        assert_eq!(node.location, 4);
        assert_eq!(node.reserved_0c, 0);
        assert_eq!(node.unit_type, 7);
        assert_eq!(node.rect, [10, 20, 30, 40]);
        assert_eq!(node.center, [20, 30]);
        assert_eq!(node.reserved_2c, 0);
        assert_eq!(node.flags, 1);

        let second = pool.a88_alloc(9, 5, None, false).unwrap();
        assert_eq!(second, 2);
        assert_eq!(pool.head, 2);
        assert_eq!(pool.nodes[0].prev, 2);
        let node = pool.a88_node(second).unwrap();
        assert_eq!(node.next, 1);
        assert_eq!(node.rect, [0; 4]);
        assert_eq!(node.center, [0; 2]);
        assert_eq!(node.flags, 0);

        assert_eq!(pool.a88_alloc(1, 1, None, false), Some(3));
        assert_eq!(pool.a88_alloc(1, 1, None, false), None);

        // Negative centres use C's truncate-toward-zero division.
        let mut pool = A88SpawnPool::a88_new(1);
        let index = pool.a88_alloc(1, 1, Some([-3, -3, 0, 0]), false).unwrap();
        let node = pool.a88_node(index).unwrap();
        // (0 - -3) / 2 + -3 = 1 - 3.
        assert_eq!(node.center, [-2, -2]);
    }

    // -- 0x00642b50 ---------------------------------------------------------

    #[test]
    fn location_center_and_spawn_rect() {
        let table = [(-1i16, -1i16), (100, 200), (300, 400)];
        assert_eq!(a88_location_center(0, &table), (0, 0));
        assert_eq!(a88_location_center(1, &table), (100, 200));
        assert_eq!(a88_location_center(2, &table), (300, 400));
        assert_eq!(a88_location_center(3, &table), (0, 0));
        assert_eq!(a88_location_center(0xfa, &table), (0, 0));

        let spawn = a88_spawn_at_location(4, (100, 200), 0b1010, true);
        assert_eq!(spawn.rect, [99, 199, 101, 201]);
        assert_eq!(spawn.flags_word, 4 << 3 | 2);
        assert!(spawn.needs_placement);
        // The low three bits of the node flags are preserved.
        assert_eq!(
            a88_spawn_at_location(1, (0, 0), 0xffff_ffff, false).flags_word,
            8 | 7
        );
    }

    // -- 0x00642bf0 ---------------------------------------------------------

    #[test]
    fn spawn_gate_thresholds_are_4999_by_2999() {
        assert!(!a88_spawn_gate(4999, 2999, false, 0).skip);
        assert!(!a88_spawn_gate(5000, 2999, false, 0).skip);
        assert!(!a88_spawn_gate(4999, 3000, false, 0).skip);
        // Both bounds exceeded and no placement yet: early return.
        assert!(a88_spawn_gate(5000, 3000, false, 0).skip);
        // A placed unit type takes the location path instead.
        assert!(!a88_spawn_gate(5000, 3000, true, 0).skip);
        // The region tag is passed through even when the gate does not skip.
        assert_eq!(a88_spawn_gate(0, 0, false, 7).region_tag, 7);
    }

    // -- 0x00642e9b / 0x00643ca4 -------------------------------------------

    #[test]
    fn sso_string_constructor_layout() {
        let string = a88_sso_string_init();
        assert_eq!(string.ptr_offset, 0x0c);
        assert_eq!(string.size, 0);
        assert_eq!(string.capacity, 0x8000_000f);
        assert_eq!(string.capacity & 0x8000_0000, 0x8000_0000);
        assert_eq!(string.capacity & 0x7fff_ffff, 15);
        assert_eq!(string.buffer, [0u8; 16]);
        assert_eq!(a88_sso_string_init_catch_frame(), string);
    }

    // -- 0x006433d7 ---------------------------------------------------------

    #[test]
    fn rotate_right_through_carry_is_a_33_bit_ring() {
        assert_eq!(
            a88_rotate_right_through_carry(0, true, 1),
            (0x8000_0000, false)
        );
        assert_eq!(a88_rotate_right_through_carry(1, false, 1), (0, true));
        assert_eq!(
            a88_rotate_right_through_carry(0x8000_0000, false, 1),
            (0x4000_0000, false)
        );
        assert_eq!(
            a88_rotate_right_through_carry(0xffff_ffff, true, 1),
            (0xffff_ffff, true)
        );
        // A zero (and 32, after masking) count is a no-op.
        assert_eq!(
            a88_rotate_right_through_carry(0x1234, true, 0),
            (0x1234, true)
        );
        assert_eq!(
            a88_rotate_right_through_carry(0x1234, true, 32),
            (0x1234, true)
        );
        // Rotating 33 times is the identity.
        let (value, carry) = (0xdead_beefu32, true);
        let mut ring = (value, carry);
        for _ in 0..33 {
            ring = a88_rotate_right_through_carry(ring.0, ring.1, 1);
        }
        assert_eq!(ring, (value, carry));
    }

    // -- 0x00643b10 / 0x00643b30 / 0x00643f40 / 0x00643fe0 ----------------

    #[test]
    fn region_snapshot_only_copies_a_valid_rect() {
        let mut node = A88RegionNode {
            rect: [11, 22, 33, 44],
            ..A88RegionNode::default()
        };
        assert_eq!(
            a88_region_snapshot(&node, [1, 2, 3, 4]),
            A88RegionSnapshot {
                valid: 0,
                rect: [1, 2, 3, 4]
            }
        );
        node.valid = 1;
        assert_eq!(
            a88_region_snapshot(&node, [1, 2, 3, 4]),
            A88RegionSnapshot {
                valid: 1,
                rect: [11, 22, 33, 44]
            }
        );
    }

    #[test]
    fn region_scratch_zeroes_three_words_and_keeps_the_probe() {
        let scratch = a88_region_scratch([0x10, 0x20]);
        assert_eq!(scratch.cleared, [0; 3]);
        assert_eq!(scratch.probe, [0x10, 0x20]);
        assert_eq!(scratch.string, a88_sso_string_init());
    }

    #[test]
    fn region_marshal_forwards_point_then_keys() {
        let node = A88RegionNode {
            key_08: 0xaa,
            key_0c: 0xbb,
            ..A88RegionNode::default()
        };
        assert_eq!(
            a88_region_marshal_point([0x11, 0x22], &node),
            [0x11, 0x22, 0xaa, 0xbb]
        );
    }

    #[test]
    fn region_rect_store_swaps_or_installs() {
        let mut node = A88RegionNode::default();
        let stored = a88_region_rect_store(&mut node, [1, 2, 3, 4]);
        assert!(!stored.swapped);
        assert_eq!(stored.buffer, [1, 2, 3, 4]);
        assert_eq!(stored.valid, 1);
        assert_eq!(node.rect, [1, 2, 3, 4]);
        assert_eq!(node.valid, 1);

        let stored = a88_region_rect_store(&mut node, [5, 6, 7, 8]);
        assert!(stored.swapped);
        assert_eq!(stored.buffer, [1, 2, 3, 4]);
        assert_eq!(node.rect, [5, 6, 7, 8]);
        assert_eq!(node.valid, 1);
    }

    // -- 0x00643860 / 0x00643910 / 0x006439eb / 0x00643aa0 ----------------

    fn region_node(index: u32) -> A88RegionNode {
        A88RegionNode {
            next: None,
            prev: None,
            key_08: index,
            key_0c: index + 1,
            key_10: index as i32,
            rect: [index; 4],
            valid: 1,
            byte_4d: 1,
            byte_4f: 1,
            byte_51: 1,
        }
    }

    #[test]
    fn region_list_reset_tears_down_the_old_tail() {
        let mut list = A88RegionList::a88_new();
        assert_eq!(list.a88_reset(), None);

        list.nodes = vec![region_node(0), region_node(1), region_node(2)];
        list.head = Some(0);
        list.tail = Some(2);
        list.count = 2;
        assert_eq!(list.a88_reset(), Some(2));
        assert_eq!(list.head, None);
        assert_eq!(list.tail, None);
        assert_eq!(list.count, 0);
        assert_eq!(list.nodes[2].valid, 0);
        assert_eq!(list.nodes[2].byte_4d, 0);
        assert_eq!(list.nodes[2].byte_4f, 0);
        assert_eq!(list.nodes[2].byte_51, 0);
        // The body of the torn-down node is untouched.
        assert_eq!(list.nodes[2].rect, [2; 4]);
    }

    #[test]
    fn region_clear_head_leaves_tail_and_count() {
        let mut list = A88RegionList::a88_new();
        list.head = Some(0);
        list.tail = Some(1);
        list.count = 2;
        list.a88_clear_head();
        assert_eq!(list.head, None);
        assert_eq!(list.tail, Some(1));
        assert_eq!(list.count, 2);
    }

    #[test]
    fn region_unlink_tail_rewires_neighbours_and_clears_bytes() {
        let mut list = A88RegionList::a88_new();
        list.nodes = vec![region_node(0), region_node(1), region_node(2)];
        list.nodes[0].next = Some(1);
        list.nodes[0].prev = None;
        list.nodes[1].next = Some(2);
        list.nodes[1].prev = Some(0);
        list.nodes[2].next = None;
        list.nodes[2].prev = Some(1);
        list.head = Some(0);
        list.tail = Some(2);

        let unlink = list.a88_unlink_tail();
        assert_eq!(
            unlink,
            A88RegionUnlink {
                removed: Some(2),
                destructor_ran: true
            }
        );
        assert_eq!(list.head, Some(0));
        assert_eq!(list.tail, Some(1));
        assert_eq!(list.nodes[1].next, None);
        assert_eq!(list.nodes[2].valid, 0);

        // Unlinking the last remaining node leaves both links empty.
        let mut list = A88RegionList::a88_new();
        list.nodes = vec![region_node(0), region_node(1)];
        list.head = Some(1);
        list.tail = Some(1);
        assert_eq!(list.a88_unlink_tail().removed, Some(1));
        assert_eq!(list.head, None);
        assert_eq!(list.tail, None);
        assert_eq!(list.a88_unlink_tail().removed, None);
    }

    #[test]
    fn region_null_gate() {
        assert!(!a88_region_list_present(0));
        assert!(a88_region_list_present(1));
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// The 14 rows not translated above, grouped by why:
//
// * `FUN_00642203`, `FUN_0064221e`, `FUN_00642236`, `FUN_00642253`, `FUN_00642291`,
//   `FUN_006423e9`: entries into the shared AI-script dispatch.  Each body is
//   the same `switch (*cursor++)` loop entered at a different case label and
//   addresses its state through the caller's `ESI`/`EDI`/`EBP` (`in_AX`,
//   `unaff_ESI`, `unaff_EDI`, `puRam00fc9b6c`).  The only closed operation
//   they share is the cursor fetch translated at `0x00642037`; the case
//   bodies themselves call out-of-batch engine helpers with caller-frame
//   operands.
// * `FUN_00642833`: the decompiler reports overlapping instructions
//   (`0x0064292e` over `0x0064292d`), an unrecovered jump table and a lost
//   stack base; every store is `CONCAT`-chained bad-instruction data.
// * `FUN_00642c60`: stack pointer lost, `swi(0x80)` and `rdtsc` garbage; the
//   only legible call is `FUN_006439eb` (translated at its own row).
// * `FUN_006430eb`: `__break`, `halt_baddata` and `rdtsc` garbage around one
//   `FUN_006439eb` call; no recoverable state.
// * `FUN_00643459`: privileged `out`/`swi(3)` instructions and
//   bad-instruction data; no engine arithmetic.
// * `FUN_006434b0`, `FUN_00643519`: catch funclets that restore
//   `ExceptionList` and enter `FUN_00c86b6e`; the only arithmetic is a
//   `rdtsc`-derived pointer in unreachable code.
// * `FUN_00643749`: three stores against the caller's frame
//   (`*(u8 *)(ESI + *EDI) = 0`, `EDI[1] = ESI`, `*piVar1 - *EDI`), where
//   `ESI` is used as a byte offset and `EDI`/`piVar1` are caller nodes; the
//   row does not define a self-contained operation.
// * `FUN_00643920`: SEH scope-table teardown; its only constant is the `-1`
//   scope index pushed onto the stack.
