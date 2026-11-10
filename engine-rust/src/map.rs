//! `.scm`/`.scx` scenario (CHK) parsing and terrain walkability.
//!
//! StarCraft maps are MPQ archives containing `staredit\scenario.chk`.
//! `tools/mpq_extract.cpp` unwraps that member; the CHK parser below is pure
//! Rust and handles the sections the simulation needs:
//! `DIM`, `ERA`, `MTXM`, `UNIT`, `MRGN`, `STR`, `OWNR`, `SIDE`, `TRIG` and
//! the extended unit/property sections used by Remastered maps.
//!
//! Walkability comes from the tileset's `vf4` file.  Each 32-byte `vf4`
//! record holds sixteen little-endian `u16` flags, one per 8x8-pixel
//! minitile of a 32x32-pixel megatile.  Bit 0 is walkable, bit 1 is the
//! middle height and bit 2 is the high height.  The executable's region
//! builder and the independent OpenBW engine agree on this layout.

use std::collections::HashMap;

use crate::trigger::RECORD_SIZE;

pub const TILE_PX: i32 = 32;
pub const WALK_TILE_PX: i32 = 8;
pub const ELEVATION_LEVELS: usize = 3;

/// One of the map's 64 `UPRP` unit-property slots used by trigger action 11
/// (Create Unit with Properties) and, when its action carries a slot number,
/// by action 44 (Create Unit).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnitProperties {
    /// Bit set of special states (cloak 0x01, burrow 0x02, in transit 0x04,
    /// hallucination 0x08, invincible 0x10).
    pub state_flags: u16,
    /// Bit set of fields the slot applies (owner bit 0x01 is defined but
    /// not read by `0x00712ce0`; HP 0x02, shields 0x04, energy 0x08,
    /// resources 0x10, hangar 0x20).
    pub field_flags: u16,
    pub owner: u8,
    pub hit_points_percent: u8,
    pub shields_percent: u8,
    pub energy_percent: u8,
    pub resource_amount: u32,
    pub hangar: u16,
    pub states: u16,
}

/// One 10-byte `THG2` record (unit/doodad sprite).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapThingy {
    pub unit_id: u16,
    pub x: i32,
    pub y: i32,
    pub player: u8,
    pub flags: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapUnit {
    /// Offset 0: unique serial used by add-on and Nydus links.
    pub serial: u32,
    pub unit_id: u16,
    pub x: i32,
    pub y: i32,
    /// Offset 16; always read, even when the owner field-set bit is clear.
    pub owner: u8,
    /// Offset 12: special states this record sets.
    pub states_set: u16,
    /// Offset 14: fields this record sets (owner/HP/shields/energy/
    /// resources/hangar/state).  Only set fields are applied.
    pub fields_set: u16,
    pub hit_point_percent: u8,
    pub shield_percent: u8,
    pub energy_percent: u8,
    pub resource_amount: u32,
    pub hangar: u16,
    /// Offset 26: the special states themselves (cloaked/burrowed/lifted/
    /// hallucinated/invincible).
    pub states: u16,
    /// Offset 32: linked unit serial (add-on/Nydus); zero when unused.
    pub link_serial: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub string_id: u16,
    pub flags: u16,
}

impl Location {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

/// `UNIS` / `UNIx` per-unit overrides.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnitSettings {
    pub use_default: Vec<u8>,
    pub hit_points: Vec<u32>,
    pub shields: Vec<u16>,
    pub armor: Vec<u8>,
    pub build_time: Vec<u16>,
    pub mineral_cost: Vec<u16>,
    pub gas_cost: Vec<u16>,
    pub string_id: Vec<u16>,
    pub weapon_damage: Vec<u16>,
    pub weapon_bonus: Vec<u16>,
}

impl UnitSettings {
    fn parse(raw: &[u8], weapons: usize) -> Option<Self> {
        if raw.len() != if weapons == 130 { 0x1048 } else { 0xFD0 } {
            return None;
        }
        let mut out = Self {
            use_default: raw[0..228].to_vec(),
            hit_points: (0..228).map(|i| u32_at(raw, 0x0E4 + 4 * i)).collect(),
            shields: (0..228).map(|i| u16_at(raw, 0x474 + 2 * i)).collect(),
            armor: raw[0x63C..0x63C + 228].to_vec(),
            build_time: (0..228).map(|i| u16_at(raw, 0x720 + 2 * i)).collect(),
            mineral_cost: (0..228).map(|i| u16_at(raw, 0x8E8 + 2 * i)).collect(),
            gas_cost: (0..228).map(|i| u16_at(raw, 0xAB0 + 2 * i)).collect(),
            string_id: (0..228).map(|i| u16_at(raw, 0xC78 + 2 * i)).collect(),
            weapon_damage: (0..weapons).map(|i| u16_at(raw, 0xE40 + 2 * i)).collect(),
            weapon_bonus: (0..weapons)
                .map(|i| u16_at(raw, 0xE40 + weapons * 2 + 2 * i))
                .collect(),
        };
        out.use_default.truncate(228);
        Some(out)
    }
}

/// `UPGS` / `UPGx` upgrade cost columns.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UpgradeSettings {
    pub use_default: Vec<u8>,
    pub mineral_base: Vec<u16>,
    pub mineral_factor: Vec<u16>,
    pub gas_base: Vec<u16>,
    pub gas_factor: Vec<u16>,
    pub time_base: Vec<u16>,
    pub time_factor: Vec<u16>,
}

/// `TECS` / `TECx` technology cost columns.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TechSettings {
    pub use_default: Vec<u8>,
    pub mineral_cost: Vec<u16>,
    pub gas_cost: Vec<u16>,
    pub research_time: Vec<u16>,
    pub energy_cost: Vec<u16>,
}

/// `PUNI` per-player unit availability.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnitAvailability {
    /// 12 players x 228 units, player-major.
    pub player: Vec<u8>,
    pub global: Vec<u8>,
    /// 12 players x 228 units: use the global byte instead.
    pub use_global: Vec<u8>,
}

impl UnitAvailability {
    fn parse(raw: &[u8]) -> Option<Self> {
        if raw.len() != 0x1644 {
            return None;
        }
        Some(Self {
            player: raw[0..0xAB0].to_vec(),
            global: raw[0xAB0..0xB94].to_vec(),
            use_global: raw[0xB94..0x1644].to_vec(),
        })
    }

    pub fn available(&self, player: usize, unit: u16) -> bool {
        if player >= 12 || unit as usize >= 228 {
            return true;
        }
        let i = player * 228 + unit as usize;
        let raw = if self.use_global.get(i).copied().unwrap_or(0) != 0 {
            self.global.get(unit as usize).copied().unwrap_or(1)
        } else {
            self.player.get(i).copied().unwrap_or(1)
        };
        raw != 0
    }
}

/// `PTEC`/`PTEx`: per-player technology availability/researched.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TechRestrictions {
    pub player_available: Vec<u8>,
    pub player_researched: Vec<u8>,
    pub global_available: Vec<u8>,
    pub global_researched: Vec<u8>,
    pub use_global: Vec<u8>,
    pub count: usize,
}

impl TechRestrictions {
    fn parse(raw: &[u8], count: usize) -> Option<Self> {
        let expected = if count == 44 { 0x688 } else { 0x390 };
        if raw.len() != expected || count == 0 {
            return None;
        }
        let per = 12 * count;
        let a = 0;
        let r = per;
        let ga = 2 * per;
        let gr = 2 * per + count;
        let ug = 2 * per + 2 * count;
        Some(Self {
            player_available: raw[a..a + per].to_vec(),
            player_researched: raw[r..r + per].to_vec(),
            global_available: raw[ga..ga + count].to_vec(),
            global_researched: raw[gr..gr + count].to_vec(),
            use_global: raw[ug..ug + per].to_vec(),
            count,
        })
    }

    pub fn available(&self, player: usize, tech: u8) -> bool {
        if player >= 12 || tech as usize >= self.count {
            return true;
        }
        let i = player * self.count + tech as usize;
        let raw = if self.use_global.get(i).copied().unwrap_or(0) != 0 {
            self.global_available
                .get(tech as usize)
                .copied()
                .unwrap_or(1)
        } else {
            self.player_available.get(i).copied().unwrap_or(1)
        };
        raw != 0
    }

    pub fn researched(&self, player: usize, tech: u8) -> bool {
        if player >= 12 || tech as usize >= self.count {
            return false;
        }
        let i = player * self.count + tech as usize;
        let raw = if self.use_global.get(i).copied().unwrap_or(0) != 0 {
            self.global_researched
                .get(tech as usize)
                .copied()
                .unwrap_or(0)
        } else {
            self.player_researched.get(i).copied().unwrap_or(0)
        };
        raw != 0
    }
}

/// `UPGR`/`PUPx`: per-player upgrade start/max levels.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UpgradeRestrictions {
    pub player_max: Vec<u8>,
    pub player_start: Vec<u8>,
    pub global_max: Vec<u8>,
    pub global_start: Vec<u8>,
    pub use_global: Vec<u8>,
    pub count: usize,
}

impl UpgradeRestrictions {
    fn parse(raw: &[u8], count: usize) -> Option<Self> {
        let expected = if count == 61 { 0x90E } else { 0x6D4 };
        if raw.len() != expected || count == 0 {
            return None;
        }
        let per = 12 * count;
        let max = 0;
        let start = per;
        let gmax = 2 * per;
        let gstart = 2 * per + count;
        let ug = 2 * per + 2 * count;
        Some(Self {
            player_max: raw[max..max + per].to_vec(),
            player_start: raw[start..start + per].to_vec(),
            global_max: raw[gmax..gmax + count].to_vec(),
            global_start: raw[gstart..gstart + count].to_vec(),
            use_global: raw[ug..ug + per].to_vec(),
            count,
        })
    }

    pub fn max_level(&self, player: usize, upgrade: u8) -> u8 {
        if player >= 12 || upgrade as usize >= self.count {
            return u8::MAX;
        }
        let i = player * self.count + upgrade as usize;
        if self.use_global.get(i).copied().unwrap_or(0) != 0 {
            self.global_max
                .get(upgrade as usize)
                .copied()
                .unwrap_or(u8::MAX)
        } else {
            self.player_max.get(i).copied().unwrap_or(u8::MAX)
        }
    }

    pub fn start_level(&self, player: usize, upgrade: u8) -> u8 {
        if player >= 12 || upgrade as usize >= self.count {
            return 0;
        }
        let i = player * self.count + upgrade as usize;
        if self.use_global.get(i).copied().unwrap_or(0) != 0 {
            self.global_start
                .get(upgrade as usize)
                .copied()
                .unwrap_or(0)
        } else {
            self.player_start.get(i).copied().unwrap_or(0)
        }
    }
}

/// Selected scenario sections for a map version.
#[derive(Clone, Debug, Default)]
pub struct ScenarioRules {
    pub unit_settings: Option<UnitSettings>,
    pub upgrade_settings: Option<UpgradeSettings>,
    pub tech_settings: Option<TechSettings>,
    pub unit_availability: Option<UnitAvailability>,
    pub tech_restrictions: Option<TechRestrictions>,
    pub upgrade_restrictions: Option<UpgradeRestrictions>,
}

#[derive(Clone, Debug, Default)]
pub struct Scenario {
    pub width: usize,
    pub height: usize,
    pub era: u16,
    pub tiles: Vec<u16>,
    pub mask: Vec<u8>,
    pub units: Vec<MapUnit>,
    /// `THG2` records in file order.  `0x1000` records are static sprites;
    /// the rest are unit doodads (doors, traps) realized by the simulation.
    pub thingies: Vec<MapThingy>,
    pub locations: Vec<Location>,
    /// The 64 `UPRP` property slots (empty when the section is absent).
    pub unit_properties: Vec<UnitProperties>,
    pub strings: Vec<String>,
    pub owners: [u8; 12],
    pub sides: [u8; 12],
    pub triggers: Vec<u8>,
    /// `FORC` force (0..=3) of players 1..8; all in force 1 without the
    /// section.
    pub forces: [u8; 8],
    /// `FORC` force names (1-based `STR` ids, 0 = unnamed): the score
    /// screen's second line ("Colonial Militia").
    pub force_names: [u16; 4],
    /// `MBRF` mission-briefing triggers (`TRIG` layout, briefing actions;
    /// see `crate::ready_room`).
    pub briefing_triggers: Vec<u8>,
    /// `SPRP` scenario properties: string ids (1-based, 0 = none) of the
    /// map's title and description, as the map browser shows them.
    pub title_id: u16,
    pub description_id: u16,
    pub version: u16,
    pub unis: Option<UnitSettings>,
    pub unix: Option<UnitSettings>,
    pub upgs: Option<UpgradeSettings>,
    pub upgx: Option<UpgradeSettings>,
    pub tecs: Option<TechSettings>,
    pub tecx: Option<TechSettings>,
    pub puni: Option<UnitAvailability>,
    pub ptec: Option<TechRestrictions>,
    pub ptex: Option<TechRestrictions>,
    pub upgr: Option<UpgradeRestrictions>,
    pub pupx: Option<UpgradeRestrictions>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChkError {
    Missing(&'static str),
    Truncated {
        section: String,
        expected: usize,
        actual: usize,
    },
    BadDimension {
        width: usize,
        height: usize,
    },
}

impl std::fmt::Display for ChkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChkError::Missing(name) => write!(f, "scenario is missing the {name} section"),
            ChkError::Truncated {
                section,
                expected,
                actual,
            } => write!(
                f,
                "scenario section {section} is truncated: expected {expected} bytes, got {actual}"
            ),
            ChkError::BadDimension { width, height } => {
                write!(f, "scenario has an invalid size {width}x{height}")
            }
        }
    }
}

impl std::error::Error for ChkError {}

fn u16_at(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}

fn u32_at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

fn i32_at(d: &[u8], o: usize) -> i32 {
    u32_at(d, o) as i32
}

/// Section names the parser consumes.  Some distributed ladder maps are
/// wrapped by map-protection tools that scramble the section order and
/// corrupt the non-essential headers; the sections StarCraft needs to start a
/// game are still present.  `locate` resynchronises on those names.
const KNOWN_SECTIONS: &[&str] = &[
    "TYPE", "VER", "IVER", "IVE2", "VCOD", "OWNR", "ERA", "DIM", "SIDE", "MTXM", "TILE", "MASK",
    "UNIT", "PUNI", "PUPx", "UPGR", "UPRP", "UPUS", "UPRe", "PTEC", "PTEx", "ISOM", "DD2", "THG2",
    "STR", "STRx", "MRGN", "TRIG", "MBRF", "FORC", "SPRP", "WAV", "CMP", "UNIS", "UNIx", "UPGS",
    "UPGx", "TECS", "TECx", "COLR", "CRGB",
];

/// Locate one section by scanning for its four-byte name.
///
/// A sequential CHK walk is tried first by the caller.  This fallback is only
/// used for missing or invalid entries, so ordinary maps keep the exact
/// on-disk layout and the scan cannot shadow a valid section with random
/// data that happens to contain the same bytes.
fn locate<'a>(d: &'a [u8], name: &str, validate: impl Fn(&'a [u8]) -> bool) -> Option<&'a [u8]> {
    let needle = name.as_bytes();
    if needle.len() != 4 {
        return None;
    }
    let mut i = 0usize;
    while i + 8 <= d.len() {
        if &d[i..i + 4] == needle {
            let size = u32_at(d, i + 4) as usize;
            if size <= d.len() - (i + 8) {
                let body = &d[i + 8..i + 8 + size];
                if validate(body) {
                    return Some(body);
                }
            }
        }
        i += 1;
    }
    None
}

impl Scenario {
    /// Parse the raw contents of `staredit\scenario.chk`.
    pub fn parse(d: &[u8]) -> Result<Self, ChkError> {
        let mut sections: HashMap<String, &[u8]> = HashMap::new();
        let mut i = 0usize;
        while i + 8 <= d.len() {
            let name = String::from_utf8_lossy(&d[i..i + 4]).trim().to_string();
            let size = u32_at(d, i + 4) as usize;
            i += 8;
            if i + size > d.len() {
                // Protected maps interleave junk headers.  Stop the
                // sequential walk; `locate` below recovers the real sections.
                break;
            }
            // Only accept names the format defines.  This keeps junk headers
            // out of the map while still allowing optional sections.
            if KNOWN_SECTIONS.iter().any(|k| name.trim_end() == *k) {
                sections.insert(name, &d[i..i + size]);
            }
            i += size;
        }

        // Recovery pass for protected maps.  A required section is searched
        // for at any offset; optional sections get the same treatment.
        let era_bytes =
            locate(d, "ERA ", |b| b.len() == 2).or_else(|| locate(d, "ERA", |b| b.len() == 2));
        if let Some(b) = era_bytes {
            sections.insert("ERA".into(), b);
        }
        let dim_bytes = locate(d, "DIM ", |b| {
            b.len() == 4 && {
                let w = u16_at(b, 0) as usize;
                let h = u16_at(b, 2) as usize;
                w > 0 && h > 0 && w <= 256 && h <= 256
            }
        })
        .or_else(|| locate(d, "DIM", |b| b.len() == 4));
        if let Some(b) = dim_bytes {
            sections.insert("DIM".into(), b);
        }

        let dim = sections.get("DIM").ok_or(ChkError::Missing("DIM"))?;
        if dim.len() < 4 {
            return Err(ChkError::Truncated {
                section: "DIM".into(),
                expected: 4,
                actual: dim.len(),
            });
        }
        let width = u16_at(dim, 0) as usize;
        let height = u16_at(dim, 2) as usize;
        if width == 0 || height == 0 || width > 256 || height > 256 {
            return Err(ChkError::BadDimension { width, height });
        }

        let tile_bytes = width * height * 2;
        if !sections
            .get("MTXM")
            .map(|s| s.len() >= tile_bytes)
            .unwrap_or(false)
            && let Some(b) = locate(d, "MTXM", |b| b.len() >= tile_bytes)
        {
            sections.insert("MTXM".into(), b);
        }
        let mask_bytes = width * height;
        if !sections
            .get("MASK")
            .map(|s| s.len() >= mask_bytes)
            .unwrap_or(false)
            && let Some(b) = locate(d, "MASK", |b| b.len() >= mask_bytes)
        {
            sections.insert("MASK".into(), b);
        }
        for (name, min) in [("OWNR", 12usize), ("SIDE", 12)] {
            if !sections.get(name).map(|s| s.len() >= min).unwrap_or(false)
                && let Some(b) = locate(d, name, |b| b.len() >= min)
            {
                sections.insert(name.to_string(), b);
            }
        }
        // `UPRP` is exactly 64 x 20 bytes.  Protected maps can hide the
        // section behind a scrambled header, so resynchronise on its exact
        // size as well.
        if !sections
            .get("UPRP")
            .map(|s| s.len() == 64 * 20)
            .unwrap_or(false)
            && let Some(b) = locate(d, "UPRP", |b| b.len() == 64 * 20)
        {
            sections.insert("UPRP".into(), b);
        }
        // Variable-record sections have a stronger size predicate.
        if !sections
            .get("UNIT")
            .map(|s| s.len() % 36 == 0)
            .unwrap_or(false)
            && let Some(b) = locate(d, "UNIT", |b| !b.is_empty() && b.len() % 36 == 0)
        {
            sections.insert("UNIT".into(), b);
        }
        if !sections
            .get("MRGN")
            .map(|s| s.len() % 20 == 0)
            .unwrap_or(false)
            && let Some(b) = locate(d, "MRGN", |b| !b.is_empty() && b.len() % 20 == 0)
        {
            sections.insert("MRGN".into(), b);
        }
        if !sections
            .get("TRIG")
            .map(|s| !s.is_empty() && s.len() % RECORD_SIZE == 0)
            .unwrap_or(false)
            && let Some(b) = locate(d, "TRIG", |b| !b.is_empty() && b.len() % RECORD_SIZE == 0)
        {
            sections.insert("TRIG".into(), b);
        }
        if !sections.get("STR").map(|s| s.len() >= 2).unwrap_or(false)
            && let Some(b) = locate(d, "STR ", |b| {
                b.len() >= 2 && (u16_at(b, 0) as usize) * 2 + 2 <= b.len()
            })
        {
            sections.insert("STR".into(), b);
        }

        let tiles_raw = sections.get("MTXM").ok_or(ChkError::Missing("MTXM"))?;
        let tile_count = width * height;
        if tiles_raw.len() < tile_count * 2 {
            return Err(ChkError::Truncated {
                section: "MTXM".into(),
                expected: tile_count * 2,
                actual: tiles_raw.len(),
            });
        }
        let tiles = (0..tile_count)
            .map(|k| u16_at(tiles_raw, k * 2))
            .collect::<Vec<_>>();

        // MASK is the initial per-player fog layer.  Missing trailing
        // bytes are fogged in retail (`0xFF`), never pre-explored.
        let mask = {
            let raw = sections.get("MASK").copied().unwrap_or(&[]);
            let mut mask = vec![0xFFu8; tile_count];
            let n = raw.len().min(tile_count);
            mask[..n].copy_from_slice(&raw[..n]);
            mask
        };

        let mut thingies = Vec::new();
        if let Some(raw) = sections.get("THG2") {
            for c in raw.as_chunks::<10>().0 {
                thingies.push(MapThingy {
                    unit_id: u16_at(c, 0),
                    x: u16_at(c, 2) as i32,
                    y: u16_at(c, 4) as i32,
                    player: c[6],
                    flags: u16_at(c, 8),
                });
            }
        }

        let mut units = Vec::new();
        if let Some(raw) = sections.get("UNIT") {
            for chunk in raw.as_chunks::<36>().0 {
                units.push(MapUnit {
                    serial: u32_at(chunk, 0),
                    x: u16_at(chunk, 4) as i32,
                    y: u16_at(chunk, 6) as i32,
                    unit_id: u16_at(chunk, 8),
                    states_set: u16_at(chunk, 12),
                    fields_set: u16_at(chunk, 14),
                    owner: chunk[16],
                    hit_point_percent: chunk[17],
                    shield_percent: chunk[18],
                    energy_percent: chunk[19],
                    resource_amount: u32_at(chunk, 20),
                    hangar: u16_at(chunk, 24),
                    states: u16_at(chunk, 26),
                    link_serial: u32_at(chunk, 32),
                });
            }
        }

        let mut locations = Vec::new();
        if let Some(raw) = sections.get("MRGN") {
            for chunk in raw.as_chunks::<20>().0 {
                locations.push(Location {
                    left: i32_at(chunk, 0),
                    top: i32_at(chunk, 4),
                    right: i32_at(chunk, 8),
                    bottom: i32_at(chunk, 12),
                    string_id: u16_at(chunk, 16),
                    flags: u16_at(chunk, 18),
                });
            }
        }

        let mut unit_properties = Vec::new();
        if let Some(raw) = sections.get("UPRP") {
            for chunk in raw.as_chunks::<20>().0 {
                unit_properties.push(UnitProperties {
                    states: u16_at(chunk, 14),
                    state_flags: u16_at(chunk, 0),
                    field_flags: u16_at(chunk, 2),
                    owner: chunk[4],
                    hit_points_percent: chunk[5],
                    shields_percent: chunk[6],
                    energy_percent: chunk[7],
                    resource_amount: u32_at(chunk, 8),
                    hangar: u16_at(chunk, 12),
                });
            }
        }

        let mut strings = Vec::new();
        if let Some(raw) = sections.get("STRx").or_else(|| sections.get("STRx ")) {
            // Remastered STRx: u32 count followed by u32 offsets and
            // NUL-terminated UTF-8 strings.  Forty-eight shipped ladder maps
            // carry STRx with no STR.
            if raw.len() >= 4 {
                let count = u32_at(raw, 0) as usize;
                let table_end = 4 + count.saturating_mul(4);
                if raw.len() >= table_end {
                    for k in 0..count {
                        let start = u32_at(raw, 4 + k * 4) as usize;
                        match raw.get(start..) {
                            Some(bytes) => {
                                let nul = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                                strings.push(String::from_utf8_lossy(&bytes[..nul]).into_owned());
                            }
                            None => strings.push(String::new()),
                        }
                    }
                }
            }
        } else if let Some(raw) = sections.get("STR")
            && raw.len() >= 2
        {
            let count = u16_at(raw, 0) as usize;
            let table_end = 2 + count * 2;
            if raw.len() >= table_end {
                // Each offset starts a NUL-terminated string.  StarEdit
                // does not keep the offsets in order (and may share
                // tails), so a string ends at its NUL, never at the next
                // entry's offset.
                for k in 0..count {
                    let start = u16_at(raw, 2 + k * 2) as usize;
                    match raw.get(start..) {
                        Some(bytes) => {
                            let nul = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                            strings.push(String::from_utf8_lossy(&bytes[..nul]).into_owned());
                        }
                        None => strings.push(String::new()),
                    }
                }
            }
        }

        let mut owners = [0u8; 12];
        if let Some(raw) = sections.get("OWNR") {
            for (dst, src) in owners.iter_mut().zip(raw.iter()) {
                *dst = *src;
            }
        }
        let mut sides = [0u8; 12];
        if let Some(raw) = sections.get("SIDE") {
            for (dst, src) in sides.iter_mut().zip(raw.iter()) {
                *dst = *src;
            }
        }
        let mut forces = [0u8; 8];
        let mut force_names = [0u16; 4];
        if let Some(raw) = sections.get("FORC") {
            for (dst, src) in forces.iter_mut().zip(raw.iter()) {
                *dst = (*src).min(3);
            }
            for (k, name) in force_names.iter_mut().enumerate() {
                if raw.len() >= 10 + 2 * k {
                    *name = u16_at(raw, 8 + 2 * k);
                }
            }
        }

        let version = sections
            .get("VER")
            .filter(|b| b.len() >= 2)
            .map(|b| u16_at(b, 0))
            .unwrap_or(0);
        let unis = sections
            .get("UNIS")
            .and_then(|b| UnitSettings::parse(b, 100));
        let unix = sections
            .get("UNIx")
            .and_then(|b| UnitSettings::parse(b, 130));
        let upgs = sections.get("UPGS").and_then(|b| {
            if b.len() != 0x256 {
                return None;
            }
            Some(UpgradeSettings {
                use_default: b[0..46].to_vec(),
                mineral_base: (0..46).map(|i| u16_at(b, 0x02E + 2 * i)).collect(),
                mineral_factor: (0..46).map(|i| u16_at(b, 0x08A + 2 * i)).collect(),
                gas_base: (0..46).map(|i| u16_at(b, 0x0E6 + 2 * i)).collect(),
                gas_factor: (0..46).map(|i| u16_at(b, 0x142 + 2 * i)).collect(),
                time_base: (0..46).map(|i| u16_at(b, 0x19E + 2 * i)).collect(),
                time_factor: (0..46).map(|i| u16_at(b, 0x1FA + 2 * i)).collect(),
            })
        });
        let upgx = sections.get("UPGx").and_then(|b| {
            if b.len() != 0x31A {
                return None;
            }
            Some(UpgradeSettings {
                use_default: b[0..61].to_vec(),
                mineral_base: (0..61).map(|i| u16_at(b, 0x03E + 2 * i)).collect(),
                mineral_factor: (0..61).map(|i| u16_at(b, 0x0B8 + 2 * i)).collect(),
                gas_base: (0..61).map(|i| u16_at(b, 0x132 + 2 * i)).collect(),
                gas_factor: (0..61).map(|i| u16_at(b, 0x1AC + 2 * i)).collect(),
                time_base: (0..61).map(|i| u16_at(b, 0x226 + 2 * i)).collect(),
                time_factor: (0..61).map(|i| u16_at(b, 0x2A0 + 2 * i)).collect(),
            })
        });
        let tecs = sections.get("TECS").and_then(|b| {
            if b.len() != 0xD8 {
                return None;
            }
            Some(TechSettings {
                use_default: b[0..24].to_vec(),
                mineral_cost: (0..24).map(|i| u16_at(b, 0x18 + 2 * i)).collect(),
                gas_cost: (0..24).map(|i| u16_at(b, 0x48 + 2 * i)).collect(),
                research_time: (0..24).map(|i| u16_at(b, 0x78 + 2 * i)).collect(),
                energy_cost: (0..24).map(|i| u16_at(b, 0xA8 + 2 * i)).collect(),
            })
        });
        let tecx = sections.get("TECx").and_then(|b| {
            if b.len() != 0x18C {
                return None;
            }
            Some(TechSettings {
                use_default: b[0..44].to_vec(),
                mineral_cost: (0..44).map(|i| u16_at(b, 0x2C + 2 * i)).collect(),
                gas_cost: (0..44).map(|i| u16_at(b, 0x84 + 2 * i)).collect(),
                research_time: (0..44).map(|i| u16_at(b, 0xDC + 2 * i)).collect(),
                energy_cost: (0..44).map(|i| u16_at(b, 0x134 + 2 * i)).collect(),
            })
        });
        let puni = sections
            .get("PUNI")
            .and_then(|b| UnitAvailability::parse(b));
        let ptec = sections
            .get("PTEC")
            .and_then(|b| TechRestrictions::parse(b, 24));
        let ptex = sections
            .get("PTEx")
            .and_then(|b| TechRestrictions::parse(b, 44));
        let upgr = sections
            .get("UPGR")
            .and_then(|b| UpgradeRestrictions::parse(b, 46));
        let pupx = sections
            .get("PUPx")
            .and_then(|b| UpgradeRestrictions::parse(b, 61));

        let sprp = sections.get("SPRP").filter(|b| b.len() >= 4);
        let title_id = sprp.map(|b| u16_at(b, 0)).unwrap_or(0);
        let description_id = sprp.map(|b| u16_at(b, 2)).unwrap_or(0);

        Ok(Self {
            title_id,
            description_id,
            version,
            unis,
            unix,
            upgs,
            upgx,
            tecs,
            tecx,
            puni,
            ptec,
            ptex,
            upgr,
            pupx,
            width,
            height,
            era: sections
                .get("ERA")
                .and_then(|s| s.get(0..2))
                .map(|s| u16_at(s, 0) & 0x07)
                .unwrap_or(0),
            tiles,
            mask,
            units,
            thingies,
            locations,
            unit_properties,
            strings,
            owners,
            sides,
            triggers: sections.get("TRIG").map(|s| s.to_vec()).unwrap_or_default(),
            forces,
            force_names,
            briefing_triggers: sections.get("MBRF").map(|s| s.to_vec()).unwrap_or_default(),
        })
    }

    /// Select base or Brood War sections by map version and mode.
    pub fn rules(&self) -> ScenarioRules {
        let extended = self.version != 59;
        ScenarioRules {
            unit_settings: if extended {
                self.unix.clone().or_else(|| self.unis.clone())
            } else {
                self.unis.clone()
            },
            upgrade_settings: if extended {
                self.upgx.clone().or_else(|| self.upgs.clone())
            } else {
                self.upgs.clone()
            },
            tech_settings: if extended {
                self.tecx.clone().or_else(|| self.tecs.clone())
            } else {
                self.tecs.clone()
            },
            unit_availability: self.puni.clone(),
            tech_restrictions: if extended {
                self.ptex.clone().or_else(|| self.ptec.clone())
            } else {
                self.ptec.clone()
            },
            upgrade_restrictions: if extended {
                self.pupx.clone().or_else(|| self.upgr.clone())
            } else {
                self.upgr.clone()
            },
        }
    }

    pub fn tile(&self, tx: i32, ty: i32) -> Option<u16> {
        if tx < 0 || ty < 0 || tx as usize >= self.width || ty as usize >= self.height {
            return None;
        }
        Some(self.tiles[ty as usize * self.width + tx as usize])
    }

    /// StarCraft start locations are map units of type 214.
    pub fn start_locations(&self) -> Vec<(i32, i32)> {
        self.units
            .iter()
            .filter(|u| u.unit_id == 214)
            .map(|u| (u.x, u.y))
            .collect()
    }

    /// Start location owned by player slot `owner`.  The CHK lists them in
    /// any order ((8)The Hunters: owners 0,1,2,7,3,6,4,5), so indexing
    /// [`Self::start_locations`] by slot puts players on the wrong base.
    pub fn start_location_of(&self, owner: u8) -> Option<(i32, i32)> {
        self.units
            .iter()
            .find(|u| u.unit_id == 214 && u.owner == owner)
            .map(|u| (u.x, u.y))
    }
    /// The scenario's `OWNR` bytes are the per-slot controller type.  Values
    /// 0 and 4 are inactive/neutral; 5 is the human player in campaign maps
    /// and 6 is a computer.  Some maps still use the older 1..=3 values, so
    /// every non-zero/non-neutral slot is loaded.
    pub fn player_slots(&self) -> Vec<PlayerSlot> {
        let starts: Vec<u8> = self
            .units
            .iter()
            .filter(|u| u.unit_id == crate::ids::unit::START_LOCATION)
            .map(|u| u.owner)
            .collect();
        let placed = self.has_placed_force();
        let mut active = [false; 12];
        let mut count;
        if placed {
            // Campaign/use-map: every controlled OWNR slot is live, even when
            // it has no start location (Enslavers uses 0/3/4/6).
            let mut any_owner = false;
            for (i, &o) in self.owners.iter().enumerate().take(12) {
                if o != 0 {
                    any_owner = true;
                }
                active[i] = o != 0 && o != 4;
            }
            for &owner in &starts {
                if (owner as usize) < 12 {
                    active[owner as usize] = true;
                }
            }
            if !any_owner {
                for mu in &self.units {
                    if mu.unit_id != crate::ids::unit::START_LOCATION
                        && !crate::sim::is_resource_type(mu.unit_id)
                        && mu.owner < 8
                    {
                        active[mu.owner as usize] = true;
                    }
                }
            }
            count = active.iter().rposition(|&a| a).map(|m| m + 1).unwrap_or(2);
        } else {
            // Melee: the start locations define the seats.  OWNR can mark a
            // spare slot that has no start location; creating a base for it
            // would add a phantom player.
            for &owner in &starts {
                if (owner as usize) < 12 {
                    active[owner as usize] = true;
                }
            }
            if starts.is_empty() {
                for (i, &o) in self.owners.iter().enumerate().take(12) {
                    active[i] = o != 0 && o != 4;
                }
                for mu in &self.units {
                    if mu.unit_id != crate::ids::unit::START_LOCATION
                        && !crate::sim::is_resource_type(mu.unit_id)
                        && mu.owner < 8
                    {
                        active[mu.owner as usize] = true;
                    }
                }
            }
            count = starts
                .iter()
                .map(|&o| o as usize + 1)
                .max()
                .unwrap_or_else(|| active.iter().rposition(|&a| a).map(|m| m + 1).unwrap_or(2));
        }
        for owner in &starts {
            count = count.max(*owner as usize + 1);
        }
        count = count.clamp(2, 8);
        for a in active.iter_mut().skip(count) {
            *a = false;
        }
        if !active[..count].iter().any(|&a| a) {
            for a in active.iter_mut().take(count) {
                *a = true;
            }
        }
        // `OWNR` value 6 marks the human seat in the shipped campaign maps
        // (and 2 marks a human seat without a start location).  The seat is
        // not always slot 0: `Z1) Among the Ruins` puts it at slot 3 and
        // `Enslavers 01` at slot 6, so the client must follow OWNR instead of
        // assuming player 0.
        let human = active
            .iter()
            .enumerate()
            .position(|(slot, &a)| a && matches!(self.owners.get(slot), Some(2 | 6)))
            .or_else(|| {
                if active[0] {
                    Some(0)
                } else {
                    active.iter().position(|&a| a)
                }
            })
            .unwrap_or(0);
        (0..count)
            .map(|slot| PlayerSlot {
                slot,
                side: self.sides.get(slot).copied().unwrap_or(255),
                owner: self.owners.get(slot).copied().unwrap_or(0),
                active: active[slot],
                human: active[slot] && slot == human,
                computer: active[slot] && slot != human,
            })
            .collect()
    }

    /// A scenario that places a real force is a campaign/use-map mission and
    /// must not receive a stock melee base.
    pub fn has_placed_force(&self) -> bool {
        self.units.iter().any(|mu| {
            mu.unit_id != crate::ids::unit::START_LOCATION
                && !crate::sim::is_resource_type(mu.unit_id)
                && (mu.owner as usize) < 8
        })
    }

    /// `STR` string by its 1-based CHK id (trigger text and sound ids); 0 or
    /// out of range is `None`.
    pub fn string(&self, id: u32) -> Option<&str> {
        let index = (id as usize).checked_sub(1)?;
        self.strings.get(index).map(String::as_str)
    }

    /// Map title from `SPRP` (the map browser's name line).
    pub fn title(&self) -> Option<&str> {
        self.string(self.title_id as u32).filter(|s| !s.is_empty())
    }

    /// Map description from `SPRP` ("Eight-player cutthroat." for
    /// (8)The Hunters in the retail browser).
    pub fn description(&self) -> Option<&str> {
        self.string(self.description_id as u32)
            .filter(|s| !s.is_empty())
    }

    /// Mission copy authored in the `STR` section: string 0 is the name,
    /// string 1 the briefing, and the first later bullet paragraph the
    /// objectives.  Melee maps return `None`.
    pub fn briefing(&self) -> Option<Briefing> {
        let clean = |s: &str| s.replace("\r\n", "\n").replace('\r', "\n");
        let name = self.strings.first().map(|s| clean(s)).unwrap_or_default();
        let mut text = self.strings.get(1).map(|s| clean(s)).unwrap_or_default();
        if text.starts_with("(USE MAP SETTINGS)") {
            // StarEdit's place-holder header; the campaign UI never shows it.
            text = text
                .lines()
                .skip_while(|l| {
                    l.trim().is_empty()
                        || l.starts_with("(USE MAP SETTINGS)")
                        || l.contains("Level Campaign")
                })
                .collect::<Vec<_>>()
                .join("\n");
        }
        let objectives = self
            .strings
            .iter()
            .skip(2)
            .map(|s| clean(s))
            .find(|s| s.trim_start().starts_with("- ") && s.len() > 12)
            .unwrap_or_default();
        if text.trim().is_empty() && objectives.trim().is_empty() {
            None
        } else {
            Some(Briefing {
                name,
                text,
                objectives,
            })
        }
    }
}

/// Mission briefing text recovered from a scenario's `STR` section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Briefing {
    pub name: String,
    pub text: String,
    pub objectives: String,
}

/// Tileset walkability and buildability flags loaded from `tileset\*.vf4`.
#[derive(Clone, Debug)]
pub struct Terrain {
    pub width: usize,
    pub height: usize,
    /// Walk grid, four cells per tile edge (8 pixels per cell).
    pub walk_width: usize,
    pub walk_height: usize,
    walk: Vec<bool>,
    build: Vec<bool>,
    elevation: Vec<u8>,
    /// vf4 flag 0x8 per walk cell ("blocks view", OpenBW `flag_very_high`).
    view_block: Vec<bool>,
}

/// Bits of [`Terrain::tile_height_flags`].
pub const TILE_MIDDLE: u8 = 1;
pub const TILE_HIGH: u8 = 2;
pub const TILE_VERY_HIGH: u8 = 4;

/// Tileset tables the terrain grid is built from.  An MTXM entry is
/// `group << 4 | index`; `cv5` maps it to a megatile and `vf4` holds 16
/// minitile flag words per **megatile**.  Indexing `vf4` by the group made
/// almost every tile of (8)The Hunters walkable (its lakes included).
#[derive(Clone, Copy, Debug)]
pub struct TileData<'a> {
    pub cv5: Option<&'a [u8]>,
    pub vf4: &'a [u8],
}

impl<'a> TileData<'a> {
    pub fn new(cv5: &'a [u8], vf4: &'a [u8]) -> Self {
        Self {
            cv5: Some(cv5),
            vf4,
        }
    }

    /// Synthetic test tables without a cv5: the MTXM group number is the
    /// megatile record (fixtures number their `vf4` records that way).
    pub fn synthetic(vf4: &'a [u8]) -> Self {
        Self { cv5: None, vf4 }
    }

    /// Megatile of an MTXM entry.
    pub fn megatile(&self, mtxm: u16) -> usize {
        // Bit 15 is the creep marker; bits 4..14 are the CV5 group.
        let group = ((mtxm >> 4) & 0x7ff) as usize;
        match self.cv5 {
            Some(cv5) => u16_at(cv5, group * 52 + 20 + (mtxm & 0x0f) as usize * 2) as usize,
            None => group,
        }
    }

    /// cv5 group flag 0x80: the tile is unbuildable (OpenBW
    /// `tile_t::flag_unbuildable`).  Synthetic tables have no flags.
    pub fn unbuildable(&self, mtxm: u16) -> bool {
        let group = ((mtxm >> 4) & 0x7ff) as usize;
        self.cv5.is_some_and(|cv5| {
            cv5.get(group * 52 + 2)
                .is_some_and(|&buildability| buildability & 0x80 != 0)
        })
    }
}

/// Owned `cv5` + `vf4` of one tileset.
#[derive(Clone, Debug, Default)]
pub struct TileTables {
    pub cv5: Vec<u8>,
    pub vf4: Vec<u8>,
}

impl TileTables {
    /// Read a `.vf4` and its `.cv5` sibling.
    pub fn read_vf4(path: &std::path::Path) -> std::io::Result<Self> {
        Ok(Self {
            vf4: std::fs::read(path)?,
            cv5: std::fs::read(path.with_extension("cv5"))?,
        })
    }

    pub fn data(&self) -> TileData<'_> {
        TileData::new(&self.cv5, &self.vf4)
    }
}

impl Terrain {
    /// Build a terrain grid from a scenario and its tileset tables.
    pub fn from_scenario(scenario: &Scenario, tiles: &TileData) -> Self {
        let vf4 = tiles.vf4;
        let walk_width = scenario.width * 4;
        let walk_height = scenario.height * 4;
        let mut walk = vec![false; walk_width * walk_height];
        let mut build = vec![false; walk_width * walk_height];
        let mut elevation = vec![0u8; walk_width * walk_height];
        let mut view_block = vec![false; walk_width * walk_height];
        for ty in 0..scenario.height {
            for tx in 0..scenario.width {
                let tile = scenario.tiles[ty * scenario.width + tx];
                let record = tiles.megatile(tile) * 32;
                let unbuildable = tiles.unbuildable(tile);
                let mut walkable_count = 0u8;
                for my in 0..4 {
                    for mx in 0..4 {
                        let idx = (ty * 4 + my) * walk_width + tx * 4 + mx;
                        if record + 32 <= vf4.len() {
                            let flag = u16_at(vf4, record + (my * 4 + mx) * 2);
                            walk[idx] = flag & 1 != 0;
                            walkable_count += (flag & 1 != 0) as u8;
                            // Bit 1/2 are elevation (middle/high), not a
                            // buildability veto.  Buildability is a whole
                            // 32x32 tile: partially walkable tiles are not
                            // buildable (OpenBW can_place_building).
                            build[idx] = false;
                            view_block[idx] = flag & 8 != 0;
                            elevation[idx] = if flag & 4 != 0 {
                                2
                            } else if flag & 2 != 0 {
                                1
                            } else {
                                0
                            };
                        }
                    }
                }
                if walkable_count == 16 && !unbuildable {
                    for my in 0..4 {
                        for mx in 0..4 {
                            let idx = (ty * 4 + my) * walk_width + tx * 4 + mx;
                            build[idx] = true;
                        }
                    }
                }
            }
        }
        // CHK MASK is the initial fog layer, not a buildability veto.
        // A clear bit means pre-explored; terrain buildability remains
        // walkability minus the CV5 unbuildable flag.
        Self {
            width: scenario.width,
            height: scenario.height,
            walk_width,
            walk_height,
            walk,
            build,
            elevation,
            view_block,
        }
    }

    /// A terrain with every cell walkable/buildable, useful for unit tests.
    pub fn open(width_tiles: usize, height_tiles: usize) -> Self {
        let walk_width = width_tiles * 4;
        let walk_height = height_tiles * 4;
        Self {
            width: width_tiles,
            height: height_tiles,
            walk_width,
            walk_height,
            walk: vec![true; walk_width * walk_height],
            build: vec![true; walk_width * walk_height],
            elevation: vec![0; walk_width * walk_height],
            view_block: vec![false; walk_width * walk_height],
        }
    }

    #[inline]
    pub fn walk_index(&self, wx: i32, wy: i32) -> Option<usize> {
        if wx < 0 || wy < 0 || wx as usize >= self.walk_width || wy as usize >= self.walk_height {
            return None;
        }
        Some(wy as usize * self.walk_width + wx as usize)
    }

    pub fn walkable_cell(&self, wx: i32, wy: i32) -> bool {
        self.walk_index(wx, wy).is_some_and(|i| self.walk[i])
    }

    /// Retail tile height flags (OpenBW `set_mega_tile_flags`): bit 0
    /// middle (middle + high cells >= 12 of 16, high < 12), bit 1 high
    /// (>= 12 high cells), bit 2 very high (any view-blocking cell).
    pub fn tile_height_flags(&self, tx: i32, ty: i32) -> u8 {
        let (mut middle, mut high, mut very) = (0, 0, false);
        for my in 0..4 {
            for mx in 0..4 {
                if let Some(i) = self.walk_index(tx * 4 + mx, ty * 4 + my) {
                    match self.elevation[i] {
                        2 => high += 1,
                        1 => middle += 1,
                        _ => {}
                    }
                    very |= self.view_block[i];
                }
            }
        }
        let mut f = 0;
        if high < 12 && middle + high >= 12 {
            f |= TILE_MIDDLE;
        }
        if high >= 12 {
            f |= TILE_HIGH;
        }
        if very {
            f |= TILE_VERY_HIGH;
        }
        f
    }

    /// Retail `tile_can_have_creep`: the tile is buildable and fully
    /// walkable (not partially), and the tile below it is buildable.
    pub fn tile_can_have_creep(&self, tx: i32, ty: i32) -> bool {
        let tile_ok = |tx: i32, ty: i32, need_walk: bool| {
            let mut walk = 0;
            for my in 0..4 {
                for mx in 0..4 {
                    let (wx, wy) = (tx * 4 + mx, ty * 4 + my);
                    if !self.buildable_cell(wx, wy) {
                        return false;
                    }
                    walk += self.walkable_cell(wx, wy) as i32;
                }
            }
            !need_walk || walk == 16
        };
        if tx < 0 || ty < 0 || tx as usize >= self.width || ty as usize >= self.height {
            return false;
        }
        tile_ok(tx, ty, true) && (ty as usize + 1 == self.height || tile_ok(tx, ty + 1, false))
    }
}

/// Normalize a `Next Scenario` string or bundled filename for matching:
/// strip the directory, extension, leading `(N)` player marker and every
/// non-alphanumeric character.  `"(1)Enslavers02a.scm"` and `"Enslavers 02a"`
/// both become `"enslavers02a"`.
pub fn scenario_file_key(name: &str) -> String {
    let file = name.rsplit(['\\', '/']).next().unwrap_or(name);
    let stem = file.rsplit_once('.').map(|(s, _)| s).unwrap_or(file);
    let stem = if stem.starts_with('(') {
        stem.find(')').map(|i| &stem[i + 1..]).unwrap_or(stem)
    } else {
        stem
    };
    stem.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// One retail player slot derived from `OWNR`/`SIDE`.#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerSlot {
    pub slot: usize,
    /// `SIDE` race byte: 0 Zerg, 1 Terran, 2 Protoss, higher values random.
    pub side: u8,
    pub owner: u8,
    pub active: bool,
    pub human: bool,
    pub computer: bool,
}

impl PlayerSlot {
    /// Race for this slot, using `fallback` for random/unused SIDE values.
    pub fn race(&self, fallback: crate::sim::Race) -> crate::sim::Race {
        match self.side {
            0 => crate::sim::Race::Zerg,
            1 => crate::sim::Race::Terran,
            2 => crate::sim::Race::Protoss,
            _ => fallback,
        }
    }
}

impl Terrain {
    /// Toggle walkability of one 8-pixel cell.  Used by map editors and by
    /// tests that need to carve a wall without a full `scenario.chk`.
    pub fn set_walkable_cell(&mut self, wx: i32, wy: i32, walkable: bool) {
        if let Some(i) = self.walk_index(wx, wy) {
            self.walk[i] = walkable;
            self.build[i] = walkable;
        }
    }

    /// Set the elevation class of one 8-pixel cell (`0` low, `1` middle,
    /// `2` high).  Mirrors [`Terrain::set_walkable_cell`] for tests and tools
    /// that need a synthetic cliff.
    pub fn set_elevation_cell(&mut self, wx: i32, wy: i32, elevation: u8) {
        if let Some(i) = self.walk_index(wx, wy) {
            self.elevation[i] = elevation.min(2);
        }
    }

    /// Number of 8-connected ground regions.  The retail region builder
    /// decomposes the walk grid the same way before building its region graph.
    pub fn walkable_regions(&self) -> usize {
        let n = self.walk.len();
        let mut seen = vec![false; n];
        let mut regions = 0usize;
        let w = self.walk_width as i32;
        let h = self.walk_height as i32;
        let mut stack = Vec::new();
        for start in 0..n {
            if !self.walk[start] || seen[start] {
                continue;
            }
            regions += 1;
            seen[start] = true;
            stack.push(start as i32);
            while let Some(i) = stack.pop() {
                let x = i % w;
                let y = i / w;
                for (dx, dy) in [
                    (-1, 0),
                    (1, 0),
                    (0, -1),
                    (0, 1),
                    (-1, -1),
                    (1, -1),
                    (-1, 1),
                    (1, 1),
                ] {
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        continue;
                    }
                    let ni = (ny * w + nx) as usize;
                    if self.walk[ni] && !seen[ni] {
                        seen[ni] = true;
                        stack.push(ni as i32);
                    }
                }
            }
        }
        regions.max(1)
    }

    pub fn buildable_cell(&self, wx: i32, wy: i32) -> bool {
        self.walk_index(wx, wy).is_some_and(|i| self.build[i])
    }

    /// Query by absolute pixel position.
    /// `0x00750ae0`: a unit rectangle is inside the map when its left/top
    /// are non-negative, its right is below the map width and its bottom is
    /// below `height - 0x20` (the retail bottom margin).
    pub fn rect_in_bounds(&self, rect: (i32, i32, i32, i32)) -> bool {
        let (left, top, right, bottom) = rect;
        left >= 0
            && right < self.width as i32 * TILE_PX
            && top >= 0
            && bottom < self.height as i32 * TILE_PX - 0x20
    }

    /// `0x00750aec`: upper/right bounds variant used by the placement
    /// search.  It checks the right edge, top and bottom margin.
    pub fn rect_upper_in_bounds(&self, rect: (i32, i32, i32, i32)) -> bool {
        let (_, top, right, bottom) = rect;
        right < self.width as i32 * TILE_PX
            && top > -1
            && bottom < self.height as i32 * TILE_PX - 0x20
    }

    pub fn walkable_px(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 {
            return false;
        }
        self.walkable_cell(x / WALK_TILE_PX, y / WALK_TILE_PX)
    }

    pub fn buildable_px(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 {
            return false;
        }
        self.buildable_cell(x / WALK_TILE_PX, y / WALK_TILE_PX)
    }

    pub fn elevation_at(&self, wx: i32, wy: i32) -> u8 {
        self.walk_index(wx, wy)
            .map(|i| self.elevation[i])
            .unwrap_or(0)
    }

    pub fn elevation_px(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 {
            return 0;
        }
        self.elevation_at(x / WALK_TILE_PX, y / WALK_TILE_PX)
    }

    /// Whether a footprint is entirely walkable/buildable.
    pub fn footprint_walkable(&self, left: i32, top: i32, right: i32, bottom: i32) -> bool {
        if left < 0 || top < 0 || right < left || bottom < top {
            return false;
        }
        let x0 = left / WALK_TILE_PX;
        let y0 = top / WALK_TILE_PX;
        let x1 = (right - 1).max(left) / WALK_TILE_PX;
        let y1 = (bottom - 1).max(top) / WALK_TILE_PX;
        for wy in y0..=y1 {
            for wx in x0..=x1 {
                if !self.walkable_cell(wx, wy) {
                    return false;
                }
            }
        }
        true
    }

    pub fn footprint_buildable(&self, left: i32, top: i32, right: i32, bottom: i32) -> bool {
        if left < 0 || top < 0 || right < left || bottom < top {
            return false;
        }
        let x0 = left / WALK_TILE_PX;
        let y0 = top / WALK_TILE_PX;
        let x1 = (right - 1).max(left) / WALK_TILE_PX;
        let y1 = (bottom - 1).max(top) / WALK_TILE_PX;
        for wy in y0..=y1 {
            for wx in x0..=x1 {
                if !self.buildable_cell(wx, wy) {
                    return false;
                }
            }
        }
        true
    }

    /// Approximate the original unit footprint for a ground unit: the
    /// `units.dat` placement box, clipped to a whole number of walk cells.
    pub fn unit_footprint(
        center_x: i32,
        center_y: i32,
        half_width: i32,
        half_height: i32,
    ) -> (i32, i32, i32, i32) {
        (
            center_x - half_width,
            center_y - half_height,
            center_x + half_width,
            center_y + half_height,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start(owner: u8, x: i32, y: i32) -> MapUnit {
        MapUnit {
            serial: 0,
            unit_id: 214,
            x,
            y,
            owner,
            states_set: 0,
            fields_set: 0,
            hit_point_percent: 100,
            shield_percent: 100,
            energy_percent: 100,
            resource_amount: 0,
            hangar: 0,
            states: 0,
            link_serial: 0,
        }
    }

    #[test]
    fn start_locations_are_looked_up_by_owner_not_file_order() {
        // (8)The Hunters stores owner 7 before owners 3..6.
        let sc = Scenario {
            units: vec![
                start(0, 384, 240),
                start(7, 320, 1552),
                start(3, 3712, 2608),
            ],
            ..Scenario::default()
        };
        assert_eq!(sc.start_location_of(7), Some((320, 1552)));
        assert_eq!(sc.start_location_of(3), Some((3712, 2608)));
        assert_eq!(sc.start_location_of(5), None);
        assert_eq!(sc.start_locations()[1], (320, 1552), "file order");
    }

    /// Build a minimal 2x2 CHK with one MTXM tile.
    fn tiny_chk() -> Vec<u8> {
        let mut out = Vec::new();
        let mut push = |name: &[u8; 4], data: &[u8]| {
            out.extend_from_slice(name);
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(data);
        };
        push(
            b"DIM ",
            &2u16
                .to_le_bytes()
                .into_iter()
                .chain(2u16.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        push(b"ERA ", &1u16.to_le_bytes());
        let mut mtxm = Vec::new();
        for _ in 0..4 {
            mtxm.extend_from_slice(&32u16.to_le_bytes());
        }
        push(b"MTXM", &mtxm);
        push(b"MASK", &[1, 1, 1, 1]);
        out
    }

    #[test]
    fn str_strings_end_at_their_nul_even_when_offsets_are_out_of_order() {
        // Three strings; the table lists them out of file order (string 2
        // is stored after string 3), as StarEdit writes them.  Ending a
        // string at the next entry's offset made string 2 empty.
        let mut str_ = Vec::new();
        str_.extend_from_slice(&3u16.to_le_bytes());
        let base = 2 + 3 * 2;
        let one = b"Name\0";
        let three = b"staredit\\wav\\combeep0.wav\0";
        let two = b"Jim Raynor: Move out!\0";
        let o1 = base;
        let o3 = o1 + one.len();
        let o2 = o3 + three.len();
        for o in [o1, o2, o3] {
            str_.extend_from_slice(&(o as u16).to_le_bytes());
        }
        str_.extend_from_slice(one);
        str_.extend_from_slice(three);
        str_.extend_from_slice(two);
        let mut chk = tiny_chk();
        chk.extend_from_slice(b"STR ");
        chk.extend_from_slice(&(str_.len() as u32).to_le_bytes());
        chk.extend_from_slice(&str_);
        let s = Scenario::parse(&chk).unwrap();
        assert_eq!(s.string(1), Some("Name"));
        assert_eq!(s.string(2), Some("Jim Raynor: Move out!"));
        assert_eq!(s.string(3), Some("staredit\\wav\\combeep0.wav"));
        assert_eq!(s.string(0), None);
        assert_eq!(s.string(4), None);
        // SPRP names the title/description strings (here 2 and 1).
        assert_eq!(s.title(), None, "no SPRP yet");
        chk.extend_from_slice(b"SPRP");
        chk.extend_from_slice(&4u32.to_le_bytes());
        chk.extend_from_slice(&2u16.to_le_bytes());
        chk.extend_from_slice(&1u16.to_le_bytes());
        let s = Scenario::parse(&chk).unwrap();
        assert_eq!(s.title(), Some("Jim Raynor: Move out!"));
        assert_eq!(s.description(), Some("Name"));
    }

    #[test]
    fn strx_strings_use_32_bit_offsets() {
        let mut strx = Vec::new();
        let one = b"Name\0";
        let two = b"Briefing\0";
        strx.extend_from_slice(&2u32.to_le_bytes());
        let base = 4 + 2 * 4;
        strx.extend_from_slice(&(base as u32).to_le_bytes());
        strx.extend_from_slice(&((base + one.len()) as u32).to_le_bytes());
        strx.extend_from_slice(one);
        strx.extend_from_slice(two);
        let mut chk = tiny_chk();
        chk.extend_from_slice(b"STRx");
        chk.extend_from_slice(&(strx.len() as u32).to_le_bytes());
        chk.extend_from_slice(&strx);
        chk.extend_from_slice(b"SPRP");
        chk.extend_from_slice(&4u32.to_le_bytes());
        chk.extend_from_slice(&2u16.to_le_bytes());
        chk.extend_from_slice(&1u16.to_le_bytes());
        let s = Scenario::parse(&chk).unwrap();
        assert_eq!(s.string(1), Some("Name"));
        assert_eq!(s.string(2), Some("Briefing"));
        assert_eq!(s.title(), Some("Briefing"));
        assert_eq!(s.description(), Some("Name"));
    }

    #[test]
    fn parses_dimensions_and_tiles() {
        let s = Scenario::parse(&tiny_chk()).unwrap();
        assert_eq!((s.width, s.height), (2, 2));
        assert_eq!(s.era, 1);
        assert_eq!(s.tiles, vec![32; 4]);
    }

    #[test]
    fn protected_map_sections_are_recovered() {
        // Distributed ladder maps are wrapped by tools that scramble section
        // order, rename optional sections and replace the leading header with
        // junk.  The sections StarCraft needs are intact; the parser must
        // resynchronise instead of failing on the first damaged header.
        let mut out = Vec::new();
        out.extend_from_slice(b"JUNK");
        out.extend_from_slice(&u32::MAX.to_le_bytes()); // impossible size
        let push = |out: &mut Vec<u8>, name: &[u8; 4], data: &[u8]| {
            out.extend_from_slice(name);
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(data);
        };
        let mut dim = Vec::new();
        dim.extend_from_slice(&2u16.to_le_bytes());
        dim.extend_from_slice(&2u16.to_le_bytes());
        push(&mut out, b"UPGx", &[0u8; 198]); // scrambled optional section
        let mut unit = vec![0u8; 36];
        unit[8..10].copy_from_slice(&37u16.to_le_bytes()); // SCV
        unit[18] = 0;
        push(&mut out, b"UNIT", &unit);
        // Remastered stores flags in the high bits of ERA; the low three bits
        // select the tileset.
        push(&mut out, b"ERA ", &0xF1u16.to_le_bytes());
        push(&mut out, b"DIM ", &dim);
        let mut mtxm = Vec::new();
        for _ in 0..4 {
            mtxm.extend_from_slice(&32u16.to_le_bytes());
        }
        push(&mut out, b"MTXM", &mtxm);
        push(&mut out, b"MASK", &[1, 1, 0, 1]);
        let mut trig = vec![0u8; 2400];
        trig[15] = 22; // always
        trig[320 + 26] = 1; // victory
        trig[2372] = 1;
        push(&mut out, b"TRIG", &trig);

        let s = Scenario::parse(&out).unwrap();
        assert_eq!((s.width, s.height), (2, 2));
        assert_eq!(s.era, 1, "ERA high bits are flags, low bits the tileset");
        assert_eq!(s.tiles, vec![32; 4]);
        assert_eq!(s.units.len(), 1);
        assert_eq!(s.units[0].unit_id, 37);
        assert_eq!(s.triggers.len(), 2400);
    }

    #[test]
    fn cv5_resolves_the_megatile_whose_vf4_flags_apply() {
        // tiny_chk's tile is 32 = group 2, index 0.  The cv5 maps group 2
        // index 0 to megatile 1; only megatile 1's record is walkable.
        let s = Scenario::parse(&tiny_chk()).unwrap();
        let mut cv5 = vec![0u8; 52 * 3];
        cv5[2 * 52 + 20] = 1;
        let mut vf4 = vec![0u8; 96];
        for i in 0..16 {
            vf4[32 + i * 2] = 1;
        }
        let tiles = TileData::new(&cv5, &vf4);
        assert_eq!(tiles.megatile(32), 1);
        assert!(Terrain::from_scenario(&s, &tiles).walkable_px(0, 0));
        // Indexing by the group (record 2) would read the unwalkable record.
        assert!(!Terrain::from_scenario(&s, &TileData::synthetic(&vf4)).walkable_px(0, 0));
    }

    #[test]
    fn vf4_flags_define_walkability() {
        let s = Scenario::parse(&tiny_chk()).unwrap();
        // Megatile 2 (tile 32 >> 4) is at record offset 64.  Make every
        // minitile walkable in the first entry and unbuildable otherwise.
        let mut vf4 = vec![0u8; 96];
        for i in 0..16 {
            vf4[64 + i * 2] = 1;
        }
        let t = Terrain::from_scenario(&s, &TileData::synthetic(&vf4));
        assert!(t.walkable_px(0, 0));
        assert!(t.walkable_px(63, 63));
        assert!(t.buildable_px(0, 0));
    }

    #[test]
    fn open_terrain_is_fully_open() {
        let t = Terrain::open(4, 4);
        assert!(t.walkable_px(0, 0));
        assert!(t.walkable_px(127, 127));
        assert!(!t.walkable_px(128, 0));
    }

    #[test]
    fn fog_mask_does_not_veto_terrain_buildability() {
        // tiny_chk's tile is megatile group 2, record 64.  Make all 16
        // minitiles walkable; a clear MASK cell must not disable them.
        let mut s = Scenario::parse(&tiny_chk()).unwrap();
        s.mask = vec![0; 4];
        let mut vf4 = vec![0u8; 96];
        for i in 0..16 {
            vf4[64 + i * 2] = 1;
        }
        let t = Terrain::from_scenario(&s, &TileData::synthetic(&vf4));
        assert!(t.buildable_px(0, 0));
        assert!(t.buildable_px(31, 31));
        assert!(t.buildable_px(63, 63));
    }

    #[test]
    fn unis_unix_upgs_tecs_and_puni_parse_at_their_retail_offsets() {
        let mut chk = tiny_chk();
        let mut unis = vec![1u8; 0xFD0];
        unis[0] = 0;
        unis[0x0E4..0x0E8].copy_from_slice(&5120u32.to_le_bytes());
        unis[0x474..0x476].copy_from_slice(&50u16.to_le_bytes());
        unis[0x63C] = 2;
        unis[0x720..0x722].copy_from_slice(&300u16.to_le_bytes());
        unis[0x8E8..0x8EA].copy_from_slice(&25u16.to_le_bytes());
        unis[0xC78..0xC7A].copy_from_slice(&5u16.to_le_bytes());
        unis[0xE40..0xE42].copy_from_slice(&8u16.to_le_bytes());
        unis[0xE40 + 100 * 2..0xE40 + 100 * 2 + 2].copy_from_slice(&2u16.to_le_bytes());
        chk.extend_from_slice(b"UNIS");
        chk.extend_from_slice(&(unis.len() as u32).to_le_bytes());
        chk.extend_from_slice(&unis);

        let mut unix = vec![1u8; 0x1048];
        unix[0] = 0;
        unix[0xE40 + 129 * 2..0xE40 + 129 * 2 + 2].copy_from_slice(&77u16.to_le_bytes());
        unix[0xE40 + 130 * 2 + 129 * 2..0xE40 + 130 * 2 + 129 * 2 + 2]
            .copy_from_slice(&9u16.to_le_bytes());
        chk.extend_from_slice(b"UNIx");
        chk.extend_from_slice(&(unix.len() as u32).to_le_bytes());
        chk.extend_from_slice(&unix);

        let mut upgs = vec![1u8; 0x256];
        upgs[7] = 0;
        upgs[0x02E + 2 * 7..0x02E + 2 * 7 + 2].copy_from_slice(&100u16.to_le_bytes());
        upgs[0x08A + 2 * 7..0x08A + 2 * 7 + 2].copy_from_slice(&10u16.to_le_bytes());
        chk.extend_from_slice(b"UPGS");
        chk.extend_from_slice(&(upgs.len() as u32).to_le_bytes());
        chk.extend_from_slice(&upgs);

        let mut tecs = vec![1u8; 0xD8];
        tecs[0] = 0;
        tecs[0x18..0x1A].copy_from_slice(&150u16.to_le_bytes());
        tecs[0xA8..0xAA].copy_from_slice(&50u16.to_le_bytes());
        chk.extend_from_slice(b"TECS");
        chk.extend_from_slice(&(tecs.len() as u32).to_le_bytes());
        chk.extend_from_slice(&tecs);

        let mut puni = vec![1u8; 0x1644];
        puni[37] = 0; // player 0, unit 37 denied
        puni[0xB94 + 37] = 0; // but this player uses the per-player byte
        chk.extend_from_slice(b"PUNI");
        chk.extend_from_slice(&(puni.len() as u32).to_le_bytes());
        chk.extend_from_slice(&puni);

        let s = Scenario::parse(&chk).unwrap();
        let u = s.unis.expect("UNIS");
        assert_eq!(u.use_default[0], 0);
        assert_eq!(u.hit_points[0], 5120);
        assert_eq!(u.shields[0], 50);
        assert_eq!(u.armor[0], 2);
        assert_eq!(u.build_time[0], 300);
        assert_eq!(u.mineral_cost[0], 25);
        assert_eq!(u.string_id[0], 5);
        assert_eq!(u.weapon_damage[0], 8);
        assert_eq!(u.weapon_bonus[0], 2);
        let x = s.unix.expect("UNIx");
        assert_eq!(x.weapon_damage.len(), 130);
        assert_eq!(x.weapon_damage[129], 77);
        assert_eq!(x.weapon_bonus[129], 9);
        assert_eq!(s.upgs.as_ref().unwrap().mineral_base[7], 100);
        assert_eq!(s.tecs.as_ref().unwrap().mineral_cost[0], 150);
        assert_eq!(s.tecs.as_ref().unwrap().energy_cost[0], 50);
        assert!(!s.puni.as_ref().unwrap().available(0, 37));
        assert!(s.puni.as_ref().unwrap().available(1, 37));
    }

    #[test]
    fn ptec_ptex_upgr_and_pupx_parse_per_player_blocks() {
        let mut chk = tiny_chk();
        let mut ptec = vec![1u8; 0x390];
        ptec[0] = 0; // player 0 tech 0 unavailable
        ptec[24] = 1; // player 0 tech 0 researched
        ptec[0x270] = 0; // player 0 tech 0 uses its own byte
        chk.extend_from_slice(b"PTEC");
        chk.extend_from_slice(&(ptec.len() as u32).to_le_bytes());
        chk.extend_from_slice(&ptec);

        let mut ptex = vec![1u8; 0x688];
        ptex[43] = 0; // player 0 tech 43 unavailable
        ptex[0x478 + 43] = 0; // use the per-player byte
        chk.extend_from_slice(b"PTEx");
        chk.extend_from_slice(&(ptex.len() as u32).to_le_bytes());
        chk.extend_from_slice(&ptex);

        let mut upgr = vec![1u8; 0x6D4];
        upgr[7] = 2; // player 0 upgrade 7 max
        upgr[0x228 + 7] = 1; // player 0 upgrade 7 start
        upgr[0x4AC + 7] = 0; // use per-player
        chk.extend_from_slice(b"UPGR");
        chk.extend_from_slice(&(upgr.len() as u32).to_le_bytes());
        chk.extend_from_slice(&upgr);

        let mut pupx = vec![1u8; 0x90E];
        pupx[60] = 4;
        pupx[0x2DC + 60] = 3;
        pupx[0x632 + 60] = 0;
        chk.extend_from_slice(b"PUPx");
        chk.extend_from_slice(&(pupx.len() as u32).to_le_bytes());
        chk.extend_from_slice(&pupx);

        let s = Scenario::parse(&chk).unwrap();
        let t = s.ptec.as_ref().unwrap();
        assert!(!t.available(0, 0));
        assert!(t.researched(0, 0));
        assert!(t.available(1, 0));
        let tx = s.ptex.as_ref().unwrap();
        assert!(!tx.available(0, 43));
        assert!(tx.available(0, 42));
        let u = s.upgr.as_ref().unwrap();
        assert_eq!(u.max_level(0, 7), 2);
        assert_eq!(u.start_level(0, 7), 1);
        let ux = s.pupx.as_ref().unwrap();
        assert_eq!(ux.max_level(0, 60), 4);
        assert_eq!(ux.start_level(0, 60), 3);
    }

    #[test]
    fn cv5_buildability_uses_the_byte_at_offset_two_and_ignores_creep_bit() {
        let mut cv5 = vec![0u8; 52 * 2];
        cv5[52 + 2] = 0x80;
        for slot in 0..16 {
            let at = 52 + 20 + slot * 2;
            cv5[at..at + 2].copy_from_slice(&7u16.to_le_bytes());
        }
        let tiles = TileData::new(&cv5, &[]);
        assert!(!tiles.unbuildable(0x0000));
        assert!(tiles.unbuildable(0x0010));
        assert!(tiles.unbuildable(0x8010), "bit 15 is the creep marker");
        assert_eq!(tiles.megatile(0x8010), 7);
    }

    #[test]
    fn thg2_records_are_ten_bytes_and_keep_their_flags() {
        let mut chk = tiny_chk();
        let mut thg2 = Vec::new();
        for (id, flags) in [(0xcdu16, 0x6280u16), (0x00cbu16, 0xa280u16)] {
            thg2.extend_from_slice(&id.to_le_bytes());
            thg2.extend_from_slice(&320u16.to_le_bytes());
            thg2.extend_from_slice(&320u16.to_le_bytes());
            thg2.push(0);
            thg2.push(0);
            thg2.extend_from_slice(&flags.to_le_bytes());
        }
        chk.extend_from_slice(b"THG2");
        chk.extend_from_slice(&(thg2.len() as u32).to_le_bytes());
        chk.extend_from_slice(&thg2);
        let s = Scenario::parse(&chk).unwrap();
        assert_eq!(s.thingies.len(), 2);
        assert_eq!(s.thingies[0].unit_id, 0xcd);
        assert_eq!(s.thingies[0].flags, 0x6280);
        assert_eq!(s.thingies[1].unit_id, 0xcb);
        assert_eq!(s.thingies[1].flags, 0xa280);
    }

    #[test]
    fn rect_in_bounds_matches_the_retail_margin() {
        let t = Terrain::open(64, 64);
        assert!(t.rect_in_bounds((0, 0, 32, 32)));
        assert!(!t.rect_in_bounds((-1, 0, 32, 32)));
        assert!(!t.rect_in_bounds((0, 0, 64 * 32, 32)));
        assert!(!t.rect_in_bounds((0, 0, 32, 64 * 32 - 1)));
    }

    #[test]
    fn rect_upper_in_bounds_matches_the_second_retail_check() {
        let t = Terrain::open(64, 64);
        assert!(t.rect_upper_in_bounds((-100, 0, 32, 32)));
        assert!(!t.rect_upper_in_bounds((0, -1, 32, 32)));
        assert!(!t.rect_upper_in_bounds((0, 0, 64 * 32, 32)));
        assert!(!t.rect_upper_in_bounds((0, 0, 32, 64 * 32 - 1)));
    }
}
