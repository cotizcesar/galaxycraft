#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave11/agent04.rs
// ===========================================================================
// Wave 11 translation batch 04; temporary staging file.
// Merged into the natural module after the wave.
//
// Batch 04 owns `0x0064e000..=0x0064ffff`.  The ledger lists 50
// `decompiled_uncited` rows in that window; 23 of them are translated below
// and the remaining 27 are listed at the end with their reason.
//
// The window is the client's **unit strength value** model.  It keeps two
// `0xe4`-entry dword tables side by side:
//
// | table       | base        | built from (`0x0064f940`)                     |
// |-------------|-------------|-----------------------------------------------|
// | slot 0      | `0x00fcd7c8`| the `0x00f8f750` weapon column (air weapon)    |
// | slot 1      | `0x00fcdb58`| the `0x00f8f580` weapon column (ground weapon) |
//
// `0x00fcd7c8 + 0xe4 * 4 = 0x00fcdb58`, so the two getters (`0x0064f870`,
// `0x0064f880`) and the slot selector in `0x0064f720` are one array pair,
// not two unrelated globals.
//
// The per-type inputs are the recovered runtime tables:
//
// | table       | width | meaning                                        |
// |-------------|-------|------------------------------------------------|
// | `0x00f8e800`| byte  | shield flag (`FUN_005b68a0`)                     |
// | `0x00f8e8e8`| u16   | shield points (`FUN_005b4b20`)                   |
// | `0x00f8eab0`| i32   | raw maximum HP (`1/256` points)                |
// | `0x00f8df38`| u16   | turret row used for the weapon columns         |
// | `0x00f8f750`| byte  | air weapon id (`0x82` = no weapon)             |
// | `0x00f8f580`| byte  | ground weapon id (`0x82` = no weapon)          |
//
// and the `weapons.dat` runtime columns read by `0x0064f120`:
// `0x00f94110` damage, `0x00f94320` cooldown, `0x00f943a8` factor (bullet
// count) and `0x00f93948` maximum range.  The value is scaled by the two
// doubles at `0x00d3cf88` (758.0) and `0x00d33be8` (100.0), i.e. by 7.58.
//
// Address to function:
//
// * `0x0064efb0` [`w11a04_copy_printable`]
// * `0x0064eff0` [`w11a04_copy_printable_ws`]
// * `0x0064f030` [`W11a04CachedPair::w11a04_cache_slot_values`]
// * `0x0064f060` [`w11a04_scale_value`]
// * `0x0064f120` [`w11a04_weapon_value`]
// * `0x0064f5f0` [`w11a04_enemy_value_sum`]
// * `0x0064f720` [`w11a04_slot_value`]
// * `0x0064f790` [`w11a04_unit_value`]
// * `0x0064f840` / `0x0064f910` [`w11a04_order4_dispatch`]
// * `0x0064f870` / `0x0064f880` [`w11a04_table_value`]
// * `0x0064f8d0` [`w11a04_enemy_value_sum_airborne`]
// * `0x0064f8f0` [`w11a04_enemy_value_sum_ground`]
// * `0x0064f940` [`w11a04_build_value_tables`]
// * `0x0064fa60` [`W11a04CachedPair::w11a04_field_138`]
// * `0x0064fa70` [`W11a04CachedPair::w11a04_field_13a`]
// * `0x0064fa80` [`w11a04_record_block`] (null branch `0x0064fabc`)
// * `0x0064fad0` [`w11a04_release_block`]
// * `0x0064fae1` [`w11a04_empty_hook`]
// * `0x0064faf0` [`w11a04_unlink_and_push`]
// * `0x0064fb60` [`w11a04_guard_chain`]
//
// Every public item is prefixed with the batch tag: functions and constants
// use `w11a04_`, the helper types use the type-case form `W11a04...` (a
// lowercase type name would be a `non_camel_case_types` warning).
//
// No `World`/`SimUnit` field is added and no other module is touched; the
// retail getters enter these functions as explicit values.

// ===========================================================================
// Text sanitation
// ===========================================================================

/// `0x0064efb0` (`FUN_0064efb0`, 49 bytes, `decompiled_uncited`).
///
/// ```c
/// if (param_3 != 0) {
///   bVar1 = *param_2;
///   while (bVar1 != 0 && param_3 != 0) {
///     if (0x1f < bVar1) { *param_1 = bVar1; param_1++; param_3--; }
///     bVar1 = param_2[1]; param_2++;
///   }
///   *param_1 = 0;
/// }
/// ```
///
/// The client's "plain text" filter: copy every source byte above `0x1f`
/// (so tab, CR, LF and all other controls are dropped) until the source NUL
/// or until the destination is full, then always store one NUL.  A zero
/// `dst.len()` stores nothing at all, matching the `param_3 != 0` guard.
///
/// Retail counts *stored* characters and writes the terminator one past a
/// destination that filled exactly; the translation reserves the last byte
/// for the terminator, so it copies at most `dst.len() - 1` characters.
/// Returns the number of characters copied.
pub fn w11a04_copy_printable(src: &[u8], dst: &mut [u8]) -> usize {
    if dst.is_empty() {
        return 0;
    }
    let mut written = 0;
    for &byte in src {
        if byte == 0 || written + 1 >= dst.len() {
            break;
        }
        if byte > 0x1f {
            dst[written] = byte;
            written += 1;
        }
    }
    dst[written] = 0;
    written
}

/// `0x0064eff0` (`FUN_0064eff0`, 62 bytes, `decompiled_uncited`).
///
/// ```c
/// if (param_3 != 0) {
///   bVar1 = *param_2;
///   while (bVar1 != 0 && 1 < param_3) {
///     if ((0x1f < bVar1) || bVar1 == 10 || bVar1 == 13 || bVar1 == 9) {
///       *param_1 = bVar1; param_1++; param_3--;
///     }
///     bVar1 = param_2[1]; param_2++;
///   }
///   *param_1 = 0;
/// }
/// ```
///
/// The multi-line text filter used by the same string table: it keeps the
/// printable range plus tab (`0x09`), LF (`0x0a`) and CR (`0x0d`) and drops
/// every other control byte.  The loop needs more than one byte of room, so
/// the terminator always fits; the translation copies at most
/// `dst.len() - 1` characters and stores the NUL.  Returns the number of
/// characters copied.
pub fn w11a04_copy_printable_ws(src: &[u8], dst: &mut [u8]) -> usize {
    if dst.is_empty() {
        return 0;
    }
    let mut written = 0;
    for &byte in src {
        if byte == 0 || written + 1 >= dst.len() {
            break;
        }
        if byte > 0x1f || matches!(byte, 0x09 | 0x0a | 0x0d) {
            dst[written] = byte;
            written += 1;
        }
    }
    dst[written] = 0;
    written
}

// ===========================================================================
// Per-type value scaling
// ===========================================================================

/// `0x0064f060` (`FUN_0064f060`, 33 bytes, `decompiled_uncited`).
///
/// The type-specific multiplier applied to a weapon value before it enters
/// the slot table.  The switch is exhaustive over the special rows and
/// returns the value unchanged for every other type:
///
/// | types                     | result          |
/// |---------------------------|-----------------|
/// | `0x07`, `0x29`, `0x40`    | `value >> 2`    |
/// | `0x0d`, `0x49`, `0x55`    | `0`             |
/// | `0x20`, `0x2b`, `0x41`    | `value * 2`     |
/// | `0x2f`, `0x32`            | `value >> 4`    |
/// | `0x53`                    | `value / 10`    |
/// | everything else           | `value`         |
///
/// The doubling and the shifts wrap in 32 bits.
pub fn w11a04_scale_value(type_id: u16, value: u32) -> u32 {
    match type_id {
        0x07 | 0x29 | 0x40 => value >> 2,
        0x0d | 0x49 | 0x55 => 0,
        0x20 | 0x2b | 0x41 => value.wrapping_mul(2),
        0x2f | 0x32 => value >> 4,
        0x53 => value / 10,
        _ => value,
    }
}

// ===========================================================================
// Weapon value (`0x0064f120`)
// ===========================================================================

/// `weapons.dat` row consumed by [`w11a04_weapon_value`], with the runtime
/// column addresses it is read from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W11a04WeaponRow {
    /// `0x00f94110`: damage.
    pub damage: u16,
    /// `0x00f94320`: cooldown.
    pub cooldown: u8,
    /// `0x00f943a8`: damage factor / bullet count.
    pub factor: u8,
    /// `0x00f93948`: maximum range.
    pub range: u32,
}

/// `units.dat` row consumed by [`w11a04_weapon_value`] and
/// [`w11a04_build_value_tables`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W11a04UnitRow {
    /// `0x00f8e800` byte: the type has shields (`FUN_005b68a0`).
    pub has_shields: bool,
    /// `0x00f8e8e8` word: maximum shield points (`FUN_005b4b20`).
    pub shield_points: u16,
    /// `0x00f8eab0` dword: raw maximum HP in `1/256` points.
    pub hp_raw: i32,
}

impl W11a04UnitRow {
    /// The `base` operand of `0x0064f120`: whole-point maximum HP plus the
    /// shield pool.  Retail shifts the raw dword arithmetically by eight and
    /// adds the word only when the shield flag byte is set.
    pub fn w11a04_base_points(&self) -> i32 {
        let hp = self.hp_raw >> 8;
        let shields = if self.has_shields {
            i32::from(self.shield_points)
        } else {
            0
        };
        hp + shields
    }
}

/// `0x0064f120` (`FUN_0064f120`, 198 bytes, `decompiled_uncited`).
///
/// ```c
/// uVar1 = DAT_00f8e800[type] ? DAT_00f8e8e8[type] : 0;
/// base  = (DAT_00f8eab0[type] >> 8) + uVar1;
/// if (base == 0) return 0;
/// range = DAT_00f93948[weapon];
/// dmg   = DAT_00f94110[weapon];
/// cd    = DAT_00f94320[weapon];
/// fac   = DAT_00f943a8[weapon];
/// v = (range / cd) * fac * dmg;
/// if (weapon == 0x82) extra = 0;
/// else extra = (dmg * fac * 0x800) / cd;
/// value = ((extra * base) >> 8) + v;
/// return (u32)((double)value * 758.0 / 100.0);
/// ```
///
/// The double conversion is the MSVC unsigned-to-double sequence
/// (`cvtdq2pd` plus the `2^32` fix-up) followed by the `FUN_00c94230` guard and
/// the `FUN_00c86ff1` double-to-integer helper; the model performs the same
/// multiply-then-divide and truncates.  Weapon `0x82` is "no weapon": the
/// built-in burst term is dropped, but the range term still counts.
///
/// A zero cooldown makes retail divide by zero; the translation returns `0`
/// for that (and for a zero `base`) so the arithmetic is defined.
pub fn w11a04_weapon_value(unit: W11a04UnitRow, weapon_id: u8, weapon: W11a04WeaponRow) -> u32 {
    let base = unit.w11a04_base_points();
    if base == 0 {
        return 0;
    }
    let cooldown = u32::from(weapon.cooldown);
    if cooldown == 0 {
        return 0;
    }
    let damage = u32::from(weapon.damage);
    let factor = u32::from(weapon.factor);

    let range_term = (weapon.range / cooldown)
        .wrapping_mul(factor)
        .wrapping_mul(damage);
    let burst_term = if weapon_id == W11A04_NO_WEAPON {
        0
    } else {
        damage.wrapping_mul(factor).wrapping_mul(0x800) / cooldown
    };
    let scaled = ((burst_term as i32).wrapping_mul(base) as u32) >> 8;
    let value = scaled.wrapping_add(range_term);

    (f64::from(value) * W11A04_VALUE_NUMERATOR / W11A04_VALUE_DENOMINATOR) as u32
}

// ===========================================================================
// Per-type slot tables and the value getters
// ===========================================================================

/// Number of `units.dat` type rows the score tables hold (`0xe4`).
pub const W11A04_TYPE_COUNT: usize = 0xe4;

/// Weapon id `0x82`: the type has no weapon in that column.
pub const W11A04_NO_WEAPON: u8 = 0x82;

/// Multiplier numerator at `0x00d3cf88` (`mulsd` operand, `758.0`).
pub const W11A04_VALUE_NUMERATOR: f64 = 758.0;

/// Multiplier denominator at `0x00d33be8` (`divsd` operand, `100.0`).
pub const W11A04_VALUE_DENOMINATOR: f64 = 100.0;

/// The two `0xe4`-dword value tables written by `0x0064f940`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W11a04ValueTables {
    /// Slot 0, the dwords at `0x00fcd7c8` (built from `0x00f8f750`).
    pub slot0: Vec<u32>,
    /// Slot 1, the dwords at `0x00fcdb58` (built from `0x00f8f580`).
    pub slot1: Vec<u32>,
}

impl W11a04ValueTables {
    /// A pair of zeroed `0xe4`-entry tables, the extent the retail builder
    /// always writes.
    pub fn w11a04_new() -> Self {
        Self {
            slot0: vec![0; W11A04_TYPE_COUNT],
            slot1: vec![0; W11A04_TYPE_COUNT],
        }
    }

    /// The table selected by the slot argument shared by `0x0064f720`,
    /// `0x0064f790` and `0x0064f5f0` (`0` -> `0x00fcd7c8`, `1` ->
    /// `0x00fcdb58`).  Retail would read adjacent memory for any other
    /// value; the model reports it as absent instead.
    pub fn w11a04_slot(&self, slot: usize) -> Option<&[u32]> {
        match slot {
            0 => Some(&self.slot0),
            1 => Some(&self.slot1),
            _ => None,
        }
    }
}

/// `0x0064f870` (`FUN_0064f870`, 15 bytes, `decompiled_uncited`) and
/// `0x0064f880` (`FUN_0064f880`, 15 bytes, `decompiled_uncited`).
///
/// ```c
/// return (&DAT_00fcd7c8)[param_1];   // 0x0064f870, slot 0
/// return (&DAT_00fcdb58)[param_1];   // 0x0064f880, slot 1
/// ```
///
/// The two getters are the same dword read from the table the caller picks:
/// [`W11a04ValueTables::slot0`] for `0x0064f870` and
/// [`W11a04ValueTables::slot1`] for `0x0064f880`.  A type id past the table
/// is `None` (retail would read whatever follows the array).
pub fn w11a04_table_value(table: &[u32], type_id: u16) -> Option<u32> {
    table.get(usize::from(type_id)).copied()
}

/// `0x0064f720` (`FUN_0064f720`, 16 bytes, `decompiled_uncited` but the body
/// spans `0x0064f720..=0x0064f78c`).
///
/// ```c
/// type = *(u16 *)(unit + 0x64);
/// value = table[type];                       // one slot's 0xe4 dwords
/// if (type == 0x7d) value = FUN_005d5ea0() * table[0];
/// if (FUN_005b6b20()) value += FUN_005b4af0() >> 1;
/// return value;
/// ```
///
/// The type-`0x7d` row has its own rule: its value is the count returned by
/// the `FUN_005d5ea0` helper times the slot's own row `0` entry.  The energy
/// bonus adds half of the `CUnit+0xa3` byte when the `FUN_005b6b20` energy flag
/// is set; `energy` is `Some(byte)` exactly then, so the flag and the value
/// travel together.
pub fn w11a04_slot_value(
    table: &[u32],
    type_id: u16,
    carried: u32,
    energy: Option<u8>,
) -> Option<u32> {
    let mut value = w11a04_table_value(table, type_id)?;
    if type_id == 0x7d {
        value = carried.wrapping_mul(*table.first()?);
    }
    if let Some(energy) = energy {
        value = value.wrapping_add(u32::from(energy >> 1));
    }
    Some(value)
}

/// The per-unit inputs of the two value getters, with the helper that
/// supplies each one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W11a04UnitState {
    /// `CUnit+0x64`, the `units.dat` type id.
    pub type_id: u16,
    /// The function's own `units.dat` row (shield flag, shields, raw HP).
    pub row: W11a04UnitRow,
    /// `FUN_005d5ea0` (read only for type `0x7d`).
    pub carried: u32,
    /// `FUN_005b4af0`'s byte when `FUN_005b6b20` is set.
    pub energy: Option<u8>,
    /// `FUN_005b4aa0`: whole-point current HP plus shields.
    pub current_points: i32,
    /// `FUN_005b6560`: whole-point maximum HP plus shields.
    pub max_points: i32,
    /// `FUN_005b5d00`: `CUnit+0xdc` bit 30, the dying flag.
    pub dying: bool,
    /// `FUN_005b4b30`: maximum HP in whole points.
    pub max_hp_points: i32,
}

/// `0x0064f790` (`FUN_0064f790`, 16 bytes, `decompiled_uncited` but the body
/// spans `0x0064f790..=0x0064f834`).
///
/// ```c
/// type = *(u16 *)(unit + 0x64);
/// if (type == 0x23 || type == 0x24 || type == 0x3b || type == 0x61) return 0;
/// if (FUN_005b5d00() && FUN_005b4aa0() < FUN_005b6560()) return 0;
/// value = FUN_0064f720(unit, slot);
/// s = FUN_005b68a0() ? FUN_005b4b20() : 0;
/// d = FUN_005b4b30();
/// divisor = (s + d != 0) ? s + d : 1;
/// return FUN_005b4aa0() * value / divisor;
/// ```
///
/// The four zero rows are types `0x23`, `0x24`, `0x3b` and `0x61`; a dying
/// unit below its maximum is worth nothing.  Otherwise the slot value is
/// scaled by the unit's current share of its whole-point HP plus shields.
/// The product wraps and the division is unsigned, exactly like the shipped
/// `imul`/`div` pair.
pub fn w11a04_unit_value(table: &[u32], unit: &W11a04UnitState) -> Option<u32> {
    if matches!(unit.type_id, 0x23 | 0x24 | 0x3b | 0x61) {
        return Some(0);
    }
    if unit.dying && unit.current_points < unit.max_points {
        return Some(0);
    }
    let value = w11a04_slot_value(table, unit.type_id, unit.carried, unit.energy)?;
    let shields = if unit.row.has_shields {
        i32::from(unit.row.shield_points)
    } else {
        0
    };
    let divisor = shields.wrapping_add(unit.max_hp_points);
    if divisor == 0 {
        return Some((unit.current_points as u32).wrapping_mul(value));
    }
    Some(
        (unit.current_points as u32)
            .wrapping_mul(value)
            .wrapping_div(divisor as u32),
    )
}

/// One candidate of the `0x0064f5f0` area scan.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W11a04Candidate {
    /// The unit's value inputs.
    pub unit: W11a04UnitState,
    /// `FUN_005b5c70`: the dynamic air bit (`CUnit+0xdc` bit 2).
    pub airborne: bool,
    /// `FUN_005b5b90` negated: the candidate is **not** allied with the
    /// searching owner.
    pub enemy: bool,
}

/// `0x0064f5f0` (`FUN_0064f5f0`, 22 bytes, `decompiled_uncited` but the body
/// spans `0x0064f5f0..=0x0064f719`).
///
/// ```c
/// list = FUN_005a5790(position);              // spatial-cell unit list
/// sum = 0;
/// for (unit in list) {
///   air = FUN_005b5c70(unit);
///   if (air != airborne_flag) continue;       // airborne bit must match
///   if (!FUN_005b5b90(player, unit)) continue; // keep only enemies
///   inline = FUN_0064f720(unit, 0) scaled like 0x0064f790;
///   other  = FUN_0064f790(unit, 1);
///   sum += max(inline, other);
/// }
/// ```
///
/// The scan sums the stronger of the two slot values of every enemy in the
/// cell whose airborne bit matches the caller's flag.  The grid list and the
/// alliance table are caller inputs here: `candidates` carries the
/// `FUN_005b5c70` bit and the inverted `FUN_005b5b90` answer, and the retail
/// 32-bit accumulator is mirrored with wrapping addition.
///
/// Retail computes the slot-0 value inline (a duplicate of
/// [`w11a04_unit_value`]); the translation calls the shared function, which
/// yields the same number.
pub fn w11a04_enemy_value_sum(
    airborne: bool,
    candidates: &[W11a04Candidate],
    tables: &W11a04ValueTables,
) -> u32 {
    let mut sum = 0u32;
    for candidate in candidates {
        if candidate.airborne != airborne || !candidate.enemy {
            continue;
        }
        let slot0 = w11a04_unit_value(&tables.slot0, &candidate.unit).unwrap_or(0);
        let slot1 = w11a04_unit_value(&tables.slot1, &candidate.unit).unwrap_or(0);
        sum = sum.wrapping_add(slot0.max(slot1));
    }
    sum
}

/// `0x0064f8d0` (`FUN_0064f8d0`, 21 bytes, `decompiled_uncited`).
///
/// ```c
/// FUN_0064f5f0(param_1, param_2, 1);
/// ```
///
/// The airborne-only wrapper around [`w11a04_enemy_value_sum`].
pub fn w11a04_enemy_value_sum_airborne(
    candidates: &[W11a04Candidate],
    tables: &W11a04ValueTables,
) -> u32 {
    w11a04_enemy_value_sum(true, candidates, tables)
}

/// `0x0064f8f0` (`FUN_0064f8f0`, 21 bytes, `decompiled_uncited`).
///
/// ```c
/// FUN_0064f5f0(param_1, param_2, 0);
/// ```
///
/// The ground-only wrapper around [`w11a04_enemy_value_sum`].
pub fn w11a04_enemy_value_sum_ground(
    candidates: &[W11a04Candidate],
    tables: &W11a04ValueTables,
) -> u32 {
    w11a04_enemy_value_sum(false, candidates, tables)
}

/// `0x0064f840` (`FUN_0064f840`, 40 bytes) and `0x0064f910`
/// (`FUN_0064f910`, 40 bytes), both `decompiled_uncited`.
///
/// ```c
/// if (*(u8 *)(*(u32 *)(unit + 0x134) + 8) != 4) {
///   FUN_0064f580(unit, 0);   // 0x0064f840
///   return;
/// }
/// tail_call FUN_00654520();
/// ```
///
/// and the same body with flag `1` and tail call `FUN_00654560` at
/// `0x0064f910`.  The two routines are one dispatch on the order-state byte
/// (`CUnit+0x134` -> `+8`): every state except `4` runs the "find a nearby
/// target" routine with the address' own flag, and state `4` hands the unit
/// to the secondary routine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W11a04Order4Action {
    /// Order state differs from `4`: run `FUN_0064f580(unit, flag)`.
    SelectNear {
        /// `0x0064f910` passes `true`, `0x0064f840` passes `false`.
        flag: bool,
    },
    /// Order state `4`: tail-call `FUN_00654520` / `FUN_00654560`.
    Secondary,
}

/// `0x0064f840` / `0x0064f910`: the order-state dispatch described on
/// [`W11a04Order4Action`].
pub fn w11a04_order4_dispatch(order_state: u8, flag: bool) -> W11a04Order4Action {
    if order_state == 4 {
        W11a04Order4Action::Secondary
    } else {
        W11a04Order4Action::SelectNear { flag }
    }
}

/// One `units.dat` row of the `0x0064f940` builder: the value inputs plus the
/// three columns that select the weapon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W11a04TableRow {
    /// Shield flag, shield points and raw maximum HP.
    pub unit: W11a04UnitRow,
    /// `0x00f8df38` word: the turret row whose weapon columns are used.
    pub turret_row: u16,
    /// `0x00f8f750` byte: the slot-0 (air) weapon id.
    pub weapon_slot0: u8,
    /// `0x00f8f580` byte: the slot-1 (ground) weapon id.
    pub weapon_slot1: u8,
}

/// `0x0064f940` (`FUN_0064f940`, 285 bytes, `decompiled_uncited`), also
/// reached through the `FUN_0063cc90` jump thunk.
///
/// The builder loops over all `0xe4` type rows and fills the two slot
/// tables.  For each slot:
///
/// 1. Types `0x23`, `0x24`, `0x3b` and `0x61` get `0`.
/// 2. The weapon is read from the *turret* row: `0x48`/`0x52` use row `0x49`,
///    `0x51`/`0x53` use row `0x55`, and every other type uses
///    `0x00f8df38[type]` unless that word is `0xe4`, which means "use the
///    type's own row".
/// 3. Weapon `0x82` (no weapon) produces `1`; every other weapon produces
///    [`w11a04_scale_value`] of [`w11a04_weapon_value`] for the *type's* row
///    and the selected weapon.
/// 4. After both slots are known, a slot that is exactly `1` is cleared to
///    `0` when the other slot is greater than `1` - a unit that really has a
///    weapon in one class must not also report the weaponless placeholder in
///    the other.
///
/// The value function is always fed the type's own `units.dat` row (`t`),
/// not the turret row that selected the weapon.  `weapons` is the
/// `weapons.dat` table indexed by weapon id; a missing row yields `0`, and
/// types past `rows.len()` stay zero (retail always runs the full `0xe4`).
pub fn w11a04_build_value_tables(
    rows: &[W11a04TableRow],
    weapons: &[W11a04WeaponRow],
) -> W11a04ValueTables {
    let mut tables = W11a04ValueTables::w11a04_new();
    let count = rows.len().min(W11A04_TYPE_COUNT);
    for type_index in 0..count {
        let first = w11a04_build_entry(rows, weapons, type_index, true);
        let second = w11a04_build_entry(rows, weapons, type_index, false);
        let slot0 = if first == 1 && second > 1 { 0 } else { first };
        let slot1 = if second == 1 && first > 1 { 0 } else { second };
        tables.slot0[type_index] = slot0;
        tables.slot1[type_index] = slot1;
    }
    tables
}

/// The turret row used for the weapon columns of type row `type_index`.
fn w11a04_weapon_row(rows: &[W11a04TableRow], type_index: usize) -> usize {
    match type_index as u16 {
        0x48 | 0x52 => 0x49,
        0x51 | 0x53 => 0x55,
        _ => {
            let row = usize::from(rows[type_index].turret_row);
            if row == W11A04_TYPE_COUNT {
                type_index
            } else {
                row
            }
        }
    }
}

/// One slot of one type row for [`w11a04_build_value_tables`].
fn w11a04_build_entry(
    rows: &[W11a04TableRow],
    weapons: &[W11a04WeaponRow],
    type_index: usize,
    first_pass: bool,
) -> u32 {
    if matches!(type_index as u16, 0x23 | 0x24 | 0x3b | 0x61) {
        return 0;
    }
    let weapon_row = w11a04_weapon_row(rows, type_index);
    let Some(row) = rows.get(weapon_row) else {
        return 0;
    };
    let weapon_id = if first_pass {
        row.weapon_slot0
    } else {
        row.weapon_slot1
    };
    if weapon_id == W11A04_NO_WEAPON {
        return 1;
    }
    let Some(weapon) = weapons.get(usize::from(weapon_id)) else {
        return 0;
    };
    w11a04_scale_value(
        type_index as u16,
        w11a04_weapon_value(rows[type_index].unit, weapon_id, *weapon),
    )
}

// ===========================================================================
// Cached selection words (`0x0064fa60` / `0x0064fa70`)
// ===========================================================================

/// The two `u16` words a unit caches at `+0x138` and `+0x13a`.
///
/// `0x0064f030` fills them with the unit's own slot-0 and slot-1 values; the
/// getters `0x0064fa60`/`0x0064fa70` read them back.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W11a04CachedPair {
    /// `+0x138`, read by `0x0064fa60`.
    pub field_138: u16,
    /// `+0x13a`, read by `0x0064fa70`.
    pub field_13a: u16,
}

impl W11a04CachedPair {
    /// `0x0064f030` (`FUN_0064f030`, 38 bytes, `decompiled_uncited`).
    ///
    /// The decompiled body prints both `0x0064f790` calls without arguments,
    /// but the disassembly pushes them explicitly:
    ///
    /// ```text
    /// 0064f031: mov  %ecx,%esi          ; this = unit
    /// 0064f033: push $0x0               ; slot 0
    /// 0064f035: push %esi               ; the unit itself
    /// 0064f036: call 0x0064f790
    /// 0064f03b: push $0x1               ; slot 1
    /// 0064f03d: push %esi
    /// 0064f03e: mov  %ax,0x138(%esi)    ; first result
    /// 0064f045: call 0x0064f790
    /// 0064f04d: mov  %ax,0x13a(%esi)    ; second result
    /// ```
    ///
    /// So the unit caches its own two slot values, truncated to 16 bits.  A
    /// type past the supplied table would read adjacent memory in retail; the
    /// model stores `0` there.
    pub fn w11a04_cache_slot_values(&mut self, unit: &W11a04UnitState, tables: &W11a04ValueTables) {
        self.field_138 = w11a04_unit_value(&tables.slot0, unit).unwrap_or(0) as u16;
        self.field_13a = w11a04_unit_value(&tables.slot1, unit).unwrap_or(0) as u16;
    }

    /// `0x0064fa60` (`FUN_0064fa60`, 15 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// return *(u16 *)(param_1 + 0x138);
    /// ```
    pub fn w11a04_field_138(&self) -> u16 {
        self.field_138
    }

    /// `0x0064fa70` (`FUN_0064fa70`, 15 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// return *(u16 *)(param_1 + 0x13a);
    /// ```
    pub fn w11a04_field_13a(&self) -> u16 {
        self.field_13a
    }
}

// ===========================================================================
// 0x34-byte record blocks (`0x0064fa80`, null branch `0x0064fabc`,
// release `0x0064fad0`)
// ===========================================================================

/// Size of one record in the array built by `0x0064fa80` (`0x34` bytes).
pub const W11A04_RECORD_BYTES: u32 = 0x34;

/// The fixed size operand `0x0064fad0` passes to the CRT release routine
/// (`0x4e24`).
pub const W11A04_RELEASE_OPERAND: u32 = 0x4e24;

/// `0x0064fa80` (`FUN_0064fa80`, 30 bytes, `decompiled_uncited`), with the
/// null branch `0x0064fabc` (`FUN_0064fabc`, 13 bytes) folded in.
///
/// ```c
/// size = param_2 * 0x34;                       // unsigned mul, OF -> 0xffffffff
/// block = malloc(size);
/// if (block != 0) { memset(block, 0, size); *param_1 = block; }
/// else { *param_1 = 0; }                        // 0x0064fabc
/// ```
///
/// `size` is computed with an unsigned 32-bit `mul`; the overflow path sets
/// `size` to `0xffffffff`, which cannot be a record block, so the model
/// answers `None` (the `0x0064fabc` null store).  A successful block is
/// zeroed.  A zero count produces an empty block, matching a `malloc(0)` that
/// succeeds.
pub fn w11a04_record_block(count: u32) -> Option<Vec<[u8; 0x34]>> {
    count.checked_mul(W11A04_RECORD_BYTES)?;
    Some(vec![[0u8; 0x34]; count as usize])
}

/// `0x0064fad0` (`FUN_0064fad0`, 17 bytes, `decompiled_uncited`).
///
/// ```c
/// if (*in_ECX != 0) FUN_00923e90(*in_ECX, 0x4e24);
/// ```
///
/// The block's destructor: release the buffer when present (the `0x4e24`
/// operand is preserved in [`W11A04_RELEASE_OPERAND`]) and leave the slot
/// null.  Releasing an already-null slot is a no-op.
pub fn w11a04_release_block(block: &mut Option<Vec<[u8; 0x34]>>) {
    *block = None;
}

/// `0x0064fae1` (`FUN_0064fae1`, 4 bytes, `decompiled_uncited`).
///
/// ```c
/// return;
/// ```
///
/// An empty destructor body: the row is a single `ret`, so the translation is
/// the no-op hook callers can name instead of an empty block.
pub fn w11a04_empty_hook() {}

// ===========================================================================
// Intrusive list move (`0x0064faf0`)
// ===========================================================================

/// One node of the intrusive list moved by `0x0064faf0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W11a04LinkNode {
    /// `+0x00`: next node.
    pub next: Option<u32>,
    /// `+0x04`: previous node.
    pub prev: Option<u32>,
    /// `+0x0c`: the owning unit.  Retail runs `FUN_005b5c70` on it; the model
    /// takes that answer as the `airborne` argument of
    /// [`w11a04_unlink_and_push`].
    pub unit: u32,
}

/// The head/count pair an owner keeps for the source list (`+0x30` and
/// `+0x0a`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W11a04ListHead {
    /// `+0x30`: first node.
    pub head: Option<u32>,
    /// `+0x0a`: `i16` node count.
    pub count: i16,
}

/// `0x0064faf0` (`FUN_0064faf0`, 104 bytes, `decompiled_uncited`).
///
/// ```c
/// if (FUN_005b5c70(node->unit) == 0) owner->count--;
/// if (owner->head == node) owner->head = node->next;
/// if (node->prev) node->prev->next = node->next;
/// if (node->next) node->next->prev = node->prev;
/// list = owner->list;              // +0x2c
/// node->prev = 0;
/// node->next = list->top;          // +0x4e20
/// if (list->top) list->top->prev = node;
/// list->top = node;                // push onto the destination chain
/// ```
///
/// The routine unlinks a node from the owner's list, decrements the count
/// only when the node's unit is **not** airborne, and pushes it onto the
/// destination chain kept at `+0x4e20` (`dst_top`).  The destination is a
/// LIFO chain: the new node's `next` is the old top and the old top's `prev`
/// points back at the new node.  Out-of-range node indices are ignored and
/// reported with `false`.
pub fn w11a04_unlink_and_push(
    nodes: &mut [W11a04LinkNode],
    source: &mut W11a04ListHead,
    dst_top: &mut Option<u32>,
    node: u32,
    airborne: bool,
) -> bool {
    let index = node as usize;
    if index >= nodes.len() {
        return false;
    }
    let (next, prev) = {
        let entry = &nodes[index];
        (entry.next, entry.prev)
    };
    if !airborne {
        source.count = source.count.wrapping_sub(1);
    }
    if source.head == Some(node) {
        source.head = next;
    }
    if let Some(prev) = prev {
        nodes[prev as usize].next = next;
    }
    if let Some(next) = next {
        nodes[next as usize].prev = prev;
    }
    nodes[index].prev = None;
    nodes[index].next = *dst_top;
    if let Some(top) = *dst_top {
        nodes[top as usize].prev = Some(node);
    }
    *dst_top = Some(node);
    true
}

// ===========================================================================
// Order guard chain (`0x0064fb60`)
// ===========================================================================

/// Inputs of the `0x0064fb60` guard chain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W11a04GuardInput {
    /// `CUnit+0x134` -> `+0x08`: the order-state byte (`None` when
    /// `CUnit+0x134` is null).
    pub order_state: Option<u8>,
    /// `CUnit+0x134` -> `+0x0c` equals the unit itself.
    pub order_target_is_self: bool,
    /// `FUN_005b5820`: the type carries `units.dat` building flag `0x1`.
    pub building: bool,
    /// `FUN_005b5970`: `CUnit+0xdc` bit 0, the ready/completed flag.
    pub flag_5970: bool,
    /// `CUnit+0x4d`: the order byte.
    pub order_byte: u8,
    /// `FUN_005b5680`: `CUnit+0xdc` bit 5.
    pub flag_5680: bool,
}

/// What the `0x0064fb60` guard chain does next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W11a04Guard {
    /// One of the early exits fires: the caller returns without work.
    Skip,
    /// The chain plays the `FUN_005d5810` cue and then falls into the shared
    /// tail at `0x0064fbe3`.
    CueThenContinue,
    /// Fall straight into the shared tail at `0x0064fbe3`.
    Continue,
}

/// `0x0064fb60` (`FUN_0064fb60`, 146 bytes, `decompiled_uncited`), the guard
/// chain in front of the shared tail at `0x0064fbe3`.
///
/// ```c
/// state = *(u32 *)(unit + 0x134);
/// if (state == 0) return;
/// if (*(u8 *)(state + 8) != 4) return;
/// if (*(u32 *)(state + 0xc) != unit) return;
/// if (FUN_005b5820()) return;
/// if (!FUN_005b5970()) return;
/// if (order == 0x6a || order == 0x96) return;
/// if (FUN_005b5680()) { FUN_005d5810(); goto tail; }
/// if (order == 0x62 || order == 0x06) return;
/// tail: ...
/// ```
///
/// The row ends at the call into the shared tail (`FUN_0064fbf2`, `FUN_0064fc55`
/// and `FUN_0064fcac` are its other entry points), so the translation reports
/// which way the chain leaves instead of modelling the tail.
pub fn w11a04_guard_chain(input: &W11a04GuardInput) -> W11a04Guard {
    if input.order_state != Some(4) {
        return W11a04Guard::Skip;
    }
    if !input.order_target_is_self {
        return W11a04Guard::Skip;
    }
    if input.building {
        return W11a04Guard::Skip;
    }
    if !input.flag_5970 {
        return W11a04Guard::Skip;
    }
    if matches!(input.order_byte, 0x6a | 0x96) {
        return W11a04Guard::Skip;
    }
    if input.flag_5680 {
        return W11a04Guard::CueThenContinue;
    }
    if matches!(input.order_byte, 0x62 | 0x06) {
        return W11a04Guard::Skip;
    }
    W11a04Guard::Continue
}

// ---------------------------------------------------------------------------
// Rows not translated
// ---------------------------------------------------------------------------
//
// * `FUN_0064e0d0` (195 bytes): MSVC C++ exception cleanup.  It builds a
//   string argument, opens an exception frame and calls `FUN_00538810` /
//   `FUN_0053a5b0`, both of which do not return.
//
// * `FUN_0064e270` (177 bytes): member routine that clears `+0x7c`, validates
//   two order records through `FUN_0064e330` and stores the parsed id.  The
//   records are pointers into the client's order list and every path is
//   wrapped in a `FUN_009778e0`-gated try block; there is no pure sub-result.
//
// * `FUN_0064e330` (194 bytes): the exception prologue/epilogue around the
//   string copy and the `FUN_0053ae90` throw split by the decompiler.
//
// * `FUN_0064e408` (`Catch@0064e408`, 6 bytes): a catch handler whose whole
//   body is the address constant `0x0064e40e`.
//
// * `FUN_0064e5ad` (23 bytes) and `FUN_0064e6bb` (31 bytes): unwind/cleanup
//   fragments that write exception registers and call a non-returning CRT
//   path.
//
// * `FUN_0064eb50` (231 bytes): member "revalidate orders" routine over
//   `+0x28..+0x7c`, restoring a three-word vector and re-running the
//   `FUN_0064e330` validator inside exception frames.  It needs the full client
//   order-record layout, which the value model does not have.
//
// * `FUN_0064ec3b` (183 bytes): the decompiler prints `(int)in_EAX * 9 +
//   0xa7840f` as a pointer and writes through it, then repeats the
//   `FUN_0064eb50` body with `unaff_ESI`; the calling convention is lost.
//
// * `FUN_0064ed0c` (27 bytes): a virtual destructor call with the object in
//   `unaff_EBP`.
//
// * `FUN_0064ed30` (33 bytes): membership probe into the string-table
//   registry.  `FUN_005e98b0` hashes an eight-byte `{pointer, length}` key, but
//   the only call site (`FUN_006fa570`) pushes a single dword, so the length
//   word is stack residue and the key extent is not recoverable.
//
// * `FUN_0064ed60` (147 bytes): destructor loop over a vector of string-table
//   entries; every iteration ends in the non-returning `FUN_0093ec70` path and
//   the entry layout belongs to the CRT string table, not to a game record.
//
// * `FUN_0064ee30` (105 bytes): registry erase keyed by a pointer that also
//   invokes the entry's virtual destructor (`call *0x10(%edx)`); the registry
//   comes from the TLS accessor `FUN_0064eeb0`.
//
// * `FUN_0064eeb0` (81 bytes): TLS initialiser that returns the address of a
//   static CRT object; it reads `_tls_index` and the thread-local block.
//
// * `FUN_0064ef10` (25 bytes): forwards the address of its own stack argument
//   to `FUN_00595a20`; it is a call-site trampoline with no game logic.
//
// * `FUN_0064ef30` (120 bytes): driver that walks the registry's bucket vector
//   and calls `FUN_0064eb50` for every entry.
//
// * `FUN_0064f1f0` (23 bytes): its body is the head of `FUN_0064f207`; the
//   decompiler split the 0x2c0-byte selection loop and gave the tail the
//   earlier address.
//
// * `FUN_0064f207` (310 bytes): the target-selection loop.  It reads twelve
//   inputs from `unaff_EBP` stack slots plus `unaff_EBX`, and `FUN_0064f39f`
//   and `FUN_0064f414` are jumps back into its body, so there is no recoverable
//   signature.
//
// * `FUN_0064f39f` (117 bytes) and `FUN_0064f414` (163 bytes): continuation
//   fragments of the same loop (`FUN_0064f414` switches on `in_AX` and reads
//   `unaff_EBX`/`unaff_ESI`/`unaff_EDI`), not independent routines.
//
// * `FUN_0064f580` (104 bytes): builds a `0x240`-wide search box around the
//   unit and calls the unmodelled `FUN_0064f1f0` loop; the box arithmetic has
//   no independent result.
//
// * `FUN_0064f650` (235 bytes): the entry overlaps the previous instruction
//   and the body divides `in_AX` by a byte extracted from `unaff_EBX`.
//
// * `FUN_0064f890` (32 bytes) and `FUN_0064f8b0` (32 bytes): one-line wrappers
//   that build the grid list (`FUN_005a5790`) and call the unmodelled
//   `FUN_0064f1f0`.
//
// * `FUN_0064fbf2` (99 bytes), `FUN_0064fc55` (87 bytes) and `FUN_0064fcac`
//   (258 bytes): other entries into the shared `0x0064fbe3` tail, with
//   `unaff_EBP`/`unaff_EBX` inputs and the 64-byte map-cell comparison.
//
// * `FUN_0064febf` (74 bytes): SEH unwind that calls `FUN_00688290` and then
//   terminates.
//
// Counts for the window: 50 ledger rows = 50 `decompiled_uncited`; 23 rows
// are translated through 20 functions (three address pairs share one
// function) and 27 are left alone for the reasons above.

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_wave11_agent04 {
    use super::*;

    fn row(hp_raw: i32, shields: u16) -> W11a04UnitRow {
        W11a04UnitRow {
            has_shields: shields != 0,
            shield_points: shields,
            hp_raw,
        }
    }

    fn state(type_id: u16, current: i32, max: i32) -> W11a04UnitState {
        W11a04UnitState {
            type_id,
            row: row(max << 8, 0),
            carried: 0,
            energy: None,
            current_points: current,
            max_points: max,
            dying: false,
            max_hp_points: max,
        }
    }

    fn tables(count: usize) -> W11a04ValueTables {
        W11a04ValueTables {
            slot0: vec![0; count],
            slot1: vec![0; count],
        }
    }

    // -- 0x0064efb0 / 0x0064eff0 --------------------------------------------

    /// `0x0064efb0`: control bytes are dropped, the terminator is always
    /// written, and a zero-length destination writes nothing.
    #[test]
    fn copy_printable_drops_controls_and_terminates() {
        let mut dst = [0xaau8; 8];
        let written = w11a04_copy_printable(b"a\x01b\tc", &mut dst);
        assert_eq!(written, 3);
        assert_eq!(&dst[..4], b"abc\0");

        let mut empty = [0xaau8; 4];
        assert_eq!(w11a04_copy_printable(b"abc", &mut empty[..0]), 0);
        assert_eq!(empty, [0xaa; 4], "zero-length destination is untouched");

        let mut one = [0xaau8; 1];
        assert_eq!(w11a04_copy_printable(b"abc", &mut one), 0);
        assert_eq!(one, [0], "the terminator reserves the only byte");
    }

    /// `0x0064eff0`: tab, CR and LF survive while the other controls do not,
    /// and the loop keeps one byte for the terminator.
    #[test]
    fn copy_printable_ws_keeps_the_line_controls() {
        let mut dst = [0xaau8; 10];
        let written = w11a04_copy_printable_ws(b"a\tb\nc\r\x02d", &mut dst);
        assert_eq!(written, 7);
        assert_eq!(&dst[..8], b"a\tb\nc\rd\0");

        let mut full = [0xaau8; 4];
        assert_eq!(w11a04_copy_printable_ws(b"abcdef", &mut full), 3);
        assert_eq!(&full, b"abc\0");
    }

    // -- 0x0064f060 ---------------------------------------------------------

    /// `0x0064f060`: each switch arm, the exhausted division and the
    /// pass-through default.
    #[test]
    fn scale_value_covers_every_arm() {
        assert_eq!(w11a04_scale_value(0x07, 400), 100);
        assert_eq!(w11a04_scale_value(0x29, 400), 100);
        assert_eq!(w11a04_scale_value(0x40, 400), 100);
        assert_eq!(w11a04_scale_value(0x0d, 400), 0);
        assert_eq!(w11a04_scale_value(0x49, 400), 0);
        assert_eq!(w11a04_scale_value(0x55, 400), 0);
        assert_eq!(w11a04_scale_value(0x20, 400), 800);
        assert_eq!(w11a04_scale_value(0x2b, 400), 800);
        assert_eq!(w11a04_scale_value(0x41, 400), 800);
        assert_eq!(w11a04_scale_value(0x2f, 400), 25);
        assert_eq!(w11a04_scale_value(0x32, 400), 25);
        assert_eq!(w11a04_scale_value(0x53, 405), 40);
        assert_eq!(w11a04_scale_value(0x03, 405), 405);
        assert_eq!(
            w11a04_scale_value(0x20, u32::MAX),
            u32::MAX.wrapping_mul(2),
            "the doubling wraps"
        );
    }

    // -- 0x0064f120 ---------------------------------------------------------

    fn marine_row() -> W11a04UnitRow {
        row(40 << 8, 0)
    }

    fn rifle() -> W11a04WeaponRow {
        W11a04WeaponRow {
            damage: 10,
            cooldown: 2,
            factor: 3,
            range: 64,
        }
    }

    /// `0x0064f120`: `(range/cooldown)*factor*damage` plus
    /// `((damage*factor*0x800/cooldown)*base)>>8`, scaled by 758/100.
    #[test]
    fn weapon_value_matches_the_recovered_expression() {
        // 64/2*3*10 = 960; 10*3*2048/2 = 30720; 30720*40>>8 = 4800;
        // (4800+960)*758/100 = 43660.8 -> 43660.
        assert_eq!(w11a04_weapon_value(marine_row(), 5, rifle()), 43_660);
    }

    /// `0x0064f120`: weapon `0x82` drops the burst term but keeps the range
    /// term; a shield row raises `base`; a zero base or cooldown is `0`.
    #[test]
    fn weapon_value_edge_cases() {
        assert_eq!(w11a04_weapon_value(marine_row(), 0x82, rifle()), 7_276);
        let shielded = row(40 << 8, 60);
        assert_eq!(shielded.w11a04_base_points(), 100);
        assert_ne!(w11a04_weapon_value(shielded, 5, rifle()), 43_660);
        assert_eq!(w11a04_weapon_value(row(0, 0), 5, rifle()), 0);
        let mut stalled = rifle();
        stalled.cooldown = 0;
        assert_eq!(w11a04_weapon_value(marine_row(), 5, stalled), 0);
    }

    // -- 0x0064f720 / 0x0064f870 / 0x0064f880 -------------------------------

    /// `0x0064f870` / `0x0064f880`: the getters read the caller's slot table
    /// and report a type past its end as absent.
    #[test]
    fn table_value_reads_the_selected_slot() {
        let mut t = tables(0xe4);
        t.slot0[7] = 11;
        t.slot1[7] = 22;
        assert_eq!(w11a04_table_value(&t.slot0, 7), Some(11));
        assert_eq!(w11a04_table_value(&t.slot1, 7), Some(22));
        assert_eq!(w11a04_table_value(&t.slot0, 0xe4), None);
    }

    /// `0x0064f720`: the `0x7d` row multiplies the carried count by row `0`
    /// and the energy byte adds its half.
    #[test]
    fn slot_value_handles_row_7d_and_energy() {
        let mut t = tables(0xe4);
        t.slot0[0] = 7;
        t.slot0[3] = 11;
        t.slot0[0x7d] = 5;
        assert_eq!(w11a04_slot_value(&t.slot0, 3, 0, None), Some(11));
        assert_eq!(w11a04_slot_value(&t.slot0, 0x7d, 2, None), Some(14));
        assert_eq!(w11a04_slot_value(&t.slot0, 3, 0, Some(9)), Some(15));
        assert_eq!(w11a04_slot_value(&t.slot0, 0xe4, 0, None), None);
    }

    // -- 0x0064f790 ---------------------------------------------------------

    /// `0x0064f790`: the four zero rows, the dying gate and the
    /// current/maximum scaling.
    #[test]
    fn unit_value_zeroes_and_scales() {
        let mut t = tables(0xe4);
        t.slot0[3] = 11;
        assert_eq!(w11a04_unit_value(&t.slot0, &state(3, 20, 40)), Some(5));
        assert_eq!(w11a04_unit_value(&t.slot0, &state(0x23, 20, 40)), Some(0));
        assert_eq!(w11a04_unit_value(&t.slot0, &state(0x24, 20, 40)), Some(0));
        assert_eq!(w11a04_unit_value(&t.slot0, &state(0x3b, 20, 40)), Some(0));
        assert_eq!(w11a04_unit_value(&t.slot0, &state(0x61, 20, 40)), Some(0));

        let mut dying = state(3, 20, 40);
        dying.dying = true;
        assert_eq!(w11a04_unit_value(&t.slot0, &dying), Some(0));
        dying.current_points = 40;
        assert_eq!(w11a04_unit_value(&t.slot0, &dying), Some(11));

        // The shield pool joins the divisor: 10 shields + 40 HP, so
        // 11 * 20 / 50 = 4.
        let mut shielded = state(3, 20, 40);
        shielded.row = row(30 << 8, 10);
        assert_eq!(w11a04_unit_value(&t.slot0, &shielded), Some(4));

        // A zero HP + shield divisor falls back to 1, i.e. no scaling.
        assert_eq!(w11a04_unit_value(&t.slot0, &state(3, 20, 0)), Some(220));
    }

    // -- 0x0064f5f0 / 0x0064f8d0 / 0x0064f8f0 -------------------------------

    fn candidate(type_id: u16, airborne: bool, enemy: bool) -> W11a04Candidate {
        W11a04Candidate {
            unit: state(type_id, 10, 10),
            airborne,
            enemy,
        }
    }

    /// `0x0064f5f0`: only enemies whose airborne bit matches are summed and
    /// each contributes the larger of the two slot values.
    #[test]
    fn enemy_value_sum_filters_and_takes_the_best_slot() {
        let mut t = tables(0xe4);
        t.slot0[3] = 11;
        t.slot1[3] = 22;
        t.slot0[4] = 30;
        t.slot1[4] = 12;

        let candidates = [
            candidate(3, true, true),
            candidate(4, true, true),
            candidate(5, false, true),
            candidate(6, true, false),
        ];
        assert_eq!(w11a04_enemy_value_sum(true, &candidates, &t), 52);
        assert_eq!(w11a04_enemy_value_sum(false, &candidates, &t), 0);

        // 0x0064f8d0 passes 1, 0x0064f8f0 passes 0.
        assert_eq!(w11a04_enemy_value_sum_airborne(&candidates, &t), 52);
        let mut ground = candidates;
        ground[2].unit = state(7, 10, 10);
        t.slot0[7] = 40;
        assert_eq!(w11a04_enemy_value_sum_ground(&ground, &t), 40);
        assert_eq!(w11a04_enemy_value_sum_airborne(&ground, &t), 52);
    }

    // -- 0x0064f840 / 0x0064f910 --------------------------------------------

    /// `0x0064f840`/`0x0064f910`: only state `4` goes to the secondary
    /// routine; every other state keeps the address' own flag.
    #[test]
    fn order4_dispatch_uses_state_four() {
        assert_eq!(
            w11a04_order4_dispatch(4, false),
            W11a04Order4Action::Secondary
        );
        assert_eq!(
            w11a04_order4_dispatch(4, true),
            W11a04Order4Action::Secondary
        );
        assert_eq!(
            w11a04_order4_dispatch(3, false),
            W11a04Order4Action::SelectNear { flag: false }
        );
        assert_eq!(
            w11a04_order4_dispatch(3, true),
            W11a04Order4Action::SelectNear { flag: true }
        );
        assert_eq!(
            w11a04_order4_dispatch(0, true),
            W11a04Order4Action::SelectNear { flag: true }
        );
    }

    // -- 0x0064f940 ---------------------------------------------------------

    fn table_row(weapon_slot0: u8, weapon_slot1: u8, turret_row: u16) -> W11a04TableRow {
        W11a04TableRow {
            unit: marine_row(),
            turret_row,
            weapon_slot0,
            weapon_slot1,
        }
    }

    /// `0x0064f940`: the weaponless placeholder `1` is cleared on the side
    /// that has a real weapon, while a unit with no weapon in either column
    /// keeps `1` in both tables.
    #[test]
    fn build_value_tables_clears_the_weaponless_side() {
        let mut rows = vec![table_row(0x82, 0x82, 0xe4); 8];
        rows[3] = table_row(0x82, 1, 0xe4);
        let weapons = [W11a04WeaponRow::default(), rifle()];

        let t = w11a04_build_value_tables(&rows, &weapons);
        assert_eq!(t.slot0[3], 0, "slot 0 was the weaponless placeholder");
        assert_eq!(t.slot1[3], 43_660);
        assert_eq!(t.slot0[5], 1);
        assert_eq!(t.slot1[5], 1);
        assert_eq!(t.slot0.len(), W11A04_TYPE_COUNT);
        assert_eq!(t.slot0[20], 0, "rows past the input stay zero");
        assert_eq!(t.slot1[20], 0);
    }

    /// `0x0064f940`: the turret table redirects the weapon column while the
    /// value keeps the type's own `units.dat` row, and `0x0064f060` scales
    /// the result for the special rows.
    #[test]
    fn build_value_tables_follows_the_turret_row_and_scale() {
        let weapons = [W11a04WeaponRow::default(), rifle()];

        // 0x48 selects row 0x49's ground weapon (1); its own row says 0x82.
        let mut rows = vec![table_row(0x82, 0x82, 0xe4); 0x56];
        rows[0x49] = table_row(0x82, 1, 0xe4);
        let t = w11a04_build_value_tables(&rows, &weapons);
        assert_eq!(t.slot1[0x48], 43_660, "weapon comes from the turret row");
        assert_eq!(
            t.slot0[0x48], 0,
            "its own slot 0 is the cleared placeholder"
        );

        // 0x07 is scaled by >> 2 and takes its weapon from its own row; the
        // weaponless placeholder in the other slot is cleared again.
        rows[0x07] = table_row(1, 0x82, 0xe4);
        let t = w11a04_build_value_tables(&rows, &weapons);
        assert_eq!(t.slot0[0x07], 43_660 >> 2);
        assert_eq!(t.slot1[0x07], 0);

        // A turret row of 0xe4 means "the type's own row".
        rows[0x30] = table_row(1, 0x82, 0xe4);
        let t = w11a04_build_value_tables(&rows, &weapons);
        assert_eq!(t.slot0[0x30], 43_660);

        // A missing weapon row yields zero, not a panic.
        rows[0x31] = table_row(9, 9, 0xe4);
        let t = w11a04_build_value_tables(&rows, &weapons);
        assert_eq!(t.slot0[0x31], 0);
        assert_eq!(t.slot1[0x31], 0);
    }

    // -- 0x0064fa60 / 0x0064fa70 --------------------------------------------

    /// `0x0064fa60`/`0x0064fa70`: the two cached words at `+0x138`/`+0x13a`,
    /// and `0x0064f030` fills them with the unit's own slot values.
    #[test]
    fn cached_pair_getters() {
        let mut t = tables(0xe4);
        t.slot0[3] = 11;
        t.slot1[3] = 22;

        let mut pair = W11a04CachedPair::default();
        pair.w11a04_cache_slot_values(&state(3, 10, 10), &t);
        assert_eq!(pair.w11a04_field_138(), 11);
        assert_eq!(pair.w11a04_field_13a(), 22);

        // Retail stores `%ax`, so the 16-bit field drops the high half.
        t.slot0[3] = 0x1_0000 + 7;
        pair.w11a04_cache_slot_values(&state(3, 10, 10), &t);
        assert_eq!(pair.w11a04_field_138(), 7);

        // A type past the table stores 0 instead of reading adjacent memory.
        pair.w11a04_cache_slot_values(&state(0xff, 10, 10), &t);
        assert_eq!(pair.w11a04_field_138(), 0);
        assert_eq!(pair.w11a04_field_13a(), 0);
    }

    // -- 0x0064fa80 / 0x0064fabc / 0x0064fad0 / 0x0064fae1 -----------------

    /// `0x0064fa80`: the block is `count * 0x34` zeroed bytes, the unsigned
    /// multiply overflow takes the `0x0064fabc` null branch, and a zero count
    /// is an empty block.
    #[test]
    fn record_block_is_checked_and_zeroed() {
        let block = w11a04_record_block(3).expect("in range");
        assert_eq!(block.len(), 3);
        assert!(block.iter().all(|record| record == &[0u8; 0x34]));

        assert_eq!(w11a04_record_block(0), Some(Vec::new()));
        assert_eq!(w11a04_record_block(u32::MAX), None);
        assert_eq!(w11a04_record_block(u32::MAX / 0x34 + 1), None);
    }

    /// `0x0064fad0`: releasing clears a present block and is a no-op on null;
    /// `0x0064fae1` is the empty hook.
    #[test]
    fn release_block_and_empty_hook() {
        let mut block = w11a04_record_block(1);
        w11a04_release_block(&mut block);
        assert_eq!(block, None);
        w11a04_release_block(&mut block);
        assert_eq!(block, None);

        w11a04_empty_hook();
        assert_eq!(W11A04_RELEASE_OPERAND, 0x4e24);
        assert_eq!(W11A04_RECORD_BYTES, 0x34);
    }

    // -- 0x0064faf0 ---------------------------------------------------------

    /// `0x0064faf0`: unlink from the source list, decrement unless airborne,
    /// and push onto the destination LIFO top.
    #[test]
    fn unlink_and_push_moves_nodes() {
        let mut nodes = vec![
            W11a04LinkNode {
                next: Some(1),
                prev: None,
                unit: 0,
            },
            W11a04LinkNode {
                next: Some(2),
                prev: Some(0),
                unit: 1,
            },
            W11a04LinkNode {
                next: None,
                prev: Some(1),
                unit: 2,
            },
        ];
        let mut source = W11a04ListHead {
            head: Some(0),
            count: 3,
        };
        let mut top = None;

        assert!(w11a04_unlink_and_push(
            &mut nodes,
            &mut source,
            &mut top,
            1,
            true
        ));
        assert_eq!(source.count, 3, "airborne units do not change the count");
        assert_eq!(source.head, Some(0));
        assert_eq!(nodes[0].next, Some(2));
        assert_eq!(nodes[2].prev, Some(0));
        assert_eq!(top, Some(1));
        assert_eq!(nodes[1].prev, None);
        assert_eq!(nodes[1].next, None);

        assert!(w11a04_unlink_and_push(
            &mut nodes,
            &mut source,
            &mut top,
            0,
            false
        ));
        assert_eq!(source.count, 2);
        assert_eq!(source.head, Some(2));
        assert_eq!(top, Some(0));
        assert_eq!(nodes[0].next, Some(1));
        assert_eq!(nodes[0].prev, None);
        assert_eq!(nodes[1].prev, Some(0));

        assert!(!w11a04_unlink_and_push(
            &mut nodes,
            &mut source,
            &mut top,
            9,
            false
        ));
        assert_eq!(source.count, 2, "an invalid node is ignored");
    }

    // -- 0x0064fb60 ---------------------------------------------------------

    fn guard(order_state: Option<u8>, order_byte: u8) -> W11a04GuardInput {
        W11a04GuardInput {
            order_state,
            order_target_is_self: true,
            building: false,
            flag_5970: true,
            order_byte,
            flag_5680: false,
        }
    }

    /// `0x0064fb60`: every early exit, the cue path and the plain fall
    /// through.
    #[test]
    fn guard_chain_outcomes() {
        assert_eq!(w11a04_guard_chain(&guard(None, 0)), W11a04Guard::Skip);
        assert_eq!(w11a04_guard_chain(&guard(Some(3), 0)), W11a04Guard::Skip);
        assert_eq!(w11a04_guard_chain(&guard(Some(4), 0x6a)), W11a04Guard::Skip);
        assert_eq!(w11a04_guard_chain(&guard(Some(4), 0x96)), W11a04Guard::Skip);
        assert_eq!(w11a04_guard_chain(&guard(Some(4), 0x62)), W11a04Guard::Skip);
        assert_eq!(w11a04_guard_chain(&guard(Some(4), 0x06)), W11a04Guard::Skip);
        assert_eq!(
            w11a04_guard_chain(&guard(Some(4), 0x01)),
            W11a04Guard::Continue
        );

        let mut cue = guard(Some(4), 0x01);
        cue.flag_5680 = true;
        assert_eq!(w11a04_guard_chain(&cue), W11a04Guard::CueThenContinue);

        let mut other_target = guard(Some(4), 0x01);
        other_target.order_target_is_self = false;
        assert_eq!(w11a04_guard_chain(&other_target), W11a04Guard::Skip);

        let mut building = guard(Some(4), 0x01);
        building.building = true;
        assert_eq!(w11a04_guard_chain(&building), W11a04Guard::Skip);

        let mut no_gate = guard(Some(4), 0x01);
        no_gate.flag_5970 = false;
        assert_eq!(w11a04_guard_chain(&no_gate), W11a04Guard::Skip);

        // The cue path ignores the 0x62/0x06 check, like the retail order.
        let mut cue_62 = guard(Some(4), 0x62);
        cue_62.flag_5680 = true;
        assert_eq!(w11a04_guard_chain(&cue_62), W11a04Guard::CueThenContinue);
    }
}
