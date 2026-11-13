//! Retail unit voice sets.
//!
//! `arr\sfxdata.tbl` lists every shipped voice file.  The unit folders are
//! stable (`sound\terran\marine`, `sound\protoss\zealot`, ...) and the file
//! stems encode the cue: `...Rdy00` ready/selected, `...Wht00` order
//! acknowledgement, `...Pss00` annoyed (repeated clicks), `...Yes00` confirm,
//! `...Dth00` death, `...Atk00` attack.
//!
//! The classic client selects folder + cue from the unit type.  This module
//! owns that mapping and the lazy WAV cache; [`crate::audio::SoundBank`] still
//! owns the small UI/effect bank.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::audio::Wav;
use crate::ids::unit as uid;
use crate::sfx::SfxTable;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VoiceCue {
    Ready,
    What,
    Pissed,
    Yes,
    Death,
    Attack,
    Hit,
    Special,
}

impl VoiceCue {
    pub fn from_stem(stem: &str) -> Option<Self> {
        let stem = stem.to_ascii_lowercase();
        if stem.contains("rdy") {
            Some(Self::Ready)
        } else if stem.contains("wht") {
            Some(Self::What)
        } else if stem.contains("pss") {
            Some(Self::Pissed)
        } else if stem.contains("yes") {
            Some(Self::Yes)
        } else if stem.contains("dth") || stem.contains("die") {
            Some(Self::Death)
        } else if stem.contains("atk")
            || stem.contains("att")
            || stem.contains("fir")
            || stem.contains("min")
        {
            Some(Self::Attack)
        } else if stem.contains("hit") {
            Some(Self::Hit)
        } else if stem.contains("rag")
            || stem.contains("clo")
            || stem.contains("sti")
            || stem.contains("las")
            || stem.contains("lkd")
        {
            Some(Self::Special)
        } else {
            None
        }
    }
}

/// Unit type -> retail voice folder.  Only folders that ship are listed;
/// units without voices (turrets, mines, most buildings) return `None`.
pub fn folder_for(unit_id: u16) -> Option<&'static str> {
    Some(match unit_id {
        uid::MARINE => "terran/marine",
        uid::GHOST => "terran/ghost",
        uid::VULTURE => "terran/vulture",
        uid::GOLIATH | uid::GOLIATH_TURRET => "terran/goliath",
        uid::SIEGE_TANK | uid::SIEGE_MODE | uid::SIEGE_TANK_TURRET | uid::SIEGE_MODE_TURRET => {
            "terran/tank"
        }
        uid::SCV => "terran/scv",
        uid::WRAITH => "terran/phoenix",
        uid::SCIENCE_VESSEL => "terran/vessel",
        uid::DROPSHIP => "terran/dropship",
        uid::BATTLECRUISER => "terran/battle",
        uid::FIREBAT => "terran/firebat",
        uid::MEDIC => "terran/medic",
        uid::LARVA => "zerg/larva",
        uid::EGG | uid::COCOON | uid::LURKER_EGG => "zerg/egg",
        uid::ZERGLING => "zerg/zergling",
        uid::HYDRALISK => "zerg/hydra",
        uid::ULTRALISK => "zerg/ultra",
        uid::BROODLING => "zerg/broodling",
        uid::DRONE => "zerg/drone",
        uid::OVERLORD => "zerg/overlord",
        uid::MUTALISK => "zerg/mutalid",
        uid::GUARDIAN => "zerg/guardian",
        uid::QUEEN => "zerg/queen",
        uid::DEFILER => "zerg/defiler",
        uid::SCOURGE => "zerg/bugguy",
        uid::DEVOURER => "zerg/devourer",
        uid::LURKER => "zerg/lurker",
        uid::PROBE => "protoss/probe",
        uid::ZEALOT => "protoss/zealot",
        uid::DRAGOON => "protoss/dragoon",
        uid::HIGH_TEMPLAR => "protoss/templar",
        uid::ARCHON => "protoss/archon",
        uid::SHUTTLE => "protoss/shuttle",
        uid::SCOUT => "protoss/scout",
        uid::ARBITER => "protoss/arbiter",
        uid::CARRIER => "protoss/carrier",
        uid::INTERCEPTOR => "protoss/intercep",
        uid::REAVER => "protoss/trilobyte",
        uid::OBSERVER => "protoss/witness",
        uid::CORSAIR => "protoss/corsair",
        uid::DARK_TEMPLAR => "protoss/darktemplar",
        uid::DARK_ARCHON => "protoss/darchon",
        // Buildings share the race building voice set.
        106..=129 => "terran/bldg",
        130..=153 => "zerg/bldg",
        154..=175 => "protoss/bldg",
        _ => return None,
    })
}

/// Voice id (index into `sfxdata.tbl`) for a cue, preferring the first file
/// in the shipped folder order.  `variant` cycles through the alternatives so
/// repeated orders do not play the same take.
pub fn voice_id(table: &SfxTable, unit_id: u16, cue: VoiceCue, variant: usize) -> Option<u16> {
    let ids = voice_ids(table, unit_id, cue);
    if ids.is_empty() {
        return None;
    }
    Some(ids[variant % ids.len()])
}

/// Every take of one cue for a unit, in file order.
pub fn voice_ids(table: &SfxTable, unit_id: u16, cue: VoiceCue) -> Vec<u16> {
    let Some(folder) = folder_for(unit_id) else {
        return Vec::new();
    };
    let mut ids: Vec<u16> = Vec::new();
    for id in 0..table.len() as u16 {
        if table.folder(id).as_deref() != Some(folder) {
            continue;
        }
        let Some(path) = table.cache_path(id) else {
            continue;
        };
        let stem = path
            .rsplit('/')
            .next()
            .unwrap_or("")
            .split('.')
            .next()
            .unwrap_or("");
        if VoiceCue::from_stem(stem) == Some(cue) {
            ids.push(id);
        }
    }
    ids
}

/// Clicks on the same unit that answer with "what?" lines before it gets
/// annoyed.  FACT (retail 1.23.10.13515, 2026-11-13: an SCV and a Probe
/// clicked 7-18 times, 2-4.5 s apart, the game audio recorded and every
/// line matched against the unit's `wht` / `pss` files): after selecting
/// the unit, clicks 1-4 say "what?", clicks 5.. play every annoyed line in
/// order, then the cycle starts over with four "what?" lines.  Time does
/// not reset the count (a 100 s pause carried it on); selecting another
/// unit does.  A click while the unit is still talking plays nothing and
/// does not count.
pub const WHAT_CLICKS: u32 = 4;

/// The line a selection click plays: `streak` is how many times in a row
/// (1-based) the player clicked this unit, `pissed_takes` how many annoyed
/// lines it has.  Clicks 1..=3 say "what?" (cycling takes); from the 4th
/// the annoyed lines play in order; after the last one it starts over
/// (`None` tells the caller to reset the streak to 1).
pub fn selection_cue(streak: u32, pissed_takes: usize) -> (VoiceCue, usize, bool) {
    let n = streak.max(1);
    if n <= WHAT_CLICKS || pissed_takes == 0 {
        return (
            VoiceCue::What,
            (n - 1) as usize,
            pissed_takes == 0 && n > WHAT_CLICKS,
        );
    }
    let k = (n - WHAT_CLICKS - 1) as usize;
    if k < pissed_takes {
        (VoiceCue::Pissed, k, false)
    } else {
        (VoiceCue::What, 0, true)
    }
}

/// The "what?" take for a selection click: random (`roll`), never the take
/// played just before (`prev`) when there is a choice.  Retail never
/// repeated a take back to back in ~30 recorded lines (INFERENCE that it
/// excludes the previous one; plain random repeats a quarter of the time).
pub fn pick_what(prev: Option<usize>, takes: usize, roll: usize) -> usize {
    if takes <= 1 {
        return 0;
    }
    let pick = roll % takes;
    match prev {
        Some(p) if p % takes == pick => (pick + 1 + roll / takes % (takes - 1)) % takes,
        _ => pick,
    }
}

/// Shipped announcer collections in Collections order, after Default.
/// The directory names are the commentators' names: Um Jae-kyung, Jeon
/// Yong-jun, Kim Jung-min, the three together, and HeyJini (INFERENCE from
/// the Korean names; the CASC paths are the shipped ones).
pub const ANNOUNCER_DIRS: [&str; 5] = ["jaekyung", "yongjun", "jungmin", "umjeonkim", "jini"];

/// `1..=5` is a shipped announcer; `0` is the default race advisor.
pub fn announcer_dir(id: u8) -> Option<&'static str> {
    ANNOUNCER_DIRS.get(id.checked_sub(1)? as usize).copied()
}

/// Rewrite an advisor path (`terran/advisor/taderr02.wav`) to the custom
/// announcer set (`scr/announcers/<dir>/taderr02.wav`).  The sets keep the
/// per-race `t…` / `p…` / `z…` stems and live directly under their directory;
/// non-advisor voices are left alone.
pub fn announcer_advisor_path(path: &str, id: u8) -> Option<String> {
    let dir = announcer_dir(id)?;
    let (_, tail) = path.split_once("/advisor/")?;
    let file = tail.rsplit('/').next()?;
    Some(format!("scr/announcers/{dir}/{file}"))
}

/// Lazily decoded voice clips.  Files live under `root/<race>/<unit>/...`
/// in the materialised cache.
pub struct VoiceBank {
    root: PathBuf,
    table: SfxTable,
    cache: RefCell<HashMap<u16, Option<Vec<i16>>>>,
    hits: RefCell<Vec<u16>>,
    announcer: Cell<u8>,
    classic: Cell<bool>,
}

impl VoiceBank {
    pub fn new(root: impl AsRef<Path>, table: SfxTable) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            table,
            cache: RefCell::new(HashMap::new()),
            hits: RefCell::new(Vec::new()),
            announcer: Cell::new(0),
            classic: Cell::new(false),
        }
    }

    /// Equip an announcer collection (`0` Default, `1..=5` shipped sets).
    pub fn set_announcer(&self, id: u8) {
        if self.announcer.get() != id {
            self.announcer.set(id);
            self.cache.borrow_mut().clear();
        }
    }

    pub fn announcer(&self) -> u8 {
        self.announcer.get()
    }

    /// Sound tab "Original 1998 Campaign": prefer `voices/sd/<rel>` when the
    /// install provides classic voices, otherwise keep the current set.
    pub fn set_classic(&self, classic: bool) {
        if self.classic.get() != classic {
            self.classic.set(classic);
            self.cache.borrow_mut().clear();
        }
    }

    /// First existing file in the selected announcer set (`gamestart_1.wav`,
    /// then `gamestart.wav`, for example).
    pub fn announcer_clip(&self, names: &[&str]) -> Option<Vec<i16>> {
        let dir = announcer_dir(self.announcer.get())?;
        for name in names {
            let path = self.root.join("scr/announcers").join(dir).join(name);
            let Ok(bytes) = std::fs::read(path) else {
                continue;
            };
            if let Some(wav) = Wav::parse(&bytes) {
                return Some(wav.to_mono_44k());
            }
        }
        None
    }

    pub fn table(&self) -> &SfxTable {
        &self.table
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Load (once) the WAV for a `sfxdata.tbl` id.  With a custom announcer
    /// equipped, advisor lines come from its set and fall back to the race
    /// advisor when the set has no matching file.
    pub fn clip(&self, id: u16) -> Option<Vec<i16>> {
        if let Some(hit) = self.cache.borrow().get(&id) {
            return hit.clone();
        }
        let mut candidates = Vec::new();
        if let Some(path) = self.table.cache_path(id)
            && let Some(over) = announcer_advisor_path(&path, self.announcer.get())
        {
            candidates.push(self.root.join(over));
        }
        if let Some(path) = self.table.cache_path(id) {
            if self.classic.get() {
                candidates.push(self.root.join("sd").join(&path));
            }
            candidates.push(self.root.join(path));
        }
        let decoded = candidates
            .into_iter()
            .find_map(|p| std::fs::read(p).ok())
            .and_then(|b| Wav::parse(&b))
            .map(|w| w.to_mono_44k());
        self.cache.borrow_mut().insert(id, decoded.clone());
        if decoded.is_some() {
            self.hits.borrow_mut().push(id);
        }
        decoded
    }

    /// How many takes of `cue` the unit has.
    pub fn takes(&self, unit_id: u16, cue: VoiceCue) -> usize {
        voice_ids(&self.table, unit_id, cue).len()
    }

    pub fn cue(&self, unit_id: u16, cue: VoiceCue, variant: usize) -> Option<Vec<i16>> {
        let id = voice_id(&self.table, unit_id, cue, variant)?;
        self.clip(id)
    }

    /// Number of voice files the bank can resolve without decoding them.
    pub fn available(&self) -> usize {
        (0..self.table.len() as u16)
            .filter(|id| {
                self.table
                    .cache_path(*id)
                    .is_some_and(|p| self.root.join(p).is_file())
            })
            .count()
    }

    /// Recently played ids, used by tests and the debug HUD.
    pub fn played(&self) -> Vec<u16> {
        self.hits.borrow().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_takes_are_random_but_never_repeat_back_to_back() {
        for prev in 0..4 {
            for roll in 0..64 {
                assert_ne!(pick_what(Some(prev), 4, roll), prev);
                assert!(pick_what(Some(prev), 4, roll) < 4);
            }
        }
        let firsts: std::collections::HashSet<usize> =
            (0..64).map(|r| pick_what(None, 4, r)).collect();
        assert_eq!(firsts.len(), 4, "every take can come first");
        assert_eq!(pick_what(Some(0), 1, 7), 0, "one take: no choice");
    }

    #[test]
    fn repeated_clicks_get_annoyed_then_start_over() {
        // A unit with four annoyed lines.
        let seq: Vec<(VoiceCue, usize, bool)> = (1..=10).map(|n| selection_cue(n, 4)).collect();
        assert_eq!(seq[0], (VoiceCue::What, 0, false));
        assert_eq!(
            seq[3],
            (VoiceCue::What, 3, false),
            "4th click still \"what?\""
        );
        assert_eq!(seq[4], (VoiceCue::Pissed, 0, false), "5th click");
        assert_eq!(seq[7], (VoiceCue::Pissed, 3, false), "last annoyed line");
        assert_eq!(seq[8], (VoiceCue::What, 0, true), "then it starts over");
        // A unit without annoyed lines keeps saying "what?".
        assert_eq!(selection_cue(5, 0).0, VoiceCue::What);
    }

    #[test]
    fn every_playable_unit_maps_to_a_shipped_voice_folder() {
        let table = SfxTable::bundled();
        let folders = table.folders();
        for id in [
            uid::MARINE,
            uid::GHOST,
            uid::VULTURE,
            uid::GOLIATH,
            uid::SIEGE_TANK,
            uid::SCV,
            uid::WRAITH,
            uid::SCIENCE_VESSEL,
            uid::DROPSHIP,
            uid::BATTLECRUISER,
            uid::FIREBAT,
            uid::MEDIC,
            uid::ZERGLING,
            uid::HYDRALISK,
            uid::ULTRALISK,
            uid::DRONE,
            uid::OVERLORD,
            uid::MUTALISK,
            uid::PROBE,
            uid::ZEALOT,
            uid::DRAGOON,
            uid::HIGH_TEMPLAR,
            uid::ARCHON,
            uid::SHUTTLE,
            uid::SCOUT,
            uid::ARBITER,
            uid::CARRIER,
            uid::REAVER,
            uid::OBSERVER,
            uid::CORSAIR,
            uid::DARK_TEMPLAR,
            uid::DARK_ARCHON,
        ] {
            let folder = folder_for(id).unwrap_or_else(|| panic!("unit {id} has no voice set"));
            assert!(folders.contains(&folder.to_string()), "unit {id}: {folder}");
        }
        assert_eq!(folder_for(uid::MINERAL_FIELD), None);
    }

    #[test]
    fn cues_resolve_to_the_shipped_takes() {
        let table = SfxTable::bundled();
        assert_eq!(
            table
                .cache_path(voice_id(&table, uid::MARINE, VoiceCue::Ready, 0).unwrap())
                .as_deref(),
            Some("terran/marine/tmardy00.wav")
        );
        assert_eq!(
            table
                .cache_path(voice_id(&table, uid::MARINE, VoiceCue::Yes, 0).unwrap())
                .as_deref(),
            Some("terran/marine/tmayes00.wav")
        );
        assert_eq!(
            table
                .cache_path(voice_id(&table, uid::MARINE, VoiceCue::Pissed, 2).unwrap())
                .as_deref(),
            Some("terran/marine/tmapss02.wav")
        );
        assert_eq!(
            table
                .cache_path(voice_id(&table, uid::ZEALOT, VoiceCue::Ready, 0).unwrap())
                .as_deref(),
            Some("protoss/zealot/pzerdy00.wav")
        );
    }

    #[test]
    fn announcer_sets_follow_the_collection_order() {
        assert_eq!(announcer_dir(0), None, "Default keeps the race advisor");
        assert_eq!(announcer_dir(1), Some("jaekyung"), "Um");
        assert_eq!(announcer_dir(2), Some("yongjun"), "Jeon");
        assert_eq!(announcer_dir(3), Some("jungmin"), "Kim");
        assert_eq!(announcer_dir(4), Some("umjeonkim"), "UmJeonKim");
        assert_eq!(announcer_dir(5), Some("jini"), "HeyJini");
        assert_eq!(announcer_dir(6), None);
    }

    #[test]
    fn announcer_sets_rewrite_advisor_paths_only() {
        assert_eq!(
            announcer_advisor_path("terran/advisor/taderr02.wav", 1).as_deref(),
            Some("scr/announcers/jaekyung/taderr02.wav")
        );
        assert_eq!(
            announcer_advisor_path("zerg/advisor/zadupd04.wav", 5).as_deref(),
            Some("scr/announcers/jini/zadupd04.wav")
        );
        assert_eq!(
            announcer_advisor_path("protoss/advisor/paderr00.wav", 4).as_deref(),
            Some("scr/announcers/umjeonkim/paderr00.wav")
        );
        assert_eq!(
            announcer_advisor_path("terran/marine/tmardy00.wav", 1),
            None
        );
        assert_eq!(
            announcer_advisor_path("terran/advisor/taderr02.wav", 0),
            None
        );
    }

    fn tiny_wav(samples: &[i16]) -> Vec<u8> {
        let data = samples.len() as u32 * 2;
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + data).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes()); // PCM
        b.extend_from_slice(&1u16.to_le_bytes()); // mono
        b.extend_from_slice(&44_100u32.to_le_bytes());
        b.extend_from_slice(&88_200u32.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&data.to_le_bytes());
        for s in samples {
            b.extend_from_slice(&s.to_le_bytes());
        }
        b
    }

    #[test]
    fn announcer_set_overrides_advisor_voices_and_start_lines() {
        let root = std::env::temp_dir().join(format!(
            "gc-announcer-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let table = SfxTable::bundled();
        let id = (0..table.len() as u16)
            .find(|&id| table.cache_path(id).as_deref() == Some("protoss/advisor/paderr00.wav"))
            .expect("advisor sound id");
        let base = root.join("protoss/advisor");
        let custom = root.join("scr/announcers/jini");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&custom).unwrap();
        std::fs::write(base.join("paderr00.wav"), tiny_wav(&[1, 1])).unwrap();
        std::fs::write(custom.join("paderr00.wav"), tiny_wav(&[9, 9])).unwrap();
        std::fs::write(custom.join("gamestart_1.wav"), tiny_wav(&[7, 7])).unwrap();

        let bank = VoiceBank::new(&root, table);
        assert_eq!(bank.clip(id).unwrap()[0], 1, "default advisor first");
        assert!(bank.announcer_clip(&["gamestart_1.wav"]).is_none());
        bank.set_announcer(5);
        assert_eq!(bank.clip(id).unwrap()[0], 9, "custom advisor first");
        assert_eq!(
            bank.announcer_clip(&["gamestart_1.wav"]).unwrap()[0],
            7,
            "custom start line"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn classic_campaign_prefers_the_sd_bank_and_falls_back() {
        let root = std::env::temp_dir().join(format!(
            "gc-classic-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let base = root.join("terran/marine");
        let sd = root.join("sd/terran/marine");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&sd).unwrap();
        let table = SfxTable::bundled();
        let id = (0..table.len() as u16)
            .find(|&id| table.cache_path(id).as_deref() == Some("terran/marine/tmardy00.wav"))
            .expect("marine ready id");
        std::fs::write(base.join("tmardy00.wav"), tiny_wav(&[1, 1])).unwrap();
        let bank = VoiceBank::new(&root, table);
        assert_eq!(bank.clip(id).unwrap()[0], 1);
        bank.set_classic(true);
        assert_eq!(bank.clip(id).unwrap()[0], 1, "fallback without sd file");
        std::fs::write(sd.join("tmardy00.wav"), tiny_wav(&[9, 9])).unwrap();
        bank.set_classic(false);
        bank.set_classic(true);
        assert_eq!(bank.clip(id).unwrap()[0], 9, "classic sd bank");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn cue_classification_covers_the_shipped_stems() {
        for (stem, cue) in [
            ("TMaRdy00", VoiceCue::Ready),
            ("TMaWht03", VoiceCue::What),
            ("TMaPss06", VoiceCue::Pissed),
            ("TMaYes02", VoiceCue::Yes),
            ("TMaDth01", VoiceCue::Death),
            ("TMaHit00", VoiceCue::Hit),
            ("ZZeAtk00", VoiceCue::Attack),
            ("PZeRag00", VoiceCue::Special),
        ] {
            assert_eq!(VoiceCue::from_stem(stem), Some(cue), "{stem}");
        }
    }
}
